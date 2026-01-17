# Docker Isolation Specification

**Status:** Planned
**Version:** 2.0
**Last Updated:** 2025-01-17

---

## 1. Overview

### Purpose

Docker isolation provides a safety layer when running Claude Code with dangerous permissions (`--dangerously-skip-permissions`). Fresher leverages the **official Claude Code devcontainer** as a foundation, adding loop-specific configuration for iterative execution.

### Goals

- **Leverage official tooling** - Use Anthropic's maintained devcontainer rather than custom images
- **Network security** - Benefit from the built-in firewall with domain whitelisting
- **Loop integration** - Configure devcontainer for Fresher's iterative execution model
- **Developer experience** - Support both VS Code devcontainers and CLI-only workflows

### Non-Goals

- **Custom base images** - Don't maintain separate Dockerfiles when official ones exist
- **Complete reimplementation** - Don't duplicate firewall or security logic
- **Production deployment** - This is for development safety, not production isolation

### Reference

The official Claude Code devcontainer is documented at:
- Docs: https://code.claude.com/docs/en/devcontainer
- Source: https://github.com/anthropics/claude-code/tree/main/.devcontainer

---

## 2. Architecture

### Component Structure

```
.fresher/
├── docker/
│   ├── devcontainer.json     # Fresher-customized devcontainer config
│   └── fresher-overlay.sh    # Additional Fresher setup (optional)
├── run.sh                    # Detects devcontainer mode
└── config.sh                 # Docker/devcontainer settings
```

### Execution Flow

```
┌─────────────────────────────────────────────────────────────────┐
│  fresher build (with FRESHER_USE_DOCKER=true)                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. Check if already in devcontainer (DEVCONTAINER=true)        │
│  2. If not, prompt user to open in VS Code devcontainer         │
│     OR use docker-compose for CLI-only workflow                 │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  DEVCONTAINER (Official Claude Code Base)                   ││
│  │                                                             ││
│  │  Built-in Security:                                         ││
│  │  ┌─────────────────────────────────────────────────────┐   ││
│  │  │  iptables firewall + ipset                          │   ││
│  │  │  ├── Allowed: npm, GitHub, Anthropic API, VS Code   │   ││
│  │  │  ├── Allowed: DNS (port 53), SSH (port 22)          │   ││
│  │  │  └── Default: REJECT all other outbound             │   ││
│  │  └─────────────────────────────────────────────────────┘   ││
│  │                                                             ││
│  │  Fresher Execution:                                         ││
│  │  ┌─────────────────────────────────────────────────────┐   ││
│  │  │  .fresher/run.sh                                    │   ││
│  │  │  └── claude --dangerously-skip-permissions          │   ││
│  │  │      └── (iterative loop execution)                 │   ││
│  │  └─────────────────────────────────────────────────────┘   ││
│  │                                                             ││
│  │  Mounts:                                                    ││
│  │  - /workspace (project root)                                ││
│  │  - /commandhistory (persistent bash history)                ││
│  │  - /home/node/.claude (Claude config persistence)           ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. Official Devcontainer Features

The Claude Code devcontainer provides these features out of the box:

### 3.1 Network Security (Firewall)

| Allowed Domain | Purpose |
|----------------|---------|
| `api.github.com`, `github.com` | Git operations, API access |
| `registry.npmjs.org` | npm package installation |
| `api.anthropic.com` | Claude API calls |
| `statsig.anthropic.com`, `statsig.com` | Analytics |
| `sentry.io` | Error tracking |
| `marketplace.visualstudio.com` | VS Code extensions |
| `vscode.blob.core.windows.net` | VS Code assets |
| DNS (port 53), SSH (port 22) | System services |

**Default policy:** REJECT all other outbound connections.

### 3.2 Pre-installed Tools

| Tool | Purpose |
|------|---------|
| Node.js 20 | JavaScript runtime |
| `@anthropic-ai/claude-code` | Claude CLI (globally installed) |
| git, gh | Version control |
| zsh + powerlevel10k | Shell with productivity features |
| fzf | Fuzzy finder |
| git-delta | Better git diffs |
| vim, nano | Editors |
| iptables, ipset | Firewall management |

### 3.3 Persistence

| Volume | Path | Purpose |
|--------|------|---------|
| `claude-code-bashhistory-*` | `/commandhistory` | Shell history |
| `claude-code-config-*` | `/home/node/.claude` | Claude configuration |

### 3.4 VS Code Extensions

Pre-configured extensions:
- `anthropic.claude-code` - Official Claude Code extension
- `dbaeumer.vscode-eslint` - ESLint integration
- `esbenp.prettier-vscode` - Code formatting
- `eamodio.gitlens` - Git visualization

---

## 4. Fresher Configuration

### 4.1 Fresher-Specific devcontainer.json

Create `.fresher/docker/devcontainer.json` that extends the official config:

```json
{
  "name": "Fresher Loop Environment",
  "build": {
    "dockerfile": "https://raw.githubusercontent.com/anthropics/claude-code/main/.devcontainer/Dockerfile",
    "args": {
      "TZ": "${localEnv:TZ:America/Los_Angeles}",
      "CLAUDE_CODE_VERSION": "latest"
    }
  },
  "runArgs": [
    "--cap-add=NET_ADMIN",
    "--cap-add=NET_RAW"
  ],
  "customizations": {
    "vscode": {
      "extensions": [
        "anthropic.claude-code"
      ],
      "settings": {
        "terminal.integrated.defaultProfile.linux": "zsh"
      }
    }
  },
  "remoteUser": "node",
  "mounts": [
    "source=fresher-bashhistory-${devcontainerId},target=/commandhistory,type=volume",
    "source=fresher-config-${devcontainerId},target=/home/node/.claude,type=volume"
  ],
  "containerEnv": {
    "NODE_OPTIONS": "--max-old-space-size=4096",
    "FRESHER_IN_DOCKER": "true",
    "DEVCONTAINER": "true"
  },
  "workspaceMount": "source=${localWorkspaceFolder},target=/workspace,type=bind,consistency=delegated",
  "workspaceFolder": "/workspace",
  "initializeCommand": "cp -r ${localWorkspaceFolder}/.fresher/docker/init-firewall.sh /tmp/ 2>/dev/null || true",
  "postStartCommand": "sudo /usr/local/bin/init-firewall.sh",
  "waitFor": "postStartCommand"
}
```

### 4.2 Environment Variables

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `FRESHER_USE_DOCKER` | boolean | `false` | Enable devcontainer mode |
| `FRESHER_IN_DOCKER` | boolean | (auto) | Set inside container |
| `DEVCONTAINER` | boolean | (auto) | Standard devcontainer indicator |

### 4.3 Config.sh Settings

```bash
# .fresher/config.sh

