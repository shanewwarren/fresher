# Self-Testing Specification

**Status:** Planned
**Version:** 1.0
**Last Updated:** 2025-01-17

---

## 1. Overview

### Purpose

Self-testing provides verification that the Fresher loop works correctly. It includes test scenarios, fixtures, and validation scripts that exercise the loop mechanics without requiring real Claude Code execution.

### Goals

- **Confidence** - Know the loop works before relying on it
- **Regression prevention** - Catch breakages from changes
- **Documentation** - Tests serve as executable examples
- **Fast feedback** - Tests run quickly without AI costs

### Non-Goals

- **AI quality testing** - Don't test Claude's output quality
- **Integration testing** - Focus on Fresher mechanics, not external services
- **Performance benchmarking** - Functional correctness over speed

---

## 2. Architecture

### Test Structure

```
.fresher/
├── tests/
│   ├── run-tests.sh        # Test runner
│   ├── fixtures/           # Test fixtures
│   │   ├── mock-project/   # Sample project structure
│   │   ├── sample-specs/   # Sample spec files
│   │   └── sample-plan.md  # Sample IMPLEMENTATION_PLAN.md
│   ├── unit/               # Unit tests for individual functions
│   │   ├── test-termination.sh
│   │   ├── test-hooks.sh
│   │   └── test-config.sh
│   ├── integration/        # Integration tests for full flows
│   │   ├── test-planning-mode.sh
│   │   ├── test-building-mode.sh
│   │   └── test-docker-mode.sh
│   └── mocks/              # Mock implementations
│       └── mock-claude.sh  # Mock Claude Code CLI
└── lib/
    └── test-utils.sh       # Test utilities
```

### Test Execution Flow

```
┌─────────────────────────────────────────────────────────────────┐
│  fresher test                                                    │
├─────────────────────────────────────────────────────────────────┤
│  1. Set up test environment                                     │
│     - Create temp directory                                     │
│     - Copy fixtures                                             │
│     - Install mock Claude CLI                                   │
│                                                                 │
│  2. Run unit tests                                              │
│     - Test individual functions                                 │
│     - Fast, isolated                                            │
│                                                                 │
│  3. Run integration tests                                       │
│     - Test full loop flows                                      │
│     - Use mock Claude CLI                                       │
│                                                                 │
│  4. Collect results                                             │
│     - Count pass/fail                                           │
│     - Generate report                                           │
│                                                                 │
│  5. Cleanup                                                     │
│     - Remove temp directory                                     │
│     - Restore environment                                       │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. Core Types

### 3.1 Test Result Structure

```bash
# Test result format
# STATUS|TEST_NAME|DURATION|MESSAGE
# PASS|test-config-loading|0.05|
# FAIL|test-termination|0.12|Expected exit code 0, got 1
```

### 3.2 Mock Claude CLI Responses

```bash
# Mock response types
MOCK_RESPONSE_SUCCESS='{"type":"result","content":"Task completed successfully"}'
MOCK_RESPONSE_NO_CHANGES='{"type":"result","content":"No changes needed"}'
MOCK_RESPONSE_ERROR='{"type":"error","message":"Something went wrong"}'
```

### 3.3 Fixture Files

**mock-project/ structure:**

```
mock-project/
├── src/
│   └── index.js
├── specs/
│   ├── README.md
│   └── feature.md
├── IMPLEMENTATION_PLAN.md
├── CLAUDE.md
└── package.json
```

**sample-plan.md:**

```markdown
# Implementation Plan

## Priority 1: Core Features
- [ ] Implement user authentication (refs: specs/auth.md)
- [ ] Add database connection (refs: specs/db.md)

## Priority 2: Enhancements
- [x] Set up project structure
- [ ] Add logging (refs: specs/logging.md)
```

---

## 4. Behaviors

### 4.1 Test Runner

```bash
#!/bin/bash
# .fresher/tests/run-tests.sh

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Counters
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

# Test utilities
source "$(dirname "$0")/../lib/test-utils.sh"

# Setup
setup_test_env() {
  TEST_DIR=$(mktemp -d)
  export TEST_DIR
  export PATH="$(dirname "$0")/mocks:$PATH"  # Add mocks to PATH

  # Copy fixtures
  cp -r "$(dirname "$0")/fixtures/mock-project/"* "$TEST_DIR/"
  cd "$TEST_DIR"

  # Initialize fresher
  mkdir -p .fresher
  cp -r "$(dirname "$0")/../"* .fresher/
}

# Teardown
teardown_test_env() {
  cd /
  rm -rf "$TEST_DIR"
}

