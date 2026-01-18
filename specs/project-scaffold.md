# Project Scaffold Specification

**Status:** Implemented
**Version:** 2.0
**Last Updated:** 2026-01-18
**Implementation:** `src/commands/init.rs`, `src/templates.rs`, `src/config.rs`

---

## 1. Overview

### Purpose

The project scaffold handles initialization of the `.fresher/` folder structure in any project. It detects project type, generates appropriate templates, and creates a self-contained configuration.

### Goals

- **Zero-config start** - `fresher init` works in any project with sensible defaults
- **Project detection** - Auto-detect language/framework for appropriate templates
- **Portable** - `.fresher/` folder is self-contained and version-controllable
- **TOML configuration** - Human-readable, well-documented config format

### Non-Goals

- **Framework scaffolding** - Don't create project structure, only Fresher structure
- **CI/CD integration** - Users configure their own pipelines
- **Remote storage** - All files are local

---

## 2. Architecture

### Generated Structure

```
.fresher/
├── config.toml               # Configuration (TOML format)
├── AGENTS.md                 # Project-specific knowledge
├── PROMPT.planning.md        # Planning mode instructions
├── PROMPT.building.md        # Building mode instructions
├── hooks/                    # Lifecycle hook scripts
│   ├── started              # Runs when loop starts
│   ├── next_iteration       # Runs before each iteration
│   └── finished             # Runs when loop ends
└── logs/                     # Iteration logs (gitignored)

specs/                        # Created if doesn't exist
└── README.md
```

### Initialization Flow

```
┌─────────────────────────────────────────────────────────────────┐
│  fresher init [--force]                                         │
├─────────────────────────────────────────────────────────────────┤
│  1. Check if .fresher/ exists                                   │
│     → If yes and no --force, exit with error                    │
│                                                                 │
│  2. Detect project type                                         │
│     → Check for package.json, Cargo.toml, go.mod, etc.         │
│     → Get default commands for detected type                    │
│                                                                 │
│  3. Create directory structure                                  │
│     → .fresher/                                                 │
│     → .fresher/hooks/                                           │
│     → .fresher/logs/                                            │
│                                                                 │
│  4. Generate files from templates                               │
│     → config.toml (with detected defaults)                      │
│     → AGENTS.md (with project name and commands)                │
│     → PROMPT.planning.md                                        │
│     → PROMPT.building.md                                        │
│     → hooks/started, next_iteration, finished (executable)      │
│                                                                 │
│  5. Create specs/ if needed                                     │
│     → mkdir specs/                                              │
│     → Create specs/README.md                                    │
│                                                                 │
│  6. Print next steps                                            │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. Core Types

### 3.1 Project Type Detection (config.rs)

```rust
pub enum ProjectType {
    Bun,
    NodeJs,
    Rust,
    Go,
    Python,
    Make,
    DotNet,
    Maven,
    Gradle,
    Generic,
}

pub fn detect_project_type() -> ProjectType {
    if Path::new("bun.lockb").exists() || Path::new("bunfig.toml").exists() {
        ProjectType::Bun
    } else if Path::new("package.json").exists() {
        ProjectType::NodeJs
    } else if Path::new("Cargo.toml").exists() {
        ProjectType::Rust
    } else if Path::new("go.mod").exists() {
        ProjectType::Go
    } else if Path::new("pyproject.toml").exists() || Path::new("setup.py").exists() {
        ProjectType::Python
    } else if Path::new("Makefile").exists() {
        ProjectType::Make
    // ... etc
    } else {
        ProjectType::Generic
    }
}
```

**Detection Priority:**

| Priority | Indicator File | Project Type | Test Command | Build Command | Lint Command |
|----------|----------------|--------------|--------------|---------------|--------------|
| 1 | `bun.lockb` or `bunfig.toml` | Bun | `bun test` | `bun run build` | `bun run lint` |
| 2 | `package.json` | Node.js | `npm test` | `npm run build` | `npm run lint` |
| 3 | `Cargo.toml` | Rust | `cargo test` | `cargo build` | `cargo clippy` |
| 4 | `go.mod` | Go | `go test ./...` | `go build` | `go fmt` |
| 5 | `pyproject.toml` / `setup.py` | Python | `pytest` | `python -m build` | `ruff check` |
| 6 | `Makefile` | Make | `make test` | `make build` | `make lint` |
| 7 | `*.csproj` / `*.sln` | .NET | `dotnet test` | `dotnet build` | - |
| 8 | `pom.xml` | Maven | `mvn test` | `mvn clean package` | `mvn checkstyle:check` |
| 9 | `build.gradle` | Gradle | `gradle test` | `gradle build` | `gradle check` |
| - | (none) | Generic | - | - | - |

### 3.2 Command Line Interface

```bash
fresher init [OPTIONS]

