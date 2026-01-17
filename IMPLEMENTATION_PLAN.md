# Implementation Plan

Generated: 2025-01-17
Based on: specs/project-scaffold.md

## Priority 1: fresher init Command

- [ ] Create `bin/fresher` CLI entry point (refs: specs/project-scaffold.md §4.4)
  - Dependencies: none
  - Complexity: low
  - Notes: Global wrapper script that detects .fresher/ and routes commands

- [ ] Implement project type detection (refs: specs/project-scaffold.md §4.1)
  - Dependencies: none
  - Complexity: low
  - Notes: Detect package.json, Cargo.toml, go.mod, pyproject.toml, etc.

- [ ] Implement `fresher init` basic scaffolding (refs: specs/project-scaffold.md §2)
  - Dependencies: project type detection
  - Complexity: medium
  - Notes: Copy/generate .fresher/ structure with detected defaults

- [ ] Add .gitignore and CLAUDE.md integration (refs: specs/project-scaffold.md §4.3)
  - Dependencies: basic scaffolding
  - Complexity: low
  - Notes: Add .fresher/logs/ to gitignore, inject specs section into CLAUDE.md

## Priority 2: Interactive Setup

- [ ] Implement interactive wizard (refs: specs/project-scaffold.md §4.2)
  - Dependencies: basic scaffolding
  - Complexity: medium
  - Notes: Prompt for test/build/lint commands, source dir, docker preference

- [ ] Add init command flags (refs: specs/project-scaffold.md §3.2)
  - Dependencies: basic scaffolding
  - Complexity: low
  - Notes: --interactive, --force, --no-hooks, --no-docker, --project-type

## Priority 3: Polish

- [ ] Add timeout to hook execution (refs: specs/lifecycle-hooks.md §4.1)
  - Dependencies: none
  - Complexity: low
  - Notes: Use `timeout` command with FRESHER_HOOK_TIMEOUT config

- [ ] Expand hook templates with full examples (refs: specs/lifecycle-hooks.md §4.2)
  - Dependencies: none
  - Complexity: low
  - Notes: Add prerequisite checks, notification examples from spec
