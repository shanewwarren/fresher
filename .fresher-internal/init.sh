#!/bin/bash
# Fresher initialization script
# Creates the .fresher/ directory structure with detected project settings

set -e

#──────────────────────────────────────────────────────────────────
# Colors
#──────────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

#──────────────────────────────────────────────────────────────────
# Parse arguments
#──────────────────────────────────────────────────────────────────
FORCE=false
INTERACTIVE=false
NO_HOOKS=false
NO_DOCKER=false
PROJECT_TYPE=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --force|-f)
      FORCE=true
      shift
      ;;
    --interactive|-i)
      INTERACTIVE=true
      shift
      ;;
    --no-hooks)
      NO_HOOKS=true
      shift
      ;;
    --no-docker)
      NO_DOCKER=true
      shift
      ;;
    --project-type)
      shift
      PROJECT_TYPE="$1"
      shift
      ;;
    --help|-h)
      echo "Usage: fresher init [options]"
      echo ""
      echo "Options:"
      echo "  --interactive, -i     Run interactive setup wizard"
      echo "  --force, -f           Overwrite existing .fresher/ without prompting"
      echo "  --no-hooks            Skip creating hook scripts"
      echo "  --no-docker           Skip Docker-related config entries"
      echo "  --project-type TYPE   Override auto-detected project type"
      echo "  --help, -h            Show this help message"
      exit 0
      ;;
    *)
      echo "Unknown option: $1"
      exit 1
      ;;
  esac
done

#──────────────────────────────────────────────────────────────────
# Check for existing .fresher/
#──────────────────────────────────────────────────────────────────
if [[ -d ".fresher" ]]; then
  if [[ "$FORCE" == "true" ]]; then
    echo -e "${YELLOW}Removing existing .fresher/ directory...${NC}"
    rm -rf .fresher
  else
    echo -e "${RED}Error: .fresher/ directory already exists${NC}"
    echo "Use --force to overwrite, or remove it manually"
    exit 1
  fi
fi

