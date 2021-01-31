use crate::config::definitions::TOOL_DIR;
use crate::path;
use crate::persist::Logger;
use crate::run::{Container, Machine, RunPlatform};
use crate::types::{EMPTY_YAML_VEC, BldError, Result};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use yaml_rust::{Yaml, YamlLoader};

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
    pub fn get_path(name: &str) -> Result<PathBuf> {
        Ok(path![
            std::env::current_dir()?,
            TOOL_DIR,
            format!("{}.yaml", name)
        ])
    }

    pub fn read(pipeline: &str) -> Result<String> {
        let path = Pipeline::get_path(pipeline)?;
        Ok(std::fs::read_to_string(path)?)
    }

    pub fn parse(src: &str, logger: Arc<Mutex<dyn Logger>>) -> Result<Pipeline> {
        let yaml = YamlLoader::load_from_str(&src)?;
        if yaml.is_empty() {
            return Err(BldError::YamlError("invalid yaml".to_string()));
        }
        let entry = yaml[0].clone();
        let pipeline = Pipeline::load(&entry, logger)?;
        Ok(pipeline)
    }

    pub fn load(yaml: &Yaml, logger: Arc<Mutex<dyn Logger>>) -> Result<Self> {
        let name = yaml["name"].as_str().map(|n| n.to_string());
        let runs_on = match yaml["runs-on"].as_str() {
            Some("machine") | None => RunPlatform::Local(Machine::new(logger)?),
            Some(target) => RunPlatform::Docker(Box::new(Container::new(target, logger))),
        };
        Ok(Self {
            name,
            runs_on,
            steps: Self::steps(yaml),
        })
    }

    fn steps(yaml: &Yaml) -> Vec<BuildStep> {
        let mut steps = Vec::<BuildStep>::new();
        let working_dir = yaml["working-dir"]
            .as_str()
            .map(|w| w.to_string());
        if let Some(entries) = &yaml["steps"].as_vec() {
            for entry in entries.iter() {
                let name = entry["name"]
                    .as_str()
                    .map(|n| n.to_string());
                let working_dir = entry["working-dir"]
                    .as_str()
                    .map(|w| w.to_string())
                    .or_else(|| working_dir.clone());
                dbg!(&working_dir);
                let call = entry["call"]
                    .as_str()
                    .map(|p| p.to_string());
                let commands: Vec<String> = entry["exec"]
                    .as_vec()
                    .or(Some(&EMPTY_YAML_VEC))
                    .unwrap()
                    .iter()
                    .map(|c| c["sh"].as_str().or(Some("")).unwrap().to_string())
                    .filter(|c| !c.is_empty())
                    .collect();
                steps.push(BuildStep::new(name, working_dir, call, commands));
            }
        }
        steps
    }
}
