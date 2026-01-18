# Loop Executor Specification

**Status:** Implemented
**Version:** 2.0
**Last Updated:** 2026-01-18
**Implementation:** `src/commands/plan.rs`, `src/commands/build.rs`, `src/streaming.rs`, `src/state.rs`, `src/hooks.rs`

---

## 1. Overview

### Purpose

The loop executor runs Claude Code in iterative cycles, providing fresh context each iteration. Implemented in Rust using async/await with Tokio, it streams output in real-time and determines whether to continue or terminate based on configurable conditions.

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

### Module Structure

```
src/
├── commands/
│   ├── plan.rs           # Planning mode loop executor
│   └── build.rs          # Building mode loop executor
├── config.rs             # Configuration (TOML + env)
├── state.rs              # State management
├── streaming.rs          # JSON stream parsing
└── hooks.rs              # Lifecycle hook execution
```

### Execution Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                    fresher plan / fresher build                 │
├─────────────────────────────────────────────────────────────────┤
│  1. Load config.toml + env overrides                            │
│  2. Check Docker isolation requirements                         │
│  3. Initialize state (iteration=0)                              │
│  4. Run hooks/started (async)                                   │
│  5. Set up Ctrl+C handler (tokio signal)                        │
│                                                                 │
│  ┌─────────────── LOOP ───────────────┐                        │
│  │  6. Check interrupt flag           │                        │
│  │  7. Check max iterations           │                        │
│  │  8. Check pending tasks (build)    │                        │
│  │  9. Increment iteration            │                        │
│  │ 10. Run hooks/next_iteration       │                        │
│  │ 11. Invoke Claude Code (async)     │──▶ Stream to terminal  │
│  │ 12. Process stream-json output     │                        │
│  │ 13. Record iteration result        │                        │
│  │ 14. Check termination conditions   │                        │
│  │ 15. If continue → loop             │                        │
│  └────────────────────────────────────┘                        │
│                                                                 │
│  16. Update final state                                         │
│  17. Run hooks/finished (with finish_type)                      │
│  18. Print summary                                              │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. Core Types

### 3.1 Configuration (config.rs)

Configuration loaded from `.fresher/config.toml` with environment variable overrides:

```rust
pub struct Config {
    pub fresher: FresherConfig,
    pub commands: CommandsConfig,
    pub paths: PathsConfig,
    pub hooks: HooksConfig,
    pub docker: DockerConfig,
}

pub struct FresherConfig {
    pub mode: String,              // "planning" or "building"
    pub max_iterations: u32,       // 0 = unlimited
    pub smart_termination: bool,   // Enable smart completion detection
    pub dangerous_permissions: bool, // Use --dangerously-skip-permissions
    pub max_turns: u32,            // Claude Code --max-turns per iteration
    pub model: String,             // Claude model to use
}
```

**Configuration Sources (precedence order):**

1. Environment variables (highest)
2. `.fresher/config.toml`
3. Built-in defaults (lowest)

**Environment Variables:**

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `FRESHER_MODE` | string | "planning" | "planning" or "building" |
| `FRESHER_MAX_ITERATIONS` | number | 0 | Maximum iterations (0=unlimited) |
| `FRESHER_SMART_TERMINATION` | boolean | true | Enable smart detection |
| `FRESHER_DANGEROUS_PERMISSIONS` | boolean | true | Skip permission prompts |
| `FRESHER_MAX_TURNS` | number | 50 | Max turns per iteration |
| `FRESHER_MODEL` | string | "sonnet" | Claude model |
| `FRESHER_HOOKS_ENABLED` | boolean | true | Enable hook execution |
| `FRESHER_HOOK_TIMEOUT` | number | 30 | Hook timeout in seconds |

### 3.2 State (state.rs)

State tracked in `.fresher/.state` (TOML format):

