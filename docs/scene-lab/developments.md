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


## Faceted schémaboy shell — 6f41d63

The human explicitly requested this skin after pausing the larger queue; other queued work remains paused. CSS polygon housing/bevels retain the mobile viewport and control positions. Averaged evaluated fixture light includes RGB, alpha, fixture dimmer and scalar gray. Optional phone tilt and rear-camera ambient tint/direction are explicitly enabled in More options → Shell lighting. Camera sampling stays local (16×12 at2Hz), requests no microphone, stops on hidden/pagehide and never automatically resumes. Secure-context/support guidance is visible on HTTP LAN. This is approximate pre-output scene shading, not daily-timeline/DMX or physical-light parity. No project format, authored commands or history changes. Guide: [mobile skin](../mobile-skin.md).

Commit6f41d63 is active on local/LAN8788; a live read-only settings smoke check confirmed unchanged project/undo/redo, zero commands and no page errors. Pure math, explicit permission/denial/cancellation/reduced-motion/cleanup tests, existing320×568 through landscape viewport checks, mobile navigation math, full Rust tests, fmt, clippy and JS syntax passed. Sensors were mocked; physical phone camera validation remains device-dependent and requires HTTPS. Pi rollout is left to the existing committed-build updater and is not claimed complete here. Lab data remains untouched; retest any mobile scene's selection/controls and optional lighting appearance only.


## Public Vercel / Linux release — 90d5b61

https://schema-s.vercel.app is live on the dedicated Vercel Schema-s project; final production deployment dpl_49Z7547ox9HdjyWCFkk19WD5fBEg. Public Windows ZIP, Linux x64/ARM64 tarballs, SHA256SUMS, manifest and inspectable Pi-class/systemd installer are available. Runtime inputs are6f41d631342c (x64 clean4bedaa0 has identical src/web/Cargo/LICENSE inputs); release tooling/site is90d5b61. Public site connects users to their own engine via a validated origin; no scene, credentials or hardware engine is uploaded. A production test caught late saved-address initialization erasing early input; input/button readiness now gates submission and the public browser regression passes.

Final unauthenticated public downloads match the local manifest hashes for all3 platforms. The actual HTTPS bootstrap downloaded/verified/staged twice in Linux, preserving a private600 token and generating boot configuration. Installer tests cover tamper refusal, exact credential/session preservation, custom-service refusal and rollback after failed health. Extracted Linux x64 executed web assets, output-disabled startup, edit/undo and durable restart in an isolated project. Rust full tests/fmt/clippy, JS/Python/shell checks pass. ARM64 ELF/build provenance verified, but the Orange Pi SSH endpoint was unreachable, so no new board/USB smoke or real service install is claimed. Installer does not change RTC, AP, OLA or firewall; existing custom Orange Pi installations use their configured updater.

Docs: [public workflow](../publishing.md), [Linux guide](../linux.md). Lab-owned Carpushit data remains untouched and was not silently bundled into this release. SL-005 small-screen marker overlap is recorded as pre-existing; fixture-row selection remains the workaround. Relevant lab retests: public connection onboarding, Linux guide and any portable-scene documentation assumptions. Other queued UI work was not started during publication.


## Desktop/mobile schematic cameras — e744860

Regular Studio Scene panels now offer3D alongside XZ/XY/YZ; schémaboy's3D view uses the shared orthographic scene widget. Direct orbit/pan/wheel, touch pan+pinch, keyboard navigation, Fit/Frame/viewpoints and independent saved poses are available. Camera preferences update without rebuilding neighboring GPU views or touching engine history. Layout selection dots remain fixed while live animation is drawn separately. Top-layer camera controls include Close/Escape and a dialog fallback; adaptive small-phone margins reserve a camera area outside the footer. Guide: [scene cameras](../scene-camera.md). Existing GPU perspective preview/navigation remains available and its desktop/touch checks still pass.

Verified math/fit/bounds, real desktop/touch gestures, explicit pinch zoom+pan, taps versus moved drags, cancellation, repeated keyboard focus, authoring-key isolation, duplicate independent cameras, pose reloads, adjacent GPU iframe identity, main workspace/skin/mobile-viewport regressions and fallback controls at320×568. Full Rust tests/fmt/clippy and JS syntax pass. Live localhost8788 smoke confirmed project/undo/redo unchanged with zero commands/errors. The old LAN address192.168.1.106 is no longer assigned; the localhost frontend is active, but the configured LAN bind could not restart. Network/firewall scope was not expanded. Pi/public-download activation is not claimed; the existing private committed-build updater handles its normal ready-build workflow.

