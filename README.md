# Schéma / Rust Engine Studio

A working headless Rust engine with a hosted web editor and a remote ASCII TUI. Both clients edit the same engine-owned project through validated commands. Animation and Art-Net dispatch run independently of UI rendering.

The engine includes all 178 documented public node keys plus eight Rust extensions. Source-level behavior, adapters and remaining conformance limits are described in [the catalog port](docs/catalog-port.md). Native gamma XML project files are not imported.

## Run

Requires Rust 1.88+.

```powershell
cd ~/dev/schema-rs
cargo run -- init --project ./schema-project
cargo run -- serve --project ./schema-project
```

Open **http://127.0.0.1:8787**. In a second terminal:

```powershell
cargo run -- tui --server 127.0.0.1:8787
```

The TUI is a native remote client of the hosted engine, not a separate evaluator. It can run over SSH or connect to a reachable server. The web client is embedded in the Rust executable; no Node build step is required.

```powershell
cargo run -- tui --server 127.0.0.1:8787 --snapshot
cargo run -- validate examples/spatial-wash.scnq.yaml
cargo run -- capture
```

Art-Net starts disabled. Captured universe buffers and live fixture results remain available in the UI. To explicitly send to a configured unicast receiver:

```powershell
cargo run -- serve --artnet 192.168.1.50:6454
```

The web output panel can also configure and enable the destination. Channel addresses are 1–512; Art-Net universes are 0–32767. Dimmer, RGB and RGBW mappings are native. Overlapping fixture routes and out-of-range footprints are rejected.

For LAN hosting, set a token of at least 16 characters and bind explicitly:

```powershell
$env:SCHEMA_TOKEN = 'replace-with-a-private-random-token'
cargo run -- serve --bind 0.0.0.0:8787
```

Enter the same token in the web **Connection** dialog, or set SCHEMA_TOKEN for the TUI. This is a trusted-network HTTP service. Internet exposure requires a TLS/authentication proxy. Loopback is the default; API responses reject foreign Origins and unauthenticated LAN requests. No public hosting provider is provisioned by this repository.

## What works

- Scene editing: drag fixtures in X/Z, edit XYZ numerically, rename, group, add/remove, choose model/universe/address and adjust fixture dimmers. Studio and mobile keep selection handles at layout positions while separate dots preview animated positions.
- Native scalar/color sources and transformations, value maps, fixture-referenced and fixed-point spatial distance/axis effects, three independent layers and Layer Above.
- ScenicScript YAML import/export with ordered repeated children and separate driver trees; supported properties are validated.
- Stack add/wrap/delete, single-block/full-branch moves, local folding, web pan/zoom, four ASCII zoom levels and property editing and LFO-driver attachment.
- One shared undo/redo history for accepted authored edits, transactional batches and partial content application. Invalid edits preserve the running model.
- Bounded Rhai scripting through Script.Value; native Rust handles common operations.
- Load, unload and hot reload [component packages](docs/plugins.md) from Studio, mobile, CLI or API without clearing authored undo history.
- Separate layout/content saves, sparse selected-fixture/layer content, group-restricted content loading, standalone stack YAML and project resources/configuration folders.
- ArtDmx packet encoding, independent UDP output worker, captured channel inspection and blackout.

The web palette, child-above-parent layout, fixture sidebar and progressive disclosure follow the sibling [Stack Lab design](../schema-stack-lab/README.md). This implementation adapts that direction to an engine-backed editor; it does not copy its browser-local evaluator or claim every prototype interaction.

## Web controls

Blocks support [copy/paste and driver transfers](docs/block-clipboard.md), including Ctrl/Cmd+C/X/V, explicit block/branch modes, and one-step undo for cross-layer moves.

Choose **schémaboy ↗** for the [portable mobile view](docs/mobile-view.md), also available directly at `/mobile`. It uses the same project, engine, catalog and undo history; **Studio view ↗** returns to Studio with selection preserved.

The header’s **Dark mode / Light mode** button switches the entire studio palette, including blocks, inspector, dialogs and forms. It follows the system theme initially and remembers an explicit choice in this browser. Theme changes do not edit the project or enter undo history.

