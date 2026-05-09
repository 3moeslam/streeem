use std::collections::HashMap;
use std::io::Write;
use std::time::Duration;

use crate::debug_log;
use crate::input_bytes::key_to_bytes;
use crate::ratatui_renderer::RatatuiRenderer;
use anyhow::Result;
use streeem_application::application::Application;
use streeem_application::command::Command;
use streeem_domain::column_count::ColumnCount;
use streeem_domain::command_spec::CommandSpec;
use streeem_domain::grid::FocusMove;
use streeem_domain::outbox::OutboxEffect;
use streeem_domain::ports::input_source::{InputSource, KeyCode};
use streeem_domain::ports::pty_spawner::PtySpawner;
use streeem_domain::ports::renderer::Renderer;
use streeem_domain::ports::terminal_size::TerminalSize;
use streeem_domain::state::State;
use streeem_domain::tile_id::TileId;
use streeem_infrastructure::crossterm_input_adapter::CrosstermInputAdapter;
use streeem_infrastructure::crossterm_terminal_size::CrosstermTerminalSize;
use streeem_infrastructure::portable_pty_spawner::PortablePtySpawner;
use streeem_presentation::key_map::{AppIntent, KeyOutcome, map as map_key};
use streeem_presentation::prompt::{PromptOutcome, PromptState};
use streeem_presentation::view::{FrameDescription, build_with_prompt};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::interval;

type ResizeCallbacks = HashMap<TileId, Box<dyn FnMut(u16, u16) + Send>>;

