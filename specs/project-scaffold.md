# Project Scaffold Specification

**Status:** Planned
**Version:** 1.0
**Last Updated:** 2025-01-17

---

## 1. Overview

### Purpose

The project scaffold handles initialization of the `.fresher/` folder structure in any project. It detects project type, generates appropriate templates, and optionally installs a global CLI for convenience.

### Goals

- **Zero-config start** - `fresher init` works in any project with sensible defaults
- **Project detection** - Auto-detect language/framework for appropriate templates
- **Interactive setup** - Guide users through configuration choices
- **Portable** - `.fresher/` folder is self-contained and version-controllable

### Non-Goals

- **Framework scaffolding** - Don't create project structure, only Fresher structure
- **CI/CD integration** - Users configure their own pipelines
- **Remote storage** - All files are local

---

## 2. Architecture

### Generated Structure

```
.fresher/
├── run.sh                    # Main loop executor (executable)
├── config.sh                 # Configuration variables
├── PROMPT.planning.md        # Planning mode instructions
├── PROMPT.building.md        # Building mode instructions
├── AGENTS.md                 # Project-specific knowledge
├── hooks/                    # Lifecycle hook scripts
│   ├── started              # Runs when loop starts
│   ├── next_iteration       # Runs before each iteration
│   └── finished             # Runs when loop ends
├── lib/                      # Supporting scripts
│   ├── termination.sh
│   ├── streaming.sh
│   └── state.sh
└── logs/                     # Iteration logs (gitignored)
    └── .gitkeep
```

### Initialization Flow

```
┌─────────────────────────────────────────────────────────────────┐
│  fresher init                                                    │
├─────────────────────────────────────────────────────────────────┤
│  1. Check if .fresher/ exists                                   │
│     → If yes, prompt: overwrite / merge / abort                 │
│                                                                 │
│  2. Detect project type                                         │
│     → Check for package.json, Cargo.toml, go.mod, etc.         │
│                                                                 │
│  3. Interactive configuration (if --interactive)                │
│     → Ask about test/build commands                             │
│     → Ask about source directories                              │
│     → Ask about Docker preference                               │
│                                                                 │
│  4. Generate files                                              │
│     → Create .fresher/ directory structure                      │
│     → Write templates with detected/configured values           │
│     → Set executable permissions on scripts                     │
│                                                                 │
│  5. Update .gitignore                                           │
│     → Add .fresher/logs/                                        │
│     → Add .fresher/.state                                       │
│                                                                 │
│  6. Create/update CLAUDE.md                                     │
│     → Add Specifications section if missing                     │
│                                                                 │
│  7. Print next steps                                            │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. Core Types

### 3.1 Project Type Detection

| Indicator File | Project Type | Test Command | Build Command |
|----------------|--------------|--------------|---------------|
| `package.json` | Node.js | `npm test` | `npm run build` |
| `Cargo.toml` | Rust | `cargo test` | `cargo build` |
| `go.mod` | Go | `go test ./...` | `go build ./...` |
| `pyproject.toml` | Python | `pytest` | `python -m build` |
| `requirements.txt` | Python (legacy) | `pytest` | - |
| `Makefile` | Make-based | `make test` | `make` |
| `*.csproj` | .NET | `dotnet test` | `dotnet build` |
| `pom.xml` | Java/Maven | `mvn test` | `mvn package` |
| `build.gradle` | Java/Gradle | `./gradlew test` | `./gradlew build` |

### 3.2 Init Options

```bash
fresher init [options]

Options:
  --interactive, -i    Run interactive setup wizard
  --force, -f          Overwrite existing .fresher/ without prompting
  --no-hooks           Skip creating hook scripts
  --no-docker          Skip Docker-related files
  --project-type TYPE  Override auto-detected project type
```

### 3.3 Configuration Structure

Generated `config.sh`:

```bash
#!/bin/bash
# Fresher configuration for {project_name}
# Generated: {timestamp}
# Project type: {detected_type}

#──────────────────────────────────────────────────────────────────
# Mode Configuration
#──────────────────────────────────────────────────────────────────
export FRESHER_MODE="${FRESHER_MODE:-planning}"

#──────────────────────────────────────────────────────────────────
# Termination Settings
#──────────────────────────────────────────────────────────────────
export FRESHER_MAX_ITERATIONS="${FRESHER_MAX_ITERATIONS:-0}"
export FRESHER_SMART_TERMINATION="${FRESHER_SMART_TERMINATION:-true}"

#──────────────────────────────────────────────────────────────────
# Claude Code Settings
#──────────────────────────────────────────────────────────────────
export FRESHER_DANGEROUS_PERMISSIONS="${FRESHER_DANGEROUS_PERMISSIONS:-true}"
export FRESHER_MAX_TURNS="${FRESHER_MAX_TURNS:-50}"
export FRESHER_MODEL="${FRESHER_MODEL:-sonnet}"

#──────────────────────────────────────────────────────────────────
# Project Commands (detected: {detected_type})
#──────────────────────────────────────────────────────────────────
export FRESHER_TEST_CMD="${FRESHER_TEST_CMD:-{detected_test_cmd}}"
export FRESHER_BUILD_CMD="${FRESHER_BUILD_CMD:-{detected_build_cmd}}"
export FRESHER_LINT_CMD="${FRESHER_LINT_CMD:-{detected_lint_cmd}}"