#──────────────────────────────────────────────────────────────────
# Project type detection
#──────────────────────────────────────────────────────────────────
detect_project_type() {
  local dir="${1:-.}"

  if [[ -f "$dir/package.json" ]]; then
    # Check for bun.lockb to determine if using bun
    if [[ -f "$dir/bun.lockb" ]]; then
      echo "bun"
    else
      echo "nodejs"
    fi
  elif [[ -f "$dir/Cargo.toml" ]]; then
    echo "rust"
  elif [[ -f "$dir/go.mod" ]]; then
    echo "go"
  elif [[ -f "$dir/pyproject.toml" ]]; then
    echo "python"
  elif [[ -f "$dir/requirements.txt" ]]; then
    echo "python"
  elif [[ -f "$dir/Makefile" ]]; then
    echo "make"
  elif ls "$dir"/*.csproj &>/dev/null 2>&1; then
    echo "dotnet"
  elif [[ -f "$dir/pom.xml" ]]; then
    echo "maven"
  elif [[ -f "$dir/build.gradle" ]]; then
    echo "gradle"
  else
    echo "generic"
  fi
}

get_test_command() {
  local type="$1"
  case "$type" in
    nodejs|bun) echo "bun test" ;;
    rust) echo "cargo test" ;;
    go) echo "go test ./..." ;;
    python) echo "pytest" ;;
    make) echo "make test" ;;
    dotnet) echo "dotnet test" ;;
    maven) echo "mvn test" ;;
    gradle) echo "./gradlew test" ;;
    *) echo "" ;;
  esac
}

get_build_command() {
  local type="$1"
  case "$type" in
    nodejs|bun) echo "bun run build" ;;
    rust) echo "cargo build" ;;
    go) echo "go build ./..." ;;
    python) echo "python -m build" ;;
    make) echo "make" ;;
    dotnet) echo "dotnet build" ;;
    maven) echo "mvn package" ;;
    gradle) echo "./gradlew build" ;;
    *) echo "" ;;
  esac
}

get_lint_command() {
  local type="$1"
  case "$type" in
    nodejs|bun) echo "bun run lint" ;;
    rust) echo "cargo clippy" ;;
    go) echo "golangci-lint run" ;;
    python) echo "ruff check" ;;
    *) echo "" ;;
  esac
}

get_src_dir() {
  local type="$1"
  case "$type" in
    nodejs|bun) echo "src" ;;
    rust) echo "src" ;;
    go) echo "." ;;
    python) echo "src" ;;
    *) echo "src" ;;
  esac
}

# Detect or use provided project type
if [[ -n "$PROJECT_TYPE" ]]; then
  DETECTED_TYPE="$PROJECT_TYPE"
else
  DETECTED_TYPE=$(detect_project_type)
fi

TEST_CMD=$(get_test_command "$DETECTED_TYPE")
BUILD_CMD=$(get_build_command "$DETECTED_TYPE")
LINT_CMD=$(get_lint_command "$DETECTED_TYPE")
SRC_DIR=$(get_src_dir "$DETECTED_TYPE")
PROJECT_NAME=$(basename "$(pwd)")
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

# Default values for Docker and max iterations
USE_DOCKER=false
MAX_ITERATIONS=0

#──────────────────────────────────────────────────────────────────
# Interactive wizard
#──────────────────────────────────────────────────────────────────
if [[ "$INTERACTIVE" == "true" ]]; then
  echo ""
  echo -e "${BLUE}╔════════════════════════════════════════════════════════════════╗${NC}"
  echo -e "${BLUE}║                    Fresher Setup Wizard                        ║${NC}"
  echo -e "${BLUE}╚════════════════════════════════════════════════════════════════╝${NC}"
  echo ""
  echo -e "Detected project type: ${GREEN}$DETECTED_TYPE${NC}"
  echo ""
  echo "Press Enter to accept the default value shown in brackets."
  echo ""

  # Test command
  read -p "? Test command [$TEST_CMD]: " input
  [[ -n "$input" ]] && TEST_CMD="$input"

  # Build command
  read -p "? Build command [$BUILD_CMD]: " input
  [[ -n "$input" ]] && BUILD_CMD="$input"

  # Lint command
  read -p "? Lint command [$LINT_CMD]: " input
  [[ -n "$input" ]] && LINT_CMD="$input"

  # Source directory
  read -p "? Source directory [$SRC_DIR]: " input
  [[ -n "$input" ]] && SRC_DIR="$input"

  # Docker isolation
  if [[ "$NO_DOCKER" != "true" ]]; then
    read -p "? Enable Docker isolation? [y/N]: " input
    if [[ "$input" =~ ^[Yy]$ ]]; then
      USE_DOCKER=true
    fi
  fi

  # Max iterations
  read -p "? Max iterations (0=unlimited) [$MAX_ITERATIONS]: " input
  [[ -n "$input" ]] && MAX_ITERATIONS="$input"

  echo ""
fi

echo -e "${BLUE}Initializing Fresher...${NC}"
echo "Project: $PROJECT_NAME"
echo "Detected type: $DETECTED_TYPE"
echo ""

#──────────────────────────────────────────────────────────────────
# Create directory structure
#──────────────────────────────────────────────────────────────────
echo "Creating .fresher/ directory structure..."

if [[ "$NO_HOOKS" != "true" ]]; then
  mkdir -p .fresher/hooks
fi
mkdir -p .fresher/lib
mkdir -p .fresher/logs

#──────────────────────────────────────────────────────────────────
# Generate config.sh
#──────────────────────────────────────────────────────────────────
cat > .fresher/config.sh << EOF
#!/bin/bash
# Fresher configuration for $PROJECT_NAME
# Generated: $TIMESTAMP
# Project type: $DETECTED_TYPE

#──────────────────────────────────────────────────────────────────
# Mode Configuration
#──────────────────────────────────────────────────────────────────
export FRESHER_MODE="\${FRESHER_MODE:-planning}"

#──────────────────────────────────────────────────────────────────
# Termination Settings
#──────────────────────────────────────────────────────────────────
export FRESHER_MAX_ITERATIONS="\${FRESHER_MAX_ITERATIONS:-$MAX_ITERATIONS}"
export FRESHER_SMART_TERMINATION="\${FRESHER_SMART_TERMINATION:-true}"

#──────────────────────────────────────────────────────────────────
# Claude Code Settings
#──────────────────────────────────────────────────────────────────
export FRESHER_DANGEROUS_PERMISSIONS="\${FRESHER_DANGEROUS_PERMISSIONS:-true}"
export FRESHER_MAX_TURNS="\${FRESHER_MAX_TURNS:-50}"
export FRESHER_MODEL="\${FRESHER_MODEL:-sonnet}"

#──────────────────────────────────────────────────────────────────
# Project Commands (detected: $DETECTED_TYPE)
#──────────────────────────────────────────────────────────────────
export FRESHER_TEST_CMD="\${FRESHER_TEST_CMD:-$TEST_CMD}"
export FRESHER_BUILD_CMD="\${FRESHER_BUILD_CMD:-$BUILD_CMD}"
export FRESHER_LINT_CMD="\${FRESHER_LINT_CMD:-$LINT_CMD}"

#──────────────────────────────────────────────────────────────────
# Paths
#──────────────────────────────────────────────────────────────────
export FRESHER_LOG_DIR="\${FRESHER_LOG_DIR:-.fresher/logs}"
export FRESHER_SPEC_DIR="\${FRESHER_SPEC_DIR:-specs}"
export FRESHER_SRC_DIR="\${FRESHER_SRC_DIR:-$SRC_DIR}"

#──────────────────────────────────────────────────────────────────
# Hook Settings
#──────────────────────────────────────────────────────────────────
export FRESHER_HOOK_TIMEOUT="\${FRESHER_HOOK_TIMEOUT:-30}"
export FRESHER_HOOKS_ENABLED="\${FRESHER_HOOKS_ENABLED:-true}"
EOF

# Add Docker config section unless --no-docker was specified
if [[ "$NO_DOCKER" != "true" ]]; then
  cat >> .fresher/config.sh << EOF

#──────────────────────────────────────────────────────────────────
# Docker/Devcontainer Configuration
#──────────────────────────────────────────────────────────────────
export FRESHER_USE_DOCKER="\${FRESHER_USE_DOCKER:-$USE_DOCKER}"

# Resource limits (passed to devcontainer)
export FRESHER_DOCKER_MEMORY="\${FRESHER_DOCKER_MEMORY:-4g}"
export FRESHER_DOCKER_CPUS="\${FRESHER_DOCKER_CPUS:-2}"
EOF
fi

echo -e "${GREEN}✓${NC} Created config.sh"

#──────────────────────────────────────────────────────────────────
# Generate run.sh (stub)
#──────────────────────────────────────────────────────────────────
cat > .fresher/run.sh << 'EOF'
#!/bin/bash
# Fresher Loop Executor
# This is a stub - full implementation coming in loop-executor phase

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Load configuration
source "$SCRIPT_DIR/config.sh"

echo "Fresher Loop Executor"
echo "====================="
echo "Mode: $FRESHER_MODE"
echo ""
echo "ERROR: Loop executor not yet implemented"
echo "This stub will be replaced with the full implementation."
exit 1
EOF
chmod +x .fresher/run.sh

echo -e "${GREEN}✓${NC} Created run.sh (stub)"

#──────────────────────────────────────────────────────────────────
# Generate AGENTS.md
#──────────────────────────────────────────────────────────────────
cat > .fresher/AGENTS.md << EOF
# Project: $PROJECT_NAME

## Commands

### Testing
\`\`\`bash
$TEST_CMD
\`\`\`

### Building
\`\`\`bash
$BUILD_CMD
\`\`\`

### Linting
\`\`\`bash
$LINT_CMD
\`\`\`

## Code Patterns

### File Organization
- Source code: \`$SRC_DIR/\`
- Tests: \`tests/\` or \`__tests__/\`
- Specifications: \`specs/\`

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
EOF

echo -e "${GREEN}✓${NC} Created AGENTS.md"

#──────────────────────────────────────────────────────────────────
# Generate prompt templates
#──────────────────────────────────────────────────────────────────
cat > .fresher/PROMPT.planning.md << 'EOF'
# Planning Mode

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
EOF

cat > .fresher/PROMPT.building.md << 'EOF'
# Building Mode

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
EOF

echo -e "${GREEN}✓${NC} Created PROMPT templates"

#──────────────────────────────────────────────────────────────────
# Generate hook scripts
#──────────────────────────────────────────────────────────────────
if [[ "$NO_HOOKS" != "true" ]]; then
  cat > .fresher/hooks/started << 'EOF'
#!/bin/bash
# .fresher/hooks/started
# Runs once when the Ralph loop begins

echo "Starting Fresher loop in $FRESHER_MODE mode"
echo "Project: ${FRESHER_PROJECT_DIR:-$(pwd)}"
echo "Max iterations: ${FRESHER_MAX_ITERATIONS:-unlimited}"

# Example: Check prerequisites
if [[ ! -f "IMPLEMENTATION_PLAN.md" ]] && [[ "$FRESHER_MODE" == "building" ]]; then
  echo "ERROR: No IMPLEMENTATION_PLAN.md found. Run planning mode first."
  exit 2  # Abort
fi

# Example: Check for uncommitted changes
if ! git diff --quiet 2>/dev/null; then
  echo "WARNING: You have uncommitted changes"
fi

# Example: Send notification (uncomment to enable)
# curl -s -X POST "$SLACK_WEBHOOK" -d "{\"text\": \"Fresher started: $FRESHER_MODE mode\"}"

exit 0
EOF
  chmod +x .fresher/hooks/started

  cat > .fresher/hooks/next_iteration << 'EOF'
#!/bin/bash
# .fresher/hooks/next_iteration
# Runs before each iteration

echo "Starting iteration ${FRESHER_ITERATION:-1}"

if [[ ${FRESHER_ITERATION:-1} -gt 1 ]]; then
  echo "Previous iteration: exit=${FRESHER_LAST_EXIT_CODE:-0}, duration=${FRESHER_LAST_DURATION:-0}s"
  echo "Commits so far: ${FRESHER_COMMITS_MADE:-0}"
fi

# Example: Skip if no changes needed (building mode only)
# if [[ "$FRESHER_MODE" == "building" ]]; then
#   pending=$(grep -c '^\s*-\s*\[\s\]' IMPLEMENTATION_PLAN.md 2>/dev/null || echo "0")
#   if [[ "$pending" -eq 0 ]]; then
#     echo "No pending tasks, skipping iteration"
#     exit 1  # Skip
#   fi
# fi

# Example: Desktop notification (macOS)
# terminal-notifier -title "Fresher" -message "Starting iteration $FRESHER_ITERATION" 2>/dev/null || true

# Example: Desktop notification (Linux)
# notify-send "Fresher" "Starting iteration $FRESHER_ITERATION" 2>/dev/null || true

exit 0
EOF
  chmod +x .fresher/hooks/next_iteration

  cat > .fresher/hooks/finished << 'EOF'
#!/bin/bash
# .fresher/hooks/finished
# Runs when the loop ends

echo ""
echo "════════════════════════════════════════"
echo "Fresher loop finished"
echo "════════════════════════════════════════"
echo "Finish type: ${FRESHER_FINISH_TYPE:-unknown}"
echo "Total iterations: ${FRESHER_TOTAL_ITERATIONS:-0}"
echo "Total commits: ${FRESHER_TOTAL_COMMITS:-0}"
echo "Duration: ${FRESHER_DURATION:-0}s"
echo "════════════════════════════════════════"

# Determine summary message
case "${FRESHER_FINISH_TYPE:-unknown}" in
  complete)
    message="All tasks completed successfully!"
    ;;
  manual)
    message="Loop stopped by user"
    ;;
  max_iterations)
    message="Reached max iterations limit"
    ;;
  no_changes)
    message="No changes made in last iteration"
    ;;
  error)
    message="Loop stopped due to error"
    ;;
  hook_abort)
    message="Loop aborted by hook"
    ;;
  *)
    message="Loop finished: ${FRESHER_FINISH_TYPE:-unknown}"
    ;;
esac

echo "$message"

# Example: Desktop notification (macOS)
# terminal-notifier -title "Fresher Complete" -message "$message" 2>/dev/null || true

# Example: Desktop notification (Linux)
# notify-send "Fresher Complete" "$message" 2>/dev/null || true

# Example: Slack/Discord webhook (uncomment to enable)
# curl -s -X POST "$SLACK_WEBHOOK" \
#   -H "Content-Type: application/json" \
#   -d "{\"text\": \"Fresher: $message\nIterations: $FRESHER_TOTAL_ITERATIONS\nCommits: $FRESHER_TOTAL_COMMITS\"}"

exit 0
EOF
  chmod +x .fresher/hooks/finished

  echo -e "${GREEN}✓${NC} Created hook scripts"
fi

#──────────────────────────────────────────────────────────────────
# Generate Docker files
#──────────────────────────────────────────────────────────────────
if [[ "$NO_DOCKER" != "true" ]]; then
  mkdir -p .fresher/docker

  # Generate devcontainer.json
  cat > .fresher/docker/devcontainer.json << 'EOF'
{
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
  "postStartCommand": "sudo /usr/local/bin/init-firewall.sh",
  "waitFor": "postStartCommand"
}
EOF

  # Generate docker-compose.yml
  cat > .fresher/docker/docker-compose.yml << 'EOF'
# Fresher Docker Compose Configuration
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

    # Initialize firewall on start, then run Fresher
    command: >
      bash -c "sudo /usr/local/bin/init-firewall.sh && /workspace/.fresher/run.sh"

volumes:
  fresher-bashhistory:
  fresher-config:
EOF

  # Generate firewall overlay script template
  cat > .fresher/docker/fresher-firewall-overlay.sh << 'EOF'
#!/bin/bash
# .fresher/docker/fresher-firewall-overlay.sh
#
# Add custom domains to the devcontainer firewall whitelist.
# Run AFTER the standard init-firewall.sh has initialized the firewall.
#
# Usage:
#   1. Uncomment and modify the CUSTOM_DOMAINS array below
#   2. Add this script to postStartCommand in devcontainer.json:
#      "postStartCommand": "sudo /usr/local/bin/init-firewall.sh && ./.fresher/docker/fresher-firewall-overlay.sh"
#
# Common use cases:
#   - Private npm registry (e.g., npm.mycompany.com)
#   - Internal APIs (e.g., api.internal.mycompany.com)
#   - Private package registries (e.g., packages.mycompany.com)
#   - Internal git servers (e.g., git.mycompany.com)

# Uncomment and customize this array with your domains
# CUSTOM_DOMAINS=(
#   "npm.mycompany.com"
#   "api.internal-service.com"
#   "packages.mycompany.com"
# )

# Exit if CUSTOM_DOMAINS is not defined or empty
if [[ -z "${CUSTOM_DOMAINS[*]}" ]]; then
  echo "No custom domains configured. Edit this file to add domains."
  exit 0
fi

echo "Adding custom domains to firewall whitelist..."

for domain in "${CUSTOM_DOMAINS[@]}"; do
  # Resolve domain to IP addresses
  ips=$(dig +short A "$domain" 2>/dev/null)

  if [[ -z "$ips" ]]; then
    echo "Warning: Could not resolve $domain"
    continue
  fi

  for ip in $ips; do
    # Validate IP address format
    if [[ $ip =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
      sudo ipset add allowed-domains "$ip" 2>/dev/null || true
      echo "Added $domain ($ip) to whitelist"
    fi
  done
done

echo "Firewall overlay complete."
EOF
  chmod +x .fresher/docker/fresher-firewall-overlay.sh

  echo -e "${GREEN}✓${NC} Created Docker configuration files"
fi

#──────────────────────────────────────────────────────────────────
# Create .gitkeep files and bin directory
#──────────────────────────────────────────────────────────────────
touch .fresher/lib/.gitkeep
touch .fresher/logs/.gitkeep
mkdir -p .fresher/bin

echo -e "${GREEN}✓${NC} Created lib/, logs/, and bin/ directories"

#──────────────────────────────────────────────────────────────────
# Generate verify.sh library
#──────────────────────────────────────────────────────────────────
cat > .fresher/lib/verify.sh << 'VERIFY_LIB_EOF'
#!/bin/bash
# Fresher Plan Verification Library
# Functions for verifying plan coverage against specs

set -e

VERIFY_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VERIFY_PROJECT_DIR="$(cd "$VERIFY_SCRIPT_DIR/../.." && pwd)"

verify_log() { echo "[verify] $*"; }
verify_error() { echo "[verify] ERROR: $*" >&2; }

# Extract requirements from spec files
extract_requirements() {
  local spec_dir="${1:-$VERIFY_PROJECT_DIR/specs}"
  [[ ! -d "$spec_dir" ]] && { verify_error "Spec directory not found: $spec_dir"; return 1; }
  find "$spec_dir" -name "*.md" -type f | sort | while read -r spec_file; do
    local spec_name=$(basename "$spec_file" .md)
    grep -nE '^### ' "$spec_file" 2>/dev/null | while IFS=: read -r ln line; do
      echo "${spec_name}|section|${ln}|$(echo "$line" | sed 's/^### *//')"
    done
    grep -nE '^\s*-\s*\[[ xX]\]' "$spec_file" 2>/dev/null | while IFS=: read -r ln line; do
      local status="pending"; [[ $line =~ \[[xX]\] ]] && status="completed"
      echo "${spec_name}|task|${status}|${ln}|$(echo "$line" | sed 's/^[[:space:]]*-[[:space:]]*\[[xX ]\][[:space:]]*//')"
    done
  done
}

