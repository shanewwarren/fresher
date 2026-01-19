# Docker UX Specification

**Status:** Implemented
**Version:** 1.0
**Last Updated:** 2026-01-18
**Implementation:** `src/docker.rs` (run_in_container, presets, image caching, streaming output)

---

## 1. Overview

### Purpose

This spec defines a seamless Docker experience where `fresher plan` and `fresher build` automatically launch Docker containers when enabled, with real-time streaming output and declarative dependency management. Users should never need to manually run `docker compose` commands.

### Goals

- **Transparent execution** - `fresher plan/build` auto-launches Docker when `use_docker=true`
- **Streaming output** - Real-time Claude output, no buffering delays
- **Dependency presets** - Declare toolchains in config.toml without editing Dockerfiles
- **Zero manual steps** - No `docker compose run` commands required from users

### Non-Goals

- **Long-running daemon** - Each command spawns a fresh container (aligns with "fresh context" philosophy)
- **Custom base images** - Use official Claude Code devcontainer as base
- **IDE integration** - VS Code devcontainer support is separate (see docker-isolation.md)

### Relationship to docker-isolation.md

This spec extends `docker-isolation.md` with UX improvements. The isolation spec covers security foundations; this spec covers the developer experience layer.

---

## 2. Architecture

### Component Structure

```
src/
├── docker.rs              # Docker orchestration (UPDATED)
│   ├── is_inside_container()
│   ├── ensure_image_built()
│   ├── run_in_container()  # NEW: Transparent execution
│   └── stream_output()     # NEW: TTY streaming
├── config.rs              # Config with presets (UPDATED)
│   └── DockerConfig
│       ├── use_docker
│       ├── presets         # NEW: ["rust", "bun"]
│       └── setup_script    # NEW: Optional custom script
└── commands/
    ├── plan.rs            # Calls docker::run_in_container()
    └── build.rs           # Calls docker::run_in_container()

.fresher/
├── config.toml            # Docker settings with presets
└── docker/
    ├── Dockerfile         # UPDATED: Multi-stage with presets
    ├── docker-compose.yml # UPDATED: For orchestration
    └── setup.sh           # Optional custom setup script
```

### Execution Flow

```
┌─────────────────────────────────────────────────────────────────────┐
│  User runs: fresher plan                                            │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  1. Load config.toml                                                │
│     └── use_docker = true                                           │
│     └── presets = ["rust", "bun"]                                   │
│                                                                     │
│  2. Check: Already in container?                                    │
│     ├── Yes → Continue with normal execution                        │
│     └── No  → Proceed to step 3                                     │
│                                                                     │
│  3. Ensure Docker image is built                                    │
│     ├── Check if image exists with matching preset hash            │
│     ├── If missing/outdated → Build with progress output            │
│     └── Cache image for subsequent runs                             │
│                                                                     │
│  4. Launch container with streaming                                 │
│     ┌─────────────────────────────────────────────────────────────┐ │
│     │  docker compose run --rm -T fresher fresher plan            │ │
│     │                                                             │ │
│     │  TTY Allocation:                                            │ │
│     │  - Allocate PTY for streaming output                        │ │
│     │  - Pass through SIGINT/SIGTERM                              │ │
│     │  - Forward exit code                                        │ │
│     └─────────────────────────────────────────────────────────────┘ │
│                                                                     │
│  5. Stream output in real-time                                      │
│     └── User sees Claude output as it happens                       │
│                                                                     │
│  6. Container exits, host fresher returns exit code                 │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 3. Core Types

### 3.1 DockerConfig (Updated)

Extended configuration for Docker settings.

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DockerConfig {
    /// Enable Docker isolation
    #[serde(default)]
    pub use_docker: bool,

    /// Memory limit for container
    #[serde(default = "default_memory")]
    pub memory: String,

    /// CPU limit for container
    #[serde(default = "default_cpus")]
    pub cpus: String,

    /// Toolchain presets to install
    #[serde(default)]
    pub presets: Vec<String>,

    /// Optional custom setup script path
    #[serde(default)]
    pub setup_script: Option<String>,
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `use_docker` | bool | No | Enable Docker isolation (default: false) |
| `memory` | String | No | Container memory limit (default: "4g") |
| `cpus` | String | No | Container CPU limit (default: "2") |
| `presets` | Vec<String> | No | Toolchain presets: "rust", "node", "bun", "python", "go" |
| `setup_script` | Option<String> | No | Path to custom setup script (relative to .fresher/) |

### 3.2 Preset

Represents a toolchain preset with installation commands.

```rust
pub struct Preset {
    pub name: &'static str,
    pub description: &'static str,
    pub install_commands: &'static [&'static str],
}

