# Schéma scene lab

The scene lab develops practical scenes and loadable example projects, exercises authoring workflows, and collects verified patterns, guides, bugs and requested additions. It does not change implementation code.

## Chats and responsibilities

- Implementation: **Schéma full system architecture analysis**, `01a178a8-206e-7871-b678-3590d226cf9e`, host `local`. Owns code, fixes, deployment and release notes.
- Scene lab: **Schéma scene lab — examples, projects and workflow findings**, `01a17f23-52a2-7fb2-b6b3-4bc7aebde853`, host `local`. Owns example scene data, guides and reproducible findings.

The user explicitly authorized two-way reporting and sharing relevant implementation developments between these chats. Send messages through the chat tools; a shared file alone does not notify the other chat. This agreement is event-driven and does not establish a scheduled monitor.

## Working agreement

Use the Rust implementation at `C:/Users/magne/dev/schema-rs`; the saved Schema project points to its original sibling `scnq-schema`.

The lab may write documentation in this folder, authored example data in `examples/scene-lab/`, and scratch work under `target/scene-lab/`. Use an isolated engine bound to loopback with output disabled. Leave the live Studio, Orange Pi, credentials, authored project and their histories untouched. No application source, tests, scripts, dependency or deployment changes by the lab.

The lab owns `findings.md` and topic-specific guides. Implementation owns `developments.md`. Both can update this index, coordinating overlapping edits. Do not commit or overwrite unrelated work.

For findings, assign stable `SL-###` IDs and record severity, baseline commit, reproduction steps, expected versus actual behavior, evidence, workaround and status. Distinguish bugs, missing capabilities and usability requests. Fixes should reference the finding ID; lab verification closes a finding as verified rather than a code change alone.

For examples, record the intended effect, fixture arrangement and patch, required models/resources, layers/drivers, native or script choices, load/run instructions, validation evidence and known limitations. Named saves currently share workspace resources/plugins and are not standalone portable bundles. Keep output disabled; software capture is not physical fixture validation.

## Update loop

1. Lab reports concrete validated scenes, blockers and new findings to implementation with artifact paths and commit baseline.
2. Implementation triages findings and records relevant changes in `developments.md` with commits, activation state, compatibility effects and suggested retests.
3. After relevant implementation changes, implementation sends the lab a concise update linking those notes. Check this agreement when resuming work.
4. Lab retests affected examples and updates findings/guides. Implementation reviews results and incorporates reusable learnings into the main docs when appropriate.

## Initial batch

- UV COB wash: two-slot model (dimmer, constant-zero program), correct next-fixture addressing, groups and midnight-crossing daily dimming.
- Spatial ripple/chase: fixture references or moving emitters, mapping, dimming and native motion.
- Repeat/Bezier: reusable independent layers and property drivers.

Start from the actual catalog and compatibility documentation, and explicitly report unsupported requirements.
