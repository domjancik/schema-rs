# Schema-s on macOS

macOS 13 or newer, Apple Silicon and Intel. The Rust engine serves Studio and schémaboy in the system browser and supports the TUI and Art-Net. Native macOS execution, Safari and physical USB validation must be completed on a Mac before a public release is advertised. Windows and Linux keep their existing launch and output paths.

## Running the app

Extract a signed release and move `Schema-s.app` to Applications. Open it normally; its menu-bar item offers Open Studio and Quit. Studio opens at `http://127.0.0.1:8797/?view=studio`. Quit interrupts only the app's own engine and waits for the durable session flush. A conflicting port produces an error rather than opening another server. The menu-bar process keeps the engine active while it runs; a closed laptop or system sleep can still interrupt lighting output.

Projects and undo history live in `~/Library/Application Support/Schema-s/project`, with engine logs in `~/Library/Logs/Schema-s/engine.log`. App replacement leaves this data directory alone. Normal session recovery disarms output. The old Linux-style project directory is not migrated or deleted automatically; existing users can use `Contents/MacOS/schema-rs desktop --project OLD_DIRECTORY` to retain that workspace.

The app binds to loopback by default. Phone control requires an explicitly configured LAN host and authentication, as on other platforms. Allow local-network access when macOS asks: Art-Net unicast needs it. The bundle supplies a usage description. No firewall, driver or administrator settings are changed automatically. See [Apple's local-network guidance](https://developer.apple.com/documentation/technotes/tn3179-understanding-local-network-privacy).

Start at login is opt-in through System Settings → General → Login Items. For headless use, a per-user LaunchAgent may run the installed engine with `desktop --no-open --bind 127.0.0.1:8797`. Do not run a second engine against the same project directory, even on another port. This implementation does not install a system daemon or alter the machine's login configuration.

## Open DMX USB

Optional FTDI D2XX output uses the same serialized sender as Windows, selecting devices by serial and retaining the queue-drain timing guard. Art-Net remains available without any USB library. Linux continues to use Art-Net/OLA rather than this backend.

The loader tries the app's `Contents/Frameworks/libftd2xx.dylib`, then explicit installed locations `/usr/local/lib/libftd2xx.dylib` and `/opt/homebrew/lib/libftd2xx.dylib`. It never searches the working directory. Missing or unloadable libraries produce a discovery error without preventing engine startup. The library must match the running architecture. A universal app needs a universal dylib for both CPU types.

Hardened runtime may reject an external library signed by another team. Bundle and sign the licensed dylib with the app instead of disabling library validation. Supply `--ftdi PATH --ftdi-license PATH` to the native builder; redistribution requires the unchanged vendor license. Nothing downloads or installs a driver automatically. Follow [current FTDI guidance](https://ftdichip.com/drivers/d2xx-drivers/); do not disable Apple security protections or unload system drivers based on older instructions. Real-device timing, disconnect/reconnect and sleep tests remain required.

## Build and sign

On a Mac with Xcode command-line tools, Rust and Python 3.11+, run:

```sh
python3 scripts/macos/build_native.py --output target/mac-dev --build-number 1
```

This builds both Rust targets and Swift launchers, combines each pair with `lipo`, collects dependency notices, creates the icon and bundles an explicit set of public files. It sets a macOS 13 deployment target. The development artifact is named `Schema-s-macos-DEVELOPMENT.tar.gz`; modes and framework symlinks survive artifact transport. It is not a public, notarized release.

For distribution, use an installed Developer ID identity and a notarytool Keychain profile:

```sh
python3 scripts/macos/build_native.py --output target/mac-release --build-number 2 \
  --identity "Developer ID Application: YOUR TEAM (TEAMID)" \
  --notary-profile schema-s-notary
```

Use a fresh output directory for each build. Signed releases require a clean Git checkout so the commit identifies the exact source. Increment the build number for every published update. The marketing version comes from Cargo.toml. Signing uses hardened runtime, signs nested code before enclosing bundles, verifies signatures, submits to notarization, staples and verifies the ticket, checks Gatekeeper, then creates the final ZIP, checksum and release attestation. No unsigned fallback is published. Credentials belong in Keychains or CI secrets; do not commit them. See [Apple's notarization workflow](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution).

`.github/workflows/macos.yml` configures native Rust tests/lints, Swift compilation and a loopback engine startup/shutdown smoke on Apple Silicon and Intel. Manual dispatch also produces a universal development bundle. It does not use signing credentials or upload a public release. There is currently no Git remote configured here, so this workflow has not run remotely.

## Updates and public downloads

The launcher can use Sparkle 2 when a trusted SDK, public Ed25519 key and HTTPS appcast are configured. The SDK is supplied explicitly, not fetched from an unpinned latest URL:

```sh
python3 scripts/macos/build_native.py --output target/mac-updatable --build-number 3 \
  --identity "Developer ID Application: YOUR TEAM (TEAMID)" --notary-profile schema-s-notary \
  --sparkle-sdk /path/to/Sparkle-SDK --update-key PUBLIC_BASE64_KEY \
  --update-feed https://YOUR-HOST/downloads/appcast.xml
```

The SDK directory must contain `Sparkle.framework` and `LICENSE`. The launcher then offers Check for Updates and uses Sparkle's standard consent/preferences. Without this complete configuration it opens the downloads page. Projects and history remain outside the application bundle. Private update signing keys stay in the release Mac's Keychain. After notarization, use the SDK's `generate_appcast` against the final release archive directory and publish its signed enclosure/feed at the configured HTTPS URLs. Follow [Sparkle's distribution instructions](https://sparkle-project.org/documentation/); no feed or automatic installation is active until a real signed channel is configured and tested, including graceful engine shutdown and update failure recovery.

Public site packaging accepts `--macos PATH/Schema-s-macos-universal.zip` alongside its existing Windows/Linux inputs. The adjacent `.release.json` from the native notarization job must match the source commit, filename, size and SHA-256. Only import artifacts from that trusted job; the sidecar itself is not a cryptographic notarization verifier. The landing page enables its Mac link only for an available notarized manifest entry. Development bundles are never promoted through this path.

## Release acceptance on real hardware

Before publishing: run both native CI jobs; launch/quit/reopen and recover undo history; confirm Safari and Chrome scene views, phone access and local-network permission behavior; test loopback Art-Net then the intended receiver; verify Open DMX on a real FTDI adapter with reconnect and sleep/wake; test a signed Sparkle upgrade and failed update with the engine running. Cross-target Rust checks and packaging fixtures do not establish these hardware results.