pub const PRESETS: &[Preset] = &[
    Preset {
        name: "rust",
        description: "Rust toolchain via rustup",
        install_commands: &[
            "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y",
            "source $HOME/.cargo/env",
        ],
    },
    Preset {
        name: "node",
        description: "Node.js (already in base image)",
        install_commands: &[], // Pre-installed
    },
    Preset {
        name: "bun",
        description: "Bun JavaScript runtime",
        install_commands: &[
            "curl -fsSL https://bun.sh/install | bash",
        ],
    },
    Preset {
        name: "python",
        description: "Python 3 with pip",
        install_commands: &[
            "sudo apt-get update && sudo apt-get install -y python3 python3-pip python3-venv",
        ],
    },
    Preset {
        name: "go",
        description: "Go programming language",
        install_commands: &[
            "curl -fsSL https://go.dev/dl/go1.22.0.linux-amd64.tar.gz | sudo tar -C /usr/local -xzf -",
            "echo 'export PATH=$PATH:/usr/local/go/bin' >> ~/.bashrc",
        ],
    },
];
```

---

## 4. Behaviors

### 4.1 Auto-Orchestration

When `use_docker=true` and not already in a container, Fresher auto-launches Docker.

**Implementation in docker.rs:**

```rust
pub fn run_in_container(config: &Config, args: &[String]) -> Result<i32> {
    // Already in container? Just return and let normal execution proceed
    if is_inside_container() {
        return Ok(-1); // Sentinel: caller should proceed normally
    }

    // Not in container but docker enabled? Orchestrate!
    if !config.docker.use_docker {
        return Ok(-1); // Docker disabled, proceed normally
    }

    // Ensure image is built (with preset hash check)
    ensure_image_built(config)?;

    // Build the command
    let compose_file = ".fresher/docker/docker-compose.yml";
    let mut cmd = Command::new("docker");
    cmd.args(["compose", "-f", compose_file, "run", "--rm"]);

    // TTY allocation for streaming
    if atty::is(atty::Stream::Stdout) {
        cmd.arg("-t");
    }

    // Pass through the fresher command
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
```

**Usage in commands/plan.rs:**

```rust
pub fn run(args: &PlanArgs) -> Result<()> {
    let config = Config::load()?;

    // Try Docker orchestration first
    let docker_args = vec!["plan".to_string()]; // Add any args
    match docker::run_in_container(&config, &docker_args)? {
        -1 => {} // Proceed with normal execution
        code => return Ok(std::process::exit(code)), // Docker handled it
    }

    // Normal execution (either docker disabled or inside container)
    // ... rest of plan logic
}
```

### 4.2 Streaming Output

Proper TTY allocation ensures real-time output.

**Key elements:**
1. Use `-t` flag when stdout is a TTY
2. Inherit stdin/stdout/stderr from parent process
3. Forward signals (SIGINT, SIGTERM) to container

**docker-compose.yml settings:**

```yaml
services:
  fresher:
    stdin_open: true  # Keep stdin open
    tty: true         # Allocate TTY
```

### 4.3 Image Building with Presets

Generate Dockerfile dynamically based on configured presets.

**Implementation:**

```rust
pub fn ensure_image_built(config: &Config) -> Result<()> {
    let preset_hash = hash_presets(&config.docker.presets);
    let image_tag = format!("fresher-dev:{}", preset_hash);

    // Check if image exists
    let exists = Command::new("docker")
        .args(["image", "inspect", &image_tag])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?
        .success();

    if exists {
        println!("[Docker] Using cached image");
        return Ok(());
    }

    println!("[Docker] Building image with presets: {:?}", config.docker.presets);

    // Generate Dockerfile
    let dockerfile = generate_dockerfile(&config.docker)?;
    fs::write(".fresher/docker/Dockerfile.generated", &dockerfile)?;

    // Build image
    let status = Command::new("docker")
        .args(["build", "-t", &image_tag, "-f", ".fresher/docker/Dockerfile.generated", ".fresher/docker"])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        bail!("Docker image build failed");
    }

    Ok(())
}

