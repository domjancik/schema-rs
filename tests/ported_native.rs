use schema_rs::{engine::Engine, model::Project, scenic};
fn run(body: &str) -> serde_json::Value {
    let mut project = Project::demo();
    project.content.get_mut("fixture-1").unwrap()[0] =
        Some(scenic::parse(&format!("name: Native\nstack:\n{body}")).unwrap());
    let mut engine = Engine::new(project).unwrap();
    engine.tick(0.);
    serde_json::to_value(&engine.snapshot().fixtures[0].result).unwrap()
}
#[test]
fn duration_speed_and_rotation_are_distinct_merged_channels() {
    let result=run("  Tool.Merge:\n    in:\n      - Create.Duration.Constant: {Value: 2.5}\n      - Create.Speed.Constant: {Range: 2, Value: 0.25}\n      - Create.Rotation.PanTilt: {Pan: 0.2, Tilt: 0.7}");
    assert_eq!(result["duration"], 2.5);
    assert_eq!(result["speed"], -0.5);
    assert_eq!(result["rotation"], serde_json::json!([0., 0.2, 0.7]));
}
#[test]
fn moving_head_slots_do_not_overwrite_other_slots() {
    let result=run("  Tool.Merge:\n    in:\n      - Create.Frost.Value: {Slot: 0, Value: 0.3}\n      - Create.Frost.Value: {Slot: 2, Value: 0.8}\n      - Create.Focus.Value: {Value: 0.4}");
    assert_eq!(result["channels"]["Frost"]["0"], 0.3);
    assert_eq!(result["channels"]["Frost"]["2"], 0.8);
    assert_eq!(result["channels"]["Focus"], 0.4);
}
#[test]
fn comparison_boolean_and_normalized_wave_are_computed() {
    let cmp=run("  Combine.Value.Compare:\n    Operator: 4\n    Epsilon: 0.01\n    in:\n      - Create.Value.Constant: {Value: 0.5}\n      - Create.Value.Constant: {Value: 0.505}");
    assert_eq!(cmp["value"], 1.);
    let or=run("  Combine.Value.Boolean:\n    And/Or: true\n    in:\n      - Create.Value.Constant: {Value: 0}\n      - Create.Value.Constant: {Value: 1}");
    assert_eq!(or["value"], 1.);
    let wave = run(
        "  Modify.Value.Wave:\n    Shape: 2\n    in:\n      - Create.Value.Constant: {Value: 0.25}",
    );
    assert_eq!(wave["value"], 1.);
}
#[test]
fn color_alpha_hue_saturation_and_clamp_are_real_operations() {
    let alpha = run(
        "  Modify.Color.Alpha.AsAlpha:\n    in:\n      - Create.Color.Color: {Color: '#4080ff'}",
    );
    assert_eq!(alpha["colors"][0], serde_json::json!([1., 1., 1., 1.]));
    let hue=run("  Modify.Color.Hue.Rotate:\n    Value: 0.5\n    in:\n      - Create.Color.Color: {Color: '#ff0000'}");
    assert_eq!(hue["colors"][0], serde_json::json!([0., 1., 1., 1.]));
    let sat=run("  Modify.Color.Saturation.Set:\n    Value: 0\n    in:\n      - Create.Color.Color: {Color: '#ff0000'}");
    assert_eq!(sat["colors"][0], serde_json::json!([1., 1., 1., 1.]));
}
#[test]
fn native_dim_value_is_supported_without_breaking_legacy_amount() {
    for property in ["Value", "Amount"] {
        let dim=run(&format!("  Modify.Color.Dim:\n    {property}: 0.25\n    in:\n      - Create.Color.Color: {{Color: '#ffffff'}}"));
        assert_eq!(dim["colors"][0], serde_json::json!([0.25, 0.25, 0.25, 1.]));
    }
}

#[test]
fn speed_to_target_uses_two_ordered_values_and_keeps_sign() {
    let result = run("  Convert.Speed.PositionToTarget:\n    in:\n      - Create.Value.Constant: {Value: 0.8}\n      - Create.Value.Constant: {Value: 0.2}");
    assert!((result["speed"].as_f64().unwrap() + 0.6).abs() < 1e-9);
}
#[test]
fn first_child_generators_do_not_union_later_sibling_banks() {
    let result=run("  Create.Gobo.Constant:\n    Slot: 2\n    in:\n      - Create.Value.Constant: {Value: 0.2}\n      - Create.Value.Constant: {Value: 0.8}");
    assert_eq!(result["value"], 0.2);
}
