// Command textmisc is the Go oracle for the small T02 packages of
// crates/nh-common: resources/kinds, hugofs/files, common/text,
// common/hstrings and common/hugio's HasBytesWriter.
//
//	go run ./tools/go-oracle/nh-common/textmisc [-root .] [-out crates/nh-common/tests/fixtures/textmisc]
//
// Inputs: the string corpus of ../corpus, the kind and component names in
// case variants, this repository's docs file paths, text with every mix of
// \r and \n, and HasBytesWriter streams (random bytes around the patterns
// Hugo uses, cut into random writes) from a fixed seed. Output:
// textmisc.json.gz. Nothing here depends on the platform: linux/amd64 and
// linux/arm64 (qemu) produce the same bytes.
package main

import (
	"flag"
	"fmt"
	"io/fs"
	"log"
	"math/rand"
	"os"
	"path/filepath"
	"strings"

	"github.com/neohugo/neohugo/common/hstrings"
	"github.com/neohugo/neohugo/common/hugio"
	"github.com/neohugo/neohugo/common/text"
	"github.com/neohugo/neohugo/hugofs/files"
	"github.com/neohugo/neohugo/resources/kinds"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/corpus"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
)

func str(s string) any { return goval.Str(s) }

func try(f func() any) (out any) {
	defer func() {
		if r := recover(); r != nil {
			out = map[string]any{"panic": fmt.Sprint(r)}
		}
	}()
	return f()
}

var kindNames = []string{
	"page", "home", "section", "taxonomy", "term", "taxonomyterm", "rss", "sitemap",
	"robotstxt", "404", "sitemapindex", "temporary", "Page", "HOME", "Taxonomy", "TAXONOMYTERM",
	"TaxonomyTerm", "RSS", "RobotsTXT", "SEction", "ſection", "K", "", " page", "page ",
}

var componentNames = []string{
	"archetypes", "static", "layouts", "content", "data", "assets", "i18n", "resources",
	"foo", "", "Content", "content/", "/content", "contentx", "assetsfoo", "i18n/en.yaml",
	"/layouts/_default/single.html", "layouts2/x", "static-files", "/", "//content",
}