#──────────────────────────────────────────────────────────────────
# Docker/Devcontainer Configuration
#──────────────────────────────────────────────────────────────────
export FRESHER_USE_DOCKER="${FRESHER_USE_DOCKER:-false}"

# Resource limits (passed to devcontainer)
export FRESHER_DOCKER_MEMORY="${FRESHER_DOCKER_MEMORY:-4g}"
export FRESHER_DOCKER_CPUS="${FRESHER_DOCKER_CPUS:-2}"
```

---

## 5. Behaviors

### 5.1 Devcontainer Detection in run.sh

```bash
# In .fresher/run.sh

# Check if we should use Docker isolation
if [[ "$FRESHER_USE_DOCKER" == "true" ]]; then
  # Already in a devcontainer?
  if [[ "$DEVCONTAINER" == "true" ]] || [[ "$FRESHER_IN_DOCKER" == "true" ]]; then
    echo "Running in devcontainer environment"
    # Continue with normal execution
  else
    echo "Docker isolation enabled but not in devcontainer."
    echo ""
    echo "Options:"
    echo "  1. Open this folder in VS Code and use 'Reopen in Container'"
    echo "  2. Run: docker compose -f .fresher/docker/docker-compose.yml run --rm fresher"
    echo ""
    echo "To disable Docker isolation: export FRESHER_USE_DOCKER=false"
    exit 1
  fi
fi

# Continue with loop execution...
```

### 5.2 CLI-Only Docker Compose

For users without VS Code, provide a docker-compose.yml:

```yaml
# .fresher/docker/docker-compose.yml
version: '3.8'

services:
  fresher:
    image: ghcr.io/anthropics/claude-code-devcontainer:latest
    container_name: fresher-${FRESHER_MODE:-loop}

    # Required for firewall setup
    cap_add:
      - NET_ADMIN
      - NET_RAW

    # Interactive mode
    stdin_open: true
    tty: true

    # Resource limits
    mem_limit: ${FRESHER_DOCKER_MEMORY:-4g}
    cpus: ${FRESHER_DOCKER_CPUS:-2}

    # Volume mounts
    volumes:
      - ${PWD}:/workspace
      - fresher-bashhistory:/commandhistory
      - fresher-config:/home/node/.claude

    # Environment
    environment:
      - FRESHER_MODE=${FRESHER_MODE:-planning}
      - FRESHER_MAX_ITERATIONS=${FRESHER_MAX_ITERATIONS:-0}
      - FRESHER_IN_DOCKER=true
      - DEVCONTAINER=true

    # User mapping
    user: node

    # Working directory
    working_dir: /workspace

    # Initialize firewall on start
    command: >
      bash -c "sudo /usr/local/bin/init-firewall.sh && /workspace/.fresher/run.sh"

