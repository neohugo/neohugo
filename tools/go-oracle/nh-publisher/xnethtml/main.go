// Command xnethtml is the Go oracle for the golang.org/x/net/html subset
// ported in crates/nh-publisher/src/xnethtml (Wave B task T07): html.Parse
// (tokenizer, every insertion mode, foreign content, doctype, entities).
//
//	go run ./tools/go-oracle/nh-publisher/xnethtml [-out crates/nh-publisher/tests/fixtures/xnethtml]
//	go run ./tools/go-oracle/nh-publisher/xnethtml -atoms crates/nh-publisher/src/xnethtml/atom_table.rs
//
// Inputs: every #data section of x/net/html's html5lib tree-construction test
// files (testdata/webkit/*.dat, parsed as documents), the adversarial element
// strings shared with the collector oracle (osupport.ElementStrings) and
// random tag soup. Each record holds the input and a dump of the parse tree
// (every node with its type, namespace, data, atom and attributes in order,
// strings quoted with strconv.QuoteToASCII), or the Go panic. Output:
// parse.jsonl.gz. Nothing here depends on the platform.
//
// -atoms regenerates the Rust atom table from x/net/html/atom's table.go.
package main

import (
	"bufio"
	"bytes"
	"flag"
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"log"
	"math/rand"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"time"
	"unicode"

	"golang.org/x/net/html"
	"golang.org/x/net/html/atom"

	"github.com/neohugo/neohugo/tools/go-oracle/nh-publisher/osupport"
)

func pkgDir(pkg string) (string, error) {
	out, err := exec.Command("go", "list", "-f", "{{.Dir}}", pkg).Output()
	if err != nil {
		return "", fmt.Errorf("go list %s: %w", pkg, err)
	}
	return strings.TrimSpace(string(out)), nil
}

// Dump writes the tree below n (n included).
func Dump(b *bytes.Buffer, n *html.Node, level int) {
	b.WriteString(strings.Repeat(" ", level))
	q := strconv.QuoteToASCII
	switch n.Type {
	case html.DocumentNode:
		b.WriteString("#document")
	case html.ElementNode:
		fmt.Fprintf(b, "E %s %s %#x", q(n.Namespace), q(n.Data), uint32(n.DataAtom))
	case html.TextNode:
		fmt.Fprintf(b, "T %s", q(n.Data))
	case html.CommentNode:
		fmt.Fprintf(b, "C %s", q(n.Data))
	case html.DoctypeNode:
		fmt.Fprintf(b, "D %s", q(n.Data))
	default:
		fmt.Fprintf(b, "? %d %s", n.Type, q(n.Data))
	}
	b.WriteByte('\n')
	for _, a := range n.Attr {
		b.WriteString(strings.Repeat(" ", level+1))
		fmt.Fprintf(b, "@ %s %s %s\n", q(a.Namespace), q(a.Key), q(a.Val))
	}
	for c := n.FirstChild; c != nil; c = c.NextSibling {
		Dump(b, c, level+1)
	}
}

type result struct {
	dump  string
	panic string
	err   string
}

func parse(in []byte) (res result, ok bool) {
	done := make(chan result, 1)
	go func() {
		var r result
		r.panic = osupport.PanicString(func() {
			n, err := html.Parse(bytes.NewReader(in))
			if err != nil {
				r.err = err.Error()
				return
			}
			var b bytes.Buffer
			Dump(&b, n, 0)
			r.dump = b.String()
		})
		done <- r
	}()
	select {
	case r := <-done:
		return r, true
	case <-time.After(10 * time.Second):
		return result{}, false
	}
}

// datInputs reads the #data sections of an html5lib .dat file.
func datInputs(fn string) ([]string, error) {
	f, err := os.Open(fn)
	if err != nil {
		return nil, err
	}
	defer func() { _ = f.Close() }()
	var out []string
	var cur []string
	in := false
	sc := bufio.NewScanner(f)
	sc.Buffer(make([]byte, 1<<20), 1<<24)
	for sc.Scan() {
		line := sc.Text()
		if strings.HasPrefix(line, "#") {
			if in {
				out = append(out, strings.Join(cur, "\n"))
			}
			in = line == "#data"
			cur = nil
			continue
		}
		if in {
			cur = append(cur, line)
		}
	}
	return out, sc.Err()
}

