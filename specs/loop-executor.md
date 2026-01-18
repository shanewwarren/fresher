# Loop Executor Specification

**Status:** Needs Update (Implemented in Rust, spec describes Bash)
**Version:** 1.0
**Last Updated:** 2026-01-18
**Implementation:** `src/commands/plan.rs`, `src/commands/build.rs`, `src/streaming.rs`

---

## 1. Overview

### Purpose

The loop executor is the core bash script that runs Claude Code in iterative cycles. Each iteration loads fresh context, executes Claude Code with dangerous permissions, streams output in real-time, and determines whether to continue or terminate.

### Goals

- **Fresh context per iteration** - Clear context between iterations to maintain quality
- **Real-time output** - Stream Claude Code output to terminal as it executes
- **Configurable termination** - Support manual (Ctrl+C), max iterations, and smart detection
- **Robust execution** - Handle signals, errors, and edge cases gracefully

### Non-Goals

- **IDE integration** - Direct IDE plugins (out of scope, use CLI)
- **Multi-model support** - Only Claude Code supported initially
- **Remote execution** - Local execution only (Docker isolation is separate spec)

---

## 2. Architecture

### Component Structure

```
.fresher/
├── run.sh              # Main loop executor script
├── config.sh           # Configuration variables
├── lib/
│   ├── termination.sh  # Termination detection logic
│   ├── streaming.sh    # Output streaming utilities
│   └── state.sh        # State management
└── logs/
    └── iteration-{n}.log
```

### Execution Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                         run.sh                                   │
├─────────────────────────────────────────────────────────────────┤
│  1. Load config.sh                                              │
│  2. Initialize state (iteration=0)                              │
│  3. Run hooks/started                                           │
│                                                                 │
│  ┌─────────────── LOOP ───────────────┐                        │
│  │  4. Increment iteration            │                        │
│  │  5. Run hooks/next_iteration       │                        │
│  │  6. Invoke Claude Code             │──▶ Stream to terminal  │
│  │  7. Capture exit code              │                        │
│  │  8. Check termination conditions   │                        │
│  │  9. If continue → loop             │                        │
│  └────────────────────────────────────┘                        │
│                                                                 │
│  10. Run hooks/finished (with finish_type)                     │
│  11. Exit                                                       │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. Core Types

### 3.1 Configuration Variables

Environment variables loaded from `config.sh`:

| Variable | Type | Required | Default | Description |
|----------|------|----------|---------|-------------|
| `FRESHER_MODE` | string | Yes | - | "planning" or "building" |
| `FRESHER_MAX_ITERATIONS` | number | No | 0 (unlimited) | Maximum iterations before auto-stop |
| `FRESHER_SMART_TERMINATION` | boolean | No | true | Enable smart completion detection |
| `FRESHER_LOG_DIR` | string | No | `.fresher/logs` | Directory for iteration logs |
| `FRESHER_DANGEROUS_PERMISSIONS` | boolean | No | true | Use --dangerously-skip-permissions |
| `FRESHER_MAX_TURNS` | number | No | 50 | Claude Code --max-turns per iteration |
| `FRESHER_MODEL` | string | No | sonnet | Claude model to use |

### 3.2 State File

State tracked in `.fresher/.state`:

```bash
ITERATION=5
LAST_EXIT_CODE=0
LAST_COMMIT_SHA=abc123
STARTED_AT=2025-01-17T10:00:00Z
TOTAL_COMMITS=3
```

### 3.3 Finish Types

Passed to `hooks/finished` as `$FRESHER_FINISH_TYPE`:

| Type | Description |
|------|-------------|
| `manual` | User pressed Ctrl+C |
| `max_iterations` | Reached FRESHER_MAX_ITERATIONS |
| `complete` | Smart detection found all tasks done |
| `no_changes` | No commits made in last iteration |
| `error` | Claude Code exited with error |

---

## 4. Behaviors

### 4.1 Claude Code Invocation

**Per-iteration command:**

```bash
claude -p "$(cat .fresher/PROMPT.${FRESHER_MODE}.md)" \
  --append-system-prompt-file .fresher/AGENTS.md \
  --dangerously-skip-permissions \
  --output-format stream-json \
  --max-turns "$FRESHER_MAX_TURNS" \
  --no-session-persistence \
  --model "$FRESHER_MODEL"
```

**Output streaming:**

Stream JSON events are parsed in real-time:
- Display assistant messages to terminal
- Capture tool calls for logging
- Extract final result for termination analysis

