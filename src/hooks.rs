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
