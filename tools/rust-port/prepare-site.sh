#!/usr/bin/env bash
# Prepare the seeksnack site exactly as it was for the golden build.
#
# Usage: tools/rust-port/prepare-site.sh <site-git-url-or-path> <workdir>
#
# Creates <workdir>/seeksnack (the directory name matters: Hugo's GetRemote
# file cache is keyed by the project directory's basename). The golden build
# used site commit ae6c922 with `npm ci` dependencies, plus files that are
# ignored by the site's git repo (reproduced here from site-overlay/).
set -euo pipefail

SITE_SRC=${1:?site git URL or path}
WORKDIR=${2:?work directory}
SITE_COMMIT=ae6c922623eb3d2796d80b5bb286b33c91f85438
HERE=$(cd "$(dirname "$0")" && pwd)

mkdir -p "$WORKDIR"
SITE="$WORKDIR/seeksnack"
rm -rf "$SITE"
git clone --quiet "$SITE_SRC" "$SITE"
git -C "$SITE" checkout --quiet "$SITE_COMMIT"

cd "$SITE"
# tool/netlify.sh regenerates static/admin/{config.yml,index.html}; the
# committed files already equal its (macOS) output, and it leaves .bak copies
# of the originals. It is not run here because GNU and BSD sed disagree on
# its `\s` pattern.
cp static/admin/config.yml static/admin/config.yml.bak
cp static/admin/index.html static/admin/index.html.bak
# Finder metadata that was present in static/ and is copied to the output.
cp "$HERE/site-overlay/static/.DS_Store" static/.DS_Store

npm ci --no-audit --no-fund
cp -r node_modules/@fortawesome/fontawesome-free/webfonts static/assets/
rm -rf resources public hugo_stats.json .hugo_build.lock

echo "prepared $SITE"
