# Plan Verification Specification

**Status:** Implemented
**Version:** 2.0
**Last Updated:** 2026-01-18
**Implementation:** `src/commands/verify.rs`, `src/verify.rs`

---

## 1. Overview

### Purpose

Plan verification validates that `IMPLEMENTATION_PLAN.md` accurately reflects the gap between specifications and the current codebase. It produces reports that guide plan updates and track progress.

### Goals

- **Accuracy** - Ensure plan tasks map to actual spec requirements
- **Completeness** - Detect requirements not represented in the plan
- **Progress tracking** - Show completion status and coverage metrics
- **Actionable output** - Produce reports that guide plan updates

### Non-Goals

- **Auto-fix** - Don't automatically modify the plan
- **Test coverage** - Don't measure code test coverage (different concern)
- **Continuous monitoring** - Run on-demand, not as a daemon

---

## 2. Architecture

### Module Structure

```
src/
├── commands/
│   └── verify.rs        # CLI handler and report printing
└── verify.rs            # Core verification logic and types
```

### Verification Flow

```
┌─────────────────────────────────────────────────────────────────┐
│  fresher verify [--json] [--plan <file>]                        │
├─────────────────────────────────────────────────────────────────┤
│  1. Load configuration                                          │
│  2. Parse IMPLEMENTATION_PLAN.md → Extract tasks                │
│  3. Parse specs/*.md → Extract requirements                     │
│                                                                 │
│  4. Analyze:                                                    │
│     - Count tasks by status (pending/completed/in-progress)     │
│     - Identify orphan tasks (no spec refs)                      │
│     - Calculate coverage per spec                               │
│                                                                 │
│  5. Generate report (terminal or JSON)                          │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. Core Types (verify.rs)

### 3.1 Task

```rust
pub struct Task {
    pub description: String,
    pub status: TaskStatus,
    pub spec_refs: Vec<String>,
    pub line_number: usize,
    pub priority: Option<u32>,
    pub dependencies: Vec<String>,
    pub complexity: Option<String>,
}

pub enum TaskStatus {
    Pending,      // [ ]
    Completed,    // [x] or [X]
    InProgress,   // [~]
}
```

### 3.2 Requirement

```rust
pub struct Requirement {
    pub spec_name: String,
    pub req_type: RequirementType,
    pub text: String,
    pub line_number: usize,
}

pub enum RequirementType {
    Section,   // ### Section Header
    Task,      // - [ ] or - [x] in spec
    Rfc2119,   // MUST, SHOULD, SHALL, etc.
}
```

### 3.3 Verification Report

```rust
pub struct VerifyReport {
    pub total_tasks: usize,
    pub pending_tasks: usize,
    pub completed_tasks: usize,
    pub in_progress_tasks: usize,
    pub tasks_with_refs: usize,
    pub orphan_tasks: usize,
    pub coverage: Vec<CoverageEntry>,
    pub tasks: Vec<Task>,
}

pub struct CoverageEntry {
    pub spec_name: String,
    pub requirement_count: usize,
    pub task_count: usize,
    pub coverage_percent: f64,
}
```

---

## 4. Behaviors

### 4.1 Plan Parsing

Tasks are extracted from `IMPLEMENTATION_PLAN.md` using regex patterns:

| Pattern | Purpose | Example |
|---------|---------|---------|
| `^##\s+Priority\s+(\d+)` | Extract priority section | `## Priority 1: Core` |
| `^(\s*)-\s*\[([ xX~])\]` | Match checkbox and status | `- [ ] Task`, `- [x] Done` |
| `\(refs?:\s*([^)]+)\)` | Extract spec references | `(refs: specs/foo.md)` |
| `Dependencies:\s*(.+)` | Extract dependencies | `- Dependencies: Module A` |
| `Complexity:\s*(low\|medium\|high)` | Extract complexity | `- Complexity: medium` |

**Status mapping:**

| Checkbox | Status |
|----------|--------|
| `[ ]` | Pending |
| `[x]` or `[X]` | Completed |
| `[~]` | In Progress |

### 4.2 Requirement Extraction

Requirements are extracted from specification files in `specs/`:

| Pattern | Type | Example |
|---------|------|---------|
| `^###\s+(.+)$` | Section | `### User Authentication` |
| `^\s*-\s*\[([ xX])\]` | Task | `- [ ] Implement login` |
| `\b(MUST\|SHOULD\|...)\b` | RFC 2119 | `User MUST be authenticated` |

