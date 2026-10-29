use schema_rs::{engine::Command, model::Project, runtime::EngineSession};
#[test]
fn commands_are_revision_checked_and_history_survives_host_storage_roundtrip() {
    let mut host = EngineSession::new(Project::demo()).unwrap();
    let original = host.publication().project.fixtures[0].dimmer;
    host.command(
        Command::SetDimmer {
            fixture: "fixture-1".into(),
            value: 0.37,
        },
        Some(0),
    )
    .unwrap();
    assert!(host.command(Command::Undo, Some(0)).is_err());
    host.advance(0.1).unwrap();
    let bytes = host.encode_session().unwrap();
    let mut restored = EngineSession::decode_session(&bytes).unwrap();
    assert_eq!(restored.publication().snapshot.revision, 1);
    assert!(restored.publication().snapshot.can_undo);
    restored.command(Command::Undo, Some(1)).unwrap();
    assert_eq!(restored.publication().project.fixtures[0].dimmer, original);
    assert!(restored.publication().snapshot.can_redo);
}
#[test]
fn host_rejects_invalid_frame_deltas_without_advancing() {
    let mut host = EngineSession::new(Project::demo()).unwrap();
    assert!(host.advance(f64::NAN).is_err());
    assert!(host.advance(-0.1).is_err());
    assert_eq!(host.publication().snapshot.frame, 0);
    host.advance(0.016).unwrap();
    assert_eq!(host.publication().snapshot.frame, 1);
}

#[test]
fn portable_host_matches_native_evaluation_and_isolates_instances() {
    let project = Project::demo();
    let mut native = schema_rs::engine::Engine::new(project.clone()).unwrap();
    let mut portable = EngineSession::new(project.clone()).unwrap();
    let untouched = EngineSession::new(project).unwrap();
    for delta in [0.01, 0.02, 0.1] {
        native.tick(delta);
        let frame = portable.advance(delta).unwrap();
        let expected = native.snapshot();
        assert_eq!(frame.snapshot.universes, expected.universes);
        assert_eq!(
            serde_json::to_value(&frame.snapshot.fixtures).unwrap(),
            serde_json::to_value(expected.fixtures).unwrap()
        );
    }
    portable
        .command(
            Command::SetDimmer {
                fixture: "fixture-1".into(),
                value: 0.2,
            },
            Some(0),
        )
        .unwrap();
    assert_eq!(untouched.publication().snapshot.revision, 0);
    assert_eq!(untouched.publication().project.fixtures[0].dimmer, 1.0);
}
