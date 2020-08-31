use clap::ArgMatches;
use crate::os::{self, OSname};
use crate::run::Pipeline;
use yaml_rust::{YamlLoader, Yaml};
use std::io::{self, Error, ErrorKind};
use std::fs;

fn sh(working_dir: &str, input: &str) -> io::Result<(String, String)> {
    let os_name = os::name();

    let shell = match os_name {
        OSname::Windows => "powershell.exe",
        OSname::Linux => "sh",
        OSname::Mac => "sh",
        OSname::Unknown => return Err(Error::new(ErrorKind::Other, "Could not spawn shell")),
    };

    let cmd = std::process::Command::new(shell)
        .arg(input)
        .current_dir(working_dir)
        .output();

    let output = match cmd {
        Ok(cmd) => cmd,
        Err(e) => return Err(Error::new(ErrorKind::Other, e.to_string())),
    };

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    Ok((stdout, stderr))
}

fn get_pipeline(matches: &ArgMatches) -> String {
    match matches.value_of("pipeline") {
        Some(name) => name.to_string(),
        None => "default".to_string(), 
    }
}

fn load_pipeline(pipeline: &str) -> io::Result<String> {
    let mut path = std::env::current_dir()?;
    path.push(".build");
    path.push(format!("{}.yaml", pipeline));

    let content = fs::read_to_string(path)?;
    Ok(content)
}

fn load_yaml(content: String) -> io::Result<Yaml> {
    match YamlLoader::load_from_str(&content) {
        Ok(yaml) => {
            if yaml.len() == 0 {
                return Err(Error::new(ErrorKind::Other, "invalid yaml".to_string()));
            }
            let entry = yaml[0].clone();
            Ok(entry)
        },
        Err(e) => Err(Error::new(ErrorKind::Other, e.to_string())),
    }
}

fn parse_pipeline(yaml: Yaml) -> io::Result<Pipeline> {
    Ok(Pipeline::load(&yaml))
}

fn print_info(pipeline: Pipeline) -> io::Result<Pipeline> {
    if let Some(name) = &pipeline.name {
        println!("<bld> Pipeline: {}", name);
    }

    println!("<bld> Runs on: {}", pipeline.runs_on);

    Ok(pipeline)
}

fn execute_steps(pipeline: Pipeline) -> io::Result<()> {
    for step in pipeline.steps.iter() {
        if let Some(name) = &step.name {
            println!("<bld> Step: {}", name);
        }

        for command in step.shell_commands.iter() {
            let (stdout, stderr) = sh(&step.working_dir, &command)?;
            println!("{}", stdout);
            println!("{}", stderr);
        }
    }

    Ok(())
}

pub fn exec(matches: &ArgMatches) -> io::Result<()> {
    let pipeline = get_pipeline(matches);

    load_pipeline(&pipeline)
        .and_then(load_yaml)
        .and_then(parse_pipeline)
        .and_then(print_info)
        .and_then(execute_steps)
}