use std::{path::Path, sync::Arc};

use anyhow::{Result, anyhow, bail};
use bld_config::{BldConfig, SshConfig, SshUserAuth, definitions::PACKAGE_ACTION_FILE_NAME, path};
use fs4::AsyncFileExt;
use git2::{Cred, FetchOptions, RemoteCallbacks, Repository, build::RepoBuilder};
use std::path::PathBuf;
use tokio::{
    fs::{File, OpenOptions, create_dir_all, remove_dir_all},
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
struct Package {
    config: Arc<BldConfig>,
    pub url: RepositoryUrl,
    pub name: String,
    pub branch: Option<RepositoryBranch>,
}

impl Package {
    pub fn from_source(config: Arc<BldConfig>, source: &str) -> Result<Self> {
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

        Ok(Self {
            config,
            url: repo_url,
            name,
            branch,
        })
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

    pub fn path(&self) -> PathBuf {
        let dir = self
            .branch
            .as_ref()
            .map(|b| format!("{}@{}", self.name, b.name))
            .unwrap_or_else(|| self.name.clone());
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

    fn fetch_options<'a>(&'a self) -> FetchOptions<'a> {
        let mut callbacks = RemoteCallbacks::new();

        callbacks.credentials(move |_url, username_from_url, _allowed_types| {
            let user = username_from_url.unwrap_or("git");

            if let RepositoryUrl::Ssh { host: ssh_host, .. } = &self.url {
                let ssh = &self.config.local.ssh;

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

    pub fn to_builder(&self) -> RepoBuilder<'_> {
        let mut builder = RepoBuilder::new();
        builder.fetch_options(self.fetch_options());
        builder
    }

    fn git_clone(&self, path: &Path) -> Result<Repository> {
        let mut builder = self.to_builder();
        builder.clone(self.url.raw(), path).map_err(|e| anyhow!(e))
    }

    fn open(&self, path: &Path) -> Result<Repository> {
        let repo = Repository::open(path)?;
        {
            let mut remote = repo.find_remote("origin")?;
            let mut fetch_options = self.fetch_options();
            remote.fetch::<&str>(&[], Some(&mut fetch_options), None)?;
        }
        Ok(repo)
    }

    async fn lock(&self) -> Result<File> {
        let mut path = self.path();
        let file_name = path
            .file_name()
            .ok_or_else(|| anyhow!("Unable to create package lock file path"))?;
        path.set_file_name(format!("lock_{}", file_name.display()));
        let file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(&path)
            .await?;
        spawn_blocking(move || {
            if let Err(e) = file.lock() {
                bail!(e);
            }
            Ok(file)
        })
        .await?
    }
}

pub struct PackageManager {
    config: Arc<BldConfig>,
}

impl PackageManager {
    pub fn new(config: Arc<BldConfig>) -> Self {
        Self { config }
    }

    fn package(&self, source: &str) -> Result<Package> {
        Package::from_source(self.config.clone(), source)
    }

    pub fn is_package(&self, source: &str) -> bool {
        self.package(source).is_ok()
    }

    async fn is_git_repository(&self, package: &Package) -> bool {
        let ceiling = path![&self.config.local.packages.cache];
        let path = package.path();
        if path.exists() {
            let package = package.clone();
            let result = spawn_blocking(move || Repository::discover_path(&path, &ceiling)).await;

            return match result {
                Ok(Ok(_)) => true,
                Ok(Err(e)) => {
                    warn!(
                        "No valid git repository found for package {} due to {e}",
                        package.name
                    );
                    false
                }
                Err(e) => {
                    warn!(
                        "No valid git repository found for package {} due to {e}",
                        package.name
                    );
                    false
                }
            };
        }
        false
    }

    async fn get(&self, package: Package) -> Result<()> {
        let path = package.path();
        let branch = package.branch.clone();

        if path.exists() {
            remove_dir_all(&path).await?;
        }

        let repository = spawn_blocking(move || package.git_clone(&path)).await??;

        if let Some(branch) = &branch {
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
                // The clone already creates a local branch for the remote's default branch, and
                // git refuses to update it while it is the current HEAD.
                if repository
                    .find_branch(&branch.name, git2::BranchType::Local)
                    .is_err()
                {
                    repository.branch(&branch.name, &commit, false)?;
                }
                repository.set_head(&branch.head)?;
            } else {
                repository.set_head_detached(commit.id())?;
            }
        }

        Ok(())
    }

    async fn is_synced(&self, package: &Package) -> bool {
        let package = package.clone();
        let mut ref_name = package
            .branch
            .as_ref()
            .map(|x| x.name.clone())
            .unwrap_or_default();
        let Ok(repository_task) = spawn_blocking(move || package.open(&package.path()))
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

    async fn sync(&self, package: &Package) -> Result<()> {
        let package = package.clone();
        let mut ref_name = package
            .branch
            .as_ref()
            .map(|x| x.name.clone())
            .unwrap_or_default();
        let repository = spawn_blocking(move || package.open(&package.path())).await??;

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

    async fn try_sync(&self, package: &Package) -> Result<()> {
        if self.is_synced(package).await {
            return Ok(());
        }

        let sync_res = self.sync(package).await;
        if self.config.local.packages.strict_sync {
            return sync_res;
        }

        if let Err(e) = sync_res {
            warn!("unable to sync package due to {}", e.to_string());
        }

        Ok(())
    }

    async fn create_cache_dir(&self) -> Result<()> {
        let path = path![&self.config.local.packages.cache];
        create_dir_all(&path).await.map_err(|e| anyhow!(e))
    }

    pub async fn read(&self, source: &str) -> Result<String> {
        self.create_cache_dir().await?;
        let package = self.package(source)?;
        let _lock_file = package.lock().await?;
        if self.is_git_repository(&package).await {
            self.try_sync(&package).await?;
        } else {
            self.get(package).await?;
        }
        let package = self.package(source)?;
        let file_path = path![&package.path(), PACKAGE_ACTION_FILE_NAME];
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
    use git2::{RepositoryInitOptions, Signature};
    use std::fs;
    use tempfile::TempDir;

    const ACTION: &str = "version: 3\nname: probe\nrunsOn: machine\n";

    fn test_manager(cache: &str) -> PackageManager {
        let mut config = BldConfig::default();
        config.local.packages = BldPackages {
            cache: cache.to_string(),
            strict_sync: false,
        };
        PackageManager::new(Arc::new(config))
    }

    /// Builds a local repository that stands in for a remote package. It holds the action file on
    /// `main`, a `feature` branch with a second commit and a `v1` tag on the first commit, so every
    /// form of package reference can be exercised from a single fixture.
    fn origin_repo(dir: &Path, name: &str) -> PathBuf {
        let path = path![dir, format!("{name}.git")];
        let mut options = RepositoryInitOptions::new();
        options.initial_head("refs/heads/main");
        let repository = Repository::init_opts(&path, &options).unwrap();

        let signature = Signature::now("bld", "bld@example.com").unwrap();
        let action = Path::new(PACKAGE_ACTION_FILE_NAME);
        fs::write(path![&path, PACKAGE_ACTION_FILE_NAME], ACTION).unwrap();

        let first = {
            let mut index = repository.index().unwrap();
            index.add_path(action).unwrap();
            index.write().unwrap();
            let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
            let oid = repository
                .commit(Some("HEAD"), &signature, &signature, "init", &tree, &[])
                .unwrap();
            repository.find_commit(oid).unwrap()
        };

        repository
            .tag_lightweight("v1", first.as_object(), false)
            .unwrap();
        repository.branch("feature", &first, false).unwrap();
        repository.set_head("refs/heads/feature").unwrap();

        {
            fs::write(
                path![&path, PACKAGE_ACTION_FILE_NAME],
                format!("{ACTION}# feature\n"),
            )
            .unwrap();
            let mut index = repository.index().unwrap();
            index.add_path(action).unwrap();
            index.write().unwrap();
            let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
            repository
                .commit(
                    Some("HEAD"),
                    &signature,
                    &signature,
                    "feature",
                    &tree,
                    &[&first],
                )
                .unwrap();
        }

        repository.set_head("refs/heads/main").unwrap();
        repository
            .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
            .unwrap();

        path
    }

    /// Owns the temporary cache and origin directories so they outlive the manager under test.
    struct TestEnv {
        cache: TempDir,
        origins: TempDir,
        manager: Arc<PackageManager>,
    }

    impl TestEnv {
        fn new() -> Self {
            let cache = TempDir::new().unwrap();
            let origins = TempDir::new().unwrap();
            let manager = Arc::new(test_manager(cache.path().to_str().unwrap()));
            Self {
                cache,
                origins,
                manager,
            }
        }

        /// Creates a fixture repository and returns the source string that refers to it.
        fn source(&self, name: &str) -> String {
            origin_repo(self.origins.path(), name)
                .to_str()
                .unwrap()
                .to_string()
        }

        fn cache_path(&self) -> &Path {
            self.cache.path()
        }

        fn cache_entries(&self) -> Vec<String> {
            let mut entries: Vec<String> = fs::read_dir(self.cache.path())
                .unwrap()
                .map(|x| x.unwrap().file_name().to_string_lossy().to_string())
                .collect();
            entries.sort();
            entries
        }
    }

    #[test]
    fn repo_info_parses_https_url_without_ref() {
        let manager = test_manager("/tmp/bld_pkg_cache");
        let package = manager.package("https://example.com/org/repo.git").unwrap();
        assert_eq!(package.name, "repo.git");
        assert!(package.branch.is_none());
        assert!(matches!(package.url, RepositoryUrl::Http { .. }));
    }

    #[test]
    fn repo_info_parses_https_url_with_branch_ref() {
        let manager = test_manager("/tmp/bld_pkg_cache");
        let package = manager
            .package("https://example.com/org/repo.git@main")
            .unwrap();
        assert_eq!(package.name, "repo.git");
        let branch = package.branch.unwrap();
        assert_eq!(branch.name, "main");
        assert_eq!(branch.refname, "refs/remotes/origin/main");
        assert_eq!(branch.head, "refs/heads/main");
    }

    #[test]
    fn repo_info_parses_ssh_url_and_extracts_host() {
        let manager = test_manager("/tmp/bld_pkg_cache");
        let package = manager
            .package("git@github.com:org/repo.git@v1.0.0")
            .unwrap();
        assert_eq!(package.name, "repo.git");
        match package.url {
            RepositoryUrl::Ssh { host, .. } => assert_eq!(host, "github.com"),
            _ => panic!("expected ssh url"),
        }
    }

    #[test]
    fn repo_info_rejects_path_traversal_in_ref() {
        let manager = test_manager("/tmp/bld_pkg_cache");
        let result = manager.package("https://example.com/org/repo.git@../../../etc");
        assert!(result.is_err());
    }

    #[test]
    fn repo_info_rejects_path_separator_in_ref() {
        let manager = test_manager("/tmp/bld_pkg_cache");
        let result = manager.package("https://example.com/org/repo.git@feature/foo");
        assert!(result.is_err());
    }

    #[test]
    fn repo_info_rejects_empty_repo_name() {
        let manager = test_manager("/tmp/bld_pkg_cache");
        let result = manager.package("https://example.com/org/");
        assert!(result.is_err());
    }

    #[test]
    fn repo_path_without_branch_uses_repo_name() {
        let manager = test_manager("/tmp/bld_pkg_cache");
        let package = manager.package("https://example.com/org/repo.git").unwrap();
        assert_eq!(package.path(), PathBuf::from("/tmp/bld_pkg_cache/repo.git"));
    }

    #[test]
    fn repo_path_with_branch_includes_branch_name() {
        let manager = test_manager("/tmp/bld_pkg_cache");
        let package = manager
            .package("https://example.com/org/repo.git@main")
            .unwrap();
        assert_eq!(
            package.path(),
            PathBuf::from("/tmp/bld_pkg_cache/repo.git@main")
        );
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
    fn ssh_credentials_agent_variant() {
        let ssh_config = SshConfig {
            host: "example.com".to_string(),
            port: SshConfig::default_port(),
            user: "git".to_string(),
            userauth: SshUserAuth::Agent,
        };
        let cred = Package::ssh_credentials(&ssh_config, "git").unwrap();
        assert!(git2::CredentialType::from_bits_truncate(cred.credtype()).is_ssh_key());
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
        let cred = Package::ssh_credentials(&ssh_config, "git").unwrap();
        assert!(git2::CredentialType::from_bits_truncate(cred.credtype()).is_user_pass_plaintext());
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
        let cred = Package::ssh_credentials(&ssh_config, "git").unwrap();
        assert!(git2::CredentialType::from_bits_truncate(cred.credtype()).is_ssh_key());
    }

    #[tokio::test]
    async fn is_git_repository_false_for_missing_directory() {
        let env = TestEnv::new();
        let package = env.manager.package(&env.source("pkg")).unwrap();
        assert!(!env.manager.is_git_repository(&package).await);
    }

    #[tokio::test]
    async fn is_git_repository_false_for_empty_directory() {
        let env = TestEnv::new();
        let package = env.manager.package(&env.source("pkg")).unwrap();
        fs::create_dir_all(package.path()).unwrap();
        assert!(!env.manager.is_git_repository(&package).await);
    }

    #[tokio::test]
    async fn is_git_repository_false_for_partial_directory() {
        let env = TestEnv::new();
        let package = env.manager.package(&env.source("pkg")).unwrap();
        fs::create_dir_all(package.path()).unwrap();
        fs::write(path![package.path(), "leftover"], "junk").unwrap();
        assert!(!env.manager.is_git_repository(&package).await);
    }

    #[tokio::test]
    async fn is_git_repository_true_for_cloned_package() {
        let env = TestEnv::new();
        let source = env.source("pkg");
        env.manager.read(&source).await.unwrap();
        let package = env.manager.package(&source).unwrap();
        assert!(env.manager.is_git_repository(&package).await);
    }

    #[tokio::test]
    async fn lock_file_is_a_sibling_of_the_package_directory() {
        let env = TestEnv::new();
        env.manager.create_cache_dir().await.unwrap();
        let package = env
            .manager
            .package("https://example.com/org/repo.git@main")
            .unwrap();

        let lock = package.lock().await.unwrap();
        drop(lock);

        assert_eq!(package.path().parent(), Some(env.cache_path()));
        assert_eq!(env.cache_entries(), vec!["lock_repo.git@main".to_string()]);
    }

    #[tokio::test]
    async fn lock_file_is_shared_between_url_forms_of_the_same_repository() {
        let env = TestEnv::new();
        env.manager.create_cache_dir().await.unwrap();

        for source in [
            "https://example.com/org/repo.git",
            "git@example.com:org/repo.git",
        ] {
            let package = env.manager.package(source).unwrap();
            let lock = package.lock().await.unwrap();
            drop(lock);
        }

        assert_eq!(env.cache_entries(), vec!["lock_repo.git".to_string()]);
    }

    #[tokio::test]
    async fn create_cache_dir_creates_missing_parents_and_is_idempotent() {
        let root = TempDir::new().unwrap();
        let cache = path![root.path(), "a", "b", "cache"];
        let manager = test_manager(cache.to_str().unwrap());

        manager.create_cache_dir().await.unwrap();
        assert!(cache.is_dir());

        manager.create_cache_dir().await.unwrap();
        assert!(cache.is_dir());
    }

    #[tokio::test]
    async fn read_clones_package_without_a_ref() {
        let env = TestEnv::new();
        let content = env.manager.read(&env.source("pkg")).await.unwrap();
        assert_eq!(content, ACTION);
    }

    #[tokio::test]
    async fn read_checks_out_a_non_default_branch() {
        let env = TestEnv::new();
        let source = format!("{}@feature", env.source("pkg"));
        let content = env.manager.read(&source).await.unwrap();
        assert!(content.ends_with("# feature\n"));

        let package = env.manager.package(&source).unwrap();
        let repository = Repository::open(package.path()).unwrap();
        assert_eq!(repository.head().unwrap().shorthand(), Some("feature"));
    }

    #[tokio::test]
    async fn read_checks_out_the_default_branch() {
        let env = TestEnv::new();
        let source = format!("{}@main", env.source("pkg"));
        let content = env.manager.read(&source).await.unwrap();
        assert_eq!(content, ACTION);

        let package = env.manager.package(&source).unwrap();
        let repository = Repository::open(package.path()).unwrap();
        assert_eq!(repository.head().unwrap().shorthand(), Some("main"));
    }

    #[tokio::test]
    async fn read_checks_out_a_tag_with_a_detached_head() {
        let env = TestEnv::new();
        let source = format!("{}@v1", env.source("pkg"));
        let content = env.manager.read(&source).await.unwrap();
        assert_eq!(content, ACTION);

        let package = env.manager.package(&source).unwrap();
        let repository = Repository::open(package.path()).unwrap();
        assert!(repository.head_detached().unwrap());
    }

    #[tokio::test]
    async fn read_fails_for_an_unknown_ref() {
        let env = TestEnv::new();
        let source = format!("{}@nope", env.source("pkg"));
        let error = env.manager.read(&source).await.unwrap_err().to_string();
        assert!(
            error.contains("nope"),
            "expected the missing ref to be reported, got: {error}"
        );
    }

    #[tokio::test]
    async fn read_reports_the_clone_error_instead_of_a_missing_file() {
        let env = TestEnv::new();
        let source = env
            .origins
            .path()
            .join("does-not-exist.git")
            .to_string_lossy()
            .to_string();

        let error = env.manager.read(&source).await.unwrap_err().to_string();
        assert!(
            !error.contains("No such file or directory (os error 2)"),
            "the underlying git error was swallowed, got: {error}"
        );
    }

    /// Issue 356: two jobs of the same layer using the same package must both succeed.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_reads_of_the_same_package_on_a_cold_cache_succeed() {
        let env = TestEnv::new();
        let source = env.source("pkg");
        let manager = env.manager.clone();

        let mut handles = vec![];
        for _ in 0..2 {
            let (manager, source) = (manager.clone(), source.clone());
            handles.push(tokio::spawn(async move { manager.read(&source).await }));
        }

        for handle in handles {
            assert_eq!(handle.await.unwrap().unwrap(), ACTION);
        }
    }

    /// Issue 356: the package directory must be complete once the concurrent reads are done.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn package_directory_is_complete_after_concurrent_reads() {
        let env = TestEnv::new();
        let source = env.source("pkg");
        let manager = env.manager.clone();

        let mut handles = vec![];
        for _ in 0..6 {
            let (manager, source) = (manager.clone(), source.clone());
            handles.push(tokio::spawn(async move { manager.read(&source).await }));
        }
        for handle in handles {
            handle.await.unwrap().unwrap();
        }

        let package = manager.package(&source).unwrap();
        assert!(path![package.path(), ".git"].exists());
        assert_eq!(
            fs::read_to_string(path![package.path(), PACKAGE_ACTION_FILE_NAME]).unwrap(),
            ACTION
        );
        Repository::open(package.path()).unwrap();
    }

    /// Issue 356: a job using a different package must not be blocked by the first one.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_reads_of_different_packages_succeed() {
        let env = TestEnv::new();
        let first = env.source("first");
        let second = env.source("second");
        let manager = env.manager.clone();

        assert_ne!(
            manager.package(&first).unwrap().path(),
            manager.package(&second).unwrap().path()
        );

        let mut handles = vec![];
        for source in [first, second] {
            let manager = manager.clone();
            handles.push(tokio::spawn(async move { manager.read(&source).await }));
        }

        for handle in handles {
            assert_eq!(handle.await.unwrap().unwrap(), ACTION);
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_reads_of_the_same_package_on_a_warm_cache_succeed() {
        let env = TestEnv::new();
        let source = env.source("pkg");
        let manager = env.manager.clone();
        manager.read(&source).await.unwrap();

        let mut handles = vec![];
        for _ in 0..6 {
            let (manager, source) = (manager.clone(), source.clone());
            handles.push(tokio::spawn(async move { manager.read(&source).await }));
        }

        for handle in handles {
            assert_eq!(handle.await.unwrap().unwrap(), ACTION);
        }
    }

    /// A cache directory left behind in a partial state is re-cloned rather than read from.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn read_repairs_a_partial_cache_directory() {
        let env = TestEnv::new();
        let source = env.source("pkg");
        let package = env.manager.package(&source).unwrap();
        fs::create_dir_all(package.path()).unwrap();
        fs::write(path![package.path(), "leftover"], "junk").unwrap();

        assert_eq!(env.manager.read(&source).await.unwrap(), ACTION);

        assert!(path![package.path(), ".git"].exists());
        assert!(!path![package.path(), "leftover"].exists());
    }

    /// Issue 356: no temporary or partial directory is left in the cache. The lock files are
    /// persisted on purpose, so they are the only entries expected next to the package directory.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_reads_leave_no_stray_entries_in_the_cache() {
        let env = TestEnv::new();
        let source = env.source("pkg");
        let manager = env.manager.clone();

        for _ in 0..3 {
            let mut handles = vec![];
            for _ in 0..6 {
                let (manager, source) = (manager.clone(), source.clone());
                handles.push(tokio::spawn(async move { manager.read(&source).await }));
            }
            for handle in handles {
                handle.await.unwrap().unwrap();
            }
        }

        assert_eq!(
            env.cache_entries(),
            vec!["lock_pkg.git".to_string(), "pkg.git".to_string()]
        );
    }
}
