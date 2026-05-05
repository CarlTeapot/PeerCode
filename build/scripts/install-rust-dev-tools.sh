#!/usr/bin/env bash
set -euo pipefail

if ! command -v sccache >/dev/null 2>&1; then
  if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo is required to install sccache automatically."
    echo "Please install Rust toolchain (cargo) and run make again."
    exit 1
  fi

  echo "Installing sccache via cargo..."
  cargo install sccache
else
  echo "sccache already installed."
fi

if ! command -v mold >/dev/null 2>&1; then
  echo "mold not found. Skipping auto-install (no cross-platform cargo package)."
  echo "Build will continue without mold unless you install it manually."
else
  echo "mold already installed."
fi

echo "Rust dev tool check complete."
