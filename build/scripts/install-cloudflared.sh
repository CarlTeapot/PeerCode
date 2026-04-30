#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARIES_DIR="$SCRIPT_DIR/../../tauri-app/src-tauri/binaries"

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux*)
    case "$ARCH" in
      x86_64)        CF_OS="linux"; CF_ARCH="amd64"; TRIPLE="x86_64-unknown-linux-gnu";  EXT="" ;;
      aarch64|arm64) CF_OS="linux"; CF_ARCH="arm64"; TRIPLE="aarch64-unknown-linux-gnu"; EXT="" ;;
      *)  echo "Unsupported Linux/WSL architecture: $ARCH" >&2; exit 1 ;;
    esac
    ;;
  Darwin*)
    case "$ARCH" in
      x86_64) CF_OS="darwin"; CF_ARCH="amd64"; TRIPLE="x86_64-apple-darwin";  EXT="" ;;
      arm64)  CF_OS="darwin"; CF_ARCH="arm64"; TRIPLE="aarch64-apple-darwin"; EXT="" ;;
      *)  echo "Unsupported macOS architecture: $ARCH" >&2; exit 1 ;;
    esac
    ;;
  MINGW*|MSYS*|CYGWIN*)
    # Git Bash / MSYS2 on Windows
    case "$ARCH" in
      x86_64)  CF_OS="windows"; CF_ARCH="amd64"; TRIPLE="x86_64-pc-windows-msvc";  EXT=".exe" ;;
      aarch64) CF_OS="windows"; CF_ARCH="arm64"; TRIPLE="aarch64-pc-windows-msvc"; EXT=".exe" ;;
      *)  echo "Unsupported Windows architecture: $ARCH" >&2; exit 1 ;;
    esac
    ;;
  *)
    echo "Unsupported OS: $OS" >&2
    exit 1
    ;;
esac

DEST="$BINARIES_DIR/cloudflared-${TRIPLE}${EXT}"

if [ -f "$DEST" ]; then
  echo "cloudflared already present ($DEST) — skipping download."
  exit 0
fi

mkdir -p "$BINARIES_DIR"

# ── Download ──────────────────────────────────────────────────────────────────
URL="https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-${CF_OS}-${CF_ARCH}${EXT}"
echo "Downloading cloudflared for ${TRIPLE}..."
echo "  → $URL"
curl -fsSL "$URL" -o "$DEST"
chmod +x "$DEST"
echo "Done: $("$DEST" --version)"
