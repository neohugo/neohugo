// Command absurl is the Go oracle for the absURL / absURLInXML replacer
// (transform/urlreplacers) and transform.Chain in crates/nh-transform (Wave B
// task T07).
//
//	go run ./tools/go-oracle/nh-transform/absurl [-out rust/testdata/oracle/transform/absurl]
//
// Every case runs through transform.New(urlreplacers.NewAbs...Transformer(path))
// .Apply with the source delivered in one piece, one byte per Read
// (iotest.OneByteReader) and in 7-byte chunks; the outputs must agree (the
// chain reads the whole source before the replacer runs), else the oracle
// fails. A Go panic is recorded as {"panic": text}.
//
// Inputs: the upstream absurlreplacer_test.go tables, the specs/output-
// publishing.md §3.3 vectors, a combinatorial grid (prefix × quote × value ×
// suffix × base path) and random documents over an alphabet of candidate
// fragments. Output: cases.jsonl.gz. The livereloadinject and metainject
// transformers (inject.go) go to inject.jsonl.gz. Nothing here depends on the
// platform.
package main

import (
	"bytes"
	"errors"
	"flag"
	"fmt"
	"io"
	"log"
	"math/rand"
	"path/filepath"
	"testing/iotest"

	"github.com/neohugo/neohugo/helpers"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-publisher/osupport"
	"github.com/neohugo/neohugo/transform"
	"github.com/neohugo/neohugo/transform/urlreplacers"
)

type chunkReader struct {
	b []byte
	n int
}

func (r *chunkReader) Read(p []byte) (int, error) {
	if len(r.b) == 0 {
		return 0, io.EOF
	}
	n := r.n
	if n > len(p) {
		n = len(p)
	}
	if n > len(r.b) {
		n = len(r.b)
	}
	copy(p, r.b[:n])
	r.b = r.b[n:]
	return n, nil
}

func run(kind, path string, in []byte, mk func() io.Reader) (out []byte, panicMsg string, err error) {
	var tr transform.Transformer
	if kind == "html" {
		tr = urlreplacers.NewAbsURLTransformer(path)
	} else {
		tr = urlreplacers.NewAbsURLInXMLTransformer(path)
	}
	c := transform.New(tr)
	var b bytes.Buffer
	panicMsg = osupport.PanicString(func() {
		err = c.Apply(&b, mk())
	})
	return b.Bytes(), panicMsg, err
}

type gen struct {
	w    *osupport.Writer
	seen map[string]bool
}

func (g *gen) add(kind, path string, in []byte) error {
	key := kind + "\x00" + path + "\x00" + string(in)
	if g.seen[key] {
		return nil
	}
	g.seen[key] = true
	out, pm, err := run(kind, path, in, func() io.Reader { return bytes.NewReader(in) })
	out1, pm1, err1 := run(kind, path, in, func() io.Reader { return iotest.OneByteReader(bytes.NewReader(in)) })
	out7, pm7, err7 := run(kind, path, in, func() io.Reader { return &chunkReader{b: in, n: 7} })
	if !bytes.Equal(out, out1) || !bytes.Equal(out, out7) || pm != pm1 || pm != pm7 || (err == nil) != (err1 == nil) || (err == nil) != (err7 == nil) {
		return fmt.Errorf("chunking changed the output of %q", in)
	}
	rec := map[string]any{"k": kind, "p": osupport.S(path), "in": osupport.B(in)}
	switch {
	case pm != "":
		rec["panic"] = pm
	case err != nil:
		rec["err"] = err.Error()
	default:
		rec["out"] = osupport.B(out)
	}
	return g.w.Write(rec)
}

