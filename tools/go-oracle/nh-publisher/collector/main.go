// Command collector is the Go oracle for publisher/htmlElementsCollector.go
// (the hugo_stats.json HTML elements collector) in crates/nh-publisher (Wave
// B task T07).
//
//	go run ./tools/go-oracle/nh-publisher/collector [-out rust/testdata/oracle/publisher/collector]
//
// The oracle runs itself again with `go build -overlay` (osupport): the overlay
// exports parseHTMLElement, isClosedByTag and the collector writer
// (osupport.PublisherPatches). Records (collector.jsonl.gz):
//
//   - "el": parseHTMLElement of every adversarial element string
//     (osupport.ElementStrings: th/caption/col/colgroup/frame/image quirks,
//     the div-substituted table tags, quotes, entities, :class / x-bind:class /
//     v-bind:class / x-transition bindings, foreign content) under three
//     BuildStats configs;
//   - "doc": whole documents, one collector Write each (as Publish does), then
//     getHTMLElements: the upstream TestClassCollector table, the html5lib
//     inputs of x/net/html, hand-written documents (comments/CDATA/script/style/
//     pre/textarea content, quotes left open across elements, U+FFFD and
//     invalid UTF-8, <!DOCTYPE) and random documents; per document and per group
//     through one collector, under five BuildStats configs;
//   - "chunks": documents written in several Writes to ONE writer (random
//     splits, and the Write calls the tdewolff HTML minifier makes when it
//     streams into the collector, as in the upstream test);
//   - "closed": isClosedByTag.
//
// Nothing here depends on the platform.
package main

import (
	"bufio"
	"bytes"
	"flag"
	"log"
	"math/rand"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strings"

	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/config/testconfig"
	"github.com/neohugo/neohugo/media"
	"github.com/neohugo/neohugo/minifiers"
	"github.com/neohugo/neohugo/output"
	"github.com/neohugo/neohugo/publisher"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-publisher/osupport"
)

// oracle is set by the overlay-added hook file.
type oracle struct {
	parseElement func(conf config.BuildStats, s string) (tag string, classes, ids []string, err error)
	collect      func(conf config.BuildStats, docs [][]byte) publisher.HTMLElements
	chunks       func(conf config.BuildStats, chunks [][]byte) (publisher.HTMLElements, string)
	closed       func(b, tag []byte) bool
}

var hooks *oracle

const hookFile = `package main

import "github.com/neohugo/neohugo/publisher"

func init() {
	hooks = &oracle{
		parseElement: publisher.OracleParseElement,
		collect:      publisher.OracleCollect,
		chunks:       publisher.OracleCollectChunks,
		closed:       publisher.OracleIsClosedByTag,
	}
}
`

var confs = map[string]config.BuildStats{
	"all":         {Enable: true},
	"noids":       {Enable: true, DisableIDs: true},
	"noclasses":   {Enable: true, DisableClasses: true},
	"notags":      {Enable: true, DisableTags: true},
	"classesonly": {Enable: true, DisableTags: true, DisableIDs: true},
}

func confNames() []string {
	var names []string
	for k := range confs {
		names = append(names, k)
	}
	sort.Strings(names)
	return names
}

