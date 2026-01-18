use anyhow::{bail, Context, Result};
use flate2::read::GzDecoder;
use semver::Version;
use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use tar::Archive;

const GITHUB_REPO: &str = "shanewwarren/fresher";
const GITHUB_API_URL: &str = "https://api.github.com";

/// Get the current installed version
pub fn get_installed_version() -> Result<Version> {
    let version_str = env!("CARGO_PKG_VERSION");
    Version::parse(version_str).context("Failed to parse installed version")
}

/// Fetch the latest release version from GitHub
pub async fn get_latest_version() -> Result<Version> {
    let url = format!("{}/repos/{}/releases/latest", GITHUB_API_URL, GITHUB_REPO);

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("User-Agent", "fresher-cli")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .context("Failed to fetch latest release")?;

    if !response.status().is_success() {
        bail!("Failed to fetch latest release: HTTP {}", response.status());
    }

    let json: serde_json::Value = response.json().await?;
    let tag = json["tag_name"]
        .as_str()
        .context("Missing tag_name in release")?;

    // Remove 'v' prefix if present
    let version_str = tag.strip_prefix('v').unwrap_or(tag);
    Version::parse(version_str).context("Failed to parse latest version")
}

/// Check if an upgrade is available
pub async fn check_upgrade() -> Result<Option<Version>> {
    let installed = get_installed_version()?;
    let latest = get_latest_version().await?;

    if latest > installed {
        Ok(Some(latest))
    } else {
        Ok(None)
    }
}

/// Download and install the latest version
pub async fn upgrade() -> Result<Version> {
    let latest = get_latest_version().await?;
    let installed = get_installed_version()?;

    if latest <= installed {
        println!("Already at latest version ({})", installed);
        return Ok(installed);
    }

    println!("Upgrading from {} to {}...", installed, latest);

    // Determine platform
    let platform = get_platform()?;
    let asset_name = format!("fresher-{}.tar.gz", platform);

    // Download the release asset
    let download_url = format!(
        "https://github.com/{}/releases/download/v{}/{}",
        GITHUB_REPO, latest, asset_name
    );

    println!("Downloading {}...", asset_name);

    let client = reqwest::Client::new();
    let response = client
        .get(&download_url)
        .header("User-Agent", "fresher-cli")
        .send()
        .await
        .context("Failed to download release")?;

    if !response.status().is_success() {
        bail!(
            "Failed to download release: HTTP {} - {}",
            response.status(),
            download_url
        );
    }

    let bytes = response.bytes().await?;

    // Extract the binary
    let current_exe = env::current_exe().context("Failed to get current executable path")?;
    let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;

    // Extract tarball
    let decoder = GzDecoder::new(bytes.as_ref());
    let mut archive = Archive::new(decoder);
    archive
        .unpack(temp_dir.path())
        .context("Failed to extract archive")?;

    // Find the binary in the extracted files
    let new_binary = temp_dir.path().join("fresher");
    if !new_binary.exists() {
        bail!("Binary not found in downloaded archive");
    }

    // Replace the current binary
    replace_binary(&current_exe, &new_binary)?;

    println!("Successfully upgraded to {}", latest);
    Ok(latest)
}

/// Get the platform identifier for downloads
fn get_platform() -> Result<&'static str> {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;

    match (os, arch) {
        ("macos", "x86_64") => Ok("x86_64-apple-darwin"),
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-gnu"),
        ("linux", "aarch64") => Ok("aarch64-unknown-linux-gnu"),
        _ => bail!("Unsupported platform: {}-{}", os, arch),
    }
}

/// Replace the current binary with a new one
fn replace_binary(current: &PathBuf, new: &PathBuf) -> Result<()> {
    // Read the new binary
    let mut new_file = File::open(new).context("Failed to open new binary")?;
    let mut new_contents = Vec::new();
    new_file
        .read_to_end(&mut new_contents)
        .context("Failed to read new binary")?;

    // Create backup of current binary
    let backup_path = current.with_extension("bak");
    if backup_path.exists() {
        fs::remove_file(&backup_path).ok();
    }

    // On Unix, we can't write to the running binary directly
    // So we rename it and write a new file
    #[cfg(unix)]
    {
        // Rename current to backup
        fs::rename(current, &backup_path).context("Failed to backup current binary")?;

        // Write new binary
        let mut dest = File::create(current).context("Failed to create new binary file")?;
        dest.write_all(&new_contents)
            .context("Failed to write new binary")?;

        // Set executable permissions
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(current)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(current, perms)?;

        // Remove backup
        fs::remove_file(&backup_path).ok();
    }

    #[cfg(windows)]
    {
        // On Windows, we need to use a different approach
        // Schedule the replacement on next run or use MoveFileEx
        bail!("Self-upgrade on Windows is not yet supported");
    }

    Ok(())
}
