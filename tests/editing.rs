use schema_rs::{
    engine::{Command, Engine},
    model::Project,
};
#[test]
fn branch_moves_reject_cycles_and_are_undoable() {
    let mut e = Engine::new(Project::demo()).unwrap();
    let stack = e.project().content["fixture-1"][0].as_ref().unwrap();
    let source = stack.root.children[0].id;
    let target = stack.root.id;
    let before = e.project().clone();
    assert!(e
        .apply(Command::MoveNode {
            fixture: "fixture-1".into(),
            layer: 0,
            node: target,
            target: source,
            branch: true,
            below: false
        })
        .is_err());
    assert_eq!(e.project(), &before);
    e.apply(Command::MoveNode {
        fixture: "fixture-1".into(),
        layer: 0,
        node: source,
        target,
        branch: true,
        below: false,
    })
    .unwrap();
}
#[test]
fn invalid_batch_does_not_partially_move_fixtures() {
    let mut e = Engine::new(Project::demo()).unwrap();
    let before = e.project().clone();
    assert!(e
        .apply(Command::Batch {
            commands: vec![
                Command::MoveFixture {
                    fixture: "fixture-1".into(),
                    position: [4., 0., 0.]
                },
                Command::SetDimmer {
                    fixture: "fixture-2".into(),
                    value: 3.
                }
            ]
        })
        .is_err());
    assert_eq!(e.project(), &before);
}
#[test]
fn normal_drag_moves_driver_root_to_owner_inputs_and_undo_restores_attachment() {
    let mut p = Project::demo();
    p.content.get_mut("fixture-1").unwrap()[0]=Some(schema_rs::scenic::parse("name: Driver drag\nstack:\n  Modify.Value.Clamp:\n    Minimum:\n      Tool.Merge:\n        in:\n          - Create.Value.Constant: {Value: 0.2}\n          - Create.Value.Constant: {Value: 0.4}\n    in:\n      - Create.Value.Constant: {Value: 0.8}\n").unwrap());
    let root = &p.content["fixture-1"][0].as_ref().unwrap().root;
    let owner = root.id;
    let driver = root.drivers["Minimum"].clone();
    let before = p.clone();
    let mut e = Engine::new(p).unwrap();
    e.apply(Command::MoveNode {
        fixture: "fixture-1".into(),
        layer: 0,
        node: driver.id,
        target: owner,
        branch: true,
        below: false,
    })
    .unwrap();
    let after = &e.project().content["fixture-1"][0].as_ref().unwrap().root;
    assert!(!after.drivers.contains_key("Minimum"));
    assert_eq!(after.children.last(), Some(&driver));
    e.apply(Command::Undo).unwrap();
    assert_eq!(e.project(), &before);
    e.apply(Command::Redo).unwrap();
    assert!(!e.project().content["fixture-1"][0]
        .as_ref()
        .unwrap()
        .root
        .drivers
        .contains_key("Minimum"));
}
#[test]
fn normal_drag_moves_inputs_inside_driver_trees() {
    let mut p = Project::demo();
    let root = &mut p.content.get_mut("fixture-1").unwrap()[0]
        .as_mut()
        .unwrap()
        .root;
    let owner = root.id;
    let source = root.drivers["Amount"].children[0].id;
    let mut e = Engine::new(p).unwrap();
    e.apply(Command::MoveNode {
        fixture: "fixture-1".into(),
        layer: 0,
        node: source,
        target: owner,
        branch: true,
        below: false,
    })
    .unwrap();
    assert_eq!(
        e.project().content["fixture-1"][0]
            .as_ref()
            .unwrap()
            .root
            .children
            .last()
            .unwrap()
            .id,
        source
    );
}

#[test]
fn indexed_add_input_validates_bounds_and_preserves_legacy_append() {
    let mut e = Engine::new(Project::demo()).unwrap();
    let yaml="name: Order\nstack:\n  Tool.Merge:\n    in:\n      - Create.Value.Constant: {Value: 0.2}\n      - Create.Value.Constant: {Value: 0.3}\n";
    e.apply(Command::ReplaceLayer {
        fixture: "fixture-1".into(),
        layer: 0,
        stack: Some(schema_rs::scenic::parse(yaml).unwrap()),
    })
    .unwrap();
    let before = e.project().clone();
    let cmd=serde_json::from_value(serde_json::json!({"type":"add_input","fixture":"fixture-1","layer":0,"node":1,"behavior":"Modify.Value.Clamp","index":1})).unwrap();
    e.apply(cmd).unwrap();
    assert_eq!(
        e.project().content["fixture-1"][0]
            .as_ref()
            .unwrap()
            .root
            .children[1]
            .behavior,
        "Modify.Value.Clamp"
    );
    e.apply(Command::Undo).unwrap();
    assert_eq!(e.project(), &before);
    let bad=serde_json::from_value(serde_json::json!({"type":"add_input","fixture":"fixture-1","layer":0,"node":1,"behavior":"Modify.Value.Clamp","index":99})).unwrap();
    assert!(e.apply(bad).is_err());
    assert_eq!(e.project(), &before);
}
