# Implementation Plan

Generated: 2025-01-17
Last Updated: 2026-01-18
Based on: specs/project-scaffold.md, specs/lifecycle-hooks.md, specs/loop-executor.md, specs/prompt-templates.md, specs/docker-isolation.md, specs/plan-verification.md, specs/self-testing.md

> **Note:** Fresher was rewritten from Bash to Rust in v2.0.0. Priorities 1-8 below reflect the original Bash implementation which has been superseded. See "Remaining Work" section for current tasks.

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

## Priority 7: Plan Verification ✅

- [x] Implement requirement extraction (refs: specs/plan-verification.md §4.1, §7 Phase 1)
  - Dependencies: none
  - Complexity: medium
  - File: `.fresher/lib/verify.sh`
  - Implementation:
    - Created `extract_requirements()` function
    - Parses `specs/*.md` files for section headers (`### Section`)
    - Extracts checkbox items (`- [ ]` and `- [x]`) with status
    - Extracts RFC 2119 keywords (skipping code blocks)
    - Output format: `spec_name|type|line_num|text` (with status for tasks)
    - Added helper functions: `count_requirements()`, `list_specs()`, `get_spec_requirements()`

- [x] Implement plan parsing (refs: specs/plan-verification.md §4.2, §7 Phase 2)
  - Dependencies: none
  - Complexity: low
  - File: `.fresher/lib/verify.sh`
  - Implementation:
    - Created `parse_plan()` function
    - Extracts tasks from IMPLEMENTATION_PLAN.md with line numbers
    - Parses status (pending/completed), description, spec references
    - Normalizes spec refs (removes `specs/` prefix for matching)
    - Output format: `status|spec_ref|line_num|description`
    - Added helpers: `count_plan_tasks()`, `get_tasks_for_spec()`, `get_orphan_tasks()`

- [x] Implement cross-reference analysis (refs: specs/plan-verification.md §4.4, §7 Phase 3)
  - Dependencies: requirement extraction, plan parsing
  - Complexity: medium
  - File: `.fresher/lib/verify.sh`
  - Implementation:
    - Created `analyze_coverage()` - outputs spec|req_count|task_count|coverage_pct
    - Created `find_uncovered_specs()` - lists specs with no plan tasks
    - Created `find_uncovered_sections()` - granular section-level coverage
    - Created `get_verification_summary()` - high-level stats for reports
    - Coverage calculated as (tasks / sections) per spec

- [x] Implement code evidence search (refs: specs/plan-verification.md §4.3, §7 Phase 4)
  - Dependencies: cross-reference analysis
  - Complexity: medium
  - File: `.fresher/lib/verify.sh`
  - Implementation:
    - Created `extract_keywords()` - filters stopwords, extracts significant terms
    - Created `find_evidence()` - searches src dirs for keyword matches
    - Created `find_all_evidence()` - finds evidence for all completed tasks
    - Supports both ripgrep (rg) and grep fallback
    - Returns file:line|matched_text format

- [x] Implement report generation (refs: specs/plan-verification.md §3.3, §7 Phase 5)
  - Dependencies: all analysis functions
  - Complexity: low
  - File: `.fresher/lib/verify.sh`
  - Implementation:
    - Created `generate_report()` function outputting markdown
    - Summary table with all metrics
    - Coverage by spec table with percentages
    - Missing coverage section listing uncovered specs
    - Orphan tasks section (if any)
    - Implementation evidence table (first 20 matches)
    - Recommendations based on analysis
    - Created `generate_report_file()` wrapper for file output

- [x] Create CLI entry point (refs: specs/plan-verification.md §5, §7 Phase 6)
  - Dependencies: report generation
  - Complexity: low
  - File: `.fresher/bin/fresher-verify`
  - Implementation:
    - All command-line options implemented:
      - `--spec-dir`, `--plan-file`, `--src-dir`, `--output`
      - `--format` (markdown/json)
      - `--quiet`/`-q` (summary only)
      - `--strict` (exit 1 if issues)
      - `--help`/`-h`
    - Sources verify.sh library
    - Exit codes: 0=ok, 1=issues (strict), 2=error
    - JSON output for both quiet and full modes

