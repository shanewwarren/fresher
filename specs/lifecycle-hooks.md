# Lifecycle Hooks Specification

**Status:** Implemented
**Version:** 1.0
**Last Updated:** 2026-01-18
**Implementation:** `src/hooks.rs` (timeout, exit codes, env vars all working)

---

## 1. Overview

### Purpose

Lifecycle hooks allow users to run custom scripts at key points in the Ralph Loop execution. This enables notifications, logging, integrations, and custom validation without modifying the core loop.

### Goals

- **Extensibility** - Users can add behavior without editing core scripts
- **Context awareness** - Hooks receive rich information about loop state
- **Fail-safe** - Hook failures are handled gracefully with configurable behavior
- **Simplicity** - Standard bash scripts, no special framework needed

### Non-Goals

- **Complex orchestration** - Hooks are simple scripts, not workflow engines
- **Remote hooks** - All hooks run locally (webhooks can be triggered from hooks)
- **Async execution** - Hooks run synchronously in sequence

---

## 2. Architecture

### Hook Directory Structure

```
.fresher/
└── hooks/
    ├── started           # Runs once when loop begins
    ├── next_iteration    # Runs before each iteration
    ├── finished          # Runs when loop ends
    └── custom/           # User-defined hooks (optional)
        └── notify-slack
```

### Execution Points

```
┌─────────────────────────────────────────────────────────────────┐
│  Loop Lifecycle                                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─ hooks/started ─────────────────────────────────────────┐   │
│  │  • Loop is starting                                      │   │
│  │  • Environment variables set                             │   │
│  │  • Can abort loop (exit non-zero)                        │   │
│  └──────────────────────────────────────────────────────────┘   │
│                           │                                     │
│                           ▼                                     │
│  ┌─ LOOP ───────────────────────────────────────────────────┐   │
│  │                                                          │   │
│  │  ┌─ hooks/next_iteration ───────────────────────────┐   │   │
│  │  │  • Iteration about to begin                       │   │   │
│  │  │  • Previous iteration results available           │   │   │
│  │  │  • Can skip iteration (exit 1)                    │   │   │
│  │  └───────────────────────────────────────────────────┘   │   │
│  │                        │                                 │   │
│  │                        ▼                                 │   │
│  │           [ Claude Code Execution ]                      │   │
│  │                        │                                 │   │
│  │                        ▼                                 │   │
│  │           [ Termination Check ]                          │   │
│  │                                                          │   │
│  └──────────────────────────────────────────────────────────┘   │
│                           │                                     │
│                           ▼                                     │
│  ┌─ hooks/finished ────────────────────────────────────────┐   │
│  │  • Loop is ending                                        │   │
│  │  • Finish type provided (manual, complete, error, etc.) │   │
│  │  • Final statistics available                            │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. Core Types

### 3.1 Environment Variables

All hooks receive these environment variables:

| Variable | Description | Available In |
|----------|-------------|--------------|
| `FRESHER_MODE` | Current mode (planning/building) | All |
| `FRESHER_ITERATION` | Current iteration number | next_iteration, finished |
| `FRESHER_MAX_ITERATIONS` | Configured max (0=unlimited) | All |
| `FRESHER_PROJECT_DIR` | Absolute path to project root | All |
| `FRESHER_LOG_DIR` | Path to logs directory | All |
| `FRESHER_STARTED_AT` | ISO timestamp when loop started | All |

### 3.2 Hook-Specific Variables

**started hook:**

| Variable | Description |
|----------|-------------|
| `FRESHER_PLAN_FILE` | Path to IMPLEMENTATION_PLAN.md |
| `FRESHER_SPEC_DIR` | Path to specs directory |

**next_iteration hook:**

| Variable | Description |
|----------|-------------|
| `FRESHER_LAST_EXIT_CODE` | Exit code from previous iteration |
| `FRESHER_LAST_DURATION` | Duration of previous iteration (seconds) |
| `FRESHER_COMMITS_MADE` | Number of commits made so far |

**finished hook:**

| Variable | Description |
|----------|-------------|
| `FRESHER_FINISH_TYPE` | How loop ended (see below) |
| `FRESHER_TOTAL_ITERATIONS` | Total iterations completed |
| `FRESHER_TOTAL_COMMITS` | Total commits made |
| `FRESHER_DURATION` | Total loop duration (seconds) |

### 3.3 Finish Types

Values for `FRESHER_FINISH_TYPE`:

| Type | Description |
|------|-------------|
| `manual` | User pressed Ctrl+C |
| `max_iterations` | Reached FRESHER_MAX_ITERATIONS |
| `complete` | All tasks marked done in plan |
| `no_changes` | No commits in last iteration |
| `error` | Claude Code exited with error |
| `hook_abort` | A hook requested termination |

### 3.4 Exit Code Conventions

| Exit Code | Meaning | Effect |
|-----------|---------|--------|
| 0 | Success | Continue normally |
| 1 | Warning/Skip | Log warning, continue (skip iteration for next_iteration) |
| 2 | Abort | Stop loop, run finished hook |
| 3+ | Error | Log error, continue |

---

## 4. Behaviors

### 4.1 Hook Execution

```bash
run_hook() {
  local hook_name="$1"
  local hook_path=".fresher/hooks/$hook_name"

  # Check if hook exists and is executable
  if [[ ! -x "$hook_path" ]]; then
    if [[ -f "$hook_path" ]]; then
      log_warn "Hook $hook_name exists but is not executable"
    fi
    return 0
  fi

  log_info "Running hook: $hook_name"

  # Execute hook with timeout
  local timeout="${FRESHER_HOOK_TIMEOUT:-30}"
  local exit_code

  if timeout "$timeout" "$hook_path"; then
    exit_code=0
  else
    exit_code=$?
  fi

  # Handle exit code
  case $exit_code in
    0)
      log_info "Hook $hook_name completed successfully"
      ;;
    1)
      log_warn "Hook $hook_name returned warning (exit 1)"
      ;;
    2)
      log_error "Hook $hook_name requested abort (exit 2)"
      FRESHER_FINISH_TYPE="hook_abort"
      return 2
      ;;
    124)
      log_error "Hook $hook_name timed out after ${timeout}s"
      ;;
    *)
      log_error "Hook $hook_name failed with exit code $exit_code"
      ;;
  esac

  return 0
}
```

### 4.2 Hook Templates

**started hook template:**

```bash
#!/bin/bash
# .fresher/hooks/started
# Runs once when the Ralph loop begins

