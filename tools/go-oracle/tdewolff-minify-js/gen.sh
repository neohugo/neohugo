#!/bin/sh
# Regenerates the tdewolff-minify-js fixtures, corpus digests and upstream
# test tables. Needs go1.27.1 (GOTOOLCHAIN=go1.27.1) and the module cache
# (go mod download github.com/tdewolff/minify/v2 github.com/evanw/esbuild
# golang.org/x/tools@v0.34.0).
#
#   SCRATCH=<scratch dir> tools/go-oracle/tdewolff-minify-js/gen.sh
#   ARCH=arm64 SCRATCH=<scratch dir> tools/go-oracle/tdewolff-minify-js/gen.sh
#       (linux/amd64 host: builds a linux/arm64 oracle with CGO_ENABLED=0 and
#       runs it under qemu-aarch64-static, as the golden toolchain was
#       darwin/arm64; the checked-in fixtures were generated this way)
#
# Checked-in outputs (crates/tdewolff-minify-js/tests/):
#   upstream_tables/mod.rs  js_test.go/util_test.go/html_test.go tables (the
#                           oracle checks every row passes in Go first)
#   fixtures/*.rec.gz       literals, embedded, html, adversarial, grammar,
#                           fuzz, repo, corpuswin, redteam (scale 1, seed 1)
#   fixtures/enum-min.txt.gz every sequence of <= 3 statement/expression
#                           tokens through v2022 and keep-inline (combined
#                           digests)
#   fixtures/corpus.tsv     per-file digests of the module-cache JS corpus
# A 20x set goes to $SCRATCH/work/tdewolff-minify-js/full20
# (TDEWOLFF_MINIFY_JS_FIXTURES=... cargo test --release --test fixtures).
#
# No float arithmetic reaches the output of minify/js (checked with
# `go tool objdump` of darwin/arm64 and linux/arm64 builds: no
# FMADD/FMSUB/FNMADD/FNMSUB in minify/v2/js, minify.Number, parse/v2/js;
# the float-register instructions there are FMOVD/FMOVQ struct copies), so
# amd64 and arm64 oracles give identical fixtures (checked byte for byte).
# The `html` fixture runs neohugo's whole M, whose CSS minifier is
# FMA-sensitive (css.HSL2RGB, strconv.ParseFloat): generate it on arm64.
set -eu
: "${SCRATCH:?set SCRATCH}"
export GOTOOLCHAIN=go1.27.1
REPO=$(cd "$(dirname "$0")/../../.." && pwd)
W=$SCRATCH/work/tdewolff-minify-js
CRATE=$REPO/crates/tdewolff-minify-js
FIX=$CRATE/tests/fixtures
mkdir -p "$W" "$FIX" "$CRATE/tests/upstream_tables"
cd "$REPO" # the oracle runs `go list -m` / `go env` inside the neohugo module
if [ "${ARCH:-}" = arm64 ]; then
	CGO_ENABLED=0 GOARCH=arm64 go build -o "$W/oracle" ./tools/go-oracle/tdewolff-minify-js
	O="qemu-aarch64-static $W/oracle"
else
	go build -o "$W/oracle" ./tools/go-oracle/tdewolff-minify-js
	O=$W/oracle
fi

# shellcheck disable=SC2086
$O tables "$CRATE/tests/upstream_tables/mod.rs"
rustfmt --edition 2024 "$CRATE/tests/upstream_tables/mod.rs"

# shellcheck disable=SC2086
$O fixtures "$FIX" 1 1
# shellcheck disable=SC2086
$O enumerate "$FIX/enum-min.txt.gz" v2022,keep-inline \
	7661722061,6c65742062,613d31,62,6966286129,656c736520,72657475726e20,72657475726e2061,7468726f772061,7b,7d,3b,0a,666f72283b3b29,7768696c65286129,66756e6374696f6e2066286129,2161,613f623a63,61262662,617c7c62,766f69642030,613d3d6e756c6c,28,29,7472797b7d6361746368286529,627265616b,636f6e74696e7565,783a,2c 3
# shellcheck disable=SC2086
$O corpus "$FIX/corpus.tsv" v2022,v2022-inline,0,keep-alpha,p3-v2022,alpha-v2019 \
	github.com/tdewolff/minify/v2@v2.23.8 github.com/evanw/esbuild@v0.25.6 golang.org/x/tools@v0.34.0 repo

# shellcheck disable=SC2086
$O fixtures "$W/full20" 20 7

# Red-team runs (out of the repository; checked with the Rust examples):
#   $O gen KIND OUT.rec.gz N SEED     # lits|logic|num|soup|prog|corpus|html
#   $O files OUT.rec.gz CFGS LIST     # whole files (e.g. a node_modules tree)
#   $O enumerate OUT.gz CFGS ALPHA N  # all short symbol sequences
#   cargo run --release --example check -- /abs/OUT.rec.gz DIFFDIR
#   cargo run --release --example enumerate -- OUT.gz
