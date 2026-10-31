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


## Manual DMX test sliders — 0ae96a2

Active on local8787/8788 after refresh. New DMX test viewport and Output → Channel test tab (Studio and schémaboy) show16 raw channel sliders with fixture/slot labels, universe/first-channel selection, Arm, test master, Zero tested channels, individual Release and Release test. Only touched slots are overridden, maximum32; existing scene output supplies untouched slots. It uses existing Art-Net/Windows USB routing and never enables output. Captured snapshot bytes include the override. Blackout wins.

Runtime channel_test inputs use a single-owner5-second lease renewed every second; hiding/closing/replacing the panel releases it, lost connections expire automatically. Lease data is excluded from session serialization. Project/revision/undo/redo do not change. Active output status displays TEST universe/channel count. The native HTTP smoke caught serde numeric map keys inside the tagged event; explicit decoding plus a JSON round-trip test fixed the real endpoint before activation.

Tests cover limits/ownership/expiry/blackout, unchanged project/history/revision, no lease after serialized-session recovery, and actual loopback ArtDmx override/release. Desktop/schémaboy browser tests include real touch input, sliders/master/zero/individual release, universe7/channel512 and close/panel-type cleanup. Native isolated API capture tests pass for channel512, competing-owner rejection and release; output remained disabled. Final Linux5-test lease/serialization/JSON/loopback suite passes. Existing USB UI, scene stacks, deletion/fold motion, fractal layout and workspace regressions pass (one timing-sensitive fractal run passed on rerun). Full final Rust/fmt/clippy and JS checks pass. Slider requests are bounded to one in-flight and the latest pending value/release; owner generation also works on HTTP LAN phones.

A guarded activation preserved exact live project at revision203,10undo/0redo and resumed clock; physical output remained disabled. Live disarmed dialog/docked panel smoke preserved exact project/history with zero authored commands/errors; no actual live test levels or hardware output were sent. Public/Pi activation not claimed. Guide: [Channel test](../channel-test.md).

Suggested lab retest when browser access returns: UV model's dimmer/program slots, nonzero universe, raw captured values, Release and session restart, isolated output-disabled only. No physical fixture parity claimed; preserve live and lab-authored examples. Existing Iso/embedded independent lab verification remains pending browser access.


## macOS support and release tooling — b3fb6b3 / 6f7ab14

Engine support uses macOS Application Support for the desktop project, with optional D2XX loading from the app's Frameworks or explicit installed paths. Windows and Mac share the existing serial-selected DMX sender; Windows retains System32-only DLL lookup. Linux retains Art-Net/OLA and excludes the D2XX loaders. Output UI now describes Windows and Mac capability. Art-Net and authored scene semantics are unchanged.

The native Swift menu-bar launcher owns a loopback engine, opens the existing browser UI only after its own engine announces readiness, and requests graceful session-flushing shutdown. Universal build tooling combines Apple Silicon and Intel engines/launchers, adds branding/licenses, and supports Developer ID signing/notarization/stapling. Optional complete Sparkle configuration uses the standard updater; no update feed is currently configured. Public packaging requires matching native notarization-job metadata; Mac download links remain unavailable without a notarized release. Guide: [macOS](../macos.md).

Verification here: full Windows Rust tests, formatting and strict clippy pass; both Apple Silicon/Intel Rust target checks and strict target clippy pass; ARM64 Linux target check passes. Mac package tests (architecture, public inputs, licensing, key/feed completeness, overwrite guard), public and Windows package tests, Python/JS syntax and native CI YAML checks pass. Isolated output-disabled browser tests pass for the shared USB panel and pending/notarized/invalid-filename Mac download states.

Native Mac execution, Swift compilation, Safari, physical USB timing and signed update behavior remain pending access to a Mac and signing identity. CI is configured to compile/test natively on both Mac runner types and exercise graceful session/undo recovery; it has not run remotely (no Git remote configured). No Mac distributable or public/Pi deployment is claimed. Live local engine was not restarted; authored scene/history and physical output were untouched.

Suggested lab follow-up: keep current examples output-disabled. When Mac hardware is available, retest UV model/channel-test, session undo recovery, Studio/schémaboy browser controls and scene preview on Safari; hardware DMX is a separate acceptance check. Existing SL-005/SL-006 and pending independent Iso/embedded verification are unchanged.


## Public macOS status — f02adb7

The public landing page is now updated at https://schema-s.vercel.app (deployment `dpl_FwNRp9P3gga3pPk6mTwGYLUErdNb`). It shows a visible pending Mac release status and explains that native validation/signing are required. The earlier Mac support commits were local and had not updated this static site; this deployment resolves the missing platform entry. No Mac archive or update feed has been published. Existing Windows/Linux downloads retain their previous bytes/source manifest, confirmed by independent public fetch/hash checks. Production mobile/connection/download browser regression passes, including Mac status visibility. Local engine, authored scenes and history were untouched. No scene retest is required for this static publication correction.


## Portable browser engine and snapshot consumers — bf5d586 / 8fde7a7 / 8aecd05

Live public browser editor: https://schema-s.vercel.app/play?view=studio; website animation consumer: https://schema-s.vercel.app/animation. Production `dpl_7dcBST27zJLQYkm5P2Pa22jrY7Hh` uses the committed-source browser artifact and real compiled WASM. Native services remain the default Cargo feature set; the browser crate excludes server/TUI/filesystem/hardware modules. Published snapshots and bounded session decoding are shared, while native filesystem storage and browser IndexedDB are separate adapters. Each Worker owns one engine. The consumer interface supports automatic subscriptions or deterministic manual frame stepping, and a bounded CSS scalar sink demonstrates non-Art-Net use. The scene model retains fixture IDs/banks for compatibility.

Studio/schémaboy use the command/publication interface locally in browser mode. Named projects, separate partial layout/content resources, revision conflicts and undo/session recovery are supported; JSON project import/export is available in Studio. Browser physical output is unavailable and cannot be enabled. Folder-based component loading/watch requires a native host; session-embedded components still evaluate. Browser lifecycle throttling, storage clearing, per-origin data and unverified Safari/Firefox execution are explicit limits. See [browser host](../browser.md).

Verification: full default native Rust tests, fmt and strict clippy passed; no-native core tests/clippy and WASM clippy/release build passed; ARM64 Linux target check passed. Real Chrome tests passed locally under deployment CSP and on the public production alias: scene editing, LFO, bounded Rhai value evaluation, saved projects/partial loads, invalid-session rejection, duplicate-session lock, revision conflicts, persisted undo after reload, independent/manual engines and disposal, Worker progress during blocked UI, CSS animation/pause and mobile transport. No HTTP engine API requests occurred. Browser/native/public/Mac package tests and JS/Python/CI-YAML checks passed. Existing native keyboard/hardware-input gating, director/undo, USB UI and workspace tests passed. The public WASM checksum matches its manifest; existing native archives retain their checksummed source/version. Public landing/native download/connection regression passes.

