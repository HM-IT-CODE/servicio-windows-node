mod commands;
mod models;
mod services;

use anyhow::Result;
use clap::{Parser, Subcommand};
use models::InstallArgs;

#[derive(Parser)]
#[command(name = "node-winsvc-core", version, about = "Windows service manager for Node.js")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Install {
        #[arg(long)] name:         String,
        #[arg(long)] display:      String,
        #[arg(long)] description:  String,
        #[arg(long)] script:       String,
        #[arg(long, default_value = "")] node_args:    String,
        #[arg(long, default_value = "")] env:          String,
        #[arg(long, default_value = "")] working_dir:  String,
        #[arg(long, default_value = "")] log_file:     String,
        #[arg(long, default_value = "auto")] start_type: String,
        #[arg(long, action = clap::ArgAction::Set, default_value_t = false)] auto_restart: bool,
    },
    Uninstall { #[arg(long)] name: String },
    Start     { #[arg(long)] name: String },
    Stop      { #[arg(long)] name: String },
    Status    { #[arg(long)] name: String },
    /// Internal: service host launched by Windows. Not for direct use.
    Run       { #[arg(long)] name: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Install { name, display, description, script,
                           node_args, env, working_dir, log_file,
                           start_type, auto_restart } => {
            let args = InstallArgs { name, display, description, script,
                                     node_args, env, working_dir, log_file,
                                     start_type, auto_restart };
            commands::install::run(&args)
        }
        Command::Uninstall { name } => commands::uninstall::run(&name),
        Command::Start     { name } => commands::start::run(&name),
        Command::Stop      { name } => commands::stop::run(&name),
        Command::Status    { name } => commands::status::run(&name),
        Command::Run       { name } => commands::run::run(&name),
    }
}
