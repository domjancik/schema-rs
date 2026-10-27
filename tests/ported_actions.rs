use schema_rs::{
    engine::{Command, Engine},
    inputs::Event,
    model::Project,
    scenic,
    server::Runtime,
};
#[test]
fn director_runs_and_assigns_without_repeating_history() {
    let mut p = Project::demo();
    let next = scenic::parse("name: New\nstack:\n  Create.Value.Constant: {Value: 0.75}").unwrap();
    p.director[0]=Some(scenic::parse("name: Director\nstack:\n  Assign.Fixture: {Enabled: true, Fixture: fixture-1, Stack: New, Duration: 0}").unwrap());
    let mut e = Engine::new(p).unwrap();
    e.apply(Command::Input {
        event: Event::Stack {
            name: "New".into(),
            stack: next,
        },
    })
    .unwrap();
    let runtime = Runtime::start_engine(e, None).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        let frame = runtime.controller.current();
        if frame.project.content["fixture-1"][0].as_ref().unwrap().name == "New" {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "director assignment did not run"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    std::thread::sleep(std::time::Duration::from_millis(150));
    assert_eq!(runtime.controller.current().snapshot.revision, 1);
}
#[test]
fn director_content_roundtrips_separately_from_layout() {
    let mut p = Project::demo();
    p.director[1] = Some(
        scenic::parse("name: Master\nstack:\n  Create.Value.Expression: {Expression: s * 2.0}")
            .unwrap(),
    );
    let folder = std::env::temp_dir().join(format!("schema-director-{}", std::process::id()));
    schema_rs::project::save(&folder, &p, "test").unwrap();
    let loaded = schema_rs::project::load(&folder, "test").unwrap();
    assert_eq!(loaded.director, p.director);
    let mut engine = Engine::new(loaded).unwrap();
    engine.tick(0.25);
    engine.tick(0.25);
    assert_eq!(engine.snapshot().director[1].value, Some(1.));
    engine.checkpoint().validate().unwrap();
    std::fs::remove_dir_all(folder).unwrap();
}
#[tokio::test]
async fn group_assignment_precedes_fixture_override_and_transition_adopts_without_extra_undo() {
    let mut p = Project::demo();
    let original = p.content["fixture-1"][0].clone();
    p.director[0]=Some(scenic::parse("name: Director\nstack:\n  Tool.Merge:\n    in:\n      - Assign.Fixture: {Enabled: true, Fixture: fixture-1, Stack: Individual, Duration: 0.05}\n      - Assign.Group: {Enabled: true, Group: front, Layer: 0, Stack: Group, Duration: 0.05}").unwrap());
    let mut e = Engine::new(p).unwrap();
    for (name, value) in [("Individual", 0.8), ("Group", 0.3)] {
        e.apply(Command::Input {
            event: Event::Stack {
                name: name.into(),
                stack: scenic::parse(&format!(
                    "name: {name}\nstack:\n  Create.Value.Constant: {{Value: {value}}}"
                ))
                .unwrap(),
            },
        })
        .unwrap();
    }
    let runtime = Runtime::start_engine(e, None).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        let frame = runtime.controller.current();
        if frame.project.content["fixture-1"][0]
            .as_ref()
            .unwrap()
            .root
            .behavior
            == "Create.Value.Constant"
        {
            assert_eq!(
                frame.project.content["fixture-2"][0].as_ref().unwrap().name,
                "Group"
            );
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let checkpoint = runtime.controller.checkpoint().await.unwrap();
    assert_eq!(checkpoint.undo.len(), 1);
    // Undo the assignment against a directly resumed engine, before another tick.
    let mut engine = Engine::from_session(checkpoint).unwrap();
    engine.apply(Command::Undo).unwrap();
    assert_eq!(engine.project().content["fixture-1"][0], original);
}
#[test]
fn pixel_setup_is_saved_layout_and_does_not_add_undo_entries() {
    let mut p = Project::demo();
    p.fixtures.truncate(1);
    p.content.retain(|id, _| id == "fixture-1");
    p.content.get_mut("fixture-1").unwrap()[0] = Some(
        scenic::parse("name: Circle\nstack:\n  Setup.Pixels.Circle: {Count: 3, Radius: 2}")
            .unwrap(),
    );
    let runtime = Runtime::start(p).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        let frame = runtime.controller.current();
        if frame.project.fixtures[0].pixel_positions.len() == 3 {
            assert!(!frame.snapshot.can_undo);
            assert_eq!(frame.project.fixtures[0].pixel_positions[0][0], -2.);
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}
