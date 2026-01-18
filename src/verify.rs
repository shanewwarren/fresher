use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// A task extracted from the implementation plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub description: String,
    pub status: TaskStatus,
    pub spec_refs: Vec<String>,
    pub line_number: usize,
    pub priority: Option<u32>,
    pub dependencies: Vec<String>,
    pub complexity: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Completed,
    InProgress,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "pending"),
            TaskStatus::Completed => write!(f, "completed"),
            TaskStatus::InProgress => write!(f, "in_progress"),
        }
    }
}

/// A requirement extracted from specifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirement {
    pub spec_name: String,
    pub req_type: RequirementType,
    pub text: String,
    pub line_number: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequirementType {
    Section,
    Task,
    Rfc2119,
}

/// Verification report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyReport {
    pub total_tasks: usize,
    pub pending_tasks: usize,
    pub completed_tasks: usize,
    pub in_progress_tasks: usize,
    pub tasks_with_refs: usize,
    pub orphan_tasks: usize,
    pub coverage: Vec<CoverageEntry>,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageEntry {
    pub spec_name: String,
    pub requirement_count: usize,
    pub task_count: usize,
    pub coverage_percent: f64,
}

/// Parse implementation plan and extract tasks
pub fn parse_plan(plan_path: &Path) -> Result<Vec<Task>> {
    let content = fs::read_to_string(plan_path)
        .with_context(|| format!("Failed to read {}", plan_path.display()))?;

    let mut tasks = Vec::new();
    let mut current_priority: Option<u32> = None;

    // Regex patterns
    let priority_re = Regex::new(r"^##\s+Priority\s+(\d+)")?;
    let checkbox_re = Regex::new(r"^(\s*)-\s*\[([ xX~])\]\s+(.+)$")?;
    let refs_re = Regex::new(r"\(refs?:\s*([^)]+)\)")?;
    let deps_re = Regex::new(r"Dependencies:\s*(.+)")?;
    let complexity_re = Regex::new(r"Complexity:\s*(low|medium|high)")?;

    for (line_num, line) in content.lines().enumerate() {
        // Check for priority section
        if let Some(caps) = priority_re.captures(line) {
            current_priority = caps.get(1).and_then(|m| m.as_str().parse().ok());
            continue;
        }

        // Check for task checkbox
        if let Some(caps) = checkbox_re.captures(line) {
            let checkbox_char = caps.get(2).map(|m| m.as_str()).unwrap_or(" ");
            let description = caps.get(3).map(|m| m.as_str()).unwrap_or("").to_string();

            let status = match checkbox_char {
                " " => TaskStatus::Pending,
                "x" | "X" => TaskStatus::Completed,
                "~" => TaskStatus::InProgress,
                _ => TaskStatus::Pending,
            };

            // Extract spec references
            let spec_refs: Vec<String> = refs_re
                .captures(&description)
                .map(|c| {
                    c.get(1)
                        .map(|m| {
                            m.as_str()
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .collect()
                        })
                        .unwrap_or_default()
                })
                .unwrap_or_default();

            // Clean description (remove refs)
            let clean_desc = refs_re.replace_all(&description, "").trim().to_string();

            tasks.push(Task {
                description: clean_desc,
                status,
                spec_refs,
                line_number: line_num + 1,
                priority: current_priority,
                dependencies: Vec::new(),
                complexity: None,
            });
        }

        // Check for dependencies on the next line after a task
        if let Some(last_task) = tasks.last_mut() {
            if let Some(caps) = deps_re.captures(line) {
                let deps = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                if deps.to_lowercase() != "none" {
                    last_task.dependencies = deps
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            }
            if let Some(caps) = complexity_re.captures(line) {
                last_task.complexity = caps.get(1).map(|m| m.as_str().to_string());
            }
        }
    }

    Ok(tasks)
}

/// Count tasks by status
pub fn count_tasks(tasks: &[Task]) -> (usize, usize, usize, usize) {
    let total = tasks.len();
    let pending = tasks.iter().filter(|t| t.status == TaskStatus::Pending).count();
    let completed = tasks.iter().filter(|t| t.status == TaskStatus::Completed).count();
    let in_progress = tasks.iter().filter(|t| t.status == TaskStatus::InProgress).count();
    (total, pending, completed, in_progress)
}

/// Check if there are pending tasks
pub fn has_pending_tasks(plan_path: &Path) -> bool {
    if !plan_path.exists() {
        return false;
    }

    match fs::read_to_string(plan_path) {
        Ok(content) => {
            let checkbox_re = Regex::new(r"^\s*-\s*\[\s\]").unwrap();
            content.lines().any(|line| checkbox_re.is_match(line))
        }
        Err(_) => false,
    }
}

/// Extract requirements from specification files
pub fn extract_requirements(spec_dir: &Path) -> Result<Vec<Requirement>> {
    let mut requirements = Vec::new();

    if !spec_dir.exists() {
        return Ok(requirements);
    }

    // RFC 2119 keywords
    let rfc2119_re = Regex::new(
        r"\b(MUST|MUST NOT|REQUIRED|SHALL|SHALL NOT|SHOULD|SHOULD NOT|RECOMMENDED|MAY|OPTIONAL)\b"
    )?;
    let section_re = Regex::new(r"^###\s+(.+)$")?;
    let checkbox_re = Regex::new(r"^(\s*)-\s*\[([ xX])\]\s+(.+)$")?;

    for entry in fs::read_dir(spec_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "md").unwrap_or(false) {
            let spec_name = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            let content = fs::read_to_string(&path)?;

            for (line_num, line) in content.lines().enumerate() {
                // Check for section headers
                if let Some(caps) = section_re.captures(line) {
                    requirements.push(Requirement {
                        spec_name: spec_name.clone(),
                        req_type: RequirementType::Section,
                        text: caps.get(1).map(|m| m.as_str()).unwrap_or("").to_string(),
                        line_number: line_num + 1,
                    });
                }

                // Check for checkboxes in specs
                if let Some(caps) = checkbox_re.captures(line) {
                    requirements.push(Requirement {
                        spec_name: spec_name.clone(),
                        req_type: RequirementType::Task,
                        text: caps.get(3).map(|m| m.as_str()).unwrap_or("").to_string(),
                        line_number: line_num + 1,
                    });
                }

                // Check for RFC 2119 keywords
                if rfc2119_re.is_match(line) {
                    requirements.push(Requirement {
                        spec_name: spec_name.clone(),
                        req_type: RequirementType::Rfc2119,
                        text: line.to_string(),
                        line_number: line_num + 1,
                    });
                }
            }
        }
    }

    Ok(requirements)
}

