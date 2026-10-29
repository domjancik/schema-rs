# Scene camera navigation

Every3D preview has an independent Camera control. On phones it starts collapsed to preserve the viewport; expand Camera to choose Orbit or Pan, use directional/zoom buttons, frame the selected fixture, fit the whole scene, or choose Top/Front/Right/Iso viewpoints. Iso selects a true orthographic isometric view, with equal axis foreshortening; the rendered preview also allows returning to Perspective in Camera → Projection.

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


## Isometric and rotation controls

Scene viewport controls expose **Iso** directly alongside3D/XZ/XY/YZ. Iso uses45° azimuth and35.264° elevation, with orthographic projection. Drag to rotate freely; **↶ / ↷** turn45° around world-up while preserving target, zoom and elevation. These controls are also directly available on schémaboy and the rendered3D preview. The preview rebuilds camera-dependent rendering passes when changing projection and keeps lighting/volumetrics/SSGI enabled according to the existing settings.

Each viewport saves its own pose and projection locally. Split views can show different angles; navigating does not recreate neighboring GPU views or author a scene command. Touch orbit, two-finger pan/pinch and existing keyboard shortcuts remain available. Square/fractal block canvases remain planar.


## Embedded fixture stacks

Scene/Fixture setup viewport controls include **Stacks off / Selected stacks / All stacks** and a stack-size slider. Each displayed stack uses that fixture's layout position as its anchor. Its root sits above the selection dot; evaluated animation stays separate. The stack stays screen-facing while the scene camera rotates, and scales with camera zoom. This applies to the lightweight planar and isometric/3D Scene viewports, not the GPU lighting preview.

Each fixture has a local layer picker. Click a block to select its fixture/node in the shared inspector and edit its properties there. Plus buttons use the existing indexed creation picker; delete uses the existing block/branch chooser; fold affects that embedded tree locally. The grip opens the Move block/branch chooser; direct pointer drag between embedded stacks is not implemented. The regular Stack editor remains available for ghost drag-and-drop, drivers and detailed layout controls. Empty layers offer Create stack.

Visibility and size are saved per viewport. Selected mode follows the current fixture selection, including a multiple selection made in Studio. Embedded selection then focuses one fixture. The overlay renders at most600 authored tree nodes per viewport; an explicit notice reports omitted stacks so dense projects can use Selected mode. Overlapping stacks retain their scene anchors; selecting a fixture raises its stack. Scene controls stay above overlays. These preferences and local folds do not alter project/undo history; authored edits use the same expected-revision engine commands as the regular editor.
