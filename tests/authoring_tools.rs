use schema_rs::{
    engine::{Command, Engine},
    model::Project,
};
use serde_json::{json, Value};
fn apply(e: &mut Engine, v: Value) {
    e.apply(serde_json::from_value::<Command>(v).unwrap())
        .unwrap();
}
fn scene() -> Project {
    let mut p = Project::demo();
    p.content.insert("fixture-1".into(),vec![Some(schema_rs::scenic::parse("name: Test\nstack:\n  Modify.Value.Invert:\n    in:\n      - Create.Value.Constant: {Value: 0.25}\n").unwrap()),None,None]);
    p
}
#[test]
fn bypass_is_persistent_undoable_passthrough_and_scenicscript_roundtrips() {
    let mut e = Engine::new(scene()).unwrap();
    e.tick(0.);
    assert_eq!(e.snapshot().fixtures[0].result.value, Some(0.75));
    apply(
        &mut e,
        json!({"type":"set_node_bypassed","fixture":"fixture-1","layer":0,"node":1,"bypassed":true}),
    );
    e.tick(0.);
    assert_eq!(e.snapshot().fixtures[0].result.value, Some(0.25));
    let stack = e.project().content["fixture-1"][0].as_ref().unwrap();
    let yaml = schema_rs::scenic::encode(stack).unwrap();
    assert!(yaml.contains("bypassed"));
    assert_eq!(schema_rs::scenic::parse(&yaml).unwrap(), *stack);
    let restored = Engine::from_session(e.checkpoint()).unwrap();
    assert_eq!(restored.project(), e.project());
    e.apply(Command::Undo).unwrap();
    e.tick(0.);
    assert_eq!(e.snapshot().fixtures[0].result.value, Some(0.75));
}
#[test]
fn debug_focus_uses_existing_trace_without_editing_project_and_clears_on_restore() {
    let mut e = Engine::new(scene()).unwrap();
    let before = e.project().clone();
    apply(
        &mut e,
        json!({"type":"set_debug_focus","target":{"fixture":"fixture-1","layer":0,"node":2}}),
    );
    e.tick(0.);
    assert_eq!(e.snapshot().fixtures[0].result.value, Some(0.25));
    assert_eq!(e.project(), &before);
    assert!(!e.current_snapshot().can_undo);
    assert!(serde_json::to_value(e.current_snapshot()).unwrap()["debug_focus"].is_object());
    let mut restored = Engine::from_session(e.checkpoint()).unwrap();
    restored.tick(0.);
    assert_eq!(restored.snapshot().fixtures[0].result.value, Some(0.75));
    apply(&mut e, json!({"type":"set_debug_focus","target":null}));
    e.tick(0.);
    assert_eq!(e.snapshot().fixtures[0].result.value, Some(0.75));
}
#[test]
fn history_jump_is_atomic_and_checkpoints_survive_session_serialization() {
    let mut e = Engine::new(scene()).unwrap();
    apply(
        &mut e,
        json!({"type":"save_checkpoint","name":"Before changes"}),
    );
    for x in [1., 2., 3.] {
        apply(
            &mut e,
            json!({"type":"move_fixture","fixture":"fixture-1","position":[x,0.,0.]}),
        );
    }
    let revision = e.revision();
    apply(&mut e, json!({"type":"history_jump","offset":-2}));
    assert_eq!(e.revision(), revision + 1);
    assert_eq!(e.project().fixtures[0].position, [1., 0., 0.]);
    apply(&mut e, json!({"type":"history_jump","offset":2}));
    assert_eq!(e.project().fixtures[0].position, [3., 0., 0.]);
    apply(&mut e, json!({"type":"history_jump","offset":-2}));
    assert_eq!(e.project().fixtures[0].position, [1., 0., 0.]);
    let project = e.project().clone();
    let invalid = serde_json::from_value(json!({"type":"history_jump","offset":-99})).unwrap();
    assert!(e.apply(invalid).is_err());
    assert_eq!(e.project(), &project);
    let mut restored = Engine::from_session(e.checkpoint()).unwrap();
    apply(
        &mut restored,
        json!({"type":"restore_checkpoint","name":"Before changes"}),
    );
    assert_eq!(
        restored.project().fixtures[0].position,
        scene().fixtures[0].position
    );
    restored.apply(Command::Undo).unwrap();
    assert_eq!(restored.project(), &project);
}
