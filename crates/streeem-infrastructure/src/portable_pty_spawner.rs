#![allow(clippy::cast_possible_wrap)]
use std::io::Read;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread;

use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use streeem_domain::command_spec::CommandSpec;
use streeem_domain::exit_status::ExitStatus;
use streeem_domain::ports::pty_spawner::{PtySpawner, SpawnError, SpawnedPty};
use streeem_domain::tile_id::TileId;

#[derive(Debug, Default)]
pub struct PortablePtySpawner;

impl PortablePtySpawner {
    pub fn new() -> Self {
        Self
    }
}

impl PtySpawner for PortablePtySpawner {
    fn spawn(&self, id: TileId, spec: &CommandSpec) -> Result<SpawnedPty, SpawnError> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| SpawnError {
                reason: e.to_string(),
            })?;

        let mut cmd = CommandBuilder::new("sh");
        cmd.arg("-c");
        cmd.arg(&spec.command);
        cmd.env("TERM", "xterm-256color");

        let mut child = pair.slave.spawn_command(cmd).map_err(|e| SpawnError {
            reason: e.to_string(),
        })?;
        drop(pair.slave);

        // Wrap master in Arc<Mutex<>> so we can share it between the writer,
        // reader thread, and resize closure.
        let master_arc: Arc<Mutex<Box<dyn MasterPty + Send>>> = Arc::new(Mutex::new(pair.master));

        let writer = master_arc
            .lock()
            .map_err(|e| SpawnError {
                reason: format!("master mutex poisoned: {e}"),
            })?
            .take_writer()
            .map_err(|e| SpawnError {
                reason: e.to_string(),
            })?;

        let (tx, rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = channel();
        let mut reader = master_arc
            .lock()
            .map_err(|e| SpawnError {
                reason: format!("master mutex poisoned: {e}"),
            })?
            .try_clone_reader()
            .map_err(|e| SpawnError {
                reason: e.to_string(),
            })?;

        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let resize_master = Arc::clone(&master_arc);
        let resize: Box<dyn FnMut(u16, u16) + Send> = Box::new(move |cols: u16, rows: u16| {
            if let Ok(m) = resize_master.lock() {
                let _ = m.resize(PtySize {
                    rows: rows.max(1),
                    cols: cols.max(1),
                    pixel_width: 0,
                    pixel_height: 0,
                });
            }
        });

        let exit = Box::new(move || {
            let status = child.wait().map(|s| {
                if s.success() {
                    ExitStatus::Code(0)
                } else {
                    ExitStatus::Code(s.exit_code() as i32)
                }
            });
            status.unwrap_or(ExitStatus::Code(-1))
        });

        let chunks = std::iter::from_fn(move || rx.recv().ok());
        Ok(SpawnedPty {
            id,
            byte_chunks: Box::new(chunks),
            writer,
            resize,
            exit,
        })
    }
}
