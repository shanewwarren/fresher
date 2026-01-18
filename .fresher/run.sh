#!/bin/bash
# Fresher Loop Executor
# Main loop that runs Claude Code in iterative cycles

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

#──────────────────────────────────────────────────────────────────
# Load Configuration
#──────────────────────────────────────────────────────────────────

source "$SCRIPT_DIR/config.sh"

#──────────────────────────────────────────────────────────────────
# State Variables
#──────────────────────────────────────────────────────────────────

ITERATION=0
LAST_EXIT_CODE=0
LAST_DURATION=0
COMMITS_MADE=0
STARTED_AT=$(date +%s)
FINISH_TYPE=""

# Export for hooks
export FRESHER_PROJECT_DIR="$PROJECT_DIR"
export FRESHER_ITERATION=0
export FRESHER_LAST_EXIT_CODE=0
export FRESHER_LAST_DURATION=0
export FRESHER_COMMITS_MADE=0
export FRESHER_TOTAL_ITERATIONS=0
export FRESHER_TOTAL_COMMITS=0
export FRESHER_DURATION=0
export FRESHER_FINISH_TYPE=""

#──────────────────────────────────────────────────────────────────
# Helper Functions
#──────────────────────────────────────────────────────────────────

log() {
  echo "[fresher] $*"
}

error() {
  echo "[fresher] ERROR: $*" >&2
}

# Run a hook script if it exists and is executable
# Returns: 0 = continue, 1 = skip iteration, 2 = abort loop
run_hook() {
  local hook_name="$1"
  local hook_path="$SCRIPT_DIR/hooks/$hook_name"

  # Check if hooks are enabled
  if [[ "$FRESHER_HOOKS_ENABLED" != "true" ]]; then
    return 0
  fi

  # Check if hook exists and is executable
  if [[ ! -x "$hook_path" ]]; then
    return 0
  fi

  # Update exported state for hook
  export FRESHER_ITERATION="$ITERATION"
  export FRESHER_LAST_EXIT_CODE="$LAST_EXIT_CODE"
  export FRESHER_LAST_DURATION="$LAST_DURATION"
  export FRESHER_COMMITS_MADE="$COMMITS_MADE"
  export FRESHER_TOTAL_ITERATIONS="$ITERATION"
  export FRESHER_TOTAL_COMMITS="$COMMITS_MADE"
  export FRESHER_DURATION=$(($(date +%s) - STARTED_AT))
  export FRESHER_FINISH_TYPE="$FINISH_TYPE"

  # Run the hook with timeout
  local exit_code=0
  if command -v timeout &> /dev/null; then
    # GNU coreutils timeout available
    timeout "${FRESHER_HOOK_TIMEOUT}s" "$hook_path" || exit_code=$?
  elif command -v gtimeout &> /dev/null; then
    # macOS with coreutils installed via Homebrew
    gtimeout "${FRESHER_HOOK_TIMEOUT}s" "$hook_path" || exit_code=$?
  else
    # No timeout command available, run without timeout
    "$hook_path" || exit_code=$?
  fi

  # Handle timeout (exit code 124)
  if [[ $exit_code -eq 124 ]]; then
    log "Warning: Hook '$hook_name' timed out after ${FRESHER_HOOK_TIMEOUT}s"
    return 0  # Continue despite timeout
  fi

  return $exit_code
}

# Count commits since a given SHA
count_commits_since() {
  local since_sha="$1"
  if [[ -z "$since_sha" ]]; then
    echo "0"
    return
  fi
  git rev-list --count "$since_sha"..HEAD 2>/dev/null || echo "0"
}

# Write current state to .fresher/.state file
write_state() {
  local state_file="$SCRIPT_DIR/.state"
  local current_sha
  current_sha=$(git rev-parse HEAD 2>/dev/null || echo "")
  local duration=$(($(date +%s) - STARTED_AT))
  local started_iso
  started_iso=$(date -r "$STARTED_AT" +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date -d "@$STARTED_AT" +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || echo "")

  cat > "$state_file" << EOF
# Fresher state file - auto-generated
# Last updated: $(date +%Y-%m-%dT%H:%M:%SZ)
ITERATION=$ITERATION
LAST_EXIT_CODE=$LAST_EXIT_CODE
LAST_COMMIT_SHA=$current_sha
STARTED_AT=$started_iso
TOTAL_COMMITS=$COMMITS_MADE
DURATION=$duration
FINISH_TYPE=$FINISH_TYPE
EOF
}

#──────────────────────────────────────────────────────────────────
# Signal Handling
#──────────────────────────────────────────────────────────────────

