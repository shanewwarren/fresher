# The Ralph Loop

An iterative execution model for AI-driven software development with two modes: **PLANNING** and **BUILDING**.

---

## Overview

The Ralph Loop is the core execution mechanism for Phases 2 and 3 of specification-driven development. It uses the same loop structure for both planning and building, swapping `PROMPT.md` to change behavior.

```
┌─────────────────────────────────────────────────────────────┐
│  Phase 1: Define Requirements (/specify)                    │
│  → specs/*.md, CLAUDE.md updates                            │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  Phase 2/3: Ralph Loop                                      │
│  ┌─────────────────┐    ┌─────────────────┐                │
│  │  PLANNING Mode  │ or │  BUILDING Mode  │                │
│  │  (gap analysis) │    │  (implementation)│                │
│  └─────────────────┘    └─────────────────┘                │
└─────────────────────────────────────────────────────────────┘
```

---

## Two Modes

| Mode | When to Use | Output |
|------|-------------|--------|
| **PLANNING** | No plan exists, or plan is stale/wrong | `IMPLEMENTATION_PLAN.md` (prioritized TODO list) |
| **BUILDING** | Plan exists | Code commits, updated plan, updated `AGENTS.md` |

### Why Use the Same Loop for Both?

- **BUILDING requires it**: Inherently iterative (many tasks × fresh context = isolation)
- **PLANNING uses it for consistency**: Same execution model, though often completes in 1-2 iterations
- **Flexibility**: If plan needs refinement, loop allows multiple passes reading its own output
- **Simplicity**: One mechanism for everything; clean file I/O; easy stop/restart

---

## Context Loading

Each iteration loads the same files to start from a known state:

```
PROMPT.md + AGENTS.md
```

- `PROMPT.md` - Mode-specific instructions (PLANNING or BUILDING)
- `AGENTS.md` - Project-specific commands, patterns, and operational knowledge

---

## PLANNING Mode

**Purpose:** Gap analysis between specs and code. Outputs a prioritized task list.

**Key constraint:** No implementation, no commits.

### Lifecycle

```
1. Subagents study specs/* (requirements)
2. Subagents study existing /src (reality)
3. Compare specs against code (gap analysis)
4. Create/update IMPLEMENTATION_PLAN.md with prioritized tasks
5. Loop ends (often in 1-2 iterations)
```

### PLANNING Prompt Focus

- Read all specs in `specs/`
- Explore codebase to understand what exists
- Identify gaps: what's specified but not implemented
- Prioritize tasks by dependencies and importance
- Output structured plan with no implementation

---

## BUILDING Mode

**Purpose:** Implement tasks from the plan, one at a time, with fresh context each iteration.

### Lifecycle

```
1. Orient      → Subagents study specs/* (requirements)
2. Read plan   → Study IMPLEMENTATION_PLAN.md
3. Select      → Pick the most important task
4. Investigate → Subagents study relevant /src ("don't assume not implemented")
5. Implement   → N subagents for file operations
6. Validate    → 1 subagent for build/tests (backpressure)
7. Update plan → Mark task done, note discoveries/bugs
8. Update AGENTS.md → If operational learnings discovered
9. Commit
10. Loop ends → Context cleared → Next iteration starts fresh
```

### BUILDING Prompt Focus

- Assumes plan exists
- Pick ONE task from plan
- Implement completely
- Run tests (backpressure)
- Commit changes
- Update plan as side effect

---

## Key Concepts

### Job to Be Done (JTBD)

High-level user need or outcome.

**Example:** "Help designers create mood boards"

### Topic of Concern

A distinct aspect or component within a JTBD.

**Example topics for mood board JTBD:**
- Image collection
- Color extraction
- Layout system
- Sharing

### Spec

Requirements document for one topic of concern. Lives in `specs/FILENAME.md`.

### Task

Unit of work derived from comparing specs to code. Tasks live in `IMPLEMENTATION_PLAN.md`.

### Relationships

```
1 JTBD → multiple topics of concern
1 topic of concern → 1 spec
1 spec → multiple tasks
```

Specs are larger than tasks. A single spec generates many implementation tasks.

---

## The "One Sentence Without And" Test

**Purpose:** Validate that a topic of concern is properly scoped.

**Rule:** Can you describe the topic in one sentence without conjoining unrelated capabilities?

**Pass:**
> "The color extraction system analyzes images to identify dominant colors"

**Fail:**
> "The user system handles authentication, profiles, and billing"

If you need "and" to describe what it does, it's probably multiple topics. Split it.

---

## Context Management

### The Problem

