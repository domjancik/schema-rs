use schema_rs::scenic;
#[test]
fn all_documented_nodes_have_executable_ports() {
    let documented: serde_json::Value =
        serde_json::from_str(include_str!("../docs/documented-catalog.json")).unwrap();
    let missing: Vec<_> = documented
        .as_object()
        .unwrap()
        .keys()
        .filter(|key| !scenic::catalog().iter().any(|b| b.key == key.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "Missing {}/178 documented ports: {}",
        missing.len(),
        missing
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
}
#[test]
fn every_catalog_default_constructs_evaluates_and_serializes() {
    use schema_rs::{
        engine::Engine,
        model::{Node, Project, Stack},
    };
    let mut failures = vec![];
    for behavior in scenic::catalog() {
        let mut p = Project::demo();
        for layers in p.content.values_mut() {
            *layers = vec![None, None, None];
        }
        p.content.get_mut("fixture-1").unwrap()[0] = Some(Stack {
            enabled: true,
            name: behavior.key.into(),
            root: Node {
                id: 1,
                behavior: behavior.key.into(),
                properties: behavior.defaults.clone(),
                children: vec![],
                drivers: Default::default(),
            },
        });
        match Engine::new(p) {
            Ok(mut e) => {
                e.tick(0.01);
                e.tick(0.02);
                let checkpoint = e.checkpoint();
                let mut resumed = Engine::from_session(checkpoint).unwrap();
                e.tick(0.03);
                resumed.tick(0.03);
                assert_eq!(
                    serde_json::to_value(e.snapshot().fixtures[0].result.clone()).unwrap(),
                    serde_json::to_value(resumed.snapshot().fixtures[0].result.clone()).unwrap(),
                    "runtime continuity for {}",
                    behavior.key
                );
                assert!(
                    !e.snapshot()
                        .diagnostics
                        .iter()
                        .any(|d| d.contains("Unsupported native")),
                    "{}",
                    behavior.key
                );
                if let Err(error) = e.checkpoint().validate() {
                    failures.push(format!("{}: checkpoint {error}", behavior.key));
                }
                if let Err(error) = serde_json::to_string(&e.snapshot()) {
                    failures.push(format!("{}: {error}", behavior.key));
                }
            }
            Err(error) => failures.push(format!("{}: {error}", behavior.key)),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
