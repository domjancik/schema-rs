"""Native Mac universal build. Credentials stay in the caller's Keychain.

Run from a Mac; no public upload or package installation is performed.
"""
import argparse
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from build_desktop import collect_notices
from build_macos import bundle, sign_and_archive


def run(*args):
    subprocess.run([str(arg) for arg in args], cwd=ROOT, check=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=ROOT / "target/macos-build")
    parser.add_argument("--build-number", required=True, type=int)
    parser.add_argument("--identity")
    parser.add_argument("--notary-profile")
    parser.add_argument("--sparkle-sdk", type=Path)
    parser.add_argument("--update-feed")
    parser.add_argument("--update-key")
    parser.add_argument("--ftdi", type=Path)
    parser.add_argument("--ftdi-license", type=Path)
    args = parser.parse_args()
    if sys.platform != "darwin":
        parser.error("Xcode/macOS required; cross-checking Rust is not a native release build")
    if bool(args.identity) != bool(args.notary_profile):
        parser.error("Signing identity and notary Keychain profile must be supplied together")
    if any([args.sparkle_sdk, args.update_key, args.update_feed]) and not all([args.sparkle_sdk, args.update_key, args.update_feed, args.identity]):
        parser.error("Sparkle SDK, public key, HTTPS feed and signed release configuration are required together")
    if args.identity:
        dirty = subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT, text=True).strip()
        if dirty:
            parser.error("Signed releases require a clean source checkout so BUILD.json identifies the exact source")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    if (output / "Schema-s.app").exists():
        parser.error("Use a fresh output folder; existing bundles are preserved")
    os.environ["MACOSX_DEPLOYMENT_TARGET"] = "13.0"
    os.environ["CARGO_TARGET_DIR"] = str(output / "cargo")
    os.environ.setdefault("CARGO_BUILD_JOBS", "2")
    engines, launchers = [], []
    for cpu, target in [("arm64", "aarch64-apple-darwin"), ("x86_64", "x86_64-apple-darwin")]:
        run("rustup", "target", "add", target)
        run("cargo", "build", "--locked", "--release", "--target", target)
        engines.append(output / "cargo" / target / "release/schema-rs")
        launcher = output / ("launcher-" + cpu)
        flags = []
        if args.sparkle_sdk:
            flags = ["-F", args.sparkle_sdk.resolve(), "-framework", "Sparkle", "-Xlinker", "-rpath", "-Xlinker", "@executable_path/../Frameworks"]
        run("xcrun", "swiftc", "-swift-version", "5", "-O", "-target", cpu + "-apple-macos13.0", *flags, ROOT / "scripts/macos/Launcher.swift", "-o", launcher)
        launchers.append(launcher)
    engine, launcher = output / "engine-universal", output / "launcher-universal"
    run("lipo", "-create", *engines, "-output", engine)
    run("lipo", "-create", *launchers, "-output", launcher)
    run("cargo", "fetch", "--locked")
    notices = output / "notices"
    collect_notices(notices)
    commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    sdk = args.sparkle_sdk.resolve() if args.sparkle_sdk else None
    app = bundle(engine, launcher, output, commit, notices, "macos-universal", args.ftdi, args.ftdi_license,
                 sdk / "Sparkle.framework" if sdk else None, sdk / "LICENSE" if sdk else None,
                 args.update_key, args.update_feed, args.build_number)
    # System icon tooling preserves the existing application branding.
    iconset = output / "AppIcon.iconset"
    iconset.mkdir()
    for size in [16, 32, 128, 256, 512]:
        for scale in [1, 2]:
            pixels = size * scale
            run("sips", "-z", pixels, pixels, ROOT / "web/app-icon-512.png", "--out", iconset / ("icon_" + str(size) + "x" + str(size) + ("@2x" if scale == 2 else "") + ".png"))
    run("iconutil", "-c", "icns", iconset, "-o", app / "Contents/Resources/AppIcon.icns")
    import plistlib
    info_path = app / "Contents/Info.plist"
    info = plistlib.loads(info_path.read_bytes())
    info["CFBundleIconFile"] = "AppIcon"
    info_path.write_bytes(plistlib.dumps(info))
    artifacts = output / "artifacts"
    artifacts.mkdir(exist_ok=True)
    if args.identity:
        print(sign_and_archive(app, artifacts, "macos-universal", args.identity, args.notary_profile))
    else:
        # tar preserves modes and symlinks across CI artifact upload/download.
        run("tar", "-czf", artifacts / "Schema-s-macos-DEVELOPMENT.tar.gz", "-C", output, "Schema-s.app")
        print("Development bundle only; not notarized or advertised as a public download")


if __name__ == "__main__":
    main()
