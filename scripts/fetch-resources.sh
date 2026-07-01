#!/usr/bin/env bash
# Fetch the pinned sing-box.exe and wintun.dll into resources/.
# Usage: ./scripts/fetch-resources.sh [SINGBOX_VERSION]
# If no version is given, the latest sing-box release is used.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RES="$ROOT/resources"
mkdir -p "$RES"

WINTUN_VERSION="0.14.1"
SINGBOX_VERSION="${1:-}"

need() { command -v "$1" >/dev/null 2>&1 || { echo "error: '$1' is required" >&2; exit 1; }; }
need curl
need unzip

if [ -z "$SINGBOX_VERSION" ]; then
  echo "Resolving latest sing-box release..."
  SINGBOX_VERSION="$(curl -fsSL https://api.github.com/repos/SagerNet/sing-box/releases/latest \
    | grep -oE '"tag_name": *"v[^"]+"' | head -1 | grep -oE 'v[0-9][^"]+')"
fi
SINGBOX_VERSION="${SINGBOX_VERSION#v}"
echo "sing-box version: $SINGBOX_VERSION"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# --- sing-box (windows amd64) ---
SB_ZIP="sing-box-${SINGBOX_VERSION}-windows-amd64.zip"
SB_URL="https://github.com/SagerNet/sing-box/releases/download/v${SINGBOX_VERSION}/${SB_ZIP}"
echo "Downloading $SB_URL"
curl -fSL "$SB_URL" -o "$TMP/$SB_ZIP"
unzip -o -q "$TMP/$SB_ZIP" -d "$TMP/sb"
find "$TMP/sb" -name 'sing-box.exe' -exec cp {} "$RES/sing-box.exe" \;
echo "-> resources/sing-box.exe"

# --- wintun.dll (amd64) ---
WT_ZIP="wintun-${WINTUN_VERSION}.zip"
WT_URL="https://www.wintun.net/builds/${WT_ZIP}"
echo "Downloading $WT_URL"
curl -fSL "$WT_URL" -o "$TMP/$WT_ZIP"
unzip -o -q "$TMP/$WT_ZIP" -d "$TMP/wt"
cp "$TMP/wt/wintun/bin/amd64/wintun.dll" "$RES/wintun.dll"
echo "-> resources/wintun.dll"

echo "Done. Pinned sing-box v${SINGBOX_VERSION}, wintun ${WINTUN_VERSION}."
echo "$SINGBOX_VERSION" > "$RES/.singbox-version"
