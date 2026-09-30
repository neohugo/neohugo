// Command hooks is the Go oracle for the render hooks of crates/nh-markup
// (Wave B task T06): neohugo's goldmark converter renders the corpus with
// recording hook renderers for every hook type (link, image, heading,
// blockquote, table, code block) that dump every hook context (Destination,
// Title, Text, PlainText, IsBlock, Ordinal, Anchor, Level, Page, PageInner,
// Attributes, Options, AttributesSlice, OptionsSlice, Position through an
// ElementPositionResolver, table THead/TBody cells with Alignment,
// blockquote Type/AlertType/AlertTitle/AlertSign, code block Type/Inner)
// and write markers into the output, so the HTML with hooks is compared too.
//
// Documents: the whole corpus with the seeksnack config; the docs/rust-port
// and adversarial documents also with the "ascii" config; the adversarial
// documents with every config. Each adversarial document is also rendered
// wrapped in hugocontext.Wrap markers (as .RenderShortcodes produces them,
// pid 7, whose DocumentLookup value is "inner:7"), and a few documents make
// the hooks fail (link destination "hook-error", code block language
// "errlang", blockquote text "BQERR") or lack a code block renderer
// (language "nohook").
//
//	go run ./tools/go-oracle/nh-markup/hooks [-root .] [-out rust/testdata/oracle/markup/hooks]
package main

import (
	"flag"
	"path/filepath"
	"strconv"
	"strings"

	"github.com/neohugo/neohugo/markup/converter"
	"github.com/neohugo/neohugo/markup/goldmark/hugocontext"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-markup/mdoracle"
)

type result struct {
	Doc  int    `json:"doc"`
	Cfg  int    `json:"cfg"`
	Wrap bool   `json:"wrap"`
	TOC  bool   `json:"toc"`
	Page string `json:"page"`
	mdoracle.Result
	Records []mdoracle.B `json:"records"`
}

type fixture struct {
	Configs []mdoracle.Config `json:"configs"`
	Docs    []mdoracle.Doc    `json:"docs"`
	Results []result          `json:"results"`
}

// errorDocs make the hooks fail or miss a renderer.
var errorDocs = []mdoracle.Doc{
	{Name: "hooks-error/link", Src: mdoracle.B("Before [text](hook-error) after [ok](x)\n")},
	{Name: "hooks-error/codeblock", Src: mdoracle.B("```errlang {.c}\ncode\n```\n")},
	{Name: "hooks-error/nohook", Src: mdoracle.B("# H\n\n```nohook\ncode\n```\n")},
	{Name: "hooks-error/blockquote", Src: mdoracle.B("> quote BQERR\n\n> ok\n")},
	{Name: "hooks-error/badattrs", Src: mdoracle.B("```go {bad attrs here\n```\n")},
	{Name: "hooks-error/badattrs2", Src: mdoracle.B("```go {=x}\ncode\n```\n")},
	{Name: "hooks-error/nullattr", Src: mdoracle.B("## H {a=null}\n")},
	{Name: "hooks-error/objattr", Src: mdoracle.B("## H {a={b=1}}\n")},
	{Name: "hooks-error/idnum", Src: mdoracle.B("## H {id=5}\n")},
}

func lookup(pid uint64) any {
	if pid == 9 {
		return nil
	}
	return "inner:" + strconv.FormatUint(pid, 10)
}

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "rust/testdata/oracle/markup/hooks", "output directory")
	flag.Parse()

	docs := append(mdoracle.LoadCorpus(*root), errorDocs...)
	var fx fixture
	fx.Configs = mdoracle.Configs
	fx.Docs = docs

	for ci, c := range mdoracle.Configs {
		p, _ := mdoracle.NewProvider(c)
		for di, d := range docs {
			adversarial := strings.HasPrefix(d.Name, "adversarial/") || strings.HasPrefix(d.Name, "hooks-error/")
			switch c.Name {
			case "seeksnack":
			case "ascii":
				if !adversarial && !strings.HasPrefix(d.Name, "docs/rust-port/") {
					continue
				}
			default:
				if !adversarial {
					continue
				}
			}
			variants := []bool{false}
			if adversarial {
				variants = append(variants, true)
			}
			for _, wrap := range variants {
				page := "page:" + d.Name
				src := []byte(d.Src)
				if wrap {
					src = []byte("Intro *text*\n\n" + hugocontext.Wrap(src, 7) + "\nOutro [l](x)\n\n" + hugocontext.Wrap([]byte("## Nil lookup\n\n![i](n.png)\n"), 9))
				}
				conv, err := p.New(converter.DocumentContext{
					Document:       page,
					DocumentLookup: lookup,
					DocumentID:     d.Name,
					Filename:       d.Name,
				})
				if err != nil {
					panic(err)
				}
				rec := &mdoracle.Recorder{}
				res, _ := mdoracle.Convert(conv, converter.RenderContext{
					Src:         src,
					RenderTOC:   !wrap,
					GetRenderer: mdoracle.RecordingRenderers(rec),
				})
				r := result{Doc: di, Cfg: ci, Wrap: wrap, TOC: !wrap, Page: page, Result: res, Records: []mdoracle.B{}}
				for _, x := range rec.Records {
					r.Records = append(r.Records, x)
				}
				fx.Results = append(fx.Results, r)
			}
		}
	}
	mdoracle.WriteGz(filepath.Join(*out, "hooks.json.gz"), fx)
}
