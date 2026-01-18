# Docker Isolation Specification

**Status:** Planned
**Version:** 2.1
**Last Updated:** 2026-01-18

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
| `update.code.visualstudio.com` | VS Code updates |
| DNS (port 53), SSH (port 22) | System services |

**Default policy:** REJECT all other outbound connections.

> **⚠️ OAuth Gap:** The official devcontainer does **NOT** whitelist `claude.ai`, which is required for OAuth authentication (Max/Pro plans). If using OAuth, you must add this domain via the firewall overlay (see Section 5.3).

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

### 4.2 Authentication

Claude Code supports two authentication methods. The container must be configured appropriately for your plan type.

#### Option A: API Key (Pay-as-you-go / Teams)

For API key authentication, pass the key as an environment variable:

```yaml
# In docker-compose.yml
environment:
  - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}
```

#### Option B: OAuth (Max / Pro Plans) - Recommended

For Claude Max/Pro plans, authentication uses OAuth which requires browser-based login. Since containers cannot open browsers directly, **authenticate on your host first** and mount the credentials:

**Step 1: Authenticate on Host**

```bash
# On your host machine (not in Docker)
claude auth login
# Browser opens → Log in with your Max/Pro account
# Credentials saved to ~/.claude/
```

**Step 2: Verify Authentication**

```bash
claude auth status
# Should show: Authenticated (Max plan)
```

**Step 3: Mount Host Credentials into Container**

Update docker-compose.yml to mount your host's Claude config:

```yaml
volumes:
  # Mount host credentials (read-only for safety)
  - ${HOME}/.claude:/home/node/.claude:ro
  # ... other volumes
```

**Alternative: Interactive Authentication in Container**

If you need to authenticate from within the container (e.g., fresh setup), `claude auth login` will print a URL instead of opening a browser:

```bash
# Inside container
claude auth login
# Output: Open this URL in your browser: https://claude.ai/oauth/...
# Copy URL → Open in host browser → Authenticate → Return to container
```

**Note:** The firewall must allow `claude.ai` and `anthropic.com` domains for OAuth to work. The official devcontainer includes these by default.

#### Credential Persistence

| Mount Type | Behavior | Use Case |
|------------|----------|----------|
| Named volume (`fresher-config`) | Persists across container restarts | API key users, isolated credentials |
| Host bind mount (`~/.claude:ro`) | Uses host credentials | Max/Pro plans, shared auth |

### 4.3 Environment Variables

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `FRESHER_USE_DOCKER` | boolean | `false` | Enable devcontainer mode |
| `FRESHER_IN_DOCKER` | boolean | (auto) | Set inside container |
| `DEVCONTAINER` | boolean | (auto) | Standard devcontainer indicator |
| `FRESHER_AUTH_METHOD` | string | `auto` | Authentication method: `api_key`, `oauth`, or `auto` |

### 4.4 Config.sh Settings

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

    # Interactive mode (required for OAuth URL display)
    stdin_open: true
    tty: true

    # Resource limits
    mem_limit: ${FRESHER_DOCKER_MEMORY:-4g}
    cpus: ${FRESHER_DOCKER_CPUS:-2}

    # Volume mounts - choose ONE authentication approach:
    volumes:
      - ${PWD}:/workspace
      - fresher-bashhistory:/commandhistory
      # OPTION A: API Key - use named volume for isolated credentials
      # - fresher-config:/home/node/.claude
      # OPTION B: OAuth/Max Plan - mount host credentials (recommended)
      - ${HOME}/.claude:/home/node/.claude:ro

    # Environment
    environment:
      - FRESHER_MODE=${FRESHER_MODE:-planning}
      - FRESHER_MAX_ITERATIONS=${FRESHER_MAX_ITERATIONS:-0}
      - FRESHER_IN_DOCKER=true
      - DEVCONTAINER=true
      # For API key users, uncomment:
      # - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}

    # User mapping
    user: node

    # Working directory
    working_dir: /workspace

    # Initialize firewall on start (includes OAuth domains for Max/Pro plans)
    command: >
      bash -c "sudo /usr/local/bin/init-firewall.sh &&
               /workspace/.fresher/docker/fresher-firewall-overlay.sh &&
               /workspace/.fresher/run.sh"

volumes:
  fresher-bashhistory:
  # Only needed for API key users with isolated credentials:
  # fresher-config:
```

> **Note:** The `fresher-firewall-overlay.sh` script adds `claude.ai` to the firewall whitelist, which is required for OAuth authentication. Without it, Max/Pro plans cannot authenticate.

### 5.3 Firewall Customization (Required for OAuth)

The official devcontainer firewall does **not** include `claude.ai`, which is required for OAuth authentication (Max/Pro plans). Create an overlay script to add it:

```bash
#!/bin/bash
# .fresher/docker/fresher-firewall-overlay.sh
# Run AFTER the standard init-firewall.sh

set -e

#──────────────────────────────────────────────────────────────────
# OAuth Domains (REQUIRED for Max/Pro plans)
#──────────────────────────────────────────────────────────────────
OAUTH_DOMAINS=(
  "claude.ai"
  "www.claude.ai"
  "auth.claude.ai"
  "console.anthropic.com"    # May be needed for some auth flows
)

#──────────────────────────────────────────────────────────────────
# Custom Domains (add your own as needed)
#──────────────────────────────────────────────────────────────────
CUSTOM_DOMAINS=(
  # "npm.mycompany.com"       # Private npm registry
  # "api.internal-service.com" # Internal APIs
)

