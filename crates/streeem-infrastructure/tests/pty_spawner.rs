#![allow(clippy::expect_used, clippy::unwrap_used)]
use streeem_domain::command_spec::CommandSpec;
use streeem_domain::ports::pty_spawner::PtySpawner;
use streeem_domain::tile_id::TileId;
use streeem_infrastructure::portable_pty_spawner::PortablePtySpawner;

#[test]
fn spawning_echo_yields_expected_output_and_zero_exit() {
    let spawner = PortablePtySpawner::new();
    let spec = CommandSpec::with_default_rows("printf hi").unwrap();
    let mut spawned = spawner
        .spawn(TileId::default_from(0), &spec)
        .expect("spawn should succeed for printf");
    let mut all = Vec::new();
    for chunk in spawned.byte_chunks.by_ref() {
        all.extend_from_slice(&chunk);
    }
    let text = String::from_utf8_lossy(&all);
    assert!(
        text.contains("hi"),
        "expected 'hi' in output, got: {text:?}"
    );
    let status = (spawned.exit)();
    assert!(status.is_success(), "expected success, got {status:?}");
}
