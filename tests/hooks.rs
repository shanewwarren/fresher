//! Integration tests for hook execution behavior

use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

use fresher::config::Config;
use fresher::hooks::{
    run_finished_hook, run_hook, run_next_iteration_hook, run_started_hook, HookResult,
    HOOK_ABORT, HOOK_CONTINUE, HOOK_SKIP,
};
use fresher::state::State;

/// Create a test project with hooks directory
fn setup_test_project() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join(".fresher/hooks")).unwrap();
    dir
}

/// Create a test config with hooks enabled
fn create_test_config(enabled: bool, timeout: u32) -> Config {
    let mut config = Config::default();
    config.hooks.enabled = enabled;
    config.hooks.timeout = timeout;
    config
}

/// Create a test state
fn create_test_state() -> State {
    State::new()
}

/// Create a hook script at the given path
fn create_hook_script(dir: &TempDir, name: &str, script: &str) {
    let hooks_dir = dir.path().join(".fresher/hooks");
    let hook_path = hooks_dir.join(name);

    let mut file = fs::File::create(&hook_path).unwrap();
    file.write_all(script.as_bytes()).unwrap();

    // Make executable
    let mut perms = fs::metadata(&hook_path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&hook_path, perms).unwrap();
}

/// Test that hook returns Continue on exit code 0
#[tokio::test]
async fn test_hook_exit_code_continue() {
    let dir = setup_test_project();
    let config = create_test_config(true, 30);
    let state = create_test_state();

    let script = format!("#!/bin/bash\nexit {}\n", HOOK_CONTINUE);
    create_hook_script(&dir, "test_hook", &script);

    let result = run_hook("test_hook", &state, &config, dir.path())
        .await
        .unwrap();

    assert!(matches!(result, HookResult::Continue));
}

/// Test that hook returns Skip on exit code 1
#[tokio::test]
async fn test_hook_exit_code_skip() {
    let dir = setup_test_project();
    let config = create_test_config(true, 30);
    let state = create_test_state();

    let script = format!("#!/bin/bash\nexit {}\n", HOOK_SKIP);
    create_hook_script(&dir, "test_hook", &script);

    let result = run_hook("test_hook", &state, &config, dir.path())
        .await
        .unwrap();

    assert!(matches!(result, HookResult::Skip));
}

/// Test that hook returns Abort on exit code 2
#[tokio::test]
async fn test_hook_exit_code_abort() {
    let dir = setup_test_project();
    let config = create_test_config(true, 30);
    let state = create_test_state();

    let script = format!("#!/bin/bash\nexit {}\n", HOOK_ABORT);
    create_hook_script(&dir, "test_hook", &script);

    let result = run_hook("test_hook", &state, &config, dir.path())
        .await
        .unwrap();

    assert!(matches!(result, HookResult::Abort));
}

/// Test that hook returns Error on invalid exit code
#[tokio::test]
async fn test_hook_invalid_exit_code() {
    let dir = setup_test_project();
    let config = create_test_config(true, 30);
    let state = create_test_state();

    create_hook_script(&dir, "test_hook", "#!/bin/bash\nexit 99\n");

    let result = run_hook("test_hook", &state, &config, dir.path())
        .await
        .unwrap();

    assert!(matches!(result, HookResult::Error(_)));
    if let HookResult::Error(msg) = result {
        assert!(msg.contains("99"));
    }
}

/// Test that hook times out correctly
#[tokio::test]
async fn test_hook_timeout() {
    let dir = setup_test_project();
    let config = create_test_config(true, 1); // 1 second timeout
    let state = create_test_state();

    // Script that sleeps longer than timeout
    create_hook_script(&dir, "test_hook", "#!/bin/bash\nsleep 10\nexit 0\n");

    let result = run_hook("test_hook", &state, &config, dir.path())
        .await
        .unwrap();

    assert!(matches!(result, HookResult::Timeout));
}

/// Test that hooks are disabled when config says so
#[tokio::test]
async fn test_hooks_disabled() {
    let dir = setup_test_project();
    let config = create_test_config(false, 30); // Hooks disabled
    let state = create_test_state();

    create_hook_script(&dir, "test_hook", "#!/bin/bash\nexit 0\n");

    let result = run_hook("test_hook", &state, &config, dir.path())
        .await
        .unwrap();

    assert!(matches!(result, HookResult::NotFound));
}

/// Test hook not found when script doesn't exist
#[tokio::test]
async fn test_hook_not_found() {
    let dir = setup_test_project();
    let config = create_test_config(true, 30);
    let state = create_test_state();

    let result = run_hook("nonexistent_hook", &state, &config, dir.path())
        .await
        .unwrap();

    assert!(matches!(result, HookResult::NotFound));
}

/// Test hook not executable returns NotFound
#[tokio::test]
async fn test_hook_not_executable() {
    let dir = setup_test_project();
    let config = create_test_config(true, 30);
    let state = create_test_state();

    // Create hook but don't make it executable
    let hooks_dir = dir.path().join(".fresher/hooks");
    let hook_path = hooks_dir.join("test_hook");
    fs::write(&hook_path, "#!/bin/bash\nexit 0\n").unwrap();

    // Ensure NOT executable
    let mut perms = fs::metadata(&hook_path).unwrap().permissions();
    perms.set_mode(0o644);
    fs::set_permissions(&hook_path, perms).unwrap();

    let result = run_hook("test_hook", &state, &config, dir.path())
        .await
        .unwrap();

    assert!(matches!(result, HookResult::NotFound));
}

