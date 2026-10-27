# Compact schémaboy release retest

Date: 2026-10-27. Requested release: 278832c, including one-viewport 2b0883a and PWA bbc8f28. Source HEAD at copy: 7a2e40e (release plus lab documentation batch). Existing debug binary copied to `target/scene-lab/retest-278832c/schema-retest.exe`, SHA256 `032D5DFED65F2C994130F3222AFED90A255BAB1EAB4E8A4995FF9BBADBAA88CF`. No compilation or source edits. Latest subsequent long-value CSS refinement/PWA cache change was announced during this run and is not independently covered here.

A fresh copy of repeat-bezier's registry ran at 127.0.0.1:18797, output disabled. Existing mint-chase and uv-midnight Project payloads were loaded only into that isolated engine for orientation checks. Live Windows/Pi systems and their histories were not accessed. The lab-specific executable avoided locking Cargo's build output.

## Confirmed checks

- Repeat main controls fit requested 320×568, 375×667, 390×844 and 844×390 sizes. The browser reported actual CSS viewports 320×568, 376×667, 391×844 and 844×391 because of its rounding/scaling. Document dimensions equaled actual viewports, with no page overflow.
- At every size, scene, encoder, A/B, Hold, Save, layer switch and block inspector were in bounds. Repeat controls also passed center-point hit checks. Mint and UV passed the same bounds checks in small portrait and landscape. [Measurements](evidence/retest-geometry.json).
- Portrait Duration 8→6 committed revision 1; Undo→8 at revision 2. Full XYZ Bézier P1 [-3,2,-2]→[-3,3,-2] committed revision 3; Undo at revision 4 preserved the original vector.
- Landscape Repeat Mode's named selector offered Forward/PingPong/Reverse. Selecting Reverse stored numeric 2 at revision 5; Undo restored 1 at revision 6. Scalar Bézier Control1's visual handle accepted ArrowUp: 0.1→0.11 at revision 7; Undo at revision 8 restored exact whole-project equality with the original repeat-bezier payload. [Summary](evidence/retest-summary.json), [undo equality](evidence/retest-undo-check.json).
- Overflow Control guide opened at 320×568. Install app opened the platform-specific installation-help dialog in landscape; no OS installation was attempted.
- UV Output & DMX opened its daily timeline with 06:00/20%, 18:00/100%, UTC offset120 and master100%. Universe patch displayed 12 occupied slots, six custom two-slot fixtures at 1/3/5/7/9/11. Opening/closing and switching tabs made no authored edit; UV still exactly equaled its original payload before the scratch long-label variant was loaded.
- A scratch mint variant used long authored fixture/stack names and a long shared Channel string. Both orientations kept the eight main controls in bounds. Tapping Channel opened its complete exact value. The older copied build wrapped the long label; no additional blocking overlap was observed. The saved portable examples were unchanged. [Long-label screenshot](evidence/retest-long-labels.png).

These were mouse/keyboard interactions with browser viewport emulation, not physical phone, touch-gesture, soft-keyboard or OS installation tests. All engine observations had output disabled and empty diagnostics.

## New defect and retained requests

[SL-003](findings.md#sl-003--output-panel-tab-labels-overlap-at-320-pixel-portrait-width): Output-panel tab labels overlap at 320-pixel width. Each navigation tab shrinks to about30 pixels despite 59–69-pixel text overflow. All tabs still opened, so this is a P3 usability issue. Exact selectors, rectangles and screenshots were sent to implementation. No other blocking viewport/menu overlap was confirmed in this focused batch. SL-001/002 remain accepted P3 follow-ups.

## Screenshots and workflow notes

[Portrait edit](evidence/retest-portrait.png) · [portrait overflow](evidence/retest-menu-portrait.png) · [landscape enum](evidence/retest-landscape.png) · [scalar curve](evidence/retest-curve-landscape.png) · [mint landscape](evidence/retest-mint-landscape.png) · [UV daily](evidence/retest-uv-output.png) · [UV patch](evidence/retest-uv-patch.png).

The initial-batch copy-first `serve --project` commands remain valid for isolated example authoring. The new `desktop` launcher has a separate app-data default and loopback8797; that default was read from release notes, not launched or mutated by this retest. Use explicit scratch `serve --project` for lab work. Original initial-batch screenshots reflect the older layout; this report records the newer compact controls.

The browser's temporary viewport override was reset after the run. The lab engine was flushed and stopped when the short retest finished. No code/tests/scripts/dependencies/configuration/deployment edits or commits were made by the lab.

Implementation follow-up: SL-003 has a reported CSS fix and implementation regression coverage; status is **fix implemented; awaiting lab verification**. This run tested the older copied binary and does not close that finding. The lab engine has been stopped; further native retesting awaits the final package notice.