/// Analyze coverage of specs by tasks
pub fn analyze_coverage(
    spec_dir: &Path,
    tasks: &[Task],
) -> Result<Vec<CoverageEntry>> {
    let requirements = extract_requirements(spec_dir)?;

    // Group requirements by spec
    let mut spec_reqs: HashMap<String, Vec<&Requirement>> = HashMap::new();
    for req in &requirements {
        spec_reqs
            .entry(req.spec_name.clone())
            .or_default()
            .push(req);
    }

    // Count tasks referencing each spec
    let mut spec_tasks: HashMap<String, usize> = HashMap::new();
    for task in tasks {
        for spec_ref in &task.spec_refs {
            // Extract spec name from path (e.g., "specs/foo.md" -> "foo")
            let spec_name = spec_ref
                .trim_start_matches("specs/")
                .trim_end_matches(".md")
                .to_string();
            *spec_tasks.entry(spec_name).or_default() += 1;
        }
    }

    // Build coverage entries
    let mut coverage = Vec::new();
    for (spec_name, reqs) in spec_reqs {
        let req_count = reqs.len();
        let task_count = spec_tasks.get(&spec_name).copied().unwrap_or(0);
        let coverage_percent = if req_count > 0 {
            (task_count as f64 / req_count as f64 * 100.0).min(100.0)
        } else {
            0.0
        };

        coverage.push(CoverageEntry {
            spec_name,
            requirement_count: req_count,
            task_count,
            coverage_percent,
        });
    }

    coverage.sort_by(|a, b| a.spec_name.cmp(&b.spec_name));
    Ok(coverage)
}

