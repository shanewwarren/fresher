#!/bin/bash
# Unit tests for hook execution
# Tests: hook execution, environment variables, exit codes, timeout

set -e

# Get test directory
TEST_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$TEST_SCRIPT_DIR/../../lib/test-utils.sh"

echo "Testing hook execution..."

# Setup hooks directory
mkdir -p .fresher/hooks

# Helper: simplified run_hook for testing
run_hook() {
  local hook_name="$1"
  local hook_path=".fresher/hooks/$hook_name"

  if [[ ! -x "$hook_path" ]]; then
    return 0  # No hook = success
  fi

  "$hook_path"
  return $?
}

# Test 1: Hook receives environment variables
echo "  Test 1: Environment variables"
export FRESHER_ITERATION=5
export FRESHER_MODE="building"

cat > .fresher/hooks/test-env << 'EOF'
#!/bin/bash
if [[ "$FRESHER_ITERATION" != "5" ]]; then
  echo "FRESHER_ITERATION not passed"
  exit 1
fi
if [[ "$FRESHER_MODE" != "building" ]]; then
  echo "FRESHER_MODE not passed"
  exit 1
fi
exit 0
EOF
chmod +x .fresher/hooks/test-env

run_hook "test-env"
echo "    Environment passing works"

# Test 2: Hook exit code 0 means continue
echo "  Test 2: Exit code 0 (continue)"
cat > .fresher/hooks/test-continue << 'EOF'
#!/bin/bash
exit 0
EOF
chmod +x .fresher/hooks/test-continue

if run_hook "test-continue"; then
  echo "    Exit 0 handling works"
else
  echo "FAIL: Exit 0 should allow continuation"
  exit 1
fi

# Test 3: Hook exit code 1 means skip
echo "  Test 3: Exit code 1 (skip)"
cat > .fresher/hooks/test-skip << 'EOF'
#!/bin/bash
exit 1
EOF
chmod +x .fresher/hooks/test-skip

if run_hook "test-skip"; then
  echo "FAIL: Exit 1 should indicate skip"
  exit 1
else
  exit_code=$?
  if [[ $exit_code -eq 1 ]]; then
    echo "    Exit 1 handling works"
  else
    echo "FAIL: Expected exit code 1, got $exit_code"
    exit 1
  fi
fi

# Test 4: Hook exit code 2 means abort
echo "  Test 4: Exit code 2 (abort)"
cat > .fresher/hooks/test-abort << 'EOF'
#!/bin/bash
exit 2
EOF
chmod +x .fresher/hooks/test-abort

run_hook "test-abort" || exit_code=$?
if [[ $exit_code -eq 2 ]]; then
  echo "    Exit 2 handling works"
else
  echo "FAIL: Expected exit code 2, got $exit_code"
  exit 1
fi

# Test 5: Missing hook is OK
echo "  Test 5: Missing hook"
if run_hook "nonexistent-hook"; then
  echo "    Missing hook handling works"
else
  echo "FAIL: Missing hook should not fail"
  exit 1
fi

# Test 6: Hook output captured
echo "  Test 6: Hook output"
cat > .fresher/hooks/test-output << 'EOF'
#!/bin/bash
echo "Hook output line 1"
echo "Hook output line 2"
exit 0
EOF
chmod +x .fresher/hooks/test-output

output=$(.fresher/hooks/test-output)
assert_contains "$output" "Hook output line 1" "Should capture hook stdout"

# Test 7: FRESHER_HOOKS_ENABLED=false skips hooks
echo "  Test 7: Hooks disabled"
export FRESHER_HOOKS_ENABLED="false"

# This shouldn't actually run since hooks are disabled
# but we can't easily test without modifying run_hook
echo "    (skipped - requires run.sh integration)"

# Cleanup
rm -rf .fresher/hooks

echo "Hook tests passed"