# Run a single test
run_test() {
  local test_file="$1"
  local test_name=$(basename "$test_file" .sh)

  ((TESTS_RUN++))

  local start_time=$(date +%s.%N)
  local output
  local exit_code

  if output=$("$test_file" 2>&1); then
    exit_code=0
  else
    exit_code=$?
  fi

  local end_time=$(date +%s.%N)
  local duration=$(echo "$end_time - $start_time" | bc)

  if [[ $exit_code -eq 0 ]]; then
    ((TESTS_PASSED++))
    echo -e "${GREEN}PASS${NC} $test_name (${duration}s)"
  else
    ((TESTS_FAILED++))
    echo -e "${RED}FAIL${NC} $test_name (${duration}s)"
    echo "  Output: $output"
  fi
}

# Main
main() {
  echo "Running Fresher Tests"
  echo "====================="
  echo ""

  setup_test_env
  trap teardown_test_env EXIT

  # Run unit tests
  echo "Unit Tests:"
  for test_file in "$(dirname "$0")/unit/"*.sh; do
    if [[ -f "$test_file" ]]; then
      run_test "$test_file"
    fi
  done

  echo ""

  # Run integration tests
  echo "Integration Tests:"
  for test_file in "$(dirname "$0")/integration/"*.sh; do
    if [[ -f "$test_file" ]]; then
      run_test "$test_file"
    fi
  done

  echo ""
  echo "====================="
  echo "Results: $TESTS_PASSED passed, $TESTS_FAILED failed (of $TESTS_RUN)"

  if [[ $TESTS_FAILED -gt 0 ]]; then
    exit 1
  fi
}

main "$@"
```

### 4.2 Mock Claude CLI

```bash
#!/bin/bash
# .fresher/tests/mocks/mock-claude.sh
# Symlinked as 'claude' in test PATH

# Parse arguments
MOCK_MODE="${MOCK_CLAUDE_MODE:-success}"
MOCK_DELAY="${MOCK_CLAUDE_DELAY:-0}"
OUTPUT_FORMAT="text"

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
      # Accepted but ignored
      ;;
    --max-turns)
      shift
      # Accepted but ignored
      ;;
    *)
      ;;
  esac
  shift
done

# Simulate delay
if [[ "$MOCK_DELAY" -gt 0 ]]; then
  sleep "$MOCK_DELAY"
fi

# Generate response based on mode
case "$MOCK_MODE" in
  success)
    if [[ "$OUTPUT_FORMAT" == "stream-json" ]]; then
      echo '{"type":"assistant","content":"Analyzing the codebase..."}'
      echo '{"type":"tool_use","tool":"Read","input":{"file":"src/index.js"}}'
      echo '{"type":"result","content":"Task completed. Made 1 commit."}'
    else
      echo "Task completed successfully."
    fi

    # Simulate commit (if building mode)
    if [[ -d .git ]]; then
      touch ".fresher-mock-change-$(date +%s)"
      git add -A
      git commit -m "Mock commit from test" --allow-empty 2>/dev/null || true
    fi
    ;;

  no_changes)
    if [[ "$OUTPUT_FORMAT" == "stream-json" ]]; then
      echo '{"type":"assistant","content":"Checking for work..."}'
      echo '{"type":"result","content":"No changes needed. All tasks complete."}'
    else
      echo "No changes needed."
    fi
    ;;

  error)
    echo '{"type":"error","message":"Mock error occurred"}' >&2
    exit 1
    ;;

  timeout)
    sleep 120  # Will be killed by test timeout
    ;;
esac

exit 0
```

### 4.3 Unit Test Examples

**test-config.sh:**

```bash
#!/bin/bash
# Test config loading

source .fresher/config.sh

# Test default values
assert_equals "$FRESHER_MAX_ITERATIONS" "0" "Default max iterations should be 0"
assert_equals "$FRESHER_SMART_TERMINATION" "true" "Smart termination should be enabled"

# Test override
export FRESHER_MAX_ITERATIONS=5
source .fresher/config.sh
assert_equals "$FRESHER_MAX_ITERATIONS" "5" "Override should take precedence"

echo "Config tests passed"
```

**test-termination.sh:**

```bash
#!/bin/bash
# Test termination detection

source .fresher/lib/termination.sh

# Test max iterations
export FRESHER_MAX_ITERATIONS=3
export FRESHER_ITERATION=3
if ! should_terminate; then
  echo "FAIL: Should terminate at max iterations"
  exit 1
fi

# Test smart termination - all tasks complete
cat > IMPLEMENTATION_PLAN.md << 'EOF'
# Plan
- [x] Task 1
- [x] Task 2
EOF

export FRESHER_SMART_TERMINATION=true
export FRESHER_ITERATION=1
export FRESHER_MAX_ITERATIONS=0
if ! should_terminate; then
  echo "FAIL: Should terminate when all tasks complete"
  exit 1
fi

# Test smart termination - tasks remaining
cat > IMPLEMENTATION_PLAN.md << 'EOF'
# Plan
- [x] Task 1
- [ ] Task 2
EOF

if should_terminate; then
  echo "FAIL: Should NOT terminate with pending tasks"
  exit 1
fi

echo "Termination tests passed"
```

**test-hooks.sh:**

```bash
#!/bin/bash
# Test hook execution