fn generate_dockerfile(docker_config: &DockerConfig) -> Result<String> {
    let mut dockerfile = String::from(
        r#"# Auto-generated by Fresher - do not edit manually
FROM ghcr.io/anthropics/claude-code-devcontainer:latest

USER root

"#,
    );

    // Add preset installation commands
    for preset_name in &docker_config.presets {
        if let Some(preset) = PRESETS.iter().find(|p| p.name == preset_name) {
            dockerfile.push_str(&format!("# Preset: {}\n", preset.name));
            for cmd in preset.install_commands {
                dockerfile.push_str(&format!("RUN {}\n", cmd));
            }
            dockerfile.push('\n');
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

    dockerfile.push_str("USER node\n");

    Ok(dockerfile)
}
```

### 4.4 Preset Hash for Caching

Cache Docker images based on preset configuration to avoid rebuilds.

```rust
fn hash_presets(presets: &[String]) -> String {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;

    let mut hasher = DefaultHasher::new();
    presets.hash(&mut hasher);
    format!("{:x}", hasher.finish())[..8].to_string()
}
```

---

## 5. Configuration

### 5.1 config.toml Example

```toml
[docker]
use_docker = true
memory = "4g"
cpus = "2"
presets = ["rust", "bun"]
# setup_script = "docker/custom-setup.sh"  # Optional
```

### 5.2 Environment Variables

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `FRESHER_USE_DOCKER` | bool | false | Override config.toml docker setting |
| `FRESHER_IN_DOCKER` | bool | (auto) | Set inside container |
| `FRESHER_DOCKER_PRESETS` | string | "" | Comma-separated presets override |

---

## 6. User Experience

### 6.1 First Run (with Docker)

```
$ fresher plan
[Docker] Building image with presets: ["rust"]
[Docker] Step 1/4: FROM ghcr.io/anthropics/claude-code-devcontainer:latest
[Docker] Step 2/4: Installing Rust toolchain...
[Docker] Step 3/4: Configuring environment...
[Docker] Step 4/4: Done
[Docker] Starting container...
[Planning] Starting iteration 1...
... streaming Claude output ...
[Planning] Plan complete. See IMPLEMENTATION_PLAN.md
```

### 6.2 Subsequent Runs (Cached)

```
$ fresher plan
[Docker] Using cached image
[Docker] Starting container...
[Planning] Starting iteration 1...
... streaming Claude output ...
```

### 6.3 Without Docker

```
$ fresher plan
[Planning] Starting iteration 1...
... normal output ...
```

### 6.4 Error: Docker Not Installed

```
$ fresher plan
Error: Docker is enabled but not installed or not running.

To install Docker: https://docs.docker.com/get-docker/
To disable Docker: Set use_docker = false in .fresher/config.toml
```

---

## 7. Security Considerations

### Container Isolation

- Containers run as non-root user (`node`)
- Network restricted by firewall (see docker-isolation.md)
- Project mounted read-write at `/workspace`
- Host credentials mounted read-only

### Credential Handling

- OAuth credentials mounted from host `~/.claude:ro`
- No credential storage inside container
- Container destroyed after each command

---

## 8. Implementation Phases

| Phase | Description | Dependencies | Complexity |
|-------|-------------|--------------|------------|
| 1 | Add `run_in_container()` to docker.rs | None | Medium |
| 2 | Update plan.rs and build.rs to call orchestration | Phase 1 | Low |
| 3 | Add preset configuration to config.rs | None | Low |
| 4 | Implement Dockerfile generation with presets | Phase 3 | Medium |
| 5 | Add image caching with preset hash | Phase 4 | Low |
| 6 | Update templates for new docker-compose.yml | Phase 1-5 | Low |
| 7 | Documentation and error messages | Phase 1-6 | Low |

---

## 9. Testing

### Test Scenarios

1. **Auto-orchestration**: `use_docker=true` should spawn container automatically
2. **Streaming**: Output should appear in real-time, not buffered
3. **Presets**: Each preset should install correctly
4. **Caching**: Second run should skip image build
5. **Signal handling**: Ctrl+C should stop container gracefully
6. **Exit codes**: Container exit code should propagate to host

### Manual Test Commands

```bash
# Test auto-orchestration
FRESHER_USE_DOCKER=true fresher plan

# Test streaming (should see output immediately)
FRESHER_USE_DOCKER=true fresher build

# Test presets
# In config.toml: presets = ["rust"]
cargo --version  # Should work inside container

# Test cache invalidation
# Change presets, run again - should rebuild
```

---

## 10. Local Development & Testing

### 10.1 Problem Statement

The current Docker workflow has a chicken-and-egg issue:

1. Fresher is installed via `cargo install --git` from GitHub during image build
2. This means you can't test Docker changes before publishing
3. Projects without `rust` preset cannot run fresher at all
4. Image builds are slow because they compile fresher from source

### 10.2 Solution: Local Binary Mounting

**Strategy:** Mount the locally-built fresher binary into the container at runtime instead of building from source.

**Benefits:**
- No compilation during Docker image build
- Instant iteration: change code → rebuild → test immediately
- Works for all presets (not just rust)
- Consistent binary between host and container

### 10.3 Configuration Changes

#### DockerConfig Extension

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DockerConfig {
    // ... existing fields ...

    /// Path to local fresher binary (for development)
    /// When set, mounts this binary instead of installing from GitHub
    #[serde(default)]
    pub local_binary: Option<String>,
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `local_binary` | Option<String> | No | Path to local fresher binary (e.g., `"./target/release/fresher"`) |

#### config.toml Example

```toml
[docker]
use_docker = true
presets = ["bun"]
# For development: mount local binary instead of building from GitHub
local_binary = "./target/release/fresher"
```

### 10.4 Implementation Changes

#### Dockerfile Generation

Remove the GitHub install logic. Fresher binary comes from mount, not build:

```rust
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

# Create directories for fresher binary and user config
RUN mkdir -p /usr/local/bin /home/node/.claude && chown -R node:node /home/node

"#,
        BASE_IMAGE
    );

    // Add preset installation commands (unchanged)
    for preset_name in &docker_config.presets {
        // ... preset logic unchanged ...
    }

    // NO LONGER: Install fresher from GitHub
    // Fresher binary is mounted at runtime

    dockerfile.push_str("USER node\n");
    Ok(dockerfile)
}
```

#### Docker Compose Generation

Add fresher binary volume mount:

```rust
pub fn generate_docker_compose(config: &DockerConfig) -> String {
    let image_tag = get_image_tag(&config.presets);

    // Determine fresher binary source
    let fresher_mount = if let Some(local_path) = &config.local_binary {
        format!("      - {}:/usr/local/bin/fresher:ro\n", local_path)
    } else {
        // Default: expect fresher pre-installed in image or use host binary
        String::new()
    };

    format!(
        r#"# Fresher Docker Compose Configuration
# Auto-generated - do not edit manually

services:
  fresher:
    image: {image_tag}
    stdin_open: true
    tty: true
    mem_limit: {memory}
    cpus: {cpus}

    volumes:
      - ${{PWD}}:/workspace
      - ${{HOME}}/.claude:/home/node/.claude
{fresher_mount}
    environment:
      - FRESHER_IN_DOCKER=true
      - DEVCONTAINER=true
      - ANTHROPIC_API_KEY=${{ANTHROPIC_API_KEY:-}}

    working_dir: /workspace
    entrypoint: ""
"#,
        image_tag = image_tag,
        memory = config.memory,
        cpus = config.cpus,
        fresher_mount = fresher_mount,
    )
}
```

### 10.5 Unit Tests

Test Dockerfile and compose generation without Docker:

```rust
// tests/docker.rs

