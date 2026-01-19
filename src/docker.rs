use anyhow::{bail, Result};
use colored::*;
use std::collections::hash_map::DefaultHasher;
use std::env;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::IsTerminal;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::config::{Config, DockerConfig};

/// Sentinel value indicating normal execution should proceed
pub const PROCEED_NORMALLY: i32 = -1;

/// Represents a toolchain preset with installation commands
pub struct Preset {
    pub name: &'static str,
    pub description: &'static str,
    pub install_commands: &'static [&'static str],
}

/// Available toolchain presets
pub const PRESETS: &[Preset] = &[
    Preset {
        name: "rust",
        description: "Rust toolchain via rustup",
        install_commands: &[
            "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y",
        ],
    },
    Preset {
        name: "node",
        description: "Node.js (already in base image)",
        install_commands: &[], // Pre-installed in claude-code-devcontainer
    },
    Preset {
        name: "bun",
        description: "Bun JavaScript runtime",
        install_commands: &["curl -fsSL https://bun.sh/install | bash"],
    },
    Preset {
        name: "python",
        description: "Python 3 with pip",
        install_commands: &[
            "apt-get update && apt-get install -y python3 python3-pip python3-venv",
        ],
    },
    Preset {
        name: "go",
        description: "Go programming language",
        install_commands: &[
            "curl -fsSL https://go.dev/dl/go1.22.0.linux-amd64.tar.gz | tar -C /usr/local -xzf -",
        ],
    },
];

/// Check if running inside a Docker/devcontainer environment
pub fn is_inside_container() -> bool {
    env::var("DEVCONTAINER")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(false)
        || env::var("FRESHER_IN_DOCKER")
            .map(|v| v.to_lowercase() == "true")
            .unwrap_or(false)
}

/// Enforce Docker isolation requirements - exit if Docker enabled but not in container
pub fn enforce_docker_isolation(use_docker: bool) -> Result<()> {
    if !use_docker || is_inside_container() {
        return Ok(());
    }

    eprintln!(
        "{}",
        "Docker isolation enabled but not in devcontainer."
            .yellow()
            .bold()
    );
    eprintln!();
    eprintln!("{}", "Options:".bold());
    eprintln!("  1. Open this folder in VS Code and use 'Reopen in Container'");
    eprintln!(
        "  2. Run: {}",
        "docker compose -f .fresher/docker/docker-compose.yml run --rm fresher".cyan()
    );
    eprintln!();
    eprintln!(
        "To disable Docker isolation: {}",
        "export FRESHER_USE_DOCKER=false".cyan()
    );

    bail!("Docker isolation required but not in container")
}

/// Run a fresher command inside a Docker container.
///
/// Returns:
/// - `Ok(PROCEED_NORMALLY)` (-1): Caller should proceed with normal execution
///   (either Docker is disabled or we're already inside a container)
/// - `Ok(exit_code)`: Docker handled the command, caller should exit with this code
/// - `Err(_)`: An error occurred
pub fn run_in_container(config: &Config, args: &[String]) -> Result<i32> {
    // Already in container? Just return and let normal execution proceed
    if is_inside_container() {
        return Ok(PROCEED_NORMALLY);
    }

    // Docker not enabled? Proceed normally
    if !config.docker.use_docker {
        return Ok(PROCEED_NORMALLY);
    }

    // Check that docker compose file exists
    let compose_file = Path::new(".fresher/docker/docker-compose.yml");
    if !compose_file.exists() {
        bail!(
            "Docker is enabled but {} not found.\n\
             Run {} to generate Docker files, or disable Docker with {}",
            compose_file.display(),
            "fresher init --force".cyan(),
            "use_docker = false".cyan()
        );
    }

    // Check if Docker is installed and running
    if !is_docker_available() {
        eprintln!(
            "{}",
            "Error: Docker is enabled but not installed or not running.".red()
        );
        eprintln!();
        eprintln!("To install Docker: {}", "https://docs.docker.com/get-docker/".cyan());
        eprintln!(
            "To disable Docker: Set {} in .fresher/config.toml",
            "use_docker = false".cyan()
        );
        bail!("Docker not available");
    }

    // Ensure image is built with configured presets
    ensure_image_built(config)?;

    println!("{}", "[Docker] Starting container...".dimmed());

    // Build the docker compose command
    let mut cmd = Command::new("docker");
    cmd.args(["compose", "-f", ".fresher/docker/docker-compose.yml", "run", "--rm"]);

    // TTY allocation for streaming output
    if std::io::stdout().is_terminal() {
        cmd.arg("-t");
    }

    // The service name
    cmd.arg("fresher");

    // Pass through the fresher command with arguments
    cmd.arg("fresher");
    cmd.args(args);

    // Execute with inherited stdio for streaming
    let status = cmd
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    Ok(status.code().unwrap_or(1))
}

