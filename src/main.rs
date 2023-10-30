use anyhow::Result;
use clap::{Parser, Subcommand};
use docker_starter_rust::{exec::ExecArgs, ls::LsArgs, rm::RmArgs, rmi::RmiArgs, run::RunArgs};

#[derive(Debug, Parser)]
pub struct Cli {
    #[clap(subcommand)]
    sub_command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Run(RunArgs),
    Exec(ExecArgs),
    Rm(RmArgs),
    Ls(LsArgs),
    Rmi(RmiArgs),
}

fn main() -> Result<()> {
    let args = Cli::parse();
    match args.sub_command {
        Command::Run(run) => run.run(),
        Command::Exec(exec) => exec.run(),
        Command::Rm(rm) => rm.run(),
        Command::Ls(ls) => ls.run(),
        Command::Rmi(rmi) => rmi.run(),
    }
}
