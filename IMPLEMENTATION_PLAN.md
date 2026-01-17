# Implementation Plan

Generated: 2025-01-17
Last Updated: 2026-01-17
Based on: specs/project-scaffold.md, specs/lifecycle-hooks.md, specs/loop-executor.md, specs/prompt-templates.md, specs/docker-isolation.md

## Priority 1: fresher init Command ✅

- [x] Create `bin/fresher` CLI entry point (refs: specs/project-scaffold.md §4.4)
  - Dependencies: none
  - Complexity: low
  - Notes: Global wrapper script that detects .fresher/ and routes commands

- [x] Implement project type detection (refs: specs/project-scaffold.md §4.1)
  - Dependencies: none
  - Complexity: low
  - Notes: Detect package.json, Cargo.toml, go.mod, pyproject.toml, etc.

- [x] Implement `fresher init` basic scaffolding (refs: specs/project-scaffold.md §2)
  - Dependencies: project type detection
  - Complexity: medium
  - Notes: Copy/generate .fresher/ structure with detected defaults

- [x] Add .gitignore and CLAUDE.md integration (refs: specs/project-scaffold.md §4.3)
  - Dependencies: basic scaffolding
  - Complexity: low
  - Notes: Add .fresher/logs/ to gitignore, inject specs section into CLAUDE.md

## Priority 2: Interactive Setup ✅

- [x] Add remaining init command flags (refs: specs/project-scaffold.md §3.2)
  - Dependencies: none
  - Complexity: low
  - File: `.fresher-internal/init.sh`
  - Flags added:
    - `--interactive, -i` - Enable interactive wizard mode
    - `--no-hooks` - Skip creating hook scripts
    - `--no-docker` - Skip Docker-related config entries
  - Previously implemented: `--force`, `--project-type`

- [x] Implement interactive wizard (refs: specs/project-scaffold.md §4.2)
  - Dependencies: --interactive flag
  - Complexity: medium
  - File: `.fresher-internal/init.sh`
  - Prompts implemented (with detected defaults):
    - Test command (e.g., `bun test`)
    - Build command (e.g., `bun run build`)
    - Lint command (e.g., `bun run lint`)
    - Source directory (e.g., `src/`)
    - Enable Docker isolation? (y/N)
    - Max iterations (0=unlimited)
  - Uses `read -p` for prompts with defaults in brackets
  - Shows detected project type before prompts
  - Note: nodejs defaults changed from npm to bun

## Priority 3: Polish ✅

- [x] Add timeout to hook execution (refs: specs/lifecycle-hooks.md §4.1)
  - Dependencies: none
  - Complexity: low
  - File: `.fresher-internal/init.sh` (hook generation section)
  - Implementation:
    - Added `FRESHER_HOOK_TIMEOUT` to config.sh (default: 30s)
    - Added `FRESHER_HOOKS_ENABLED` to config.sh (default: true)
    - Note: Actual timeout wrapping in run.sh will be part of loop-executor phase

- [x] Expand hook templates with full examples (refs: specs/lifecycle-hooks.md §4.2)
  - Dependencies: none
  - Complexity: low
  - File: `.fresher-internal/init.sh` (hook generation section)
  - Templates expanded:
    - `started`: Prerequisite checks (IMPLEMENTATION_PLAN.md, git uncommitted changes), notification example
    - `next_iteration`: Previous iteration stats, skip logic example, desktop notifications (macOS/Linux)
    - `finished`: Summary stats with box drawing, case statement for all finish types, Slack/Discord webhook example

## Priority 4: Loop Executor ✅

- [x] Implement basic loop with manual termination (refs: specs/loop-executor.md §4, §7 Phase 1)
  - Dependencies: none
  - Complexity: low
  - File: `.fresher/run.sh`
  - Implementation:
    - Load config.sh and initialize state variables
    - Export all hook environment variables (FRESHER_ITERATION, FRESHER_LAST_EXIT_CODE, etc.)
    - Validate FRESHER_MODE, prompt file existence, and claude CLI availability
    - `run_hook()` helper respects FRESHER_HOOKS_ENABLED, handles exit codes (0=continue, 1=skip, 2=abort)
    - Run hooks/started before loop, hooks/next_iteration before each Claude invocation
    - Build Claude command with: -p, --append-system-prompt-file, --dangerously-skip-permissions, --max-turns, --model, --no-session-persistence
    - Track commit counts per iteration using git rev-list
    - Trap SIGINT/SIGTERM → FINISH_TYPE="manual", trap EXIT → cleanup() runs hooks/finished
    - Check for error exit codes from Claude

- [x] Implement output streaming and logging (refs: specs/loop-executor.md §4.4, §7 Phase 2)
  - Dependencies: basic loop
  - Complexity: medium
  - File: `.fresher/run.sh`
  - Implementation:
    - Added `--output-format stream-json` to Claude command
    - Added jq dependency validation
    - Implemented streaming parser with real-time display of assistant messages
    - Added support for `content_block_delta` events for streaming text
    - All events logged to `.fresher/logs/iteration-{n}.log`
    - Used `set -o pipefail` for proper exit code handling through pipes

- [x] Add max iterations termination (refs: specs/loop-executor.md §4.2, §7 Phase 3)
  - Dependencies: basic loop
  - Complexity: low
  - File: `.fresher/run.sh`
  - Implementation:
    - Check `$ITERATION -ge $FRESHER_MAX_ITERATIONS` (when non-zero)
    - Set finish_type to "max_iterations" and exit cleanly

