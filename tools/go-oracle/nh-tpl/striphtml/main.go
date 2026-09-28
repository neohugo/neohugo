// Command striphtml is the Go oracle for tpl.StripHTML (tpl/template.go) in
// crates/nh-tpl (Wave B task T13).
//
//	GOTOOLCHAIN=go1.27.1 go run ./tools/go-oracle/nh-tpl/striphtml [-root .] [-out crates/nh-tpl/tests/fixtures/striphtml]
//
// Inputs: hand-written edge cases (the replacer's </p>, <br>, <br />
// placeholders and their overlaps, tags, comments, attributes with '>' in
// quotes, script/style/textarea/title contents, entities, every Unicode space
// kind, invalid UTF-8), every 1- and 2-byte string over an alphabet of
// HTML-significant bytes, seeded random HTML-ish strings, and slices of this
// repository's docs/ content (markdown with inline HTML). Input and output are
// hex-encoded (Go strings are bytes). Output: striphtml.json.gz.
package main

import (
	"bytes"
	"compress/gzip"
	"encoding/hex"
	"encoding/json"
	"flag"
	"io/fs"
	"log"
	"math/rand"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"github.com/neohugo/neohugo/tpl"
)

var edge = []string{
	"", "plain text", "a < b", "a > b", "<", ">", "<>", "</>", "<p>para</p>", "<p>a</p><p>b</p>",
	"line1<br>line2", "line1<br />line2", "line1<br/>line2", "<br><br />", "</p></p>", "a\nb", "a\n\nb",
	"<b>bold</b> and <i>it</i>", "<a href=\"x>y\">link</a>", "<a title='a>b'>t</a>", "<!-- c -->x", "<!-- <b> -->y",
	"<script>var a = 1 < 2;</script>z", "<style>p{}</style>s", "<textarea><b>x</b></textarea>", "<title>t</title>",
	"&amp; &lt; &#39;", "<p>  spaced   out  </p>", "a \t\n\r\v\f b", "a  b", "a  b", "a　 b",
	"<p>\u0085x</p>", "\xff<b>\xfe</b>", "<\xff>", "<p>x", "x</p", "<br", "<br /", "</p", "</P>", "<BR>", "<p class=\"c\">p</p>",
	"<img src=\"a.png\" alt=\"<x>\">", "<div><p>nested</p></div>", "<ul><li>a</li><li>b</li></ul>", "text<br>\n<br>text",
	"<p>a</p>\n<p>b</p>\n", " leading", "trailing ", "   ", "<p></p>", "___hugonl_", "<p>___hugonl_</p>",
	"<!DOCTYPE html><html><body>b</body></html>", "<?xml version=\"1.0\"?><r/>", "<a\nhref=x>y</a>", "a<b",
	"<em>日本語</em><br>テキスト", "<p>é́</p>",
}

func main() {
	root := flag.String("root", ".", "neohugo module root")
	out := flag.String("out", "crates/nh-tpl/tests/fixtures/striphtml", "output directory")
	flag.Parse()

	inputs := append([]string{}, edge...)
	alphabet := []string{"<", ">", "/", "p", "b", "r", " ", "\n", "\"", "'", "=", "!", "-", "a", "\xff", " ", "&"}
	for _, a := range alphabet {
		inputs = append(inputs, a)
		for _, b := range alphabet {
			inputs = append(inputs, a+b)
		}
	}
	pieces := []string{"<p>", "</p>", "<br>", "<br />", "<b>", "</b>", "<a href=\"x\">", "</a>", "<!--", "-->", "<script>", "</script>",
		"<style>", "</style>", " ", "  ", "\n", "\t", " ", "text", "日本", "<", ">", "&amp;", "\"", "'", "\xff", "<br/>", "</P>"}
	r := rand.New(rand.NewSource(1))
	for i := 0; i < 4000; i++ {
		var b strings.Builder
		n := 1 + r.Intn(12)
		for j := 0; j < n; j++ {
			b.WriteString(pieces[r.Intn(len(pieces))])
		}
		inputs = append(inputs, b.String())
	}

	var docs []string
	err := filepath.WalkDir(filepath.Join(*root, "docs", "content", "en"), func(p string, d fs.DirEntry, err error) error {
		if err != nil || d.IsDir() || !strings.HasSuffix(p, ".md") {
			return err
		}
		docs = append(docs, p)
		return nil
	})
	if err != nil {
		log.Fatal(err)
	}
	sort.Strings(docs)
	for i, p := range docs {
		if i%10 != 0 {
			continue
		}
		b, err := os.ReadFile(p)
		if err != nil {
			log.Fatal(err)
		}
		if len(b) > 3000 {
			b = b[:3000]
		}
		inputs = append(inputs, string(b))
	}

	var cases [][2]string
	for _, in := range inputs {
		cases = append(cases, [2]string{hex.EncodeToString([]byte(in)), hex.EncodeToString([]byte(tpl.StripHTML(in)))})
	}
	b, err := json.Marshal(map[string]any{"cases": cases})
	if err != nil {
		log.Fatal(err)
	}
	var buf bytes.Buffer
	zw, err := gzip.NewWriterLevel(&buf, gzip.BestCompression)
	if err != nil {
		log.Fatal(err)
	}
	if _, err := zw.Write(b); err != nil {
		log.Fatal(err)
	}
	if err := zw.Close(); err != nil {
		log.Fatal(err)
	}
	dir := *out
	if !filepath.IsAbs(dir) {
		dir = filepath.Join(*root, dir)
	}
	if err := os.MkdirAll(dir, 0o755); err != nil {
		log.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, "striphtml.json.gz"), buf.Bytes(), 0o644); err != nil {
		log.Fatal(err)
	}
	log.Printf("striphtml: %d cases", len(cases))
}