### 4.2 Termination Detection

**Priority order:**

1. **Manual (SIGINT)** - Trap Ctrl+C, run cleanup, exit
2. **Max iterations** - Check `$ITERATION -ge $FRESHER_MAX_ITERATIONS`
3. **Smart detection** (if enabled):
   - **Primary**: Parse `IMPLEMENTATION_PLAN.md` for uncompleted tasks
   - **Fallback**: Check if no commits were made this iteration

**Plan-based detection logic:**

```bash
# Count uncompleted tasks in IMPLEMENTATION_PLAN.md
pending_tasks=$(grep -cE '^\s*-\s*\[\s\]' IMPLEMENTATION_PLAN.md 2>/dev/null || echo "0")

if [[ "$pending_tasks" -eq 0 ]]; then
  FINISH_TYPE="complete"
  return 0  # Should terminate
fi
```

**No-change detection logic:**

```bash
# Check if HEAD moved since iteration started
current_sha=$(git rev-parse HEAD 2>/dev/null)
if [[ "$current_sha" == "$LAST_COMMIT_SHA" ]]; then
  FINISH_TYPE="no_changes"
  return 0  # Should terminate
fi
```

### 4.3 Signal Handling

```bash
cleanup() {
  local exit_code=$?

  # Run finished hook
  FRESHER_FINISH_TYPE="${FRESHER_FINISH_TYPE:-manual}"
  run_hook "finished"

  # Write final state
  write_state

  exit $exit_code
}

trap cleanup EXIT
trap 'FRESHER_FINISH_TYPE="manual"; exit 130' INT TERM
```

### 4.4 Output Streaming

Real-time display using process substitution:

```bash
claude ... --output-format stream-json | while IFS= read -r line; do
  # Parse JSON event
  event_type=$(echo "$line" | jq -r '.type // empty')

  case "$event_type" in
    "assistant")
      # Display assistant text
      echo "$line" | jq -r '.content // empty'
      ;;
    "tool_use")
      # Log tool call
      echo "$line" >> "$LOG_FILE"
      ;;
    "result")
      # Capture final result for analysis
      LAST_RESULT="$line"
      ;;
  esac

  # Also write to log file
  echo "$line" >> "$LOG_FILE"
done
```

---

## 5. Configuration

### 5.1 config.sh Template

```bash
#!/bin/bash
# Fresher configuration

# Mode: "planning" or "building"
export FRESHER_MODE="${FRESHER_MODE:-planning}"

# Termination settings
export FRESHER_MAX_ITERATIONS="${FRESHER_MAX_ITERATIONS:-0}"  # 0 = unlimited
export FRESHER_SMART_TERMINATION="${FRESHER_SMART_TERMINATION:-true}"

# Claude Code settings
export FRESHER_DANGEROUS_PERMISSIONS="${FRESHER_DANGEROUS_PERMISSIONS:-true}"
export FRESHER_MAX_TURNS="${FRESHER_MAX_TURNS:-50}"
export FRESHER_MODEL="${FRESHER_MODEL:-sonnet}"

# Logging
export FRESHER_LOG_DIR="${FRESHER_LOG_DIR:-.fresher/logs}"

# Docker (see docker-isolation spec)
export FRESHER_USE_DOCKER="${FRESHER_USE_DOCKER:-false}"
```

---

## 6. Security Considerations

### Dangerous Permissions

- The `--dangerously-skip-permissions` flag allows Claude to execute any action without confirmation
- Only use in trusted environments or with Docker isolation
- Log all tool calls for audit trail

### State File Protection

- `.fresher/.state` contains iteration metadata
- Should be gitignored to prevent conflicts
- Not sensitive but could affect loop behavior if tampered

---

## 7. Implementation Phases

| Phase | Description | Dependencies | Complexity |
|-------|-------------|--------------|------------|
| 1 | Basic loop with manual termination | None | Low |
| 2 | Output streaming and logging | Phase 1 | Medium |
| 3 | Max iterations termination | Phase 1 | Low |
| 4 | Smart termination detection | Phase 2 | Medium |
| 5 | Signal handling and cleanup | Phase 1 | Medium |

---

## 8. Open Questions

- [ ] Should the loop support resuming from a specific iteration?
- [ ] How to handle Claude Code crashes vs intentional exits?
- [ ] Should iteration logs be auto-rotated or manually cleaned?
