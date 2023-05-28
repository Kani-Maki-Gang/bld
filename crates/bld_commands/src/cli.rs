use crate::add::AddCommand;
use crate::auth::AuthCommand;
use crate::check::CheckCommand;
use crate::command::BldCommand;
use crate::config::ConfigCommand;
use crate::edit::EditCommand;
use crate::hist::HistCommand;
use crate::init::InitCommand;
use crate::inspect::InspectCommand;
use crate::list::ListCommand;
use crate::monit::MonitCommand;
use crate::pull::PullCommand;
use crate::push::PushCommand;
use crate::remove::RemoveCommand;
use crate::run::RunCommand;
use crate::server::ServerCommand;
use crate::stop::StopCommand;
use crate::supervisor::SupervisorCommand;
use crate::worker::WorkerCommand;
use anyhow::Result;
use bld_config::definitions::VERSION;
use clap::{Parser, Subcommand};
use tracing_subscriber::filter::LevelFilter;

#[derive(Subcommand)]
enum Commands {
    Login(AuthCommand),
    Check(CheckCommand),
    Config(ConfigCommand),
    Edit(EditCommand),
    Hist(HistCommand),
    Init(InitCommand),
    Add(AddCommand),
    Inspect(InspectCommand),
    Ls(ListCommand),
    Monit(MonitCommand),
    Pull(PullCommand),
    Push(PushCommand),
    Rm(RemoveCommand),
    Run(RunCommand),
    Server(ServerCommand),
    Stop(StopCommand),
    Supervisor(SupervisorCommand),
    Worker(WorkerCommand),
}

#[derive(Parser)]
#[command(author = "Kostas Vlachos", name = "Bld", version = VERSION, about = "A simple CI/CD")]
#[command(propagate_version = true)]
pub struct Cli {
    #[arg(short = 'v', long = "verbose", help = "Sets the level of verbosity")]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

impl Cli {
    fn tracing_level(&self) -> LevelFilter {
        if self.verbose {
            LevelFilter::DEBUG
        } else {
            LevelFilter::INFO
        }
    }

    fn tracing(&self) {
        tracing_subscriber::fmt()
            .with_max_level(self.tracing_level())
            .init()
    }
}

impl BldCommand for Cli {
    fn exec(self) -> Result<()> {
        self.tracing();

        match self.command {
            Commands::Login(auth) => auth.exec(),
            Commands::Check(check) => check.exec(),
            Commands::Config(config) => config.exec(),
            Commands::Edit(edit) => edit.exec(),
            Commands::Hist(hist) => hist.exec(),
            Commands::Init(init) => init.exec(),
            Commands::Add(add) => add.exec(),
            Commands::Inspect(inspect) => inspect.exec(),
            Commands::Ls(list) => list.exec(),
            Commands::Monit(monit) => monit.exec(),
            Commands::Pull(pull) => pull.exec(),
            Commands::Push(push) => push.exec(),
            Commands::Rm(remove) => remove.exec(),
            Commands::Run(run) => run.exec(),
            Commands::Server(server) => server.exec(),
            Commands::Stop(stop) => stop.exec(),
            Commands::Supervisor(supervisor) => supervisor.exec(),
            Commands::Worker(worker) => worker.exec(),
        }
    }
}
