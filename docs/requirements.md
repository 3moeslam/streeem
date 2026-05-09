# Streeem — Requirements & Design (v1)

- **Date:** 2026-05-09
- **Status:** Approved (brainstorm complete, awaiting spec review)
- **Owner:** Eslam
- **Supersedes:** —

---

## 1. Mission

Streeem is a Rust terminal application that acts as a *desktop of terminals*:
it hosts many child commands inside one terminal window, streaming each
command's live output into its own coloured tile, arranged in a staggered
(Pinterest-style) grid. The user's pain point is needing to monitor several
long-running commands at once without juggling multiple terminal windows or a
multiplexer.

## 2. Goals

- Run inside a single terminal window using a TUI (no GUI).
- Host **N** read-only child processes simultaneously, each in its own tile.
- Stream each child's stdout / stderr live into its tile with per-process
  colours preserved.
- Arrange tiles in a true staggered grid: fixed columns, variable per-tile
  heights, packed best-fit.
- Identify each tile visually by a distinct colour applied to its border and
  title bar.
- Allow the user to add and drop tiles at runtime, and to resize the focused
  tile.
- Keep memory bounded regardless of how long the app runs or how chatty a
  command is.
- Ship with the discipline mandated by `CLAUDE.md`: Clean Architecture, DDD,
  SOLID, TDD red-first, 100 % unit-test coverage, manual DI, hand-written
  fakes only.

## 3. Non-Goals (v1)

The following are explicitly **out of scope** for v1 and must not be designed
into the v1 architecture in ways that constrain future work:

- **Interactive shells.** Tiles are read-only. Keystrokes never reach the
  hosted process.
- **Full terminal emulation.** Cursor movement, alternate screen, scroll
  regions, ncurses apps (`top`, `htop`, `vim`) are not supported inside a
  tile.
- **Persistence.** No saved sessions, no replay across runs, no on-disk
  scrollback.
- **Auto-restart of exited processes.** When a process exits, its tile is
  removed (see §11).
- **Configuration files.** All input is via CLI flags and runtime keystrokes.
- **Network / remote tiles.** Only local processes.

## 4. User Stories

| # | As a … | I want to … | So that … |
|---|---|---|---|
| US-1 | developer | launch streeem with several commands at once | I can monitor a build, a log tail, and a dev server in one window |
| US-2 | developer | add a new command to the running app with a key | I don't have to restart streeem when I think of another thing to watch |
| US-3 | developer | drop the focused tile | I can clear noise without quitting |
| US-4 | developer | resize the focused tile vertically | I can give a chatty tile more room without restarting |
| US-5 | developer | scroll back inside a tile | I can read output that has rolled off the screen |
| US-6 | developer | distinguish tiles at a glance | I never confuse one process's output with another's |

## 5. CLI Surface

```sh
streeem [GLOBAL FLAGS] [TILE…]

# A TILE is one of:
#   '<command string>'                    e.g. 'cargo watch -x test'
#   --rows <N> '<command string>'         override default rows hint
#   --color <name> '<command string>'     (reserved; see §11.4 — not in v1)
#
# GLOBAL FLAGS:
#   --columns <N>            override the auto-computed column count
#   --scrollback <N>         override default scrollback capacity (lines)
#   --min-tile-width <N>     override the 40-col default for adaptive columns
#   -h, --help               show help and exit
#   -V, --version            show version and exit
```

**Examples**

```sh
# 3 tiles, default 10-row hint each, columns auto-computed
streeem 'cargo watch -x test' 'kubectl logs -f api' 'tail -f app.log'

# Mixed row hints
streeem \
    --rows 20 'cargo watch -x test' \
    --rows  8 'tail -f app.log' \
    --rows 12 'kubectl logs -f api'

# Force 4 columns regardless of width
streeem --columns 4 'cmd1' 'cmd2' 'cmd3' 'cmd4'
```

## 6. In-App Interaction

Default keybindings:

