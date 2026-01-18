//! Integration tests for the `fresher verify` command
//!
//! Note: These tests must run serially because they change the working directory.
//! Run with: cargo test --test verify -- --test-threads=1

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tempfile::TempDir;

// Mutex to serialize tests that change working directory
static TEST_MUTEX: Mutex<()> = Mutex::new(());

/// Acquire the test mutex, recovering from poison if needed
fn acquire_lock() -> std::sync::MutexGuard<'static, ()> {
    TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner())
}

/// Create a test project with .fresher directory structure
/// Returns the TempDir and the original working directory
fn setup_test_project() -> (TempDir, PathBuf) {
    let original_dir = std::env::current_dir().unwrap();
    let dir = TempDir::new().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();

    // Create minimal .fresher structure
    fs::create_dir_all(dir.path().join(".fresher")).unwrap();
    fs::create_dir_all(dir.path().join("specs")).unwrap();

    // Create default config.toml
    let config = r#"
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
"#;
    fs::write(dir.path().join(".fresher/config.toml"), config).unwrap();

    (dir, original_dir)
}

/// Restore the working directory after test
fn teardown_test_project(original_dir: PathBuf) {
    let _ = std::env::set_current_dir(original_dir);
}

/// Test verify command with missing plan file
#[tokio::test]
async fn test_verify_missing_plan_file() {
    let _lock = acquire_lock();
    let (_dir, original_dir) = setup_test_project();

    let result = fresher::commands::verify::run(false, "nonexistent.md".to_string()).await;
    teardown_test_project(original_dir);

    assert!(result.is_ok());
}

/// Test verify command with valid plan file
#[tokio::test]
async fn test_verify_with_valid_plan() {
    let _lock = acquire_lock();
    let (dir, original_dir) = setup_test_project();

    let plan_content = r#"# Implementation Plan

## Priority 1: Core Features

- [x] Implement feature A (refs: specs/feature.md)
- [ ] Implement feature B (refs: specs/feature.md)
- [~] Implement feature C
"#;
    fs::write(dir.path().join("IMPLEMENTATION_PLAN.md"), plan_content).unwrap();

    let spec_content = r#"# Feature Spec

### Feature Details

The system MUST implement feature A.
"#;
    fs::write(dir.path().join("specs/feature.md"), spec_content).unwrap();

    let result =
        fresher::commands::verify::run(false, "IMPLEMENTATION_PLAN.md".to_string()).await;
    teardown_test_project(original_dir);

    assert!(result.is_ok());
}

/// Test verify command produces valid JSON output
#[tokio::test]
async fn test_verify_json_output() {
    let _lock = acquire_lock();
    let (dir, original_dir) = setup_test_project();

    let plan_content = r#"# Implementation Plan

## Priority 1: Test

- [x] Completed task (refs: specs/test.md)
- [ ] Pending task
"#;
    fs::write(dir.path().join("IMPLEMENTATION_PLAN.md"), plan_content).unwrap();
    fs::write(dir.path().join("specs/test.md"), "### Test Section\n").unwrap();

    let result =
        fresher::commands::verify::run(true, "IMPLEMENTATION_PLAN.md".to_string()).await;
    teardown_test_project(original_dir);

    assert!(result.is_ok());
}

/// Test verify correctly counts task statuses
#[tokio::test]
async fn test_verify_task_counts() {
    let _lock = acquire_lock();
    let (dir, original_dir) = setup_test_project();

    let plan_content = r#"# Implementation Plan

## Priority 1

- [x] Completed 1
- [x] Completed 2
- [ ] Pending 1
- [ ] Pending 2
- [ ] Pending 3
- [~] In Progress 1
"#;
    fs::write(dir.path().join("IMPLEMENTATION_PLAN.md"), plan_content).unwrap();

    let plan_path = dir.path().join("IMPLEMENTATION_PLAN.md");
    let tasks = fresher::verify::parse_plan(&plan_path).unwrap();
    teardown_test_project(original_dir);

    assert_eq!(tasks.len(), 6);

    let completed = tasks
        .iter()
        .filter(|t| t.status == fresher::verify::TaskStatus::Completed)
        .count();
    let pending = tasks
        .iter()
        .filter(|t| t.status == fresher::verify::TaskStatus::Pending)
        .count();
    let in_progress = tasks
        .iter()
        .filter(|t| t.status == fresher::verify::TaskStatus::InProgress)
        .count();

    assert_eq!(completed, 2);
    assert_eq!(pending, 3);
    assert_eq!(in_progress, 1);
}

