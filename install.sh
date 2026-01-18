#!/bin/bash
# Fresher Binary Installer
# Usage: curl -fsSL https://raw.githubusercontent.com/shanewwarren/fresher/main/install.sh | bash
#
# Options:
#   --version=X.Y.Z   Install specific version (default: latest)
#   --prefix=PATH     Installation prefix (default: /usr/local)
#   --check           Check if update is available without installing
#   --help, -h        Show help message

set -e

#──────────────────────────────────────────────────────────────────
# Colors
#──────────────────────────────────────────────────────────────────

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

#──────────────────────────────────────────────────────────────────
# Configuration
#──────────────────────────────────────────────────────────────────

GITHUB_REPO="shanewwarren/fresher"
INSTALL_VERSION=""
INSTALL_PREFIX="/usr/local"
CHECK_ONLY=false

#──────────────────────────────────────────────────────────────────
# Parse Arguments
#──────────────────────────────────────────────────────────────────

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version=*)
      INSTALL_VERSION="${1#*=}"
      shift
      ;;
    --prefix=*)
      INSTALL_PREFIX="${1#*=}"
      shift
      ;;
    --check)
      CHECK_ONLY=true
      shift
      ;;
    --help|-h)
      cat << EOF
Fresher Binary Installer

Usage: curl -fsSL https://raw.githubusercontent.com/shanewwarren/fresher/main/install.sh | bash

Options:
  --version=X.Y.Z   Install specific version (default: latest)
  --prefix=PATH     Installation prefix (default: /usr/local)
  --check           Check if update is available without installing
  --help, -h        Show this help message

Examples:
  # Install latest version
  curl -fsSL https://raw.githubusercontent.com/shanewwarren/fresher/main/install.sh | bash

  # Install specific version
  curl -fsSL ... | bash -s -- --version=2.0.0

  # Install to custom location
  curl -fsSL ... | bash -s -- --prefix=\$HOME/.local

  # Check for updates
  fresher upgrade --check
EOF
      exit 0
      ;;
    *)
      echo -e "${RED}Unknown option: $1${NC}" >&2
      exit 1
      ;;
  esac
done

#──────────────────────────────────────────────────────────────────
# Helper Functions
#──────────────────────────────────────────────────────────────────

log() {
  echo -e "${GREEN}[fresher]${NC} $*"
}

warn() {
  echo -e "${YELLOW}[fresher]${NC} $*"
}

error() {
  echo -e "${RED}[fresher]${NC} ERROR: $*" >&2
}

# Detect OS and architecture
detect_platform() {
  local os arch

  os=$(uname -s | tr '[:upper:]' '[:lower:]')
  arch=$(uname -m)

  case "$os" in
    darwin)
      os="apple-darwin"
      ;;
    linux)
      os="unknown-linux-gnu"
      ;;
    *)
      error "Unsupported OS: $os"
      exit 1
      ;;
  esac

  case "$arch" in
    x86_64|amd64)
      arch="x86_64"
      ;;
    arm64|aarch64)
      arch="aarch64"
      ;;
    *)
      error "Unsupported architecture: $arch"
      exit 1
      ;;
  esac

  echo "${arch}-${os}"
}

