# Docker Isolation Specification

**Status:** Planned
**Version:** 1.0
**Last Updated:** 2025-01-17

---

## 1. Overview

### Purpose

Docker isolation provides a safety layer when running Claude Code with dangerous permissions (`--dangerously-skip-permissions`). By executing inside a container with resource limits and optional network isolation, the blast radius of unintended actions is contained.

### Goals

- **Safety** - Limit what Claude can access and modify
- **Resource control** - Prevent runaway processes from consuming system resources
- **Reproducibility** - Same environment across different machines
- **Transparency** - User can see exactly what's happening inside the container

### Non-Goals

- **Complete security** - Docker is not a security boundary; determined code can escape
- **Production deployment** - This is for development safety, not production isolation
- **Multi-container orchestration** - Single container execution only

---

## 2. Architecture

### Component Structure

```
.fresher/
├── docker/
│   ├── Dockerfile          # Container image definition
│   ├── docker-compose.yml  # Orchestration config
│   └── entrypoint.sh       # Container entry point
├── run.sh                  # Detects Docker mode
└── config.sh               # Docker settings
```

### Execution Flow

```
┌─────────────────────────────────────────────────────────────────┐
│  fresher build (with FRESHER_USE_DOCKER=true)                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. Check Docker availability                                   │
│  2. Build image if needed                                       │
│  3. Start container with:                                       │
│     - Volume mounts (project files)                             │
│     - Resource limits                                           │
│     - Network configuration                                     │
│     - Environment variables                                     │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  CONTAINER                                               │   │
│  │  ┌─────────────────────────────────────────────────────┐ │   │
│  │  │  entrypoint.sh                                      │ │   │
│  │  │  └── run.sh (loop executor)                         │ │   │
│  │  │      └── claude (Claude Code CLI)                   │ │   │
│  │  └─────────────────────────────────────────────────────┘ │   │
│  │                                                          │   │
│  │  Mounts:                                                 │   │
│  │  - /workspace (project root, rw)                         │   │
│  │  - /workspace/specs (read-only)                          │   │
│  │                                                          │   │
│  │  Limits:                                                 │   │
│  │  - Memory: 4GB                                           │   │
│  │  - CPU: 2 cores                                          │   │
│  │  - PIDs: 256                                             │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  4. Stream output to host terminal                              │
│  5. Cleanup on exit                                             │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. Core Types

### 3.1 Docker Configuration

Environment variables in `config.sh`:

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `FRESHER_USE_DOCKER` | boolean | false | Enable Docker isolation |
| `FRESHER_DOCKER_IMAGE` | string | `fresher:local` | Docker image to use |
| `FRESHER_DOCKER_MEMORY` | string | `4g` | Memory limit |
| `FRESHER_DOCKER_CPUS` | string | `2` | CPU limit |
| `FRESHER_DOCKER_NETWORK` | string | `bridge` | Network mode |
| `FRESHER_DOCKER_BUILD` | boolean | true | Auto-build image if missing |

### 3.2 Mount Configuration

| Host Path | Container Path | Mode | Description |
|-----------|---------------|------|-------------|
| `$(pwd)` | `/workspace` | rw | Project root |
| `$(pwd)/specs` | `/workspace/specs` | ro | Specifications (read-only) |
| `$(pwd)/.fresher` | `/workspace/.fresher` | rw | Fresher config and state |
| `~/.claude` | `/home/coder/.claude` | ro | Claude Code config (optional) |

### 3.3 Resource Limits

| Resource | Default | Configurable | Description |
|----------|---------|--------------|-------------|
| Memory | 4GB | Yes | Hard memory limit |
| Memory reservation | 2GB | Yes | Soft memory limit |
| CPUs | 2 | Yes | CPU core limit |
| PIDs | 256 | Yes | Process limit (prevents fork bombs) |

---

## 4. Behaviors

### 4.1 Dockerfile

```dockerfile
# .fresher/docker/Dockerfile
FROM ubuntu:22.04

# Install dependencies
RUN apt-get update && apt-get install -y \
    curl \
    git \
    jq \
    ripgrep \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Install Claude Code CLI
