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

  # Run the hook
  local exit_code=0
  "$hook_path" || exit_code=$?

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

#──────────────────────────────────────────────────────────────────
# Signal Handling
#──────────────────────────────────────────────────────────────────

cleanup() {
  local exit_code=$?

  # Set finish type if not already set
  if [[ -z "$FINISH_TYPE" ]]; then
    FINISH_TYPE="manual"
  fi

  # Update final state
  export FRESHER_TOTAL_ITERATIONS="$ITERATION"
  export FRESHER_TOTAL_COMMITS="$COMMITS_MADE"
  export FRESHER_DURATION=$(($(date +%s) - STARTED_AT))
  export FRESHER_FINISH_TYPE="$FINISH_TYPE"

  # Run finished hook
  run_hook "finished" || true

  exit $exit_code
}

# Trap SIGINT (Ctrl+C) and SIGTERM
trap 'FINISH_TYPE="manual"; exit 130' INT TERM
trap cleanup EXIT

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

  # Invoke Claude Code
  log "Invoking Claude Code..."
  echo ""

  LAST_EXIT_CODE=0
  "${CLAUDE_CMD[@]}" || LAST_EXIT_CODE=$?

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

  # Continue to next iteration
  # (Termination conditions will be added in later phases)
done
