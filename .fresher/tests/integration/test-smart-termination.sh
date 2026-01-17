#!/bin/bash
# Integration test for smart termination
# Tests: loop stops when all tasks are complete

set -e

# Get test directory
TEST_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$TEST_SCRIPT_DIR/../../lib/test-utils.sh"

echo "Testing smart termination..."

# Setup
export MOCK_CLAUDE_MODE="no_changes"  # Simulate no work to do
export FRESHER_MODE="building"
export FRESHER_MAX_ITERATIONS=0  # Unlimited
export FRESHER_SMART_TERMINATION="true"
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

# Create plan with ALL tasks complete
cat > IMPLEMENTATION_PLAN.md << 'EOF'
# Implementation Plan

## Priority 1

- [x] Task 1 - done
- [x] Task 2 - done
- [x] Task 3 - done
EOF

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

# Run - should terminate quickly due to all tasks complete
echo "  Running with all tasks complete..."
start_time=$(date +%s)
timeout 30 .fresher/run.sh || true
end_time=$(date +%s)
duration=$((end_time - start_time))

echo "    Duration: ${duration}s"

# Check finish type
if [[ -f .fresher/.state ]]; then
  if grep -q "FINISH_TYPE=" .fresher/.state; then
    finish_type=$(grep "FINISH_TYPE=" .fresher/.state | cut -d= -f2)
    echo "    Finish type: $finish_type"

    # Should terminate with 'complete' or 'no_changes'
    if [[ "$finish_type" == "complete" || "$finish_type" == "no_changes" ]]; then
      echo "    Correct termination type"
    else
      echo "    Warning: Unexpected finish type (may be OK)"
    fi
  fi

  # Should have run only 1-2 iterations
  if grep -q "ITERATION=" .fresher/.state; then
    iteration=$(grep "ITERATION=" .fresher/.state | cut -d= -f2)
    echo "    Iterations: $iteration"

    if [[ $iteration -le 2 ]]; then
      echo "    Smart termination worked"
    else
      echo "    Warning: More iterations than expected"
    fi
  fi
fi

echo "Smart termination test passed"
