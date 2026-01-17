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
