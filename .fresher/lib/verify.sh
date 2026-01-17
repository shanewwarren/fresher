#!/bin/bash
# Fresher Plan Verification Library
# Functions for verifying plan coverage against specs

set -e

#──────────────────────────────────────────────────────────────────
# Configuration
#──────────────────────────────────────────────────────────────────

VERIFY_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VERIFY_PROJECT_DIR="$(cd "$VERIFY_SCRIPT_DIR/../.." && pwd)"

#──────────────────────────────────────────────────────────────────
# Helper Functions
#──────────────────────────────────────────────────────────────────

verify_log() {
  echo "[verify] $*"
}

verify_error() {
  echo "[verify] ERROR: $*" >&2
}

#──────────────────────────────────────────────────────────────────
# Requirement Extraction
#──────────────────────────────────────────────────────────────────

# Extract requirements from spec files
# Output format: spec_name|type|[status|]text
# Types: section, task, rfc2119
#
# Usage: extract_requirements [spec_dir]
extract_requirements() {
  local spec_dir="${1:-$VERIFY_PROJECT_DIR/specs}"

  if [[ ! -d "$spec_dir" ]]; then
    verify_error "Spec directory not found: $spec_dir"
    return 1
  fi

  # Find all markdown files in spec directory
  find "$spec_dir" -name "*.md" -type f | sort | while read -r spec_file; do
    local spec_name
    spec_name=$(basename "$spec_file" .md)

    # Extract section headers (### Level 3 headers)
    grep -nE '^### ' "$spec_file" 2>/dev/null | while IFS=: read -r line_num line; do
      local req_text
      req_text=$(echo "$line" | sed 's/^### *//')
      echo "${spec_name}|section|${line_num}|${req_text}"
    done

    # Extract checkbox items (- [ ] or - [x])
    grep -nE '^\s*-\s*\[[ xX]\]' "$spec_file" 2>/dev/null | while IFS=: read -r line_num line; do
      local status req_text
      if [[ $line =~ \[[xX]\] ]]; then
        status="completed"
      else
        status="pending"
      fi
      # Remove leading whitespace, dash, and checkbox
      req_text=$(echo "$line" | sed 's/^[[:space:]]*-[[:space:]]*\[[xX ]\][[:space:]]*//')
      echo "${spec_name}|task|${status}|${line_num}|${req_text}"
    done

    # Extract RFC 2119 keywords (MUST, SHOULD, SHALL, REQUIRED)
    # Only match lines with these keywords in uppercase (per RFC 2119 convention)
    # Use awk to skip code blocks (between ``` markers)
    awk '
      /^```/ { in_code = !in_code; next }
      !in_code && /\b(MUST|SHOULD|SHALL|REQUIRED|MUST NOT|SHALL NOT|SHOULD NOT|RECOMMENDED|MAY|OPTIONAL)\b/ {
        print NR ":" $0
      }
    ' "$spec_file" 2>/dev/null | while IFS=: read -r line_num line; do
      # Skip lines that are just explaining RFC 2119 keywords or table headers
      if [[ $line =~ "RFC 2119" ]] || [[ $line =~ "Pattern" ]] || [[ $line =~ "| \`" ]] || [[ $line =~ ^[[:space:]]*\| ]]; then
        continue
      fi
      # Clean up the line (remove leading/trailing whitespace, markdown formatting)
      local req_text
      req_text=$(echo "$line" | sed 's/^[[:space:]]*//' | sed 's/[[:space:]]*$//')
      echo "${spec_name}|rfc2119|${line_num}|${req_text}"
    done
  done
}

#──────────────────────────────────────────────────────────────────
# Plan Parsing
#──────────────────────────────────────────────────────────────────

# Parse tasks from implementation plan file
# Output format: status|spec_ref|line_num|description
#
# Usage: parse_plan [plan_file]
parse_plan() {
  local plan_file="${1:-$VERIFY_PROJECT_DIR/IMPLEMENTATION_PLAN.md}"

  if [[ ! -f "$plan_file" ]]; then
    verify_error "Plan file not found: $plan_file"
    return 1
  fi

  # Extract tasks with their metadata (checkboxes at any indentation)
  grep -nE '^\s*-\s*\[[ xX]\]' "$plan_file" 2>/dev/null | while IFS=: read -r line_num line; do
    local status description spec_ref

    # Determine status
    if [[ $line =~ \[[xX]\] ]]; then
      status="completed"
    else
      status="pending"
    fi

    # Extract task description (remove checkbox, trim refs)
    description=$(echo "$line" | sed 's/^[[:space:]]*-[[:space:]]*\[[xX ]\][[:space:]]*//' | sed 's/[[:space:]]*(refs:[^)]*)$//')

    # Extract spec reference if present
    if [[ $line =~ refs:[[:space:]]*([a-zA-Z0-9_/.-]+\.md) ]]; then
      spec_ref="${BASH_REMATCH[1]}"
      # Normalize path (remove specs/ prefix if present for matching)
      spec_ref=$(echo "$spec_ref" | sed 's|^specs/||')
    else
      spec_ref="none"
    fi

    echo "${status}|${spec_ref}|${line_num}|${description}"
  done
}

# Count plan tasks by status
# Usage: count_plan_tasks [plan_file]
count_plan_tasks() {
  local plan_file="${1:-$VERIFY_PROJECT_DIR/IMPLEMENTATION_PLAN.md}"
  local tasks
  tasks=$(parse_plan "$plan_file") || return 1

  # Use grep -c with || true to avoid exit on no match
  local total pending completed orphans with_refs
  total=$(echo "$tasks" | grep -c . 2>/dev/null) || total=0
  pending=$(echo "$tasks" | grep -c '^pending|' 2>/dev/null) || pending=0
  completed=$(echo "$tasks" | grep -c '^completed|' 2>/dev/null) || completed=0
  orphans=$(echo "$tasks" | grep -c '|none|' 2>/dev/null) || orphans=0
  with_refs=$((total - orphans))

  echo "total:$total"
  echo "pending:$pending"
  echo "completed:$completed"
  echo "orphans:$orphans"
  echo "with_refs:$with_refs"
}

# Get tasks referencing a specific spec
# Usage: get_tasks_for_spec spec_name [plan_file]
get_tasks_for_spec() {
  local spec_name="$1"
  local plan_file="${2:-$VERIFY_PROJECT_DIR/IMPLEMENTATION_PLAN.md}"

  # Match spec name (with or without .md extension)
  parse_plan "$plan_file" | grep "|${spec_name%.md}.md|" || true
}

# Get orphan tasks (no spec reference)
# Usage: get_orphan_tasks [plan_file]
get_orphan_tasks() {
  local plan_file="${1:-$VERIFY_PROJECT_DIR/IMPLEMENTATION_PLAN.md}"

  parse_plan "$plan_file" | grep '|none|' || true
}

#──────────────────────────────────────────────────────────────────
# Cross-Reference Analysis
#──────────────────────────────────────────────────────────────────

# Analyze coverage: which specs have plan tasks and which don't
# Output format: spec_name|req_count|task_count|coverage_pct
#
# Usage: analyze_coverage [spec_dir] [plan_file]
analyze_coverage() {
  local spec_dir="${1:-$VERIFY_PROJECT_DIR/specs}"
  local plan_file="${2:-$VERIFY_PROJECT_DIR/IMPLEMENTATION_PLAN.md}"

  # Get list of specs
  local specs
  specs=$(list_specs "$spec_dir")

  # For each spec, count requirements and tasks
  echo "$specs" | while read -r spec_name; do
    # Count requirements in this spec (sections only for coverage)
    local req_count
    req_count=$(get_spec_requirements "$spec_name" "$spec_dir" | grep -c '|section|' 2>/dev/null) || req_count=0

    # Count tasks referencing this spec
    local task_count
    task_count=$(get_tasks_for_spec "$spec_name" "$plan_file" | grep -c . 2>/dev/null) || task_count=0

    # Calculate coverage (avoid division by zero)
    local coverage
    if [[ $req_count -gt 0 ]]; then
      coverage=$((task_count * 100 / req_count))
      # Cap at 100%
      [[ $coverage -gt 100 ]] && coverage=100
    else
      coverage=0
    fi

    echo "${spec_name}|${req_count}|${task_count}|${coverage}"
  done
}

# Find specs with no corresponding plan tasks
# Output: spec names with zero tasks
#
# Usage: find_uncovered_specs [spec_dir] [plan_file]
find_uncovered_specs() {
  local spec_dir="${1:-$VERIFY_PROJECT_DIR/specs}"
  local plan_file="${2:-$VERIFY_PROJECT_DIR/IMPLEMENTATION_PLAN.md}"

  analyze_coverage "$spec_dir" "$plan_file" | while IFS='|' read -r spec_name req_count task_count coverage; do
    if [[ "$task_count" -eq 0 && "$req_count" -gt 0 ]]; then
      echo "$spec_name"
    fi
  done
}

# Find sections from specs that don't have corresponding plan tasks
# This is more granular than spec-level coverage
# Output: spec_name|section_name|line_num
#
# Usage: find_uncovered_sections [spec_dir] [plan_file]
find_uncovered_sections() {
  local spec_dir="${1:-$VERIFY_PROJECT_DIR/specs}"
  local plan_file="${2:-$VERIFY_PROJECT_DIR/IMPLEMENTATION_PLAN.md}"

  # Get all plan tasks for keyword matching
  local plan_tasks
  plan_tasks=$(parse_plan "$plan_file") || return 1

  # Check each section requirement
  extract_requirements "$spec_dir" | grep '|section|' | while IFS='|' read -r spec_name type line_num section_name; do
    # Check if spec has any tasks
    local has_task
    has_task=$(echo "$plan_tasks" | grep -c "|${spec_name}.md|" 2>/dev/null) || has_task=0

    if [[ "$has_task" -eq 0 ]]; then
      echo "${spec_name}|${section_name}|${line_num}"
    fi
  done
}

# Get summary statistics for verification
# Usage: get_verification_summary [spec_dir] [plan_file]
get_verification_summary() {
  local spec_dir="${1:-$VERIFY_PROJECT_DIR/specs}"
  local plan_file="${2:-$VERIFY_PROJECT_DIR/IMPLEMENTATION_PLAN.md}"

  # Requirement counts
  local req_stats
  req_stats=$(count_requirements "$spec_dir")
  local total_reqs
  total_reqs=$(echo "$req_stats" | grep '^total:' | cut -d: -f2)
  local section_reqs
  section_reqs=$(echo "$req_stats" | grep '^sections:' | cut -d: -f2)

  # Task counts
  local task_stats
  task_stats=$(count_plan_tasks "$plan_file")
  local total_tasks
  total_tasks=$(echo "$task_stats" | grep '^total:' | cut -d: -f2)
  local pending_tasks
  pending_tasks=$(echo "$task_stats" | grep '^pending:' | cut -d: -f2)
  local completed_tasks
  completed_tasks=$(echo "$task_stats" | grep '^completed:' | cut -d: -f2)
  local orphan_tasks
  orphan_tasks=$(echo "$task_stats" | grep '^orphans:' | cut -d: -f2)

  # Spec coverage
  local total_specs covered_specs
  total_specs=$(list_specs "$spec_dir" | grep -c . 2>/dev/null) || total_specs=0
  covered_specs=$(analyze_coverage "$spec_dir" "$plan_file" | awk -F'|' '$3 > 0 {count++} END {print count+0}')

  echo "total_requirements:$total_reqs"
  echo "section_requirements:$section_reqs"
  echo "total_tasks:$total_tasks"
  echo "pending_tasks:$pending_tasks"
  echo "completed_tasks:$completed_tasks"
  echo "orphan_tasks:$orphan_tasks"
  echo "total_specs:$total_specs"
  echo "covered_specs:$covered_specs"
}

#──────────────────────────────────────────────────────────────────
# Code Evidence Search
#──────────────────────────────────────────────────────────────────

# Extract keywords from a task description for searching
# Filters out common words and returns significant terms
#
# Usage: extract_keywords "task description"
extract_keywords() {
  local description="$1"

  # Common words to filter out
  local stopwords="the|and|for|with|from|this|that|have|will|should|must|create|implement|add|update|file|function|method|class|spec|refs|none"

  echo "$description" | \
    grep -oE '\b[A-Za-z]{4,}\b' | \
    tr '[:upper:]' '[:lower:]' | \
    grep -vE "^($stopwords)$" | \
    sort -u | \
    head -5
}

# Search for code evidence of a task
# Returns: file_path:line_num|matched_text (first match only)
#
# Usage: find_evidence "task description" [src_dirs...]
find_evidence() {
  local task_description="$1"
  shift
  local src_dirs="${*:-.fresher .fresher-internal src lib}"

  # Extract keywords from task
  local keywords
  keywords=$(extract_keywords "$task_description")

  if [[ -z "$keywords" ]]; then
    return 1
  fi

  # Check if ripgrep is available
  local use_rg=false
  if command -v rg &> /dev/null; then
    use_rg=true
  fi

  # Search for each keyword
  local result=""
  for keyword in $keywords; do
    [[ -n "$result" ]] && break

    for src_dir in $src_dirs; do
      [[ ! -d "$VERIFY_PROJECT_DIR/$src_dir" ]] && continue
      [[ -n "$result" ]] && break

      # Search for the keyword
      local search_result
      if $use_rg; then
        search_result=$(rg -n "$keyword" "$VERIFY_PROJECT_DIR/$src_dir" 2>/dev/null | head -1)
      else
        search_result=$(grep -rn "$keyword" "$VERIFY_PROJECT_DIR/$src_dir" 2>/dev/null | head -1)
      fi

      if [[ -n "$search_result" ]]; then
        local file line_num matched
        file=$(echo "$search_result" | cut -d: -f1)
        line_num=$(echo "$search_result" | cut -d: -f2)
        matched=$(echo "$search_result" | cut -d: -f3- | sed 's/^[[:space:]]*//' | cut -c1-60)
        result="${file#$VERIFY_PROJECT_DIR/}:${line_num}|${matched}"
      fi
    done
  done

  if [[ -n "$result" ]]; then
    echo "$result"
    return 0
  fi

  return 1
}

# Find evidence for all completed tasks
# Output: task_description|file:line|evidence
#
# Usage: find_all_evidence [plan_file] [src_dirs...]
find_all_evidence() {
  local plan_file="${1:-$VERIFY_PROJECT_DIR/IMPLEMENTATION_PLAN.md}"
  shift
  local src_dirs="${*:-.fresher .fresher-internal src lib}"

  # Get completed tasks and process them
  local tasks
  tasks=$(parse_plan "$plan_file" | grep '^completed|') || return 0

  echo "$tasks" | while IFS='|' read -r status spec_ref line_num description; do
    local evidence
    evidence=$(find_evidence "$description" $src_dirs)
    if [[ -n "$evidence" ]]; then
      echo "${description}|${evidence}"
    fi
  done
}

#──────────────────────────────────────────────────────────────────
# Report Generation
#──────────────────────────────────────────────────────────────────

# Generate the full verification report in markdown format
# Usage: generate_report [spec_dir] [plan_file] [src_dirs]
generate_report() {
  local spec_dir="${1:-$VERIFY_PROJECT_DIR/specs}"
  local plan_file="${2:-$VERIFY_PROJECT_DIR/IMPLEMENTATION_PLAN.md}"
  local src_dirs="${3:-.fresher .fresher-internal src lib}"

  local timestamp
  timestamp=$(date +%Y-%m-%dT%H:%M:%SZ)

  # Header
  cat << EOF
# Verification Report

Generated: ${timestamp}
Plan: ${plan_file#$VERIFY_PROJECT_DIR/}
Specs: ${spec_dir#$VERIFY_PROJECT_DIR/}/*.md

EOF

  # Summary section
  echo "## Summary"
  echo ""
  local summary
  summary=$(get_verification_summary "$spec_dir" "$plan_file")
  local total_reqs section_reqs total_tasks pending_tasks completed_tasks orphan_tasks total_specs covered_specs
  total_reqs=$(echo "$summary" | grep '^total_requirements:' | cut -d: -f2)
  section_reqs=$(echo "$summary" | grep '^section_requirements:' | cut -d: -f2)
  total_tasks=$(echo "$summary" | grep '^total_tasks:' | cut -d: -f2)
  pending_tasks=$(echo "$summary" | grep '^pending_tasks:' | cut -d: -f2)
  completed_tasks=$(echo "$summary" | grep '^completed_tasks:' | cut -d: -f2)
  orphan_tasks=$(echo "$summary" | grep '^orphan_tasks:' | cut -d: -f2)
  total_specs=$(echo "$summary" | grep '^total_specs:' | cut -d: -f2)
  covered_specs=$(echo "$summary" | grep '^covered_specs:' | cut -d: -f2)

  cat << EOF
| Metric | Count |
|--------|-------|
| Total spec requirements | ${total_reqs} |
| Section requirements | ${section_reqs} |
| Plan tasks | ${total_tasks} |
| Completed tasks | ${completed_tasks} |
| Pending tasks | ${pending_tasks} |
| Orphan tasks (no spec ref) | ${orphan_tasks} |
| Total specs | ${total_specs} |
| Specs with tasks | ${covered_specs} |

EOF

  # Coverage by spec
  echo "## Coverage by Spec"
  echo ""
  echo "| Spec | Sections | Tasks | Coverage |"
  echo "|------|----------|-------|----------|"
  analyze_coverage "$spec_dir" "$plan_file" | while IFS='|' read -r spec_name req_count task_count coverage; do
    # Skip README
    [[ "$spec_name" == "README" ]] && continue
    echo "| ${spec_name}.md | ${req_count} | ${task_count} | ${coverage}% |"
  done
  echo ""

  # Missing coverage (specs without tasks)
  echo "## Missing Coverage"
  echo ""
  echo "Specs without implementation tasks:"
  echo ""
  local uncovered
  uncovered=$(find_uncovered_specs "$spec_dir" "$plan_file")
  if [[ -n "$uncovered" ]]; then
    echo "$uncovered" | while read -r spec_name; do
      echo "### From specs/${spec_name}.md"
      echo ""
      get_spec_requirements "$spec_name" "$spec_dir" | grep '|section|' | while IFS='|' read -r sn type line_num section; do
        echo "- [ ] ${section} (line ${line_num})"
      done
      echo ""
    done
  else
    echo "_All specs have corresponding implementation tasks._"
    echo ""
  fi

  # Orphan tasks
  echo "## Orphan Tasks"
  echo ""
  echo "Plan tasks without spec references:"
  echo ""
  local orphans
  orphans=$(get_orphan_tasks "$plan_file")
  if [[ -n "$orphans" ]]; then
    echo "$orphans" | while IFS='|' read -r status spec_ref line_num description; do
      local checkbox="[ ]"
      [[ "$status" == "completed" ]] && checkbox="[x]"
      echo "- ${checkbox} ${description} (line ${line_num})"
    done
  else
    echo "_All tasks have spec references._"
  fi
  echo ""

  # Implementation evidence
  echo "## Implementation Evidence"
  echo ""
  echo "Completed tasks with code evidence found:"
  echo ""
  echo "| Task | Evidence | Location |"
  echo "|------|----------|----------|"
  local evidence_output
  evidence_output=$(find_all_evidence "$plan_file" $src_dirs)
  if [[ -n "$evidence_output" ]]; then
    echo "$evidence_output" | head -20 | while IFS='|' read -r task location snippet; do
      # Truncate task description
      local short_task
      short_task=$(echo "$task" | cut -c1-40)
      [[ ${#task} -gt 40 ]] && short_task="${short_task}..."
      # Escape pipes in snippet
      snippet=$(echo "$snippet" | sed 's/|/\\|/g')
      echo "| ${short_task} | \`${snippet}\` | ${location} |"
    done
  else
    echo "| _No evidence found_ | | |"
  fi
  echo ""

  # Recommendations
  echo "## Recommendations"
  echo ""
  local rec_num=1

  if [[ -n "$orphans" ]]; then
    echo "${rec_num}. Add spec references to orphan tasks or remove if out of scope"
    ((rec_num++))
  fi

  if [[ -n "$uncovered" ]]; then
    echo "${rec_num}. Create plan tasks for specs without coverage: $(echo "$uncovered" | tr '\n' ', ' | sed 's/,$//')"
    ((rec_num++))
  fi

  if [[ "$pending_tasks" -gt 0 ]]; then
    echo "${rec_num}. Complete the ${pending_tasks} pending implementation tasks"
    ((rec_num++))
  fi

  if [[ $rec_num -eq 1 ]]; then
    echo "_No issues found - plan is well-aligned with specifications._"
  fi
  echo ""
}

# Generate report and write to file
# Usage: generate_report_file [output_file] [spec_dir] [plan_file] [src_dirs]
generate_report_file() {
  local output_file="${1:-$VERIFY_PROJECT_DIR/VERIFICATION_REPORT.md}"
  shift
  generate_report "$@" > "$output_file"
  echo "$output_file"
}

#──────────────────────────────────────────────────────────────────
# Requirement Counting
#──────────────────────────────────────────────────────────────────

# Count requirements by type
# Usage: count_requirements [spec_dir]
count_requirements() {
  local spec_dir="${1:-$VERIFY_PROJECT_DIR/specs}"
  local requirements
  requirements=$(extract_requirements "$spec_dir")

  local total sections tasks rfc2119 pending completed
  total=$(echo "$requirements" | grep -c . || echo 0)
  sections=$(echo "$requirements" | grep -c '|section|' || echo 0)
  tasks=$(echo "$requirements" | grep -c '|task|' || echo 0)
  rfc2119=$(echo "$requirements" | grep -c '|rfc2119|' || echo 0)
  pending=$(echo "$requirements" | grep -c '|task|pending|' || echo 0)
  completed=$(echo "$requirements" | grep -c '|task|completed|' || echo 0)

  echo "total:$total"
  echo "sections:$sections"
  echo "tasks:$tasks"
  echo "rfc2119:$rfc2119"
  echo "pending:$pending"
  echo "completed:$completed"
}

# Get requirements for a specific spec file
# Usage: get_spec_requirements spec_name [spec_dir]
get_spec_requirements() {
  local spec_name="$1"
  local spec_dir="${2:-$VERIFY_PROJECT_DIR/specs}"

  extract_requirements "$spec_dir" | grep "^${spec_name}|"
}

# List all spec files
# Usage: list_specs [spec_dir]
list_specs() {
  local spec_dir="${1:-$VERIFY_PROJECT_DIR/specs}"

  if [[ ! -d "$spec_dir" ]]; then
    verify_error "Spec directory not found: $spec_dir"
    return 1
  fi

  find "$spec_dir" -name "*.md" -type f | sort | while read -r spec_file; do
    basename "$spec_file" .md
  done
}
