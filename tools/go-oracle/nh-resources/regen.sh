#!/bin/sh
# Regenerates the rust/testdata/oracle/resources fixtures from the Go oracles (run from the
# repository root), natively, then converts them to plain JSON (tools/neohugo/fixtures2json.py).
#
#   tools/go-oracle/nh-resources/regen.sh [topic ...]
#
# Topics: resources keys transform images (default: all). Output bytes are no longer compared
# with Go (docs/rust-port/REWRITE_PLAN.md §1.1), so the old linux/arm64 + qemu build is not
# needed.
#
# The synthetic site (rust/testdata/oracle/resources/site) is made by gensite.py.
set -eu

export GOTOOLCHAIN=go1.27.1

topics=${*:-resources keys transform images}
for t in $topics; do
	case "$t" in
	resources | keys | transform | images) ;;
	*)
		echo "unknown topic $t" >&2
		exit 1
		;;
	esac
	out="rust/testdata/oracle/resources/$t"
	TZ=UTC go run "./tools/go-oracle/nh-resources/$t" -root . -out "$out"
	python3 tools/neohugo/fixtures2json.py convert "$out" "$out" >/dev/null
done
