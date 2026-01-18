# Docker UX Specification

**Status:** Planned
**Version:** 1.0
**Last Updated:** 2025-01-18

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

## 10. Open Questions

- [ ] Should there be a `fresher docker status` command to show image/container state?
- [ ] How to handle preset version pinning (e.g., specific Rust version)?
- [ ] Should presets support dependencies (e.g., "rust" requires "build-essential")?
- [ ] How to handle preset conflicts (unlikely but possible)?