#──────────────────────────────────────────────────────────────────
# Paths
#──────────────────────────────────────────────────────────────────
export FRESHER_LOG_DIR="${FRESHER_LOG_DIR:-.fresher/logs}"
export FRESHER_SPEC_DIR="${FRESHER_SPEC_DIR:-specs}"
export FRESHER_SRC_DIR="${FRESHER_SRC_DIR:-{detected_src_dir}}"

#──────────────────────────────────────────────────────────────────
# Docker (optional)
#──────────────────────────────────────────────────────────────────
export FRESHER_USE_DOCKER="${FRESHER_USE_DOCKER:-false}"
export FRESHER_DOCKER_IMAGE="${FRESHER_DOCKER_IMAGE:-fresher:local}"
```

---

## 4. Behaviors

### 4.1 Project Type Detection

```bash
detect_project_type() {
  local dir="${1:-.}"

  if [[ -f "$dir/package.json" ]]; then
    echo "nodejs"
  elif [[ -f "$dir/Cargo.toml" ]]; then
    echo "rust"
  elif [[ -f "$dir/go.mod" ]]; then
    echo "go"
  elif [[ -f "$dir/pyproject.toml" ]] || [[ -f "$dir/requirements.txt" ]]; then
    echo "python"
  elif [[ -f "$dir/Makefile" ]]; then
    echo "make"
  elif ls "$dir"/*.csproj &>/dev/null; then
    echo "dotnet"
  elif [[ -f "$dir/pom.xml" ]]; then
    echo "maven"
  elif [[ -f "$dir/build.gradle" ]]; then
    echo "gradle"
  else
    echo "generic"
  fi
}
```

### 4.2 Interactive Setup

When `--interactive` flag is used:

```
╔════════════════════════════════════════════════════════════════╗
║                    Fresher Setup Wizard                         ║
╚════════════════════════════════════════════════════════════════╝

Detected project type: nodejs

? Test command [npm test]:
? Build command [npm run build]:
? Lint command [npm run lint]:
? Source directory [src/]:
? Enable Docker isolation? [y/N]:
? Max iterations (0=unlimited) [0]:

Creating .fresher/ structure...
✓ Created run.sh
✓ Created config.sh
✓ Created PROMPT.planning.md
✓ Created PROMPT.building.md
✓ Created AGENTS.md
✓ Created hooks/
✓ Updated .gitignore

Next steps:
  1. Review .fresher/AGENTS.md and add project-specific knowledge
  2. Create specs in specs/ directory
  3. Run: fresher plan
```

### 4.3 CLAUDE.md Integration

If `CLAUDE.md` exists but lacks a Specifications section:

```bash
inject_specs_section() {
  local claude_md="CLAUDE.md"

  if [[ ! -f "$claude_md" ]]; then
    # Create minimal CLAUDE.md
    cat > "$claude_md" << 'EOF'
# Project Guidelines

## Specifications

**IMPORTANT:** Before implementing any feature, consult `specs/README.md`.

- **Assume NOT implemented.** Specs describe intent; code describes reality.
- **Check the codebase first.** Search actual code before concluding.
- **Use specs as guidance.** Follow design patterns in relevant spec.
- **Spec index:** `specs/README.md` lists all specs by category.
EOF
    return
  fi

  # Check if section exists
  if grep -q "## Specifications" "$claude_md"; then
    return
  fi

  # Insert after first heading or at start
  # (implementation uses sed or awk)
}
```

### 4.4 Global CLI Installation

Optional global CLI for convenience:

```bash
# Install globally
fresher install-global

# Creates wrapper script at ~/.local/bin/fresher (or /usr/local/bin)
# that detects .fresher/ in current directory and runs it
```

Global CLI wrapper:

```bash
#!/bin/bash
# fresher - global CLI wrapper

FRESHER_DIR=".fresher"

if [[ ! -d "$FRESHER_DIR" ]]; then
  echo "Error: No .fresher/ directory found in current directory"
  echo "Run 'fresher init' to initialize Fresher in this project"
  exit 1
fi

case "${1:-}" in
  init)
    shift
    exec "$0-init" "$@"
    ;;
  plan|planning)
    export FRESHER_MODE="planning"
    exec "$FRESHER_DIR/run.sh"
    ;;
  build|building)
    export FRESHER_MODE="building"
    exec "$FRESHER_DIR/run.sh"
    ;;
  *)
    echo "Usage: fresher <command>"
    echo ""
    echo "Commands:"
    echo "  init      Initialize Fresher in current directory"
    echo "  plan      Run planning mode"
    echo "  build     Run building mode"
    exit 1
    ;;
esac
```

---

## 5. Security Considerations

### Executable Permissions

- `run.sh` must be executable (`chmod +x`)
- Hook scripts must be executable
- Warn if permissions are incorrect

### Gitignore Entries

Automatically add to `.gitignore`:

```
# Fresher
.fresher/logs/
.fresher/.state
```

These prevent:
- Log accumulation in repo
- State file conflicts between developers

---

## 6. Implementation Phases

| Phase | Description | Dependencies | Complexity |
|-------|-------------|--------------|------------|
| 1 | Basic init with detection | None | Medium |
| 2 | Template generation | prompt-templates | Low |
| 3 | Interactive wizard | Phase 1 | Medium |
| 4 | CLAUDE.md integration | Phase 1 | Low |
| 5 | Global CLI installation | Phase 1-4 | Medium |

---

## 7. Open Questions

- [ ] Should `fresher init` create a `specs/` directory too?
- [ ] How to handle monorepos with multiple project types?
- [ ] Should there be a `fresher upgrade` command for updating templates?
