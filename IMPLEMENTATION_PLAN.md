# Implementation Plan

Generated: 2025-01-17
Based on: specs/project-scaffold.md

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

## Priority 2: Interactive Setup ← NEXT

- [ ] Add remaining init command flags (refs: specs/project-scaffold.md §3.2)
  - Dependencies: none
  - Complexity: low
  - File: `.fresher-internal/init.sh`
  - Flags to add:
    - `--interactive, -i` - Enable interactive wizard mode
    - `--no-hooks` - Skip creating hook scripts
    - `--no-docker` - Skip Docker-related config entries
  - Already implemented: `--force`, `--project-type`

- [ ] Implement interactive wizard (refs: specs/project-scaffold.md §4.2)
  - Dependencies: --interactive flag
  - Complexity: medium
  - File: `.fresher-internal/init.sh`
  - Prompts to implement (with detected defaults):
    - Test command (e.g., `npm test`)
    - Build command (e.g., `npm run build`)
    - Lint command (e.g., `npm run lint`)
    - Source directory (e.g., `src/`)
    - Enable Docker isolation? (y/N)
    - Max iterations (0=unlimited)
  - Use `read -p` for prompts with defaults in brackets
  - Show detected project type before prompts

## Priority 3: Polish

- [ ] Add timeout to hook execution (refs: specs/lifecycle-hooks.md §4.1)
  - Dependencies: none
  - Complexity: low
  - Notes: Use `timeout` command with FRESHER_HOOK_TIMEOUT config

- [ ] Expand hook templates with full examples (refs: specs/lifecycle-hooks.md §4.2)
  - Dependencies: none
  - Complexity: low
  - Notes: Add prerequisite checks, notification examples from spec
