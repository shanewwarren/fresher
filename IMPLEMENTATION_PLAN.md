# Implementation Plan

Generated: 2025-01-17
Last Updated: 2025-01-17
Based on: specs/project-scaffold.md, specs/lifecycle-hooks.md

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

## Priority 3: Polish ← NEXT

- [ ] Add timeout to hook execution (refs: specs/lifecycle-hooks.md §4.1)
  - Dependencies: none
  - Complexity: low
  - File: `.fresher-internal/init.sh` (hook generation section)
  - Implementation:
    - Add `FRESHER_HOOK_TIMEOUT` to config.sh (default: 30s)
    - Wrap hook calls in `timeout $FRESHER_HOOK_TIMEOUT` in run.sh
    - Handle timeout exit code (124) gracefully

- [ ] Expand hook templates with full examples (refs: specs/lifecycle-hooks.md §4.2)
  - Dependencies: none
  - Complexity: low
  - File: `.fresher-internal/init.sh` (hook generation section)
  - Templates to expand:
    - `started`: Add prerequisite checks (git clean, deps installed)
    - `next_iteration`: Add notification example (terminal-notifier/notify-send)
    - `finished`: Add summary stats, optional Slack/Discord webhook example

---

## Future Work (Not Yet Planned)

These specs need implementation plans created:

| Spec | Status | Description |
|------|--------|-------------|
| plan-verification.md | Planned | Gap analysis comparing plan against specs and code |
| self-testing.md | Planned | Test scenarios to verify the loop works correctly |
| docker-isolation.md | Planned | Devcontainer integration using official Claude Code image |