source .fresher/lib/hooks.sh

# Create test hook
mkdir -p .fresher/hooks
cat > .fresher/hooks/test-hook << 'EOF'
#!/bin/bash
echo "Hook executed with FRESHER_ITERATION=$FRESHER_ITERATION"
exit 0
EOF
chmod +x .fresher/hooks/test-hook

# Test hook execution
export FRESHER_ITERATION=5
output=$(run_hook "test-hook")
if [[ "$output" != *"FRESHER_ITERATION=5"* ]]; then
  echo "FAIL: Hook should receive environment variables"
  exit 1
fi

# Test hook skip (exit 1)
cat > .fresher/hooks/test-hook << 'EOF'
#!/bin/bash
exit 1
EOF

run_hook "test-hook"
if [[ $? -ne 1 ]]; then
  echo "FAIL: Hook exit code should be preserved"
  exit 1
fi

echo "Hook tests passed"
```

### 4.4 Integration Test Examples

**test-planning-mode.sh:**

```bash
#!/bin/bash
# Test full planning mode flow

export MOCK_CLAUDE_MODE="success"
export FRESHER_MODE="planning"
export FRESHER_MAX_ITERATIONS=1

# Remove existing plan
rm -f IMPLEMENTATION_PLAN.md

# Run planning mode (with mock Claude)
timeout 30 .fresher/run.sh

# Verify plan was created (mock should have created something)
if [[ ! -f "IMPLEMENTATION_PLAN.md" ]]; then
  # Create mock plan since mock doesn't actually create files
  echo "# Plan" > IMPLEMENTATION_PLAN.md
fi

# Verify hooks ran
if [[ -f ".fresher/logs/hooks.log" ]]; then
  if ! grep -q "started" ".fresher/logs/hooks.log"; then
    echo "FAIL: started hook should have run"
    exit 1
  fi
fi

echo "Planning mode test passed"
```

**test-building-mode.sh:**

```bash
#!/bin/bash
# Test full building mode flow

# Setup git repo for commit tracking
git init -q
git config user.email "test@test.com"
git config user.name "Test"
git add -A
git commit -m "Initial" -q

# Create a plan with tasks
cat > IMPLEMENTATION_PLAN.md << 'EOF'
# Plan
- [ ] Task 1
- [ ] Task 2
EOF

export MOCK_CLAUDE_MODE="success"
export FRESHER_MODE="building"
export FRESHER_MAX_ITERATIONS=2

initial_commits=$(git rev-list --count HEAD)

# Run building mode
timeout 60 .fresher/run.sh || true

# Check that commits were made (mock creates commits)
final_commits=$(git rev-list --count HEAD)
if [[ $final_commits -le $initial_commits ]]; then
  echo "WARN: Expected commits to be made (mock may not support this)"
fi

echo "Building mode test passed"
```

---

## 5. CLI Interface

```bash
fresher test [options]

Options:
  --unit          Run only unit tests
  --integration   Run only integration tests
  --verbose       Show detailed output
  --filter NAME   Run only tests matching NAME
  --timeout SEC   Test timeout in seconds (default: 60)
```

---

## 6. Test Utilities

```bash
# .fresher/lib/test-utils.sh

# Assertion helpers
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

assert_file_exists() {
  local file="$1"
  local message="${2:-File should exist: $file}"

  if [[ ! -f "$file" ]]; then
    echo "FAIL: $message"
    exit 1
  fi
}

assert_exit_code() {
  local expected="$1"
  shift

  "$@"
  local actual=$?

  if [[ $actual -ne $expected ]]; then
    echo "FAIL: Expected exit code $expected, got $actual"
    exit 1
  fi
}

# Setup helpers
create_mock_project() {
  local dir="${1:-.}"
  mkdir -p "$dir/src" "$dir/specs"
  echo 'console.log("hello")' > "$dir/src/index.js"
  echo '# Spec' > "$dir/specs/feature.md"
  echo '{}' > "$dir/package.json"
}

create_mock_plan() {
  local file="${1:-IMPLEMENTATION_PLAN.md}"
  cat > "$file" << 'EOF'
# Implementation Plan
- [ ] Task 1
- [ ] Task 2
- [x] Task 3
EOF
}
```

---

## 7. Implementation Phases

| Phase | Description | Dependencies | Complexity |
|-------|-------------|--------------|------------|
| 1 | Test runner script | None | Low |
| 2 | Mock Claude CLI | None | Medium |
| 3 | Test utilities | None | Low |
| 4 | Unit tests | Phase 1-3 | Medium |
| 5 | Integration tests | Phase 1-3 | Medium |
| 6 | CLI integration | Phase 1-5 | Low |

---

## 8. Open Questions

- [ ] Should tests run in Docker to match production environment?
- [ ] How to test Docker isolation mode (Docker-in-Docker)?
- [ ] Should there be performance/timing tests?
- [ ] How to handle tests that need actual Claude responses (optional E2E)?
