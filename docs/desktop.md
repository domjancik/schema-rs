# Schema-s for Windows

Download the portable ZIP from `/welcome#download`, extract it, and double-click **Start Schema-s.cmd**. Windows 10/11 x64 and a current browser are required. This is an unsigned preview; there is no installer, native window wrapper, automatic updater or signing certificate.

The launcher runs the bundled native Rust engine and opens `http://127.0.0.1:8797/welcome` in your default browser. Choose Open Studio or Play schémaboy. The console is the engine: leave it open while using the app; stop with Ctrl+C to flush the current session. Browser tabs can be closed and reopened independently.

Projects and undo history live in `%LOCALAPPDATA%/Schema-s/project`, separately from the extracted application. Replacing the application folder does not replace that data. Normal startup restores authored state/history and disarms Art-Net; enable output explicitly when ready. Loading a named project also disarms output. Phone/LAN hosting is a separate deliberate server configuration, not enabled by the desktop launcher.

If port 8797 is occupied, the launcher reports the error rather than taking over another engine. Use `schema-rs.exe desktop --bind 127.0.0.1:8799` to choose another port, or `--project DIR` for another workspace. `--no-open` starts the engine without opening a browser. Browser storage/install preferences are origin-specific; server-side project data is independent of the port.

## Build and distribute

On Windows, run `python scripts/build_desktop.py`. Rust 1.88+ and Windows build tools are needed for building, not for running the archive. The script uses a locked release build, collects dependency license notices, and creates:

- `target/distribution/current/Schema-s-Windows-x64.zip`
- `target/distribution/current/SHA256SUMS.txt`
- `target/distribution/current/latest.json` (version, source commit, bytes, SHA-256)

The archive contains only the executable, static launcher, usage guide, license and dependency notices. It excludes live projects, passwords, tokens and session files. Verify the checksum and smoke-run the extracted binary before publishing.

The development frontend serves this staging folder at `/downloads/`. Native hosts serve only the known archive/checksum/metadata names from `<project>/downloads/`; copy those three public release files there. Without a complete matching release, the landing page displays download unavailable. There is currently no configured remote Git repository, public download host, or public release URL for this implementation. macOS/Linux desktop archives are not produced by this build.