Native local runtime was not restarted and no live scene/history was edited. Clean committed web inputs exclude unrelated lab data/formatting. The browser CI is configured, not remotely executed. Suggested independent lab retests when browser access is available: UV/cyclic dimming preview and partial saves, spatial/Repeat/Bezier on the browser host, and CSS consumers; keep physical output disabled. SL-005/SL-006 and pending independent Iso/embedded verification remain open. This is not a hardware output or Safari validation claim.


## Public source and unsigned Mac download — c3ba9eb / 5a6931d / ef89564 / f74524c

Source/history is now public at https://github.com/domjancik/schema-rs, as explicitly requested. Only committed source was pushed; uncommitted lab examples/evidence, local credentials and live sessions were excluded. The new Mac preview is available at https://schema-s.vercel.app/downloads/Schema-s-macos-universal-unsigned.zip and https://github.com/domjancik/schema-rs/releases/tag/macos-preview-f74524c. It is explicitly unsigned/not notarized; an ad-hoc integrity signature requires no Apple account. The download note explains Privacy & Security → Open Anyway on first launch when needed.

Native Mac CI now ran successfully on Apple Silicon and Intel (run40882769727), including full Rust tests/lints, Swift compilation, loopback engine startup/shutdown and session/undo recovery. The universal app was built on a Mac, both executable architecture sets verified independently, and bundle signature integrity verified before packaging. Packaging seals the embedded engine before the outer app. Initial timing-sensitive update-barrier/snapshot-retention assertions were hardened, and parity checks were updated for current Clippy while still passing Rust1.88 tests locally. Browser CI also passed publicly. Physical Mac USB, GUI/Safari and fixture validation are not implied by these checks.

Website production `dpl_EAGPLDMzyrMjhsTHSPXs7XirRGne` now enables the Mac preview link with the unsigned/not-notarized notice. Actual public downloads for all four native archives match their checksums; existing Windows/Linux versions and browser artifact remain unchanged. Native/public/package browser regressions pass. No live local restart, authored scene/history edit, Pi change or Apple signing credentials. Suggested lab follow-up when Mac hardware/browser access is available: existing output-disabled scene examples, undo persistence and preview; keep SL-005/SL-006 and pending Iso/embedded verification open.


## Creator credit and Studio navigation — 44bfb12

Public https://schema-s.vercel.app/app now aliases the browser Studio. The app status footer and product landing footer include a quiet `domj 2026` link to https://domj.net, opening separately to preserve the editor. About is integrated into the Schéma brand with an accessible label, tooltip and focus treatment; the separate toolbar link is removed. The status text still truncates rather than crowding the frame/credit. Native and development source routes also define `/app`; those route-table changes require host refresh, while existing local Studio markup/styles update after refresh. No live engine restart or authored edit occurred.

Production `dpl_86bKESDBven2qPVdzVdh3Eex2v5k` verified in fresh Chrome: /app loads the WASM editor, both credit hrefs match, About is on the brand, no standalone toolbar About, no page errors, mobile landing fits390px. Full local Rust tests/fmt/clippy, JS syntax and browser/public package checks pass. Existing native archives remain unchanged; no Mac/Pi deployment is claimed for this web styling change.


## Scene view copy cleanup — 2026-10-29

Removed the implementation-oriented scene-view blurb, slogan and persistent rendering disclaimer. Scene selection now names Carpushit directly; its separate preview is labeled "preview only". Effects use Indirect light and Light beams, with the indirect-light limitation available on the control. Project replacement still confirms its effect, disables output and explains undo. No engine or project behavior changed.

Chrome preview regression passed on isolated ports8822/8823: GPU rendering, preset preview without commands, explicit revisioned replacement, WebGL fallback, GLB import/recovery and lifecycle. Required Rust tests/fmt/strict clippy and JavaScript syntax checks passed. Local8788 was already stopped; no user engine/session was restarted or edited. Native archives remain unchanged.


## Explicit driver removal across editing views — 2026-10-29

Studio property rows now show a named Remove action with an accessible property-specific label; the driver inspector also offers Remove driver to return to its owner. The shared inspector makes these actions available from regular/split stacks and fixture-embedded scene stacks. Schémaboy property menus offer Edit driver and Remove driver, and its block inspector offers Remove current driver while inside one. Hold+driver remains supported. All removals use revisioned set_driver commands, preserve the stored property value and are one undoable edit.

Schémaboy no longer refuses deletion inside drivers: deleting the driver root detaches its attachment, block-only deletion promotes its sole input, and deleting an ordinary driver input preserves its owner. Nested removal returns to the surviving parent context. A new real-engine browser regression checks Studio, embedded stacks, all three mobile views, nested attachments, root promotion/input deletion, unchanged owner values and full one-step undo restoration. Existing split-pane/chrome deletion and real-engine Shift/touch control regressions pass. No physical output was enabled and no user session was modified. Native distribution archives are not rebuilt for these frontend changes.


## Broad faceted Schémaboy surface — 2026-10-29

Replaced four narrow bevel/edge strips with ten large, adjoining triangles over a shallow uneven ridge. Each triangle has a normal derived from shared 3D vertices and shades separately against scene/camera lighting and phone tilt. The layer is behind content, no screen cutouts or bright borders remain, and encoder/strip colors are unchanged. The shell has fixed readable ink and muted colors in both page themes. No new layout rows, WebGL renderer or animation loop: ten CSS paints at the existing bounded scene/sensor rates.

Tilt now calibrates to the first valid handheld orientation before clamping, avoiding beta saturation above60degrees; screen rotation recalibrates. A camera centroid of zero is now used as centered illumination rather than falling back to the default direction. Existing explicit permissions, reduced-motion gating, local16x12/twice-per-second sampling and cleanup remain. Real phone hardware has not been tested.

Core tests check full-area coverage, shallow unit normals, opposing illumination response and handheld calibration. Chrome verifies actual face backgrounds change under synthetic tilt and directional camera pixels, preserved encoder color/no screen bevels, permission lifecycle and320/390/landscape viewport fit. Neutral/tilted screenshots inspected in target/skin-surface-neutral.png and skin-surface-tilted.png. Driver-removal and real-engine mobile touch/Shift regressions pass; the nested-driver regression now waits for UI context restoration after the engine publication. Required Rust tests/fmt/strict clippy and JS checks pass. No live user engine/session edited; native downloads unchanged. Suggested phone retest: enable tilt from the holding position, lean each way, then try camera lighting in an HTTPS context.


## Studio product branding — 2026-10-29

Removed the legacy RUST badge from Studio. The header reads schema-s, the document title is Schema-s / Studio, and the project subtitle is Studio. About remains linked from the brand. Read-only local Chrome check verified the exact name/title and link; JavaScript syntax and scoped diff checks pass. This is static copy only; no engine/session or native archive changes. Public /app still routes to the browser Studio.


