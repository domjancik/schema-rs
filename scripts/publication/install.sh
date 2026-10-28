#!/bin/sh
set -eu
HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
command -v python3 >/dev/null 2>&1 || { echo 'Python 3 is required (Debian/Armbian: sudo apt install python3).' >&2; exit 1; }
exec python3 "$HERE/install_linux.py" "$@"
