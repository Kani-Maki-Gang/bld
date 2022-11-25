use crate::pipeline::runs_on::RunsOn;
use crate::pipeline::variable::Variable;
use crate::pipeline::artifacts::Artifacts;
use crate::pipeline::external::{External, ExternalDetails};
use crate::pipeline::step::BuildStep;
use anyhow::{anyhow, Result, Error};
use bld_core::proxies::PipelineFileSystemProxy;
use std::collections::HashMap;
use tracing::debug;
use yaml_rust::yaml::{Yaml, YamlLoader};

pub fn err_variable_in_yaml() -> Error {
    anyhow!("error in variable section")
}

#[derive(Debug, Default)]
pub struct Pipeline {
    pub name: Option<String>,
    pub runs_on: RunsOn,
    pub dispose: bool,
    pub environment: Vec<Variable>,
    pub variables: Vec<Variable>,
    pub artifacts: Vec<Artifacts>,
    pub external: Vec<External>,
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
            external: Self::external(yaml)?,
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

    fn external(yaml: &Yaml) -> Result<Vec<External>> {
        let mut externals = vec![];
        if let Some(entries) = yaml["external"].as_vec() {
            for entry in entries {
                let name = entry["name"].as_str().unwrap_or("").to_string();
                let pipeline = entry["pipeline"].as_str().unwrap_or("").to_string();
                let variables = Self::variables(entry, "variables")?;
                let environment = Self::variables(entry, "environment")?;

                let external = match entry["server"].as_str() {
                    Some(server) => External::Server {
                        server: server.to_string(),
                        details: ExternalDetails::new(name, pipeline, variables, environment),
                    },
                    None => External::Local(ExternalDetails::new(
                        name,
                        pipeline,
                        variables,
                        environment,
                    )),
                };

                externals.push(external);
            }
        }
        Ok(externals)
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

                        let external = step["external"]
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

                        BuildStep::new(name, working_dir, external, commands)
                    })
                    .collect()
            })
            .unwrap_or_else(Vec::new)
    }

    pub fn dependencies(
        proxy: &PipelineFileSystemProxy,
        name: &str,
    ) -> Result<HashMap<String, String>> {
        Self::dependencies_recursive(proxy, name).map(|mut hs| {
            hs.remove(name);
            hs.into_iter().collect()
        })
    }

    fn dependencies_recursive(
        proxy: &PipelineFileSystemProxy,
        name: &str,
    ) -> Result<HashMap<String, String>> {
        debug!("Parsing pipeline {name}");

        let src = proxy
            .read(name)
            .map_err(|_| anyhow!("Pipeline {name} not found"))?;

        let pipeline = Pipeline::parse(&src)?;
        let mut set = HashMap::new();
        set.insert(name.to_string(), src);

        for external in pipeline.external.iter() {
            match external {
                External::Local(details) => {
                    let subset = Self::dependencies_recursive(proxy, &details.pipeline)?;
                    for (k, v) in subset {
                        set.insert(k, v);
                    }
                }
                External::Server { .. } => {}
            }
        }

        Ok(set)
    }
}