RUN curl -fsSL https://claude.ai/install.sh | bash

# Create non-root user
RUN useradd -m -s /bin/bash -u 1000 coder
USER coder

# Set working directory
WORKDIR /workspace

# Copy entrypoint
COPY --chown=coder:coder entrypoint.sh /home/coder/entrypoint.sh
RUN chmod +x /home/coder/entrypoint.sh

# Environment
ENV PATH="/home/coder/.local/bin:$PATH"
ENV FRESHER_IN_DOCKER="true"

ENTRYPOINT ["/home/coder/entrypoint.sh"]
```

### 4.2 Entrypoint Script

```bash
#!/bin/bash
# .fresher/docker/entrypoint.sh

set -e

echo "Fresher Docker Container"
echo "========================"
echo "User: $(whoami)"
echo "Workspace: $(pwd)"
echo "Mode: ${FRESHER_MODE:-not set}"
echo ""

# Verify mounts
if [[ ! -d "/workspace/.fresher" ]]; then
  echo "ERROR: .fresher directory not mounted"
  exit 1
fi

# Run the fresher loop
exec /workspace/.fresher/run.sh "$@"
```

### 4.3 Docker Compose Configuration

```yaml
# .fresher/docker/docker-compose.yml
version: '3.8'

services:
  fresher:
    build:
      context: .
      dockerfile: Dockerfile
    image: ${FRESHER_DOCKER_IMAGE:-fresher:local}
    container_name: fresher-${FRESHER_MODE:-loop}

    # Interactive mode for Claude Code
    stdin_open: true
    tty: true

    # Resource limits
    mem_limit: ${FRESHER_DOCKER_MEMORY:-4g}
    mem_reservation: ${FRESHER_DOCKER_MEMORY_RESERVATION:-2g}
    cpus: ${FRESHER_DOCKER_CPUS:-2}
    pids_limit: ${FRESHER_DOCKER_PIDS:-256}

    # Network configuration
    network_mode: ${FRESHER_DOCKER_NETWORK:-bridge}

    # Volume mounts
    volumes:
      - type: bind
        source: ${PROJECT_DIR:-.}
        target: /workspace
      - type: bind
        source: ${PROJECT_DIR:-.}/specs
        target: /workspace/specs
        read_only: true
      - type: bind
        source: ${HOME}/.claude
        target: /home/coder/.claude
        read_only: true

    # Environment
    environment:
      - FRESHER_MODE=${FRESHER_MODE:-planning}
      - FRESHER_MAX_ITERATIONS=${FRESHER_MAX_ITERATIONS:-0}
      - FRESHER_SMART_TERMINATION=${FRESHER_SMART_TERMINATION:-true}
      - FRESHER_IN_DOCKER=true

    # User mapping
    user: "${UID:-1000}:${GID:-1000}"

    # Working directory
    working_dir: /workspace
```

### 4.4 Docker Mode Detection in run.sh

```bash
# In .fresher/run.sh

if [[ "$FRESHER_USE_DOCKER" == "true" ]] && [[ -z "$FRESHER_IN_DOCKER" ]]; then
  # We're on host, need to launch Docker
  echo "Starting Fresher in Docker container..."

  # Ensure image exists
  if [[ "$FRESHER_DOCKER_BUILD" == "true" ]]; then
    if ! docker image inspect "$FRESHER_DOCKER_IMAGE" &>/dev/null; then
      echo "Building Docker image..."
      docker build -t "$FRESHER_DOCKER_IMAGE" .fresher/docker/
    fi
  fi

  # Export variables for docker-compose
  export PROJECT_DIR="$(pwd)"
  export UID="$(id -u)"
  export GID="$(id -g)"

  # Run via docker-compose
  exec docker compose -f .fresher/docker/docker-compose.yml run --rm fresher
fi

# If we reach here, either Docker is disabled or we're already in container
# Continue with normal execution...
```

### 4.5 Network Isolation Options

```bash
# In config.sh

# Option 1: Bridge (default) - container can access network
export FRESHER_DOCKER_NETWORK="bridge"

