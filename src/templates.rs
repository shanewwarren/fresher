/// Embedded prompt templates for fresher

/// Planning mode prompt template
pub const PROMPT_PLANNING: &str = r#"# Planning Mode

You are analyzing specifications against the current codebase to create an implementation plan.

## Your Task

1. **Read all specifications** in `specs/` directory
2. **Explore the codebase** to understand what exists
3. **Identify gaps** between specs and implementation
4. **Create or update** `IMPLEMENTATION_PLAN.md` with prioritized tasks

## Constraints

- DO NOT implement anything
- DO NOT make commits
- DO NOT modify source code
- ONLY output the implementation plan

## Process

### Step 1: Understand Requirements
Use subagents to read and summarize each spec file in `specs/`.

### Step 2: Analyze Current State
Use subagents to explore `src/` (or equivalent) and document:
- What features are implemented
- What patterns are in use
- What's partially complete

### Step 3: Gap Analysis
For each requirement in specs, determine:
- [ ] Not started
- [ ] Partially implemented
- [ ] Fully implemented

### Step 4: Create Plan
Write `IMPLEMENTATION_PLAN.md` with:

```markdown
# Implementation Plan

Generated: {timestamp}
Based on: specs/*.md

## Priority 1: Critical Path
- [ ] Task description (refs: specs/foo.md)
  - Dependencies: none
  - Complexity: low/medium/high

## Priority 2: Core Features
- [ ] Task description (refs: specs/bar.md)
  - Dependencies: Priority 1 tasks
  - Complexity: medium

## Priority 3: Enhancements
...
```

## Output Format

Your final output should confirm:
1. Which specs were analyzed
2. How many gaps were identified
3. That IMPLEMENTATION_PLAN.md was created/updated

## Important

- Assume specs describe INTENT, not reality
- Always verify against actual code before concluding something is implemented
- Tasks should be small enough to complete in one building iteration
- Include spec references for traceability
"#;

/// Building mode prompt template
pub const PROMPT_BUILDING: &str = r#"# Building Mode

You are implementing tasks from the existing implementation plan.

## Your Task

