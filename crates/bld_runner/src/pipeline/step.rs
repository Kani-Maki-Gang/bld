#[derive(Debug)]
pub struct BuildStep {
    pub name: Option<String>,
    pub working_dir: Option<String>,
    pub external: Vec<String>,
    pub commands: Vec<String>,
}

impl BuildStep {
    pub fn new(
        name: Option<String>,
        working_dir: Option<String>,
        external: Vec<String>,
        commands: Vec<String>,
    ) -> Self {
        Self {
            name,
            working_dir,
            external,
            commands,
        }
    }
}
