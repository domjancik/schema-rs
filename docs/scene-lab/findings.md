# Scene-lab findings

Baseline source HEAD at intake: `9427a13ba623ec661a3c08a1067aa5f0684fcf16`. Date: 2026-10-27. Existing binary SHA256 and isolated run details: [baseline](evidence/run-baseline.json). No implementation defect was confirmed by the initial batch. The entries below are usability/capability requests; neither represents a failed documented semantic contract. Output remained disabled, with no hardware validation.

## SL-001 — Repeat needs visible runtime recording/playback status

- Kind: usability request. Severity: P3. Status: reported, awaiting implementation triage.
- Scope: Modify.Repeat inspector, observed in schémaboy at desktop viewport; native runtime data from isolated engine API. Studio's runtime presentation was not independently exhaustively checked.
- Reproduce: load repeat-bezier; let the 402-sample recording guard complete; open Layer 3's Repeat inspector. Compare `/api/session` state `repeat-1/2/20.native` with the inspector.
- Desired: distinguish Recording, Playing and Held, expose captured sample count/duration and make the re-arm action discoverable. These are requested controls, not a claim about existing product requirements.
- Actual: the authored Record remains true after the guard latches and playback starts. Inspector shows Record true, Play true and PingPong; there is no visible runtime sample count or latched/stopped-recording indicator. A user can reasonably read Record true as ongoing capture.
- Evidence: [controls screenshot](evidence/repeat-controls.png), [exact playback check](evidence/repeat-playback-check.json): count 402, latch true, authored Record true, selected sample 20 matches actual 0.9065674255109621.
- Workaround: inspect `/api/session` for native state; toggle Record false then true to start a fresh recording. The portable example documents the bounded frame window.
- Requested implementation addition: a read-only runtime status/count published for Repeat and rendered beside authored controls; retain authored/runtime ownership separation. A semantic change to the 402-frame guard is not requested.

## SL-002 — Portable recorded clips are absent from named project saves

- Kind: optional capability request / confirmed compatibility boundary. Severity: P3. Status: reported; documented behavior verified, not a bug.
- Scope: named scene storage versus native session recovery. Resource/package bundling and broader snapshot behavior are outside this finding.
- Reproduce: load repeat-bezier; wait for 402 samples; set repeat-1 Layer 3 Repeat Record false; save as a new named scene; load that saved scene. Query `/api/session` before and after.
- Desired, if portable recorded clips become a product goal: explicitly export/import a captured bank sequence as an asset, or clearly communicate that named saves keep the graph but exclude recordings.
- Actual: recorded count 402 before load, zero after; Record remains false and Repeat output equals its live Bézier child. Named loads correctly reset node runtime. By contrast, automatic session recovery during the isolated binary handover retained the 402 samples and undo history.
- Evidence: [named-load boundary](evidence/repeat-named-load-boundary.json), [recovered runtime summary](evidence/repeat-runtime-summary.json), implementation [project contract](../projects.md) and [session contract](../session-history.md).
- Workaround: keep Record true in a portable generative scene so it records afresh after load; use trusted session recovery for runtime continuity. Captured phase depends on the current engine clock; named scenes keep that global clock. Do not describe these saves as portable fixed recordings.
- Requested implementation addition: consider a narrow bank-sequence asset format or a named-save warning for retained runtime nodes. No code changes were made by the lab.

## Batch verification and non-findings

All three scenes retained complete authored state in named roundtrips and fresh CLI project loads. Five daily clock probes and 24 spatial sample comparisons passed. Repeat's selected playback sample matched exactly. Studio fixture rename/undo and schémaboy Duration 8→6→undo worked. [Guide and evidence](initial-batch.md).

Automation encountered stale AX element IDs when snapshot-driven property lists re-rendered; stable role/name selectors succeeded. This was an automation targeting limitation, not filed as an application bug. Phone-width and physical-touch behavior were not exercised by this batch. No claim is made about every authoring gesture or catalog node.
