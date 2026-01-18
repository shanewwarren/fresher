#!/bin/bash
# Fresher Installer
# Usage: curl -fsSL https://raw.githubusercontent.com/fresher/fresher/main/install.sh | bash
#
# Options:
#   --version=X.Y.Z   Install specific version (default: latest)
#   --source=PATH     Install from local directory instead of GitHub
#   --force           Overwrite existing .fresher/ directory
#   --dry-run         Show what would be done without making changes
#   --no-docker       Skip Docker configuration files
#   --no-hooks        Skip hook script creation
#   --help, -h        Show help message

set -e

#──────────────────────────────────────────────────────────────────
# Colors
#──────────────────────────────────────────────────────────────────

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

#──────────────────────────────────────────────────────────────────
# Configuration
#──────────────────────────────────────────────────────────────────

GITHUB_REPO="${FRESHER_GITHUB_REPO:-fresher/fresher}"
INSTALL_VERSION=""
SOURCE_PATH=""
FORCE=false
DRY_RUN=false
NO_DOCKER=false
NO_HOOKS=false

#──────────────────────────────────────────────────────────────────
# Parse Arguments
#──────────────────────────────────────────────────────────────────

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version=*)
      INSTALL_VERSION="${1#*=}"
      shift
      ;;
    --source=*)
      SOURCE_PATH="${1#*=}"
      shift
      ;;
    --force)
      FORCE=true
      shift
      ;;
    --dry-run)
      DRY_RUN=true
      shift
      ;;
    --no-docker)
      NO_DOCKER=true
      shift
      ;;
    --no-hooks)
      NO_HOOKS=true
      shift
      ;;
    --help|-h)
      cat << EOF
Fresher Installer

Usage: curl -fsSL https://raw.githubusercontent.com/fresher/fresher/main/install.sh | bash

Options:
  --version=X.Y.Z   Install specific version (default: latest)
  --source=PATH     Install from local directory instead of GitHub
  --force           Overwrite existing .fresher/ directory
  --dry-run         Show what would be done without making changes
  --no-docker       Skip Docker configuration files
  --no-hooks        Skip hook script creation
  --help, -h        Show this help message

Examples:
  # Install latest version
  curl -fsSL https://raw.githubusercontent.com/fresher/fresher/main/install.sh | bash

  # Install specific version
  curl -fsSL ... | bash -s -- --version=1.2.0

  # Install from local source
  bash install.sh --source=/path/to/fresher

  # Preview installation
  bash install.sh --source=/path/to/fresher --dry-run
EOF
      exit 0
      ;;
    *)
      echo -e "${RED}Unknown option: $1${NC}" >&2
      exit 1
      ;;
  esac
done

#──────────────────────────────────────────────────────────────────
# Helper Functions
#──────────────────────────────────────────────────────────────────

log() {
  echo -e "${GREEN}[fresher]${NC} $*"
}

warn() {
  echo -e "${YELLOW}[fresher]${NC} $*"
}

error() {
  echo -e "${RED}[fresher]${NC} ERROR: $*" >&2
}

# Fetch latest version from GitHub API
get_latest_version() {
  local api_url="https://api.github.com/repos/${GITHUB_REPO}/releases/latest"
  local response

  if command -v curl &> /dev/null; then
    response=$(curl -fsSL "$api_url" 2>/dev/null) || return 1
  elif command -v wget &> /dev/null; then
    response=$(wget -qO- "$api_url" 2>/dev/null) || return 1
  else
    return 1
  fi

  # Parse tag_name from JSON
  local version
  if command -v jq &> /dev/null; then
    version=$(echo "$response" | jq -r '.tag_name // empty')
  else
    version=$(echo "$response" | grep -o '"tag_name"[[:space:]]*:[[:space:]]*"[^"]*"' | sed 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/')
  fi

  # Strip leading 'v' if present
  version="${version#v}"
  echo "$version"
}

# Download release tarball
download_release() {
  local version="$1"
  local dest_file="$2"
  local url="https://github.com/${GITHUB_REPO}/archive/refs/tags/v${version}.tar.gz"

  log "Downloading version ${version}..."

  if command -v curl &> /dev/null; then
    curl -fsSL "$url" -o "$dest_file" || return 1
  elif command -v wget &> /dev/null; then
    wget -q "$url" -O "$dest_file" || return 1
  else
    error "Neither curl nor wget available"
    return 1
  fi
}

# Detect project type for config generation
detect_project_type() {
  local dir="${1:-.}"

  if [[ -f "$dir/package.json" ]]; then
    if [[ -f "$dir/bun.lockb" ]]; then
      echo "bun"
    else
      echo "nodejs"
    fi
  elif [[ -f "$dir/Cargo.toml" ]]; then
    echo "rust"
  elif [[ -f "$dir/go.mod" ]]; then
    echo "go"
  elif [[ -f "$dir/pyproject.toml" ]] || [[ -f "$dir/requirements.txt" ]]; then
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
  case "$1" in
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
  case "$1" in
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
  case "$1" in
    nodejs|bun) echo "bun run lint" ;;
    rust) echo "cargo clippy" ;;
    go) echo "golangci-lint run" ;;
    python) echo "ruff check" ;;
    *) echo "" ;;
  esac
}

