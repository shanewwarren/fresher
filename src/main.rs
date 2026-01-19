use anyhow::Result;
use clap::Parser;
use fresher::cli::{Cli, Commands, DockerCommands};
use fresher::commands;

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
        Commands::Docker { command } => match command {
            DockerCommands::Shell => commands::docker::run_shell().await,
            DockerCommands::Build => commands::docker::run_build().await,
        },
        Commands::MigratePlan { force, dry_run } => {
            commands::migrate::run(force, dry_run).await
        }
    }
}
