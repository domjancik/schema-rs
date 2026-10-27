# Component plugins

Component packages extend the running engine with reusable ScenicScript graphs. Common math, color, spatial, dimming, animation and lighting nodes execute in native Rust. Use a static `Script.Value` graph node for custom bounded Rhai math. This version does not load DLLs, arbitrary Rust machine code or WASM.

## Load a package

Create `<project>/plugins/gain/plugin.json`. Copy the working example from `examples/plugins/gain/plugin.json`, then open **Components** in Studio or schémaboy. Enter `gain` and choose **Load package**. The library gains `Plugin.demo.Gain`, with its description, category, icon and editable/drivable Gain property. Unloaded component instances remain in authored stacks and display an engine diagnostic with no output until the package is loaded again.

The same engine operations are available from a terminal, including while a TUI is connected:

```powershell
schema-rs plugins list --server 127.0.0.1:8787
schema-rs plugins load gain --server 127.0.0.1:8787
schema-rs plugins reload demo --server 127.0.0.1:8787
schema-rs plugins unload demo --server 127.0.0.1:8787
schema-rs plugins watch off --server 127.0.0.1:8787
schema-rs plugins watch on --server 127.0.0.1:8787
```

`SCHEMA_TOKEN` supplies the same authentication as other CLI operations. `GET /api/plugins` returns package status, available project folders, watcher errors and persistence errors. The four POST endpoints are `/api/plugins/load` (`{"folder":"gain"}`), `/reload` and `/unload` (`{"id":"demo"}`), and `/watch` (`{"enabled":true}`), each prefixed with `/api/plugins`. They retain the normal token and same-origin guards. HTTP clients do not choose arbitrary filesystem paths.

## Manifest and instances

```json
{
  "api_version": 1,
  "id": "demo",
  "version": "1.0.0",
  "components": [{
    "name": "Gain",
    "description": "Scale incoming values with a native map.",
    "category": "Plugin",
    "domain": "Value",
    "defaults": {"Gain": 2.0},
    "graph": {"Modify.Value.MapTo": {
      "Minimum": 0.0, "Maximum": "$Gain",
      "in": [{"Plugin.Input": null}]
    }}
  }]
}
```

Use it in a ScenicScript stack after loading its package:

```yaml
name: Component example
stack:
  Plugin.demo.Gain:
    Gain: 3
    in:
      - Create.Value.Constant: {Value: 0.4}
```

This produces 1.2 before fixture dimming/mapping. `Plugin.Input` reads the instance's incoming channel bank. It is an internal graph node and is not offered as an independent library block. Native graph input order and separate driver trees are retained. Other loaded components may be referenced by key; dependencies must exist and enabled packages cannot depend on unloaded packages.

Declared defaults define the instance property schema: finite numbers, booleans, strings and complete XYZ vectors. Bind a whole graph property with `"$Gain"` or `"$Position"`; partial vector element bindings are rejected. `"$$literal"` escapes a leading dollar. A static Rhai node can read `params["Gain"]`, alongside `input`, time, fixture coordinates and the existing script context. Dynamic script source parameters are rejected so compilation stays outside evaluation.

Project-relative File/Resource references inside graphs are discovered by the existing input service, including paths bound from instance parameters. The same resource path containment rules apply. Structural `Transition.Fade` and `Modify.Memorize` nodes must remain in authored stacks; they cannot address virtual component nodes. Custom component enums/ranges and native binary plugins are not implemented.

## Reload, history and persistence

Hot reload is enabled by default for loaded packages. The watcher polls at 250 ms and requires two stable reads. Compilation and project/history validation run outside the frame thread. The engine swaps a complete prepared registry at a frame boundary. It rejects stale edits prepared against another registry generation. Browser catalogs follow the separately published plugin generation even when the authored revision has not changed.

Invalid JSON, scripts, dependencies or schemas retain the last working registry and report an error. Schema changes are also checked against current host stack inputs and every undo/redo project; removing a property or component still needed by history is rejected. Unloading retains definitions for validation, export, session restore and undo, while disabling execution and hiding catalog entries. It never deletes scene instances. A package used by another loaded package cannot be unloaded first.

Load/unload/reload are runtime configuration changes, not authored edits, and create no undo transaction. Undo restores authored scenes using the currently loaded implementation. Changed component graphs reset their internal runtime state; unrelated components retain theirs. Internal state is isolated per fixture, layer and component instance. A reload of identical definitions preserves runtime state. Existing clocks and ordinary native node state continue.

`registry/config/plugins.json` stores enabled flags, watcher preference and last working manifests. `.session/session.json` embeds the registry alongside authored projects, undo/redo, clock and native component runtime state. Restart therefore does not require source files to recover the last working scene; the watcher reports missing/invalid source files and keeps recovered code. Automatic session recovery continues to disarm physical output. Saved layout/content files keep their existing boundaries and contain component instances rather than inlined implementation graphs; copy `plugins/` and the plugin configuration when transferring the project.

Limits: safe folder/ID names up to 64 characters, API version 1, 1 MiB manifests, 4 MiB registry, 64 packages, 256 components, 64 parameters per component, dependency depth 8 and expanded graph size 4096. Existing parse, evaluation and Rhai execution budgets also apply. The runtime catalog and registry are isolated per engine.