// Upstream transform/urlreplacers/absurlreplacer_test.go inputs.
var upstream = []string{
	"<!DOCTYPE html><html><head><script src=\"foobar.js\"></script><script src=\"/barfoo.js\"></script></head><body><nav><h1>title</h1></nav><article>content <a href=\"foobar\">foobar</a>. <a href=\"/foobar\">Follow up</a></article></body></html>",
	"<!DOCTYPE html><html><head><script src='foobar.js'></script><script src='/barfoo.js'></script></head><body><nav><h1>title</h1></nav><article>content <a href='foobar'>foobar</a>. <a href='/foobar'>Follow up</a></article></body></html>",
	"<!DOCTYPE html><html><head><script src=\"http://user@host:10234/foobar.js\"></script></head><body><nav><h1>title</h1></nav><article>content <a href=\"https://host/foobar\">foobar</a>. Follow up</article></body></html>",
	"<!DOCTYPE html><html><head><script src=\"//host/foobar.js\"></script><script src='//host2/barfoo.js'></head><body><nav><h1>title</h1></nav><article>content <a href=\"//host/foobar\">foobar</a>. <a href='//host2/foobar'>Follow up</a></article></body></html>",
	"<?xml version=\"1.0\" encoding=\"utf-8\" standalone=\"yes\" ?><feed xmlns=\"http://www.w3.org/2005/Atom\"><entry><content type=\"html\">&lt;p&gt;&lt;a href=&#34;/foobar&#34;&gt;foobar&lt;/a&gt;&lt;/p&gt; &lt;p&gt;A video: &lt;iframe src=&#39;/foo&#39;&gt;&lt;/iframe&gt;&lt;/p&gt;</content></entry></feed>",
	"<?xml version=\"1.0\" encoding=\"utf-8\" standalone=\"yes\" ?><feed xmlns=\"http://www.w3.org/2005/Atom\"><entry><content type=\"html\">&lt;p&gt;&lt;a href=&#34;//foobar&#34;&gt;foobar&lt;/a&gt;&lt;/p&gt; &lt;p&gt;A video: &lt;iframe src=&#39;//foo&#39;&gt;&lt;/iframe&gt;&lt;/p&gt;</content></entry></feed>",
	"No replacements.",
	"ᚠᛇᚻ ᛒᛦᚦ ᚠᚱᚩᚠᚢᚱ\nᚠᛁᚱᚪ ᚷᛖᚻᚹᛦᛚᚳᚢᛗ",
	`End of file: src="/`,
	`Srcsett with no closing quote: srcset="/img/small.jpg do be do be do.`,
	`Pre. src='//schemaless' src='/normal'  <a href="//schemaless">Schemaless</a>. <a href="/normal">normal</a>. Post.`,
	`Pre. src=&#39;//schemaless&#39; src=&#39;/normal&#39;  <a href=&#39;//schemaless&#39;>Schemaless</a>. <a href=&#39;/normal&#39;>normal</a>. Post.`,
	`Pre. <img srcset="/img/small.jpg 200w, /img/medium.jpg 300w, /img/big.jpg 700w" alt="text" src="/img/foo.jpg">`,
	`Pre. <img srcset='/img/small.jpg 200w, /img/big.jpg 700w' alt="text" src="/img/foo.jpg"> POST.`,
	`Pre. <img srcset=&#34;/img/small.jpg 200w, /img/big.jpg 700w&#34; alt=&#34;text&#34; src=&#34;/img/foo.jpg&#34;>`,
	"Pre.\nMissing start quote: <img srcset=/img/small.jpg 200w, /img/big.jpg 700w\" alt=\"text\"> src='/img/foo.jpg'> FOO.\n<img srcset='/img.jpg'>\nschemaless: <img srcset='//img.jpg' src='//basic.jpg'>\nschemaless2: <img srcset=\"//img.jpg\" src=\"//basic.jpg2> POST\n",
	"Pre.\nMissing start quote: &lt;img srcset=/img/small.jpg 200w /img/big.jpg 700w&quot; alt=&quot;text&quot;&gt; src=&#39;/img/foo.jpg&#39;&gt; FOO.\n&lt;img srcset=&#39;/img.jpg&#39;&gt;\nschemaless: &lt;img srcset=&#39;//img.jpg&#39; src=&#39;//basic.jpg&#39;&gt;\nschemaless2: &lt;img srcset=&quot;//img.jpg&quot; src=&quot;//basic.jpg2&gt; POST\n",
	`PRE. a href="/img/small.jpg" input action="/foo.html" meta url=/redirect/to/page/ POST.`,
	`Link: <a href=/asdf>ASDF</a>`,
	`Link: <a href=/asdf   >ASDF</a>`,
}

// specs/output-publishing.md §3.3 vectors.
var spec = []string{
	`<a href="/about">`, `<a href='/about'>`, `<a href=/about>`, `<a href="//cdn.x/a.js">`, `href="https://x/a"`,
	`<img data-src="/a.png">`, `<form action="/search">`, `content="0; url=/foo"`, `content="0; url=https://seeksnack.com/"`,
	`<img srcset="/a.png 1x, /b.png 2x">`, `srcset="/a.png 1x,  https://x/b.png   2x"`, `<img srcset=/a.png>`,
	`style="background:url(/a.png)"`, `<p>use href="/x"</p>`, `<script>…'src="/x"'…</script>`, `<a HREF="/x">`, `<a href= "/x">`,
	`<a href="/">`, `<a href="/`, `href=""`, `/foo`, `/foo bar src="/a"`, `"/x"`, `'/x' href="/y"`,
	`<img srcset="/a.png?href=/b 1x"><a href="/c">`, `&lt;img src=&#34;/a.png&#34;&gt;`, `<atom:link href="/index.xml"/>`, `href='/a'`, `<x href=/a>`,
	`href="/docs/a"`, `href="/docs"`, `href="/docsx"`, `srcset="/docs/a.png 1x, /docs/b.png 2x, /c.png"`,
}