# Parse tasks from plan
parse_plan() {
  local plan_file="${1:-$VERIFY_PROJECT_DIR/IMPLEMENTATION_PLAN.md}"
  [[ ! -f "$plan_file" ]] && { verify_error "Plan file not found: $plan_file"; return 1; }
  grep -nE '^\s*-\s*\[[ xX]\]' "$plan_file" 2>/dev/null | while IFS=: read -r ln line; do
    local status="pending"; [[ $line =~ \[[xX]\] ]] && status="completed"
    local desc=$(echo "$line" | sed 's/^[[:space:]]*-[[:space:]]*\[[xX ]\][[:space:]]*//' | sed 's/[[:space:]]*(refs:[^)]*)$//')
    local spec_ref="none"
    [[ $line =~ refs:[[:space:]]*([a-zA-Z0-9_/.-]+\.md) ]] && spec_ref=$(echo "${BASH_REMATCH[1]}" | sed 's|^specs/||')
    echo "${status}|${spec_ref}|${ln}|${desc}"
  done
}

list_specs() {
  local spec_dir="${1:-$VERIFY_PROJECT_DIR/specs}"
  [[ ! -d "$spec_dir" ]] && return 1
  find "$spec_dir" -name "*.md" -type f | sort | while read -r f; do basename "$f" .md; done
}

