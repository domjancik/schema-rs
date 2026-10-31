use schema_rs::{
    bundle::Bundle,
    engine::{Command, Engine},
    library::{Item, Kind},
    model::Project,
    scenic,
};
#[test]
fn portable_library_store_is_non_destructive_and_conflict_checked() {
    let root = std::env::temp_dir().join(format!(
        "schema-transfer-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = Engine::new(Project::demo()).unwrap();
    let before = serde_json::to_value(source.checkpoint()).unwrap();
    let mut bundle = Bundle::from_session(&source.checkpoint());
    bundle
        .resources
        .insert("example.txt".into(), b"portable".to_vec());
    let item = Item::new(Kind::Project, "night".into(), "Night".into(), bundle, None);
    let stored = schema_rs::library::store(&root, &item, None).unwrap();
    assert!(schema_rs::library::store(&root, &item, None).is_err());
    let roundtrip = schema_rs::library::read(&root, &stored.key).unwrap();
    assert_eq!(roundtrip.bundle.resources["example.txt"], b"portable");
    assert!(schema_rs::library::store(&root, &item, Some("stale")).is_err());
    schema_rs::library::store(&root, &item, Some(&stored.hash)).unwrap();
    assert_eq!(serde_json::to_value(source.checkpoint()).unwrap(), before);
    assert!(schema_rs::library::read(&root, "../night").is_err());
    let mut invalid = item.clone();
    invalid.version = 99;
    assert!(invalid.validate().is_err());
    invalid.version = 1;
    invalid.bundle.resources.insert("../escape".into(), vec![1]);
    assert!(invalid.validate().is_err());
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn content_apply_preserves_layout_output_and_other_layer_state_with_one_undo() {
    let mut p = Project::demo();
    p.output.enabled = true;
    p.content.get_mut("fixture-1").unwrap()[1] = Some(
        scenic::parse("name: Running\nstack:\n  Create.Value.Random: {Interval: 20, Seed: 7}")
            .unwrap(),
    );
    let mut target = Engine::new(p).unwrap();
    target.tick(0.2);
    target.tick(0.2);
    let before = target.checkpoint();
    let source = Engine::new(Project::demo()).unwrap();
    let entries = vec![schema_rs::model::ContentEntry {
        fixture: "fixture-1".into(),
        layer: 0,
        stack: Some(
            scenic::parse("name: New\nstack:\n  Create.Value.Constant: {Value: 0.3}").unwrap(),
        ),
    }];
    let item = Item::new(
        Kind::Content,
        "look".into(),
        "Look".into(),
        Bundle::from_session(&source.checkpoint()),
        Some(schema_rs::storage_formats::Content {
            version: schema_rs::migrations::CONTENT_VERSION,
            entries,
        }),
    );
    let prepared = Engine::prepare_library(target.checkpoint(), item).unwrap();
    target.install_bundle(prepared).unwrap();
    let after = target.checkpoint();
    assert_eq!(after.project.fixtures, before.project.fixtures);
    assert_eq!(after.project.output, before.project.output);
    assert_eq!(after.undo.len(), before.undo.len() + 1);
    let a = serde_json::to_value(&after).unwrap();
    let b = serde_json::to_value(&before).unwrap();
    assert_eq!(a["states"]["fixture-1/1/1"], b["states"]["fixture-1/1/1"]);
    target.apply(Command::Undo).unwrap();
    assert_eq!(target.project(), before.project.as_ref());
}

#[test]
fn peer_addresses_and_keys_are_constrained() {
    use schema_rs::library_api::{allowed_address, Peer};
    for origin in [
        "file:///etc/passwd",
        "http://user:pass@127.0.0.1:8787",
        "http://127.0.0.1:8787/api/session",
        "http://127.0.0.1:8787/?token=secret",
    ] {
        assert!(Peer {
            origin: origin.into(),
            token: String::new()
        }
        .url()
        .is_err());
    }
    assert!(Peer {
        origin: "http://127.0.0.1:8787".into(),
        token: "1357".into()
    }
    .url()
    .is_ok());
    // Synthetic address-class fixtures; these are not configured deployment targets.
    for ip in ["127.0.0.1", "10.23.45.67", "192.168.234.56", "::1", "fd00::1"] {
        assert!(allowed_address(ip.parse().unwrap()));
    }
    for ip in [
        "0.0.0.0",
        "8.8.8.8",
        "169.254.169.254",
        "224.0.0.1",
        "::ffff:169.254.169.254",
        "2606:4700:4700::1111",
    ] {
        assert!(!allowed_address(ip.parse().unwrap()));
    }
}
#[tokio::test]
async fn actual_two_server_transfer_checks_auth_and_never_activates() {
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use schema_rs::server::{router, AppState, Runtime};
    use tower::ServiceExt;
    let base = std::env::temp_dir().join(format!("schema-library-network-{}", std::process::id()));
    let a = base.join("a");
    let b = base.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    let source = Runtime::start(Project::demo()).unwrap();
    let destination = Runtime::start(Project::demo()).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let remote = router(AppState::new(
        destination.controller.clone(),
        b.clone(),
        Some("1357".into()),
    ));
    let task = tokio::spawn(async move {
        axum::serve(
            listener,
            remote.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .unwrap()
    });
    let app = router(AppState::new(source.controller.clone(), a.clone(), None));
    async fn req(
        app: axum::Router,
        path: &str,
        value: serde_json::Value,
    ) -> (axum::http::StatusCode, serde_json::Value) {
        let r = app
            .oneshot(
                Request::post(path)
                    .header("content-type", "application/json")
                    .body(Body::from(value.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = r.status();
        let bytes = to_bytes(r.into_body(), 64 * 1024 * 1024).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap_or_default())
    }
    let before = destination.controller.checkpoint().await.unwrap();
    let (status, item) = req(
        app.clone(),
        "/api/library/export",
        serde_json::json!({"key":"current"}),
    )
    .await;
    assert!(status.is_success(), "{item}");
    assert!(
        item["bundle"].is_string(),
        "Typed payload must survive JavaScript relays unchanged"
    );
    let peer = serde_json::json!({"origin":format!("http://{addr}"),"token":"1357"});
    let (status, result) = req(
        app.clone(),
        "/api/library/peer",
        serde_json::json!({"peer":peer,"operation":"import","item":item}),
    )
    .await;
    assert!(status.is_success(), "{result}");
    assert_eq!(result["stored_only"], true);
    let (status, result) = req(
        app.clone(),
        "/api/library/peer",
        serde_json::json!({"peer":peer,"operation":"list"}),
    )
    .await;
    assert!(status.is_success());
    assert!(result["entries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["key"] == "portable:project:current-project"));
    let before_content = destination.controller.current().project.clone();
    let source_bundle = Bundle::from_session(&source.controller.checkpoint().await.unwrap());
    let content = Item::new(
        Kind::Content,
        "network-look".into(),
        "Network look".into(),
        source_bundle,
        Some(schema_rs::storage_formats::Content {
            version: schema_rs::migrations::CONTENT_VERSION,
            entries: vec![schema_rs::model::ContentEntry {
                fixture: "fixture-1".into(),
                layer: 0,
                stack: None,
            }],
        }),
    );
    let (status, result) = req(
        app.clone(),
        "/api/library/peer",
        serde_json::json!({"peer":peer,"operation":"import","item":content}),
    )
    .await;
    assert!(status.is_success(), "{result}");
    assert!(
        !b.join("registry/content/network-look.json").exists(),
        "Staging must not trigger an armed Director registry lookup"
    );
    let response=req(router(AppState::new(destination.controller.clone(),b.clone(),None)),"/api/library/publish",serde_json::json!({"key":"portable:content:network-look","expected_revision":before.snapshot.revision})).await;
    assert!(response.0.is_success(), "{}", response.1);
    assert!(b.join("registry/content/network-look.json").is_file());
    assert_eq!(destination.controller.current().project, before_content);
    let after = destination.controller.checkpoint().await.unwrap();
    assert_eq!(before.project, after.project);
    assert_eq!(before.undo, after.undo);
    assert_eq!(before.snapshot.revision, after.snapshot.revision);
    let (status,_) =req(app,"/api/library/peer",serde_json::json!({"peer":{"origin":format!("http://{addr}"),"token":"wrong"},"operation":"list"})).await;
    assert!(!status.is_success());
    task.abort();
    std::fs::remove_dir_all(base).unwrap();
}

#[test]
fn staging_validates_in_isolation_while_activation_rejects_package_conflicts() {
    let engine = Engine::new(Project::demo()).unwrap();
    let mut bundle = Bundle::from_session(&engine.checkpoint());
    let package = serde_json::json!({"folder":"gain","enabled":true,"manifest":{"api_version":1,"id":"demo","version":"1.0.0","components":[{"name":"Gain","description":"Portable graph","defaults":{"Gain":2},"graph":{"Create.Value.Constant":{"Value":"$Gain"}}}]}});
    bundle.plugins.packages.insert(
        "demo".into(),
        serde_json::from_value(package.clone()).unwrap(),
    );
    let item = Item::new(
        Kind::Project,
        "plugin-copy".into(),
        "Plugin copy".into(),
        bundle.clone(),
        None,
    );
    let mut conflicting = bundle.plugins.clone();
    let mut different = package;
    different["manifest"]["components"][0]["defaults"]["Gain"] = serde_json::json!(3);
    conflicting
        .packages
        .insert("demo".into(), serde_json::from_value(different).unwrap());
    let registry = std::sync::Arc::new(schema_rs::plugins::Registry::compile(conflicting).unwrap());
    schema_rs::scenic::with_plugins(registry.clone(), || item.validate()).unwrap();
    let target = Engine::new_with_plugins(Project::demo(), registry).unwrap();
    assert!(Engine::prepare_library(target.checkpoint(), item).is_err());
}

#[test]
fn unrelated_workspace_init_snapshot_does_not_block_project_loading() {
    use schema_rs::inputs::Event;
    let mut target = Engine::new(Project::demo()).unwrap();
    let old = vec![schema_rs::model::ContentEntry {
        fixture: "fixture-1".into(),
        layer: 0,
        stack: None,
    }];
    target
        .apply(Command::Input {
            event: Event::Content {
                name: "init".into(),
                entries: old.clone(),
            },
        })
        .unwrap();
    let mut bundle = Bundle::from_session(&target.checkpoint());
    bundle.assets.content.insert("init".into(), vec![]);
    let item = Item::new(
        Kind::Project,
        "new-project".into(),
        "New project".into(),
        bundle.clone(),
        None,
    );
    let prepared = Engine::prepare_library(target.checkpoint(), item).unwrap();
    target.install_bundle(prepared).unwrap();
    assert_eq!(
        serde_json::to_value(&target.checkpoint().inputs.content["init"]).unwrap(),
        serde_json::to_value(old).unwrap()
    );
    bundle.project.director[0] = Some(
        scenic::parse("name: Uses init\nstack:\n  Assign.Content: {Content: init, Enabled: false}")
            .unwrap(),
    );
    let referenced = Item::new(
        Kind::Project,
        "uses-init".into(),
        "Uses init".into(),
        bundle,
        None,
    );
    assert!(
        Engine::prepare_library(target.checkpoint(), referenced).is_err(),
        "An explicit init dependency must still be checked for conflicts"
    );
}

#[test]
fn cross_device_project_load_keeps_destination_routing_but_disarms_output() {
    let source = Engine::new(Project::demo()).unwrap();
    let mut destination = Project::demo();
    destination.output.target = "127.0.0.1:16454".into();
    destination.output.enabled = true;
    let routing = destination.output.clone();
    let mut target = Engine::new(destination).unwrap();
    let item = Item::new(
        Kind::Project,
        "cross-device".into(),
        "Cross device".into(),
        Bundle::from_session(&source.checkpoint()),
        None,
    );
    let prepared = Engine::prepare_library(target.checkpoint(), item).unwrap();
    target.install_bundle(prepared).unwrap();
    let mut expected = routing;
    expected.enabled = false;
    assert_eq!(target.project().output, expected);
}
