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
async fn hosted_clients_share_commands_history_and_validation() {
    let runtime = Runtime::start(Project::demo()).unwrap();
    let app = router(AppState::new(
        runtime.controller.clone(),
        std::env::temp_dir().join("schema-rs-api-tests"),
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
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    let state: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(state["project"]["fixtures"].as_array().unwrap().len(), 8);
    let command = serde_json::json!({"command":{"type":"move_fixture","fixture":"fixture-1","position":[1,2,3]}});
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/command")
                .header("content-type", "application/json")
                .body(Body::from(command.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    runtime
        .controller
        .command(Command::Undo, None)
        .await
        .unwrap();
}
#[tokio::test]
async fn cross_origin_mutations_are_rejected() {
    let runtime = Runtime::start(Project::demo()).unwrap();
    let app = router(AppState::new(
        runtime.controller.clone(),
        std::env::temp_dir(),
        None,
    ));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/command")
                .header("host", "127.0.0.1:8787")
                .header("origin", "https://evil.example")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"command":{"type":"undo"}}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
#[tokio::test]
async fn retained_ui_snapshot_does_not_stop_engine() {
    let runtime = Runtime::start(Project::demo()).unwrap();
    let snapshot = runtime.controller.current();
    let frame = snapshot.snapshot.frame;
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while runtime.controller.current().snapshot.frame <= frame + 2 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("retaining an old publication must not block evaluation");
    assert!(runtime.controller.current().snapshot.frame > frame + 2);
    assert_eq!(snapshot.snapshot.frame, frame);
}
#[tokio::test]
async fn revision_conflicts_reject_stale_edits() {
    let runtime = Runtime::start(Project::demo()).unwrap();
    runtime
        .controller
        .command(
            Command::MoveFixture {
                fixture: "fixture-1".into(),
                position: [1., 0., 0.],
            },
            Some(0),
        )
        .await
        .unwrap();
    assert!(runtime
        .controller
        .command(Command::Undo, Some(0))
        .await
        .is_err());
}

#[tokio::test]
async fn native_studio_serves_shared_interface_assets() {
    let runtime = Runtime::start(Project::demo()).unwrap();
    let app = router(AppState::new(
        runtime.controller.clone(),
        std::env::temp_dir(),
        None,
    ));
    for path in [
        "/",
        "/mobile",
        "/preview",
        "/interface.css",
        "/fixture-placement.js",
        "/fixture-placement.css",
        "/servers-core.js",
        "/servers-ui.js",
        "/servers.css",
        "/app-updates.js",
        "/vendor/qrcode.js",
        "/vendor/jsQR.js",
        "/vendor/QR-LICENSES.txt",
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        let body = to_bytes(response.into_body(), 2 * 1024 * 1024)
            .await
            .unwrap();
        let body = std::str::from_utf8(&body).unwrap();
        if path.starts_with("/vendor/") {
            assert!(!body.is_empty());
        } else if path == "/servers.css" {
            assert!(body.contains(".servers-dialog"));
        } else if path.ends_with(".js") && path != "/fixture-placement.js" {
            assert!(body.contains("use strict"));
        } else if path == "/fixture-placement.js" {
            assert!(body.contains("SchemaPlacement"));
        } else if path == "/fixture-placement.css" {
            assert!(body.contains("fixture-placement-overlay"));
        } else if path.ends_with(".css") {
            assert!(body.contains("--ui-accent-fill"));
        } else {
            assert!(body.contains("href=\"/interface.css\""), "{path}");
        }
    }
}