// Upstream publisher/htmlElementsCollector_test.go TestClassCollector inputs.
var upstreamDocs = []string{
	`<body class="b a"></body>`,
	`<div class="b a b"></div><div class="b a b"></div>x'`,
	`<body class='b a'></body>`,
	`<body class=b id=myelement></body>`,
	`<i>`,
	`< body class="b a"></body><div></div>`,
	"<table class=\"cl1\">\n    <thead class=\"cl2\"><tr class=\"cl3\"><td class=\"cl4\"></td></tr></thead>\n    <tbody class=\"cl5\"><tr class=\"cl6\"><td class=\"cl7\"></td></tr></tbody>\n</table>",
	"<TABLE class=\"CL1\">\n    <THEAD class=\"CL2\"><TR class=\"CL3\"><TD class=\"CL4\"></TD></TR></THEAD>\n    <TBODY class=\"CL5\"><TR class=\"CL6\"><TD class=\"CL7\"></TD></TR></TBODY>\n</TABLE>",
	`<a class="b a" href=/></a>`,
	"<body>\n    <div x-bind:class=\"{\n        'class1': data.open,\n        'class2 class3': data.foo == 'bar'\n         }\">\n    </div>\n</body>",
	`<div x-bind:class="{ 'bg-black':  filter.checked }" class="inline-block mr-1 mb-2 rounded  bg-gray-300 px-2 py-2">FOO</div>`,
	`<div x-bind:class="{ 'text-gray-800':  !checked, 'text-white': checked }"></div>`,
	"<div x-bind:class=\"{ 'text-gray-800':  !checked, \n\t\t\t\t\t 'text-white': checked }\"></div>",
	"<a x-bind:class=\"{\n                'text-a': a && b,\n                'text-b': !a && b || c,\n                'pl-3': a === 1,\n                 pl-2: b == 3,\n                'text-gray-600': (a > 1)\n                }\" class=\"block w-36 cursor-pointer pr-3 no-underline capitalize\"></a>",
	`<button :class="isActive(32) ? 'border-gray-500 bg-white pt border-t-2' : 'border-transparent hover:bg-gray-100'"></button>`,
	`<button :class="{ 'border-gray-500 bg-white pt border-t-2': isActive(32), 'border-transparent hover:bg-gray-100': !isActive(32) }"></button>`,
	`<div x-transition:enter-start="opacity-0 transform mobile:-translate-x-8 sm:-translate-y-8">`,
	`<div v-bind:class="{ active: isActive }"></div>`,
	`<a class="missingclass" title="Plus d'information">my text</a><div></div>`,
	`<script><span>foo</span><span>bar</span></script><div class="foo"></div>`,
	`<style>p{color: red;font-size: 20px;}</style><div class="foo"></div>`,
	`<pre class="preclass"><span>foo</span><span>bar</span></pre><div class="foo"></div>`,
	`<textarea class="textareaclass"><span>foo</span><span>bar</span></textarea><div class="foo"></div>`,
	`<!DOCTYPE html>`,
	`<!-- example comment -->`,
	`<div></div><!-- example comment --><span><span>`,
	`<div><hr/></div>`,
	`<svg><style/><g><path class="foo"/></g></svg>`,
	`<!-- Hero Area Image d'accueil --><i class="foo">`,
	`<DIV></DIV>`,
	`<script>if (a < b) { nothing(); }</SCRIPT><div></div>`,
	`<hr	id="a" class="foo"><div class="bar">d</div>`,
	"<form\n\t\t\tid=\"a\"\n\t\t\taction=\"www.example.com\"\n\t\t\tmethod=\"post\"\n></form>\n<div id=\"b\" class=\"foo\">d</div>",
	strings.Repeat(`神真美好 `, 37) + "<div id=\"神真美好\" class=\"foo\">" + strings.Repeat(`神真美好 `, 11) + "   <span>神真美好</span>",
}

