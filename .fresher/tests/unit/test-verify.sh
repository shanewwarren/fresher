#!/bin/bash
# Unit tests for verification functions
# Tests: requirement extraction, plan parsing, coverage analysis

set -e

# Get test directory
TEST_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$TEST_SCRIPT_DIR/../../lib/test-utils.sh"

echo "Testing verification functions..."

# Check if verify.sh exists
if [[ ! -f "$TEST_SCRIPT_DIR/../../lib/verify.sh" ]]; then
  echo "  Skipping: verify.sh not found"
  exit 0
fi

source "$TEST_SCRIPT_DIR/../../lib/verify.sh"

# Override the project directory for testing
VERIFY_PROJECT_DIR="$PWD"

# Setup test specs directory
mkdir -p specs
cat > specs/feature.md << 'EOF'
# Feature Specification

### Section 1: Core

- [ ] Implement core functionality
- [x] Design API

The system MUST validate input.
The system SHOULD log errors.

### Section 2: Integration

- [ ] Add API endpoint
EOF

# Setup test plan
cat > IMPLEMENTATION_PLAN.md << 'EOF'
# Implementation Plan

## Priority 1

- [x] Design API (refs: specs/feature.md §Section 1)
- [ ] Implement core functionality (refs: specs/feature.md §Section 1)

## Priority 2

- [ ] Add API endpoint (refs: specs/feature.md §Section 2)
EOF

# Test 1: List specs
echo "  Test 1: List specs"
specs=$(list_specs)
assert_contains "$specs" "feature" "Should list feature spec"

# Test 2: Extract requirements
echo "  Test 2: Extract requirements"
reqs=$(extract_requirements specs)
assert_contains "$reqs" "Implement core functionality" "Should extract pending task"
assert_contains "$reqs" "Design API" "Should extract completed task"

# Test 3: Count requirements
echo "  Test 3: Count requirements"
count_output=$(count_requirements specs)
count=$(echo "$count_output" | grep '^total:' | cut -d: -f2)
if [[ $count -ge 3 ]]; then
  echo "    Found $count requirements"
else
  echo "FAIL: Expected at least 3 requirements, found $count"
  exit 1
fi

# Test 4: Parse plan
echo "  Test 4: Parse plan"
tasks=$(parse_plan)
assert_contains "$tasks" "Design API" "Should parse completed task"
assert_contains "$tasks" "Implement core" "Should parse pending task"
assert_contains "$tasks" "completed" "Should have completion status"
assert_contains "$tasks" "pending" "Should have pending status"

# Test 5: Count plan tasks
echo "  Test 5: Count plan tasks"
task_count_output=$(count_plan_tasks)
task_count=$(echo "$task_count_output" | grep '^total:' | cut -d: -f2)
assert_equals "$task_count" "3" "Should count 3 tasks"

# Test 6: Coverage analysis
echo "  Test 6: Coverage analysis"
coverage=$(analyze_coverage)
assert_contains "$coverage" "feature" "Should include feature spec in coverage"

# Test 7: Find uncovered specs
echo "  Test 7: Uncovered specs"
# Create a spec with no plan references (### headers are extracted as sections)
cat > specs/orphan.md << 'EOF'
# Orphan Spec

### Section 1

- [ ] Task with no plan reference
EOF

uncovered=$(find_uncovered_specs specs)
assert_contains "$uncovered" "orphan" "Should find orphan spec as uncovered"

# Test 8: Orphan tasks (tasks without spec refs)
echo "  Test 8: Orphan tasks"
cat >> IMPLEMENTATION_PLAN.md << 'EOF'

## Priority 3

- [ ] Orphan task without any spec reference
EOF

orphans=$(get_orphan_tasks)
assert_contains "$orphans" "Orphan task" "Should detect orphan task"

# Test 9: Get spec requirements
echo "  Test 9: Get spec requirements"
spec_reqs=$(get_spec_requirements "feature")
assert_contains "$spec_reqs" "Core" "Should get feature spec sections"

# Cleanup
rm -rf specs IMPLEMENTATION_PLAN.md

echo "Verification tests passed"
