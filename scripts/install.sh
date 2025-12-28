#!/usr/bin/env bash
# Installation script for noslop
# Downloads and installs the latest release from GitHub

set -e

# Configuration
REPO="noslop-sh/noslop"
BINARY_NAME="noslop"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Detect OS and architecture
detect_platform() {
    local os
    local arch

    case "$(uname -s)" in
        Linux*)     os="linux";;
        Darwin*)    os="macos";;
        MINGW*|MSYS*|CYGWIN*) os="windows";;
        *)          echo -e "${RED}Unsupported OS$(uname -s)${NC}"; exit 1;;
    esac

    case "$(uname -m)" in
        x86_64|amd64) arch="x86_64";;
        aarch64|arm64) arch="aarch64";;
        *)          echo -e "${RED}Unsupported architecture: $(uname -m)${NC}"; exit 1;;
    esac

    echo "${os}-${arch}"
}

# Get latest release version
get_latest_version() {
    curl -s "https://api.github.com/repos/${REPO}/releases/latest" \
        | grep '"tag_name":' \
        | sed -E 's/.*"([^"]+)".*/\1/'
}

# Download and install
main() {
    echo -e "${GREEN}Installing noslop...${NC}"
    echo ""

    # Detect platform
    local platform=$(detect_platform)
    echo "Platform: ${platform}"

    # Get latest version
    echo "Fetching latest release..."
    local version=$(get_latest_version)

    if [ -z "$version" ]; then
        echo -e "${RED}Failed to fetch latest version${NC}"
        exit 1
    fi

    echo "Latest version: ${version}"

    # Construct download URL
    local ext="tar.gz"
    if [[ "$platform" == *"windows"* ]]; then
        ext="zip"
    fi

    local asset_name="${BINARY_NAME}-${platform}"
    local download_url="https://github.com/${REPO}/releases/download/${version}/${asset_name}.${ext}"

    echo "Downloading from: ${download_url}"
    echo ""

    # Create temp directory
    local tmp_dir=$(mktemp -d)
    cd "$tmp_dir"

    # Download
    if ! curl -sL "$download_url" -o "noslop.${ext}"; then
        echo -e "${RED}Failed to download${NC}"
        echo -e "${YELLOW}Falling back to cargo install...${NC}"
        cargo install noslop
        exit 0
    fi

    # Extract
    if [[ "$ext" == "zip" ]]; then
        unzip -q "noslop.${ext}"
    else
        tar xzf "noslop.${ext}"
    fi

    # Make executable
    chmod +x "$BINARY_NAME"

    # Create install directory if it doesn't exist
    mkdir -p "$INSTALL_DIR"

    # Install
    mv "$BINARY_NAME" "$INSTALL_DIR/"

    # Cleanup
    cd -
    rm -rf "$tmp_dir"

    echo ""
    echo -e "${GREEN}✓ noslop installed successfully!${NC}"
    echo ""
    echo "Installation location: $INSTALL_DIR/$BINARY_NAME"
    echo ""

    # Check if install dir is in PATH
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        echo -e "${YELLOW}Warning: $INSTALL_DIR is not in your PATH${NC}"
        echo ""
        echo "Add it to your PATH by adding this to your shell profile:"
        echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
        echo ""
    fi

    echo "Verify installation:"
    echo "  $BINARY_NAME --version"
    echo ""
    echo "Get started:"
    echo "  $BINARY_NAME --help"
}

main "$@"