- [x] Add fresher verify command routing (refs: specs/plan-verification.md §5)
  - Dependencies: CLI entry point
  - Complexity: low
  - File: `bin/fresher`
  - Implementation:
    - Added `verify` subcommand to main fresher CLI
    - Added `cmd_verify()` function that routes to `.fresher/bin/fresher-verify`
    - Added verify options and examples to help text

## Priority 8: Self-Testing ✅

- [x] Create test runner script (refs: specs/self-testing.md §4.1, §7 Phase 1)
  - Dependencies: none
  - Complexity: low
  - File: `.fresher/tests/run-tests.sh`
  - Implementation:
    - Create tests/ directory structure (unit/, integration/, fixtures/, mocks/)
    - Implement setup_test_env() - creates temp dir, copies fixtures, sets PATH
    - Implement teardown_test_env() - cleanup temp dir
    - Implement run_test() - executes test file, tracks pass/fail, times execution
    - Main function runs unit then integration tests
    - Exit 1 if any tests fail, 0 otherwise

- [x] Create mock Claude CLI (refs: specs/self-testing.md §4.2, §7 Phase 2)
  - Dependencies: none
  - Complexity: medium
  - File: `.fresher/tests/mocks/mock-claude.sh`
  - Implementation:
    - Parse Claude CLI arguments (-p, --output-format, --max-turns, etc.)
    - Support MOCK_CLAUDE_MODE env var (success, no_changes, error, timeout)
    - Support MOCK_CLAUDE_DELAY env var for timing tests
    - Output stream-json format when requested
    - Simulate commits in success mode when git repo exists
    - Symlink as 'claude' in test PATH

- [x] Create test utilities library (refs: specs/self-testing.md §6, §7 Phase 3)
  - Dependencies: none
  - Complexity: low
  - File: `.fresher/lib/test-utils.sh`
  - Implementation:
    - assert_equals() - compare actual vs expected values
    - assert_contains() - check string contains substring
    - assert_file_exists() - verify file presence
    - assert_exit_code() - run command and check exit code
    - create_mock_project() - scaffolds test project structure
    - create_mock_plan() - generates sample IMPLEMENTATION_PLAN.md

- [x] Create test fixtures (refs: specs/self-testing.md §3.3, §7 Phase 3)
  - Dependencies: none
  - Complexity: low
  - File: `.fresher/tests/fixtures/`
  - Implementation:
    - mock-project/ with src/, specs/, package.json, CLAUDE.md
    - sample-specs/ with README.md and feature.md
    - sample-plan.md with pending and completed tasks

- [x] Implement unit tests (refs: specs/self-testing.md §4.3, §7 Phase 4)
  - Dependencies: test runner, mock CLI, test utilities
  - Complexity: medium
  - Files: `.fresher/tests/unit/test-*.sh`
  - Implementation:
    - test-config.sh - config loading and environment overrides
    - test-termination.sh - max iterations, smart termination detection
    - test-hooks.sh - hook execution, environment passing, exit codes
    - test-verify.sh - verification functions (from Priority 7)

- [x] Implement integration tests (refs: specs/self-testing.md §4.4, §7 Phase 5)
  - Dependencies: test runner, mock CLI, test utilities
  - Complexity: medium
  - Files: `.fresher/tests/integration/test-*.sh`
  - Implementation:
    - test-planning-mode.sh - full planning loop with mock Claude
    - test-building-mode.sh - building loop with commit tracking
    - test-max-iterations.sh - termination at iteration limit
    - test-smart-termination.sh - termination when tasks complete

- [x] Add fresher test command (refs: specs/self-testing.md §5, §7 Phase 6)
  - Dependencies: all test phases
  - Complexity: low
  - File: `bin/fresher`
  - Implementation:
    - Add `test` subcommand to main fresher CLI
    - Options: --unit, --integration, --verbose, --filter, --timeout
    - Route to `.fresher/tests/run-tests.sh`
    - Add help text for test command

