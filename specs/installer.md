# Installation and Upgrade Specification

**Status:** Implemented
**Version:** 2.0
**Last Updated:** 2026-01-18
**Implementation:** `src/commands/upgrade.rs`, `src/upgrade.rs`

---

## 1. Overview

### Purpose

Fresher is distributed as a compiled Rust binary with self-upgrade capabilities. Users can install via `cargo install` or download prebuilt binaries from GitHub releases. The upgrade command fetches the latest release and replaces the running binary.

### Goals

- **Multiple installation methods** - Cargo install, binary download, or build from source
- **Self-upgrading binary** - The `fresher upgrade` command updates itself
- **Cross-platform support** - macOS (Intel/Apple Silicon), Linux (x64/ARM64)
- **Version awareness** - Check installed version against latest release

### Non-Goals

- **Package manager distribution** - No Homebrew/apt packages (initially)
- **Auto-updates** - Users must explicitly run upgrade command
- **Rollback** - No built-in version downgrade (reinstall specific version)
- **Windows support** - Not yet implemented

---

## 2. Installation Methods

### 2.1 Cargo Install (Recommended)

```bash
cargo install fresher
```

**Requirements:**
- Rust toolchain installed
- Builds from source on the user's machine

### 2.2 Binary Download

Download prebuilt binaries from GitHub releases:

```bash
# macOS Apple Silicon
curl -L https://github.com/shanewwarren/fresher/releases/latest/download/fresher-aarch64-apple-darwin.tar.gz | tar xz
sudo mv fresher /usr/local/bin/

# macOS Intel
curl -L https://github.com/shanewwarren/fresher/releases/latest/download/fresher-x86_64-apple-darwin.tar.gz | tar xz
sudo mv fresher /usr/local/bin/

# Linux x64
curl -L https://github.com/shanewwarren/fresher/releases/latest/download/fresher-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv fresher /usr/local/bin/

# Linux ARM64
curl -L https://github.com/shanewwarren/fresher/releases/latest/download/fresher-aarch64-unknown-linux-gnu.tar.gz | tar xz
sudo mv fresher /usr/local/bin/
```

### 2.3 Build from Source

```bash
git clone https://github.com/shanewwarren/fresher.git
cd fresher
cargo build --release
cp target/release/fresher /usr/local/bin/
```

---

## 3. Architecture

### Module Structure

```
src/
├── commands/
│   └── upgrade.rs        # CLI handler for upgrade command
└── upgrade.rs            # Core upgrade logic
```

### Upgrade Flow

```
┌─────────────────────────────────────────────────────────────────┐
│  fresher upgrade [--check]                                       │
├─────────────────────────────────────────────────────────────────┤
│  1. Get installed version from CARGO_PKG_VERSION                │
│  2. Fetch latest release from GitHub API                        │
│  3. Compare versions (semver)                                   │
│                                                                 │
│  If --check:                                                    │
│     Display versions and exit                                   │
│                                                                 │
│  If upgrade needed:                                             │
│  4. Determine platform (os + arch)                              │
│  5. Download release tarball for platform                       │
│  6. Extract binary to temp directory                            │
│  7. Replace current executable                                  │
│  8. Set executable permissions                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 4. Core Types

### 4.1 Version Management

Version is retrieved from Cargo.toml at compile time:

```rust
pub fn get_installed_version() -> Result<Version> {
    let version_str = env!("CARGO_PKG_VERSION");
    Version::parse(version_str)
}
```

### 4.2 Platform Detection

```rust
fn get_platform() -> Result<&'static str> {
    match (env::consts::OS, env::consts::ARCH) {
        ("macos", "x86_64") => Ok("x86_64-apple-darwin"),
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-gnu"),
        ("linux", "aarch64") => Ok("aarch64-unknown-linux-gnu"),
        _ => bail!("Unsupported platform"),
    }
}
```

### 4.3 GitHub API Integration

```rust
const GITHUB_REPO: &str = "shanewwarren/fresher";
const GITHUB_API_URL: &str = "https://api.github.com";

