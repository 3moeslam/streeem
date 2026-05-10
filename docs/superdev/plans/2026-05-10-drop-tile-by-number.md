# Plan: Drop Tile by Number (`k` in command mode)

## Goal
Add a `k` keybinding in command mode that opens a numbered prompt, accepting a tile's display number and dropping that tile — reusing the existing `PromptPurpose` enum with a new `DropTileByIndex` variant.

## Best-Practice Brief
- **Extend a sum type (enum) for new behavior** — OCP: open for extension, closed for modification of callers. Adding a variant exhaustively handled in `match` causes compile-time errors on missed cases. Aligns with DDD (typed intent), Modularity (variant lives where `PromptPurpose` lives), and Clean Code (intention-revealing name).
- **Reuse existing `PromptState` infrastructure** — SRP preserved; prompt logic stays in one place; no new types introduced.

## Assumptions
- `tiles` in `RenderSnapshot` are in display order (confirmed: `query.rs` iterates `state.grid.tiles` in insertion order, `focus_index` i+1).
- `TileId::default_from` is available under `test-support` feature (confirmed in `tile_id.rs`).
- `Command::DropTile(TileId)` already exists (confirmed in `command.rs` usage in runtime).

## Risks & Mitigations
- **`DropTileByIndex` clones `Vec<TileId>` on `std::mem::take`** — fine; it's a short-lived prompt state, not hot path.
- **Lint `unwrap_used`/`expect_used`/`panic`** — `parse::<usize>()` uses `match`, no `unwrap`. Tests use `#![cfg_attr(test, allow(clippy::panic))]` already present.

## Affected Files
- `crates/streeem-presentation/src/prompt.rs` — **modified** (new variant, method, label arm, Enter arm, 3 tests)
- `crates/streeem-presentation/src/key_map.rs` — **modified** (STATUS_BAR_TEXT_COMMAND constant)
- `crates/streeem-bin/src/runtime.rs` — **modified** (`'k'` arm in command-mode match)
- `Cargo.toml` — **modified** (version 0.2.7 → 0.2.8)

## Parallelism Evaluation
Single-track. All four changes are in three crates that have a dependency chain (`streeem-bin` depends on `streeem-presentation`); the prompt change must compile before the runtime change references the new method. Fewer than 3 steps per isolated group — worktree overhead exceeds any gain.

## Change Sequence

- [x] **S1** Extend `PromptPurpose` + add `open_for_drop` + update `label()` + update Enter handler in `prompt.rs` + add 3 tests (RED → GREEN → REFACTOR)
- [x] **S2** Update `STATUS_BAR_TEXT_COMMAND` in `key_map.rs`
- [x] **S3** Wire `'k'` arm in `runtime.rs`
- [x] **S4** Bump version in `Cargo.toml` to 0.2.8
- [x] **S5** `cargo build --workspace` + `cargo test --workspace --features test-support` + `cargo fmt --all -- --check` + `cargo clippy` pass
- [x] **S6** Commit

## Test Strategy
- `open_for_drop_sets_drop_purpose`: type "2", Enter → `Submitted(Command::DropTile(tiles[1]))`
- `drop_with_invalid_input_cancels`: "abc" → `Cancelled`; "99" → `Cancelled`
- `label_returns_drop_for_drop_purpose`: `label()` == "drop"

## Edge Cases
- Empty tile list: `open_for_drop` only called when `!tiles_in_order.is_empty()`.
- Out-of-range number (e.g., "99"): `n <= tiles.len()` guard → `Cancelled`.
- Non-numeric input ("abc"): `parse::<usize>()` returns `Err` → `Cancelled`.
- "0": fails `n >= 1` guard → `Cancelled`.
- Esc still cancels the drop prompt (handled by existing Esc arm).

## Performance Notes
`Vec<TileId>` clone is O(n tiles) — negligible for any real terminal session.

## Security Notes
No untrusted external input; keystrokes come from local terminal.

## Permissions Required
- `cargo build --workspace`
- `cargo test --workspace --features test-support`
- `cargo fmt --all` and `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --features test-support -- -D warnings`
- `./scripts/coverage.sh` (if it exists)
- `git add`, `git commit`

## Rollback Plan
Single commit; `git revert` removes all changes cleanly.
