#!/bin/sh
# Regenerates the Wave B task T16 (js-css-pipeline) fixtures of crates/nh-resource-transformers
# (run from the repository root):
#
#   CC_ARM64=<zig cc wrapper> CXX_ARM64=<zig c++ wrapper> \
#     POSTCSS_BIN=<node_modules/.bin/postcss of postcss-cli 11.0.1> \
#     tools/go-oracle/nh-resource-transformers/t16regen.sh [topic ...]
#
# Topics: jsbuild tocss postcss (default: all).
#
# - jsbuild: esbuild (Go, linked in) has no platform-dependent output here; built and run natively.
# - tocss, postcss: LibSass formats numbers with C++ floating point, which arm64 compilers
#   contract into FMA (the golden build is darwin/arm64). Built for linux/arm64 with cgo (libsass,
#   libwebp) and zig as the C/C++ cross-compiler, run under qemu-aarch64-static
#   (docs/rust-port/HANDOFF.md §3). CC_ARM64 must run `zig cc -target aarch64-linux-musl "$@"
#   -UNDEBUG`, CXX_ARM64 the same with `zig c++`.
# - postcss additionally needs node and postcss-cli: POSTCSS_BIN is linked into the site copy as
#   node_modules/.bin/postcss (the oracle runs with the site copy as its working directory).
#
# The synthetic site (crates/nh-resource-transformers/tests/fixtures/t16site) is written by
# t16site.py. Every fixture regenerates byte for byte; nothing is written into the repository
# except the fixtures (the oracles work on temporary copies of the site).
set -eu

export GOTOOLCHAIN=go1.27.1
BIN=${TMPDIR:-/tmp}/nh-t16-oracle
mkdir -p "$BIN"

topics=${*:-jsbuild tocss postcss}
for t in $topics; do
	out="crates/nh-resource-transformers/tests/fixtures/$t"
	case "$t" in
	jsbuild)
		go run ./tools/go-oracle/nh-resource-transformers/jsbuild -root . -out "$out"
		;;
	tocss | postcss)
		: "${CC_ARM64:?set CC_ARM64 to a zig cc wrapper for aarch64-linux-musl}"
		: "${CXX_ARM64:?set CXX_ARM64 to a zig c++ wrapper for aarch64-linux-musl}"
		GOARCH=arm64 CGO_ENABLED=1 CC="$CC_ARM64" CXX="$CXX_ARM64" \
			go build -ldflags '-linkmode external -extldflags -static' \
			-o "$BIN/$t" "./tools/go-oracle/nh-resource-transformers/$t"
		if [ "$t" = postcss ]; then
			: "${POSTCSS_BIN:?set POSTCSS_BIN to node_modules/.bin/postcss of postcss-cli}"
			TZ=UTC qemu-aarch64-static "$BIN/$t" -root . -out "$out" -postcss "$POSTCSS_BIN"
		else
			TZ=UTC qemu-aarch64-static "$BIN/$t" -root . -out "$out"
		fi
		rm -f "$BIN/$t"
		;;
	*)
		echo "unknown topic $t" >&2
		exit 1
		;;
	esac
done
