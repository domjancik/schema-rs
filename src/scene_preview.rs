use crate::server::AppState;
use axum::{http::header, routing::get, Router};
pub(crate) fn routes() -> Router<AppState> {
    let mut router = Router::new();
    router = router.route(
        "/scene-camera.mjs",
        get(|| async {
            (
                [(header::CONTENT_TYPE, "text/javascript")],
                include_bytes!("../web/scene-camera.mjs").as_slice(),
            )
        }),
    );
    router = router.route(
        "/preview",
        get(|| async {
            (
                [(header::CONTENT_TYPE, "text/html")],
                include_bytes!("../web/preview.html").as_slice(),
            )
        }),
    );
    router = router.route(
        "/scene-preview.css",
        get(|| async {
            (
                [(header::CONTENT_TYPE, "text/css")],
                include_bytes!("../web/scene-preview.css").as_slice(),
            )
        }),
    );
    router = router.route(
        "/scene-preview-core.mjs",
        get(|| async {
            (
                [(header::CONTENT_TYPE, "text/javascript")],
                include_bytes!("../web/scene-preview-core.mjs").as_slice(),
            )
        }),
    );
    router = router.route(
        "/scene-preview-renderer.mjs",
        get(|| async {
            (
                [(header::CONTENT_TYPE, "text/javascript")],
                include_bytes!("../web/scene-preview-renderer.mjs").as_slice(),
            )
        }),
    );
    router = router.route(
        "/scene-preview-ui.mjs",
        get(|| async {
            (
                [(header::CONTENT_TYPE, "text/javascript")],
                include_bytes!("../web/scene-preview-ui.mjs").as_slice(),
            )
        }),
    );
    router = router.route(
        "/scene-preview-launcher.js",
        get(|| async {
            (
                [(header::CONTENT_TYPE, "text/javascript")],
                include_bytes!("../web/scene-preview-launcher.js").as_slice(),
            )
        }),
    );
    router = router.route(
        "/scene-preview-page.js",
        get(|| async {
            (
                [(header::CONTENT_TYPE, "text/javascript")],
                include_bytes!("../web/scene-preview-page.js").as_slice(),
            )
        }),
    );
    router = router.route(
        "/vendor/scene-three.mjs",
        get(|| async {
            (
                [(header::CONTENT_TYPE, "text/javascript")],
                include_bytes!("../web/vendor/scene-three.mjs").as_slice(),
            )
        }),
    );
    router = router.route(
        "/vendor/scene-three.mjs.LEGAL.txt",
        get(|| async {
            (
                [(header::CONTENT_TYPE, "text/plain")],
                include_bytes!("../web/vendor/scene-three.mjs.LEGAL.txt").as_slice(),
            )
        }),
    );
    router = router.route(
        "/vendor/THREE-LICENSE.txt",
        get(|| async {
            (
                [(header::CONTENT_TYPE, "text/plain")],
                include_bytes!("../web/vendor/THREE-LICENSE.txt").as_slice(),
            )
        }),
    );
    router = router.route(
        "/vendor/VERSIONS.json",
        get(|| async {
            (
                [(header::CONTENT_TYPE, "application/json")],
                include_bytes!("../web/vendor/VERSIONS.json").as_slice(),
            )
        }),
    );
    router = router.route(
        "/scene-assets/carpushit.glb",
        get(|| async {
            (
                [(header::CONTENT_TYPE, "model/gltf-binary")],
                include_bytes!("../web/scene-assets/carpushit.glb").as_slice(),
            )
        }),
    );
    router = router.route(
        "/scene-assets/carpushit.project.json",
        get(|| async {
            (
                [(header::CONTENT_TYPE, "application/json")],
                include_bytes!("../web/scene-assets/carpushit.project.json").as_slice(),
            )
        }),
    );
    router = router.route(
        "/scene-assets/carpushit-reference.json",
        get(|| async {
            (
                [(header::CONTENT_TYPE, "application/json")],
                include_bytes!("../web/scene-assets/carpushit-reference.json").as_slice(),
            )
        }),
    );
    router = router.route(
        "/scene-assets/carpushit-lights.json",
        get(|| async {
            (
                [(header::CONTENT_TYPE, "application/json")],
                include_bytes!("../web/scene-assets/carpushit-lights.json").as_slice(),
            )
        }),
    );
    router = router.route(
        "/scene-assets/carpushit-provenance.json",
        get(|| async {
            (
                [(header::CONTENT_TYPE, "application/json")],
                include_bytes!("../web/scene-assets/carpushit-provenance.json").as_slice(),
            )
        }),
    );
    router
}