var handDocs = []string{
	`<p class="a">x</p><!-- <div class="incomment"> --><span class="b">`,
	`<!----><!---><div class="after-odd-comment"></div>--><i class="i">`,
	`<![CDATA[<div class="cdata">]]><em class="em">`,
	`<?php echo "<div class='php'>"; ?><b class="bb">`,
	`<script type="text/template"><div class="tpl"></div></script ><u class="u">`,
	"<script>a</script\t><s class=\"s\">",
	"<style>x</ style><q class=\"q\">",
	`<pre>unclosed <div class="inpre">`,
	`<prefix class="prefixed"><div class="d"></div></prefix>`,
	`<textareax class="tx"></textarea><div class="d2">`,
	`<scripty class="sy"></scripty><div class="d3">`,
	`<div title='open quote><span class="hidden-by-quote"></span><i class='x'>`,
	`<div a="1' b='2"><span class="sp">`,
	`<a title=it's class=c>y</a><b class='d'>`,
	"<div class=\"a\">�<span class=\"afterfffd\">",
	"<div class=\"a\">\xff<span class=\"afterinvalid\">",
	"<div class=\"\xe2\x82\">",
	`<<div class="double-lt">`,
	`< div class="space-lt"><div class="ok">`,
	`<!DOCTYPE html><!doctype x><!DOCTYPEfoo class="z"><div class="after-doctype">`,
	`</div class="endtag"><div class="real">`,
	`<div/><br/><img src=x class="img"/><image class="image-tag">`,
	`<th class="thc" id="thid"><td class="tdc" id="tdid"><caption class="cap"><col class="colc"><colgroup class="cg"><frame class="fr">`,
	`<svg viewBox="0 0 1 1" class="svgc"><foreignObject class="fo"><div class="indiv">`,
	`<math class="m"><mi class="mi">`,
	`<html class="h" id="hid"><head class="hd"><body class="bd" id="bid">`,
	`<template class="t"><tr class="trt"></template>`,
	`<select class="sel"><option class="opt"><textarea class="ta">`,
	`<div x-transition:enter="a b" x-transition:leave="c" transition="d" data-transition="e" :transition="f">`,
	`<div :class="{ 'a': x, 'b c':\ny }" x-bind:class="['k', 'l']" v-bind:class="m ? 'n' : 'o'">`,
	"<div :class=\"{\n  'multi-a':\n  x,\n  plain: y,\n  'q':z }\">",
	`<div class="&amp;a &lt;b &notit; &#x41;">`,
	`<p id=one id=two class=three class=four>`,
	`<p ID="UP" CLASS="UPC" Class="Mixed">`,
	`<div class=x>` + strings.Repeat("<i>", 50) + `<i class="deep">`,
	`<a href="/x">` + strings.Repeat("<b class=\"b\">", 5) + `</a>`,
	"<div class=\"nbsp\"><span class=\"ls\">",
	`<DİV class="dotted-i"><ſcript class="long-s">x</ſcript><div class="after-long-s">`,
	`<div class="z"></div><div class="z"></div><div class="z"></div>`,
	"", "<", ">", "<>", "<a", "<a>", "</>", "<!", "<!-", "<!--", "<!-- x", "<a b='", `<a b="c>`, "<a\n>",
}

// soupAlphabet mixes element strings, text and the collector's special cases.
var soupAlphabet = []string{
	"<div class=\"a b\">", "<span id=s>", "</div>", "</span>", "text ", "'", "\"", "<!--", "-->", "<pre>", "</pre>",
	"<script>", "</script>", "<style>", "</style>", "<textarea>", "</textarea>", "<!DOCTYPE html>", "<th class=t>",
	"<tr class=r>", "<td id=d>", "<image class=im>", "<svg class=sv>", "<p class='q r'>", "<a :class=\"{ 'k': 1 }\">",
	"<b x-transition=\"tt\">", "\n", "\t", " ", ">", "<", "</ script>", "</SCRIPT>", "<br/>", "<hr/>", "é", "神",
	"<i class=i1>", "<i class=i2>", "<![CDATA[", "]]>",
}

func splitRandom(rnd *rand.Rand, b []byte, runeAligned bool) [][]byte {
	var out [][]byte
	for len(b) > 0 {
		n := 1 + rnd.Intn(12)
		if n > len(b) {
			n = len(b)
		}
		if runeAligned {
			for n < len(b) && b[n]&0xC0 == 0x80 {
				n++
			}
		}
		out = append(out, b[:n])
		b = b[n:]
	}
	return out
}

func encList(l []string) any {
	if l == nil {
		return nil
	}
	out := make([]any, len(l))
	for i, s := range l {
		out[i] = osupport.S(s)
	}
	return out
}

func encElements(e publisher.HTMLElements) map[string]any {
	return map[string]any{"tags": encList(e.Tags), "classes": encList(e.Classes), "ids": encList(e.IDs)}
}

