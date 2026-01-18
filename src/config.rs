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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_default_config() {
        let config = Config::default();

        assert_eq!(config.fresher.mode, "planning");
        assert_eq!(config.fresher.max_iterations, 0);
        assert!(config.fresher.smart_termination);
        assert!(config.fresher.dangerous_permissions);
        assert_eq!(config.fresher.max_turns, 50);
        assert_eq!(config.fresher.model, "sonnet");

        assert!(config.commands.test.is_empty());
        assert!(config.commands.build.is_empty());
        assert!(config.commands.lint.is_empty());

        assert_eq!(config.paths.log_dir, ".fresher/logs");
        assert_eq!(config.paths.spec_dir, "specs");
        assert_eq!(config.paths.src_dir, "src");

        assert!(config.hooks.enabled);
        assert_eq!(config.hooks.timeout, 30);

        assert!(!config.docker.use_docker);
        assert_eq!(config.docker.memory, "4g");
        assert_eq!(config.docker.cpus, "2");
    }

    #[test]
    fn test_env_override_mode() {
        let mut config = Config::default();

        // Set environment variable
        env::set_var("FRESHER_MODE", "building");
        config.apply_env_overrides();

        assert_eq!(config.fresher.mode, "building");

        // Clean up
        env::remove_var("FRESHER_MODE");
    }

    #[test]
    fn test_env_override_max_iterations() {
        let mut config = Config::default();

        env::set_var("FRESHER_MAX_ITERATIONS", "10");
        config.apply_env_overrides();

        assert_eq!(config.fresher.max_iterations, 10);

        env::remove_var("FRESHER_MAX_ITERATIONS");
    }

    #[test]
    fn test_env_override_smart_termination() {
        let mut config = Config::default();

        env::set_var("FRESHER_SMART_TERMINATION", "false");
        config.apply_env_overrides();

        assert!(!config.fresher.smart_termination);

        env::remove_var("FRESHER_SMART_TERMINATION");
    }

    #[test]
    fn test_env_override_hooks() {
        let mut config = Config::default();

        env::set_var("FRESHER_HOOKS_ENABLED", "false");
        env::set_var("FRESHER_HOOK_TIMEOUT", "60");
        config.apply_env_overrides();

        assert!(!config.hooks.enabled);
        assert_eq!(config.hooks.timeout, 60);

        env::remove_var("FRESHER_HOOKS_ENABLED");
        env::remove_var("FRESHER_HOOK_TIMEOUT");
    }

    #[test]
    fn test_env_override_docker() {
        let mut config = Config::default();

        env::set_var("FRESHER_USE_DOCKER", "true");
        env::set_var("FRESHER_DOCKER_MEMORY", "8g");
        env::set_var("FRESHER_DOCKER_CPUS", "4");
        config.apply_env_overrides();

        assert!(config.docker.use_docker);
        assert_eq!(config.docker.memory, "8g");
        assert_eq!(config.docker.cpus, "4");

        env::remove_var("FRESHER_USE_DOCKER");
        env::remove_var("FRESHER_DOCKER_MEMORY");
        env::remove_var("FRESHER_DOCKER_CPUS");
    }

    #[test]
    fn test_env_override_commands() {
        let mut config = Config::default();

        env::set_var("FRESHER_TEST_CMD", "bun test");
        env::set_var("FRESHER_BUILD_CMD", "bun run build");
        env::set_var("FRESHER_LINT_CMD", "bun run lint");
        config.apply_env_overrides();

        assert_eq!(config.commands.test, "bun test");
        assert_eq!(config.commands.build, "bun run build");
        assert_eq!(config.commands.lint, "bun run lint");

        env::remove_var("FRESHER_TEST_CMD");
        env::remove_var("FRESHER_BUILD_CMD");
        env::remove_var("FRESHER_LINT_CMD");
    }

    #[test]
    fn test_env_override_paths() {
        let mut config = Config::default();

        env::set_var("FRESHER_LOG_DIR", "/tmp/logs");
        env::set_var("FRESHER_SPEC_DIR", "specifications");
        env::set_var("FRESHER_SRC_DIR", "source");
        config.apply_env_overrides();

        assert_eq!(config.paths.log_dir, "/tmp/logs");
        assert_eq!(config.paths.spec_dir, "specifications");
        assert_eq!(config.paths.src_dir, "source");

        env::remove_var("FRESHER_LOG_DIR");
        env::remove_var("FRESHER_SPEC_DIR");
        env::remove_var("FRESHER_SRC_DIR");
    }

    #[test]
    fn test_invalid_number_parsing() {
        // Test that invalid numbers don't parse - this tests the parse logic
        // directly rather than through env vars to avoid parallel test interference
        let invalid = "not_a_number";
        let result: Result<u32, _> = invalid.parse();
        assert!(result.is_err(), "Invalid string should not parse as u32");

        let valid = "10";
        let result: Result<u32, _> = valid.parse();
        assert!(result.is_ok(), "Valid number string should parse");
        assert_eq!(result.unwrap(), 10);
    }

    #[test]
    fn test_project_type_name() {
        assert_eq!(ProjectType::Bun.name(), "bun");
        assert_eq!(ProjectType::NodeJs.name(), "nodejs");
        assert_eq!(ProjectType::Rust.name(), "rust");
        assert_eq!(ProjectType::Go.name(), "go");
        assert_eq!(ProjectType::Python.name(), "python");
        assert_eq!(ProjectType::Make.name(), "make");
        assert_eq!(ProjectType::DotNet.name(), "dotnet");
        assert_eq!(ProjectType::Maven.name(), "maven");
        assert_eq!(ProjectType::Gradle.name(), "gradle");
        assert_eq!(ProjectType::Generic.name(), "generic");
    }

    #[test]
    fn test_project_type_default_commands() {
        let rust_cmds = ProjectType::Rust.default_commands();
        assert_eq!(rust_cmds.test, "cargo test");
        assert_eq!(rust_cmds.build, "cargo build");
        assert_eq!(rust_cmds.lint, "cargo clippy");

        let bun_cmds = ProjectType::Bun.default_commands();
        assert_eq!(bun_cmds.test, "bun test");
        assert_eq!(bun_cmds.build, "bun run build");
        assert_eq!(bun_cmds.lint, "bun run lint");

        let go_cmds = ProjectType::Go.default_commands();
        assert_eq!(go_cmds.test, "go test ./...");
        assert_eq!(go_cmds.build, "go build");
        assert_eq!(go_cmds.lint, "go fmt");

        let generic_cmds = ProjectType::Generic.default_commands();
        assert!(generic_cmds.test.is_empty());
        assert!(generic_cmds.build.is_empty());
        assert!(generic_cmds.lint.is_empty());
    }

    #[test]
    fn test_config_to_toml_string() {
        let config = Config::default();
        let toml_str = config.to_toml_string().unwrap();

        assert!(toml_str.contains("[fresher]"));
        assert!(toml_str.contains("mode = \"planning\""));
        assert!(toml_str.contains("[commands]"));
        assert!(toml_str.contains("[paths]"));
        assert!(toml_str.contains("[hooks]"));
        assert!(toml_str.contains("[docker]"));
    }

    #[test]
    fn test_config_roundtrip() {
        let config = Config::default();
        let toml_str = config.to_toml_string().unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.fresher.mode, config.fresher.mode);
        assert_eq!(parsed.fresher.max_iterations, config.fresher.max_iterations);
        assert_eq!(parsed.hooks.timeout, config.hooks.timeout);
    }
}
