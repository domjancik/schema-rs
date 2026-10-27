use schema_rs::{
    engine::{Command, Engine},
    model::Project,
    plugins::{Registry, State},
    scenic,
};
use serde_json::json;
use std::sync::Arc;
fn package(min: f64) -> String {
    json!({"api_version":1,"id":"demo","version":"1.0.0","components":[{"name":"Gain","description":"Scale the incoming value using native map nodes.","defaults":{"Gain":2.0},"graph":{"Modify.Value.MapTo":{"Minimum":min,"Maximum":"$Gain","in":[{"Plugin.Input":null}]}}}]}).to_string()
}
fn setup(registry: Arc<Registry>) -> Engine {
    let mut p = Project::demo();
    for layers in p.content.values_mut() {
        *layers = vec![None, None, None];
    }
    p.content.get_mut("fixture-1").unwrap()[0]=Some(scenic::with_plugins(registry.clone(),||scenic::parse("name: Plugin test\nstack:\n  Plugin.demo.Gain:\n    Gain: 3\n    in:\n      - Create.Value.Constant: {Value: 0.4}\n")).unwrap());
    Engine::new_with_plugins(p, registry).unwrap()
}
#[test]
fn plugin_graph_binds_parameters_and_inputs_without_global_catalog_leaks() {
    let registry = Arc::new(Registry::default().load_json("demo", &package(0.)).unwrap());
    let mut e = setup(registry);
    e.tick(0.);
    assert!((e.snapshot().fixtures[0].result.value.unwrap() - 1.2).abs() < 1e-9);
    assert!(scenic::defaults("Plugin.demo.Gain").is_err());
}
#[test]
fn hot_reload_is_atomic_retains_authored_history_and_unload_is_explicit() {
    let reg = Arc::new(Registry::default().load_json("demo", &package(0.)).unwrap());
    let mut e = setup(reg.clone());
    e.apply(Command::SetDimmer {
        fixture: "fixture-1".into(),
        value: 0.5,
    })
    .unwrap();
    let revision = e.revision();
    let updated = Arc::new(reg.load_json("demo", &package(1.)).unwrap());
    let prepared = Engine::prepare_plugin_update(e.checkpoint(), updated.clone()).unwrap();
    e.install_plugin_update(prepared).unwrap();
    e.tick(0.);
    assert!((e.snapshot().fixtures[0].result.value.unwrap() - 1.8).abs() < 1e-9);
    assert_eq!(e.revision(), revision);
    e.apply(Command::Undo).unwrap();
    e.tick(0.);
    assert_eq!(e.project().fixtures[0].dimmer, 1.);
    let unloaded = Arc::new(updated.set_enabled("demo", false).unwrap());
    e.install_plugin_update(Engine::prepare_plugin_update(e.checkpoint(), unloaded).unwrap())
        .unwrap();
    e.tick(0.);
    assert!(e.snapshot().fixtures[0].result.value.is_none());
    assert!(e
        .snapshot()
        .diagnostics
        .iter()
        .any(|d| d.contains("unloaded")));
}
#[test]
fn incompatible_reload_keeps_last_working_code_and_session_restores_plugin_runtime() {
    let reg = Arc::new(Registry::default().load_json("demo", &package(0.)).unwrap());
    let mut e = setup(reg.clone());
    e.tick(0.1);
    let mut bad: serde_json::Value = serde_json::from_str(&package(1.)).unwrap();
    bad["components"][0]["defaults"] = json!({"Other":1});
    assert!(reg.load_json("demo", &bad.to_string()).is_err());
    let checkpoint = e.checkpoint();
    checkpoint.validate().unwrap();
    let text = serde_json::to_string(&checkpoint).unwrap();
    let restored = serde_json::from_str(&text).unwrap();
    let mut resumed = Engine::from_session(restored).unwrap();
    e.tick(0.1);
    resumed.tick(0.1);
    assert_eq!(
        e.snapshot().fixtures[0].result,
        resumed.snapshot().fixtures[0].result
    );
}
#[test]
fn plugin_cycles_unsupported_api_and_builtin_collision_are_rejected() {
    assert!(Registry::default()
        .load_json(
            "demo",
            r#"{"api_version":2,"id":"demo","version":"1","components":[]}"#
        )
        .is_err());
    let mut value: serde_json::Value = serde_json::from_str(&package(0.)).unwrap();
    value["components"][0]["graph"] = json!({"Plugin.demo.Gain":null});
    assert!(Registry::default()
        .load_json("demo", &value.to_string())
        .is_err());
    let state = State::default();
    assert_eq!(state.generation, 0);
}
#[test]
fn scripted_components_receive_parameters_and_stateful_native_components_resume() {
    let mut value: serde_json::Value = serde_json::from_str(&package(0.)).unwrap();
    value["components"][0]["graph"] =
        json!({"Script.Value":{"Code":"input * params[\"Gain\"]","in":[{"Plugin.Input":null}]}});
    let reg = Arc::new(
        Registry::default()
            .load_json("demo", &value.to_string())
            .unwrap(),
    );
    let mut e = setup(reg);
    e.tick(0.);
    assert!((e.snapshot().fixtures[0].result.value.unwrap() - 1.2).abs() < 1e-9);
    value["components"][0]["graph"] =
        json!({"Modify.Value.AttackDecay":{"Attack":1.0,"Decay":1.0,"in":[{"Plugin.Input":null}]}});
    let reg = Arc::new(
        Registry::default()
            .load_json("demo", &value.to_string())
            .unwrap(),
    );
    let mut e = setup(reg);
    e.tick(0.1);
    e.tick(0.1);
    let session = e.checkpoint();
    session.validate().unwrap();
    let mut resumed = Engine::from_session(
        serde_json::from_str(&serde_json::to_string(&session).unwrap()).unwrap(),
    )
    .unwrap();
    e.tick(0.1);
    resumed.tick(0.1);
    assert_eq!(
        e.snapshot().fixtures[0].result,
        resumed.snapshot().fixtures[0].result
    );
    assert!(e.snapshot().fixtures[0].result.value.unwrap() > 0.);
}
#[test]
fn history_schema_and_plugin_generation_are_checked_before_commit() {
    let reg = Arc::new(Registry::default().load_json("demo", &package(0.)).unwrap());
    let mut e = setup(reg.clone());
    e.apply(Command::SetProperty {
        fixture: "fixture-1".into(),
        layer: 0,
        node: 1,
        key: "Gain".into(),
        value: json!(4.0),
    })
    .unwrap();
    let mut changed: serde_json::Value = serde_json::from_str(&package(0.)).unwrap();
    changed["components"][0]["defaults"] = json!({"Other":2.0});
    changed["components"][0]["graph"] = json!({"Create.Value.Constant":{"Value":1.0}});
    let bad = Arc::new(reg.load_json("demo", &changed.to_string()).unwrap());
    assert!(Engine::prepare_plugin_update(e.checkpoint(), bad).is_err());
    let prepared = scenic::with_plugins(reg.clone(), || {
        Engine::prepare(
            e.project().clone(),
            Command::SetDimmer {
                fixture: "fixture-1".into(),
                value: 0.7,
            },
        )
    })
    .unwrap();
    let good = Arc::new(reg.load_json("demo", &package(1.)).unwrap());
    e.install_plugin_update(Engine::prepare_plugin_update(e.checkpoint(), good).unwrap())
        .unwrap();
    assert!(e.install(prepared).is_err());
    assert_eq!(e.project().fixtures[0].dimmer, 1.0);
}

