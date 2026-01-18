use anyhow::{bail, Context, Result};
use colored::*;
use std::path::Path;
use std::process::Command;

const DOCKER_COMPOSE_PATH: &str = ".fresher/docker/docker-compose.yml";
const SERVICE_NAME: &str = "fresher";

/// Check if docker-compose.yml exists
fn check_docker_files() -> Result<()> {
    if !Path::new(DOCKER_COMPOSE_PATH).exists() {
        bail!(
            "Docker compose file not found at {}.\n\
             Run 'fresher init' first to generate Docker configuration.",
            DOCKER_COMPOSE_PATH
        );
    }
    Ok(())
}

/// Check if docker compose is available (v2 or v1)
fn get_docker_compose_command() -> Result<Vec<String>> {
    // Try docker compose (v2) first
    let output = Command::new("docker")
        .args(["compose", "version"])
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            return Ok(vec!["docker".to_string(), "compose".to_string()]);
        }
    }

    // Fall back to docker-compose (v1)
    let output = Command::new("docker-compose")
        .arg("version")
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            return Ok(vec!["docker-compose".to_string()]);
        }
    }

    bail!("Docker Compose not found. Install Docker Desktop or docker-compose.")
}

/// Run docker shell subcommand
pub async fn run_shell() -> Result<()> {
    check_docker_files()?;
    let compose_cmd = get_docker_compose_command()?;

    println!("{}", "Opening shell in Fresher devcontainer...".cyan());
    println!();

    // Build the command: docker compose -f <path> run --rm fresher /bin/bash
    let mut cmd = Command::new(&compose_cmd[0]);

    // Add remaining compose command parts (if any)
    for part in &compose_cmd[1..] {
        cmd.arg(part);
    }

    cmd.args(["-f", DOCKER_COMPOSE_PATH])
        .args(["run", "--rm", SERVICE_NAME, "/bin/bash"]);

    // Run interactively (inherits stdin/stdout/stderr)
    let status = cmd
        .status()
        .context("Failed to run docker compose")?;

    if !status.success() {
        bail!("Docker shell exited with non-zero status");
    }

    Ok(())
}

/// Run docker build subcommand
pub async fn run_build() -> Result<()> {
    check_docker_files()?;
    let compose_cmd = get_docker_compose_command()?;

    println!("{}", "Building Fresher devcontainer image...".cyan());
    println!();

    // Build the command: docker compose -f <path> build
    let mut cmd = Command::new(&compose_cmd[0]);

    // Add remaining compose command parts (if any)
    for part in &compose_cmd[1..] {
        cmd.arg(part);
    }

    cmd.args(["-f", DOCKER_COMPOSE_PATH, "build"]);

    // Run with output visible
    let status = cmd
        .status()
        .context("Failed to run docker compose build")?;

    if !status.success() {
        bail!("Docker build failed");
    }

    println!();
    println!("{}", "Docker image built successfully.".green().bold());
    println!("Run {} to start an interactive shell.", "fresher docker shell".cyan());

    Ok(())
}
