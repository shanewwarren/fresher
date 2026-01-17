#!/bin/bash
# Unit tests for config loading
# Tests: config.sh loading, default values, environment overrides

set -e

# Get test directory
TEST_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$TEST_SCRIPT_DIR/../../lib/test-utils.sh"

echo "Testing config loading..."

# Test 1: Default values are set correctly
echo "  Test 1: Default values"
unset FRESHER_MAX_ITERATIONS FRESHER_SMART_TERMINATION FRESHER_MODE
source "$TEST_SCRIPT_DIR/../../config.sh"

assert_equals "$FRESHER_MAX_ITERATIONS" "0" "Default max iterations should be 0"
assert_equals "$FRESHER_SMART_TERMINATION" "true" "Smart termination should be enabled by default"
assert_equals "$FRESHER_MODE" "planning" "Default mode should be planning"
assert_equals "$FRESHER_HOOKS_ENABLED" "true" "Hooks should be enabled by default"

# Test 2: Environment overrides work
echo "  Test 2: Environment overrides"
export FRESHER_MAX_ITERATIONS=5
export FRESHER_MODE="building"
export FRESHER_SMART_TERMINATION="false"
source "$TEST_SCRIPT_DIR/../../config.sh"

assert_equals "$FRESHER_MAX_ITERATIONS" "5" "Max iterations override should work"
assert_equals "$FRESHER_MODE" "building" "Mode override should work"
assert_equals "$FRESHER_SMART_TERMINATION" "false" "Smart termination override should work"

# Test 3: Command defaults exist
echo "  Test 3: Command defaults"
unset FRESHER_TEST_CMD FRESHER_BUILD_CMD FRESHER_LINT_CMD
source "$TEST_SCRIPT_DIR/../../config.sh"

# These should have defaults (may vary by project type)
assert_true "$FRESHER_TEST_CMD" "Test command should have a default"
assert_true "$FRESHER_BUILD_CMD" "Build command should have a default"
assert_true "$FRESHER_LINT_CMD" "Lint command should have a default"

# Test 4: Path defaults exist
echo "  Test 4: Path defaults"
unset FRESHER_LOG_DIR FRESHER_SPEC_DIR FRESHER_SRC_DIR
source "$TEST_SCRIPT_DIR/../../config.sh"

assert_equals "$FRESHER_LOG_DIR" ".fresher/logs" "Log dir should default to .fresher/logs"
assert_equals "$FRESHER_SPEC_DIR" "specs" "Spec dir should default to specs"
assert_equals "$FRESHER_SRC_DIR" "src" "Src dir should default to src"

# Test 5: Docker defaults
echo "  Test 5: Docker defaults"
unset FRESHER_USE_DOCKER FRESHER_DOCKER_MEMORY FRESHER_DOCKER_CPUS
source "$TEST_SCRIPT_DIR/../../config.sh"

assert_equals "$FRESHER_USE_DOCKER" "false" "Docker should be disabled by default"
assert_equals "$FRESHER_DOCKER_MEMORY" "4g" "Docker memory should default to 4g"
assert_equals "$FRESHER_DOCKER_CPUS" "2" "Docker cpus should default to 2"

# Test 6: Hook timeout default
echo "  Test 6: Hook timeout default"
unset FRESHER_HOOK_TIMEOUT
source "$TEST_SCRIPT_DIR/../../config.sh"

assert_equals "$FRESHER_HOOK_TIMEOUT" "30" "Hook timeout should default to 30"

echo "Config tests passed"
