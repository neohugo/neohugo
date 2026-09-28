// Command summary is the Go oracle for page.ExtractSummaryFromHTML,
// page.ExtractSummaryFromHTMLWithDivider and the HtmlSummary methods
// (resources/page/page_markup.go) in crates/nh-page (Wave B task T11).
//
//	go run ./tools/go-oracle/nh-page/summary [-root .] [-out crates/nh-page/tests/fixtures/summary]
//
// Like the paths oracle it runs itself again with `go run -overlay`; the
// overlay adds hooks to both extract functions that record every call of the
// real builds (docs/, hugolib/testsite and the synthetic psupport sites):
// the rendered HTML of every page (auto summaries; manual summaries with
// Hugo's internal divider; CJK pages), its media type, word count and the
// results. The same HTML is then run through both functions with other word
// counts, the AsciiDoc and reStructuredText media types and extra dividers,
// and adversarial HTML strings are added (paragraph and div variants,
// dividers at every position, invalid UTF-8, whitespace runs, CJK text).
//
// Output: build.json.gz (the recorded calls and their variants),
// adversarial.json.gz. Nothing here depends on the platform.
package main

import (
	"crypto/sha256"
	"encoding/hex"
	"flag"
	"log"
	"path/filepath"
	"sort"
	"strings"
	"sync"

	"github.com/neohugo/neohugo/media"
	"github.com/neohugo/neohugo/resources/page"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-page/psupport"
)

// installHooks is set by the overlay-added hook file (see hookFile).
var installHooks func(record func(kind string, mt media.Type, input, divider string, numWords int, isCJK bool))

const hookFile = `package main

import (
	"github.com/neohugo/neohugo/media"
	"github.com/neohugo/neohugo/resources/page"
)

func init() {
	installHooks = func(record func(kind string, mt media.Type, input, divider string, numWords int, isCJK bool)) {
		page.OracleHookSummary = record
	}
}
`

var patches = []psupport.Patch{
	{
		File:   "resources/page/page_markup.go",
		After:  "func ExtractSummaryFromHTML(mt media.Type, input string, numWords int, isCJK bool) (result HtmlSummary) {",
		Insert: "\n\tif OracleHookSummary != nil {\n\t\tdefer func() { OracleHookSummary(\"auto\", mt, input, \"\", numWords, isCJK) }()\n\t}\n",
		Append: "\n// OracleHookSummary is set by the nh-page summary oracle.\nvar OracleHookSummary func(kind string, mt media.Type, input, divider string, numWords int, isCJK bool)\n",
	},
	{
		File:   "resources/page/page_markup.go",
		After:  "func ExtractSummaryFromHTMLWithDivider(mt media.Type, input, divider string) (result HtmlSummary) {",
		Insert: "\n\tif OracleHookSummary != nil {\n\t\tdefer func() { OracleHookSummary(\"manual\", mt, input, divider, 0, false) }()\n\t}\n",
	},
}

type call struct {
	kind     string
	mt       media.Type
	input    string
	divider  string
	numWords int
	isCJK    bool
}

var (
	mu        sync.Mutex
	recording bool
	calls     []call
)

func recordSummary(kind string, mt media.Type, input, divider string, numWords int, isCJK bool) {
	mu.Lock()
	defer mu.Unlock()
	if recording {
		calls = append(calls, call{kind, mt, input, divider, numWords, isCJK})
	}
}

func lh(l, h int) []int { return []int{l, h} }

// digest encodes a result string: itself when short, else its length and
// SHA-256 (the rendered pages are large and every variant repeats them).
func digest(s string) any {
	if len(s) <= 160 {
		return goval.Str(s)
	}
	sum := sha256.Sum256([]byte(s))
	return map[string]any{"len": len(s), "sha256": hex.EncodeToString(sum[:])}
}

func dump(r page.HtmlSummary) map[string]any {
	return map[string]any{
		"summary":               digest(r.Summary()),
		"contentWithoutSummary": digest(r.ContentWithoutSummary()),
		"content":               digest(r.Content()),
		"truncated":             r.Truncated(),
		"summaryLowHigh":        lh(r.SummaryLowHigh.Low, r.SummaryLowHigh.High),
		"summaryEndTag":         lh(r.SummaryEndTag.Low, r.SummaryEndTag.High),
		"wrapperStart":          lh(r.WrapperStart.Low, r.WrapperStart.High),
		"wrapperEnd":            lh(r.WrapperEnd.Low, r.WrapperEnd.High),
		"divider":               lh(r.Divider.Low, r.Divider.High),
	}
}

