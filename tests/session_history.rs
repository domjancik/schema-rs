use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use schema_rs::{
    engine::Command,
    model::Project,
    server::{router, AppState, Runtime},
};
use tower::ServiceExt;
#[tokio::test]
async fn session_export_contains_current_project_and_both_history_stacks() {
    let runtime = Runtime::start(Project::demo()).unwrap();
    runtime
        .controller
        .command(
            Command::MoveFixture {
                fixture: "fixture-1".into(),
                position: [1., 2., 3.],
            },
            None,
        )
        .await
        .unwrap();
    runtime
        .controller
        .command(
            Command::SetDimmer {
                fixture: "fixture-1".into(),
                value: 0.4,
            },
            None,
        )
        .await
        .unwrap();
    runtime
        .controller
        .command(Command::Undo, None)
        .await
        .unwrap();
    let app = router(AppState::new(
        runtime.controller.clone(),
        std::env::temp_dir(),
        None,
    ));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/session")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let session: serde_json::Value = serde_json::from_slice(
        &to_bytes(response.into_body(), 8 * 1024 * 1024)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(session["version"], 1);
    assert_eq!(session["undo"].as_array().unwrap().len(), 1);
    assert_eq!(session["redo"].as_array().unwrap().len(), 1);
    assert_eq!(
        session["project"]["fixtures"][0]["position"],
        serde_json::json!([1., 2., 3.])
    );
    assert_eq!(session["snapshot"]["revision"], 3);
}
#[test]
fn serialized_session_restores_undo_redo_and_new_edit_discards_redo() {
    use schema_rs::engine::Engine;
    let mut engine = Engine::new(Project::demo()).unwrap();
    let original = engine.project().clone();
    engine
        .apply(Command::MoveFixture {
            fixture: "fixture-1".into(),
            position: [2., 3., 4.],
        })
        .unwrap();
    let moved = engine.project().clone();
    engine
        .apply(Command::SetDimmer {
            fixture: "fixture-1".into(),
            value: 0.3,
        })
        .unwrap();
    let dimmed = engine.project().clone();
    engine.apply(Command::Undo).unwrap();
    engine.tick(0.);
    let json = serde_json::to_vec(&engine.checkpoint()).unwrap();
    let checkpoint = serde_json::from_slice(&json).unwrap();
    let mut resumed = Engine::from_session(checkpoint).unwrap();
    assert_eq!(resumed.project(), &moved);
    assert_eq!(resumed.revision(), 3);
    resumed.apply(Command::Redo).unwrap();
    assert_eq!(resumed.project(), &dimmed);
    resumed.apply(Command::Undo).unwrap();
    resumed.apply(Command::Undo).unwrap();
    assert_eq!(resumed.project(), &original);
    resumed
        .apply(Command::MoveFixture {
            fixture: "fixture-1".into(),
            position: [9., 0., 0.],
        })
        .unwrap();
    assert!(resumed.apply(Command::Redo).is_err());
}
#[test]
fn serialized_runtime_continues_rng_filters_and_clock_without_reseeding() {
    use schema_rs::{engine::Engine, scenic};
    let mut project = Project::demo();
    project.content.get_mut("fixture-1").unwrap()[0]=Some(scenic::parse("name: Random\nstack:\n  Create.Value.Random: {Seed: 42, Interval: 0.2, Filter Time: 0.15}").unwrap());
    let mut original = Engine::new(project).unwrap();
    for _ in 0..17 {
        original.tick(0.1);
    }
    let checkpoint =
        serde_json::from_slice(&serde_json::to_vec(&original.checkpoint()).unwrap()).unwrap();
    let mut resumed = Engine::from_session(checkpoint).unwrap();
    for _ in 0..20 {
        original.tick(0.1);
        resumed.tick(0.1);
        assert_eq!(original.snapshot().time, resumed.snapshot().time);
        assert_eq!(
            original.snapshot().fixtures[0].result,
            resumed.snapshot().fixtures[0].result
        );
    }
}
#[test]
fn unknown_version_bad_history_and_foreign_runtime_keys_are_rejected() {
    use schema_rs::{engine::Engine, session::Session};
    let mut engine = Engine::new(Project::demo()).unwrap();
    engine.tick(0.);
    let good = serde_json::to_value(engine.checkpoint()).unwrap();
    for kind in 0..3 {
        let mut bad = good.clone();
        match kind {
            0 => bad["version"] = serde_json::json!(99),
            1 => {
                bad["undo"] = serde_json::json!(vec![bad["project"].clone(); 129]);
            }
            _ => {
                let states = bad["states"].as_object_mut().unwrap();
                let state = states.values().next().unwrap().clone();
                states.insert("foreign/0/1".into(), state);
            }
        }
        let session: Session = serde_json::from_value(bad).unwrap();
        assert!(Engine::from_session(session).is_err());
    }
}
#[tokio::test]
async fn persistent_runtime_flushes_final_session_and_preserves_redo() {
    use schema_rs::{engine::Engine, session};
    let root = std::env::temp_dir().join(format!(
        "schema-history-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let path = root.join(".session/session.json");
    let runtime =
        Runtime::start_engine(Engine::new(Project::demo()).unwrap(), Some(path.clone())).unwrap();
    runtime
        .controller
        .command(
            Command::SetDimmer {
                fixture: "fixture-1".into(),
                value: 0.2,
            },
            None,
        )
        .await
        .unwrap();
    runtime
        .controller
        .command(Command::Undo, None)
        .await
        .unwrap();
    let expected = runtime.controller.current().project.clone();
    runtime.controller.persist_session().await.unwrap();
    assert_eq!(runtime.controller.session_status().saved_revision, Some(2));
    drop(runtime);
    let session = session::load(&path).unwrap();
    assert_eq!(*session.project, expected);
    assert_eq!(session.redo.len(), 1);
    let mut resumed = Engine::from_session(session).unwrap();
    resumed.apply(Command::Redo).unwrap();
    assert_eq!(resumed.project().fixtures[0].dimmer, 0.2);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(root.join(".session")).unwrap();
    std::fs::remove_dir(root).unwrap();
}
#[test]
fn repeated_atomic_checkpoints_do_not_leave_partial_files() {
    use schema_rs::{
        engine::Engine,
        session::{self, Store},
    };
    let root = std::env::temp_dir().join(format!(
        "schema-history-atomic-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let path = root.join("session.json");
    let store = Store::new(path.clone());
    let mut engine = Engine::new(Project::demo()).unwrap();
    engine.tick(0.);
    store.save(&engine.checkpoint()).unwrap();
    engine
        .apply(Command::SetDimmer {
            fixture: "fixture-1".into(),
            value: 0.4,
        })
        .unwrap();
    engine.tick(0.);
    store.save(&engine.checkpoint()).unwrap();
    assert_eq!(session::load(&path).unwrap().undo.len(), 1);
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(root).unwrap();
}
#[test]
fn invalid_checkpoint_cannot_replace_last_good_history() {
    use schema_rs::{
        engine::Engine,
        session::{self, Store},
    };
    let root = std::env::temp_dir().join(format!(
        "schema-history-invalid-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let path = root.join("session.json");
    let store = Store::new(path.clone());
    let mut engine = Engine::new(Project::demo()).unwrap();
    engine.tick(0.);
    let mut checkpoint = engine.checkpoint();
    store.save(&checkpoint).unwrap();
    checkpoint.version = 99;
    assert!(store.save(&checkpoint).is_err());
    assert_eq!(session::load(&path).unwrap().version, 1);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(root).unwrap();
}
#[test]
fn valid_deep_program_can_be_restored_from_a_checkpoint() {
    use schema_rs::{
        engine::Engine,
        model::{Node, Stack},
        session::{self, Store},
    };
    use std::collections::BTreeMap;
    let mut root = Node {
        id: 64,
        behavior: "Create.Value.Constant".into(),
        properties: BTreeMap::new(),
        children: vec![],
        drivers: BTreeMap::new(),
    };
    for id in (1..64).rev() {
        root = Node {
            id,
            behavior: "Tool.Merge".into(),
            properties: BTreeMap::new(),
            children: vec![root],
            drivers: BTreeMap::new(),
        };
    }
    let mut p = Project::demo();
    p.content.get_mut("fixture-1").unwrap()[0] = Some(Stack {
        name: "Deep".into(),
        root,
    });
    let engine = Engine::new(p).unwrap();
    let folder = std::env::temp_dir().join(format!(
        "schema-history-deep-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let path = folder.join("session.json");
    Store::new(path.clone()).save(&engine.checkpoint()).unwrap();
    let loaded = session::load(&path);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(folder).unwrap();
    assert!(loaded.is_ok(), "{:?}", loaded.err());
}
#[test]
fn automatic_recovery_disarms_output_in_current_and_historical_projects() {
    use schema_rs::engine::Engine;
    let mut project = Project::demo();
    project.output.enabled = true;
    let mut engine = Engine::new(project).unwrap();
    engine
        .apply(Command::SetDimmer {
            fixture: "fixture-1".into(),
            value: 0.2,
        })
        .unwrap();
    engine
        .apply(Command::SetDimmer {
            fixture: "fixture-1".into(),
            value: 0.6,
        })
        .unwrap();
    engine.apply(Command::Undo).unwrap();
    let mut session = engine.checkpoint();
    session.disarm_output();
    assert!(!session.project.output.enabled);
    assert!(session
        .undo
        .iter()
        .chain(&session.redo)
        .all(|p| !p.output.enabled));
    let mut restored = Engine::from_session(session).unwrap();
    restored.apply(Command::Redo).unwrap();
    assert!(!restored.project().output.enabled);
    restored.apply(Command::Undo).unwrap();
    restored.apply(Command::Undo).unwrap();
    assert!(!restored.project().output.enabled);
}
#[tokio::test]
async fn session_routes_keep_token_and_origin_guards() {
    let runtime = Runtime::start(Project::demo()).unwrap();
    let app = router(AppState::new(
        runtime.controller.clone(),
        std::env::temp_dir(),
        Some("session-test-token".into()),
    ));
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/session")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/session/save")
                .header("host", "127.0.0.1:8790")
                .header("origin", "https://other.example")
                .header("x-schema-token", "session-test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/session")
                .header("x-schema-token", "session-test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