#[cfg(test)]
mod docker_tests {
    use fresher::docker::*;
    use fresher::config::DockerConfig;

    #[test]
    fn test_generate_dockerfile_no_presets() {
        let config = DockerConfig {
            use_docker: true,
            memory: "4g".to_string(),
            cpus: "2".to_string(),
            presets: vec![],
            setup_script: None,
            local_binary: None,
        };

        let dockerfile = generate_dockerfile(&config).unwrap();

        assert!(dockerfile.contains("FROM node:20-bookworm"));
        assert!(dockerfile.contains("npm install -g @anthropic-ai/claude-code"));
        // Should NOT contain cargo install fresher
        assert!(!dockerfile.contains("cargo install"));
        assert!(!dockerfile.contains("shanewwarren/fresher"));
    }

    #[test]
    fn test_generate_dockerfile_with_presets() {
        let config = DockerConfig {
            use_docker: true,
            memory: "4g".to_string(),
            cpus: "2".to_string(),
            presets: vec!["rust".to_string(), "bun".to_string()],
            setup_script: None,
            local_binary: None,
        };

        let dockerfile = generate_dockerfile(&config).unwrap();

        assert!(dockerfile.contains("rustup.rs"));
        assert!(dockerfile.contains("bun.sh/install"));
        assert!(dockerfile.contains("CARGO_HOME"));
    }

