#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

TARGET_TRIPLE="${1:-}"
OUTPUT_PATH="${2:-}"

if [ -z "$TARGET_TRIPLE" ] || [ -z "$OUTPUT_PATH" ]; then
  echo "Usage: $0 <target-triple> <output-path>" >&2
  exit 1
fi

case "$TARGET_TRIPLE" in
  x86_64-unknown-linux-gnu)   GOOS_DEFAULT="linux";   GOARCH_DEFAULT="amd64" ;;
  aarch64-unknown-linux-gnu)  GOOS_DEFAULT="linux";   GOARCH_DEFAULT="arm64" ;;
  x86_64-apple-darwin)        GOOS_DEFAULT="darwin";  GOARCH_DEFAULT="amd64" ;;
  aarch64-apple-darwin)       GOOS_DEFAULT="darwin";  GOARCH_DEFAULT="arm64" ;;
  x86_64-pc-windows-msvc)     GOOS_DEFAULT="windows"; GOARCH_DEFAULT="amd64" ;;
  aarch64-pc-windows-msvc)    GOOS_DEFAULT="windows"; GOARCH_DEFAULT="arm64" ;;
  *)
    echo "Unsupported target triple: $TARGET_TRIPLE" >&2
    exit 1
    ;;
esac

# Allow overrides for cross-compilation workflows.
GOOS_VALUE="${GOOS:-$GOOS_DEFAULT}"
GOARCH_VALUE="${GOARCH:-$GOARCH_DEFAULT}"
OUT_ABS="$REPO_ROOT/$OUTPUT_PATH"

mkdir -p "$(dirname "$OUT_ABS")"

echo "Building gateway for $TARGET_TRIPLE (GOOS=$GOOS_VALUE GOARCH=$GOARCH_VALUE)"
(
  cd "$REPO_ROOT/gateway"
  CGO_ENABLED=0 GOOS="$GOOS_VALUE" GOARCH="$GOARCH_VALUE" \
    go build -o "$OUT_ABS" ./cmd/server/
)

echo "Done: $OUT_ABS"