## Everyday authoring QoL — 2026-10-30

Implemented the three selected improvements plus two high-value additions: Duplicate branch (Ctrl/Cmd+D), fuzzy Find fixture (Ctrl/Cmd+K), per-property Reset to default, Ctrl/Cmd+S project saving, and panel fixture/layer menu refresh after scene replacement/addition/undo (SL-006 implementation; independent lab verification pending). Actions are exposed in Studio and Schémaboy menus. Finder searches names, stable IDs, groups and universe/address and has keyboard navigation plus44px touch targets. It changes selection only.

Duplicate inserts a fresh sibling with all driver trees, leaving the clipboard intact. Layer/driver roots copy to a new independent layer in one atomic batch, respect the32-layer limit and preserve disabled source layers; undo returns to the source layer. Reset restores only the selected catalog property and is disabled while driven. Save opens the existing Projects dialog; it blurs and waits for pending field edits before opening, and Ctrl/Cmd+Y also supports redo. Shortcuts do not duplicate or search from editable fields/modal dialogs. Fixed selection-only polling replacing a newly focused desktop property field.

Pure planner tests and real isolated-engine browser tests pass: full branch/driver copies, root layers, reset/default equality, one-step exact undo, desktop/mobile search and reset, dirty-field save, typing guards, and fixture option additions/names after full project replacement and undo. Existing driver-removal, split-pane/chrome deletion and mobile skin/permissions regressions pass. Required Rust tests/fmt/strict clippy and JS/diff checks pass. No live user project/history or physical output edited; native archives remain unchanged. Public web publication follows the commit. Retest nested Repeat/Bezier duplication and property reset, and independently confirm SL-006 on Carpushit switching; retain SL-005.

Remaining priorities: dense-scene marker/footer occlusion (SL-005); searchable command/shortcut palette; fixture snapping/alignment/distribution; per-block bypass with defined signal semantics; portable project bundles including resources and component dependencies; inspectable undo history and named scene checkpoints. These are follow-up candidates, not new implementation claims.


## Shared application styling — 2026-10-30

