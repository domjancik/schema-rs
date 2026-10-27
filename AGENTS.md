# Working in schema-rs

Keep the engine as the sole owner of authored/running state. Clients submit commands with expected revisions and render published snapshots. Preserve independent fixture layers, separate driver trees and the layout/content/stack project boundaries.

Use native Rust for common real-time operations; Script.Value is the bounded extension boundary. Add semantic regression tests before behavior changes. Never register an unsupported behavior as a pass-through. Declare compatibility limits in docs/compatibility.md.

Validate with cargo fmt --check, cargo test, cargo clippy --all-targets -- -D warnings, and node --check web/app.js. Transport tests must use loopback/captured packets. Keep output disabled in browser/test projects unless explicitly testing a loopback receiver. Do not claim physical fixture or gamma parity from these tests.
