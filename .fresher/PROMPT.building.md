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
