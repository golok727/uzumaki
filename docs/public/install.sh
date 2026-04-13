#!/bin/sh
# Uzumaki installer — downloads the latest release binary from GitHub.
# Usage: curl -fsSL https://uzumaki.dev/install.sh | sh

set -e

REPO="golok727/uzumaki"
INSTALL_DIR="${UZUMAKI_INSTALL:-$HOME/.uzumaki/bin}"

# Detect OS
case "$(uname -s)" in
  Darwin) OS="macos" ;;
  Linux)  OS="linux" ;;
  *)      echo "error: unsupported OS: $(uname -s)"; exit 1 ;;
esac

# Detect architecture
case "$(uname -m)" in
  x86_64|amd64)  ARCH="x64" ;;
  arm64|aarch64)  ARCH="arm64" ;;
  *)              echo "error: unsupported architecture: $(uname -m)"; exit 1 ;;
esac

ASSET="uzumaki-${OS}-${ARCH}.zip"

# Fetch latest version tag
if [ -z "$UZUMAKI_VERSION" ]; then
  VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')
  if [ -z "$VERSION" ]; then
    echo "error: could not determine latest version"
    exit 1
  fi
else
  VERSION="v${UZUMAKI_VERSION#v}"
fi

URL="https://github.com/${REPO}/releases/download/${VERSION}/${ASSET}"

echo ""
echo "  \033[1;38;5;75mUzumaki\033[0m installer"
echo ""
echo "  Version:  ${VERSION}"
echo "  Platform: ${OS}-${ARCH}"
echo "  Install:  ${INSTALL_DIR}"
echo ""

# Download and extract
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

echo "  Downloading ${URL}..."
curl -fsSL "$URL" -o "$TMP_DIR/$ASSET"

echo "  Extracting..."
unzip -qo "$TMP_DIR/$ASSET" -d "$TMP_DIR"

# Install
mkdir -p "$INSTALL_DIR"
mv "$TMP_DIR/uzumaki" "$INSTALL_DIR/uzumaki"
chmod +x "$INSTALL_DIR/uzumaki"

echo ""
echo "  \033[32mUzumaki was installed successfully!\033[0m"
echo ""

# Check if already in PATH
case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *)
    echo "  Add Uzumaki to your PATH by adding this to your shell profile:"
    echo ""
    echo "    export PATH=\"${INSTALL_DIR}:\$PATH\""
    echo ""
    # Try to detect shell and suggest the right file
    SHELL_NAME=$(basename "$SHELL" 2>/dev/null || echo "")
    case "$SHELL_NAME" in
      zsh)  echo "  e.g. add it to ~/.zshrc" ;;
      bash) echo "  e.g. add it to ~/.bashrc" ;;
      fish) echo "  or run: set -Ux fish_user_paths ${INSTALL_DIR} \$fish_user_paths" ;;
    esac
    echo ""
    ;;
esac
