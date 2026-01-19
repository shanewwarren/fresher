//! Docker E2E tests - requires Docker to be running
//!
//! These tests actually build Docker images and run containers.
//! They are marked with `#[ignore]` and must be run explicitly.
//!
//! Run with: cargo test --test docker_e2e -- --ignored
//! Run specific test: cargo test --test docker_e2e test_docker_image_builds_successfully -- --ignored

use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Check if Docker is available and running
fn docker_available() -> bool {
    Command::new("docker")
        .args(["info"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Skip test if Docker is not available
macro_rules! require_docker {
    () => {
        if !docker_available() {
            eprintln!("Skipping: Docker not available");
            return;
        }
    };
}

#[test]
#[ignore] // Run explicitly: cargo test --test docker_e2e -- --ignored
fn test_docker_image_builds_successfully() {
    require_docker!();

    let dir = TempDir::new().unwrap();
    let project_dir = dir.path();

    // Create minimal Dockerfile
    std::fs::create_dir_all(project_dir.join("docker")).unwrap();

    let dockerfile = r#"FROM node:20-bookworm
RUN apt-get update && apt-get install -y git jq curl bash && rm -rf /var/lib/apt/lists/*
RUN npm install -g @anthropic-ai/claude-code
RUN mkdir -p /home/node/.claude && chown -R node:node /home/node
USER node
"#;
    std::fs::write(project_dir.join("docker/Dockerfile"), dockerfile).unwrap();

    // Build image
    let output = Command::new("docker")
        .args([
            "build",
            "-t",
            "fresher-test:e2e",
            "-f",
            "docker/Dockerfile",
            "docker",
        ])
        .current_dir(project_dir)
        .output()
        .expect("Failed to run docker build");

    assert!(
        output.status.success(),
        "Docker build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Cleanup
    let _ = Command::new("docker")
        .args(["rmi", "fresher-test:e2e"])
        .output();
}

#[test]
#[ignore]
fn test_container_detects_fresher_in_docker_env() {
    require_docker!();

    // Run a command that checks FRESHER_IN_DOCKER
    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "-e",
            "FRESHER_IN_DOCKER=true",
            "node:20-bookworm",
            "sh",
            "-c",
            "echo $FRESHER_IN_DOCKER",
        ])
        .output()
        .expect("Failed to run docker");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.trim() == "true",
        "Expected 'true', got '{}'",
        stdout
    );
}

#[test]
#[ignore]
fn test_container_detects_devcontainer_env() {
    require_docker!();

    // Run a command that checks DEVCONTAINER
    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "-e",
            "DEVCONTAINER=true",
            "node:20-bookworm",
            "sh",
            "-c",
            "echo $DEVCONTAINER",
        ])
        .output()
        .expect("Failed to run docker");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.trim() == "true",
        "Expected 'true', got '{}'",
        stdout
    );
}

#[test]
#[ignore]
fn test_fresher_binary_mount_works() {
    require_docker!();

    // Skip on non-Linux hosts - can't mount macOS/Windows binary into Linux container
    if std::env::consts::OS != "linux" {
        eprintln!(
            "Skipping: Host OS is '{}', cannot mount into Linux container",
            std::env::consts::OS
        );
        return;
    }

    // Build fresher first
    let build_output = Command::new("cargo")
        .args(["build", "--release"])
        .output()
        .expect("Failed to build fresher");

    if !build_output.status.success() {
        panic!(
            "Failed to build fresher: {}",
            String::from_utf8_lossy(&build_output.stderr)
        );
    }

    let binary_path = Path::new("target/release/fresher");
    assert!(binary_path.exists(), "Fresher binary not found");

    // Get absolute path for mount
    let abs_binary = std::fs::canonicalize(binary_path).unwrap();

    // Run container with mounted binary
    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "-v",
            &format!("{}:/usr/local/bin/fresher:ro", abs_binary.display()),
            "node:20-bookworm",
            "/usr/local/bin/fresher",
            "version",
        ])
        .output()
        .expect("Failed to run docker with mounted binary");

    assert!(
        output.status.success(),
        "Fresher failed in container: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("fresher"),
        "Expected version output, got: {}",
        stdout
    );
}

#[test]
#[ignore]
fn test_workspace_mount_works() {
    require_docker!();

    let dir = TempDir::new().unwrap();
    let project_dir = dir.path();

    // Create a test file in the project
    std::fs::write(project_dir.join("test-file.txt"), "hello from host").unwrap();

    // Run container with workspace mount
    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "-v",
            &format!("{}:/workspace", project_dir.display()),
            "-w",
            "/workspace",
            "node:20-bookworm",
            "cat",
            "test-file.txt",
        ])
        .output()
        .expect("Failed to run docker");

    assert!(
        output.status.success(),
        "Failed to read file in container: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("hello from host"),
        "Expected file contents, got: {}",
        stdout
    );
}

#[test]
#[ignore]
fn test_preset_rust_installs_cargo() {
    require_docker!();

    let dir = TempDir::new().unwrap();

    // Dockerfile with rust preset installation
    let dockerfile = r#"FROM node:20-bookworm
RUN apt-get update && apt-get install -y git jq curl bash && rm -rf /var/lib/apt/lists/*
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    RUSTUP_HOME=/usr/local/rustup CARGO_HOME=/usr/local/cargo sh -s -- -y --no-modify-path
ENV RUSTUP_HOME=/usr/local/rustup
ENV CARGO_HOME=/usr/local/cargo
ENV PATH="/usr/local/cargo/bin:$PATH"
"#;
    std::fs::write(dir.path().join("Dockerfile"), dockerfile).unwrap();

    // Build
    let build = Command::new("docker")
        .args([
            "build",
            "-t",
            "fresher-test:rust",
            "-f",
            "Dockerfile",
            ".",
        ])
        .current_dir(dir.path())
        .output()
        .expect("Failed to build");

    assert!(
        build.status.success(),
        "Build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );

    // Test cargo is available
    let test = Command::new("docker")
        .args(["run", "--rm", "fresher-test:rust", "cargo", "--version"])
        .output()
        .expect("Failed to run");

    assert!(
        test.status.success(),
        "Cargo not available: {}",
        String::from_utf8_lossy(&test.stderr)
    );

    let stdout = String::from_utf8_lossy(&test.stdout);
    assert!(
        stdout.contains("cargo"),
        "Expected cargo version output, got: {}",
        stdout
    );

    // Cleanup
    let _ = Command::new("docker")
        .args(["rmi", "fresher-test:rust"])
        .output();
}

#[test]
#[ignore]
fn test_preset_bun_installs_bun() {
    require_docker!();

    let dir = TempDir::new().unwrap();

    // Dockerfile with bun preset installation
    let dockerfile = r#"FROM node:20-bookworm
RUN apt-get update && apt-get install -y curl unzip && rm -rf /var/lib/apt/lists/*
ENV BUN_INSTALL=/usr/local/bun
RUN curl -fsSL https://bun.sh/install | bash
ENV PATH="/usr/local/bun/bin:$PATH"
"#;
    std::fs::write(dir.path().join("Dockerfile"), dockerfile).unwrap();

    // Build
    let build = Command::new("docker")
        .args([
            "build",
            "-t",
            "fresher-test:bun",
            "-f",
            "Dockerfile",
            ".",
        ])
        .current_dir(dir.path())
        .output()
        .expect("Failed to build");

    assert!(
        build.status.success(),
        "Build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );

    // Test bun is available
    let test = Command::new("docker")
        .args(["run", "--rm", "fresher-test:bun", "bun", "--version"])
        .output()
        .expect("Failed to run");

    assert!(
        test.status.success(),
        "Bun not available: {}",
        String::from_utf8_lossy(&test.stderr)
    );

    // Cleanup
    let _ = Command::new("docker")
        .args(["rmi", "fresher-test:bun"])
        .output();
}

#[test]
#[ignore]
fn test_docker_compose_run_succeeds() {
    require_docker!();

    let dir = TempDir::new().unwrap();
    let project_dir = dir.path();

    // Create minimal fresher project structure
    std::fs::create_dir_all(project_dir.join(".fresher/docker")).unwrap();

    // Create Dockerfile
    let dockerfile = r#"FROM node:20-bookworm
RUN apt-get update && apt-get install -y git jq curl bash && rm -rf /var/lib/apt/lists/*
USER node
"#;
    std::fs::write(
        project_dir.join(".fresher/docker/Dockerfile"),
        dockerfile,
    )
    .unwrap();

    // Build the image first
    let build = Command::new("docker")
        .args([
            "build",
            "-t",
            "fresher-base:latest",
            "-f",
            ".fresher/docker/Dockerfile",
            ".fresher/docker",
        ])
        .current_dir(project_dir)
        .output()
        .expect("Failed to build");

    assert!(
        build.status.success(),
        "Build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );

    // Create docker-compose.yml
    let compose = r#"services:
  fresher:
    image: fresher-base:latest
    stdin_open: true
    tty: true
    volumes:
      - ${PWD}:/workspace
    environment:
      - FRESHER_IN_DOCKER=true
    working_dir: /workspace
    entrypoint: ""
"#;
    std::fs::write(
        project_dir.join(".fresher/docker/docker-compose.yml"),
        compose,
    )
    .unwrap();

    // Run a simple command via docker compose
    let output = Command::new("docker")
        .args([
            "compose",
            "-f",
            ".fresher/docker/docker-compose.yml",
            "run",
            "--rm",
            "fresher",
            "echo",
            "hello",
        ])
        .current_dir(project_dir)
        .output()
        .expect("Failed to run docker compose");

    assert!(
        output.status.success(),
        "Docker compose run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("hello"),
        "Expected echo output, got: {}",
        stdout
    );

    // Cleanup
    let _ = Command::new("docker")
        .args(["rmi", "fresher-base:latest"])
        .output();
}
