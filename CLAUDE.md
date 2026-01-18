# Fresher

A portable Ralph Loop implementation for AI-driven iterative development using Claude Code.

## Project Overview

Fresher is a Rust CLI tool that implements the Ralph Loop methodology - an iterative execution model with two modes (PLANNING and BUILDING) that uses fresh context each iteration for specification-driven development.

## Specifications

**IMPORTANT:** Before implementing any feature, consult `specs/README.md`.

- **Assume NOT implemented.** Specs describe intent; code describes reality.
- **Check the codebase first.** Search actual code before concluding.
- **Use specs as guidance.** Follow design patterns in relevant spec.
- **Spec index:** `specs/README.md` lists all specs by category.

## Commands

### Development
```bash
# Run tests
cargo test

# Build release binary
cargo build --release

# Run from source
cargo run -- <command>
```

### Using Fresher
```bash
# Initialize in a project
fresher init

# Run planning mode
fresher plan

# Run building mode
fresher build

# Verify implementation coverage
fresher verify

# Check for updates
fresher upgrade --check
```

## Architecture

```
src/
├── main.rs              # Entry point
├── lib.rs               # Library exports
├── cli.rs               # Command-line parsing
├── config.rs            # Configuration (TOML)
├── state.rs             # State management
├── hooks.rs             # Lifecycle hooks
├── streaming.rs         # Claude API streaming
├── templates.rs         # Prompt templates
├── verify.rs            # Plan verification
├── upgrade.rs           # Self-upgrade
├── docker.rs            # Docker isolation
└── commands/            # Command implementations
    ├── init.rs          # fresher init
    ├── plan.rs          # fresher plan
    ├── build.rs         # fresher build
    ├── verify.rs        # fresher verify
    ├── upgrade.rs       # fresher upgrade
    ├── docker.rs        # fresher docker
    └── version.rs       # fresher version

.fresher/                # Generated project config
├── AGENTS.md            # Project-specific knowledge
├── PROMPT.planning.md   # Planning mode instructions
├── PROMPT.building.md   # Building mode instructions
├── docker/              # Docker isolation files
└── logs/                # Execution logs
```

## Key Concepts

- **Fresh context per iteration** - Each loop clears context to stay in the "smart zone"
- **Subagent delegation** - Main agent coordinates, subagents handle heavy work
- **Backpressure** - Tests and builds provide validation gates
- **Smart termination** - Detects when all tasks are complete

## Development Notes

- Written in Rust with async support
- Uses `clap` for CLI argument parsing
- Configuration stored in `.fresher/fresher.toml`
- Docker isolation is optional but recommended for dangerous permissions
- Hooks execute shell scripts for extensibility
