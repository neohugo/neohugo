#!/bin/sh
# Regenerates the crates/nh-images fixtures from the Go oracles, built for linux/arm64 and run
# under qemu-aarch64-static (the golden build is darwin/arm64: gift's resampling and the flate
# encoder fuse float operations on arm64, and the webp encoder is libwebp through cgo).
#
#   CC_ARM64=<zig cc wrapper> tools/go-oracle/nh-images/regen.sh [topic ...]
#
# CC_ARM64 must run `zig cc -target aarch64-linux-musl "$@" -UNDEBUG` (see
# docs/rust-port/HANDOFF.md §3). Topics: config process exif (default: all). Run from the
# repository root. Every fixture regenerates byte for byte.
set -eu

: "${CC_ARM64:?set CC_ARM64 to a zig cc wrapper for aarch64-linux-musl}"
export GOTOOLCHAIN=go1.27.1
BIN=${TMPDIR:-/tmp}/nh-images-oracle
mkdir -p "$BIN"

topics=${*:-config process exif}
for t in $topics; do
	GOARCH=arm64 CGO_ENABLED=1 CC="$CC_ARM64" \
		go build -ldflags '-linkmode external -extldflags -static' \
		-o "$BIN/$t" "./tools/go-oracle/nh-images/$t"
	TZ=UTC qemu-aarch64-static "$BIN/$t" -root . -out "crates/nh-images/tests/fixtures/$t"
done
