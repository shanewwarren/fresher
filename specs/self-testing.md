# Self-Testing Specification

**Status:** Implemented
**Version:** 2.0
**Last Updated:** 2026-01-18
**Implementation:** Unit tests in `src/*.rs`, integration tests in `tests/*.rs`

---

## 1. Overview

### Purpose

Self-testing provides verification that Fresher works correctly. The test suite includes unit tests for individual modules and integration tests that exercise full command flows in isolated temporary directories.

### Goals

- **Confidence** - Know Fresher works before relying on it
- **Regression prevention** - Catch breakages from changes
- **Documentation** - Tests serve as executable examples
- **Fast feedback** - Tests run quickly without AI costs

### Non-Goals

- **AI quality testing** - Don't test Claude's output quality
- **End-to-end testing** - Focus on Fresher mechanics, not external services
- **Performance benchmarking** - Functional correctness over speed

---

## 2. Architecture

### Test Structure

```
fresher/
├── src/
│   ├── config.rs          # Contains unit tests
│   ├── verify.rs          # Contains unit tests
│   ├── streaming.rs       # Contains unit tests
│   └── hooks.rs           # Contains unit tests
└── tests/
    ├── init.rs            # Integration tests for `fresher init`
    ├── verify.rs          # Integration tests for `fresher verify`
    └── hooks.rs           # Integration tests for hooks
```

### Test Categories

| Category | Location | Purpose |
|----------|----------|---------|
| **Unit Tests** | `src/*.rs` | Test individual functions in isolation |
| **Integration Tests** | `tests/*.rs` | Test full command flows |

### Test Execution

```bash
# Run all tests
cargo test

# Run unit tests only
cargo test --lib

# Run integration tests only
cargo test --test init
cargo test --test verify
cargo test --test hooks

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_init_creates_directory_structure
```

---

## 3. Test Utilities

### Temporary Directory Setup

Tests use `tempfile::TempDir` for isolated test environments:

```rust
fn setup_test_project() -> (TempDir, PathBuf) {
    let original_dir = std::env::current_dir().unwrap();
    let dir = TempDir::new().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();
    (dir, original_dir)
}

fn teardown_test_project(original_dir: PathBuf) {
    let _ = std::env::set_current_dir(original_dir);
}
```

### Test Serialization

Tests that change the working directory are serialized using a mutex:

```rust
static TEST_MUTEX: Mutex<()> = Mutex::new(());

fn acquire_lock() -> MutexGuard<'static, ()> {
    TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner())
}
```

### Async Test Pattern

```rust
#[tokio::test]
async fn test_example() {
    let _lock = acquire_lock();
    let (dir, original_dir) = setup_test_project();

    // Test logic here
    let result = fresher::commands::init::run(false).await;

    teardown_test_project(original_dir);

    result.unwrap();
    assert!(dir.path().join(".fresher").exists());
}
```

---

## 4. Unit Tests

### 4.1 Config Tests (src/config.rs)

| Test | Purpose |
|------|---------|
| `test_default_config` | Verify default configuration values |
| `test_env_override_*` | Verify environment variable overrides |
| `test_project_type_name` | Verify project type name mapping |
| `test_project_type_default_commands` | Verify default commands per project type |
| `test_config_to_toml_string` | Verify TOML serialization |
| `test_config_roundtrip` | Verify config can be serialized and parsed back |

### 4.2 Verify Tests (src/verify.rs)

| Test | Purpose |
|------|---------|
| `test_parse_plan_*` | Test plan parsing (pending, completed, in-progress) |
| `test_count_tasks` | Verify task counting logic |
| `test_has_pending_tasks_*` | Test pending task detection |
| `test_extract_requirements_*` | Test spec requirement extraction |
| `test_analyze_coverage_*` | Test coverage calculation |
| `test_generate_report` | Test full report generation |

### 4.3 Streaming Tests (src/streaming.rs)

| Test | Purpose |
|------|---------|
| `test_parse_event_*` | Test parsing of stream-json events |
| `test_stream_handler_*` | Test stream handler configuration |
| `test_process_stream_*` | Test async stream processing |

### 4.4 Hooks Tests (src/hooks.rs)

