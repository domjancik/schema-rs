# Port the documented Schéma catalog

Objective: implement all 178 public keys in ../scnq-schema/codex/blocks/records, preserving supported current Rust extensions and existing live edits/history. Source evidence: Main/Exports/src plus editable Main/vl where no export exists. Do not infer native behavior from the Python editor. Do not register unsupported pass-throughs.

Baseline: 39 registered behaviors; scalar/color/Position/reference banks, three fixture layers, native Rhai, independent Art-Net worker, validated commands, staged session serialization. Live engine 8787 and UI 8788 currently run the prior build; do not reset their undo history while porting.

## Steps

1. Inventory and preserve all documented keys/defaults/types/evidence in docs/documented-catalog.json. Establish an executable coverage gate and explicit unresolved-source list. Native parameter defaults are distinct from missing-key fallbacks. Keep current extension compatibility aliases (notably Dim Amount).
2. Extend typed result banks and native runtime state for rotation, duration, speed, moving-head slots, pixel metadata, progress/pick/gates, provider-backed audio/textures, and deferred operations. Preserve serialization/undo semantics. Tests first; isolate bank merge rules from per-behavior evaluation.
3. Port pure generators/converters/combiners and color/pixel/spatial operations with numerical regression tests; correct existing deviations only against source evidence. Add catalog metadata, typed editors, enums and standalone icons.
4. Port retained timing/envelopes/delay/recording/selection/transition behaviors. Specify clock/seed/helper limits. Persist their state in session checkpoints. Test edge boundaries, frame-rate behavior and runtime continuity.
5. Add bounded host-provider contracts and native data processing for audio, textures, OSC, controller/keyboard/Xbox/remote/global/sequence/resource context. Providers report missing services explicitly; documented fallback behavior remains distinct from an unsupported no-op. Keep file/network/DSP acquisition off evaluation scheduling where appropriate.
6. Port setup, macro, deferred assignment/director/global/memory actions and transitions, using engine-owned commands and off-thread preparation. Avoid per-frame assignment storms and cycles. Validate driver/context ownership and history boundaries.
7. Trace the editable-only, deprecated, legacy Pong and test-widget registrations individually. Implement observed semantics; list genuinely absent source definitions as unresolved with exact evidence, never as working placeholders.
8. Run all cargo fmt/test/clippy, catalog completeness/default construction and semantic fixtures, Node syntax and browser catalog/editor tests on a separate capture-only server. Report per-key capabilities/limits; request activation only if necessary to protect current live history.

Each step depends on the prior bank/provider contracts; pure families can be factored into separate modules without changing ownership. No remote/PR workflow is assumed (current checkout reports no usable Git work tree). Rollback uses the existing running executable and untouched live project plus checkpoints.

## Exit criteria

All 178 documented keys must have real implementations and per-key semantic fixtures covering nondefault parameters and composition. Track implemented, blocked and unverified separately; a blocked key is incomplete work and cannot satisfy the completeness gate. No currently documented missing source has yet been established; resolve constructor references against actual CategoryReference selections. No silent provider fallback unless native contract declares it. Current extensions remain usable, project structure stays modular, history remains bounded and serializable, and output tests stay on loopback. Clearly distinguish implemented ports from exact native dependency/runtime parity.

Progress: all 178 documented keys implemented and broad construction/evaluation/checkpoint-continuity coverage added. Focused family and integration regressions are present. Exhaustive per-key nondefault/composition golden conformance remains unverified; the stricter conformance exit criterion is not claimed complete. See docs/catalog-port.md and docs/port-coverage.json.

Review corrections: retain full `contract` objects (178/178), resolve editable definitions before classifying blockers, add actual pixel positions/direction and explicit setup-channel context, and build concrete acquisition/processing adapters (not availability-only provider stubs). Assignment pipeline must resolve named registries and target identity including layer, apply group before fixture commands, execute fixture evaluations before director assignments, keep node/setup/director phases separate, deduplicate same-name assignments and adopt finished transition children.
