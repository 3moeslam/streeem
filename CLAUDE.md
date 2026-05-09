# Streeem

A Rust CLI desktop that hosts multiple terminals inside its own terminal area.
Hosted terminals are color-coded and arranged in a staggered grid layout — the
workspace behaves like a "desktop of terminals" inside a single terminal window.

## Tech

- Language: Rust, edition 2024
- Layout: **Cargo workspace, one crate per Clean Architecture layer**.
- Dependencies: none yet. Add only what is necessary; justify each addition in
  the commit/PR message.

## Architecture: Clean Architecture + DDD

Four layers, dependencies point **inward only**. The workspace enforces this
through `Cargo.toml` `[dependencies]` — the compiler refuses any inward leak.

```
streeem/
  Cargo.toml                 # [workspace]
  crates/
    streeem-domain/          # pure: no tokio, no I/O, no UI types
    streeem-application/     # deps: streeem-domain
    streeem-infrastructure/  # deps: streeem-domain, streeem-application
    streeem-presentation/    # deps: all of the above
    streeem-bin/             # composition root (main.rs); deps: all
```

Layer responsibilities:

1. **Domain** — entities, value objects, aggregates, domain events, port
   traits, domain services. Pure logic, fully synchronous, no `tokio`,
   no `crossterm`, no `std::process`.
2. **Application** — use cases / command handlers that orchestrate the domain
   through port traits. No direct I/O; calls go through ports.
3. **Infrastructure** — adapters: PTY spawning, OS calls, terminal output,
   external (exp) server client. Implements ports defined inward.
4. **Presentation** — input handling, layout/render loop, CLI argument
   parsing.
5. **Bin** — the composition root: constructs concretes, injects them, runs.

Rules:

- Inner crates MUST NOT depend on outer crates. Adding such a dep in a
  `Cargo.toml` is a review-blocker.
- Outer crates depend on inner crates only through traits defined inward
  (DIP).
- `streeem-domain/Cargo.toml` MUST stay free of `tokio`, `crossterm`,
  `std::process`-using crates, and any I/O dependency.

### Module Discipline ("a mod for everything")

Every conceptual unit lives in its own `mod`. Examples:

- `mod terminal_host` — lifecycle of one hosted terminal.
- `mod grid_layout` — staggered grid placement and resizing.
- `mod color_palette` — assigning and tracking colors per terminal.
- `mod pty_server` — spawning and pumping PTYs.
- `mod exp_client` — domain abstraction for the external (exp) server; infra
  impl behind a port trait.

Inside each `mod`, repeat the layered structure as sub-`mod`s
(`domain`, `application`, `infrastructure`, `presentation` — only the layers
that module actually needs). A module is a small clean-architecture cell.

## SOLID — non-negotiable

- **SRP** — one reason to change per type / function / module. Light files are
  the indicator: when a file grows, split first, ask second.
- **OCP** — extend by adding a new impl of a trait, never by editing existing
  types to take a new branch.
- **LSP** — trait impls honour the trait's documented contract. Don't panic
  where the trait promises a `Result::Err`.
- **ISP** — keep traits narrow. Prefer `trait PtySpawner` + `trait PtyReader`
  over one fat `trait Pty`.
- **DIP** — depend on traits defined inward; concrete types live outward and
  are wired at the composition root.

## Clean Code

- Code describes itself. Names carry intent (`spawn_hosted_terminal`, not
  `do_term`). No abbreviations except domain-standard (`pty`, `tty`).
- Comments only for non-obvious WHY (invariants, workarounds, surprises).
  Never for WHAT — the names already say that.
- Functions do one thing. If you need "and" to describe what a function does,
  split it.
- No `unwrap()` / `expect()` outside tests and the top-level `main` result
  handling.

## TDD — non-negotiable, red-first

For every behaviour change:

1. Write a failing test that names the behaviour.
2. Run it and confirm it fails for the intended reason (not a compile error,
   unless the missing API IS the behaviour).
3. Write the minimum code to make it green.
4. Refactor with the suite green.

- **Coverage discipline.** Goal: 100% line coverage. Current floor enforced by `scripts/coverage.sh`: 75%. Infrastructure adapters and the runtime loop are the dominant gap (PortablePtySpawner error paths, RatatuiRenderer draw paths, the long-running tokio select loop). Closing the gap is a v1.1 task; new domain/application/presentation code is expected to maintain or improve coverage.
- Unit tests live next to the code as `#[cfg(test)] mod tests` in the same
  file. Integration tests under `tests/`.
- Test names read as sentences:
  `assigns_next_unused_color_when_palette_has_capacity`.

## Manual DI + Hand-Written Fakes

- No DI frameworks. Wiring happens at the composition root (`main.rs` or a
  dedicated `mod composition`) by constructing concrete types and passing
  them in via constructors.
- Every external dependency is reached through a trait.
- For each trait, ship a hand-written **fake** in a `mod fakes` next to the
  trait, gated by `#[cfg(test)]` (or behind a `test-support` feature when
  cross-module reuse is needed).
- Fakes are deterministic and observable (record calls, expose recorded
  state).
- **No mocking framework** (`mockall`, `mockito`, etc.). Hand-written fakes
  only.

## Forbidden

- `unwrap()` / `expect()` outside tests and `main`'s top-level handling.
- Business logic in `main.rs` or anywhere in a presentation module.
- Mocking frameworks (use hand-written fakes).
- Files over ~200 lines without a written reason — treat it as an SRP smell
  and split.
- Adding a dependency without a one-line justification in the commit.
- Claiming a task done before: tests went red → green, `cargo clippy
  -D warnings` is clean, and coverage stayed at 100%.

## Commands

```sh
cargo test                                  # all tests
cargo test <name>                           # single test
cargo clippy --all-targets -- -D warnings   # lints (fail on warning)
cargo fmt --all                             # format
cargo llvm-cov --fail-under-lines 100       # coverage gate (install once)
./scripts/coverage.sh                 # full pre-commit gate (fmt, clippy, test, coverage)
```

## Definition of Done

- Test added first (red), then made green.
- 100% line coverage maintained.
- `cargo clippy -D warnings` and `cargo fmt --check` clean.
- File sizes stay light; no module reached into another's responsibility.
- Public types and traits documented with intent, not mechanics.