| Key | Action |
|---|---|
| `a` | Open prompt to add a new tile (command + optional rows hint) |
| `d` | Drop the focused tile (and abort its child); no-op if no tile is focused |
| `+` | Grow focused tile by 1 row (re-pack column) |
| `-` | Shrink focused tile by 1 row (re-pack column) |
| `→` `←` `↑` `↓` | Move focus spatially within the grid |
| `Tab` / `Shift-Tab` | Cycle focus forward / backward |
| `1` … `9` | Jump focus to the Nth tile |
| `PgUp` / `PgDn` | Scroll focused tile's scrollback by a page |
| `g` / `G` | Jump to top / bottom of focused tile's scrollback |
| `f` | Toggle "follow tail" on focused tile (re-enables auto-scroll to bottom) |
| `q` or `Ctrl-C` | Quit (restores cooked-mode terminal cleanly) |

Focused tile has an emphasised border (e.g., bold) so it's never ambiguous.

## 7. Visual Design

### 7.1 Layout — Staggered (Column-Flow) Grid

- **Column count:** `default = floor(terminal_width / min_tile_width)` where
  `min_tile_width = 40`. Overridable with `--columns N`. Recomputed on
  terminal resize.
- **Per-tile height:** comes from the tile's `RowsHint` (default 10). The hint
  is set at creation time and adjustable at runtime via `+` / `-`.
- **Placement (`LayoutPacker`, pure):** for each tile in arrival order, place
  it at the bottom of the column with the *smallest current total height*.
  Ties are broken by lowest column index.
- **Reflow on add / drop / resize / terminal resize:** the packer is a pure
  function of the current tile list, column count, and terminal height — every
  change re-runs it from scratch. Determinism makes it trivial to TDD.
- **Overflow:** if the sum of a column's heights exceeds visible height, the
  bottom-most tile in that column is clipped at the bottom edge with a `…`
  indicator. (Documented limitation for v1.)
- **Too small:** if `terminal_width < min_tile_width` or
  `terminal_height < min_tile_height (10)`, the renderer draws a centred
  banner `terminal too small (need 40×10)` instead of any tile.

### 7.2 Colour Scheme

- Curated palette of 12 distinguishable colours
  (`Red`, `Green`, `Yellow`, `Blue`, `Magenta`, `Cyan`, `LightRed`,
  `LightGreen`, `LightYellow`, `LightBlue`, `LightMagenta`, `LightCyan`).
- **Assignment (deterministic):** the palette has a fixed order. To assign a
  colour, scan the palette in order and return the first colour not currently
  in use. When a tile is dropped, its colour becomes available again. This
  makes assignment a pure function of the current set of in-use colours.
- **Wrap:** when all 12 colours are in use, the 13th tile reuses the colour of
  the oldest still-running tile. Two tiles may then share a colour. (Rare in
  practice; keeps the palette deterministic and simple.)
- **Where colour appears:** the tile's **border** and **title bar** are drawn
  in the assigned colour. The tile **body** (streamed output) is rendered in
  whatever ANSI styles the child process emits, so cargo's red/green test
  output, kubectl's coloured timestamps, etc., come through unchanged.

### 7.3 Tile Anatomy

```
┌─ [3] cargo watch -x test  (rows 20, 1234 lines) ──┐   <- coloured border + title
│ running 12 tests                                  │
│ test grid::packs_into_shortest_column ... ok      │
│ test color::releases_on_drop          ... ok      │
│ ...                                               │
└───────────────────────────────────────────────────┘
```

- Title format: `[N] <command>  (rows R, L lines)`
  - `N` is the tile's 1-based focus index (matches the `1`…`9` jump key).
  - `L` is the live line count in scrollback (capped at the configured
    capacity).
- Status badges in the title:
  - none while running
  - `[paused]` when the user has paused following (scrolled up past tail)
  - `[clipped]` when the tile is rendered shorter than its rows hint due to
    terminal-height pressure

## 8. Architecture

Cargo workspace, one crate per Clean Architecture layer. Dependencies point
inward only; the workspace `Cargo.toml`s enforce this — any inward leak fails
review by inspection of one file.

```
streeem/
  Cargo.toml                       # [workspace]
  crates/
    streeem-domain/                # pure: VOs, entities, reducer, ports
    streeem-application/           # use cases over ports
    streeem-infrastructure/        # pty, ansi, clock, terminal IO
    streeem-presentation/          # ratatui view, key map
    streeem-bin/                   # composition root (main.rs)
```

