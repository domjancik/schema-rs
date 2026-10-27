use schema_rs::{
    artnet,
    engine::{Command, Engine},
    model::*,
    scenic,
};
use serde_json::json;
fn constant(value: f64) -> Stack {
    scenic::parse(&format!(
        "name: Constant\nstack:\n  Create.Value.Constant:\n    Value: {value}\n"
    ))
    .unwrap()
}
#[test]
fn scenic_preserves_order_and_separate_drivers() {
    let s=scenic::parse("name: Driven\nstack:\n  Combine.Multiply:\n    in:\n      - Create.Value.Constant: {Value: 0.5}\n      - Create.Value.Constant:\n          Value:\n            Create.Value.Constant: {Value: 0.2}\n").unwrap();
    assert_eq!(s.root.children.len(), 2);
    assert!(s.root.children[1].drivers.contains_key("Value"));
    let round = scenic::parse(&scenic::encode(&s).unwrap()).unwrap();
    assert_eq!(round.root.children.len(), 2);
}
#[test]
fn invalid_and_unknown_programs_are_rejected() {
    assert!(scenic::parse("name: Bad\nstack:\n  Mystery.Block: {}\n").is_err());
    assert!(scenic::parse(
        "name: Bad\nstack:\n  Create.Value.Constant: {}\n  Create.Value.LFO: {}\n"
    )
    .is_err());
    assert!(scenic::parse("name: Bad\nstack:\n  Create.Value.Constant: {Value: nope}\n").is_err());
}
#[test]
fn drivers_take_effect_on_next_evaluation() {
    let s=scenic::parse("name: Driver\nstack:\n  Create.Value.Constant:\n    Value:\n      Create.Value.Constant: {Value: 0.2}\n").unwrap();
    let mut e = Engine::new(Project::demo()).unwrap();
    e.apply(Command::ReplaceLayer {
        fixture: "fixture-1".into(),
        layer: 0,
        stack: Some(s),
    })
    .unwrap();
    e.tick(0.0);
    let first = e.snapshot().fixtures[0].result.value.unwrap();
    e.tick(0.1);
    assert_eq!(e.snapshot().fixtures[0].result.value, Some(0.2));
    assert_ne!(first, 0.2);
}
#[test]
fn undo_redo_are_authored_transactions_and_invalid_edits_are_atomic() {
    let mut e = Engine::new(Project::demo()).unwrap();
    let before = e.project().clone();
    e.apply(Command::MoveFixture {
        fixture: "fixture-1".into(),
        position: [2., 3., 4.],
    })
    .unwrap();
    e.apply(Command::Undo).unwrap();
    assert_eq!(e.project(), &before);
    e.apply(Command::Redo).unwrap();
    assert_eq!(e.project().fixtures[0].position, [2., 3., 4.]);
    let after = e.project().clone();
    assert!(e
        .apply(Command::SetDimmer {
            fixture: "fixture-1".into(),
            value: 2.
        })
        .is_err());
    assert_eq!(e.project(), &after);
}
#[test]
fn layer_above_flows_higher_to_lower() {
    let mut p = Project::demo();
    p.content.get_mut("fixture-1").unwrap()[1] = Some(constant(0.3));
    p.content.get_mut("fixture-1").unwrap()[0]=Some(scenic::parse("name: Above\nstack:\n  Modify.Value.Invert:\n    in:\n      - Create.Layer.Above: {Layer: 0}\n").unwrap());
    let mut e = Engine::new(p).unwrap();
    e.tick(0.0);
    assert!((e.snapshot().fixtures[0].result.value.unwrap() - 0.7).abs() < 1e-9);
}
#[test]
fn native_spatial_maps_and_fixture_dimming_reach_physical_channels() {
    let mut p = Project::demo();
    p.fixtures[0].position = [5., 0., 0.];
    p.fixtures[0].model = FixtureModel::Dimmer;
    p.fixtures[0].dimmer = 0.5;
    p.content.get_mut("fixture-1").unwrap()[0]=Some(scenic::parse("name: Spatial\nstack:\n  Modify.Value.MapFromTo:\n    Source Min: 0\n    Source Max: 10\n    Minimum: 0\n    Maximum: 1\n    in:\n      - Spatial.Distance:\n          Position: [0, 0, 0]\n").unwrap());
    let mut e = Engine::new(p).unwrap();
    e.tick(0.0);
    assert_eq!(e.snapshot().universes[&0][0], 64);
}
#[test]
fn artdmx_header_padding_bounds_and_final_slot() {
    let packet = artnet::packet(0x1234, 1, &[255, 128, 0]).unwrap();
    assert_eq!(&packet[..8], b"Art-Net\0");
    assert_eq!(&packet[8..12], &[0, 0x50, 0, 14]);
    assert_eq!(&packet[14..18], &[0x34, 0x12, 0, 4]);
    assert_eq!(packet.len(), 22);
    assert!(artnet::packet(32768, 1, &[0, 0]).is_err());
    let mut p = Project::demo();
    p.fixtures.truncate(1);
    p.fixtures[0].address = 512;
    p.fixtures[0].model = FixtureModel::Dimmer;
    p.content.get_mut("fixture-1").unwrap()[0] = Some(constant(1.));
    let mut e = Engine::new(p).unwrap();
    e.tick(0.);
    assert_eq!(e.snapshot().universes[&0][511], 255);
}
#[test]
fn scripts_are_bounded_and_have_explicit_context() {
    let mut p = Project::demo();
    p.content.get_mut("fixture-1").unwrap()[0]=Some(scenic::parse("name: Script\nstack:\n  Script.Value:\n    Code: 'x * 0.25 + input'\n    in:\n      - Create.Value.Constant: {Value: 0.1}\n").unwrap());
    p.fixtures[0].position = [2., 0., 0.];
    let mut e = Engine::new(p).unwrap();
    e.tick(0.);
    assert!((e.snapshot().fixtures[0].result.value.unwrap() - 0.6).abs() < 1e-9);
    let bad = scenic::parse("name: Loop\nstack:\n  Script.Value:\n    Code: 'loop {}'\n").unwrap();
    e.apply(Command::ReplaceLayer {
        fixture: "fixture-1".into(),
        layer: 0,
        stack: Some(bad),
    })
    .unwrap();
    e.tick(1.);
    assert!(!e.snapshot().diagnostics.is_empty());
}
#[test]
fn partial_content_does_not_replace_layout_or_other_layers() {
    let mut e = Engine::new(Project::demo()).unwrap();
    let fixture = e.project().fixtures[0].clone();
    let higher = e.project().content["fixture-1"][1].clone();
    e.apply(Command::ApplyContent {
        entries: vec![ContentEntry {
            fixture: "fixture-1".into(),
            layer: 0,
            stack: Some(constant(0.8)),
        }],
    })
    .unwrap();
    assert_eq!(e.project().fixtures[0], fixture);
    assert_eq!(e.project().content["fixture-1"][1], higher);
}
#[test]
fn project_folders_roundtrip_and_content_names_cannot_escape() {
    let root = std::env::temp_dir().join(format!("schema-rs-test-{}", std::process::id()));
    let p = Project::demo();
    schema_rs::project::save(&root, &p, "init").unwrap();
    let loaded = schema_rs::project::load(&root, "init").unwrap();
    assert_eq!(p, loaded);
    assert!(schema_rs::project::save(&root, &p, "../escape").is_err());
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn property_commands_reject_unknown_properties() {
    let mut e = Engine::new(Project::demo()).unwrap();
    let id = e.project().content["fixture-1"][0]
        .as_ref()
        .unwrap()
        .root
        .id;
    assert!(e
        .apply(Command::SetProperty {
            fixture: "fixture-1".into(),
            layer: 0,
            node: id,
            key: "Missing".into(),
            value: json!(1)
        })
        .is_err());
}
