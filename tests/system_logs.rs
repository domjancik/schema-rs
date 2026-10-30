use schema_rs::{
    logs::{Journal, Query},
    model::Snapshot,
};

#[test]
fn logs_record_transitions_without_frame_spam_and_do_not_persist() {
    let mut log = Journal::new();
    let mut snapshot = Snapshot {
        output_status: "Disabled".into(),
        ..Snapshot::default()
    };
    log.observe(&snapshot);
    let baseline = log.read(Query::default()).entries.len();
    for _ in 0..100 {
        log.observe(&snapshot);
    }
    assert_eq!(log.read(Query::default()).entries.len(), baseline);
    snapshot.diagnostics = vec!["Missing fixture".into()];
    log.observe(&snapshot);
    log.observe(&snapshot);
    let result = log.read(Query::default());
    assert_eq!(
        result
            .entries
            .iter()
            .filter(|e| e.message == "Missing fixture")
            .count(),
        1
    );
    assert!(result.entries.iter().any(|e| e.level == "warning"));
    snapshot.diagnostics.clear();
    snapshot.blackout = true;
    log.observe(&snapshot);
    assert!(log
        .read(Query::default())
        .entries
        .iter()
        .any(|e| e.message.contains("cleared")));
    assert!(log
        .read(Query::default())
        .entries
        .iter()
        .any(|e| e.message.contains("Blackout enabled")));
}

#[test]
fn logs_are_bounded_cursor_paginated_and_unicode_safe() {
    let mut log = Journal::new();
    let mut snapshot = Snapshot::default();
    for i in 0..700 {
        snapshot.revision = i;
        log.observe(&snapshot);
    }
    let first = log.read(Query {
        after: 1,
        limit: 50,
    });
    assert!(first.truncated);
    assert_eq!(first.entries.len(), 50);
    let next = log.read(Query {
        after: first.next,
        limit: 200,
    });
    assert!(next.entries[0].id > first.next);
    assert!(
        log.read(Query {
            after: 0,
            limit: usize::MAX
        })
        .entries
        .len()
            <= 200
    );
    snapshot.diagnostics = vec!["é".repeat(10000)];
    log.observe(&snapshot);
    let end = log.read(Query {
        after: next.latest - 1,
        limit: 200,
    });
    assert!(end
        .entries
        .iter()
        .all(|e| e.message.chars().count() <= 1024));
    assert!(
        log.read(Query {
            after: u64::MAX,
            limit: 200
        })
        .reset
    );
}

#[test]
fn session_log_reads_are_ephemeral_and_not_part_of_state_or_history() {
    let mut session =
        schema_rs::runtime::EngineSession::new(schema_rs::model::Project::demo()).unwrap();
    session.advance(0.).unwrap();
    let before = session.encode_session().unwrap();
    let history = session.history();
    let page = session.logs(Query::default());
    assert!(!page.entries.is_empty());
    assert!(
        page.entries
            .iter()
            .all(|entry| !entry.message.trim().is_empty()),
        "empty output status must not become a blank event"
    );
    assert_eq!(session.encode_session().unwrap(), before);
    assert_eq!(session.history(), history);
    assert!(serde_json::to_value(session.publication())
        .unwrap()
        .get("logs")
        .is_none());
    let restored = schema_rs::runtime::EngineSession::decode_session(&before).unwrap();
    assert_eq!(restored.logs(Query::default()).entries.len(), 1);
}

#[tokio::test]
async fn native_logs_api_is_authenticated_bounded_and_read_only() {
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use schema_rs::server::{router, AppState, Runtime};
    use tower::ServiceExt;
    let runtime = Runtime::start(schema_rs::model::Project::demo()).unwrap();
    let token = "log-test-token-0123456789";
    let app = router(AppState::new(
        runtime.controller.clone(),
        std::env::temp_dir(),
        Some(token.into()),
    ));
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/logs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/logs")
                .header("x-schema-token", token)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"after":0,"limit":1}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let page: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 65536).await.unwrap()).unwrap();
    assert_eq!(page["entries"].as_array().unwrap().len(), 1);
    assert!(!page.to_string().contains(token));
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/logs")
                .header("x-schema-token", token)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"after":-1}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(runtime.controller.current().snapshot.revision, 0);
    assert!(!runtime.controller.current().project.output.enabled);
}
