#!/usr/bin/env bash
# Build the prepared seeksnack site with the neohugo binary of this tree (or a
# Go neohugo built at 44529028) using the golden build settings.
#
# Usage: tools/rust-port/build-site.sh <neohugo-binary> <workdir>/seeksnack <outdir>
#
# - --clock pins `now` (the copyright year appears in the output).
# - HUGO_NUMWORKERMULTIPLIER=1 makes the Go build deterministic (13 taxonomy
#   terms collide on the same URL; the last one in tree order wins).
# - HUGO_CACHEDIR points at the checked-in GetRemote (YouTube API) cache so
#   the build is offline and reproducible.
# - The resources/ directory must not exist: the golden is a cold-cache build.
# - esbuild 0.25.6 is needed for js.Build; neohugo looks for it via
#   NEOHUGO_ESBUILD_BINARY (install with
#   `tools/neohugo/node.sh && tools/esbuild/install.sh`).
set -euo pipefail

BIN=${1:?neohugo binary}
SITE=${2:?prepared site dir}
OUT=${3:?output dir}
HERE=$(cd "$(dirname "$0")" && pwd)

[ "$(basename "$SITE")" = seeksnack ] || { echo "site dir must be named seeksnack" >&2; exit 1; }
rm -rf "$SITE/resources" "$OUT"
cd "$SITE"
HUGO_CACHEDIR="$HERE/testdata/hugo_cache" HUGO_NUMWORKERMULTIPLIER=1 \
  "$BIN" --minify --clock 2026-09-27T12:00:00Z -d "$OUT"
