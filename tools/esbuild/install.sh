#!/bin/sh
# Installs the esbuild CLI that the Rust rewrite (rust/crates/esbuild) runs over `--service`: the
# platform binary of the esbuild package that tools/neohugo/node/package.json pins, copied from
# the node modules of tools/neohugo/node.sh. Run by hand and by CI (.github/workflows/rust.yml).
#
#   tools/neohugo/node.sh && tools/esbuild/install.sh   # -> tools/esbuild/bin/esbuild
#
# The copy must print the pinned version. On Windows it is bin/esbuild.exe, copied to bin/esbuild
# as well: the tests look for that path, and Rust's Command runs the .exe beside it. Until
# 44529028 the binary was built with Go from go.mod's pin (tools/esbuild/build.sh); the npm build
# of that version gives byte-identical output.
#
# Tests and the acceptance harness then set
#   NEOHUGO_ESBUILD_BINARY=$REPO/tools/esbuild/bin/esbuild
set -eu

here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../.." && pwd)
pins=$repo/tools/neohugo/node/package.json

want=$(sed -n 's/^ *"esbuild": *"\([^"]*\)".*/\1/p' "$pins")
if [ -z "$want" ]; then
	echo "install.sh: tools/neohugo/node/package.json pins no esbuild" >&2
	exit 1
fi

case $(uname -s) in
Linux) os=linux ;;
Darwin) os=darwin ;;
MINGW* | MSYS* | CYGWIN*) os=win32 ;;
*) os=$(uname -s) ;;
esac
case $(uname -m) in
x86_64 | amd64) arch=x64 ;;
aarch64 | arm64) arch=arm64 ;;
*) arch=$(uname -m) ;;
esac

modules=$("$repo/tools/neohugo/node.sh" path)
case $os-$arch in
linux-x64 | linux-arm64 | darwin-x64 | darwin-arm64)
	src=$modules/@esbuild/$os-$arch/bin/esbuild exe= ;;
win32-x64)
	src=$modules/@esbuild/win32-x64/esbuild.exe exe=.exe ;;
*)
	echo "install.sh: no esbuild package for $(uname -s) $(uname -m)" >&2
	exit 1
	;;
esac
if [ ! -d "$modules" ]; then
	echo "install.sh: no node modules at $modules: run tools/neohugo/node.sh first" >&2
	exit 1
fi
if [ ! -f "$src" ]; then
	echo "install.sh: no $src: run tools/neohugo/node.sh first" >&2
	exit 1
fi

bin=$here/bin
mkdir -p "$bin"
rm -f "$bin/esbuild" "$bin/esbuild.exe"
cp "$src" "$bin/esbuild$exe"
chmod +x "$bin/esbuild$exe"
got=$("$bin/esbuild$exe" --version) || got=
if [ "$got" != "$want" ]; then
	rm -f "$bin/esbuild$exe"
	echo "install.sh: $src prints '$got', tools/neohugo/node/package.json pins $want" \
		"(run tools/neohugo/node.sh)" >&2
	exit 1
fi
if [ -n "$exe" ]; then
	cp "$bin/esbuild.exe" "$bin/esbuild"
fi
echo "tools/esbuild/bin/esbuild$exe $got (from $src)"