#[test]
fn partial_vector_parameter_bindings_are_rejected_instead_of_frozen() {
    let value = json!({"api_version":1,"id":"demo","version":"1","components":[{"name":"Position","description":"Parameterized position","defaults":{"X":0.0},"graph":{"Create.Position.Constant":{"Position":["$X",0.0,0.0]}}}]});
    assert!(Registry::default()
        .load_json("demo", &value.to_string())
        .is_err());
}

#[test]
fn plugin_resources_include_bound_paths_in_nested_components() {
    let manifest = json!({"api_version":1,"id":"demo","version":"1","components":[{"name":"File","description":"Check a project file","defaults":{"Path":"example.wav"},"graph":{"Create.Value.System.FileExists":{"File":"$Path"}}}]});
    let registry = Arc::new(
        Registry::default()
            .load_json("demo", &manifest.to_string())
            .unwrap(),
    );
    let mut project = Project::demo();
    project.content.get_mut("fixture-1").unwrap()[0] = Some(
        scenic::with_plugins(registry.clone(), || {
            scenic::parse("name: File check\nstack:\n  Plugin.demo.File: {Path: custom.wav}\n")
        })
        .unwrap(),
    );
    assert!(registry.resource_names(&project).contains("custom.wav"));
}

#[test]
fn loading_an_unrelated_package_preserves_existing_component_state() {
    let mut value: serde_json::Value = serde_json::from_str(&package(0.)).unwrap();
    value["components"][0]["graph"] =
        json!({"Modify.Value.AttackDecay":{"Attack":1.0,"Decay":1.0,"in":[{"Plugin.Input":null}]}});
    let reg = Arc::new(
        Registry::default()
            .load_json("demo", &value.to_string())
            .unwrap(),
    );
    let mut e = setup(reg.clone());
    e.tick(0.1);
    e.tick(0.1);
    let mut expected = Engine::from_session(e.checkpoint()).unwrap();
    let extra = package(0.).replace("\"demo\"", "\"extra\"");
    let next = Arc::new(reg.load_json("extra", &extra).unwrap());
    e.install_plugin_update(Engine::prepare_plugin_update(e.checkpoint(), next).unwrap())
        .unwrap();
    e.tick(0.1);
    expected.tick(0.1);
    assert_eq!(
        e.snapshot().fixtures[0].result,
        expected.snapshot().fixtures[0].result
    );
}
#[test]
fn invalid_script_and_history_only_schema_changes_do_not_install() {
    let reg = Arc::new(Registry::default().load_json("demo", &package(0.)).unwrap());
    let mut e = setup(reg.clone());
    e.apply(Command::ReplaceLayer {
        fixture: "fixture-1".into(),
        layer: 0,
        stack: None,
    })
    .unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&package(0.)).unwrap();
    value["components"][0]["defaults"] = json!({"Other":2.0});
    value["components"][0]["graph"] = json!({"Create.Value.Constant":{"Value":1.0}});
    let next = Arc::new(reg.load_json("demo", &value.to_string()).unwrap());
    assert!(Engine::prepare_plugin_update(e.checkpoint(), next).is_err());
    value["components"][0]["defaults"] = json!({"Gain":2.0});
    value["components"][0]["graph"] = json!({"Script.Value":{"Code":"let broken = ;"}});
    let next = Arc::new(reg.load_json("demo", &value.to_string()).unwrap());
    assert!(Engine::prepare_plugin_update(e.checkpoint(), next).is_err());
    assert_eq!(e.plugins().state.generation, reg.state.generation);
    e.apply(Command::Undo).unwrap();
    e.tick(0.);
    assert!((e.snapshot().fixtures[0].result.value.unwrap() - 1.2).abs() < 1e-9);
}

#[test]
fn root_component_references_cannot_bypass_expanded_node_limit() {
    let constant = || json!({"Create.Value.Constant":{"Value":1.0}});
    let leaf = json!({"Tool.Merge":{"in":(0..2047).map(|_|constant()).collect::<Vec<_>>()}});
    let mut inputs = vec![json!({"Plugin.demo.Leaf":null})];
    inputs.extend((0..2046).map(|_| constant()));
    let graph = json!({"Tool.Merge":{"in":inputs}});
    let manifest = json!({"api_version":1,"id":"demo","version":"1","components":[{"name":"Leaf","description":"Large leaf","graph":leaf},{"name":"Parent","description":"Exactly budget sized","graph":graph},{"name":"Wrapper","description":"One node too many","graph":{"Plugin.demo.Parent":null}}]});
    assert!(Registry::default()
        .load_json("demo", &manifest.to_string())
        .is_err());
}
