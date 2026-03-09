#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUT_DIR="${1:-deploy/cli/dist}"
OUT_DIR_FULL="$ROOT/$OUT_DIR"

cd "$ROOT"

VERSION="$(cargo pkgid -p hypertide-cli)"
VERSION="${VERSION##*#}"
VERSION="${VERSION##*@}"

cargo build -p hypertide-cli --bin ht --release

mkdir -p "$OUT_DIR_FULL"

STAGING="$OUT_DIR_FULL/hypertide-cli-$VERSION-linux-x86_64"
mkdir -p "$STAGING"
rm -f "$STAGING/ht"
cp "$ROOT/target/release/ht" "$STAGING/ht"

ARCHIVE="$OUT_DIR_FULL/hypertide-cli-$VERSION-linux-x86_64.tar.gz"
rm -f "$ARCHIVE"
tar -C "$STAGING" -czf "$ARCHIVE" ht

echo "packaged CLI artifact: $ARCHIVE"
