#!/bin/sh
# Regenerates the tdewolff-minify-js fixtures, corpus digests and upstream
# test tables. Needs go1.27.1 (GOTOOLCHAIN=go1.27.1) and the module cache
# (go mod download github.com/tdewolff/minify/v2 github.com/evanw/esbuild
# golang.org/x/tools@v0.34.0).
#
#   SCRATCH=<scratch dir> tools/go-oracle/tdewolff-minify-js/gen.sh
#
# Checked-in outputs (crates/tdewolff-minify-js/tests/):
#   upstream_tables/mod.rs  js_test.go/util_test.go/html_test.go tables (the
#                           oracle checks every row passes in Go first)
#   fixtures/*.rec.gz       literals, embedded, html, adversarial, grammar,
#                           fuzz, repo, corpuswin (scale 1, seed 1)
#   fixtures/corpus.tsv     per-file digests of the module-cache JS corpus
# A 20x set goes to $SCRATCH/work/tdewolff-minify-js/full20
# (TDEWOLFF_MINIFY_JS_FIXTURES=... cargo test --release --test fixtures).
#
# No float arithmetic reaches the output of minify/js (checked with
# `go tool objdump` of a darwin/arm64 build: no FMADD/FMSUB/FNMADD/FNMSUB,
# no float instructions at all in minify/v2/js, minify.Number, parse/v2/js),
# so fixtures generated on any platform are valid.
set -eu
: "${SCRATCH:?set SCRATCH}"
export GOTOOLCHAIN=go1.27.1
REPO=$(cd "$(dirname "$0")/../../.." && pwd)
W=$SCRATCH/work/tdewolff-minify-js
CRATE=$REPO/crates/tdewolff-minify-js
FIX=$CRATE/tests/fixtures
mkdir -p "$W" "$FIX" "$CRATE/tests/upstream_tables"
cd "$REPO" # the oracle runs `go list -m` / `go env` inside the neohugo module
go build -o "$W/oracle" ./tools/go-oracle/tdewolff-minify-js
O=$W/oracle

"$O" tables "$CRATE/tests/upstream_tables/mod.rs"
rustfmt --edition 2024 "$CRATE/tests/upstream_tables/mod.rs"

"$O" fixtures "$FIX" 1 1
"$O" corpus "$FIX/corpus.tsv" v2022,v2022-inline,0,keep-alpha,p3-v2022,alpha-v2019 \
	github.com/tdewolff/minify/v2@v2.23.8 github.com/evanw/esbuild@v0.25.6 golang.org/x/tools@v0.34.0 repo

"$O" fixtures "$W/full20" 20 7
