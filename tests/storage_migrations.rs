use schema_rs::{
    engine::{Command, Engine},
    migrations::{self, Kind},
    model::Project,
};
use serde_json::{json, Value};
const SESSION: &[u8] = include_bytes!("fixtures/storage-v1/session-late.json");
#[test]
fn old_project_loads_before_typed_decoding_and_migration_is_idempotent() {
    let original: Value =
        serde_json::from_str(include_str!("fixtures/storage-v1/project-early.json")).unwrap();
    let migrated = migrations::migrate(Kind::Project, original.clone()).unwrap();
    assert_eq!(migrated.document["version"], 2);
    assert!(migrated.report.changed());
    assert_eq!(original["version"], 1);
    let again = migrations::migrate(Kind::Project, migrated.document.clone()).unwrap();
    assert_eq!(again.document, migrated.document);
    assert!(!again.report.changed());
    let project: Project = serde_json::from_value(original).unwrap();
    assert_eq!(project.version, 2);
    project.validate().unwrap();
    assert_eq!(project.fixtures[0].pixel_positions, vec![[0.; 3]]);
    assert_eq!(project.director.len(), 3);
}
#[test]
fn historical_session_preserves_runtime_providers_plugins_and_every_history_entry() {
    let original: Value = serde_json::from_slice(SESSION).unwrap();
    let session = schema_rs::session::decode(SESSION).unwrap();
    let migrated = serde_json::to_value(&session).unwrap();
    assert_eq!(session.version, 2);
    for key in ["states", "inputs", "plugins", "snapshot", "resources"] {
        assert_eq!(
            migrated[key], original[key],
            "{key} changed during migration"
        );
    }
    assert_eq!(
        session.undo.len(),
        original["undo"].as_array().unwrap().len()
    );
    assert_eq!(
        session.redo.len(),
        original["redo"].as_array().unwrap().len()
    );
    for project in session
        .undo
        .iter()
        .chain(&session.redo)
        .chain(session.checkpoints.values())
        .chain(std::iter::once(&session.project))
    {
        assert_eq!(project.version, 2);
    }
    let mut engine = Engine::from_session(session).unwrap();
    let current = engine.project().clone();
    engine.apply(Command::Redo).unwrap();
    assert_eq!(engine.project().fixtures[0].dimmer, 0.35);
    engine.apply(Command::Undo).unwrap();
    assert_eq!(engine.project(), &current);
    engine
        .apply(
            serde_json::from_value(
                json!({"type":"restore_checkpoint","name":"Before layout edit"}),
            )
            .unwrap(),
        )
        .unwrap();
    assert_ne!(
        engine.project().fixtures[0].position,
        current.fixtures[0].position
    );
}
#[test]
fn old_bundle_migrates_without_losing_component_or_resource_data() {
    let original: Value =
        serde_json::from_str(include_str!("fixtures/storage-v1/bundle.json")).unwrap();
    let bundle: schema_rs::bundle::Bundle = serde_json::from_value(original.clone()).unwrap();
    assert_eq!(bundle.version, 2);
    assert_eq!(bundle.project.version, 2);
    let encoded = serde_json::to_value(bundle).unwrap();
    for key in ["assets", "plugins", "resources"] {
        assert_eq!(encoded[key], original[key]);
    }
}
#[test]
fn future_nested_versions_malformed_versions_and_unknown_fields_are_never_dropped() {
    for version in [json!(0), json!(99), json!("1"), Value::Null] {
        let mut project: Value =
            serde_json::from_str(include_str!("fixtures/storage-v1/project-early.json")).unwrap();
        project["version"] = version;
        assert!(serde_json::from_value::<Project>(project).is_err());
    }
    let mut session: Value = serde_json::from_slice(SESSION).unwrap();
    session["undo"][0]["version"] = json!(99);
    let error = schema_rs::session::decode(&serde_json::to_vec(&session).unwrap())
        .err()
        .unwrap();
    assert!(error.contains("undo[0]"), "{error}");
    let mut project: Value =
        serde_json::from_str(include_str!("fixtures/storage-v1/project-early.json")).unwrap();
    project["unknown_authored_data"] = json!({"keep":true});
    assert!(serde_json::from_value::<Project>(project).is_err());
}

