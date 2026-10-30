# Saved format migrations

Projects, runtime sessions, portable bundles, split layouts/content/output settings, and named-project manifests each carry an independent integer `version`. Their current versions are **2**. An ordered Rust registry runs each applicable step before typed decoding and full validation. Native and WASM hosts share it; callers cannot bypass migration by deserializing an embedded project directly.

Version 1 remains readable, including the early project format, plain content-entry arrays and unwrapped output configuration. The 1 → 2 steps freeze historical defaults and add explicit envelopes for split content and output settings. Layout, content, driver trees and output configuration retain their existing separation. Versions advance independently when a format changes.

Sessions migrate the current project, every undo and redo entry, and every named checkpoint. Runtime node state, clock, inputs, embedded components and resource bytes remain intact. Restoring a browser session never passes it through a JavaScript numeric object roundtrip; raw JSON stays in Rust, preserving 64-bit random-generator state exactly. Native recovery and browser recovery retain their existing physical-output disarming policies. Project/bundle import is a single undoable edit with output disabled.

## Original data and recovery

Loading migrates in memory and never rewrites the source. Before replacing a legacy native document, saving validates it and creates an exclusive, byte-for-byte copy in `.migration-backups/` beside that file. Backups have names such as `session.json.v1-to-v2.0.json`; existing backups are never overwritten. A full split-project save checks all existing documents before changing any. Individual files use atomic replacement; a split save is not a crash-atomic transaction across three files. Named-project saves continue using immutable generations and an atomic manifest pointer.

Browser database version 2 adds a `migration-backups` store without eagerly rewriting documents. Replacement and backup of the original IndexedDB record happen in one transaction. Failed writes roll back both. Saved projects and split resources now retain raw JSON strings instead of parsed numeric objects; old records remain readable. Session locks and compare-before-write checks prevent stale saves from replacing another editor's data.

If a migration, validation or backup fails, saving reports an error and retains the source. Unknown/future versions, duplicate JSON keys and unrecognized typed fields are rejected instead of silently losing data. Documents are limited to 64 MiB with bounded nesting. A newer IndexedDB schema also refuses an older app without modifying stored records.

To recover an original native document, stop its owning server first, copy the corresponding backup to a separate working folder, and open that copy with a compatible engine. Version 2 files cannot be opened by older version 1 engines. There are no automatic reverse migrations. Keep the original backups when downgrading. Browser originals are available as complete records in IndexedDB's `migration-backups` store; raw session exports are also supported by `SchemaPlayer.checkpointText()`.

## Inspect or migrate explicitly

```sh
schema-rs migrate --format session --input old-session.json
schema-rs migrate --format session --input old-session.json --output migrated-session.json
```

The first command validates and reports migration steps without writing. The second creates a new file exclusively; it refuses to overwrite an existing output or its input. Other formats are `project`, `bundle`, `layout`, `content`, `output` and `manifest`. Neither command starts the running engine or physical outputs.

`GET /api/storage/versions` reports supported versions. `POST /api/storage/migrate` accepts `format` and exactly one of `text` (raw JSON) or `document` (an already decoded value), and returns the validated document plus a path-specific step report. Prefer `text` for saved files so clients do not round large integers. Raw project and bundle exports are `/api/project/raw` and `/api/projects/bundle/raw`; raw imports use `/api/projects/import` and the `text` form of `/api/projects/bundle`, with an expected revision.

Signed appliance builds advertise migration verification only after checking an archived version 1 session with their actual binary. The updater migrates an independent comparison copy before activating that binary, compares restored history and runtime data against it, and keeps the exact original rollback checkpoint. Failed migration or restore triggers recovery. An incompatible downgrade fails safely rather than discarding history.

## Regression coverage

`tests/fixtures/storage-v1/` contains historical output-disabled projects, a complete runtime session and a portable bundle. Tests cover defaults and idempotence; all retained projects and undo/redo behavior; runtime/provider/component/resource retention; nested future versions; unknown and duplicate fields; byte-for-byte native backups; immutable project generations; CLI no-overwrite behavior; native raw APIs; actual WASM/IndexedDB upgrades, exact 64-bit state and transaction rollback. `tests/update_device_transaction_test.py` covers migrated comparison and updater recovery failures.

For a future format change, increment only the affected version, add an explicit adjacent registry step, retain its older fixtures, and test migration, idempotence, history preservation, rejection and backup/recovery behavior. Changing serde defaults alone is not a migration.
