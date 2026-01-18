mod cli;
mod commands;
mod config;
mod docker;
mod hooks;
mod state;
mod streaming;
mod templates;
mod upgrade;
mod verify;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { force } => commands::init::run(force).await,
        Commands::Plan { max_iterations } => commands::plan::run(max_iterations).await,
        Commands::Build { max_iterations } => commands::build::run(max_iterations).await,
        Commands::Verify { json, plan_file } => commands::verify::run(json, plan_file).await,
        Commands::Upgrade { check } => commands::upgrade::run(check).await,
        Commands::Version => commands::version::run(),
    }
}
