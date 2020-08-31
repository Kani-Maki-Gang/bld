use std::fs;
use std::io::{self, Error, ErrorKind};
use std::path::Path;
use std::path::Component::Normal;

static DEFAULT_PIPELINE: &str = r"name: Default Pipeline
runs-on: machine
steps: 
- name: echo 
  exec:
  - sh: echo 'hello world'
";

fn build_dir_exists() -> io::Result<bool> {
    let curr_dir = std::env::current_dir()?;
    for entry in fs::read_dir(&curr_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let component = path.components().last();
            if let Some(Normal(name)) = component {
                if name == ".build" {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

fn create_build_dir() -> io::Result<()> {
    let mut path = std::env::current_dir()?;
    path.push(".build");
    fs::create_dir(path)?;
    println!("[-] .build directory initialized");
    Ok(())
}

fn create_default_yaml() -> io::Result<()> {
    let path = Path::new("./.build/default.yaml");
    fs::write(path, DEFAULT_PIPELINE)?;
    println!("[-] default yaml file created");
    Ok(())
}

pub fn exec() -> io::Result<()> {
    let build_dir_exists = build_dir_exists()?;
    if !build_dir_exists {
        return create_build_dir().and_then(|_| create_default_yaml());
    } 

    let message = ".build dir already exists in the current directory";
    let error = Error::new(ErrorKind::Other, message);
    Err(error)
}