use schema_rs::{bundle::Bundle, engine::Engine, model::Project};
#[test]
fn portable_import_keeps_resources_components_and_undo_while_disarming_output() {
    let mut source = Engine::new(Project::demo()).unwrap();
    let mut bundle = Bundle::from_session(&source.checkpoint());
    bundle.project.name = "Portable scene".into();
    bundle.project.output.enabled = true;
    bundle
        .resources
        .insert("textures/example.bin".into(), vec![1, 2, 3]);
    let mut target = Engine::new(Project::demo()).unwrap();
    let original = target.project().clone();
    let prepared = Engine::prepare_bundle(target.checkpoint(), bundle).unwrap();
    target.install_bundle(prepared).unwrap();
    assert_eq!(target.project().name, "Portable scene");
    assert!(!target.project().output.enabled);
    assert_eq!(
        target.checkpoint().resources["textures/example.bin"],
        vec![1, 2, 3]
    );
    target.apply(schema_rs::engine::Command::Undo).unwrap();
    assert_eq!(target.project(), &original);
    let resumed = Engine::from_session(target.checkpoint()).unwrap();
    assert_eq!(
        Bundle::from_session(&resumed.checkpoint()).resources["textures/example.bin"],
        vec![1, 2, 3]
    );
    source.tick(0.);
}
#[test]
fn invalid_paths_and_stale_imports_are_rejected_atomically() {
    let mut target = Engine::new(Project::demo()).unwrap();
    let original = target.project().clone();
    let mut bundle = Bundle::from_session(&target.checkpoint());
    bundle.resources.insert("../secret".into(), vec![1]);
    assert!(Engine::prepare_bundle(target.checkpoint(), bundle).is_err());
    assert_eq!(target.project(), &original);
    let bundle = Bundle::from_session(&target.checkpoint());
    let prepared = Engine::prepare_bundle(target.checkpoint(), bundle).unwrap();
    target
        .apply(schema_rs::engine::Command::MoveFixture {
            fixture: "fixture-1".into(),
            position: [1., 2., 3.],
        })
        .unwrap();
    assert!(target.install_bundle(prepared).is_err());
    assert_eq!(target.project().fixtures[0].position, [1., 2., 3.]);
}

#[test]
fn graph_plugins_travel_and_conflicting_packages_do_not_modify_the_engine() {
    let manifest =
        serde_json::from_str(include_str!("../examples/plugins/gain/plugin.json")).unwrap();
    let package = schema_rs::plugins::Package {
        folder: "gain".into(),
        enabled: true,
        manifest,
    };
    let mut e = Engine::new(Project::demo()).unwrap();
    let mut bundle = Bundle::from_session(&e.checkpoint());
    bundle
        .plugins
        .packages
        .insert("demo".into(), package.clone());
    let prepared = Engine::prepare_bundle(e.checkpoint(), bundle).unwrap();
    e.install_bundle(prepared).unwrap();
    assert!(e.plugins().component("Plugin.demo.Gain").is_some());
    let encoded = serde_json::to_vec(&e.checkpoint()).unwrap();
    let restored = Engine::from_session(schema_rs::session::decode(&encoded).unwrap()).unwrap();
    assert!(restored.plugins().component("Plugin.demo.Gain").is_some());
    let before = e.project().clone();
    let mut conflict = Bundle::from_session(&e.checkpoint());
    conflict
        .plugins
        .packages
        .get_mut("demo")
        .unwrap()
        .manifest
        .version = "2.0.0".into();
    assert!(Engine::prepare_bundle(e.checkpoint(), conflict).is_err());
    assert_eq!(e.project(), &before);
}
