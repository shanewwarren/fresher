#!/bin/bash
# State management for Fresher loop executor

STATE_FILE="${FRESHER_STATE_FILE:-.fresher/.state}"

#──────────────────────────────────────────────────────────────────
# Initialize state
#──────────────────────────────────────────────────────────────────
init_state() {
  export FRESHER_ITERATION=0
  export FRESHER_LAST_EXIT_CODE=0
  export FRESHER_LAST_COMMIT_SHA=$(git rev-parse HEAD 2>/dev/null || echo "")
  export FRESHER_STARTED_AT=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
  export FRESHER_TOTAL_COMMITS=0

  write_state
}

#──────────────────────────────────────────────────────────────────
# Load state from file
#──────────────────────────────────────────────────────────────────
load_state() {
  if [[ -f "$STATE_FILE" ]]; then
    source "$STATE_FILE"
  else
    init_state
  fi
}

#──────────────────────────────────────────────────────────────────
# Write state to file
#──────────────────────────────────────────────────────────────────
write_state() {
  cat > "$STATE_FILE" << EOF
FRESHER_ITERATION=$FRESHER_ITERATION
FRESHER_LAST_EXIT_CODE=$FRESHER_LAST_EXIT_CODE
FRESHER_LAST_COMMIT_SHA=$FRESHER_LAST_COMMIT_SHA
FRESHER_STARTED_AT=$FRESHER_STARTED_AT
FRESHER_TOTAL_COMMITS=$FRESHER_TOTAL_COMMITS
EOF
}

#──────────────────────────────────────────────────────────────────
# Increment iteration counter
#──────────────────────────────────────────────────────────────────
increment_iteration() {
  ((FRESHER_ITERATION++))
  write_state
}

#──────────────────────────────────────────────────────────────────
# Update commit tracking
#──────────────────────────────────────────────────────────────────
update_commit_tracking() {
  local current_sha=$(git rev-parse HEAD 2>/dev/null || echo "")

  if [[ -n "$current_sha" && "$current_sha" != "$FRESHER_LAST_COMMIT_SHA" ]]; then
    # Count new commits since last tracked SHA
    if [[ -n "$FRESHER_LAST_COMMIT_SHA" ]]; then
      local new_commits=$(git rev-list --count "$FRESHER_LAST_COMMIT_SHA".."$current_sha" 2>/dev/null || echo "1")
      ((FRESHER_TOTAL_COMMITS += new_commits))
    else
      ((FRESHER_TOTAL_COMMITS++))
    fi
    FRESHER_LAST_COMMIT_SHA="$current_sha"
    write_state
    return 0  # Changes were made
  fi

  return 1  # No changes
}

#──────────────────────────────────────────────────────────────────
# Clear state file
#──────────────────────────────────────────────────────────────────
clear_state() {
  rm -f "$STATE_FILE"
}

#──────────────────────────────────────────────────────────────────
# Get elapsed time since start
#──────────────────────────────────────────────────────────────────
get_elapsed_seconds() {
  local start_epoch=$(date -j -f "%Y-%m-%dT%H:%M:%SZ" "$FRESHER_STARTED_AT" "+%s" 2>/dev/null || date -d "$FRESHER_STARTED_AT" "+%s" 2>/dev/null || echo "0")
  local now_epoch=$(date "+%s")
  echo $((now_epoch - start_epoch))
}
