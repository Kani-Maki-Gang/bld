use anyhow::{bail, Result};
use bld_config::{os_name, OSname};
use tokio::process::Command;

pub fn get_shell(args: &mut Vec<&str>) -> Result<Command> {
    let (shell, mut shell_args) = match os_name() {
        OSname::Windows => ("powershell.exe", vec![]),
        OSname::Linux => ("bash", vec!["-c"]),
        OSname::Mac => ("sh", vec!["-c"]),
        OSname::Unknown => bail!("could not spawn shell"),
    };
    shell_args.append(args);

    let mut shell = Command::new(shell);
    shell.args(shell_args);

    Ok(shell)
}