const DOUBLE_ESC_WINDOW: Duration = Duration::from_millis(500);
const COMMAND_MODE_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn run(
    initial_specs: Vec<CommandSpec>,
    columns_override: Option<u16>,
    min_tile_width: Option<u16>,
) -> Result<()> {
    let size_adapter = CrosstermTerminalSize;
    let mtw = min_tile_width.unwrap_or(40);
    let (tw, th) = size_adapter.size();
    let auto_cols = (tw / mtw.max(1)).max(1);
    let cols_value = columns_override.unwrap_or(auto_cols);
    let columns = ColumnCount::new(cols_value)
        .map_err(|e| anyhow::anyhow!("invalid columns value: {e:?}"))?;
    let state = State::with_layout_config(columns, tw, th, columns_override, mtw);
    let mut app = Application::new(state);
    let pty = PortablePtySpawner::new();
    let mut renderer =
        RatatuiRenderer::enter().map_err(|e| anyhow::anyhow!("renderer: {}", e.0))?;
    let mut input = CrosstermInputAdapter::new();

    let (cmd_tx, mut cmd_rx) = mpsc::channel::<Command>(1024);
    let mut readers: HashMap<TileId, JoinHandle<()>> = HashMap::new();
    let mut writers: HashMap<TileId, Box<dyn Write + Send>> = HashMap::new();
    let mut resize_callbacks: ResizeCallbacks = HashMap::new();

    for spec in initial_specs {
        cmd_tx.send(Command::AddTile(spec)).await.ok();
    }

    let mut tick = interval(Duration::from_millis(33));
    let mut prompt = PromptState::default();

    // Double-Esc command mode state.
    let mut last_esc_at: Option<std::time::Instant> = None;
    let mut command_mode_until: Option<std::time::Instant> = None;

    'outer: loop {
        tokio::select! {
            Some(command) = cmd_rx.recv() => {
                let outbox = app.dispatch(command);
                process_outbox(&pty, &cmd_tx, &mut readers, &mut writers, &mut resize_callbacks, outbox).await;
            }
            _ = tick.tick() => {
                while let Some(key) = input.poll_event() {
                    if prompt.active {
                        match prompt.handle(key) {
                            PromptOutcome::Submitted(cmd) => {
                                let outbox = app.dispatch(cmd);
                                process_outbox(&pty, &cmd_tx, &mut readers, &mut writers, &mut resize_callbacks, outbox).await;
                            }
                            PromptOutcome::InvalidSubmission(_)
                            | PromptOutcome::Cancelled
                            | PromptOutcome::Continue => {}
                        }
                        continue;
                    }

                    let now = std::time::Instant::now();
                    let in_command_mode =
                        command_mode_until.is_some_and(|t| now < t);

                    if in_command_mode {
                        if key.code == KeyCode::Esc {
                            // Esc exits command mode.
                            debug_log::log("cmd-mode: exit (Esc)");
                            command_mode_until = None;
                            last_esc_at = None;
                            continue;
                        }

                        if let KeyCode::Char(c) = key.code {
                            match c.to_ascii_lowercase() {
                                'q' => {
                                    debug_log::log("cmd-mode: quit");
                                    break 'outer;
                                }
                                'a' => {
                                    debug_log::log("cmd-mode: add tile prompt");
                                    prompt.open();
                                    command_mode_until = None;
                                }
                                'x' => {
                                    debug_log::log("cmd-mode: drop tile");
                                    if let Some(id) = app.snapshot().focused {
                                        let outbox = app.dispatch(Command::DropTile(id));
                                        process_outbox(&pty, &cmd_tx, &mut readers, &mut writers, &mut resize_callbacks, outbox).await;
                                    }
                                    command_mode_until = None;
                                }
                                'n' => {
                                    debug_log::log("cmd-mode: next tile");
                                    let outbox = app.dispatch(Command::MoveFocus(FocusMove::CycleForward));
                                    process_outbox(&pty, &cmd_tx, &mut readers, &mut writers, &mut resize_callbacks, outbox).await;
                                    command_mode_until = None;
                                }
                                'p' => {
                                    debug_log::log("cmd-mode: prev tile");
                                    let outbox = app.dispatch(Command::MoveFocus(FocusMove::CycleBackward));
                                    process_outbox(&pty, &cmd_tx, &mut readers, &mut writers, &mut resize_callbacks, outbox).await;
                                    command_mode_until = None;
                                }
                                'f' => {
                                    debug_log::log("cmd-mode: toggle follow-tail");
                                    if let Some(id) = app.snapshot().focused {
                                        let outbox =
                                            app.dispatch(Command::ToggleFollowTail(id));
                                        process_outbox(&pty, &cmd_tx, &mut readers, &mut writers, &mut resize_callbacks, outbox).await;
                                    }
                                    command_mode_until = None;
                                }
                                _ => {
                                    // Unknown command-mode key; exit command mode.
                                    debug_log::log(&format!(
                                        "cmd-mode: unknown key {:?}, exiting",
                                        c
                                    ));
                                    command_mode_until = None;
                                }
                            }
                        } else {
                            // Non-char key in command mode — exit command mode.
                            command_mode_until = None;
                        }
                        continue;
                    }

                    // Not in command mode. Handle Esc specially for double-Esc detection.
                    if key.code == KeyCode::Esc {
                        if let Some(prev) = last_esc_at
                            && now.duration_since(prev) < DOUBLE_ESC_WINDOW
                        {
                            // Second Esc within the window — enter command mode.
                            // The first Esc was already forwarded; that's fine.
                            debug_log::log("double-Esc: entering command mode");
                            command_mode_until = Some(now + COMMAND_MODE_TIMEOUT);
                            last_esc_at = None;
                            continue;
                        }
                        last_esc_at = Some(now);
                        // Fall through to forward this Esc to the tile.
                    } else {
                        // Non-Esc key resets the esc timer.
                        last_esc_at = None;
                    }

                    // Try the legacy Ctrl+Q / Ctrl+A keymap for backward compat.
                    match map_key(key, &app.snapshot()) {
                        KeyOutcome::Intent(AppIntent::Quit) => {
                            debug_log::log("command: ^Q");
                            break 'outer;
                        }
                        KeyOutcome::Intent(AppIntent::PromptAddTile) => prompt.open(),
                        KeyOutcome::Command(c) => {
                            let outbox = app.dispatch(c);
                            process_outbox(&pty, &cmd_tx, &mut readers, &mut writers, &mut resize_callbacks, outbox).await;
                        }
                        KeyOutcome::Forward => {
                            if let Some(bytes) = key_to_bytes(key) {
                                let focused = app.snapshot().focused;
                                let has_writer = focused.is_some_and(|id| writers.contains_key(&id));
                                debug_log::log(&format!(
                                    "forward: key={:?} bytes={:?} focused={:?} has_writer={}",
                                    key.code, bytes, focused, has_writer
                                ));
                                if let Some(focused_id) = focused
                                    && let Some(writer) = writers.get_mut(&focused_id)
                                {
                                    if let Err(e) = writer.write_all(&bytes) {
                                        debug_log::log(&format!("forward: write FAILED: {e}"));
                                    }
                                    if let Err(e) = writer.flush() {
                                        debug_log::log(&format!("forward: flush FAILED: {e}"));
                                    }
                                }
                            }
                        }
                    }
                }

                let (w, h) = size_adapter.size();
                if (w, h) != app.snapshot().terminal_size {
                    let outbox = app.dispatch(Command::OnTerminalResized { width: w, height: h });
                    process_outbox(&pty, &cmd_tx, &mut readers, &mut writers, &mut resize_callbacks, outbox).await;
                }

                // Converge each tile's buffer size to the rendered inner area.
                let snap = app.snapshot();
                let mut resize_cmds: Vec<Command> = Vec::new();
                for placement in &snap.placements {
                    let inner_w = placement.width.saturating_sub(2);
                    let inner_h = placement.height.saturating_sub(2);
                    if inner_w == 0 || inner_h == 0 {
                        continue;
                    }
                    if let Some(tile_snap) = snap.tiles.iter().find(|t| t.id == placement.tile_id) {
                        let cur_h = tile_snap.cells.len() as u16;
                        let cur_w = tile_snap
                            .cells
                            .first()
                            .map(|r| r.len() as u16)
                            .unwrap_or(0);
                        if (cur_w, cur_h) != (inner_w, inner_h) {
                            debug_log::log(&format!(
                                "resize buffer: tile={:?} from ({cur_w}x{cur_h}) to ({inner_w}x{inner_h})",
                                placement.tile_id
                            ));
                            resize_cmds.push(Command::ResizeTileBuffer {
                                id: placement.tile_id,
                                width: inner_w,
                                height: inner_h,
                            });
                        }
                    }
                }
                for cmd in resize_cmds {
                    let outbox = app.dispatch(cmd);
                    process_outbox(&pty, &cmd_tx, &mut readers, &mut writers, &mut resize_callbacks, outbox).await;
                }

                if app.state().dirty {
                    let now = std::time::Instant::now();
                    let in_command_mode = command_mode_until.is_some_and(|t| now < t);
                    let bar = if in_command_mode {
                        streeem_presentation::key_map::STATUS_BAR_TEXT_COMMAND
                    } else {
                        streeem_presentation::key_map::STATUS_BAR_TEXT_FORWARDING
                    };
                    let mut frame: FrameDescription = build_with_prompt(
                        &app.snapshot(),
                        if prompt.active {
                            Some(prompt.buffer.clone())
                        } else {
                            None
                        },
                    );
                    if let FrameDescription::Tiles { ref mut status_bar, .. } = frame {
                        *status_bar = bar.to_string();
                    }
                    renderer.render(&frame).map_err(|e| anyhow::anyhow!("render: {}", e.0))?;
                }
            }
        }
    }
    Ok(())
}

