use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use schema_rs::{
    engine::Engine,
    model::Project,
    server::{router, AppState, Runtime},
};
use tower::ServiceExt;
#[tokio::test]
async fn preview_assets_and_preset_do_not_mutate_the_live_project() {
    let runtime = Runtime::start(Project::demo()).unwrap();
    let original = runtime.controller.current().project.clone();
    let app = router(AppState::new(
        runtime.controller.clone(),
        std::env::temp_dir(),
        None,
    ));
    for (path, mime) in [
        ("/preview", "text/html"),
        ("/scene-preview-renderer.mjs", "text/javascript"),
        ("/vendor/scene-three.mjs", "text/javascript"),
        ("/scene-assets/carpushit.glb", "model/gltf-binary"),
        ("/scene-assets/carpushit.project.json", "application/json"),
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        assert!(response.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with(mime));
        let body = to_bytes(response.into_body(), 12 * 1024 * 1024)
            .await
            .unwrap();
        if path.ends_with("project.json") {
            let project: Project = serde_json::from_slice(&body).unwrap();
            assert_eq!(project.fixtures.len(), 38);
            assert!(!project.output.enabled);
            let mut engine = Engine::new(project).unwrap();
            engine.tick(0.1);
            assert!(engine.snapshot().diagnostics.is_empty());
        }
        if path.ends_with(".glb") {
            assert_eq!(&body[..4], b"glTF");
        }
    }
    assert_eq!(runtime.controller.current().project, original);
}
#[tokio::test]
async fn preview_panels_allow_only_same_origin_embedding() {
    let runtime = Runtime::start(Project::demo()).unwrap();
    let app = router(AppState::new(
        runtime.controller.clone(),
        std::env::temp_dir(),
        None,
    ));
    for (path, expected) in [
        ("/preview?panel=example", "frame-ancestors 'self'"),
        ("/", "frame-ancestors 'none'"),
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert!(response.headers()["content-security-policy"]
            .to_str()
            .unwrap()
            .contains(expected));
    }
}