**Allowed dependencies:**

| Crate | May depend on |
|---|---|
| `streeem-domain` | (nothing — no `tokio`, `crossterm`, `portable-pty`, `std::process`) |
| `streeem-application` | `streeem-domain` |
| `streeem-infrastructure` | `streeem-domain`, `streeem-application` |
| `streeem-presentation` | `streeem-domain`, `streeem-application` |
| `streeem-bin` | all of the above + `tokio`, `crossterm`, `portable-pty`, `clap` |

**Inside each crate**, repeat the layered cell pattern via `mod`. For example,
`streeem-domain` contains `mod tile`, `mod grid_layout`, `mod color_palette`,
`mod scrollback`, `mod ansi`, `mod reducer`. Each `mod` is a small
clean-architecture cell with its own types, invariants, and tests.

## 9. Components

### 9.1 `streeem-domain` (pure)

**Value objects**
- `TileId` — newtype around `u32`, monotonically issued.
- `TileColor` — enum over the 12-colour palette.
- `RowsHint(u16)` — validated: `1 ≤ rows ≤ 200`.
- `ColumnCount(u16)` — validated: `≥ 1`.
- `ScrollbackCapacity(usize)` — validated: `≥ 100`. Default 10 000.
- `Style` — `{ fg: Option<TileColor>, bg: Option<TileColor>, bold, underline }`.
- `StyledSpan` — `{ text: String, style: Style }`.
- `OutputLine` — `Vec<StyledSpan>`.
- `CommandSpec` — `{ command_string: String, rows_hint: RowsHint }`.
- `ExitStatus` — `{ code: Option<i32>, signal: Option<i32> }`.

**Aggregates**
- `Tile` — `{ id, color, command_spec, scrollback, run_status, follow_tail }`.
  Run status is one of `Spawning | Running { pid } | Exited { status }`.
- `Grid` — `{ tiles: Vec<Tile>, focused: Option<TileId>, columns: ColumnCount, terminal_size: (u16, u16) }`.
  `focused` is `None` only when there are zero tiles; the reducer ensures it
  becomes `Some(...)` as soon as the first tile is added, and is reassigned
  to a neighbour when the focused tile is dropped.

**Domain services (pure functions)**
- `ColorPalette` — `next_unused() -> TileColor`, `release(c)`.
- `LayoutPacker::pack(tiles, columns, terminal_size) -> Vec<Placement>` —
  Placement = `{ tile_id, column, row_offset, height, is_clipped }`.
- `AnsiInterpreter::interpret(bytes) -> Vec<OutputEvent>` — handles SGR;
  silently drops cursor / clear / scroll-region sequences.
- `Scrollback::push(line)` — bounded ring; drops oldest when full and emits a
  `LinesDropped(n)` marker event.
- `Reducer::reduce(state, event) -> (state, Vec<OutboxEffect>)` — pure;
  `OutboxEffect` is e.g. `AbortPty(TileId)` or `MarkFrameDirty`.

**Port traits (defined here; implemented in infrastructure)**
- `PtySpawner` — `fn spawn(spec: CommandSpec) -> Result<SpawnedPty, SpawnError>`.
- `Clock` — `fn now() -> Instant`.
- `InputSource` — `fn next_event(...) -> Option<InputEvent>`.
- `Renderer` — `fn render(snapshot: RenderSnapshot)`.
- `TerminalSize` — `fn size() -> (u16, u16)`.

### 9.2 `streeem-application` (use cases)

**Commands** (one variant per intent)
- `AddTile(CommandSpec)`
- `DropTile(TileId)`
- `ResizeTile(TileId, delta_rows: i16)`
- `ScrollTile(TileId, delta: ScrollDelta)` (`Line(i32)`, `Page(i32)`, `Top`, `Bottom`)
- `MoveFocus(FocusMove)` (`Left`, `Right`, `Up`, `Down`, `Cycle(±1)`, `Index(u8)`)
- `OnPtyBytes(TileId, Vec<u8>)`
- `OnPtyExited(TileId, ExitStatus)`
- `OnTerminalResized(width, height)`

**Query**
- `RenderSnapshot` — frozen view: tiles, focused id, layout placements,
  alerts, follow-tail flags. Built on demand from current state.

