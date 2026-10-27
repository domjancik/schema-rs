use schema_rs::{
    engine::{Command, Engine},
    model::Project,
    scenic,
};
#[test]
fn linear_crossfade_uses_all_ordered_inputs() {
    let s=scenic::parse("name: Mix\nstack:\n  Combine.Crossfade:\n    Fader: 0.5\n    in:\n      - Create.Value.Constant: {Value: 0}\n      - Create.Value.Constant: {Value: 0.5}\n      - Create.Value.Constant: {Value: 1}\n").unwrap();
    let mut e = Engine::new(Project::demo()).unwrap();
    e.apply(Command::ReplaceLayer {
        fixture: "fixture-1".into(),
        layer: 0,
        stack: Some(s),
    })
    .unwrap();
    e.tick(0.);
    assert_eq!(e.snapshot().fixtures[0].result.value, Some(0.5));
}
#[test]
fn crossfade_accepts_documented_shapes_and_rejects_out_of_range() {
    for shape in 0..=10 {
        assert!(scenic::parse(&format!(
            "name: Shape\nstack:\n  Combine.Crossfade: {{Shape: {shape}}}\n"
        ))
        .is_ok());
    }
    assert!(scenic::parse("name: Shape\nstack:\n  Combine.Crossfade: {Shape: 11}\n").is_err());
}
