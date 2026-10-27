# Automatic Orange Pi updates

Run `powershell -File scripts/start-update-server.ps1` on the Windows development host. The dashboard is [Update control](http://127.0.0.1:8890/). Python 3.11+ with `paramiko` and `cryptography`, Rust (native Windows and `aarch64-unknown-linux-musl` target), Node and Chrome are required. Pass `-Python` for another environment. The launcher reuses an existing healthy server and otherwise starts it hidden; it does not install a Windows boot task.

## Everyday workflow

1. Commit the engine/UI changes you want to ship. Uncommitted work is never released. The watcher detects changed build inputs, snapshots that exact commit and builds in its own cache, independent of the working checkout.
2. Formatting, full Rust tests, clippy, JavaScript syntax, native startup/routes and Chrome WebGPU smoke must pass before a release is signed and marked ready. ARM64 engine and Windows download are released together.
3. While the update server runs, it polls the controller's configured LAN/AP endpoints over pinned SSH. A connected device receives the newest ready release automatically. An offline device catches up on reconnect. Already-installed releases are skipped.
4. Use **Build latest commit** or **Push ready build** explicitly when needed. Pushes can queue while offline. **Pause automatic pushes** keeps builds running; **Pause commit watching** disables automatic builds. A failed release is quarantined until **Retry latest push** or a newer release.
5. **Roll back to previous** uses a previously trusted controller release and pins it. **Resume automatic pushes** clears the pin. Rollback restores software while retaining the current authored scene/history, rather than reverting the scene to a historical release.

The dashboard shows connectivity, installed/ready release, job phases, validation results and bounded logs. It binds loopback only. Same-origin session and CSRF checks protect actions; the API accepts only fixed workflow actions. It does not expose credentials or arbitrary commands.

## State and trust

`%LOCALAPPDATA%/Schema-s/updates/` holds `config.json`, SQLite jobs/policies, immutable signed releases, build snapshots/cache and logs. Configuration selects repository, port, known endpoints, SSH host-key alias/key and Credential Manager target. The signing private key stays in Windows Credential Manager at `schema-rs/updates/signing-key`; sudo reuses `schema-rs/orangepi/sudo`. Only the public key is installed on the controller. Back up the signing credential securely before moving hosts; a different key is rejected and requires deliberate rotation.

The controller's root-owned `/usr/local/libexec/schema-update/` helper accepts fixed-name, bounded, regular artifacts signed by that authority. It verifies all hashes, target architecture, desktop metadata and monotonic sequence before stopping the engine. Sequence validation avoids dependence on controller wall-clock/RTC time. SSH rejects unknown host keys. The release staging folder alone grants no installation authority.

Each deployment checkpoints the session, stops Schéma gracefully, keeps a private checkpoint backup, switches the immutable release/download symlinks, and starts the new engine with `.update-in-progress` present. This short barrier freezes evaluation/actions/clock and rejects edits during exact project/undo/redo/revision verification. Successful validation releases the barrier. OLA PID must remain unchanged. Output configuration is preserved; there is a brief Schéma output interruption while its service restarts.

A root journal plus systemd `ExecStopPost` recovery reverses failed switches. A boot recovery dependency restores an interrupted transaction before Schéma starts. The transaction runs independently of the host SSH session. If the host disappears, the device completes or recovers; the host reconciles its operation ID when it returns. Existing AP, OLA configuration, firewall and external RTC are not modified.

A ready release is trusted code from a local commit, not a sandboxed untrusted build. Keep repository and signing-host access limited to maintainers. Builds and root checkpoint backups are retained for diagnosis; disk cleanup/retention is currently manual. Schema/scene-format migrations that require irreversible writes need a separate migration strategy; this workflow verifies the current serialized format and rolls back mismatches.

## Verification

- `python tests/update_server_test.py` checks readiness/signatures/tampering, policy, durable queues/sequence and GUI authorization.
- On Linux, `python tests/update_device_signature_test.py` verifies OpenSSL ingestion, tampered artifacts/signatures, stale sequences and nonblocking FIFO rejection using temporary data.
- On Linux, `python tests/update_device_transaction_test.py` runs isolated fake-service transaction tests, including failed health checks and boot recovery. It never accesses the live controller paths/services.
- `cargo test --test update_barrier` verifies held evaluation, HTTP/direct edit rejection, clock resumption and undo.
- Full repository checks remain mandatory. Ready-build workflows execute them again against their committed source snapshot.

The first live deployment and subsequent idempotent push are recorded in [verification evidence](update-server-verification.json). The final host/helper hardening is active independently of the native engine release.
