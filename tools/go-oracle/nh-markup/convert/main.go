// Command convert is the Go oracle for crates/nh-markup (Wave B task T06):
// neohugo's goldmark converter (markup/goldmark, the real Provider) renders
// every document of the corpus (tools/go-oracle/nh-markup/mdoracle) with
// every configuration of mdoracle.Configs, without link, image, heading and
// blockquote hooks (the default renderers), with Go replicas of the table
// hook (embedded _markup/render-table.html) and of a code block hook (Hugo
// always has both), and with RenderTOC on. It records the HTML (or the
// error or panic) and the table of contents: the Fragments (headings tree,
// identifiers, headings map) and ToHTML for the configured TOC levels and
// two fixed level pairs. It also records SanitizeAnchorName and the decoded
// markup config of each configuration.
//
//	go run ./tools/go-oracle/nh-markup/convert [-root .] [-out rust/testdata/oracle/markup/convert]
package main

import (
	"encoding/json"
	"flag"
	"path/filepath"
	"strings"

	"github.com/neohugo/neohugo/markup/converter"
	"github.com/neohugo/neohugo/markup/tableofcontents"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-markup/mdoracle"
)

type tocDump struct {
	Present     bool          `json:"present"`
	Headings    []headingDump `json:"headings"`
	Identifiers []mdoracle.B  `json:"identifiers"`
	MapKeys     []mdoracle.B  `json:"map_keys"`
	MapValues   []headingDump `json:"map_values"`
	HTML        []mdoracle.B  `json:"html"`
}

type headingDump struct {
	ID       mdoracle.B    `json:"id"`
	Level    int           `json:"level"`
	Title    mdoracle.B    `json:"title"`
	Headings []headingDump `json:"headings"`
}

func dumpHeadings(h tableofcontents.Headings) []headingDump {
	out := []headingDump{}
	for _, x := range h {
		out = append(out, dumpHeading(x))
	}
	return out
}

func dumpHeading(x *tableofcontents.Heading) headingDump {
	return headingDump{ID: mdoracle.B(x.ID), Level: x.Level, Title: mdoracle.B(x.Title), Headings: dumpHeadings(x.Headings)}
}

// tocLevels are the ToHTML calls recorded for every document (the config's
// own levels come first).
var tocLevels = [][2]int{{1, -1}, {2, 3}, {3, 4}, {0, 2}}

func dumpTOC(f *tableofcontents.Fragments, start, end int, ordered bool) tocDump {
	if f == nil {
		return tocDump{}
	}
	d := tocDump{Present: true, Headings: dumpHeadings(f.Headings), Identifiers: []mdoracle.B{}}
	for _, id := range f.Identifiers {
		d.Identifiers = append(d.Identifiers, mdoracle.B(id))
	}
	for _, k := range mdoracle.SortedKeys(f.HeadingsMap) {
		d.MapKeys = append(d.MapKeys, mdoracle.B(k))
		d.MapValues = append(d.MapValues, dumpHeading(f.HeadingsMap[k]))
	}
	levels := append([][2]int{{start, end}}, tocLevels...)
	for i, l := range levels {
		o := ordered
		if i > 0 {
			o = i%2 == 0
		}
		h, err := f.ToHTML(l[0], l[1], o)
		if err != nil {
			panic(err)
		}
		d.HTML = append(d.HTML, mdoracle.B(h))
	}
	return d
}

// result is one conversion. To keep the fixture small, an HTML (TOC) equal
// to the one of an earlier config for the same document is stored as the
// index of that config in HTMLSame (TOCSame).
type result struct {
	Doc int `json:"doc"`
	Cfg int `json:"cfg"`
	mdoracle.Result
	HTMLSame *int    `json:"html_same,omitempty"`
	TOC      tocDump `json:"toc"`
	TOCSame  *int    `json:"toc_same,omitempty"`
}

type configDump struct {
	mdoracle.Config
	Dump string `json:"dump"`
}