get_src_dir() {
  case "$1" in
    go) echo "." ;;
    *) echo "src" ;;
  esac
}

#──────────────────────────────────────────────────────────────────
# Main Installation
#──────────────────────────────────────────────────────────────────

echo ""
echo -e "${BLUE}╔════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                     Fresher Installer                          ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check for existing .fresher/
if [[ -d ".fresher" ]]; then
  if [[ "$FORCE" == "true" ]]; then
    warn "Removing existing .fresher/ directory..."
    if [[ "$DRY_RUN" != "true" ]]; then
      rm -rf .fresher
    fi
  else
    error ".fresher/ directory already exists"
    echo ""
    echo "Options:"
    echo "  1. Use --force to overwrite"
    echo "  2. Use '.fresher/bin/fresher upgrade' to upgrade existing installation"
    echo "  3. Remove .fresher/ manually"
    exit 2
  fi
fi

# Determine source
TEMP_DIR=""
FRESHER_SOURCE=""

if [[ -n "$SOURCE_PATH" ]]; then
  # Local source
  if [[ ! -d "$SOURCE_PATH" ]]; then
    error "Source path not found: $SOURCE_PATH"
    exit 1
  fi

  # Determine if source is repo root or .fresher dir
  if [[ -d "$SOURCE_PATH/.fresher" ]]; then
    FRESHER_SOURCE="$SOURCE_PATH/.fresher"
  elif [[ -f "$SOURCE_PATH/run.sh" ]]; then
    FRESHER_SOURCE="$SOURCE_PATH"
  else
    error "Invalid source: no .fresher/ or run.sh found"
    exit 1
  fi

  if [[ -f "$FRESHER_SOURCE/VERSION" ]]; then
    INSTALL_VERSION=$(cat "$FRESHER_SOURCE/VERSION" | tr -d '[:space:]')
  else
    INSTALL_VERSION="local"
  fi

  log "Installing from local source: $SOURCE_PATH"
  log "Version: $INSTALL_VERSION"

else
  # Download from GitHub
  if [[ -z "$INSTALL_VERSION" ]]; then
    log "Checking for latest version..."
    INSTALL_VERSION=$(get_latest_version) || {
      error "Could not fetch latest version from GitHub"
      exit 1
    }
  fi

  log "Installing version: $INSTALL_VERSION"

  TEMP_DIR=$(mktemp -d)
  trap "rm -rf $TEMP_DIR" EXIT

  TARBALL="$TEMP_DIR/fresher-${INSTALL_VERSION}.tar.gz"
  EXTRACT_DIR="$TEMP_DIR/fresher-${INSTALL_VERSION}"

  download_release "$INSTALL_VERSION" "$TARBALL" || {
    error "Failed to download release"
    exit 1
  }

  mkdir -p "$EXTRACT_DIR"
  tar -xzf "$TARBALL" -C "$EXTRACT_DIR" --strip-components=1

  if [[ -d "$EXTRACT_DIR/.fresher" ]]; then
    FRESHER_SOURCE="$EXTRACT_DIR/.fresher"
  else
    FRESHER_SOURCE="$EXTRACT_DIR"
  fi
fi

# Verify source has required files
if [[ ! -f "$FRESHER_SOURCE/run.sh" ]]; then
  error "Invalid source: run.sh not found in $FRESHER_SOURCE"
  exit 1
fi

#──────────────────────────────────────────────────────────────────
# Copy Core Files
#──────────────────────────────────────────────────────────────────

log "Installing Fresher..."

if [[ "$DRY_RUN" == "true" ]]; then
  log "[dry-run] Would create .fresher/ directory structure"
