use schema_rs::{
    model::{ContentEntry, Project},
    project,
};
#[test]
fn content_only_save_keeps_layout_and_supports_sparse_layers() {
    let root = std::env::temp_dir().join(format!("schema-rs-partial-{}", std::process::id()));
    let p = Project::demo();
    project::save(&root, &p, "init").unwrap();
    let layout = std::fs::read(root.join("registry/layouts/init.json")).unwrap();
    let entries = vec![ContentEntry {
        fixture: "fixture-1".into(),
        layer: 1,
        stack: None,
    }];
    project::save_content(&root, &entries, "partial").unwrap();
    assert_eq!(project::read_content(&root, "partial").unwrap(), entries);
    assert_eq!(
        std::fs::read(root.join("registry/layouts/init.json")).unwrap(),
        layout
    );
    std::fs::remove_dir_all(root).unwrap();
}
