mod commands;
mod models;
mod services;
// El nucleo solo toma de `setup` la firma del paquete y el constructor, para
// el subcomando `empaquetar`. El resto lo usa el binario del instalador.
#[allow(dead_code)] mod setup;

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
        // allow_hyphen_values: el valor son los flags de node ("--expose-gc
        // --max-old-space-size=512"). Sin esto clap los toma como argumentos
        // propios y falla con "unexpected argument '--expose-gc'".
        #[arg(long, default_value = "", allow_hyphen_values = true)] node_args: String,
        #[arg(long, default_value = "", allow_hyphen_values = true)] env:       String,
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
    /// Checks dependencies (node.exe, script, ProgramData). Exits != 0 on failure.
    Ping      { #[arg(long, default_value = "")] script: String },
    /// Internal: builds the final installer by appending the app to the stub.
    Empaquetar {
        #[arg(long)] stub:     String,
        #[arg(long)] origen:   String,
        #[arg(long)] incluir:  String,
        #[arg(long)] core:     String,
        #[arg(long, default_value = "")] node: String,
        #[arg(long)] manifest: String,
        #[arg(long)] salida:   String,
        #[arg(long, default_value_t = 19)] nivel: i64,
    },
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
        Command::Run       { name }   => commands::run::run(&name),
        Command::Ping      { script } => commands::ping::run(&script),
        Command::Empaquetar { stub, origen, incluir, core, node, manifest, salida, nivel } => {
            let args = commands::empaquetar::Args {
                stub, origen, incluir, core, node, manifest, salida, nivel,
            };
            commands::empaquetar::run(&args)
        }
    }
}
