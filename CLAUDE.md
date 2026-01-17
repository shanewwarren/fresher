# Fresher

A portable Ralph Loop implementation for AI-driven iterative development using Claude Code.

## Project Overview

Fresher provides the bash infrastructure to run the Ralph Loop methodology - an iterative execution model with two modes (PLANNING and BUILDING) that uses fresh context each iteration for specification-driven development.

## Specifications

**IMPORTANT:** Before implementing any feature, consult `specs/README.md`.

- **Assume NOT implemented.** Specs describe intent; code describes reality.
- **Check the codebase first.** Search actual code before concluding.
- **Use specs as guidance.** Follow design patterns in relevant spec.
- **Spec index:** `specs/README.md` lists all specs by category.

## Commands

### Testing
```bash
# Run self-tests (once implemented)
.fresher/tests/run-tests.sh
```

### Running Fresher
```bash
# Planning mode
FRESHER_MODE=planning .fresher/run.sh

# Building mode
FRESHER_MODE=building .fresher/run.sh
```

## Architecture

```
.fresher/
├── run.sh                 # Main loop executor
├── config.sh              # Configuration
├── PROMPT.planning.md     # Planning mode instructions
├── PROMPT.building.md     # Building mode instructions
├── AGENTS.md              # Project-specific knowledge
├── hooks/                 # Lifecycle hooks
├── lib/                   # Supporting scripts
├── docker/                # Docker isolation files
└── tests/                 # Self-testing
```

## Key Concepts

- **Fresh context per iteration** - Each loop clears context to stay in the "smart zone"
- **Subagent delegation** - Main agent coordinates, subagents handle heavy work
- **Backpressure** - Tests and builds provide validation gates
- **Smart termination** - Detects when all tasks are complete

## Development Notes

- This is a bash-based CLI tool
- Primary entry point is `.fresher/run.sh`
- Docker isolation is optional but recommended for dangerous permissions
- Hooks allow extensibility without modifying core scripts