cleanup() {
  local exit_code=$?

  # Set finish type if not already set
  if [[ -z "$FINISH_TYPE" ]]; then
    FINISH_TYPE="manual"
  fi

  # Update final state for hooks
  export FRESHER_TOTAL_ITERATIONS="$ITERATION"
  export FRESHER_TOTAL_COMMITS="$COMMITS_MADE"
  export FRESHER_DURATION=$(($(date +%s) - STARTED_AT))
  export FRESHER_FINISH_TYPE="$FINISH_TYPE"

  # Write final state to file
  write_state

  # Run finished hook
  run_hook "finished" || true

  exit $exit_code
}

# Trap SIGINT (Ctrl+C) and SIGTERM
trap 'FINISH_TYPE="manual"; exit 130' INT TERM
trap cleanup EXIT

#──────────────────────────────────────────────────────────────────
# Docker Isolation Check
#──────────────────────────────────────────────────────────────────

if [[ "$FRESHER_USE_DOCKER" == "true" ]]; then
  # Already in a devcontainer?
  if [[ "$DEVCONTAINER" == "true" ]] || [[ "$FRESHER_IN_DOCKER" == "true" ]]; then
    log "Running in devcontainer environment"
  else
    error "Docker isolation enabled but not in devcontainer."
    echo ""
    echo "Options:"
    echo "  1. Open this folder in VS Code and use 'Reopen in Container'"
    echo "     (copy .fresher/docker/devcontainer.json to .devcontainer/)"
    echo "  2. Run: docker compose -f .fresher/docker/docker-compose.yml run --rm fresher"
    echo ""
    echo "To disable Docker isolation: export FRESHER_USE_DOCKER=false"
    exit 1
  fi
fi

#──────────────────────────────────────────────────────────────────
# Validation
#──────────────────────────────────────────────────────────────────

# Validate mode
if [[ "$FRESHER_MODE" != "planning" && "$FRESHER_MODE" != "building" ]]; then
  error "Invalid FRESHER_MODE: $FRESHER_MODE (must be 'planning' or 'building')"
  exit 1
fi

# Check prompt file exists
PROMPT_FILE="$SCRIPT_DIR/PROMPT.${FRESHER_MODE}.md"
if [[ ! -f "$PROMPT_FILE" ]]; then
  error "Prompt file not found: $PROMPT_FILE"
  exit 1
fi

# Check claude command exists
if ! command -v claude &> /dev/null; then
  error "claude command not found. Install Claude Code CLI first."
  exit 1
fi

# Check jq command exists (required for stream-json parsing)
if ! command -v jq &> /dev/null; then
  error "jq command not found. Install jq for JSON parsing."
  exit 1
fi

#──────────────────────────────────────────────────────────────────
# Log Directory Setup
#──────────────────────────────────────────────────────────────────

# Ensure log directory exists
if [[ ! -d "$FRESHER_LOG_DIR" ]]; then
  mkdir -p "$FRESHER_LOG_DIR"
fi

#──────────────────────────────────────────────────────────────────
# Main Loop
#──────────────────────────────────────────────────────────────────

log "Starting Fresher loop"
log "Mode: $FRESHER_MODE"
log "Press Ctrl+C to stop"
echo ""

# Run started hook
if ! run_hook "started"; then
  hook_exit=$?
  if [[ $hook_exit -eq 2 ]]; then
    FINISH_TYPE="hook_abort"
    error "Started hook aborted the loop"
    exit 1
  fi
fi

# Record initial commit SHA for change detection
INITIAL_SHA=$(git rev-parse HEAD 2>/dev/null || echo "")

