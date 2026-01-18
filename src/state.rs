use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Represents the current state of a fresher run
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishType {
    Manual,
    Error,
    MaxIterations,
    Complete,
    NoChanges,
}

impl std::fmt::Display for FinishType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FinishType::Manual => write!(f, "manual"),
            FinishType::Error => write!(f, "error"),
            FinishType::MaxIterations => write!(f, "max_iterations"),
            FinishType::Complete => write!(f, "complete"),
            FinishType::NoChanges => write!(f, "no_changes"),
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            iteration: 0,
            last_exit_code: 0,
            last_commit_sha: None,
            started_at: Utc::now(),
            total_commits: 0,
            duration: 0,
            finish_type: None,
            iteration_start: None,
            iteration_sha: None,
        }
    }
}

impl State {
    /// Create a new state
    pub fn new() -> Self {
        Self::default()
    }

    /// Load state from .fresher/.state file
    pub fn load() -> Result<Option<Self>> {
        let path = Path::new(".fresher/.state");
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(path).context("Failed to read .fresher/.state")?;
        let state: State = toml::from_str(&content).context("Failed to parse .fresher/.state")?;
        Ok(Some(state))
    }

    /// Save state to .fresher/.state file
    pub fn save(&self) -> Result<()> {
        let path = Path::new(".fresher/.state");
        let content = toml::to_string_pretty(self).context("Failed to serialize state")?;
        fs::write(path, content).context("Failed to write .fresher/.state")?;
        Ok(())
    }

    /// Start a new iteration
    pub fn start_iteration(&mut self, commit_sha: Option<String>) {
        self.iteration += 1;
        self.iteration_start = Some(Utc::now());
        self.iteration_sha = commit_sha;
    }

    /// Record iteration completion
    pub fn complete_iteration(&mut self, exit_code: i32, commits: u32) {
        self.last_exit_code = exit_code;
        self.total_commits += commits;
        if commits > 0 {
            self.last_commit_sha = get_current_sha();
        }
        self.update_duration();
    }

    /// Update the total duration
    pub fn update_duration(&mut self) {
        self.duration = (Utc::now() - self.started_at).num_seconds() as u64;
    }

    /// Set the finish type
    pub fn set_finish(&mut self, finish_type: FinishType) {
        self.finish_type = Some(finish_type);
        self.update_duration();
    }

    /// Get environment variables for hooks
    pub fn to_env_vars(&self) -> Vec<(String, String)> {
        let mut vars = vec![
            ("FRESHER_ITERATION".to_string(), self.iteration.to_string()),
            ("FRESHER_LAST_EXIT_CODE".to_string(), self.last_exit_code.to_string()),
            ("FRESHER_TOTAL_COMMITS".to_string(), self.total_commits.to_string()),
            ("FRESHER_DURATION".to_string(), self.duration.to_string()),
            ("FRESHER_TOTAL_ITERATIONS".to_string(), self.iteration.to_string()),
        ];

        if let Some(sha) = &self.last_commit_sha {
            vars.push(("FRESHER_LAST_COMMIT_SHA".to_string(), sha.clone()));
        }

        if let Some(finish) = &self.finish_type {
            vars.push(("FRESHER_FINISH_TYPE".to_string(), finish.to_string()));
        }

        vars
    }
}

/// Get current git SHA
pub fn get_current_sha() -> Option<String> {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}

/// Count commits since a given SHA
pub fn count_commits_since(sha: &str) -> u32 {
    std::process::Command::new("git")
        .args(["rev-list", "--count", &format!("{}..HEAD", sha)])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .and_then(|s| s.trim().parse().ok())
            } else {
                None
            }
        })
        .unwrap_or(0)
}
