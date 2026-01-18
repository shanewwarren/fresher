use anyhow::Result;

/// Run the version command - display version and build information
pub fn run() -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    let name = env!("CARGO_PKG_NAME");

    println!("{} v{}", name, version);
    println!();
    println!("Repository: https://github.com/shanewwarren/fresher");
    println!("License: MIT");

    // Show build info if available
    if let Some(hash) = option_env!("FRESHER_BUILD_HASH") {
        println!("Build: {}", hash);
    }

    Ok(())
}
