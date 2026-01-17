#!/bin/bash
# .fresher/lib/test-utils.sh
# Test utilities and assertion helpers for Fresher self-tests

# ============================================================================
# Assertion Functions
# ============================================================================

# Assert two values are equal
# Usage: assert_equals "actual" "expected" "message"
assert_equals() {
  local actual="$1"
  local expected="$2"
  local message="${3:-Assertion failed}"

  if [[ "$actual" != "$expected" ]]; then
    echo "FAIL: $message"
    echo "  Expected: $expected"
    echo "  Actual:   $actual"
    exit 1
  fi
}

# Assert two values are not equal
# Usage: assert_not_equals "actual" "unexpected" "message"
assert_not_equals() {
  local actual="$1"
  local unexpected="$2"
  local message="${3:-Values should not be equal}"

  if [[ "$actual" == "$unexpected" ]]; then
    echo "FAIL: $message"
    echo "  Should not be: $unexpected"
    echo "  Actual:        $actual"
    exit 1
  fi
}

# Assert string contains substring
# Usage: assert_contains "haystack" "needle" "message"
assert_contains() {
  local haystack="$1"
  local needle="$2"
  local message="${3:-Assertion failed}"

  if [[ "$haystack" != *"$needle"* ]]; then
    echo "FAIL: $message"
    echo "  Expected to contain: $needle"
    echo "  Actual: $haystack"
    exit 1
  fi
}

# Assert string does not contain substring
# Usage: assert_not_contains "haystack" "needle" "message"
assert_not_contains() {
  local haystack="$1"
  local needle="$2"
  local message="${3:-String should not contain value}"

  if [[ "$haystack" == *"$needle"* ]]; then
    echo "FAIL: $message"
    echo "  Should not contain: $needle"
    echo "  Actual: $haystack"
    exit 1
  fi
}

# Assert file exists
# Usage: assert_file_exists "path" "message"
assert_file_exists() {
  local file="$1"
  local message="${2:-File should exist: $file}"

  if [[ ! -f "$file" ]]; then
    echo "FAIL: $message"
    exit 1
  fi
}

# Assert file does not exist
# Usage: assert_file_not_exists "path" "message"
assert_file_not_exists() {
  local file="$1"
  local message="${2:-File should not exist: $file}"

  if [[ -f "$file" ]]; then
    echo "FAIL: $message"
    exit 1
  fi
}

# Assert directory exists
# Usage: assert_dir_exists "path" "message"
assert_dir_exists() {
  local dir="$1"
  local message="${2:-Directory should exist: $dir}"

  if [[ ! -d "$dir" ]]; then
    echo "FAIL: $message"
    exit 1
  fi
}

# Assert command exits with expected code
# Usage: assert_exit_code expected_code command [args...]
assert_exit_code() {
  local expected="$1"
  shift

  set +e
  "$@"
  local actual=$?
  set -e

  if [[ $actual -ne $expected ]]; then
    echo "FAIL: Expected exit code $expected, got $actual"
    echo "  Command: $*"
    exit 1
  fi
}

# Assert command succeeds (exit code 0)
# Usage: assert_success command [args...]
assert_success() {
  assert_exit_code 0 "$@"
}

# Assert command fails (non-zero exit code)
# Usage: assert_failure command [args...]
assert_failure() {
  set +e
  "$@"
  local actual=$?
  set -e

  if [[ $actual -eq 0 ]]; then
    echo "FAIL: Expected command to fail, but it succeeded"
    echo "  Command: $*"
    exit 1
  fi
}

# Assert value is true (non-empty and not "false")
# Usage: assert_true "value" "message"
assert_true() {
  local value="$1"
  local message="${2:-Value should be true}"

  if [[ -z "$value" || "$value" == "false" || "$value" == "0" ]]; then
    echo "FAIL: $message"
    echo "  Value: $value"
    exit 1
  fi
}

# Assert value is false (empty, "false", or "0")
# Usage: assert_false "value" "message"
assert_false() {
  local value="$1"
  local message="${2:-Value should be false}"

  if [[ -n "$value" && "$value" != "false" && "$value" != "0" ]]; then
    echo "FAIL: $message"
    echo "  Value: $value"
    exit 1
  fi
}

