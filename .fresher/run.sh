#!/bin/bash
# Fresher Loop Executor
# Runs Claude Code in iterative cycles with fresh context each iteration

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

#──────────────────────────────────────────────────────────────────
# Colors
#──────────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

#──────────────────────────────────────────────────────────────────
# Load configuration and libraries
#──────────────────────────────────────────────────────────────────
source "$SCRIPT_DIR/config.sh"
source "$SCRIPT_DIR/lib/state.sh"
source "$SCRIPT_DIR/lib/termination.sh"

# Export project directory for hooks
export FRESHER_PROJECT_DIR="$PROJECT_DIR"

#──────────────────────────────────────────────────────────────────
# Logging
#──────────────────────────────────────────────────────────────────
log_info() {
  echo -e "${BLUE}[fresher]${NC} $1"
}

log_success() {
  echo -e "${GREEN}[fresher]${NC} $1"
}

log_warn() {
  echo -e "${YELLOW}[fresher]${NC} $1"
}

log_error() {
  echo -e "${RED}[fresher]${NC} $1"
}

#──────────────────────────────────────────────────────────────────
# Hook execution
#──────────────────────────────────────────────────────────────────
run_hook() {
  local hook_name="$1"
  local hook_path="$SCRIPT_DIR/hooks/$hook_name"

  if [[ ! -x "$hook_path" ]]; then
    if [[ -f "$hook_path" ]]; then
      log_warn "Hook $hook_name exists but is not executable"
    fi
    return 0
  fi

  local exit_code=0
  "$hook_path" || exit_code=$?

  case $exit_code in
    0)
      # Success
      ;;
    1)
      log_warn "Hook $hook_name returned warning (exit 1)"
      ;;
    2)
      log_error "Hook $hook_name requested abort (exit 2)"
      export FRESHER_FINISH_TYPE="hook_abort"
      return 2
      ;;
    *)
      log_error "Hook $hook_name failed with exit code $exit_code"
      ;;
  esac

  return 0
}

#──────────────────────────────────────────────────────────────────
# Cleanup handler
#──────────────────────────────────────────────────────────────────
cleanup() {
  local exit_code=$?

  echo ""
  log_info "Shutting down..."

  # Set finish type if not already set
  FRESHER_FINISH_TYPE="${FRESHER_FINISH_TYPE:-manual}"

  # Export final statistics
  export FRESHER_TOTAL_ITERATIONS="$FRESHER_ITERATION"
  export FRESHER_DURATION=$(get_elapsed_seconds)

  # Run finished hook
  run_hook "finished" || true

  # Write final state
  write_state

  # Print summary
  echo ""
  echo -e "${CYAN}════════════════════════════════════════════════════════════════${NC}"
  log_info "$(get_finish_message)"
  log_info "Total iterations: $FRESHER_TOTAL_ITERATIONS"
  log_info "Total commits: $FRESHER_TOTAL_COMMITS"
  log_info "Duration: ${FRESHER_DURATION}s"
  echo -e "${CYAN}════════════════════════════════════════════════════════════════${NC}"

  exit $exit_code
}

#──────────────────────────────────────────────────────────────────
# Signal handlers
#──────────────────────────────────────────────────────────────────
trap cleanup EXIT
trap 'FRESHER_FINISH_TYPE="manual"; exit 130' INT TERM

#──────────────────────────────────────────────────────────────────
# Validate environment
#──────────────────────────────────────────────────────────────────
validate_environment() {
  # Check for claude CLI
  if ! command -v claude &>/dev/null; then
    log_error "Claude Code CLI not found. Please install it first."
    log_error "Visit: https://claude.ai/code"
    exit 1
  fi

  # Check mode
  if [[ "$FRESHER_MODE" != "planning" && "$FRESHER_MODE" != "building" ]]; then
    log_error "Invalid FRESHER_MODE: $FRESHER_MODE"
    log_error "Must be 'planning' or 'building'"
    exit 1
  fi

  # Check prompt file exists
  local prompt_file="$SCRIPT_DIR/PROMPT.${FRESHER_MODE}.md"
  if [[ ! -f "$prompt_file" ]]; then
    log_error "Prompt file not found: $prompt_file"
    exit 1
  fi

  # Check AGENTS.md exists
  if [[ ! -f "$SCRIPT_DIR/AGENTS.md" ]]; then
    log_error "AGENTS.md not found: $SCRIPT_DIR/AGENTS.md"
    exit 1
  fi
}