# Combine all domains
ALL_DOMAINS=("${OAUTH_DOMAINS[@]}" "${CUSTOM_DOMAINS[@]}")

echo "Adding custom domains to firewall whitelist..."

for domain in "${ALL_DOMAINS[@]}"; do
  # Skip empty/commented entries
  [[ -z "$domain" || "$domain" == \#* ]] && continue

  ips=$(dig +short A "$domain" 2>/dev/null || true)
  if [[ -z "$ips" ]]; then
    echo "  Warning: Could not resolve $domain"
    continue
  fi

  for ip in $ips; do
    if [[ $ip =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
      sudo ipset add allowed-domains "$ip" 2>/dev/null || true
      echo "  Added $domain ($ip)"
    fi
  done
done

echo "Firewall overlay complete."
```

**Usage:** This script must run after the container starts. Update `postStartCommand` in devcontainer.json:

```json
"postStartCommand": "sudo /usr/local/bin/init-firewall.sh && /workspace/.fresher/docker/fresher-firewall-overlay.sh"
```

Or for docker-compose, update the command:

```yaml
command: >
  bash -c "sudo /usr/local/bin/init-firewall.sh &&
           /workspace/.fresher/docker/fresher-firewall-overlay.sh &&
           /workspace/.fresher/run.sh"
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

### Quick Start (Max/Pro Plans with OAuth)

This is the recommended flow for Claude Max or Pro subscribers:

```bash
# 1. Authenticate on your HOST (not in Docker)
claude auth login
# Browser opens → Log in → Credentials saved to ~/.claude/

# 2. Verify authentication
claude auth status

# 3. Enable Docker in Fresher config
echo 'export FRESHER_USE_DOCKER=true' >> .fresher/config.sh

# 4. Ensure firewall overlay script exists (adds claude.ai to whitelist)
# See Section 5.3 for the full script
cat .fresher/docker/fresher-firewall-overlay.sh  # Verify it exists
chmod +x .fresher/docker/fresher-firewall-overlay.sh

# 5. Run via docker-compose (mounts your host credentials)
docker compose -f .fresher/docker/docker-compose.yml run --rm fresher
```

> **Important:** The official devcontainer does NOT whitelist `claude.ai`. The `fresher-firewall-overlay.sh` script (see Section 5.3) is required for OAuth to work.

### Quick Start (API Key)

For pay-as-you-go or Teams users with API keys:

```bash
# 1. Set your API key
export ANTHROPIC_API_KEY="sk-ant-..."

# 2. Enable Docker in Fresher config
echo 'export FRESHER_USE_DOCKER=true' >> .fresher/config.sh

# 3. Update docker-compose.yml to use API key authentication
# (uncomment ANTHROPIC_API_KEY line, comment out ~/.claude mount)

# 4. Run via docker-compose
docker compose -f .fresher/docker/docker-compose.yml run --rm fresher
```

### Quick Start (VS Code Devcontainer)

```bash
# 1. Authenticate on host first (if using Max/Pro)
claude auth login

# 2. Copy devcontainer config
mkdir -p .devcontainer
cp .fresher/docker/devcontainer.json .devcontainer/

# 3. Open in VS Code
code .
# Then: Cmd+Shift+P → "Remote-Containers: Reopen in Container"

# 4. Run Fresher inside container
fresher build
```

### Re-authenticating in Container

If your OAuth session expires while in the container:

```bash
# Inside container - prints URL instead of opening browser
claude auth login
# Copy the URL → Paste in host browser → Authenticate
# Credentials update in mounted ~/.claude directory
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
- [x] ~~How to support Claude Max/Pro plans without API keys?~~ → Mount host OAuth credentials (see Section 4.2)
- [ ] Should there be a `fresher docker shell` command for debugging?
- [ ] How to handle additional package registries (private npm, cargo, etc.)?
- [ ] Should Fresher auto-detect when devcontainer.json exists in project root?

---

## 11. Troubleshooting

### OAuth Authentication Issues

**Problem:** `claude auth status` shows "Invalid API key" despite successful OAuth login

This is a [known issue](https://github.com/anthropics/claude-code/issues/8002). The status display may be misleading, but if Claude Code operations work, authentication is valid.

**Problem:** Can't authenticate - no browser available in container

Use the URL-based flow:
```bash
claude auth login
# Copy the printed URL to your host browser
# Complete authentication
# Credentials are saved to ~/.claude/
```

**Problem:** OAuth tokens expired during long loop

Re-authenticate from within the container (if `~/.claude` is mounted read-write):
```bash
claude auth login
```

Or re-authenticate on the host and restart the container.

**Problem:** "Permission denied" accessing mounted `~/.claude`

Ensure the container user (`node`) can read the mounted directory:
```bash
# On host, check permissions
ls -la ~/.claude/

# The container runs as uid 1000 (node), so files should be readable
chmod 644 ~/.claude/.credentials.json
```

**Problem:** OAuth authentication fails with network/connection error

The official devcontainer firewall does NOT whitelist `claude.ai`. Ensure the firewall overlay script runs:

```bash
# Check if claude.ai is in the whitelist
docker exec <container> ipset list allowed-domains | grep -i claude

# If not found, run the overlay script manually
docker exec <container> /workspace/.fresher/docker/fresher-firewall-overlay.sh

# Verify claude.ai IPs are now whitelisted
docker exec <container> ipset list allowed-domains
```

**Problem:** Firewall overlay script not found

Ensure the script exists and is executable:
```bash
# Create the script (see Section 5.3 for full contents)
# Then make it executable
chmod +x .fresher/docker/fresher-firewall-overlay.sh
```
