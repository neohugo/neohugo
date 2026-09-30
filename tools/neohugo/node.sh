#!/bin/sh
# Node tooling of the target sites (docs/rust-port/REWRITE_PLAN.md §7.2, D8): installs the node
# modules pinned by tools/neohugo/node/package-lock.json, which the Go and the Rust builds of
# the sites run or import:
#   - @tailwindcss/cli, tailwindcss, @tailwindcss/typography: css.TailwindCSS of docs-reduced;
#   - alpinejs, @alpinejs/{focus,persist}, @hotwired/turbo: js.Build imports of docs-reduced;
#   - postcss, postcss-cli: css.PostCSS of the seeksnack reconstruction.
#
#   tools/neohugo/node.sh [install]   npm ci into the node_modules directory (network)
#   tools/neohugo/node.sh check       exit 1 unless the installed modules match the lock file
#   tools/neohugo/node.sh path        print the node_modules directory
#
# The directory is $NEOHUGO_NODE_MODULES, else tools/neohugo/node_modules of the main checkout
# (all worktrees share it; gitignored). A build uses it as oracle.sh does: a `node_modules`
# symlink in the site directory (Hugo looks up `node_modules/.bin/<tool>` in the project and
# esbuild resolves imports there) and `node_modules/.bin` on PATH.
set -eu

here=$(cd "$(dirname "$0")" && pwd)

node_modules_dir() {
	if [ -n "${NEOHUGO_NODE_MODULES:-}" ]; then
		echo "$NEOHUGO_NODE_MODULES"
		return
	fi
	common=$(git -C "$here" rev-parse --path-format=absolute --git-common-dir)
	echo "$(dirname "$common")/tools/neohugo/node_modules"
}

lock="$here/node/package-lock.json"
target=$(node_modules_dir)
want=$(sha256sum "$lock" | cut -d' ' -f1)
stamp="$target/.neohugo-lock-sha256"

case "${1:-install}" in
path)
	echo "$target"
	;;
check)
	if [ -f "$stamp" ] && [ "$(cat "$stamp")" = "$want" ]; then
		echo "node.sh: $target matches $lock"
	else
		echo "node.sh: $target is missing or does not match $lock (run tools/neohugo/node.sh)" >&2
		exit 1
	fi
	;;
install)
	command -v npm >/dev/null || { echo "node.sh: npm not found" >&2; exit 1; }
	parent=$(dirname "$target")
	mkdir -p "$parent"
	# Staged next to the target (same file system), with its own npm cache, then moved.
	stage=$(mktemp -d "$parent/.node-stage.XXXXXX")
	trap 'rm -rf "$stage"' EXIT
	cp "$here/node/package.json" "$lock" "$stage/"
	(cd "$stage" && npm ci --no-audit --no-fund --update-notifier=false --loglevel=error \
		--cache "$stage/.npm-cache")
	echo "$want" >"$stage/node_modules/.neohugo-lock-sha256"
	rm -rf "$target"
	mv "$stage/node_modules" "$target"
	for bin in tailwindcss postcss; do
		[ -x "$target/.bin/$bin" ] || { echo "node.sh: $target/.bin/$bin missing" >&2; exit 1; }
	done
	echo "node.sh: installed into $target ($(du -sh "$target" | cut -f1)):"
	for pkg in @tailwindcss/cli tailwindcss @tailwindcss/typography alpinejs @alpinejs/focus \
		@alpinejs/persist @hotwired/turbo postcss postcss-cli; do
		echo "  $pkg $(node -p "require('$target/$pkg/package.json').version")"
	done
	;;
*)
	echo "usage: tools/neohugo/node.sh [install|check|path]" >&2
	exit 2
	;;
esac