# Assert file content matches pattern
# Usage: assert_file_contains "file" "pattern" "message"
assert_file_contains() {
  local file="$1"
  local pattern="$2"
  local message="${3:-File should contain pattern}"

  if [[ ! -f "$file" ]]; then
    echo "FAIL: File does not exist: $file"
    exit 1
  fi

  if ! grep -q "$pattern" "$file"; then
    echo "FAIL: $message"
    echo "  File: $file"
    echo "  Pattern: $pattern"
    exit 1
  fi
}

# ============================================================================
# Setup Helpers
# ============================================================================

# Create a minimal mock project structure
# Usage: create_mock_project [directory]
create_mock_project() {
  local dir="${1:-.}"

  mkdir -p "$dir/src" "$dir/specs"
  echo 'console.log("hello")' > "$dir/src/index.js"
  echo '# Feature Spec' > "$dir/specs/feature.md"
  echo '{}' > "$dir/package.json"
  echo '# Test Project' > "$dir/CLAUDE.md"
}

# Create a mock IMPLEMENTATION_PLAN.md
# Usage: create_mock_plan [file] [pending_count] [completed_count]
create_mock_plan() {
  local file="${1:-IMPLEMENTATION_PLAN.md}"
  local pending="${2:-2}"
  local completed="${3:-1}"

  cat > "$file" << 'HEADER'
# Implementation Plan

## Priority 1: Core Features
HEADER

  local i
  for ((i=1; i<=completed; i++)); do
    echo "- [x] Completed task $i" >> "$file"
  done

  for ((i=1; i<=pending; i++)); do
    echo "- [ ] Pending task $i (refs: specs/feature.md)" >> "$file"
  done
}

# Create a minimal fresher config
# Usage: create_mock_config [directory]
create_mock_config() {
  local dir="${1:-.fresher}"

  mkdir -p "$dir"
  cat > "$dir/config.sh" << 'EOF'
#!/bin/bash
# Fresher Configuration (test)

FRESHER_MODE="${FRESHER_MODE:-building}"
FRESHER_MAX_ITERATIONS="${FRESHER_MAX_ITERATIONS:-0}"
FRESHER_MAX_TURNS="${FRESHER_MAX_TURNS:-100}"
FRESHER_SMART_TERMINATION="${FRESHER_SMART_TERMINATION:-true}"
FRESHER_HOOKS_ENABLED="${FRESHER_HOOKS_ENABLED:-true}"
FRESHER_HOOK_TIMEOUT="${FRESHER_HOOK_TIMEOUT:-30}"
FRESHER_USE_DOCKER="${FRESHER_USE_DOCKER:-false}"
FRESHER_MODEL="${FRESHER_MODEL:-sonnet}"
EOF
}

# Initialize a test git repository
# Usage: init_test_git [directory]
init_test_git() {
  local dir="${1:-.}"

  (
    cd "$dir"
    if ! git rev-parse --git-dir >/dev/null 2>&1; then
      git init -q
      git config user.email "test@fresher.local"
      git config user.name "Fresher Test"
      git add -A 2>/dev/null || true
      git commit -m "Initial test commit" -q --allow-empty 2>/dev/null || true
    fi
  )
}

# ============================================================================
# Utility Functions
# ============================================================================

# Get the number of git commits
# Usage: get_commit_count
get_commit_count() {
  git rev-list --count HEAD 2>/dev/null || echo "0"
}

# Wait for a condition with timeout
# Usage: wait_for condition_command timeout_seconds
wait_for() {
  local condition="$1"
  local timeout="${2:-10}"
  local elapsed=0

  while ! eval "$condition" && [[ $elapsed -lt $timeout ]]; do
    sleep 0.1
    elapsed=$((elapsed + 1))
  done

  if [[ $elapsed -ge $timeout ]]; then
    return 1
  fi
  return 0
}

# Capture output of a command
# Usage: output=$(capture command args)
capture() {
  "$@" 2>&1
}

# Skip test with message
# Usage: skip_test "reason"
skip_test() {
  local reason="${1:-Test skipped}"
  echo "SKIP: $reason"
  exit 0
}

# Mark test as todo (expected to fail)
# Usage: todo_test "reason"
todo_test() {
  local reason="${1:-Test not yet implemented}"
  echo "TODO: $reason"
  exit 0
}
