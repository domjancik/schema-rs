# Public publication

Production: https://schema-s.vercel.app (Vercel project `schema-s`, scope `domjanciks-projects`). The public site is static HTML/CSS/JS plus actual release archives; no Rust engine, live project, secrets or session history is deployed. Device CTAs open `/connect`, validate a user-entered HTTP(S) origin without credentials/query/path, then navigate to that host's Studio or schémaboy. Home-screen installation remains on the device origin.

## Ready artifacts

The first public runtime is source 6f41d631342c. Its Windows/ARM64 files come from signed ready release `6f41d631342c-e3a5a3ca5478-4`, which passed full Rust tests, fmt, clippy, JS syntax and native Chrome GPU smoke checks. Linux x64 was built from clean 4bedaa0; `git diff 6f41d63 4bedaa0 -- src web Cargo.toml Cargo.lock LICENSE` is empty, so runtime inputs are identical. Uncommitted scene-lab data and paused desktop refinements were excluded.

Public assets under `/downloads/`:

- `Schema-s-Windows-x64.zip`
- `Schema-s-linux-x64.tar.gz`
- `Schema-s-linux-arm64.tar.gz`
- `install.sh` (architecture-selecting HTTPS bootstrap)
- `SHA256SUMS.txt` and `latest.json` (bytes, SHA-256 and source identity)

Linux archives contain a native binary, inspectable installer, build metadata, license, dependency notices and usage guide. The boot installer supports systemd on 64-bit ARM/x64 and preserves projects/credentials/history. It refuses existing custom Schema services so it cannot replace a configured Orange Pi updater or touch its RTC/AP/OLA setup. See [Linux usage](linux.md).

## Repeat the release

Use a clean committed runtime snapshot. Never package the live working directory or project. Build Windows x64 and both `aarch64-unknown-linux-musl` / `x86_64-unknown-linux-musl` targets. On the Windows host, the installed Rust `rust-lld.exe` linker supports the musl targets with per-target `-C linker-flavor=ld.lld -C link-self-contained=yes`. Use a single worker where memory is limited. Collect dependency notices with `scripts/build_desktop.py`'s `collect_notices`, and produce the Windows ZIP with its `package` function or use the matching signed ready release.

Run `python scripts/build_public_release.py --linux-x64 BINARY --linux-arm64 BINARY --windows ZIP --notices DIRECTORY --commit SOURCE_COMMIT`. It writes only explicit public files to `target/publication/site`. The exporter excludes live data and unrelated files from the download folder. Installer `.sh`/`.py` files use LF. Check package contents and hashes, then run:

```
python tests/public_release_test.py
python3 tests/public_installer_test.py
python3 tests/linux-release-smoke.py target/publication/downloads/Schema-s-linux-x64.tar.gz
```

The last two run on Linux (WSL works). The smoke extracts to a temporary directory, starts only loopback/output-disabled engines, exercises editing/undo and verifies persisted history across graceful restart. Test ARM64 on a reachable 64-bit board independently; no USB fixture parity is implied by packaging or the x64 smoke.

Link only the public staging directory with `npx --yes vercel@latest link --cwd target/publication/site --project schema-s --scope domjanciks-projects --yes`. The CLI can create `.env.local` while linking; remove that generated file from staging before deploying. `.vercel` and `.env.local` are never part of the public allowlist. No GitHub integration, public source repository, domain purchase or paid resource is configured.

Deploy with `npx --yes vercel@latest deploy --cwd target/publication/site --scope domjanciks-projects --prod --yes`. Verify the unauthenticated production alias, not only the deployment-specific URL (which can be protected). Set `PUBLIC_SITE_URL=https://schema-s.vercel.app` and `PLAYWRIGHT_MODULE` to run `node tests/public-site-browser.cjs`. Fetch every archive independently and compare bytes/SHA-256 against the published manifest. Test the public bootstrap with `--prefix` to stage files without changing local services.

The website advertises only artifacts present in its manifest. Repeat publication manually when a new ready build is intended for public distribution; the existing Orange Pi signed updater remains a separate private workflow. Download bandwidth follows the hosting account's plan. Native/USB output and network routing remain device-local.

## Current verification limits

Linux x64 executed successfully in Ubuntu/WSL, including web assets, output-disabled startup, authored edit, undo and durable restart. Installer tests cover preservation, unmanaged-service refusal, checksum failure and rollback on unhealthy startup. System files are staged under a temporary prefix during installation tests. No new service was installed on the real Orange Pi, whose SSH endpoint was unavailable during publication. The ARM64 archive is architecture-verified and contains the signed pipeline's ready binary; board/USB behavior was not retested for this public release.

Known mobile issue SL-005: at320×568, the scene's footer can cover fixture markers. The scene lab reproduced this identically before and after the skin; fixture-row selection works. This publication does not claim it fixed. Existing main controls remain within the tested phone viewport.
