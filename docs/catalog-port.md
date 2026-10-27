# Documented catalog port

The engine registers all **178 documented public keys**, plus eight Rust extensions (186 total). Every registered key has an evaluation path; the catalog test constructs every default program, evaluates it repeatedly, validates its checkpoint, and compares continued evaluation after session restore. Focused numerical and integration tests cover each implementation family. This does **not** establish exact equivalence to the original runtime for every parameter and composition. The imported source contracts and evidence are retained in `documented-catalog.json`; `port-coverage.json` records implementation locations and validation levels.

## Runtime families

| Module | Responsibilities |
| --- | --- |
| `engine.rs` | Original scalar/color arithmetic, maps, drivers, references, layer ordering, script evaluation and model transactions |
| `native/mod.rs` | Typed channels, rotation, duration/speed, moving-head slots, boolean/comparison, hue/saturation, resources |
| `native/timed.rs` | Envelopes, filters, edges, delay, recording, selection, fades/flashes and retained timing |
| `native/pixels.rs` | Color spreads, segment/rainbow, resampling, shift/blur/scramble, mix/max, setup and channel picking |
| `native/spatial.rs` | Fixture-relative distance, group coordinates, Copycat, pulse volumes and rotation effects |
| `native/providers.rs` | Audio/texture processing and OSC, controller, keyboard, gamepad, remote/global/sequence/resource context |
| `native/actions.rs` | Deferred fixture/group/content/director assignments, global fades, memory and transition adoption |
| `io.rs` | Off-thread UDP acquisition, WAV decoding, project-resource checks, registry refresh and UTC clock |

Source contracts distinguish merged children, first-child preservation, fresh banks and child selection. Children execute before selection. Fixture setup channel is explicit and defaults to zero; it is not the fixture's ordinal index. Earlier fixture results are visible in the same frame; later fixtures retain their previous-frame state. Director programs execute after fixtures.

## Inputs and resources

```powershell
cargo run -- serve --project ./schema-project --osc 127.0.0.1:9000
```

OSC is opt-in. This accepts individual OSC messages with integer, float, double, int64, string, boolean and nil arguments. Bundle scheduling, wildcard dispatch and discovery are not implemented. The bind address is independent of the HTTP listener; use an explicit interface when receiving from another machine.

The web client forwards keyboard state while a text field is not being edited, standard gamepad controls, and fixture selection. **MIDI** requests browser MIDI access; CC messages map to controller indices and notes map to pads 0–15. **Microphone** requests permission and sends bounded PCM buffers to audio input slot 0; click again to stop. No device access starts without pressing its button. Host integrations can POST typed events to `/api/inputs`, using the same origin/token guards as other mutations. Provider updates do not enter authored history or increment authored revision.

The input worker scans named resources and registry files once per second. Resource paths resolve within the project's `resources/` directory, including symlink containment checks. PCM WAV (8/16/24/32-bit integer or 32-bit float, up to eight channels mixed to mono) is supported. Files are limited to 16 MiB, and loaded sample data is limited to two million samples. Set `Create.Resource.Constant.Resource` to the relative WAV path and feed it to `Create.Audio.Sample`. Play, Loop, Gain and Speed control the generated Audio bank. This is engine signal processing; no speaker output backend is included.

Reusable `registry/stacks/*.scnq.yaml` entries are addressed by their stack name. `registry/content/*.json` snapshots are addressed by filename stem. The `/api/inputs` endpoint can also supply named stacks/content, remote banks, sequences, globals, external colors, analysis metadata, textures and legacy Pong state. Missing resources produce diagnostics or the node's documented default.

CPU audio analysis provides RMS, peak, power, a 16-bin DFT, zero-crossing pitch and basic spectral-onset/BPM estimation. Integrations may supply analyzed beat/BPM metadata directly. Beat consumers use audio-event identity, so keyboard or clock updates cannot retrigger a held beat. The scheduled clock is UTC; integrations can provide an explicit Clock event. Textures are bounded CPU pixel arrays, without GPU handles or native shader rendering.

## Actions, layout and undo

Assignments resolve against the named registry after evaluation, prepare edits outside the frame thread, and commit through the engine. Group assignments precede fixture-specific overrides; targets include layer identity. Repeated assignment to the same stack name is ignored. Assignments form authored undo transactions; finishing a transition adopts the incoming child without creating a second undo entry. Setup pixel changes and memory-load adoption are internal model changes, also without extra undo entries. Pixel setup updates validated layout and is included in project saves.

Director programs have three independent layers, are selectable in the web sidebar, and are saved with content rather than layout. Layout retains fixture rotation, pixel coordinates/direction and setup channel. Multiple DMX pixels encode according to the fixture's dimmer/RGB/RGBW model. Additional moving-head channels remain typed result channels until a fixture profile maps them to DMX.

Native timer/filter/random/selection buffers and provider context are serialized with session history. Delay/recording buffers retain at most 1024 frame samples, and each input registry retains at most 256 entries. Microphone streams, MIDI device handles, UDP sockets and other acquisition handles are reacquired by adapters; they are not serialized.

## Compatibility corrections and limits

- Steps rounds to `Count - 1` intervals; maps extrapolate rather than silently clipping. Crossfade supports all eleven documented shapes.
- `Create.Value.Expression` uses `s` as elapsed engine seconds. It supports bounded Rhai numeric syntax plus common NCalc-style math names; it is not a complete NCalc parser. Errors retain the last successful scalar.
- Controller Pad resolves to the actual `ValueControllerPadBee` editable definition with Pad 0–15. Keyboard resolves to the keyboard plugin's Key Name property. The older record descriptions for these definitions were incomplete.
- SequenceIndex divides by sequence count. Rotation Follow keeps native target Lantern as its default and adds an explicit Fixture selector. Radius Constant preserves the returned raw Radius, including the source's discarded audio calculation.
- Speed Audio now uses the traced `AudioLerp` formula: interpolate the declared period limits using `min(base + level * amount, 1)`.
- Native dependency-specific interpolation, random distributions, filters, beat analysis, gamma phase initialization and GPU resource behavior have not been compared against original-runtime golden captures. These use explicit Rust/CPU implementations with focused tests; exact sample-for-sample parity is unverified.
- Complex .NET literal encodings, native gamma XML projects, arbitrary plugin loading, full NCalc syntax and audio/GPU device output are outside this port. Native editable-only widget and legacy Pong nodes expose their values through the engine/host contracts rather than recreating the original Windows UI.

The catalog completeness/continuity gate is broad. The numerical fixtures are family-level regression evidence; exhaustive nondefault/composition golden fixtures for every key remain a conformance task. See `port-coverage.json` instead of interpreting registration as a full conformance claim.
