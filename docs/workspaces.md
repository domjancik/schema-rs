# Studio panels and fixture layers

Studio stores a versioned workspace layout in browser storage, separately from authored project data and engine undo. Use the workspace tabs to create, duplicate (New workspace), rename, switch or delete layouts. The current workspace is restored on reload. Use Workspace → Undo layout to reverse layout changes; project Undo remains an engine command.

Every panel header has a view selector and a ⋯ menu. Split side by side or above/below, resize with the divider (pointer or arrow keys), and close a panel to join its space back into its sibling. The last panel cannot be closed. Drag the ⠿ header grip over another panel: its center exchanges the view instances; its edges dock the dragged panel beside the target. Ctrl+Space maximizes/restores the focused panel; Escape restores it. Header double-click also toggles maximize.

Available views are Stack editor, Fixture setup, Scene view, Inspector, Fixtures, Output, Telemetry, System logs, DMX test and 3D preview. Multiple instances are supported. Stack instances retain their own fixture, layer, selection, folded branches, pan/zoom and driver context. The active stack uses the full existing editor and shared inspector. Other visible stacks remain selectable/editable and can accept cross-panel block drops. Scene views retain their own projection, pan and zoom, and consume live engine snapshots. Layout handles remain at authored fixture positions while separate live dots follow animation. A 3D preview uses its own rendering instance. Fixture setup retains the scene and fixture controls. Output is a separate panel, placed below the Stack editor in the default Authoring workspace. Its divider resizes the vertical split. Use the Output toolbar button to focus it or add it below a stack in an existing layout. Custom layouts are retained; the original factory layout upgrades once.

On small screens, Studio keeps Undo, Redo and Save in one compact header row. The ⋯ menu contains Connection, Servers and the other application actions. A bottom row of SVG icons and short labels switches between full-width panels. It reserves space below the status strip, including the phone safe area. The active panel is marked; duplicate views are numbered, and additional panels scroll horizontally within the row. Arrow keys, Home and End also switch panels. Selection is remembered per workspace. Workspace → Show split layout restores the complete split arrangement; Focus one panel returns to the focused view. Tapping a bottom-row panel returns from split mode to that single panel. Desktop layouts and divider positions are retained.

Each Output panel has its own destination draft, universe and channel selection. Apply submits a revision-checked engine command. Incoming snapshots do not overwrite edited settings; after a conflict, Reload settings explicitly restores the latest engine configuration. Patch, Models, Dimming and Channel test remain accessible from Output. Live channel levels update independently of stack editing.

Inspectors follow the current stack selection by default. Pin an inspector with its header checkbox to keep its captured fixture/layer/node while other views change. The first unpinned inspector retains the full existing driver and visual-curve editor. Additional/pinned inspectors provide direct property editing; they do not run or mutate the graph locally.

## Layers

Use Split layers to open the Layers workspace with separate stack editors, or Single layer to return to Authoring. Each layer has an enable checkbox, also exposed in each stack panel header. Disabling a layer skips its evaluation, output contribution, emitters and actions while retaining its authored program and runtime state. Enabling/disabling is one undoable engine transaction. The global engine clock continues while a layer is off.

The + control adds an independent empty layer to the selected fixture or Director, up to32. Other fixtures retain their own counts. Layer1 remains the fixture output; higher layers are independent programs available to layer-reference blocks, retaining the existing evaluation semantics. This change does not invent additive mixing. A Layers workspace shows up to12 layer panels plus an inspector; any panel can select another layer from its header.

Old three-layer projects and enabled stacks remain compatible. Enabled is omitted from JSON/YAML unless false; muted programs and extra layers persist in content snapshots, project saves and serialized undo/redo sessions. Content application grows a destination fixture's layer list when necessary, within the32-layer bound. Older binaries that require exactly three layers cannot load projects with extra layers or the new disabled flag; failed update/rollback validation must retain the newer compatible engine.

In schémaboy, the layer control cycles through the actual fixture layer count. Hold/Shift with that control, or Help → Layers, opens selection, enable and add controls. The single-viewport layout is retained.

Workspace limits are16 panels per workspace and12 named workspaces. Layout preferences are currently per browser, rather than a shared project layout or an OS window manager.

## Fixture setup navigation

The Fixture setup map supports dragging empty space to pan and scrolling to zoom around the pointer. On touch screens, drag empty space with one finger or pinch/pan with two. The **Pan** toggle lets a drag move the view even when it starts on a fixture; turn it off to place fixtures. Mouse middle/right drag also pans. The −/+ buttons zoom, **Fit** frames all authored fixture positions and **Reset** returns to the original top-down view. When the map has keyboard focus, arrows pan, +/− zoom, F fits and Home resets.

The main setup camera is a per-browser preference restored on reload. Navigating, fitting and resetting do not edit the project or add undo entries. Dragging a fixture with Pan off still submits one undoable move, using world coordinates at the current pan/zoom and preserving height. Placement is no longer clamped to the old −10…+10 map. A two-finger gesture or pointer cancellation cancels an unfinished fixture drag rather than committing it. On overlapping handles, touch picking opens on lift, allowing a second finger to start navigation first. Animated positions remain separate from authored selection handles. The grid and coordinate readout follow the view.

## Fixture marker colors

Set **Marker color** in Fixture setup; Shift-select fixtures in Studio to color-mark them together in one undoable edit. **Clear** returns to the default marker. Schémaboy exposes the same control under its menu → Fixture setup. Swatches appear in fixture navigation/search, setup and scene markers, embedded stack headers and universe patch groups.

Markers are saved layout metadata (`fixture.marker_color`, optional `#RRGGBB`), independent of emitted light color, fixture dimming and DMX. Existing unmarked files stay unmarked. Use color blocks when you actually want to change light output. Marked files require an engine with marker-color support.
