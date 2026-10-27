# Initial three-scene batch

Verified 2026-10-27 against source HEAD `9427a13ba623ec661a3c08a1067aa5f0684fcf16` at intake, using the existing binary copied to `target/scene-lab/schema-scene-lab.exe`. The lab did not rebuild it. Exact binary fingerprint: [run baseline](evidence/run-baseline.json). All experiments used isolated loopback engines and disabled Art-Net. No physical DMX, gamma runtime parity, phone hardware, or complete catalog coverage is claimed.

## Load a complete scene

The three folders under `examples/scene-lab/` contain portable `registry/layouts/init.json`, `registry/content/init.json`, and `registry/config/output.json`. They need no resources, plugins, scripts, audio or MIDI. Copy one into a new scratch project before running; the engine creates a mutable session there.

From `C:/Users/magne/dev/schema-rs` in PowerShell:

```powershell
$scene = 'mint-chase' # also uv-midnight or repeat-bezier
$destination = "target/scene-lab/try-$scene"
New-Item -ItemType Directory -Path $destination
Copy-Item "examples/scene-lab/$scene/registry" "$destination/registry" -Recurse
.\target\scene-lab\schema-scene-lab.exe serve --project $destination --bind 127.0.0.1:18797
```

Use a free port and an unused destination. The batch's 18797 engine must be stopped before reusing that port. Open Studio at the printed loopback URL, or append `/mobile` for schémaboy. This copy-first registry workflow was exercised independently on ports 18798, 18799 and 18800; those disposable verification engines were stopped afterward. [Fresh-load evidence](evidence/fresh-cli-loads.json) confirms exact authored JSON equality for all three projects, fixture counts 6/9/4, empty diagnostics and output disabled.

For an already running **isolated lab engine only**, each adjacent `*.project.json` can instead be loaded as a complete ReplaceProject command:

```powershell
$base = 'http://127.0.0.1:18797'
$state = Invoke-RestMethod "$base/api/state"
$project = Get-Content examples/scene-lab/mint-chase.project.json -Raw | ConvertFrom-Json
$body = @{ expected_revision=$state.snapshot.revision; command=@{type='replace_project';project=$project} } | ConvertTo-Json -Depth 80
Invoke-RestMethod "$base/api/command" -Method Post -ContentType application/json -Body $body
```

This API workflow was used to author the projects. ReplaceProject replaces the whole scene and is undoable. Do not point it at the user's live Studio. Named saves in the batch workspace are `uv-midnight`, `mint-chase` and `repeat-bezier`. Studio **Projects** and schémaboy **SAVE / Projects** expose those entries. The selected save title becomes the scene name on load; use the same title for exact project equality. [Roundtrip evidence](evidence/project-roundtrips.json) compares every authored field after save/reload, including fixture patch, models, timeline, layers and drivers.

## UV COB midnight wash

Six one-pixel UV fixtures form two rows at X -3/0/3 m, Y 2 m, Z -2/+2 m. Every fixture belongs to `uv` and either `front` or `back`. Universe 0 addresses are **1, 3, 5, 7, 9, 11**. The custom project model `lab-uv-cob` has exactly two slots: normalized Value dimmer and raw Constant 0 program. The stock `model: dimmer` remains a fallback; `dmx_model` selects the actual custom footprint. There are no patch overlaps.

Layer 1 contains native `Create.Value.Constant` at 0.8. Other layers are explicitly empty. The daily master has 06:00 at 0.2 and 18:00 at 1, with fixed UTC offset +120 minutes. It interpolates through midnight at 0.6, producing round(255 × 0.8 × 0.6) = **122**. Check your desired local offset before use; this is not automatic DST or an IANA calendar.

[Daily checks](evidence/uv-daily-checks.json) used explicit Clock inputs on the isolated engine: local 00:00→122, 06:00→41, 18:00→204, 23:59→123 and 00:01→122. Each of six program slots stayed zero. The host replaces the synthetic Clock input on its next one-second update; these inputs are probes, not a way to set the computer clock or schedule a permanent test. Animation pause does not freeze daily wall-clock dimming.

To author through Studio, select a UV fixture, inspect its model and address, then open Output: DMX models shows the ordered slots, Daily dimming shows the two points and offset, and Universe patch shows each two-slot footprint. These output-panel authoring controls were documented by implementation; this batch verified their serialized engine data/API behavior, not every gesture in those panels. The UV display is a scalar preview, not spectral simulation or evidence of a fixture's program-channel specification.

Portable stack: [uv-wash](../../examples/scene-lab/uv-wash.scnq.yaml). Importing this stack alone does not create fixtures, patching, models or daily dimming; use the complete project for those.

## Mint moving field and fixture-reference distance

One RGB source fixture at address 1 carries a mint point emitter with Range 4 m, Falloff 2, Channel `mint`. Its native cubic path runs from [-4,1,0] to [4,1,0] in 8 seconds, looping with Ping Pong. Handles coincide with the endpoints, producing eased travel. Position is parameter-time motion, not constant-speed arc-length motion.

