use anyhow::Result;
use colored::*;
use std::path::Path;

use crate::config::Config;
use crate::verify::{generate_report, TaskStatus, VerifyReport};

/// Run the verify command
pub async fn run(json_output: bool, plan_file: String) -> Result<()> {
    let config = Config::load().unwrap_or_default();
    let plan_path = Path::new(&plan_file);
    let spec_dir = Path::new(&config.paths.spec_dir);

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