/// Test verify extracts spec references correctly
#[tokio::test]
async fn test_verify_spec_refs() {
    let _lock = acquire_lock();
    let (dir, original_dir) = setup_test_project();

    let plan_content = r#"# Implementation Plan

- [ ] Task with single ref (refs: specs/a.md)
- [ ] Task with multiple refs (refs: specs/b.md, specs/c.md)
- [ ] Task without refs
"#;
    fs::write(dir.path().join("IMPLEMENTATION_PLAN.md"), plan_content).unwrap();

    let plan_path = dir.path().join("IMPLEMENTATION_PLAN.md");
    let tasks = fresher::verify::parse_plan(&plan_path).unwrap();
    teardown_test_project(original_dir);

    assert_eq!(tasks.len(), 3);
    assert_eq!(tasks[0].spec_refs.len(), 1);
    assert_eq!(tasks[1].spec_refs.len(), 2);
    assert_eq!(tasks[2].spec_refs.len(), 0);
}

/// Test verify generates coverage report
#[tokio::test]
async fn test_verify_coverage_report() {
    let _lock = acquire_lock();
    let (dir, original_dir) = setup_test_project();

    let plan_content = r#"# Implementation Plan

- [ ] Task A (refs: specs/feature.md)
- [ ] Task B (refs: specs/feature.md)
"#;
    fs::write(dir.path().join("IMPLEMENTATION_PLAN.md"), plan_content).unwrap();

    let spec_content = r#"# Feature Spec

### Section 1
Content here

### Section 2
More content
"#;
    fs::write(dir.path().join("specs/feature.md"), spec_content).unwrap();

    let plan_path = dir.path().join("IMPLEMENTATION_PLAN.md");
    let spec_dir = dir.path().join("specs");
    let report = fresher::verify::generate_report(&plan_path, &spec_dir).unwrap();
    teardown_test_project(original_dir);

    assert_eq!(report.total_tasks, 2);
    assert_eq!(report.tasks_with_refs, 2);
    assert_eq!(report.orphan_tasks, 0);
    assert!(!report.coverage.is_empty());
}

/// Test verify handles orphan tasks (tasks without spec refs)
#[tokio::test]
async fn test_verify_orphan_tasks() {
    let _lock = acquire_lock();
    let (dir, original_dir) = setup_test_project();

    let plan_content = r#"# Implementation Plan

- [ ] Task with ref (refs: specs/feature.md)
- [ ] Orphan task 1
- [ ] Orphan task 2
"#;
    fs::write(dir.path().join("IMPLEMENTATION_PLAN.md"), plan_content).unwrap();
    fs::write(dir.path().join("specs/feature.md"), "### Section\n").unwrap();

    let plan_path = dir.path().join("IMPLEMENTATION_PLAN.md");
    let spec_dir = dir.path().join("specs");
    let report = fresher::verify::generate_report(&plan_path, &spec_dir).unwrap();
    teardown_test_project(original_dir);

    assert_eq!(report.total_tasks, 3);
    assert_eq!(report.tasks_with_refs, 1);
    assert_eq!(report.orphan_tasks, 2);
}

/// Test verify extracts priorities correctly
#[tokio::test]
async fn test_verify_priorities() {
    let _lock = acquire_lock();
    let (dir, original_dir) = setup_test_project();

    let plan_content = r#"# Implementation Plan

## Priority 1: Foundation

- [ ] Task in P1

## Priority 2: Features

- [ ] Task in P2

## Priority 3: Polish

- [ ] Task in P3
"#;
    fs::write(dir.path().join("IMPLEMENTATION_PLAN.md"), plan_content).unwrap();

    let plan_path = dir.path().join("IMPLEMENTATION_PLAN.md");
    let tasks = fresher::verify::parse_plan(&plan_path).unwrap();
    teardown_test_project(original_dir);

    assert_eq!(tasks.len(), 3);
    assert_eq!(tasks[0].priority, Some(1));
    assert_eq!(tasks[1].priority, Some(2));
    assert_eq!(tasks[2].priority, Some(3));
}

