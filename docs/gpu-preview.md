# GPU scene preview

Open **Scene preview** above the Studio stack editor, **GPU** in schémaboy’s screen-view controls, or `/preview`. The preview is a separate GPU canvas: orbit with drag, pan with right-drag/two fingers, zoom/pinch and use Fit scene. Clicking a layout handle selects the fixture in the originating editor. Fixture editing remains in the normal validated engine UI.

## Live output and reference presets

Live fixture brightness is decoded from published capture universes, including global dimming, fixture gain, custom model ceilings, RGBW white, channel footprints and blackout. Fixed program slots are ignored as light channels. Color/value slots use their captured coarse bytes; arbitrary custom control channels are not interpreted as fixture photometry. Missing captures stay dark. Published fixture/pixel position and rotation drive motion; layout handles stay at the authored layout positions. The Light emitters option displays native point/plane/spot field emitters, whose conceptual intensity is independent of a hardware profile’s photometry.

Studio/schémaboy share their existing publication with the canvas. The standalone page reads `/api/state` at eight Hz. Neither path evaluates node programs or advances the engine clock. Graphics render independently with bounded quality settings and stop when the dialog closes or the document is hidden. The engine continues to run if the graphics context fails.

Choose **Carpushit · reference preset** for a static full-look reference with the actual assembly and 38 circuits. That selection, camera/settings and scenery import issue no engine commands. **Use Carpushit in project…** opens a review; **Replace current scene** performs one revision-checked, undoable ReplaceProject with output disabled. The bundled template is the lighting layout/full look; the example workspace’s stored-content library and disarmed recall director are not implicitly imported into a different host’s registry. The complete authored example remains under `examples/scene-lab/carpushit/`.

The Carpushit GLB is 58,533 triangles, six material roles, 4,560,244 bytes, already in Schema Y-up metres/front negative Z. Do not transform it again. The six UV strips use rectangular area approximations of the source paths; the visible violet is an illustrative proxy, not fluorescence. Bulbs retain the captured 127-byte ceiling. Geometry/source mapping and provenance live under `web/scene-assets/`. The assembly is a user-provided local/private asset and is not relicensed by the code’s MIT license.

## Generic scenery and renderer controls

**Import scenery** accepts a self-contained GLB up to 32 MB in metres. Materials are retained; embedded lights are disabled so they do not invent output alongside the engine’s fixtures. External URI resources/compression decoders are not fetched. Imports and preview settings are local to this view, not saved into engine-owned project data. Clear scenery returns to the neutral floor. The fixture/light adapter and renderer have no Carpushit-specific geometry or node-evaluation logic.

**Auto** uses Three.js WebGPURenderer with WebGL2 fallback; WebGL2 can be selected explicitly. WebGPU needs a secure browser context, so the current plain-HTTP Pi/LAN addresses generally use WebGL2. The chosen backend is shown in the status line. WebGL2 devices without floating-point color targets retain basic direct lighting; advanced effect controls are disabled and the status explicitly says basic lighting. No GPU work runs on the Pi server.

Direct point, spot and rectangular area lights feed a depth-aware, quarter-resolution ray-marched volumetric pass. SSGI gathers indirect illumination/occlusion from visible color/depth/normal buffers. Temporal antialiasing stabilizes fine geometry and SSGI; history resets on blackout/project changes. Haze, exposure, light gain and room fill are display controls. Shadows are budgeted. This is an uncalibrated real-time scene view, not CUDA ray tracing, caustics, radiometry, UV fluorescence or a complete offscreen light-transport simulation.

Quality budgets: Low 12 lights/20 fps target; Balanced 40 lights/30 fps; High 64 lights/60 fps, with resolution and shadow budgets. Phones default Low. All layout handles remain available up to the marker limits; the status shows rendered/total sources. Actual frame rate depends on browser/GPU. Markers cap at 4,096 pixels and 2,048 fixture handles; profiles without interpretable value/RGB slots need explicit light-emitter descriptions.

## Offline build and verification

All modules/assets are embedded and served locally; no CDN/runtime package download is required. Pinned vendor versions/notices are in `web/vendor/`. To rebuild the vendor bundle:

```powershell
npm install --prefix target/gpu-deps --ignore-scripts --no-audit --no-fund three@0.186.1 esbuild@0.25.12
node scripts/gpu/build-vendor.cjs
```

Verification includes pure capture/rotation/immutability adapter tests, embedded asset and preset validation, actual native-Chrome WebGPU and software-WebGL2 frames, all38Carpushitlights, point/plane/spot shader paths, effect toggles, import error recovery, explicit mocked revisioned load, closed-view lifecycle, and Studio/phone viewport entry points. Preview tests reject unintended engine commands and do not enable physical output. Native Chrome GPU verification uses an isolated browser profile with GPU test flags; no performance guarantee or physical fixture test is implied.

Primary references: [Three.js universal renderer](https://threejs.org/manual/pages/webgpurenderer), [SSGI example](https://threejs.org/examples/webgpu_postprocessing_ssgi), [volume lighting](https://threejs.org/examples/webgpu_volume_lighting). Optik’s local scene view informed the workflow separation; no Optik/CUDA simulator code is required.