    #[test]
    fn test_generate_compose_with_local_binary() {
        let config = DockerConfig {
            use_docker: true,
            memory: "4g".to_string(),
            cpus: "2".to_string(),
            presets: vec![],
            setup_script: None,
            local_binary: Some("./target/release/fresher".to_string()),
        };

        let compose = generate_docker_compose(&config);

        assert!(compose.contains("./target/release/fresher:/usr/local/bin/fresher:ro"));
        assert!(compose.contains("FRESHER_IN_DOCKER=true"));
    }

    #[test]
    fn test_generate_compose_without_local_binary() {
        let config = DockerConfig {
            use_docker: true,
            memory: "4g".to_string(),
            cpus: "2".to_string(),
            presets: vec!["rust".to_string()],
            setup_script: None,
            local_binary: None,
        };

        let compose = generate_docker_compose(&config);

        // Should NOT have fresher mount line
        assert!(!compose.contains("/usr/local/bin/fresher"));
    }

    #[test]
    fn test_hash_presets_deterministic() {
        let presets1 = vec!["rust".to_string(), "bun".to_string()];
        let presets2 = vec!["rust".to_string(), "bun".to_string()];
        let presets3 = vec!["bun".to_string(), "rust".to_string()];

        assert_eq!(hash_presets(&presets1), hash_presets(&presets2));
        // Order matters for hash
        assert_ne!(hash_presets(&presets1), hash_presets(&presets3));
    }

    #[test]
    fn test_get_image_tag_no_presets() {
        let presets: Vec<String> = vec![];
        assert_eq!(get_image_tag(&presets), "fresher-base:latest");
    }

    #[test]
    fn test_get_image_tag_with_presets() {
        let presets = vec!["rust".to_string()];
        let tag = get_image_tag(&presets);
        assert!(tag.starts_with("fresher-dev:"));
        assert!(tag.len() > "fresher-dev:".len());
    }

    #[test]
    fn test_is_inside_container_false() {
        // In normal test environment, should be false
        std::env::remove_var("DEVCONTAINER");
        std::env::remove_var("FRESHER_IN_DOCKER");
        assert!(!is_inside_container());
    }

    #[test]
    fn test_is_inside_container_devcontainer() {
        std::env::set_var("DEVCONTAINER", "true");
        assert!(is_inside_container());
        std::env::remove_var("DEVCONTAINER");
    }

    #[test]
    fn test_is_inside_container_fresher_flag() {
        std::env::set_var("FRESHER_IN_DOCKER", "true");
        assert!(is_inside_container());
        std::env::remove_var("FRESHER_IN_DOCKER");
    }
}
```

### 10.6 E2E Integration Tests

Full Docker tests that actually build images and run containers:

```rust
// tests/docker_e2e.rs

//! Docker E2E tests - requires Docker to be running
//! Run with: cargo test --test docker_e2e -- --ignored

use std::process::Command;
use std::path::Path;
use tempfile::TempDir;

