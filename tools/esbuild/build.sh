#!/bin/sh
# Builds the pinned esbuild CLI that the Rust port (crates/nh-esbuild) runs over `--service`.
#
#   tools/esbuild/build.sh            # -> tools/esbuild/bin/esbuild
#
# neohugo links esbuild as a Go library; the Rust port runs the same code as a binary instead
# (crates/nh-esbuild/PORTING.md, deviation 1). The binary is built from the neohugo Go module
# itself, so it is exactly the go.mod-pinned version (github.com/evanw/esbuild v0.25.6), offline
# from the module cache: `go build` of a package in a required module changes neither go.mod
# nor go.sum. The script checks both, and the version the binary prints.
#
# Tests and the acceptance harness then set
#   NEOHUGO_ESBUILD_BINARY=$REPO/tools/esbuild/bin/esbuild
set -eu

export GOTOOLCHAIN="${GOTOOLCHAIN:-go1.27.1}"
export GOPROXY="${GOPROXY:-off}"

root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$root"

want=$(go list -m -f '{{.Version}}' github.com/evanw/esbuild)
before=$(cat go.mod go.sum | cksum)

CGO_ENABLED=0 go build -trimpath -o tools/esbuild/bin/esbuild github.com/evanw/esbuild/cmd/esbuild

after=$(cat go.mod go.sum | cksum)
if [ "$before" != "$after" ]; then
	echo "build.sh: go.mod/go.sum changed" >&2
	exit 1
fi

got=$(tools/esbuild/bin/esbuild --version)
if [ "v$got" != "$want" ]; then
	echo "build.sh: tools/esbuild/bin/esbuild prints $got, go.mod pins $want" >&2
	exit 1
fi
echo "tools/esbuild/bin/esbuild $got"
