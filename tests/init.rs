//! Integration tests for the `fresher init` command
//!
//! Note: These tests should run serially because they change the working directory.
//! Run with: cargo test --test init -- --test-threads=1

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Mutex;
use tempfile::TempDir;

// Mutex to serialize tests that change working directory
static TEST_MUTEX: Mutex<()> = Mutex::new(());

/// Acquire the test mutex, recovering from poison if needed
fn acquire_lock() -> std::sync::MutexGuard<'static, ()> {
    TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner())
}

/// Create a test project in a temporary directory and change to it
/// Returns the TempDir and the original working directory
fn setup_test_project() -> (TempDir, PathBuf) {
    let original_dir = std::env::current_dir().unwrap();
    let dir = TempDir::new().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();
    (dir, original_dir)
}

/// Restore the working directory after test
fn teardown_test_project(original_dir: PathBuf) {
    let _ = std::env::set_current_dir(original_dir);
}

/// Test that init creates the basic directory structure
#[tokio::test]
async fn test_init_creates_directory_structure() {
    let _lock = acquire_lock();
    let (dir, original_dir) = setup_test_project();

    let result = fresher::commands::init::run(false).await;
    teardown_test_project(original_dir);

    result.unwrap();

    assert!(dir.path().join(".fresher").exists());
    assert!(dir.path().join(".fresher/hooks").exists());
    assert!(dir.path().join(".fresher/logs").exists());
    assert!(dir.path().join(".fresher/docker").exists());
    assert!(dir.path().join("specs").exists());
}

/// Test that init creates config.toml
#[tokio::test]
async fn test_init_creates_config_file() {
    let _lock = acquire_lock();
    let (dir, original_dir) = setup_test_project();

    let result = fresher::commands::init::run(false).await;
    teardown_test_project(original_dir);

    result.unwrap();

    let config_path = dir.path().join(".fresher/config.toml");
    assert!(config_path.exists());

    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("[fresher]"));
    assert!(content.contains("[commands]"));
    assert!(content.contains("[paths]"));
    assert!(content.contains("[hooks]"));
    assert!(content.contains("[docker]"));
}

/// Test that init creates AGENTS.md
#[tokio::test]
async fn test_init_creates_agents_md() {
    let _lock = acquire_lock();
    let (dir, original_dir) = setup_test_project();

    let result = fresher::commands::init::run(false).await;
    teardown_test_project(original_dir);

    result.unwrap();

    let agents_path = dir.path().join(".fresher/AGENTS.md");
    assert!(agents_path.exists());

    let content = fs::read_to_string(&agents_path).unwrap();
    assert!(content.contains("# Project:"));
}

/// Test that init creates prompt templates
#[tokio::test]
async fn test_init_creates_prompt_templates() {
    let _lock = acquire_lock();
    let (dir, original_dir) = setup_test_project();

    let result = fresher::commands::init::run(false).await;
    teardown_test_project(original_dir);

    result.unwrap();

    let planning_path = dir.path().join(".fresher/PROMPT.planning.md");
    let building_path = dir.path().join(".fresher/PROMPT.building.md");

    assert!(planning_path.exists());
    assert!(building_path.exists());

    let planning_content = fs::read_to_string(&planning_path).unwrap();
    assert!(planning_content.contains("Planning Mode"));

    let building_content = fs::read_to_string(&building_path).unwrap();
    assert!(building_content.contains("Building Mode"));
}

/// Test that init creates hook scripts with executable permissions
#[tokio::test]
async fn test_init_creates_executable_hooks() {
    let _lock = acquire_lock();
    let (dir, original_dir) = setup_test_project();

    let result = fresher::commands::init::run(false).await;
    teardown_test_project(original_dir);

    result.unwrap();

    let hooks = ["started", "next_iteration", "finished"];

    for hook in hooks {
        let hook_path = dir.path().join(".fresher/hooks").join(hook);
        assert!(hook_path.exists(), "Hook {} should exist", hook);

        let metadata = fs::metadata(&hook_path).unwrap();
        let mode = metadata.permissions().mode();
        assert!(
            mode & 0o111 != 0,
            "Hook {} should be executable (mode: {:o})",
            hook,
            mode
        );
    }
}