async fn process_outbox(
    pty: &PortablePtySpawner,
    tx: &mpsc::Sender<Command>,
    readers: &mut HashMap<TileId, JoinHandle<()>>,
    writers: &mut HashMap<TileId, Box<dyn Write + Send>>,
    resize_callbacks: &mut ResizeCallbacks,
    effects: Vec<OutboxEffect>,
) {
    for effect in effects {
        match effect {
            OutboxEffect::SpawnPty { id, spec } => match pty.spawn(id, &spec) {
                Ok(spawned) => {
                    debug_log::log(&format!("spawn OK: tile={:?} cmd={:?}", id, spec.command));
                    tx.send(Command::OnPtySpawned(id)).await.ok();
                    writers.insert(id, spawned.writer);
                    resize_callbacks.insert(id, spawned.resize);
                    let tx_for_task = tx.clone();
                    let handle = tokio::task::spawn_blocking(move || {
                        let mut chunks = spawned.byte_chunks;
                        for chunk in chunks.by_ref() {
                            let _ =
                                tx_for_task.blocking_send(Command::OnPtyBytes { id, bytes: chunk });
                        }
                        let status = (spawned.exit)();
                        let _ = tx_for_task.blocking_send(Command::OnPtyExited { id, status });
                    });
                    readers.insert(id, handle);
                }
                Err(e) => {
                    debug_log::log(&format!(
                        "spawn FAILED: cmd={:?} reason={}",
                        spec.command, e.reason
                    ));
                    tx.send(Command::OnPtySpawnFailed {
                        spec,
                        reason: e.reason,
                    })
                    .await
                    .ok();
                }
            },
            OutboxEffect::AbortPty(id) => {
                if let Some(handle) = readers.remove(&id) {
                    handle.abort();
                }
                writers.remove(&id);
                resize_callbacks.remove(&id);
            }
            OutboxEffect::ResizePty { id, cols, rows } => {
                if let Some(cb) = resize_callbacks.get_mut(&id) {
                    cb(cols, rows);
                }
            }
            OutboxEffect::RecordAlert(_) => {}
            OutboxEffect::MarkFrameDirty => {}
        }
    }
}
