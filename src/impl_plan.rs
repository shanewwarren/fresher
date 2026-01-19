//! Hierarchical implementation plan support
//!
//! This module provides types and functions for working with hierarchical
//! implementation plans stored in the `impl/` directory structure.

use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Status of a feature in the hierarchical plan
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureState {
    Pending,
    InProgress,
    Complete,
    Archived,
}

impl std::fmt::Display for FeatureState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeatureState::Pending => write!(f, "⏳ Pending"),
            FeatureState::InProgress => write!(f, "🔄 In Progress"),
            FeatureState::Complete => write!(f, "✅ Complete"),
            FeatureState::Archived => write!(f, "📦 Archived"),
        }
    }
}

/// Summary of a feature's status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureStatus {
    pub name: String,
    pub file: PathBuf,
    pub status: FeatureState,
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub pending_tasks: usize,
    pub spec_ref: Option<String>,
}

impl FeatureStatus {
    /// Calculate completion percentage
    pub fn completion_percent(&self) -> f64 {
        if self.total_tasks == 0 {
            100.0
        } else {
            (self.completed_tasks as f64 / self.total_tasks as f64) * 100.0
        }
    }
}

/// Index of all features in the hierarchical plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplIndex {
    pub impl_dir: PathBuf,
    pub features: Vec<FeatureStatus>,
    pub current_focus: Option<String>,
    pub cross_cutting_tasks: CrossCuttingTasks,
}

/// Cross-cutting tasks from impl/README.md
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CrossCuttingTasks {
    pub total: usize,
    pub completed: usize,
    pub pending: usize,
}

impl ImplIndex {
    /// Load hierarchical plan from impl directory
    pub fn load(impl_dir: &Path) -> Result<Self> {
        let readme_path = impl_dir.join("README.md");
        if !readme_path.exists() {
            anyhow::bail!("impl/README.md not found");
        }

        let mut features = Vec::new();

        // Scan for feature files
        for entry in fs::read_dir(impl_dir)? {
            let entry = entry?;
            let path = entry.path();

            // Skip non-markdown files, README, and .archive directory
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if filename == "README.md" || filename == ".archive" || !filename.ends_with(".md") {
                    continue;
                }
            } else {
                continue;
            }

            if let Ok(status) = parse_feature_file(&path) {
                features.push(status);
            }
        }

        // Sort features by name
        features.sort_by(|a, b| a.name.cmp(&b.name));

        // Parse current focus from README
        let readme_content = fs::read_to_string(&readme_path)?;
        let current_focus = parse_current_focus(&readme_content);

        // Count cross-cutting tasks in README
        let cross_cutting_tasks = count_cross_cutting_tasks(&readme_content);

        Ok(ImplIndex {
            impl_dir: impl_dir.to_path_buf(),
            features,
            current_focus,
            cross_cutting_tasks,
        })
    }

    /// Get total task count across all features
    pub fn total_tasks(&self) -> usize {
        self.features.iter().map(|f| f.total_tasks).sum::<usize>() + self.cross_cutting_tasks.total
    }

    /// Get completed task count across all features
    pub fn completed_tasks(&self) -> usize {
        self.features.iter().map(|f| f.completed_tasks).sum::<usize>()
            + self.cross_cutting_tasks.completed
    }

    /// Get pending task count across all features
    pub fn pending_tasks(&self) -> usize {
        self.features.iter().map(|f| f.pending_tasks).sum::<usize>()
            + self.cross_cutting_tasks.pending
    }

    /// Check if all tasks are complete
    pub fn is_complete(&self) -> bool {
        self.pending_tasks() == 0
    }

    /// Get the feature that should be focused next
    pub fn select_next_focus(&self) -> Option<&FeatureStatus> {
        // Priority order:
        // 1. Features with in-progress status
        // 2. Features with pending tasks (lowest task count first for quick wins)

        // First check for in-progress features
        let in_progress = self
            .features
            .iter()
            .find(|f| f.status == FeatureState::InProgress);
        if in_progress.is_some() {
            return in_progress;
        }

        // Then find feature with pending tasks (smallest first for quick wins)
        self.features
            .iter()
            .filter(|f| f.pending_tasks > 0)
            .min_by_key(|f| f.pending_tasks)
    }
}

