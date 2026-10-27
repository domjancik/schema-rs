use schema_rs::{
    engine::{Command, Engine},
    model::Project,
};
#[test]
fn engine_resume_keeps_clock_revision_and_driver_values() {
    let mut old = Engine::new(Project::demo()).unwrap();
    for _ in 0..100 {
        old.tick(1. / 60.);
    }
    old.apply(Command::MoveFixture {
        fixture: "fixture-1".into(),
        position: [2., 0., 0.],
    })
    .unwrap();
    old.tick(1. / 60.);
    let snapshot = old.snapshot();
    old.tick(0.);
    let result = old.snapshot().fixtures[0].result.clone();
    let mut resumed = Engine::new(old.project().clone()).unwrap();
    resumed.resume_snapshot(snapshot.clone()).unwrap();
    resumed.tick(0.);
    assert_eq!(resumed.snapshot().time, snapshot.time);
    assert_eq!(resumed.snapshot().revision, snapshot.revision);
    assert_eq!(resumed.snapshot().fixtures[0].result, result);
    assert!(!resumed.snapshot().can_undo);
}
