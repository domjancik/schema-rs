use schema_rs::{
    engine::Engine,
    model::{DebugFocus, Project},
};
fn target(node: u64) -> DebugFocus {
    DebugFocus {
        fixture: "fixture-1".into(),
        layer: 0,
        node,
    }
}
#[test]
fn inspection_is_one_shot_exact_and_does_not_change_history_or_runtime() {
    let mut p = Project::demo();
    p.content.get_mut("fixture-1").unwrap()[0]=Some(schema_rs::scenic::parse("name: Inspection\nstack:\n  Modify.Color.Dim:\n    Amount: 0.5\n    in:\n      - Create.Color.Color: {Color: '#ff0000'}\n").unwrap());
    let root = p.content["fixture-1"][0].as_ref().unwrap().root.id;
    let mut observed = Engine::new(p.clone()).unwrap();
    let mut control = Engine::new(p).unwrap();
    assert!(observed
        .request_inspection(vec![target(root)])
        .unwrap()
        .blocks
        .is_empty());
    observed.tick(0.02);
    control.tick(0.02);
    let sample = observed.request_inspection(vec![]).unwrap();
    assert_eq!(sample.blocks.len(), 1);
    assert_eq!(sample.blocks[0].incoming.attributes["colors"][0][0], 1.0);
    assert_eq!(sample.blocks[0].outgoing.attributes["colors"][0][0], 0.5);
    assert_eq!(observed.current_snapshot().revision, 0);
    let mut a = serde_json::to_value(observed.checkpoint()).unwrap();
    let mut b = serde_json::to_value(control.checkpoint()).unwrap();
    a["snapshot"]["evaluation_us"] = 0.into();
    b["snapshot"]["evaluation_us"] = 0.into();
    assert_eq!(a, b);
    observed.tick(0.02);
    assert_eq!(
        observed.request_inspection(vec![]).unwrap().frame,
        sample.frame
    );
    assert!(observed.request_inspection(vec![target(u64::MAX)]).is_err());
    assert!(observed
        .request_inspection(vec![target(root); 257])
        .is_err());
}

#[test]
fn large_attributes_are_sampled_before_serialization() {
    let bank = schema_rs::model::Bank {
        colors: Some(vec![[1., 0., 0., 0.5]; 65536]),
        texture: Some(schema_rs::inputs::Texture {
            width: 512,
            height: 128,
            pixels: vec![[0.; 4]; 65536],
        }),
        ..Default::default()
    };
    let preview = schema_rs::inspection::Attributes::capture(&bank);
    assert_eq!(preview.counts["colors"], 65536);
    assert_eq!(preview.attributes["colors"].as_array().unwrap().len(), 8);
    assert!(serde_json::to_vec(&preview).unwrap().len() < 2048);
}

#[test]
fn separate_clients_do_not_displace_each_others_samples() {
    let mut engine = Engine::new(Project::demo()).unwrap();
    let root = engine.project().content["fixture-1"][0]
        .as_ref()
        .unwrap()
        .root
        .id;
    let a = target(root);
    let mut b = a.clone();
    b.fixture = "fixture-2".into();
    engine.request_inspection(vec![a.clone()]).unwrap();
    engine.tick(0.01);
    engine.request_inspection(vec![b.clone()]).unwrap();
    engine.tick(0.01);
    let samples = engine.request_inspection(vec![]).unwrap();
    assert_eq!(samples.blocks.len(), 2);
    assert_eq!(engine.request_inspection(vec![a]).unwrap().blocks.len(), 1);
    assert_eq!(engine.request_inspection(vec![b]).unwrap().blocks.len(), 1);
}
#[tokio::test]
async fn native_transport_exposes_compact_state_and_separate_inspection() {
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
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/state/compact")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
            .unwrap();
    assert!(json["snapshot"]["fixtures"]
        .as_array()
        .unwrap()
        .iter()
        .all(|f| f["nodes"].as_object().unwrap().is_empty()));
    let root = runtime.controller.current().project.content["fixture-1"][0]
        .as_ref()
        .unwrap()
        .root
        .id;
    for _ in 0..2 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/inspection")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({"targets":[target(root)]}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        tokio::time::sleep(std::time::Duration::from_millis(40)).await;
    }
    assert_eq!(runtime.controller.current().snapshot.revision, 0);
    assert!(!runtime
        .controller
        .inspection(vec![target(root)])
        .await
        .unwrap()
        .blocks
        .is_empty());
}

#[test]
fn bypass_and_director_samples_keep_their_real_targets() {
    let mut p = Project::demo();
    let mut stack = p.content["fixture-1"][0].clone().unwrap();
    stack.root.bypassed = true;
    let id = stack.root.id;
    p.director[0] = Some(stack);
    let mut engine = Engine::new(p).unwrap();
    let t = DebugFocus {
        fixture: "__director".into(),
        layer: 0,
        node: id,
    };
    engine.request_inspection(vec![t.clone()]).unwrap();
    engine.tick(0.01);
    let frame = engine.request_inspection(vec![]).unwrap();
    let b = &frame.blocks[0];
    assert_eq!(b.target, t);
    assert!(b.bypassed);
    assert_eq!(b.incoming.attributes, b.outgoing.attributes);
}
