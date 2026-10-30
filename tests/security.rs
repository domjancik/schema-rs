use schema_rs::model::Project;
#[test]
fn scene_labels_cannot_inject_terminal_control_sequences() {
    let mut p = Project::demo();
    p.fixtures[0].name = "bad\u{1b}[2J".into();
    assert!(p.validate().is_err());
}

#[tokio::test]
async fn four_digit_api_keys_throttle_guesses_by_real_peer() {
    use axum::{
        body::Body,
        extract::ConnectInfo,
        http::{Request, StatusCode},
    };
    use schema_rs::server::{router, AppState, Runtime};
    use tower::ServiceExt;
    let runtime = Runtime::start(Project::demo()).unwrap();
    let app = router(AppState::new(
        runtime.controller.clone(),
        std::env::temp_dir(),
        Some("1357".into()),
    ));
    let request = |pin: &str, peer: &str| {
        Request::builder()
            .uri("/api/state")
            .header("x-schema-token", pin)
            .header("x-forwarded-for", "10.42.0.99")
            .extension(ConnectInfo(peer.parse::<std::net::SocketAddr>().unwrap()))
            .body(Body::empty())
            .unwrap()
    };
    for _ in 0..5 {
        assert_eq!(
            app.clone()
                .oneshot(request("0000", "10.42.0.20:1234"))
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    let locked = app
        .clone()
        .oneshot(request("1357", "10.42.0.20:9999"))
        .await
        .unwrap();
    assert_eq!(locked.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(locked.headers()["retry-after"], "60");
    assert_eq!(
        app.oneshot(request("1357", "10.42.0.21:1234"))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
}

#[test]
fn lan_access_supports_explicit_four_digit_pins_and_existing_long_tokens() {
    use schema_rs::server::valid_lan_token;
    for token in ["0000", "1357", "existing-long-token"] {
        assert!(valid_lan_token(token));
    }
    for token in ["", "123", "12345", "abcd", "１２３４"] {
        assert!(!valid_lan_token(token));
    }
}
