//go:build gotemplate_oracle

package main

import (
	"bytes"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
	"unicode"
	"unicode/utf8"

	htmltemplate "github.com/neohugo/neohugo/tools/go-oracle/gotemplate/fork/htmltemplate"
	"github.com/neohugo/neohugo/tools/go-oracle/gotemplate/fork/texttemplate/parse"
)

// escdump: parses a directory of layouts into html/template namespaces the
// way tpl/tplimpl does (one shared namespace for everything that does not
// need a base template; one CloneShallow namespace per overlay+baseof),
// escapes every file-level template in sorted order and dumps every tree of
// every namespace (including the derived name$htmltemplate_* templates)
// after escaping. crates/gotemplate/tests/html_escdump.rs replays the same
// procedure and compares.
//
//	escdump <outfile> <layoutsdir>...
func init() { register("escdump", runEscDump) }

func runEscDump(args []string) error {
	if len(args) < 2 {
		return fmt.Errorf("usage: escdump <outfile> <layoutsdir> [<layoutsdir>]")
	}
	var out bytes.Buffer
	for _, dir := range args[1:] {
		if err := escDumpDir(&out, dir); err != nil {
			return err
		}
	}
	return os.WriteFile(args[0], out.Bytes(), 0o644)
}

type layoutFile struct {
	name    string
	content string
}

func readLayouts(dir string) ([]layoutFile, error) {
	var files []layoutFile
	err := filepath.WalkDir(dir, func(p string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if d.IsDir() {
			return nil
		}
		ext := filepath.Ext(p)
		if ext != ".html" && ext != ".xml" {
			return nil
		}
		b, err := os.ReadFile(p)
		if err != nil {
			return err
		}
		rel, _ := filepath.Rel(dir, p)
		files = append(files, layoutFile{name: filepath.ToSlash(rel), content: removeLeadingBOM(string(b))})
		return nil
	})
	sort.Slice(files, func(i, j int) bool { return files[i].name < files[j].name })
	return files, err
}

// collectIdents parses in SkipFuncCheck mode and returns every identifier
// (function name) used, so the real parse can be given a func map.
func collectIdents(name, text string, into map[string]bool) {
	t := parse.New(name)
	t.Mode = parse.SkipFuncCheck
	trees := make(map[string]*parse.Tree)
	if _, err := t.Parse(text, "", "", trees); err != nil {
		return
	}
	var walk func(n parse.Node)
	walk = func(n parse.Node) {
		switch n := n.(type) {
		case nil:
		case *parse.ListNode:
			if n == nil {
				return
			}
			for _, c := range n.Nodes {
				walk(c)
			}
		case *parse.ActionNode:
			walk(n.Pipe)
		case *parse.PipeNode:
			if n == nil {
				return
			}
			for _, c := range n.Cmds {
				walk(c)
			}
		case *parse.CommandNode:
			for _, a := range n.Args {
				walk(a)
			}
		case *parse.IdentifierNode:
			into[n.Ident] = true
		case *parse.ChainNode:
			walk(n.Node)
		case *parse.IfNode:
			walk(n.Pipe)
			walk(n.List)
			walk(n.ElseList)
		case *parse.RangeNode:
			walk(n.Pipe)
			walk(n.List)
			walk(n.ElseList)
		case *parse.WithNode:
			walk(n.Pipe)
			walk(n.List)
			walk(n.ElseList)
		case *parse.TemplateNode:
			walk(n.Pipe)
		}
	}
	for _, tr := range trees {
		walk(tr.Root)
	}
}

// Go: tpl/tplimpl/templates.go:needsBaseTemplate
var baseTemplateDefineRe = regexp.MustCompile(`^{{-?\s*define`)

