# Carpushit scene and presets

Output-disabled project with38 provisional intensity circuits, based on the user's exported v15 Carpushit assembly. Ten bulbs have a fixed127/255 channel ceiling; six roof UV strip runs,16 mirrored scattered fixtures, eyes/ears/rear watchers are independently grouped.

- `carpushit.project.json`: complete API Project payload.
- `registry/`: copy this WHOLE folder into a new scratch workspace, then run `serve --project <scratch> --bind 127.0.0.1:<freeport>` with a copied executable. Do not run directly in these deliverable folders because the engine writes session state.
- `registry/content/carpushit-*.json`: eight sparse stored-content presets. Primary effects use API layer1; optional exterior twinkle uses API2. The output compositor on API0 remains independent.
- `registry/stacks/*.scnq.yaml`: validated standalone block examples.
- `preset-catalog.json`, `fixture-patch.csv`: names, target-layer assumptions and stable fixture patch.
- `assets/cue-sheet.json`: native120-second Director sequence metadata. The actual Director starts DISARMED in project content.
- `assets/carpushit-preview.glb`, `preview-bundle.json`, `preview-lights.json`, `reference-full.json`: frozen private preview assets with explicit source provenance and read-only reference lighting. The GLB is already Schéma Y-up/metres/front-Z. No browser evaluator or automatic project load is authorized by selecting this reference.

[Full guide, commands, cue timings, evidence and pending UI capabilities](../../../docs/scene-lab/carpushit.md).

The scattered-light count assumes3+2+3 fixtures per side,16total, pending human clarification. The patch and preview directions are authored assumptions, not hardware measurements. UV is a visible violet proxy, not fluorescence. Existing preset loading accepts names/groups, but the requested browsable selector and timeline editor remain pending implementation.