Lab retests:33-fixture Carpushit camera orbit/pan/pinch, selection dots during animation, Camera/Close controls at320×568 and390×844, and SL-005 footer hit targets. Our isolated8-fixture hit checks pass; confirm against the lab's denser scene before marking its finding resolved. Restart only your isolated Node frontend to register the new schematic-camera.js/css asset map. Lab-owned data and earlier uncommitted desktop chrome/driver refinements remain separate.


## Original top-right block chrome — 173d831

Port completed from schema-stack-lab e4d6ae8 and reviewed with Analyze Schéma visual stacking and interactions. Persistent fold → grab → delete controls are on each Studio block. Fold selects locally; leaves have no glyph. Drag shares the existing 5px planner, branch/block preference and Shift override; Escape cancels and successful drops suppress synthetic clicks for 250ms. Delete chooser/inspector/Delete/Backspace share native expected-revision handling, block-only promotion versus branch deletion, root ambiguity protection and surviving-parent selection. Coarse controls are 32 canvas pixels and scale with zoom; no physical 32px guarantee. Category icons are 14px and no longer overlap labels. Guide: [block chrome](../block-chrome.md).

The tests found and fixed driver-root deletion: native delete_node explicitly rejects driver roots. Clearing now uses set_driver(null); one-child promotion uses an immutable replace_layer candidate to retain child IDs. Nested driver contexts unwind to the nearest surviving context, and inactive panes render their actual driver trees. Each delete remains one undoable engine transaction; no schema/API change. Stale 409 rejection leaves authored content intact. Fold/drag preferences do not author scene changes.

Verified isolated output-disabled browser tests for all controls, thresholds, real grip branch/Shift movement, cancellation, parent selection, owned-driver removal, stale revision, inactive panes, driver ID preservation and full undo. Existing creation/indexed insertion/ghost drop, pure editing/layout 17 cases, layer workspaces, independent cameras, full Rust tests/fmt/clippy and JS syntax pass. Local8788 read-only smoke confirmed chooser/Cancel focus, zero commands and exact project/undo/redo preservation. No engine restart, LAN/firewall change or Pi/public-download activation claim. Lab-authored scene data remains untouched. Retest block controls in dense example stacks/driver panes; camera dense-hit SL-005 findings remain separate and open.


## Studio fold motion — 94d46cb

Local8788 is active; refresh Studio to load the new asset. Children tuck/shrink into their parent on fold and fan out with a short stagger and soft spring settle on unfold. The upper-right fold control stays fixed on screen; rapid toggles reverse from their current visual positions. Same-context telemetry/selection rerenders preserve the transition; authored revisions, context changes, hidden pages and reduced-motion changes clean up immediately. Reduced motion toggles directly. The transient effect is bounded to96 cards, and inert ghosts carry no model IDs or interactive controls.

Pure motion/anchor/budget tests and isolated browser tests cover rapid physical clicks, keyboard Space/Enter, context cancellation, cleanup and reduced motion. Existing indexed insertion/ghost-drop/undo, layers and independent Scene camera regressions pass. Full Rust tests, fmt, clippy and JS syntax pass. Local read-only fold/unfold smoke blocked non-GET API requests, observed zero authored commands/browser errors and compared project/undo/redo exactly. Only the frontend restarted; the engine continued running. Public/Pi builds are not claimed activated.

Received the lab chrome retest results; SL-005 dense camera selection and SL-006 stale fixture options remain separate open findings. Suggested fold retest: dense Repeat/Carpushit stacks, quick repeated folding and reduced motion. Preserve authored examples and use an isolated output-disabled engine.


## Fractal and square Stack layouts — d9079d2

Active on source-served local8788 after refresh. Stack panel header **Layout** exposes independent **Sibling layout: Expanded / Fractal** and **Block shape: Rectangle / Square** choices. Saved per panel/workspace, including duplicate Stack views and drivers. Default remains Expanded + Rectangle. Fractal siblings share the parent's width with scaled gaps, single-child chains retain size, and further forks shrink recursively. Square means each card's physical width equals its height; expanded wide combiners therefore become taller squares. Fractal + Square stays within root width. Zoom now reaches1600% for small branches.

One geometry model supplies rendering, proportional connection tabs, input insertion boundaries, frozen drag hit geometry, Fit and fold anchors. A regression caught a pointer target changing with the live drag preview; using original geometry fixed branch reorder and scaled ghost projection. Logical input order/Cursor columns and authored data are unchanged.

Pure tests verify recursive containment, square aspect, scaled connector/port alignment, folded trees, seam tolerance and view validation. Isolated browser checks cover layout combinations, saved independent duplicate panels, exact indexed picker access, scaled ghost drag/cancel, keyboard/animated folds and pinned controls. Existing chrome/branch/Shift/undo, insertion/creation ghosts, layers and camera browser suites pass, as do full Rust tests/fmt/clippy and JS syntax. Live four-combination smoke blocked writes and preserved exact project/undo/redo with zero authored commands/browser errors. No engine restart or public/Pi activation claimed.

