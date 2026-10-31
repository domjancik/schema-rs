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


## Transfer projects and content between servers

Open **Projects → Transfer manager** in Studio or Schémaboy. The manager shows **This server** and **Other server**, including their actual addresses and current project names. Each side lists the running project, named saves, content presets, and received portable copies. Search narrows the list.

Choose a saved server or enter the Orange Pi engine address, such as `http://lighting-server.local:8787`, using its wired or access-point connection. Enter that server’s Schéma access key and press Connect. This is the engine key, not the captive-portal administrator PIN. Addresses can be remembered through the existing saved-server list; remote keys remain only in the open dialog and are cleared on close. Native transfers accept private LAN/loopback addresses, with normal certificate verification for HTTPS endpoints. They do not discover or scan the network.

Select an item on This server, review its **Copy ID** and **Display name**, then **Send copy →**. Selecting an item on Other server changes the action to **← Receive copy**. A transfer saves a portable copy; it never loads the project, replaces the destination scene, changes output settings or adds undo entries. Existing portable IDs require an explicit Replace choice and the current content hash. A concurrent replacement is rejected, and successful replacements retain the previous file in library backups. Publishing under an existing conflicting preset name requires a new copy ID.

Received copies appear under **Portable library**. Select a local copy and choose **Load project · output off**, or **Apply content**. A project load restores its resources/component definitions, retains the destination server’s hardware routing, and disables physical output as one undoable change. Content applies its saved fixture IDs/layers, retaining fixture layout, output configuration and unaffected layer runtime state. It does not remap fixture IDs: transfer/load the complete project first when the other server has a different layout. Receiving content does not register it with the active Director: an armed cue could otherwise react merely to a file arriving. Choose **Publish preset** separately to make it available in ordinary Content library/Director selectors. Publishing may cause active cues using that name to run. Existing different presets cannot be replaced by publishing; use a different copy ID. When a preset introduces component dependencies, apply it from the manager to install those dependencies before publishing. Resource or component-name conflicts are reported instead of overwriting existing definitions. An unreferenced workspace startup snapshot (`init`) stays in the archive without replacing the destination’s startup provider; an explicit dependency on that name still receives the ordinary conflict check.

**Download file** and **Import file… → Save file here** provide the same portable workflow without a simultaneous network connection. Files contain authored data, registered assets, bounded resource bytes and component definitions; they do not transfer live undo history, engine credentials or hardware enablement. The complete shared workspace asset set is included, matching portable bundles. Current library files use `schema-library-item` version 1, containing existing versioned bundle/content documents. Older portable project bundles can also be imported. Unsupported future versions fail without changing existing saved data.

The public/browser-only editor supports file download/import and persistent portable copies in IndexedDB. Direct server-to-server connections require a hosted PC/Pi instance; the browser UI offers the file path instead of bypassing browser network protections. Its IndexedDB upgrade adds a library store (schema 3), preserving existing projects, sessions and migration backups.

Native copies are saved as `library/project/<id>.json` or `library/content/<id>.json`; replacements retain `library/backups/…` files. Stored project copies remain distinct from the existing named project saves. Payloads preserve typed numbers as JSON text across JavaScript relays, preventing `1.0`/`1` conversion from creating false resource conflicts. Transfers are limited to 64 MiB and use the existing API authentication/same-origin protections. The peer bridge pins resolved private addresses, refuses redirects/proxies and does not forward a key to another host. Network and disk work stays outside engine evaluation.

API: `GET /api/library`; `POST /api/library/export {key}`; `/api/library/import {item, expected_hash?}`; `/api/library/load {key, expected_revision}`; `/api/library/publish {key, expected_revision}`. Hosted instances also expose `/api/library/peer {peer:{origin,token}, operation:list|export|import, …}` for the fixed library operations only. Direct copies require both engines to support library protocol 1; update older devices before connecting.
