#!/bin/bash
# Integration test for max iterations termination
# Tests: loop stops at configured max iterations

set -e

# Get test directory
TEST_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$TEST_SCRIPT_DIR/../../lib/test-utils.sh"

echo "Testing max iterations termination..."

# Setup
export MOCK_CLAUDE_MODE="success"
export FRESHER_MODE="building"
export FRESHER_MAX_ITERATIONS=3
export FRESHER_SMART_TERMINATION="false"
export FRESHER_HOOKS_ENABLED="false"
export FRESHER_USE_DOCKER="false"

# Ensure config exists
if [[ ! -f .fresher/config.sh ]]; then
  create_mock_config
fi

# Create prompts if missing
if [[ ! -f .fresher/PROMPT.building.md ]]; then
  echo "# Building Prompt" > .fresher/PROMPT.building.md
fi

# Create plan with enough tasks to exceed max iterations
create_mock_plan IMPLEMENTATION_PLAN.md 10 1

# Initialize git
init_test_git .

# Check if run.sh exists
if [[ ! -f .fresher/run.sh ]]; then
  echo "  Skipping: run.sh not found"
  exit 0
fi

# Check if mock claude is in PATH
if ! command -v claude >/dev/null 2>&1; then
  echo "  Skipping: mock claude not in PATH"
  exit 0
fi

# Run and capture timing
start_time=$(date +%s)
echo "  Running with max_iterations=3..."
timeout 120 .fresher/run.sh || true
end_time=$(date +%s)
duration=$((end_time - start_time))

echo "    Duration: ${duration}s"

# Check final iteration in state
if [[ -f .fresher/.state ]]; then
  if grep -q "ITERATION=" .fresher/.state; then
    iteration=$(grep "ITERATION=" .fresher/.state | cut -d= -f2)
    echo "    Final iteration: $iteration"

    # Should have stopped at or before max_iterations
    if [[ $iteration -le $FRESHER_MAX_ITERATIONS ]]; then
      echo "    Stopped at or before max iterations"
    else
      echo "FAIL: Exceeded max iterations (got $iteration, max $FRESHER_MAX_ITERATIONS)"
      exit 1
    fi
  fi

  # Check finish type
  if grep -q "FINISH_TYPE=" .fresher/.state; then
    finish_type=$(grep "FINISH_TYPE=" .fresher/.state | cut -d= -f2)
    echo "    Finish type: $finish_type"
  fi
fi

echo "Max iterations test passed"