/// Parse a feature file and extract task status
fn parse_feature_file(path: &Path) -> Result<FeatureStatus> {
    let content = fs::read_to_string(path).context("Failed to read feature file")?;

    let name = path
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let checkbox_pending = Regex::new(r"^\s*-\s*\[\s\]").unwrap();
    let checkbox_complete = Regex::new(r"^\s*-\s*\[[xX]\]").unwrap();
    let checkbox_in_progress = Regex::new(r"^\s*-\s*\[~\]").unwrap();
    let spec_ref_re = Regex::new(r"\*\*Spec:\*\*\s*\[.*?\]\((.*?)\)").unwrap();

    let mut pending = 0;
    let mut completed = 0;
    let mut in_progress = 0;
    let mut spec_ref = None;

    for line in content.lines() {
        if checkbox_pending.is_match(line) {
            pending += 1;
        } else if checkbox_complete.is_match(line) {
            completed += 1;
        } else if checkbox_in_progress.is_match(line) {
            in_progress += 1;
        }

        if spec_ref.is_none() {
            if let Some(caps) = spec_ref_re.captures(line) {
                spec_ref = caps.get(1).map(|m| m.as_str().to_string());
            }
        }
    }

    let total = pending + completed + in_progress;

    let status = if completed == total && total > 0 {
        FeatureState::Complete
    } else if completed > 0 || in_progress > 0 {
        FeatureState::InProgress
    } else {
        FeatureState::Pending
    };

    Ok(FeatureStatus {
        name,
        file: path.to_path_buf(),
        status,
        total_tasks: total,
        completed_tasks: completed,
        pending_tasks: pending + in_progress,
        spec_ref,
    })
}

/// Parse current focus from README content
fn parse_current_focus(content: &str) -> Option<String> {
    // Look for "**Active:**" pattern
    let active_re = Regex::new(r"\*\*Active:\*\*\s*\[(.*?)\]").unwrap();
    if let Some(caps) = active_re.captures(content) {
        return caps.get(1).map(|m| m.as_str().to_string());
    }

    // Alternative: look for "Current Focus" section
    let focus_re = Regex::new(r"##\s*Current Focus[\s\S]*?\[(.*?)\.md\]").unwrap();
    if let Some(caps) = focus_re.captures(content) {
        return caps.get(1).map(|m| m.as_str().to_string());
    }

    None
}

/// Count cross-cutting tasks in README
fn count_cross_cutting_tasks(content: &str) -> CrossCuttingTasks {
    let checkbox_pending = Regex::new(r"^\s*-\s*\[\s\]").unwrap();
    let checkbox_complete = Regex::new(r"^\s*-\s*\[[xX]\]").unwrap();

    let mut pending = 0;
    let mut completed = 0;

    // Find the cross-cutting section
    let in_cross_cutting = content.contains("## Cross-Cutting");
    if !in_cross_cutting {
        return CrossCuttingTasks::default();
    }

    // Simple approach: count all checkboxes in the README that aren't in feature status table
    // This is a heuristic - a more robust approach would parse sections
    for line in content.lines() {
        // Skip table rows (status overview table)
        if line.contains('|') {
            continue;
        }

        if checkbox_pending.is_match(line) {
            pending += 1;
        } else if checkbox_complete.is_match(line) {
            completed += 1;
        }
    }

    CrossCuttingTasks {
        total: pending + completed,
        completed,
        pending,
    }
}

/// Archive a completed feature file
pub fn archive_feature(impl_dir: &Path, feature_name: &str) -> Result<PathBuf> {
    let feature_path = impl_dir.join(format!("{}.md", feature_name));
    let archive_dir = impl_dir.join(".archive");
    let archive_path = archive_dir.join(format!("{}.md", feature_name));

    if !feature_path.exists() {
        anyhow::bail!("Feature file not found: {}", feature_path.display());
    }

    // Create archive directory if needed
    fs::create_dir_all(&archive_dir).context("Failed to create .archive directory")?;

    // Move the file
    fs::rename(&feature_path, &archive_path).context("Failed to archive feature file")?;

    Ok(archive_path)
}

/// Check if a hierarchical plan exists
pub fn has_hierarchical_plan(impl_dir: &Path) -> bool {
    impl_dir.join("README.md").exists()
}

/// List all feature files (excluding archived)
pub fn list_feature_files(impl_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut features = Vec::new();

    if !impl_dir.exists() {
        return Ok(features);
    }

    for entry in fs::read_dir(impl_dir)? {
        let entry = entry?;
        let path = entry.path();

        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            if filename != "README.md"
                && filename != ".archive"
                && filename.ends_with(".md")
                && path.is_file()
            {
                features.push(path);
            }
        }
    }

    features.sort();
    Ok(features)
}

