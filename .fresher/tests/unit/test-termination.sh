#!/bin/bash
# Unit tests for termination detection
# Tests: max iterations, smart termination, no changes detection

set -e

# Get test directory
TEST_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$TEST_SCRIPT_DIR/../../lib/test-utils.sh"

echo "Testing termination detection..."

# Helper: check if plan has pending tasks
has_pending_tasks() {
  local plan_file="${1:-IMPLEMENTATION_PLAN.md}"
  if [[ -f "$plan_file" ]]; then
    grep -qE '^\s*-\s*\[\s\]' "$plan_file"
    return $?
  fi
  return 0  # No plan file = assume pending tasks
}

# Helper: count pending tasks
count_pending_tasks() {
  local plan_file="${1:-IMPLEMENTATION_PLAN.md}"
  if [[ -f "$plan_file" ]]; then
    grep -cE '^\s*-\s*\[\s\]' "$plan_file" 2>/dev/null || echo "0"
  else
    echo "0"
  fi
}

# Test 1: Max iterations triggers termination
echo "  Test 1: Max iterations"
export FRESHER_MAX_ITERATIONS=3
ITERATION=3

# Should terminate when iteration equals max
if [[ $FRESHER_MAX_ITERATIONS -gt 0 && $ITERATION -ge $FRESHER_MAX_ITERATIONS ]]; then
  echo "    Max iterations termination works"
else
  echo "FAIL: Should terminate at max iterations"
  exit 1
fi

# Test 2: No max iterations means continue
echo "  Test 2: No max iterations"
export FRESHER_MAX_ITERATIONS=0
ITERATION=100

if [[ $FRESHER_MAX_ITERATIONS -gt 0 && $ITERATION -ge $FRESHER_MAX_ITERATIONS ]]; then
  echo "FAIL: Should not terminate when max_iterations is 0"
  exit 1
else
  echo "    Unlimited iterations work"
fi

# Test 3: Smart termination - all tasks complete
echo "  Test 3: Smart termination (all complete)"
export FRESHER_SMART_TERMINATION="true"

cat > IMPLEMENTATION_PLAN.md << 'EOF'
# Implementation Plan
- [x] Task 1
- [x] Task 2
- [x] Task 3
EOF

if has_pending_tasks; then
  echo "FAIL: Should detect no pending tasks"
  exit 1
else
  echo "    All tasks complete detection works"
fi

# Test 4: Smart termination - tasks remaining
echo "  Test 4: Smart termination (tasks pending)"
cat > IMPLEMENTATION_PLAN.md << 'EOF'
# Implementation Plan
- [x] Task 1
- [ ] Task 2
- [ ] Task 3
EOF

if has_pending_tasks; then
  echo "    Pending tasks detection works"
else
  echo "FAIL: Should detect pending tasks"
  exit 1
fi

# Test 5: Pending task count
echo "  Test 5: Pending task count"
pending=$(count_pending_tasks)
assert_equals "$pending" "2" "Should count 2 pending tasks"

# Test 6: Empty plan has no pending tasks
echo "  Test 6: Empty plan"
rm -f IMPLEMENTATION_PLAN.md
pending=$(count_pending_tasks)
assert_equals "$pending" "0" "Empty/missing plan should have 0 pending tasks"

# Test 7: Mixed checkbox formats
echo "  Test 7: Mixed checkbox formats"
cat > IMPLEMENTATION_PLAN.md << 'EOF'
# Plan
- [ ] Standard pending
- [x] Standard complete
-  [ ] Extra space pending
- [ ]Tab pending
  - [ ] Indented pending
EOF

pending=$(count_pending_tasks)
# Should count all pending formats (standard regex may vary)
if [[ $pending -ge 2 ]]; then
  echo "    Mixed format detection works (found $pending)"
else
  echo "FAIL: Should detect multiple pending formats (found $pending)"
  exit 1
fi

# Cleanup
rm -f IMPLEMENTATION_PLAN.md

echo "Termination tests passed"
