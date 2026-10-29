use schema_rs::desktop::project_root_for;
#[test]
fn mac_uses_application_support_while_other_platforms_keep_their_paths() {
    use std::path::{Path, PathBuf};
    let h = Path::new("/Users/artist");
    assert_eq!(
        project_root_for("macos", Some(h), None, None).unwrap(),
        PathBuf::from("/Users/artist/Library/Application Support/Schema-s/project")
    );
    assert_eq!(
        project_root_for("linux", Some(h), Some(Path::new("/data")), None).unwrap(),
        PathBuf::from("/data/Schema-s/project")
    );
    assert_eq!(
        project_root_for("windows", None, None, Some(Path::new("C:/Local"))).unwrap(),
        PathBuf::from("C:/Local/Schema-s/project")
    );
    assert!(project_root_for("macos", None, None, None).is_err());
}
#[test]
fn usb_capability_is_host_based_and_requires_no_vendor_library_to_start() {
    assert_eq!(
        schema_rs::open_dmx::supported(),
        cfg!(any(windows, target_os = "macos"))
    );
}
