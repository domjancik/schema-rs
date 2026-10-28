# Scene camera navigation

Every3D preview has an independent Camera control. On phones it starts collapsed to preserve the viewport; expand Camera to choose Orbit or Pan, use directional/zoom buttons, frame the selected fixture, fit the whole scene, or choose Top/Front/Right/Iso viewpoints. These are perspective viewpoints with a consistent world-up axis.

- Mouse: drag to orbit; choose Pan for left-drag panning. Existing right/middle-button and wheel controls remain available.
- Touch: one finger follows the Orbit/Pan mode; two fingers pan and pinch to zoom. Multi-touch and moved drags never select fixtures accidentally.
- Focus the canvas for keyboard navigation: arrows orbit (Shift+arrows pan), WASD pan, +/− zoom, F fits, Home selects Iso, and 1/3/7 select Front/Right/Top.
- Frame uses the selected fixture's current displayed position. A camera action changes only the view, never layout, authored parameters or output.

Camera pose and mode save locally per preview/panel identity and restore on reload or graphics reinitialization. Two3D panels can retain different cameras. Ctrl+Space maximizes a preview even when keyboard focus is inside its embedded view; Escape restores it. Maximization keeps the view mounted and pauses other hidden GPU previews rather than recreating renderers.

Only the preview route allows same-origin framing; the rest of Studio retains its framing restriction. External sites cannot embed the preview. Same-origin messages for maximize/visibility are checked against the actual hosted iframe window.

Navigation uses pointer/touch input and requires no device-camera or motion-sensor permission. It does not implement a fly-through physics controller or change the render quality/lighting model.

Verification: pure navigation math/bounds/preset tests; desktop mouse/keyboard and real two-finger Chrome touch tests; persisted poses; two independent GPU panels; embedded maximize/restore and hidden-view pause; native same-origin framing policy; full Rust tests/fmt/clippy; existing Chrome WebGPU/WebGL2 preview/import/recovery tests. Browser tests intercept engine commands and use a frozen publication.


## Lightweight Studio and schémaboy 3D views

The regular Studio Scene panel now has a 3D option alongside XZ/XY/YZ. schémaboy's 3D button uses the same lightweight orthographic camera widget. The GPU preview retains its existing perspective camera and lighting renderer. No additional GPU renderer is created for the schematic views.

Drag to orbit; Shift/right/middle drag pans. Choose Pan in Camera for one-finger panning. Two fingers pan and pinch to zoom; mouse wheel/trackpad zooms. Camera contains Fit, Frame, Iso/Front/Right/Top, directional buttons and zoom controls. Controls use the browser's top layer so short phone screens and panel stacking cannot cover the menu; Close or Escape dismisses it. Browsers without popover support use a modal dialog fallback.

Focus the scene for arrow-key orbit, Shift+arrow pan, +/−zoom, F fit, Home Iso and 1/3/7 Front/Right/Top. Repeated keys retain camera focus. In schémaboy, the existing Pan-mode D-pad pans (Hold zooms), and the wheel orbits (Hold tilts). The existing zoom/Fit buttons work on the camera. Selecting a block or fixture in other modes retains its existing controls.

Fit includes authored fixture positions and currently published animated positions. Frame centers the selected fixture's stable layout position. Layout dots remain selectable while separate live dots show animation. Tap selects; moved drags and multi-touch never select. Pointer cancellation restores the camera at gesture start. Adaptive margins and a reserved small-phone canvas keep the 3D markers out of the fixture/footer controls.

Mobile poses persist locally per origin; Studio poses persist independently per panel in workspace preferences. Camera saves update those preferences without remounting neighboring views/GPU instances, changing authored state or adding undo entries. Switching schematic modes retains the 3D pose. No device-camera or orientation permission is requested. This is an orbiting orthographic schematic, not a fly-through/perspective lighting renderer; choose the GPU preview for that rendering view.

Additional verification: math/fit/adaptive-margin and workspace validation tests; real Chrome desktop drag/keyboard and touch pan/pinch, selection suppression/taps, cancellation, small-phone hit targets, persisted mobile/workspace poses, duplicate independent Scene panels, and a neighboring GPU iframe surviving navigation. Existing GPU mouse/touch camera regressions remain passing. Camera tests freeze output-disabled snapshots and intercept all engine commands.
