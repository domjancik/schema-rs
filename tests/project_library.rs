use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use schema_rs::{
    engine::Command,
    model::Project,
    project,
    server::{router, AppState, Runtime},
};
use tower::ServiceExt;
async fn request(
    app: axum::Router,
    method: &str,
    url: &str,
    body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let response = app
        .oneshot(
            Request::builder()
                .method(method)
                .uri(url)
                .header("content-type", "application/json")
                .body(if method == "GET" {
                    Body::empty()
                } else {
                    Body::from(body.to_string())
                })
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or_default())
}
#[tokio::test]
async fn named_projects_roundtrip_select_and_undo_without_affecting_clock() {
    let root = std::env::temp_dir().join(format!("schema-library-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let p = Project::demo();
    project::save(&root, &p, "init").unwrap();
    let runtime = Runtime::start(p.clone()).unwrap();
    let app = router(AppState::new(
        runtime.controller.clone(),
        root.clone(),
        None,
    ));
    let (status,_)=request(app.clone(),"POST","/api/projects/save",serde_json::json!({"id":"night","title":"Night scene","overwrite":false,"expected_revision":0})).await;
    assert_eq!(status, StatusCode::OK);
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("projects/night/project.json")).unwrap())
            .unwrap();
    let generation = manifest["generation"].as_str().unwrap();
    assert!(root
        .join(format!(
            "projects/night/revisions/{generation}/registry/layouts/init.json"
        ))
        .exists());
    assert!(root
        .join(format!(
            "projects/night/revisions/{generation}/registry/content/init.json"
        ))
        .exists());
    runtime
        .controller
        .command(
            Command::SetDimmer {
                fixture: "fixture-1".into(),
                value: 0.23,
            },
            None,
        )
        .await
        .unwrap();
    let before = runtime.controller.current();
    let (status, _) = request(
        app.clone(),
        "POST",
        "/api/projects/load",
        serde_json::json!({"id":"night","expected_revision":0}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(runtime.controller.current().project, before.project);
    let (status, _) = request(
        app.clone(),
        "POST",
        "/api/projects/load",
        serde_json::json!({"id":"night","expected_revision":before.snapshot.revision}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let loaded = runtime.controller.current();
    assert_eq!(loaded.project.name, "Night scene");
    assert_eq!(loaded.project.fixtures[0].dimmer, 1.);
    assert!(!loaded.project.output.enabled);
    assert!(loaded.snapshot.time >= before.snapshot.time);
    runtime
        .controller
        .command(Command::Undo, None)
        .await
        .unwrap();
    assert_eq!(runtime.controller.current().project, before.project);
    let (status, list) =
        request(app.clone(), "GET", "/api/projects", serde_json::Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    let entries = list["projects"].as_array().unwrap();
    assert!(entries
        .iter()
        .any(|p| p["id"] == "night" && p["title"] == "Night scene"));
    assert!(entries
        .iter()
        .any(|p| p["id"] == "init" && p["legacy"] == true));
    let (status,_)=request(app.clone(),"POST","/api/projects/save",serde_json::json!({"id":"night","title":"Changed","overwrite":false,"expected_revision":runtime.controller.current().snapshot.revision})).await;
    assert_eq!(status, StatusCode::CONFLICT);
    let again: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("projects/night/project.json")).unwrap())
            .unwrap();
    assert_eq!(manifest, again);
    for id in ["../escape", "C:\\escape", "bad/name", ""] {
        let (status,_)=request(app.clone(),"POST","/api/projects/save",serde_json::json!({"id":id,"title":"Bad","overwrite":false,"expected_revision":runtime.controller.current().snapshot.revision})).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    drop(app);
    drop(runtime);
    std::fs::remove_dir_all(root).unwrap();
}
#[tokio::test]
async fn projects_keep_independent_output_and_reject_invalid_loads_atomically() {
    let root = std::env::temp_dir().join(format!("schema-library-output-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let runtime = Runtime::start(Project::demo()).unwrap();
    let app = router(AppState::new(
        runtime.controller.clone(),
        root.clone(),
        None,
    ));
    for (id, port) in [("a", 6454), ("b", 6455)] {
        let mut config = runtime.controller.current().project.output.clone();
        config.target = format!("127.0.0.1:{port}");
        runtime
            .controller
            .command(Command::SetOutput { config }, None)
            .await
            .unwrap();
        let rev = runtime.controller.current().snapshot.revision;
        assert_eq!(
            request(
                app.clone(),
                "POST",
                "/api/projects/save",
                serde_json::json!({"id":id,"title":id,"overwrite":false,"expected_revision":rev})
            )
            .await
            .0,
            StatusCode::OK
        );
    }
    let rev = runtime.controller.current().snapshot.revision;
    assert_eq!(
        request(
            app.clone(),
            "POST",
            "/api/projects/load",
            serde_json::json!({"id":"a","expected_revision":rev})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        runtime.controller.current().project.output.target,
        "127.0.0.1:6454"
    );
    let before = runtime.controller.current();
    assert_eq!(
        request(
            app.clone(),
            "POST",
            "/api/projects/load",
            serde_json::json!({"id":"missing","expected_revision":before.snapshot.revision})
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(runtime.controller.current().project, before.project);
    drop(app);
    drop(runtime);
    std::fs::remove_dir_all(root).unwrap();
}
#[tokio::test]
async fn project_routes_share_auth_and_origin_guards() {
    let runtime = Runtime::start(Project::demo()).unwrap();
    let app = router(AppState::new(
        runtime.controller.clone(),
        std::env::temp_dir(),
        Some("test-projects-token".into()),
    ));
    assert_eq!(
        request(app.clone(), "GET", "/api/projects", serde_json::Value::Null)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/projects/load")
                .header("host", "127.0.0.1:8787")
                .header("origin", "https://evil.example")
                .header("x-schema-token", "test-projects-token")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"id":"init","expected_revision":0}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[test]
fn overwriting_keeps_prior_generation_and_does_not_clobber_unrelated_temp_files() {
    let root =
        std::env::temp_dir().join(format!("schema-library-generations-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let p = Project::demo();
    schema_rs::project_library::store(&root, &p, "show", "First", false, 1).unwrap();
    let pointer = root.join("projects/show/project.json");
    let first: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&pointer).unwrap()).unwrap();
    let other_temp = pointer.with_extension("tmp");
    std::fs::write(&other_temp, "unrelated").unwrap();
    schema_rs::project_library::store(&root, &p, "show", "Second", true, 2).unwrap();
    assert_eq!(std::fs::read_to_string(other_temp).unwrap(), "unrelated");
    let second: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&pointer).unwrap()).unwrap();
    assert_ne!(first["generation"], second["generation"]);
    let old = root
        .join("projects/show/revisions")
        .join(first["generation"].as_str().unwrap());
    assert_eq!(project::load(&old, "init").unwrap().name, "First");
    assert_eq!(
        schema_rs::project_library::load(&root, "show")
            .unwrap()
            .name,
        "Second"
    );
    assert!(schema_rs::project_library::store(&root, &p, "show", "\nInvalid", true, 3).is_err());
    assert_eq!(
        schema_rs::project_library::load(&root, "show")
            .unwrap()
            .name,
        "Second"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn content_selector_lists_only_content_files_and_requires_authentication() {
    let root = std::env::temp_dir().join(format!("schema-content-list-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let mut p = Project::demo();
    p.output.enabled = false;
    project::save(&root, &p, "init").unwrap();
    project::save_content(&root, &[], "night").unwrap();
    project::save_layout(&root, &p, "layout-only").unwrap();
    std::fs::write(root.join("registry/content/bad.name.json"), "{}").unwrap();
    std::fs::create_dir(root.join("registry/content/directory.json")).unwrap();
    let runtime = Runtime::start(p).unwrap();
    let app = router(AppState::new(
        runtime.controller.clone(),
        root.clone(),
        None,
    ));
    let (status, body) = request(app.clone(), "GET", "/api/content", serde_json::Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["contents"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["init", "night"]
    );
    assert_eq!(body["contents"][1]["targets"], serde_json::json!([]));
    assert!(!body["contents"][0]["targets"]
        .as_array()
        .unwrap()
        .is_empty());
    let (status, _) = request(
        app,
        "POST",
        "/api/project/save",
        serde_json::json!({"name":"stale","scope":"content","expected_revision":999}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(!root.join("registry/content/stale.json").exists());
    let app = router(AppState::new(
        runtime.controller.clone(),
        root.clone(),
        Some("1357".into()),
    ));
    let (status, _) = request(app, "GET", "/api/content", serde_json::Value::Null).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    drop(runtime);
    std::fs::remove_dir_all(root).unwrap();
}
