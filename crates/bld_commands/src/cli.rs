use crate::auth::AuthCommand;
use crate::cat::CatCommand;
use crate::check::CheckCommand;
use crate::command::BldCommand;
use crate::config::ConfigCommand;
use crate::copy::CopyCommand;
use crate::edit::EditCommand;
use crate::hist::HistCommand;
use crate::init::InitCommand;
use crate::list::ListCommand;
use crate::monit::MonitCommand;
use crate::r#move::MoveCommand;
use crate::pull::PullCommand;
use crate::push::PushCommand;
use crate::remove::RemoveCommand;
use crate::run::RunCommand;
use crate::server::ServerCommand;
use crate::stop::StopCommand;
use crate::supervisor::SupervisorCommand;
use crate::worker::WorkerCommand;
use crate::{add::AddCommand, cron::command::CronCommand};
use anyhow::Result;
use bld_config::definitions::VERSION;
use clap::{Parser, Subcommand};

#[derive(Subcommand)]
enum Commands {
    Login(AuthCommand),
    Cat(CatCommand),
    Check(CheckCommand),
    Config(ConfigCommand),
    Cp(CopyCommand),
    Cron(CronCommand),
    Edit(EditCommand),
    Hist(HistCommand),
    Init(InitCommand),
    Add(AddCommand),
    Ls(ListCommand),
    Monit(MonitCommand),
    Mv(MoveCommand),
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
    #[command(subcommand)]
    command: Commands,
}

impl Cli {
    pub fn invoke(self) -> Result<()> {
        match self.command {
            Commands::Login(auth) => auth.invoke(),
            Commands::Cat(cat) => cat.invoke(),
            Commands::Check(check) => check.invoke(),
            Commands::Config(config) => config.invoke(),
            Commands::Cp(copy) => copy.invoke(),
            Commands::Cron(cron) => cron.invoke(),
            Commands::Edit(edit) => edit.invoke(),
            Commands::Hist(hist) => hist.invoke(),
            Commands::Init(init) => init.invoke(),
            Commands::Add(add) => add.invoke(),
            Commands::Ls(list) => list.invoke(),
            Commands::Monit(monit) => monit.invoke(),
            Commands::Mv(r#move) => r#move.invoke(),
            Commands::Pull(pull) => pull.invoke(),
            Commands::Push(push) => push.invoke(),
            Commands::Rm(remove) => remove.invoke(),
            Commands::Run(run) => run.invoke(),
            Commands::Server(server) => server.invoke(),
            Commands::Stop(stop) => stop.invoke(),
            Commands::Supervisor(supervisor) => supervisor.invoke(),
            Commands::Worker(worker) => worker.invoke(),
        }
    }
}
