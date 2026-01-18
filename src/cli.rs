use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "fresher")]
#[command(author = "Shane Warren")]
#[command(version)]
#[command(about = "AI-driven iterative development using the Ralph Loop methodology")]
#[command(long_about = "Fresher provides the infrastructure to run the Ralph Loop methodology - \
an iterative execution model with two modes (PLANNING and BUILDING) that uses fresh context \
each iteration for specification-driven development.")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize .fresher/ in a project
    Init {
        /// Overwrite existing configuration
        #[arg(short, long)]
        force: bool,
    },

    /// Run planning mode - analyze specs and create implementation plan
    Plan {
        /// Maximum iterations (0 = unlimited)
        #[arg(short, long, env = "FRESHER_MAX_ITERATIONS")]
        max_iterations: Option<u32>,
    },

    /// Run building mode - implement tasks from the plan
    Build {
        /// Maximum iterations (0 = unlimited)
        #[arg(short, long, env = "FRESHER_MAX_ITERATIONS")]
        max_iterations: Option<u32>,
    },

    /// Verify implementation plan against specs
    Verify {
        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Path to implementation plan file
        #[arg(short, long, default_value = "IMPLEMENTATION_PLAN.md")]
        plan_file: String,
    },

    /// Self-upgrade to the latest version
    Upgrade {
        /// Check for updates without installing
        #[arg(long)]
        check: bool,
    },

    /// Show version information
    Version,
}
