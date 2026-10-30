# Session history and restart recovery

The server writes a local, versioned checkpoint at `<project>/.session/session.json`. This is separate from portable `registry/layouts`, `registry/content`, stacks and configuration. `.session/` is ignored by Git.

## What is serialized

- Current authored project and engine revision.
- Ordered undo and redo stacks, with at most 128 transactions combined. A new accepted edit still clears redo; batches remain one entry.
- Animation clock/frame, pause and blackout controls.
- Native node state: driven values, held banks, integrator accumulators, random generator state/targets/filter progress, and retained movement phase.

Each checkpoint contains its own current project. History is never attached to a separately loaded, unrelated layout/content pair. Compiled Rhai ASTs, sockets and thread handles are not serialized; programs in the current project and history are validated and scripts recompiled before the restored engine starts.

Undo/redo continues to restore authored state and reset node runtime state; it does not rewind time or physical output. Restart recovery is different: it restores the captured runtime state so seeded generators and filters can continue. History remains shared across web/TUI operators, not per-user collaborative undo.

## How it is saved

The owning engine captures a coherent checkpoint after a frame. Historical project mementos use shared immutable references, avoiding deep history copies on every frame. JSON encoding, size enforcement, file synchronization and replacement run on the dedicated session writer thread. The engine and Art-Net worker never wait for disk I/O.

Accepted authored revisions and pause/blackout changes schedule a checkpoint. With no edits, runtime/clock state is captured every two seconds. The writer checks pending work every 100 ms and coalesces to the latest checkpoint. Ordinary command acknowledgment means committed in memory, not yet durable on disk. Abrupt termination can lose pending edits or up to about two seconds of runtime progress; slow or failed storage can extend the window. Check status or explicitly flush before replacing a process.

**Save project**, `POST /api/session/save`, and graceful shutdown explicitly flush. File writes use a unique temporary file, flush/sync, then rename over the previous checkpoint. An invalid or failed checkpoint does not replace the last good file. Writes are serialized; an older frame cannot overwrite a newer one. This is an atomic single-file checkpoint, not a multi-file project transaction or an unlimited command journal. Run one engine process per project directory.

## Recovery and output policy

`serve --project PATH` automatically recovers `PATH/.session/session.json` when present. Recovery errors stop startup rather than silently discarding history. If external registry files have intentionally changed, use `--fresh-session` to load those files with empty history; this intentionally replaces the local checkpoint once running. Back up a checkpoint first if needed.

Automatic recovery preserves the existing disarmed-output startup policy. Output is disabled in the recovered current project and every history entry, so undo cannot inadvertently re-enable an old output configuration. Use the normal output controls or explicit `--artnet IP:PORT` to re-arm.

For an intentional development handover that preserves the checkpoint's output settings, use:

```powershell
schema-rs serve --project ./schema-project --resume-session ./schema-project/.session/session.json
```

`--resume-session`, `--resume-state` and `--fresh-session` are mutually exclusive. Explicit `--resume-session` adopts the complete trusted checkpoint, including its output settings. The legacy `--resume-state` publication format contains no undo/redo and can only reconstruct selected node state; it remains a one-time migration path from older servers. It cannot recover undo entries the older process never exported.

Session schema version is 1. Unknown versions, history over 128 entries, invalid projects/scripts, missing runtime node references, incompatible driven properties and invalid clock/runtime values are rejected. Loading and writing are capped at 64 MiB. JSON nesting is bounded at 160 levels to accommodate valid depth-64 node trees. Session files are trusted local state, not an API upload format or cross-version native plugin-resource snapshot.

## API

All routes use the same existing token, Host and Origin checks as the other engine APIs.

| Route | Result |
|---|---|
| `GET /api/session` | Coherent complete checkpoint, suitable for download or explicit restart |
| `POST /api/session/save` | Wait for a checkpoint to be written to the configured session file |
| `GET /api/session/status` | Persistence enabled, saved revision/frame and last storage error |

Normal `GET /api/state` responses do not include history or private runtime state. Exporting a checkpoint does not edit the project or enter undo history.

## Verification

`tests/session_history.rs` covers undo/redo round trips, branching after undo, seeded runtime continuation, coherent API capture, invalid-version/history/runtime rejection, invalid-save protection, repeated atomic replacement, shutdown flushing, valid deep programs and disarmed recovery across history.

`node tests/session-restart.cjs` starts an isolated capture-only process, makes edits and undo, flushes, terminates the process and starts it again using only its project path. It verifies exact authored recovery, redo/undo ordering, clock continuity and Save project flushing. It never edits the live studio. `SCHEMA_BINARY` selects a built executable; the default is `target/debug/schema-rs.exe`.

## Browsing and named checkpoints

Commands → History and checkpoints exposes the shared retained timeline and atomically jumps to an offset using an expected revision. Named authored checkpoints are separate from the runtime recovery checkpoint described above. Up to 32 names survive restart; restore creates one undoable project replacement with physical output disabled. Embedded resources and graph packages remain available for current and historical projects. Debug output focus is transient and never resumes from a saved session. See [authoring tools](authoring.md).
