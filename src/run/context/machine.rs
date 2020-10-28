use crate::os::{self, OSname};
use crate::persist::Dumpster;
use std::io::{self, Error, ErrorKind};
use std::process::Command;
use std::sync::{Arc, Mutex};

pub struct Machine {
    pub dumpster: Arc<Mutex<dyn Dumpster>>
}

impl Machine {
    pub fn new(dumpster: Arc<Mutex<dyn Dumpster>>) -> io::Result<Self> {
        Ok(Self { dumpster })
    }

    pub fn sh(&self, working_dir: &Option<String>, input: &str) -> io::Result<()> {
        let mut dumpster = self.dumpster.lock().unwrap();
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
        dumpster.dump(&output);

        Ok(())
    }
}