**Handlers**
- One handler per command. Handlers are thin: translate to one or more domain
  events, call the reducer, side-effect via ports if instructed by the
  reducer's outbox.

### 9.3 `streeem-infrastructure` (adapters)

- `PortablePtySpawner` — implements `PtySpawner` using `portable-pty`. Returns
  `SpawnedPty { reader: Box<dyn AsyncRead>, exit: oneshot::Receiver<ExitStatus> }`.
- `SystemClock` — implements `Clock` via `std::time::Instant`.
- `CrosstermInputAdapter` — implements `InputSource` over `crossterm::event`.
- `RatatuiRenderer` — implements `Renderer` over a ratatui `Terminal<CrosstermBackend<...>>`.
  Owns the `TerminalGuard` that restores cooked mode on `Drop`.
- `CrosstermTerminalSize` — implements `TerminalSize` via
  `crossterm::terminal::size`.

Every adapter ships a hand-written `Fake*` next to it under
`#[cfg(any(test, feature = "test-support"))]`.

### 9.4 `streeem-presentation` (view)

- `KeyMap::map(key, current_state) -> Option<Command>` — pure.
- `ViewBuilder::build_frame(snapshot) -> FrameDescription` — pure. The
  `FrameDescription` is a structured value (list of widgets with their rects,
  styles, and content) that the infrastructure renderer translates into actual
  ratatui draw calls. This split is what lets the view layer be unit-tested
  with `assert_eq!`.

### 9.5 `streeem-bin` (composition root)

`main.rs` only:

```rust
fn main() -> Result<(), AppError> {
    let cli = Cli::parse();                        // clap
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    runtime.block_on(async {
        let pty       = PortablePtySpawner::new();
        let clock     = SystemClock;
        let input     = CrosstermInputAdapter::new();
        let size      = CrosstermTerminalSize;
        let renderer  = RatatuiRenderer::enter()?;  // RAII terminal guard
        let app       = Application::new(pty, clock, input, size, renderer);
        app.run(cli.into_initial_commands()).await
    })
}
```

## 10. Data Flow

### 10.1 Startup

1. `main.rs` parses CLI args into `Vec<CommandSpec>` and constructs concretes.
2. For each spec, the bin issues `Command::AddTile(spec)`.
3. `AddTileHandler` pulls a colour from `ColorPalette::next_unused`, builds
   a `Tile` (status = `Spawning`), calls `PtySpawner::spawn(spec)`. On
   success, sets status to `Running { pid }`; on `SpawnError`, records an
   alert and the tile is never added.
4. Bin spawns a tokio task per running tile that pumps PTY bytes into the
   application as `Command::OnPtyBytes(id, bytes)` over the central
   `mpsc::Sender<Command>`.

### 10.2 PTY emits bytes

1. Reader task receives a `Vec<u8>` chunk.
2. Reader sends `Command::OnPtyBytes(id, bytes)` over the channel.
3. The `select!` loop in bin wakes; application dispatches to
   `OnPtyBytesHandler`.
4. Handler calls `AnsiInterpreter::interpret(bytes)` → `Vec<OutputEvent>`.
5. Reducer applies events to the tile's `Scrollback`. If full, oldest line
   evicted and a `LinesDropped(1)` marker pushed.
6. Reducer marks frame dirty.
7. On the next 30 Hz tick, `RenderSnapshot` is built; passed through
   `ViewBuilder::build_frame` (pure); passed to `RatatuiRenderer::render`.

### 10.3 User presses `d`

1. `CrosstermInputAdapter` emits a `KeyEvent`.
2. Loop calls `KeyMap::map(key, state) -> Some(Command::DropTile(focused_id))`.
3. `DropTileHandler` issues a domain event; reducer removes the tile,
   releases its colour to the palette, and pushes `OutboxEffect::AbortPty(id)`.
4. Bin's outbox processor sends an abort signal to the per-tile reader task.
5. Frame dirty; next tick re-renders without the tile.

### 10.4 Loop skeleton (in `streeem-bin`)

