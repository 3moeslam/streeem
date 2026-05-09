//! Diagnostic logging to /tmp/streeem.log for debugging input/PTY flow.
//!
//! Writes are best-effort and silent on error. Each entry is timestamped.
//! The TUI takes over the terminal so println/eprintln can't help; this
//! file gives us a way to see what happened after the fact.

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;
use std::time::SystemTime;

static LOG: Mutex<Option<std::fs::File>> = Mutex::new(None);

pub fn init() {
    let f = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("/tmp/streeem.log")
        .ok();
    if let Ok(mut guard) = LOG.lock() {
        *guard = f;
    }
    log("=== streeem session started ===");
}

pub fn log(msg: &str) {
    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    if let Ok(mut guard) = LOG.lock()
        && let Some(f) = guard.as_mut()
    {
        let _ = writeln!(f, "[{ts}] {msg}");
        let _ = f.flush();
    }
}
