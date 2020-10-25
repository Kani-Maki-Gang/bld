mod pipeline;

pub use pipeline::*;

use crate::definitions::TOOL_DIR;
use crate::run::{RunPlatform};
use crate::term::print_info;
use std::fs;
use std::future::Future;
use std::io::{self, Error, ErrorKind};
use std::pin::Pin;
use yaml_rust::YamlLoader;

async fn parse(src: &str) -> io::Result<Pipeline> {
    match YamlLoader::load_from_str(&src) {
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

fn info(pipeline: &Pipeline) -> io::Result<()> {
    if let Some(name) = &pipeline.name {
        print_info(&format!("Pipeline: {}", name))?;
    }
    print_info(&format!("Runs on: {}", pipeline.runs_on))?;
    Ok(())
}

async fn steps(pipeline: &Pipeline) -> io::Result<()> {
    for step in pipeline.steps.iter() {
        if let Some(name) = &step.name {
            print_info(&format!("Step: {}", name))?;
        }

        if let Some(call) = &step.call {
            from_file(call.clone()).await.await?;
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

pub fn read(pipeline: &str) -> io::Result<String> {
    let mut path = std::env::current_dir()?;
    path.push(TOOL_DIR);
    path.push(format!("{}.yaml", pipeline));
    fs::read_to_string(path)
}

pub async fn from_src(src: String) -> Pin<Box<dyn Future<Output = io::Result<()>>>> {
    Box::pin(async move {
        let pipeline = parse(&src).await?;
        info(&pipeline)?;
        steps(&pipeline).await
    })
}

pub async fn from_file(pipeline_name: String) -> Pin<Box<dyn Future<Output = io::Result<()>>>> {
    Box::pin(async move {
        let src = read(&pipeline_name)?;
        from_src(src).await.await
    })
}