Please retest Repeat/Carpushit dense branches and driver trees using both options, especially scaling at repeated forks, Fit/zoom, plus targets and ghost dragging. Preserve example data and run output disabled. Received the fold-motion retest report: repeated toggles/ghost cleanup and exact project preservation passed; the isolated first-Repeat-fold10px global shift was not attributed to a confirmed bug. SL-005 and SL-006 remain separate open findings.


## SL-007 Fit correction — 1501b98

Lab confirmed Expanded + Square Carpushit clipping in a340px-high canvas at the old25% minimum. Fit now measures full card/tab extents with top/bottom control margins, can go below25%, and clears stale canvas scroll offsets. Per-panel view validation/manual zoom preserve fitted scales down to1e-6. Active on local8788 source frontend after refresh; no engine restart.

New pure tall-square containment and real short-viewport expanded-square browser bounds/reload regressions pass. All layout, fold, insertion/ghost drag/undo, chrome, layers and camera browser checks pass; Rust tests (34 suites), fmt/clippy and JS syntax pass. Live four-mode smoke preserved exact project/undo/redo, with writes blocked and zero authored commands/errors. SL-007 remains pending lab verification on the exact Carpushit reproduction; lab owns findings status.


## Windows Open DMX USB — d3b89b3

Active on local8787/8788 after refresh. Output → Connection adds Art-Net/Open DMX USB transport selection, authenticated read-only FTDI discovery, serial-number adapter selection, one zero-based universe,1–40 FPS (UI starts at30) and an undoable expected-revision save. Old projects retain their Art-Net wire representation. Native secondary Output panels show the selected transport and link to connection setup.

Windows dynamically loads the optional official FTDI D2XX DLL from System32 only. Device handling is isolated on the output worker, with bounded TX drain/write timeouts, full start-code/512-slot frames, missing-universe zero filling, short-write errors, release/retry and no UI/evaluation coupling. The Windows-only module is excluded from Linux; unsupported saved USB selection reports unavailable instead of silently changing to Art-Net. Linux OLA/Art-Net configuration is untouched.

Windows full36-suite Rust tests/fmt/clippy and USB/layout/fold/layer browser checks pass. Linux x64 full36-suite tests ran offline in WSL after crates.io network timeout; ARM64 GNU and the existing Pi-class ARM64 musl build both pass. No dependency/Cargo.lock changes. Tests include mocked protocol ordering/write failures, backwards-compatible configuration, API auth, undo/redo and continued evaluation during a nonexistent-device error. No physical output/timing parity is claimed.

Activation used a fresh guarded checkpoint and preserved exact project, revision199,6undo/0redo, plugins and resumed clock; output stayed disabled. Live read-only discovery/form smoke preserved exact project/history with zero authored commands/errors. The optional FTDI DLL loads, but currently lists zero available devices while QLC+5 is running; it may own the attached FTDI port. No application was closed, driver changed or USB DMX sent. Public/Pi deployment is not claimed. Guide: [Open DMX on Windows](../open-dmx-windows.md).

Suggested lab retest: unchanged UV two-slot model and nonzero-universe project save/reload/undo in output-disabled isolated engines; Linux capabilities should disable choosing USB and retain Art-Net/OLA workflows. Leave actual Windows USB transmission to explicit hardware testing.


## Panel dropdown readability — 9f973fa

Local8788 serves the updated stylesheet after refresh. Panel header selects and native option lists now have explicit theme-matched paper/text colors and color-scheme,12px text (previously10px), and a24px minimum closed height. Applies to view type, fixture, layer and layout selects. Chrome open menus were visually checked in both themes; computed text/background contrast is12.7:1 light and14.7:1 dark. Live browser smoke blocked non-GET requests and observed zero authored commands/errors. Full Rust tests/fmt/clippy and app JS syntax pass. No engine restart, scene/history edit, public/Pi activation or unrelated lab changes. Optional retest: native menu contrast on other Windows browsers/themes.


## Isometric/rotatable viewports — b7fec61

Active on local8788 source frontend after refresh. Scene panels expose Iso alongside3D/XZ/XY/YZ, with direct45° left/right rotation and existing free drag/touch orbit. Iso is now mathematically isometric (45° yaw,35.264° pitch), preserving equal world-axis scale. Direct controls are available in schémaboy as well. Each panel's target/zoom/orientation remains independent and saves locally; entering Iso restores Orbit mode.

