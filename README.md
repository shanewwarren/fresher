# Fresher

> Portable Ralph Loop implementation for AI-driven iterative development

Fresher brings the Ralph Loop methodology to any project. Run Claude Code in iterative loops with fresh context, specification-driven development, and automatic termination detection.

## What is the Ralph Loop?

The Ralph Loop is an iterative execution model with two modes:

- **PLANNING** - Generate implementation plans from specifications
- **BUILDING** - Execute plans with test/build backpressure

Each iteration starts with fresh context, keeping Claude in the "smart zone" for optimal performance.

## Installation

### From Cargo (Recommended)

```bash
cargo install fresher
```

### From Binary Release

Download the latest release for your platform:

```bash
# macOS (Apple Silicon)
curl -fsSL https://github.com/shanewwarren/fresher/releases/latest/download/fresher-aarch64-apple-darwin.tar.gz | tar xz
sudo mv fresher /usr/local/bin/

# macOS (Intel)
curl -fsSL https://github.com/shanewwarren/fresher/releases/latest/download/fresher-x86_64-apple-darwin.tar.gz | tar xz
sudo mv fresher /usr/local/bin/

# Linux (x86_64)
curl -fsSL https://github.com/shanewwarren/fresher/releases/latest/download/fresher-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv fresher /usr/local/bin/
```

### Self-Upgrade

Check for updates and upgrade to the latest version:

```bash
fresher upgrade --check  # Check for updates
fresher upgrade          # Install latest version

# If installed in /usr/local/bin or similar, use sudo:
sudo fresher upgrade
```

## Quick Start

### 1. Initialize

```bash
cd your-project
fresher init
```

This creates a `.fresher/` directory with configuration and templates. Fresher auto-detects your project type (Node.js, Rust, Go, Python, etc.) and configures appropriate defaults.

### 2. Add Specifications

Write specs in `specs/` describing what you want to build:

```markdown
# Feature: User Authentication

## Requirements

- Users can sign up with email/password
- Passwords must be hashed with bcrypt
- Sessions expire after 24 hours
```

### 3. Run Planning Mode

```bash
fresher plan
```

Claude analyzes your specs and codebase, then creates `IMPLEMENTATION_PLAN.md` with prioritized tasks.

### 4. Review the Plan

Check `IMPLEMENTATION_PLAN.md` to ensure the plan matches your intent.

### 5. Run Building Mode

```bash
fresher build
```

Claude works through the plan, implementing one task per iteration with test/build validation.

The loop runs until all tasks are complete, max iterations reached, or you stop it manually (Ctrl+C).

## Commands

| Command | Description |
|---------|-------------|
| `fresher init` | Initialize `.fresher/` in a project |
| `fresher plan` | Run planning mode (analyze specs, create plan) |
| `fresher build` | Run building mode (implement tasks from plan) |
| `fresher verify` | Verify plan coverage against specs |
| `fresher upgrade` | Self-upgrade to latest version |
| `fresher version` | Show version information |
| `fresher docker shell` | Open interactive shell in devcontainer |
| `fresher docker build` | Build the devcontainer image |

### Command Options

```bash
# Limit iterations
fresher plan --max-iterations 5
fresher build --max-iterations 10

# Force overwrite existing config
fresher init --force

# Verify with JSON output
fresher verify --json

# Check for updates without installing
fresher upgrade --check
```

## Configuration

Fresher uses TOML configuration in `.fresher/config.toml`:

