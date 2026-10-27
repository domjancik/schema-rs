# Light fields and motion

## Use an emitter and a receiver

Put `Light.Emitter` in one fixture’s evaluated stack and `Light.Receiver` in other fixtures. Match their Channel (default `default`). A receiver may restrict Fixture by stable ID/name or an incoming `Create.Reference.Fixture`. Empty Channel accepts all channels; Include Self defaults off.

Import [moving-emitter](../examples/lighting-motion/moving-emitter.scnq.yaml) onto the source and [receiver](../examples/lighting-motion/receiver.scnq.yaml) onto the receiving fixtures. The emitter follows a cubic 3D path; the stage shows evaluated X/Z positions. Authored setup coordinates remain separate. The receiver writes additive RGB plus scalar received intensity; normal fixture dimming/blackout and Art-Net encoding apply afterward. Each source/receiver needs its own fixture and patch address.

Emitters are carried in result banks. Merge joins declarations, a closed gate drops them, and sample-and-hold can retain them. Declarations surviving any evaluated fixture layer join the scene field, deduplicated by fixture/layer/node. A declaration without an incoming Position follows the fixture’s final evaluated Position, then setup position. Color tints an incoming color (or white), and Intensity multiplies an incoming Value (or one). Drive Intensity/Range directly to modulate these parameters. Modifiers outside an emitter do not retroactively rewrite its declaration.

## 3D field definitions

All coordinates and sizes are metres, Direction is a world-space nonzero XYZ vector, and Type accepts Point/Plane/Spot or numeric 0/1/2.

- **Point:** isotropic spherical field. At distance d less than Range r, attenuation is `(1 - d/r)^Falloff`; outside Range it is zero.
- **Plane:** finite oriented rectangle, Width × Height. Attenuation uses the 3D distance to its closest point. Direction sets its normal; the field occupies the forward half-space unless Two Sided is checked. The in-plane basis is deterministic (world Y reference, world X near vertical normals).
- **Spot:** point attenuation multiplied by a cone mask. Angle is the full outer cone angle in degrees. Softness sets the fractional angular feather, using smoothstep between outer and inner cone boundaries.

These fields work throughout 3D space, including height. They sample analytic illumination; they do not model occlusion, inverse-square physical photometry, multiple bounces, atmospheric scattering or a rendered fog volume. Up to 1024 carried scene declarations are sampled. Emitters are gathered from a probe using cloned node state, then receivers evaluate against that frozen same-frame field. Actual node state advances only once. Receiver-dependent light feedback is not iterated; keep emission parameters independent of received light.

## Native position and random ports

The registered public keys now include `Create.Position.Current`, `Create.Position.XYZ`, `Create.Position.Group.Center`, `Create.Position.Group.XYZ`, `Create.Position.Group.Slide`, `Create.Position.Group.Random`, `Create.Value.Random`, and `Modify.Value.RandomizeTrigger`.

Current copies setup position. XYZ supports independent signed world-coordinate drivers. Group center/XYZ use setup bounds. Empty Group means all fixtures; an unknown group reports a diagnostic and produces no Position. Padding expands bounds in metres. Group Slide moves one selected axis through the bounds; Speed 0–1 maps to a 100–0.5 second period. Group Random maps Speed to a 100–0.2 second target interval, interpolates while Continuous, and jumps otherwise. Speed Audio modulates period using host audio level and the native AudioLerp formula.

Source references: `scnq-schema/Main/Exports/src/VVVV.Schema.Core.vl/VVVV.Schema.Core.vl.3.cs` PosCenterBee.Update_ (4708), PosSlideBee.Update_ (4903), PosRandomBee.Update_ (6400); `.4.cs` ValueRandomBee.Update_ (394), XYZGroupPositionSource.Update_ (1122), Current.Update_ (1406), RandomizeTrigger.Update_ (6507). Public names and supported controls are ported; exact dependency-helper phase initialization, tween shapes and random sequences are not claimed as native parity.

This Rust port uses SplitMix64 with independent fixture/node seeds when Seed=0, or an explicit integer seed. Interval controls target changes; Randomness adds bounded jitter. Filter Time linearly interpolates from the previous value to each target. Pause freezes both the target clock and filter. Parameter edits retain generator state; changing Seed restarts it. RandomizeTrigger draws on a rising edge above 0.5 and retains its decision until the input falls. Seed is static; Speed Audio can be driven. Undo restores authored state and follows the existing policy of resetting runtime node state.

`Create.Position.Random` is a Rust extension for random motion within Minimum/Maximum XYZ bounds, using the same timing/seed controls. Examples: [random position](../examples/lighting-motion/random-position.scnq.yaml), [XYZ animation](../examples/lighting-motion/xyz-drivers.scnq.yaml).

## Bézier animation

`Create.Position.Bezier` evaluates a cubic with endpoints P0/P3 and handles P1/P2. All four points are full 3D vectors. Its inspector provides an X/Z handle editor and live position marker; drag a handle for one committed edit/undo step, or nudge with arrows (Shift for larger steps). Numeric fields edit height as well.

`Create.Value.Bezier` uses Start, Control1, Control2 and End for scalar animation. Both nodes use Progress when nonnegative, otherwise incoming Value, otherwise engine time / Duration. Explicit/incoming progress clamps to 0–1. Clock playback may Loop and Ping Pong; with Loop off it clamps at the final endpoint. Engine pause stops the clock. Duration is in seconds. This is parameter-time traversal, not constant-speed arc-length motion. These Bézier nodes are new native Rust extensions, not discovered native Schéma registrations.

The library includes descriptions, search keywords and individual SVGs for all 186 supported blocks. Light Type and Axis controls have named choices; Group lists the current fixture groups.

## Verification

`tests/lighting_motion.rs` and `tests/motion_drivers.rs` cover 3D distance, light shape/direction, additive color, channels, fixture-order independence, same-frame position, gated emission, probe/state isolation, signed drivers, paused randomness, all new motion/random behaviors, cubic endpoints, height and clock pause. `tests/lighting-motion-ui.cjs` tests the actual handle gesture and undo, retained Y, lighting browsing and named type selection on a capture-only engine.

## Visual scalar curve editing

Create.Value.Bezier has a shared visual editor in the Studio inspector and schémaboy block inspector / Start, Control1, Control2 and End property menus. Drag each handle vertically; horizontal positions stay fixed at thirds, matching the native scalar cubic. The four controls are values, not CSS timing-curve X/Y pairs. Numeric fields remain available for exact values and wider ranges. Driver-controlled handles are read-only; edit their driver instead. Mouse, touch and keyboard edits submit one revision-checked command per completed gesture; Escape, pointer cancellation or lost capture discards the local preview.

## Recorded Repeat

The original public Repeat registration is Modify.Repeat: Record captures merged property banks, Play advances the retained playback index, and Mode selects Forward, PingPong or Reverse. PingPong starts with the reverse pass and includes repeated endpoint frames (for three samples: 2,1,0,0,1,2), matching the original Boolean-switch phase. Turning Record on clears the previous recording; turning it off restarts playback at the first frame (last frame in Reverse and PingPong). Turning Play off holds the last displayed sample. Captured banks and playback indices serialize in session checkpoints.

The original counter guard `> 400` permits 402 samples, then returns to playback. Rust preserves that bounded window with a runtime latch; it does not rewrite the authored Record property from the evaluation loop. Toggle Record off and on to start another recording. With no recorded samples, Repeat passes through the merged input. Capture is frame-based, so playback speed follows the engine evaluation rate. Create.Color.Pixel.Segment also retains its separate Repeat pattern parameter.
