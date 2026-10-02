#!/usr/bin/env bash
# Build the prepared seeksnack site with the neohugo binary of this tree (or a
# Go neohugo built at 44529028) using the golden build settings.
#
# Usage: tools/rust-port/build-site.sh <neohugo-binary> <workdir>/seeksnack <outdir>
#
# - --clock pins `now` (the copyright year appears in the output).
# - HUGO_NUMWORKERMULTIPLIER=1 makes the Go build deterministic (13 taxonomy
#   terms collide on the same URL; the last one in tree order wins).
# - NEOHUGO_CACHEDIR (this tree's neohugo; the Go build reads HUGO_CACHEDIR)
#   points at the checked-in GetRemote (YouTube API) cache so the build is
#   offline and reproducible.
# - The resources/ directory must not exist: the golden is a cold-cache build.
# - This tree's neohugo bundles js.Build in process; the Go build at 44529028
#   linked esbuild 0.25.6 in.
set -euo pipefail

BIN=${1:?neohugo binary}
SITE=${2:?prepared site dir}
OUT=${3:?output dir}
HERE=$(cd "$(dirname "$0")" && pwd)

[ "$(basename "$SITE")" = seeksnack ] || { echo "site dir must be named seeksnack" >&2; exit 1; }
rm -rf "$SITE/resources" "$OUT"
cd "$SITE"
NEOHUGO_CACHEDIR="$HERE/testdata/hugo_cache" \
  HUGO_CACHEDIR="$HERE/testdata/hugo_cache" HUGO_NUMWORKERMULTIPLIER=1 \
  "$BIN" --minify --clock 2026-09-27T12:00:00Z -d "$OUT"