volumes:
  fresher-bashhistory:
  fresher-config:
```

### 5.3 Firewall Customization (Optional)

If your project needs additional domains (e.g., private npm registry), create an overlay:

```bash
#!/bin/bash
# .fresher/docker/fresher-firewall-overlay.sh

# Add custom domains to the whitelist
# Run AFTER the standard init-firewall.sh

CUSTOM_DOMAINS=(
  "npm.mycompany.com"
  "api.internal-service.com"
)

for domain in "${CUSTOM_DOMAINS[@]}"; do
  ips=$(dig +short A "$domain" 2>/dev/null)
  for ip in $ips; do
    if [[ $ip =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
      sudo ipset add allowed-domains "$ip" 2>/dev/null || true
      echo "Added $domain ($ip) to whitelist"
    fi
  done
done
```

---

## 6. Security Considerations

### What the Official Devcontainer Provides

- **Domain-based firewall** - Only whitelisted services accessible
- **Default-deny networking** - All non-whitelisted connections rejected
- **Non-root execution** - Runs as `node` user
- **Capability restrictions** - Only NET_ADMIN/NET_RAW for firewall setup

### Security Warnings

From the official documentation:

> While the devcontainer provides substantial protections, no system is completely immune to all attacks. When executed with `--dangerously-skip-permissions`, devcontainers don't prevent a malicious project from exfiltrating anything accessible in the container including Claude Code credentials. **Only use devcontainers when developing with trusted repositories.**

### Best Practices

1. **Only use with trusted projects** - The firewall doesn't prevent credential theft
2. **Review firewall rules** - Check what domains are whitelisted
3. **Don't mount sensitive directories** - Avoid `~/.ssh`, `~/.aws`, etc.
4. **Keep devcontainer updated** - Pull latest official images regularly
5. **Monitor Claude's activities** - Review iteration logs

### What's NOT Protected

- Credentials accessible inside the container
- Files mounted into the container
- Side-channel attacks through allowed domains
- Kernel-level exploits (containers share host kernel)

---

## 7. Usage

### Quick Start (VS Code)

```bash
# 1. Enable Docker in Fresher config
echo 'export FRESHER_USE_DOCKER=true' >> .fresher/config.sh

# 2. Copy devcontainer config
mkdir -p .devcontainer
cp .fresher/docker/devcontainer.json .devcontainer/

# 3. Open in VS Code
code .
# Then: Cmd+Shift+P → "Remote-Containers: Reopen in Container"

# 4. Run Fresher inside container
fresher build
```

### Quick Start (CLI-Only)

```bash
# 1. Enable Docker in Fresher config
echo 'export FRESHER_USE_DOCKER=true' >> .fresher/config.sh

# 2. Run via docker-compose
docker compose -f .fresher/docker/docker-compose.yml run --rm fresher
```

### Disabling Docker Isolation

```bash
# Temporarily
FRESHER_USE_DOCKER=false fresher build

# Permanently
echo 'export FRESHER_USE_DOCKER=false' >> .fresher/config.sh
```

---

## 8. Implementation Phases

| Phase | Description | Dependencies | Complexity |
|-------|-------------|--------------|------------|
| 1 | Create Fresher devcontainer.json | None | Low |
| 2 | Create docker-compose.yml for CLI | Phase 1 | Low |
| 3 | Add detection logic to run.sh | loop-executor | Low |
| 4 | Document firewall customization | Phase 1 | Low |
| 5 | Test with various project types | Phase 1-4 | Medium |

---

## 9. Migration from v1.0

If you previously used the custom Dockerfile approach from v1.0:

1. **Remove custom Dockerfile** - No longer needed
2. **Update devcontainer.json** - Use the official base image
3. **Remove manual resource limits** - Handled by devcontainer
4. **Update firewall customizations** - Use overlay script instead

---

## 10. Open Questions

- [x] ~~Should Docker image be pre-built or always built locally?~~ → Use official image
- [x] ~~How to handle GUI tools inside the container?~~ → VS Code handles this via devcontainer
- [ ] Should there be a `fresher docker shell` command for debugging?
- [ ] How to handle additional package registries (private npm, cargo, etc.)?
- [ ] Should Fresher auto-detect when devcontainer.json exists in project root?
