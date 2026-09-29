#!/bin/sh
# Builds the Go neohugo for linux/arm64 (static, cgo via zig), the reference compare.sh runs under
# qemu-aarch64-static on amd64 hosts: the Rust port reproduces the arm64 Go build (FMA sites in
# flate, gift, strconv; docs/rust-port/HANDOFF.md §3).
#
#   ZIG=<path to zig> tools/rust-port/i01/build-go-arm64.sh <output binary>
#
# -UNDEBUG keeps LibSass's assertions like cgo's clang (zig cc -O2 defines NDEBUG).
set -eu

out=${1:?output binary}
zig=${ZIG:?set ZIG to the zig binary}
root=$(cd "$(dirname "$0")/../../.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

printf '#!/bin/sh\nexec "%s" cc -target aarch64-linux-musl -UNDEBUG "$@"\n' "$zig" >"$tmp/cc"
printf '#!/bin/sh\nexec "%s" c++ -target aarch64-linux-musl -UNDEBUG "$@"\n' "$zig" >"$tmp/cxx"
chmod +x "$tmp/cc" "$tmp/cxx"

cd "$root"
GOTOOLCHAIN=${GOTOOLCHAIN:-go1.27.1} CGO_ENABLED=1 GOOS=linux GOARCH=arm64 CC="$tmp/cc" CXX="$tmp/cxx" \
	go build -ldflags '-linkmode external -extldflags -static' -o "$out" .
