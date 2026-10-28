#!/bin/sh
# Download first, inspect, then run: sudo sh install.sh
set -eu
BASE=https://schema-s.vercel.app/downloads
if [ "${1:-}" = '--help' ]; then echo 'sudo sh install.sh [--port 8787] [--bind 0.0.0.0] [--dry-run]'; exit 0; fi
for tool in curl python3 sha256sum tar; do command -v "$tool" >/dev/null 2>&1 || { echo "Required: $tool" >&2; exit 1; }; done
case $(uname -m) in aarch64|arm64) PLATFORM=linux-arm64;; x86_64|amd64) PLATFORM=linux-x64;; *) echo 'Requires 64-bit ARM or x64 Linux; 32-bit Raspberry Pi OS is unsupported.' >&2;exit 1;; esac
WORK=$(mktemp -d)
trap 'rm -rf -- "$WORK"' EXIT HUP INT TERM
curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 "$BASE/latest.json" -o "$WORK/latest.json"
python3 - "$WORK/latest.json" "$PLATFORM" > "$WORK/artifact" <<'PY'
import json,re,sys
r=json.load(open(sys.argv[1]));a=r['platforms'][sys.argv[2]]
assert a['filename']=='Schema-s-'+sys.argv[2]+'.tar.gz' and re.fullmatch('[0-9a-f]{64}',a['sha256'])
print(a['filename']);print(a['sha256'])
PY
NAME=$(sed -n '1p' "$WORK/artifact");SUM=$(sed -n '2p' "$WORK/artifact")
curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 "$BASE/$NAME" -o "$WORK/$NAME"
printf '%s  %s\n' "$SUM" "$WORK/$NAME" | sha256sum -c -
# Extraction is user-owned until the downloaded, checksummed installer is invoked.
tar -xzf "$WORK/$NAME" -C "$WORK"
sh "$WORK/Schema-s/install.sh" "$@"