Eight one-pixel RGB samples occupy addresses 4/7/10/13/16/19/22/25, at X -3.75/-1.25/1.25/3.75 m. The `field` row at Z -1 samples Light.Receiver restricted to the stable ID `source`, then applies Dim color Amount 0.8. The `reference` row at Z +1 uses Create.Value.Distance to the same stable ID, maps distance 0..4 to 1..0, explicitly clamps 0..1, converts to grayscale and dims by 0.8. Mapping extrapolates: the clamp is necessary to bound out-of-range bank values even though DMX encoding also clamps.

[Numeric checks](evidence/mint-chase-numerical-checks.json) set source node 2 Progress to 0, 0.5 and 1. Positions were exactly X -4/0/4. All 24 receiver comparisons matched within 1e-6: field red = (114/255) × 0.8 × max(0,1-d/4)^2; reference red = 0.8 × max(0,1-d/4). The saved scene retains automatic Progress -1. Referenced fixture setup and animated positions remain distinct.

Only native operations are needed; no Script.Value. This is a spatial chase, not an independently timed expanding ripple. Analytic fields do not simulate occlusion or physical photometry. Emitter/receiver are Rust extensions. Portable stacks: [emitter](../../examples/scene-lab/mint-emitter.scnq.yaml), [receiver](../../examples/scene-lab/mint-receiver.scnq.yaml), [distance](../../examples/scene-lab/reference-distance.scnq.yaml). They require a fixture with stable ID `source` and the documented arrangement.

## Repeat, Bézier and independent layers

Four one-pixel RGB fixtures at addresses 1/4/7/10 have setup X -3/-1/1/3 m. Every fixture has three independent trees:

| UI layer | Tree | Connection |
|---|---|---|
| Layer 1 (API 0) | Tool.Merge + rose Color + Dim color | A Layer Above child with Layer 0 reads UI Layer 2 position. Dim Amount has its own Layer Above driver with Layer 1, reading UI Layer 3 scalar. |
| Layer 2 (API 1) | Position Bézier | Own fixed X, Z -2→2 and raised Y handles at 2 m, 8-second Ping Pong. |
| Layer 3 (API 2) | Repeat around scalar Bézier | Scalar 0.1→1, 4-second Ping Pong, Record true, Play true, Repeat Mode 1 (PingPong). |

Only Layer 1 supplies the final fixture bank. Higher layers are not implicitly mixed; these explicit references compose them. Layer Above's Layer property counts relative higher layers from zero. The Amount driver is a separate tree, not a normal child. Owner-before-driver timing introduces the documented one-frame delay. RGB output uses the resulting color; a scalar elsewhere is not automatically a color dimmer.

[Exact playback evidence](evidence/repeat-playback-check.json) found 402 recorded banks and a latched guard while authored Record remained true. At native index 2834, PingPong selected sample 20 and both expected/actual scalar were 0.9065674255109621. PingPong starts its reverse pass and repeats endpoints. Playback is frame-based; the observed engine ran at 60 Hz, so the 402-sample window spans about 6.7 seconds. This is not an 8-second full-cycle recorder. Setting Record off/on re-arms capture. A true Record property in a portable project re-records after load at the current global clock phase; it does not embed a fixed recorded clip.

Session recovery during the binary-copy handover retained 402 samples, runtime and undo history ([runtime summary](evidence/repeat-runtime-summary.json)). Named project loading resets runtime. With Record false, saving/loading preserved the authored tree but changed sample count 402→0, and Repeat passed its live child through ([boundary evidence](evidence/repeat-named-load-boundary.json)). That matches the documented contract. Use session recovery for runtime continuity; use this project to generate a fresh recording. Exportable independent portable clips are a requested addition, not existing support.

Portable layer stacks: [composite](../../examples/scene-lab/rose-composite.scnq.yaml), [motion](../../examples/scene-lab/independent-motion.scnq.yaml), [recorded scalar](../../examples/scene-lab/recorded-easing.scnq.yaml). Install all three in their stated layers; the motion stack has the first fixture's fixed X, so use each fixture's complete project tree to preserve spacing.

## Exercised UI steps

At 127.0.0.1:18797, Studio selected repeat-1. Editing Fixture name to `Recorded rose 1 lab check` committed revision 10; Undo restored the original name at revision 11. Layer 2 exposed the XYZ Bézier inspector and retained all four full 3D vectors. Switching to schémaboy preserved fixture and layer. Tapping Duration 8, entering 6, then Apply property showed Duration 6; Undo restored 8. The screenshot captures the committed mobile edit before undo. This was mouse-driven schémaboy at desktop viewport, not a physical phone/touch or phone-width test.

[Studio inspector](evidence/repeat-studio.png) · [schémaboy edit](evidence/repeat-mobile.png) · [Repeat controls](evidence/repeat-controls.png).

Dynamic fixture/property lists are rebuilt from snapshots: automation element IDs became stale; stable role/name locators worked. The initial setValue committed on change before a subsequent Enter attempted to target the old ID. No scene defect is inferred from these automation targeting errors.

## Validation and remaining scope

All seven exported ScenicScript files passed `schema-scene-lab.exe validate <file>`. All three projects passed fresh CLI loads and exact named save/reload comparison, with empty diagnostics and output disabled. Numeric UV and mint checks and exact Repeat playback are linked above. No application code, tests, dependencies, scripts or deployment were changed. Full cargo/frontend checks belong to implementation and were not run for this authored-data batch. Findings and requested additions: [findings](findings.md).