echo "Starting Fresher loop in $FRESHER_MODE mode"
echo "Project: $FRESHER_PROJECT_DIR"
echo "Max iterations: ${FRESHER_MAX_ITERATIONS:-unlimited}"

# Example: Check prerequisites
if [[ ! -f "IMPLEMENTATION_PLAN.md" ]] && [[ "$FRESHER_MODE" == "building" ]]; then
  echo "ERROR: No IMPLEMENTATION_PLAN.md found. Run planning mode first."
  exit 2  # Abort
fi

# Example: Send notification
# curl -X POST "$SLACK_WEBHOOK" -d "{\"text\": \"Fresher started: $FRESHER_MODE mode\"}"

exit 0
```

**next_iteration hook template:**

```bash
#!/bin/bash
# .fresher/hooks/next_iteration
# Runs before each iteration

echo "Starting iteration $FRESHER_ITERATION"

if [[ $FRESHER_ITERATION -gt 1 ]]; then
  echo "Previous iteration: exit=$FRESHER_LAST_EXIT_CODE, duration=${FRESHER_LAST_DURATION}s"
  echo "Commits so far: $FRESHER_COMMITS_MADE"
fi

# Example: Skip if no changes needed
if [[ "$FRESHER_MODE" == "building" ]]; then
  pending=$(grep -c '^\s*-\s*\[\s\]' IMPLEMENTATION_PLAN.md 2>/dev/null || echo "0")
  if [[ "$pending" -eq 0 ]]; then
    echo "No pending tasks, skipping iteration"
    exit 1  # Skip
  fi
fi