# Fetch latest version from GitHub API
get_latest_version() {
  local api_url="https://api.github.com/repos/${GITHUB_REPO}/releases/latest"
  local response

  if command -v curl &> /dev/null; then
    response=$(curl -fsSL "$api_url" 2>/dev/null) || return 1
  elif command -v wget &> /dev/null; then
    response=$(wget -qO- "$api_url" 2>/dev/null) || return 1
  else
    error "Neither curl nor wget available"
    return 1
  fi

  # Parse tag_name from JSON
  local version
  if command -v jq &> /dev/null; then
    version=$(echo "$response" | jq -r '.tag_name // empty')
  else
    version=$(echo "$response" | grep -o '"tag_name"[[:space:]]*:[[:space:]]*"[^"]*"' | sed 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/')
  fi

  # Strip leading 'v' if present
  version="${version#v}"
  echo "$version"
}

# Get currently installed version
get_installed_version() {
  if command -v fresher &> /dev/null; then
    fresher version 2>/dev/null | head -1 | sed 's/fresher v//' | awk '{print $1}'
  else
    echo ""
  fi
}

# Download binary from GitHub releases
download_binary() {
  local version="$1"
  local platform="$2"
  local dest_file="$3"
  local url="https://github.com/${GITHUB_REPO}/releases/download/v${version}/fresher-${platform}.tar.gz"

  log "Downloading fresher v${version} for ${platform}..."

  if command -v curl &> /dev/null; then
    curl -fsSL "$url" -o "$dest_file" || return 1
  elif command -v wget &> /dev/null; then
    wget -q "$url" -O "$dest_file" || return 1
  else
    error "Neither curl nor wget available"
    return 1
  fi
}

#──────────────────────────────────────────────────────────────────
# Main Installation
#──────────────────────────────────────────────────────────────────

echo ""
echo -e "${BLUE}╔════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                     Fresher Installer                          ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Detect platform
PLATFORM=$(detect_platform)
log "Detected platform: ${PLATFORM}"

# Determine version to install
if [[ -z "$INSTALL_VERSION" ]]; then
  log "Checking for latest version..."
  INSTALL_VERSION=$(get_latest_version) || {
    error "Could not fetch latest version from GitHub"
    echo ""
    echo "You can try specifying a version manually:"
    echo "  curl -fsSL ... | bash -s -- --version=2.0.0"
    exit 1
  }
fi

# Get currently installed version
CURRENT_VERSION=$(get_installed_version)

# Check only mode
if [[ "$CHECK_ONLY" == "true" ]]; then
  if [[ -z "$CURRENT_VERSION" ]]; then
    log "Fresher is not installed"
    log "Latest version: ${INSTALL_VERSION}"
  elif [[ "$CURRENT_VERSION" == "$INSTALL_VERSION" ]]; then
    log "Fresher is up to date (v${CURRENT_VERSION})"
  else
    log "Current version: ${CURRENT_VERSION}"
    log "Latest version:  ${INSTALL_VERSION}"
    echo ""
    echo "Run the following to upgrade:"
    echo "  fresher upgrade"
  fi
  exit 0
fi

log "Installing version: ${INSTALL_VERSION}"

if [[ -n "$CURRENT_VERSION" ]]; then
  if [[ "$CURRENT_VERSION" == "$INSTALL_VERSION" ]]; then
    log "Version ${INSTALL_VERSION} is already installed"
    exit 0
  fi
  log "Upgrading from v${CURRENT_VERSION} to v${INSTALL_VERSION}"
fi

# Create temp directory
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Download binary
TARBALL="$TEMP_DIR/fresher.tar.gz"
download_binary "$INSTALL_VERSION" "$PLATFORM" "$TARBALL" || {
  error "Failed to download binary"
  echo ""
  echo "Please check that:"
  echo "  1. Version ${INSTALL_VERSION} exists"
  echo "  2. Binary for ${PLATFORM} is available"
  echo ""
  echo "You can browse releases at:"
  echo "  https://github.com/${GITHUB_REPO}/releases"
  exit 1
}

# Extract binary
log "Extracting..."
tar -xzf "$TARBALL" -C "$TEMP_DIR"

# Verify binary exists
if [[ ! -f "$TEMP_DIR/fresher" ]]; then
  error "Binary not found in downloaded archive"
  exit 1
fi

# Install binary
INSTALL_DIR="${INSTALL_PREFIX}/bin"
INSTALL_PATH="${INSTALL_DIR}/fresher"

log "Installing to ${INSTALL_PATH}..."

# Check if we can write to install directory
if [[ ! -d "$INSTALL_DIR" ]]; then
  if ! mkdir -p "$INSTALL_DIR" 2>/dev/null; then
    warn "Cannot create ${INSTALL_DIR}, trying with sudo..."
    sudo mkdir -p "$INSTALL_DIR"
  fi
fi

if [[ -w "$INSTALL_DIR" ]]; then
  mv "$TEMP_DIR/fresher" "$INSTALL_PATH"
  chmod +x "$INSTALL_PATH"
else
  warn "Cannot write to ${INSTALL_DIR}, using sudo..."
  sudo mv "$TEMP_DIR/fresher" "$INSTALL_PATH"
  sudo chmod +x "$INSTALL_PATH"
fi

#──────────────────────────────────────────────────────────────────
# Verify Installation
#──────────────────────────────────────────────────────────────────

if command -v fresher &> /dev/null; then
  INSTALLED_VERSION=$(fresher version 2>/dev/null | head -1 | sed 's/fresher v//' | awk '{print $1}')
  echo ""
  echo -e "${GREEN}✓ Fresher v${INSTALLED_VERSION} installed successfully!${NC}"
else
  # Check if it's a PATH issue
  if [[ -x "$INSTALL_PATH" ]]; then
    echo ""
    echo -e "${GREEN}✓ Fresher v${INSTALL_VERSION} installed to ${INSTALL_PATH}${NC}"
    echo ""
    echo -e "${YELLOW}Note: ${INSTALL_DIR} may not be in your PATH${NC}"
    echo ""
    echo "Add to your shell config:"
    echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
  else
    error "Installation failed"
    exit 1
  fi
fi

#──────────────────────────────────────────────────────────────────
# Next Steps
#──────────────────────────────────────────────────────────────────

echo ""
echo "Next steps:"
echo "  1. Navigate to your project directory"
echo "  2. Run: ${CYAN}fresher init${NC}"
echo "  3. Add specifications to specs/"
echo "  4. Run: ${CYAN}fresher plan${NC}"
echo ""
echo "For help:"
echo "  fresher --help"
echo ""