// Fetch latest release
let url = format!("{}/repos/{}/releases/latest", GITHUB_API_URL, GITHUB_REPO);
```

---

## 5. CLI Interface

### fresher upgrade

Upgrades the Fresher binary to the latest version:

```bash
fresher upgrade [OPTIONS]

Options:
  --check    Check for updates without installing
  -h, --help Print help
```

### Check Mode Output

```
Installed version: 1.2.3
Checking for updates... update available!

  1.2.3 → 1.3.0

Run fresher upgrade to upgrade
```

### Upgrade Output

```
Upgrading from 1.2.3 to 1.3.0...
Downloading fresher-aarch64-apple-darwin.tar.gz...
Successfully upgraded to 1.3.0

✓ Successfully upgraded to version 1.3.0
```

### Already Up-to-Date

```
Already at the latest version (1.3.0)
```

---

## 6. Behaviors

### 6.1 Version Check

```rust
pub async fn check_upgrade() -> Result<Option<Version>> {
    let installed = get_installed_version()?;
    let latest = get_latest_version().await?;

    if latest > installed {
        Ok(Some(latest))
    } else {
        Ok(None)
    }
}
```

### 6.2 Binary Replacement (Unix)

The upgrade process handles replacing a running binary:

```rust
fn replace_binary(current: &PathBuf, new: &PathBuf) -> Result<()> {
    // Read new binary contents
    let new_contents = fs::read(new)?;

    // Create backup
    let backup_path = current.with_extension("bak");
    fs::rename(current, &backup_path)?;

    // Write new binary
    fs::write(current, &new_contents)?;

    // Set executable permissions (0o755)
    let mut perms = fs::metadata(current)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(current, perms)?;

    // Remove backup
    fs::remove_file(&backup_path).ok();

    Ok(())
}
```

---

## 7. Error Handling

| Condition | Behavior |
|-----------|----------|
| Network failure | Return error with details |
| GitHub API error | Return HTTP status and message |
| Missing release | Return "Binary not found in archive" |
| Unsupported platform | Return "Unsupported platform: {os}-{arch}" |
| Permission denied | Return write error |
| Windows | Return "Self-upgrade on Windows not supported" |

---

## 8. Release Artifacts

Each GitHub release includes these artifacts:

| Platform | Artifact Name |
|----------|---------------|
| macOS Intel | `fresher-x86_64-apple-darwin.tar.gz` |
| macOS Apple Silicon | `fresher-aarch64-apple-darwin.tar.gz` |
| Linux x64 | `fresher-x86_64-unknown-linux-gnu.tar.gz` |
| Linux ARM64 | `fresher-aarch64-unknown-linux-gnu.tar.gz` |

**Archive contents:**
```
fresher-{platform}.tar.gz
└── fresher          # The compiled binary
```

---

## 9. Dependencies

```toml
[dependencies]
reqwest = { version = "0.11", features = ["json"] }
semver = "1"
flate2 = "1"
tar = "0.4"
tempfile = "3"
serde_json = "1"
```

---

## 10. Security Considerations

### Binary Verification

Currently, binaries are downloaded via HTTPS from GitHub releases. Future enhancements could include:
- SHA256 checksum verification
- GPG signature verification

### Permissions

- Upgrade requires write access to the binary location
- No sudo/root required if binary is in user-writable location
- Binary permissions set to 755 (rwxr-xr-x)

### Network

- All connections use HTTPS
- User-Agent header identifies the client
- No credentials stored

---

## 11. Future Enhancements

- **Windows support**: Handle Windows binary replacement
- **Checksum verification**: Verify downloaded binary integrity
- **Version pinning**: Allow specifying exact version to install
- **Homebrew formula**: `brew install fresher`
- **Release notes**: Show changelog during upgrade
- **Offline mode**: Support installing from local file