Added interface.css as the shared application material layer for Studio, Schémaboy and scene preview, served by both development/native hosts and exported to the browser build. Shared light/dark tokens unify fields, native select styling, checkbox treatments, typography, border/radius, focus/hover states and dialog surfaces. Components retain their view geometry and identities: original Schéma-inspired mint/block outlines (https://schema.scenic.tools/), current workspace panels, family colors, and the ten-facet handheld shell. The landing/download page already has its own product design and remains unchanged.

Refined the Studio toolbar, panel menus, workspace/layer tabs, sidebar, inspector property controls, Projects, component manager, block picker, Output and mobile dialogs. Property fields have full row width with driver/reset actions grouped below. Studio toolbars wrap at compact sizes and its workspace/footer fit the remaining viewport. Output sections now expose a pressed state and visible selected treatment; its heading is simply Output. Phone form fields stay16px and controls/close buttons retain44px touch sizing. Improved property-caption contrast within block family colors and removed a newly detected theme-transition contrast flash. No authoring/evaluation semantics changed.

Visual review covered original app imagery and before/after Studio, compact desktop, picker, Projects, Output and mobile snapshots in target/style-review/. Read-only interface tests pass light/dark field/primary/block-caption contrast >=4.5, immediate theme-switch contrast, keyboard focus, selected sections, Studio widths850/1024/1100/1500 and phone dialog portrait/landscape bounds. QoL, driver removal, top-right chrome/deletion, independent panel resizing/persistence, fractal/square geometry, mobile skin permissions and WebGPU/WebGL2 scene preview regressions pass. Full Rust tests/fmt/strict clippy pass, including native page/stylesheet routing; JS and browser-package checks pass. User scenes/history and hardware output untouched; native archives remain unchanged. Web publication follows the commit.

Independent scene lab also verified SL-006 fixed on7f3b10b (including Carpushit project replacement/undo), fresh duplicate driver IDs and default reset/undo. SL-005 and earlier unverified camera/embedded variants remain open. Suggested styling retest: dense stack input/driver/reset control reachability, panel menus, phone Output/Projects and light/dark colors.


## Fixture placement in 3D — 2026-10-30

Added the shared fixture-placement module to Studio 3D Scene views, Schémaboy 3D and live WebGPU/WebGL2 previews. Place exposes XYZ metres, draft nudges, Apply, Y=0, free/1cm/10cm/50cm/1m snap and XZ/XY/YZ drag planes. Show move gizmo (or G) draws labeled/color-coded world axes and a plane handle at the authored layout position, with 44px hit targets and keyboard nudges. Frame layout targets authored coordinates when evaluated positions differ. Small phones can explicitly expand the scene and return with Done; regular controls remain available outside that temporary mode. Camera/screen reservations keep the Place touch button off fixture markers.

Moves preview locally, freeze camera navigation during dragging, and send one expected-revision MoveFixture on release or Apply. Escape, pointer/touch cancel, focus/visibility loss and revision/selection changes discard the preview. New authoring commands, engine evaluation and history formats are unnecessary; undo uses the existing engine transaction. Homogeneous projection solves axes/planes correctly in perspective and orthographic views. Degenerate/end-on handles hide, while precise fields remain available. Gizmos stay above embedded stacks, camera controls stay above gizmos, static handles retain DOM identity between frames, and reference presets disable placement. Imported geometry keeps its existing transform.

Verification: pure movement/depth/snap/invalid-coordinate tests pass; isolated real-engine Chrome tests cover Studio/mobile/WebGL/WebGPU XYZ Apply, deferred axis and snapped-plane drags, exact one-step undo, keyboard nudges/G, actual CDP touch end/cancel, conflicting-edit cancellation, invalid values, layout-versus-evaluated coordinates, Frame layout, read-only preset gating and320x568 expanded/Done. Existing mobile camera pointer/hit-target, desktop/touch GPU camera, independent/maximized/hidden panel, interface contrast/viewport and renderer lifecycle regressions pass. Actual isolated WASM editor placement/undo, durable projects/history, scripting/animation and no HTTP engine calls also pass. Full Rust tests/fmt/strict clippy, native script/style serving and JS/browser-package checks pass. Visual artifacts are in target/placement-review/. No user project/history or physical output was changed; native public archives remain unchanged. Web publication follows this commit.

Suggested independent lab retest: place a Carpushit fixture while its position output animates, snap it on a selected plane, undo, and check dense embedded stacks plus phone expand/Done. Keep SL-005 and unrelated pending camera cases open until independently verified. User guide: docs/scene-camera.md#fixture-placement and docs/gpu-preview.md.


## Direct Scene panel navigation — 2026-10-30

Scene preview pages now render as persistent full-size views. Embedded panels omit the duplicate heading, close button, Studio link and intermediate Open scene view screen. Escape leaves an embedded view open (and still restores a maximized panel). Standalone previews offer Return to Studio, using /app in browser mode and the native Studio route otherwise. Previously the iframe-local Studio link attempted to embed a page protected by frame-ancestors none, producing a security error. The security policy remains unchanged; navigation is corrected instead.

Regression first reproduced the unwanted opening screen, then passed for two independent panels, Escape, maximize/restore and renderer preservation. Local production-CSP WASM tests cover embedded navigation, standalone return to /app, shared session/project preservation and zero HTTP engine requests. Native standalone return, WebGPU/WebGL2 rendering, modal close/reopen, imported model recovery and phone bounds also pass. Web publication follows this commit; native archives and live user scenes/history are untouched. Suggested lab retest: change a Carpushit panel to Scene preview, Escape/maximize/restore, switch panel type, and open a standalone preview then Return to Studio.


## Preview texture under production CSP — 2026-10-30

Follow-up to Scene navigation commit5718b80: the published smoke check also exposed Three.js fetching its embedded Bayer PNG through ImageBitmapLoader, which connect-src self correctly blocked. The pinned vendor build now extracts the original2065-byte PNG to /scene-assets/bayer.png and substitutes only its URL in the bundle. Native and dev hosts serve the asset with image/png; browser packaging includes it automatically. No CSP directive or remote origin is added. Vendor versions/licensing remain unchanged and the recorded bundle hash is regenerated. A byte comparison confirmed the only vendor source delta is the texture URL.

The production-policy WASM regression now fails on CSP console violations: reproduced the blocked data URL before the change and passes after it. Native route coverage verifies PNG signature/content type and no project mutation. Independent panels and WebGPU/WebGL2 preview/lifecycle tests remain covered. Both navigation and texture corrections are being published; native archives remain unchanged.


## Saved servers, QR pairing and app updates — 2026-10-30

Added a shared Servers interface to Studio, schémaboy and the public connection page. It stores validated canonical HTTP(S) origins, names and preferred UI views, deduplicates origins and supports edit/remove. Open links load the host’s own UI in a separate window with noopener/noreferrer; the local browser workspace is retained. Credentials, query tokens and unsupported URL paths/schemes are rejected. No server probing, remote transport adapter or extra discovery is introduced.

Saved hosts and native current-host addresses generate a direct-URL QR locally, with copy/download controls. The app decodes camera frames or local images using pinned integrity-checked qrcode-generator2.0.4(MIT) and jsQR1.4.0(Apache-2.0). Review precedes Save/Open. Stream cleanup covers close/cancel/hide/navigation and late permission grants. Camera use needs HTTPS/localhost; image/manual pairing remains available on LAN HTTP. The dialog keeps Close visible and uses readable phone action rows and an immediately framed QR.

Servers → App updates checks the public build manifest and a waiting service worker, waits for asset installation before announcing readiness, and activates only on explicit reload/update. Reload waits for edited-field writes, embedded-preview writes and durable browser session save. Offline helper cache now includes the saved-host screen and local QR tools; no scene/API data is cached. Native interface reload and engine deployment remain distinct. This is an app update, not an upgrade of existing native download archives.

Verification: address/credential/storage tests and QR encoder/decoder round trip; real Chrome UI save/reload, actual QR image decode and explicit review, text/XSS safety, denied camera, normal and late stream cleanup,320/390/844 viewport bounds; production-CSP browser/WASM app update preserves project/undo/redo/saved servers with no HTTP engine calls. A separate real-worker regression reproduces the pre-install readiness race, then verifies explicit activation, durable history, saved servers and preservation of unrelated caches. Offline install/reconnect test exercises cached generation and image decoding without network access. Public host opening uses a mocked external UI, verifying its own-version route. Editor QoL and shared interface regressions remain covered. Full Rust fmt/tests/strict clippy and native route, browser export, Windows and macOS package checks pass. Screenshots: target/server-review/. Full guide: docs/servers.md. Web activation follows the final commit; native archives and user projects/hardware remain untouched.

Suggested lab retest: save two controller origins, switch each preferred view, export/scan its QR, verify login stays host-local, then exercise offline pairing and app update with a browser scene that has undo history. No physical camera permission or native service update is claimed by the automated checks.


## Optional four-digit access PINs — 2026-10-30

Native LAN hosting supports an explicitly configured four-ASCII-digit PIN or an existing16+ character token. Four-digit guesses are bounded using the real TCP peer, not forwarded headers, with per-peer/shared windows and bounded storage. Missing/stale long tokens do not consume the PIN allowance. Portal forms support configured four or six digits while preserving CSRF/session protection. Provisioning validators accept the WPA2 minimum password length. Defaults do not contain shared credentials.

Full native fmt/tests/clippy, true-peer throttling and deterministic expiry, legacy-key/PIN, portal proxy/CSRF and deployment regression checks pass. Signed native builds and isolated smoke checks pass. Operational activation records are kept privately; no device addresses, credentials or deployment metadata are added here. Generic guide: docs/access-pins.md.

## Compact Studio and independent Output panels

- Studio uses a single-row mobile header with Undo, Redo and Save; secondary actions remain available in the overflow menu, including Connection. The panel selector opens every view at full width. Workspace offers focused-panel and full-split presentation without replacing the saved desktop arrangement.
- Output now occupies its own vertically split pane below Stack editor in the factory Authoring layout. Custom layouts remain intact, and Output opens or focuses its own pane when requested. Fixture setup and Stack editor no longer contain output telemetry.
- Duplicate Output instances retain independent drafts, universe and channel selection. Revision conflicts retain edits until explicit reload; incoming telemetry never overwrites an unsaved destination.
- Verified compact bounds at 320×568, 390×844, 768×1024 and 844×390, desktop split persistence, menu access, contrast and independent Output drafts against an output-disabled isolated engine. Dense-scene marker/footer issue SL-005 remains open pending its own independent retest.
- Scene views now retain their unchanged 3D renderer across snapshot refreshes, preserving open fixture-position dialogs. Studio/schémaboy/WebGL/WebGPU placement, camera instances and actual WASM editing/session recovery passed.

## Authoring workflow tools

- Added deterministic overlap picking and collision-aware schematic labels with measured widths; long labels are shortened without losing accessible fixture names. Alt+click cycles crowded schematic candidates; touch opens a searchable chooser. 3D previews support overlap picking too.
- Commands (Ctrl/Cmd+Shift+P) searches authoring/project/output actions and exposes shortcut/state help. Arrange fixtures submits snapping, alignment and distribution as one revision-checked batch.
- Per-block bypass is persisted and undoable. Temporary output focus uses existing block telemetry rather than re-evaluating nodes; normal evaluation/actions continue, while that fixture’s output/emitter bank comes from the chosen block. Selection, enabled evaluation, bypass, debug output, capture/live, pause, warnings and blackout have distinct labeled states.
- Added a shared browsable undo/redo timeline and up to 32 durable named authored checkpoints. Restoring a named checkpoint is undoable and disarms physical output. Debug focus clears on restart or when its block disappears.
- Portable `.schema-bundle.json` export/import includes graph components, provider assets and bounded resource files. Imports are prepared/validated before one atomic commit, preserve previous scenes for Undo, reject conflicting names and traversal/symlinks, and leave physical output disabled. Projects exposes bundle import/export directly.
- Native engine, router, portal, plugin-conflict and serialized recovery tests pass. Actual WASM tests cover bypass, history/checkpoints, resources/plugins, reload recovery and transient debug state. Shared Studio/schémaboy workflows and 320px/landscape bounds pass. See [authoring guide](../authoring.md) for semantics and limits. Activation verified after committed-source export and signed update validation.

- Release verification caught and fixed Commands opening before the first scene publication, plus hidden compact-footer error messages. Commands now waits for connection; errors remain visible and accessible. Dedicated delayed-start/no-mutation regression added.

- Final authoring code: `22e1e52` (engine), `30654cc` (UI), `62cad51` (startup/error visibility). Published web asset/WASM hashes and ready Studio/schémaboy startup verified. Signed native activation preserved authored scene and history; fresh login, embedded authoring assets and the history endpoint were checked. Scene lab notified for independent dense-scene and workflow retests.


## Versioned storage migrations: 17360df

- Shared ordered Rust migrations now upgrade v1 projects, split layouts/content/output, named-project manifests, sessions and bundles to v2 before typed decoding. All undo/redo projects and named checkpoints migrate; runtime state, clock, providers, components and bytes remain intact.
- Loads leave sources untouched. Native legacy replacements keep exact originals in adjacent `.migration-backups`; browser backups and replacement share an IndexedDB transaction. Future versions, unknown fields and duplicate keys fail without silently losing saved data.
- Browser recovery uses raw JSON through Rust, preserving 64-bit random-generator state. Raw project/bundle import/export and raw saved project/resource records avoid client numeric conversions. Existing output-disarming rules remain.
- Historical v1 golden fixtures, native CLI/API/backups, actual WASM history/runtime retention, atomic IndexedDB failures and migration-aware updater rollback are tested. See [storage migration contract](../storage-migrations.md). No reverse migration is offered; retain originals for older engines.
- Active in the public browser app and the signed appliance release from `17360df`. Public WASM/JS hashes match the committed build. Appliance activation retained revision 3, undo 3 / redo 0, every retained project migrated to v2, and exact original session bytes are backed up. OLA was unchanged and engine/portal/AP services are healthy. Full Rust tests/fmt/clippy, browser-only clippy, actual WASM migration/runtime suites, package/portal tests, updater recovery tests and ARM64 signature tests passed. The native release also passed its historical-session CLI smoke before signing. Suggested lab retests: open older examples/bundles, save/reopen partial layouts/content, exercise undo/redo and checkpoints after restart, and confirm component/resources survive. Keep isolated engines output-disabled.


## Live block attributes: 11b9624

- Produced colors now have real swatches on block cards, alongside scalar/vector summaries. These are separate from state/status styling. The block inspector displays incoming merged-child attributes and outgoing attributes separately, including RGBA values and bounded metadata for arrays/channels/audio/textures/emitters.
- Studio workspace/pinned inspectors and schémaboy share the view. A compact swatch/value button beside schémaboy's selected-block label opens incoming/outgoing details without expanding its main viewport.
- Visible targets request bounded one-shot capture at most 4 Hz. Capture happens in actual evaluation, never in the lighting probe or a second graph evaluation. No targets/hidden tab means no new demand. Runtime and undo state remain unchanged. Ordinary UI state uses a compact route without node traces; full state/recovery compatibility remains.
- See [block inspection](../block-inspection.md) for limits and APIs. Suggested retests: changing colors/pixel maps through modifiers, drivers and layers; bypass, pinned and embedded-stack targets; phone portrait/landscape; and zero demand with views hidden. Keep output disabled in lab engines.
- Active in the public browser app and signed appliance release from `11b9624`. Public JS/CSS/WASM hashes match the committed sources. The live appliance returned real incoming/outgoing samples, compact snapshots omitted node traces, and inspection left project, undo/redo, components, resources and checkpoints unchanged. Verification observed revision 10, undo 8 / redo 0; activation preserved history and OLA was unchanged. Engine/portal/AP services are healthy. Full Rust tests/fmt/clippy, native and real WASM inspection tests, browser authoring/migration suites, package validation and signed native/GPU/storage smoke checks passed.


## Commands browser shortcut: a47ab93

Commands now uses unmodified **F2**, with the Commands/menu button retained. Ctrl/Cmd+Shift+P is no longer intercepted, leaving browser print/private-window shortcuts alone. Shortcut help, tooltip, accessible keyboard metadata and canonical authoring guide use F2. Held/composing/modified F2 does not trigger the palette. Real browser regression covers Studio/schémaboy, click access, shortcut help, typing and untouched P chords. No authored/engine behavior changes. Active on the public browser app and signed appliance release from `a47ab93`. Published script/build hashes match. Live Chrome through the Pi portal verified F2 in Studio/schémaboy and F2 help, with no authored commands submitted. Signed activation preserved scene/history and OLA. Full native release checks and real WASM shortcut regression passed.


## Brightness readouts and daily time simulation: 6287a46

- Numeric output values/meters now stay visible alongside color swatches, including output focus; the preview strip no longer hides them. The old color-only numeric readout remains mean RGB of the first pixel. Also guarded bank lookup before the first publication.
- Output → Dimming shows engine current schedule time, live/current-level readouts and a solid current-time marker. Preview time can be scrubbed or played at 1×/60×/3600×, with a separate dashed marker.
- Applying simulated time to output is explicit, schedule-only and transient. A visible global indicator can return to live. Owner-scoped cleanup and a 15-second heartbeat lease prevent orphaned simulation; mode is excluded from projects/session/undo. Existing publication carries lightweight clock status; no extra frame polling.
- Native clock/HTTP tests cover output bytes, midnight/offsets, ownership, expiry and unchanged history. Actual WASM tests cover focus brightness, clock markers, preview versus output, playback/live return, close cleanup and 320px phone layout. See [daily dimming](../daily-dimming.md). Suggested isolated retests: midnight curves, nonzero offsets, saved versus draft curves, a second view taking over simulation and disconnect recovery. Simulation view IDs use HTTP-compatible getRandomValues; real Chrome over the Pi HTTP portal verified editor/preview without secure-context randomUUID or output changes.
- Active on the public browser app and signed appliance release from `6287a46` (brightness fix `637ee7c`, shared clock/UI `6117540`). Public asset/WASM hashes match the release sources. Actual Chrome over the Pi HTTP portal verified numeric readouts, engine time/current marker, local scrubbing and the clock API proxy without activating simulation or authored edits. Signed activation preserved scene/history and OLA; native/GPU/storage smoke checks passed. The incompatible intermediate build was stopped before Pi activation.


## Carpushit startup scene activated

At the user's explicit request, the current scene-revision-2 Carpushit authoring project is loaded on the appliance: 33 fixtures, FULL initial state, bulb model ceilings retained, eight named content presets and eight standalone stack providers validated with the installed engine. The original authored data was copied privately to the device; no scene assets were newly published or committed.

The previous UV project was saved separately and an exact recovery-session/registry backup retained. Loading added one undo entry. The active Carpushit scene is saved as named project `carpushit`, default `init` layout/content/output and the durable startup session. The existing boot-enabled service continues resuming its session; a real service restart restored the exact project and undo/redo (revision 13, undo 11 / redo 0), preset providers and controls. OLA, AP, DHCP and portal processes were unchanged.

Physical output remains disabled, matching the previous state. The scene's native cue Director remains disarmed as authored. The patch is the authored 33-circuit scene patch; no physical fixture addressing or UV-channel remapping was assumed. No application code change or firmware update was needed.

## Universe patch fixture groups

The universe grid now draws each fixture footprint as one labelled, coloured frame, matching the corresponding fixture card. Individual channel numbers and exact-address drop targets remain intact. Fixtures crossing rows keep the same colour/name with a continuation marker; the grid responds between 32 desktop and 16 mobile columns. Channel descriptions include custom-model slot names, including constant Program channels. Mobile fixture cards now fit their container.

Source implementation only; the Orange Pi and public deployment have not been updated for this change. No live project, output, session or history changes. Browser regression covers adjacent fixtures, custom two-channel models, single-channel fixtures at 512, multi-pixel row wrapping, mobile resizing, drag/drop address preservation and close/reopen. Retest the UV two-slot example and crowded multi-channel patches when activated.

## Accented frontend product name

Display branding now uses Schéma-s/schéma consistently across Studio, landing-page wordmarks and copy, browser animation demo, mobile About link, server pairing/offline screens, page titles/accessibility names, installed-app full name and the Orange Pi captive-login template. Existing schémaboy names were retained. Technical identifiers, package/download filenames, API/storage keys, hostnames, routes and the PWA installation identity are unchanged.

Source change only, pending publication/device activation. Browser export packaging, visible HTML/accessibility text and manifest-identity checks pass; the existing product browser assertion expects the accented page title. Retest Studio/landing/server and phone-install labels on activation. No live scene/history/output changes.

## System logs view

A dedicated System logs panel, menu dialog (including schémaboy) and Commands action expose bounded native/browser runtime events. Engine-owned 512-event memory buffers deduplicate unchanged diagnostics/output statuses and record startup, diagnostic recovery, revisions, pause/blackout; native hosts also record action errors and session-save failure/recovery. Reads are authenticated, cursor-paginated and capped at 200 events, separate from normal state publications and persistent project/session/undo formats. Independent viewers offer search, levels, pause/follow, copy/download and local Clear view; one shared poller runs only with visible unpaused demand. See docs/system-logs.md for limits, including no OS/systemd/OLA journal access.

Source only, not pushed or activated on the Orange Pi or public deployment. Live Carpushit, output, clocks and histories remain untouched. Retest diagnostic appearance/resolution, output errors and mobile log access on isolated output-disabled examples after activation.

Validation: complete native cargo test, fmt and strict Clippy; WASM release build and strict Clippy; actual WASM Studio/menu/workspace and schémaboy logs integration; viewer search/levels/pause/restart/cleanup/safe-text/mobile tests; authenticated API/cursor/bounds/non-persistence tests; browser packaging and workspace regressions all passed.

## Mobile Studio panel icon row

Mobile Studio replaces the top panel dropdown with an in-layout bottom navigation row. Each existing workspace pane has an SVG icon, short label and accessible name; duplicate view instances are numbered. Selection persists per workspace, and active styling follows panel changes. Overflow scrolls inside the row, touch targets exceed 44px, keyboard arrows/Home/End work, and bottom safe-area padding reserves space rather than overlaying the editor. Tapping an icon in split mode returns to that single panel; desktop split layouts remain intact. No project/session/undo schema changes.

Source only, pending publication/device activation. The running Orange Pi scene and output are untouched. Retest 320px phones, landscape, duplicate Stack instances, Output/Logs access and switching back to desktop after activation. Details in docs/workspaces.md.

Validation for `Use bottom icon navigation for mobile panels`: native cargo fmt/test/strict Clippy and final embedded binary build passed; actual WASM mobile touch/overflow/duplicate/persistence/keyboard/viewport checks, full browser runtime and Commands checks passed. Final isolated native compact Studio, desktop workspace and authoring regressions passed. Navigation buttons use a separate target attribute so existing panel identity selectors remain unique.

## Block color/value layout

The independently positioned attribute strip and numeric indicator are replaced by a shared flex footer in HTML blocks. Both now participate in width allocation; attribute previews clip to remaining space and very long values ellipsize while retaining their full tooltip. The color preview, meter and numeric output share one horizontal row, with the bar flexing within the block. Existing/scaled block faces use the same layout, with no DOM measurement loop, engine changes, new inspection traffic or authored/history changes.

Source only, pending deployment. Retest colored/scalar blocks, long negative numbers, fractal/square faces, independent/embedded stacks and output focus after activation. Regression coverage checks 240/178/120/96px widths, the production #world cascade, scaled .layer-world faces, single-row alignment and meter containment; the real WASM Studio/phone and on-demand inspection checks also run.

Validation for `Keep block color, bar and value in one flex row`: overlap and one-row regressions first failed as expected, then passed against real WASM/production styles, long-value and narrow/scaled face cases, Studio output focus, phone inspection and demand throttling. Browser runtime, layout/grid, mobile navigation and packaging checks passed; native fmt/test/strict Clippy passed. No live service or scene was changed. A separate test-only commit waits for media-query completion before the mobile navigation desktop assertion.

## Fixture setup pan and zoom

The main setup map now supports background/middle/right drag pan, pointer-centred wheel zoom, single-finger background pan, two-finger pan/pinch, a Pan-over-fixtures toggle, zoom buttons, Fit and Reset. Keyboard arrows/+/-/F/Home work with map focus. Camera preferences persist per browser; view navigation never enters project/history. Fixture dragging uses inverse projection at the current camera, preserves Y, commits once and can move beyond the original ±10 bounds. Pinch/cancel gestures cancel pending fixture drags. Authored handles remain independent of animated previews; the adaptive grid is bounded and follows the camera.

Source only, pending publication/device activation. Live Pi/scene/output/history are untouched. Retest crowded picking, fixture drags after zoom, far-away layouts/Fit, touch pinch beginning on a fixture, and browser reload. See docs/workspaces.md.

Validation for `Add pan and zoom navigation to Fixture setup`: pure projection/inverse/anchor/fit/grid tests, real WASM mouse/touch gestures (including overlapping handles), tap picking, 44px phone targets, saved camera, modifier shortcut preservation and world-correct one-command drag all pass. Full native fmt/test/strict Clippy, final native build/lint, WASM app/mobile navigation/packaging and isolated native layout-selection, authoring and compact-Studio regressions passed. The legacy selection regression was separately refreshed for compact state, periodic clock input stubbing and the current mobile 3D handle markup.

## Port 18798 upgraded and enrolled — 2026-10-30

At the user’s explicit request, the retained random-dim workspace at `http://127.0.0.1:18798/?view=studio` was upgraded from its older copied lab engine to signed release `f553bb1558ec-8ef694475951-38` (source `f553bb1`). The original v1 session was backed up and migrated to v2. Carpushit - dim random breathing retains all 34 fixtures, revision 7 and seven undo entries; output remains disabled. Native executable digest/signature and served frontend content against current HEAD were checked after activation.

The exact workspace is enrolled as `scene-lab-random-dim` in the existing updater. The updater reloaded enrollment and automatically advanced the first managed build to f553bb1, verifying history restoration. Use the same URL for authoring and retests. Example source data and named projects were retained.


## Organizational fixture colors — 2026-10-30

Commits `ce40419` and `2c905be` add optional `marker_color` fixture metadata and Fixture setup color/clear controls in Studio and Schémaboy. Desktop multi-selection commits one native batch. Markers appear in fixture navigation, authored layout handles/rings, embedded stack headers, overlap/finder choices and grouped universe patch borders. They never enter evaluated attributes, lighting or DMX. Older projects remain unmarked; sessions, bundles and layout saves retain marks. Marker-only Undo, Redo and HistoryJump retain running node state.

Signed release `2c905be32999-1e39cda32b4b-40` is active on managed local ports 18798/8788/8797; the enrolled Pi push succeeded with scene/history preserved and OLA unchanged. Public website publication remains separately pending authentication; this note does not claim public activation. Full native tests, fmt and strict Clippy, browser marker/standalone universe regressions, and browser plus macOS CI passed. Release signature and live Studio HTTP were independently verified.

The scene lab applied the accepted family palette to all 34 fixtures in live Carpushit random-dim on 18798, using one expected-revision batch, and saved the named project. Revision 8, eight undo entries, output disabled. Comparing against the private pre-edit session proves only marker fields changed: programs, patch, dimming and positions are identical. Scalar-only playback stays neutral white. Prepared Carpushit color banks and preview emission metadata are white; violet identifies UV fixtures for navigation. Bundled preset geometry/roster and GLB are unchanged.

The scene lab has the release details and an isolated-clone retest request for marker-only undo/redo/history while Random.Dim runs. Do not exercise this verification against live user history. Marker guides and evidence live under `docs/scene-lab/carpushit-markers.md` and `docs/scene-lab/evidence/carpushit-markers/`.


## Content/preset selectors and fixture group assignment — 2026-10-30

Commit `97c75a3`: Projects distinguishes saved projects and the existing bundled Carpushit reference preset. Shared Content library in Projects/Apply/mobile offers search, saved-content selection, target group selection, sparse fixture/layer scope, explicit overwrite choice and revision-checked saves/loads. Content-block references use on-demand saved-content selectors. Fixture setup and scene Place dialogs share searchable group checkboxes, counts, mixed multi-selection and explicit new group creation; changes preserve other fixture metadata and memberships. No automatic scene/preset loads or roster changes.

SL-008 fix: native provider now uses the validated versioned Content decoder instead of directly parsing a legacy array. Regression reproduces omission, then verifies registration, changed content and restart with v2 saves. This is source/test verification; the lab should retest after activation before closing the finding. This selector work covers the discovery/group-recall portion of SL-004, not its additional timeline/preset-authoring requests. Existing sparse targets, API layers and the canonical33 versus bundled38 reference distinction are retained.

Retest content selection and save/load from Studio/Schémaboy, Assign.Content reference selection, group mixed checks and additive membership, group edits followed by position drafts, and current-format provider refresh/restart. Keep live 18798 and Pi scenes/history unchanged; use isolated output-disabled clones.


Activation verified on 2026-10-31 local time: signed release `97c75a312b08-7990b809f88d-41` is installed on managed local ports 18798/8788/8797; enrolled Pi delivery succeeded with scene/history preserved and OLA unchanged. Signature and all artifact hashes were independently checked using the public verification key. A read-only real-browser check of live 18798 confirmed fixture group checks, the preset selector, and the saved-content/group selectors; full project and complete undo/redo arrays are identical before/after. Carpushit random-dim remains revision 8, eight undo entries, 34 fixtures and output disabled. Public website activation is not claimed; its separate authentication prerequisite remains pending.

All native tests, fmt, strict Clippy/build, actual WASM/native selector integration, existing named-project/marker/runtime regressions, browser packaging, and browser/Linux/macOS CI passed. The scene lab independently verified SL-008 registration/change/restart and exact sparse Carpushit scope (10 bulb API1 targets for slow-pulse, 22 outside API2 targets for twinkle-on), with all other authored data preserved. SL-008 is independently verified fixed; SL-004 timeline work remains separate. See [independent report](content-selector-retest-2026-10-30.md).


## DMX color Lightness source — 2026-10-31

Commit `48ff863`: native/WASM DMX models accept `lightness`, shown as Lightness (HSL) in Output → Models → Slot source. It uses per-pixel `(max RGB + min RGB) / 2`, matching original Schéma’s LightnessMapping ToHSL conversion. Colors take precedence over a separate scalar Value; scalar-only banks use neutral gray. Alpha, fixture/master dimming and blackout apply once; program constants remain unchanged. Existing models, fixture assignments and live output are not modified automatically. New-source models need an updated engine.

Retest color-derived dimmer models with saturated red (50% HSL lightness), grayscale/scalar output, master/fixture dimming, blackout and serialized model/undo roundtrips in isolated capture-only engines.


Lightness activation verified: signed release `48ff863d43b1-e8ba54892395-42` is active on managed ports 18798/8788/8797 and the enrolled Orange Pi; Pi delivery reports scene/history preserved and OLA unchanged. Public-key signature and artifact hashes checked. Complete project, undo, redo and revision were independently identical across activation on 18798 and 8788. Full native tests/fmt/strict Clippy/build, real-WASM source selection/save/reopen/byte encoding, output/universe and content/group regressions, and browser/Linux/macOS CI passed. Live Studio source-option verification is read-only; no existing models or fixture assignments were changed. Public website activation is not claimed.


## Studio numeric and enum editing — 2026-10-31

Commits `e478a67`, `cee00f6`, `072bca9` and `8468526`: a dedicated property-editors module adds exact numeric entry plus suggested sliders in Studio block/fixture editors, focused wheel/arrow changes (Shift fine, Alt coarse), one native revisioned edit per wheel burst or released slider, Escape/pointer cancellation and stale-revision rejection. Suggested ranges can expand to authored values; they do not clamp arbitrary program values. Small enums expose two-to-six visible, keyboard-accessible choices, including Oscillator Type. Bigger enums retain dropdowns. Generic focused numbers in Output/placement dialogs also receive wheel adjustment; their existing Apply/Save semantics remain intact.

Tests use isolated output-disabled native/WASM workspaces. Retest oscillator Period/Type, numeric drivers, manual values beyond suggested ranges, a canceled slider, wheel history/focus, simultaneous author conflicts, fixture coordinate/dimmer controls, and enums in embedded/independent inspectors. No project/storage format changes or live scene/model assignments are part of this update.


Pre-activation precision follow-up: scene-lab examples such as 0.00625 now retain their authored decimal precision when nudged, instead of rounding to the base wheel step’s display precision. Native safe-integer nudges remain exact. Pure and real-browser regressions verify manual 0.00625 and Shift nudge to 0.01125 for oscillator Period, and normalized-value nudges to 0.01625/0.00725.


Activation verified: signed release `84685269ead8-2abc7f5d4984-44` is installed on managed ports 18798/8788/8797 and the enrolled Pi. Public-key signature and artifact hashes checked. Complete projects, undo/redo arrays and revisions are identical to pre-activation checkpoints on 18798 (revision 8) and 8788 (revision 203); Pi delivery reports scene/history preserved and OLA unchanged. Live fixture numeric controls and Carpushit’s stored master Value/slider were checked read-only. Served code contains all final precision, integer-step and steady-slider corrections. Public website activation is not claimed.

Validation includes native fmt/full test/strict Clippy/build, pure range/precision specs, actual native and WASM interaction tests, real slider release beyond its bounds, batched wheel/focus, fine 0.00625 exact values, scalar/enum transition ordering, invalid input recovery, cancellation, conflict rejection, fixture XYZ/dimmer edits, existing Bezier/Light.Emitter enum workflow, and recent content/group/marker/Lightness regressions. Browser/Linux/macOS CI passed. Slider bounds now remain fixed during a pointer gesture even when an exact authored value expanded the suggested range. Generic numeric dialogs honor HTML’s default integer step (including DMX bytes); explicit fractional steps retain their behavior.


## Accented product branding audit — 2026-10-31

Activated in signed release `dbe086bb67d3-52492b1b0c4f-47`: audited visible HTML text, accessibility labels and app manifest; local Studio and published /app already render Schéma-s. Corrected remaining native labels in the Windows console title (UTF-8 codepage before the accented title), macOS bundle display/name/menu/alerts/network-permission text, terminal heading, Linux service description and human-readable scene coordinate metadata. Stable package filenames, executable/app-data paths, service identifiers and protocol keys retain their existing spelling. A branding regression checks visible frontend/PWA names and native labels; package tests check the generated Windows launcher and macOS plist. No authored project/history changes.


## Stack editor breadcrumbs — 2026-10-31

Activated in signed release `dbe086bb67d3-52492b1b0c4f-47`: Studio’s active and independent/split Stack editors share fixture/layer selectors, a pinned Root action and clickable block/driver ancestry. A driver crumb returns to that driver root; Root clears the driver trail and returns to the current layer root. Layer/fixture changes and Director/empty layers are handled. Selected ancestors are revealed through folds, and invalid driver tails recover to the nearest valid prefix. Navigation stays in existing client/workspace context and never submits authored engine commands.

Validation: breadcrumb core and WASM browser tests pass, alongside existing property/content/fixture-marker/browser-runtime regressions. Coverage exercises two nested drivers, parent/root navigation, hidden selected blocks, Director and empty layers, keyboard use, narrow Studio layouts, independent-pane context preservation, and exact unchanged project/undo/redo/revision. Retest nested drivers and scene-to-stack selection on isolated output-disabled examples; no live project changes are part of this feature.


## Attribute-specific block readouts — 2026-10-31

Activated in signed release `dbe086bb67d3-52492b1b0c4f-47`: Studio and Schémaboy show the numeric value readout/meter only when the evaluated outgoing bank contains a finite scalar `value`. Color-only output keeps its swatches without converting mean RGB to a synthetic scalar. Zero remains visible; removing a scalar hides and clears stale numeric widgets. The shared readout function also serves Studio telemetry, initial block rendering and Schémaboy SVG blocks. No extra inspection requests or authored-state changes.

The real WASM inspection regression covers color-only output, mixed color/scalar zero, scalar removal, SVG readouts, output focus, single-row/narrow/scaled layout, and bounded/on-demand polling. Retest color-only and mixed attribute stacks on isolated output-disabled examples after rollout.


## Final recovered-queue rollout verification — 2026-10-31

The eight implementation requests from recovered queue items 17–24 are active through commit `dbe086bb67d3a4ba0f80842be758a2bddefe2355`; item 25 received its requested Reference-node design only, with no implementation started. The signed final release `dbe086bb67d3-52492b1b0c4f-47` includes accented branding (`fbd01c4`), breadcrumbs (`9471cae`), and scalar-only numeric readouts (`dbe086b`). Browser and macOS CI passed.

Verified current on local desktop 8797, Studio 8788, scene-lab Carpushit 18798, and the configured Orange Pi at 10.0.0.2. Independently verified release signature and hashes of served app/mobile/inspection/breadcrumb assets against committed source. Exact project, undo and redo hashes were unchanged across activation. Revisions/undo counts remain 8797: 0/0; 8788: 203/10; 18798: 8/8; Pi: 36/34. Pi output remains enabled and its updater reports OLA unchanged. Read-only live 18798 browser smoke confirms Schéma-s branding, Root breadcrumb and scalar-dependent widgets with no page errors. Private hash-only evidence is retained at `target/deploy/final-queue-rollout-verification.json`.

Public Vercel publication remains blocked: delivery reports `needs_auth`, requiring the dedicated Vercel deployment token already requested from the user. Do not describe the public app/downloads as updated to this commit. Scene-lab may retest activated features on isolated output-disabled engines; live scenes and history remain untouched.


## Public deployment credential connected — 2026-10-31

The user connected a dedicated Vercel deployment token through the local masked field. Project access was validated before saving directly to Windows Credential Manager; no secret was added to repository files, logs or command arguments. Started the existing approved Schéma automatic updates scheduled task after finding its supervisor stopped; health endpoint is healthy.

The public-publication blocker above is resolved. Updater now reports `current` for signed release `dbe086bb67d3-52492b1b0c4f-47`, source `dbe086bb67d3a4ba0f80842be758a2bddefe2355`, at https://schema-s.vercel.app/app. Browser and download manifests independently return the same commit; production verification streamed and matched all Windows x64, Linux x64, Linux ARM64 and unsigned macOS universal downloads against expected lengths/checksums. Existing local/Pi deployments and scene data were not changed for credential setup.