get_spec_requirements() { extract_requirements "${2:-$VERIFY_PROJECT_DIR/specs}" | grep "^${1}|"; }
get_tasks_for_spec() { parse_plan "${2:-$VERIFY_PROJECT_DIR/IMPLEMENTATION_PLAN.md}" | grep "|${1%.md}.md|" || true; }
get_orphan_tasks() { parse_plan "${1:-$VERIFY_PROJECT_DIR/IMPLEMENTATION_PLAN.md}" | grep '|none|' || true; }

count_requirements() {
  local reqs=$(extract_requirements "${1:-$VERIFY_PROJECT_DIR/specs}")
  echo "total:$(echo "$reqs" | grep -c . 2>/dev/null || echo 0)"
  echo "sections:$(echo "$reqs" | grep -c '|section|' 2>/dev/null || echo 0)"
}

count_plan_tasks() {
  local tasks=$(parse_plan "${1:-$VERIFY_PROJECT_DIR/IMPLEMENTATION_PLAN.md}") || return 1
  local total=$(echo "$tasks" | grep -c . 2>/dev/null) || total=0
  local pending=$(echo "$tasks" | grep -c '^pending|' 2>/dev/null) || pending=0
  local completed=$(echo "$tasks" | grep -c '^completed|' 2>/dev/null) || completed=0
  local orphans=$(echo "$tasks" | grep -c '|none|' 2>/dev/null) || orphans=0
  echo "total:$total"; echo "pending:$pending"; echo "completed:$completed"; echo "orphans:$orphans"
}

