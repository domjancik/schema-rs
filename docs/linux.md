# Schema-s on Linux and Raspberry Pi–class devices

Use 64-bit Debian, Ubuntu, Raspberry Pi OS or Armbian on ARM64, or an x64 Linux PC. The self-contained musl binary does not need Rust or glibc. A modern browser runs the controls; the native engine stays on the host. 32-bit ARM, macOS and GPU simulation on the server are not included.

## Portable use

Download the matching `.tar.gz` and `SHA256SUMS.txt`. Verify with `sha256sum --ignore-missing -c SHA256SUMS.txt`, then extract the archive. Inside `Schema-s`, run:

```
./schema-rs desktop --no-open
```

Open the address printed by the engine (normally `http://127.0.0.1:8797/welcome`). Art-Net starts disabled. Press Ctrl+C for graceful session save. Projects and history persist separately from the executable under the desktop app-data directory, or pass `--project PATH` explicitly. The TUI is `./schema-rs tui --server 127.0.0.1:8797`.

## Boot service / phone access

The extracted package includes an inspectable installer. Python3 and systemd are required (normally included in Debian/Armbian). Read `install_linux.py`, then run:

```
sudo sh ./install.sh
```

It installs to `/opt/schema/releases/`, serves on `0.0.0.0:8787`, enables `schema.service` at boot, and uses `/var/lib/schema` for project/session data. Use `--port 8797` or `--bind 127.0.0.1` to change listening. `--dry-run` prints the plan; `--no-start` enables boot startup without starting immediately. Network hosting uses a generated token in root-readable `/etc/schema/schema.env`; retrieve it with `sudo cat /etc/schema/schema.env`, then enter it in Studio's Connection control. Open `http://DEVICE-IP:8787/welcome` from the same network. Do not expose this plain HTTP control service directly to the Internet. Phone installation/camera permissions need a trusted HTTPS device origin; the public marketing site does not replace the local engine.

Alternatively download the public bootstrap, inspect it, then run it. It selects x64/ARM64 and verifies the package checksum over HTTPS:

```
curl -fL https://schema-s.vercel.app/downloads/install.sh -o install-schema.sh
less install-schema.sh
sudo sh install-schema.sh
```

There are no automatic apt installs, AP/firewall changes, OLA changes or RTC writes. Existing custom `schema.service` installations (including the managed Orange Pi) are refused with instructions to use their existing updater. Generic-installer upgrades retain credentials, authored projects and serialized undo/redo. The native server's normal restart disarms Art-Net. An unsuccessful service health check rolls the executable/service launcher back; retained project data is not deleted. Stop with `sudo systemctl stop schema`, inspect with `journalctl -u schema`, and disable boot with `sudo systemctl disable schema`.

OLA/Open DMX bridging and Wi-Fi AP setup are separate device-specific tasks; this installer serves native Art-Net output and browser/TUI controls. It preserves existing external RTC configuration. ARM64 binaries should be tested on the intended board; building an ARM binary alone does not verify a USB DMX adapter or fixture.


Windows-to-ARM64 release builds now include the TLS client used by the library manager. The update builder provisions a workspace-local Zig 0.15.2 C compiler from the [official release](https://ziglang.org/download/0.15.2/), checks the pinned archive SHA-256 before extraction, and uses it only for ARM64 C dependencies. This is a build-time tool; the deployed Pi binary remains self-contained. Compiler paths/caches stay under the updater’s tools directory. Native Linux/macOS builds use their normal toolchains. HTTPS certificate verification remains enabled.