exit 0
```

**finished hook template:**

```bash
#!/bin/bash
# .fresher/hooks/finished
# Runs when the loop ends

echo "Fresher loop finished"
echo "Finish type: $FRESHER_FINISH_TYPE"
echo "Total iterations: $FRESHER_TOTAL_ITERATIONS"
echo "Total commits: $FRESHER_TOTAL_COMMITS"
echo "Duration: ${FRESHER_DURATION}s"

# Example: Summary notification
case "$FRESHER_FINISH_TYPE" in
  complete)
    message="All tasks completed successfully!"
    ;;
  manual)
    message="Loop stopped by user"
    ;;
  max_iterations)
    message="Reached max iterations limit"
    ;;
  error)
    message="Loop stopped due to error"
    ;;
  *)
    message="Loop finished: $FRESHER_FINISH_TYPE"
    ;;
esac

echo "$message"

# Example: Send notification
# curl -X POST "$SLACK_WEBHOOK" -d "{\"text\": \"$message\"}"

exit 0
```

### 4.3 Custom Hooks

Users can create additional hooks in `.fresher/hooks/custom/`:

```bash
# .fresher/hooks/custom/notify-slack
#!/bin/bash
# Called from other hooks: .fresher/hooks/custom/notify-slack "message"

message="${1:-Fresher notification}"
webhook="${FRESHER_SLACK_WEBHOOK:-}"

if [[ -z "$webhook" ]]; then
  exit 0
fi

curl -s -X POST "$webhook" \
  -H "Content-Type: application/json" \
  -d "{\"text\": \"$message\"}"
```

Usage from main hooks:

```bash
# In .fresher/hooks/finished
.fresher/hooks/custom/notify-slack "Build completed: $FRESHER_TOTAL_COMMITS commits"
```

---

## 5. Configuration

### Hook Configuration in config.sh

```bash
# Hook settings
export FRESHER_HOOK_TIMEOUT="${FRESHER_HOOK_TIMEOUT:-30}"
export FRESHER_HOOKS_ENABLED="${FRESHER_HOOKS_ENABLED:-true}"
export FRESHER_HOOK_FAIL_BEHAVIOR="${FRESHER_HOOK_FAIL_BEHAVIOR:-continue}"

# Per-hook enable/disable
export FRESHER_HOOK_STARTED_ENABLED="${FRESHER_HOOK_STARTED_ENABLED:-true}"
export FRESHER_HOOK_NEXT_ITERATION_ENABLED="${FRESHER_HOOK_NEXT_ITERATION_ENABLED:-true}"
export FRESHER_HOOK_FINISHED_ENABLED="${FRESHER_HOOK_FINISHED_ENABLED:-true}"
```

### Disabling Hooks

```bash
# Disable all hooks
FRESHER_HOOKS_ENABLED=false fresher build

# Disable specific hook
FRESHER_HOOK_NEXT_ITERATION_ENABLED=false fresher build
```

---

## 6. Security Considerations

### Executable Permissions

- Hooks must be executable (`chmod +x`)
- Non-executable hook files are logged as warnings but don't fail

### Environment Isolation

- Hooks inherit the loop's environment
- Sensitive variables (API keys) should be in config.sh or shell environment
- Hook output is logged; avoid printing secrets

### Timeout Protection

- Default 30-second timeout prevents hung hooks
- Configurable via `FRESHER_HOOK_TIMEOUT`

---

## 7. Implementation Phases

| Phase | Description | Dependencies | Complexity |
|-------|-------------|--------------|------------|
| 1 | Basic hook execution | loop-executor | Low |
| 2 | Environment variable passing | Phase 1 | Low |
| 3 | Exit code handling | Phase 1 | Low |
| 4 | Hook templates in scaffold | project-scaffold | Low |
| 5 | Timeout and error handling | Phase 1 | Medium |

---

## 8. Open Questions

- [ ] Should hooks support async execution (background)?
- [ ] Should there be a post_iteration hook (after Claude but before termination check)?
- [ ] How to handle hook output in the terminal (separate from Claude output)?
