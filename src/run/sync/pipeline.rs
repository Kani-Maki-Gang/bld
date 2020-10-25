use crate::run::{Container, Machine, RunPlatform};
use std::io;
use yaml_rust::Yaml;

#[derive(Debug)]
pub struct BuildStep {
    pub name: Option<String>,
    pub working_dir: Option<String>,
    pub call: Option<String>,
    pub commands: Vec<String>,
}

impl BuildStep {
    pub fn new(
        name: Option<String>,
        working_dir: Option<String>,
        call: Option<String>,
        commands: Vec<String>,
    ) -> Self {
        Self {
            name,
            working_dir,
            call,
            commands,
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
        let working_dir = match &yaml["working-dir"].as_str() {
            Some(wd) => Some(wd.to_string()),
            None => None,
        };

        if let Some(entries) = &yaml["steps"].as_vec() {
            for entry in entries.iter() {
                let name = match entry["name"].as_str() {
                    Some(name) => Some(name.to_string()),
                    None => None,
                };

                let working_dir = match &entry["working-dir"].as_str() {
                    Some(wd) => Some(wd.to_string()),
                    None => working_dir.clone(),
                };

                let call = match entry["call"].as_str() {
                    Some(pipeline) => Some(pipeline.to_string()),
                    None => None,
                };

                let commands = match entry["exec"].as_vec() {
                    Some(commands) => commands
                        .iter()
                        .map(|c| match c["sh"].as_str() {
                            Some(command) => command.to_string(),
                            None => String::new(),
                        })
                        .filter(|c| !c.is_empty())
                        .collect::<Vec<String>>(),
                    None => Vec::<String>::new(),
                };

                steps.push(BuildStep::new(name, working_dir, call, commands));
            }
        }

        steps
    }
}