# Main execution loop
while true; do
  # Increment iteration
  ((ITERATION++))

  # Record iteration start
  ITERATION_START=$(date +%s)
  ITERATION_SHA=$(git rev-parse HEAD 2>/dev/null || echo "")

  log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  log "Iteration $ITERATION"
  log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

  # Run next_iteration hook
  if ! run_hook "next_iteration"; then
    hook_exit=$?
    if [[ $hook_exit -eq 1 ]]; then
      log "Skipping iteration (hook returned 1)"
      continue
    elif [[ $hook_exit -eq 2 ]]; then
      FINISH_TYPE="hook_abort"
      error "next_iteration hook aborted the loop"
      exit 1
    fi
  fi

  # Build Claude Code command
  CLAUDE_CMD=(claude)
  CLAUDE_CMD+=(-p "$(cat "$PROMPT_FILE")")

  # Add AGENTS.md if it exists
  if [[ -f "$SCRIPT_DIR/AGENTS.md" ]]; then
    CLAUDE_CMD+=(--append-system-prompt-file "$SCRIPT_DIR/AGENTS.md")
  fi

  # Add dangerous permissions flag if enabled
  if [[ "$FRESHER_DANGEROUS_PERMISSIONS" == "true" ]]; then
    CLAUDE_CMD+=(--dangerously-skip-permissions)
  fi

  # Add max turns
  CLAUDE_CMD+=(--max-turns "$FRESHER_MAX_TURNS")

  # Disable session persistence (fresh context each iteration)
  CLAUDE_CMD+=(--no-session-persistence)

  # Set model
  CLAUDE_CMD+=(--model "$FRESHER_MODEL")

  # Add stream-json output format for parsing (requires --verbose with -p)
  CLAUDE_CMD+=(--verbose --output-format stream-json)

  # Set up iteration log file
  LOG_FILE="$FRESHER_LOG_DIR/iteration-${ITERATION}.log"

  # Invoke Claude Code with streaming
  log "Invoking Claude Code..."
  log "Log file: $LOG_FILE"
  echo ""

  # Use pipefail to capture Claude's exit code through the pipe
  set -o pipefail

  LAST_EXIT_CODE=0
  "${CLAUDE_CMD[@]}" 2>&1 | while IFS= read -r line; do
    # Write every line to log file
    echo "$line" >> "$LOG_FILE"

    # Try to parse as JSON
    event_type=$(echo "$line" | jq -r '.type // empty' 2>/dev/null)

    case "$event_type" in
      "assistant")
        # Extract and display assistant message content
        content=$(echo "$line" | jq -r '.message.content[]? | select(.type == "text") | .text // empty' 2>/dev/null)
        if [[ -n "$content" ]]; then
          echo "$content"
        fi
        ;;
      "content_block_delta")
        # Stream text deltas in real-time
        delta=$(echo "$line" | jq -r '.delta.text // empty' 2>/dev/null)
        if [[ -n "$delta" ]]; then
          printf "%s" "$delta"
        fi
        ;;
      "content_block_stop")
        # End of a content block - add newline if needed
        ;;
      "result")
        # Final result - could be used for termination analysis
        # For now, just log it (already written to log file above)
        ;;
      "")
        # Not JSON or empty type - might be stderr or plain text
        # Display as-is if it's not empty
        if [[ -n "$line" ]]; then
          echo "$line"
        fi
        ;;
    esac
  done || LAST_EXIT_CODE=$?

  # Restore default behavior
  set +o pipefail

  echo ""

  # Record iteration duration
  LAST_DURATION=$(($(date +%s) - ITERATION_START))

  # Count new commits this iteration
  CURRENT_SHA=$(git rev-parse HEAD 2>/dev/null || echo "")
  if [[ -n "$ITERATION_SHA" && -n "$CURRENT_SHA" && "$ITERATION_SHA" != "$CURRENT_SHA" ]]; then
    NEW_COMMITS=$(git rev-list --count "$ITERATION_SHA".."$CURRENT_SHA" 2>/dev/null || echo "0")
    COMMITS_MADE=$((COMMITS_MADE + NEW_COMMITS))
    log "Commits this iteration: $NEW_COMMITS (total: $COMMITS_MADE)"
  fi

  log "Iteration $ITERATION complete (exit code: $LAST_EXIT_CODE, duration: ${LAST_DURATION}s)"

  # Check for error exit
  if [[ $LAST_EXIT_CODE -ne 0 ]]; then
    FINISH_TYPE="error"
    error "Claude Code exited with error code $LAST_EXIT_CODE"
    exit $LAST_EXIT_CODE
  fi

  # Check max iterations limit
  if [[ "$FRESHER_MAX_ITERATIONS" -gt 0 && "$ITERATION" -ge "$FRESHER_MAX_ITERATIONS" ]]; then
    FINISH_TYPE="max_iterations"
    log "Reached max iterations limit ($FRESHER_MAX_ITERATIONS)"
    exit 0
  fi

  # Smart termination detection
  if [[ "$FRESHER_SMART_TERMINATION" == "true" ]]; then
    # Check for completion: no pending tasks in IMPLEMENTATION_PLAN.md
    if [[ -f "$PROJECT_DIR/IMPLEMENTATION_PLAN.md" ]]; then
      pending_tasks=$(grep -cE '^\s*-\s*\[\s\]' "$PROJECT_DIR/IMPLEMENTATION_PLAN.md" 2>/dev/null || echo "0")
      if [[ "$pending_tasks" -eq 0 ]]; then
        FINISH_TYPE="complete"
        log "All tasks in IMPLEMENTATION_PLAN.md are complete!"
        exit 0
      fi
    fi

    # Check for no changes: HEAD didn't move this iteration
    if [[ -n "$ITERATION_SHA" && -n "$CURRENT_SHA" && "$ITERATION_SHA" == "$CURRENT_SHA" ]]; then
      FINISH_TYPE="no_changes"
      log "No commits made this iteration - stopping to avoid infinite loop"
      exit 0
    fi
  fi

  # Continue to next iteration
done