var soup = []string{
	"<", ">", "</", "/>", "<!--", "-->", "--!>", "<!", "<?", "?>", "<![CDATA[", "]]>", "<!DOCTYPE html>", "<!doctype x PUBLIC \"-//W3C//DTD HTML 4.01//EN\">",
	"<table>", "</table>", "<tr>", "<td>", "<th class=t>", "</td>", "<caption>", "<col>", "<colgroup>", "<tbody>", "<select>", "<option>",
	"<optgroup>", "<svg>", "</svg>", "<math>", "<mi>", "<foreignObject>", "<desc>", "<title>", "</title>", "<style>", "</style>",
	"<script>", "</script>", "<textarea>", "</textarea>", "<pre>", "\n", "<template>", "</template>", "<frameset>", "<frame>",
	"<noscript>", "<plaintext>", "<xmp>", "<iframe>", "<image src=x>", "<a href=1>", "</a>", "<b>", "</b>", "<i>", "<p>", "</p>",
	"<div class=\"c d\">", "</div>", "<li>", "<dd>", "<form>", "</form>", "<input type=hidden>", "<br>", "</br>", "<hr/>",
	"<html lang=x>", "<head>", "</head>", "<body id=b>", "</body>", "</html>", "&amp;", "&lt", "&#x41;", "&notit;", "&", "text", " ",
	"\x00", "\r\n", "\r", "<font color=red>", "<nobr>", "<ruby><rt>", "<annotation-xml encoding=text/html>", "<select><textarea>",
	"<x-el a='1' b=\"2\" c=3 d>", "<A HREF=Q>", "<svg viewbox=0><path d=M/>", "<math definitionurl=u>", "<svg><title>t</title>",
}

func main() {
	out := flag.String("out", "crates/nh-publisher/tests/fixtures/xnethtml", "output directory")
	atoms := flag.String("atoms", "", "write the Rust atom table to this file and exit")
	flag.Parse()

	if *atoms != "" {
		if err := writeAtoms(*atoms); err != nil {
			log.Fatal(err)
		}
		return
	}

	dir, err := pkgDir("golang.org/x/net/html")
	if err != nil {
		log.Fatal(err)
	}
	files, err := filepath.Glob(filepath.Join(dir, "testdata", "webkit", "*.dat"))
	if err != nil {
		log.Fatal(err)
	}
	more, err := filepath.Glob(filepath.Join(dir, "testdata", "webkit", "scripted", "*.dat"))
	if err != nil {
		log.Fatal(err)
	}
	files = append(files, more...)
	sort.Strings(files)

	var inputs []string
	for _, fn := range files {
		ins, err := datInputs(fn)
		if err != nil {
			log.Fatal(err)
		}
		inputs = append(inputs, ins...)
	}
	nDat := len(inputs)
	inputs = append(inputs, osupport.ElementStrings()...)
	rnd := rand.New(rand.NewSource(3))
	for i := 0; i < 15000; i++ {
		var b strings.Builder
		for j := 1 + rnd.Intn(25); j > 0; j-- {
			b.WriteString(soup[rnd.Intn(len(soup))])
		}
		inputs = append(inputs, b.String())
	}

	w, err := osupport.Create(filepath.Join(*out, "parse.jsonl.gz"))
	if err != nil {
		log.Fatal(err)
	}
	seen := map[string]bool{}
	hangs, panics := 0, 0
	for _, in := range inputs {
		if seen[in] {
			continue
		}
		seen[in] = true
		r, ok := parse([]byte(in))
		if !ok {
			hangs++
			continue
		}
		rec := map[string]any{"in": osupport.S(in)}
		switch {
		case r.panic != "":
			panics++
			rec["panic"] = r.panic
		case r.err != "":
			rec["err"] = r.err
		default:
			rec["dump"] = r.dump
		}
		if err := w.Write(rec); err != nil {
			log.Fatal(err)
		}
	}
	if err := w.Close(); err != nil {
		log.Fatal(err)
	}
	log.Printf("xnethtml: %d records (%d html5lib inputs from %d files), %d panics, %d hangs skipped", w.N(), nDat, len(files), panics, hangs)
}