/// Generate a full verification report
pub fn generate_report(plan_path: &Path, spec_dir: &Path) -> Result<VerifyReport> {
    let tasks = parse_plan(plan_path)?;
    let (total, pending, completed, in_progress) = count_tasks(&tasks);

    let tasks_with_refs = tasks.iter().filter(|t| !t.spec_refs.is_empty()).count();
    let orphan_tasks = total - tasks_with_refs;

    let coverage = analyze_coverage(spec_dir, &tasks)?;

    Ok(VerifyReport {
        total_tasks: total,
        pending_tasks: pending,
        completed_tasks: completed,
        in_progress_tasks: in_progress,
        tasks_with_refs,
        orphan_tasks,
        coverage,
        tasks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_temp_plan(content: &str) -> (TempDir, std::path::PathBuf) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("IMPLEMENTATION_PLAN.md");
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        (dir, path)
    }

    fn create_temp_spec(dir: &TempDir, name: &str, content: &str) {
        let specs_dir = dir.path().join("specs");
        std::fs::create_dir_all(&specs_dir).unwrap();
        let path = specs_dir.join(name);
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn test_parse_plan_empty() {
        let (_dir, path) = create_temp_plan("");
        let tasks = parse_plan(&path).unwrap();
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_parse_plan_pending_task() {
        let content = "- [ ] Implement feature X";
        let (_dir, path) = create_temp_plan(content);
        let tasks = parse_plan(&path).unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].status, TaskStatus::Pending);
        assert_eq!(tasks[0].description, "Implement feature X");
    }

    #[test]
    fn test_parse_plan_completed_task() {
        let content = "- [x] Completed task";
        let (_dir, path) = create_temp_plan(content);
        let tasks = parse_plan(&path).unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].status, TaskStatus::Completed);
    }

    #[test]
    fn test_parse_plan_completed_task_uppercase() {
        let content = "- [X] Completed task uppercase";
        let (_dir, path) = create_temp_plan(content);
        let tasks = parse_plan(&path).unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].status, TaskStatus::Completed);
    }

    #[test]
    fn test_parse_plan_in_progress_task() {
        let content = "- [~] In progress task";
        let (_dir, path) = create_temp_plan(content);
        let tasks = parse_plan(&path).unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].status, TaskStatus::InProgress);
    }

    #[test]
    fn test_parse_plan_with_spec_refs() {
        let content = "- [ ] Task with refs (refs: specs/foo.md)";
        let (_dir, path) = create_temp_plan(content);
        let tasks = parse_plan(&path).unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].spec_refs, vec!["specs/foo.md"]);
        assert_eq!(tasks[0].description, "Task with refs");
    }

    #[test]
    fn test_parse_plan_with_multiple_refs() {
        let content = "- [ ] Multi refs (refs: specs/a.md, specs/b.md)";
        let (_dir, path) = create_temp_plan(content);
        let tasks = parse_plan(&path).unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].spec_refs.len(), 2);
        assert!(tasks[0].spec_refs.contains(&"specs/a.md".to_string()));
        assert!(tasks[0].spec_refs.contains(&"specs/b.md".to_string()));
    }

    #[test]
    fn test_parse_plan_with_priority() {
        let content = r#"## Priority 3: Something

- [ ] Task in priority 3
"#;
        let (_dir, path) = create_temp_plan(content);
        let tasks = parse_plan(&path).unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].priority, Some(3));
    }

    #[test]
    fn test_parse_plan_with_dependencies() {
        let content = r#"- [ ] Task with deps
  - Dependencies: Module A, Module B
"#;
        let (_dir, path) = create_temp_plan(content);
        let tasks = parse_plan(&path).unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].dependencies.len(), 2);
        assert!(tasks[0].dependencies.contains(&"Module A".to_string()));
        assert!(tasks[0].dependencies.contains(&"Module B".to_string()));
    }

    #[test]
    fn test_parse_plan_with_complexity() {
        let content = r#"- [ ] Complex task
  - Complexity: medium
"#;
        let (_dir, path) = create_temp_plan(content);
        let tasks = parse_plan(&path).unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].complexity, Some("medium".to_string()));
    }

    #[test]
    fn test_parse_plan_dependencies_none() {
        let content = r#"- [ ] Independent task
  - Dependencies: none
"#;
        let (_dir, path) = create_temp_plan(content);
        let tasks = parse_plan(&path).unwrap();

        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].dependencies.is_empty());
    }

    #[test]
    fn test_count_tasks() {
        let tasks = vec![
            Task {
                description: "Task 1".to_string(),
                status: TaskStatus::Pending,
                spec_refs: vec![],
                line_number: 1,
                priority: None,
                dependencies: vec![],
                complexity: None,
            },
            Task {
                description: "Task 2".to_string(),
                status: TaskStatus::Completed,
                spec_refs: vec![],
                line_number: 2,
                priority: None,
                dependencies: vec![],
                complexity: None,
            },
            Task {
                description: "Task 3".to_string(),
                status: TaskStatus::InProgress,
                spec_refs: vec![],
                line_number: 3,
                priority: None,
                dependencies: vec![],
                complexity: None,
            },
        ];

        let (total, pending, completed, in_progress) = count_tasks(&tasks);

        assert_eq!(total, 3);
        assert_eq!(pending, 1);
        assert_eq!(completed, 1);
        assert_eq!(in_progress, 1);
    }

    #[test]
    fn test_has_pending_tasks_true() {
        let content = "- [ ] Pending task";
        let (_dir, path) = create_temp_plan(content);

        assert!(has_pending_tasks(&path));
    }

    #[test]
    fn test_has_pending_tasks_false() {
        let content = "- [x] Completed task";
        let (_dir, path) = create_temp_plan(content);

        assert!(!has_pending_tasks(&path));
    }

    #[test]
    fn test_has_pending_tasks_missing_file() {
        let path = Path::new("/nonexistent/path/plan.md");
        assert!(!has_pending_tasks(path));
    }

    #[test]
    fn test_extract_requirements_empty_dir() {
        let dir = TempDir::new().unwrap();
        let specs_dir = dir.path().join("specs");
        std::fs::create_dir_all(&specs_dir).unwrap();

        let reqs = extract_requirements(&specs_dir).unwrap();
        assert!(reqs.is_empty());
    }

    #[test]
    fn test_extract_requirements_section_header() {
        let dir = TempDir::new().unwrap();
        create_temp_spec(&dir, "feature.md", "### Feature Details\n\nSome content.");

        let specs_dir = dir.path().join("specs");
        let reqs = extract_requirements(&specs_dir).unwrap();

        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].req_type, RequirementType::Section);
        assert_eq!(reqs[0].text, "Feature Details");
        assert_eq!(reqs[0].spec_name, "feature");
    }

    #[test]
    fn test_extract_requirements_rfc2119() {
        let dir = TempDir::new().unwrap();
        create_temp_spec(&dir, "spec.md", "The system MUST validate input.\nIt SHOULD log errors.");

        let specs_dir = dir.path().join("specs");
        let reqs = extract_requirements(&specs_dir).unwrap();

        let rfc_reqs: Vec<_> = reqs.iter().filter(|r| r.req_type == RequirementType::Rfc2119).collect();
        assert_eq!(rfc_reqs.len(), 2);
    }

    #[test]
    fn test_extract_requirements_checkbox() {
        let dir = TempDir::new().unwrap();
        create_temp_spec(&dir, "spec.md", "- [ ] Pending spec task\n- [x] Done spec task");

        let specs_dir = dir.path().join("specs");
        let reqs = extract_requirements(&specs_dir).unwrap();

        let task_reqs: Vec<_> = reqs.iter().filter(|r| r.req_type == RequirementType::Task).collect();
        assert_eq!(task_reqs.len(), 2);
    }

    #[test]
    fn test_analyze_coverage_empty() {
        let dir = TempDir::new().unwrap();
        let specs_dir = dir.path().join("specs");
        std::fs::create_dir_all(&specs_dir).unwrap();

        let tasks: Vec<Task> = vec![];
        let coverage = analyze_coverage(&specs_dir, &tasks).unwrap();

        assert!(coverage.is_empty());
    }

    #[test]
    fn test_analyze_coverage_with_refs() {
        let dir = TempDir::new().unwrap();
        create_temp_spec(&dir, "feature.md", "### Section 1\n### Section 2\n");

        let tasks = vec![
            Task {
                description: "Task 1".to_string(),
                status: TaskStatus::Pending,
                spec_refs: vec!["specs/feature.md".to_string()],
                line_number: 1,
                priority: None,
                dependencies: vec![],
                complexity: None,
            },
            Task {
                description: "Task 2".to_string(),
                status: TaskStatus::Pending,
                spec_refs: vec!["specs/feature.md".to_string()],
                line_number: 2,
                priority: None,
                dependencies: vec![],
                complexity: None,
            },
        ];

        let specs_dir = dir.path().join("specs");
        let coverage = analyze_coverage(&specs_dir, &tasks).unwrap();

        assert_eq!(coverage.len(), 1);
        assert_eq!(coverage[0].spec_name, "feature");
        assert_eq!(coverage[0].task_count, 2);
        assert_eq!(coverage[0].requirement_count, 2); // 2 sections
    }

    #[test]
    fn test_generate_report() {
        let dir = TempDir::new().unwrap();

        // Create plan
        let plan_content = r#"## Priority 1: Test

- [x] Completed task (refs: specs/feature.md)
- [ ] Pending task
"#;
        let plan_path = dir.path().join("plan.md");
        std::fs::write(&plan_path, plan_content).unwrap();

        // Create spec
        create_temp_spec(&dir, "feature.md", "### Feature Section\n");

        let specs_dir = dir.path().join("specs");
        let report = generate_report(&plan_path, &specs_dir).unwrap();

        assert_eq!(report.total_tasks, 2);
        assert_eq!(report.completed_tasks, 1);
        assert_eq!(report.pending_tasks, 1);
        assert_eq!(report.tasks_with_refs, 1);
        assert_eq!(report.orphan_tasks, 1);
    }

    #[test]
    fn test_task_status_display() {
        assert_eq!(TaskStatus::Pending.to_string(), "pending");
        assert_eq!(TaskStatus::Completed.to_string(), "completed");
        assert_eq!(TaskStatus::InProgress.to_string(), "in_progress");
    }

    #[test]
    fn test_parse_plan_line_numbers() {
        let content = r#"Line 1
Line 2
- [ ] Task on line 3
Line 4
- [x] Task on line 5
"#;
        let (_dir, path) = create_temp_plan(content);
        let tasks = parse_plan(&path).unwrap();

        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].line_number, 3);
        assert_eq!(tasks[1].line_number, 5);
    }
}
