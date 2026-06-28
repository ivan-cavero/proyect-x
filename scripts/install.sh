#!/bin/bash
# install.sh — Install project-x on Linux/macOS

set -e

REPO="ivan-cavero/project-x"
VERSION="latest"
INSTALL_DIR="/usr/local/bin"

echo "🔧 Installing Project-X..."

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
    linux)  PLATFORM="linux" ;;
    darwin) PLATFORM="macos" ;;
    *)      echo "❌ Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
    x86_64)  ARCH_NAME="x86_64" ;;
    aarch64) ARCH_NAME="aarch64" ;;
    *)       echo "❌ Unsupported architecture: $ARCH"; exit 1 ;;
esac

BINARY="project-x-${PLATFORM}-${ARCH_NAME}"
URL="https://github.com/${REPO}/releases/latest/download/${BINARY}"

echo "  Platform: ${PLATFORM} ${ARCH_NAME}"
echo "  Downloading: ${URL}"

# Download
curl -fsSL "$URL" -o "/tmp/project-x"

# Make executable
chmod +x "/tmp/project-x"

# Install
if [ -w "$INSTALL_DIR" ]; then
    mv "/tmp/project-x" "${INSTALL_DIR}/project-x"
else
    sudo mv "/tmp/project-x" "${INSTALL_DIR}/project-x"
fi

# Verify
echo ""
echo "✅ Project-X installed successfully!"
echo ""
project-x --version
echo ""
echo "  Next steps:"
echo "    project-x init my-project"
echo "    project-x run --goal \"your goal here\""