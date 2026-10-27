# Block library

Open **Add input**, **Insert below**, or **Create stack** to browse the 186 supported behaviors.

Blocks generate, shape and combine values, colors and positions. Ordered children feed their parent; separate property drivers animate settings. The engine remains responsible for evaluation and committed edits.

- Choose an operation family: Create, Modify, Combine, Filter, Convert, Assign, Pick, Global, Setup, Transition, Spatial, Script, Tool and legacy/test categories.
- Narrow by data domain and native/extension implementation. Counts reflect the active filters and search.
- Search friendly names, ScenicScript keys, descriptions, synonyms and property names. Search tolerates small typos, transpositions and shortened terms.
- Click to preview the description, channel contract and defaults. Add explicitly, double-click, or press Enter. Up/Down browses from the search field.
- Favorites and the last 12 successfully inserted blocks persist locally in this browser. They do not change project files or engine history.
- Insertions use revision-checked engine commands and the existing undo/redo history. Browsing and previewing make no authored edits.

`web/block-metadata.json` is the shared description source, embedded in the Rust catalog API and used by the live frontend preview. The runtime's registered behaviors remain authoritative for defaults and supported keys. Descriptions cover current implementation behavior and limits.

Each block has a distinct, self-contained SVG in `web/icons/<ScenicScript-key>.svg`, served at `/icons/<ScenicScript-key>.svg`. Every 48 x 48 icon includes a title and description and has no scripts, external dependencies or font requirements. The same artwork appears in the browser, stack and inspector.

The library adapts to light/dark mode and narrow screens. `docs/block-library-light.png`, `docs/block-library-dark.png` and `docs/block-library-mobile.png` show the verified UI.

Verification: `node --test tests/catalog-browser.test.cjs`; `node tests/catalog-picker.cjs` (read-only); `SCHEMA_URL=<isolated test server> node tests/catalog-editing.cjs` (inserts and undoes a block). Browser scripts use the same Playwright environment variables as the existing smoke tests.
