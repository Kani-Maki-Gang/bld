use std::sync::Arc;

use anyhow::{Result, bail};
use bld_config::{BldConfig, definitions::PACKAGE_ACTION_FILE_NAME, path};
use git2::Repository;
use regex::Regex;
use std::path::PathBuf;
use tokio::{fs::File, io::AsyncReadExt};
use tracing::error;

pub struct RepositoryBranch {
    name: String,
    refname: String,
    head: String,
}

pub struct RepositoryInfo {
    pub url: String,
    pub name: String,
    pub branch: Option<RepositoryBranch>,
}

pub struct PackageManager {
    config: Arc<BldConfig>,
    regex: Regex,
}

impl PackageManager {
    pub fn new(config: Arc<BldConfig>) -> Self {
        // Regex to parse git URLs (HTTPS and SSH) with optional @branch/tag
        // Examples:
        //   https://github.com/user/repo.git@branch
        //   git@github.com:user/repo.git@tag
        // Captures:
        //   1: Full URL without @branch (e.g., https://github.com/user/repo.git)
        //   2: Repository name (e.g., repo)
        //   3: Branch/tag (e.g., main)
        let regex = Regex::new(
            r"^((?:https?://[^/]+/|git@[^:]+:)(?:[^/]+/)*([^@/]+?)(?:\.git)?)(?:@(.+))?$",
        )
        .expect("Invalid regex pattern");

        Self { config, regex }
    }

    fn resolve_info(&self, source: &str) -> Result<RepositoryInfo> {
        let Some(captures) = self.regex.captures(source) else {
            bail!("Failed to parse git repository URL: {}", source);
        };

        let url = captures
            .get(1)
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| anyhow::anyhow!("Failed to extract repository URL"))?;

