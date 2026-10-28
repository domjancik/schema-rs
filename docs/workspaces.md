# Studio panels and fixture layers

Studio stores a versioned workspace layout in browser storage, separately from authored project data and engine undo. Use the workspace tabs to create, duplicate (New workspace), rename, switch or delete layouts. The current workspace is restored on reload. Use Workspace → Undo layout to reverse layout changes; project Undo remains an engine command.

Every panel header has a view selector and a ⋯ menu. Split side by side or above/below, resize with the divider (pointer or arrow keys), and close a panel to join its space back into its sibling. The last panel cannot be closed. Drag the ⠿ header grip over another panel: its center exchanges the view instances; its edges dock the dragged panel beside the target. Ctrl+Space maximizes/restores the focused panel; Escape restores it. Header double-click also toggles maximize.

Available views are Stack editor, Fixture setup, Scene view, Inspector, Fixtures, Output, Telemetry and 3D preview. Multiple instances are supported. Stack instances retain their own fixture, layer, selection, folded branches, pan/zoom and driver context. The active stack uses the full existing editor and shared inspector. Other visible stacks remain selectable/editable and can accept cross-panel block drops. Scene views retain their own projection, pan and zoom, and consume live engine snapshots. Layout handles remain at authored fixture positions while separate live dots follow animation. A 3D preview uses its own rendering instance. Fixture setup retains the original scene/fixture/output controls.

Inspectors follow the current stack selection by default. Pin an inspector with its header checkbox to keep its captured fixture/layer/node while other views change. The first unpinned inspector retains the full existing driver and visual-curve editor. Additional/pinned inspectors provide direct property editing; they do not run or mutate the graph locally.

## Layers

Use Split layers to open the Layers workspace with separate stack editors, or Single layer to return to Authoring. Each layer has an enable checkbox, also exposed in each stack panel header. Disabling a layer skips its evaluation, output contribution, emitters and actions while retaining its authored program and runtime state. Enabling/disabling is one undoable engine transaction. The global engine clock continues while a layer is off.

The + control adds an independent empty layer to the selected fixture or Director, up to32. Other fixtures retain their own counts. Layer1 remains the fixture output; higher layers are independent programs available to layer-reference blocks, retaining the existing evaluation semantics. This change does not invent additive mixing. A Layers workspace shows up to12 layer panels plus an inspector; any panel can select another layer from its header.

Old three-layer projects and enabled stacks remain compatible. Enabled is omitted from JSON/YAML unless false; muted programs and extra layers persist in content snapshots, project saves and serialized undo/redo sessions. Content application grows a destination fixture's layer list when necessary, within the32-layer bound. Older binaries that require exactly three layers cannot load projects with extra layers or the new disabled flag; failed update/rollback validation must retain the newer compatible engine.

In schémaboy, the layer control cycles through the actual fixture layer count. Hold/Shift with that control, or Help → Layers, opens selection, enable and add controls. The single-viewport layout is retained.

Workspace limits are16 panels per workspace and12 named workspaces. Layout preferences are currently per browser, rather than a shared project layout or an OS window manager.