```toml
[fresher]
mode = "planning"
max_iterations = 0        # 0 = unlimited
smart_termination = true  # Stop when all tasks complete
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

### Configuration Reference

| Section | Key | Description | Default |
|---------|-----|-------------|---------|
| `fresher` | `mode` | Execution mode | `"planning"` |
| | `max_iterations` | Iteration limit (0=unlimited) | `0` |
| | `smart_termination` | Stop when tasks complete | `true` |
| | `dangerous_permissions` | Skip Claude permission prompts | `true` |
| | `max_turns` | Claude max turns per iteration | `50` |
| | `model` | Claude model to use | `"sonnet"` |
| `commands` | `test` | Test command | Auto-detected |
| | `build` | Build command | Auto-detected |
| | `lint` | Lint command | Auto-detected |
| `paths` | `log_dir` | Log directory | `".fresher/logs"` |
| | `spec_dir` | Specifications directory | `"specs"` |
| | `src_dir` | Source directory | `"src"` |
| `hooks` | `enabled` | Enable lifecycle hooks | `true` |
| | `timeout` | Hook timeout (seconds) | `30` |
| `docker` | `use_docker` | Require Docker isolation | `false` |
| | `memory` | Container memory limit | `"4g"` |
| | `cpus` | Container CPU limit | `"2"` |

### Environment Variables

All config values can be overridden with environment variables:

```bash
# Override max iterations
FRESHER_MAX_ITERATIONS=10 fresher build

# Use building mode
FRESHER_MODE=building fresher plan  # (mode flag has no effect here)

# Disable hooks
FRESHER_HOOKS_ENABLED=false fresher build
```

| Variable | Config Key |
|----------|------------|
| `FRESHER_MODE` | `fresher.mode` |
| `FRESHER_MAX_ITERATIONS` | `fresher.max_iterations` |
| `FRESHER_SMART_TERMINATION` | `fresher.smart_termination` |
| `FRESHER_DANGEROUS_PERMISSIONS` | `fresher.dangerous_permissions` |
| `FRESHER_MAX_TURNS` | `fresher.max_turns` |
| `FRESHER_MODEL` | `fresher.model` |
| `FRESHER_TEST_CMD` | `commands.test` |
| `FRESHER_BUILD_CMD` | `commands.build` |
| `FRESHER_LINT_CMD` | `commands.lint` |
| `FRESHER_LOG_DIR` | `paths.log_dir` |
| `FRESHER_SPEC_DIR` | `paths.spec_dir` |
| `FRESHER_SRC_DIR` | `paths.src_dir` |
| `FRESHER_HOOKS_ENABLED` | `hooks.enabled` |
| `FRESHER_HOOK_TIMEOUT` | `hooks.timeout` |
| `FRESHER_USE_DOCKER` | `docker.use_docker` |
| `FRESHER_DOCKER_MEMORY` | `docker.memory` |
| `FRESHER_DOCKER_CPUS` | `docker.cpus` |

## Hooks

Customize behavior at lifecycle events with shell scripts in `.fresher/hooks/`:

| Hook | When | Use Case |
|------|------|----------|
| `hooks/started` | Loop begins | Notify team, check prerequisites |
| `hooks/next_iteration` | Each iteration | Log progress, update dashboard |
| `hooks/finished` | Loop ends | Send notification, cleanup |

### Exit Codes

- `0` - Continue normally
- `1` - Skip this iteration (next_iteration hook only)
- `2` - Abort the loop

### Environment Variables in Hooks

| Variable | Description |
|----------|-------------|
| `FRESHER_ITERATION` | Current iteration number |
| `FRESHER_TOTAL_ITERATIONS` | Total iterations completed |
| `FRESHER_TOTAL_COMMITS` | Total commits made |
| `FRESHER_DURATION` | Total duration in seconds |
| `FRESHER_FINISH_TYPE` | Exit reason: `manual`, `error`, `max_iterations`, `complete`, `no_changes` |

### Example Hook

```bash
#!/bin/bash
# hooks/finished - notify on completion

echo "Fresher loop finished"
echo "  Iterations: $FRESHER_TOTAL_ITERATIONS"
echo "  Commits: $FRESHER_TOTAL_COMMITS"
echo "  Duration: ${FRESHER_DURATION}s"
echo "  Finish type: $FRESHER_FINISH_TYPE"

