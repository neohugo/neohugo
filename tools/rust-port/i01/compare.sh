#!/usr/bin/env bash
# I01 end-to-end comparison: builds one site with the Go neohugo and with neohugo-rs, from fresh
# copies at the same path, with the same flags and environment, and compares the output trees
# (plus hugo_stats.json) byte for byte.
#
# Usage: tools/rust-port/i01/compare.sh <site>... | t24 | all     (sites: tools/rust-port/i01/sites.py list)
#
# Environment:
#   NEOHUGO_GO    the Go binary       (default: $I01_WORK/bin/neohugo-go-arm64 if it exists, else
#                                      $I01_WORK/bin/neohugo-go). An aarch64 binary runs under
#                                      qemu-aarch64-static on other hosts. The Rust port reproduces
#                                      the arm64 Go build (FMA in flate, gift, strconv; see
#                                      docs/rust-port/HANDOFF.md §3), so on amd64 the reference must
#                                      be the arm64 build (build-go-arm64.sh, deleted in T00 of
#                                      REWRITE_PLAN.md; at tag go-parity-final); the native amd64
#                                      build differs in processed-image bytes. The rewrite compares
#                                      structurally with tools/neohugo/compare.sh (T03) instead.
#   NEOHUGO_RS    the Rust binary     (default: /tmp/targets/i01/release/neohugo-rs)
#   I01_WORK      scratch dir         (default: $TMPDIR/neohugo-i01); every build happens below it,
#                                      never in the repository
#   I01_NODE_MODULES  a node_modules dir with postcss-cli (copied into the sites that use postCSS)
#   I01_KEEP=1    keep the site copies and outputs
#
# Flags: --minify --clock 2026-09-27T12:00:00Z -d <out>, run from the site dir, one render worker
# (HUGO_NUMWORKERMULTIPLIER=1), a fresh HUGO_CACHEDIR per build, a clean environment (HOME in the
# scratch dir), NEOHUGO_ESBUILD_BINARY=<repo>/tools/esbuild/bin/esbuild (tools/esbuild/build.sh).
# Each build's ERROR/WARN lines are printed; the full logs stay in $I01_WORK/<site>/.
set -uo pipefail
export PYTHONDONTWRITEBYTECODE=1
# I01_NOMINIFY=1 builds without --minify (to tell template differences from minifier ones).
minify_flag=--minify
[ "${I01_NOMINIFY:-}" = 1 ] && minify_flag=

HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../../.." && pwd)
WORK=${I01_WORK:-${TMPDIR:-/tmp}/neohugo-i01}
if [ -z "${NEOHUGO_GO:-}" ] && [ -x "$WORK/bin/neohugo-go-arm64" ]; then
	GO_BIN=$WORK/bin/neohugo-go-arm64
else
	GO_BIN=${NEOHUGO_GO:-$WORK/bin/neohugo-go}
fi
GO_RUN=()
if file -L "$GO_BIN" | grep -q 'ARM aarch64' && [ "$(uname -m)" != aarch64 ]; then
	GO_RUN=(qemu-aarch64-static)
fi
RS_BIN=${NEOHUGO_RS:-/tmp/targets/i01/release/neohugo-rs}
ESBUILD=$ROOT/tools/esbuild/bin/esbuild
NODE_DIR=$(dirname "$(command -v node || echo /usr/bin/node)")

case "$WORK" in "$ROOT"|"$ROOT"/*) echo "I01_WORK must be outside the repository" >&2; exit 2 ;; esac
[ -x "$GO_BIN" ] || { echo "Go binary not found: $GO_BIN" >&2; exit 2; }
[ -x "$RS_BIN" ] || { echo "Rust binary not found: $RS_BIN" >&2; exit 2; }
[ -x "$ESBUILD" ] || { echo "run tools/esbuild/build.sh first" >&2; exit 2; }

# "t24" expands to every T24 build site, "all" to every site.
sites=()
for a in "$@"; do
	case $a in
	t24) mapfile -t -O "${#sites[@]}" sites < <(python3 "$HERE/sites.py" list | grep '^t24-') ;;
	all) mapfile -t -O "${#sites[@]}" sites < <(python3 "$HERE/sites.py" list) ;;
	*) sites+=("$a") ;;
	esac
done

status=0
for site in "${sites[@]}"; do
	base=$site
	case $site in t24-*) base=${site#t24-} ;; esac
	W=$WORK/$site
	rm -rf "$W"
	mkdir -p "$W"
	for impl in go rust; do
		run=("${GO_RUN[@]}" "$GO_BIN")
		[ $impl = rust ] && run=("$RS_BIN")
		S=$W/site/$base
		rm -rf "$W/site" "$W/cache" "$W/home"
		mkdir -p "$W/site" "$W/cache" "$W/home"
		python3 "$HERE/sites.py" make "$site" "$S" || exit 2
		python3 "$HERE/sites.py" cache "$site" "$W/cache" || exit 2
		if [ -n "${I01_NODE_MODULES:-}" ] && [ -f "$S/postcss.config.js" ]; then
			cp -a "$I01_NODE_MODULES" "$S/node_modules"
		fi
		start=$(date +%s.%N)
		(cd "$S" && env -i HOME="$W/home" PATH="$NODE_DIR:/usr/local/bin:/usr/bin:/bin" \
			HUGO_CACHEDIR="$W/cache" HUGO_NUMWORKERMULTIPLIER=1 NEOHUGO_ESBUILD_BINARY="$ESBUILD" \
			"${run[@]}" ${minify_flag} --clock 2026-09-27T12:00:00Z -d "$W/out-$impl") >"$W/$impl.log" 2>&1
		code=$?
		end=$(date +%s.%N)
		echo "== $site [$impl] exit $code ($(awk "BEGIN{printf \"%.1f\", $end - $start}") s)"
		grep -E "ERROR|WARN|panicked|Error:" "$W/$impl.log" | head -40 | sed -e "s#$W#\$W#g" -e "s/^/   /"
		mkdir -p "$W/out-$impl"
		[ -f "$S/hugo_stats.json" ] && cp "$S/hugo_stats.json" "$W/out-$impl/.hugo_stats.json.site"
		echo "$code" >"$W/$impl.exit"
	done
	[ "${I01_KEEP:-}" = 1 ] || rm -rf "$W/site" "$W/cache" "$W/home"
	python3 "$HERE/diff.py" "$W/out-go" "$W/out-rust" --label "$site" --json "$W/diff.json" \
		--logs "$W/go.log" "$W/rust.log" || status=1
	if [ "$(cat "$W/go.exit")" != "$(cat "$W/rust.exit")" ]; then
		echo "$site: exit codes differ: go $(cat "$W/go.exit"), rust $(cat "$W/rust.exit")"
		status=1
	fi
	[ "${I01_KEEP:-}" = 1 ] || rm -rf "$W/out-go" "$W/out-rust"
done
exit $status
