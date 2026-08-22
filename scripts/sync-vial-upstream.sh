#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/third_party/vial-gui"
UPSTREAM="https://github.com/vial-kb/vial-gui.git"
PIN="aef8222a2d0429a183b2ed692d5f9efcfd383f08"

mkdir -p "$ROOT/third_party"

if [[ -d "$DEST/.git" ]]; then
  git -C "$DEST" remote set-url origin "$UPSTREAM"
  git -C "$DEST" fetch --tags --prune origin
else
  rm -rf "$DEST"
  git clone "$UPSTREAM" "$DEST"
fi

git -C "$DEST" checkout --detach "$PIN"
printf 'Vial upstream synced to %s\n' "$PIN"
printf 'License: GPL-2.0; see third_party/vial-gui/COPYING and THIRD_PARTY_NOTICES.md\n'
