# Changelog

All notable changes to Fresher will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.1.0] - 2026-01-18

### Added

- `fresher docker shell` command - opens interactive bash shell in devcontainer
- `fresher docker build` command - builds the devcontainer image
- Docker isolation enforcement - prevents running plan/build outside container when `docker.use_docker = true`
- OAuth authentication support for Claude Max/Pro plans in Docker
- 45 integration tests for init, verify, and hooks modules

### Changed

- Removed legacy Bash implementation files (5,300+ lines of unused code)
- Updated CLAUDE.md to reflect Rust architecture
- Updated specs to document Rust v2.0 architecture

### Fixed

- Flaky test in config module due to env variable pollution between parallel tests

## [2.0.0] - 2026-01-18

### Changed

- **Complete rewrite from Bash to Rust** - Fresher is now a compiled binary with improved performance, reliability, and cross-platform support
- Configuration format changed from `config.sh` (Bash) to `config.toml` (TOML)
- Commands are now subcommands of the `fresher` binary:
  - `FRESHER_MODE=planning .fresher/run.sh` → `fresher plan`
  - `FRESHER_MODE=building .fresher/run.sh` → `fresher build`
- Project structure simplified - no longer requires `.fresher-internal/` directory

### Added

- `fresher init` command with automatic project type detection
- `fresher plan` command for planning mode
- `fresher build` command for building mode
- `fresher verify` command for plan verification against specs
- `fresher upgrade` command for self-updating to latest version
- `fresher version` command for version information
- `fresher docker shell` command for interactive devcontainer shell
- `fresher docker build` command for building devcontainer image
- TOML configuration with structured sections:
  - `[fresher]` - Core settings (mode, iterations, termination)
  - `[commands]` - Test, build, lint commands
  - `[paths]` - Log, spec, and source directories
  - `[hooks]` - Hook settings
  - `[docker]` - Docker isolation settings
- Support for 10 project types: Bun, Node.js, Rust, Go, Python, Make, .NET, Maven, Gradle, Generic
- JSON output option for `fresher verify` command
- Environment variable overrides for all configuration options
- Docker isolation enforcement when `docker.use_docker = true`
- Comprehensive test suite with 120 tests (75 unit + 45 integration)

### Removed

- Bash implementation (`.fresher/run.sh`, `.fresher/lib/*.sh`, etc.)
- Shell-based configuration (`config.sh`)
- Interactive wizard mode (`--interactive` flag) - may be re-added in future
- `--no-hooks` and `--no-docker` init flags - configure via `config.toml` instead

### Migration Guide

1. **Backup your configuration**
   ```bash
   cp .fresher/config.sh .fresher/config.sh.backup
   ```

2. **Re-initialize with v2.0**
   ```bash
   fresher init --force
   ```

3. **Transfer settings to config.toml**

   Old format (`config.sh`):
   ```bash
   FRESHER_TEST_CMD="npm test"
   FRESHER_BUILD_CMD="npm run build"
   FRESHER_MAX_ITERATIONS=50
   ```

   New format (`config.toml`):
   ```toml
   [fresher]
   max_iterations = 50

   [commands]
   test = "npm test"
   build = "npm run build"
   ```

4. **Update any scripts**

   Replace:
   ```bash
   FRESHER_MODE=planning .fresher/run.sh
   ```

   With:
   ```bash
   fresher plan
   ```

5. **Hooks remain compatible** - Shell hooks in `.fresher/hooks/` work the same way

## [1.x] - Legacy Bash Implementation

The original Bash implementation provided the core Ralph Loop functionality:

- Planning and building modes via `run.sh`
- Shell-based configuration
- Lifecycle hooks
- Smart termination detection
- Docker isolation support

This version is superseded by the Rust rewrite in v2.0.0.
