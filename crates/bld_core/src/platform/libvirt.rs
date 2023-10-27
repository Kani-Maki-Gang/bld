use anyhow::Result;
use std::sync::Arc;
use tracing::debug;
use virt::{connect::Connect, domain::Domain, stream::Stream};

use crate::logger::LoggerSender;

pub struct LibvirtConnectOptions<'a> {
    pub uri: &'a str,
    pub domain: &'a str,
    pub start_before_run: bool,
    pub shutdown_after_run: bool,
}

impl<'a> LibvirtConnectOptions<'a> {
    pub fn new(
        uri: &'a str,
        domain: &'a str,
        start_before_run: bool,
        shutdown_after_run: bool,
    ) -> Self {
        Self {
            uri,
            domain,
            start_before_run,
            shutdown_after_run,
        }
    }
}

pub struct Libvirt {
    conn: Connect,
    domain: Domain,
}

impl Libvirt {
    pub fn new<'a>(connect: LibvirtConnectOptions<'a>) -> Result<Self> {
        let conn = Connect::open(connect.uri)?;
        let domain = Domain::lookup_by_name(&conn, connect.domain)?;
        Ok(Self { conn, domain })
    }

    pub async fn sh(
        &self,
        logger: Arc<LoggerSender>,
        working_dir: &Option<String>,
        command: &str,
    ) -> Result<()> {
        debug!("reached shell method of libvirt platform");

        let mut bash = String::from("bash -c");

        if let Some(wd) = working_dir {
            let cd = format!(" cd {wd} &&");
            bash.push_str(&cd);
        }

        let command = format!(" {command}");
        bash.push_str(&command);

        debug!("opening new channel with name channel-pty");
        let console_stream = Stream::new(&self.conn, 0)?;
        self.domain.open_console("serial0", &console_stream, 0)?;

        debug!("sending command over the channel stream");
        let command_bytes = command.as_bytes();
        console_stream.send(command_bytes)?;

        debug!("receving command over the channel stream");
        let mut output = vec![];
        console_stream.recv(&mut output)?;

        let str_output = String::from_utf8(output)?;
        logger.write_line(str_output).await?;

        debug!("closing channel stream");
        console_stream.finish()?;

        Ok(())
    }

    pub fn dispose(&self) -> Result<()> {
        Ok(())
    }
}