# Option 2: None - complete network isolation
export FRESHER_DOCKER_NETWORK="none"

# Option 3: Host - full network access (less safe)
export FRESHER_DOCKER_NETWORK="host"
```

**Recommendation by mode:**

| Mode | Recommended Network | Reason |
|------|---------------------|--------|
| Planning | `bridge` | May need to fetch docs |
| Building | `none` or `bridge` | Depends on if builds need network |

---

## 5. Security Considerations

### What Docker Isolation Provides

- **Filesystem boundaries** - Can only access mounted paths
- **Resource limits** - Cannot exhaust host memory/CPU
- **Process isolation** - Container processes are separate
- **User namespacing** - Runs as non-root user

### What Docker Does NOT Provide

- **Full security** - Docker is not a security sandbox
- **Kernel isolation** - Containers share host kernel
- **Protection from privileged escapes** - Determined code can escape

### Best Practices

1. **Never mount sensitive directories** (`~/.ssh`, `~/.aws`, etc.)
2. **Use `--network none`** when network isn't needed
3. **Set resource limits** to prevent resource exhaustion
4. **Review Docker logs** for unexpected behavior
5. **Keep images updated** for security patches

### Secrets Handling

```bash
# DON'T: Pass secrets as environment variables
docker run -e API_KEY=secret123 ...  # Visible in docker inspect

# DO: Use secret mounts
docker run --secret api_key ...  # Mounted at /run/secrets/api_key
```

For Claude Code API key:

```yaml
# docker-compose.yml
services:
  fresher:
    secrets:
      - claude_api_key

secrets:
  claude_api_key:
    file: ~/.config/claude/api_key
```

---

## 6. Configuration

### Quick Start

```bash
# Enable Docker isolation
echo 'export FRESHER_USE_DOCKER=true' >> .fresher/config.sh

# Run normally - will auto-launch in Docker
fresher build
```

### Full Configuration Example

```bash
# .fresher/config.sh

# Enable Docker
export FRESHER_USE_DOCKER="${FRESHER_USE_DOCKER:-true}"
export FRESHER_DOCKER_IMAGE="${FRESHER_DOCKER_IMAGE:-fresher:local}"

# Resource limits
export FRESHER_DOCKER_MEMORY="${FRESHER_DOCKER_MEMORY:-4g}"
export FRESHER_DOCKER_MEMORY_RESERVATION="${FRESHER_DOCKER_MEMORY_RESERVATION:-2g}"
export FRESHER_DOCKER_CPUS="${FRESHER_DOCKER_CPUS:-2}"
export FRESHER_DOCKER_PIDS="${FRESHER_DOCKER_PIDS:-256}"

# Network (none for maximum isolation)
export FRESHER_DOCKER_NETWORK="${FRESHER_DOCKER_NETWORK:-none}"

# Auto-build image if missing
export FRESHER_DOCKER_BUILD="${FRESHER_DOCKER_BUILD:-true}"
```

### Disabling Docker

```bash
# Temporarily disable
FRESHER_USE_DOCKER=false fresher build

# Permanently disable
echo 'export FRESHER_USE_DOCKER=false' >> .fresher/config.sh
```

---

## 7. Implementation Phases

| Phase | Description | Dependencies | Complexity |
|-------|-------------|--------------|------------|
| 1 | Dockerfile creation | None | Low |
| 2 | Docker Compose setup | Phase 1 | Medium |
| 3 | run.sh Docker detection | loop-executor | Low |
| 4 | Volume mount configuration | Phase 2 | Medium |
| 5 | Resource limits | Phase 2 | Low |
| 6 | Network isolation options | Phase 2 | Low |
| 7 | Secrets handling | Phase 2 | Medium |

---

## 8. Open Questions

- [ ] Should Docker image be pre-built and distributed, or always built locally?
- [ ] How to handle GUI tools (if any) inside the container?
- [ ] Should there be a `fresher docker shell` command for debugging?
- [ ] How to handle Docker-in-Docker scenarios (if host is already containerized)?