| Test | Purpose |
|------|---------|
| `test_run_hook_*` | Test hook execution with various exit codes |
| `test_run_hook_timeout` | Test hook timeout handling |
| `test_run_hook_env_vars` | Test environment variable passing |

---

## 5. Integration Tests

### 5.1 Init Tests (tests/init.rs)

| Test | Purpose |
|------|---------|
| `test_init_creates_directory_structure` | Verify directory creation |
| `test_init_creates_config_file` | Verify config.toml generation |
| `test_init_creates_agents_md` | Verify AGENTS.md generation |
| `test_init_creates_prompt_templates` | Verify prompt template generation |
| `test_init_creates_executable_hooks` | Verify hook permissions |
| `test_init_creates_specs_readme` | Verify specs directory creation |
| `test_init_preserves_existing_specs` | Verify existing specs preserved |
| `test_init_fails_without_force` | Verify error on existing .fresher |
| `test_init_with_force_overwrites` | Verify --force behavior |
| `test_init_detects_*_project` | Verify project type detection |
| `test_hooks_have_shebang` | Verify hook shebang lines |

### 5.2 Verify Tests (tests/verify.rs)

| Test | Purpose |
|------|---------|
| `test_verify_missing_plan_file` | Test handling of missing plan |
| `test_verify_with_valid_plan` | Test with valid plan file |
| `test_verify_json_output` | Test JSON output format |
| `test_verify_task_counts` | Test task status counting |
| `test_verify_spec_refs` | Test spec reference extraction |
| `test_verify_coverage_report` | Test coverage calculation |
| `test_verify_orphan_tasks` | Test orphan task detection |
| `test_verify_priorities` | Test priority extraction |
| `test_verify_has_pending_tasks` | Test pending task detection |
| `test_verify_empty_specs` | Test with empty specs directory |
| `test_verify_rfc2119_extraction` | Test RFC 2119 keyword extraction |
| `test_verify_task_dependencies` | Test dependency parsing |

### 5.3 Hooks Tests (tests/hooks.rs)

Tests for lifecycle hook execution (started, next_iteration, finished).

---

## 6. Running Tests

### Command Reference

```bash
# Run all tests
cargo test

# Run with verbose output
cargo test -- --nocapture

# Run tests matching a pattern
cargo test verify

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test init
cargo test --test verify

# Run tests in parallel (default)
cargo test

# Run tests serially (for debugging)
cargo test -- --test-threads=1

# Run ignored tests
cargo test -- --ignored

# Run tests with coverage (requires cargo-tarpaulin)
cargo tarpaulin --out Html
```

### Test Statistics

| Category | Test Count |
|----------|------------|
| Unit Tests | 75 |
| Integration Tests | 45 |
| **Total** | **120** |

---

## 7. Dependencies

```toml
[dev-dependencies]
tempfile = "3"
tokio = { version = "1", features = ["full", "test-util"] }
```

---

## 8. Best Practices

### Test Isolation

- Each test creates its own temporary directory
- Tests restore the original working directory after completion
- Use mutex for tests that change global state (working directory)

### Async Tests

- Use `#[tokio::test]` for async test functions
- Integration tests call async command functions directly

### Assertions

```rust
// Basic assertions
assert!(condition);
assert_eq!(actual, expected);
assert!(result.is_ok());
assert!(result.is_err());

// With messages
assert!(path.exists(), "Expected {} to exist", path.display());
assert_eq!(count, 3, "Expected 3 tasks, found {}", count);
```

### Test Files

Tests create temporary files with realistic content:

```rust
let plan_content = r#"# Implementation Plan

## Priority 1: Core

- [x] Completed task (refs: specs/feature.md)
- [ ] Pending task
"#;
fs::write(dir.path().join("IMPLEMENTATION_PLAN.md"), plan_content).unwrap();
```

---

## 9. Future Enhancements

- **Snapshot testing**: Compare command output against saved snapshots
- **Fuzzing**: Test with randomly generated plans and specs
- **Benchmark tests**: Track performance regressions
- **Mock Claude CLI**: Test loop execution without real AI calls
- **CI integration**: Run tests on pull requests
