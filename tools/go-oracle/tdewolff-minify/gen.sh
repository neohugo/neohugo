#!/bin/sh
# Regenerates the tdewolff-minify tables, fixtures and corpus digests.
#
#   SCRATCH=<dir with golden/, canon/ and work/minify/> tools/go-oracle/tdewolff-minify/gen.sh
#
# Checked-in outputs: crates/tdewolff-minify/src/{html,css,svg}/{hash,table}.rs,
# src/html/entities.rs, crates/tdewolff-minify/tests/fixtures/*.txt.gz and
# tests/fixtures/corpus/*.tsv. A 30x fixture set goes to
# $SCRATCH/work/tdewolff-minify/fixtures-full
# (TDEWOLFF_MINIFY_FIXTURES=... cargo test --release --test fixtures).
#
# The golden toolchain is darwin/arm64 and CSS colors are FMA-sensitive
# (css/util.go:39, parse HSL2RGB), so the oracle always runs as an arm64
# binary: natively on arm64 hosts, elsewhere as linux/arm64 under
# qemu-aarch64-static (apt install qemu-user-static), which reproduces the
# darwin/arm64 fixtures byte for byte.
set -eu
: "${SCRATCH:?set SCRATCH}"
REPO=$(cd "$(dirname "$0")/../../.." && pwd)
W=$SCRATCH/work/tdewolff-minify
CRATE=$REPO/crates/tdewolff-minify
FIX=$CRATE/tests/fixtures
mkdir -p "$W" "$FIX/corpus"
cd "$REPO" # the oracle runs `go list -m` inside the neohugo module
case $(uname -m) in
arm64 | aarch64)
	go build -o "$W/oracle" ./tools/go-oracle/tdewolff-minify
	O=$W/oracle
	;;
*)
	GOOS=linux GOARCH=arm64 CGO_ENABLED=0 go build -o "$W/oracle-arm64" ./tools/go-oracle/tdewolff-minify
	O="${QEMU:-qemu-aarch64-static} $W/oracle-arm64"
	;;
esac

$O tables "$CRATE/src"
(cd "$CRATE" && cargo fmt)

export SEEKSNACK_GOLDEN=$SCRATCH/golden/nominify
export CORPUS2=$SCRATCH/work/minify/corpus2
export NUMBERS_JSONL=$SCRATCH/work/minify/vectors/numbers.jsonl
$O fixtures "$FIX"
$O fixtures "$W/fixtures-full" 30
# red-team regressions: redteam.txt.gz (depth 100000), redteam-nested.txt.gz (depth 3333)
$O rt regress 100000 0 "$FIX"

C=$FIX/corpus
$O digests "$SCRATCH/golden/nominify" "$C/golden-nominify.tsv" .html .xml .json
$O digests "$SCRATCH/canon/pristine-seeksnack" "$C/pristine.tsv" .html .xml .json .css .scss .svg
for k in css json svg css-resource; do
	$O nested "$CORPUS2" "$k" "$C/corpus2-$k.tsv"
done
