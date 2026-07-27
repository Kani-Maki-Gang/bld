use std::{path::Path, sync::Arc};

use anyhow::{Result, anyhow, bail};
use bld_config::{BldConfig, SshConfig, SshUserAuth, definitions::PACKAGE_ACTION_FILE_NAME, path};
use git2::{Cred, FetchOptions, RemoteCallbacks, Repository, build::RepoBuilder};
use std::path::PathBuf;
use tokio::{
    fs::{File, remove_dir_all},
    io::AsyncReadExt,
    task::spawn_blocking,
};
use tracing::{error, warn};

#[derive(Clone)]
struct RepositoryBranch {
    name: String,
    refname: String,
    head: String,
}

#[derive(Clone)]
enum RepositoryUrl {
    Ssh { raw: String, host: String },
    Http { raw: String },
}

impl RepositoryUrl {
    fn raw(&self) -> &str {
        match self {
            Self::Ssh { raw, .. } | Self::Http { raw } => raw,
        }
    }
}

#[derive(Clone)]
struct RepositoryInfo {
    pub url: RepositoryUrl,
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

    fn validate_path_segment(segment: &str) -> Result<()> {
        if segment.is_empty()
            || segment == "."
            || segment == ".."
            || segment.contains('/')
            || segment.contains('\\')
        {
            bail!("Invalid or unsafe package reference segment '{segment}'");
        }
        Ok(())
    }

    fn repo_info(&self, source: &str) -> Result<RepositoryInfo> {
        let mut branch: Option<RepositoryBranch> = None;
        let mut url = source.to_string();

        if let Some((left, right)) = source.rsplit_once(".git@") {
            let name = right.to_string();
            Self::validate_path_segment(&name)?;
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
        Self::validate_path_segment(&name)?;

        let repo_url = if url.starts_with("git@") {
            let host = url
                .replace("git@", "")
                .rsplit_once(":")
                .ok_or_else(|| anyhow!("unable to deduce host"))?
                .0
                .to_string();
            RepositoryUrl::Ssh { raw: url, host }
        } else {
            RepositoryUrl::Http { raw: url }
        };

        Ok(RepositoryInfo {
            url: repo_url,
            name,
            branch,
        })
    }

    fn repo_path(&self, info: &RepositoryInfo) -> PathBuf {
        let dir = info
            .branch
            .as_ref()
            .map(|b| format!("{}@{}", &info.name, b.name))
            .unwrap_or_else(|| info.name.clone());
        path![&self.config.local.packages.cache, dir]
    }

    fn ssh_credentials(ssh_config: &SshConfig, user: &str) -> Result<Cred, git2::Error> {
        match &ssh_config.userauth {
            SshUserAuth::Keys {
                private_key,
                public_key,
            } => Cred::ssh_key(
                user,
                public_key.as_deref().map(Path::new),
                Path::new(&private_key),
                None,
            ),
            SshUserAuth::Password { password } => Cred::userpass_plaintext(user, password),
            SshUserAuth::Agent => Cred::ssh_key_from_agent(user),
        }
    }

    fn repo_fetch_options<'a>(
        config: Arc<BldConfig>,
        info: &'a RepositoryInfo,
    ) -> FetchOptions<'a> {
        let mut callbacks = RemoteCallbacks::new();

