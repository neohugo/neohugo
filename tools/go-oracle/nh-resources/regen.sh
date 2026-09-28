#!/bin/sh
# Regenerates the crates/nh-resources fixtures from the Go oracles (run from the repository root).
#
#   CC_ARM64=<zig cc wrapper> tools/go-oracle/nh-resources/regen.sh [topic ...]
#
# Topics: resources keys transform images (default: all).
#
# - resources, keys: names, keys, paths, metadata and plain content only (no float code): built
#   and run natively (they build hugolib and the libsass client, which the zig cross-compiler
#   route does not need to handle).
# - transform, images: the minifier's number formatting, gift's resampling, the flate encoder
#   and libwebp fuse float operations on arm64 (the golden build is darwin/arm64), so these are
#   built for linux/arm64 with cgo (libwebp) and zig as the C cross-compiler and run under
#   qemu-aarch64-static (docs/rust-port/HANDOFF.md §3). CC_ARM64 must run
#   `zig cc -target aarch64-linux-musl "$@" -UNDEBUG`.
#
# The synthetic site (crates/nh-resources/tests/fixtures/site) is made by gensite.py.
# Every fixture regenerates byte for byte.
set -eu

export GOTOOLCHAIN=go1.27.1
BIN=${TMPDIR:-/tmp}/nh-resources-oracle
mkdir -p "$BIN"

topics=${*:-resources keys transform images}
for t in $topics; do
	out="crates/nh-resources/tests/fixtures/$t"
	case "$t" in
	resources | keys)
		go run "./tools/go-oracle/nh-resources/$t" -root . -out "$out"
		;;
	transform | images)
		: "${CC_ARM64:?set CC_ARM64 to a zig cc wrapper for aarch64-linux-musl}"
		GOARCH=arm64 CGO_ENABLED=1 CC="$CC_ARM64" \
			go build -ldflags '-linkmode external -extldflags -static' \
			-o "$BIN/$t" "./tools/go-oracle/nh-resources/$t"
		TZ=UTC qemu-aarch64-static "$BIN/$t" -root . -out "$out"
		rm -f "$BIN/$t"
		;;
	*)
		echo "unknown topic $t" >&2
		exit 1
		;;
	esac
done
