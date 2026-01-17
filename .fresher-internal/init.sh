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
PROJECT_TYPE=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --force|-f)
      FORCE=true
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
      echo "  --force, -f           Overwrite existing .fresher/ without prompting"
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
    nodejs) echo "npm test" ;;
    bun) echo "bun test" ;;
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
    nodejs) echo "npm run build" ;;
    bun) echo "bun run build" ;;
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
    nodejs) echo "npm run lint" ;;
    bun) echo "bun run lint" ;;
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

echo -e "${BLUE}Initializing Fresher...${NC}"
echo "Project: $PROJECT_NAME"
echo "Detected type: $DETECTED_TYPE"
echo ""

#──────────────────────────────────────────────────────────────────
# Create directory structure
#──────────────────────────────────────────────────────────────────
echo "Creating .fresher/ directory structure..."

mkdir -p .fresher/hooks
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
export FRESHER_MAX_ITERATIONS="\${FRESHER_MAX_ITERATIONS:-0}"
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
# Docker (optional)
#──────────────────────────────────────────────────────────────────
export FRESHER_USE_DOCKER="\${FRESHER_USE_DOCKER:-false}"
export FRESHER_DOCKER_IMAGE="\${FRESHER_DOCKER_IMAGE:-fresher:local}"
EOF

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
# Generate stub prompt templates
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

<!-- Full template content will be added in prompt-templates phase -->
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

<!-- Full template content will be added in prompt-templates phase -->
EOF

echo -e "${GREEN}✓${NC} Created PROMPT templates (stubs)"

#──────────────────────────────────────────────────────────────────
# Generate hook scripts
#──────────────────────────────────────────────────────────────────
cat > .fresher/hooks/started << 'EOF'
#!/bin/bash
# .fresher/hooks/started
# Runs once when the Fresher loop begins

echo "Starting Fresher loop in $FRESHER_MODE mode"
echo "Project: ${FRESHER_PROJECT_DIR:-$(pwd)}"
echo "Max iterations: ${FRESHER_MAX_ITERATIONS:-unlimited}"

exit 0
EOF
chmod +x .fresher/hooks/started

cat > .fresher/hooks/next_iteration << 'EOF'
#!/bin/bash
# .fresher/hooks/next_iteration
# Runs before each iteration

echo "Starting iteration ${FRESHER_ITERATION:-1}"

exit 0
EOF
chmod +x .fresher/hooks/next_iteration

cat > .fresher/hooks/finished << 'EOF'
#!/bin/bash
# .fresher/hooks/finished
# Runs when the loop ends

echo "Fresher loop finished"
echo "Finish type: ${FRESHER_FINISH_TYPE:-unknown}"
echo "Total iterations: ${FRESHER_TOTAL_ITERATIONS:-0}"

exit 0
EOF
chmod +x .fresher/hooks/finished

echo -e "${GREEN}✓${NC} Created hook scripts"

#──────────────────────────────────────────────────────────────────
# Create .gitkeep files
#──────────────────────────────────────────────────────────────────
touch .fresher/lib/.gitkeep
touch .fresher/logs/.gitkeep

echo -e "${GREEN}✓${NC} Created lib/ and logs/ directories"

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
echo "  ├── run.sh              (loop executor stub)"
echo "  ├── config.sh           (configuration)"
echo "  ├── PROMPT.planning.md  (planning mode template)"
echo "  ├── PROMPT.building.md  (building mode template)"
echo "  ├── AGENTS.md           (project knowledge)"
echo "  ├── hooks/"
echo "  │   ├── started"
echo "  │   ├── next_iteration"
echo "  │   └── finished"
echo "  ├── lib/"
echo "  └── logs/"
echo ""
echo "Next steps:"
echo "  1. Review .fresher/AGENTS.md and add project-specific knowledge"
echo "  2. Create specifications in specs/ directory"
echo "  3. Run: fresher plan"
