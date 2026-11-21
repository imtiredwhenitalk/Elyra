#!/usr/bin/env sh
# Refresh the Linux packaging icon from the Tauri icon set.
# Run from anywhere: sh packaging/linux/copy-icon.sh
set -eu

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
root=$(CDPATH= cd -- "$here/../.." && pwd)
src="$root/apps/desktop/src-tauri/icons/128x128@2x.png"
dst="$here/elyra.png"

if [ ! -f "$src" ]; then
  echo "Missing source icon: $src" >&2
  exit 1
fi

cp "$src" "$dst"

# Freedesktop hicolor icons for desktop entries should include a 256x256 PNG.
if ! file "$dst" | grep -q '256 x 256'; then
  echo "Expected a 256x256 PNG at $dst, got: $(file "$dst")" >&2
  exit 1
fi

echo "Copied 256x256 icon to $dst"
if command -v sha256sum >/dev/null 2>&1; then
  echo "SHA-256: $(sha256sum "$dst" | awk '{print $1}')"
fi