```rust
pub struct State {
    pub iteration: u32,
    pub last_exit_code: i32,
    pub last_commit_sha: Option<String>,
    pub started_at: DateTime<Utc>,
    pub total_commits: u32,
    pub duration: u64,
    pub finish_type: Option<FinishType>,
    pub iteration_start: Option<DateTime<Utc>>,
    pub iteration_sha: Option<String>,
}

pub enum FinishType {
    Manual,         // User pressed Ctrl+C
    Error,          // Claude Code exited with error
    MaxIterations,  // Reached max_iterations limit
    Complete,       // All tasks in plan completed
    NoChanges,      // No commits made in iteration
}
```

### 3.3 Stream Events (streaming.rs)

Claude Code stream-json events parsed in real-time:

```rust
pub enum StreamEvent {
    System(SystemEvent),              // Session init info
    Assistant(AssistantEvent),        // Text and tool calls
    User(UserEvent),                  // Tool results
    ContentBlockStart(ContentBlockStartEvent),
    ContentBlockDelta(ContentBlockDeltaEvent),
    ContentBlockStop(ContentBlockStopEvent),
    Result(ResultEvent),              // Final summary
    Unknown,
}

pub struct ProcessResult {
    pub exit_code: i32,
    pub duration_ms: Option<u64>,
    pub cost_usd: Option<f64>,
    pub num_turns: Option<u32>,
    pub is_error: bool,
    pub result_text: Option<String>,
}
```

---

## 4. Behaviors

### 4.1 Claude Code Invocation

**Per-iteration command built in Rust:**

```rust
let mut cmd = Command::new("claude");
cmd.arg("-p").arg(prompt);

if agents_path.exists() {
    cmd.arg("--append-system-prompt-file").arg(agents_path);
}

if config.fresher.dangerous_permissions {
    cmd.arg("--dangerously-skip-permissions");
}

cmd.arg("--output-format").arg("stream-json");
cmd.arg("--max-turns").arg(config.fresher.max_turns.to_string());
cmd.arg("--no-session-persistence"); // Critical: fresh context
cmd.arg("--model").arg(&config.fresher.model);
cmd.arg("--verbose");
```

**Output streaming with StreamHandler:**

- Displays assistant text to terminal in real-time
- Shows formatted tool calls (Read, Write, Edit, Bash, etc.)
- Logs to `.fresher/logs/` directory
- Captures final result for termination analysis

### 4.2 Termination Detection

**Priority order:**

1. **Manual (SIGINT)** - Tokio signal handler sets atomic flag
2. **Max iterations** - Check `iteration >= max_iterations` (when non-zero)
3. **Smart detection** (if enabled):
   - **Task completion**: Parse `IMPLEMENTATION_PLAN.md` for `- [ ]` patterns
   - **No changes**: Check if no commits made this iteration

**Task completion detection:**

```rust
// In src/verify.rs
pub fn has_pending_tasks(plan_path: &Path) -> bool {
    let content = fs::read_to_string(plan_path).unwrap_or_default();
    let re = Regex::new(r"^\s*-\s*\[\s\]").unwrap();
    content.lines().any(|line| re.is_match(line))
}
```

**No-change detection:**

```rust
let current_sha = get_current_sha();
if current_sha == state.iteration_sha && commits_this_iteration == 0 {
    state.set_finish(FinishType::NoChanges);
    break;
}
```

### 4.3 Signal Handling

Async signal handling with Tokio:

```rust
let should_stop = Arc::new(AtomicBool::new(false));
let should_stop_clone = should_stop.clone();

tokio::spawn(async move {
    signal::ctrl_c().await.ok();
    should_stop_clone.store(true, Ordering::SeqCst);
    println!("Received interrupt, finishing current iteration...");
});

// In main loop:
if should_stop.load(Ordering::SeqCst) {
    state.set_finish(FinishType::Manual);
    break;
}
```

### 4.4 Hook Execution

Hooks run as external processes with timeout:

