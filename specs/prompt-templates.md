# Prompt Templates Specification

**Status:** In Progress (stubs exist)
**Version:** 1.0
**Last Updated:** 2025-01-17

---

## 1. Overview

### Purpose

Prompt templates are the mode-specific instructions that shape Claude's behavior during each iteration. The PLANNING prompt drives gap analysis and plan creation, while the BUILDING prompt drives task implementation. Swapping the prompt file is what changes the loop's behavior.

### Goals

- **Clear mode distinction** - Planning produces plans, building produces code
- **Consistent structure** - Both prompts follow similar patterns for predictability
- **Actionable instructions** - Claude knows exactly what to do each iteration
- **Backpressure integration** - Building mode enforces test/build validation

### Non-Goals

- **Project-specific instructions** - Those belong in AGENTS.md
- **Tool configuration** - Handled by Claude Code flags
- **Multi-language support** - English only initially

---

## 2. Architecture

### File Structure

```
.fresher/
├── PROMPT.planning.md    # Planning mode instructions
├── PROMPT.building.md    # Building mode instructions
└── AGENTS.md             # Project-specific knowledge (always loaded)
```

### Loading Order

```
┌──────────────────────────────────────────────────────────────┐
│  Claude Code Invocation                                       │
├──────────────────────────────────────────────────────────────┤
│  1. System prompt (Claude Code default)                      │
│  2. --append-system-prompt-file .fresher/AGENTS.md           │
│  3. -p "$(cat .fresher/PROMPT.{mode}.md)"                    │
└──────────────────────────────────────────────────────────────┘
```

The AGENTS.md provides project context, while PROMPT.{mode}.md provides the specific task instructions.

---

## 3. PLANNING Mode Template

### 3.1 Purpose

Planning mode performs gap analysis: comparing specifications against the current codebase to identify what's missing, then producing a prioritized implementation plan.

### 3.2 Template Content

```markdown
# Planning Mode

You are analyzing specifications against the current codebase to create an implementation plan.

## Your Task

1. **Read all specifications** in `specs/` directory
2. **Explore the codebase** to understand what exists
3. **Identify gaps** between specs and implementation
4. **Create or update** `IMPLEMENTATION_PLAN.md` with prioritized tasks

## Constraints

- DO NOT implement anything
- DO NOT make commits
- DO NOT modify source code
- ONLY output the implementation plan

## Process

### Step 1: Understand Requirements
Use subagents to read and summarize each spec file in `specs/`.

### Step 2: Analyze Current State
Use subagents to explore `src/` (or equivalent) and document:
- What features are implemented
- What patterns are in use
- What's partially complete

### Step 3: Gap Analysis
For each requirement in specs, determine:
- [ ] Not started
- [ ] Partially implemented
- [ ] Fully implemented

### Step 4: Create Plan
Write `IMPLEMENTATION_PLAN.md` with:

```markdown
# Implementation Plan