#[test]
fn native_load_is_read_only_and_first_successful_save_keeps_original_bytes() {
    let root =
        std::env::temp_dir().join(format!("schema-migration-project-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    schema_rs::project::initialize(&root).unwrap();
    let p: Value =
        serde_json::from_str(include_str!("fixtures/storage-v1/project-early.json")).unwrap();
    let layout = json!({"version":1,"name":p["name"],"fixtures":p["fixtures"]});
    let entries = p["content"]["fixture-1"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(layer, stack)| json!({"fixture":"fixture-1","layer":layer,"stack":stack}))
        .collect::<Vec<_>>();
    let files = [
        (
            "registry/layouts/init.json",
            serde_json::to_vec_pretty(&layout).unwrap(),
        ),
        (
            "registry/content/init.json",
            serde_json::to_vec_pretty(&entries).unwrap(),
        ),
        (
            "registry/config/output.json",
            serde_json::to_vec_pretty(&p["output"]).unwrap(),
        ),
    ];
    for (path, bytes) in &files {
        std::fs::write(root.join(path), bytes).unwrap();
    }
    let loaded = schema_rs::project::load(&root, "init").unwrap();
    assert_eq!(loaded.version, 2);
    for (path, bytes) in &files {
        assert_eq!(std::fs::read(root.join(path)).unwrap(), *bytes);
    }
    schema_rs::project::save(&root, &loaded, "init").unwrap();
    for (path, bytes) in &files {
        let current: Value =
            serde_json::from_slice(&std::fs::read(root.join(path)).unwrap()).unwrap();
        assert_eq!(current["version"], 2);
        let parent = root.join(path).parent().unwrap().join(".migration-backups");
        assert!(std::fs::read_dir(parent)
            .unwrap()
            .any(|entry| std::fs::read(entry.unwrap().path()).unwrap() == *bytes));
    }
    let mut future: Value =
        serde_json::from_slice(&std::fs::read(root.join(files[1].0)).unwrap()).unwrap();
    future["version"] = json!(99);
    std::fs::write(root.join(files[1].0), serde_json::to_vec(&future).unwrap()).unwrap();
    let previous = std::fs::read(root.join(files[0].0)).unwrap();
    assert!(schema_rs::project::save(&root, &loaded, "init").is_err());
    assert_eq!(std::fs::read(root.join(files[0].0)).unwrap(), previous);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn session_upgrade_backup_is_exact_and_failed_save_never_overwrites_future_data() {
    let root =
        std::env::temp_dir().join(format!("schema-migration-session-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("session.json");
    std::fs::write(&path, SESSION).unwrap();
    let session = schema_rs::session::load(&path).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), SESSION);
    let store = schema_rs::session::Store::new(path.clone());
    store.save(&session).unwrap();
    assert!(std::fs::read_dir(root.join(".migration-backups"))
        .unwrap()
        .any(|entry| std::fs::read(entry.unwrap().path()).unwrap() == SESSION));
    let mut future: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    future["version"] = json!(99);
    let future = serde_json::to_vec(&future).unwrap();
    std::fs::write(&path, &future).unwrap();
    assert!(store.save(&session).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), future);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn duplicate_keys_and_unknown_saved_runtime_fields_are_rejected_without_silent_loss() {
    let text = std::str::from_utf8(SESSION).unwrap().replacen(
        "\"version\": 1",
        "\"version\": 99, \"version\": 1",
        1,
    );
    assert!(
        schema_rs::session::decode(text.as_bytes()).is_err(),
        "duplicate versions must not hide future data"
    );
    for field in ["inputs", "snapshot"] {
        let mut value: Value = serde_json::from_slice(SESSION).unwrap();
        value[field]["unknown_future_state"] = json!({"preserve":true});
        assert!(
            schema_rs::session::decode(&serde_json::to_vec(&value).unwrap()).is_err(),
            "unknown {field} state must not disappear"
        );
    }
}

#[test]
fn immutable_named_projects_keep_the_old_manifest_and_generation() {
    let root =
        std::env::temp_dir().join(format!("schema-migration-library-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let generation = root.join("projects/legacy/revisions/old-generation");
    schema_rs::project::initialize(&generation).unwrap();
    let project: Value =
        serde_json::from_str(include_str!("fixtures/storage-v1/project-early.json")).unwrap();
    let layout = json!({"version":1,"name":project["name"],"fixtures":project["fixtures"]});
    let entries = project["content"]["fixture-1"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(layer, stack)| json!({"fixture":"fixture-1","layer":layer,"stack":stack}))
        .collect::<Vec<_>>();
    for (path, value) in [
        ("registry/layouts/init.json", layout),
        ("registry/content/init.json", json!(entries)),
        ("registry/config/output.json", project["output"].clone()),
    ] {
        std::fs::write(generation.join(path), serde_json::to_vec(&value).unwrap()).unwrap();
    }
    let pointer = root.join("projects/legacy/project.json");
    let manifest=serde_json::to_vec(&json!({"version":1,"generation":"old-generation","entry":{"id":"legacy","title":"Legacy saved project","fixtures":1,"saved_revision":7,"modified_unix_ms":123,"legacy":false}})).unwrap();
    std::fs::write(&pointer, &manifest).unwrap();
    let original = std::fs::read(generation.join("registry/layouts/init.json")).unwrap();
    let loaded = schema_rs::project_library::load(&root, "legacy").unwrap();
    assert_eq!(std::fs::read(&pointer).unwrap(), manifest);
    schema_rs::project_library::store(&root, &loaded, "legacy", "Migrated project", true, 7)
        .unwrap();
    assert_eq!(
        std::fs::read(generation.join("registry/layouts/init.json")).unwrap(),
        original
    );
    assert!(
        std::fs::read_dir(root.join("projects/legacy/.migration-backups"))
            .unwrap()
            .any(|entry| std::fs::read(entry.unwrap().path()).unwrap() == manifest)
    );
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn migration_cli_is_a_dry_run_by_default_and_never_overwrites_input_or_existing_output() {
    let root = std::env::temp_dir().join(format!("schema-migration-cli-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let input = root.join("old.json");
    let output = root.join("new.json");
    std::fs::write(&input, SESSION).unwrap();
    let binary = env!("CARGO_BIN_EXE_schema-rs");
    let result = std::process::Command::new(binary)
        .args(["migrate", "--format", "session", "--input"])
        .arg(&input)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!output.exists());
    assert_eq!(std::fs::read(&input).unwrap(), SESSION);
    let result = std::process::Command::new(binary)
        .args(["migrate", "--format", "session", "--input"])
        .arg(&input)
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(result.status.success());
    let migrated: Value = serde_json::from_slice(&std::fs::read(&output).unwrap()).unwrap();
    assert_eq!(migrated["version"], 2);
    assert!(!migrated["undo"].as_array().unwrap().is_empty());
    let previous = std::fs::read(&output).unwrap();
    let refused = std::process::Command::new(binary)
        .args(["migrate", "--format", "session", "--input"])
        .arg(&input)
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!refused.status.success());
    assert_eq!(std::fs::read(&output).unwrap(), previous);
    assert_eq!(std::fs::read(&input).unwrap(), SESSION);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn text_project_import_migrates_atomically_and_preserves_undo() {
    let mut project: Value =
        serde_json::from_str(include_str!("fixtures/storage-v1/project-early.json")).unwrap();
    project["content"]["fixture-1"][0]["root"]["behavior"] = json!("Create.Value.Random");
    project["content"]["fixture-1"][0]["root"]["properties"] = json!({"Seed":42_u64});
    let mut engine = Engine::new(Project::demo()).unwrap();
    let original = engine.project().clone();
    let command = serde_json::from_value(
        json!({"type":"import_project","json":serde_json::to_string(&project).unwrap()}),
    )
    .unwrap();
    engine.apply(command).unwrap();
    assert_eq!(
        engine.project().content["fixture-1"][0]
            .as_ref()
            .unwrap()
            .root
            .properties["Seed"]
            .as_u64(),
        Some(42)
    );
    assert!(!engine.project().output.enabled);
    engine.apply(Command::Undo).unwrap();
    assert_eq!(engine.project(), &original);
}

#[tokio::test]
async fn migration_and_raw_import_apis_use_the_same_versioned_decoder() {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use schema_rs::server::{router, AppState, Runtime};
    use tower::ServiceExt;
    let runtime = Runtime::start(Project::demo()).unwrap();
    let app = router(AppState::new(
        runtime.controller.clone(),
        std::env::temp_dir(),
        None,
    ));
    let source = include_str!("fixtures/storage-v1/project-early.json");
    for (path, body) in [
        (
            "/api/storage/migrate",
            json!({"format":"project","text":source}),
        ),
        (
            "/api/projects/import",
            json!({"json":source,"expected_revision":0}),
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(path)
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        let value: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        if path.contains("migrate") {
            assert_eq!(value["document"]["version"], 2);
            assert_eq!(value["report"]["steps"][0]["from"], 1);
        }
    }
    for (path, kind) in [
        ("/api/project/raw", Kind::Project),
        ("/api/projects/bundle/raw", Kind::Bundle),
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()["content-type"],
            "application/octet-stream"
        );
        let value =
            migrations::parse(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap()).unwrap();
        assert!(!migrations::validated(kind, value).unwrap().report.changed());
    }
    runtime
        .controller
        .command(Command::Undo, None)
        .await
        .unwrap();
    assert_eq!(runtime.controller.current().project, Project::demo());
}
