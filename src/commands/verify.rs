use anyhow::Result;
use colored::*;
use std::path::Path;

use crate::config::Config;
use crate::impl_plan::{has_hierarchical_plan, ImplIndex};
use crate::verify::{generate_report, TaskStatus, VerifyReport};

/// Run the verify command
pub async fn run(json_output: bool, plan_file: String) -> Result<()> {
    let config = Config::load().unwrap_or_default();
    let plan_path = Path::new(&plan_file);
    let spec_dir = Path::new(&config.paths.spec_dir);
    let impl_dir = Path::new(&config.paths.impl_dir);

    // Check for hierarchical plan first
    if has_hierarchical_plan(impl_dir) {
        return run_hierarchical(json_output, impl_dir, spec_dir).await;
    }

    // Fall back to legacy single-file verification
    if !plan_path.exists() {
        if json_output {
            let empty = serde_json::json!({
                "error": "Plan file not found",
                "path": plan_file
            });
            println!("{}", serde_json::to_string_pretty(&empty)?);
        } else {
            eprintln!("Plan file not found: {}", plan_file);
            eprintln!();
            eprintln!("Run {} first to create an implementation plan.", "fresher plan".cyan());
        }
        return Ok(());
    }

    let report = generate_report(plan_path, spec_dir)?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_report(&report);
    }

    Ok(())
}

