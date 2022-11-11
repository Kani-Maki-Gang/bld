use anyhow::{anyhow, Result, Error};
use std::fmt::{self, Display, Formatter};
use yaml_rust::{Yaml, YamlLoader};

pub fn err_variable_in_yaml() -> Error {
    anyhow!("error in variable section")
}

#[derive(Debug)]
pub enum RunsOn {
    Machine,
    Docker(String),
}

impl Default for RunsOn {
    fn default() -> Self {
        Self::Machine
    }
}

impl Display for RunsOn {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Machine => write!(f, "machine"),
            Self::Docker(image) => write!(f, "docker [ {} ]", image),
        }
    }
}

#[derive(Debug)]
pub struct Variable {
    pub name: String,
    pub default_value: String,
}

impl Variable {
    pub fn new(name: String, default_value: String) -> Self {
        Variable {
            name,
            default_value,
        }
    }
}

#[derive(Debug)]
pub struct Invoke {
    pub name: String,
    pub server: String,
    pub pipeline: String,
    pub variables: Vec<Variable>,
    pub environment: Vec<Variable>,
}

impl Invoke {
    pub fn new(
        name: String,
        server: String,
        pipeline: String,
        variables: Vec<Variable>,
        environment: Vec<Variable>,
    ) -> Self {
        Self {
            name,
            server,
            pipeline,
            variables,
            environment,
        }
    }
}

#[derive(Debug)]
pub struct BuildStep {
    pub name: Option<String>,
    pub working_dir: Option<String>,
    pub invoke: Vec<String>,
    pub call: Vec<String>,
    pub commands: Vec<String>,
}

impl BuildStep {
    pub fn new(
        name: Option<String>,
        working_dir: Option<String>,
        invoke: Vec<String>,
        call: Vec<String>,
        commands: Vec<String>,
    ) -> Self {
        Self {
            name,
            working_dir,
            invoke,
            call,
            commands,
        }
    }
}

#[derive(Debug)]
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
            ignore_errors,
            after,
        }
    }
}

#[derive(Debug, Default)]
pub struct Pipeline {
    pub name: Option<String>,
    pub runs_on: RunsOn,
    pub dispose: bool,
    pub environment: Vec<Variable>,
    pub variables: Vec<Variable>,
    pub artifacts: Vec<Artifacts>,
    pub invoke: Vec<Invoke>,
    pub steps: Vec<BuildStep>,
}

impl Pipeline {
    pub fn parse(src: &str) -> Result<Pipeline> {
        let yaml = YamlLoader::load_from_str(src)?;
        if yaml.is_empty() {
            return Err(anyhow!("invalid yaml"));
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
            dispose: yaml["dispose"].as_bool().unwrap_or(true),
            environment: Self::variables(yaml, "environment")?,
            variables: Self::variables(yaml, "variables")?,
            artifacts: Self::artifacts(yaml),
            invoke: Self::invoke(yaml)?,
            steps: Self::steps(yaml),
        })
    }

    fn variables(yaml: &Yaml, section: &str) -> Result<Vec<Variable>> {
        let mut variables = Vec::<Variable>::new();
        if let Some(entries) = &yaml[section].as_vec() {
            for variable in entries.iter() {
                let hash = variable.as_hash().ok_or_else(err_variable_in_yaml)?;
                let name = hash
                    .keys()
                    .next()
                    .and_then(|k| k.as_str())
                    .map(|k| k.to_string())
                    .ok_or_else(err_variable_in_yaml)?;
                let default_value = hash
                    .values()
                    .next()
                    .and_then(|v| v.as_str())
                    .map(|v| v.to_string())
                    .ok_or_else(err_variable_in_yaml)?;
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
                let ignore_errors = artifact["ignore-errors"].as_bool().unwrap_or(false);
                artifacts.push(Artifacts::new(method, from, to, after, ignore_errors));
            }
        }
        artifacts
    }

    fn invoke(yaml: &Yaml) -> Result<Vec<Invoke>> {
        let mut invoke = vec![];
        if let Some(entries) = yaml["invoke"].as_vec() {
            for entry in entries {
                let name = entry["name"].as_str().unwrap_or("").to_string();
                let server = entry["server"].as_str().unwrap_or("").to_string();
                let pipeline = entry["pipeline"].as_str().unwrap_or("").to_string();
                let variables = Self::variables(entry, "variables")?;
                let environment = Self::variables(entry, "environment")?;
                invoke.push(Invoke::new(name, server, pipeline, variables, environment));
            }
        }
        Ok(invoke)
    }

    fn steps(yaml: &Yaml) -> Vec<BuildStep> {
        let working_dir = yaml["working-dir"].as_str().map(|w| w.to_string());
        yaml["steps"]
            .as_vec()
            .map(|steps| {
                steps
                    .iter()
                    .map(|step| {
                        let name = step["name"].as_str().map(|n| n.to_string());
                        let working_dir = step["working-dir"]
                            .as_str()
                            .map(|w| w.to_string())
                            .or_else(|| working_dir.clone());

                        let invoke = step["invoke"]
                            .as_vec()
                            .unwrap_or(&Vec::<Yaml>::new())
                            .iter()
                            .map(|c| c.as_str().unwrap_or("").to_string())
                            .filter(|c| !c.is_empty())
                            .collect();

                        let call = step["call"]
                            .as_vec()
                            .unwrap_or(&Vec::<Yaml>::new())
                            .iter()
                            .map(|c| c.as_str().unwrap_or("").to_string())
                            .filter(|c| !c.is_empty())
                            .collect();

                        let commands: Vec<String> = step["exec"]
                            .as_vec()
                            .unwrap_or(&Vec::<Yaml>::new())
                            .iter()
                            .map(|c| c.as_str().unwrap_or("").to_string())
                            .filter(|c| !c.is_empty())
                            .collect();

                        BuildStep::new(name, working_dir, invoke, call, commands)
                    })
                    .collect()
            })
            .unwrap_or_else(Vec::new)
    }
}
