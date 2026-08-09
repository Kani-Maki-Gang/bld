use std::path::PathBuf;

/// A directory under the system's temp dir that holds the files a child runner needs and
/// that is removed once the test is done with it.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("bld_runner_test_{name}"));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("unable to create temp dir");
        Self(path)
    }

    pub fn write(&self, name: &str, content: &str) {
        std::fs::write(self.0.join(name), content).expect("unable to write file");
    }

    pub fn root_dir(&self) -> String {
        self.0.to_string_lossy().into_owned()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
