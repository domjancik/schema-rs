use crate::server::AppState;
use axum::{http::header, routing::get, Router};
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/app",
            get(|| async { axum::response::Html(include_str!("../web/index.html")) }),
        )
        .route(
            "/welcome",
            get(|| async { axum::response::Html(include_str!("../web/welcome.html")) }),
        )
        .route(
            "/landing.css",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/css")],
                    include_str!("../web/landing.css"),
                )
            }),
        )
        .route(
            "/landing.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript")],
                    include_str!("../web/landing.js"),
                )
            }),
        )
        .route(
            "/servers-core.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript")],
                    include_str!("../web/servers-core.js"),
                )
            }),
        )
        .route(
            "/servers-ui.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript")],
                    include_str!("../web/servers-ui.js"),
                )
            }),
        )
        .route(
            "/app-updates.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript")],
                    include_str!("../web/app-updates.js"),
                )
            }),
        )
        .route(
            "/servers.css",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/css")],
                    include_str!("../web/servers.css"),
                )
            }),
        )
        .route(
            "/vendor/qrcode.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript")],
                    include_str!("../web/vendor/qrcode.js"),
                )
            }),
        )
        .route(
            "/vendor/jsQR.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript")],
                    include_str!("../web/vendor/jsQR.js"),
                )
            }),
        )
        .route(
            "/vendor/QR-LICENSES.txt",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/plain")],
                    include_str!("../web/vendor/QR-LICENSES.txt"),
                )
            }),
        )
        .route(
            "/output-panel.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript")],
                    include_str!("../web/output-panel.js"),
                )
            }),
        )
        .route(
            "/output-panel.css",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/css")],
                    include_str!("../web/output-panel.css"),
                )
            }),
        )
        .route(
            "/studio-shell.js",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/javascript")],
                    include_str!("../web/studio-shell.js"),
                )
            }),
        )
        .route(
            "/studio-shell.css",
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "text/css")],
                    include_str!("../web/studio-shell.css"),
                )
            }),
        )
        .route("/downloads/latest.json", get(release_info))
        .route("/downloads/Schema-s-Windows-x64.zip", get(download_zip))
        .route("/downloads/SHA256SUMS.txt", get(download_checksums))
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

const ARCHIVE: &str = "Schema-s-Windows-x64.zip";
#[derive(serde::Serialize, serde::Deserialize)]
struct Release {
    available: bool,
    version: String,
    commit: String,
    sha256: String,
    bytes: u64,
    filename: String,
}
fn release(root: &std::path::Path) -> Option<Release> {
    let folder = root.join("downloads");
    let meta = std::fs::metadata(folder.join("latest.json")).ok()?;
    if !meta.is_file() || meta.len() > 8192 {
        return None;
    }
    let release: Release =
        serde_json::from_slice(&std::fs::read(folder.join("latest.json")).ok()?).ok()?;
    let zip = std::fs::metadata(folder.join(ARCHIVE)).ok()?;
    if !release.available
        || release.filename != ARCHIVE
        || release.sha256.len() != 64
        || !release.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        || !zip.is_file()
        || zip.len() != release.bytes
        || zip.len() > 128 * 1024 * 1024
    {
        return None;
    }
    Some(release)
}
async fn release_info(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> axum::Json<serde_json::Value> {
    let info = tokio::task::spawn_blocking(move || release(&state.root))
        .await
        .ok()
        .flatten();
    axum::Json(
        info.map(|release| serde_json::to_value(release).unwrap_or_default())
            .unwrap_or_else(|| serde_json::json!({"available":false})),
    )
}
async fn download_zip(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    let data = tokio::task::spawn_blocking(move || {
        release(&state.root)?;
        std::fs::read(state.root.join("downloads").join(ARCHIVE)).ok()
    })
    .await
    .ok()
    .flatten();
    match data {
        Some(data) => (
            [
                (header::CONTENT_TYPE, "application/zip"),
                (
                    header::CONTENT_DISPOSITION,
                    "attachment; filename=\"Schema-s-Windows-x64.zip\"",
                ),
            ],
            data,
        )
            .into_response(),
        None => axum::http::StatusCode::NOT_FOUND.into_response(),
    }
}
async fn download_checksums(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    let info = tokio::task::spawn_blocking(move || release(&state.root))
        .await
        .ok()
        .flatten();
    match info {
        Some(info) => (
            [(header::CONTENT_TYPE, "text/plain")],
            format!("{}  {}\n", info.sha256, ARCHIVE),
        )
            .into_response(),
        None => axum::http::StatusCode::NOT_FOUND.into_response(),
    }
}
