// Command general is the Go oracle for the free functions of helpers
// (general.go, path.go, content.go) and helpers.ProcessingStatsTable in
// crates/nh-helpers (Wave B task T08).
//
//	go run ./tools/go-oracle/nh-helpers/general [-root .] [-out crates/nh-helpers/tests/fixtures/general]
//
// Inputs: the nh-common corpus (this repository's titles, terms, headings and
// file names plus adversarial strings), every content path and file body of
// docs/content and hugolib/testsite/content, string lists built from the
// corpus, seeded random byte streams for ReaderContains (NUL bytes included,
// to reach Go's stale-buffer search), byte counts around every unit boundary,
// synthetic TOC HTML, and ProcessingStats tables for 0-4 languages with
// ASCII, Thai, CJK, emoji and long names and counters up to 2^64-1.
//
// Output: general.json.gz. Nothing here depends on the platform.
package main

import (
	"bytes"
	"flag"
	"fmt"
	"io/fs"
	"log"
	"math"
	"math/rand"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"unicode/utf8"

	"github.com/mattn/go-runewidth"
	"github.com/neohugo/neohugo/helpers"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/corpus"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-helpers/hsupport"
	"github.com/rivo/uniseg"
)

var titleStyles = []string{"ap", "AP", "Chicago", "chicago", "go", "Go", "firstupper", "none", "", "unknown"}

var dottedExtra = []string{
	"", ".", "./", "/", "//", "a", "a/", "/a", "/a/", "a/b", "a/b/", "/a/b/c.html", "a.b/c",
	"a/b.c/d", "a/b/c.toolong", "a/b/c.toolon", "a/b/c.x", "a/b/c.", "a/.b", "../a", "a/../../b",
	"a\\b\\c.md", "x/y/z.ภาษา", "x/y/z.ภาษาไทยมาก", "x/y.a\nb", "a/b/c.md\n", "p/q/.hidden",
	"posts/my post/index.md", "/sect/doc1.md", "sect/doc1", "sect/", "sect/sub/", "a.b.c/d.e.f",
}

var tocInputs = []string{
	"", "<p>no toc</p>", "<nav>\n</nav>\n\n<p>x</p>", "<nav>\n<ul>\n<li><a href=\"#a\">A</a></li>\n</ul>\n</nav>\n<p>body</p>",
	"<p>pre</p><nav>\n<ul>\n<li><a href=\"#a\">A</a>\n<ul>\n<li><a href=\"#b\">B</a></li>\n</ul></li>\n</ul>\n</nav><p>post</p>",
	"<nav>\n<ul>\n<li>no link</li>\n</ul>\n</nav>", "<nav>other</nav>", "<nav>\n<ul>" + strings.Repeat(" ", 80) + "<li><a href=\"#x\">X</a></li>\n</ul>\n</nav>",
	"<nav>\n<ul>\n<li><a href=\"#a\">A</a></li>\n</ul>\n</nav>\n<nav>\n<ul>\n<li><a href=\"#b\">B</a></li>\n</ul>\n</nav>",
	"<nav><ul><li><a href=\"#a\">A</a></li></ul></nav>",
}

var byteCounts = []uint64{
	0, 1, 2, 1023, 1024, 1025, 1 << 20, 1<<20 + 1, 1 << 30, 1<<30 + 1, 5 << 30, 1 << 40,
	math.MaxUint64, math.MaxUint64 - 1, math.MaxUint64 - 1023, math.MaxUint64 - 1024,
	math.MaxUint64 - (1 << 20), math.MaxUint64 - (1 << 20) + 1, math.MaxUint64 - (1 << 30),
	math.MaxUint64 - (1 << 30) + 1, 1 << 63, 123456789, 999999999999,
}

