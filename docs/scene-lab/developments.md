# Implementation developments for scene lab

## Baseline: 9427a13

- Studio and schémaboy now have a local fullscreen toggle. Native browser enter/exit and phone-width touch were verified. Unsupported browsers hide the control; Escape synchronizes its state.
- Add buttons occupy the top interaction plane above adjacent blocks and fold controls (`cd74310`).
- Both changes are active in the Windows frontend at port 8788 and the Orange Pi embedded UI. Pi activation preserved authored scene and undo/redo; OLA was not restarted.
- Named project storage, native catalog, cyclic daily dimming, custom DMX models, universe patching and durable history are described in the main docs. Read compatibility limits before designing examples.

No implementation changes are requested from the scene lab. Report bugs and additions with stable SL IDs and reproduction evidence.

## Product and phone release: 278832c

- `2b0883a`: one-viewport schémaboy, single header, secondary actions in an overflow menu. Verified at 320×568, 375×667, 390×844 and 844×390 with visible scene, encoder, A/B, Hold and Save; menu/help remains usable. Fullscreen still passes mouse and phone-width touch checks.
- `bbc8f28`: manifest/icons, browser-provided install handoff and platform/context-specific help. A `/mobile` worker caches only a disconnected navigation page; API state and commands stay online and uncached. Trusted HTTPS is needed for full PWA installation from a remote phone; current plain HTTP Pi addresses can offer browser-dependent home-screen shortcuts.
- `278832c`: `/welcome` product page, availability-gated real ZIP downloads and a `desktop` command. Desktop defaults to loopback port8797 and persistent app-data storage. It uses the system browser, not a native window wrapper. Normal startup disarms output.
- Active on Windows frontend port8788 (local and existing LAN address). Existing Windows engine process, scene and history were preserved. Active on the Pi native/portal frontend, with exact project/undo/redo equality at revision3 after restart. OLA was not restarted; no RTC files/time or AP/firewall settings changed.
- Windows archive metadata: `0.1.0-preview`, source `278832cb5d36`, 4,391,779 bytes, SHA-256 `5131a3a36c9648a7216a65a9891b2cb188ad7f1a453102744e6ab5de23ab1a13`. Actual extracted binary launch, embedded routes, output disabled and project/history recovery were tested. Browser download matches the archive hash. Public archive copies contain no live data or credentials.
- Validation: Rust fmt/full tests/clippy; JS syntax; package allowlist/checksum test; native desktop launch test; PWA routes/manifest and no-API-cache tests; actual browser navigation/install-event handoff/offline/reconnect; phone viewport and fullscreen checks. No physical phone OS installation or fixture output was claimed.

Suggested lab retests: author/edit/undo using the compact overflow menu and portrait/landscape controls; ensure the viewport stays usable when selecting different property types; check example instructions against desktop app-data storage and named-project behavior. The node evaluation and scene formats are unchanged. Keep findings SL-001 and SL-002 open as P3 follow-up requests.

## Final mobile polish: 8dbdbc49de76

- `9fe7a00`: stress-tested long scene/property/group labels. Explicit min-width/implicit grid constraints and ellipsis keep controls visible; tap opens the full editor. Disconnected fallback needs no network images and its worker cache is versioned.
- `8dbdbc4`: fixes lab finding SL-003, where global icon-nav dimensions collapsed Output tab labels. Tabs now have natural text widths, >=44px targets and wrapping. RED/GREEN browser regression passes three phone/orientation sizes. Lab status remains awaiting independent verification.
- Both fixes are active on the existing local/LAN frontend and the Pi. Final Pi activation verified exact project/undo/redo equality at revision3 and unchanged OLA PID. No AP, firewall or RTC changes.
- Final Windows release: source `8dbdbc49de76`, 4,392,087 bytes, SHA-256 `1e811df76109901cf1df414f273506ba2cc6d08c31098c63cf14595dd98afb23`. The extracted archive and actual live download were retested successfully, including output-disabled launch and authored/history recovery.
- [Focused lab retest](mobile-retest-278832c.md) verified numeric, enum, vector and visual-curve edits plus exact undo restoration; it found SL-003 and no other blocking viewport/menu issue. Its source baseline predates the final fixes.

