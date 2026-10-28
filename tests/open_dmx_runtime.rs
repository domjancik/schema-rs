use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use schema_rs::{
    engine::Command,
    model::{OutputTransport, Project},
    server::{router, AppState, Runtime},
};
use tower::ServiceExt;
#[tokio::test]
async fn discovery_requires_existing_api_auth_and_reports_platform() {
    let runtime = Runtime::start(Project::demo()).unwrap();
    let app = router(AppState::new(
        runtime.controller.clone(),
        std::env::temp_dir(),
        Some("test-only-token".into()),
    ));
    let missing = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/output/devices")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::UNAUTHORIZED);
    let good = app
        .oneshot(
            Request::builder()
                .uri("/api/output/devices")
                .header("x-schema-token", "test-only-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(good.status(), StatusCode::OK);
    let json: serde_json::Value =
        serde_json::from_slice(&to_bytes(good.into_body(), 100_000).await.unwrap()).unwrap();
    assert_eq!(json["open_dmx"], cfg!(windows));
    assert!(json["devices"].is_array());
}
#[tokio::test]
async fn output_selection_is_undoable_and_missing_usb_never_stalls_evaluation() {
    let runtime = Runtime::start(Project::demo()).unwrap();
    let c = &runtime.controller;
    let original = c.current();
    let mut config = original.project.output.clone();
    config.transport = OutputTransport::OpenDmx;
    config.device = "SCHEMA_TEST_MISSING".into();
    config.fps = 30;
    config.universe = 7;
    c.command(
        Command::SetOutput {
            config: config.clone(),
        },
        Some(0),
    )
    .await
    .unwrap();
    assert_eq!(c.current().project.output, config);
    c.command(Command::Undo, None).await.unwrap();
    assert_eq!(c.current().project.output, original.project.output);
    c.command(Command::Redo, None).await.unwrap();
    assert_eq!(c.current().project.output, config);
    config.enabled = true;
    c.command(Command::SetOutput { config }, None)
        .await
        .unwrap();
    let before = c.current().snapshot.frame;
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    let now = c.current();
    assert!(now.snapshot.frame > before);
    assert!(
        now.snapshot.output_status.starts_with("Output error:"),
        "{}",
        now.snapshot.output_status
    );
    let mut disabled = now.project.output.clone();
    disabled.enabled = false;
    c.command(Command::SetOutput { config: disabled }, None)
        .await
        .unwrap();
}
