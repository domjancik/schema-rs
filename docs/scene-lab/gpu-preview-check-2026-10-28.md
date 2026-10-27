# Carpushit GPU preview: brief alignment check

2026-10-28, browser at http://127.0.0.1:8788/preview?scene=carpushit&quality=balanced. Read-only reference selection only; no project-load button, engine commands, output changes or live scene edits. One camera orbit inspected another side.

Observed WebGPU rendering, status38fixtures/38of38lights, about23–24fps at the existing browser size. Roof violet illumination follows the roof, wing sources sit above the wing geometry, and lateral/forward/rear markers retain the expected mirrored arrangement. No blocking axis, scale, origin or fixture-to-mesh alignment issue was observed in these two views. This was not a full rendering, backend, photometry or UI test.

Copied web/scene-assets/carpushit.glb matches the frozen source SHA25626724b97b48a81d3530f22d80b13e5a873f8eb263be4a28ba76f787a91143b48. carpushit-lights.json exactly equals authored preview-lights.json; all38 copied reference fixtures exactly match reference-full.json, including positions, colors and bulb127/255 intensity.

[Screen capture](evidence/carpushit/gpu-preview-check-2026-10-28.png). No blocking mapping findings. Directions/ranges, UV tint and screen-space effects remain authored visualization approximations, not hardware or physical-lighting validation. The app's new editable preset selector/timeline is not covered by this check.
