use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use schema_rs::{
    model::Project,
    server::{router, AppState, Runtime},
};
use tower::ServiceExt;
#[tokio::test]
async fn phone_install_assets_are_embedded_and_controller_only() {
    let runtime = Runtime::start(Project::demo()).unwrap();
    let app = router(AppState::new(
        runtime.controller.clone(),
        std::env::temp_dir(),
        None,
    ));
    for (path, mime) in [
        ("/app.webmanifest", "application/manifest+json"),
        ("/install-ui.js", "text/javascript"),
        ("/mobile-worker.js", "text/javascript"),
        ("/offline.html", "text/html"),
        ("/app-icon-192.png", "image/png"),
        ("/app-icon-512.png", "image/png"),
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
        let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        if path.ends_with("webmanifest") {
            let m: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(m["start_url"], "/mobile");
            assert_eq!(m["display"], "standalone");
            assert_eq!(m["scope"], "/mobile");
            assert_eq!(m["icons"].as_array().unwrap().len(), 2);
        }
    }
}
