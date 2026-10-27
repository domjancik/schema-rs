# Block clipboard and driver transfers

Studio's block inspector offers **Copy block**, **Copy branch**, **Cut branch**, **Paste**, **Paste options…**, **Move to…**, and **Move out of driver** while a driver is open. Empty layers offer **Paste into empty layer**. The schémaboy block inspector and block-library actions expose the same clipboard and movement operations.

| Shortcut | Action |
| --- | --- |
| Ctrl/Cmd+C | Copy full branch |
| Ctrl/Cmd+Shift+C | Copy block only |
| Ctrl/Cmd+X | Cut full branch |
| Ctrl/Cmd+Shift+X | Cut block only |
| Ctrl/Cmd+V | Paste as input, or begin an empty layer |
| Ctrl/Cmd+Shift+V | Open destination/placement options |

Text fields retain their normal text clipboard behavior. Copy writes a versioned `schema-block` JSON envelope to the system clipboard and a browser-local fallback. Paste reads the system clipboard when permitted; the browser fallback supports local use when clipboard access is unavailable. Paste options also accepts manually supplied block JSON. No clipboard read happens until an explicit paste action.

A **block-only** copy keeps its properties and attached parameter-driver trees, with ordinary input children omitted. A branch copy includes ordinary descendants and all driver trees. Pasted copies receive fresh IDs throughout. Parameters, references and explicit random seeds retain their authored values.

**Move to…** chooses a destination fixture, layer, block, placement and optional driver property. Append, insert below, property-driver and empty-layer placements are supported. The picker labels driver paths as well as ordinary inputs. Existing drivers are protected unless **Replace the existing driver (undoable)** is checked; choosing a node inside that existing driver and appending there is another option. Numeric and boolean driver properties follow catalog exclusions and server validation.

Moving a single block promotes its ordinary input children at the old position; its own parameter drivers travel with it. Moving a driver root out removes its driver attachment. A single-block driver-root move can promote its only input back into the driver slot; a root with multiple ordinary inputs must move as a full branch because promotion would be ambiguous. Whole layer roots can move to another valid destination, leaving the old layer empty. Cycles and moves into the moving node's own subtree are rejected.

Studio also accepts drag/drop onto a property’s ∿ button. Existing-driver replacement opens the explicit destination sheet. Dragging driver-tree nodes onto ordinary blocks moves them back into the stack; **Move out of driver** appends the selected driver branch to its owner's ordinary inputs. Source and target IDs are preserved within one layer. Cross-layer moves receive IDs unique to their destination. Nested driver navigation retains a context trail.

A cut is deferred: it leaves authored state untouched until successful paste. Cuts capture source identity and revision; a stale revision is rejected rather than removing an unexpected node. Navigating between fixtures/layers or variants does not itself invalidate a cut. Successful cut/paste clears the cut marker; subsequent pastes are copies.

`block-edit.js` builds immutable drafts of affected layer programs. The UI submits one existing `Batch` of `ReplaceLayer` commands with an expected revision. Rust validates the resulting project, scripts and driver contracts before installation. Source and destination therefore commit together as one undo transaction, including cross-fixture/layer edits. Rejected edits leave published state unchanged. No new engine command or live-engine restart is required. Browser polling keeps the dragged DOM stable through the gesture.

Clipboard bounds: 512 KiB of JSON text, at most 2048 nodes and 64 tree levels, finite safe-integer IDs and known structural fields. The engine additionally validates supported behaviors, properties, script budgets and final program limits.

Verification: `node --test tests/block-edit.test.cjs`; `SCHEMA_URL=<isolated capture-only frontend> node tests/block-clipboard-ui.cjs`. The browser regression uses the real clipboard, drags onto a driver, moves into/out of driver scope, cuts across fixtures/layers, checks exact undo restoration, and pastes in schémaboy. It does not edit the live studio scene.