/// Check if Docker is installed and the daemon is running
fn is_docker_available() -> bool {
    Command::new("docker")
        .args(["info"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Ensure the Docker image is built with the configured presets.
/// Uses a hash of presets to determine if rebuild is needed.
/// Also updates docker-compose.yml to use the correct image tag.
pub fn ensure_image_built(config: &Config) -> Result<()> {
    // Always update docker-compose.yml to match current config
    update_docker_compose(config)?;

    // Determine image tag based on presets
    let image_tag = get_image_tag(&config.docker.presets);

    // Check if image exists
    let exists = Command::new("docker")
        .args(["image", "inspect", &image_tag])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if exists {
        println!("{}", "[Docker] Using cached image".dimmed());
        return Ok(());
    }

    // Show appropriate message based on presets
    if config.docker.presets.is_empty() {
        println!("{}", "[Docker] Building base image...".dimmed());
    } else {
        println!(
            "{}",
            format!("[Docker] Building image with presets: {:?}", config.docker.presets).dimmed()
        );
    }

    // Generate Dockerfile
    let dockerfile = generate_dockerfile(&config.docker)?;
    let dockerfile_path = Path::new(".fresher/docker/Dockerfile.generated");
    fs::write(dockerfile_path, &dockerfile)?;

    // Build image
    let status = Command::new("docker")
        .args([
            "build",
            "-t",
            &image_tag,
            "-f",
            ".fresher/docker/Dockerfile.generated",
            ".fresher/docker",
        ])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        bail!("Docker image build failed");
    }

    println!("{}", "[Docker] Image built successfully".green());
    Ok(())
}

/// Generate a hash of the presets for image caching
pub fn hash_presets(presets: &[String]) -> String {
    let mut hasher = DefaultHasher::new();
    presets.hash(&mut hasher);
    format!("{:x}", hasher.finish())[..8].to_string()
}

/// Generate a Dockerfile based on configured presets
pub fn generate_dockerfile(docker_config: &DockerConfig) -> Result<String> {
    let mut dockerfile = format!(
        r#"# Auto-generated by Fresher - do not edit manually
FROM {}

# Install system dependencies
RUN apt-get update && apt-get install -y \
    git \
    jq \
    curl \
    bash \
    && rm -rf /var/lib/apt/lists/*

# Install Claude Code CLI globally
RUN npm install -g @anthropic-ai/claude-code

# Create non-root user home directory
RUN mkdir -p /home/node/.claude && chown -R node:node /home/node

"#,
        BASE_IMAGE
    );

    // Add preset installation commands
    for preset_name in &docker_config.presets {
        if let Some(preset) = PRESETS.iter().find(|p| p.name == preset_name) {
            if !preset.install_commands.is_empty() {
                dockerfile.push_str(&format!("# Preset: {} - {}\n", preset.name, preset.description));
                for cmd in preset.install_commands {
                    dockerfile.push_str(&format!("RUN {}\n", cmd));
                }
                dockerfile.push('\n');
            }
        } else {
            eprintln!(
                "{}",
                format!("Warning: Unknown preset '{}', skipping", preset_name).yellow()
            );
        }
    }

    // Add custom setup script if specified
    if let Some(script_path) = &docker_config.setup_script {
        dockerfile.push_str(&format!(
            r#"# Custom setup script
COPY {} /tmp/custom-setup.sh
RUN chmod +x /tmp/custom-setup.sh && /tmp/custom-setup.sh

"#,
            script_path
        ));
    }

    // Set PATH for Rust and Go if those presets are used
    let mut env_additions = Vec::new();
    if docker_config.presets.iter().any(|p| p == "rust") {
        env_additions.push("$HOME/.cargo/bin");
    }
    if docker_config.presets.iter().any(|p| p == "go") {
        env_additions.push("/usr/local/go/bin");
    }
    if docker_config.presets.iter().any(|p| p == "bun") {
        env_additions.push("$HOME/.bun/bin");
    }
    if !env_additions.is_empty() {
        dockerfile.push_str(&format!(
            "ENV PATH=\"{}:$PATH\"\n\n",
            env_additions.join(":")
        ));
    }

    // Install fresher CLI if rust preset is available
    if docker_config.presets.iter().any(|p| p == "rust") {
        dockerfile.push_str("# Install fresher CLI\n");
        dockerfile.push_str("RUN . $HOME/.cargo/env && cargo install fresher\n\n");
    }

    dockerfile.push_str("USER node\n");

    Ok(dockerfile)
}

/// Base image for Fresher containers (Node.js with common tools)
pub const BASE_IMAGE: &str = "node:20-bookworm";

/// Get the image tag for the current configuration
pub fn get_image_tag(presets: &[String]) -> String {
    // Always use a built image (fresher-base or fresher-dev with presets)
    if presets.is_empty() {
        "fresher-base:latest".to_string()
    } else {
        format!("fresher-dev:{}", hash_presets(presets))
    }
}

/// Generate docker-compose.yml content for the current configuration.
/// Uses the image tag based on configured presets.
pub fn generate_docker_compose(config: &DockerConfig) -> String {
    let image_tag = get_image_tag(&config.presets);

    format!(
        r#"# Fresher Docker Compose Configuration
# Auto-generated - do not edit manually
#
# Usage (automatic):
#   fresher plan  # or fresher build
#
# Usage (manual):
#   docker compose -f .fresher/docker/docker-compose.yml run --rm fresher fresher plan

services:
  fresher:
    image: {image_tag}

    # Interactive mode for streaming output
    stdin_open: true
    tty: true

    # Resource limits
    mem_limit: {memory}
    cpus: {cpus}

    # Volume mounts
    volumes:
      - ${{PWD}}:/workspace
      # Mount Claude credentials (OAuth tokens)
      - ${{HOME}}/.claude:/home/node/.claude:ro

    # Environment
    environment:
      - FRESHER_IN_DOCKER=true
      - DEVCONTAINER=true
      # For API key users, set ANTHROPIC_API_KEY in your environment
      - ANTHROPIC_API_KEY=${{ANTHROPIC_API_KEY:-}}

    # Working directory
    working_dir: /workspace

    # Clear default entrypoint (node image defaults to 'node')
    entrypoint: ""
"#,
        image_tag = image_tag,
        memory = config.memory,
        cpus = config.cpus,
    )
}

/// Update the docker-compose.yml file with current configuration.
/// Called before running containers to ensure image tag is correct.
pub fn update_docker_compose(config: &Config) -> Result<()> {
    let compose_content = generate_docker_compose(&config.docker);
    let compose_path = Path::new(".fresher/docker/docker-compose.yml");

    // Ensure directory exists
    if let Some(parent) = compose_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(compose_path, compose_content)?;
    Ok(())
}
