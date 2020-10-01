use crate::os::{self, OSname};
use std::io::{self, Error, ErrorKind};
use std::process::Command;

#[derive(Clone, Debug)]
pub struct Machine;

impl Machine {
    pub fn new() -> io::Result<Self> {
        Ok(Self)
    }

    pub fn sh(&self, working_dir: &Option<String>, input: &str) -> io::Result<()> {
        let os_name = os::name();

        let (shell, mut args) = match os_name {
            OSname::Windows => ("powershell.exe", Vec::<&str>::new()),
            OSname::Linux => ("bash", vec!["-c"]),
            OSname::Mac => ("sh", vec!["-c"]),
            OSname::Unknown => return Err(Error::new(ErrorKind::Other, "Could not spawn shell")),
        };
        args.push(input);

        let mut command = Command::new(shell);
        command.args(&args);

        if let Some(dir) = working_dir {
            command.current_dir(dir);
        }

        let process = command.output()?;

        let mut output = String::from_utf8_lossy(&process.stderr).to_string();
        output.push_str(&format!("\r\n{}", String::from_utf8_lossy(&process.stdout)));
        println!("{}", &output);

        Ok(())
    }
}
