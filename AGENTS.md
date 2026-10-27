# Working in schema-rs

Keep the engine as the sole owner of authored/running state. Clients submit commands with expected revisions and render published snapshots. Preserve independent fixture layers, separate driver trees and the layout/content/stack project boundaries.

Use native Rust for common real-time operations; Script.Value is the bounded extension boundary. Add semantic regression tests before behavior changes. Never register an unsupported behavior as a pass-through. Declare compatibility limits in docs/compatibility.md.

Validate with cargo fmt --check, cargo test, cargo clippy --all-targets -- -D warnings, and node --check web/app.js. Transport tests must use loopback/captured packets. Keep output disabled in browser/test projects unless explicitly testing a loopback receiver. Do not claim physical fixture or gamma parity from these tests.

## Scene lab coordination

The user authorized ongoing two-way collaboration with the scene lab chat. Read `docs/scene-lab/README.md` for chat IDs, ownership and the reporting agreement. After relevant authoring, node, project, output or UI changes, record the commit and activation state in `docs/scene-lab/developments.md` and send the scene lab a concise update with affected examples to retest. Review its findings before re-investigating known issues. The lab authors data/docs only and uses isolated output-disabled engines; preserve live user scenes and history.
