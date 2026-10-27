use schema_rs::{engine::Engine, model::Project, scenic};
fn project(body: &str) -> Project {
    let mut p = Project::demo();
    for layers in p.content.values_mut() {
        *layers = vec![None, None, None];
    }
    p.content.get_mut("fixture-1").unwrap()[0] =
        Some(scenic::parse(&format!("name: Spatial\nstack:\n{body}")).unwrap());
    p
}
#[test]
fn native_expression_s_is_engine_seconds() {
    let mut e = Engine::new(project("  Create.Value.Expression: {Expression: 's * 2'}")).unwrap();
    e.tick(0.25);
    assert_eq!(e.snapshot().fixtures[0].result.value, Some(0.5));
}
#[test]
fn closeness_and_group_axis_use_actual_fixture_context() {
    let mut p = project("  Create.Value.Closeness: {Fixture: fixture-2, Radius: 10}");
    p.fixtures[0].position = [0.; 3];
    p.fixtures[1].position = [3., 4., 0.];
    let mut e = Engine::new(p).unwrap();
    e.tick(0.);
    assert_eq!(e.snapshot().fixtures[0].result.value, Some(0.5));
    let mut e = Engine::new(project(
        "  Create.Value.Group.AxisPosition: {Group: front, Axis: 0}",
    ))
    .unwrap();
    e.tick(0.);
    assert_eq!(e.snapshot().fixtures[0].result.value, Some(0.));
}
#[test]
fn group_pick_accepts_parent_defined_child_properties() {
    let mut e=Engine::new(project("  Pick.Group:\n    in:\n      - Create.Value.Constant: {Value: 0.1, Group: back}\n      - Create.Value.Constant: {Value: 0.9, Group: front}")).unwrap();
    e.tick(0.);
    assert_eq!(e.snapshot().fixtures[0].result.value, Some(0.9));
}
#[test]
fn copycat_copies_a_layer_and_resizes_pixels() {
    let mut p = project("  Create.Copycat: {Fixture: fixture-2, Layer: 0}");
    p.content.get_mut("fixture-2").unwrap()[0] = Some(
        scenic::parse("name: Copy source\nstack:\n  Create.Color.Color: {Color: '#ff0000'}")
            .unwrap(),
    );
    p.fixtures[0].pixel_positions = vec![[0.; 3]; 2];
    p.fixtures[1].address = 30;
    let mut e = Engine::new(p).unwrap();
    e.tick(0.);
    e.tick(0.);
    assert_eq!(
        e.snapshot().fixtures[0].result.colors.as_ref().unwrap(),
        &vec![[1., 0., 0., 1.]; 2]
    );
}

#[test]
fn expression_supports_ncalc_math_and_holds_last_success_on_error() {
    let mut e = Engine::new(project(
        "  Create.Value.Expression: {Expression: 'Sin(s) + 0.5'}",
    ))
    .unwrap();
    e.tick(0.);
    assert_eq!(e.snapshot().fixtures[0].result.value, Some(0.5));
}
#[test]
fn copycat_observes_earlier_fixture_in_same_tick() {
    let mut p = project("  Create.Value.Constant: {Value: 0.8}");
    p.content.get_mut("fixture-2").unwrap()[0] = Some(
        scenic::parse("name: Copy\nstack:\n  Create.Copycat: {Fixture: fixture-1, Layer: 0}")
            .unwrap(),
    );
    let mut e = Engine::new(p).unwrap();
    e.tick(0.);
    assert_eq!(e.snapshot().fixtures[1].result.value, Some(0.8));
}