/// Run verification for hierarchical plan structure
async fn run_hierarchical(json_output: bool, impl_dir: &Path, _spec_dir: &Path) -> Result<()> {
    let index = ImplIndex::load(impl_dir)?;

    if json_output {
        // Create JSON output compatible with existing format
        let output = serde_json::json!({
            "plan_type": "hierarchical",
            "impl_dir": impl_dir.display().to_string(),
            "total_tasks": index.total_tasks(),
            "completed_tasks": index.completed_tasks(),
            "pending_tasks": index.pending_tasks(),
            "in_progress_tasks": 0,  // Calculated differently for hierarchical
            "current_focus": index.current_focus,
            "is_complete": index.is_complete(),
            "features": index.features.iter().map(|f| {
                serde_json::json!({
                    "name": f.name,
                    "file": f.file.display().to_string(),
                    "status": format!("{:?}", f.status),
                    "total_tasks": f.total_tasks,
                    "completed_tasks": f.completed_tasks,
                    "pending_tasks": f.pending_tasks,
                    "completion_percent": f.completion_percent(),
                    "spec_ref": f.spec_ref,
                })
            }).collect::<Vec<_>>(),
            "cross_cutting": {
                "total": index.cross_cutting_tasks.total,
                "completed": index.cross_cutting_tasks.completed,
                "pending": index.cross_cutting_tasks.pending,
            },
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        print_hierarchical_report(&index);
    }

    Ok(())
}

/// Print hierarchical plan verification report
fn print_hierarchical_report(index: &ImplIndex) {
    println!(
        "{}",
        "Implementation Plan Verification (Hierarchical)".bold()
    );
    println!("{}", "=".repeat(50));
    println!();

    // Feature summary table
    println!("{}", "Feature Summary".bold());
    for feature in &index.features {
        let bar_len = 20;
        let filled = (feature.completion_percent() / 100.0 * bar_len as f64) as usize;
        let empty = bar_len - filled;
        let bar = format!("[{}{}]", "█".repeat(filled), "░".repeat(empty));

        let status_icon = match feature.status {
            crate::impl_plan::FeatureState::Complete => "✅".to_string(),
            crate::impl_plan::FeatureState::InProgress => "🔄".to_string(),
            crate::impl_plan::FeatureState::Pending => "⏳".to_string(),
            crate::impl_plan::FeatureState::Archived => "📦".to_string(),
        };

        let progress = format!(
            "{:.0}% ({}/{})",
            feature.completion_percent(),
            feature.completed_tasks,
            feature.total_tasks
        );

        let colored_progress = if feature.completion_percent() >= 100.0 {
            progress.green()
        } else if feature.completion_percent() >= 50.0 {
            progress.yellow()
        } else {
            progress.normal()
        };

        println!(
            "  {:20} {} {} {}",
            feature.name, bar, colored_progress, status_icon
        );
    }
    println!();

    // Current focus
    if let Some(focus) = &index.current_focus {
        println!("{}", "Current Focus".bold());
        println!("  Active: {}", focus.cyan());

        // Find next task in focused feature
        if let Some(focused_feature) = index.features.iter().find(|f| {
            f.name == focus.trim_end_matches(".md")
                || format!("{}.md", f.name) == *focus
        }) {
            if focused_feature.pending_tasks > 0 {
                println!(
                    "  {} pending tasks remaining",
                    focused_feature.pending_tasks.to_string().yellow()
                );
            }
        }
        println!();
    }

    // Cross-cutting tasks
    if index.cross_cutting_tasks.total > 0 {
        println!("{}", "Cross-Cutting Tasks".bold());
        println!(
            "  Total: {}, Completed: {}, Pending: {}",
            index.cross_cutting_tasks.total,
            index.cross_cutting_tasks.completed.to_string().green(),
            if index.cross_cutting_tasks.pending > 0 {
                index.cross_cutting_tasks.pending.to_string().yellow()
            } else {
                index.cross_cutting_tasks.pending.to_string().green()
            }
        );
        println!();
    }

    // Task summary
    println!("{}", "Task Summary".bold());
    println!(
        "  Total tasks:     {}",
        index.total_tasks().to_string().cyan()
    );
    println!(
        "  Completed:       {}",
        format!(
            "{} ({}%)",
            index.completed_tasks(),
            if index.total_tasks() > 0 {
                index.completed_tasks() * 100 / index.total_tasks()
            } else {
                0
            }
        )
        .green()
    );
    println!(
        "  Pending:         {}",
        index.pending_tasks().to_string().red()
    );
    println!();

    // Status summary
    if index.is_complete() {
        println!("{} All tasks completed!", "✓".green());
    } else {
        let next_focus = index.select_next_focus();
        if let Some(feature) = next_focus {
            println!(
                "{} {} tasks remaining across {} features",
                "→".yellow(),
                index.pending_tasks(),
                index.features.iter().filter(|f| f.pending_tasks > 0).count()
            );
            println!(
                "  Next focus: {} ({} pending)",
                feature.name.cyan(),
                feature.pending_tasks
            );
        } else {
            println!(
                "{} {} tasks remaining",
                "→".yellow(),
                index.pending_tasks()
            );
        }
    }
}

fn print_report(report: &VerifyReport) {
    println!("{}", "Implementation Plan Verification".bold());
    println!("{}", "=".repeat(40));
    println!();

    // Task summary
    println!("{}", "Task Summary".bold());
    println!("  Total tasks:     {}", report.total_tasks.to_string().cyan());
    println!(
        "  Completed:       {}",
        format!("{} ({}%)",
            report.completed_tasks,
            if report.total_tasks > 0 {
                report.completed_tasks * 100 / report.total_tasks
            } else {
                0
            }
        ).green()
    );
    println!(
        "  In Progress:     {}",
        report.in_progress_tasks.to_string().yellow()
    );
    println!(
        "  Pending:         {}",
        report.pending_tasks.to_string().red()
    );
    println!();

    // Traceability
    println!("{}", "Traceability".bold());
    println!(
        "  Tasks with refs: {}",
        report.tasks_with_refs.to_string().cyan()
    );
    println!(
        "  Orphan tasks:    {}",
        if report.orphan_tasks > 0 {
            report.orphan_tasks.to_string().yellow()
        } else {
            report.orphan_tasks.to_string().green()
        }
    );
    println!();

    // Coverage
    if !report.coverage.is_empty() {
        println!("{}", "Spec Coverage".bold());
        for entry in &report.coverage {
            let bar_len = 20;
            let filled = (entry.coverage_percent / 100.0 * bar_len as f64) as usize;
            let empty = bar_len - filled;
            let bar = format!(
                "[{}{}]",
                "█".repeat(filled),
                "░".repeat(empty)
            );

            let coverage_str = format!("{:.0}%", entry.coverage_percent);
            let colored_coverage = if entry.coverage_percent >= 80.0 {
                coverage_str.green()
            } else if entry.coverage_percent >= 50.0 {
                coverage_str.yellow()
            } else {
                coverage_str.red()
            };

            println!(
                "  {:20} {} {} ({} reqs, {} tasks)",
                entry.spec_name,
                bar,
                colored_coverage,
                entry.requirement_count,
                entry.task_count
            );
        }
        println!();
    }

    // Pending tasks list
    let pending: Vec<_> = report
        .tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Pending)
        .collect();

    if !pending.is_empty() {
        println!("{}", "Pending Tasks".bold());
        for task in pending.iter().take(10) {
            let priority = task
                .priority
                .map(|p| format!("P{}", p))
                .unwrap_or_else(|| "P?".to_string());
            println!(
                "  {} {} {}",
                format!("[{}]", priority).dimmed(),
                "○".red(),
                task.description
            );
        }
        if pending.len() > 10 {
            println!("  {} more...", format!("... and {}", pending.len() - 10).dimmed());
        }
        println!();
    }

    // Status summary
    if report.pending_tasks == 0 && report.total_tasks > 0 {
        println!("{} All tasks completed!", "✓".green());
    } else if report.pending_tasks > 0 {
        println!(
            "{} {} tasks remaining",
            "→".yellow(),
            report.pending_tasks
        );
    }
}