func html5libInputs() ([]string, error) {
	out, err := exec.Command("go", "list", "-f", "{{.Dir}}", "golang.org/x/net/html").Output()
	if err != nil {
		return nil, err
	}
	files, err := filepath.Glob(filepath.Join(strings.TrimSpace(string(out)), "testdata", "webkit", "*.dat"))
	if err != nil {
		return nil, err
	}
	sort.Strings(files)
	var inputs []string
	for _, fn := range files {
		f, err := os.Open(fn)
		if err != nil {
			return nil, err
		}
		var cur []string
		in := false
		sc := bufio.NewScanner(f)
		sc.Buffer(make([]byte, 1<<20), 1<<24)
		for sc.Scan() {
			line := sc.Text()
			if strings.HasPrefix(line, "#") {
				if in {
					inputs = append(inputs, strings.Join(cur, "\n"))
				}
				in = line == "#data"
				cur = nil
				continue
			}
			if in {
				cur = append(cur, line)
			}
		}
		_ = f.Close()
		if err := sc.Err(); err != nil {
			return nil, err
		}
	}
	return inputs, nil
}

func main() {
	root := flag.String("root", ".", "neohugo module root")
	out := flag.String("out", "rust/testdata/oracle/publisher/collector", "output directory")
	flag.Parse()

	if !osupport.IsChild() {
		if err := osupport.RunOverlaid(*root, "./tools/go-oracle/nh-publisher/collector", osupport.PublisherPatches,
			"hook_overlay.go", hookFile, []string{"-root", *root, "-out", *out}); err != nil {
			log.Fatal(err)
		}
		return
	}
	if hooks == nil {
		log.Fatal("collector: hooks not installed (not run through the overlay)")
	}

	w, err := osupport.Create(filepath.Join(*out, "collector.jsonl.gz"))
	if err != nil {
		log.Fatal(err)
	}
	write := func(rec map[string]any) {
		if err := w.Write(rec); err != nil {
			log.Fatal(err)
		}
	}

	// "el": parseHTMLElement.
	for _, s := range osupport.ElementStrings() {
		for _, cn := range []string{"all", "noids", "noclasses"} {
			var tag string
			var classes, ids []string
			var err error
			pm := osupport.PanicString(func() {
				tag, classes, ids, err = hooks.parseElement(confs[cn], s)
			})
			rec := map[string]any{"t": "el", "conf": cn, "s": osupport.S(s)}
			switch {
			case pm != "":
				rec["panic"] = pm
			case err != nil:
				rec["err"] = err.Error()
			default:
				rec["tag"] = osupport.S(tag)
				rec["classes"] = encList(classes)
				rec["ids"] = encList(ids)
			}
			write(rec)
		}
	}

	// "doc": whole documents.
	html5lib, err := html5libInputs()
	if err != nil {
		log.Fatal(err)
	}
	rnd := rand.New(rand.NewSource(5))
	var random []string
	for i := 0; i < 4000; i++ {
		var b strings.Builder
		for j := 1 + rnd.Intn(30); j > 0; j-- {
			b.WriteString(soupAlphabet[rnd.Intn(len(soupAlphabet))])
		}
		random = append(random, b.String())
	}
	elementDocs := osupport.ElementStrings()

	groups := map[string][]string{"upstream": upstreamDocs, "hand": handDocs, "html5lib": html5lib, "random": random, "elements": elementDocs}
	for _, g := range []string{"upstream", "hand", "html5lib", "random", "elements"} {
		docs := groups[g]
		// Each document alone (every config for the small groups, "all" for the others).
		docConfs := []string{"all"}
		if g == "upstream" || g == "hand" {
			docConfs = confNames()
		}
		for _, cn := range docConfs {
			for _, d := range docs {
				var res publisher.HTMLElements
				pm := osupport.PanicString(func() { res = hooks.collect(confs[cn], [][]byte{[]byte(d)}) })
				rec := map[string]any{"t": "doc", "g": g, "conf": cn, "doc": osupport.S(d)}
				if pm != "" {
					rec["panic"] = pm
				} else {
					rec["want"] = encElements(res)
				}
				write(rec)
			}
		}
		// The whole group through one collector (the element set is shared); the Rust
		// test takes the documents from the group's "doc" records.
		var all [][]byte
		for _, d := range docs {
			all = append(all, []byte(d))
		}
		for _, cn := range confNames() {
			var res publisher.HTMLElements
			pm := osupport.PanicString(func() { res = hooks.collect(confs[cn], all) })
			rec := map[string]any{"t": "group", "g": g, "conf": cn, "n": len(all)}
			if pm != "" {
				rec["panic"] = pm
			} else {
				rec["want"] = encElements(res)
			}
			write(rec)
		}
	}

	// "chunks": several Writes to one writer.
	chunkDocs := append(append(append([]string{}, upstreamDocs...), handDocs...), random[:500]...)
	for i, d := range chunkDocs {
		for _, aligned := range []bool{true, false} {
			chunks := splitRandom(rnd, []byte(d), aligned)
			var enc []any
			for _, c := range chunks {
				enc = append(enc, osupport.B(c))
			}
			var res publisher.HTMLElements
			var errs string
			pm := osupport.PanicString(func() { res, errs = hooks.chunks(confs["all"], chunks) })
			rec := map[string]any{"t": "chunks", "conf": "all", "chunks": enc, "i": i}
			if pm != "" {
				rec["panic"] = pm
			} else {
				rec["want"] = encElements(res)
				rec["err"] = errs
			}
			write(rec)
		}
	}
	// The Write calls of the tdewolff HTML minifier streaming into the
	// collector (the upstream test's minify variant).
	for _, d := range append(append([]string{}, upstreamDocs...), handDocs...) {
		rw := &recordingWriter{}
		if err := minifyHTML(rw, d); err != nil {
			continue
		}
		var enc []any
		for _, c := range rw.chunks {
			enc = append(enc, osupport.B(c))
		}
		res, errs := hooks.chunks(confs["all"], rw.chunks)
		write(map[string]any{"t": "chunks", "conf": "all", "chunks": enc, "want": encElements(res), "err": errs, "minified": true})
	}

	// "closed": isClosedByTag.
	closedCases := [][2]string{
		{"", "div"}, {"foo", "div"}, {"foo<div>", "div"}, {"foo/div>", "div"}, {"foo//div>", "div"},
		{"foo</>", "div"}, {"foo</div>", "div"}, {"foo<  / div>", "div"}, {"foo<  / div   \n>", "div"},
		{"foo</DIV>", "div"}, {`</defs><g><g><path fill="#010101" d=asdf"/>`, "div"},
		{"x</script\t>", "script"}, {"x</scr ipt>", "script"}, {"x< /script>", "script"}, {"x</ſcript>", "script"},
		{"x</pre >", "PRE"}, {"</a/b>", "b"}, {"<//x>", "x"}, {"</x/>", "x"}, {"</ x x>", "x"}, {">", "x"}, {"</>", ""},
	}
	for _, c := range closedCases {
		write(map[string]any{"t": "closed", "b": osupport.S(c[0]), "tag": osupport.S(c[1]), "want": hooks.closed([]byte(c[0]), []byte(c[1]))})
	}
	alpha := []string{"<", "/", ">", " ", "\t", "\n", "\r", "x", "X", "div", "DIV", "a"}
	for i := 0; i < 3000; i++ {
		var b strings.Builder
		for j := rnd.Intn(9); j >= 0; j-- {
			b.WriteString(alpha[rnd.Intn(len(alpha))])
		}
		tag := []string{"x", "div", "a", "X"}[rnd.Intn(4)]
		write(map[string]any{"t": "closed", "b": osupport.S(b.String()), "tag": tag, "want": hooks.closed([]byte(b.String()), []byte(tag))})
	}

	if err := w.Close(); err != nil {
		log.Fatal(err)
	}
	log.Printf("collector: %d records", w.N())
}

type recordingWriter struct {
	chunks [][]byte
}

func (w *recordingWriter) Write(p []byte) (int, error) {
	w.chunks = append(w.chunks, bytes.Clone(p))
	return len(p), nil
}

// minifyHTML streams doc through neohugo's HTML minifier (the default
// config) into w, as the upstream TestClassCollector minify variant does.
func minifyHTML(w *recordingWriter, doc string) error {
	m, err := minifiers.New(media.DefaultTypes, output.DefaultFormats, testconfig.GetTestConfig(nil, nil))
	if err != nil {
		return err
	}
	return m.Minify(media.Builtin.HTMLType, w, strings.NewReader(doc))
}