func run(c call) map[string]any {
	return goval.CallRaw(func() (any, error) {
		if c.kind == "manual" {
			return dump(page.ExtractSummaryFromHTMLWithDivider(c.mt, c.input, c.divider)), nil
		}
		return dump(page.ExtractSummaryFromHTML(c.mt, c.input, c.numWords, c.isCJK)), nil
	})
}

// table encodes calls, with the media types and the inputs in tables.
type table struct {
	types   []any
	index   map[string]int
	inputs  []any
	inIndex map[string]int
}

func (t *table) in(s string) int {
	if i, ok := t.inIndex[s]; ok {
		return i
	}
	t.inIndex[s] = len(t.inputs)
	t.inputs = append(t.inputs, goval.Str(s))
	return len(t.inputs) - 1
}

func (t *table) mt(m media.Type) int {
	k := m.Type + "|" + m.SubType
	if i, ok := t.index[k]; ok {
		return i
	}
	t.index[k] = len(t.types)
	t.types = append(t.types, psupport.MediaTypeDump(m))
	return len(t.types) - 1
}

func (t *table) encode(c call, src string) map[string]any {
	return map[string]any{
		"src":      src,
		"kind":     c.kind,
		"mt":       t.mt(c.mt),
		"input":    t.in(c.input),
		"divider":  goval.Str(c.divider),
		"numWords": c.numWords,
		"isCJK":    c.isCJK,
		"want":     run(c),
	}
}

func main() {
	root := flag.String("root", ".", "neohugo module root")
	out := flag.String("out", "crates/nh-page/tests/fixtures/summary", "output directory")
	flag.Parse()

	if !psupport.IsChild() {
		if err := psupport.RunOverlaid(*root, "./tools/go-oracle/nh-page/summary", patches, "zz_hooks_overlay.go", hookFile, []string{"-root", *root, "-out", *out}); err != nil {
			log.Fatal(err)
		}
		return
	}
	if installHooks == nil {
		log.Fatal("the recording hook is not installed")
	}
	installHooks(recordSummary)
	outDir := psupport.OutDir(*root, *out)

	sites, err := psupport.RepoSites(*root)
	if err != nil {
		log.Fatal(err)
	}
	// One synthetic site: they share their content.
	sites = append(sites, psupport.SyntheticSites()[0])

	var recs []call
	for _, s := range sites {
		mu.Lock()
		recording = true
		calls = nil
		mu.Unlock()
		if _, err := psupport.Build(s); err != nil {
			log.Fatal(err)
		}
		mu.Lock()
		recording = false
		recs = append(recs, calls...)
		calls = nil
		mu.Unlock()
	}

	// Deduplicate and sort the recorded calls.
	seen := map[call]bool{}
	var uniq []call
	for _, c := range recs {
		if !seen[c] {
			seen[c] = true
			uniq = append(uniq, c)
		}
	}
	sort.SliceStable(uniq, func(i, j int) bool {
		if uniq[i].input != uniq[j].input {
			return uniq[i].input < uniq[j].input
		}
		return uniq[i].kind+uniq[i].divider < uniq[j].kind+uniq[j].divider
	})

	t := &table{index: map[string]int{}, inIndex: map[string]int{}}
	ct := media.DefaultContentTypes
	var cases []map[string]any
	for _, c := range uniq {
		cases = append(cases, t.encode(c, "build"))
		// Variants of the same HTML.
		for _, n := range []int{0, 1, 5, 20, 1000} {
			v := c
			v.kind = "auto"
			v.numWords = n
			cases = append(cases, t.encode(v, "variant"))
		}
		v := c
		v.kind, v.numWords, v.isCJK = "auto", 30, !c.isCJK
		cases = append(cases, t.encode(v, "variant"))
		for _, mt := range []media.Type{ct.AsciiDoc, ct.ReStructuredText, ct.HTML} {
			v := c
			v.mt = mt
			v.kind, v.numWords = "auto", 25
			cases = append(cases, t.encode(v, "variant"))
			v.kind, v.divider = "manual", "</p>"
			cases = append(cases, t.encode(v, "variant"))
		}
		for _, d := range []string{"<p>", "</h2>", "\n\n", "HUGOMORE42"} {
			v := c
			v.kind, v.divider = "manual", d
			cases = append(cases, t.encode(v, "variant"))
		}
	}
	header := map[string]any{"types": t.types, "inputs": t.inputs}
	if err := goval.WriteCasesGz(filepath.Join(outDir, "build.json.gz"), header, cases); err != nil {
		log.Fatal(err)
	}
	log.Printf("summary: %d recorded calls (%d distinct), %d cases", len(recs), len(uniq), len(cases))

	adv := adversarial()
	t2 := &table{index: map[string]int{}, inIndex: map[string]int{}}
	var acases []map[string]any
	for _, c := range adv {
		acases = append(acases, t2.encode(c, "adv"))
	}
	if err := goval.WriteCasesGz(filepath.Join(outDir, "adversarial.json.gz"), map[string]any{"types": t2.types, "inputs": t2.inputs}, acases); err != nil {
		log.Fatal(err)
	}
	log.Printf("summary: %d adversarial cases", len(acases))
}

