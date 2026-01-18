# Installer Specification

**Status:** Planned
**Version:** 1.0
**Last Updated:** 2026-01-17

---

## 1. Overview

### Purpose

The installer provides a one-command mechanism to add Fresher to any project and upgrade existing installations. It fetches versioned releases from GitHub, handles the core vs custom file distinction, and preserves user customizations during upgrades.

### Goals

- **One-command install** - Users can add Fresher with a single curl/bash command
- **Clean upgrades** - Core files are replaced while custom files are preserved
- **Version awareness** - Track installed version and detect available upgrades
- **Offline capability** - Support local installation for development

### Non-Goals

- **Package manager distribution** - No npm/brew/apt packages (keep it simple)
- **Auto-updates** - Users must explicitly run upgrade command
- **Rollback** - No built-in version downgrade (users can reinstall specific version)

---

## 2. Architecture

### Component Structure

```
.fresher/
├── VERSION                   # Installed version tracking
├── run.sh                    # Core: replaced on upgrade
├── config.sh                 # Mixed: values preserved
├── AGENTS.md                 # Custom: preserved
├── PROMPT.planning.md        # Core: replaced
├── PROMPT.building.md        # Core: replaced
├── hooks/                    # Custom: preserved
│   ├── started
│   ├── next_iteration
│   └── finished
├── lib/                      # Core: replaced
├── bin/                      # Core: replaced
├── docker/                   # Core: replaced
└── tests/                    # Core: replaced
```

### File Classification

| Classification | Files | Upgrade Behavior |
|----------------|-------|------------------|
| **Core** | run.sh, PROMPT.*.md, lib/*, bin/*, docker/*, tests/* | Always replaced |
| **Custom** | AGENTS.md, hooks/* | Never touched |
| **Mixed** | config.sh | Template replaced, values preserved |
| **Generated** | .state, logs/* | Ignored (gitignored) |
| **Metadata** | VERSION | Updated to new version |

### Installation Flow

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  Fetch Release  │────▶│  Extract Files  │────▶│  Detect Project │
└─────────────────┘     └─────────────────┘     └─────────────────┘
                                                         │
                                                         ▼
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  Write VERSION  │◀────│  Run Init Hook  │◀────│  Generate Config│
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

### Upgrade Flow

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  Read VERSION   │────▶│  Fetch Release  │────▶│  Backup Config  │
└─────────────────┘     └─────────────────┘     └─────────────────┘
                                                         │
                                                         ▼
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  Update VERSION │◀────│  Restore Values │◀────│  Replace Core   │
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

---

## 3. Core Types

### 3.1 VERSION File

Plain text file containing the installed Fresher version.

```
1.2.3
```

| Field | Type | Description |
|-------|------|-------------|
| version | semver string | Installed version (e.g., "1.2.3") |

### 3.2 Config Values

Values extracted from config.sh during upgrade.

```bash
# Variables to preserve during upgrade
FRESHER_TEST_CMD="npm test"
FRESHER_BUILD_CMD="npm run build"
FRESHER_LINT_CMD="npm run lint"
FRESHER_SRC_DIR="src"
FRESHER_MAX_ITERATIONS="50"
```

| Variable | Type | Description |
|----------|------|-------------|
| FRESHER_TEST_CMD | string | Command to run tests |
| FRESHER_BUILD_CMD | string | Command to build project |
| FRESHER_LINT_CMD | string | Command to run linter |
| FRESHER_SRC_DIR | string | Source directory path |
| FRESHER_MAX_ITERATIONS | number | Max loop iterations |

---

## 4. API / Behaviors

### 4.1 Install Command

**Purpose:** Install Fresher into the current project

```bash
# Install latest release
curl -fsSL https://raw.githubusercontent.com/{org}/fresher/main/install.sh | bash

# Install specific version
curl -fsSL https://raw.githubusercontent.com/{org}/fresher/main/install.sh | bash -s -- --version=1.2.3

# Install from local development copy
./install.sh --source=/path/to/fresher
```

**Flags:**

| Flag | Description | Default |
|------|-------------|---------|
| `--version=X.Y.Z` | Install specific version | latest |
| `--source=PATH` | Install from local directory | GitHub |
| `--force` | Overwrite existing installation | false |
| `--dry-run` | Show what would be installed | false |

**Exit Codes:**

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Installation failed |
| 2 | Already installed (use --force or upgrade) |
| 3 | Network error |

### 4.2 Upgrade Command

**Purpose:** Upgrade existing Fresher installation

```bash
# Upgrade to latest
.fresher/bin/fresher upgrade

# Upgrade to specific version
.fresher/bin/fresher upgrade --version=1.3.0

# Check for updates without installing
.fresher/bin/fresher upgrade --check
```

**Flags:**

| Flag | Description | Default |
|------|-------------|---------|
| `--version=X.Y.Z` | Upgrade to specific version | latest |
| `--check` | Only check for updates | false |
| `--dry-run` | Show what would change | false |

**Upgrade Process:**

1. Read current VERSION
2. Fetch target version manifest
3. Compare versions (skip if same unless --force)
4. Extract current config.sh values
5. Replace all core files
6. Generate new config.sh with preserved values
7. Update VERSION file
8. Print changelog summary

### 4.3 Version Check

**Purpose:** Display installed and available versions

```bash
.fresher/bin/fresher version

# Output:
# Installed: 1.2.3
# Latest:    1.3.0
# Upgrade available! Run: .fresher/bin/fresher upgrade
```

---

## 5. Configuration

| Variable | Type | Description | Default |
|----------|------|-------------|---------|
| `FRESHER_RELEASE_URL` | string | GitHub releases API URL | `https://api.github.com/repos/{org}/fresher/releases` |
| `FRESHER_RAW_URL` | string | Raw content URL for install script | `https://raw.githubusercontent.com/{org}/fresher` |

---

## 6. Security Considerations

### Script Verification

- Install script should be auditable (single file, readable)
- Consider adding checksum verification for releases
- Warn users about piping curl to bash (document alternative)

### Alternative Installation

```bash
# Download and inspect before running
curl -fsSL https://raw.githubusercontent.com/{org}/fresher/main/install.sh -o install.sh
cat install.sh  # Review the script
bash install.sh
```

### Permissions

- Install script should not require sudo
- All files installed with user permissions
- Executables set to 755 (run.sh, hooks, bin/*)

---

## 7. Implementation Phases

| Phase | Description | Dependencies | Complexity |
|-------|-------------|--------------|------------|
| 1 | Create install.sh with basic install from GitHub | GitHub repo setup | Medium |
| 2 | Add VERSION file tracking and version command | Phase 1 | Low |
| 3 | Implement upgrade command with config preservation | Phase 2 | High |
| 4 | Add --check, --dry-run, and --source flags | Phase 3 | Low |

---

## 8. Open Questions

- [x] Where should install.sh live? → Repository root, fetched via raw.githubusercontent.com
- [x] How to handle config value preservation? → Extract with grep/sed, regenerate template, inject values
- [ ] Should we support GitHub Enterprise URLs?
- [ ] Add GPG signature verification for releases?
