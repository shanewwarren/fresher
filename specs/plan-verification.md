# Plan Verification Specification

**Status:** Planned
**Version:** 1.0
**Last Updated:** 2025-01-17

---

## 1. Overview

### Purpose

Plan verification provides tooling to validate that `IMPLEMENTATION_PLAN.md` accurately reflects the gap between specifications and the current codebase. It helps catch stale plans, missed requirements, and scope drift.

### Goals

- **Accuracy** - Ensure plan tasks map to actual spec requirements
- **Completeness** - Detect requirements not represented in the plan
- **Freshness** - Identify when plan is stale compared to code changes
- **Actionable output** - Produce reports that guide plan updates

### Non-Goals

- **Auto-fix** - Don't automatically modify the plan
- **Test coverage** - Don't measure code test coverage (different concern)
- **Continuous monitoring** - Run on-demand, not as a daemon

---

## 2. Architecture

### Component Structure

```
.fresher/
├── lib/
│   └── verify.sh           # Verification logic
└── bin/
    └── fresher-verify      # CLI entry point

# Output
VERIFICATION_REPORT.md      # Generated report (project root)
```

### Verification Flow

```
┌─────────────────────────────────────────────────────────────────┐
│  fresher verify                                                  │
├─────────────────────────────────────────────────────────────────┤
│  1. Parse specs/*.md → Extract requirements                     │
│  2. Parse IMPLEMENTATION_PLAN.md → Extract tasks                │
│  3. Scan codebase → Find implementation evidence                │
│                                                                 │
│  4. Cross-reference:                                            │
│     - Spec requirements → Plan tasks                            │
│     - Plan tasks → Code evidence                                │
│     - Spec requirements → Code evidence                         │
│                                                                 │
│  5. Generate VERIFICATION_REPORT.md                             │
│  6. Return exit code (0=ok, 1=issues found)                     │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. Core Types

### 3.1 Requirement Extraction

Requirements are extracted from spec files using patterns:

| Pattern | Example | Type |
|---------|---------|------|
| `### Section` | `### User Authentication` | Section requirement |
| `- [ ] Task` | `- [ ] Implement login` | Uncompleted task |
| `- [x] Task` | `- [x] Add logout` | Completed task |
| `MUST/SHOULD` | `User MUST be authenticated` | RFC 2119 requirement |

### 3.2 Plan Task Structure

Tasks in `IMPLEMENTATION_PLAN.md` follow this format:

```markdown
- [ ] Task description (refs: specs/filename.md)
  - Dependencies: task-id, task-id
  - Complexity: low/medium/high
```

Parsed into:

| Field | Description |
|-------|-------------|
| `status` | `pending` or `completed` |
| `description` | Task text |
| `spec_ref` | Referenced spec file |
| `dependencies` | List of dependent task IDs |
| `complexity` | Estimated complexity |

### 3.3 Verification Report Structure

```markdown
# Verification Report

Generated: {timestamp}
Plan: IMPLEMENTATION_PLAN.md
Specs: specs/*.md

## Summary

| Metric | Count |
|--------|-------|
| Total spec requirements | 45 |
| Requirements with tasks | 38 |
| Requirements without tasks | 7 |
| Plan tasks | 42 |
| Completed tasks | 15 |
| Pending tasks | 27 |
| Orphan tasks (no spec ref) | 4 |

## Coverage by Spec

| Spec | Requirements | Covered | Coverage |
|------|--------------|---------|----------|
| auth.md | 12 | 10 | 83% |
| api.md | 18 | 16 | 89% |
| db.md | 15 | 12 | 80% |

## Missing Coverage

Requirements without plan tasks:

### From specs/auth.md
- [ ] Password reset flow
- [ ] Session timeout handling

### From specs/api.md
- [ ] Rate limiting
- [ ] API versioning

## Orphan Tasks

Plan tasks without spec references:

- [ ] Refactor database queries
- [ ] Add logging

## Implementation Evidence

Tasks with code evidence found:

| Task | Evidence | Location |
|------|----------|----------|
| User login | `authenticateUser` | src/auth/login.ts:45 |
| API routes | `router.get` | src/routes/index.ts:12 |

## Recommendations

1. Add spec references to orphan tasks or remove if out of scope
2. Create plan tasks for missing requirements
3. Review completed tasks with no code evidence
```

---

## 4. Behaviors

### 4.1 Requirement Extraction

```bash
extract_requirements() {
  local spec_dir="${1:-specs}"
  local output_file="/tmp/requirements.txt"

  > "$output_file"

  find "$spec_dir" -name "*.md" -type f | while read spec_file; do
    spec_name=$(basename "$spec_file" .md)

    # Extract section headers as requirements
    grep -E '^###\s+' "$spec_file" | while read line; do
      req_text=$(echo "$line" | sed 's/^###\s*//')
      echo "${spec_name}|section|${req_text}" >> "$output_file"
    done

    # Extract checkbox items
    grep -E '^\s*-\s*\[[ x]\]' "$spec_file" | while read line; do
      if [[ $line =~ \[x\] ]]; then
        status="completed"
      else
        status="pending"
      fi
      req_text=$(echo "$line" | sed 's/^.*\]\s*//')
      echo "${spec_name}|task|${status}|${req_text}" >> "$output_file"
    done

    # Extract MUST/SHOULD statements
    grep -iE '\b(MUST|SHOULD|SHALL|REQUIRED)\b' "$spec_file" | while read line; do
      echo "${spec_name}|rfc2119|${line}" >> "$output_file"
    done
  done

  cat "$output_file"
}
```