Generated: {timestamp}
Based on: specs/*.md

## Priority 1: Critical Path
- [ ] Task description (refs: specs/foo.md)
  - Dependencies: none
  - Complexity: low/medium/high

## Priority 2: Core Features
- [ ] Task description (refs: specs/bar.md)
  - Dependencies: Priority 1 tasks
  - Complexity: medium

## Priority 3: Enhancements
...
```

## Output Format

Your final output should confirm:
1. Which specs were analyzed
2. How many gaps were identified
3. That IMPLEMENTATION_PLAN.md was created/updated

## Important

- Assume specs describe INTENT, not reality
- Always verify against actual code before concluding something is implemented
- Tasks should be small enough to complete in one building iteration
- Include spec references for traceability
```

### 3.3 Key Behaviors

| Behavior | Description |
|----------|-------------|
| No commits | Planning never modifies tracked files |
| Subagent delegation | Use Task tool for heavy reading |
| Gap-focused | Output is delta between spec and reality |
| Plan file creation | Always produces IMPLEMENTATION_PLAN.md |

---

## 4. BUILDING Mode Template

### 4.1 Purpose

Building mode implements tasks from the plan, one at a time, with validation and commits.

### 4.2 Template Content

```markdown
# Building Mode

You are implementing tasks from the existing implementation plan.

## Your Task

1. **Read** `IMPLEMENTATION_PLAN.md`
2. **Select** the highest priority incomplete task
3. **Investigate** relevant code (don't assume not implemented)
4. **Implement** the task completely
5. **Validate** with tests and builds
6. **Update** the plan (mark complete, note discoveries)
7. **Commit** changes

## Constraints

- ONE task per iteration
- Must pass all validation before committing
- Update AGENTS.md if you discover operational knowledge

## Process

### Step 1: Read Plan
Open `IMPLEMENTATION_PLAN.md` and identify the first unchecked task (`- [ ]`).

### Step 2: Investigate
Before implementing, use subagents to:
- Read the referenced spec for requirements
- Search for existing related code
- Understand current patterns

**CRITICAL**: Never assume something isn't implemented. Always check first.

### Step 3: Implement
Write the code to complete the task. Follow patterns in AGENTS.md.

### Step 4: Validate
Run the project's validation commands:
- Tests: `{test_command from AGENTS.md}`
- Build: `{build_command from AGENTS.md}`
- Lint: `{lint_command from AGENTS.md}`

If validation fails:
- Fix the issues
- Re-run validation
- Do not proceed until passing

### Step 5: Update Plan
In `IMPLEMENTATION_PLAN.md`:
- Change `- [ ]` to `- [x]` for completed task
- Add notes about any discoveries or issues
- Add new tasks if scope expanded

### Step 6: Commit
Create a commit with:
- Clear message describing the change
- Reference to the spec if applicable

## Output Format

Your final output should confirm:
1. Which task was implemented
2. Validation results (pass/fail)
3. Commit SHA (if successful)

## Important

- Quality over speed - one well-implemented task is better than multiple broken ones
- If stuck on a task, document blockers and move to next
- Update AGENTS.md with any commands, patterns, or knowledge discovered
```

### 4.3 Key Behaviors

| Behavior | Description |
|----------|-------------|
| Single task focus | One task per iteration |
| Validation required | Tests must pass before commit |
| Plan updates | Mark tasks complete as side effect |
| Knowledge capture | Update AGENTS.md with discoveries |

---

## 5. AGENTS.md Template

### 5.1 Purpose

Project-specific operational knowledge that applies to both modes.

### 5.2 Template Content

```markdown
# Project: {project_name}

## Commands

### Testing
```bash
npm test
# or: pytest, go test ./..., cargo test
```

### Building
```bash
npm run build
# or: make, go build, cargo build
```

### Linting
```bash
npm run lint
# or: ruff check, golangci-lint run, cargo clippy
```

## Code Patterns

### File Organization
- Source code: `src/`
- Tests: `tests/` or `__tests__/`
- Specifications: `specs/`

### Naming Conventions
- {describe project conventions}

### Architecture Notes
- {describe key architectural patterns}

## Operational Knowledge

### Known Issues
- {document any gotchas}

### Dependencies
- {list key dependencies and their purposes}

## Fresher Notes

*This section is updated by the Ralph Loop as it learns about the project.*
```

---

## 6. Configuration

### Mode Selection

The mode is selected via environment variable or CLI argument:

```bash
# Via environment
FRESHER_MODE=planning .fresher/run.sh

# Via CLI (if global fresher installed)
fresher plan
fresher build
```

### Template Customization

Projects can modify the templates in `.fresher/PROMPT.*.md` to:
- Add project-specific constraints
- Modify the process steps
- Adjust output format

---

## 7. Implementation Phases

| Phase | Description | Dependencies | Complexity |
|-------|-------------|--------------|------------|
| 1 | Create PLANNING template | None | Low |
| 2 | Create BUILDING template | None | Low |
| 3 | Create AGENTS.md template | None | Low |
| 4 | Interactive customization | project-scaffold | Medium |

---

## 8. Open Questions

- [ ] Should templates support variable substitution (e.g., `{project_name}`)?
- [ ] How to handle projects with non-standard structures?
- [ ] Should there be a "hybrid" mode for small changes?
