# Scene camera navigation

Every3D preview has an independent Camera control. On phones it starts collapsed to preserve the viewport; expand Camera to choose Orbit or Pan, use directional/zoom buttons, frame the selected fixture, fit the whole scene, or choose Top/Front/Right/Iso viewpoints. These are perspective viewpoints with a consistent world-up axis.

- Mouse: drag to orbit; choose Pan for left-drag panning. Existing right/middle-button and wheel controls remain available.
- Touch: one finger follows the Orbit/Pan mode; two fingers pan and pinch to zoom. Multi-touch and moved drags never select fixtures accidentally.
- Focus the canvas for keyboard navigation: arrows orbit (Shift+arrows pan), WASD pan, +/− zoom, F fits, Home selects Iso, and1/3/7 select Front/Right/Top.
- Frame uses the selected fixture's current displayed position. A camera action changes only the view, never layout, authored parameters or output.

Camera pose and mode save locally per preview/panel identity and restore on reload or graphics reinitialization. Two3D panels can retain different cameras. Ctrl+Space maximizes a preview even when keyboard focus is inside its embedded view; Escape restores it. Maximization keeps the view mounted and pauses other hidden GPU previews rather than recreating renderers.

Only the preview route allows same-origin framing; the rest of Studio retains its framing restriction. External sites cannot embed the preview. Same-origin messages for maximize/visibility are checked against the actual hosted iframe window.

Navigation uses pointer/touch input and requires no device-camera or motion-sensor permission. It does not implement a fly-through physics controller or change the render quality/lighting model.

Verification: pure navigation math/bounds/preset tests; desktop mouse/keyboard and real two-finger Chrome touch tests; persisted poses; two independent GPU panels; embedded maximize/restore and hidden-view pause; native same-origin framing policy; full Rust tests/fmt/clippy; existing Chrome WebGPU/WebGL2 preview/import/recovery tests. Browser tests intercept engine commands and use a frozen publication.
