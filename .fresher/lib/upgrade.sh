#!/bin/bash
# Fresher Upgrade Library
# Functions for version management and config preservation during upgrades

set -e

#──────────────────────────────────────────────────────────────────
# Configuration
#──────────────────────────────────────────────────────────────────

UPGRADE_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
UPGRADE_FRESHER_DIR="$(cd "$UPGRADE_SCRIPT_DIR/.." && pwd)"
UPGRADE_PROJECT_DIR="$(cd "$UPGRADE_FRESHER_DIR/.." && pwd)"

# GitHub repository for releases
FRESHER_GITHUB_REPO="${FRESHER_GITHUB_REPO:-fresher/fresher}"

#──────────────────────────────────────────────────────────────────
# Helper Functions
#──────────────────────────────────────────────────────────────────

upgrade_log() {
  echo "[upgrade] $*"
}

upgrade_error() {
  echo "[upgrade] ERROR: $*" >&2
}

#──────────────────────────────────────────────────────────────────
# Version Functions
#──────────────────────────────────────────────────────────────────

# Get installed version from VERSION file
# Usage: get_installed_version [fresher_dir]
get_installed_version() {
  local fresher_dir="${1:-$UPGRADE_FRESHER_DIR}"
  local version_file="$fresher_dir/VERSION"

  if [[ -f "$version_file" ]]; then
    cat "$version_file" | tr -d '[:space:]'
  else
    echo "unknown"
  fi
}

