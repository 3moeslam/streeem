use std::collections::HashMap;
use std::time::Duration;

use anyhow::Result;
use streeem_application::application::Application;
use streeem_application::command::Command;
use streeem_domain::ansi::AnsiInterpreter;
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
use streeem_infrastructure::ratatui_renderer::RatatuiRenderer;
use streeem_presentation::key_map::{AppIntent, KeyOutcome, map as map_key};
use streeem_presentation::prompt::{PromptOutcome, PromptState};
use streeem_presentation::view::{FrameDescription, build_with_prompt};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::interval;

pub async fn run(initial_specs: Vec<CommandSpec>, columns_override: Option<u16>) -> Result<()> {
    let size_adapter = CrosstermTerminalSize;
    let (tw, th) = size_adapter.size();
    let columns = ColumnCount::new(columns_override.unwrap_or_else(|| (tw / 40).max(1)))
        .map_err(|e| anyhow::anyhow!("invalid columns value: {e:?}"))?;
    let state = State::new(columns, tw, th);
    let mut app = Application::new(state);
    let pty = PortablePtySpawner::new();
    let mut renderer =
        RatatuiRenderer::enter().map_err(|e| anyhow::anyhow!("renderer: {}", e.0))?;
    let mut input = CrosstermInputAdapter::new();

    let (cmd_tx, mut cmd_rx) = mpsc::channel::<Command>(1024);
    let mut readers: HashMap<TileId, JoinHandle<()>> = HashMap::new();

    for spec in initial_specs {
        cmd_tx.send(Command::AddTile(spec)).await.ok();
    }

    let mut tick = interval(Duration::from_millis(33));
    let mut prompt = PromptState::default();

    loop {
        tokio::select! {
            Some(command) = cmd_rx.recv() => {
                let outbox = app.dispatch(command);
                process_outbox(&pty, &cmd_tx, &mut readers, outbox).await;
            }
            _ = tick.tick() => {
                if let Some(key) = input.poll_event() {
                    if prompt.active {
                        match prompt.handle(key) {
                            PromptOutcome::Submitted(cmd) => {
                                let outbox = app.dispatch(cmd);
                                process_outbox(&pty, &cmd_tx, &mut readers, outbox).await;
                            }
                            PromptOutcome::InvalidSubmission(_)
                            | PromptOutcome::Cancelled
                            | PromptOutcome::Continue => {}
                        }
                    } else {
                        match map_key(key, &app.snapshot()) {
                            KeyOutcome::Intent(AppIntent::Quit) => break,
                            KeyOutcome::Intent(AppIntent::PromptAddTile) => prompt.open(),
                            KeyOutcome::Command(c) => {
                                let outbox = app.dispatch(c);
                                process_outbox(&pty, &cmd_tx, &mut readers, outbox).await;
                            }
                            KeyOutcome::Ignored => {}
                        }
                    }
                }
                let (w, h) = size_adapter.size();
                if (w, h) != app.snapshot().terminal_size {
                    let outbox = app.dispatch(Command::OnTerminalResized { width: w, height: h });
                    process_outbox(&pty, &cmd_tx, &mut readers, outbox).await;
                }
                if app.state().dirty {
                    let frame: FrameDescription = build_with_prompt(
                        &app.snapshot(),
                        if prompt.active {
                            Some(prompt.buffer.clone())
                        } else {
                            None
                        },
                    );
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
    effects: Vec<OutboxEffect>,
) {
    for effect in effects {
        match effect {
            OutboxEffect::SpawnPty { id, spec } => match pty.spawn(id, &spec) {
                Ok(spawned) => {
                    tx.send(Command::OnPtySpawned(id)).await.ok();
                    let tx_for_task = tx.clone();
                    let handle = tokio::task::spawn_blocking(move || {
                        let mut interpreter = AnsiInterpreter::new();
                        let mut chunks = spawned.byte_chunks;
                        for chunk in chunks.by_ref() {
                            let lines = interpreter.feed(&chunk);
                            if !lines.is_empty() {
                                let _ =
                                    tx_for_task.blocking_send(Command::OnPtyOutput { id, lines });
                            }
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
            }
            OutboxEffect::RecordAlert(_) => {}
            OutboxEffect::MarkFrameDirty => {}
        }
    }
}
