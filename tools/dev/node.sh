#!/bin/sh
# Node tooling of the target sites (docs/rust-port/REWRITE_PLAN.md §7.2, D8): installs the node
# modules pinned by tools/dev/node/package-lock.json, which the builds of the sites and the
# tests run or import:
#   - @tailwindcss/cli, tailwindcss, @tailwindcss/typography: css.TailwindCSS of docs-reduced;
#   - alpinejs, @alpinejs/{focus,persist}, @hotwired/turbo: js.Build imports of docs-reduced;
#   - postcss, postcss-cli: css.PostCSS of ssg-resources' CSS chain tests;
#   - @babel/cli, @babel/core: the real-tool Babel test of ssg-resources (js.Babel).
# CI (.github/workflows/ci.yml) runs this script and points FUGO_{POSTCSS,TAILWINDCSS,BABEL}_BIN
# and FUGO_NODE_MODULES into the result.
#
#   tools/dev/node.sh [install]   npm ci into the node_modules directory (network)
#   tools/dev/node.sh check       exit 1 unless the installed modules match the lock file
#   tools/dev/node.sh path        print the node_modules directory
#
# The directory is $FUGO_NODE_MODULES, else tools/dev/node_modules of the main checkout
# (all worktrees share it; gitignored). A build uses it through a `node_modules` symlink in the
# site directory (Hugo looks up `node_modules/.bin/<tool>` in the project and js.Build resolves
# imports there) and `node_modules/.bin` on PATH.
set -eu

here=$(cd "$(dirname "$0")" && pwd)

node_modules_dir() {
	if [ -n "${FUGO_NODE_MODULES:-}" ]; then
		echo "$FUGO_NODE_MODULES"
		return
	fi
	common=$(git -C "$here" rev-parse --path-format=absolute --git-common-dir)
	echo "$(dirname "$common")/tools/dev/node_modules"
}

lock="$here/node/package-lock.json"
target=$(node_modules_dir)
want=$(sha256sum "$lock" | cut -d' ' -f1)
stamp="$target/.lock-sha256"

case "${1:-install}" in
path)
	echo "$target"
	;;
check)
	if [ -f "$stamp" ] && [ "$(cat "$stamp")" = "$want" ]; then
		echo "node.sh: $target matches $lock"
	else
		echo "node.sh: $target is missing or does not match $lock (run tools/dev/node.sh)" >&2
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
	echo "$want" >"$stage/node_modules/.lock-sha256"
	rm -rf "$target"
	mv "$stage/node_modules" "$target"
	for bin in tailwindcss postcss babel; do
		[ -x "$target/.bin/$bin" ] || { echo "node.sh: $target/.bin/$bin missing" >&2; exit 1; }
	done
	echo "node.sh: installed into $target ($(du -sh "$target" | cut -f1)):"
	for pkg in @tailwindcss/cli tailwindcss @tailwindcss/typography alpinejs @alpinejs/focus \
		@alpinejs/persist @hotwired/turbo postcss postcss-cli @babel/cli @babel/core; do
		echo "  $pkg $(node -p "require('$target/$pkg/package.json').version")"
	done
	;;
*)
	echo "usage: tools/dev/node.sh [install|check|path]" >&2
	exit 2
	;;
esac
