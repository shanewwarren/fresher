# Fresher

> Portable Ralph Loop implementation for AI-driven iterative development

Fresher brings the Ralph Loop methodology to any project. Run Claude Code in iterative loops with fresh context, specification-driven development, and automatic termination detection.

## What is the Ralph Loop?

The Ralph Loop is an iterative execution model with two modes:

- **PLANNING** - Generate implementation plans from specifications
- **BUILDING** - Execute plans with test/build backpressure

Each iteration starts with fresh context, keeping Claude in the "smart zone" for optimal performance.

## Quick Start

### Installation

```bash
curl -fsSL https://raw.githubusercontent.com/shanewwarren/fresher/main/install.sh | bash
```

Or inspect before running:

```bash
curl -fsSL https://raw.githubusercontent.com/shanewwarren/fresher/main/install.sh -o install.sh
cat install.sh
bash install.sh
```

### Your First Loop

1. **Planning mode** - Create an implementation plan:
   ```bash
   FRESHER_MODE=planning .fresher/run.sh
   ```

2. **Review the plan** - Check `IMPLEMENTATION_PLAN.md`

3. **Building mode** - Execute the plan:
   ```bash
   FRESHER_MODE=building .fresher/run.sh
   ```

The loop runs until all tasks are complete or max iterations reached.

## Modes

### Planning Mode

Planning mode generates an `IMPLEMENTATION_PLAN.md` based on your specifications.

```bash
FRESHER_MODE=planning .fresher/run.sh
```

**What happens:**
- Claude reads specs from `specs/` directory
- Generates prioritized task list
- Creates implementation plan with phases
- Terminates when plan is complete

**Best for:** Starting new features, refactoring, understanding scope

### Building Mode

Building mode executes tasks from the implementation plan.

```bash
FRESHER_MODE=building .fresher/run.sh
```

**What happens:**
- Claude reads the implementation plan
- Works through tasks iteratively
- Runs tests/builds as backpressure
- Terminates when all tasks complete

**Best for:** Implementing features, fixing bugs, making changes

## Configuration

### config.sh Options

| Variable | Description | Default |
|----------|-------------|---------|
| `FRESHER_TEST_CMD` | Command to run tests | Auto-detected |
| `FRESHER_BUILD_CMD` | Command to build project | Auto-detected |
| `FRESHER_LINT_CMD` | Command to run linter | `""` |
| `FRESHER_SRC_DIR` | Source directory | Auto-detected |
| `FRESHER_MAX_ITERATIONS` | Max loop iterations | `50` |
| `FRESHER_LOG_LEVEL` | Log verbosity (debug/info/warn) | `info` |

### Environment Variables

Override config.sh values at runtime:

```bash
FRESHER_MAX_ITERATIONS=10 FRESHER_MODE=building .fresher/run.sh
```

### Hooks

Customize behavior at lifecycle events:

| Hook | When | Use Case |
|------|------|----------|
| `hooks/started` | Loop begins | Notify team, check prerequisites |
| `hooks/next_iteration` | Each iteration | Log progress, update dashboard |
| `hooks/finished` | Loop ends | Send notification, cleanup |

Example hook:

```bash
#!/usr/bin/env bash
# hooks/finished - notify on completion
curl -X POST "$SLACK_WEBHOOK" -d '{"text":"Fresher loop completed!"}'
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

This workflow ensures Claude understands your intent before writing code.

## Docker Isolation

For maximum safety, run Fresher in a Docker container:

```bash
docker compose -f .fresher/docker/docker-compose.yml up
```

This provides:
- Isolated filesystem
- Network restrictions
- Resource limits
- Reproducible environment

See [docker-isolation.md](specs/docker-isolation.md) for details.

## Upgrading

Check for updates:

```bash
.fresher/bin/fresher version
```

Upgrade to latest:

```bash
.fresher/bin/fresher upgrade
```

**What's preserved during upgrade:**
- `config.sh` values (test/build commands, etc.)
- `AGENTS.md` content
- `hooks/*` customizations

**What's replaced:**
- Core scripts (`run.sh`, `lib/*`, `bin/*`)
- Prompt templates (`PROMPT.*.md`)
- Docker configuration
- Test framework

## Troubleshooting

### Loop won't terminate

**Symptoms:** Hits max iterations without completing

**Solutions:**
1. Check if tests are failing - fix failing tests
2. Reduce scope in implementation plan
3. Increase `FRESHER_MAX_ITERATIONS` if progress is being made

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
3. Verify shebang line: `#!/usr/bin/env bash`

## Project Structure

```
project/
├── .fresher/
│   ├── run.sh                 # Main loop executor
│   ├── VERSION                # Installed version
│   ├── config.sh              # Configuration
│   ├── PROMPT.planning.md     # Planning mode instructions
│   ├── PROMPT.building.md     # Building mode instructions
│   ├── AGENTS.md              # Project-specific knowledge
│   ├── hooks/                 # Lifecycle hooks
│   │   ├── started
│   │   ├── next_iteration
│   │   └── finished
│   ├── lib/                   # Supporting scripts
│   ├── bin/                   # CLI tools
│   ├── docker/                # Docker isolation
│   └── logs/                  # Iteration logs (gitignored)
├── specs/                     # Specification files
├── IMPLEMENTATION_PLAN.md     # Generated by planning mode
└── CLAUDE.md                  # Project context
```

## Contributing

See [specs/README.md](specs/README.md) for technical specifications.

## License

MIT
