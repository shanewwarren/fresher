#!/bin/bash
# Termination detection for Fresher loop executor

#──────────────────────────────────────────────────────────────────
# Check if loop should terminate
# Sets FRESHER_FINISH_TYPE and returns 0 if should terminate
#──────────────────────────────────────────────────────────────────
should_terminate() {
  # Priority 1: Max iterations (if set and > 0)
  if [[ "${FRESHER_MAX_ITERATIONS:-0}" -gt 0 ]]; then
    if [[ "$FRESHER_ITERATION" -ge "$FRESHER_MAX_ITERATIONS" ]]; then
      export FRESHER_FINISH_TYPE="max_iterations"
      return 0
    fi
  fi

  # Priority 2: Smart termination (if enabled)
  if [[ "${FRESHER_SMART_TERMINATION:-true}" == "true" ]]; then
    # Check 2a: All tasks complete in IMPLEMENTATION_PLAN.md
    if check_plan_complete; then
      export FRESHER_FINISH_TYPE="complete"
      return 0
    fi

    # Check 2b: No commits made this iteration (only after first iteration)
    if [[ "$FRESHER_ITERATION" -gt 1 ]]; then
      if ! update_commit_tracking; then
        export FRESHER_FINISH_TYPE="no_changes"
        return 0
      fi
    fi
  fi

  # Continue looping
  return 1
}

#──────────────────────────────────────────────────────────────────
# Check if IMPLEMENTATION_PLAN.md has no pending tasks
#──────────────────────────────────────────────────────────────────
check_plan_complete() {
  local plan_file="${FRESHER_PLAN_FILE:-IMPLEMENTATION_PLAN.md}"

  # No plan file = not complete (planning mode should create one)
  if [[ ! -f "$plan_file" ]]; then
    return 1
  fi

  # Count uncompleted tasks (- [ ] pattern)
  local pending_tasks
  pending_tasks=$(grep -cE '^\s*-\s*\[\s\]' "$plan_file" 2>/dev/null)
  pending_tasks="${pending_tasks:-0}"

  if [[ "$pending_tasks" -eq 0 ]]; then
    # Also check there are SOME completed tasks (not an empty plan)
    local completed_tasks
    completed_tasks=$(grep -cE '^\s*-\s*\[x\]' "$plan_file" 2>/dev/null)
    completed_tasks="${completed_tasks:-0}"

    if [[ "$completed_tasks" -gt 0 ]]; then
      return 0  # Plan is complete
    fi
  fi

  return 1  # Still has pending tasks or empty plan
}

#──────────────────────────────────────────────────────────────────
# Check for error conditions
#──────────────────────────────────────────────────────────────────
check_error_termination() {
  local exit_code="$1"

  # Non-zero exit from Claude Code is an error
  if [[ "$exit_code" -ne 0 ]]; then
    export FRESHER_FINISH_TYPE="error"
    export FRESHER_LAST_EXIT_CODE="$exit_code"
    return 0
  fi

  return 1
}

#──────────────────────────────────────────────────────────────────
# Get human-readable finish message
#──────────────────────────────────────────────────────────────────
get_finish_message() {
  local finish_type="${1:-$FRESHER_FINISH_TYPE}"

  case "$finish_type" in
    manual)
      echo "Loop stopped by user (Ctrl+C)"
      ;;
    max_iterations)
      echo "Reached maximum iterations ($FRESHER_MAX_ITERATIONS)"
      ;;
    complete)
      echo "All tasks in implementation plan completed!"
      ;;
    no_changes)
      echo "No changes made in last iteration"
      ;;
    error)
      echo "Claude Code exited with error (code: $FRESHER_LAST_EXIT_CODE)"
      ;;
    hook_abort)
      echo "Loop aborted by hook"
      ;;
    *)
      echo "Loop finished: $finish_type"
      ;;
  esac
}
