#!/bin/sh
# Regenerates the tdewolff-parse-js fixtures and corpus digests.
#
#   SCRATCH=<dir with golden/, canon/ and work/minify/> tools/go-oracle/tdewolff-parse-js/gen.sh
#
# Checked-in outputs (crates/tdewolff-parse-js/tests/fixtures/):
#   literals.rec.gz   every string literal of the upstream parse/js + minify/js
#                     tests and hand-written edge cases, with full dumps
#   fuzz.rec.gz       30000 seeded mutations of those literals (digests)
#   fuzzcorpus.rec.gz 4000 mutated windows (<=1 KiB) of the JS corpus (digests)
#   corpus.tsv        per-file digests for the whole JS corpus
#   ../upstream_tables.rs  the upstream test tables (gen_upstream_tables.py)
# Larger sets for release-mode runs go to $SCRATCH/work/tdewolff-parse-js.
set -eu
: "${SCRATCH:?set SCRATCH}"
REPO=$(cd "$(dirname "$0")/../../.." && pwd)
W=$SCRATCH/work/tdewolff-parse-js
FIX=$REPO/crates/tdewolff-parse-js/tests/fixtures
mkdir -p "$W" "$FIX"
cd "$REPO"
go build -o "$W/oracle" ./tools/go-oracle/tdewolff-parse-js
O=$W/oracle

python3 tools/go-oracle/tdewolff-parse-js/gen_upstream_tables.py \
	"$(go env GOMODCACHE)/github.com/tdewolff/parse/v2@v2.8.1/js" \
	"$REPO/crates/tdewolff-parse-js/tests/upstream_tables.rs"
rustfmt --edition 2024 "$REPO/crates/tdewolff-parse-js/tests/upstream_tables.rs"

ROOTS="$SCRATCH/work/minify/corpus2 $SCRATCH/golden/canonical $SCRATCH/golden/nominify $SCRATCH/canon/pristine-seeksnack"

"$O" fixtures "$FIX" 30000 1
"$O" fuzzcorpus "$FIX/fuzzcorpus.rec.gz" 4000 2 1024 $ROOTS
# shellcheck disable=SC2086
"$O" corpus "$FIX/corpus.tsv" $ROOTS

# Large sets (TDEWOLFF_PARSE_JS_FIXTURES=$W/full cargo test --release --test fixtures)
mkdir -p "$W/full"
"$O" fixtures "$W/full" 300000 7
"$O" fuzzcorpus "$W/full/fuzzcorpus.rec.gz" 100000 8 8192 $ROOTS