type fixture struct {
	Configs []configDump   `json:"configs"`
	Docs    []mdoracle.Doc `json:"docs"`
	Results []result       `json:"results"`
	// Deep holds the deeply nested documents of deepDocs rendered with the
	// seeksnack config (the Rust test regenerates the inputs and renders them
	// on a 2 MiB stack).
	Deep []mdoracle.Result `json:"deep"`
}

// deepDocs is mirrored by the Rust test (tests/convert.rs).
func deepDocs() []string {
	return []string{
		strings.Repeat("> ", 3000) + "deep *quote*\n",
		strings.Repeat("- ", 3000) + "item\n",
		strings.Repeat("*", 5000) + "x" + strings.Repeat("*", 5000) + "\n",
		"## " + strings.Repeat("**a ", 2000) + strings.Repeat("**", 2000) + "\n",
		strings.Repeat("![", 1000) + "x" + strings.Repeat("](i)", 1000) + "\n",
		strings.Repeat("[", 3000) + "x" + strings.Repeat("](u)", 3000) + "\n",
		"# " + strings.Repeat("[a ", 1500) + strings.Repeat("](u)", 1500) + "\n\n" + strings.Repeat("> ", 500) + "| a |\n" + strings.Repeat("> ", 500) + "|---|\n",
	}
}

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "rust/testdata/oracle/markup/convert", "output directory")
	flag.Parse()

	docs := mdoracle.LoadCorpus(*root)
	var fx fixture
	fx.Docs = docs
	seenHTML := make([]map[string]int, len(docs))
	seenTOC := make([]map[string]int, len(docs))
	for i := range docs {
		seenHTML[i] = map[string]int{}
		seenTOC[i] = map[string]int{}
	}
	for ci, c := range mdoracle.Configs {
		p, m := mdoracle.NewProvider(c)
		fx.Configs = append(fx.Configs, configDump{Config: c, Dump: mdoracle.DumpMarkupConfig(m)})
		for di, d := range docs {
			conv, err := p.New(converter.DocumentContext{DocumentID: d.Name, Filename: d.Name})
			if err != nil {
				panic(err)
			}
			res, rr := mdoracle.Convert(conv, converter.RenderContext{
				Src:         d.Src,
				RenderTOC:   true,
				GetRenderer: mdoracle.ReplicaRenderers,
			})
			r := result{Doc: di, Cfg: ci, Result: res}
			if rr != nil {
				r.TOC = dumpTOC(rr.(converter.TableOfContentsProvider).TableOfContents(),
					m.TableOfContents.StartLevel, m.TableOfContents.EndLevel, m.TableOfContents.Ordered)
			}
			if r.Err == "" && r.Panic == "" {
				if first, ok := seenHTML[di][string(r.HTML)]; ok {
					r.HTML = nil
					r.HTMLSame = &first
				} else {
					seenHTML[di][string(r.HTML)] = ci
				}
				key, err := json.Marshal(r.TOC)
				if err != nil {
					panic(err)
				}
				if first, ok := seenTOC[di][string(key)]; ok {
					r.TOC = tocDump{}
					r.TOCSame = &first
				} else {
					seenTOC[di][string(key)] = ci
				}
			}
			fx.Results = append(fx.Results, r)
		}
	}
	p, _ := mdoracle.NewProvider(mdoracle.ConfigByName("seeksnack"))
	for i, d := range deepDocs() {
		conv, err := p.New(converter.DocumentContext{DocumentID: "deep", Filename: "deep"})
		if err != nil {
			panic(err)
		}
		res, _ := mdoracle.Convert(conv, converter.RenderContext{
			Src:         []byte(d),
			RenderTOC:   i%2 == 0,
			GetRenderer: mdoracle.ReplicaRenderers,
		})
		fx.Deep = append(fx.Deep, res)
	}
	mdoracle.WriteGz(filepath.Join(*out, "convert.json.gz"), fx)
}
