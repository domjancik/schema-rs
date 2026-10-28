# Original-style Studio block controls

Source reference: `C:/Users/magne/dev/schema-stack-lab`, commit `e4d6ae8`, `dist/app.mjs` / `dist/style.css`. Reviewed with the **Analyze Schéma visual stacking and interactions** chat. Studio retains the engine's authored-state, revision and undo contracts.

Each block has persistent top-right controls in this order: **fold → grab ⠿ → delete ⌫**. Leaves have no fold control. Fine-pointer grab/delete are 23×22 canvas pixels at right 5/top 4 with a 2px gap; fold sits at right 57/top 5. Labels reserve their space, category icons stay 14px, rounded corners are 6px and hover/focus increases visibility. Coarse-pointer controls use 32px canvas buttons and more header space. They scale with canvas zoom; they do not guarantee 32 physical screen pixels at low zoom. schémaboy retains its separate A/B and touch interaction model.

- Fold selects the node and hides/shows inputs locally. It changes no authored state or undo entry.
- Grab and block-body dragging share a 5 screen-pixel threshold. Default is branch; the layer bar's Branch/Block drag preference persists locally. Shift at pointer-down forces block-only movement. Block-only moves promote old inputs and carry attached parameter drivers. Escape cancels. A completed drag suppresses synthetic clicks for 250ms. Existing plus slots and ghost projection retain their interaction priority.
- Delete opens the same chooser as inspector Delete… and Delete/Backspace in a Stack panel. Cancel has initial focus. Block only promotes ordered inputs and removes drivers owned by the deleted block. Full branch removes the subtree. Block-only is disabled for a root with multiple inputs. The surviving parent remains selected; root deletion falls back to the resulting root or empty layer.
- Driver-root deletion uses the existing driver-removal command. One-child driver-root promotion submits an immutable layer replacement to preserve its surviving IDs. Invalid nested driver contexts unwind to the nearest surviving context. Inactive driver panes render their actual driver tree before controls activate it.

A chooser captures fixture/layer/revision when opened. Concurrent edits cause native revision rejection and keep the chooser open with an error; it does not silently delete a newer tree. Each successful delete is one engine transaction and supports shared undo/redo and the existing serialized history. No new project format or native deletion API is introduced.

Verification: `tests/block-chrome-browser.cjs` uses an isolated output-disabled engine and checks geometry, icon/label separation, leaf/fold behavior, cancellation, root ambiguity, nested-parent selection, block/branch deletion and undo, real grip moves/Shift override/threshold/Escape, preference persistence, touch activation, stale 409 rejection, inactive driver controls and ID-preserving driver promotion/full undo. Authored-state assertions poll the actual wire publication with a bounded timeout. Existing indexed creation/ghost-drop, layout/editing, layer and workspace/camera regressions are also run. The live Studio is checked read-only; test deletion never runs against its project.


## Fold motion

Studio folding tucks visible inputs into the parent rather than removing them in one frame. Unfolding fans them back out with a short stagger and restrained spring settle. The block shell and fold glyph give a small squash/twist response. The parent's upper-right corner stays pinned in the active canvas, keeping the fold control under the pointer. Reversing mid-animation starts from the current visual positions.

Disappearing cards are temporary inert, aria-hidden visual copies without model IDs or controls. Entering cards become interactive after settling; the parent stays responsive throughout. Keyboard Space/Enter toggles the focused fold without triggering transport shortcuts. Context/structural changes and drag previews cancel outstanding motion; hidden-page and reduced-motion changes clean it up immediately. Reduced motion uses an immediate toggle with no ghost cloning.

Work is bounded to 96 cards, prioritizing the folding subtree. Larger trees keep their correct final layout without pretending uncaptured cards are new entries. Web Animations runs only during transitions (approximately 220–360ms); no permanent frame loop, engine command, authored mutation or undo entry is added. Motion has tests for exits/entries, anchoring, rapid reversals, cleanup, layer switching, keyboard operation and reduced-motion interruption. schémaboy's separate SVG stack presentation is unchanged.


## Fractal and square layouts

Open **Layout** in a Stack editor panel header. **Sibling layout → Fractal** makes each group of input siblings share its parent's width, including a proportionally scaled gap. A single input keeps its parent's size; every additional fork scales recursively. **Expanded** keeps the original leaf-column layout. **Block shape → Square** makes every visible card's width equal its height; **Rectangle** keeps the original card proportions. The two choices can be combined.

These are local workspace preferences saved independently per panel, including duplicate Stack views and driver trees. They do not change project data, evaluation or scene undo history. Expanded + Square keeps combiners wide enough for all inputs, so those combiner squares are taller. Fractal + Square keeps the complete tree within the root's width. Zoom supports up to1600% for inspecting small branches; clicking a card still opens its full inspector.

Geometry drives card faces, scaled connection tabs, insertion buttons, drag seams, ghost previews, fold anchoring and Fit together. The logical child order and Cursor column mapping stay unchanged. A dense fractal view intentionally makes distant branches small; zoom in for precise pointer edits.


Fit includes card/tab extents and reserves space for the canvas controls. It can zoom below25% for tall expanded square trees, resets canvas scroll offsets, and saves the resulting scale in the panel. Manual zoom keeps that fitted scale continuous. This addresses lab finding SL-007; the original Carpushit retest remains with the lab.