else
  mkdir -p .fresher
  mkdir -p .fresher/lib
  mkdir -p .fresher/bin
  mkdir -p .fresher/logs

  # Copy core files
  for file in run.sh VERSION; do
    if [[ -f "$FRESHER_SOURCE/$file" ]]; then
      cp "$FRESHER_SOURCE/$file" ".fresher/$file"
    fi
  done

  # Copy PROMPT templates
  for file in "$FRESHER_SOURCE"/PROMPT.*.md; do
    if [[ -f "$file" ]]; then
      cp "$file" ".fresher/"
    fi
  done

  # Copy lib/ directory
  if [[ -d "$FRESHER_SOURCE/lib" ]]; then
    cp -r "$FRESHER_SOURCE/lib/"* ".fresher/lib/" 2>/dev/null || true
  fi

  # Copy bin/ directory
  if [[ -d "$FRESHER_SOURCE/bin" ]]; then
    cp -r "$FRESHER_SOURCE/bin/"* ".fresher/bin/" 2>/dev/null || true
    chmod +x .fresher/bin/* 2>/dev/null || true
  fi

  # Copy tests/ directory
  if [[ -d "$FRESHER_SOURCE/tests" ]]; then
    cp -r "$FRESHER_SOURCE/tests" ".fresher/"
  fi

  # Copy docker/ directory (unless --no-docker)
  if [[ "$NO_DOCKER" != "true" && -d "$FRESHER_SOURCE/docker" ]]; then
    cp -r "$FRESHER_SOURCE/docker" ".fresher/"
  fi

  chmod +x .fresher/run.sh

  log "Copied core files"
fi

#──────────────────────────────────────────────────────────────────
# Generate Project-Specific Files
#──────────────────────────────────────────────────────────────────

PROJECT_TYPE=$(detect_project_type)
PROJECT_NAME=$(basename "$(pwd)")
TEST_CMD=$(get_test_command "$PROJECT_TYPE")
BUILD_CMD=$(get_build_command "$PROJECT_TYPE")
LINT_CMD=$(get_lint_command "$PROJECT_TYPE")
SRC_DIR=$(get_src_dir "$PROJECT_TYPE")
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

log "Detected project type: $PROJECT_TYPE"

if [[ "$DRY_RUN" == "true" ]]; then
  log "[dry-run] Would generate config.sh for $PROJECT_TYPE project"
else
  # Generate config.sh
  cat > .fresher/config.sh << EOF
#!/bin/bash
# Fresher configuration for $PROJECT_NAME
# Generated: $TIMESTAMP
# Project type: $PROJECT_TYPE

#──────────────────────────────────────────────────────────────────
# Mode Configuration
#──────────────────────────────────────────────────────────────────
export FRESHER_MODE="\${FRESHER_MODE:-planning}"

#──────────────────────────────────────────────────────────────────
# Termination Settings
#──────────────────────────────────────────────────────────────────
export FRESHER_MAX_ITERATIONS="\${FRESHER_MAX_ITERATIONS:-0}"
export FRESHER_SMART_TERMINATION="\${FRESHER_SMART_TERMINATION:-true}"

#──────────────────────────────────────────────────────────────────
# Claude Code Settings
#──────────────────────────────────────────────────────────────────
export FRESHER_DANGEROUS_PERMISSIONS="\${FRESHER_DANGEROUS_PERMISSIONS:-true}"
export FRESHER_MAX_TURNS="\${FRESHER_MAX_TURNS:-50}"
export FRESHER_MODEL="\${FRESHER_MODEL:-sonnet}"

#──────────────────────────────────────────────────────────────────
# Project Commands (detected: $PROJECT_TYPE)
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

#──────────────────────────────────────────────────────────────────
# Docker/Devcontainer Configuration
#──────────────────────────────────────────────────────────────────
export FRESHER_USE_DOCKER="\${FRESHER_USE_DOCKER:-false}"

# Resource limits (passed to devcontainer)
export FRESHER_DOCKER_MEMORY="\${FRESHER_DOCKER_MEMORY:-4g}"
export FRESHER_DOCKER_CPUS="\${FRESHER_DOCKER_CPUS:-2}"
EOF

  log "Generated config.sh"
fi

#──────────────────────────────────────────────────────────────────
# Create AGENTS.md
#──────────────────────────────────────────────────────────────────

if [[ "$DRY_RUN" == "true" ]]; then
  log "[dry-run] Would create AGENTS.md"
else
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

  log "Created AGENTS.md"
fi

#──────────────────────────────────────────────────────────────────
# Create Hook Scripts
#──────────────────────────────────────────────────────────────────

if [[ "$NO_HOOKS" != "true" ]]; then
  if [[ "$DRY_RUN" == "true" ]]; then
    log "[dry-run] Would create hook scripts"
  else
    mkdir -p .fresher/hooks

    cat > .fresher/hooks/started << 'EOF'
#!/bin/bash
# .fresher/hooks/started
# Runs once when the Ralph loop begins

echo "Starting Fresher loop in $FRESHER_MODE mode"
exit 0
EOF

    cat > .fresher/hooks/next_iteration << 'EOF'
#!/bin/bash
# .fresher/hooks/next_iteration
# Runs before each iteration

echo "Starting iteration ${FRESHER_ITERATION:-1}"
exit 0
EOF

    cat > .fresher/hooks/finished << 'EOF'
#!/bin/bash
# .fresher/hooks/finished
# Runs when the loop ends

echo "Fresher loop finished: ${FRESHER_FINISH_TYPE:-unknown}"
echo "Iterations: ${FRESHER_TOTAL_ITERATIONS:-0}, Commits: ${FRESHER_TOTAL_COMMITS:-0}"
exit 0
EOF

    chmod +x .fresher/hooks/*
    log "Created hook scripts"
  fi
fi

#──────────────────────────────────────────────────────────────────
# Update .gitignore
#──────────────────────────────────────────────────────────────────

GITIGNORE_ENTRIES="
# Fresher
.fresher/logs/
.fresher/.state
"

if [[ "$DRY_RUN" == "true" ]]; then
  log "[dry-run] Would update .gitignore"
else
  if [[ -f ".gitignore" ]]; then
    if ! grep -q ".fresher/logs/" .gitignore 2>/dev/null; then
      echo "$GITIGNORE_ENTRIES" >> .gitignore
      log "Updated .gitignore"
    fi
  else
    echo "$GITIGNORE_ENTRIES" > .gitignore
    log "Created .gitignore"
  fi
fi

#──────────────────────────────────────────────────────────────────
# Create .gitkeep files
#──────────────────────────────────────────────────────────────────

if [[ "$DRY_RUN" != "true" ]]; then
  touch .fresher/lib/.gitkeep 2>/dev/null || true
  touch .fresher/logs/.gitkeep 2>/dev/null || true
fi

#──────────────────────────────────────────────────────────────────
# Create specs/ directory if needed
#──────────────────────────────────────────────────────────────────

if [[ ! -d "specs" ]]; then
  if [[ "$DRY_RUN" == "true" ]]; then
    log "[dry-run] Would create specs/ directory"
  else
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
EOF
    log "Created specs/ directory"
  fi
fi

#──────────────────────────────────────────────────────────────────
# Summary
#──────────────────────────────────────────────────────────────────

echo ""
if [[ "$DRY_RUN" == "true" ]]; then
  echo -e "${YELLOW}Dry run complete. No changes made.${NC}"
else
  echo -e "${GREEN}Fresher installed successfully!${NC}"
  echo ""
  echo "Installed structure:"
  echo "  .fresher/"
  echo "  ├── run.sh              (loop executor)"
  echo "  ├── config.sh           (configuration)"
  echo "  ├── VERSION             ($INSTALL_VERSION)"
  echo "  ├── PROMPT.*.md         (mode templates)"
  echo "  ├── AGENTS.md           (project knowledge)"
  echo "  ├── bin/                (CLI commands)"
  echo "  ├── lib/                (libraries)"
  [[ "$NO_HOOKS" != "true" ]] && echo "  ├── hooks/              (lifecycle scripts)"
  [[ "$NO_DOCKER" != "true" ]] && echo "  ├── docker/             (container config)"
  echo "  └── logs/               (iteration logs)"
  echo ""
  echo "Next steps:"
  echo "  1. Review .fresher/AGENTS.md and add project-specific knowledge"
  echo "  2. Create specifications in specs/ directory"
  echo "  3. Run: .fresher/bin/fresher plan"
  echo ""
  echo "Or add .fresher/bin to your PATH:"
  echo "  export PATH=\"\$PWD/.fresher/bin:\$PATH\""
  echo "  fresher plan"
fi
echo ""
