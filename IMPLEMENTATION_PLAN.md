# Implementation Plan

Generated: 2025-01-17
Last Updated: 2026-01-17
Based on: specs/project-scaffold.md, specs/lifecycle-hooks.md, specs/loop-executor.md, specs/prompt-templates.md

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

## Priority 4: Loop Executor (In Progress)

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

- [ ] Implement output streaming and logging (refs: specs/loop-executor.md §4.4, §7 Phase 2) ← NEXT
  - Dependencies: basic loop
  - Complexity: medium
  - File: `.fresher/run.sh`
  - Implementation:
    - Use `--output-format stream-json` with Claude Code
    - Parse JSON events and display assistant messages in real-time
    - Log all events to `.fresher/logs/iteration-{n}.log`

- [ ] Add max iterations termination (refs: specs/loop-executor.md §4.2, §7 Phase 3)
  - Dependencies: basic loop
  - Complexity: low
  - File: `.fresher/run.sh`
  - Implementation:
    - Check `$ITERATION -ge $FRESHER_MAX_ITERATIONS` (when non-zero)
    - Set finish_type to "max_iterations"

- [ ] Implement smart termination detection (refs: specs/loop-executor.md §4.2, §7 Phase 4)
  - Dependencies: output streaming
  - Complexity: medium
  - File: `.fresher/run.sh`
  - Implementation:
    - Parse IMPLEMENTATION_PLAN.md for uncompleted tasks (`- [ ]`)
    - Check if HEAD moved since iteration started (no_changes detection)
    - Set appropriate finish_type (complete, no_changes)

- [ ] Add signal handling and cleanup (refs: specs/loop-executor.md §4.3, §7 Phase 5)
  - Dependencies: basic loop
  - Complexity: medium
  - File: `.fresher/run.sh`
  - Implementation:
    - Trap EXIT for cleanup
    - Write final state to `.fresher/.state`
    - Ensure hooks/finished always runs

- [ ] Implement hook timeout wrapping (refs: specs/lifecycle-hooks.md §4.1)
  - Dependencies: basic loop
  - Complexity: low
  - File: `.fresher/run.sh`
  - Implementation:
    - Wrap hook calls in `timeout $FRESHER_HOOK_TIMEOUT`
    - Handle timeout exit code (124) gracefully
    - Respect FRESHER_HOOKS_ENABLED flag

## Priority 5: Prompt Templates

- [ ] Create full PLANNING mode template (refs: specs/prompt-templates.md §3)
  - Dependencies: none
  - Complexity: low
  - File: `.fresher/PROMPT.planning.md`
  - Implementation:
    - Replace stub with full template from spec
    - Include gap analysis process, constraints, output format

- [ ] Create full BUILDING mode template (refs: specs/prompt-templates.md §4)
  - Dependencies: none
  - Complexity: low
  - File: `.fresher/PROMPT.building.md`
  - Implementation:
    - Replace stub with full template from spec
    - Include task selection, validation, commit workflow

- [ ] Update init.sh to generate full templates (refs: specs/prompt-templates.md §7)
  - Dependencies: full templates created
  - Complexity: low
  - File: `.fresher-internal/init.sh`
  - Implementation:
    - Update PROMPT.planning.md generation
    - Update PROMPT.building.md generation

---

## Future Work (Not Yet Planned)

These specs need implementation plans created:

| Spec | Status | Description |
|------|--------|-------------|
| plan-verification.md | Planned | Gap analysis comparing plan against specs and code |
| self-testing.md | Planned | Test scenarios to verify the loop works correctly |
| docker-isolation.md | Planned | Devcontainer integration using official Claude Code image |
