use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

use crate::config::Config;
use crate::state::State;

/// Hook exit codes
pub const HOOK_CONTINUE: i32 = 0;
pub const HOOK_SKIP: i32 = 1;
pub const HOOK_ABORT: i32 = 2;

/// Hook execution result
#[derive(Debug)]
pub enum HookResult {
    /// Continue execution
    Continue,
    /// Skip this iteration (only for next_iteration)
    Skip,
    /// Abort the loop
    Abort,
    /// Hook not found or not executable
    NotFound,
    /// Hook timed out
    Timeout,
    /// Hook failed with error
    Error(String),
}

/// Run a hook script
pub async fn run_hook(
    hook_name: &str,
    state: &State,
    config: &Config,
    project_dir: &Path,
) -> Result<HookResult> {
    if !config.hooks.enabled {
        return Ok(HookResult::NotFound);
    }

    let hook_path = project_dir.join(".fresher/hooks").join(hook_name);

    // Check if hook exists and is executable
    if !hook_path.exists() {
        return Ok(HookResult::NotFound);
    }

    // Check if executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = std::fs::metadata(&hook_path)?;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Ok(HookResult::NotFound);
        }
    }

    // Build environment variables
    let mut env_vars = state.to_env_vars();
    env_vars.push(("FRESHER_PROJECT_DIR".to_string(), project_dir.display().to_string()));
    env_vars.push(("FRESHER_MODE".to_string(), config.fresher.mode.clone()));

    // Create command
    let mut cmd = Command::new(&hook_path);
    cmd.current_dir(project_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .envs(env_vars);

    // Run with timeout
    let timeout_duration = Duration::from_secs(config.hooks.timeout as u64);

    let result = timeout(timeout_duration, cmd.status()).await;

    match result {
        Ok(Ok(status)) => {
            let code = status.code().unwrap_or(-1);
            match code {
                HOOK_CONTINUE => Ok(HookResult::Continue),
                HOOK_SKIP => Ok(HookResult::Skip),
                HOOK_ABORT => Ok(HookResult::Abort),
                _ => Ok(HookResult::Error(format!("Hook exited with code {}", code))),
            }
        }
        Ok(Err(e)) => Ok(HookResult::Error(format!("Failed to run hook: {}", e))),
        Err(_) => Ok(HookResult::Timeout),
    }
}

/// Run the 'started' hook
pub async fn run_started_hook(state: &State, config: &Config, project_dir: &Path) -> Result<bool> {
    match run_hook("started", state, config, project_dir).await? {
        HookResult::Continue | HookResult::NotFound => Ok(true),
        HookResult::Abort => {
            eprintln!("Started hook requested abort");
            Ok(false)
        }
        HookResult::Timeout => {
            eprintln!("Warning: started hook timed out");
            Ok(true) // Continue despite timeout
        }
        HookResult::Error(e) => {
            eprintln!("Warning: started hook error: {}", e);
            Ok(true) // Continue despite error
        }
        HookResult::Skip => Ok(true), // Skip doesn't apply to started
    }
}

/// Run the 'next_iteration' hook
/// Returns: (should_continue, should_skip_iteration)
pub async fn run_next_iteration_hook(
    state: &State,
    config: &Config,
    project_dir: &Path,
) -> Result<(bool, bool)> {
    match run_hook("next_iteration", state, config, project_dir).await? {
        HookResult::Continue | HookResult::NotFound => Ok((true, false)),
        HookResult::Skip => Ok((true, true)),
        HookResult::Abort => {
            eprintln!("Next iteration hook requested abort");
            Ok((false, false))
        }
        HookResult::Timeout => {
            eprintln!("Warning: next_iteration hook timed out");
            Ok((true, false)) // Continue despite timeout
        }
        HookResult::Error(e) => {
            eprintln!("Warning: next_iteration hook error: {}", e);
            Ok((true, false)) // Continue despite error
        }
    }
}