For live frontend development without restarting an active engine, run `node scripts/dev-web.cjs` and open http://127.0.0.1:8788. This serves local UI files and proxies API requests to the existing engine on port 8787. A normal Rust rebuild embeds the same assets for the next engine launch.

Select a fixture; Shift-click extends the application selection. Drag stage fixtures to move them (one committed edit per completed drag). The inspector edits name, XYZ, groups, dimmer and routing. Switch Layer 1/2/3 independently.

Click a stack block to inspect properties. **+** appends an input; **Insert below** wraps the selected branch. Drag a block onto another to append its branch; hold Shift at drag start to move only the block, leaving its old inputs in place. Hold Alt when dropping to insert below the target. Cycle-forming moves are rejected by the engine.

Distance blocks provide a **Fixture** dropdown. It shows scene names and stores stable IDs; referenced fixtures continue resolving after renaming. An incoming Create.Reference.Fixture child overrides the selector. Spatial.Distance keeps its fixed Position fallback when no fixture is selected. See [reference resolution](docs/fixture-references.md).

Use **∿** to add/edit a numeric property driver and **×** to remove it. Driver trees are separate programs. Import/export uses ScenicScript, not the internal API JSON model. Apply to selection/all/group changes only the current layer. The Apply dialog also saves selected fixture/layer content and loads saved content with an optional group restriction.

F fits; +/- zoom; Delete removes a block; Ctrl+Z/Ctrl+Shift+Z undo/redo. Pause stops animation clock advancement. Blackout overrides encoded output without changing authored dimmers or entering history.

## Terminal controls

| Keys | Action |
|---|---|
| Tab | Scene / stack view |
| , / . | Previous / next fixture |
| 1 / 2 / 3 | Independent fixture layer |
| + / - | Four levels: topology, names, properties, controls |
| Arrows in scene | Move X/Z by 0.25 m |
| y / Y in scene | Raise / lower Y |
| [ / ] in scene | Fixture dimmer |
| e / n / Delete in scene | Edit fixture JSON / add / remove |
| Arrows in stack | Child / parent / siblings |
| a / w | Add input / wrap node; enter behavior key |
| p / d | Edit property literal / attach LFO driver |
| Delete / D | Remove block / remove branch |
| x / X then v / V | Mark single/block branch, then append / insert below target |
| i / o | Import / export ScenicScript file |
| u / r | Undo / redo |
| Space / b | Pause clock / blackout |
| s / q | Save project / quit client |

The TUI prints ASCII only. File import/export occurs on the client machine; project saves occur in the server's configured project folder.

## Project structure

```text
schema-project/
├── registry/
│   ├── config/output.json
│   ├── config/window/
│   ├── layouts/init.json
│   ├── content/init.json
│   ├── stacks/*.scnq.yaml
│   ├── mapping/
│   └── favorites/
└── resources/
```

Layout stores fixture setup. Content stores fixture-ID/layer/program attachments, including explicit empty layers. Sparse content files leave unmentioned layers untouched. Stacks remain separate portable YAML files. Save All writes layout/content/configuration and stack copies; it does not erase history. Individual file writes use a temporary file and rename; Save All is not a multi-file atomic transaction. Browser MIDI and project-scoped PCM WAV input are available; speaker/GPU output and complete mapping-file compatibility remain separate concerns.