# Send Slack notification
# curl -X POST "$SLACK_WEBHOOK" -d "{\"text\": \"Fresher: $FRESHER_FINISH_TYPE\"}"

exit 0
```

## Docker Isolation

For maximum safety, run Fresher in a Docker container with network restrictions and resource limits.

### Setup

1. Enable Docker in config:

```toml
[docker]
use_docker = true
memory = "4g"
cpus = "2"
```

2. Build the devcontainer:

```bash
fresher docker build
```

3. Run Fresher:

```bash
fresher docker shell
# Inside container:
fresher plan
fresher build
```

### What Docker Provides

- **Isolated filesystem** - Changes contained to mounted volume
- **Network restrictions** - Firewall limits outbound connections
- **Resource limits** - Memory and CPU constraints
- **Reproducible environment** - Consistent execution context

### Manual Docker Compose

You can also run directly with Docker Compose:

```bash
docker compose -f .fresher/docker/docker-compose.yml up
```

## Project Structure

```
project/
├── .fresher/
│   ├── config.toml           # Configuration (TOML)
│   ├── AGENTS.md             # Project-specific knowledge
│   ├── PROMPT.planning.md    # Planning mode instructions
│   ├── PROMPT.building.md    # Building mode instructions
│   ├── hooks/                # Lifecycle hooks
│   │   ├── started
│   │   ├── next_iteration
│   │   └── finished
│   └── logs/                 # Iteration logs (gitignored)
├── specs/                    # Specification files
├── IMPLEMENTATION_PLAN.md    # Generated by planning mode
└── CLAUDE.md                 # Project context
```

## Best Practices

### When to Use Planning vs Building

| Scenario | Mode | Why |
|----------|------|-----|
| Starting a new feature | Planning | Get a comprehensive plan first |
| Bug fix with known cause | Building | Jump straight to implementation |
| Large refactoring | Planning | Understand full scope before starting |
| Small tweaks | Building | Quick iterations are fine |
| Unclear requirements | Planning | Let Claude help scope the work |

### Writing Effective AGENTS.md

The `AGENTS.md` file provides project-specific context to Claude:

```markdown
## Project Commands

- Test: `npm test`
- Build: `npm run build`
- Lint: `npm run lint`

## Architecture Notes

- Uses React with TypeScript
- State management via Zustand
- API calls go through `src/api/client.ts`

## Known Issues

- The `Widget` component has a race condition (see #123)
- Don't modify `legacy/` folder without consulting team
```

### Spec-Driven Development

1. **Write specs first** - Define what you're building in `specs/`
2. **Run planning mode** - Let Claude create an implementation plan
3. **Review the plan** - Ensure it matches your intent
4. **Run building mode** - Execute with confidence

## Verification

Verify your implementation plan covers all specifications:

```bash
fresher verify
```

Output shows:
- Coverage percentage per spec
- Uncovered requirements
- Orphan tasks (no spec reference)

```bash
# JSON output for CI/CD
fresher verify --json
```

## Troubleshooting

### Loop won't terminate

**Symptoms:** Hits max iterations without completing

**Solutions:**
1. Check if tests are failing - fix failing tests
2. Reduce scope in implementation plan
3. Increase `max_iterations` if progress is being made

### Claude seems confused

**Symptoms:** Repetitive actions, not following plan

**Solutions:**
1. Check `AGENTS.md` for conflicting instructions
2. Simplify the implementation plan
3. Clear and restart with fresh context

### Permission errors

**Symptoms:** Claude can't read/write files

**Solutions:**
1. Check file permissions in project
2. If using Docker, check volume mounts
3. Verify `.fresher/` is not read-only

### Hooks not running

**Symptoms:** Custom hooks have no effect

**Solutions:**
1. Ensure hooks are executable: `chmod +x .fresher/hooks/*`
2. Check hook scripts for syntax errors
3. Verify shebang line: `#!/bin/bash`

## Contributing

See [specs/README.md](specs/README.md) for technical specifications.

## License

MIT
