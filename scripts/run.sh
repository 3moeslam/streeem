#!/usr/bin/env bash
# Build and run streeem locally.
#
# Usage:
#   ./scripts/run.sh                     # default sample: shell + demo tile
#   ./scripts/run.sh 'cmd1' 'cmd2' ...   # pass-through to streeem
#   ./scripts/run.sh --release 'cmd1'    # build with release profile (faster, slower to compile)
#
# Defaults:
#   - Debug build (fast iteration). Use --release for the optimised binary.
#   - Two tiles: bash + a labeled ticking demo, side-by-side.
#
# Exits non-zero on build failure or if the binary can't be located.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

PROFILE_FLAG=""
PROFILE_DIR="debug"
if [[ "${1:-}" == "--release" ]]; then
  PROFILE_FLAG="--release"
  PROFILE_DIR="release"
  shift
fi

echo ">> building streeem ($PROFILE_DIR profile)..."
cargo build -p streeem $PROFILE_FLAG

BIN="$REPO_ROOT/target/$PROFILE_DIR/streeem"
if [[ ! -x "$BIN" ]]; then
  echo "!! binary not found at $BIN" >&2
  exit 1
fi

if [[ $# -eq 0 ]]; then
  echo ">> launching streeem with two sample tiles"
  echo "   (Esc Esc to enter command mode  •  q to quit  •  log: /tmp/streeem.log)"
  exec "$BIN" \
    --name shell "${SHELL:-bash} -i" \
    --name demo  "$REPO_ROOT/scripts/demo-tile.sh demo"
else
  echo ">> launching streeem with custom args"
  exec "$BIN" "$@"
fi