- 200K+ tokens advertised ≈ 176K truly usable
- 40-60% context utilization for "smart zone" (where model performs best)
- Tight tasks + 1 task per loop = 100% smart zone context utilization

### The Solution

**Use main agent as a scheduler, not a worker:**

```
Main Context (scheduler)
    │
    ├── Spawn subagent for research (~156KB, then garbage collected)
    ├── Spawn subagent for implementation
    ├── Spawn subagent for validation
    └── Coordinate results, manage state
```

**Benefits:**
- Each subagent gets fresh ~156KB context
- Fan out to avoid polluting main context
- Subagents act as memory extension
- Main context stays clean for coordination

### Key Principles

- Don't allocate expensive work to main context
- Spawn subagents whenever possible
- Use subagents as memory extension
- Simplicity and brevity win
- Verbose inputs degrade determinism
- Prefer Markdown over JSON for work tracking (better token efficiency)

---

## Steering Ralph: Patterns + Backpressure

Creating the right signals and gates to steer Ralph's successful output is critical.

### Steer Upstream (Inputs)

**Ensure deterministic setup:**

1. Allocate first ~5,000 tokens for specs
2. Every loop's context starts with same files (`PROMPT.md` + `AGENTS.md`)
3. Your existing code shapes what gets used and generated

**Pattern steering:**

If Ralph generates wrong patterns, add/update utilities and existing code patterns to steer it toward correct ones. The codebase itself is a steering mechanism.

### Steer Downstream (Backpressure)

**Create gates that reject invalid work:**

- Tests
- Type checks
- Lints
- Builds
- Any validation that produces pass/fail

**How it works:**

- `PROMPT.md` says "run tests" generically
- `AGENTS.md` specifies actual commands (project-specific backpressure)
- Failed validation = task not complete = loop continues

**Beyond code validation:**

Some acceptance criteria resist programmatic checks:
- Creative quality
- Aesthetics
- UX feel

**Solution:** LLM-as-judge tests can provide backpressure for subjective criteria with binary pass/fail.

### Remind Ralph to Use Backpressure

Include in prompts:
> "Important: When authoring documentation, capture the why — tests and implementation importance."

---

## File Structure

```
project/
├── PROMPT.md              # Mode-specific instructions (swap for PLANNING vs BUILDING)
├── AGENTS.md              # Project-specific commands and patterns
├── IMPLEMENTATION_PLAN.md # Generated by PLANNING mode, consumed by BUILDING mode
├── specs/
│   ├── README.md          # Index of all specs
│   └── {topic}.md         # One spec per topic of concern
└── src/                   # Implementation
```

---

## PROMPT.md Examples

### PLANNING Mode PROMPT.md

```markdown
# Planning Mode

You are analyzing specifications against the current codebase to create an implementation plan.

## Your Task

1. Read all specs in `specs/`
2. Explore the codebase to understand what exists
3. Identify gaps between specs and implementation
4. Create `IMPLEMENTATION_PLAN.md` with prioritized tasks

## Constraints

- DO NOT implement anything
- DO NOT make commits
- DO NOT modify source code
- ONLY output the implementation plan

## Output Format

Create `IMPLEMENTATION_PLAN.md` with:
- Prioritized task list
- Dependencies between tasks
- Spec references for each task
```

### BUILDING Mode PROMPT.md

```markdown
# Building Mode

You are implementing tasks from the existing implementation plan.

## Your Task

1. Read `IMPLEMENTATION_PLAN.md`
2. Select the highest priority incomplete task
3. Investigate relevant code (don't assume not implemented)
4. Implement the task completely
5. Run tests and validation
6. Update the plan (mark complete, note discoveries)
7. Commit changes

## Constraints

- ONE task per iteration
- Must pass all tests before committing
- Update AGENTS.md if you discover operational knowledge
```

---

## Loop Execution

### Single Iteration

```
Load Context: PROMPT.md + AGENTS.md
       │
       ▼
Execute Mode-Specific Logic
       │
       ▼
Write Outputs (plan updates, code, commits)
       │
       ▼
Clear Context
       │
       ▼
Next Iteration (fresh start)
```

### Termination Conditions

**PLANNING mode:**
- Plan is complete and comprehensive
- Usually 1-2 iterations

**BUILDING mode:**
- All tasks in plan marked complete
- Or: external stop signal
- Or: max iterations reached

---

## Summary

The Ralph Loop provides:

1. **Isolation** - Fresh context each iteration prevents accumulation of noise
2. **Consistency** - Same mechanism for planning and building
3. **Steering** - Upstream (specs, patterns) and downstream (tests, backpressure)
4. **Scalability** - Subagents extend memory without polluting main context
5. **Simplicity** - One loop, two modes, clean file I/O
