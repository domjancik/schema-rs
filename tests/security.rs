use schema_rs::model::Project;
#[test]
fn scene_labels_cannot_inject_terminal_control_sequences() {
    let mut p = Project::demo();
    p.fixtures[0].name = "bad\u{1b}[2J".into();
    assert!(p.validate().is_err());
}
