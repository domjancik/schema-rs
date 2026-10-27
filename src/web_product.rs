use crate::server::AppState;
use axum::{http::header, routing::get, Router};
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/app.webmanifest",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "application/manifest+json")],
                    include_str!("../web/app.webmanifest"),
                )
            }),
        )
        .route(
            "/install-ui.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript")],
                    include_str!("../web/install-ui.js"),
                )
            }),
        )
        .route(
            "/mobile-worker.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript")],
                    include_str!("../web/mobile-worker.js"),
                )
            }),
        )
        .route(
            "/offline.html",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/html")],
                    include_str!("../web/offline.html"),
                )
            }),
        )
        .route(
            "/app-icon-192.png",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "image/png")],
                    include_bytes!("../web/app-icon-192.png").as_slice(),
                )
            }),
        )
        .route(
            "/app-icon-512.png",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "image/png")],
                    include_bytes!("../web/app-icon-512.png").as_slice(),
                )
            }),
        )
}
