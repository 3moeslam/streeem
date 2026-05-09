use std::collections::HashMap;
use std::io::Write;
use std::time::Duration;

use crate::input_bytes::key_to_bytes;
use crate::input_mode::Mode;
use crate::ratatui_renderer::RatatuiRenderer;
use anyhow::Result;
use streeem_application::application::Application;
use streeem_application::command::Command;
use streeem_domain::column_count::ColumnCount;
use streeem_domain::command_spec::CommandSpec;
use streeem_domain::outbox::OutboxEffect;
use streeem_domain::ports::input_source::InputSource;
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
    let mut mode = Mode::Command;

    for spec in initial_specs {
        cmd_tx.send(Command::AddTile(spec)).await.ok();
    }

    let mut tick = interval(Duration::from_millis(33));
    let mut prompt = PromptState::default();

    'outer: loop {
        tokio::select! {
            Some(command) = cmd_rx.recv() => {
                let outbox = app.dispatch(command);
                process_outbox(&pty, &cmd_tx, &mut readers, &mut writers, outbox).await;
            }
            _ = tick.tick() => {
                while let Some(key) = input.poll_event() {
                    match mode {
                        Mode::Input => {
                            if key.code == streeem_domain::ports::input_source::KeyCode::Esc {
                                mode = Mode::Command;
                            } else if let Some(bytes) = key_to_bytes(key)
                                && let Some(focused_id) = app.snapshot().focused
                                && let Some(writer) = writers.get_mut(&focused_id)
                            {
                                let _ = writer.write_all(&bytes);
                                let _ = writer.flush();
                            }
                        }
                        Mode::Command => {
                            if prompt.active {
                                match prompt.handle(key) {
                                    PromptOutcome::Submitted(cmd) => {
                                        let outbox = app.dispatch(cmd);
                                        process_outbox(&pty, &cmd_tx, &mut readers, &mut writers, outbox).await;
                                    }
                                    PromptOutcome::InvalidSubmission(_)
                                    | PromptOutcome::Cancelled
                                    | PromptOutcome::Continue => {}
                                }
                            } else {
                                match map_key(key, &app.snapshot()) {
                                    KeyOutcome::Intent(AppIntent::Quit) => break 'outer,
                                    KeyOutcome::Intent(AppIntent::PromptAddTile) => prompt.open(),
                                    KeyOutcome::Intent(AppIntent::EnterInputMode) => {
                                        mode = Mode::Input;
                                    }
                                    KeyOutcome::Command(c) => {
                                        let outbox = app.dispatch(c);
                                        process_outbox(&pty, &cmd_tx, &mut readers, &mut writers, outbox).await;
                                    }
                                    KeyOutcome::Ignored => {}
                                }
                            }
                        }
                    }
                }
                let (w, h) = size_adapter.size();
                if (w, h) != app.snapshot().terminal_size {
                    let outbox = app.dispatch(Command::OnTerminalResized { width: w, height: h });
                    process_outbox(&pty, &cmd_tx, &mut readers, &mut writers, outbox).await;
                }
                if app.state().dirty {
                    let mut frame: FrameDescription = build_with_prompt(
                        &app.snapshot(),
                        if prompt.active {
                            Some(prompt.buffer.clone())
                        } else {
                            None
                        },
                    );
                    if mode == Mode::Input
                        && let FrameDescription::Tiles { ref mut status_bar, .. } = frame
                    {
                        let focused_label = app
                            .snapshot()
                            .focused
                            .and_then(|id| {
                                app.state()
                                    .grid
                                    .tiles
                                    .iter()
                                    .find(|t| t.id == id)
                                    .and_then(|t| t.name.clone())
                            })
                            .unwrap_or_else(|| "tile".to_string());
                        *status_bar = format!("[INPUT — {focused_label}]   Esc:exit input mode");
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
    effects: Vec<OutboxEffect>,
) {
    for effect in effects {
        match effect {
            OutboxEffect::SpawnPty { id, spec } => match pty.spawn(id, &spec) {
                Ok(spawned) => {
                    tx.send(Command::OnPtySpawned(id)).await.ok();
                    writers.insert(id, spawned.writer);
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
            }
            OutboxEffect::RecordAlert(_) => {}
            OutboxEffect::MarkFrameDirty => {}
        }
    }
}