analyze_coverage() {
  local spec_dir="${1:-$VERIFY_PROJECT_DIR/specs}"
  local plan_file="${2:-$VERIFY_PROJECT_DIR/IMPLEMENTATION_PLAN.md}"
  list_specs "$spec_dir" | while read -r spec_name; do
    local req_count=$(get_spec_requirements "$spec_name" "$spec_dir" | grep -c '|section|' 2>/dev/null) || req_count=0
    local task_count=$(get_tasks_for_spec "$spec_name" "$plan_file" | grep -c . 2>/dev/null) || task_count=0
    local coverage=0; [[ $req_count -gt 0 ]] && coverage=$((task_count * 100 / req_count))
    [[ $coverage -gt 100 ]] && coverage=100
    echo "${spec_name}|${req_count}|${task_count}|${coverage}"
  done
}

find_uncovered_specs() {
  analyze_coverage "${1:-$VERIFY_PROJECT_DIR/specs}" "${2:-$VERIFY_PROJECT_DIR/IMPLEMENTATION_PLAN.md}" | while IFS='|' read -r spec req task cov; do
    [[ "$task" -eq 0 && "$req" -gt 0 ]] && echo "$spec"
  done
}

get_verification_summary() {
  local spec_dir="${1:-$VERIFY_PROJECT_DIR/specs}"
  local plan_file="${2:-$VERIFY_PROJECT_DIR/IMPLEMENTATION_PLAN.md}"
  local req_stats=$(count_requirements "$spec_dir")
  local task_stats=$(count_plan_tasks "$plan_file")
  echo "$req_stats" | grep '^total:' | sed 's/^total:/total_requirements:/'
  echo "$req_stats" | grep '^sections:' | sed 's/^sections:/section_requirements:/'
  echo "$task_stats" | grep '^total:' | sed 's/^total:/total_tasks:/'
  echo "$task_stats" | grep '^pending:'
  echo "$task_stats" | grep '^completed:' | sed 's/^completed:/completed_tasks:/'
  echo "$task_stats" | grep '^orphans:' | sed 's/^orphans:/orphan_tasks:/'
  local total_specs=$(list_specs "$spec_dir" | grep -c . 2>/dev/null) || total_specs=0
  local covered=$(analyze_coverage "$spec_dir" "$plan_file" | awk -F'|' '$3 > 0 {c++} END {print c+0}')
  echo "total_specs:$total_specs"; echo "covered_specs:$covered"
}

