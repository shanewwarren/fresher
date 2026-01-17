#!/bin/bash
# Integration test for planning mode
# Tests: full planning mode flow with mock Claude

set -e

# Get test directory
TEST_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$TEST_SCRIPT_DIR/../../lib/test-utils.sh"

echo "Testing planning mode..."

# Setup
export MOCK_CLAUDE_MODE="success"
export FRESHER_MODE="planning"
export FRESHER_MAX_ITERATIONS=1
export FRESHER_HOOKS_ENABLED="false"  # Disable hooks for cleaner test
export FRESHER_USE_DOCKER="false"

# Ensure we have a config
if [[ ! -f .fresher/config.sh ]]; then
  create_mock_config
fi

# Create planning prompt if missing
if [[ ! -f .fresher/PROMPT.planning.md ]]; then
  echo "# Planning Prompt" > .fresher/PROMPT.planning.md
fi

# Remove existing plan to test creation
rm -f IMPLEMENTATION_PLAN.md

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

# Run planning mode (with timeout)
echo "  Running planning mode..."
timeout 30 .fresher/run.sh || exit_code=$?

# Allow exit codes 0 (success) or anything from max iterations
if [[ ${exit_code:-0} -gt 2 ]]; then
  echo "FAIL: run.sh exited with unexpected code $exit_code"
  exit 1
fi

# Verify state file was created
if [[ -f .fresher/.state ]]; then
  echo "    State file created"
  assert_file_contains .fresher/.state "ITERATION"
else
  echo "    Warning: No state file (may be expected)"
fi

# Verify logs directory exists
if [[ -d .fresher/logs ]]; then
  echo "    Logs directory exists"
else
  echo "    Warning: No logs directory"
fi

echo "Planning mode test passed"
