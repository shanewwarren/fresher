#!/bin/bash
# .fresher/tests/mocks/mock-claude.sh
# Mock Claude Code CLI for testing
# Symlink as 'claude' in test PATH

# Mock behavior controlled by environment variables
MOCK_MODE="${MOCK_CLAUDE_MODE:-success}"
MOCK_DELAY="${MOCK_CLAUDE_DELAY:-0}"
MOCK_COMMITS="${MOCK_CLAUDE_COMMITS:-1}"

# Parsed arguments
OUTPUT_FORMAT="text"
PROMPT=""
MAX_TURNS=""
MODEL=""
NO_SESSION=""
APPEND_SYSTEM=""

# Parse Claude CLI arguments
while [[ $# -gt 0 ]]; do
  case "$1" in
    -p|--print)
      shift
      PROMPT="$1"
      ;;
    --output-format)
      shift
      OUTPUT_FORMAT="$1"
      ;;
    --dangerously-skip-permissions)
      # Accepted, ignored
      ;;
    --max-turns)
      shift
      MAX_TURNS="$1"
      ;;
    --model)
      shift
      MODEL="$1"
      ;;
    --no-session-persistence)
      NO_SESSION="true"
      ;;
    --append-system-prompt)
      shift
      # Ignored
      ;;
    --append-system-prompt-file)
      shift
      APPEND_SYSTEM="$1"
      ;;
    *)
      # Unknown args ignored
      ;;
  esac
  shift
done

# Simulate processing delay
if [[ "$MOCK_DELAY" -gt 0 ]]; then
  sleep "$MOCK_DELAY"
fi

# Helper: emit stream-json event
emit_event() {
  local type="$1"
  local content="$2"
  echo "{\"type\":\"$type\",\"content\":\"$content\"}"
}

# Helper: emit content_block_delta for streaming text
emit_delta() {
  local text="$1"
  echo "{\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"$text\"}}"
}

# Helper: simulate a commit
make_mock_commit() {
  if [[ -d .git ]]; then
    local timestamp
    timestamp=$(date +%s%N 2>/dev/null || date +%s)
    touch ".fresher-mock-change-$timestamp"
    git add -A 2>/dev/null || true
    git commit -m "Mock commit from test" --allow-empty -q 2>/dev/null || true
  fi
}

# Generate response based on mode
case "$MOCK_MODE" in
  success)
    if [[ "$OUTPUT_FORMAT" == "stream-json" ]]; then
      emit_event "message_start" ""
      emit_delta "Analyzing the codebase..."
      emit_event "content_block_stop" ""
      emit_event "tool_use" "Reading files"
      emit_delta "Making changes to implement the task..."
      emit_event "content_block_stop" ""
      emit_delta "Task completed successfully."
      emit_event "message_stop" ""
      emit_event "result" "Task completed. Made changes and committed."
    else
      echo "Analyzing the codebase..."
      echo "Making changes..."
      echo "Task completed successfully."
    fi

    # Simulate commits
    for ((i=0; i<MOCK_COMMITS; i++)); do
      make_mock_commit
    done
    ;;

  no_changes)
    if [[ "$OUTPUT_FORMAT" == "stream-json" ]]; then
      emit_event "message_start" ""
      emit_delta "Checking for remaining work..."
      emit_event "content_block_stop" ""
      emit_delta "All tasks appear to be complete. No changes needed."
      emit_event "message_stop" ""
      emit_event "result" "No changes needed. All tasks complete."
    else
      echo "Checking for work..."
      echo "No changes needed. All tasks complete."
    fi
    ;;

  error)
    if [[ "$OUTPUT_FORMAT" == "stream-json" ]]; then
      emit_event "error" "Mock error occurred"
    else
      echo "Error: Mock error occurred" >&2
    fi
    exit 1
    ;;

  timeout)
    # Sleep longer than test timeout
    sleep 120
    ;;

  partial)
    # Simulate partial work - do some but not all
    if [[ "$OUTPUT_FORMAT" == "stream-json" ]]; then
      emit_event "message_start" ""
      emit_delta "Working on task..."
      emit_event "content_block_stop" ""
      emit_event "result" "Made partial progress."
    else
      echo "Working on task..."
      echo "Made partial progress."
    fi
    make_mock_commit
    ;;

  *)
    echo "Unknown MOCK_CLAUDE_MODE: $MOCK_MODE" >&2
    exit 2
    ;;
esac

exit 0
