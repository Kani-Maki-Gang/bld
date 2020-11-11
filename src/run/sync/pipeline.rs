use crate::definitions::TOOL_DIR;
use crate::helpers::err;
use crate::path;
use crate::persist::Logger;
use crate::run::{Container, Machine, RunPlatform};
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use yaml_rust::{Yaml, YamlLoader};

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
    pub fn read(pipeline: &str) -> io::Result<String> {
        let path = path![
            std::env::current_dir()?,
            TOOL_DIR,
            format!("{}.yaml", pipeline)
        ];
        std::fs::read_to_string(path)
    }

    pub async fn parse(src: &str, logger: Arc<Mutex<dyn Logger>>) -> io::Result<Pipeline> {
        match YamlLoader::load_from_str(&src) {
            Ok(yaml) => {
                if yaml.len() == 0 {
                    return err("invalid yaml".to_string());
                }
                let entry = yaml[0].clone();
                let pipeline = Pipeline::load(&entry, logger).await?;
                Ok(pipeline)
            }
            Err(e) => err(e.to_string()),
        }
    }

    pub async fn load(yaml: &Yaml, logger: Arc<Mutex<dyn Logger>>) -> io::Result<Self> {
        let name = match yaml["name"].as_str() {
            Some(n) => Some(n.to_string()),
            None => None,
        };

        let runs_on = match yaml["runs-on"].as_str() {
            Some("machine") | None => RunPlatform::Local(Machine::new(logger)?),
            Some(target) => RunPlatform::Docker(Container::new(target, logger).await?),
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
