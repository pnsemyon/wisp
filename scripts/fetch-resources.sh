#!/usr/bin/env bash
# Fetch the pinned sing-box.exe and wintun.dll into resources/.
# Usage: ./scripts/fetch-resources.sh [SINGBOX_TAG]
# Defaults to the pinned tag below (kept in sync with the config schema
# wisp-core generates); pass an explicit tag (e.g. v1.13.15-extended-2.5.0)
# to override, or an empty string to resolve the latest release instead.
#
# The bundled engine is shtorm-7/sing-box-extended, a fork of mainline
# sing-box (identical config schema) that adds Xray transports, notably
# xhttp, which mainline sing-box doesn't support. See
# crates/wisp-core/src/singbox.rs for the config-generation side of this.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RES="$ROOT/resources"
mkdir -p "$RES"

WINTUN_VERSION="0.14.1"
SINGBOX_REPO="shtorm-7/sing-box-extended"
# Pinned so the bundled engine's config schema always matches what
# wisp-core generates. Bumping this requires re-validating build_config
# against the new sing-box binary (see crates/wisp-core/src/singbox.rs).
DEFAULT_SINGBOX_TAG="v1.13.14-extended-2.5.0"
SINGBOX_TAG="${1-$DEFAULT_SINGBOX_TAG}"

need() { command -v "$1" >/dev/null 2>&1 || { echo "error: '$1' is required" >&2; exit 1; }; }
need curl
need unzip

if [ -z "$SINGBOX_TAG" ]; then
  echo "Resolving latest sing-box-extended release..."
  # Authenticate the API call when a token is available (e.g. in CI) so we don't
  # hit GitHub's low unauthenticated rate limit on shared runner IPs.
  api_auth=()
  if [ -n "${GITHUB_TOKEN:-}" ]; then
    api_auth=(-H "Authorization: Bearer ${GITHUB_TOKEN}")
  fi
  SINGBOX_TAG="$(curl -fsSL "${api_auth[@]}" "https://api.github.com/repos/${SINGBOX_REPO}/releases/latest" \
    | grep -oE '"tag_name": *"v[^"]+"' | head -1 | grep -oE 'v[0-9][^"]+')"
fi
# Asset version is the tag without the leading "v" (e.g. "1.13.14-extended-2.5.0").
SINGBOX_ASSET_VERSION="${SINGBOX_TAG#v}"
echo "sing-box-extended tag: $SINGBOX_TAG (asset version: $SINGBOX_ASSET_VERSION)"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# --- sing-box (windows amd64), from the xhttp-capable extended fork ---
# Archive contains a nested sing-box-<asset-version>-windows-amd64/ folder;
# the recursive find below handles that regardless of the exact layout.
SB_ZIP="sing-box-${SINGBOX_ASSET_VERSION}-windows-amd64.zip"
SB_URL="https://github.com/${SINGBOX_REPO}/releases/download/${SINGBOX_TAG}/${SB_ZIP}"
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

echo "Done. Pinned sing-box-extended ${SINGBOX_TAG}, wintun ${WINTUN_VERSION}."
echo "$SINGBOX_ASSET_VERSION" > "$RES/.singbox-version"
