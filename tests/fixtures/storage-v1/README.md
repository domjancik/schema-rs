# Historical storage fixtures

`session-late.json`, `project-late.json` and `bundle.json` were captured through the HTTP API of an unchanged v1 engine binary, in an isolated output-disabled test project. They contain a native random generator, a graph component, undo/redo, a named checkpoint, provider data and embedded bytes. The history includes the original demo prior to bundle import. No end-user scene, credentials or device state is included.

`project-early.json` uses the documented early v1 field set, without later optional fixture/layout fields. These checked-in inputs must remain v1; update expected results rather than regenerating them with the current serializer.