        let name = captures
            .get(2)
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| anyhow::anyhow!("Failed to extract repository name"))?;

        let branch = captures.get(3).map(|m| {
            let name = m.as_str().to_string();
            let refname = format!("refs/remotes/origin/{name}");
            let head = format!("refs/heads/{name}");
            RepositoryBranch {
                name,
                refname,
                head,
            }
        });

        Ok(RepositoryInfo { url, name, branch })
    }

    fn repository_path(&self, info: &RepositoryInfo) -> PathBuf {
        let dir = info
            .branch
            .as_ref()
            .map(|b| format!("{}@{}", &info.name, b.name))
            .unwrap_or_else(|| info.name.clone());
        path![&self.config.local.packages.cache, dir]
    }

    pub async fn exists(&self, source: &str) -> bool {
        let Ok(info) = self.resolve_info(source) else {
            return false;
        };
        let repository_path = self.repository_path(&info);
        repository_path.exists()
    }

    pub async fn get(&self, source: &str) -> Result<()> {
        let info = self.resolve_info(source)?;
        let repository_path = self.repository_path(&info);
        let repository = Repository::clone(&info.url, &repository_path)?;

        // Checkout the specified branch/tag if provided
        if let Some(branch) = &info.branch {
            // Try remote branch first, then tag
            let tag_ref = format!("refs/tags/{}", branch.name);

            let (commit, is_branch) =
                if let Ok(obj) = repository.revparse_single(&branch.refname) {
                    (obj.peel_to_commit()?, true)
                } else if let Ok(obj) = repository.revparse_single(&tag_ref) {
                    (obj.peel_to_commit()?, false)
                } else {
                    bail!(
                        "Unable to find branch or tag '{}' in repository",
                        branch.name
                    );
                };

            repository.checkout_tree(commit.as_object(), None)?;

            if is_branch {
                // For branches, create local branch and set HEAD
                repository.branch(&branch.name, &commit, false)?;
                repository.set_head(&branch.head)?;
            } else {
                // For tags, set HEAD to detached state
                repository.set_head_detached(commit.id())?;
            }
        }

        Ok(())
    }

    pub async fn is_synced(&self, source: &str) -> bool {
        let Ok(info) = self.resolve_info(source).inspect_err(|e| {
            error!(
                "unable to resolve repository information due to {}",
                e.to_string()
            )
        }) else {
            return false;
        };

        let repository_path = self.repository_path(&info);
        let Ok(repository) = Repository::open(&repository_path)
            .inspect_err(|e| error!("unable to open git repository due to {}", e.to_string()))
        else {
            return false;
        };

        // Get the ref name (from info or default to HEAD)
        let ref_name = match &info.branch {
            Some(branch) => branch.name.clone(),
            None => {
                let Ok(head) = repository.head() else {
                    error!("unable to get HEAD reference");
                    return false;
                };
                match head.shorthand() {
                    Some(name) => name.to_string(),
                    None => {
                        error!("unable to get branch name from HEAD");
                        return false;
                    }
                }
            }
        };

        // Fetch from remote (fetch all to get both branches and tags)
        let Ok(mut remote) = repository.find_remote("origin") else {
            error!("unable to find remote 'origin'");
            return false;
        };

        if let Err(e) = remote.fetch::<&str>(&[], None, None) {
            error!("unable to fetch from remote: {}", e);
            return false;
        }

        // Get local HEAD commit
        let Ok(head) = repository.head() else {
            error!("unable to get HEAD");
            return false;
        };
        let Ok(local_oid) = head.peel_to_commit().map(|c| c.id()) else {
            error!("unable to get local commit");
            return false;
        };

        // Resolve remote ref using revparse (handles both branches and tags)
        let remote_spec = if repository.find_reference(&format!("refs/remotes/origin/{}", ref_name)).is_ok() {
            format!("refs/remotes/origin/{}", ref_name)
        } else {
            format!("refs/tags/{}", ref_name)
        };

        let Ok(remote_obj) = repository.revparse_single(&remote_spec) else {
            error!("unable to find remote reference: {}", remote_spec);
            return false;
        };
        let Ok(remote_oid) = remote_obj.peel_to_commit().map(|c| c.id()) else {
            error!("unable to get remote commit");
            return false;
        };

        local_oid == remote_oid
    }

    pub async fn sync(&self, source: &str) -> Result<()> {
        let info = self.resolve_info(source)?;
        let repository_path = self.repository_path(&info);
        let repository = Repository::open(&repository_path)?;

        // Get the ref name (from info or default to HEAD)
        let ref_name = match &info.branch {
            Some(branch) => branch.name.clone(),
            None => {
                let head = repository.head()?;
                head.shorthand()
                    .ok_or_else(|| anyhow::anyhow!("unable to get branch name from HEAD"))?
                    .to_string()
            }
        };

        // Fetch from remote (fetch all to get both branches and tags)
        let mut remote = repository.find_remote("origin")?;
        remote.fetch::<&str>(&[], None, None)?;

        // Resolve remote ref using revparse (handles both branches and tags)
        let remote_spec = if repository.find_reference(&format!("refs/remotes/origin/{}", ref_name)).is_ok() {
            format!("refs/remotes/origin/{}", ref_name)
        } else {
            format!("refs/tags/{}", ref_name)
        };

        let remote_obj = repository.revparse_single(&remote_spec)?;
        let remote_commit = remote_obj.peel_to_commit()?;

        // Reset to the remote commit
        repository.checkout_tree(remote_commit.as_object(), None)?;

        // Check if this is a branch or tag
        let is_branch = repository.find_reference(&format!("refs/remotes/origin/{}", ref_name)).is_ok();

        if is_branch {
            repository.reset(remote_commit.as_object(), git2::ResetType::Hard, None)?;
        } else {
            // For tags, set HEAD to detached state
            repository.set_head_detached(remote_commit.id())?;
        }

        Ok(())
    }

    pub async fn read(&self, source: &str) -> Result<String> {
        let info = self.resolve_info(source)?;
        let repository_path = self.repository_path(&info);
        let file_path = path![&repository_path, PACKAGE_ACTION_FILE_NAME];
        let mut handle = File::open(file_path).await?;
        let mut content = String::new();
        handle.read_to_string(&mut content).await?;
        Ok(content)
    }
}