**RFC 2119 keywords detected:**
- MUST, MUST NOT
- REQUIRED, SHALL, SHALL NOT
- SHOULD, SHOULD NOT
- RECOMMENDED, MAY, OPTIONAL

### 4.3 Coverage Analysis

Coverage is calculated per spec as:

```rust
coverage_percent = (task_count / requirement_count * 100).min(100.0)
```

Where:
- `task_count` = tasks referencing the spec
- `requirement_count` = sections + tasks + RFC2119 statements in spec

### 4.4 Pending Task Detection

Used by `fresher build` to check if work remains:

```rust
pub fn has_pending_tasks(plan_path: &Path) -> bool {
    let content = fs::read_to_string(plan_path).unwrap_or_default();
    let checkbox_re = Regex::new(r"^\s*-\s*\[\s\]").unwrap();
    content.lines().any(|line| checkbox_re.is_match(line))
}
```

---

## 5. CLI Interface

### Command

```bash
fresher verify [OPTIONS]

Options:
  --json                Output JSON instead of terminal format
  --plan <FILE>         Plan file path [default: IMPLEMENTATION_PLAN.md]
  -h, --help            Print help
```

### Terminal Output

```
Implementation Plan Verification
========================================

Task Summary
  Total tasks:     42
  Completed:       15 (35%)
  In Progress:     2
  Pending:         25

Traceability
  Tasks with refs: 38
  Orphan tasks:    4

Spec Coverage
  auth                 [████████░░░░░░░░░░░░] 40% (5 reqs, 2 tasks)
  api                  [████████████████░░░░] 80% (10 reqs, 8 tasks)
  database             [████████████████████] 100% (3 reqs, 4 tasks)

Pending Tasks
  [P1] ○ Implement user login
  [P1] ○ Add session management
  [P2] ○ Create API endpoints
  ... and 22 more...

→ 25 tasks remaining
```

### JSON Output

```json
{
  "total_tasks": 42,
  "pending_tasks": 25,
  "completed_tasks": 15,
  "in_progress_tasks": 2,
  "tasks_with_refs": 38,
  "orphan_tasks": 4,
  "coverage": [
    {
      "spec_name": "auth",
      "requirement_count": 5,
      "task_count": 2,
      "coverage_percent": 40.0
    }
  ],
  "tasks": [
    {
      "description": "Implement user login",
      "status": "pending",
      "spec_refs": ["specs/auth.md"],
      "line_number": 15,
      "priority": 1,
      "dependencies": [],
      "complexity": "medium"
    }
  ]
}
```

---

## 6. Error Handling

| Condition | Behavior |
|-----------|----------|
| Plan file not found | Print error, suggest `fresher plan` |
| Specs directory missing | Return empty coverage |
| Invalid regex in file | Skip that pattern, continue |
| Malformed task | Skip task, continue parsing |

---

## 7. Integration

### Used by Build Command

The `fresher build` command uses `has_pending_tasks()` to determine if there's work to do before starting the loop.

### Hook Integration

```bash
# .fresher/hooks/started
fresher verify --json > .fresher/logs/verify-report.json
```

### CI Integration

```yaml
- name: Check Plan Coverage
  run: |
    fresher verify --json > coverage.json
    ORPHANS=$(jq '.orphan_tasks' coverage.json)
    if [ "$ORPHANS" -gt 0 ]; then
      echo "::warning::$ORPHANS tasks without spec references"
    fi
```

---

## 8. Implementation Notes

### Key Functions

| Function | Purpose |
|----------|---------|
| `parse_plan(path)` | Extract tasks from IMPLEMENTATION_PLAN.md |
| `extract_requirements(spec_dir)` | Extract requirements from specs/*.md |
| `analyze_coverage(spec_dir, tasks)` | Calculate coverage metrics |
| `generate_report(plan, specs)` | Build full VerifyReport |
| `has_pending_tasks(plan)` | Quick check for pending work |
| `count_tasks(tasks)` | Count by status |

### Test Coverage

The module includes comprehensive tests for:
- Empty/missing files
- All task statuses (pending, completed, in-progress)
- Spec references (single and multiple)
- Priority sections
- Dependencies and complexity parsing
- Requirement extraction (sections, checkboxes, RFC2119)
- Coverage calculations
- Line number tracking

---

## 9. Future Enhancements

- **Evidence search**: Search codebase for implementation evidence
- **Historical tracking**: Track coverage trends over time
- **Auto-task generation**: Suggest tasks for uncovered requirements
- **Spec freshness**: Detect when specs have changed since planning
