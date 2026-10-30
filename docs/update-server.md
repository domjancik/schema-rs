# Automatic Schéma delivery

Run `powershell -File scripts/start-update-server.ps1` on the Windows development host. The dashboard is [Update control](http://127.0.0.1:8890/). Python 3.11+ with `paramiko` and `cryptography`, Rust (native Windows and `aarch64-unknown-linux-musl` target), Node and Chrome are required. Pass `-Python` for another environment. The launcher reuses an existing healthy server and otherwise starts it hidden; the approved current-user “Schéma automatic updates” logon task runs the same launcher with standard permissions. PowerShell execution policy is unchanged.

## Everyday workflow

1. Commit the engine/UI changes you want to ship. Uncommitted work is never released. The watcher detects changed build inputs, snapshots that exact commit and builds in its own cache, independent of the working checkout.
2. Formatting, full Rust tests, clippy, JavaScript syntax, native startup/routes and Chrome WebGPU smoke must pass before a release is signed and marked ready. ARM64 engine and Windows download are released together. Updater regression tests also gate readiness.
3. While the update server runs, it polls the controller's configured LAN/AP endpoints over pinned SSH. A connected device receives the newest ready release automatically. An offline device catches up on reconnect. Already-installed releases are skipped.
4. Use **Build latest commit** or **Push ready build** explicitly when needed. Pushes can queue while offline. **Pause automatic pushes** keeps builds running; **Pause commit watching** disables automatic builds. A failed release is quarantined until **Retry latest push** or a newer release.
5. **Roll back to previous** uses a previously trusted controller release and pins it. **Resume automatic pushes** clears the pin. Rollback restores software while retaining the current authored scene/history, rather than reverting the scene to a historical release.

Configured local Studio and desktop hosts receive the same verified Windows build. A shutdown-file request flushes their final session; the new process starts behind a barrier and its migrated project, history, inputs, plugin state and clock are compared before edits resume. Failed activation restores the original checkpoint bytes and previous executable. An older unmanaged desktop with active work must be closed once before enrollment.

Public delivery fast-forwards the enrolled GitHub main branch, waits for the exact commit’s Browser engine (including Linux x64) and macOS workflows, and publishes the browser app plus Windows, Linux x64/ARM64 and unsigned macOS downloads. It checks source provenance and independently streams and hashes every production download through Node. Publication staging is sealed and reused across retries, so package hashes remain stable. Commands run without interactive stdin and timed-out subprocess trees are stopped. An enrolled `publication.vercel_config` directory reuses the current user’s existing Vercel login; no token is copied into project configuration. Failed CI withholds publication; disconnected devices catch up when available. This is enrollment, not network discovery: only configured targets are updated.

The dashboard shows connectivity, installed/ready release, per-target delivery, job phases, validation results and bounded logs. **Retry delivery** retries failed local/public targets; automatic retries never bypass validation. It binds loopback only. Same-origin session and CSRF checks protect actions; the API accepts only fixed workflow actions. It does not expose credentials or arbitrary commands.

## State and trust

`%LOCALAPPDATA%/Schema-s/updates/` holds `config.json`, SQLite jobs/policies, immutable signed releases, build snapshots/cache and logs. Configuration selects repository, port, known endpoints, SSH host-key alias/key and Credential Manager target. The signing private key stays in Windows Credential Manager at `schema-rs/updates/signing-key`; sudo reuses `schema-rs/orangepi/sudo`. Only the public key is installed on the controller. Back up the signing credential securely before moving hosts; a different key is rejected and requires deliberate rotation.

The controller's root-owned `/usr/local/libexec/schema-update/` helper accepts fixed-name, bounded, regular artifacts signed by that authority. It verifies all hashes, target architecture, desktop metadata and monotonic sequence before stopping the engine. Sequence validation avoids dependence on controller wall-clock/RTC time. SSH rejects unknown host keys. The release staging folder alone grants no installation authority.

Each deployment checkpoints the session, stops Schéma gracefully, keeps a private checkpoint backup, switches the immutable release/download symlinks, and starts the new engine with `.update-in-progress` present. This short barrier freezes evaluation/actions/clock and rejects edits during exact project/undo/redo/revision verification. Successful validation releases the barrier. OLA PID must remain unchanged. Output configuration is preserved; there is a brief Schéma output interruption while its service restarts.

A root journal plus systemd `ExecStopPost` recovery reverses failed switches. A boot recovery dependency restores an interrupted transaction before Schéma starts. The transaction runs independently of the host SSH session. If the host disappears, the device completes or recovers; the host reconciles its operation ID when it returns. Existing AP, OLA configuration, firewall and external RTC are not modified.

A ready release is trusted code from a local commit, not a sandboxed untrusted build. Keep repository and signing-host access limited to maintainers. Builds and root checkpoint backups are retained for diagnosis; disk cleanup/retention is currently manual. Candidate session migrations run on a comparison copy. Activation preserves the original bytes for rollback and verifies the normalized state before releasing the barrier.

## Verification

- `python tests/update_delivery_test.py` checks exact-commit CI gates, rollback, session comparisons and enrolled-target validation.
- `python tests/update_server_test.py` checks readiness/signatures/tampering, policy, durable queues/sequence and GUI authorization.
- On Linux, `python tests/update_device_signature_test.py` verifies OpenSSL ingestion, tampered artifacts/signatures, stale sequences and nonblocking FIFO rejection using temporary data.
- On Linux, `python tests/update_device_transaction_test.py` runs isolated fake-service transaction tests, including failed health checks and boot recovery. It never accesses the live controller paths/services.
- `cargo test --test update_barrier` verifies held evaluation, HTTP/direct edit rejection, clock resumption and undo.
- Full repository checks remain mandatory. Ready-build workflows execute them again against their committed source snapshot.

The first live deployment and subsequent idempotent push are recorded in [verification evidence](update-server-verification.json). The final host/helper hardening is active independently of the native engine release.

## Queued scene-authoring work

Named signals should alias stable properties on genuine zero-footprint virtual fixtures, using the normal native evaluation graph and driver trees. A shared Scene controls panel should expose live values and driver editing in Studio and Schémaboy. Preserve revisioned undo, project/session/bundle persistence and safe dangling-reference behavior. Global output dimming remains distinct from creative controls. Scene-lab examples are authored data, not an instruction to replace a live installation scene.

## Unattended Vercel authentication

An interactive OAuth login may be rejected by a Windows login task. A dedicated deployment token can be entered in a masked local dialog using `target/deploy-venv/Scripts/python.exe scripts/update_server/connect_vercel.py --state "$env:LOCALAPPDATA/Schema-s/updates"`. The helper verifies access to the enrolled project and writes the token directly to Windows Credential Manager (`schema-rs/updates/vercel-token`), then retries publication. Never paste the token into chat or project files. The publisher passes it only through the child process environment. Native device delivery continues while public publication shows **needs auth**. The configured direct `publication.vercel_cli` entry point avoids npm-wrapper credential differences.