// writeAtoms generates the Rust atom table (names, values, atomText) from
// x/net/html/atom's table.go, checked against atom.Lookup and Atom.String.
func writeAtoms(dst string) error {
	dir, err := pkgDir("golang.org/x/net/html/atom")
	if err != nil {
		return err
	}
	fset := token.NewFileSet()
	f, err := parser.ParseFile(fset, filepath.Join(dir, "table.go"), nil, 0)
	if err != nil {
		return err
	}
	type entry struct {
		goName string
		value  uint64
	}
	var entries []entry
	var atomText string
	for _, d := range f.Decls {
		gd, ok := d.(*ast.GenDecl)
		if !ok {
			continue
		}
		for _, s := range gd.Specs {
			vs, ok := s.(*ast.ValueSpec)
			if !ok || len(vs.Values) != 1 {
				continue
			}
			if gd.Tok != token.CONST {
				continue
			}
			if id, ok := vs.Type.(*ast.Ident); ok && id.Name == "Atom" {
				lit, ok := vs.Values[0].(*ast.BasicLit)
				if !ok {
					continue
				}
				v, err := strconv.ParseUint(lit.Value, 0, 32)
				if err != nil {
					return err
				}
				entries = append(entries, entry{vs.Names[0].Name, v})
				continue
			}
			if vs.Names[0].Name != "atomText" {
				continue
			}
			// atomText is a concatenation of string literals.
			var sb strings.Builder
			var walk func(e ast.Expr) error
			walk = func(e ast.Expr) error {
				switch x := e.(type) {
				case *ast.BinaryExpr:
					if err := walk(x.X); err != nil {
						return err
					}
					return walk(x.Y)
				case *ast.BasicLit:
					s, err := strconv.Unquote(x.Value)
					if err != nil {
						return err
					}
					sb.WriteString(s)
					return nil
				}
				return fmt.Errorf("atomText: unexpected %T", e)
			}
			if err := walk(vs.Values[0]); err != nil {
				return err
			}
			atomText = sb.String()
		}
	}
	if atomText == "" || len(entries) == 0 {
		return fmt.Errorf("table.go: no atoms")
	}
	var b bytes.Buffer
	b.WriteString("// Code generated by tools/go-oracle/nh-publisher/xnethtml -atoms from golang.org/x/net@v0.41.0/html/atom/table.go; DO NOT EDIT.\n\n")
	b.WriteString("//! The atoms of `golang.org/x/net/html/atom` (Go `Atom` values, names and `atomText`).\n\n")
	b.WriteString("use super::Atom;\n\n")
	type named struct {
		name string
		a    uint64
	}
	var byName []named
	for _, e := range entries {
		a := atom.Atom(e.value)
		name := a.String()
		if atom.Lookup([]byte(name)) != a {
			return fmt.Errorf("atom %s: Lookup(%q) != %#x", e.goName, name, e.value)
		}
		fmt.Fprintf(&b, "/// `%s`\npub const %s: Atom = %#x;\n", name, rustConst(e.goName), e.value)
		byName = append(byName, named{name, e.value})
	}
	sort.Slice(byName, func(i, j int) bool { return byName[i].name < byName[j].name })
	fmt.Fprintf(&b, "\n/// Go `atomText`.\npub(super) const ATOM_TEXT: &[u8] = b%s;\n", strconv.Quote(atomText))
	fmt.Fprintf(&b, "\n/// Every atom by name (sorted), for `Lookup`.\npub(super) static BY_NAME: [(&[u8], Atom); %d] = [\n", len(byName))
	for _, n := range byName {
		fmt.Fprintf(&b, "    (b%s, %#x),\n", strconv.Quote(n.name), n.a)
	}
	b.WriteString("];\n")
	return os.WriteFile(dst, b.Bytes(), 0o644)
}

// rustConst converts a Go atom name (AnnotationXml) to a Rust constant name
// (ANNOTATION_XML).
func rustConst(s string) string {
	var b strings.Builder
	for i, r := range s {
		if i > 0 && unicode.IsUpper(r) {
			b.WriteByte('_')
		}
		b.WriteRune(unicode.ToUpper(r))
	}
	return b.String()
}
