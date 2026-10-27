use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use schema_rs::{
    engine::{Command, Engine},
    model::Project,
    server::{router, AppState, Runtime},
};
use tower::ServiceExt;
#[tokio::test]
async fn update_barrier_holds_clock_actions_and_edits_until_validation_completes() {
    let root = std::env::temp_dir().join(format!("schema-update-barrier-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let barrier = root.join(".update-in-progress");
    std::fs::write(&barrier, b"validation").unwrap();
    let mut engine = Engine::new(Project::demo()).unwrap();
    engine
        .apply(Command::SetDimmer {
            fixture: "fixture-1".into(),
            value: 0.37,
        })
        .unwrap();
    engine.tick(0.2);
    let expected = engine.checkpoint();
    let runtime = Runtime::start_engine_guarded(
        Engine::from_session(expected.clone()).unwrap(),
        None,
        Some(barrier.clone()),
    )
    .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(70)).await;
    let held = runtime.controller.checkpoint().await.unwrap();
    assert_eq!(held.project, expected.project);
    assert_eq!(held.undo, expected.undo);
    assert_eq!(held.snapshot.frame, expected.snapshot.frame);
    assert_eq!(held.snapshot.time, expected.snapshot.time);
    assert!(runtime
        .controller
        .command(Command::Pause { paused: true }, None)
        .await
        .unwrap_err()
        .contains("updating"));
    let app = router(AppState::new(
        runtime.controller.clone(),
        root.clone(),
        None,
    ));
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/state")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/command")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"command":{"type":"undo"}}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    std::fs::remove_file(&barrier).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(70)).await;
    assert!(runtime.controller.current().snapshot.frame > expected.snapshot.frame);
    assert_eq!(runtime.controller.current().project, *expected.project);
    runtime
        .controller
        .command(Command::Undo, None)
        .await
        .unwrap();
    drop(runtime);
    let _ = std::fs::remove_dir_all(root);
}
