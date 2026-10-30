# ScenicScript compatibility

This engine consumes the reference `name` / `stack` envelope, one public behavior key per block declaration, ordered `in` sequences and nested property-driver declarations. Repeated behavior names remain independent children. Import and export are supported for the declared catalogue; unknown behaviors/properties and malformed values are rejected. Optional original envelope metadata is currently accepted but not retained through export.

The selected source reference is `../scnq-schema/Main/Exports/src`, with contracts in `../scnq-schema/codex/blocks/records`. UI design reference: `../schema-stack-lab`, sibling chat “Analyze Schéma visual stacking and interactions.” No source application patches or sibling prototype files were modified.

## Documented native catalog

All 178 public identifiers have executable Rust implementations. See [catalog port](catalog-port.md) for module boundaries, typed banks, adapters, history behavior and known parity limits. The complete source inventory is [documented-catalog.json](documented-catalog.json); [port-coverage.json](port-coverage.json) distinguishes broad catalog checks from focused semantic evidence.

ScenicScript retains ordered children, separate numeric/boolean drivers, enum names or indices and typed XYZ literals. Nodes follow their source's merge/first-child/selection rules. Maps extrapolate, Steps rounds to Count - 1 intervals, and Crossfade accepts all eleven native shapes. Fixture pixel geometry and setup channel are explicit. Complex original .NET literals and arbitrary scalar spreads are not generalized by this importer.

## Rust extensions

| Key | Properties | Meaning |
|---|---|---|
| Spatial.Distance | Fixture, Position: [x,y,z], Auto/Setup | Fixture-target distance when selected or referenced; empty Fixture retains fixed-point behavior. |
| Spatial.Axis | Axis: 0/1/2, Minimum, Maximum | Clamped normalized fixture position along X/Y/Z. |
| Modify.Color.Dim | Amount | Multiply RGB by a clamped 0–1 amount, preserving alpha. |
| Script.Value | Code | Bounded Rhai expression/program returning a scalar. |

These extension keys are not claims about native gamma registrations. Their programs require schema-rs or an extension implementation in another runtime. Most routine work should use native blocks. Use Script.Value for custom math, for example `input * (x + 5.0) / 10.0`; constants/index are exposed as floating-point values.

Drivers retain the inspected export's owner-before-driver timing; they do not use the sibling mock editor's immediate-driver evaluation. Custom driver mapping ranges and non-scalar driver payloads are not serialized by this version. Vector literals for extensions use three-element numeric arrays; complex original typed .NET literal encodings are unsupported.

This is source-informed structural interoperability plus tested Rust behavior, not full Schéma runtime conformance. The repository tests establish explicit cases, including ordered children, typed parameters, driver delay, higher-layer input, map/dimmer-to-DMX output and state-edit history.

## Lighting and motion additions

See [lighting/motion contracts and examples](lighting-motion.md). Position/current/group and random public names are ported with explicit Rust timing/seed behavior; audio modulation uses the native AudioLerp formula with host audio context. Light.Emitter, Light.Receiver, Create.Position.Random and both Bézier nodes are native Rust extensions. Analytic 3D light fields are not physical photometry or rendered fog. Random/tween dependency equivalence has not been measured against native Schéma.

## Component packages

[Component plugins](plugins.md) add engine-scoped `Plugin.<id>.<name>` nodes implemented by native graphs and bounded static Rhai scripts. They require schema-rs and its loaded package registry. This is not a native gamma plugin ABI or binary dynamic loader. Unloaded definitions remain available for scenes, serialization and history, while execution returns no output with a diagnostic. Structural transitions/memorize inside templates, partial vector bindings, and dynamic script source parameters are rejected.

Modify.Repeat preserves source-derived frame playback modes, duplicated bank capture and the 402-sample recording guard. The guard is a serialized runtime latch instead of an evaluation-time authored Record mutation; toggle Record off then on to re-arm it. See [the Repeat contract](lighting-motion.md#recorded-repeat).


Windows and macOS Open DMX USB output use optional FTDI D2XX libraries; Art-Net remains platform-independent. One selected universe/device, no USB input/RDM or simultaneous transports. Linux excludes the D2XX loaders and continues using Art-Net/OLA. macOS uses Application Support and an optional native menu-bar launcher; real Mac execution/USB validation remains pending. See [macOS support and release limits](macos.md). See [Open DMX on Windows](open-dmx-windows.md).


## Browser host

The optional WASM browser host uses the same evaluator, ScenicScript parser, native blocks, bounded Rhai scripting, command validation and undo/session schema. It supports Studio/schémaboy editing and snapshot subscribers for web animation. Browser storage replaces native project folders; physical transports, native file watches and folder-based component loading are unavailable. Existing session-embedded component definitions evaluate normally. The scene schema still uses fixture IDs/banks; its outputs can drive non-lighting consumers without an Art-Net adapter. Browser lifecycle scheduling does not provide unattended lighting guarantees. See [browser host and embedding](browser.md).


## Saved host selection

The client supports saved HTTP(S) host origins, direct Studio/schémaboy links and QR generation/reading. Opening a host uses its own UI and login in a separate browser/app window. There is no remote engine adapter or native/browser network discovery in this addition. Saved hosts are scoped to the app origin/profile; credentials are excluded. Camera QR input requires a secure context; image/manual pairing is available without camera access. App reload/update preserves the browser session and history; native engine upgrades remain a host operation. See [servers.md](servers.md).

## Authoring state additions

Nodes optionally carry `bypassed: true`, also preserved in ScenicScript. Sessions optionally include named authored checkpoints and embedded portable resources. These fields are omitted when empty/default. Older engines reject projects/sessions that use these additions; update the target engine before transferring them. A rollback must retain the compatible engine if the older binary cannot validate its saved session. Portable bundles use their own versioned format and are imported through the project API rather than decoded as ordinary projects. See [authoring tools](authoring.md).


Saved formats use shared [versioned storage migrations](storage-migrations.md). Version 1 projects, sessions and bundles remain readable; first replacement preserves their originals. New saves use version 2, including every retained undo/redo project.
