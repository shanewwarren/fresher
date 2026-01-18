use anyhow::{bail, Result};
use colored::*;
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;
use tokio::signal;

use crate::config::Config;
use crate::docker;
use crate::hooks;
use crate::state::{count_commits_since, get_current_sha, FinishType, State};
use crate::streaming::{process_stream, StreamHandler};
use crate::templates;
use crate::verify::has_pending_tasks;

/// Run the build command - building mode loop
pub async fn run(max_iterations: Option<u32>) -> Result<()> {
    // Check for .fresher directory
    if !Path::new(".fresher").exists() {
        bail!(
            ".fresher/ not found. Run {} first.",
            "fresher init".cyan()
        );
    }

    // Check for implementation plan
    let plan_path = Path::new("IMPLEMENTATION_PLAN.md");
    if !plan_path.exists() {
        bail!(
            "IMPLEMENTATION_PLAN.md not found. Run {} first to create a plan.",
            "fresher plan".cyan()
        );
    }

    // Load configuration
    let mut config = Config::load()?;
    config.fresher.mode = "building".to_string();

    // Check Docker isolation requirements
    docker::enforce_docker_isolation(config.docker.use_docker)?;

    // Override max_iterations if provided
    if let Some(max) = max_iterations {
        config.fresher.max_iterations = max;
    }

    // Check for claude command
    if which::which("claude").is_err() {
        bail!(
            "claude command not found. Please install Claude Code first.\n\
             Visit: https://claude.ai/claude-code"
        );
    }

    let project_dir = std::env::current_dir()?;

    // Initialize state
    let mut state = State::new();

    println!("{}", "Starting Fresher (Building Mode)".bold().green());
    println!("{}", "─".repeat(40));
    println!();

    // Run started hook
    if !hooks::run_started_hook(&state, &config, &project_dir).await? {
        return Ok(());
    }

    // Set up Ctrl+C handler
    let should_stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let should_stop_clone = should_stop.clone();

    tokio::spawn(async move {
        signal::ctrl_c().await.ok();
        should_stop_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        println!("\n{}", "Received interrupt, finishing current iteration...".yellow());
    });

    // Main loop
    loop {
        // Check for interrupt
        if should_stop.load(std::sync::atomic::Ordering::SeqCst) {
            state.set_finish(FinishType::Manual);
            break;
        }

        // Check max iterations
        if config.fresher.max_iterations > 0 && state.iteration >= config.fresher.max_iterations {
            state.set_finish(FinishType::MaxIterations);
            println!("\n{}", "Max iterations reached".yellow());
            break;
        }

        // Check if there are pending tasks
        if !has_pending_tasks(plan_path) {
            state.set_finish(FinishType::Complete);
            println!("{}", "All tasks complete!".green());
            break;
        }

        // Start new iteration
        let iteration_sha = get_current_sha();
        state.start_iteration(iteration_sha.clone());

        println!(
            "{} {}",
            format!("Iteration {}", state.iteration).bold().cyan(),
            "─".repeat(30)
        );

        // Run next_iteration hook
        let (should_continue, should_skip) =
            hooks::run_next_iteration_hook(&state, &config, &project_dir).await?;

        if !should_continue {
            state.set_finish(FinishType::Manual);
            break;
        }

        if should_skip {
            println!("{}", "Skipping iteration (hook requested)".yellow());
            continue;
        }

        // Build claude command
        let prompt = get_prompt(&config)?;
        let result = run_claude_iteration(&prompt, &config).await?;

        // Record iteration result
        let commits_this_iteration = iteration_sha
            .as_ref()
            .map(|sha| count_commits_since(sha))
            .unwrap_or(0);

        state.complete_iteration(result.exit_code, commits_this_iteration);
        state.save()?;

        // Print iteration summary
        if commits_this_iteration > 0 {
            println!(
                "  {} {}",
                "Commits:".dimmed(),
                commits_this_iteration.to_string().green()
            );
        }

        // Check for errors
        if result.exit_code != 0 {
            state.set_finish(FinishType::Error);
            eprintln!("\n{}", format!("Claude exited with code {}", result.exit_code).red());
            break;
        }

        // Smart termination: check for no changes
        if config.fresher.smart_termination {
            let current_sha = get_current_sha();
            if current_sha == state.iteration_sha && commits_this_iteration == 0 {
                state.set_finish(FinishType::NoChanges);
                println!("\n{}", "No changes made this iteration".yellow());
                break;
            }
        }

        println!();
    }

    // Finalize
    state.update_duration();
    state.save()?;

    // Run finished hook
    hooks::run_finished_hook(&state, &config, &project_dir).await?;

    // Print summary
    println!();
    println!("{}", "Summary".bold());
    println!("{}", "─".repeat(40));
    println!("  Iterations: {}", state.iteration.to_string().cyan());
    println!("  Commits:    {}", state.total_commits.to_string().cyan());
    println!("  Duration:   {}s", state.duration.to_string().cyan());
    if let Some(finish) = &state.finish_type {
        println!("  Finished:   {}", finish.to_string().yellow());
    }

    Ok(())
}

/// Get the prompt for building mode
fn get_prompt(_config: &Config) -> Result<String> {
    // Try to read custom prompt first
    let custom_prompt_path = Path::new(".fresher/PROMPT.building.md");
    if custom_prompt_path.exists() {
        return Ok(std::fs::read_to_string(custom_prompt_path)?);
    }

    // Fall back to embedded template
    Ok(templates::PROMPT_BUILDING.to_string())
}

/// Run a single Claude iteration
async fn run_claude_iteration(
    prompt: &str,
    config: &Config,
) -> Result<crate::streaming::ProcessResult> {
    let mut cmd = Command::new("claude");

    // Build arguments
    cmd.arg("-p").arg(prompt);

    // Add system prompt file if it exists
    let agents_path = Path::new(".fresher/AGENTS.md");
    if agents_path.exists() {
        cmd.arg("--append-system-prompt-file").arg(agents_path);
    }

    // Add flags based on config
    if config.fresher.dangerous_permissions {
        cmd.arg("--dangerously-skip-permissions");
    }

    cmd.arg("--output-format").arg("stream-json");
    cmd.arg("--max-turns").arg(config.fresher.max_turns.to_string());
    cmd.arg("--no-session-persistence"); // Critical: fresh context
    cmd.arg("--model").arg(&config.fresher.model);
    cmd.arg("--verbose");

    // Set up stdio
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::inherit());

    // Spawn process
    let mut child = cmd.spawn()?;

    // Process stdout stream
    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let handler = StreamHandler::new();

    let result = process_stream(tokio::io::BufReader::new(stdout), &handler).await?;

    // Wait for process to complete
    let status = child.wait().await?;

    Ok(crate::streaming::ProcessResult {
        exit_code: status.code().unwrap_or(-1),
        ..result
    })
}
