use schema_rs::{model::Project, tui};
#[test]
fn progressive_ascii_render_reveals_properties_and_keeps_children_above_parent() {
    let p = Project::demo();
    let root = &p.content["fixture-1"][0].as_ref().unwrap().root;
    let far = tui::render_stack(root, root.id, 0, 160, 60);
    let near = tui::render_stack(root, root.id, 3, 160, 60);
    assert!(!far.contains("Modify.Color.Dim"));
    assert!(near.contains("Modify.Color.Dim"));
    assert!(near.contains("Amount"));
    assert!(near.is_ascii());
    let color = near
        .lines()
        .position(|s| s.contains("Create.Color.Color"))
        .unwrap();
    let dim = near
        .lines()
        .position(|s| s.contains("Modify.Color.Dim"))
        .unwrap();
    assert!(color < dim);
}
#[test]
fn ascii_clipping_has_fixed_dimensions() {
    let p = Project::demo();
    let root = &p.content["fixture-1"][0].as_ref().unwrap().root;
    let output = tui::render_stack(root, root.id, 3, 17, 9);
    assert_eq!(output.lines().count(), 9);
    assert!(output.lines().all(|s| s.len() == 17));
}
