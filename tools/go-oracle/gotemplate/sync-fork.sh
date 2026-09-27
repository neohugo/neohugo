#!/bin/sh
# Copies the forked Go template packages (tpl/internal/go_templates, which
# are internal to github.com/neohugo/neohugo/tpl and so cannot be imported
# from tools/) into ./fork with rewritten import paths, so the oracle can
# use the exact code neohugo executes. ./fork is gitignored; the oracle's
# own files carry the build tag `gotemplate_oracle` so that `go build ./...`
# and `go vet ./...` on a fresh checkout never see the missing package.
#
# Usage (from the repository root or this directory):
#   tools/go-oracle/gotemplate/sync-fork.sh
#   GOTOOLCHAIN=go1.27.1 go run -tags gotemplate_oracle ./tools/go-oracle/gotemplate <mode> ...
set -eu
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../../.." && pwd)
src="$root/tpl/internal/go_templates"
dst="$here/fork"
rm -rf "$dst"
for pkg in texttemplate texttemplate/parse htmltemplate fmtsort; do
	mkdir -p "$dst/$pkg"
	for f in "$src/$pkg"/*.go; do
		case "$f" in *_test.go) continue ;; esac
		{
			echo "//go:build gotemplate_oracle"
			echo
			sed -e 's#github.com/neohugo/neohugo/tpl/internal/go_templates/#github.com/neohugo/neohugo/tools/go-oracle/gotemplate/fork/#g' "$f"
		} > "$dst/$pkg/$(basename "$f")"
	done
done
# Export a few unexported helpers the oracle needs (escaper funcs, stripTags,
# isJSType ...) through an extra file in the copied package.
cat > "$dst/htmltemplate/oracle_exports.go" <<'GO'
//go:build gotemplate_oracle

package template

// EscFuncs exposes the escaper func map (funcMap) to the oracle.
func EscFuncs() map[string]any { return funcMap }

// StripTagsExported exposes stripTags.
func StripTagsExported(s string) string { return stripTags(s) }

// IsJSTypeExported exposes isJSType.
func IsJSTypeExported(s string) bool { return isJSType(s) }
GO
cat > "$dst/htmltemplate/oracle_exports_escdump.go" <<'GO'
//go:build gotemplate_oracle

package template

import texttemplate "github.com/neohugo/neohugo/tools/go-oracle/gotemplate/fork/texttemplate"

// TextTemplate exposes the underlying text/template of an html template.
func (t *Template) TextTemplate() *texttemplate.Template { return t.text }
GO
echo "fork copied to $dst"
