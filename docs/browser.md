# Browser engine

The browser host runs the same Rust evaluator in a dedicated module Worker. Studio and schémaboy submit the existing revision-checked commands; no HTTP engine, Art-Net or USB driver is required. Computed scene banks and DMX bytes remain inspectable. Physical output, OSC sockets, native folders and filesystem watches belong to native hosts.

The core has no mandatory server/TUI dependencies. `runtime::EngineSession` owns the engine, checks command revisions, advances explicit frame deltas, resolves actions, and encodes/decodes bounded session bytes including undo history. `runtime::Published` is shared by native and browser hosts. Native filesystem session storage is a separate adapter. Browser monotonic time uses web-time; native clocks retain std::time.

The browser host supplies scheduling, wall-clock input, IndexedDB storage and Worker messaging. Each tab has its own session ID and engine; reload restores that session. Named projects are shared within this browser origin, while loading remains an undoable command. Session files and projects are local browser data, not server uploads. Browser storage can be cleared or denied; use Export project to retain a portable JSON copy. Explicit Save reports storage errors. Partial layout/content saves use separate IndexedDB resources rather than native folders.

## Build

Install the `wasm32-unknown-unknown` Rust target and a wasm-bindgen CLI matching Cargo.lock, then run `python scripts/build_browser.py --bindgen PATH_TO_WASM_BINDGEN`. The standalone site is `target/browser-site`; serve it over localhost/HTTPS with `/play` mapped to studio.html, `/mobile` to mobile.html and `/preview` to preview.html. Direct studio.html also works. For public exports use `--source-ref HEAD` after committing the implementation; this takes the web inputs from Git rather than unrelated local edits. The builder collects dependency notices and emits a checksum manifest. Browser CI builds and runs the actual Worker/editor under Chrome. Native features remain the default for existing CLI, server and release builds.

## Embed

```js
import { SchemaPlayer, bindCSS } from '/schema-player.mjs';
const player = await SchemaPlayer.create({ project: myProject });
const stop = player.subscribe(snapshot => drawScene(snapshot.fixtures));
const unbind = bindCSS(player, element, {
  fixture: 'fixture-1', property: '--motion', map: value => value * 100
});
// Later: stop(); unbind(); player.dispose();
```

`SchemaPlayer.create` defaults to an ephemeral engine for embedding. `request`, `command`, `input`, `checkpoint` and `restore` reuse the typed Rust model; commands accept an expected revision. For deterministic embedding or export, use `autoTick: false` and `await player.advance(dt)`; the host supplies every frame delta. Dispose terminates the Worker and rejects outstanding requests. Snapshot consumers may animate DOM, Canvas, WebGL or external application state. No output adapter is assumed by the engine host. The CSS adapter writes only explicit CSS custom properties with finite scalar values. The animation-demo.html page provides a running LFO example.

Browser scheduling is not an unattended real-time lighting clock: background tabs and locked phones may suspend it. A Worker keeps evaluation separate from rendering but does not bypass lifecycle throttling. File-backed plugin loading/watch and native registry folders require a native host; components already embedded in a restored session still evaluate. Chrome acceptance tests cover actual WASM editing, animation, revision conflicts, durable undo, project save/load, mobile transport, isolated deterministic instances and disposal. Safari/Firefox execution remains unverified. The new Worker resource requires WebAssembly compilation permission (`wasm-unsafe-eval`) in its own CSP. Existing public pages keep their prior policy; only the new preview page permits same-origin framing. No general JavaScript eval is enabled.


The shared contract is commands in, frame deltas/host inputs in, and snapshots/session bytes out. Storage and presentation are adapters: IndexedDB is replaceable, and a CSS binding is one snapshot consumer alongside Canvas/WebGL or an application-specific renderer. Fixture IDs/banks remain the current scene data model for compatibility; this change removes host/transport coupling without renaming that model or implying that every native integration is available in a browser.
