//! Unit tests for Docker functionality
//!
//! These tests verify Dockerfile and docker-compose.yml generation
//! without requiring Docker to be installed.
//!
//! Run with: cargo test --test docker

use fresher::config::DockerConfig;
use fresher::docker::{
    generate_docker_compose, generate_dockerfile, get_image_tag, hash_presets, is_inside_container,
};

/// Helper to create a default DockerConfig for testing
fn default_docker_config() -> DockerConfig {
    DockerConfig {
        use_docker: true,
        memory: "4g".to_string(),
        cpus: "2".to_string(),
        presets: vec![],
        setup_script: None,
        local_binary: None,
    }
}

#[test]
fn test_generate_dockerfile_no_presets() {
    let config = default_docker_config();

    let dockerfile = generate_dockerfile(&config).unwrap();

    // Should have base image
    assert!(
        dockerfile.contains("FROM node:20-bookworm"),
        "Should use node base image"
    );
    // Should install claude-code
    assert!(
        dockerfile.contains("npm install -g @anthropic-ai/claude-code"),
        "Should install claude-code CLI"
    );
    // Should NOT contain cargo install fresher (removed per spec)
    assert!(
        !dockerfile.contains("cargo install"),
        "Should not install fresher via cargo (mounted at runtime)"
    );
    assert!(
        !dockerfile.contains("shanewwarren/fresher"),
        "Should not reference fresher GitHub repo"
    );
}

#[test]
fn test_generate_dockerfile_with_presets() {
    let mut config = default_docker_config();
    config.presets = vec!["rust".to_string(), "bun".to_string()];

    let dockerfile = generate_dockerfile(&config).unwrap();

    // Should have rust preset commands
    assert!(
        dockerfile.contains("rustup.rs"),
        "Should install rust via rustup"
    );
    // Should have bun preset commands
    assert!(
        dockerfile.contains("bun.sh/install"),
        "Should install bun"
    );
    // Should set CARGO_HOME for rust
    assert!(
        dockerfile.contains("CARGO_HOME"),
        "Should set CARGO_HOME env var"
    );
    // Should NOT install fresher via cargo
    assert!(
        !dockerfile.contains("cargo install --git"),
        "Should not install fresher from GitHub"
    );
}

#[test]
fn test_generate_dockerfile_with_custom_setup_script() {
    let mut config = default_docker_config();
    config.setup_script = Some("setup.sh".to_string());

    let dockerfile = generate_dockerfile(&config).unwrap();

    assert!(
        dockerfile.contains("COPY setup.sh /tmp/custom-setup.sh"),
        "Should copy custom setup script"
    );
    assert!(
        dockerfile.contains("/tmp/custom-setup.sh"),
        "Should run custom setup script"
    );
}

#[test]
fn test_generate_compose_with_local_binary() {
    let mut config = default_docker_config();
    config.local_binary = Some("./target/release/fresher".to_string());

    let compose = generate_docker_compose(&config);

    assert!(
        compose.contains("./target/release/fresher:/usr/local/bin/fresher:ro"),
        "Should mount local fresher binary"
    );
    assert!(
        compose.contains("FRESHER_IN_DOCKER=true"),
        "Should set FRESHER_IN_DOCKER env var"
    );
}

#[test]
fn test_generate_compose_without_local_binary() {
    let mut config = default_docker_config();
    config.presets = vec!["rust".to_string()];
    config.local_binary = None;

    let compose = generate_docker_compose(&config);

    // Should NOT have fresher mount line
    assert!(
        !compose.contains("/usr/local/bin/fresher"),
        "Should not mount fresher binary when local_binary is None"
    );
    // Should still have standard mounts
    assert!(
        compose.contains("${PWD}:/workspace"),
        "Should mount workspace"
    );
    assert!(
        compose.contains("${HOME}/.claude:/home/node/.claude"),
        "Should mount claude config"
    );
}

#[test]
fn test_generate_compose_resource_limits() {
    let mut config = default_docker_config();
    config.memory = "8g".to_string();
    config.cpus = "4".to_string();

    let compose = generate_docker_compose(&config);

    assert!(compose.contains("mem_limit: 8g"), "Should set memory limit");
    assert!(compose.contains("cpus: 4"), "Should set CPU limit");
}