        callbacks.credentials(move |_url, username_from_url, _allowed_types| {
            let user = username_from_url.unwrap_or("git");

            if let RepositoryUrl::Ssh { host: ssh_host, .. } = &info.url {
                let ssh = &config.local.ssh;

                let Some(ssh_config) = ssh.iter().find(|x| &x.1.host == ssh_host).map(|x| x.1)
                else {
                    return Cred::default();
                };

                Self::ssh_credentials(ssh_config, user)
            } else {
                Cred::username(user)
            }
        });

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        fetch_options
    }

    fn repo_builder<'a>(config: Arc<BldConfig>, info: &'a RepositoryInfo) -> RepoBuilder<'a> {
        let mut builder = RepoBuilder::new();
        builder.fetch_options(Self::repo_fetch_options(config, info));
        builder
    }

    fn repo_clone(config: Arc<BldConfig>, info: RepositoryInfo, path: &Path) -> Result<Repository> {
        let mut builder = Self::repo_builder(config, &info);
        builder.clone(info.url.raw(), path).map_err(|e| anyhow!(e))
    }

    fn repo_open(config: Arc<BldConfig>, info: RepositoryInfo, path: &Path) -> Result<Repository> {
        let repo = Repository::open(path)?;
        {
            let mut remote = repo.find_remote("origin")?;
            let mut fetch_options = Self::repo_fetch_options(config, &info);
            remote.fetch::<&str>(&[], Some(&mut fetch_options), None)?;
        }
        Ok(repo)
    }

    pub fn is_package(&self, source: &str) -> bool {
        self.repo_info(source).is_ok()
    }

    pub fn exists(&self, source: &str) -> bool {
        let Ok(info) = self.repo_info(source) else {
            return false;
        };
        let repository_path = self.repo_path(&info);
        repository_path.exists()
    }

    pub async fn get(&self, source: &str) -> Result<()> {
        let info = self.repo_info(source)?;
        let path = self.repo_path(&info);

        if path.exists() {
            remove_dir_all(&path).await?;
        }

        let info_clone = info.clone();
        let config = self.config.clone();
        let repository =
            spawn_blocking(move || Self::repo_clone(config, info_clone, &path)).await??;

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
        let Ok(info) = self.repo_info(source).inspect_err(|e| {
            error!(
                "unable to resolve repository information due to {}",
                e.to_string()
            )
        }) else {
            return false;
        };

        let path = self.repo_path(&info);
        let mut ref_name = info
            .branch
            .as_ref()
            .map(|x| x.name.clone())
            .unwrap_or_default();
        let config = self.config.clone();
        let Ok(repository_task) = spawn_blocking(move || Self::repo_open(config, info, &path))
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

        if ref_name.is_empty() {
            let Ok(head) = repository.head() else {
                error!("unable to get HEAD reference");
                return false;
            };

            let Some(head) = head.shorthand() else {
                error!("unable to get branch name from HEAD");
                return false;
            };

            ref_name = head.to_string()
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
        let info = self.repo_info(source)?;
        let path = self.repo_path(&info);
        let mut ref_name = info
            .branch
            .as_ref()
            .map(|x| x.name.clone())
            .unwrap_or_default();
        let config = self.config.clone();
        let repository = spawn_blocking(move || Self::repo_open(config, info, &path)).await??;

        if ref_name.is_empty() {
            let head = repository.head()?;
            ref_name = head
                .shorthand()
                .ok_or_else(|| anyhow::anyhow!("unable to get branch name from HEAD"))?
                .to_string();
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
        let info = self.repo_info(source)?;
        let repository_path = self.repo_path(&info);
        let file_path = path![&repository_path, PACKAGE_ACTION_FILE_NAME];
        let mut handle = File::open(file_path).await?;
        let mut content = String::new();
        handle.read_to_string(&mut content).await?;
        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bld_config::{BldPackages, SshConfig, SshUserAuth};

    fn test_manager(cache: &str) -> PackageManager {
        let mut config = BldConfig::default();
        config.local.packages = BldPackages {
            cache: cache.to_string(),
            strict_sync: false,
        };
        PackageManager::new(Arc::new(config))
    }

    #[test]
    fn repo_info_parses_https_url_without_ref() {
        let manager = test_manager("/tmp/bld_pkg_cache");
        let info = manager
            .repo_info("https://example.com/org/repo.git")
            .unwrap();
        assert_eq!(info.name, "repo.git");
        assert!(info.branch.is_none());
        assert!(matches!(info.url, RepositoryUrl::Http { .. }));
    }

    #[test]
    fn repo_info_parses_https_url_with_branch_ref() {
        let manager = test_manager("/tmp/bld_pkg_cache");
        let info = manager
            .repo_info("https://example.com/org/repo.git@main")
            .unwrap();
        assert_eq!(info.name, "repo.git");
        let branch = info.branch.unwrap();
        assert_eq!(branch.name, "main");
        assert_eq!(branch.refname, "refs/remotes/origin/main");
        assert_eq!(branch.head, "refs/heads/main");
    }

    #[test]
    fn repo_info_parses_ssh_url_and_extracts_host() {
        let manager = test_manager("/tmp/bld_pkg_cache");
        let info = manager
            .repo_info("git@github.com:org/repo.git@v1.0.0")
            .unwrap();
        assert_eq!(info.name, "repo.git");
        match info.url {
            RepositoryUrl::Ssh { host, .. } => assert_eq!(host, "github.com"),
            _ => panic!("expected ssh url"),
        }
    }

    #[test]
    fn repo_info_rejects_path_traversal_in_ref() {
        let manager = test_manager("/tmp/bld_pkg_cache");
        let result = manager.repo_info("https://example.com/org/repo.git@../../../etc");
        assert!(result.is_err());
    }

    #[test]
    fn repo_info_rejects_path_separator_in_ref() {
        let manager = test_manager("/tmp/bld_pkg_cache");
        let result = manager.repo_info("https://example.com/org/repo.git@feature/foo");
        assert!(result.is_err());
    }

    #[test]
    fn repo_info_rejects_empty_repo_name() {
        let manager = test_manager("/tmp/bld_pkg_cache");
        let result = manager.repo_info("https://example.com/org/");
        assert!(result.is_err());
    }

    #[test]
    fn repo_path_without_branch_uses_repo_name() {
        let manager = test_manager("/tmp/bld_pkg_cache");
        let info = manager
            .repo_info("https://example.com/org/repo.git")
            .unwrap();
        let path = manager.repo_path(&info);
        assert_eq!(path, PathBuf::from("/tmp/bld_pkg_cache/repo.git"));
    }

    #[test]
    fn repo_path_with_branch_includes_branch_name() {
        let manager = test_manager("/tmp/bld_pkg_cache");
        let info = manager
            .repo_info("https://example.com/org/repo.git@main")
            .unwrap();
        let path = manager.repo_path(&info);
        assert_eq!(path, PathBuf::from("/tmp/bld_pkg_cache/repo.git@main"));
    }

    #[test]
    fn is_package_true_for_valid_source() {
        let manager = test_manager("/tmp/bld_pkg_cache");
        assert!(manager.is_package("https://example.com/org/repo.git"));
    }

    #[test]
    fn is_package_false_for_traversal_attempt() {
        let manager = test_manager("/tmp/bld_pkg_cache");
        assert!(!manager.is_package("https://example.com/org/repo.git@../../etc"));
    }

    #[test]
    fn exists_false_for_unknown_package() {
        let manager = test_manager("/tmp/bld_pkg_cache_nonexistent_xyz");
        assert!(!manager.exists("https://example.com/org/repo.git"));
    }

    #[test]
    fn ssh_credentials_agent_variant() {
        let ssh_config = SshConfig {
            host: "example.com".to_string(),
            port: SshConfig::default_port(),
            user: "git".to_string(),
            userauth: SshUserAuth::Agent,
        };
        let cred = PackageManager::ssh_credentials(&ssh_config, "git").unwrap();
        assert!(git2::CredentialType::from_bits_truncate(cred.credtype() as u32).is_ssh_key());
    }

    #[test]
    fn ssh_credentials_password_variant() {
        let ssh_config = SshConfig {
            host: "example.com".to_string(),
            port: SshConfig::default_port(),
            user: "git".to_string(),
            userauth: SshUserAuth::Password {
                password: "secret".to_string(),
            },
        };
        let cred = PackageManager::ssh_credentials(&ssh_config, "git").unwrap();
        assert!(
            git2::CredentialType::from_bits_truncate(cred.credtype() as u32)
                .is_user_pass_plaintext()
        );
    }

    #[test]
    fn ssh_credentials_keys_variant() {
        let ssh_config = SshConfig {
            host: "example.com".to_string(),
            port: SshConfig::default_port(),
            user: "git".to_string(),
            userauth: SshUserAuth::Keys {
                public_key: None,
                private_key: "/tmp/does-not-need-to-exist".to_string(),
            },
        };
        let cred = PackageManager::ssh_credentials(&ssh_config, "git").unwrap();
        assert!(git2::CredentialType::from_bits_truncate(cred.credtype() as u32).is_ssh_key());
    }
}