/// Test that init creates specs/README.md if specs directory doesn't exist
#[tokio::test]
async fn test_init_creates_specs_readme() {
    let _lock = acquire_lock();
    let (dir, original_dir) = setup_test_project();

    let result = fresher::commands::init::run(false).await;
    teardown_test_project(original_dir);

    result.unwrap();

    let readme_path = dir.path().join("specs/README.md");
    assert!(readme_path.exists());

    let content = fs::read_to_string(&readme_path).unwrap();
    assert!(content.contains("Specifications"));
}

/// Test that init does not overwrite existing specs directory
#[tokio::test]
async fn test_init_preserves_existing_specs() {
    let _lock = acquire_lock();
    let (dir, original_dir) = setup_test_project();

    // Create existing specs directory with content
    fs::create_dir_all(dir.path().join("specs")).unwrap();
    fs::write(dir.path().join("specs/existing.md"), "Existing spec").unwrap();

    let result = fresher::commands::init::run(false).await;
    teardown_test_project(original_dir);

    result.unwrap();

    // Verify existing content is preserved
    let existing_path = dir.path().join("specs/existing.md");
    assert!(existing_path.exists());
    let content = fs::read_to_string(&existing_path).unwrap();
    assert_eq!(content, "Existing spec");

    // README.md should not exist since specs/ already existed
    let readme_path = dir.path().join("specs/README.md");
    assert!(!readme_path.exists());
}

/// Test that init fails if .fresher already exists without --force
#[tokio::test]
async fn test_init_fails_without_force() {
    let _lock = acquire_lock();
    let (dir, original_dir) = setup_test_project();

    // Create existing .fresher directory
    fs::create_dir_all(dir.path().join(".fresher")).unwrap();

    let result = fresher::commands::init::run(false).await;
    teardown_test_project(original_dir);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("already exists"));
}

/// Test that init with --force overwrites existing .fresher
#[tokio::test]
async fn test_init_with_force_overwrites() {
    let _lock = acquire_lock();
    let (dir, original_dir) = setup_test_project();

    // Create existing .fresher directory with some content
    fs::create_dir_all(dir.path().join(".fresher")).unwrap();
    fs::write(dir.path().join(".fresher/old_file.txt"), "old content").unwrap();

    let result = fresher::commands::init::run(true).await;
    teardown_test_project(original_dir);

    result.unwrap();

    // Verify new content is created
    assert!(dir.path().join(".fresher/config.toml").exists());
    assert!(dir.path().join(".fresher/hooks/started").exists());
}

/// Test that init detects Rust project type
#[tokio::test]
async fn test_init_detects_rust_project() {
    let _lock = acquire_lock();
    let (dir, original_dir) = setup_test_project();

    // Create Cargo.toml to trigger Rust detection
    fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();

    let result = fresher::commands::init::run(false).await;
    teardown_test_project(original_dir);

    result.unwrap();

    let config_path = dir.path().join(".fresher/config.toml");
    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("cargo test") || content.contains("cargo"));
}

/// Test that init detects Bun project type
#[tokio::test]
async fn test_init_detects_bun_project() {
    let _lock = acquire_lock();
    let (dir, original_dir) = setup_test_project();

    // Create bun.lockb to trigger Bun detection
    fs::write(dir.path().join("bun.lockb"), "").unwrap();

    let result = fresher::commands::init::run(false).await;
    teardown_test_project(original_dir);

    result.unwrap();

    let config_path = dir.path().join(".fresher/config.toml");
    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("bun test") || content.contains("bun"));
}

/// Test that init detects Node.js project type
#[tokio::test]
async fn test_init_detects_nodejs_project() {
    let _lock = acquire_lock();
    let (dir, original_dir) = setup_test_project();

    // Create package.json to trigger Node.js detection (without bun.lockb)
    fs::write(dir.path().join("package.json"), "{}").unwrap();

    let result = fresher::commands::init::run(false).await;
    teardown_test_project(original_dir);

    result.unwrap();

    let config_path = dir.path().join(".fresher/config.toml");
    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("npm test") || content.contains("npm"));
}

