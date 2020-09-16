use crate::run::{Container, Machine, RunPlatform};
use std::io;
use yaml_rust::Yaml;

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
            shell_commands,
        }
    }
}

pub struct Pipeline {
    pub name: Option<String>,
    pub runs_on: RunPlatform,
    pub steps: Vec<BuildStep>,
}

impl Pipeline {
    pub async fn load(yaml: &Yaml) -> io::Result<Self> {
        let name = match yaml["name"].as_str() {
            Some(n) => Some(n.to_string()),
            None => None,
        };

        let runs_on = match yaml["runs-on"].as_str() {
            Some("machine") | None => RunPlatform::Local(Machine::new()?),
            Some(target) => RunPlatform::Docker(Container::new(target).await?),
        };

        Ok(Self {
            name,
            runs_on,
            steps: Self::steps(yaml),
        })
    }

    fn steps(yaml: &Yaml) -> Vec<BuildStep> {
        let mut steps = Vec::<BuildStep>::new();
        let working_dir = match &yaml["working_dir"].as_str() {
            Some(wd) => wd.to_string(),
            None => match std::env::current_dir() {
                Ok(wd) => wd.display().to_string(),
                Err(_) => String::new(),
            },
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
                            None => String::new(),
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
