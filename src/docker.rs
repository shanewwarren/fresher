use anyhow::{bail, Result};
use colored::*;
use std::env;

/// Check if running inside a Docker/devcontainer environment
pub fn is_inside_container() -> bool {
    env::var("DEVCONTAINER").map(|v| v.to_lowercase() == "true").unwrap_or(false)
        || env::var("FRESHER_IN_DOCKER").map(|v| v.to_lowercase() == "true").unwrap_or(false)
}

/// Enforce Docker isolation requirements - exit if Docker enabled but not in container
pub fn enforce_docker_isolation(use_docker: bool) -> Result<()> {
    if !use_docker || is_inside_container() {
        return Ok(());
    }

    eprintln!("{}", "Docker isolation enabled but not in devcontainer.".yellow().bold());
    eprintln!();
    eprintln!("{}", "Options:".bold());
    eprintln!("  1. Open this folder in VS Code and use 'Reopen in Container'");
    eprintln!("  2. Run: {}", "docker compose -f .fresher/docker/docker-compose.yml run --rm fresher".cyan());
    eprintln!();
    eprintln!("To disable Docker isolation: {}", "export FRESHER_USE_DOCKER=false".cyan());

    bail!("Docker isolation required but not in container")
}
