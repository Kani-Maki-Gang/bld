use yaml_rust::Yaml;
use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub enum RunPlatform {
    Machine,
    Docker(String),
}

impl Display for RunPlatform {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Machine => write!(f, "machine"),
            Self::Docker(s) => write!(f, "docker [ {} ]", s),
        }
    }
}

#[derive(Debug)]
pub struct BuildStep {
    pub name: Option<String>,
    pub working_dir: String,
    pub shell_commands: Vec<String>,
}

impl BuildStep {
    pub fn new(name: Option<String>, working_dir: String, shell_commands: Vec<String>) -> Self {
        Self {
            name,
            working_dir,
            shell_commands
        }
    }
}

#[derive(Debug)]
pub struct Pipeline {
    pub name: Option<String>,
    pub runs_on: RunPlatform, 
    pub steps: Vec<BuildStep>,
}

impl Pipeline {
    pub fn load(yaml: &Yaml) -> Self {
        let name = match yaml["name"].as_str() {
            Some(n) => Some(n.to_string()),
            None => None,
        };

        let runs_on = match yaml["runs_on"].as_str() {
            Some("machine") => RunPlatform::Machine,
            Some(target) => RunPlatform::Docker(target.to_string()),
            None => RunPlatform::Machine,
        };

        Self {
            name,
            runs_on,
            steps: Self::steps(yaml),
        }
    }

    fn steps(yaml: &Yaml) -> Vec<BuildStep> {
        let mut steps = Vec::<BuildStep>::new(); 
        let working_dir = match &yaml["working_dir"].as_str() {
            Some(wd) => wd.to_string(),
            None => match std::env::current_dir() {
                Ok(wd) => wd.display().to_string(),
                Err(_) => String::new()
            }
        };

        if let Some(entries) = &yaml["steps"].as_vec() {
            for entry in entries.iter() {
                let name = match entry["name"].as_str() {
                    Some(name) => Some(name.to_string()),
                    None => None, 
                };

                let working_dir = match &entry["working_dir"].as_str() {
                    Some(wd) => wd.to_string(),
                    None => String::from(&working_dir),
                };

                if let Some(commands) = entry["exec"].as_vec() {
                    let commands = commands
                        .iter()
                        .map(|c| match c["sh"].as_str() {
                            Some(command) => command.to_string(),
                            None => String::new()
                        })
                        .filter(|c| !c.is_empty())
                        .collect::<Vec<String>>();

                    steps.push(BuildStep::new(name, working_dir, commands));
                }
            }
        }

        steps
    }
}