## Validate

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
node --check web/app.js
```

Tests include real UDP loopback receivers, engine/UI isolation, parsing, driver timing, layers, scene transforms, history, atomic edits and partial persistence. No physical fixture was used. [Browser verification](docs/browser-verification.json) and [architecture](docs/architecture.md) explain the exercised behavior and limits.

Optional browser smoke test: install Playwright for Node, start the server with a fresh demo project, then run `node tests/web-smoke.cjs` from this directory. PLAYWRIGHT_MODULE and CHROMIUM_EXECUTABLE can select an existing installation. The smoke test edits its test project and writes docs/studio.png.

## Block browsing

The behavior library includes category/domain filters, typo-tolerant search, favorites, recently used blocks and keyboard navigation. Each of the 186 blocks has a description, input/output contract, default-property preview and an independent SVG icon. See [block library](docs/block-library.md) for controls and asset locations.

## Light fields and animation

Use Light.Emitter on a source fixture and Light.Receiver on other fixtures to sample point, plane and spot fields in 3D. Native position/group/random ports and timed 3D/scalar Bézier extensions are available in the library. The Bézier inspector supports draggable X/Z handles with full XYZ fields and undo. See [contracts and importable examples](docs/lighting-motion.md).


## Session recovery

The server checkpoints its project, up to 128 undo/redo transactions, clock and native runtime state in `.session/session.json`, separate from portable project files. Normal startup recovers that session with output disarmed; Save project and graceful shutdown flush it. Explicit `--resume-session FILE` supports history-preserving development restarts with the saved output settings. See [session history](docs/session-history.md) for status/export APIs, validation and durability limits.

## LAN access without restarting the engine

The web frontend can expose an existing loopback engine while preserving its running scene and undo history:

```powershell
$env:SCHEMA_WEB_BIND = '192.168.1.106' # Use this computer's current LAN IPv4 address
$env:SCHEMA_WEB_PORT = '8788'
$env:SCHEMA_ENGINE_URL = 'http://127.0.0.1:8787'
$env:SCHEMA_WEB_TOKEN = '<four-digit-code-or-long-random-access-key>'
node scripts/dev-web.cjs
```

A separate frontend bound to `127.0.0.1:8788` can keep serving local clients on the same port. Open `http://192.168.1.106:8788/` or `/mobile` from another device on the same network. Use **Connection** in Studio or **⌘** on mobile to enter the access key. The frontend checks Host and same-origin headers and requires the key for API requests. If the upstream engine also requires a token, set `SCHEMA_ENGINE_TOKEN` separately. Neither credential is logged. Four-digit codes are supported with five failed attempts per client address per minute; longer random keys remain supported. The frontend defaults to localhost; a non-loopback bind without a valid key is rejected.

Allow inbound TCP 8788 for the Node executable on this LAN interface and local subnet in Windows Firewall. This trusted-network HTTP setup does not configure router port forwarding or public hosting.

## Block picker

Studio and schémaboy share the same responsive catalog. On phones it fills the available viewport, with search and horizontally scrollable category chips above a separately scrolling result list. Tap a row's **+** to add once, or **ⓘ** to inspect the description, inputs/output and properties before adding. The details action stays reachable in small viewports; Back keeps the search and filter context. Search does not automatically summon the phone keyboard when the picker opens.

Favorites and recently added blocks are shared between both views on the same browser origin. Domain and implementation filters are under **Filters**. Desktop supports arrow-key browsing and Enter to add. Escape closes the picker (or returns from details on a phone). Opening, filtering, inspecting and canceling make no scene edit; an accepted insertion is one engine transaction and can be undone. Revision conflicts remain visible inside the picker. Clipboard actions remain in the block inspector.

Schémaboy property editors use named enum choices and scene-backed group/fixture selectors. Enum values keep their native numeric representation, fixture selections save stable IDs, and missing references remain selectable until explicitly changed. Opening or canceling the editor never changes the scene; Apply commits one undoable edit.

Schémaboy control fixes are adapted from `schema-stack-lab` commit `ce5a0f7` and its button modifier handling. Shift+A inserts below; Shift+B opens deletion choices; Shift+driver removes an existing driver. These work after slider/encoder focus and through Shift-click. Form typing stays local to the field, control keys are handled once, and repeated A/B/Space presses are suppressed. Keyboard Shift and pointer Hold release independently; blur clears both. Two-finger Hold+A/B, driver and D-pad controls use pointer events with duplicate-click suppression. In Edit mode the modifier gives fine adjustments; in Pan mode the D-pad zooms and the wheel tilts/pans vertically.

Touch slider/encoder gestures have one pointer owner. Other fingers cannot replace the active drag; Hold may still be used for fine adjustment. Slider movement stays relative, jitter/taps create no edit, and release commits once. Cancellation or lost capture discards the preview. The editing encoder tracks vertical movement from its initial Y coordinate and preserves fractional movement between steps.

## Daily output and patching

The [output designer](docs/output-design.md) adds a cyclic daily dimming timeline, extensible ordered DMX slot models (including the two-channel UV COB), and a 512-slot universe view with atomic drag reindexing. [Stack interactions](docs/stack-interactions.md) cover desktop projected drag ghosts, exact insertion points and schémaboy’s width-aware grid Cursor.