/// List archived feature files
pub fn list_archived_files(impl_dir: &Path) -> Result<Vec<PathBuf>> {
    let archive_dir = impl_dir.join(".archive");
    let mut archived = Vec::new();

    if !archive_dir.exists() {
        return Ok(archived);
    }

    for entry in fs::read_dir(&archive_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            archived.push(path);
        }
    }

    archived.sort();
    Ok(archived)
}

// ============================================================================
// Migration Support
// ============================================================================

use crate::verify::{parse_plan, Task, TaskStatus};
use std::collections::HashMap;

/// Result of a migration analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationAnalysis {
    pub legacy_path: PathBuf,
    pub total_tasks: usize,
    pub tasks_by_spec: HashMap<String, Vec<Task>>,
    pub orphan_tasks: Vec<Task>,
    pub should_migrate: bool,
    pub threshold: u32,
}

/// Analyze a legacy plan file for migration
pub fn analyze_migration(
    legacy_path: &Path,
    threshold: u32,
) -> Result<MigrationAnalysis> {
    let tasks = parse_plan(legacy_path)?;

    // Group tasks by spec reference
    let mut tasks_by_spec: HashMap<String, Vec<Task>> = HashMap::new();
    let mut orphan_tasks = Vec::new();

    for task in tasks.iter() {
        if task.spec_refs.is_empty() {
            orphan_tasks.push(task.clone());
        } else {
            // Use first spec ref for grouping
            let spec_name = extract_spec_name(&task.spec_refs[0]);
            tasks_by_spec
                .entry(spec_name)
                .or_default()
                .push(task.clone());
        }
    }

    let total_tasks = tasks.len();
    let should_migrate = total_tasks >= threshold as usize;

    Ok(MigrationAnalysis {
        legacy_path: legacy_path.to_path_buf(),
        total_tasks,
        tasks_by_spec,
        orphan_tasks,
        should_migrate,
        threshold,
    })
}

/// Extract spec name from spec reference path
fn extract_spec_name(spec_ref: &str) -> String {
    spec_ref
        .trim_start_matches("specs/")
        .trim_end_matches(".md")
        .replace('/', "-")
        .to_string()
}

/// Migrate a legacy plan to hierarchical structure
pub fn migrate_plan(
    legacy_path: &Path,
    impl_dir: &Path,
    _threshold: u32,
) -> Result<MigrationResult> {
    let analysis = analyze_migration(legacy_path, 0)?; // Use 0 to force migration

    // Create impl directory structure
    fs::create_dir_all(impl_dir)?;
    fs::create_dir_all(impl_dir.join(".archive"))?;

    let mut created_files = Vec::new();

    // Create feature files
    for (spec_name, tasks) in &analysis.tasks_by_spec {
        let feature_path = impl_dir.join(format!("{}.md", spec_name));
        let content = generate_feature_file(spec_name, tasks);
        fs::write(&feature_path, content)?;
        created_files.push(feature_path);
    }

    // Create README.md index
    let readme_path = impl_dir.join("README.md");
    let readme_content =
        generate_readme(&analysis.tasks_by_spec, &analysis.orphan_tasks);
    fs::write(&readme_path, readme_content)?;
    created_files.push(readme_path);

    // Backup legacy file
    let backup_path = legacy_path.with_extension("md.backup");
    fs::rename(legacy_path, &backup_path)?;

    Ok(MigrationResult {
        impl_dir: impl_dir.to_path_buf(),
        backup_path,
        created_files,
        feature_count: analysis.tasks_by_spec.len(),
        task_count: analysis.total_tasks,
        orphan_count: analysis.orphan_tasks.len(),
    })
}

/// Result of a successful migration
#[derive(Debug, Clone)]
pub struct MigrationResult {
    pub impl_dir: PathBuf,
    pub backup_path: PathBuf,
    pub created_files: Vec<PathBuf>,
    pub feature_count: usize,
    pub task_count: usize,
    pub orphan_count: usize,
}