Options:
  -f, --force    Overwrite existing .fresher/ without prompting
  -h, --help     Print help
```

---

## 4. Generated Files

### 4.1 config.toml

```toml
# Fresher configuration
# Generated: 2026-01-18T10:00:00Z
# Project type: rust

[fresher]
mode = "planning"
max_iterations = 0
smart_termination = true
dangerous_permissions = true
max_turns = 50
model = "sonnet"

[commands]
test = "cargo test"
build = "cargo build"
lint = "cargo clippy"

[paths]
log_dir = ".fresher/logs"
spec_dir = "specs"
src_dir = "src"

[hooks]
enabled = true
timeout = 30

[docker]
use_docker = false
memory = "4g"
cpus = "2"
```

### 4.2 AGENTS.md

Project-specific knowledge file with detected commands:

```markdown
# Project: {project_name}

## Commands

### Testing
{test_command}

### Building
{build_command}

### Linting
{lint_command}

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
```

### 4.3 Prompt Templates

**PROMPT.planning.md** - Instructions for planning mode:
- Read all specifications in `specs/`
- Explore the codebase
- Identify gaps between specs and implementation
- Create/update `IMPLEMENTATION_PLAN.md`
- **Do NOT implement anything or make commits**

**PROMPT.building.md** - Instructions for building mode:
- Read `IMPLEMENTATION_PLAN.md`
- Select highest priority incomplete task
- Investigate relevant code
- Implement the task
- Validate with tests and builds
- Update plan and commit changes

### 4.4 Hook Scripts

Executable bash scripts with standard exit code conventions:

| Hook | When | Exit 0 | Exit 1 | Exit 2 |
|------|------|--------|--------|--------|
| `started` | Loop begins | Continue | - | Abort |
| `next_iteration` | Before each iteration | Continue | Skip iteration | Abort |
| `finished` | Loop ends | - | - | - |

**Environment variables available to hooks:**

| Variable | Description |
|----------|-------------|
| `FRESHER_ITERATION` | Current iteration number |
| `FRESHER_TOTAL_ITERATIONS` | Total iterations (in finished) |
| `FRESHER_TOTAL_COMMITS` | Total commits made |
| `FRESHER_DURATION` | Duration in seconds |
| `FRESHER_FINISH_TYPE` | How loop ended (in finished) |
| `FRESHER_MODE` | Current mode |
| `FRESHER_PROJECT_DIR` | Project root |

---

## 5. Implementation (init.rs)

### Main Flow

```rust
pub async fn run(force: bool) -> Result<()> {
    let fresher_dir = Path::new(".fresher");

    // Check if already initialized
    if fresher_dir.exists() && !force {
        bail!(".fresher/ already exists. Use --force to overwrite.");
    }

    // Detect project type
    let project_type = detect_project_type();
    let commands = project_type.default_commands();

    // Create directory structure
    create_directory_structure()?;

    // Generate files from templates
    // - config.toml
    // - AGENTS.md
    // - PROMPT.planning.md
    // - PROMPT.building.md
    // - hooks/started, next_iteration, finished

    // Create specs/ if needed
    if !Path::new("specs").exists() {
        fs::create_dir_all("specs")?;
        fs::write("specs/README.md", "# Specifications\n...")?;
    }

    println!("Initialized .fresher/ directory");
    // Print next steps...

    Ok(())
}
```

### Hook Creation

Hooks are created with executable permissions:

```rust
fn create_hook(path: &str, content: &str) -> Result<()> {
    fs::write(path, content)?;

    // Make executable (Unix)
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms)?;

    Ok(())
}
```

---

## 6. Output

### Success Output

```
Detected project type: rust

Initialized .fresher/ directory

Next steps:
  1. Review .fresher/config.toml and adjust settings
  2. Add specifications to specs/
  3. Run 'fresher plan' to create an implementation plan
  4. Run 'fresher build' to implement tasks
```

### Error: Already Initialized

```
Error: .fresher/ already exists. Use --force to overwrite, or manually remove it first.
```

---

## 7. Files Not Created

The following are **not** generated by `fresher init`:

| File | Reason |
|------|--------|
| `.gitignore` entries | User manages their own gitignore |
| `CLAUDE.md` | Project-specific, user creates |
| `IMPLEMENTATION_PLAN.md` | Created by `fresher plan` |
| `.fresher/.state` | Created during runtime |
| Docker files | Created separately if needed |

---

## 8. Future Enhancements

- **Interactive mode** (`--interactive`): Guide users through configuration
- **Template customization**: Allow user-provided templates
- **Monorepo support**: Multiple project types in subdirectories
- **Update command**: Update templates while preserving custom config