/// Check if Docker is available
fn docker_available() -> bool {
    Command::new("docker")
        .args(["info"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
#[ignore] // Run explicitly: cargo test --test docker_e2e -- --ignored
fn test_docker_image_builds_successfully() {
    if !docker_available() {
        eprintln!("Skipping: Docker not available");
        return;
    }

    let dir = TempDir::new().unwrap();
    let project_dir = dir.path();

    // Create minimal fresher project
    std::fs::create_dir_all(project_dir.join(".fresher/docker")).unwrap();

    // Generate Dockerfile with no presets (fastest build)
    let dockerfile = r#"
FROM node:20-bookworm
RUN apt-get update && apt-get install -y git jq curl bash && rm -rf /var/lib/apt/lists/*
RUN npm install -g @anthropic-ai/claude-code
RUN mkdir -p /home/node/.claude && chown -R node:node /home/node
USER node
"#;
    std::fs::write(project_dir.join(".fresher/docker/Dockerfile"), dockerfile).unwrap();

    // Build image
    let output = Command::new("docker")
        .args([
            "build",
            "-t", "fresher-test:e2e",
            "-f", ".fresher/docker/Dockerfile",
            ".fresher/docker"
        ])
        .current_dir(project_dir)
        .output()
        .expect("Failed to run docker build");

    assert!(output.status.success(), "Docker build failed: {}",
        String::from_utf8_lossy(&output.stderr));

    // Cleanup
    let _ = Command::new("docker")
        .args(["rmi", "fresher-test:e2e"])
        .output();
}

#[test]
#[ignore]
fn test_container_detects_fresher_in_docker_env() {
    if !docker_available() {
        eprintln!("Skipping: Docker not available");
        return;
    }

    // Run a command that checks FRESHER_IN_DOCKER
    let output = Command::new("docker")
        .args([
            "run", "--rm",
            "-e", "FRESHER_IN_DOCKER=true",
            "node:20-bookworm",
            "sh", "-c", "echo $FRESHER_IN_DOCKER"
        ])
        .output()
        .expect("Failed to run docker");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.trim() == "true", "Expected 'true', got '{}'", stdout);
}

#[test]
#[ignore]
fn test_fresher_binary_mount_works() {
    if !docker_available() {
        eprintln!("Skipping: Docker not available");
        return;
    }

    // Build fresher first
    let build_output = Command::new("cargo")
        .args(["build", "--release"])
        .output()
        .expect("Failed to build fresher");

    if !build_output.status.success() {
        panic!("Failed to build fresher: {}",
            String::from_utf8_lossy(&build_output.stderr));
    }

    let binary_path = Path::new("target/release/fresher");
    assert!(binary_path.exists(), "Fresher binary not found");

    // Get absolute path for mount
    let abs_binary = std::fs::canonicalize(binary_path).unwrap();

    // Run container with mounted binary
    let output = Command::new("docker")
        .args([
            "run", "--rm",
            "-v", &format!("{}:/usr/local/bin/fresher:ro", abs_binary.display()),
            "node:20-bookworm",
            "/usr/local/bin/fresher", "version"
        ])
        .output()
        .expect("Failed to run docker with mounted binary");

    assert!(output.status.success(), "Fresher failed in container: {}",
        String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("fresher"), "Expected version output, got: {}", stdout);
}

#[test]
#[ignore]
fn test_preset_rust_installs_cargo() {
    if !docker_available() {
        eprintln!("Skipping: Docker not available");
        return;
    }

    let dir = TempDir::new().unwrap();

    // Dockerfile with rust preset
    let dockerfile = r#"
FROM node:20-bookworm
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
            "build", "-t", "fresher-test:rust",
            "-f", "Dockerfile", "."
        ])
        .current_dir(dir.path())
        .output()
        .expect("Failed to build");

    assert!(build.status.success(), "Build failed: {}",
        String::from_utf8_lossy(&build.stderr));

    // Test cargo is available
    let test = Command::new("docker")
        .args([
            "run", "--rm",
            "fresher-test:rust",
            "cargo", "--version"
        ])
        .output()
        .expect("Failed to run");

    assert!(test.status.success(), "Cargo not available: {}",
        String::from_utf8_lossy(&test.stderr));

    // Cleanup
    let _ = Command::new("docker").args(["rmi", "fresher-test:rust"]).output();
}
```

### 10.7 Test Execution

```bash
# Run unit tests (fast, no Docker needed)
cargo test docker

# Run E2E tests (requires Docker)
cargo test --test docker_e2e -- --ignored

# Run specific E2E test
cargo test --test docker_e2e test_fresher_binary_mount_works -- --ignored

# Run all tests including Docker E2E
cargo test -- --include-ignored
```

### 10.8 CI/CD Integration

Add to `.github/workflows/test.yml`:

```yaml
name: Tests

on: [push, pull_request]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --lib
      - run: cargo test --test init
      - run: cargo test --test verify
      - run: cargo test --test hooks

  docker-e2e:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Build fresher
        run: cargo build --release
      - name: Run Docker E2E tests
        run: cargo test --test docker_e2e -- --ignored
```

---

## 11. Open Questions

- [x] ~~Should there be a `fresher docker status` command to show image/container state?~~ Deferred
- [ ] How to handle preset version pinning (e.g., specific Rust version)?
- [ ] Should presets support dependencies (e.g., "rust" requires "build-essential")?
- [ ] How to handle preset conflicts (unlikely but possible)?
- [ ] Should `local_binary` auto-detect from `./target/release/fresher` if present?