```rust
pub async fn run_hook(
    hook_name: &str,
    state: &State,
    config: &Config,
    project_dir: &Path,
) -> Result<HookResult> {
    let hook_path = project_dir.join(".fresher/hooks").join(hook_name);

    // Build environment from state
    let env_vars = state.to_env_vars();

    // Run with configured timeout
    let timeout_duration = Duration::from_secs(config.hooks.timeout as u64);
    let result = timeout(timeout_duration, cmd.status()).await;

    // Map exit codes to HookResult
    match code {
        0 => HookResult::Continue,
        1 => HookResult::Skip,
        2 => HookResult::Abort,
        _ => HookResult::Error(...)
    }
}
```

**Hook environment variables:**

| Variable | Description |
|----------|-------------|
| `FRESHER_ITERATION` | Current iteration number |
| `FRESHER_LAST_EXIT_CODE` | Previous Claude exit code |
| `FRESHER_TOTAL_COMMITS` | Total commits so far |
| `FRESHER_DURATION` | Run duration in seconds |
| `FRESHER_FINISH_TYPE` | Finish type (in finished hook) |
| `FRESHER_MODE` | Current mode |
| `FRESHER_PROJECT_DIR` | Project root directory |

---

## 5. Configuration

### 5.1 config.toml Template

```toml
[fresher]
mode = "planning"
max_iterations = 0
smart_termination = true
dangerous_permissions = true
max_turns = 50
model = "sonnet"

[commands]
test = "cargo test"
build = "cargo build"
lint = "cargo clippy"

[paths]
log_dir = ".fresher/logs"
spec_dir = "specs"
src_dir = "src"

[hooks]
enabled = true
timeout = 30

[docker]
use_docker = false
memory = "4g"
cpus = "2"
```

---

## 6. Commands

### fresher plan

Runs planning mode - creates or updates IMPLEMENTATION_PLAN.md:

```bash
fresher plan [--max-iterations <n>]
```

**Behavior:**

1. Requires `.fresher/` directory (run `fresher init` first)
2. Loads `PROMPT.planning.md` or embedded template
3. Runs iterations until manual stop or smart termination
4. Creates/updates IMPLEMENTATION_PLAN.md

### fresher build

Runs building mode - implements tasks from the plan:

```bash
fresher build [--max-iterations <n>]
```

**Behavior:**

1. Requires `.fresher/` directory
2. Requires `IMPLEMENTATION_PLAN.md` (run `fresher plan` first)
3. Checks for pending tasks before each iteration
4. Terminates when all tasks are complete (`- [x]`)

---

## 7. Security Considerations

### Dangerous Permissions

- The `--dangerously-skip-permissions` flag allows Claude to execute any action without confirmation
- Only use in trusted environments or with Docker isolation
- All tool calls logged via stream output

### State File Protection

- `.fresher/.state` contains iteration metadata
- Should be gitignored to prevent conflicts
- Not sensitive but could affect loop behavior if tampered

---

## 8. Error Handling

| Condition | Behavior |
|-----------|----------|
| `.fresher/` not found | Exit with error, suggest `fresher init` |
| `claude` command not found | Exit with error, link to installation |
| IMPLEMENTATION_PLAN.md not found (build) | Exit with error, suggest `fresher plan` |
| Claude exits non-zero | Set `FinishType::Error`, stop loop |
| Hook timeout | Log warning, continue execution |
| Hook error | Log warning, continue execution |
| SIGINT received | Set `FinishType::Manual`, finish current work |

---

## 9. Implementation Notes

### Differences from v1.0 (Bash)

| Aspect | v1.0 (Bash) | v2.0 (Rust) |
|--------|-------------|-------------|
| Configuration | `config.sh` (sourced) | `config.toml` (parsed) |
| State format | Shell variables | TOML file |
| Streaming | jq + while read | serde_json + tokio |
| Signal handling | trap | tokio::signal |
| Async | Sequential | Async/await |
| Error handling | Exit codes | Result<T, Error> |

### Dependencies

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
anyhow = "1"
colored = "2"
chrono = { version = "0.4", features = ["serde"] }
which = "6"
regex = "1"
```
