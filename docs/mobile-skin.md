# Faceted schémaboy shell

The shell uses CSS polygon facets and bevels. Existing controls, Studio navigation and the single-viewport mobile layout retain their positions and touch areas. No GPU scene renderer is started for the skin.

Open **More options (•••) → Shell lighting**:

- Scene color works immediately without permissions. It averages evaluated fixture RGB, alpha and fixture dimmer; scalar-only fixtures contribute gray. Blackout removes the scene light contribution. Empty scenes use the Schéma mint accent.
- **Enable phone tilt** requests orientation access where the browser requires it, then changes the apparent lighting across the facets. Controls stay stationary. Reduced motion prevents tilt activation.
- **Enable camera lighting** requests only the rear camera, without audio. Coarse average tint and image brightness distribution influence the shell. This gives an ambient impression, not calibrated physical relighting or environment reconstruction.

Camera and motion are opt-in for each visit. Disable either with the same button. Both stop when the page is hidden or left; they require explicit re-enabling on return. Closing the settings keeps an enabled sensor running so the shell remains lit during use. The browser's camera indicator remains visible while capturing.

Camera/motion require a secure context and browser/device support. Localhost development qualifies; the existing plain HTTP phone LAN URL does not. Settings explain this without requesting permission. Use a trusted HTTPS origin to enable those features on a phone. This change does not install certificates or alter network access.

## Resource and privacy boundaries

Scene averaging runs at most 5 Hz and samples at most 64 pixels per fixture without copying the frame. Tilt repaint is capped at approximately 12 Hz and smoothed. Camera capture requests 160×120 at 2 fps (maximum 5); actual browser constraints may differ. Analysis uses a 16×12 canvas twice per second. No new animation loop, external dependency or network request is used. Hidden pages stop the camera and remove the orientation listener; a late permission grant is stopped immediately. Canvas samples and video references clear on shutdown.

Images and sensor readings stay in memory in the browser. They are never recorded, uploaded, written to project files or submitted as engine commands. The skin has no undo entries. Scene color is an aesthetic estimate from evaluated banks, before the daily output timeline and DMX model mapping; it is not a light meter or DMX parity preview.

Verified with pure lighting math tests and Chrome tests for explicit opt-in, tilt response, reduced motion, camera cancellation/denial, insecure-context guidance, hidden-page cleanup, no automatic restart, zero engine commands and 320px/390px portrait plus landscape viewport fit. Camera streams and orientation permissions are mocked; physical phone/browser camera behavior remains device-dependent.
