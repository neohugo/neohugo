#!/bin/sh
# Regenerates the rust/testdata/oracle/images fixtures from the Go oracles (run from the
# repository root), natively, then converts them to plain JSON (tools/neohugo/fixtures2json.py).
#
#   tools/go-oracle/nh-images/regen.sh [topic ...]
#
# Topics: config process exif (default: all). Output bytes are no longer compared with Go
# (docs/rust-port/REWRITE_PLAN.md §1.1), so the old linux/arm64 + qemu build is not needed.
set -eu

export GOTOOLCHAIN=go1.27.1

topics=${*:-config process exif}
for t in $topics; do
	out="rust/testdata/oracle/images/$t"
	TZ=UTC go run "./tools/go-oracle/nh-images/$t" -root . -out "$out"
	python3 tools/neohugo/fixtures2json.py convert "$out" "$out" >/dev/null
done
