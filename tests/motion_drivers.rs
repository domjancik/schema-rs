use schema_rs::{
    engine::{Command, Engine},
    model::Project,
    scenic,
};
fn engine(yaml: &str) -> Engine {
    let mut p = Project::demo();
    p.content.get_mut("fixture-1").unwrap()[0] = Some(scenic::parse(yaml).unwrap());
    Engine::new(p).unwrap()
}
#[test]
fn position_drivers_keep_signed_world_coordinates() {
    let mut e=engine("name: Signed\nstack:\n  Create.Position.XYZ:\n    X:\n      Create.Value.Constant: {Value: -3}\n    Y:\n      Create.Value.Constant: {Value: 5}\n");
    e.tick(0.);
    e.tick(0.);
    assert_eq!(
        e.snapshot().fixtures[0].result.position,
        Some([-3., 5., 0.])
    );
}
#[test]
fn pausing_a_random_node_does_not_restart_its_sequence() {
    let mut e = engine(
        "name: Random\nstack:\n  Create.Value.Random: {Interval: 0.1, Filter Time: 0, Seed: 42}",
    );
    for _ in 0..8 {
        e.tick(0.1);
    }
    let before = e.snapshot().fixtures[0].result.value;
    e.apply(Command::SetProperty {
        fixture: "fixture-1".into(),
        layer: 0,
        node: 1,
        key: "Pause".into(),
        value: serde_json::json!(true),
    })
    .unwrap();
    e.tick(0.1);
    assert_eq!(e.snapshot().fixtures[0].result.value, before);
}
