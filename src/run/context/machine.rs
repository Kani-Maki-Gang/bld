use crate::os::{self, OSname};
use crate::persist::Logger;
use std::io::{self, Error, ErrorKind};
use std::process::Command;
use std::sync::{Arc, Mutex};

pub struct Machine {
    pub logger: Arc<Mutex<dyn Logger>>,
}

impl Machine {
    pub fn new(logger: Arc<Mutex<dyn Logger>>) -> io::Result<Self> {
        Ok(Self { logger })
    }

    pub fn sh(&self, working_dir: &Option<String>, input: &str) -> io::Result<()> {
        let mut logger = self.logger.lock().unwrap();
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
        logger.dump(&output);

        Ok(())
    }
}
