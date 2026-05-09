#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --features test-support -- -D warnings
cargo test --workspace --features test-support
cargo llvm-cov --workspace --features test-support --fail-under-lines 75
