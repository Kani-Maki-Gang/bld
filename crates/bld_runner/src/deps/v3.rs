#[cfg(feature = "all")]
use {bld_core::fs::FileSystem, bld_pkg::PackageManager};

#[cfg(feature = "all")]
pub struct RemoteDependency<'a> {
    pub server: Option<&'a str>,
    pub name: &'a str,
}

#[cfg(feature = "all")]
impl<'a> RemoteDependency<'a> {
    pub fn new(server: &'a str, name: &'a str) -> Self {
        Self {
            server: Some(server),
            name,
        }
    }
}

#[cfg(feature = "all")]
pub enum Dependency<'a> {
    LocalFile(&'a str),
    LocalPackage(&'a str),
    Remote(Box<RemoteDependency<'a>>),
    Job(&'a str),
}

impl<'a> Dependency<'a> {
    pub fn is_local(&self) -> bool {
        matches!(self, Self::LocalFile(_) | Self::LocalPackage(_))
    }

    pub fn get_local(&self) -> Option<&'a str> {
        match self {
            Self::LocalFile(file) => Some(file),
            Self::LocalPackage(package) => Some(package),
            _ => None,
        }
    }

    pub fn is_remote(&self) -> bool {
        matches!(self, Self::Remote(_))
    }

    pub fn is_job(&self) -> bool {
        matches!(self, Self::Job(_))
    }

    pub fn get_job(&self) -> Option<&'a str> {
        match self {
            Self::Job(job) => Some(job),
            _ => None,
        }
    }
}

#[allow(async_fn_in_trait)]
#[cfg(feature = "all")]
pub trait Dependencies<'a> {
    async fn local_deps(&'a self, manager: &PackageManager, fs: &FileSystem)
    -> Vec<Dependency<'a>>;
    async fn remote_deps(&'a self) -> Vec<Dependency<'a>>;
    async fn jobs(&'a self) -> Vec<Dependency<'a>>;
    async fn all(&'a self, manager: &PackageManager, fs: &FileSystem) -> Vec<Dependency<'a>>;
}