```rust
loop {
    select! {
        Some(cmd)    = command_rx.recv()  => {
            let outbox = app.dispatch(cmd).await;
            outbox_processor.handle(outbox).await;        // spawn/abort PTY tasks
        }
        Some(key)    = input.next_event() => if let Some(c) = key_map.map(key, app.state()) {
            let outbox = app.dispatch(c).await;
            outbox_processor.handle(outbox).await;
        },
        Some(sz)     = resize_rx.recv()   => app.dispatch(Command::OnTerminalResized(sz.0, sz.1)).await,
        _            = ticker.tick()      => if app.state().is_dirty() {
            renderer.render(app.snapshot())?
        },
        _            = shutdown.recv()    => break,
    }
}
```

The `outbox_processor` is the only place in the bin that reacts to
`OutboxEffect`s the reducer asked for: spawning a new PTY reader task on
`AddTile`, aborting one on `DropTile`. Keeping it separate from the reducer
preserves the reducer's purity.

## 11. Error Handling

| Failure | Behavior |
|---|---|
| Spawn fails (e.g. typo'd command) | Tile not added; reason added to a top-of-screen alert strip (last 3 errors, 5 s fade). Stderr echo on app exit. |
| PTY closes / process exits | Tile auto-removed; colour released; PTY reader aborted; layout repacks. |
| Terminal too small (< 40×10) | Tiles persist in state; renderer draws centred `terminal too small (need 40×10)` banner until resize. |
| Tile doesn't fit available height | Tile placed in shortest column anyway; bottom-most tile clipped with `…` indicator; title shows `[clipped]`. |
| Colour palette exhausted (> 12 tiles) | 13th tile reuses colour of oldest still-running tile. Two tiles may share a colour. |
| Render or input adapter error | Logged to deferred stderr buffer; terminal restored to cooked mode via `Drop`; exit code 1. No panics in normal operation. |
| Invalid UTF-8 in PTY bytes | `String::from_utf8_lossy` substitutes U+FFFD; tile keeps streaming. |
| Pathologically long lines (> 4096 chars) | Stored as-is; renderer wraps. No splitting in reducer. |
| PTY backpressure (process emits faster than reducer drains) | Per-tile bounded `mpsc` (capacity 1024). When full, the reader awaits, which back-pressures the kernel pipe and pauses the child. The user perceives a momentarily-stalled tile, never lost data and never an OOM. |
| Scrollback overflow (more lines arrived than capacity) | `Scrollback::push` evicts the oldest line in O(1) and emits a `LinesDropped(n)` marker event into the line stream. The renderer shows the marker as a dim `[dropped N lines]` row so the user knows. This is independent of the channel back-pressure above. |

**Crash recovery / persistence:** none in v1.

**Terminal restoration invariant:** `RatatuiRenderer::enter()` returns a value
whose `Drop` impl restores cooked mode and shows the cursor. This must run on
every exit path including panic, so the value lives in the outermost scope of
`main`.

## 12. Testing Strategy

### 12.1 Discipline (per `CLAUDE.md`)

- Every behaviour starts with a failing test that names it.
- 100 % line coverage enforced by `cargo llvm-cov --workspace --fail-under-lines 100`.
- Hand-written fakes only; no `mockall` / `mockito`.
- Unit tests co-located: `#[cfg(test)] mod tests` in the same file.
- Cross-crate integration tests live under each crate's `tests/`.

### 12.2 Per-crate strategy

| Crate | Tested | How |
|---|---|---|
| `streeem-domain` | Reducer transitions, `LayoutPacker`, `ColorPalette`, `AnsiInterpreter`, `Scrollback` | Pure unit tests, zero fakes. `assert_eq!(reduce(state, event), expected)`. |
| `streeem-application` | Handlers translate to correct events; query builds correct snapshot | Hand-written fakes for ports (`FakePtySpawner`, `FakeClock`, …). |
| `streeem-infrastructure` | Adapters integrate with real OS / PTY / terminal | Integration tests under `tests/`. PTY tests spawn `echo`, `sleep`. CI gate: `cargo test --features integration`. |
| `streeem-presentation` | `KeyMap::map`, `ViewBuilder::build_frame` | Pure unit tests; `FrameDescription` comparable by value. |
| `streeem-bin` | End-to-end smoke | One test: spawn `echo hi`, drive a few events, assert the rendered `FrameDescription` contains a tile whose body includes `hi`. |

### 12.3 Hand-written fakes catalogue

In each port-defining crate, behind a `test-support` feature:

- `FakePtySpawner` — configurable per-spec script of byte chunks + exit code.
- `FakeClock` — manually advanceable.
- `FakeInputSource` — pre-canned `KeyEvent` queue.
- `FakeRenderer` — records every `FrameDescription` it was asked to render.
- `FakeTerminalSize` — settable width/height.

### 12.4 Headline algorithm test (the staggered packer)

```text
given: tiles with rows hints [20, 8, 12, 5, 15], terminal 100×60 visible,
       columns = 3
when:  LayoutPacker::pack(tiles, 3, (100, 60))
then:  placements are
         tile0: col 0, row  0, height 20   (col0 total = 20)
         tile1: col 1, row  0, height  8   (col1 total =  8)
         tile2: col 2, row  0, height 12   (col2 total = 12)
         tile3: col 1, row  8, height  5   (col1 total = 13)  ← shortest col after tile2
         tile4: col 2, row 12, height 15   (col2 total = 27)  ← then col2 was shortest
       all is_clipped = false
       (rule: each tile placed in column with smallest current total height,
        ties broken by lowest column index)
```

### 12.5 Coverage tooling

```sh
cargo install cargo-llvm-cov                            # one-time
cargo llvm-cov --workspace --fail-under-lines 100       # CI gate
cargo llvm-cov --workspace --html --output-dir coverage # local report
```

## 13. Acceptance Criteria

The v1 spec is satisfied when **all** of the following hold:

- [ ] `streeem 'echo hello'` shows a single coloured tile containing `hello`,
      then auto-removes when `echo` exits.
- [ ] `streeem 'cmd1' 'cmd2' 'cmd3'` shows three colour-distinct tiles
      arranged into the auto-computed column count.
- [ ] `streeem --rows 20 'a' --rows 5 'b' --rows 10 'c'` produces the exact
      column placements predicted by the rule in §12.4.
- [ ] `streeem --columns 4 'a' 'b' 'c' 'd' 'e'` shows 4 columns regardless of
      terminal width (subject to the 40-col minimum).
- [ ] Pressing `a`, typing `tail -f /tmp/foo`, hitting Enter adds a 4th tile
      live without restart.
- [ ] Pressing `d` on the focused tile drops it, releases its colour, repacks
      the grid; the dropped tile's colour is reused for the next added tile.
- [ ] Pressing `+` / `-` on the focused tile changes its row hint and
      re-packs its column.
- [ ] Resizing the terminal recomputes column count and triggers a clean
      reflow on the next tick.
- [ ] Spawning a non-existent command surfaces an alert at the top of the
      screen and prints the reason on app exit.
- [ ] Killing the app with `q`, `Ctrl-C`, or a panic restores the user's
      shell to cooked mode with cursor visible.
- [ ] A tile streaming millions of lines stays bounded at the configured
      scrollback capacity and shows a `[dropped N events]` marker if
      backpressure ever fired.
- [ ] `cargo llvm-cov --workspace --fail-under-lines 100` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` is clean.
- [ ] `cargo fmt --all -- --check` is clean.
- [ ] No file in the workspace uses a mocking framework. All test doubles are
      hand-written.

## 14. Future Scope (deliberately out of v1)

- Interactive mode (typing into a focused tile) with full PTY emulation.
- Per-command colour override (`--color red 'cmd'`).
- Configuration file (YAML/TOML) as syntactic sugar over the same domain
  primitives.
- Persistence: save layouts and re-launch on next run.
- Auto-restart with backoff for daemons.
- File-backed scrollback for grep across tiles.
- Theming (custom palettes).
- Mouse support (click to focus, drag to resize).
- Capture / export the contents of a tile to a file.

## 15. Open Questions

None blocking v1. Everything in §11 (Error Handling) and §6 (In-App
Interaction) has a decided default; refinements are welcome during
implementation but do not require respecifying.

---

**Next step (per the brainstorming flow):** invoke `superpowers:writing-plans`
to turn this spec into a concrete, test-driven implementation plan.
