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
#[tokio::test]
async fn product_page_and_real_download_availability_are_separate_from_studio() {
    let runtime = Runtime::start(Project::demo()).unwrap();
    let root = std::env::temp_dir().join(format!("schema-product-download-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let app = router(AppState::new(
        runtime.controller.clone(),
        root.clone(),
        None,
    ));
    for path in ["/welcome", "/landing.css", "/landing.js"] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{path}");
    }
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/downloads/latest.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let value: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap()).unwrap();
    assert_eq!(value["available"], false);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/downloads/Schema-s-Windows-x64.zip")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let downloads = root.join("downloads");
    std::fs::create_dir_all(&downloads).unwrap();
    std::fs::write(downloads.join("Schema-s-Windows-x64.zip"), b"PKab").unwrap();
    std::fs::write(downloads.join("latest.json"),serde_json::json!({"available":true,"version":"0.1.0-preview","commit":"abc123","sha256":"a".repeat(64),"bytes":4,"filename":"Schema-s-Windows-x64.zip"}).to_string()).unwrap();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/downloads/latest.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let value: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap()).unwrap();
    assert_eq!(value["available"], true);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/downloads/Schema-s-Windows-x64.zip")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers()["content-disposition"]
        .to_str()
        .unwrap()
        .contains("attachment"));
    assert_eq!(
        &to_bytes(response.into_body(), 100).await.unwrap()[..],
        b"PKab"
    );
    let _ = std::fs::remove_dir_all(root);
}