## Restore useful top-panel controls: bb1222f

The human requested restoration of direct Studio return and the useful top panel. **Studio ↗** is now always visible; Output/DMX, Components, theme, connection, help and fullscreen are direct buttons again. Small portrait screens use two compact rows, with single-row tools on landscape/desktop; About/install remain secondary. Avoid moving these primary tools into overflow when refining the viewport.

Verified direct navigation/hit targets at five sizes, existing single-viewport/long-label/Output-tab/PWA flows, fullscreen, Rust fmt/full tests/clippy and app JS syntax. The rebuilt extracted Windows archive passed the direct-toolbar browser test and project/history recovery; its live download hash matches. Pi release `bb1222fd4f64` is active with exact project/undo/redo preserved at revision3, output off, and OLA not restarted. Existing local/LAN frontend serves the source update without restarting its engine. No RTC/AP/firewall changes.

This supersedes the earlier menu-first navigation guidance. Current Carpushit data work remains a separate scene-lab task; its new authored files are not included in this toolbar commit.

## GPU scene view: 337234a2dd36

Open Scene preview in Studio, GPU in schémaboy, or `/preview?scene=carpushit`. Implemented generic WebGPURenderer/WebGL2 fallback, actual point/spot/area lights, depth-aware volumetric pass, SSGI and temporal AA. Self-contained GLB scenery is local to the view. Phone quality defaults Low12 lights; Balanced40 renders all38Carpushitcircuits. Captured DMX controls brightness/ceilings/dimming/blackout; published positions/rotations drive motion. No UI graph evaluation or CUDA simulation.

The source Carpushit assembly is bundled as a read-only reference; selection changes no engine state. Explicit reviewed project loading is one revision-checked ReplaceProject with output disabled. Its full-look template excludes the separate authoring workspace’s resource-recalling director. Generic geometry/light code contains no Carpushit-specific mesh logic. Preset strips use rectangular area approximations and visible UV proxies. Source-asset provenance is distinct from code MIT.

Verified nativeChromeWebGPU and WebGL2 fallback, all38lights, generic emitters, import/recovery, limitedfloat fallback, contextloss reopening, close/reopen lifecycle, and editor entrypoints. Full Rust tests/fmt/clippy and JS checks passed. The scene lab’s [independent alignment check](gpu-preview-check-2026-10-28.md) found no blocking mapping issue. [Guide](../gpu-preview.md), [appearance](../gpu-preview-carpushit.png), [release evidence](../gpu-preview-verification.json).

Active local/LAN8788, Pi native/portal, and extracted/smoke-tested Windows archive. Local native engine was not restarted; its exact project/history were unchanged during frontend activation. Pi337234a2dd36 preserved exact project/undo/redo atrevision3 with outputoff; OLA was not restarted. No RTC/AP/firewall changes. Archive SHA256 `debada14f8e2bf4dfe72184d43b49c202f88ab5e06e016231a7a2d43b11ccdd5`, 6,507,769 bytes. Native graphics work runs on the client, not the Pi.

Carpushit’s authored examples/native cue data and SL-004 preset-library/cue-editor request remain a distinct scope. The GPU preview does not implement a new cue timeline or claim physical lighting calibration.

## Automatic controller updates: 4eae03e, 3ae2c94 and follow-up hardening

The Orange Pi now receives signed ready builds from the running Windows update host at `http://127.0.0.1:8890/`. Committed engine/UI inputs are snapshotted and pass fmt/full tests/clippy/JS checks, ARM64 + Windows compilation and native Chrome WebGPU smoke before publication. The dashboard has automatic/paused policies, offline queues, build/push logs, explicit retry and trusted rollback with a pin. Failed builds never become ready; failed deployments are quarantined. No project/node formats changed.