1. **Read** `IMPLEMENTATION_PLAN.md`
2. **Select** the highest priority incomplete task
3. **Investigate** relevant code (don't assume not implemented)
4. **Implement** the task completely
5. **Validate** with tests and builds
6. **Update** the plan (mark complete, note discoveries)
7. **Commit** changes

## Constraints

- ONE task per iteration
- Must pass all validation before committing
- Update AGENTS.md if you discover operational knowledge

## Process

### Step 1: Read Plan
Open `IMPLEMENTATION_PLAN.md` and identify the first unchecked task (`- [ ]`).

### Step 2: Investigate
Before implementing, use subagents to:
- Read the referenced spec for requirements
- Search for existing related code
- Understand current patterns

**CRITICAL**: Never assume something isn't implemented. Always check first.

### Step 3: Implement
Write the code to complete the task. Follow patterns in AGENTS.md.

### Step 4: Validate
Run the project's validation commands:
- Tests: `{test_command from AGENTS.md}`
- Build: `{build_command from AGENTS.md}`
- Lint: `{lint_command from AGENTS.md}`

If validation fails:
- Fix the issues
- Re-run validation
- Do not proceed until passing

### Step 5: Update Plan
In `IMPLEMENTATION_PLAN.md`:
- Change `- [ ]` to `- [x]` for completed task
- Add notes about any discoveries or issues
- Add new tasks if scope expanded

### Step 6: Commit
Create a commit with:
- Clear message describing the change
- Reference to the spec if applicable

## Output Format

Your final output should confirm:
1. Which task was implemented
2. Validation results (pass/fail)
3. Commit SHA (if successful)

## Important

- Quality over speed - one well-implemented task is better than multiple broken ones
- If stuck on a task, document blockers and move to next
- Update AGENTS.md with any commands, patterns, or knowledge discovered
"#;

/// Default AGENTS.md template
pub const AGENTS_TEMPLATE: &str = r#"# Project: {project_name}

## Commands

### Testing
```bash
{test_command}
```

### Building
```bash
{build_command}
```

### Linting
```bash
{lint_command}
```

## Code Patterns

### File Organization
- Source code: `{src_dir}/`
- Tests: `tests/` or `__tests__/`
- Specifications: `{spec_dir}/`

### Naming Conventions
<!-- Describe your project's naming conventions here -->

### Architecture Notes
<!-- Describe key architectural patterns here -->

## Operational Knowledge

### Known Issues
<!-- Document any gotchas here -->

### Dependencies
<!-- List key dependencies and their purposes here -->

## Fresher Notes

*This section is updated by the Ralph Loop as it learns about the project.*
"#;

/// Default config.toml template
pub const CONFIG_TEMPLATE: &str = r#"# Fresher configuration
# Generated: {timestamp}
# Project type: {project_type}

[fresher]
mode = "planning"
max_iterations = 0
smart_termination = true
dangerous_permissions = true
max_turns = 50
model = "sonnet"

[commands]
test = "{test_command}"
build = "{build_command}"
lint = "{lint_command}"

[paths]
log_dir = ".fresher/logs"
spec_dir = "specs"
src_dir = "{src_dir}"

[hooks]
enabled = true
timeout = 30

[docker]
use_docker = false
memory = "4g"
cpus = "2"
"#;

/// Example hook script for started hook
pub const HOOK_STARTED: &str = r#"#!/bin/bash
# Hook: started
# Runs once when the loop starts
# Exit 0 to continue, exit 2 to abort loop

echo "Fresher loop starting..."

# Example: Check for required tools
# if ! command -v claude &> /dev/null; then
#     echo "Error: claude command not found"
#     exit 2
# fi

exit 0
"#;

/// Example hook script for next_iteration hook
pub const HOOK_NEXT_ITERATION: &str = r#"#!/bin/bash
# Hook: next_iteration
# Runs before each iteration
# Exit 0 to continue, exit 1 to skip iteration, exit 2 to abort loop

echo "Starting iteration $FRESHER_ITERATION..."

# Example: Skip iteration if disk space is low
# available=$(df -k . | tail -1 | awk '{print $4}')
# if [ "$available" -lt 1048576 ]; then
#     echo "Warning: Low disk space, skipping iteration"
#     exit 1
# fi

exit 0
"#;

/// Example hook script for finished hook
pub const HOOK_FINISHED: &str = r#"#!/bin/bash
# Hook: finished
# Runs when the loop ends (any exit condition)
# Environment variables available:
#   FRESHER_TOTAL_ITERATIONS - Total iterations completed
#   FRESHER_TOTAL_COMMITS - Total commits made
#   FRESHER_DURATION - Total duration in seconds
#   FRESHER_FINISH_TYPE - How loop ended (manual, error, max_iterations, complete, no_changes)

echo "Fresher loop finished"
echo "  Iterations: $FRESHER_TOTAL_ITERATIONS"
echo "  Commits: $FRESHER_TOTAL_COMMITS"
echo "  Duration: ${FRESHER_DURATION}s"
echo "  Finish type: $FRESHER_FINISH_TYPE"

# Example: Send notification
# curl -X POST "https://slack.webhook/..." -d "{\"text\": \"Fresher completed: $FRESHER_FINISH_TYPE\"}"

exit 0
"#;

/// Docker compose template
pub const DOCKER_COMPOSE_TEMPLATE: &str = r#"# Fresher Docker Compose Configuration
# For CLI-only workflow (without VS Code devcontainers)
#
# Usage:
#   docker compose -f .fresher/docker/docker-compose.yml run --rm fresher

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

    # Initialize firewall and run fresher
    command: >
      bash -c "sudo /usr/local/bin/init-firewall.sh &&
               /workspace/.fresher/docker/fresher-firewall-overlay.sh 2>/dev/null || true &&
               /workspace/.fresher/run.sh"

volumes:
  fresher-bashhistory:
  # Only needed for API key users with isolated credentials:
  # fresher-config:
"#;

/// Devcontainer.json template
pub const DEVCONTAINER_TEMPLATE: &str = r#"{
  "name": "Fresher Loop Environment",
  "image": "ghcr.io/anthropics/claude-code-devcontainer:latest",
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
  "postStartCommand": "sudo /usr/local/bin/init-firewall.sh && /workspace/.fresher/docker/fresher-firewall-overlay.sh 2>/dev/null || true",
  "waitFor": "postStartCommand"
}
"#;

/// Firewall overlay script template
pub const FIREWALL_OVERLAY_TEMPLATE: &str = r#"#!/bin/bash
# Fresher Firewall Overlay Script
# Run AFTER the standard init-firewall.sh
#
# This script adds custom domains to the firewall whitelist.
# The official devcontainer does NOT whitelist claude.ai, which is
# required for OAuth authentication (Max/Pro plans).

set -e

#──────────────────────────────────────────────────────────────────
# OAuth Domains (REQUIRED for Max/Pro plans)
#──────────────────────────────────────────────────────────────────
OAUTH_DOMAINS=(
  "claude.ai"
  "www.claude.ai"
  "auth.claude.ai"
  "console.anthropic.com"
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
"#;

/// Run script template (entry point for Docker)
pub const RUN_SCRIPT_TEMPLATE: &str = r#"#!/bin/bash
# Fresher Run Script
# Entry point for Docker execution

set -e

# Determine mode from environment
MODE="${FRESHER_MODE:-planning}"

echo "Starting fresher in $MODE mode..."

# Check if fresher is installed
if ! command -v fresher &> /dev/null; then
  echo "Error: fresher command not found"
  echo "Please install fresher or ensure it's in your PATH"
  exit 1
fi

# Run the appropriate mode
case "$MODE" in
  planning|plan)
    exec fresher plan
    ;;
  building|build)
    exec fresher build
    ;;
  *)
    echo "Unknown mode: $MODE"
    echo "Valid modes: planning, building"
    exit 1
    ;;
esac
"#;
