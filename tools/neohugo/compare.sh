#!/usr/bin/env bash
# Acceptance comparison of one site (docs/rust-port/REWRITE_PLAN.md §7.2, §7.3): builds the
# candidate (the Rust build), compares it with the reference (the Go build's committed golden
# data) through tools/neohugo/structdiff.py, level by level (L1-L4, the structure oracle, A7),
# and applies the ratchet (testdata/baselines/<label>.json).
#
#   tools/neohugo/compare.sh <site> [--docs-patches i01|reduced] [--ref golden]
#                            [--task ID]... [--update] [--report-only] [--show N]
#
# Sites: testsite, seeksnack, docs-i01, docs-reduced (`docs --docs-patches <variant>` is the same
# as `docs-<variant>`).
#
# Sides:
#   --ref golden   the reference, the only one: the committed golden data of
#                  testdata/golden/<label> (manifests of both passes and the structure dump;
#                  T01's oracle.sh wrote them with the Go build, frozen at 44529028)
#   candidate      neohugo: the site from `sites.py make <label> --overlay sites/<site>`;
#                  its structure dump comes from the unminified build (NEOHUGO_STRUCTURE_OUT)
#
# The candidate builds each pass from a freshly generated site, from the site directory, with
#   <binary> --clock 2026-09-27T12:00:00Z [--minify] -d <out>
# (the minified pass gives L1 and L4, the unminified pass L1, L2 and L3) in the clean
# environment of the golden builds: HOME and HUGO_CACHEDIR in the work directory (the site's
# golden GetRemote entries, `sites.py cache`), TZ=UTC, HUGO_NUMWORKERMULTIPLIER=1, every proxy
# variable pointing at a refusing port (outbound HTTP disabled), the node modules of
# tools/neohugo/node.sh as a `node_modules` symlink in the site plus `node_modules/.bin` on PATH,
# NEOHUGO_NODE_MODULES and NEOHUGO_ESBUILD_BINARY. Manifests come from tools/neohugo/manifest.py
# (the candidate's with --full-text, for the A7 similarity of the worst pages).
#
# Ratchet: --task names the changes files (tools/neohugo/changes/<ID>.md) that list this task's
# changes; an unlisted new difference fails; --update writes the baseline for the listed changes;
# --report-only never fails. Without a baseline file every difference is new.
#
# Environment:
#   KEEP=1                  keep the sites, outputs, manifests and logs of the work directory
#   NEOHUGO_COMPARE_WORK    work directory (default: $TMPDIR/neohugo-compare), outside the repo
#   NEOHUGO_BINARY          the neohugo binary (default: `cargo build --offline --locked -p
#                           neohugo`, then a copy of target/debug/neohugo in the work dir)
#   NEOHUGO_NODE_MODULES    the node modules (default: see tools/neohugo/node.sh)
#   NEOHUGO_ESBUILD_BINARY  esbuild for the Rust build (default: tools/esbuild/bin/esbuild of the
#                           main checkout)
#   NEOHUGO_TASK            default for --task
# The report and structdiff.json stay in the work directory (<work>/<label>/); everything else
# there is deleted unless KEEP=1.
set -euo pipefail
export PYTHONDONTWRITEBYTECODE=1

HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../.." && pwd)
SITES_PY=$ROOT/tools/rust-port/i01/sites.py
MANIFEST_PY=$HERE/manifest.py
STRUCTDIFF_PY=$HERE/structdiff.py
CLOCK=2026-09-27T12:00:00Z

log() { echo "compare.sh: $*" >&2; }
usage() {
	sed -n '2,/^set -euo/p' "$0" | sed -e '$d' -e 's/^# \{0,1\}//' >&2
	exit 2
}

main_checkout() {
	dirname "$(git -C "$HERE" rev-parse --path-format=absolute --git-common-dir)"
}

site= variant= ref=golden update= report_only= show=40
tasks=()
[ -n "${NEOHUGO_TASK:-}" ] && tasks+=("$NEOHUGO_TASK")
while [ $# -gt 0 ]; do
	case $1 in
	--docs-patches) variant=$2; shift 2 ;;
	--ref) ref=$2; shift 2 ;;
	--task) tasks+=("$2"); shift 2 ;;
	--update) update=1; shift ;;
	--report-only) report_only=1; shift ;;
	--show) show=$2; shift 2 ;;
	-h | --help) usage ;;
	-*) log "unknown option $1"; usage ;;
	*) [ -z "$site" ] || usage; site=$1; shift ;;
	esac
done
[ -n "$site" ] || usage

case $site in
docs) label=docs-${variant:-i01} ;;
docs-i01 | docs-reduced)
	[ -z "$variant" ] || [ "docs-$variant" = "$site" ] || { log "$site contradicts --docs-patches $variant"; exit 2; }
	label=$site ;;
testsite | seeksnack)
	[ -z "$variant" ] || { log "--docs-patches applies to docs only"; exit 2; }
	label=$site ;;
*) log "unknown site $site (testsite, seeksnack, docs-i01, docs-reduced)"; exit 2 ;;
esac
overlay=$ROOT/sites/${label%%-*}