// adversarial returns synthetic inputs for both functions.
func adversarial() []call {
	ct := media.DefaultContentTypes
	mts := []media.Type{ct.Markdown, ct.HTML, ct.AsciiDoc, ct.ReStructuredText, ct.Pandoc, ct.EmacsOrgMode}
	const d = "HUGOMORE42"
	inputs := []string{
		"",
		"<p>one</p>",
		"<p>one two three</p>\n<p>four five six</p>\n<p>seven</p>",
		"<p>a</p>",
		"<p> a </p>",
		"<p>a b c d e f g h i j k l m n o p q r s t u v w x y z</p>",
		"<p>word word word　word</p>",
		"<p>中文内容测试。这是一个很长的段落。</p><p>第二段内容。</p>",
		"<p>日本語 の テキスト です</p>",
		"<p><a href=\"x\">link text</a> more <em>words</em> here</p>",
		"<p>x <b class='y'>bold</b> z</p><p>next</p>",
		"<div class=\"document\">\n<p>rst para one two</p>\n<p>three</p>\n</div>",
		"<div class=\"document\"><div class=\"paragraph\"><p>adoc</p></div></div>",
		"<div class=\"paragraph\">\n<p>one two three</p>\n</div>\n<div class=\"paragraph\">\n<p>four</p>\n</div>",
		"no paragraphs at all, just text",
		"<p>unclosed paragraph",
		"<h2>heading</h2><p>para one two</p>",
		"<pre><code>code block\nlines</code></pre><p>after code</p>",
		"<p>invalid \xff\xfe bytes here</p><p>\xc3</p>",
		// An invalid last byte: Go tests i+utf8.RuneLen(U+FFFD) (3) against the length.
		"<p>one two\xff</p><p>three four</p>", "<p>x\xff</p><p>y z</p>", "<p>ab \xe0\xb8</p><p>c</p>",
		"<p>tab\tseparated\twords</p>",
		"<p>trailing space </p>",
		"<p>" + strings.Repeat("w ", 100) + "</p><p>" + strings.Repeat("z ", 10) + "</p>",
		"<p>Intro text.</p>\n<p>" + d + "</p>\n<p>Rest of it.</p>",
		"<p>Intro text " + d + " continues here.</p>\n<p>More.</p>",
		"<p>" + d + "</p><p>after</p>",
		"<p>before</p><p>" + d + "</p>",
		d,
		"<p>a</p>" + d,
		d + "<p>a</p>",
		"<div>\n<p>x</p>\n" + d + "\n</div>",
		"<p><strong>" + d + "</strong></p>",
		"<p >x</p><p class=\"a\">" + d + "</p>\n",
		"<pre>" + d + "</pre>",
		"<div class=\"paragraph\"><p>a " + d + " b</p></div>",
		"<div class=\"document\">\n<p>rst</p>\n<p>" + d + "</p>\n<p>after</p>\n</div>",
		"<p>x</p>\n\n\n" + d + "\n\n<p>y</p>",
		"<p>a</p><p>b</p>" + d + "\n",
		"<ul>\n<li>item " + d + "</li>\n</ul>\n",
		"<p>é" + d + "é</p>",
		"<p>\xff" + d + "\xfe</p>",
		"<pé>x</pé>" + d,
		"<div>" + d + "</div>",
		"<p>a</p>\n<div>b</div>\n" + d + "<p>c</p>",
		"<p>a <p>nested</p> " + d + "</p>",
		"<p>x>y</p>" + d,
		"<p>>" + d,
		"<p" + d + ">",
	}
	var out []call
	for _, in := range inputs {
		for _, mt := range mts {
			for _, n := range []int{0, 1, 2, 3, 5, 10, 70} {
				out = append(out, call{kind: "auto", mt: mt, input: in, numWords: n})
				out = append(out, call{kind: "auto", mt: mt, input: in, numWords: n, isCJK: true})
			}
			for _, div := range []string{d, "<p>", "</p>", "x", "", "HUGOMORE"} {
				out = append(out, call{kind: "manual", mt: mt, input: in, divider: div})
			}
		}
	}
	return out
}
