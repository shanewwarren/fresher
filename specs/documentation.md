# Documentation Specification

**Status:** Planned
**Version:** 1.0
**Last Updated:** 2026-01-17

---

## 1. Overview

### Purpose

Comprehensive documentation enables users to install, configure, and effectively use Fresher. The README.md serves as the primary entry point, covering installation, basic usage, configuration, and best practices for running the Ralph Loop methodology.

### Goals

- **Quick start** - Users can install and run their first loop in under 5 minutes
- **Complete reference** - All configuration options and features documented
- **Best practices** - Guidance on effective use of planning vs building modes
- **Troubleshooting** - Common issues and solutions

### Non-Goals

- **Tutorial content** - Step-by-step project walkthroughs (defer to blog posts)
- **Video documentation** - Text-based docs only
- **API reference** - Fresher is a CLI tool, not a library

---

## 2. Architecture

### Documentation Structure

```
/
├── README.md                 # Primary documentation (comprehensive)
├── CLAUDE.md                 # Project context for Claude Code
├── LICENSE                   # License file
└── specs/
    └── README.md             # Spec index (technical, for contributors)
```

### README Sections

```
README.md
├── Header (logo, badges, tagline)
├── What is Fresher?
├── Quick Start
│   ├── Installation
│   └── Your First Loop
├── Modes
│   ├── Planning Mode
│   └── Building Mode
├── Configuration
│   ├── config.sh Options
│   ├── Environment Variables
│   └── Hooks
├── Best Practices
│   ├── When to Use Planning vs Building
│   ├── Writing Effective AGENTS.md
│   └── Spec-Driven Development
├── Docker Isolation
├── Troubleshooting
├── Upgrading
├── Contributing
└── License
```

---

## 3. Content Specifications

### 3.1 Header Section

```markdown
# Fresher

> Portable Ralph Loop implementation for AI-driven iterative development

[![Version](https://img.shields.io/badge/version-1.0.0-blue.svg)]()
[![License](https://img.shields.io/badge/license-MIT-green.svg)]()

Fresher brings the Ralph Loop methodology to any project. Run Claude Code in
iterative loops with fresh context, specification-driven development, and
automatic termination detection.
```

### 3.2 Quick Start Section

```markdown
## Quick Start

### Installation

```bash
curl -fsSL https://raw.githubusercontent.com/{org}/fresher/main/install.sh | bash
```

Or inspect before running:

```bash
curl -fsSL https://raw.githubusercontent.com/{org}/fresher/main/install.sh -o install.sh
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
```

### 3.3 Modes Section

```markdown
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
```

### 3.4 Configuration Section

```markdown
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
```

### 3.5 Best Practices Section

```markdown
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
```

### 3.6 Docker Isolation Section

```markdown
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
```

### 3.7 Troubleshooting Section

```markdown
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
```

### 3.8 Upgrading Section

```markdown
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
```

---

## 4. Implementation Phases

| Phase | Description | Dependencies | Complexity |
|-------|-------------|--------------|------------|
| 1 | Create README.md structure | None | Low |
| 2 | Write Quick Start and Modes sections | Phase 1 | Medium |
| 3 | Write Configuration and Best Practices | Phase 2 | Medium |
| 4 | Write Troubleshooting and Upgrading | Phase 3 | Low |
| 5 | Add badges and polish | Phase 4 | Low |

---

## 5. Open Questions

- [ ] Should we add a CHANGELOG.md?
- [ ] Include animated GIF/asciicast demo in README?
- [ ] Create a separate CONTRIBUTING.md file?
