#!/usr/bin/env bash
# The Go oracle of the Rust rewrite (docs/rust-port/REWRITE_PLAN.md §6.4, §7.2): builds the
# target sites with the Go neohugo, natively and with outbound HTTP disabled, and writes the
# golden data to rust/testdata/golden (schemas in its README.md).
#
#   tools/neohugo/oracle.sh install        build the Go binaries (bin/neohugo, bin/neohugo-structure)
#   tools/neohugo/oracle.sh sites [LABEL…] manifests + structure dumps (default: all labels)
#   tools/neohugo/oracle.sh images         the golden images (golden/images/manifest.json recipes)
#   tools/neohugo/oracle.sh all            install (when a binary is missing), sites, images
#   tools/neohugo/oracle.sh check          regenerate everything into a temporary directory and
#                                          diff it against rust/testdata/golden (idempotency)
#   tools/neohugo/oracle.sh bin            print the binary directory
#
# Labels: testsite, seeksnack, docs-i01, docs-reduced (manifests of a minified and an unminified
# build, plus the structure dump) and mini (structure dump only).
#
# Per label and pass, the site is generated afresh by tools/rust-port/i01/sites.py outside the
# repository and built from its directory with
#   neohugo --clock 2026-09-27T12:00:00Z [--minify] -d <out>
# in a clean environment: HOME and HUGO_CACHEDIR in the work directory (the cache holds the
# site's golden GetRemote entries, `sites.py cache`), TZ=UTC, one render worker
# (HUGO_NUMWORKERMULTIPLIER=1: colliding targets have one last writer), every proxy variable
# pointing at a refusing port (127.0.0.1:9), and the node modules of tools/neohugo/node.sh as a
# `node_modules` symlink in the site plus `node_modules/.bin` on PATH. Then
#   golden/<label>/manifest.minified.json[.gz]    tools/neohugo/manifest.py, levels L1,L4
#   golden/<label>/manifest.unminified.json[.gz]  levels L1,L2,L3
#   golden/<label>/structure.json[.gz]            the structure oracle (tools/go-oracle/structure)
# Files over 256 KiB are gzipped (deterministically); the other form is removed.
#
# Environment:
#   NEOHUGO_TOOLS_BIN      the binaries (default: tools/neohugo/bin of the main checkout, which
#                          every worktree shares; gitignored)
#   NEOHUGO_NODE_MODULES   the node modules (default: see tools/neohugo/node.sh)
#   NEOHUGO_ORACLE_WORK    work directory (default: $TMPDIR/neohugo-oracle), outside the repo
#   NEOHUGO_GOLDEN         output directory (default: rust/testdata/golden of this checkout)
#   KEEP=1                 keep the sites, outputs and logs of the work directory
# `install` needs Go and the module cache (GOPROXY is used for anything missing).
set -euo pipefail
export PYTHONDONTWRITEBYTECODE=1

HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../.." && pwd)
SITES_PY=$ROOT/tools/rust-port/i01/sites.py
MANIFEST_PY=$HERE/manifest.py
CLOCK=2026-09-27T12:00:00Z
LABELS=(testsite seeksnack docs-i01 docs-reduced mini)
GZIP_OVER=$((256 * 1024))

main_checkout() {
	dirname "$(git -C "$HERE" rev-parse --path-format=absolute --git-common-dir)"
}

BIN=${NEOHUGO_TOOLS_BIN:-$(main_checkout)/tools/neohugo/bin}
NODE_MODULES=${NEOHUGO_NODE_MODULES:-$("$HERE/node.sh" path)}
WORK=${NEOHUGO_ORACLE_WORK:-${TMPDIR:-/tmp}/neohugo-oracle}
GOLDEN=${NEOHUGO_GOLDEN:-$ROOT/rust/testdata/golden}