---

## Rust v2.0 Implementation Status ✅

The following features have been fully implemented in Rust:

| Component | Rust Files | Status |
|-----------|-----------|--------|
| CLI & Commands | `src/cli.rs`, `src/commands/*.rs` | ✅ Complete |
| Init Command | `src/commands/init.rs` | ✅ Complete |
| Plan/Build Commands | `src/commands/plan.rs`, `src/commands/build.rs` | ✅ Complete |
| Verify Command | `src/commands/verify.rs`, `src/verify.rs` | ✅ Complete |
| Upgrade Command | `src/commands/upgrade.rs`, `src/upgrade.rs` | ✅ Complete |
| Streaming Output | `src/streaming.rs` | ✅ Complete |
| Hooks System | `src/hooks.rs` | ✅ Complete |
| Configuration | `src/config.rs` | ✅ Complete |
| State Management | `src/state.rs` | ✅ Complete |
| Templates | `src/templates.rs` | ✅ Complete |

---

## Remaining Work

### Priority 9: Docker Isolation Execution

- [x] Wire Docker config to loop execution (refs: specs/docker-isolation.md §5.1)
  - Dependencies: Docker config exists in `src/config.rs`
  - Complexity: low
  - Implementation:
    - Created `src/docker.rs` module with container detection
    - Checks `DEVCONTAINER=true` or `FRESHER_IN_DOCKER=true` env vars
    - Added `enforce_docker_isolation()` to plan/build commands
    - Shows informative message with options if Docker required but not in container
    - Exits with error (status 1) to prevent execution outside container

- [ ] Add `fresher docker` subcommand (refs: specs/docker-isolation.md §10)
  - Dependencies: Docker execution
  - Complexity: low
  - Implementation needed:
    - `fresher docker shell` - Open interactive shell in container
    - `fresher docker build` - Build the devcontainer image

### Priority 10: Rust Testing

- [ ] Add unit tests for core modules (refs: specs/self-testing.md)
  - Dependencies: none
  - Complexity: medium
  - Files to test:
    - `src/config.rs` - Config loading and env overrides
    - `src/verify.rs` - Spec parsing and coverage analysis
    - `src/hooks.rs` - Hook execution and exit codes
    - `src/streaming.rs` - Stream JSON parsing

- [ ] Add integration tests (refs: specs/self-testing.md)
  - Dependencies: unit tests
  - Complexity: medium
  - Tests needed:
    - `fresher init` creates correct structure
    - `fresher verify` produces correct report
    - Hook timeout and abort behavior

### Priority 11: Documentation

- [ ] Update README.md for v2.0 (refs: specs/documentation.md)
  - Dependencies: none
  - Complexity: medium
  - Content needed:
    - Installation (cargo install, binary download)
    - Quick start with new commands
    - Configuration (TOML format)
    - Docker isolation setup

- [ ] Add CHANGELOG.md
  - Dependencies: none
  - Complexity: low
  - Content: v2.0.0 release notes (Rust rewrite)

### Priority 12: Spec Updates

- [ ] Rewrite loop-executor.md for Rust architecture
  - Dependencies: none
  - Complexity: medium
  - Changes: Remove bash references, document Rust implementation

- [ ] Rewrite project-scaffold.md for Rust architecture
  - Dependencies: none
  - Complexity: medium
  - Changes: Document `fresher init` command, TOML config

- [ ] Rewrite plan-verification.md for Rust architecture
  - Dependencies: none
  - Complexity: low
  - Changes: Document `fresher verify` command

- [ ] Rewrite installer.md for Rust architecture
  - Dependencies: none
  - Complexity: low
  - Changes: Document `fresher upgrade` and GitHub releases

- [ ] Update self-testing.md for Rust testing
  - Dependencies: Rust tests implemented
  - Complexity: low
  - Changes: Replace bash test examples with Rust test examples
