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
