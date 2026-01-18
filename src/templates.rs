/// Embedded prompt templates for fresher

/// Planning mode prompt template
pub const PROMPT_PLANNING: &str = r#"# Planning Mode

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
"#;

/// Building mode prompt template
pub const PROMPT_BUILDING: &str = r#"# Building Mode

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
"#;

/// Default AGENTS.md template
pub const AGENTS_TEMPLATE: &str = r#"# Project: {project_name}

## Commands

### Testing
```bash
{test_command}
```

### Building
```bash
{build_command}
```

### Linting
```bash
{lint_command}
```

## Code Patterns

### File Organization
- Source code: `{src_dir}/`
- Tests: `tests/` or `__tests__/`
- Specifications: `{spec_dir}/`

### Naming Conventions
<!-- Describe your project's naming conventions here -->

### Architecture Notes
<!-- Describe key architectural patterns here -->

## Operational Knowledge

### Known Issues
<!-- Document any gotchas here -->

### Dependencies
<!-- List key dependencies and their purposes here -->

## Fresher Notes

*This section is updated by the Ralph Loop as it learns about the project.*
"#;

/// Default config.toml template
pub const CONFIG_TEMPLATE: &str = r#"# Fresher configuration
# Generated: {timestamp}
# Project type: {project_type}

[fresher]
mode = "planning"
max_iterations = 0
smart_termination = true
dangerous_permissions = true
max_turns = 50
model = "sonnet"

[commands]
test = "{test_command}"
build = "{build_command}"
lint = "{lint_command}"

[paths]
log_dir = ".fresher/logs"
spec_dir = "specs"
src_dir = "{src_dir}"

[hooks]
enabled = true
timeout = 30

[docker]
use_docker = false
memory = "4g"
cpus = "2"
"#;

/// Example hook script for started hook
pub const HOOK_STARTED: &str = r#"#!/bin/bash
# Hook: started
# Runs once when the loop starts
# Exit 0 to continue, exit 2 to abort loop

echo "Fresher loop starting..."

# Example: Check for required tools
# if ! command -v claude &> /dev/null; then
#     echo "Error: claude command not found"
#     exit 2
# fi

exit 0
"#;

/// Example hook script for next_iteration hook
pub const HOOK_NEXT_ITERATION: &str = r#"#!/bin/bash
# Hook: next_iteration
# Runs before each iteration
# Exit 0 to continue, exit 1 to skip iteration, exit 2 to abort loop

echo "Starting iteration $FRESHER_ITERATION..."

# Example: Skip iteration if disk space is low
# available=$(df -k . | tail -1 | awk '{print $4}')
# if [ "$available" -lt 1048576 ]; then
#     echo "Warning: Low disk space, skipping iteration"
#     exit 1
# fi

exit 0
"#;

/// Example hook script for finished hook
pub const HOOK_FINISHED: &str = r#"#!/bin/bash
# Hook: finished
# Runs when the loop ends (any exit condition)
# Environment variables available:
#   FRESHER_TOTAL_ITERATIONS - Total iterations completed
#   FRESHER_TOTAL_COMMITS - Total commits made
#   FRESHER_DURATION - Total duration in seconds
#   FRESHER_FINISH_TYPE - How loop ended (manual, error, max_iterations, complete, no_changes)

echo "Fresher loop finished"
echo "  Iterations: $FRESHER_TOTAL_ITERATIONS"
echo "  Commits: $FRESHER_TOTAL_COMMITS"
echo "  Duration: ${FRESHER_DURATION}s"
echo "  Finish type: $FRESHER_FINISH_TYPE"

# Example: Send notification
# curl -X POST "https://slack.webhook/..." -d "{\"text\": \"Fresher completed: $FRESHER_FINISH_TYPE\"}"

exit 0
"#;
