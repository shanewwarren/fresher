use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::Path;

/// Fresher configuration loaded from environment and config.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub fresher: FresherConfig,
    pub commands: CommandsConfig,
    pub paths: PathsConfig,
    pub hooks: HooksConfig,
    pub docker: DockerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FresherConfig {
    pub mode: String,
    pub max_iterations: u32,
    pub smart_termination: bool,
    pub dangerous_permissions: bool,
    pub max_turns: u32,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandsConfig {
    pub test: String,
    pub build: String,
    pub lint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    pub log_dir: String,
    pub spec_dir: String,
    pub src_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HooksConfig {
    pub enabled: bool,
    pub timeout: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerConfig {
    pub use_docker: bool,
    pub memory: String,
    pub cpus: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            fresher: FresherConfig {
                mode: "planning".to_string(),
                max_iterations: 0,
                smart_termination: true,
                dangerous_permissions: true,
                max_turns: 50,
                model: "sonnet".to_string(),
            },
            commands: CommandsConfig {
                test: String::new(),
                build: String::new(),
                lint: String::new(),
            },
            paths: PathsConfig {
                log_dir: ".fresher/logs".to_string(),
                spec_dir: "specs".to_string(),
                src_dir: "src".to_string(),
            },
            hooks: HooksConfig {
                enabled: true,
                timeout: 30,
            },
            docker: DockerConfig {
                use_docker: false,
                memory: "4g".to_string(),
                cpus: "2".to_string(),
            },
        }
    }
}

impl Config {
    /// Load configuration from .fresher/config.toml and environment variables.
    /// Environment variables take precedence over config file values.
    pub fn load() -> Result<Self> {
        let config_path = Path::new(".fresher/config.toml");

        // Start with defaults
        let mut config = if config_path.exists() {
            let content = std::fs::read_to_string(config_path)
                .context("Failed to read .fresher/config.toml")?;
            toml::from_str(&content)
                .context("Failed to parse .fresher/config.toml")?
        } else {
            Config::default()
        };

        // Override with environment variables
        config.apply_env_overrides();

        Ok(config)
    }

    /// Apply environment variable overrides (env vars take precedence)
    fn apply_env_overrides(&mut self) {
        // Mode
        if let Ok(val) = env::var("FRESHER_MODE") {
            self.fresher.mode = val;
        }

        // Termination settings
        if let Ok(val) = env::var("FRESHER_MAX_ITERATIONS") {
            if let Ok(n) = val.parse() {
                self.fresher.max_iterations = n;
            }
        }
        if let Ok(val) = env::var("FRESHER_SMART_TERMINATION") {
            self.fresher.smart_termination = val.to_lowercase() == "true";
        }

        // Claude Code settings
        if let Ok(val) = env::var("FRESHER_DANGEROUS_PERMISSIONS") {
            self.fresher.dangerous_permissions = val.to_lowercase() == "true";
        }
        if let Ok(val) = env::var("FRESHER_MAX_TURNS") {
            if let Ok(n) = val.parse() {
                self.fresher.max_turns = n;
            }
        }
        if let Ok(val) = env::var("FRESHER_MODEL") {
            self.fresher.model = val;
        }

        // Commands
        if let Ok(val) = env::var("FRESHER_TEST_CMD") {
            self.commands.test = val;
        }
        if let Ok(val) = env::var("FRESHER_BUILD_CMD") {
            self.commands.build = val;
        }
        if let Ok(val) = env::var("FRESHER_LINT_CMD") {
            self.commands.lint = val;
        }

        // Paths
        if let Ok(val) = env::var("FRESHER_LOG_DIR") {
            self.paths.log_dir = val;
        }
        if let Ok(val) = env::var("FRESHER_SPEC_DIR") {
            self.paths.spec_dir = val;
        }
        if let Ok(val) = env::var("FRESHER_SRC_DIR") {
            self.paths.src_dir = val;
        }

        // Hooks
        if let Ok(val) = env::var("FRESHER_HOOKS_ENABLED") {
            self.hooks.enabled = val.to_lowercase() == "true";
        }
        if let Ok(val) = env::var("FRESHER_HOOK_TIMEOUT") {
            if let Ok(n) = val.parse() {
                self.hooks.timeout = n;
            }
        }

        // Docker
        if let Ok(val) = env::var("FRESHER_USE_DOCKER") {
            self.docker.use_docker = val.to_lowercase() == "true";
        }
        if let Ok(val) = env::var("FRESHER_DOCKER_MEMORY") {
            self.docker.memory = val;
        }
        if let Ok(val) = env::var("FRESHER_DOCKER_CPUS") {
            self.docker.cpus = val;
        }
    }

    /// Generate a config.toml content string
    pub fn to_toml_string(&self) -> Result<String> {
        toml::to_string_pretty(self).context("Failed to serialize config to TOML")
    }
}

/// Detect project type based on manifest files
pub fn detect_project_type() -> ProjectType {
    if Path::new("bun.lockb").exists() || Path::new("bunfig.toml").exists() {
        ProjectType::Bun
    } else if Path::new("package.json").exists() {
        ProjectType::NodeJs
    } else if Path::new("Cargo.toml").exists() {
        ProjectType::Rust
    } else if Path::new("go.mod").exists() {
        ProjectType::Go
    } else if Path::new("pyproject.toml").exists() || Path::new("setup.py").exists() {
        ProjectType::Python
    } else if Path::new("Makefile").exists() {
        ProjectType::Make
    } else if Path::new("*.csproj").exists() || Path::new("*.sln").exists() {
        ProjectType::DotNet
    } else if Path::new("pom.xml").exists() {
        ProjectType::Maven
    } else if Path::new("build.gradle").exists() || Path::new("build.gradle.kts").exists() {
        ProjectType::Gradle
    } else {
        ProjectType::Generic
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectType {
    Bun,
    NodeJs,
    Rust,
    Go,
    Python,
    Make,
    DotNet,
    Maven,
    Gradle,
    Generic,
}

impl ProjectType {
    pub fn name(&self) -> &'static str {
        match self {
            ProjectType::Bun => "bun",
            ProjectType::NodeJs => "nodejs",
            ProjectType::Rust => "rust",
            ProjectType::Go => "go",
            ProjectType::Python => "python",
            ProjectType::Make => "make",
            ProjectType::DotNet => "dotnet",
            ProjectType::Maven => "maven",
            ProjectType::Gradle => "gradle",
            ProjectType::Generic => "generic",
        }
    }

    pub fn default_commands(&self) -> CommandsConfig {
        match self {
            ProjectType::Bun => CommandsConfig {
                test: "bun test".to_string(),
                build: "bun run build".to_string(),
                lint: "bun run lint".to_string(),
            },
            ProjectType::NodeJs => CommandsConfig {
                test: "npm test".to_string(),
                build: "npm run build".to_string(),
                lint: "npm run lint".to_string(),
            },
            ProjectType::Rust => CommandsConfig {
                test: "cargo test".to_string(),
                build: "cargo build".to_string(),
                lint: "cargo clippy".to_string(),
            },
            ProjectType::Go => CommandsConfig {
                test: "go test ./...".to_string(),
                build: "go build".to_string(),
                lint: "go fmt".to_string(),
            },
            ProjectType::Python => CommandsConfig {
                test: "pytest".to_string(),
                build: "python -m build".to_string(),
                lint: "ruff check".to_string(),
            },
            ProjectType::Make => CommandsConfig {
                test: "make test".to_string(),
                build: "make build".to_string(),
                lint: "make lint".to_string(),
            },
            ProjectType::DotNet => CommandsConfig {
                test: "dotnet test".to_string(),
                build: "dotnet build".to_string(),
                lint: String::new(),
            },
            ProjectType::Maven => CommandsConfig {
                test: "mvn test".to_string(),
                build: "mvn clean package".to_string(),
                lint: "mvn checkstyle:check".to_string(),
            },
            ProjectType::Gradle => CommandsConfig {
                test: "gradle test".to_string(),
                build: "gradle build".to_string(),
                lint: "gradle check".to_string(),
            },
            ProjectType::Generic => CommandsConfig {
                test: String::new(),
                build: String::new(),
                lint: String::new(),
            },
        }
    }
}
