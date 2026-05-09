#![cfg_attr(
    any(test, feature = "test-support"),
    allow(clippy::expect_used, clippy::unwrap_used)
)]
//! Spawns a child process attached to a PTY and returns its byte stream + exit handle.

use std::io::Write;

use crate::command_spec::CommandSpec;
use crate::exit_status::ExitStatus;
use crate::tile_id::TileId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnError {
    pub reason: String,
}

pub struct SpawnedPty {
    pub id: TileId,
    pub byte_chunks: Box<dyn Iterator<Item = Vec<u8>> + Send>,
    pub writer: Box<dyn Write + Send>,
    pub exit: Box<dyn FnOnce() -> ExitStatus + Send>,
}

pub trait PtySpawner: Send + Sync {
    fn spawn(&self, id: TileId, spec: &CommandSpec) -> Result<SpawnedPty, SpawnError>;
}

#[cfg(any(test, feature = "test-support"))]
pub mod fakes {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    /// A `Write` implementation that records all bytes written for test assertions.
    pub struct RecordingWriter {
        pub log: Arc<Mutex<Vec<u8>>>,
    }

    impl Write for RecordingWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.log
                .lock()
                .expect("RecordingWriter mutex")
                .extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    pub struct FakePtySpawner {
        scripts: Mutex<Vec<FakeScript>>,
        recorded: Mutex<Vec<(TileId, CommandSpec)>>,
        writer_logs: Mutex<HashMap<TileId, Arc<Mutex<Vec<u8>>>>>,
    }

    impl Default for FakePtySpawner {
        fn default() -> Self {
            Self::new()
        }
    }

    pub struct FakeScript {
        pub command_substring: String,
        pub bytes: Vec<Vec<u8>>,
        pub exit: ExitStatus,
        pub spawn_error: Option<String>,
    }

    impl FakePtySpawner {
        pub fn new() -> Self {
            Self {
                scripts: Mutex::new(Vec::new()),
                recorded: Mutex::new(Vec::new()),
                writer_logs: Mutex::new(HashMap::new()),
            }
        }

        pub fn add_script(&self, script: FakeScript) {
            self.scripts.lock().expect("scripts mutex").push(script);
        }

        pub fn recorded_spawns(&self) -> Vec<(TileId, CommandSpec)> {
            self.recorded.lock().expect("recorded mutex").clone()
        }

        /// Returns all bytes written to the writer for the given tile id.
        pub fn writer_log(&self, id: TileId) -> Vec<u8> {
            self.writer_logs
                .lock()
                .expect("writer_logs mutex")
                .get(&id)
                .map(|arc| arc.lock().expect("writer log mutex").clone())
                .unwrap_or_default()
        }
    }

    impl PtySpawner for FakePtySpawner {
        fn spawn(&self, id: TileId, spec: &CommandSpec) -> Result<SpawnedPty, SpawnError> {
            self.recorded
                .lock()
                .expect("recorded mutex")
                .push((id, spec.clone()));
            let mut scripts = self.scripts.lock().expect("scripts mutex");
            let pos = scripts
                .iter()
                .position(|s| spec.command.contains(&s.command_substring))
                .ok_or_else(|| SpawnError {
                    reason: format!("no FakeScript matches command {:?}", spec.command),
                })?;
            let script = scripts.remove(pos);
            if let Some(reason) = script.spawn_error {
                return Err(SpawnError { reason });
            }
            let log: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
            self.writer_logs
                .lock()
                .expect("writer_logs mutex")
                .insert(id, Arc::clone(&log));
            let writer = RecordingWriter { log };
            let bytes = script.bytes.into_iter();
            let exit_status = script.exit;
            Ok(SpawnedPty {
                id,
                byte_chunks: Box::new(bytes),
                writer: Box::new(writer),
                exit: Box::new(move || exit_status),
            })
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn spawn_returns_scripted_bytes() {
            let s = FakePtySpawner::new();
            s.add_script(FakeScript {
                command_substring: "echo".to_string(),
                bytes: vec![b"hi\n".to_vec()],
                exit: ExitStatus::Code(0),
                spawn_error: None,
            });
            let spec = CommandSpec::with_default_rows("echo hi").unwrap();
            let mut spawned = s.spawn(TileId::default_from(0), &spec).unwrap();
            assert_eq!(spawned.byte_chunks.next(), Some(b"hi\n".to_vec()));
            assert_eq!((spawned.exit)(), ExitStatus::Code(0));
        }

        #[test]
        fn spawn_returns_error_when_script_says_so() {
            let s = FakePtySpawner::new();
            s.add_script(FakeScript {
                command_substring: "fail".to_string(),
                bytes: vec![],
                exit: ExitStatus::Code(0),
                spawn_error: Some("not found".to_string()),
            });
            let spec = CommandSpec::with_default_rows("fail-me").unwrap();
            assert!(s.spawn(TileId::default_from(0), &spec).is_err());
        }

        #[test]
        fn spawn_records_each_call() {
            let s = FakePtySpawner::new();
            s.add_script(FakeScript {
                command_substring: "echo".to_string(),
                bytes: vec![],
                exit: ExitStatus::Code(0),
                spawn_error: None,
            });
            let spec = CommandSpec::with_default_rows("echo a").unwrap();
            let _ = s.spawn(TileId::default_from(3), &spec);
            assert_eq!(s.recorded_spawns(), vec![(TileId::default_from(3), spec)]);
        }

        #[test]
        fn writer_log_records_bytes_written() {
            let s = FakePtySpawner::new();
            s.add_script(FakeScript {
                command_substring: "echo".to_string(),
                bytes: vec![],
                exit: ExitStatus::Code(0),
                spawn_error: None,
            });
            let spec = CommandSpec::with_default_rows("echo hi").unwrap();
            let id = TileId::default_from(0);
            let mut spawned = s.spawn(id, &spec).unwrap();
            spawned.writer.write_all(b"hi").unwrap();
            assert_eq!(s.writer_log(id), b"hi");
        }
    }
}
