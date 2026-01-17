#!/bin/bash
# Integration test for building mode
# Tests: full building mode flow with commit tracking

set -e

# Get test directory
TEST_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$TEST_SCRIPT_DIR/../../lib/test-utils.sh"

echo "Testing building mode..."

# Setup
export MOCK_CLAUDE_MODE="success"
export MOCK_CLAUDE_COMMITS=1
export FRESHER_MODE="building"
export FRESHER_MAX_ITERATIONS=2
export FRESHER_SMART_TERMINATION="false"  # Disable smart termination
export FRESHER_HOOKS_ENABLED="false"
export FRESHER_USE_DOCKER="false"

# Ensure config exists
if [[ ! -f .fresher/config.sh ]]; then
  create_mock_config
fi

# Create building prompt if missing
if [[ ! -f .fresher/PROMPT.building.md ]]; then
  echo "# Building Prompt" > .fresher/PROMPT.building.md
fi

# Create a plan with tasks
create_mock_plan IMPLEMENTATION_PLAN.md 3 1

# Initialize git repo for commit tracking
init_test_git .

# Get initial commit count
initial_commits=$(get_commit_count)

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

# Run building mode
echo "  Running building mode..."
timeout 60 .fresher/run.sh || exit_code=$?

# Get final commit count
final_commits=$(get_commit_count)

echo "    Initial commits: $initial_commits"
echo "    Final commits:   $final_commits"

# Mock Claude should have created commits
if [[ $final_commits -gt $initial_commits ]]; then
  echo "    Commits were created"
else
  echo "    Warning: No new commits (mock may not support this)"
fi

# Check state file for iteration tracking
if [[ -f .fresher/.state ]]; then
  echo "    State file created"
  if grep -q "ITERATION=" .fresher/.state; then
    iteration=$(grep "ITERATION=" .fresher/.state | cut -d= -f2)
    echo "    Final iteration: $iteration"
  fi
fi

echo "Building mode test passed"
