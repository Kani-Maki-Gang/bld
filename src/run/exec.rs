use crate::definitions::{TOOL_DEFAULT_PIPELINE, TOOL_DIR};
use crate::run::{Pipeline, RunPlatform};
use clap::ArgMatches;
use std::fs;
use std::future::Future;
use std::io::{self, Error, ErrorKind};
use std::pin::Pin;
use yaml_rust::YamlLoader;

async fn load(pipeline: &str) -> io::Result<Pipeline> {
    let mut path = std::env::current_dir()?;
    path.push(TOOL_DIR);
    path.push(format!("{}.yaml", pipeline));

    let content = fs::read_to_string(path)?;

    match YamlLoader::load_from_str(&content) {
        Ok(yaml) => {
            if yaml.len() == 0 {
                return Err(Error::new(ErrorKind::Other, "invalid yaml".to_string()));
            }
            let entry = yaml[0].clone();
            let pipeline = Pipeline::load(&entry).await?;
            Ok(pipeline)
        }
        Err(e) => Err(Error::new(ErrorKind::Other, e.to_string())),
    }
}

fn info(pipeline: Pipeline) -> io::Result<Pipeline> {
    if let Some(name) = &pipeline.name {
        println!("<bld> Pipeline: {}", name);
    }

    println!("<bld> Runs on: {}", pipeline.runs_on);

    Ok(pipeline)
}

async fn steps(pipeline: Pipeline) -> io::Result<()> {
    for step in pipeline.steps.iter() {
        if let Some(name) = &step.name {
            println!("<bld> Step: {}", name);
        }

        if let Some(call) = &step.call {
            invoke(call.clone()).await.await?;
        }

        for command in step.commands.iter() {
            match &pipeline.runs_on {
                RunPlatform::Docker(container) => {
                    let mut container = container.clone();
                    let result = container.sh(&step.working_dir, &command).await;
                    if let Err(e) = result {
                        container.dispose().await?;
                        return Err(e);
                    }
                }
                RunPlatform::Local(machine) => machine.sh(&step.working_dir, &command)?,
            }
        }
    }

    if let RunPlatform::Docker(container) = &pipeline.runs_on {
        container.dispose().await?;
    }

    Ok(())
}

async fn invoke(pipeline: String) -> Pin<Box<dyn Future<Output = io::Result<()>>>> {
    Box::pin(async move {
        let pipeline = load(&pipeline).await.and_then(info)?;
        steps(pipeline).await
    })
}

pub async fn exec(matches: &ArgMatches<'_>) -> io::Result<()> {
    let pipeline = match matches.value_of("pipeline") {
        Some(name) => name.to_string(),
        None => TOOL_DEFAULT_PIPELINE.to_string(),
    };
    invoke(pipeline).await.await
}