/// Test run_started_hook returns true for continue
#[tokio::test]
async fn test_started_hook_continue() {
    let dir = setup_test_project();
    let config = create_test_config(true, 30);
    let state = create_test_state();

    create_hook_script(&dir, "started", "#!/bin/bash\nexit 0\n");

    let result = run_started_hook(&state, &config, dir.path()).await.unwrap();

    assert!(result);
}

/// Test run_started_hook returns false for abort
#[tokio::test]
async fn test_started_hook_abort() {
    let dir = setup_test_project();
    let config = create_test_config(true, 30);
    let state = create_test_state();

    create_hook_script(&dir, "started", "#!/bin/bash\nexit 2\n");

    let result = run_started_hook(&state, &config, dir.path()).await.unwrap();

    assert!(!result);
}

/// Test run_started_hook continues on timeout
#[tokio::test]
async fn test_started_hook_timeout_continues() {
    let dir = setup_test_project();
    let config = create_test_config(true, 1);
    let state = create_test_state();

    create_hook_script(&dir, "started", "#!/bin/bash\nsleep 10\nexit 0\n");

    let result = run_started_hook(&state, &config, dir.path()).await.unwrap();

    // Should continue despite timeout
    assert!(result);
}

/// Test run_next_iteration_hook returns skip flag
#[tokio::test]
async fn test_next_iteration_hook_skip() {
    let dir = setup_test_project();
    let config = create_test_config(true, 30);
    let state = create_test_state();

    create_hook_script(&dir, "next_iteration", "#!/bin/bash\nexit 1\n");

    let (should_continue, should_skip) =
        run_next_iteration_hook(&state, &config, dir.path())
            .await
            .unwrap();

    assert!(should_continue);
    assert!(should_skip);
}

/// Test run_next_iteration_hook abort stops loop
#[tokio::test]
async fn test_next_iteration_hook_abort() {
    let dir = setup_test_project();
    let config = create_test_config(true, 30);
    let state = create_test_state();

    create_hook_script(&dir, "next_iteration", "#!/bin/bash\nexit 2\n");

    let (should_continue, should_skip) =
        run_next_iteration_hook(&state, &config, dir.path())
            .await
            .unwrap();

    assert!(!should_continue);
    assert!(!should_skip);
}

/// Test run_finished_hook completes without error
#[tokio::test]
async fn test_finished_hook() {
    let dir = setup_test_project();
    let config = create_test_config(true, 30);
    let state = create_test_state();

    create_hook_script(&dir, "finished", "#!/bin/bash\nexit 0\n");

    let result = run_finished_hook(&state, &config, dir.path()).await;

    assert!(result.is_ok());
}

/// Test hooks receive environment variables
#[tokio::test]
async fn test_hooks_receive_env_vars() {
    let dir = setup_test_project();
    let config = create_test_config(true, 30);
    let mut state = create_test_state();
    state.iteration = 5;
    state.total_commits = 10;

    // Script that checks env vars
    let script = r#"#!/bin/bash
if [ "$FRESHER_ITERATION" != "5" ]; then
    exit 99
fi
if [ "$FRESHER_TOTAL_COMMITS" != "10" ]; then
    exit 99
fi
exit 0
"#;
    create_hook_script(&dir, "test_hook", script);

    let result = run_hook("test_hook", &state, &config, dir.path())
        .await
        .unwrap();

    assert!(
        matches!(result, HookResult::Continue),
        "Hook should receive correct env vars, got {:?}",
        result
    );
}

/// Test hook receives FRESHER_MODE from config
#[tokio::test]
async fn test_hooks_receive_mode() {
    let dir = setup_test_project();
    let mut config = create_test_config(true, 30);
    config.fresher.mode = "building".to_string();
    let state = create_test_state();

    let script = r#"#!/bin/bash
if [ "$FRESHER_MODE" != "building" ]; then
    exit 99
fi
exit 0
"#;
    create_hook_script(&dir, "test_hook", script);

    let result = run_hook("test_hook", &state, &config, dir.path())
        .await
        .unwrap();

    assert!(matches!(result, HookResult::Continue));
}

/// Test hook receives FRESHER_PROJECT_DIR
#[tokio::test]
async fn test_hooks_receive_project_dir() {
    let dir = setup_test_project();
    let config = create_test_config(true, 30);
    let state = create_test_state();

    // Script checks project dir is set
    let script = r#"#!/bin/bash
if [ -z "$FRESHER_PROJECT_DIR" ]; then
    exit 99
fi
exit 0
"#;
    create_hook_script(&dir, "test_hook", script);

    let result = run_hook("test_hook", &state, &config, dir.path())
        .await
        .unwrap();

    assert!(matches!(result, HookResult::Continue));
}