#──────────────────────────────────────────────────────────────────
# Build Claude command
#──────────────────────────────────────────────────────────────────
build_claude_command() {
  local cmd="claude"

  # Add prompt
  cmd+=" -p \"\$(cat $SCRIPT_DIR/PROMPT.${FRESHER_MODE}.md)\""

  # Add system prompt file
  cmd+=" --append-system-prompt-file $SCRIPT_DIR/AGENTS.md"

  # Add dangerous permissions if enabled
  if [[ "${FRESHER_DANGEROUS_PERMISSIONS:-true}" == "true" ]]; then
    cmd+=" --dangerously-skip-permissions"
  fi

  # Add max turns
  cmd+=" --max-turns ${FRESHER_MAX_TURNS:-50}"

  # Add model
  cmd+=" --model ${FRESHER_MODEL:-sonnet}"

  # Disable session persistence for fresh context each iteration
  cmd+=" --no-session-persistence"

  # Output format for streaming
  cmd+=" --output-format stream-json"

  echo "$cmd"
}

#──────────────────────────────────────────────────────────────────
# Run Claude Code iteration
#──────────────────────────────────────────────────────────────────
run_iteration() {
  local iteration="$1"
  local log_file="$FRESHER_LOG_DIR/iteration-${iteration}.log"

  # Ensure log directory exists
  mkdir -p "$FRESHER_LOG_DIR"

  # Record commit SHA before iteration
  local sha_before=$(git rev-parse HEAD 2>/dev/null || echo "")

  log_info "Running Claude Code..."
  echo ""

  # Build and run Claude command with streaming output
  local exit_code=0

  # Run Claude with output streaming
  eval "claude \
    -p \"\$(cat $SCRIPT_DIR/PROMPT.${FRESHER_MODE}.md)\" \
    --append-system-prompt-file $SCRIPT_DIR/AGENTS.md \
    $([ \"${FRESHER_DANGEROUS_PERMISSIONS:-true}\" == \"true\" ] && echo \"--dangerously-skip-permissions\") \
    --max-turns ${FRESHER_MAX_TURNS:-50} \
    --model ${FRESHER_MODEL:-sonnet} \
    --no-session-persistence \
    --output-format stream-json" 2>&1 | tee -a "$log_file" | while IFS= read -r line; do
      # Try to parse as JSON and extract content
      if echo "$line" | jq -e '.type' &>/dev/null; then
        local event_type=$(echo "$line" | jq -r '.type // empty')
        case "$event_type" in
          assistant)
            # Display assistant message content
            local content=$(echo "$line" | jq -r '.message.content[]?.text // empty' 2>/dev/null)
            if [[ -n "$content" ]]; then
              echo "$content"
            fi
            ;;
          result)
            # Final result
            local result_text=$(echo "$line" | jq -r '.result // empty' 2>/dev/null)
            if [[ -n "$result_text" ]]; then
              echo ""
              echo "$result_text"
            fi
            ;;
        esac
      else
        # Not JSON, print as-is (could be stderr or other output)
        echo "$line"
      fi
    done || exit_code=$?

  echo ""

  # Update state with exit code
  FRESHER_LAST_EXIT_CODE="$exit_code"

  # Track commits
  update_commit_tracking || true

  return $exit_code
}

#──────────────────────────────────────────────────────────────────
# Main loop
#──────────────────────────────────────────────────────────────────
main() {
  # Print header
  echo -e "${CYAN}════════════════════════════════════════════════════════════════${NC}"
  echo -e "${CYAN}  Fresher Loop Executor${NC}"
  echo -e "${CYAN}════════════════════════════════════════════════════════════════${NC}"
  echo ""
  log_info "Mode: $FRESHER_MODE"
  log_info "Max iterations: ${FRESHER_MAX_ITERATIONS:-0} (0=unlimited)"
  log_info "Smart termination: ${FRESHER_SMART_TERMINATION:-true}"
  log_info "Model: ${FRESHER_MODEL:-sonnet}"
  echo ""

  # Validate environment
  validate_environment

  # Initialize state
  init_state

  # Run started hook
  if ! run_hook "started"; then
    exit 1
  fi

  # Main loop
  while true; do
    # Increment iteration
    increment_iteration

    echo ""
    echo -e "${CYAN}────────────────────────────────────────────────────────────────${NC}"
    log_info "Iteration $FRESHER_ITERATION"
    echo -e "${CYAN}────────────────────────────────────────────────────────────────${NC}"
    echo ""

    # Run next_iteration hook
    if ! run_hook "next_iteration"; then
      if [[ "$FRESHER_FINISH_TYPE" == "hook_abort" ]]; then
        break
      fi
    fi

    # Run Claude Code
    local iteration_exit_code=0
    run_iteration "$FRESHER_ITERATION" || iteration_exit_code=$?

    # Check for errors
    if check_error_termination "$iteration_exit_code"; then
      log_error "Claude Code exited with error"
      break
    fi

    # Check termination conditions
    if should_terminate; then
      log_success "$(get_finish_message)"
      break
    fi

    log_info "Iteration $FRESHER_ITERATION complete. Continuing..."
  done
}

# Change to project directory
cd "$PROJECT_DIR"

# Run main
main "$@"
