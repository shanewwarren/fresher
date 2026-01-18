use anyhow::{bail, Context, Result};
use chrono::Utc;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crate::config::{detect_project_type, Config};
use crate::templates;

/// Run the init command - initialize .fresher/ in a project
pub async fn run(force: bool) -> Result<()> {
    let fresher_dir = Path::new(".fresher");

    // Check if already initialized
    if fresher_dir.exists() && !force {
        bail!(
            ".fresher/ already exists. Use --force to overwrite, or manually remove it first."
        );
    }

    // Detect project type
    let project_type = detect_project_type();
    let commands = project_type.default_commands();

    println!("Detected project type: {}", project_type.name());

    // Create directory structure
    create_directory_structure()?;

    // Get project name from current directory
    let project_name = std::env::current_dir()?
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string());

    let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    // Create config.toml
    let mut config = Config::default();
    config.commands = commands.clone();
    let config_content = templates::CONFIG_TEMPLATE
        .replace("{timestamp}", &timestamp)
        .replace("{project_type}", project_type.name())
        .replace("{test_command}", &commands.test)
        .replace("{build_command}", &commands.build)
        .replace("{lint_command}", &commands.lint)
        .replace("{src_dir}", &config.paths.src_dir);

    fs::write(".fresher/config.toml", config_content)
        .context("Failed to write config.toml")?;

    // Create AGENTS.md
    let agents_content = templates::AGENTS_TEMPLATE
        .replace("{project_name}", &project_name)
        .replace("{test_command}", &commands.test)
        .replace("{build_command}", &commands.build)
        .replace("{lint_command}", &commands.lint)
        .replace("{src_dir}", &config.paths.src_dir)
        .replace("{spec_dir}", &config.paths.spec_dir);

    fs::write(".fresher/AGENTS.md", agents_content)
        .context("Failed to write AGENTS.md")?;

    // Create prompt templates
    fs::write(".fresher/PROMPT.planning.md", templates::PROMPT_PLANNING)
        .context("Failed to write PROMPT.planning.md")?;
    fs::write(".fresher/PROMPT.building.md", templates::PROMPT_BUILDING)
        .context("Failed to write PROMPT.building.md")?;

    // Create hook scripts
    create_hook(".fresher/hooks/started", templates::HOOK_STARTED)?;
    create_hook(".fresher/hooks/next_iteration", templates::HOOK_NEXT_ITERATION)?;
    create_hook(".fresher/hooks/finished", templates::HOOK_FINISHED)?;

    // Create Docker files
    fs::write(".fresher/docker/Dockerfile", templates::DOCKERFILE_TEMPLATE)
        .context("Failed to write Dockerfile")?;
    fs::write(".fresher/docker/docker-compose.yml", templates::DOCKER_COMPOSE_TEMPLATE)
        .context("Failed to write docker-compose.yml")?;
    fs::write(".fresher/docker/devcontainer.json", templates::DEVCONTAINER_TEMPLATE)
        .context("Failed to write devcontainer.json")?;
    create_hook(".fresher/run.sh", templates::RUN_SCRIPT_TEMPLATE)?;

    // Create specs directory if it doesn't exist
    if !Path::new("specs").exists() {
        fs::create_dir_all("specs").context("Failed to create specs directory")?;
        fs::write("specs/README.md", "# Specifications\n\nAdd your specification files here.\n")
            .context("Failed to write specs/README.md")?;
    }

    println!("Initialized .fresher/ directory");
    println!();
    println!("Next steps:");
    println!("  1. Review .fresher/config.toml and adjust settings");
    println!("  2. Add specifications to specs/");
    println!("  3. Run 'fresher plan' to create an implementation plan");
    println!("  4. Run 'fresher build' to implement tasks");

    Ok(())
}

fn create_directory_structure() -> Result<()> {
    let dirs = [
        ".fresher",
        ".fresher/hooks",
        ".fresher/logs",
        ".fresher/docker",
    ];

    for dir in dirs {
        fs::create_dir_all(dir).with_context(|| format!("Failed to create {}", dir))?;
    }

    Ok(())
}

fn create_hook(path: &str, content: &str) -> Result<()> {
    fs::write(path, content).with_context(|| format!("Failed to write {}", path))?;

    // Make executable
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms)?;

    Ok(())
}
