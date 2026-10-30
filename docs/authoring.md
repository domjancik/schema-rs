# Authoring tools

Open **Commands** in the application menu, or press **F2**. Search actions by name or related words, move through results with the arrow keys, and press Enter. **State and shortcut help** explains the shared color meanings and keyboard controls. Commands work in Studio and schémaboy.

## Picking and arranging fixtures

Click or tap overlapping fixture markers to open **Choose fixture**. Search the candidates by name, group, ID or universe/address. In schematic views, Alt+click cycles candidates. Layout markers stay at authored positions while animated output moves separately. Labels are placed around their markers without overlapping; labels that cannot fit are omitted, while the markers and overlap chooser remain available. Label placement is cached until geometry, camera or selection changes.

Open **Arrange fixtures** from Commands or the fixture-position dialog. Choose the fixtures explicitly, then Snap to grid, Align or Distribute evenly. Snap rounds X/Y/Z to a grid anchored at the origin. Align affects one axis and uses the active fixture, minimum, center or maximum. Distribution preserves the endpoints and spaces intermediate fixtures along the chosen axis. Apply submits one revision-checked batch; Undo reverses the arrangement as one edit. Existing gizmo snapping and numeric XYZ editing remain available.

## Comparing a block

The block inspector provides **Bypass block** and **Focus output**. Both are searchable commands for the selected block.

Bypass is authored state: the block passes through its merged inputs, skips its own operation and property drivers, and retains its children. The setting survives project/session saves, copy/paste and ScenicScript export as `bypassed: true`. It is undoable. A bypassed source with no inputs produces an empty bank. Re-enabling continues evaluation at the current engine time.

Focus output is a temporary engine debug mode, inspired by the original Schéma node-debug workflow. The chosen block’s already-evaluated bank replaces that fixture’s root output, including its position, value, color and emitters. The normal program, other fixtures, layer dependencies and actions continue to evaluate; focusing never evaluates a stateful block twice. Property-driver blocks can be focused too. The amber state banner and **Stop output focus** restore normal output. Focus does not edit the project or create an undo entry, clears when the block disappears, and is deliberately not restored on restart. Director blocks do not directly drive fixture output.

Selection is blue; enabled evaluation uses mint checks; bypass is dashed and labeled; debug output uses amber. Capture-only/browser preview, live output, pause, warnings and blackout have explicit labels. Actual fixture colors remain output data rather than selection indicators. Scene previews also show physical-output/debug state.

## History and checkpoints

**History and checkpoints** shows the retained engine timeline, its current position and redo entries. Choose an entry to jump atomically; this does not replay intermediate physical frames. History is shared among connected operators and limited to 128 transactions. New authored edits clear redo. If another operator changes the scene, refresh the list rather than applying an outdated history offset.

Create up to 32 uniquely named checkpoints of authored project state. They survive session persistence and can be restored after further edits. Restore is one undoable project replacement, with physical output disabled. Checkpoints do not rewind animation time, native runtime state or external devices. Removing a checkpoint changes session metadata rather than creating an authored undo entry.

## Portable project bundles

Projects and Commands offer **Export portable bundle** / **Import portable bundle**. A `.schema-bundle.json` file contains the authored project (layout, programs, models and daily dimming), embedded graph component packages, provider resources and raw files from the project’s `resources/` directory. Native export also includes analyzed WAV data, so supported sample blocks can evaluate in the browser without filesystem access.

Imports are fully validated and prepared off the native evaluation thread, then committed atomically with a revision check. Existing undo history and named checkpoints are retained; Undo restores the previous scene. Resource and component names shared with existing state must have identical contents. Rename conflicting resources/packages, or use a fresh session. This preserves resources required by undo entries. Imported files remain embedded in the session rather than being extracted onto the host filesystem; no executables or native libraries are loaded.

Bundles omit access credentials, network pairing details and the source session’s undo/runtime history. Physical output is disabled on import. Raw resources are limited to 256 files and 8 MiB combined; the JSON import limit is 64 MiB. Symbolic links, absolute paths and traversal paths are rejected. Large media and arbitrary native plugins require a future resource/package transport; current components are bounded graph/script definitions.

Workspace arrangements and temporary meshes imported only into a preview are view preferences; they are not authored project data. Place durable project files in `resources/` before exporting a bundle.


Blocks now show evaluated color/attribute previews. Inspectors show incoming and outgoing banks separately; schémaboy exposes them through the swatch/value button beside the selected block. See [live block attributes](block-inspection.md) for sampling and performance limits.