/// Test that init detects Go project type
#[tokio::test]
async fn test_init_detects_go_project() {
    let _lock = acquire_lock();
    let (dir, original_dir) = setup_test_project();

    // Create go.mod to trigger Go detection
    fs::write(dir.path().join("go.mod"), "module test").unwrap();

    let result = fresher::commands::init::run(false).await;
    teardown_test_project(original_dir);

    result.unwrap();

    let config_path = dir.path().join(".fresher/config.toml");
    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("go test") || content.contains("go build"));
}

/// Test that init detects Python project type
#[tokio::test]
async fn test_init_detects_python_project() {
    let _lock = acquire_lock();
    let (dir, original_dir) = setup_test_project();

    // Create pyproject.toml to trigger Python detection
    fs::write(dir.path().join("pyproject.toml"), "[project]\nname = \"test\"").unwrap();

    let result = fresher::commands::init::run(false).await;
    teardown_test_project(original_dir);

    result.unwrap();

    let config_path = dir.path().join(".fresher/config.toml");
    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("pytest") || content.contains("python"));
}

/// Test that hook scripts have proper shebang
#[tokio::test]
async fn test_hooks_have_shebang() {
    let _lock = acquire_lock();
    let (dir, original_dir) = setup_test_project();

    let result = fresher::commands::init::run(false).await;
    teardown_test_project(original_dir);

    result.unwrap();

    let hooks = ["started", "next_iteration", "finished"];

    for hook in hooks {
        let hook_path = dir.path().join(".fresher/hooks").join(hook);
        let content = fs::read_to_string(&hook_path).unwrap();
        assert!(
            content.starts_with("#!/"),
            "Hook {} should have shebang",
            hook
        );
    }
}

/// Test that init creates Docker files for isolation support
#[tokio::test]
async fn test_init_creates_docker_files() {
    let _lock = acquire_lock();
    let (dir, original_dir) = setup_test_project();

    let result = fresher::commands::init::run(false).await;
    teardown_test_project(original_dir);

    result.unwrap();

    // Check docker directory and files exist
    assert!(dir.path().join(".fresher/docker").exists());
    assert!(dir.path().join(".fresher/docker/docker-compose.yml").exists());
    assert!(dir.path().join(".fresher/docker/devcontainer.json").exists());
    assert!(dir.path().join(".fresher/docker/fresher-firewall-overlay.sh").exists());
    assert!(dir.path().join(".fresher/run.sh").exists());

    // Check docker-compose.yml content
    let compose_content = fs::read_to_string(dir.path().join(".fresher/docker/docker-compose.yml")).unwrap();
    assert!(compose_content.contains("services:"));
    assert!(compose_content.contains("fresher:"));
    assert!(compose_content.contains("FRESHER_IN_DOCKER=true"));

    // Check devcontainer.json content
    let devcontainer_content = fs::read_to_string(dir.path().join(".fresher/docker/devcontainer.json")).unwrap();
    assert!(devcontainer_content.contains("Fresher Loop Environment"));
    assert!(devcontainer_content.contains("claude-code-devcontainer"));

    // Check firewall overlay is executable
    let firewall_path = dir.path().join(".fresher/docker/fresher-firewall-overlay.sh");
    let metadata = fs::metadata(&firewall_path).unwrap();
    let mode = metadata.permissions().mode();
    assert!(mode & 0o111 != 0, "Firewall overlay should be executable");

    // Check run.sh is executable
    let run_path = dir.path().join(".fresher/run.sh");
    let metadata = fs::metadata(&run_path).unwrap();
    let mode = metadata.permissions().mode();
    assert!(mode & 0o111 != 0, "run.sh should be executable");

    // Check run.sh invokes fresher commands
    let run_content = fs::read_to_string(&run_path).unwrap();
    assert!(run_content.contains("fresher plan"));
    assert!(run_content.contains("fresher build"));
}