generate_report() {
  local spec_dir="${1:-$VERIFY_PROJECT_DIR/specs}"
  local plan_file="${2:-$VERIFY_PROJECT_DIR/IMPLEMENTATION_PLAN.md}"
  local timestamp=$(date +%Y-%m-%dT%H:%M:%SZ)
  echo "# Verification Report"
  echo ""; echo "Generated: ${timestamp}"; echo ""
  echo "## Summary"; echo ""
  local summary=$(get_verification_summary "$spec_dir" "$plan_file")
  echo "| Metric | Count |"; echo "|--------|-------|"
  echo "| Total requirements | $(echo "$summary" | grep '^total_requirements:' | cut -d: -f2) |"
  echo "| Plan tasks | $(echo "$summary" | grep '^total_tasks:' | cut -d: -f2) |"
  echo "| Completed | $(echo "$summary" | grep '^completed_tasks:' | cut -d: -f2) |"
  echo "| Pending | $(echo "$summary" | grep '^pending_tasks:' | cut -d: -f2) |"
  echo ""; echo "## Coverage by Spec"; echo ""
  echo "| Spec | Sections | Tasks | Coverage |"; echo "|------|----------|-------|----------|"
  analyze_coverage "$spec_dir" "$plan_file" | while IFS='|' read -r s r t c; do
    [[ "$s" == "README" ]] && continue
    echo "| ${s}.md | $r | $t | ${c}% |"
  done
  echo ""; echo "## Missing Coverage"; echo ""
  local uncovered=$(find_uncovered_specs "$spec_dir" "$plan_file")
  if [[ -n "$uncovered" ]]; then
    echo "$uncovered" | while read -r s; do echo "- specs/${s}.md"; done
  else
    echo "_All specs have implementation tasks._"
  fi
  echo ""
}
VERIFY_LIB_EOF

