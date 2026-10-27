use schema_rs::{
    engine::{Command, Engine},
    model::Project,
    scenic,
};
#[test]
fn native_names_and_enum_literals_match_scenicscript() {
    assert!(
        scenic::parse("name: Wave\nstack:\n  Create.Value.LFO: {Type: Sine, Period: '5'}\n")
            .is_ok()
    );
    assert!(scenic::parse("name: Layer\nstack:\n  Create.Layer.Above: {Layer: 0}\n").is_ok());
}
#[test]
fn boolean_drivers_and_integrator_reset_are_supported() {
    let s=scenic::parse("name: Gate\nstack:\n  Filter.Manual:\n    Open:\n      Create.Value.Constant: {Value: 0}\n    in:\n      - Create.Value.Constant: {Value: 0.5}\n").unwrap();
    let mut e = Engine::new(Project::demo()).unwrap();
    e.apply(Command::ReplaceLayer {
        fixture: "fixture-1".into(),
        layer: 0,
        stack: Some(s),
    })
    .unwrap();
    e.tick(0.);
    e.tick(0.);
    assert!(e.snapshot().fixtures[0].result.value.is_none());
}
#[test]
fn color_inversion_uses_amount_instead_of_always_inverting() {
    let s=scenic::parse("name: Color\nstack:\n  Modify.Color.Invert:\n    Value: 0\n    in:\n      - Create.Color.Color: {Color: '#ff0000'}\n").unwrap();
    let mut e = Engine::new(Project::demo()).unwrap();
    e.apply(Command::ReplaceLayer {
        fixture: "fixture-1".into(),
        layer: 0,
        stack: Some(s),
    })
    .unwrap();
    e.tick(0.);
    assert_eq!(
        e.snapshot().fixtures[0].result.colors.as_ref().unwrap()[0],
        [1., 0., 0., 1.]
    );
}