case "$WORK" in "$ROOT" | "$ROOT"/*) echo "oracle.sh: the work directory must be outside the repository" >&2; exit 2 ;; esac

log() { echo "oracle.sh: $*" >&2; }

install() {
	command -v go >/dev/null || { log "go not found"; exit 1; }
	mkdir -p "$BIN"
	local before after
	before=$(cat "$ROOT/go.mod" "$ROOT/go.sum" | cksum)
	log "building $BIN/neohugo"
	(cd "$ROOT" && GOFLAGS=-mod=readonly go build -trimpath -ldflags="-s -w" -o "$BIN/neohugo" .)
	log "building $BIN/neohugo-structure (go build -overlay)"
	(cd "$ROOT" && GOFLAGS=-mod=readonly go run -trimpath ./tools/go-oracle/structure -install "$BIN/neohugo-structure")
	after=$(cat "$ROOT/go.mod" "$ROOT/go.sum" | cksum)
	[ "$before" = "$after" ] || { log "go.mod/go.sum changed"; exit 1; }
	"$BIN/neohugo" version
}

need_bins() {
	for b in neohugo neohugo-structure; do
		[ -x "$BIN/$b" ] || { log "$BIN/$b missing (run tools/neohugo/oracle.sh install)"; exit 1; }
	done
}

# build <label> <site-name> <dir> <binary> [args…]: a fresh site in <dir>/<site-name>, built by
# <binary>; the output goes to <dir>/out, the log to <dir>/log.
build() {
	local label=$1 name=$2 dir=$3 bin=$4
	shift 4
	rm -rf "$dir"
	mkdir -p "$dir/home" "$dir/cache"
	python3 "$SITES_PY" make "$label" "$dir/$name" >/dev/null
	python3 "$SITES_PY" cache "$label" "$dir/cache"
	ln -s "$NODE_MODULES" "$dir/$name/node_modules"
	local node_dir
	node_dir=$(dirname "$(command -v node || echo /usr/bin/node)")
	if ! (cd "$dir/$name" && env -i HOME="$dir/home" TZ=UTC LANG=C.UTF-8 \
		PATH="$NODE_MODULES/.bin:$node_dir:/usr/local/bin:/usr/bin:/bin" \
		HTTP_PROXY=http://127.0.0.1:9 HTTPS_PROXY=http://127.0.0.1:9 ALL_PROXY=http://127.0.0.1:9 \
		http_proxy=http://127.0.0.1:9 https_proxy=http://127.0.0.1:9 all_proxy=http://127.0.0.1:9 \
		NO_PROXY= no_proxy= HUGO_CACHEDIR="$dir/cache" HUGO_NUMWORKERMULTIPLIER=1 \
		${NH_STRUCTURE_OUT:+NH_STRUCTURE_OUT="$NH_STRUCTURE_OUT"} \
		"$bin" --clock "$CLOCK" "$@" -d "$dir/out" >"$dir/log" 2>&1); then
		log "$label: the build failed:"
		sed -e "s#$dir#\$W#g" "$dir/log" >&2
		exit 1
	fi
	grep -E "ERROR|WARN" "$dir/log" | sed -e "s#$dir#\$W#g" -e "s/^/  [$label] /" >&2 || true
}

# put <tmp-file> <dest-without-.gz>: moves a finished JSON file into place, gzipped when large.
put() {
	local tmp=$1 dest=$2
	rm -f "$dest" "$dest.gz"
	if [ "$(stat -c %s "$tmp")" -gt "$GZIP_OVER" ]; then
		python3 -c 'import gzip, io, sys
data = open(sys.argv[1], "rb").read()
buf = io.BytesIO()
with gzip.GzipFile(filename="", mode="wb", fileobj=buf, compresslevel=9, mtime=0) as gz:
    gz.write(data)
open(sys.argv[2], "wb").write(buf.getvalue())' "$tmp" "$dest.gz"
		rm -f "$tmp"
	else
		mv "$tmp" "$dest"
	fi
}

sites() {
	need_bins
	"$HERE/node.sh" check >/dev/null
	local labels=("$@")
	[ ${#labels[@]} -gt 0 ] || labels=("${LABELS[@]}")
	for label in "${labels[@]}"; do
		local dir=$WORK/$label out=$GOLDEN/$label
		mkdir -p "$out" "$WORK"
		case $label in docs-*) python3 "$SITES_PY" patches --check >&2 ;; esac
		if [ "$label" != mini ]; then
			for pass in minified unminified; do
				local levels=L1,L2,L3 args=()
				if [ $pass = minified ]; then
					levels=L1,L4
					args=(--minify)
				fi
				build "$label" "$label" "$dir" "$BIN/neohugo" "${args[@]}"
				python3 "$MANIFEST_PY" extract "$dir/out" --project "$dir/$label" --site "$label" \
					--pass $pass --levels $levels -o "$WORK/$label.manifest.$pass.json"
				put "$WORK/$label.manifest.$pass.json" "$out/manifest.$pass.json"
			done
		fi
		NH_STRUCTURE_OUT=$WORK/$label.structure.json build "$label" "$label" "$dir" "$BIN/neohugo-structure"
		put "$WORK/$label.structure.json" "$out/structure.json"
		[ "${KEEP:-}" = 1 ] || rm -rf "$dir"
		log "$label: $(python3 "$MANIFEST_PY" summary "$out"/*.json*)"
	done
}

images() {
	need_bins
	local dir=$WORK/images out=$GOLDEN/images
	build images images "$dir" "$BIN/neohugo"
	python3 - "$dir/out/index.html" "$dir/out" "$out" <<'EOF'
import json, os, shutil, sys
index, pub, out = sys.argv[1:]
recipes = json.load(open(os.path.join(out, "manifest.json"), encoding="utf-8"))
want = {r["golden"] for r in recipes}
for f in os.listdir(out):
    if f != "manifest.json" and f not in want:
        os.remove(os.path.join(out, f))
seen = set()
for line in open(index, encoding="utf-8"):
    parts = line.split()
    if len(parts) != 2:
        continue
    name, rel = parts
    shutil.copyfile(os.path.join(pub, rel.lstrip("/")), os.path.join(out, name))
    seen.add(name)
missing = sorted(want - seen)
if missing:
    sys.exit(f"oracle.sh images: no output for {missing}")
print(f"oracle.sh: images: {len(seen)} golden images in {out}", file=sys.stderr)
EOF
	[ "${KEEP:-}" = 1 ] || rm -rf "$dir"
}

check() {
	local tmp
	tmp=$(mktemp -d "${TMPDIR:-/tmp}/neohugo-oracle-check.XXXXXX")
	mkdir -p "$tmp/golden/images"
	cp "$GOLDEN/images/manifest.json" "$tmp/golden/images/"
	NEOHUGO_GOLDEN=$tmp/golden NEOHUGO_ORACLE_WORK=$tmp/work "$0" sites
	NEOHUGO_GOLDEN=$tmp/golden NEOHUGO_ORACLE_WORK=$tmp/work "$0" images
	local status=0
	for label in "${LABELS[@]}" images; do
		if diff -r "$GOLDEN/$label" "$tmp/golden/$label" >/dev/null; then
			log "check: $label identical"
		else
			log "check: $label DIFFERS:"
			diff -rq "$GOLDEN/$label" "$tmp/golden/$label" >&2 || true
			status=1
		fi
	done
	rm -rf "$tmp"
	return $status
}

case "${1:-}" in
install) install ;;
sites) shift; sites "$@" ;;
images) images ;;
all)
	[ -x "$BIN/neohugo" ] && [ -x "$BIN/neohugo-structure" ] || install
	sites
	images
	;;
check) check ;;
bin) echo "$BIN" ;;
*)
	sed -n '2,/^set -euo/p' "$0" | sed -e '$d' -e 's/^# \{0,1\}//' >&2
	exit 2
	;;
esac
