# Saved projects

Open **Projects** in Studio or the **SAVE / Projects** control in schémaboy. Choose **New saved project**, enter a display name and filesystem ID, then **Save new project**. Select an existing entry to **Save changes** or **Load selected project**. IDs use letters, digits, hyphens and underscores. The selected save target is remembered in that browser; it is a library selection, independent of engine undo.

Saving captures one published revision and does not alter the running scene, clock or undo history. Overwriting is explicit through selecting a saved entry and using Save changes. The API rejects an existing ID unless overwrite is true. Concurrent authored changes cause a revision conflict; refresh and retry.

Loading replaces the whole scene for all connected clients as one validated engine command. It resets node runtime, keeps the global clock, and disables Art-Net output. Undo restores the previous scene and output configuration; redo restores the loaded scene. The existing durable session preserves this history across restarts. Invalid or stale loads leave the running project unchanged.

Each saved project retains its own fixture layout, DMX models, daily dimming, content, director programs, stack exports and output configuration:

```text
<workspace>/projects/<id>/
  project.json                 # atomic pointer + listing metadata
  revisions/<generation>/
    registry/layouts/init.json
    registry/content/init.json
    registry/stacks/*.scnq.yaml
    registry/config/output.json
```

Generations are immutable. The manifest is replaced only after all files have been written, so a failed save cannot point at a partially written generation. Previous generations remain on disk; there is currently no pruning or version-selection UI. Resource files and installed component packages remain shared by the hosted workspace under resources/ and plugins/; named saves are scene storage, not portable media/package bundles. Loading validates against the currently loaded component registry.

Existing paired registry/layouts/<name>.json and registry/content/<name>.json snapshots appear as legacy entries. Layout-only and selected-content save/load controls continue using their existing folders. Saving a legacy entry creates a modern named project without changing its old snapshot files.

API: GET /api/projects; POST /api/projects/save with id, title, overwrite, expected_revision; POST /api/projects/load with id, expected_revision. Existing token and same-origin guards apply. Disk I/O and validation run outside engine evaluation; IDs and filesystem containment are checked.
