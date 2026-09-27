#!/bin/sh
# Regenerates the tdewolff-parse fixtures and corpus digests.
#
#   SCRATCH=<dir with golden/ and work/minify/> tools/go-oracle/tdewolff-parse/gen.sh
#
# Checked-in outputs: crates/tdewolff-parse/tests/fixtures/{*.txt,corpus/*.tsv,
# fuzz/*.fz,fnfuzz/*.txt}. The corpora themselves, a 30x fixture set and (with
# LARGE=1) multi-million-input fuzz sets stay under $SCRATCH/work/tdewolff-parse.
set -eu
: "${SCRATCH:?set SCRATCH}"
REPO=$(cd "$(dirname "$0")/../../.." && pwd)
W=$SCRATCH/work/tdewolff-parse
FIX=$REPO/crates/tdewolff-parse/tests/fixtures
mkdir -p "$W" "$FIX/corpus"
cd "$REPO"
go build -o "$W/oracle" ./tools/go-oracle/tdewolff-parse
O=$W/oracle
export NUMBERS_JSONL=$SCRATCH/work/minify/vectors/numbers.jsonl

"$O" fixtures "$FIX"
"$O" fixtures "$W/fixtures-full" 30

G=$SCRATCH/golden
P=$SCRATCH/canon/pristine-seeksnack
M=$SCRATCH/work/minify/corpus2
ROOTS="$G/nominify $P $M $W/embedded"

rm -rf "$W/embedded"
"$O" extract "$G/nominify" "$W/embedded"

# Checked-in randomized sets: stream digests (tests/fuzz.rs, inputs include
# mutated windows of the corpora above) and function records
# (tests/{root,strconv,misc}.rs *_fnfuzz).
rm -rf "$FIX/fuzz" "$FIX/fnfuzz"
# shellcheck disable=SC2086
"$O" fuzz "$FIX/fuzz" 1000 7 1 200 $ROOTS
"$O" fnfuzz "$FIX/fnfuzz" 100 5
rm "$FIX/fnfuzz/entities.txt"

# Large randomized sets outside the repository (run with
#   TDEWOLFF_PARSE_FUZZ=$W/fz cargo test --release --test fuzz -- --ignored
#   TDEWOLFF_PARSE_FNFUZZ=$W/fn cargo test --release -- fnfuzz).
if [ "${LARGE:-0}" = 1 ]; then
	rm -rf "$W/fz" "$W/fn"
	# shellcheck disable=SC2086
	"$O" fuzz "$W/fz" 300000 12 -1 3000 $ROOTS
	"$O" fnfuzz "$W/fn" 200000 33
	# every number of the corpora (TDEWOLFF_PARSE_FNFUZZ=$W/cnums ... strconv_fnfuzz)
	# shellcheck disable=SC2086
	"$O" corpusnums "$W/cnums" $ROOTS
fi

C=$FIX/corpus
"$O" corpus html "$G/nominify" "$C/golden-nominify.html.tsv" .html
"$O" corpus html "$G/canonical" "$C/golden-canonical.html.tsv" .html
"$O" corpus xml "$G/nominify" "$C/golden-nominify.xml.tsv" .xml
"$O" corpus xml "$G/canonical" "$C/golden-canonical.xml.tsv" .xml
"$O" corpus json "$G/nominify" "$C/golden-nominify.json.tsv" .json
"$O" corpus css "$M/css" "$C/corpus2-css.css.tsv" .in .out
"$O" corpus csslex "$M/css" "$C/corpus2-css.csslex.tsv" .in .out
"$O" corpus css "$M/css-resource" "$C/corpus2-css-resource.css.tsv" .in .out
"$O" corpus csslex "$M/css-resource" "$C/corpus2-css-resource.csslex.tsv" .in .out
"$O" corpus xml "$M/svg" "$C/corpus2-svg.xml.tsv" .in .out
"$O" corpus json "$M/json" "$C/corpus2-json.json.tsv" .in .out
"$O" corpus css "$W/embedded/css" "$C/embedded-css.css.tsv"
"$O" corpus csslex "$W/embedded/css" "$C/embedded-css.csslex.tsv"
"$O" corpus cssinline "$W/embedded/cssinline" "$C/embedded-cssinline.cssinline.tsv"
"$O" corpus xml "$W/embedded/svg" "$C/embedded-svg.xml.tsv"
"$O" corpus json "$W/embedded/json" "$C/embedded-json.json.tsv"

# Extra (adversarial) corpora from the pristine site sources (read only).
"$O" corpus css "$P" "$C/pristine.css.tsv" .css .scss
"$O" corpus csslex "$P" "$C/pristine.csslex.tsv" .css .scss
"$O" corpus cssinline "$P" "$C/pristine.cssinline.tsv" .css
"$O" corpus xml "$P" "$C/pristine.xml.tsv" .svg .xml
"$O" corpus json "$P" "$C/pristine.json.tsv" .json
"$O" corpus html "$P" "$C/pristine.html.tsv" .html
"$O" corpus htmltmpl "$P" "$C/pristine.htmltmpl.tsv" .html