echo -e "${GREEN}✓${NC} Created lib/verify.sh"

#──────────────────────────────────────────────────────────────────
# Generate fresher-verify CLI
#──────────────────────────────────────────────────────────────────
cat > .fresher/bin/fresher-verify << 'VERIFY_CLI_EOF'
#!/bin/bash
set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/verify.sh"

SPEC_DIR="$VERIFY_PROJECT_DIR/specs"
PLAN_FILE="$VERIFY_PROJECT_DIR/IMPLEMENTATION_PLAN.md"
OUTPUT_FILE="$VERIFY_PROJECT_DIR/VERIFICATION_REPORT.md"
QUIET=false; STRICT=false; FORMAT="markdown"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --spec-dir) SPEC_DIR="$2"; shift 2;;
    --plan-file) PLAN_FILE="$2"; shift 2;;
    --output) OUTPUT_FILE="$2"; shift 2;;
    --format) FORMAT="$2"; shift 2;;
    --quiet|-q) QUIET=true; shift;;
    --strict) STRICT=true; shift;;
    --help|-h)
      echo "Usage: fresher verify [options]"
      echo "  --quiet, -q   Summary only"
      echo "  --strict      Exit 1 if issues"
      echo "  --format FMT  markdown|json"
      exit 0;;
    *) echo "Unknown: $1" >&2; exit 2;;
  esac
done

[[ ! -d "$SPEC_DIR" ]] && { echo "Error: Spec dir not found: $SPEC_DIR" >&2; exit 2; }
[[ ! -f "$PLAN_FILE" ]] && { echo "Error: Plan not found: $PLAN_FILE" >&2; exit 2; }

summary=$(get_verification_summary "$SPEC_DIR" "$PLAN_FILE")
orphans=$(echo "$summary" | grep '^orphan_tasks:' | cut -d: -f2)
total_specs=$(echo "$summary" | grep '^total_specs:' | cut -d: -f2)
covered=$(echo "$summary" | grep '^covered_specs:' | cut -d: -f2)
uncovered=$((total_specs - covered - 1)); [[ $uncovered -lt 0 ]] && uncovered=0
has_issues=false; [[ "$orphans" -gt 0 || "$uncovered" -gt 0 ]] && has_issues=true

if [[ "$QUIET" == "true" ]]; then
  if [[ "$FORMAT" == "json" ]]; then
    echo "{\"total_tasks\":$(echo "$summary"|grep '^total_tasks:'|cut -d: -f2),\"has_issues\":$has_issues}"
  else
    echo "Tasks: $(echo "$summary"|grep '^total_tasks:'|cut -d: -f2)"
    echo "Status: $([[ "$has_issues" == "true" ]] && echo "ISSUES FOUND" || echo "OK")"
  fi
else
  generate_report "$SPEC_DIR" "$PLAN_FILE" | tee "$OUTPUT_FILE"
  echo ""; echo "Report: $OUTPUT_FILE"
fi

[[ "$STRICT" == "true" && "$has_issues" == "true" ]] && exit 1
exit 0
VERIFY_CLI_EOF
chmod +x .fresher/bin/fresher-verify

echo -e "${GREEN}✓${NC} Created bin/fresher-verify"

#──────────────────────────────────────────────────────────────────
# Update .gitignore
#──────────────────────────────────────────────────────────────────
GITIGNORE_ENTRIES="
# Fresher
.fresher/logs/
.fresher/.state
"

if [[ -f ".gitignore" ]]; then
  if ! grep -q ".fresher/logs/" .gitignore 2>/dev/null; then
    echo "$GITIGNORE_ENTRIES" >> .gitignore
    echo -e "${GREEN}✓${NC} Updated .gitignore"
  else
    echo -e "${YELLOW}○${NC} .gitignore already has Fresher entries"
  fi
