# schémaboy mobile view

Choose **schémaboy ↗** in Studio or open `/mobile`. **Studio view ↗** returns to Studio. This is a selectable presentation of the same running engine and project. The phone remembers an explicitly chosen mobile variant; Studio remains available. Switching carries fixture, layer, selected block and driver context through client-only session storage, without a model command or history entry.

The source design is the latest schémaboy in `../schema-stack-lab` at commit `188da8e`, confirmed by the “Analyze Schéma visual stacking and interactions” chat. Its [mobile handoff](../../scnq-schema/codex/analysis/schemaboy-mobile-variant-handoff.md) distinguishes the approved compact design from older rejected 3D-block screenshots. The new view reuses that design’s markup/style direction but uses `engine-client.js` for authoritative Rust state and commands. It contains no browser evaluator or independent project history.

- A full-width square scene at widths up to 700px; centered handheld layout on desktop. The page scrolls on short screens and respects safe-area padding.
- **Stack** is a flat, child-above-parent tree. **Linear** and **3D** show real fixture positions/results. A/B, D-pad, wheel, Hold, Select/Edit/Pan, layer cycling and fixture dots provide compact controls.
- The roller previews a relative numeric adjustment with a soft visual response and value fill. Release submits one captured fixture/layer/node/property/revision command. Pointer cancellation, focus loss or hidden-page cancellation discard the draft. Rejected stale edits refresh from the engine. Fine mode is ten times smaller for continuous numeric values; enums stay integral.
- Tap the property/value for exact typed entry, or the block/family row for the full inspector sheet, descriptions and all properties. Numeric drivers can be added/opened with ∿; Hold+∿ removes them. B returns through driver context.
- A browses the actual engine catalog with categories and fuzzy search. Hold+A wraps below the selection. Menus expose copy-to-fixtures/groups, export, fixture XYZ editing, deletion and single-block/full-branch lift-and-place movement. Invalid engine edits retain authoritative state.
- Save writes the engine project. Pause, blackout, undo/redo and live output status use existing engine controls. Merely entering this view never enables output or opens microphone/MIDI devices.

The compact variant uses explicit lift-and-place movement. Direct touch dragging with branch ghosts, projected space-making and indexed sibling insertion remains separate interaction work identified in the handoff. Clipboard and the Move to destination picker now support moving into and out of nested drivers; see [block clipboard](block-clipboard.md). The older direct delete menu still directs driver-root removal through the owner’s ∿ control. Native authored/runtime semantics remain those of the connected engine, including which node catalog version is running.

Files: `web/mobile.html`, `mobile.css`, `mobile.js`, `mobile-core.js`, `view-variant.js`, and shared `engine-client.js`. Rust embeds the same assets; `scripts/dev-web.cjs` serves them live without restarting the engine.

Validation: `node --test tests/mobile-core.test.cjs`; `SCHEMA_URL=<isolated capture-only frontend> node tests/mobile-ui.cjs`. The browser test covers a non-mutating gesture preview, one-edit commit/undo, real touch cancellation, library creation and undo, selection-preserving switches, and square/no-overflow layouts at 320, 390, 430 and 700px plus desktop. Existing catalog/editor/native-input browser tests cover Studio through the shared transport. Screenshots are in `docs/schemaboy-*.png`.

## Direct navigation and toolbar

Keep **Studio ↗** visible in the schémaboy header. Output/DMX, Components, theme, connection, help and fullscreen remain direct top-panel buttons. Small portrait phones use two compact rows; landscape and desktop keep one row. About and installation remain in the secondary menu. Preserve these controls when refining viewport fit.

`tests/mobile-toolbar.cjs` checks direct hit targets and real return navigation at 320×568, 390×844, 568×320, 844×390 and 1360×1000. The viewport, long-label, Output-tab and installation regressions remain applicable.