### 4.2 Plan Parsing

```bash
parse_plan() {
  local plan_file="${1:-IMPLEMENTATION_PLAN.md}"
  local output_file="/tmp/plan_tasks.txt"

  > "$output_file"

  if [[ ! -f "$plan_file" ]]; then
    echo "ERROR: Plan file not found: $plan_file" >&2
    return 1
  fi

  # Extract tasks with their metadata
  grep -E '^\s*-\s*\[[ x]\]' "$plan_file" | while read line; do
    if [[ $line =~ \[x\] ]]; then
      status="completed"
    else
      status="pending"
    fi

    # Extract task description
    description=$(echo "$line" | sed 's/^.*\]\s*//' | sed 's/(refs:.*)$//')

    # Extract spec reference if present
    if [[ $line =~ refs:\ *([a-zA-Z0-9_/-]+\.md) ]]; then
      spec_ref="${BASH_REMATCH[1]}"
    else
      spec_ref="none"
    fi

    echo "${status}|${spec_ref}|${description}" >> "$output_file"
  done

  cat "$output_file"
}
```

### 4.3 Code Evidence Search

```bash
find_evidence() {
  local task_description="$1"
  local src_dir="${2:-src}"

  # Extract keywords from task (nouns and verbs)
  keywords=$(echo "$task_description" | \
    grep -oE '\b[A-Za-z]{4,}\b' | \
    tr '[:upper:]' '[:lower:]' | \
    sort -u | \
    head -5)

  # Search for each keyword
  for keyword in $keywords; do
    # Search function/class definitions
    rg -l "(function|class|const|interface|type).*${keyword}" "$src_dir" 2>/dev/null && return 0
  done

  return 1
}
```

### 4.4 Cross-Reference Analysis

```bash
analyze_coverage() {
  local requirements_file="$1"
  local tasks_file="$2"

  echo "## Coverage Analysis"
  echo ""

  # Count totals
  total_reqs=$(wc -l < "$requirements_file")
  total_tasks=$(wc -l < "$tasks_file")

  # Find requirements without tasks
  echo "### Requirements Without Tasks"
  while IFS='|' read spec type rest; do
    # Check if any task references this spec
    if ! grep -q "$spec" "$tasks_file"; then
      echo "- [$spec] $rest"
    fi
  done < "$requirements_file"

  # Find tasks without spec references
  echo ""
  echo "### Tasks Without Spec References"
  grep '|none|' "$tasks_file" | while IFS='|' read status spec desc; do
    echo "- [ ] $desc"
  done
}
```

---

## 5. CLI Interface

### Commands

```bash
fresher verify [options]

Options:
  --spec-dir DIR       Specification directory (default: specs/)
  --plan-file FILE     Plan file path (default: IMPLEMENTATION_PLAN.md)
  --src-dir DIR        Source directory for evidence (default: src/)
  --output FILE        Report output file (default: VERIFICATION_REPORT.md)
  --format FORMAT      Output format: markdown|json (default: markdown)
  --quiet              Only output summary metrics
  --strict             Exit with error if any issues found
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Verification passed, no issues |
| 1 | Verification completed, issues found |
| 2 | Verification failed (missing files, etc.) |

---

## 6. Integration Points

### Pre-Planning Hook

Run verification before planning mode to provide context:

```bash
# .fresher/hooks/started
if [[ "$FRESHER_MODE" == "planning" ]]; then
  fresher verify --quiet
  echo "Verification complete. See VERIFICATION_REPORT.md for details."
fi
```

### Pre-Building Hook

Ensure plan is current before building:

```bash
# .fresher/hooks/started
if [[ "$FRESHER_MODE" == "building" ]]; then
  if ! fresher verify --strict --quiet; then
    echo "WARNING: Plan may be stale. Consider running planning mode first."
  fi
fi
```

### CI Integration

```yaml
# .github/workflows/verify.yml
- name: Verify Implementation Plan
  run: |
    fresher verify --strict --format json > verification.json
    if [ $? -ne 0 ]; then
      echo "::warning::Implementation plan has coverage gaps"
    fi
```

---

## 7. Implementation Phases

| Phase | Description | Dependencies | Complexity |
|-------|-------------|--------------|------------|
| 1 | Requirement extraction | None | Medium |
| 2 | Plan parsing | None | Low |
| 3 | Cross-reference analysis | Phase 1, 2 | Medium |
| 4 | Code evidence search | Phase 3 | Medium |
| 5 | Report generation | Phase 1-4 | Low |
| 6 | CLI interface | Phase 5 | Low |

---

## 8. Open Questions

- [ ] Should verification track historical coverage trends?
- [ ] How to handle specs that are intentionally not in current plan (future work)?
- [ ] Should there be a "fix" mode that auto-generates missing tasks?
