#[cfg(feature = "all")]
use bld_core::fs::FileSystem;

#[cfg(feature = "all")]
use bld_pkg::PackageManager;

pub struct RemoteDependency<'a> {
    pub server: Option<&'a str>,
    pub name: &'a str,
}

impl<'a> RemoteDependency<'a> {
    pub fn new(server: &'a str, name: &'a str) -> Self {
        Self { server, name }
    }
}

pub enum Dependency<'a> {
    LocalFile(&'a str),
    LocalPackage(&'a str),
    Remote(Box<RemoteDependency<'a>>),
    Job(&'a str),
}

#[allow(async_fn_in_trait)]
#[cfg(feature = "all")]
pub trait Dependencies<'a> {
    async fn local_deps(&'a self, manager: &PackageManager, fs: &FileSystem) -> Vec<Dependency<'a>>;
    async fn remote_deps(&'a self) -> Vec<Dependency<'a>>;
    async fn jobs(&'a self) -> Vec<Dependency<'a>>;
    async fn all(&'a self, manager: &PackageManager) -> Vec<Dependency<'a>>;
}
