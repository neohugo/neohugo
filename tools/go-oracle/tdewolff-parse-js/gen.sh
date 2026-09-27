#!/bin/sh
# Regenerates the tdewolff-parse-js fixtures and corpus digests.
#
#   SCRATCH=<dir with golden/, canon/ and work/minify/> tools/go-oracle/tdewolff-parse-js/gen.sh
#   ARCH=arm64 SCRATCH=... tools/go-oracle/tdewolff-parse-js/gen.sh   # linux/amd64 host:
#       builds a linux/arm64 oracle (CGO_ENABLED=0) and runs it under
#       qemu-aarch64-static, as the golden toolchain was darwin/arm64
#       (parse/js has no float arithmetic, so amd64 gives the same bytes;
#       checked for every fixture)
#
# Checked-in outputs (crates/tdewolff-parse-js/tests/fixtures/):
#   literals.rec.gz   every string literal of the upstream parse/js + minify/js
#                     tests and hand-written edge cases, with full dumps
#   fuzz.rec.gz       30000 seeded mutations of those literals (digests)
#   tables.txt.gz     exhaustive token/rune/AsIdentifierName/VarsByUses tables
#   redteam.rec.gz    the inputs of the minify-js oracle's redteam.rec.gz
#                     (literal-heavy programs, logic trees, numbers, token
#                     soup, generated programs; digests), via `reparse`
#   enum-ws.txt.gz    every sequence of <= 3 whitespace/line-terminator/
#                     comment/BOM/hashbang tokens (combined digests)
#   limits.tsv        nesting limits and uint16 wrap-around (specs + digests)
#   fuzzcorpus.rec.gz 4000 mutated windows (<=1 KiB) of the JS corpus (digests)
#   corpus.tsv        per-file digests for the whole JS corpus
#   ../upstream_tables.rs  the upstream test tables (gen_upstream_tables.py)
# Larger sets for release-mode runs go to $SCRATCH/work/tdewolff-parse-js.
set -eu
: "${SCRATCH:?set SCRATCH}"
export GOTOOLCHAIN=go1.27.1
REPO=$(cd "$(dirname "$0")/../../.." && pwd)
W=$SCRATCH/work/tdewolff-parse-js
FIX=$REPO/crates/tdewolff-parse-js/tests/fixtures
mkdir -p "$W" "$FIX"
cd "$REPO"
if [ "${ARCH:-}" = arm64 ]; then
	CGO_ENABLED=0 GOARCH=arm64 go build -o "$W/oracle" ./tools/go-oracle/tdewolff-parse-js
	O="qemu-aarch64-static $W/oracle"
else
	go build -o "$W/oracle" ./tools/go-oracle/tdewolff-parse-js
	O=$W/oracle
fi

python3 tools/go-oracle/tdewolff-parse-js/gen_upstream_tables.py \
	"$(go env GOMODCACHE)/github.com/tdewolff/parse/v2@v2.8.1/js" \
	"$REPO/crates/tdewolff-parse-js/tests/upstream_tables.rs"
rustfmt --edition 2024 "$REPO/crates/tdewolff-parse-js/tests/upstream_tables.rs"

ROOTS="$SCRATCH/work/minify/corpus2 $SCRATCH/golden/canonical $SCRATCH/golden/nominify $SCRATCH/canon/pristine-seeksnack"

# shellcheck disable=SC2086
$O fixtures "$FIX" 30000 1
# shellcheck disable=SC2086
$O tables "$FIX/tables.txt.gz" 1
# red-team sets (run tools/go-oracle/tdewolff-minify-js/gen.sh first)
# shellcheck disable=SC2086
$O reparse "$REPO/crates/tdewolff-minify-js/tests/fixtures/redteam.rec.gz" "$FIX/redteam.rec.gz"
# shellcheck disable=SC2086
$O enumerate "$FIX/enum-ws.txt.gz" \
	0d,0d0a,0a,e280a8,e280a9,c2a0,efbbbf,20,09,0b,3c212d2d,2d2d3e,2f2a,2a2f,2f2f,61,31,2f,3d,28,29,27,60,7d,7b,24,5c,2321,2d2d,3c 3
# nesting limits and uint16 wrap-around (~20 min: 65536 distinct names are
# quadratic to declare)
# shellcheck disable=SC2086
$O limits "$FIX/limits.tsv"
# shellcheck disable=SC2086
$O fuzzcorpus "$FIX/fuzzcorpus.rec.gz" 4000 2 1024 $ROOTS
# shellcheck disable=SC2086
$O corpus "$FIX/corpus.tsv" $ROOTS

# Large sets (TDEWOLFF_PARSE_JS_FIXTURES=$W/full cargo test --release --test fixtures)
mkdir -p "$W/full"
# shellcheck disable=SC2086
$O fixtures "$W/full" 300000 7
# shellcheck disable=SC2086
$O fuzzcorpus "$W/full/fuzzcorpus.rec.gz" 100000 8 8192 $ROOTS
