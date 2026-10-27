use schema_rs::{
    engine::{Command, Engine},
    model::Project,
    scenic,
};
fn engine(yaml: &str) -> Engine {
    let mut p = Project::demo();
    p.fixtures[0].position = [0., 0., 0.];
    p.fixtures[1].position = [3., 4., 0.];
    p.content.get_mut("fixture-1").unwrap()[0] = Some(scenic::parse(yaml).unwrap());
    Engine::new(p).unwrap()
}
#[test]
fn spatial_distance_can_select_a_fixture_and_follows_its_motion() {
    let mut e = engine("name: Distance\nstack:\n  Spatial.Distance: {Fixture: fixture-2}\n");
    e.tick(0.);
    assert_eq!(e.snapshot().fixtures[0].result.value, Some(5.));
    e.apply(Command::MoveFixture {
        fixture: "fixture-2".into(),
        position: [6., 8., 0.],
    })
    .unwrap();
    e.tick(0.);
    assert_eq!(e.snapshot().fixtures[0].result.value, Some(10.));
}
#[test]
fn native_reference_child_overrides_selector_and_is_consumed() {
    let mut e=engine("name: Referenced\nstack:\n  Create.Value.Distance:\n    Fixture: missing-selector\n    in:\n      - Create.Reference.Fixture: {Fixture: 'Wash 02'}\n");
    e.tick(0.);
    assert_eq!(e.snapshot().fixtures[0].result.value, Some(5.));
    assert!(
        serde_json::to_value(&e.snapshot().fixtures[0].result).unwrap()["fixture_reference"]
            .is_null()
    );
}
#[test]
fn auto_source_uses_input_position_while_setup_forces_scene_position() {
    let yaml="name: Source\nstack:\n  Create.Value.Distance:\n    Fixture: fixture-2\n    Auto/Setup: false\n    in:\n      - Create.Position.Constant: {Position: [3, 0, 0]}\n";
    let mut e = engine(yaml);
    e.tick(0.);
    assert_eq!(e.snapshot().fixtures[0].result.value, Some(4.));
    let mut setup = engine(&yaml.replace("Auto/Setup: false", "Auto/Setup: true"));
    setup.tick(0.);
    assert_eq!(setup.snapshot().fixtures[0].result.value, Some(5.));
}
#[test]
fn reference_target_prefers_evaluated_position_with_live_fixture_order() {
    let mut e = engine("name: Distance\nstack:\n  Create.Value.Distance: {Fixture: fixture-2}\n");
    let target = scenic::parse(
        "name: Position\nstack:\n  Create.Position.Constant: {Position: [0, 0, 12]}\n",
    )
    .unwrap();
    e.apply(Command::ReplaceLayer {
        fixture: "fixture-2".into(),
        layer: 0,
        stack: Some(target),
    })
    .unwrap();
    e.tick(0.);
    assert_eq!(e.snapshot().fixtures[0].result.value, Some(5.));
    e.tick(0.);
    assert_eq!(e.snapshot().fixtures[0].result.value, Some(12.));
}
#[test]
fn missing_fixture_produces_diagnostic_instead_of_distance_to_origin() {
    let mut e = engine("name: Missing\nstack:\n  Spatial.Distance: {Fixture: no-such-fixture}\n");
    e.tick(0.);
    assert!(e.snapshot().fixtures[0].result.value.is_none());
    assert!(e
        .snapshot()
        .diagnostics
        .iter()
        .any(|d| d.contains("no-such-fixture")));
}
#[test]
fn fixed_point_programs_remain_compatible() {
    let mut e = engine("name: Point\nstack:\n  Spatial.Distance: {Position: [0, 0, 7]}\n");
    e.tick(0.);
    assert_eq!(e.snapshot().fixtures[0].result.value, Some(7.));
}