var paths = []string{
	"https://example.org/", "https://example.org/docs/", "http://base/", "https://seeksnack.com/",
	"https://example.org/docs", "https://example.org", "./", "../", "../../", "../../../", ".",
	"/", "", "https://example.org/%E2%82%AC/", "https://example.org/a%20b/", "http://x/?q=1",
	"https://example.org/docs/?q=/docs", ":bad", "https://example.org/d%ffx/", "//host/sub/",
	"https://user@host:8080/p/", "https://example.org/docs/sub/", "docs/", "/docs/",
	helpers.GetDottedRelativePath(filepath.FromSlash("/post/sub/")),
}

var (
	prefixes = []string{"src=", "href=", "url=", "action=", "srcset=", "SRC=", "data-src=", "xsrc=", "src =", "src=\n", "hre", "=", "srcset"}
	quotes   = []string{"\"", "'", "", "&#34;", "&#39;", "&quot;", "&#x27;", "`", "\"'", "&#34", "&#3"}
	values   = []string{
		"/", "//", "//cdn/x.js", "/a", "/docs/a", "/docs", "/docsx", "/docs/", "http://x/a", "", "/a b", "/ä",
		" /a", "/a 1x, /b 2x", "/a 1x,/b 2x", "/a\t1x,\n/b  2x", "/a 1x, //b 2x", "/a 1x, http://b/c 2x",
		"/\xff", "/\xe2\x82", "/a>b", "/a?href=/b", "/a#src=/b", "/a%20b", "/docs/docs/a", "x/a",
	}
	suffixes = []string{"\"", "'", "", ">", " alt=\"x\">", "&#34;>", "&#39;", "\" src=\"/y\">", " href='/z'", "\n", "\"/>"}
)

var alphabet = []string{
	"src=", "href=", "url=", "action=", "srcset=", "\"", "'", "&#34;", "&#39;", "/", "//", "/a", "/docs/", "docs/",
	" ", "  ", "\n", "\t", ">", "<", "a", "x", "1x,", "200w", ",", "ä", "\xff", "http://h/", "=", "<img ", "<a ",
	"\x00", " ", " ", "&", "#", "?", "src", "set=",
}

func main() {
	out := flag.String("out", "rust/testdata/oracle/transform/absurl", "output directory")
	flag.Parse()

	w, err := osupport.Create(filepath.Join(*out, "cases.jsonl.gz"))
	if err != nil {
		log.Fatal(err)
	}
	g := &gen{w: w, seen: map[string]bool{}}
	check := func(err error) {
		if err != nil {
			log.Fatal(err)
		}
	}

	for _, kind := range []string{"html", "xml"} {
		for _, p := range paths {
			for _, s := range upstream {
				check(g.add(kind, p, []byte(s)))
			}
			for _, s := range spec {
				check(g.add(kind, p, []byte(s)))
			}
		}
	}

	// The grid: every prefix × quote × value × suffix, with a short lead-in
	// (HTML with a base path that has a root, XML with a relative one).
	for _, kind := range []string{"html", "xml"} {
		p := "https://example.org/docs/"
		if kind == "xml" {
			p = "../"
		}
		{
			for _, pre := range prefixes {
				for _, q := range quotes {
					for _, v := range values {
						for _, suf := range suffixes {
							check(g.add(kind, p, []byte("<x "+pre+q+v+suf)))
						}
					}
				}
			}
		}
	}

	// Random documents.
	rnd := rand.New(rand.NewSource(7))
	for i := 0; i < 10000; i++ {
		var b bytes.Buffer
		n := 1 + rnd.Intn(40)
		for j := 0; j < n; j++ {
			b.WriteString(alphabet[rnd.Intn(len(alphabet))])
		}
		kind := "html"
		if rnd.Intn(3) == 0 {
			kind = "xml"
		}
		check(g.add(kind, paths[rnd.Intn(len(paths))], b.Bytes()))
	}

	// Long srcset values around the 2000-byte guard.
	for _, n := range []int{1990, 1998, 1999, 2000, 2001, 2002, 2010} {
		v := "/" + string(bytes.Repeat([]byte("a"), n-1))
		check(g.add("html", "https://example.org/", []byte(`<img srcset="`+v+`">`)))
		check(g.add("xml", "https://example.org/", []byte(`<img srcset=&#34;`+v+`&#34;>`)))
	}

	check(w.Close())
	if w.N() == 0 {
		log.Fatal(errors.New("no cases"))
	}
	log.Printf("absurl: %d cases", w.N())

	writeInject(*out)
}