/// Generate a feature file content
fn generate_feature_file(spec_name: &str, tasks: &[Task]) -> String {
    let timestamp = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let title = spec_name
        .split('-')
        .map(|s| {
            let mut c = s.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    let mut content = format!(
        r#"# {} Implementation

**Spec:** [specs/{}.md](../specs/{}.md)
**Status:** Pending
**Last Updated:** {}

---

## Dependencies

- ⏳ None blocking

---

## Tasks

"#,
        title, spec_name, spec_name, timestamp
    );

    // Group tasks by priority
    let mut by_priority: HashMap<Option<u32>, Vec<&Task>> = HashMap::new();
    for task in tasks {
        by_priority.entry(task.priority).or_default().push(task);
    }

    // Sort priorities
    let mut priorities: Vec<_> = by_priority.keys().copied().collect();
    priorities.sort_by(|a, b| {
        match (a, b) {
            (Some(x), Some(y)) => x.cmp(y),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });

    for priority in priorities {
        let tasks_in_priority = &by_priority[&priority];
        let priority_label = priority
            .map(|p| format!("Priority {}", p))
            .unwrap_or_else(|| "Uncategorized".to_string());

        content.push_str(&format!("### {}\n\n", priority_label));

        for (idx, task) in tasks_in_priority.iter().enumerate() {
            let task_id = format!(
                "P{}.{}",
                priority.unwrap_or(99),
                idx + 1
            );
            let checkbox = match task.status {
                TaskStatus::Pending => "[ ]",
                TaskStatus::Completed => "[x]",
                TaskStatus::InProgress => "[~]",
            };

            content.push_str(&format!(
                "#### {}: {}\n\n",
                task_id,
                task.description.split('(').next().unwrap_or(&task.description).trim()
            ));
            content.push_str(&format!("- {} {}\n", checkbox, task.description));

            if let Some(complexity) = &task.complexity {
                content.push_str(&format!("  - **Complexity:** {}\n", complexity));
            }

            if !task.dependencies.is_empty() {
                content.push_str(&format!(
                    "  - **Dependencies:** {}\n",
                    task.dependencies.join(", ")
                ));
            }

            content.push('\n');
        }
    }

    content
}

/// Generate README.md index content
fn generate_readme(
    tasks_by_spec: &HashMap<String, Vec<Task>>,
    orphan_tasks: &[Task],
) -> String {
    let timestamp = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let mut content = format!(
        r#"# Implementation Plan

**Generated:** {}
**Based on:** specs/*.md
**Project:** (migrated from IMPLEMENTATION_PLAN.md)

---

## Status Overview

| Feature | Status | Progress | Spec |
|---------|--------|----------|------|
"#,
        timestamp
    );

    // Sort features alphabetically
    let mut features: Vec<_> = tasks_by_spec.keys().collect();
    features.sort();

    for spec_name in &features {
        let tasks = &tasks_by_spec[*spec_name];
        let completed = tasks.iter().filter(|t| t.status == TaskStatus::Completed).count();
        let total = tasks.len();
        let status = if completed == total && total > 0 {
            "✅ Complete"
        } else if completed > 0 {
            "🔄 In Progress"
        } else {
            "⏳ Pending"
        };

        content.push_str(&format!(
            "| [{}](./{}.md) | {} | {}/{} | [spec](../specs/{}.md) |\n",
            spec_name, spec_name, status, completed, total, spec_name
        ));
    }

    // Current focus - first feature with pending tasks
    let current_focus = features
        .iter()
        .find(|name| {
            tasks_by_spec[**name]
                .iter()
                .any(|t| t.status == TaskStatus::Pending)
        })
        .map(|s| s.to_string());

    content.push_str(&format!(
        r#"
---

## Current Focus

"#
    ));

    if let Some(focus) = &current_focus {
        content.push_str(&format!("**Active:** [{}.md](./{}.md)\n\n", focus, focus));
    } else {
        content.push_str("**Active:** None (all features complete or empty)\n\n");
    }

    // Cross-cutting tasks (orphans)
    if !orphan_tasks.is_empty() {
        content.push_str(
            r#"---

## Cross-Cutting Tasks

Tasks not tied to a specific feature:

"#,
        );

        for task in orphan_tasks {
            let checkbox = match task.status {
                TaskStatus::Pending => "[ ]",
                TaskStatus::Completed => "[x]",
                TaskStatus::InProgress => "[~]",
            };
            content.push_str(&format!("- {} {}\n", checkbox, task.description));
        }
    }

    content.push_str(
        r#"
---

## Archived Features

Completed features moved to `.archive/`:

(none yet)
"#,
    );

    content
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_impl_dir() -> TempDir {
        let dir = TempDir::new().unwrap();
        let impl_dir = dir.path().join("impl");
        fs::create_dir_all(&impl_dir).unwrap();
        fs::create_dir_all(impl_dir.join(".archive")).unwrap();
        dir
    }

    fn write_file(dir: &Path, name: &str, content: &str) {
        let path = dir.join(name);
        let mut file = fs::File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn test_parse_feature_file_pending() {
        let dir = create_test_impl_dir();
        let impl_dir = dir.path().join("impl");

        write_file(
            &impl_dir,
            "auth.md",
            r#"# Auth Implementation

**Spec:** [specs/auth.md](../specs/auth.md)

## Tasks

- [ ] Implement login
- [ ] Add logout
"#,
        );

        let status = parse_feature_file(&impl_dir.join("auth.md")).unwrap();

        assert_eq!(status.name, "auth");
        assert_eq!(status.status, FeatureState::Pending);
        assert_eq!(status.total_tasks, 2);
        assert_eq!(status.completed_tasks, 0);
        assert_eq!(status.pending_tasks, 2);
        assert_eq!(status.spec_ref, Some("../specs/auth.md".to_string()));
    }

    #[test]
    fn test_parse_feature_file_in_progress() {
        let dir = create_test_impl_dir();
        let impl_dir = dir.path().join("impl");

        write_file(
            &impl_dir,
            "auth.md",
            r#"# Auth Implementation

- [x] Implement login
- [ ] Add logout
"#,
        );

        let status = parse_feature_file(&impl_dir.join("auth.md")).unwrap();

        assert_eq!(status.status, FeatureState::InProgress);
        assert_eq!(status.completed_tasks, 1);
        assert_eq!(status.pending_tasks, 1);
    }

    #[test]
    fn test_parse_feature_file_complete() {
        let dir = create_test_impl_dir();
        let impl_dir = dir.path().join("impl");

        write_file(
            &impl_dir,
            "auth.md",
            r#"# Auth Implementation

- [x] Implement login
- [X] Add logout
"#,
        );

        let status = parse_feature_file(&impl_dir.join("auth.md")).unwrap();

        assert_eq!(status.status, FeatureState::Complete);
        assert_eq!(status.total_tasks, 2);
        assert_eq!(status.completed_tasks, 2);
        assert_eq!(status.pending_tasks, 0);
    }

    #[test]
    fn test_parse_current_focus() {
        let content = r#"## Current Focus

**Active:** [auth.md](./auth.md)
"#;

        assert_eq!(parse_current_focus(content), Some("auth.md".to_string()));
    }

    #[test]
    fn test_count_cross_cutting_tasks() {
        let content = r#"## Status Overview

| Feature | Status |
|---------|--------|
| auth | ⏳ |

## Cross-Cutting Tasks

- [ ] Update README
- [x] Add CI/CD
- [ ] Write docs
"#;

        let tasks = count_cross_cutting_tasks(content);
        assert_eq!(tasks.total, 3);
        assert_eq!(tasks.completed, 1);
        assert_eq!(tasks.pending, 2);
    }

    #[test]
    fn test_impl_index_load() {
        let dir = create_test_impl_dir();
        let impl_dir = dir.path().join("impl");

        write_file(
            &impl_dir,
            "README.md",
            r#"# Implementation Plan

## Current Focus

**Active:** [auth.md](./auth.md)

## Cross-Cutting Tasks

- [ ] Global task
"#,
        );

        write_file(
            &impl_dir,
            "auth.md",
            r#"# Auth

- [ ] Task 1
- [x] Task 2
"#,
        );

        write_file(
            &impl_dir,
            "api.md",
            r#"# API

- [ ] Task A
"#,
        );

        let index = ImplIndex::load(&impl_dir).unwrap();

        assert_eq!(index.features.len(), 2);
        assert_eq!(index.current_focus, Some("auth.md".to_string()));
        assert_eq!(index.total_tasks(), 4); // 2 + 1 + 1 cross-cutting
        assert_eq!(index.completed_tasks(), 1);
        assert_eq!(index.pending_tasks(), 3);
    }

    #[test]
    fn test_select_next_focus() {
        let dir = create_test_impl_dir();
        let impl_dir = dir.path().join("impl");

        write_file(&impl_dir, "README.md", "# Plan");

        write_file(
            &impl_dir,
            "big.md",
            r#"
- [ ] Task 1
- [ ] Task 2
- [ ] Task 3
"#,
        );

        write_file(
            &impl_dir,
            "small.md",
            r#"
- [ ] Task A
"#,
        );

        let index = ImplIndex::load(&impl_dir).unwrap();
        let focus = index.select_next_focus().unwrap();

        // Should select small.md (fewer pending tasks)
        assert_eq!(focus.name, "small");
    }

    #[test]
    fn test_archive_feature() {
        let dir = create_test_impl_dir();
        let impl_dir = dir.path().join("impl");

        write_file(&impl_dir, "done.md", "# Done\n\n- [x] Complete");

        let archive_path = archive_feature(&impl_dir, "done").unwrap();

        assert!(archive_path.exists());
        assert!(!impl_dir.join("done.md").exists());
        assert_eq!(archive_path, impl_dir.join(".archive/done.md"));
    }

    #[test]
    fn test_list_feature_files() {
        let dir = create_test_impl_dir();
        let impl_dir = dir.path().join("impl");

        write_file(&impl_dir, "README.md", "# Index");
        write_file(&impl_dir, "auth.md", "# Auth");
        write_file(&impl_dir, "api.md", "# API");
        write_file(&impl_dir.join(".archive"), "old.md", "# Old");

        let features = list_feature_files(&impl_dir).unwrap();

        assert_eq!(features.len(), 2);
        assert!(features.iter().any(|p| p.ends_with("api.md")));
        assert!(features.iter().any(|p| p.ends_with("auth.md")));
    }

    #[test]
    fn test_feature_status_completion_percent() {
        let status = FeatureStatus {
            name: "test".to_string(),
            file: PathBuf::from("test.md"),
            status: FeatureState::InProgress,
            total_tasks: 4,
            completed_tasks: 1,
            pending_tasks: 3,
            spec_ref: None,
        };

        assert_eq!(status.completion_percent(), 25.0);
    }

    #[test]
    fn test_has_hierarchical_plan() {
        let dir = create_test_impl_dir();
        let impl_dir = dir.path().join("impl");

        write_file(&impl_dir, "README.md", "# Plan");

        assert!(has_hierarchical_plan(&impl_dir));
        assert!(!has_hierarchical_plan(dir.path()));
    }

    // Migration tests

    #[test]
    fn test_analyze_migration() {
        let dir = TempDir::new().unwrap();
        let plan_path = dir.path().join("IMPLEMENTATION_PLAN.md");

        let content = r#"## Priority 1: Core

- [ ] Task A (refs: specs/auth.md)
- [x] Task B (refs: specs/auth.md)
- [ ] Task C (refs: specs/api.md)
- [ ] Orphan task without ref
"#;
        fs::write(&plan_path, content).unwrap();

        let analysis = analyze_migration(&plan_path, 3).unwrap();

        assert_eq!(analysis.total_tasks, 4);
        assert_eq!(analysis.tasks_by_spec.len(), 2); // auth and api
        assert_eq!(analysis.orphan_tasks.len(), 1);
        assert!(analysis.should_migrate); // 4 >= 3
    }

    #[test]
    fn test_analyze_migration_below_threshold() {
        let dir = TempDir::new().unwrap();
        let plan_path = dir.path().join("IMPLEMENTATION_PLAN.md");

        let content = r#"- [ ] Task A (refs: specs/auth.md)
- [ ] Task B (refs: specs/auth.md)
"#;
        fs::write(&plan_path, content).unwrap();

        let analysis = analyze_migration(&plan_path, 8).unwrap();

        assert!(!analysis.should_migrate); // 2 < 8
    }

    #[test]
    fn test_migrate_plan() {
        let dir = TempDir::new().unwrap();
        let plan_path = dir.path().join("IMPLEMENTATION_PLAN.md");
        let impl_dir = dir.path().join("impl");

        let content = r#"## Priority 1: Core

- [ ] Task A (refs: specs/auth.md)
- [x] Task B (refs: specs/auth.md)
- [ ] Task C (refs: specs/api.md)
"#;
        fs::write(&plan_path, content).unwrap();

        let result = migrate_plan(&plan_path, &impl_dir, 0).unwrap();

        // Check migration result
        assert_eq!(result.feature_count, 2);
        assert_eq!(result.task_count, 3);

        // Check files were created
        assert!(impl_dir.join("README.md").exists());
        assert!(impl_dir.join("auth.md").exists());
        assert!(impl_dir.join("api.md").exists());
        assert!(impl_dir.join(".archive").exists());

        // Check backup was created
        assert!(result.backup_path.exists());
        assert!(!plan_path.exists());
    }

    #[test]
    fn test_extract_spec_name() {
        assert_eq!(extract_spec_name("specs/auth.md"), "auth");
        assert_eq!(extract_spec_name("specs/api-gateway.md"), "api-gateway");
        assert_eq!(extract_spec_name("auth.md"), "auth");
    }
}
