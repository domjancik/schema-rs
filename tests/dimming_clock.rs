use schema_rs::{
    dimming_clock::{Clock, Control},
    engine::Engine,
    model::Project,
};
use std::time::{Duration, Instant};
#[test]
fn simulated_clock_wraps_midnight_and_expires_without_changing_live_time() {
    let now = Instant::now();
    let mut clock = Clock::default();
    clock
        .control_at(
            Control::Simulate {
                owner: "test".into(),
                seconds: 86390.,
                speed: 60.,
            },
            now,
        )
        .unwrap();
    assert_eq!(clock.seconds_at(1234, now + Duration::from_secs(1)), 50);
    assert_eq!(clock.seconds_at(1234, now + Duration::from_secs(16)), 1234);
    assert!(clock
        .control_at(
            Control::Renew {
                owner: "test".into()
            },
            now + Duration::from_secs(16)
        )
        .is_err());
    assert!(clock
        .control_at(
            Control::Simulate {
                owner: "test".into(),
                seconds: 0.,
                speed: f64::INFINITY
            },
            now
        )
        .is_err());
}
#[test]
fn simulation_is_scoped_to_dimming_and_never_enters_projects_or_history() {
    let mut p = Project::demo();
    p.fixtures.truncate(1);
    p.content.retain(|id, _| id == "fixture-1");
    p.content.get_mut("fixture-1").unwrap()[0] = Some(
        schema_rs::scenic::parse("name: Constant\nstack:\n  Create.Value.Constant: {Value: 1}\n")
            .unwrap(),
    );
    p.fixtures[0].model = schema_rs::model::FixtureModel::Dimmer;
    p.dimming.enabled = true;
    p.dimming.points = vec![
        schema_rs::output_design::Point {
            minute: 0,
            level: 0.,
        },
        schema_rs::output_design::Point {
            minute: 720,
            level: 1.,
        },
    ];
    let mut engine = Engine::new(p.clone()).unwrap();
    engine
        .set_dimming_clock(Control::Simulate {
            owner: "test".into(),
            seconds: 43200.,
            speed: 0.,
        })
        .unwrap();
    engine.tick(0.01);
    assert_eq!(engine.current_snapshot().universes[&0][0], 255);
    assert_eq!(engine.project(), &p);
    assert_eq!(engine.revision(), 0);
    let session = serde_json::to_value(engine.checkpoint()).unwrap();
    assert_eq!(session["inputs"]["clock_seconds"], 0);
    assert!(!serde_json::to_string(&session)
        .unwrap()
        .contains("simulating"));
    assert!(!engine.current_snapshot().can_undo);
    engine
        .set_dimming_clock(Control::Live {
            owner: Some("another-view".into()),
        })
        .unwrap();
    assert!(engine.dimming_status().simulating);
    engine
        .set_dimming_clock(Control::Live {
            owner: Some("test".into()),
        })
        .unwrap();
    engine.tick(0.01);
    assert_eq!(engine.current_snapshot().universes[&0][0], 0);
}

#[test]
fn newer_view_owns_the_lease_and_old_cleanup_cannot_stop_it() {
    let now = Instant::now();
    let mut clock = Clock::default();
    for owner in ["old", "new"] {
        clock
            .control_at(
                Control::Simulate {
                    owner: owner.into(),
                    seconds: 43200.,
                    speed: 0.,
                },
                now,
            )
            .unwrap();
    }
    clock
        .control_at(
            Control::Live {
                owner: Some("old".into()),
            },
            now,
        )
        .unwrap();
    assert_eq!(clock.seconds_at(0, now), 43200);
    assert!(clock
        .control_at(
            Control::Renew {
                owner: "old".into()
            },
            now
        )
        .is_err());
    clock
        .control_at(
            Control::Renew {
                owner: "new".into(),
            },
            now + Duration::from_secs(10),
        )
        .unwrap();
    assert_eq!(clock.seconds_at(0, now + Duration::from_secs(20)), 43200);
    clock
        .control_at(Control::Live { owner: None }, now + Duration::from_secs(20))
        .unwrap();
    assert_eq!(clock.seconds_at(123, now + Duration::from_secs(20)), 123);
}
#[tokio::test]
async fn native_clock_api_is_transient_and_validated() {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use schema_rs::server::{router, AppState, Runtime};
    use tower::ServiceExt;
    let runtime = Runtime::start(Project::demo()).unwrap();
    let app = router(AppState::new(
        runtime.controller.clone(),
        std::env::temp_dir(),
        None,
    ));
    for (body, expected) in [
        (
            serde_json::json!({"action":"simulate","owner":"native-test","seconds":43200,"speed":0}),
            StatusCode::OK,
        ),
        (
            serde_json::json!({"action":"simulate","owner":"native-test","seconds":86400,"speed":0}),
            StatusCode::BAD_REQUEST,
        ),
        (serde_json::json!({"action":"live"}), StatusCode::OK),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/dimming/clock")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        if expected == StatusCode::OK {
            let value: serde_json::Value =
                serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap())
                    .unwrap();
            assert_eq!(value["simulating"], body["action"] == "simulate");
        }
    }
    assert_eq!(runtime.controller.current().snapshot.revision, 0);
    assert!(!runtime.controller.current().snapshot.can_undo);
}
