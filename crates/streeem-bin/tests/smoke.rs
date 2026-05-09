#![allow(clippy::unwrap_used, clippy::panic, clippy::while_let_on_iterator)]

use streeem_application::application::Application;
use streeem_application::command::Command;
use streeem_application::query::RenderSnapshot;
use streeem_domain::column_count::ColumnCount;
use streeem_domain::command_spec::CommandSpec;
use streeem_domain::exit_status::ExitStatus;
use streeem_domain::ports::pty_spawner::PtySpawner;
use streeem_domain::ports::pty_spawner::fakes::{FakePtySpawner, FakeScript};
use streeem_domain::state::State;
use streeem_presentation::view::{FrameDescription, build as build_view};

#[test]
fn add_tile_and_pty_output_results_in_visible_text() {
    let mut app = Application::new(State::new(ColumnCount::new(1).unwrap(), 100, 30));
    let spec = CommandSpec::with_default_rows("echo hello").unwrap();
    let _ = app.dispatch(Command::AddTile(spec.clone()));
    let id = app.state().grid.tiles[0].id;

    let pty = FakePtySpawner::new();
    pty.add_script(FakeScript {
        command_substring: "echo".to_string(),
        bytes: vec![b"hello\n".to_vec()],
        exit: ExitStatus::Code(0),
        spawn_error: None,
    });
    let mut spawned = pty.spawn(id, &spec).unwrap();
    let mut all = Vec::new();
    while let Some(chunk) = spawned.byte_chunks.next() {
        all.extend_from_slice(&chunk);
    }
    let _ = app.dispatch(Command::OnPtyBytes { id, bytes: all });

    let snap: RenderSnapshot = app.snapshot();
    let frame = build_view(&snap);
    match frame {
        FrameDescription::Tiles { tiles, .. } => {
            assert_eq!(tiles.len(), 1);
            let body_text: String = tiles[0].cells[0]
                .iter()
                .take_while(|c| c.ch != ' ')
                .map(|c| c.ch)
                .collect();
            assert!(body_text.contains("hello"), "body was: {body_text:?}");
        }
        FrameDescription::TooSmallBanner { .. } => panic!("unexpected banner"),
    }
}
