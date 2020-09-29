use crate::definitions;
use crate::term::print_info;
use std::fs;
use std::io::{self, Error, ErrorKind};
use std::path::Component::Normal;
use std::path::Path;

fn build_dir_exists() -> io::Result<bool> {
    let curr_dir = std::env::current_dir()?;
    for entry in fs::read_dir(&curr_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let component = path.components().last();
            if let Some(Normal(name)) = component {
                if name == definitions::TOOL_DIR {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

fn create_build_dir() -> io::Result<()> {
    let mut path = std::env::current_dir()?;
    path.push(definitions::TOOL_DIR);
    fs::create_dir(path)?;

    let message = format!("{} directory created", definitions::TOOL_DIR);
    print_info(&message)?;

    Ok(())
}

fn create_default_yaml() -> io::Result<()> {
    let mut path = Path::new("./").to_path_buf();
    path.push(definitions::TOOL_DIR);
    path.push(format!("{}.yaml", definitions::TOOL_DEFAULT_PIPELINE));
    fs::write(path, definitions::DEFAULT_PIPELINE_CONTENT)?;

    let message = format!("{} yaml file created", definitions::TOOL_DEFAULT_PIPELINE);
    print_info(&message)?;

    Ok(())
}

fn create_config_yaml() -> io::Result<()> {
    let mut path = Path::new("./").to_path_buf();
    path.push(definitions::TOOL_DIR);
    path.push(format!("{}.yaml", definitions::TOOL_DEFAULT_CONFIG));

    let content = definitions::default_config_content();

    fs::write(path, &content)?; 

    print_info("config file created")?;

    Ok(())
}

pub fn exec() -> io::Result<()> {
    let build_dir_exists = build_dir_exists()?;
    if !build_dir_exists {
        return create_build_dir()
            .and_then(|_| create_default_yaml())
            .and_then(|_| create_config_yaml());
    }

    let message = format!("{} dir already exists in the current directory", definitions::TOOL_DIR);
    let error = Error::new(ErrorKind::Other, message);
    Err(error)
}
