#!/bin/bash
# .fresher/tests/run-tests.sh
# Test runner for Fresher self-tests

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FRESHER_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Counters
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_SKIPPED=0

# Options
RUN_UNIT=true
RUN_INTEGRATION=true
VERBOSE=false
FILTER=""
TEST_TIMEOUT=60

# Parse arguments
while [[ $# -gt 0 ]]; do
  case "$1" in
    --unit)
      RUN_UNIT=true
      RUN_INTEGRATION=false
      ;;
    --integration)
      RUN_UNIT=false
      RUN_INTEGRATION=true
      ;;
    --verbose|-v)
      VERBOSE=true
      ;;
    --filter)
      shift
      FILTER="$1"
      ;;
    --timeout)
      shift
      TEST_TIMEOUT="$1"
      ;;
    --help|-h)
      echo "Usage: run-tests.sh [options]"
      echo ""
      echo "Options:"
      echo "  --unit          Run only unit tests"
      echo "  --integration   Run only integration tests"
      echo "  --verbose, -v   Show detailed output"
      echo "  --filter NAME   Run only tests matching NAME"
      echo "  --timeout SEC   Test timeout in seconds (default: 60)"
      echo "  --help, -h      Show this help"
      exit 0
      ;;
    *)
      echo "Unknown option: $1"
      exit 1
      ;;
  esac
  shift
done

# Load test utilities
if [[ -f "$FRESHER_DIR/lib/test-utils.sh" ]]; then
  source "$FRESHER_DIR/lib/test-utils.sh"
else
  echo -e "${YELLOW}Warning: test-utils.sh not found, some features may be unavailable${NC}"
fi

# Setup test environment
setup_test_env() {
  TEST_DIR=$(mktemp -d)
  export TEST_DIR
  export ORIGINAL_DIR="$PWD"

  # Add mocks to PATH (before system PATH)
  export PATH="$SCRIPT_DIR/mocks:$PATH"

  # Copy fixtures if they exist
  if [[ -d "$SCRIPT_DIR/fixtures/mock-project" ]]; then
    cp -r "$SCRIPT_DIR/fixtures/mock-project/"* "$TEST_DIR/" 2>/dev/null || true
  fi

  # Create minimal project structure if fixtures don't exist
  mkdir -p "$TEST_DIR/src" "$TEST_DIR/specs" "$TEST_DIR/.fresher"

  # Copy fresher to test directory
  cp -r "$FRESHER_DIR/"* "$TEST_DIR/.fresher/" 2>/dev/null || true

  # Initialize a git repo for tests that need it
  cd "$TEST_DIR"
  if ! git rev-parse --git-dir >/dev/null 2>&1; then
    git init -q
    git config user.email "test@fresher.local"
    git config user.name "Fresher Test"
    # Create initial commit
    echo "# Test Project" > README.md
    git add -A
    git commit -m "Initial test commit" -q
  fi

  if $VERBOSE; then
    echo -e "${BLUE}Test environment: $TEST_DIR${NC}"
  fi
}

# Teardown test environment
teardown_test_env() {
  cd "$ORIGINAL_DIR" 2>/dev/null || cd /
  if [[ -n "$TEST_DIR" && -d "$TEST_DIR" ]]; then
    rm -rf "$TEST_DIR"
  fi
}

# Run a single test
run_test() {
  local test_file="$1"
  local test_name
  test_name=$(basename "$test_file" .sh)

  # Apply filter if set
  if [[ -n "$FILTER" && "$test_name" != *"$FILTER"* ]]; then
    return 0
  fi

  ((TESTS_RUN++))

  # Get start time (portable)
  local start_time
  if command -v gdate >/dev/null 2>&1; then
    start_time=$(gdate +%s.%N)
  elif date +%s.%N >/dev/null 2>&1; then
    start_time=$(date +%s.%N)
  else
    start_time=$(date +%s)
  fi

  local output
  local exit_code

  # Run test with timeout
  if command -v timeout >/dev/null 2>&1; then
    if output=$(timeout "$TEST_TIMEOUT" bash "$test_file" 2>&1); then
      exit_code=0
    else
      exit_code=$?
    fi
  elif command -v gtimeout >/dev/null 2>&1; then
    if output=$(gtimeout "$TEST_TIMEOUT" bash "$test_file" 2>&1); then
      exit_code=0
    else
      exit_code=$?
    fi
  else
    # No timeout command available, run directly
    if output=$(bash "$test_file" 2>&1); then
      exit_code=0
    else
      exit_code=$?
    fi
  fi

  # Get end time and calculate duration
  local end_time duration
  if command -v gdate >/dev/null 2>&1; then
    end_time=$(gdate +%s.%N)
  elif date +%s.%N >/dev/null 2>&1; then
    end_time=$(date +%s.%N)
  else
    end_time=$(date +%s)
  fi

  if command -v bc >/dev/null 2>&1; then
    duration=$(echo "$end_time - $start_time" | bc 2>/dev/null || echo "0")
  else
    duration=$((${end_time%.*} - ${start_time%.*}))
  fi

  # Handle timeout exit code (124)
  if [[ $exit_code -eq 124 ]]; then
    ((TESTS_FAILED++))
    echo -e "${RED}TIMEOUT${NC} $test_name (>${TEST_TIMEOUT}s)"
    if $VERBOSE; then
      echo "  Output: $output"
    fi
    return
  fi

  if [[ $exit_code -eq 0 ]]; then
    ((TESTS_PASSED++))
    echo -e "${GREEN}PASS${NC} $test_name (${duration}s)"
    if $VERBOSE && [[ -n "$output" ]]; then
      echo "$output" | sed 's/^/  /'
    fi
  else
    ((TESTS_FAILED++))
    echo -e "${RED}FAIL${NC} $test_name (${duration}s)"
    echo "$output" | sed 's/^/  /'
  fi
}

# Run tests in a directory
run_test_dir() {
  local dir="$1"
  local label="$2"
  local found=0

  if [[ ! -d "$dir" ]]; then
    if $VERBOSE; then
      echo -e "${YELLOW}No $label directory found${NC}"
    fi
    return
  fi

  echo "$label:"

  for test_file in "$dir"/*.sh; do
    if [[ -f "$test_file" ]]; then
      found=1
      # Setup fresh environment for each test
      setup_test_env
      trap teardown_test_env EXIT

      run_test "$test_file"

      teardown_test_env
      trap - EXIT
    fi
  done

  if [[ $found -eq 0 ]]; then
    echo -e "  ${YELLOW}(no tests found)${NC}"
  fi

  echo ""
}

# Main
main() {
  echo "Running Fresher Tests"
  echo "====================="
  echo ""

  # Run unit tests
  if $RUN_UNIT; then
    run_test_dir "$SCRIPT_DIR/unit" "Unit Tests"
  fi

  # Run integration tests
  if $RUN_INTEGRATION; then
    run_test_dir "$SCRIPT_DIR/integration" "Integration Tests"
  fi

  echo "====================="
  echo -e "Results: ${GREEN}$TESTS_PASSED passed${NC}, ${RED}$TESTS_FAILED failed${NC} (of $TESTS_RUN)"

  if [[ $TESTS_FAILED -gt 0 ]]; then
    exit 1
  fi

  if [[ $TESTS_RUN -eq 0 ]]; then
    echo -e "${YELLOW}Warning: No tests were run${NC}"
    exit 0
  fi
}

main "$@"
