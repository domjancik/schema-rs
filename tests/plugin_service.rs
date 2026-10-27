use schema_rs::{
    model::Project,
    plugins::Registry,
    server::{AppState, Runtime},
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
fn manifest(min: f64) -> String {
    serde_json::json!({"api_version":1,"id":"demo","version":"1.0","components":[{"name":"Gain","description":"Native gain component for watcher tests.","defaults":{"Gain":3.0},"graph":{"Modify.Value.MapTo":{"Minimum":min,"Maximum":"$Gain","in":[{"Plugin.Input":null}]}}}]}).to_string()
}
#[tokio::test]
async fn file_loading_hot_reload_errors_unload_and_configuration_restore() {
    let root = std::env::temp_dir().join(format!("schema-plugin-test-{}", std::process::id()));
    let directory = root.join("plugins/demo");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("plugin.json"), manifest(0.)).unwrap();
    let runtime = Runtime::start(Project::demo()).unwrap();
    let app = AppState::new(runtime.controller.clone(), root.clone(), None);
    let watcher = app.start_plugin_watcher().unwrap();
    app.plugins.load("demo").await.unwrap();
    let catalog = runtime.controller.plugin_catalog();
    assert!(catalog.iter().any(|v| v["key"] == "Plugin.demo.Gain"));
    let registry = runtime.controller.current_plugins();
    let stack=schema_rs::scenic::with_plugins(registry,||schema_rs::scenic::parse("name: Dynamic\nstack:\n  Plugin.demo.Gain:\n    in:\n      - Create.Value.Constant: {Value: 0.4}\n")).unwrap();
    runtime
        .controller
        .command(
            schema_rs::engine::Command::ReplaceLayer {
                fixture: "fixture-1".into(),
                layer: 0,
                stack: Some(stack),
            },
            None,
        )
        .await
        .unwrap();
    let revision = runtime.controller.current().snapshot.revision;
    std::fs::write(directory.join("plugin.json"), manifest(1.)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if runtime.controller.current().snapshot.fixtures[0]
            .result
            .value
            .is_some_and(|v| (v - 1.8).abs() < 1e-9)
        {
            break;
        }
        assert!(Instant::now() < deadline, "hot reload timed out");
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
    assert_eq!(runtime.controller.current().snapshot.revision, revision);
    let generation = runtime.controller.current_plugins().state.generation;
    std::fs::write(directory.join("plugin.json"), "{broken").unwrap();
    tokio::time::sleep(Duration::from_millis(900)).await;
    assert_eq!(
        runtime.controller.current_plugins().state.generation,
        generation
    );
    assert!(app
        .plugins
        .status()
        .await
        .unwrap()
        .errors
        .contains_key("demo"));
    app.plugins.unload("demo").await.unwrap();
    let unloaded_generation = runtime.controller.current_plugins().state.generation;
    std::fs::write(directory.join("plugin.json"), manifest(0.5)).unwrap();
    tokio::time::sleep(Duration::from_millis(700)).await;
    assert_eq!(
        runtime.controller.current_plugins().state.generation,
        unloaded_generation
    );
    assert!(!runtime
        .controller
        .plugin_catalog()
        .iter()
        .any(|v| v["key"] == "Plugin.demo.Gain"));
    assert!(runtime
        .controller
        .current()
        .snapshot
        .diagnostics
        .iter()
        .any(|d| d.contains("unloaded")));
    let (saved, _) = schema_rs::plugin_host::load_configuration(&root).unwrap();
    assert!(!saved.state.packages["demo"].enabled);
    drop(watcher);
    drop(runtime);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn loading_rejects_resource_escape_and_dynamic_structural_operations() {
    let registry = Registry::default();
    assert!(registry.load_json("../escape", &manifest(0.)).is_err());
    let mut value: serde_json::Value = serde_json::from_str(&manifest(0.)).unwrap();
    value["components"][0]["graph"] = serde_json::json!({"Transition.Fade":null});
    assert!(registry.load_json("demo", &value.to_string()).is_err());
    let _: Arc<Registry> = Arc::new(registry);
}

#[tokio::test]
async fn plugin_http_routes_require_token_and_same_origin() {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let runtime = Runtime::start(Project::demo()).unwrap();
    let app = schema_rs::server::router(AppState::new(
        runtime.controller.clone(),
        std::env::temp_dir(),
        Some("component-test-token".into()),
    ));
    for path in [
        "/api/plugins",
        "/api/plugins/load",
        "/api/plugins/reload",
        "/api/plugins/unload",
        "/api/plugins/watch",
    ] {
        let method = if path == "/api/plugins" {
            "GET"
        } else {
            "POST"
        };
        let request = Request::builder()
            .method(method)
            .uri(path)
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();
        assert_eq!(
            app.clone().oneshot(request).await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );
        let request = Request::builder()
            .method(method)
            .uri(path)
            .header("host", "127.0.0.1:8787")
            .header("origin", "https://elsewhere.example")
            .header("x-schema-token", "component-test-token")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap();
        assert_eq!(
            app.clone().oneshot(request).await.unwrap().status(),
            StatusCode::FORBIDDEN
        );
    }
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/plugins")
                .header("x-schema-token", "component-test-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