# Get latest version from GitHub releases API
# Usage: get_latest_version
get_latest_version() {
  local api_url="https://api.github.com/repos/${FRESHER_GITHUB_REPO}/releases/latest"

  # Try curl first, then wget
  local response
  if command -v curl &> /dev/null; then
    response=$(curl -fsSL "$api_url" 2>/dev/null) || {
      upgrade_error "Failed to fetch latest version from GitHub"
      return 1
    }
  elif command -v wget &> /dev/null; then
    response=$(wget -qO- "$api_url" 2>/dev/null) || {
      upgrade_error "Failed to fetch latest version from GitHub"
      return 1
    }
  else
    upgrade_error "Neither curl nor wget available"
    return 1
  fi

  # Parse tag_name from JSON (strip leading 'v' if present)
  local version
  if command -v jq &> /dev/null; then
    version=$(echo "$response" | jq -r '.tag_name // empty')
  else
    # Fallback: basic grep/sed parsing
    version=$(echo "$response" | grep -o '"tag_name"[[:space:]]*:[[:space:]]*"[^"]*"' | sed 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/')
  fi

  # Strip leading 'v' if present
  version="${version#v}"

  if [[ -z "$version" ]]; then
    upgrade_error "Could not parse version from GitHub response"
    return 1
  fi

  echo "$version"
}

# Compare two semantic versions
# Returns: 0 if v1 == v2, 1 if v1 > v2, 2 if v1 < v2
# Usage: compare_versions "1.2.3" "1.2.4"
compare_versions() {
  local v1="$1"
  local v2="$2"

  # Handle 'unknown' version
  if [[ "$v1" == "unknown" ]]; then
    echo "2"
    return 0
  fi
  if [[ "$v2" == "unknown" ]]; then
    echo "1"
    return 0
  fi

  # Split into arrays
  IFS='.' read -ra v1_parts <<< "$v1"
  IFS='.' read -ra v2_parts <<< "$v2"

  # Pad arrays to same length
  local max_len=${#v1_parts[@]}
  [[ ${#v2_parts[@]} -gt $max_len ]] && max_len=${#v2_parts[@]}

  for ((i = 0; i < max_len; i++)); do
    local n1="${v1_parts[i]:-0}"
    local n2="${v2_parts[i]:-0}"

    # Strip any non-numeric suffix (e.g., "1-beta" -> "1")
    n1="${n1%%[^0-9]*}"
    n2="${n2%%[^0-9]*}"

    # Default to 0 if empty
    n1="${n1:-0}"
    n2="${n2:-0}"

    if [[ "$n1" -gt "$n2" ]]; then
      echo "1"
      return 0
    elif [[ "$n1" -lt "$n2" ]]; then
      echo "2"
      return 0
    fi
  done

  echo "0"
}

#──────────────────────────────────────────────────────────────────
# Config Extraction/Restoration
#──────────────────────────────────────────────────────────────────

# Extract config values from config.sh
# Output format: VAR_NAME=value (one per line)
# This extracts the default values set in the config file
# Usage: extract_config_values [config_file]
extract_config_values() {
  local config_file="${1:-$UPGRADE_FRESHER_DIR/config.sh}"

  if [[ ! -f "$config_file" ]]; then
    upgrade_error "Config file not found: $config_file"
    return 1
  fi

  # Pattern: export FRESHER_VAR="${FRESHER_VAR:-value}"
  # Extract VAR and value pairs
  grep -E '^export FRESHER_[A-Z_]+=' "$config_file" 2>/dev/null | while read -r line; do
    # Extract variable name
    local var_name
    var_name=$(echo "$line" | sed -n 's/^export \(FRESHER_[A-Z_]*\)=.*/\1/p')

    # Extract default value (between :- and }")
    local value
    value=$(echo "$line" | sed -n 's/.*:-\(.*\)}"/\1/p')

    # If no :- pattern, try to get direct value
    if [[ -z "$value" ]]; then
      value=$(echo "$line" | sed -n 's/^export FRESHER_[A-Z_]*="\([^"]*\)"/\1/p')
    fi

    if [[ -n "$var_name" && -n "$value" ]]; then
      echo "${var_name}=${value}"
    fi
  done
}

# Restore config values into a new config.sh file
# Reads VAR=value pairs from stdin or a file
# Usage: restore_config_values new_config_file < preserved_values
#    or: restore_config_values new_config_file preserved_values_file
restore_config_values() {
  local config_file="$1"
  local values_file="$2"

  if [[ ! -f "$config_file" ]]; then
    upgrade_error "Config file not found: $config_file"
    return 1
  fi

  # Read values from file or stdin
  local values
  if [[ -n "$values_file" && -f "$values_file" ]]; then
    values=$(cat "$values_file")
  else
    values=$(cat)
  fi

  # Process each saved value
  echo "$values" | while IFS='=' read -r var_name value; do
    [[ -z "$var_name" ]] && continue

    # Escape special characters in value for sed
    local escaped_value
    escaped_value=$(printf '%s\n' "$value" | sed 's/[&/\]/\\&/g')

    # Replace the default value in the config file
    # Pattern: ${VAR:-old_value}" -> ${VAR:-new_value}"
    if grep -q "^export ${var_name}=" "$config_file" 2>/dev/null; then
      sed -i.bak "s|\(export ${var_name}=\"\${${var_name}:-\)[^}]*\(}\"\)|\1${escaped_value}\2|" "$config_file"
      rm -f "${config_file}.bak"
    fi
  done
}

# Save config values to a temporary file
# Returns the path to the temp file
# Usage: save_config_values [config_file]
save_config_values() {
  local config_file="${1:-$UPGRADE_FRESHER_DIR/config.sh}"
  local temp_file
  temp_file=$(mktemp)

  extract_config_values "$config_file" > "$temp_file"
  echo "$temp_file"
}

#──────────────────────────────────────────────────────────────────
# File Classification
#──────────────────────────────────────────────────────────────────

# Core files that should be replaced during upgrade
CORE_FILES=(
  "run.sh"
  "PROMPT.planning.md"
  "PROMPT.building.md"
  "VERSION"
)

CORE_DIRS=(
  "lib"
  "bin"
  "docker"
  "tests"
)

# Custom files that should be preserved during upgrade
CUSTOM_FILES=(
  "AGENTS.md"
  ".state"
)

CUSTOM_DIRS=(
  "hooks"
  "logs"
)

# Mixed files - preserve values, replace template
MIXED_FILES=(
  "config.sh"
)

# Check if a path is a core file/directory
# Usage: is_core_path "path/to/file"
is_core_path() {
  local path="$1"
  local basename
  basename=$(basename "$path")

  for f in "${CORE_FILES[@]}"; do
    [[ "$basename" == "$f" ]] && return 0
  done

  for d in "${CORE_DIRS[@]}"; do
    [[ "$path" == "$d" || "$path" == "$d/"* ]] && return 0
  done

  return 1
}

# Check if a path is a custom file/directory
# Usage: is_custom_path "path/to/file"
is_custom_path() {
  local path="$1"
  local basename
  basename=$(basename "$path")

  for f in "${CUSTOM_FILES[@]}"; do
    [[ "$basename" == "$f" ]] && return 0
  done

  for d in "${CUSTOM_DIRS[@]}"; do
    [[ "$path" == "$d" || "$path" == "$d/"* ]] && return 0
  done

  return 1
}

# Check if a path is a mixed file
# Usage: is_mixed_path "path/to/file"
is_mixed_path() {
  local path="$1"
  local basename
  basename=$(basename "$path")

  for f in "${MIXED_FILES[@]}"; do
    [[ "$basename" == "$f" ]] && return 0
  done

  return 1
}

#──────────────────────────────────────────────────────────────────
# Download Functions
#──────────────────────────────────────────────────────────────────

# Download a release tarball from GitHub
# Usage: download_release version dest_file
download_release() {
  local version="$1"
  local dest_file="$2"

  # Construct download URL
  local url="https://github.com/${FRESHER_GITHUB_REPO}/archive/refs/tags/v${version}.tar.gz"

  upgrade_log "Downloading version ${version}..."

  if command -v curl &> /dev/null; then
    curl -fsSL "$url" -o "$dest_file" || {
      upgrade_error "Failed to download release"
      return 1
    }
  elif command -v wget &> /dev/null; then
    wget -q "$url" -O "$dest_file" || {
      upgrade_error "Failed to download release"
      return 1
    }
  else
    upgrade_error "Neither curl nor wget available"
    return 1
  fi

  upgrade_log "Download complete"
}

# Extract a tarball to a directory
# Usage: extract_tarball tarball_file dest_dir
extract_tarball() {
  local tarball="$1"
  local dest_dir="$2"

  mkdir -p "$dest_dir"
  tar -xzf "$tarball" -C "$dest_dir" --strip-components=1
}

#──────────────────────────────────────────────────────────────────
# Upgrade Operations
#──────────────────────────────────────────────────────────────────

# Perform the upgrade
# Usage: perform_upgrade source_dir target_dir [dry_run]
perform_upgrade() {
  local source_dir="$1"
  local target_dir="$2"
  local dry_run="${3:-false}"

  # Ensure source has a .fresher directory
  local source_fresher="$source_dir"
  if [[ -d "$source_dir/.fresher" ]]; then
    source_fresher="$source_dir/.fresher"
  fi

  if [[ ! -f "$source_fresher/run.sh" ]]; then
    upgrade_error "Invalid source: no run.sh found"
    return 1
  fi

  # Save current config values
  local saved_config=""
  if [[ -f "$target_dir/config.sh" ]]; then
    saved_config=$(save_config_values "$target_dir/config.sh")
    upgrade_log "Saved config values to $saved_config"
  fi

  # Copy core files
  for file in "${CORE_FILES[@]}"; do
    if [[ -f "$source_fresher/$file" ]]; then
      if [[ "$dry_run" == "true" ]]; then
        upgrade_log "[dry-run] Would copy: $file"
      else
        cp "$source_fresher/$file" "$target_dir/$file"
        upgrade_log "Updated: $file"
      fi
    fi
  done

  # Copy core directories
  for dir in "${CORE_DIRS[@]}"; do
    if [[ -d "$source_fresher/$dir" ]]; then
      if [[ "$dry_run" == "true" ]]; then
        upgrade_log "[dry-run] Would replace directory: $dir/"
      else
        rm -rf "$target_dir/$dir"
        cp -r "$source_fresher/$dir" "$target_dir/$dir"
        upgrade_log "Updated: $dir/"
      fi
    fi
  done

  # Handle mixed files (config.sh)
  for file in "${MIXED_FILES[@]}"; do
    if [[ -f "$source_fresher/$file" ]]; then
      if [[ "$dry_run" == "true" ]]; then
        upgrade_log "[dry-run] Would update $file (preserving values)"
      else
        cp "$source_fresher/$file" "$target_dir/$file"
        if [[ -n "$saved_config" && -f "$saved_config" ]]; then
          restore_config_values "$target_dir/$file" "$saved_config"
        fi
        upgrade_log "Updated: $file (values preserved)"
      fi
    fi
  done

  # Clean up saved config
  if [[ -n "$saved_config" && -f "$saved_config" ]]; then
    rm -f "$saved_config"
  fi

  upgrade_log "Upgrade complete"
}