/// Test verify handles has_pending_tasks function
#[tokio::test]
async fn test_verify_has_pending_tasks() {
    let _lock = acquire_lock();
    let (dir, original_dir) = setup_test_project();

    let plan_with_pending = "- [ ] Pending task\n- [x] Completed task";
    fs::write(dir.path().join("plan_pending.md"), plan_with_pending).unwrap();

    let plan_all_complete = "- [x] Completed task 1\n- [x] Completed task 2";
    fs::write(dir.path().join("plan_complete.md"), plan_all_complete).unwrap();

    let has_pending = fresher::verify::has_pending_tasks(&dir.path().join("plan_pending.md"));
    let all_complete = fresher::verify::has_pending_tasks(&dir.path().join("plan_complete.md"));
    teardown_test_project(original_dir);

    assert!(has_pending);
    assert!(!all_complete);
}

/// Test verify handles empty specs directory
#[tokio::test]
async fn test_verify_empty_specs() {
    let _lock = acquire_lock();
    let (dir, original_dir) = setup_test_project();

    let plan_content = "- [ ] Task without spec refs\n";
    fs::write(dir.path().join("IMPLEMENTATION_PLAN.md"), plan_content).unwrap();

    let plan_path = dir.path().join("IMPLEMENTATION_PLAN.md");
    let spec_dir = dir.path().join("specs");
    let report = fresher::verify::generate_report(&plan_path, &spec_dir).unwrap();
    teardown_test_project(original_dir);

    assert_eq!(report.total_tasks, 1);
    assert!(report.coverage.is_empty());
}

/// Test verify handles RFC 2119 keywords in specs
#[tokio::test]
async fn test_verify_rfc2119_extraction() {
    let _lock = acquire_lock();
    let (dir, original_dir) = setup_test_project();

    let spec_content = r#"# Test Spec

The system MUST validate input.
The system SHOULD handle errors gracefully.
The system MAY cache results.
"#;
    fs::write(dir.path().join("specs/test.md"), spec_content).unwrap();

    let spec_dir = dir.path().join("specs");
    let reqs = fresher::verify::extract_requirements(&spec_dir).unwrap();
    teardown_test_project(original_dir);

    let rfc2119_count = reqs
        .iter()
        .filter(|r| r.req_type == fresher::verify::RequirementType::Rfc2119)
        .count();

    assert_eq!(rfc2119_count, 3);
}

/// Test verify handles dependencies in tasks
#[tokio::test]
async fn test_verify_task_dependencies() {
    let _lock = acquire_lock();
    let (dir, original_dir) = setup_test_project();

    let plan_content = r#"# Implementation Plan

- [ ] Task with deps
  - Dependencies: Module A, Module B
  - Complexity: high

- [ ] Task without deps
  - Dependencies: none
"#;
    fs::write(dir.path().join("IMPLEMENTATION_PLAN.md"), plan_content).unwrap();

    let plan_path = dir.path().join("IMPLEMENTATION_PLAN.md");
    let tasks = fresher::verify::parse_plan(&plan_path).unwrap();
    teardown_test_project(original_dir);

    assert_eq!(tasks.len(), 2);
    assert_eq!(tasks[0].dependencies.len(), 2);
    assert!(tasks[0].dependencies.contains(&"Module A".to_string()));
    assert!(tasks[0].dependencies.contains(&"Module B".to_string()));
    assert_eq!(tasks[0].complexity, Some("high".to_string()));
    assert!(tasks[1].dependencies.is_empty());
}

/// Test verify with JSON output for missing plan file
#[tokio::test]
async fn test_verify_json_missing_plan() {
    let _lock = acquire_lock();
    let (_dir, original_dir) = setup_test_project();

    let result = fresher::commands::verify::run(true, "missing.md".to_string()).await;
    teardown_test_project(original_dir);

    assert!(result.is_ok());
}