Engine commit `4eae03e` adds the startup update barrier: evaluation, actions, clock and edits wait while the root installer verifies restored authored state/history, then resume. The installer journals/checkpoints, switches immutable software/downloads and recovers failed/interrupted switches via systemd. Later hardening also checks session inputs/node states/plugins and clock/control fields for new deployments. Lab retest focus: saved Repeat/recorded-state examples should resume through future updates with authored history intact; use isolated engines as before.

Activated on the Pi through the actual automatic pipeline: release `a87a8e1e677c-9a31fa092371-1` (engine source `a87a8e1e677c`; later commits harden only the host/helper/tests). Exact project/undo/redo preserved at revision3, three undo entries, no redo, outputoff; OLA PID1038 unchanged. The clock resumed and barrier cleared. Host restart and already-current manual push were verified; repeated polls do not enqueue duplicate updates. Local/LAN Studio engine was not restarted. AP/RTC/firewall were not changed. Existing external RTC issue remains outside this update scope.

[Operator workflow](../update-server.md) and [sanitized live verification](../update-server-verification.json). Six host tests, four isolated Linux transaction tests, five real OpenSSL ingestion tests, responsive dashboard controls and all committed-build gates passed. The initial disk-full build failed safely without publication; only regenerable Schéma compiler caches were removed before a successful retry. Separate Carpushit preset/cue UI request SL-004 remains pending.

## Workspace panels and dynamic layers: 2e6bc06 / cc25c24

Studio now has named browser workspaces, editor-type selectors, split/join, pointer/keyboard resize, header drag docking/exchange, maximize/restore and layout undo. Stack instances retain independent fixture/layer/selection/pan/zoom/folding/driver contexts. Scene/telemetry views consume live snapshots. Existing fixture setup and full unpinned inspector remain available. [Guide](../workspaces.md).

Fixtures and the Director support1–32 layers (legacy defaults stay3). Enable is one undoable engine command. Disabled stacks retain authored/runtime state and skip evaluation. Extra layers/mutes persist in named/project/session saves and ScenicScript. Layer1 remains output; no additive-mixing change. Content application grows destinations within the32 bound. schémaboy supports actual cycling and Hold/Help layer controls; TUI adds numeric/bracket navigation.

A synthetic regression caught non-exact JSON floating-point parsing in saved audio/runtime values. Exact float round trips now pass. The local saved revision199 checkpoint restored project/undo/redo/inputs/node state/plugins/clock exactly before barrier release: six undo entries, outputoff. Local/LAN8788 is active. Workspace navigation leaves revision/history unchanged. Rust fmt/full tests/clippy, JS syntax, workspace model and Chrome split/resize/exchange/dock/maximize/independent-view/reload checks pass; isolated layer mute/add/undo checks pass. Pi rollout follows the committed snapshot; confirm its receipt before claiming activation there.

The Carpushit revision2 /33-fixture single-roof-UV handoff remains a separate bundled-reference refresh. No lab-authored example or live project was replaced. The newer human priority order is tracked in plans/work-queue.md; camera navigation is next.

## Camera navigation and embedded preview lifecycle

Camera navigation now provides desktop/touch orbit/pan/zoom, keyboard movement, viewpoint presets and framing. It persists per preview/panel identity. Drag/multi-touch selection guards prevent accidental fixture selection. Maximize keeps GPU instances mounted; Ctrl+Space/Escape works inside embedded views and hidden previews pause rendering. Same-origin preview framing is now allowed explicitly, while external embedding remains blocked. Guide: [scene camera](../scene-camera.md).

Verified pure math and native framing tests, mouse/keyboard/two-finger gestures, independent GPU cameras, saved poses, maximize/restore/pause, and existing Chrome WebGPU/WebGL2 preview/import/recovery suite. Full Rust fmt/tests/clippy and client syntax pass. Local/LAN frontend active with unchanged authored scene/history. The previous panel/layer release7cad54ff9a24-71f4412ef7df-2 is confirmed active on the Pi; its automatic transaction succeeded and retained scene/history/OLA. The next committed camera snapshot will deploy through the same updater. No lab-authored data or live project replacement.
