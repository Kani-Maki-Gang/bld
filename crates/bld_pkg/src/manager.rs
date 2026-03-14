use std::sync::Arc;

use anyhow::{Result, anyhow, bail};
use bld_config::{BldConfig, definitions::PACKAGE_ACTION_FILE_NAME, path};
use git2::Repository;
use std::path::PathBuf;
use tokio::{fs::File, io::AsyncReadExt, task::spawn_blocking};
use tracing::{error, warn};

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
}

impl PackageManager {
    pub fn new(config: Arc<BldConfig>) -> Self {
        Self { config }
    }

    fn resolve_info(&self, source: &str) -> Result<RepositoryInfo> {
        let mut branch: Option<RepositoryBranch> = None;
        let mut url = source.to_string();

        if let Some((left, right)) = source.rsplit_once(".git@") {
            let name = right.to_string();
            let refname = format!("refs/remotes/origin/{name}");
            let head = format!("refs/heads/{name}");
            branch = Some(RepositoryBranch {
                name,
                refname,
                head,
            });
            url = format!("{left}.git");
        }

        let (_, name) = url
            .rsplit_once("/")
            .ok_or_else(|| anyhow!("Unable to deduce repository name for package {source}"))?;
        let name = name.to_string();

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

    pub fn is_package(&self, source: &str) -> bool {
        self.resolve_info(source).is_ok()
    }

    pub fn exists(&self, source: &str) -> bool {
        let Ok(info) = self.resolve_info(source) else {
            return false;
        };
        let repository_path = self.repository_path(&info);
        repository_path.exists()
    }

    pub async fn get(&self, source: &str) -> Result<()> {
        let info = self.resolve_info(source)?;
        let repository_path = self.repository_path(&info);
        let repository =
            spawn_blocking(move || Repository::clone(&info.url, &repository_path)).await??;

        if let Some(branch) = &info.branch {
            let tag_ref = format!("refs/tags/{}", branch.name);

            let (commit, is_branch) = if let Ok(obj) = repository.revparse_single(&branch.refname) {
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
                repository.branch(&branch.name, &commit, false)?;
                repository.set_head(&branch.head)?;
            } else {
                repository.set_head_detached(commit.id())?;
            }
        }

        Ok(())
    }

    async fn is_synced(&self, source: &str) -> bool {
        let Ok(info) = self.resolve_info(source).inspect_err(|e| {
            error!(
                "unable to resolve repository information due to {}",
                e.to_string()
            )
        }) else {
            return false;
        };

        let repository_path = self.repository_path(&info);

        let Ok(repository_task) = spawn_blocking(move || Repository::open(&repository_path))
            .await
            .inspect_err(|e| error!("unable to spawn repository open task due to {e}"))
        else {
            return false;
        };

        let Ok(repository) =
            repository_task.inspect_err(|e| error!("unable to open git repository due to {e}"))
        else {
            return false;
        };

        let ref_name = if let Some(branch) = info.branch {
            branch.name.clone()
        } else {
            let Ok(head) = repository.head() else {
                error!("unable to get HEAD reference");
                return false;
            };

            let Some(head) = head.shorthand() else {
                error!("unable to get branch name from HEAD");
                return false;
            };

            head.to_string()
        };

        let Ok(mut remote) = repository.find_remote("origin") else {
            error!("unable to find remote 'origin'");
            return false;
        };

        if let Err(e) = remote.fetch::<&str>(&[], None, None) {
            error!("unable to fetch from remote: {}", e);
            return false;
        }

        let Ok(head) = repository.head() else {
            error!("unable to get HEAD");
            return false;
        };
        let Ok(local_oid) = head.peel_to_commit().map(|c| c.id()) else {
            error!("unable to get local commit");
            return false;
        };

        let remote_spec = if repository
            .find_reference(&format!("refs/remotes/origin/{}", ref_name))
            .is_ok()
        {
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

    async fn sync(&self, source: &str) -> Result<()> {
        let info = self.resolve_info(source)?;
        let repository_path = self.repository_path(&info);

        let repository = spawn_blocking(move || -> Result<Repository> {
            let repo = Repository::open(&repository_path)?;
            {
                let mut remote = repo.find_remote("origin")?;
                remote.fetch::<&str>(&[], None, None)?;
            }
            Ok(repo)
        })
        .await??;

        let ref_name = if let Some(branch) = info.branch {
            branch.name.clone()
        } else {
            let head = repository.head()?;
            head.shorthand()
                .ok_or_else(|| anyhow::anyhow!("unable to get branch name from HEAD"))?
                .to_string()
        };

        let remote_spec = if repository
            .find_reference(&format!("refs/remotes/origin/{}", ref_name))
            .is_ok()
        {
            format!("refs/remotes/origin/{}", ref_name)
        } else {
            format!("refs/tags/{}", ref_name)
        };

        let remote_obj = repository.revparse_single(&remote_spec)?;
        let remote_commit = remote_obj.peel_to_commit()?;

        repository.checkout_tree(remote_commit.as_object(), None)?;

        let is_branch = repository
            .find_reference(&format!("refs/remotes/origin/{}", ref_name))
            .is_ok();

        if is_branch {
            repository.reset(remote_commit.as_object(), git2::ResetType::Hard, None)?;
        } else {
            repository.set_head_detached(remote_commit.id())?;
        }

        Ok(())
    }

    pub async fn try_sync(&self, source: &str) -> Result<()> {
        if self.is_synced(source).await {
            return Ok(());
        }

        let sync_res = self.sync(source).await;
        if self.config.local.packages.strict_sync {
            return sync_res;
        }

        if let Err(e) = sync_res {
            warn!("unable to sync package due to {}", e.to_string());
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
