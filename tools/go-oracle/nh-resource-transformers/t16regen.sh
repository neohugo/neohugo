#!/bin/sh
# Regenerates the Wave B task T16 (js-css-pipeline) fixtures under
# rust/testdata/oracle/resource-transformers (run from the repository root), natively, then
# converts them to plain JSON (tools/neohugo/fixtures2json.py):
#
#   POSTCSS_BIN=<node_modules/.bin/postcss of postcss-cli 11.0.1> \
#     tools/go-oracle/nh-resource-transformers/t16regen.sh [topic ...]
#
# Topics: jsbuild tocss postcss (default: all). Output bytes are no longer compared with Go
# (docs/rust-port/REWRITE_PLAN.md §1.1), so the old linux/arm64 + qemu build is not needed.
# tocss and postcss build LibSass through cgo (a C/C++ compiler must be available).
#
# postcss additionally needs node and postcss-cli: POSTCSS_BIN is linked into the site copy as
# node_modules/.bin/postcss (the oracle runs with the site copy as its working directory).
#
# The synthetic site (rust/testdata/oracle/resource-transformers/t16site) is written by
# t16site.py. Nothing is written into the repository except the fixtures (the oracles work on
# temporary copies of the site).
set -eu

export GOTOOLCHAIN=go1.27.1

topics=${*:-jsbuild tocss postcss}
for t in $topics; do
	out="rust/testdata/oracle/resource-transformers/$t"
	case "$t" in
	jsbuild | tocss)
		TZ=UTC go run "./tools/go-oracle/nh-resource-transformers/$t" -root . -out "$out"
		;;
	postcss)
		: "${POSTCSS_BIN:?set POSTCSS_BIN to node_modules/.bin/postcss of postcss-cli}"
		TZ=UTC go run ./tools/go-oracle/nh-resource-transformers/postcss -root . -out "$out" \
			-postcss "$POSTCSS_BIN"
		;;
	*)
		echo "unknown topic $t" >&2
		exit 1
		;;
	esac
	python3 tools/neohugo/fixtures2json.py convert "$out" "$out" >/dev/null
done