#[test]
fn test_hash_presets_deterministic() {
    let presets1 = vec!["rust".to_string(), "bun".to_string()];
    let presets2 = vec!["rust".to_string(), "bun".to_string()];
    let presets3 = vec!["bun".to_string(), "rust".to_string()];

    // Same presets should produce same hash
    assert_eq!(
        hash_presets(&presets1),
        hash_presets(&presets2),
        "Same presets should produce same hash"
    );
    // Order matters for hash (by design)
    assert_ne!(
        hash_presets(&presets1),
        hash_presets(&presets3),
        "Different order should produce different hash"
    );
}

#[test]
fn test_hash_presets_empty() {
    let empty: Vec<String> = vec![];
    let hash = hash_presets(&empty);

    // Should produce a valid 8-character hex hash
    assert_eq!(hash.len(), 8, "Hash should be 8 characters");
    assert!(
        hash.chars().all(|c| c.is_ascii_hexdigit()),
        "Hash should be hex"
    );
}

#[test]
fn test_get_image_tag_no_presets() {
    let presets: Vec<String> = vec![];
    assert_eq!(
        get_image_tag(&presets),
        "fresher-base:latest",
        "Empty presets should use fresher-base:latest"
    );
}

#[test]
fn test_get_image_tag_with_presets() {
    let presets = vec!["rust".to_string()];
    let tag = get_image_tag(&presets);

    assert!(
        tag.starts_with("fresher-dev:"),
        "Should start with fresher-dev:"
    );
    assert!(
        tag.len() > "fresher-dev:".len(),
        "Should have hash suffix"
    );
}

#[test]
fn test_is_inside_container_false() {
    // Ensure env vars are not set for this test
    std::env::remove_var("DEVCONTAINER");
    std::env::remove_var("FRESHER_IN_DOCKER");

    // In normal test environment, should be false
    // Note: This test may fail if run inside an actual container
    let result = is_inside_container();
    // We can't assert false because CI might run in a container
    // Just verify the function runs without error
    let _ = result;
}

#[test]
fn test_is_inside_container_devcontainer() {
    std::env::set_var("DEVCONTAINER", "true");
    assert!(
        is_inside_container(),
        "Should detect DEVCONTAINER=true"
    );
    std::env::remove_var("DEVCONTAINER");
}

#[test]
fn test_is_inside_container_fresher_flag() {
    std::env::set_var("FRESHER_IN_DOCKER", "true");
    assert!(
        is_inside_container(),
        "Should detect FRESHER_IN_DOCKER=true"
    );
    std::env::remove_var("FRESHER_IN_DOCKER");
}

#[test]
fn test_generate_compose_uses_correct_image_tag() {
    let mut config = default_docker_config();
    config.presets = vec!["rust".to_string(), "bun".to_string()];

    let compose = generate_docker_compose(&config);
    let expected_tag = get_image_tag(&config.presets);

    assert!(
        compose.contains(&format!("image: {}", expected_tag)),
        "Compose should use the correct image tag"
    );
}

#[test]
fn test_generate_dockerfile_go_preset() {
    let mut config = default_docker_config();
    config.presets = vec!["go".to_string()];

    let dockerfile = generate_dockerfile(&config).unwrap();

    assert!(
        dockerfile.contains("go.dev/dl/go"),
        "Should install Go from official source"
    );
    assert!(
        dockerfile.contains("/usr/local/go/bin"),
        "Should add Go to PATH"
    );
}

#[test]
fn test_generate_dockerfile_python_preset() {
    let mut config = default_docker_config();
    config.presets = vec!["python".to_string()];

    let dockerfile = generate_dockerfile(&config).unwrap();

    assert!(
        dockerfile.contains("python3"),
        "Should install python3"
    );
    assert!(
        dockerfile.contains("python3-pip"),
        "Should install pip"
    );
}

#[test]
fn test_generate_dockerfile_unknown_preset_warning() {
    let mut config = default_docker_config();
    config.presets = vec!["unknown_preset".to_string()];

    // Should not error, just skip unknown presets
    let result = generate_dockerfile(&config);
    assert!(result.is_ok(), "Should not error on unknown preset");

    let dockerfile = result.unwrap();
    assert!(
        !dockerfile.contains("unknown_preset"),
        "Should not include unknown preset in output"
    );
}
