use schema_rs::{
    bundle::Bundle,
    engine::{Command, Engine},
    model::{Fixture, Project},
};
use serde_json::{json, Value};
fn marked(fixture: &Fixture, color: Value) -> Fixture {
    let mut value = serde_json::to_value(fixture).unwrap();
    value["marker_color"] = color;
    serde_json::from_value(value).unwrap()
}
#[test]
fn marking_is_metadata_only_undoable_and_survives_session_bundle_and_layout() {
    let mut engine = Engine::new(Project::demo()).unwrap();
    engine.tick(0.2);
    let before = serde_json::to_value(engine.checkpoint()).unwrap();
    let fixture = marked(&engine.project().fixtures[0], json!("#A78BFA"));
    engine.apply(Command::SetFixture { fixture }).unwrap();
    let after = serde_json::to_value(engine.checkpoint()).unwrap();
    assert_eq!(after["project"]["fixtures"][0]["marker_color"], "#A78BFA");
    for key in ["inputs", "states"] {
        assert_eq!(before[key], after[key], "{key}");
    }
    assert_eq!(
        before["snapshot"]["universes"],
        after["snapshot"]["universes"]
    );
    assert_eq!(
        before["snapshot"]["fixtures"][0]["result"],
        after["snapshot"]["fixtures"][0]["result"]
    );
    engine.apply(Command::Undo).unwrap();
    assert_eq!(
        serde_json::to_value(engine.checkpoint()).unwrap()["states"],
        before["states"],
        "Marker undo must not restart running nodes"
    );
    assert!(
        serde_json::to_value(engine.project()).unwrap()["fixtures"][0]
            .get("marker_color")
            .is_none()
    );
    engine.apply(Command::Redo).unwrap();
    assert_eq!(
        serde_json::to_value(engine.checkpoint()).unwrap()["states"],
        before["states"],
        "Marker redo must not restart running nodes"
    );
    engine.apply(Command::HistoryJump { offset: -1 }).unwrap();
    assert_eq!(
        serde_json::to_value(engine.checkpoint()).unwrap()["states"],
        before["states"],
        "Marker history navigation must not restart running nodes"
    );
    engine.apply(Command::Redo).unwrap();
    let session: schema_rs::session::Session =
        serde_json::from_slice(&serde_json::to_vec(&engine.checkpoint()).unwrap()).unwrap();
    let resumed = Engine::from_session(session).unwrap();
    assert_eq!(
        serde_json::to_value(resumed.project()).unwrap()["fixtures"][0]["marker_color"],
        "#A78BFA"
    );
    let bundle: Bundle = serde_json::from_slice(
        &serde_json::to_vec(&Bundle::from_session(&resumed.checkpoint())).unwrap(),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(bundle.project).unwrap()["fixtures"][0]["marker_color"],
        "#A78BFA"
    );
    let layout: schema_rs::storage_formats::Layout=serde_json::from_value(json!({"version":schema_rs::migrations::LAYOUT_VERSION,"name":"Marked","fixtures":resumed.project().fixtures})).unwrap();
    assert_eq!(
        serde_json::to_value(layout).unwrap()["fixtures"][0]["marker_color"],
        "#A78BFA"
    );
}
#[test]
fn invalid_marker_is_rejected_atomically_and_old_fixtures_default_unmarked() {
    let mut engine = Engine::new(Project::demo()).unwrap();
    let before = engine.project().clone();
    let fixture: Fixture =
        serde_json::from_value(serde_json::to_value(&before.fixtures[0]).unwrap()).unwrap();
    assert!(serde_json::to_value(&fixture)
        .unwrap()
        .get("marker_color")
        .is_none());
    for color in ["purple", "#abc", "#12345678", "url(evil)", "#zzzzzz"] {
        let fixture = marked(&fixture, json!(color));
        assert!(engine.apply(Command::SetFixture { fixture }).is_err());
        assert_eq!(engine.project(), &before);
        assert!(!engine.current_snapshot().can_undo);
    }
}

#[test]
fn bundled_carpushit_white_emission_keeps_the_dimmer_patch_and_level() {
    let project: Project =
        serde_json::from_str(include_str!("../web/scene-assets/carpushit.project.json")).unwrap();
    assert!(project.fixtures.iter().all(|f| f.marker_color.is_some()));
    fn tint(node: &mut schema_rs::model::Node) {
        if node.behavior == "Create.Color.Color" {
            node.properties.insert("Color".into(), json!("#862bff"));
        }
        for child in node.children.iter_mut().chain(node.drivers.values_mut()) {
            tint(child);
        }
    }
    let mut colored = project.clone();
    for layers in colored.content.values_mut() {
        for stack in layers.iter_mut().flatten() {
            tint(&mut stack.root);
        }
    }
    let mut before = Engine::new(colored).unwrap();
    let mut after = Engine::new(project).unwrap();
    before.tick(0.1);
    after.tick(0.1);
    assert_eq!(
        before.current_snapshot().universes,
        after.current_snapshot().universes,
        "Hardware dimmer levels must not depend on cosmetic emission hue"
    );
    for fixture in after.current_snapshot().fixtures {
        for color in fixture.result.colors.unwrap_or_default() {
            assert_eq!(color[0], color[1]);
            assert_eq!(color[1], color[2]);
        }
    }
}
