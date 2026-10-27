use schema_rs::scenic;
#[test]
fn all_registered_behaviors_have_complete_browsing_metadata() {
    for b in scenic::catalog() {
        let row = serde_json::to_value(b).unwrap();
        for field in [
            "name",
            "category",
            "domain",
            "summary",
            "description",
            "icon",
        ] {
            assert!(
                !row[field].as_str().unwrap_or("").is_empty(),
                "{} missing {}",
                b.key,
                field
            );
        }
        assert!(row["keywords"].as_array().unwrap().len() >= 2);
    }
}