var textInputs = []string{
	"", "\n", "\r", "\r\n", "\n\r", "a", "a\n", "a\r\n", "\nA\n", "A\r\n", "a\n\n", "a\r\r\n\n",
	"line 1\nline 2\n\nline 3", "line 1\nline 2\n\nline 3\n", "\n\n\n", "a\nb", "é\n", "\xff\n",
	"a\n\xff", " \n ", "a\r", "x\ny\rz\r\nw",
}

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "crates/nh-common/tests/fixtures/textmisc", "output directory")
	flag.Parse()

	// text.Position.String colours its output when stdout is a terminal.
	devnull, err := os.Open(os.DevNull)
	if err != nil {
		log.Fatal(err)
	}
	os.Stdout = devnull

	strs, err := corpus.Strings(*root)
	if err != nil {
		log.Fatal(err)
	}

	var cases []map[string]any

	// kinds
	for _, s := range append(append([]string{}, kindNames...), strs...) {
		cases = append(cases, map[string]any{
			"kinds":                       str(s),
			"GetKindMain":                 str(kinds.GetKindMain(s)),
			"GetKindAny":                  str(kinds.GetKindAny(s)),
			"IsBranch":                    kinds.IsBranch(s),
			"IsDeprecatedAndReplacedWith": str(kinds.IsDeprecatedAndReplacedWith(s)),
		})
	}

	// files
	fileNames := append([]string{}, componentNames...)
	docs := filepath.Join(*root, "docs")
	err = filepath.WalkDir(docs, func(p string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		rel, err := filepath.Rel(docs, p)
		if err != nil {
			return err
		}
		if strings.HasPrefix(rel, "rust-port") || strings.Count(rel, "/") > 1 {
			return nil
		}
		fileNames = append(fileNames, filepath.ToSlash(rel), "/"+filepath.ToSlash(rel))
		return nil
	})
	if err != nil {
		log.Fatal(err)
	}
	for _, s := range fileNames {
		cases = append(cases, map[string]any{
			"files":                  str(s),
			"ResolveComponentFolder": str(files.ResolveComponentFolder(s)),
			"IsComponentFolder":      files.IsComponentFolder(s),
			"IsContentDataExt":       files.IsContentDataExt(s),
		})
	}

	// text
	for _, s := range append(append([]string{}, textInputs...), strs...) {
		var lines []any
		text.VisitLinesAfter(s, func(line string) { lines = append(lines, str(line)) })
		cases = append(cases, map[string]any{
			"text":            str(s),
			"Chomp":           str(text.Chomp(s)),
			"Puts":            str(text.Puts(s)),
			"VisitLinesAfter": lines,
		})
	}
	for _, p := range []text.Position{
		{}, {Filename: "/my/file.txt", LineNumber: 12, ColumnNumber: 13, Offset: 14},
		{Filename: "a%sb", LineNumber: -1, ColumnNumber: 0, Offset: -1},
		{Filename: "ไทย.md", LineNumber: 1, ColumnNumber: 1},
	} {
		cases = append(cases, map[string]any{
			"position": []any{str(p.Filename), p.LineNumber, p.ColumnNumber, p.Offset},
			"String":   str(p.String()),
			"IsValid":  p.IsValid(),
		})
	}

	// hstrings: EqualFold between neighbours and case variants.
	for i, s := range strs {
		t := strs[(i+1)%len(strs)]
		for _, o := range []string{strings.ToUpper(s), strings.ToTitle(s), t} {
			cases = append(cases, map[string]any{
				"fold":            []any{str(s), str(o), str(t)},
				"EqualFold":       hstrings.StringEqualFold(s).EqualFold(o),
				"Eq":              hstrings.StringEqualFold(s).Eq(o),
				"InSlicEqualFold": hstrings.InSlicEqualFold([]string{t, o}, s),
				"InSlice":         hstrings.InSlice([]string{t, o}, s),
			})
		}
	}

	// hugio.HasBytesWriter
	r := rand.New(rand.NewSource(20260928))
	patternSets := [][]string{
		{"__foo"}, {"__h_pp_l1"}, {"__h_pp_l1", "__h_pp_l2"}, {"ab", "ba"}, {"a"},
		{"__hdeps_links_start__", "__hdeps_links_end__"}, {"é"}, {"\x00"},
	}
	chunks := []string{"ab cfo", "abc __f", "oo bar", "__foo", "__h_pp_l1", "_h_pp_l", "__h", "pp_l2",
		"__hdeps_links_start__", "__hdeps_links_end__", "x", "é", "\x00", "a", "b", " ", "\n"}
	for i := 0; i < 600; i++ {
		ps := patternSets[r.Intn(len(patternSets))]
		var stream strings.Builder
		for j, n := 0, r.Intn(40); j < n; j++ {
			stream.WriteString(chunks[r.Intn(len(chunks))])
		}
		s := stream.String()
		// Random write boundaries.
		var cuts []int
		for pos := 0; pos < len(s); {
			n := 1 + r.Intn(12)
			if pos+n > len(s) {
				n = len(s) - pos
			}
			cuts = append(cuts, n)
			pos += n
		}
		h := &hugio.HasBytesWriter{}
		for _, p := range ps {
			h.Patterns = append(h.Patterns, &hugio.HasBytesPattern{Pattern: []byte(p)})
		}
		res := try(func() any {
			pos := 0
			for _, n := range cuts {
				if _, err := h.Write([]byte(s[pos : pos+n])); err != nil {
					return err.Error()
				}
				pos += n
			}
			var m []any
			for _, p := range h.Patterns {
				m = append(m, p.Match)
			}
			return m
		})
		var pss []any
		for _, p := range ps {
			pss = append(pss, str(p))
		}
		cases = append(cases, map[string]any{
			"hasBytes": pss,
			"stream":   str(s),
			"writes":   cuts,
			"matched":  res,
		})
	}

	header := map[string]any{}
	p := filepath.Join(*out, "textmisc.json.gz")
	if err := goval.WriteCasesGz(p, header, cases); err != nil {
		log.Fatal(err)
	}
	log.Printf("%s: %d cases", p, len(cases))
}