NODE_MODULES=${NEOHUGO_NODE_MODULES:-$("$HERE/node.sh" path)}
ESBUILD=${NEOHUGO_ESBUILD_BINARY:-$(main_checkout)/tools/esbuild/bin/esbuild}
GOLDEN=$ROOT/testdata/golden/$label
BASELINE=$ROOT/testdata/baselines/$label.json
WORK_ROOT=${NEOHUGO_COMPARE_WORK:-${TMPDIR:-/tmp}/neohugo-compare}
W=$WORK_ROOT/$label
case "$WORK_ROOT" in "$ROOT" | "$ROOT"/*) log "the work directory must be outside the repository"; exit 2 ;; esac

[ "$ref" = golden ] || { log "--ref golden (the only reference)"; exit 2; }
[ -d "$GOLDEN" ] || { log "no golden data at $GOLDEN"; exit 2; }
[ -d "$NODE_MODULES" ] || log "warning: no node modules at $NODE_MODULES (tools/neohugo/node.sh)"

rm -rf "$W"
mkdir -p "$W"
cleanup() {
	[ "${KEEP:-}" = 1 ] && return
	find "$W" -mindepth 1 -maxdepth 1 ! -name report.txt ! -name structdiff.json -exec rm -rf {} +
}
trap cleanup EXIT

neohugo_binary() {
	if [ -n "${NEOHUGO_BINARY:-}" ]; then
		echo "$NEOHUGO_BINARY"
		return
	fi
	log "building neohugo (cargo build --offline --locked -p neohugo)"
	(cd "$ROOT" && cargo build --offline --locked -q -p neohugo --bin neohugo >&2)
	local target=${CARGO_TARGET_DIR:-$ROOT/target}
	case $target in /*) ;; *) target=$ROOT/$target ;; esac
	# A copy: the target directory is shared, another build may replace the file.
	mkdir -p "$W/bin"
	cp "$target/debug/neohugo" "$W/bin/neohugo"
	echo "$W/bin/neohugo"
}

# build <dir> <structure-out|""> [args…]: a fresh site in <dir>/<label>, built by neohugo into
# <dir>/out; the log goes to <dir>/log.
build() {
	local dir=$1 structure=$2
	shift 2
	rm -rf "$dir"
	mkdir -p "$dir/home" "$dir/cache"
	python3 "$SITES_PY" make "$label" "$dir/$label" --overlay "$overlay" >/dev/null
	python3 "$SITES_PY" cache "$label" "$dir/cache"
	ln -s "$NODE_MODULES" "$dir/$label/node_modules"
	local node_dir
	node_dir=$(dirname "$(command -v node || echo /usr/bin/node)")
	local env=(HOME="$dir/home" TZ=UTC LANG=C.UTF-8
		PATH="$NODE_MODULES/.bin:$node_dir:/usr/local/bin:/usr/bin:/bin"
		HTTP_PROXY=http://127.0.0.1:9 HTTPS_PROXY=http://127.0.0.1:9 ALL_PROXY=http://127.0.0.1:9
		http_proxy=http://127.0.0.1:9 https_proxy=http://127.0.0.1:9 all_proxy=http://127.0.0.1:9
		NO_PROXY= no_proxy= HUGO_CACHEDIR="$dir/cache" HUGO_NUMWORKERMULTIPLIER=1
		NEOHUGO_NODE_MODULES="$NODE_MODULES" NEOHUGO_ESBUILD_BINARY="$ESBUILD")
	[ -n "$structure" ] && env+=(NEOHUGO_STRUCTURE_OUT="$structure")
	local start end
	start=$(date +%s)
	if ! (cd "$dir/$label" && env -i "${env[@]}" "$BIN" --clock "$CLOCK" "$@" -d "$dir/out" >"$dir/log" 2>&1); then
		log "$label: the build failed ($dir/log):"
		sed -e "s#$dir#\$W#g" "$dir/log" | tail -40 >&2
		exit 1
	fi
	end=$(date +%s)
	log "$label: build $(basename "$dir") in $((end - start)) s"
	grep -E "ERROR|WARN" "$dir/log" | sed -e "s#$dir#\$W#g" -e "s/^/  [rust] /" | head -20 >&2 || true
}

# side <name>: builds both passes (and the structure dump) of one side; sets <name>_min,
# <name>_unmin, <name>_structure to the manifest and dump files.
side() {
	local name=$1
	for pass in minified unminified; do
		local dir=$W/$name-$pass args=() levels=L1,L2,L3 full=() structure=
		if [ $pass = minified ]; then
			args=(--minify)
			levels=L1,L4
		else
			full=(--full-text)
			structure=$W/$name.structure.json
		fi
		build "$dir" "$structure" "${args[@]}"
		python3 "$MANIFEST_PY" extract "$dir/out" --project "$dir/$label" --site "$label" \
			--pass $pass --levels $levels "${full[@]}" -o "$W/$name.manifest.$pass.json"
		[ "${KEEP:-}" = 1 ] || rm -rf "$dir"
	done
	eval "${name}_min=\$W/\$name.manifest.minified.json"
	eval "${name}_unmin=\$W/\$name.manifest.unminified.json"
	eval "${name}_structure=\$W/\$name.structure.json"
}

BIN=$(neohugo_binary)
ref_min=$GOLDEN/manifest.minified.json
ref_unmin=$GOLDEN/manifest.unminified.json
ref_structure=$GOLDEN/structure.json
side cand

args=(compare --site "$label" --show "$show"
	--ref-name golden --ref-min "$ref_min" --ref-unmin "$ref_unmin" --ref-structure "$ref_structure"
	--cand-name rust --cand-min "$cand_min" --cand-unmin "$cand_unmin" --cand-structure "$cand_structure"
	--json "$W/structdiff.json" --report "$W/report.txt" --baseline "$BASELINE")
for t in "${tasks[@]}"; do args+=(--task "$t"); done
[ -n "$update" ] && args+=(--update)
[ -n "$report_only" ] && args+=(--report-only)
status=0
python3 "$STRUCTDIFF_PY" "${args[@]}" || status=$?
log "$label: report $W/report.txt, $W/structdiff.json$([ "${KEEP:-}" = 1 ] && echo "; outputs kept in $W")"
exit $status
