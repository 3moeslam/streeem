#!/usr/bin/env bash
# Build a release binary of streeem and install it into a user-bin directory.
#
# Usage:
#   ./scripts/install-local.sh             # install to ~/.local/bin (no sudo)
#   ./scripts/install-local.sh --system    # install to /usr/local/bin (uses sudo)
#   ./scripts/install-local.sh --prefix /opt/homebrew/bin
#
# After install, ensure the install directory is on your PATH:
#   export PATH="$HOME/.local/bin:$PATH"   # for the user-bin default
#
# Verifies the install by running `streeem --version` from the new location.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

INSTALL_DIR="$HOME/.local/bin"
NEEDS_SUDO=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --system)
      INSTALL_DIR="/usr/local/bin"
      NEEDS_SUDO="sudo"
      shift
      ;;
    --prefix)
      INSTALL_DIR="$2"
      shift 2
      ;;
    -h|--help)
      sed -n '2,15p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "!! unknown argument: $1" >&2
      exit 64
      ;;
  esac
done

if ! command -v cargo >/dev/null 2>&1; then
  cat >&2 <<'EOF'
!! cargo not found. Install the Rust toolchain first:
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
EOF
  exit 1
fi

echo ">> building streeem (release profile)..."
cargo build -p streeem --release

SRC="$REPO_ROOT/target/release/streeem"
if [[ ! -x "$SRC" ]]; then
  echo "!! built binary missing at $SRC" >&2
  exit 1
fi

echo ">> installing $SRC -> $INSTALL_DIR/streeem"
$NEEDS_SUDO mkdir -p "$INSTALL_DIR"
$NEEDS_SUDO install -m 0755 "$SRC" "$INSTALL_DIR/streeem"

echo ">> verifying install"
if ! "$INSTALL_DIR/streeem" --version >/dev/null 2>&1; then
  echo "!! installed binary failed to run" >&2
  exit 1
fi

echo ""
echo "✓ installed: $INSTALL_DIR/streeem ($("$INSTALL_DIR/streeem" --version))"
echo ""
if ! command -v streeem >/dev/null 2>&1; then
  cat <<EOF
ℹ  '$INSTALL_DIR' is not on your PATH yet. Add it to your shell rc:

      echo 'export PATH="$INSTALL_DIR:\$PATH"' >> ~/.zshrc
      source ~/.zshrc

EOF
fi

echo "Try it:    streeem --name shell '\${SHELL:-bash} -i'"
