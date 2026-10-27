# Scene-lab examples

Three complete output-disabled projects, plus standalone ScenicScript stacks:

- `uv-midnight/`: six two-slot UV COB fixtures, grouped wash, cyclic daily dimming through midnight.
- `mint-chase/`: moving native point light and two rows comparing field reception with fixture-reference distance mapping.
- `repeat-bezier/`: four fixtures, independent color/motion/recording layers and a separate dimming driver.

Copy the chosen registry into an isolated scratch project before running. Do not run directly from these artifact folders: the engine writes session state. Full verified loading commands, patch/layer assumptions, numerical evidence and compatibility limits are in [the initial-batch guide](../../docs/scene-lab/initial-batch.md).

The adjacent `*.project.json` files are complete API Project payloads; `*.scnq.yaml` files are portable stack imports. A stack alone does not carry layout, custom models, daily timeline or recorded runtime. No resources/plugins are required. These scenes use native engine nodes, including documented Rust extensions, and no Script.Value code. Physical fixture behavior is unverified.
