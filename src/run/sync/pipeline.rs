use crate::config::definitions::TOOL_DIR;
use crate::helpers::errors::err_variable_in_yaml;
use crate::path;
use crate::types::{BldError, Result, EMPTY_YAML_VEC};
use std::fmt::{self, Display, Formatter};
use std::path::PathBuf;
use yaml_rust::{Yaml, YamlLoader};

pub enum RunsOn {
    Machine,
    Docker(String),
}

impl Display for RunsOn {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Machine => write!(f, "machine"),
            Self::Docker(image) => write!(f, "docker [ {} ]", image),
        }
    }
}

pub struct Variable {
    pub name: String,
    pub default_value: Option<String>,
}

impl Variable {
    pub fn new(name: String, default_value: Option<String>) -> Self {
        Variable {
            name,
            default_value,
        }
    }
}

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

pub struct Artifacts {
    pub method: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub ignore_errors: bool,
    pub after: Option<String>,
}

impl Artifacts {
    pub fn new(
        method: Option<String>,
        from: Option<String>,
        to: Option<String>,
        after: Option<String>,
        ignore_errors: bool,
    ) -> Self {
        Self {
            method,
            from,
            to,
            after,
            ignore_errors,
        }
    }
}

pub struct Pipeline {
    pub name: Option<String>,
    pub runs_on: RunsOn,
    pub dispose: bool,
    pub variables: Vec<Variable>,
    pub artifacts: Vec<Artifacts>,
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

    pub fn parse(src: &str) -> Result<Pipeline> {
        let yaml = YamlLoader::load_from_str(&src)?;
        if yaml.is_empty() {
            return Err(BldError::YamlError("invalid yaml".to_string()));
        }
        let entry = yaml[0].clone();
        let pipeline = Pipeline::load(&entry)?;
        Ok(pipeline)
    }

    pub fn load(yaml: &Yaml) -> Result<Self> {
        Ok(Self {
            name: yaml["name"].as_str().map(|n| n.to_string()),
            runs_on: match yaml["runs-on"].as_str() {
                Some("machine") | None => RunsOn::Machine,
                Some(target) => RunsOn::Docker(target.to_string()),
            },
            dispose: yaml["dispose"].as_bool().or(Some(true)).unwrap(),
            variables: Self::variables(yaml)?,
            artifacts: Self::artifacts(yaml),
            steps: Self::steps(yaml),
        })
    }

    fn variables(yaml: &Yaml) -> Result<Vec<Variable>> {
        let mut variables = Vec::<Variable>::new();
        if let Some(entries) = &yaml["variables"].as_vec() {
            for variable in entries.iter() {
                let name = variable["name"]
                    .as_str()
                    .ok_or_else(err_variable_in_yaml)?
                    .to_string();
                let default_value = variable["default-value"].as_str().map(|d| d.to_string());
                variables.push(Variable::new(name, default_value));
            }
        }
        Ok(variables)
    }

    fn artifacts(yaml: &Yaml) -> Vec<Artifacts> {
        let mut artifacts = Vec::<Artifacts>::new();
        if let Some(entries) = &yaml["artifacts"].as_vec() {
            for artifact in entries.iter() {
                let method = artifact["method"].as_str().map(|m| m.to_string());
                let from = artifact["from"].as_str().map(|p| p.to_string());
                let to = artifact["to"].as_str().map(|p| p.to_string());
                let after = artifact["after"].as_str().map(|a| a.to_string());
                let ignore_errors = artifact["ignore-errors"].as_bool().or(Some(false)).unwrap();
                artifacts.push(Artifacts::new(method, from, to, after, ignore_errors));
            }
        }
        artifacts
    }

    fn steps(yaml: &Yaml) -> Vec<BuildStep> {
        let mut steps = Vec::<BuildStep>::new();
        let working_dir = yaml["working-dir"].as_str().map(|w| w.to_string());
        if let Some(entries) = &yaml["steps"].as_vec() {
            for step in entries.iter() {
                let name = step["name"].as_str().map(|n| n.to_string());
                let working_dir = step["working-dir"]
                    .as_str()
                    .map(|w| w.to_string())
                    .or_else(|| working_dir.clone());
                let call = step["call"].as_str().map(|p| p.to_string());
                let commands: Vec<String> = step["exec"]
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