func contentFiles(root string) (paths []string, bodies []string, err error) {
	for _, r := range []string{"docs/content", "hugolib/testsite/content"} {
		base := filepath.Join(root, r)
		err = filepath.WalkDir(base, func(p string, d fs.DirEntry, err error) error {
			if err != nil {
				return err
			}
			rel, err := filepath.Rel(base, p)
			if err != nil {
				return err
			}
			paths = append(paths, filepath.ToSlash(rel))
			if d.IsDir() {
				return nil
			}
			if strings.HasSuffix(p, ".md") {
				b, err := os.ReadFile(p)
				if err != nil {
					return err
				}
				bodies = append(bodies, string(b))
			}
			return nil
		})
		if err != nil {
			return nil, nil, err
		}
	}
	return paths, bodies, nil
}

func strList(v []string) []any {
	out := make([]any, len(v))
	for i, s := range v {
		out[i] = hsupport.Str(s)
	}
	return out
}

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "crates/nh-helpers/tests/fixtures/general", "output directory")
	flag.Parse()

	strs, err := corpus.Strings(*root)
	if err != nil {
		log.Fatal(err)
	}
	paths, bodies, err := contentFiles(*root)
	if err != nil {
		log.Fatal(err)
	}
	var valid []string
	for _, s := range strs {
		if utf8.ValidString(s) {
			valid = append(valid, s)
		}
	}

	var cases []map[string]any
	add := func(fn string, in any, r any) {
		cases = append(cases, map[string]any{"fn": fn, "in": in, "r": r})
	}

	for _, s := range valid {
		add("FirstUpper", s, hsupport.Call(func() string { return helpers.FirstUpper(s) }))
		add("MakeTitle", s, hsupport.Call(func() string { return helpers.MakeTitle(s) }))
		var titles []any
		for _, st := range titleStyles {
			f := helpers.GetTitleFunc(st)
			titles = append(titles, hsupport.Call(func() string { return f(s) }))
		}
		add("GetTitleFunc", s, titles)
	}
	for _, s := range strs {
		add("TotalWords", hsupport.Str(s), helpers.TotalWords(s))
	}
	for _, b := range bodies {
		add("TotalWords", hsupport.Str(b), helpers.TotalWords(b))
	}
	for _, p := range append(append([]string{}, dottedExtra...), paths...) {
		add("GetDottedRelativePath", p, hsupport.Call(func() string { return helpers.GetDottedRelativePath(p) }))
		rel, err := helpers.MakePathRelative(p, "sect/", "/a/", "docs")
		add("MakePathRelative", p, []any{rel, hsupport.Err(err)})
	}

	// String lists.
	rng := rand.New(rand.NewSource(8))
	var lists [][]string
	lists = append(lists, nil, []string{}, []string{"a"}, []string{"a", "a"}, []string{"b", "a", "b", "a", "c"},
		[]string{"", "", "x"}, []string{"Z", "a", "A", "z"})
	for range 300 {
		n := rng.Intn(8)
		l := make([]string, n)
		for j := range l {
			l[j] = valid[rng.Intn(len(valid))]
			if rng.Intn(3) == 0 && j > 0 {
				l[j] = l[rng.Intn(j)]
			}
		}
		lists = append(lists, l)
	}
	for _, l := range lists {
		in := strList(l)
		add("UniqueStrings", in, strList(helpers.UniqueStrings(l)))
		cp := append([]string(nil), l...)
		add("UniqueStringsReuse", in, strList(helpers.UniqueStringsReuse(cp)))
		cp = append([]string(nil), l...)
		us := helpers.UniqueStringsSorted(cp)
		var usv any
		if us == nil {
			usv = nil
		} else {
			usv = strList(us)
		}
		add("UniqueStringsSorted", in, usv)
		lower := helpers.SliceToLower(l)
		var lv any
		if lower == nil {
			lv = nil
		} else {
			lv = strList(lower)
		}
		add("SliceToLower", map[string]any{"nil": l == nil, "l": in}, lv)
		for _, c := range []string{"", "or", "และ"} {
			add("StringSliceToList", []any{in, c}, hsupport.Str(helpers.StringSliceToList(l, c)))
		}
		if len(l) > 0 {
			pre := l[:rng.Intn(len(l)+1)]
			add("HasStringsPrefix", []any{in, strList(pre)}, helpers.HasStringsPrefix(l, pre))
			suf := l[rng.Intn(len(l)+1):]
			add("HasStringsSuffix", []any{in, strList(suf)}, helpers.HasStringsSuffix(l, suf))
			other := []string{l[0], "zz"}
			add("HasStringsPrefix", []any{in, strList(other)}, helpers.HasStringsPrefix(l, other))
			add("HasStringsSuffix", []any{in, strList(other)}, helpers.HasStringsSuffix(l, other))
		}
		groups := helpers.ExtractAndGroupRootPaths(l)
		var gv any
		if groups != nil {
			var gs []any
			for _, g := range groups {
				var sl any
				if g.Slice != nil {
					sl = strList(g.Slice)
				}
				gs = append(gs, map[string]any{"name": hsupport.Str(g.Name), "slice": sl, "string": hsupport.Str(g.String())})
			}
			gv = gs
		}
		add("ExtractAndGroupRootPaths", in, gv)
		add("ExtractRootPaths", in, strList(helpers.ExtractRootPaths(l)))
	}
	// Path lists for the root path grouping.
	for range 200 {
		n := 1 + rng.Intn(6)
		l := make([]string, n)
		lead := rng.Intn(2) == 0
		for j := range l {
			p := paths[rng.Intn(len(paths))]
			if lead {
				p = "/" + p
			}
			l[j] = p
		}
		groups := helpers.ExtractAndGroupRootPaths(l)
		var gs []any
		for _, g := range groups {
			var sl any
			if g.Slice != nil {
				sl = strList(g.Slice)
			}
			gs = append(gs, map[string]any{"name": hsupport.Str(g.Name), "slice": sl, "string": hsupport.Str(g.String())})
		}
		add("ExtractAndGroupRootPaths", strList(l), gs)
		add("ExtractRootPaths", strList(l), strList(helpers.ExtractRootPaths(l)))
	}

	for _, bc := range byteCounts {
		add("FormatByteCount", fmt.Sprint(bc), helpers.FormatByteCount(bc))
	}
	for range 200 {
		bc := rng.Uint64() >> uint(rng.Intn(64))
		add("FormatByteCount", fmt.Sprint(bc), helpers.FormatByteCount(bc))
	}

	// ReaderContains.
	alphabet := []byte("ab\x00")
	for i := range 3000 {
		n := rng.Intn(40)
		b := make([]byte, n)
		for j := range b {
			b[j] = alphabet[rng.Intn(len(alphabet))]
		}
		m := rng.Intn(5)
		sub := make([]byte, m)
		for j := range sub {
			sub[j] = alphabet[rng.Intn(len(alphabet))]
		}
		if i%10 == 0 && n > 3 {
			k := rng.Intn(n - 2)
			sub = append([]byte(nil), b[k:k+1+rng.Intn(3)]...)
		}
		add("ReaderContains", []any{hsupport.Str(string(b)), hsupport.Str(string(sub))}, helpers.ReaderContains(bytes.NewReader(b), sub))
	}
	add("ReaderContains", nil, helpers.ReaderContains(nil, []byte("a")))
	for _, s := range []string{"", "a", strings.Repeat("xyz", 1000)} {
		add("ReaderToBytes", s, hsupport.Str(string(helpers.ReaderToBytes(strings.NewReader(s)))))
		add("ReaderToString", s, hsupport.Str(helpers.ReaderToString(strings.NewReader(s))))
	}

	for _, t := range tocInputs {
		nc, toc := helpers.ExtractTOC([]byte(t))
		var tv any
		if toc != nil {
			tv = hsupport.Str(string(toc))
		}
		add("ExtractTOC", t, []any{hsupport.Str(string(nc)), tv})
	}

	for _, r := range []rune{' ', '\t', '\n', '\r', '\v', '\f', 'a', 0x85, 0xa0, 0x3000} {
		add("IsWhitespace", int(r), helpers.IsWhitespace(r))
	}

	// ProcessingStatsTable.
	names := []string{"en", "th", "", "nn", "zh-cn", "ไทย", "日本語", "😀", "a very long language name with spaces and more more more more more more more more", "x_y.z",
		"pt-br", "PT_BR", "camelCaseName", "HTTPServer", "v1.2.3", "a.b", ".x.", "1.5", "  spaced  ", "tab\there", "two\nlines", "\x1b[31mred\x1b[0m",
		"\x1b]8;;http://x\x07link\x1b]8;;\x07", "👨‍👩‍👧 family", "🇹🇭🇯🇵", "e\u0301\u0302", "한국어", "ｆｕｌｌ", "ขนมไทยอร่อยมากมากมากมากมากมาก", "中文中文中文中文中文中文中文中文中文中文中文中文",
		"x", "__", "ß", "ǅungla", "ΣΊΣΥΦΟΣ", "123abc", "ABCdef", "   ", "\u200b", "a\u00adb"}
	for i := range 400 {
		n := i % 7
		if i < 7 {
			n = i
		}
		var stats []*helpers.ProcessingStats
		var in []any
		for range n {
			st := helpers.NewProcessingStats(names[rng.Intn(len(names))])
			vals := []*uint64{&st.Pages, &st.PaginatorPages, &st.Files, &st.Static, &st.ProcessedImages, &st.Aliases, &st.Cleaned}
			var vs []any
			for _, v := range vals {
				switch rng.Intn(4) {
				case 0:
					*v = 0
				case 1:
					*v = uint64(rng.Intn(100))
				case 2:
					*v = rng.Uint64() >> uint(rng.Intn(64))
				default:
					*v = uint64(rng.Intn(100000))
				}
				vs = append(vs, fmt.Sprint(*v))
			}
			stats = append(stats, st)
			in = append(in, map[string]any{"name": st.Name, "vals": vs})
		}
		var buf bytes.Buffer
		helpers.ProcessingStatsTable(&buf, stats...)
		add("ProcessingStatsTable", in, hsupport.Str(buf.String()))
	}

	sort.SliceStable(cases, func(i, j int) bool { return cases[i]["fn"].(string) < cases[j]["fn"].(string) })

	// go-runewidth v0.0.16 RuneWidth over every code point (run-length encoded) and uniseg
	// v0.2.0 grapheme clusters + StringWidth over the corpus and names (the tables behind
	// ProcessingStatsTable's column widths).
	widths := map[string]any{}
	for _, ea := range []bool{false, true} {
		c := runewidth.NewCondition()
		c.EastAsianWidth = ea
		var runs [][2]int
		last := -1
		for r := rune(-1); r <= 0x110000; r++ {
			w := c.RuneWidth(r)
			if w != last {
				runs = append(runs, [2]int{int(r), w})
				last = w
			}
		}
		widths[fmt.Sprint("runs_", ea)] = runs
	}
	var gcases []any
	gstrs := append(append([]string{}, valid...), names...)
	gstrs = append(gstrs, "a\r\nb", "\r\r\n\n", "\u1100\u1161\u11a8", "\u1100\u1100\uac00\u11a8", "\U0001F1F9\U0001F1ED\U0001F1EF",
		"\u0915\u094d\u0937", "\u0600\u0661", "\U0001F469\u200d\U0001F4bb", "a\u200db", "\u00a9\ufe0f", "x\u0903y", "\x00\x01", "")
	for _, s := range gstrs {
		g := uniseg.NewGraphemes(s)
		var pos [][2]int
		for g.Next() {
			a, b := g.Positions()
			pos = append(pos, [2]int{a, b})
		}
		c0 := runewidth.NewCondition()
		c0.EastAsianWidth = false
		c1 := runewidth.NewCondition()
		c1.EastAsianWidth = true
		gcases = append(gcases, []any{s, pos, c0.StringWidth(s), c1.StringWidth(s)})
	}
	widths["graphemes"] = gcases
	wp := filepath.Join(*out, "widths.json.gz")
	if err := hsupport.WriteGz(wp, widths, nil); err != nil {
		log.Fatal(err)
	}
	p := filepath.Join(*out, "general.json.gz")
	if err := hsupport.WriteGz(p, map[string]any{"titleStyles": titleStyles}, cases); err != nil {
		log.Fatal(err)
	}
	log.Printf("%s: %d cases", p, len(cases))
}
