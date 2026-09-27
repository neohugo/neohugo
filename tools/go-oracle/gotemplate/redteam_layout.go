//go:build gotemplate_oracle

// Red-team mode "rtlayout": the real layouts of this repository (as in
// escdump: docs/layouts, create/skeletons/theme/layouts,
// tpl/tplimpl/embedded/templates) with random HTML-significant mutations
// (quotes, tags, comments, script/style boundaries, URL and JS fragments
// inserted or deleted anywhere), parsed with stub functions, escaped,
// dumped and executed. Written in the htmlexec script format for
// crates/gotemplate/tests/html_exec.rs (html_redteam).
//
//	rtlayout <out.txt.gz> <seed> <n>
//
// Extra operation: O funcs V stubs NAMES   (NAMES comma-separated; every
// stub is func(...any) any returning nil).
package main

import (
	"bufio"
	"fmt"
	"math/rand"
	"os"
	"sort"
	"strings"

	htmltemplate "github.com/neohugo/neohugo/tools/go-oracle/gotemplate/fork/htmltemplate"
)

func init() {
	register("rtlayout", rtLayoutMain)
}

var rtLayoutDirs = []string{"docs/layouts", "create/skeletons/theme/layouts", "tpl/tplimpl/embedded/templates"}

var rtLayoutInserts = []string{
	`"`, `'`, `<`, `>`, `=`, `/`, `*`, "`", `{`, `}`, ` `, "\n", `<script>`, `</script>`, `<style>`, `</style>`,
	`<!--`, `-->`, ` href=`, ` onclick="`, ` style="`, `url(`, `javascript:`, `<a href="`, `<textarea>`, `</title>`,
	`<p title=`, `//`, `/*`, `*/`, `${`, `\`, `&quot;`, `<script type="application/ld+json">`, `?`, `#`, `&`,
}

func rtLayoutMain(args []string) error {
	out, seed, n, err := rtArgs("rtlayout", args)
	if err != nil {
		return err
	}
	scripts, err := rtLayoutScripts(rand.New(rand.NewSource(seed)), n)
	if err != nil {
		return err
	}
	fmt.Fprintf(os.Stderr, "rtlayout: seed %d, %d scripts\n", seed, n)
	return rtWriteGz(out, func(w *bufio.Writer) error { return rtWriteScripts(w, scripts) })
}

// rtLayoutScripts builds n scripts over mutated layouts (run from the
// repository root).
func rtLayoutScripts(r *rand.Rand, n int) ([]*script, error) {
	var files []layoutFile
	idents := map[string]bool{}
	for _, dir := range rtLayoutDirs {
		fs, err := readLayouts(dir)
		if err != nil {
			return nil, err
		}
		for _, f := range fs {
			collectIdents(f.name, f.content, idents)
			f.name = dir + "/" + f.name
			files = append(files, f)
		}
	}
	var names []string
	for name := range idents {
		names = append(names, name)
	}
	sort.Strings(names)
	stubs := strings.Join(names, ",")
	var scripts []*script
	for i := 0; i < n; i++ {
		f := files[r.Intn(len(files))]
		src := f.content
		for k := r.Intn(3); k >= 0; k-- {
			src = rtMutateLayout(r, src)
		}
		s := newScript(fmt.Sprintf("rtlayout/%d/%s", i, f.name)).
			op("new", "t", "c").
			op("funcs", "t", "stubs", stubs).
			op("parse", "t", src).
			op("prepare", "t").
			op("dump", "t")
		for k := r.Intn(2); k >= 0; k-- {
			s.op("execc", "t", corpusValues[r.Intn(len(corpusValues))])
		}
		scripts = append(scripts, s)
	}
	return scripts, nil
}

// rtMutateLayout inserts a token at, or deletes a few bytes from, a random
// position outside actions (so the template usually still parses).
func rtMutateLayout(r *rand.Rand, s string) string {
	if len(s) == 0 {
		return s
	}
	if r.Intn(5) == 0 {
		// Inside an action (parser and lexer errors on real templates).
		p := r.Intn(len(s))
		if open := strings.LastIndex(s[:p], "{{"); open >= 0 && !strings.Contains(s[open:p], "}}") {
			tok := []string{"|", "(", ")", ".", "$", ":=", "=", " ", "\n", "-", `"`, "`", "'", "end", "else",
				"$x", ".X", "nil", "1", "}}", "{{", "/*", "*/", ","}[r.Intn(24)]
			return s[:p] + tok + s[p:]
		}
	}
	for try := 0; try < 20; try++ {
		p := r.Intn(len(s))
		// Outside an action: the last "{{" before p is closed before p.
		if open := strings.LastIndex(s[:p], "{{"); open >= 0 && !strings.Contains(s[open:p], "}}") {
			continue
		}
		if r.Intn(4) == 0 {
			q := p + 1 + r.Intn(4)
			if q > len(s) || strings.Contains(s[p:q], "{") || strings.Contains(s[p:q], "}") {
				continue
			}
			return s[:p] + s[q:]
		}
		return s[:p] + rtLayoutInserts[r.Intn(len(rtLayoutInserts))] + s[p:]
	}
	return s
}

// rtStubFuncs is the "stubs" func set: every name returns nil.
func rtStubFuncs(names string) htmltemplate.FuncMap {
	fm := htmltemplate.FuncMap{}
	for _, n := range strings.Split(names, ",") {
		if n != "" {
			fm[n] = func(args ...any) any { return nil }
		}
	}
	return fm
}