- [x] Implement smart termination detection (refs: specs/loop-executor.md §4.2, §7 Phase 4)
  - Dependencies: output streaming
  - Complexity: medium
  - File: `.fresher/run.sh`
  - Implementation:
    - Respects `FRESHER_SMART_TERMINATION` config (default: true)
    - Parses IMPLEMENTATION_PLAN.md for uncompleted tasks (`^\s*-\s*\[\s\]`)
    - Sets `finish_type="complete"` when all tasks done
    - Checks if HEAD moved since iteration started
    - Sets `finish_type="no_changes"` to prevent infinite loops

- [x] Add signal handling and cleanup (refs: specs/loop-executor.md §4.3, §7 Phase 5)
  - Dependencies: basic loop
  - Complexity: medium
  - File: `.fresher/run.sh`
  - Implementation:
    - Added `write_state()` function to persist state to `.fresher/.state`
    - State file includes: ITERATION, LAST_EXIT_CODE, LAST_COMMIT_SHA, STARTED_AT, TOTAL_COMMITS, DURATION, FINISH_TYPE
    - Called from `cleanup()` before running finished hook
    - EXIT trap ensures cleanup always runs
    - `.fresher/.state` already in .gitignore

- [x] Implement hook timeout wrapping (refs: specs/lifecycle-hooks.md §4.1)
  - Dependencies: basic loop
  - Complexity: low
  - File: `.fresher/run.sh`
  - Implementation:
    - Updated `run_hook()` to wrap execution in timeout
    - Supports GNU `timeout`, macOS `gtimeout`, or graceful fallback
    - Handles timeout exit code (124) with warning, continues execution
    - Uses `FRESHER_HOOK_TIMEOUT` config (default: 30s)

## Priority 5: Prompt Templates ✅

- [x] Create full PLANNING mode template (refs: specs/prompt-templates.md §3)
  - Dependencies: none
  - Complexity: low
  - File: `.fresher/PROMPT.planning.md`
  - Implementation:
    - Replaced stub with full template from spec
    - Includes gap analysis process, constraints, output format

- [x] Create full BUILDING mode template (refs: specs/prompt-templates.md §4)
  - Dependencies: none
  - Complexity: low
  - File: `.fresher/PROMPT.building.md`
  - Implementation:
    - Replaced stub with full template from spec
    - Includes task selection, validation, commit workflow

- [x] Update init.sh to generate full templates (refs: specs/prompt-templates.md §7)
  - Dependencies: full templates created
  - Complexity: low
  - File: `.fresher-internal/init.sh`
  - Implementation:
    - Updated PROMPT.planning.md generation
    - Updated PROMPT.building.md generation

## Priority 6: Docker Isolation ✅

- [x] Create Fresher devcontainer.json (refs: specs/docker-isolation.md §4.1, §8 Phase 1)
  - Dependencies: none
  - Complexity: low
  - File: `.fresher/docker/devcontainer.json`
  - Implementation:
    - Create `.fresher/docker/` directory
    - Generate devcontainer.json that uses official Claude Code image
    - Configure volume mounts for bash history and Claude config
    - Set container environment variables (FRESHER_IN_DOCKER, DEVCONTAINER)
    - Add NET_ADMIN/NET_RAW capabilities for firewall

- [x] Create docker-compose.yml for CLI workflow (refs: specs/docker-isolation.md §5.2, §8 Phase 2)
  - Dependencies: devcontainer.json
  - Complexity: low
  - File: `.fresher/docker/docker-compose.yml`
  - Implementation:
    - Use official ghcr.io/anthropics/claude-code-devcontainer image
    - Configure resource limits (memory, CPU)
    - Set up volume mounts matching devcontainer
    - Pass through FRESHER_MODE and other env vars
    - Run firewall init and .fresher/run.sh

- [x] Add Docker detection logic to run.sh (refs: specs/docker-isolation.md §5.1, §8 Phase 3)
  - Dependencies: loop-executor (complete)
  - Complexity: low
  - File: `.fresher/run.sh`
  - Implementation:
    - Check FRESHER_USE_DOCKER config
    - Detect if already in devcontainer (DEVCONTAINER or FRESHER_IN_DOCKER)
    - If Docker enabled but not in container, show instructions and exit
    - Continue normal execution if in container or Docker disabled

- [x] Update init.sh to generate Docker files (refs: specs/docker-isolation.md §8)
  - Dependencies: devcontainer.json, docker-compose.yml created
  - Complexity: low
  - File: `.fresher-internal/init.sh`
  - Implementation:
    - Create `.fresher/docker/` directory during init
    - Generate devcontainer.json with project-specific settings
    - Generate docker-compose.yml
    - Generate fresher-firewall-overlay.sh
    - Skip if --no-docker flag is set

- [x] Create firewall overlay script template (refs: specs/docker-isolation.md §5.3, §8 Phase 4)
  - Dependencies: devcontainer.json
  - Complexity: low
  - File: `.fresher/docker/fresher-firewall-overlay.sh`
  - Implementation:
    - Template script for adding custom domains to whitelist
    - Commented out by default
    - Documentation for common use cases (private npm, internal APIs)

---

## Future Work (Not Yet Planned)

These specs need implementation plans created:

| Spec | Status | Description |
|------|--------|-------------|
| plan-verification.md | Planned | Gap analysis comparing plan against specs and code |
| self-testing.md | Planned | Test scenarios to verify the loop works correctly |
