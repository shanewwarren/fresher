use anyhow::{bail, Result};
use colored::*;
use std::path::Path;

use crate::config::Config;
use crate::impl_plan::{analyze_migration, has_hierarchical_plan, migrate_plan};

/// Run the migrate-plan command
pub async fn run(force: bool, dry_run: bool) -> Result<()> {
    let config = Config::load().unwrap_or_default();
    let legacy_path = Path::new("IMPLEMENTATION_PLAN.md");
    let impl_dir = Path::new(&config.paths.impl_dir);

    // Check if hierarchical plan already exists
    if has_hierarchical_plan(impl_dir) {
        bail!(
            "Hierarchical plan already exists at {}/\n\
             Remove the existing impl/ directory to re-migrate.",
            config.paths.impl_dir
        );
    }

    // Check if legacy plan exists
    if !legacy_path.exists() {
        bail!(
            "No legacy plan found at IMPLEMENTATION_PLAN.md\n\
             Run {} first to create a plan.",
            "fresher plan".cyan()
        );
    }

    // Analyze the migration
    let threshold = if force { 0 } else { config.fresher.single_file_threshold };
    let analysis = analyze_migration(legacy_path, threshold)?;

    println!("{}", "Migration Analysis".bold());
    println!("{}", "=".repeat(40));
    println!();

    println!("  Source:          {}", legacy_path.display());
    println!("  Target:          {}/", config.paths.impl_dir);
    println!("  Total tasks:     {}", analysis.total_tasks.to_string().cyan());
    println!(
        "  Features found:  {}",
        analysis.tasks_by_spec.len().to_string().cyan()
    );
    println!(
        "  Orphan tasks:    {}",
        if analysis.orphan_tasks.is_empty() {
            "0".green()
        } else {
            analysis.orphan_tasks.len().to_string().yellow()
        }
    );
    println!();

    // List features to be created
    println!("{}", "Features to create:".bold());
    for (spec_name, tasks) in &analysis.tasks_by_spec {
        let completed = tasks
            .iter()
            .filter(|t| t.status == crate::verify::TaskStatus::Completed)
            .count();
        println!(
            "  {} {}/{} tasks",
            format!("{}.md", spec_name).cyan(),
            completed,
            tasks.len()
        );
    }
    println!();

    if !analysis.orphan_tasks.is_empty() {
        println!("{}", "Orphan tasks (will go in README.md):".bold());
        for task in &analysis.orphan_tasks {
            let checkbox = if task.status == crate::verify::TaskStatus::Completed {
                "[x]"
            } else {
                "[ ]"
            };
            println!("  {} {}", checkbox.dimmed(), task.description);
        }
        println!();
    }

    // Check threshold
    if !analysis.should_migrate && !force {
        println!(
            "{} Task count ({}) is below threshold ({}).",
            "Note:".yellow(),
            analysis.total_tasks,
            threshold
        );
        println!("Use {} to migrate anyway.", "--force".cyan());
        return Ok(());
    }

    // Dry run - just show what would happen
    if dry_run {
        println!("{} Dry run - no changes made.", "✓".green());
        println!();
        println!("To perform the migration, run:");
        println!("  {} {}", "fresher migrate-plan".cyan(), if force { "--force" } else { "" });
        return Ok(());
    }

    // Confirm migration
    println!(
        "{} This will create hierarchical plan structure and backup the original.",
        "→".yellow()
    );

    // Perform migration
    let result = migrate_plan(legacy_path, impl_dir, threshold)?;

    println!();
    println!("{}", "Migration Complete".bold().green());
    println!("{}", "=".repeat(40));
    println!();
    println!("  Created:     {}/", result.impl_dir.display());
    println!("  Features:    {}", result.feature_count.to_string().cyan());
    println!("  Total tasks: {}", result.task_count.to_string().cyan());
    println!("  Backup:      {}", result.backup_path.display());
    println!();

    println!("Files created:");
    for file in &result.created_files {
        println!("  {}", file.display());
    }
    println!();

    println!("{} Plan migrated successfully!", "✓".green());
    println!();
    println!("Next steps:");
    println!("  1. Review the generated files in {}/", config.paths.impl_dir);
    println!("  2. Run {} to verify the structure", "fresher verify".cyan());
    println!("  3. Run {} to continue building", "fresher build".cyan());

    Ok(())
}