else
  echo "$GITIGNORE_ENTRIES" > .gitignore
  echo -e "${GREEN}✓${NC} Created .gitignore"
fi

#──────────────────────────────────────────────────────────────────
# CLAUDE.md integration
#──────────────────────────────────────────────────────────────────
SPECS_SECTION='## Specifications

**IMPORTANT:** Before implementing any feature, consult `specs/README.md`.

- **Assume NOT implemented.** Specs describe intent; code describes reality.
- **Check the codebase first.** Search actual code before concluding.
- **Use specs as guidance.** Follow design patterns in relevant spec.
- **Spec index:** `specs/README.md` lists all specs by category.'

if [[ ! -f "CLAUDE.md" ]]; then
  # Create minimal CLAUDE.md
  cat > CLAUDE.md << EOF
# Project Guidelines

$SPECS_SECTION
EOF
  echo -e "${GREEN}✓${NC} Created CLAUDE.md"
elif ! grep -q "## Specifications" CLAUDE.md 2>/dev/null; then
  # Inject specs section after first heading or at end
  # Find the line number of the first heading
  first_heading_line=$(grep -n "^#" CLAUDE.md | head -1 | cut -d: -f1)

  if [[ -n "$first_heading_line" ]]; then
    # Find the next heading after the first one
    next_heading_line=$(tail -n +$((first_heading_line + 1)) CLAUDE.md | grep -n "^#" | head -1 | cut -d: -f1)

    if [[ -n "$next_heading_line" ]]; then
      # Insert before the next heading
      insert_line=$((first_heading_line + next_heading_line))
      {
        head -n $((insert_line - 1)) CLAUDE.md
        echo ""
        echo "$SPECS_SECTION"
        echo ""
        tail -n +$insert_line CLAUDE.md
      } > CLAUDE.md.tmp && mv CLAUDE.md.tmp CLAUDE.md
    else
      # No other headings, append at end
      echo "" >> CLAUDE.md
      echo "$SPECS_SECTION" >> CLAUDE.md
    fi
  else
    # No headings at all, just append
    echo "" >> CLAUDE.md
    echo "$SPECS_SECTION" >> CLAUDE.md
  fi
  echo -e "${GREEN}✓${NC} Added Specifications section to CLAUDE.md"
else
  echo -e "${YELLOW}○${NC} CLAUDE.md already has Specifications section"
fi

#──────────────────────────────────────────────────────────────────
# Create specs/ directory if it doesn't exist
#──────────────────────────────────────────────────────────────────
if [[ ! -d "specs" ]]; then
  mkdir -p specs
  cat > specs/README.md << 'EOF'
# Specifications

This directory contains design specifications for the project.

## Status Legend

- **Planned** - Design complete, not yet implemented
- **In Progress** - Currently being implemented
- **Implemented** - Feature complete and tested

## Specifications

| Spec | Status | Purpose |
|------|--------|---------|
| (none yet) | - | - |

---

## Creating Specifications

Each spec should include:

1. **Overview** - Purpose, goals, non-goals
2. **Architecture** - Structure and flow diagrams
3. **Core Types** - Data structures and interfaces
4. **Behaviors** - How the feature works
5. **Security Considerations** - Potential risks
6. **Implementation Phases** - Ordered steps

See existing specs for examples.
EOF
  echo -e "${GREEN}✓${NC} Created specs/ directory with README.md"
else
  echo -e "${YELLOW}○${NC} specs/ directory already exists"
fi

#──────────────────────────────────────────────────────────────────
# Print summary
#──────────────────────────────────────────────────────────────────
echo ""
echo -e "${GREEN}Fresher initialized successfully!${NC}"
echo ""
echo "Created structure:"
echo "  .fresher/"
echo "  ├── run.sh              (loop executor)"
echo "  ├── config.sh           (configuration)"
echo "  ├── PROMPT.planning.md  (planning mode template)"
echo "  ├── PROMPT.building.md  (building mode template)"
echo "  ├── AGENTS.md           (project knowledge)"
if [[ "$NO_HOOKS" != "true" ]]; then
  echo "  ├── hooks/"
  echo "  │   ├── started"
  echo "  │   ├── next_iteration"
  echo "  │   └── finished"
fi
if [[ "$NO_DOCKER" != "true" ]]; then
  echo "  ├── docker/"
  echo "  │   ├── devcontainer.json"
  echo "  │   ├── docker-compose.yml"
  echo "  │   └── fresher-firewall-overlay.sh"
fi
echo "  ├── lib/"
echo "  └── logs/"
echo ""
echo "Next steps:"
echo "  1. Review .fresher/AGENTS.md and add project-specific knowledge"
echo "  2. Create specifications in specs/ directory"
echo "  3. Run: fresher plan"