func needsBaseTemplate(templ string) bool {
	idx := -1
	inComment := false
	for i := 0; i < len(templ); {
		if !inComment && strings.HasPrefix(templ[i:], "{{/*") {
			inComment = true
			i += 4
		} else if !inComment && strings.HasPrefix(templ[i:], "{{- /*") {
			inComment = true
			i += 6
		} else if inComment && strings.HasPrefix(templ[i:], "*/}}") {
			inComment = false
			i += 4
		} else if inComment && strings.HasPrefix(templ[i:], "*/ -}}") {
			inComment = false
			i += 6
		} else {
			r, size := utf8.DecodeRuneInString(templ[i:])
			if !inComment {
				if strings.HasPrefix(templ[i:], "{{") {
					idx = i
					break
				} else if !unicode.IsSpace(r) {
					break
				}
			}
			i += size
		}
	}
	if idx == -1 {
		return false
	}
	return baseTemplateDefineRe.MatchString(templ[idx:])
}

// Go: tpl/tplimpl/templates.go:removeLeadingBOM
func removeLeadingBOM(s string) string {
	const bom = '\ufeff'
	for i, r := range s {
		if i == 0 && r != bom {
			return s
		}
		if i > 0 {
			return s[i:]
		}
	}
	return s
}

type namespace struct {
	label string
	tmpl  *htmltemplate.Template
}

func escDumpDir(out *bytes.Buffer, dir string) error {
	files, err := readLayouts(dir)
	if err != nil {
		return err
	}
	idents := map[string]bool{}
	for _, f := range files {
		collectIdents(f.name, f.content, idents)
	}
	funcs := htmltemplate.FuncMap{}
	for name := range idents {
		funcs[name] = func(args ...any) any { return nil }
	}

	var base *layoutFile
	for i := range files {
		if files[i].name == "baseof.html" || files[i].name == "_default/baseof.html" {
			base = &files[i]
		}
	}

	fmt.Fprintf(out, "=== DIR %s\n", dir)
	shared := htmltemplate.New("").Funcs(funcs)
	var sharedNames []string
	var overlays []layoutFile
	for _, f := range files {
		if base != nil && f.name == base.name {
			continue
		}
		if base != nil && needsBaseTemplate(f.content) {
			overlays = append(overlays, f)
			continue
		}
		if _, err := shared.New(f.name).Parse(f.content); err != nil {
			fmt.Fprintf(out, "--- PARSE %s: ERR %s\n", f.name, err)
			continue
		}
		sharedNames = append(sharedNames, f.name)
	}
	namespaces := []namespace{{"shared", shared}}
	var overlayNames []string
	for _, f := range overlays {
		tt := htmltemplate.Must(shared.CloneShallow()).New(f.name)
		if _, err := tt.Parse(base.content); err != nil {
			fmt.Fprintf(out, "--- PARSE base %s: ERR %s\n", f.name, err)
			continue
		}
		if _, err := tt.Parse(f.content); err != nil {
			fmt.Fprintf(out, "--- PARSE %s: ERR %s\n", f.name, err)
			continue
		}
		namespaces = append(namespaces, namespace{"base:" + f.name, tt})
		overlayNames = append(overlayNames, f.name)
	}

	// Escape (Prepare) every file-level template, in sorted order.
	for _, name := range sharedNames {
		t := shared.Lookup(name)
		if _, err := t.Prepare(); err != nil {
			fmt.Fprintf(out, "--- ESCAPE %s: ERR %s\n", name, err)
		} else {
			fmt.Fprintf(out, "--- ESCAPE %s: OK\n", name)
		}
	}
	for i, ns := range namespaces[1:] {
		if _, err := ns.tmpl.Prepare(); err != nil {
			fmt.Fprintf(out, "--- ESCAPE %s: ERR %s\n", overlayNames[i], err)
		} else {
			fmt.Fprintf(out, "--- ESCAPE %s: OK\n", overlayNames[i])
		}
	}

	// Dump every tree of every namespace.
	for _, ns := range namespaces {
		fmt.Fprintf(out, "=== NAMESPACE %s\n", ns.label)
		text := ns.tmpl.TextTemplate()
		ts := text.Templates()
		sort.Slice(ts, func(i, j int) bool { return ts[i].Name() < ts[j].Name() })
		for _, t := range ts {
			s := "<nil>"
			if t.Tree != nil && t.Root != nil {
				s = t.Root.String()
			}
			fmt.Fprintf(out, "### TEMPLATE %q %d\n%s\n", t.Name(), len(s), s)
		}
	}
	return nil
}
