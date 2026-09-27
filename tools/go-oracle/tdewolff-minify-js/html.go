package main

import (
	"bytes"
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"html"
	"math/rand"
	"path/filepath"
	"regexp"
	"sort"
	"strconv"

	"github.com/tdewolff/minify/v2"
	"github.com/tdewolff/minify/v2/css"
	mhtml "github.com/tdewolff/minify/v2/html"
	"github.com/tdewolff/minify/v2/js"
	"github.com/tdewolff/minify/v2/json"
	"github.com/tdewolff/minify/v2/svg"
	"github.com/tdewolff/minify/v2/xml"
	"github.com/tdewolff/parse/v2/buffer"
)

// Integration with the rest of minify: the HTML minifier hands <script>
// contents and on* attributes to the JS minifier (inline=1 for attributes),
// in place on its own input buffer.

// Regexps registered by neohugo's minifiers.New (minifiers/minifiers.go).
const (
	jsPattern   = "^(application|text)/(x-)?(java|ecma)script$"
	jsonPattern = `^(application|text)/(x-|(ld|manifest)\+)?json$`
)

// hugoM mirrors neohugo's minifiers.New with the seeksnack [minify] config
// (the defaults: js.Minifier{Version: 2022}).
func hugoM() *minify.M {
	m := minify.New()
	m.Add("text/css", &css.Minifier{Precision: 0, KeepCSS2: true})
	jsm := &js.Minifier{Version: 2022}
	m.Add("text/javascript", jsm)
	m.AddRegexp(regexp.MustCompile(jsPattern), jsm)
	m.Add("application/json", &json.Minifier{})
	m.AddRegexp(regexp.MustCompile(jsonPattern), &json.Minifier{})
	m.Add("image/svg+xml", &svg.Minifier{KeepComments: false, Precision: 0})
	m.Add("application/rss+xml", &xml.Minifier{KeepWhitespace: false})
	m.Add("application/xml", &xml.Minifier{KeepWhitespace: false})
	m.Add("text/html", &mhtml.Minifier{
		KeepDocumentTags:    true,
		KeepSpecialComments: true,
		KeepEndTags:         true,
		KeepDefaultAttrVals: true,
		KeepWhitespace:      false,
	})
	return m
}

// upstreamHTMLM is the M of html_test.go:TestHTMLCSSJS.
func upstreamHTMLM() *minify.M {
	m := minify.New()
	m.AddFunc("text/html", mhtml.Minify)
	m.AddFunc("text/css", css.Minify)
	m.AddFunc("application/javascript", js.Minify)
	m.AddFunc("image/svg+xml", svg.Minify)
	return m
}

func htmlM(cfg string) *minify.M {
	switch cfg {
	case "hugo":
		return hugoM()
	case "upstream":
		return upstreamHTMLM()
	}
	panic("bad html config " + cfg)
}

// runHTML minifies a private copy of the HTML document in and returns the
// record fields [output, err, after].
func runHTML(cfg string, in []byte) [][]byte {
	buf := cp(in)
	pristine := append([]byte(nil), buf[:cap(buf)]...)
	var w bytes.Buffer
	var err error
	func() {
		defer func() {
			if r := recover(); r != nil {
				err = fmt.Errorf("PANIC: %v", r)
			}
		}()
		err = htmlM(cfg).Minify("text/html", &w, buffer.NewReader(buf))
	}()
	var after []byte
	if !bytes.Equal(buf[:cap(buf)], pristine) {
		after = append([]byte("A"), buf[:cap(buf)]...)
	}
	return [][]byte{w.Bytes(), errField(err), after}
}

// htmlLiterals returns the string literals of minify's html_test.go.
func htmlLiterals() [][]byte {
	fn := filepath.Join(filepath.Dir(minifyJSDir()), "html", "html_test.go")
	fset := token.NewFileSet()
	f, err := parser.ParseFile(fset, fn, nil, 0)
	if err != nil {
		panic(err)
	}
	seen := map[string]bool{}
	var lits []string
	ast.Inspect(f, func(n ast.Node) bool {
		if bl, ok := n.(*ast.BasicLit); ok && bl.Kind == token.STRING {
			if s, err := strconv.Unquote(bl.Value); err == nil && !seen[s] {
				seen[s] = true
				lits = append(lits, s)
			}
		}
		return true
	})
	sort.Strings(lits)
	var out [][]byte
	for _, s := range lits {
		out = append(out, []byte(s))
	}
	return out
}

var htmlWrappers = []string{
	"<script>%s</script>",
	"<p>a</p>\n<script type=\"text/javascript\">%s</script>\n<p>b</p>",
	"<script type=module>%s</script><script>%s</script>",
	"<button onclick=\"%s\">b</button>",
	"<a href=# onmouseover='%s' onclick=\"javascript:%s\">x</a>",
	"<body onload=\"  %s  \"><script>\n%s\n</script></body>",
	"<svg><script>%s</script></svg>",
	"<div><script type=\"application/ld+json\">{\"a\": 1}</script><script async src=x.js></script><script>%s</script></div>",
}

// genHTML writes the `html` fixture: documents that embed the literals in
// scripts and event-handler attributes, the html_test.go literals, and the
// upstream TestHTMLCSSJS table, through neohugo's M.
func genHTML(dir string, lits [][]byte, seed int64) {
	w := newRecWriter(dir, "html")
	rnd := rand.New(rand.NewSource(seed))
	for k, src := range lits {
		if k%4 != 0 {
			continue
		}
		wrap := htmlWrappers[rnd.Intn(len(htmlWrappers))]
		s := string(src)
		if bytes.Contains([]byte(wrap), []byte("on")) && rnd.Intn(2) == 0 {
			s = html.EscapeString(s)
		}
		var doc []byte
		if bytes.Count([]byte(wrap), []byte("%s")) == 1 {
			doc = []byte(fmt.Sprintf(wrap, s))
		} else {
			doc = []byte(fmt.Sprintf(wrap, s, s))
		}
		w.rec(append([][]byte{[]byte("html"), []byte("hugo"), doc}, runHTML("hugo", doc)...)...)
	}
	for _, doc := range htmlLiterals() {
		for _, cfg := range []string{"hugo", "upstream"} {
			w.rec(append([][]byte{[]byte("html"), []byte(cfg), doc}, runHTML(cfg, doc)...)...)
		}
	}
	w.close()
}