/// Run the 'finished' hook
pub async fn run_finished_hook(state: &State, config: &Config, project_dir: &Path) -> Result<()> {
    match run_hook("finished", state, config, project_dir).await? {
        HookResult::Continue | HookResult::NotFound | HookResult::Skip | HookResult::Abort => Ok(()),
        HookResult::Timeout => {
            eprintln!("Warning: finished hook timed out");
            Ok(())
        }
        HookResult::Error(e) => {
            eprintln!("Warning: finished hook error: {}", e);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_config(hooks_enabled: bool, timeout: u32) -> Config {
        Config {
            fresher: crate::config::FresherConfig {
                mode: "building".to_string(),
                max_iterations: 0,
                smart_termination: true,
                dangerous_permissions: true,
                max_turns: 50,
                model: "sonnet".to_string(),
            },
            commands: crate::config::CommandsConfig {
                test: String::new(),
                build: String::new(),
                lint: String::new(),
            },
            paths: crate::config::PathsConfig {
                log_dir: ".fresher/logs".to_string(),
                spec_dir: "specs".to_string(),
                src_dir: "src".to_string(),
            },
            hooks: crate::config::HooksConfig {
                enabled: hooks_enabled,
                timeout,
            },
            docker: crate::config::DockerConfig {
                use_docker: false,
                memory: "4g".to_string(),
                cpus: "2".to_string(),
                presets: Vec::new(),
                setup_script: None,
            },
        }
    }

    fn create_test_state() -> State {
        State::new()
    }

    fn create_hook_script(dir: &TempDir, name: &str, script: &str) -> std::path::PathBuf {
        let hooks_dir = dir.path().join(".fresher/hooks");
        fs::create_dir_all(&hooks_dir).unwrap();
        let hook_path = hooks_dir.join(name);
        let mut file = fs::File::create(&hook_path).unwrap();
        file.write_all(script.as_bytes()).unwrap();

        // Make executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&hook_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&hook_path, perms).unwrap();
        }

        hook_path
    }

    #[test]
    fn test_hook_constants() {
        assert_eq!(HOOK_CONTINUE, 0);
        assert_eq!(HOOK_SKIP, 1);
        assert_eq!(HOOK_ABORT, 2);
    }

    #[tokio::test]
    async fn test_run_hook_disabled() {
        let dir = TempDir::new().unwrap();
        let config = create_test_config(false, 30);
        let state = create_test_state();

        let result = run_hook("started", &state, &config, dir.path()).await.unwrap();
        assert!(matches!(result, HookResult::NotFound));
    }

    #[tokio::test]
    async fn test_run_hook_not_found() {
        let dir = TempDir::new().unwrap();
        let config = create_test_config(true, 30);
        let state = create_test_state();

        // Create hooks directory but no hook file
        fs::create_dir_all(dir.path().join(".fresher/hooks")).unwrap();

        let result = run_hook("nonexistent", &state, &config, dir.path()).await.unwrap();
        assert!(matches!(result, HookResult::NotFound));
    }

    #[tokio::test]
    async fn test_run_hook_continue() {
        let dir = TempDir::new().unwrap();
        let config = create_test_config(true, 30);
        let state = create_test_state();

        create_hook_script(&dir, "test_hook", "#!/bin/bash\nexit 0\n");

        let result = run_hook("test_hook", &state, &config, dir.path()).await.unwrap();
        assert!(matches!(result, HookResult::Continue));
    }

    #[tokio::test]
    async fn test_run_hook_skip() {
        let dir = TempDir::new().unwrap();
        let config = create_test_config(true, 30);
        let state = create_test_state();

        create_hook_script(&dir, "test_hook", "#!/bin/bash\nexit 1\n");

        let result = run_hook("test_hook", &state, &config, dir.path()).await.unwrap();
        assert!(matches!(result, HookResult::Skip));
    }

    #[tokio::test]
    async fn test_run_hook_abort() {
        let dir = TempDir::new().unwrap();
        let config = create_test_config(true, 30);
        let state = create_test_state();

        create_hook_script(&dir, "test_hook", "#!/bin/bash\nexit 2\n");

        let result = run_hook("test_hook", &state, &config, dir.path()).await.unwrap();
        assert!(matches!(result, HookResult::Abort));
    }

    #[tokio::test]
    async fn test_run_hook_error_exit_code() {
        let dir = TempDir::new().unwrap();
        let config = create_test_config(true, 30);
        let state = create_test_state();

        create_hook_script(&dir, "test_hook", "#!/bin/bash\nexit 99\n");

        let result = run_hook("test_hook", &state, &config, dir.path()).await.unwrap();
        assert!(matches!(result, HookResult::Error(_)));
    }

    #[tokio::test]
    async fn test_run_hook_timeout() {
        let dir = TempDir::new().unwrap();
        // Very short timeout to trigger timeout
        let config = create_test_config(true, 1);
        let state = create_test_state();

        create_hook_script(&dir, "test_hook", "#!/bin/bash\nsleep 10\nexit 0\n");

        let result = run_hook("test_hook", &state, &config, dir.path()).await.unwrap();
        assert!(matches!(result, HookResult::Timeout));
    }

    #[tokio::test]
    async fn test_run_hook_env_vars() {
        let dir = TempDir::new().unwrap();
        let config = create_test_config(true, 30);
        let mut state = create_test_state();
        state.iteration = 5;
        state.last_exit_code = 0;

        // Hook that checks environment variables
        let script = r#"#!/bin/bash
if [ "$FRESHER_ITERATION" = "5" ] && [ "$FRESHER_MODE" = "building" ]; then
    exit 0
else
    exit 99
fi
"#;
        create_hook_script(&dir, "test_hook", script);

        let result = run_hook("test_hook", &state, &config, dir.path()).await.unwrap();
        assert!(matches!(result, HookResult::Continue));
    }

    #[tokio::test]
    async fn test_run_started_hook_continue() {
        let dir = TempDir::new().unwrap();
        let config = create_test_config(true, 30);
        let state = create_test_state();

        create_hook_script(&dir, "started", "#!/bin/bash\nexit 0\n");

        let should_continue = run_started_hook(&state, &config, dir.path()).await.unwrap();
        assert!(should_continue);
    }

    #[tokio::test]
    async fn test_run_started_hook_abort() {
        let dir = TempDir::new().unwrap();
        let config = create_test_config(true, 30);
        let state = create_test_state();

        create_hook_script(&dir, "started", "#!/bin/bash\nexit 2\n");

        let should_continue = run_started_hook(&state, &config, dir.path()).await.unwrap();
        assert!(!should_continue);
    }

    #[tokio::test]
    async fn test_run_next_iteration_hook_skip() {
        let dir = TempDir::new().unwrap();
        let config = create_test_config(true, 30);
        let state = create_test_state();

        create_hook_script(&dir, "next_iteration", "#!/bin/bash\nexit 1\n");

        let (should_continue, should_skip) = run_next_iteration_hook(&state, &config, dir.path()).await.unwrap();
        assert!(should_continue);
        assert!(should_skip);
    }

    #[tokio::test]
    async fn test_run_next_iteration_hook_abort() {
        let dir = TempDir::new().unwrap();
        let config = create_test_config(true, 30);
        let state = create_test_state();

        create_hook_script(&dir, "next_iteration", "#!/bin/bash\nexit 2\n");

        let (should_continue, should_skip) = run_next_iteration_hook(&state, &config, dir.path()).await.unwrap();
        assert!(!should_continue);
        assert!(!should_skip);
    }

    #[tokio::test]
    async fn test_run_finished_hook() {
        let dir = TempDir::new().unwrap();
        let config = create_test_config(true, 30);
        let state = create_test_state();

        create_hook_script(&dir, "finished", "#!/bin/bash\nexit 0\n");

        // Finished hook always returns Ok(()) regardless of exit code
        let result = run_finished_hook(&state, &config, dir.path()).await;
        assert!(result.is_ok());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_run_hook_not_executable() {
        let dir = TempDir::new().unwrap();
        let config = create_test_config(true, 30);
        let state = create_test_state();

        // Create hooks directory
        let hooks_dir = dir.path().join(".fresher/hooks");
        fs::create_dir_all(&hooks_dir).unwrap();

        // Create a non-executable file
        let hook_path = hooks_dir.join("test_hook");
        fs::write(&hook_path, "#!/bin/bash\nexit 0\n").unwrap();

        // Don't make it executable
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&hook_path).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&hook_path, perms).unwrap();

        let result = run_hook("test_hook", &state, &config, dir.path()).await.unwrap();
        assert!(matches!(result, HookResult::NotFound));
    }
}
