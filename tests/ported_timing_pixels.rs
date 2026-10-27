use schema_rs::{
    engine::{Command, Engine},
    model::Project,
    scenic,
};
fn engine(key: &str, properties: &str) -> Engine {
    let mut p = Project::demo();
    for layers in p.content.values_mut() {
        *layers = vec![None, None, None];
    }
    let yaml=format!("name: Timed\nstack:\n  {key}:\n{properties}    in:\n      - Create.Value.Constant: {{Value: 0}}\n");
    p.content.get_mut("fixture-1").unwrap()[0] = Some(scenic::parse(&yaml).unwrap());
    Engine::new(p).unwrap()
}
fn input(e: &mut Engine, value: f64) {
    e.apply(Command::SetProperty {
        fixture: "fixture-1".into(),
        layer: 0,
        node: 2,
        key: "Value".into(),
        value: serde_json::json!(value),
    })
    .unwrap();
}
fn value(e: &Engine) -> f64 {
    e.snapshot().fixtures[0].result.value.unwrap()
}
#[test]
fn edges_and_toggle_keep_state_when_child_values_are_edited() {
    let mut e = engine("Modify.Value.Edge.Rising", "");
    e.tick(0.);
    assert_eq!(value(&e), 0.);
    input(&mut e, 1.);
    e.tick(0.1);
    assert_eq!(value(&e), 1.);
    e.tick(0.1);
    assert_eq!(value(&e), 0.);
    input(&mut e, 0.);
    e.tick(0.1);
    input(&mut e, 1.);
    e.tick(0.1);
    assert_eq!(value(&e), 1.);
    let mut toggle = engine("Modify.Value.Toggle", "");
    toggle.tick(0.);
    input(&mut toggle, 1.);
    toggle.tick(0.1);
    assert_eq!(value(&toggle), 1.);
    input(&mut toggle, 0.);
    toggle.tick(0.1);
    assert_eq!(value(&toggle), 1.);
    input(&mut toggle, 1.);
    toggle.tick(0.1);
    assert_eq!(value(&toggle), 0.);
}
#[test]
fn delay_and_hold_have_explicit_time_boundaries() {
    let mut e = engine("Modify.Delay", "    Duration: 0.2\n");
    e.tick(0.);
    input(&mut e, 1.);
    e.tick(0.1);
    assert_eq!(e.snapshot().fixtures[0].result.value, None);
    e.tick(0.1);
    assert_eq!(value(&e), 0.);
    e.tick(0.1);
    assert_eq!(value(&e), 1.);
    let mut hold = engine("Modify.Value.Hold", "    Time: 0.2\n");
    hold.tick(0.);
    input(&mut hold, 1.);
    hold.tick(0.1);
    input(&mut hold, 0.);
    hold.tick(0.1);
    assert_eq!(value(&hold), 1.);
    hold.tick(0.2);
    assert_eq!(value(&hold), 0.);
}
#[test]
fn channel_selection_uses_setup_channel_not_fixture_ordinal() {
    let mut p = Project::demo();
    p.fixtures[0].setup_channel = 1;
    p.content.get_mut("fixture-1").unwrap()[0]=Some(scenic::parse("name: Pick\nstack:\n  Pick.Channel:\n    in:\n      - Create.Value.Constant: {Value: 0.2}\n      - Create.Value.Constant: {Value: 0.8}").unwrap());
    let mut e = Engine::new(p).unwrap();
    e.tick(0.);
    assert_eq!(value(&e), 0.8);
}
#[test]
fn pixel_segment_uses_fixture_count_direction_and_multiple_dmx_pixels() {
    let mut p = Project::demo();
    p.fixtures.truncate(1);
    p.fixtures[0].pixel_positions = vec![[0., 0., 0.], [1., 0., 0.], [2., 0., 0.], [3., 0., 0.]];
    p.content.retain(|id, _| id == "fixture-1");
    p.content.get_mut("fixture-1").unwrap()[0]=Some(scenic::parse("name: Segment\nstack:\n  Create.Color.Pixel.Segment:\n    Repeat: 1\n    in:\n      - Create.Color.Color: {Color: '#ff0000'}\n      - Create.Color.Color: {Color: '#0000ff'}").unwrap());
    let mut e = Engine::new(p).unwrap();
    e.tick(0.);
    assert_eq!(
        e.snapshot().fixtures[0]
            .result
            .colors
            .as_ref()
            .unwrap()
            .len(),
        4
    );
    assert_eq!(
        &e.snapshot().universes[&0][..12],
        &[255, 0, 0, 255, 0, 0, 0, 0, 255, 0, 0, 255]
    );
}