Rendered3D previews add a direct Iso preset and rotation buttons plus Camera → Projection (Perspective/Orthographic). Iso selects a true orthographic camera; projection changes rebuild camera-dependent SSGI/volumetric/temporal passes and preserve framing at the target. Zoom span saves/restores, touch pan/pinch works orthographically, Frame adjusts orthographic scale, and Fit retains the current viewing angle. Existing perspective poses remain valid. Stack/block canvases stay planar.

Pure tests verify equal projected axis lengths,45° rotation invariants, projection validation and span-preserving navigation. Isolated Chrome tests pass for split Scene independence/reload, keyboard activation without transport commands, mobile controls, WebGL2 and actual WebGPU rendering/projection switches/reload, and real two-finger orthographic pan/pinch. Existing camera/mobile/workspace/layer/fractal suites pass; Rust full tests/fmt/clippy and JS/module syntax pass. Screenshots were visually inspected. Live Iso/rotation smoke blocked writes and preserved exact project/undo/redo with zero authored commands/errors; no engine restart or public/Pi activation claim.

Suggested retest: dense Repeat/Carpushit layout points from Iso, split viewpoints, camera rotations and retained selection, GPU perspective/orthographic switching and phone gestures. Keep SL-005 dense selection and SL-006 stale fixture options separate; no closure claimed. Use isolated output-disabled engines and preserve live scene/example data.


## Fixture-anchored embedded stacks — 6bac303

Active on source-served local8788 after refresh. Scene/Fixture setup viewport controls offer Stacks off / Selected stacks / All stacks and size. Stacks share the scene, screen-facing and anchored above each fixture's authored layout dot; camera zoom/pan/rotation changes their placement/scale, while evaluated fixture animation does not move editing anchors. Planar and lightweight3D/isometric views are supported; the GPU preview is unchanged.

Embedded blocks select their fixture/node in the shared inspector. Property edits and add/delete use existing expected-revision commands; per-fixture layer pickers, local fold/unfold, indexed plus buttons and empty-layer creation are available. Grips open the Move chooser; direct embedded ghost dragging is not implemented. Regular Stack editor remains available for drag/driver editing. View scope/size save per viewport, local folds/layers are transient. A600-node render budget reports omitted stacks explicitly; selection raises overlapping stacks without changing anchors. Camera controls remain above overlays.

Pure scope/anchor/budget/preference tests and isolated browser tests pass for fixture-specific inspector/edit envelopes (including identical node IDs on different fixtures), creation picker, local folds, planar→Iso tracking, All/Selected/Off and reload. Isometric/WebGL/WebGPU/touch, independent camera/workspace, fractal layout and block chrome/drag/undo browser suites pass. Live read-only smoke sent zero authored commands/errors and preserved exact project/undo/redo; only frontend proxies restarted to register the asset, engine continued running. Final full Rust tests, fmt, clippy and JavaScript syntax checks pass.

Suggested retest: Repeat/Carpushit selected/all overlays, overlap/size/zoom, layer/empty-layer navigation, fixture-specific edits and deleted-fixture lifecycle in isolated output-disabled engines. Dense scene selection SL-005 and stale fixture options SL-006 remain separate findings. No live project/example data edits or public/Pi activation claimed.


## Deletion then restack — a945c92

Active on local8788 source frontend after refresh. Accepted block-only/branch/root deletes shrink/fade removed cards over160ms, then settle survivors over300ms. Matching main, split and fixture-embedded stack views reuse bounded motion capture; stale/rejected deletes do not animate. One engine transaction remains immediately undoable. Reduced motion applies final geometry directly; changing motion preference cancels ghosts. No authored timing/delay or engine schema change.

A regression exposed an unchanged telemetry refresh remounting an embedded stack after deletion repaired node selection, cutting its exit short. The publication/view key now records the repaired selection before motion starts, so matching views keep their transitions until completion or a genuinely newer context/revision. Ghosts are inert, accessibility-hidden, stripped of model IDs and removed after animation. The96-card per-view motion budget remains.

Pure tests verify exit-before-restack timing. Frozen output-disabled browser tests cover one command, block-only promotion, branch/root exits, simultaneous main/scene ghosts, delayed geometry, cleanup, undo, reduced-motion interruption and conflict refusal. Existing fold, embedded editing, chrome/delete/driver/undo, indexed insertion/ghost drag and fractal suites pass. Live chooser/Cancel smoke sent zero authored commands/errors and preserved exact project/undo/redo. Final full Rust tests/fmt/clippy and JS syntax pass. No engine restart, live deletion or public/Pi activation claimed.

Suggested lab retest when browser tools are available again: delete/promotion/branch and undo on dense Repeat/Carpushit, matching embedded views and reduced motion. Use isolated output-disabled engines; preserve live authored projects. Received your browser-disabled Work-settings report: Iso/embedded lab verification remains pending, with no bypass requested.
