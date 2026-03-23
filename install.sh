#!/bin/sh
# Doubleword CLI installer
# Usage: curl -fsSL https://doubleword.ai/install.sh | sh
set -e

REPO="doublewordai/dw"
BINARY_NAME="dw"

# Colors (if terminal supports it)
if [ -t 1 ]; then
    BOLD='\033[1m'
    GREEN='\033[0;32m'
    RED='\033[0;31m'
    RESET='\033[0m'
else
    BOLD=''
    GREEN=''
    RED=''
    RESET=''
fi

info() {
    printf "${BOLD}%s${RESET}\n" "$1"
}

success() {
    printf "${GREEN}%s${RESET}\n" "$1"
}

error() {
    printf "${RED}Error: %s${RESET}\n" "$1" >&2
    exit 1
}

# Detect platform
detect_platform() {
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    ARCH=$(uname -m)

    case "$OS" in
        linux) OS="linux" ;;
        darwin) OS="darwin" ;;
        *) error "Unsupported operating system: $OS" ;;
    esac

    case "$ARCH" in
        x86_64 | amd64) ARCH="amd64" ;;
        aarch64 | arm64) ARCH="arm64" ;;
        *) error "Unsupported architecture: $ARCH" ;;
    esac

    PLATFORM="${OS}-${ARCH}"
}

# Get the latest release version from GitHub
get_latest_version() {
    if command -v curl >/dev/null 2>&1; then
        VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"v?([^"]+)".*/\1/')
    elif command -v wget >/dev/null 2>&1; then
        VERSION=$(wget -qO- "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"v?([^"]+)".*/\1/')
    else
        error "Neither curl nor wget found. Please install one of them."
    fi

    if [ -z "$VERSION" ]; then
        error "Could not determine latest version. Check https://github.com/${REPO}/releases"
    fi
}

# Download the binary
download() {
    DOWNLOAD_URL="https://github.com/${REPO}/releases/download/v${VERSION}/${BINARY_NAME}-${PLATFORM}"
    CHECKSUM_URL="https://github.com/${REPO}/releases/download/v${VERSION}/checksums.txt"

    info "Downloading dw v${VERSION} for ${PLATFORM}..."

    TMP_DIR=$(mktemp -d)
    TMP_FILE="${TMP_DIR}/${BINARY_NAME}"

    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$DOWNLOAD_URL" -o "$TMP_FILE"
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$DOWNLOAD_URL" -O "$TMP_FILE"
    fi

    if [ ! -f "$TMP_FILE" ]; then
        error "Download failed. Check https://github.com/${REPO}/releases"
    fi

    chmod +x "$TMP_FILE"

    # Verify checksum if sha256sum is available
    if command -v sha256sum >/dev/null 2>&1; then
        CHECKSUM_FILE="${TMP_DIR}/checksums.txt"
        if command -v curl >/dev/null 2>&1; then
            curl -fsSL "$CHECKSUM_URL" -o "$CHECKSUM_FILE" 2>/dev/null || true
        fi
        if [ -f "$CHECKSUM_FILE" ]; then
            EXPECTED=$(grep "${BINARY_NAME}-${PLATFORM}" "$CHECKSUM_FILE" | awk '{print $1}')
            ACTUAL=$(sha256sum "$TMP_FILE" | awk '{print $1}')
            if [ -n "$EXPECTED" ] && [ "$EXPECTED" != "$ACTUAL" ]; then
                error "Checksum verification failed!"
            fi
        fi
    fi
}

# Install the binary
install() {
    # Prefer ~/.local/bin (no sudo needed), fall back to /usr/local/bin
    if [ -d "$HOME/.local/bin" ] || mkdir -p "$HOME/.local/bin" 2>/dev/null; then
        INSTALL_DIR="$HOME/.local/bin"
    elif [ -w "/usr/local/bin" ]; then
        INSTALL_DIR="/usr/local/bin"
    else
        info "Installing to /usr/local/bin (requires sudo)..."
        sudo mv "$TMP_FILE" "/usr/local/bin/${BINARY_NAME}"
        INSTALL_DIR="/usr/local/bin"
        rm -rf "$TMP_DIR"
        return
    fi

    mv "$TMP_FILE" "${INSTALL_DIR}/${BINARY_NAME}"
    rm -rf "$TMP_DIR"

    # Check if install dir is in PATH
    case ":$PATH:" in
        *":${INSTALL_DIR}:"*) ;;
        *)
            echo ""
            info "Add ${INSTALL_DIR} to your PATH:"
            echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
            echo ""
            echo "Add this line to your ~/.bashrc, ~/.zshrc, or shell config."
            ;;
    esac
}

main() {
    echo ""
    echo "  ╔══════════════════════════════════════╗"
    echo "  ║     DOUBLEWORD BATCH INFERENCE       ║"
    echo "  ║              CLI                     ║"
    echo "  ╚══════════════════════════════════════╝"
    echo ""

    detect_platform
    get_latest_version
    download
    install

    echo ""
    success "Installed dw v${VERSION} to ${INSTALL_DIR}/${BINARY_NAME}"
    echo ""
    info "Get started:"
    echo "  dw login              # Authenticate via browser"
    echo "  dw login --api-key    # Authenticate with an API key"
    echo "  dw examples list      # Browse example use-cases"
    echo "  dw --help             # See all commands"
    echo ""
}

main
