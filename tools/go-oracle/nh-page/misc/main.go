// Command misc is the Go oracle for the smaller parts of resources/page and
// resources/page/pagemeta in crates/nh-page (Wave B task T11):
// DecodeCascadeConfig (cascade config decoding, merging, errors and the
// logged pattern warning), PageMatcher.Matches, NewOutputFormat and
// OutputFormats.Get, MarkupToMediaType, PageConfig.Init/Compile and
// NamedPageMetaValue (on page.NopPage).
//
//	go run ./tools/go-oracle/nh-page/misc [-out crates/nh-page/tests/fixtures/misc]
//
// Output: misc.json.gz. Nothing here depends on the platform.
package main

import (
	"flag"
	"io"
	"log"
	"path/filepath"
	"sort"

	"github.com/bep/logg"
	"github.com/neohugo/neohugo/common/loggers"
	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/common/neohugo"
	"github.com/neohugo/neohugo/media"
	"github.com/neohugo/neohugo/output"
	"github.com/neohugo/neohugo/resources/page"
	"github.com/neohugo/neohugo/resources/page/pagemeta"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-page/psupport"
)

// testSite answers Hugo().Environment.
type testSite struct {
	page.Site
	env string
}

func (s testSite) Hugo() neohugo.HugoInfo { return neohugo.HugoInfo{Environment: s.env} }

// basePage is embedded under its own name (page.Page has a Page method).
type basePage = page.Page

// testPage answers what PageMatcher.Matches reads.
type testPage struct {
	basePage
	kind, lang, path, env string
}

func (p testPage) Kind() string    { return p.kind }
func (p testPage) Lang() string    { return p.lang }
func (p testPage) Path() string    { return p.path }
func (p testPage) Site() page.Site { return testSite{env: p.env} }

func matcherDump(m page.PageMatcher) map[string]any {
	return map[string]any{"path": m.Path, "kind": m.Kind, "lang": m.Lang, "environment": m.Environment}
}

var cascadeInputs = []any{
	nil,
	maps.Params{"params": maps.Params{"a": 1}, "_target": maps.Params{"kind": "page"}},
	[]map[string]any{
		{"params": map[string]any{"a": "av"}, "target": map[string]any{"kind": "page", "Environment": "production"}},
		{"params": map[string]any{"b": "bv"}, "target": map[string]any{"kind": "page"}},
	},
	[]map[string]any{
		{"params": map[string]any{"A": "1", "Nested": map[string]any{"X": 1}}, "_target": map[string]any{"path": "/Blog/**", "lang": "en"}},
		{"title": "T", "Draft": true, "_target": map[string]any{"kind": "{home,section}"}},
		{"params": map[string]any{"A": "2", "b": 3}, "_target": map[string]any{"path": "/Blog/**", "lang": "en"}, "weight": 5},
	},
	[]any{
		map[string]any{"params": map[string]any{"x": 1}},
		map[string]any{"params": map[string]any{"y": 2}},
	},
	[]map[string]any{{"kind": "page", "params": map[string]any{"a": 1}}},
	[]map[string]any{{"params": map[string]any{"a": 1}, "target": map[string]any{"kind": "bogus"}}},
	[]map[string]any{{"params": map[string]any{"a": 1}, "target": map[string]any{"kind": "[page"}}},
	[]map[string]any{{"params": map[string]any{"a": 1}, "target": map[string]any{"path": "/blog/*.md"}}},
	[]map[string]any{{"params": "notamap", "target": map[string]any{"path": "/a"}}},
	[]map[string]any{{"params": map[string]any{"a": 1}, "target": map[string]any{"environment": "{production,development}", "path": "C:\\x"}}},
	[]map[string]any{{"params": map[string]any{"a": 1}, "target": "notamap"}},
	[]map[string]any{{"params": map[string]any{"a": 1}, "target": map[string]any{"kind": 42}}},
	"notacascade",
	[]map[string]any{{"path": "/x"}},
	[]map[string]any{{"lang": "en"}},
	[]map[string]any{{"cascade": map[string]any{"x": 1}, "outputs": []any{"html"}}},
	[]map[string]any{{}},
}

var matchers = []page.PageMatcher{
	{}, {Kind: "page"}, {Kind: "{home,section}"}, {Kind: "[bad"}, {Lang: "en"}, {Lang: "{en,th}"},
	{Path: "/blog/**"}, {Path: "/blog"}, {Path: "/blog/*"}, {Path: "**"}, {Path: "/BLOG/**"},
	{Path: "{/blog/**,/docs/**}"}, {Path: "["}, {Environment: "production"}, {Environment: "dev*"},
	{Kind: "page", Lang: "th", Path: "/blog/**", Environment: "production"},
}

var testPages = []testPage{
	{kind: "page", lang: "en", path: "/blog/post", env: "production"},
	{kind: "page", lang: "th", path: "/blog/sub/deep", env: "development"},
	{kind: "section", lang: "en", path: "/blog", env: "production"},
	{kind: "home", lang: "en", path: "/", env: "production"},
	{kind: "home", lang: "en", path: "", env: "production"},
	{kind: "term", lang: "th", path: "/tags/go", env: "staging"},
	{kind: "page", lang: "en", path: "/Docs/Install", env: "production"},
	{kind: "page", lang: "en", path: "docs/rel", env: "production"},
	{kind: "taxonomy", lang: "nn", path: "/categories", env: ""},
}

var markups = []string{
	"md", "markdown", "MD", "Markdown", "html", "htm", "HTML", "goldmark", "asciidoc", "asciidocext",
	"adoc", "ad", "rst", "org", "pandoc", "pdc", "txt", "text", "unknown", "", "json", "text/html",
	"text/markdown", "markdown+x", "mmark", "emacs-org-mode",
}

func main() {
	out := flag.String("out", "crates/nh-page/tests/fixtures/misc", "output directory")
	flag.Parse()

	var cases []map[string]any
	add := func(c map[string]any) { cases = append(cases, c) }

	for _, in := range cascadeInputs {
		// Encoded before the call: Go mutates nested maps of the input in place
		// (PrepareParams), after hashing it.
		inEnc := goval.Encode(in)
		logger := loggers.New(loggers.Options{Level: logg.LevelWarn, StdErr: io.Discard, StdOut: io.Discard, StoreErrors: true})
		want := goval.CallRaw(func() (any, error) {
			ns, err := page.DecodeCascadeConfig(logger, true, in)
			if err != nil {
				return nil, err
			}
			var entries []any
			for _, k := range ns.Config.Keys() {
				v, _ := ns.Config.Get(k)
				entries = append(entries, map[string]any{
					"target": matcherDump(k),
					"params": goval.Encode(v.Params),
					"fields": goval.Encode(v.Fields),
				})
			}
			return map[string]any{"entries": entries, "sourceHash": ns.SourceHash}, nil
		})
		add(map[string]any{"fn": "cascade", "in": inEnc, "want": want, "errors": logger.Errors()})
	}

	for _, m := range matchers {
		for _, p := range testPages {
			add(map[string]any{"fn": "matches", "matcher": matcherDump(m), "page": map[string]any{"kind": p.kind, "lang": p.lang, "path": p.path, "env": p.env}, "want": m.Matches(p)})
		}
	}

	custom := output.Format{Name: "custom", MediaType: media.Builtin.HTMLType, Rel: "customrel", BaseName: "index"}
	formats := append(output.Formats{}, output.DefaultFormats...)
	formats = append(formats, custom, output.Format{Name: "HTML", Rel: "x"}, output.Format{Name: "Json", Rel: "y"})
	for _, f := range formats {
		for _, canonical := range []bool{false, true} {
			o := page.NewOutputFormat("/rel/", "https://x/abs/", canonical, f)
			add(map[string]any{"fn": "newOutputFormat", "format": psupport.FormatDump(f), "canonical": canonical, "want": map[string]any{"rel": o.Rel, "name": o.Name(), "permalink": o.Permalink(), "relPermalink": o.RelPermalink()}})
		}
	}
	var ofs page.OutputFormats
	for _, f := range output.DefaultFormats[:5] {
		ofs = append(ofs, page.NewOutputFormat("/r/"+f.Name, "p", false, f))
	}
	var ofsDump []any
	for _, o := range ofs {
		ofsDump = append(ofsDump, psupport.FormatDump(o.Format))
	}
	for _, n := range []string{"html", "HTML", "Json", "rss", "amp", "css", "nope", ""} {
		o := ofs.Get(n)
		var want any
		if o != nil {
			want = o.RelPermalink()
		}
		add(map[string]any{"fn": "outputFormatsGet", "formats": ofsDump, "name": n, "want": want})
	}

	for _, s := range markups {
		mt := pagemeta.MarkupToMediaType(s, media.DefaultTypes)
		add(map[string]any{"fn": "markupToMediaType", "in": s, "want": psupport.MediaTypeDump(mt)})
	}

	type pcIn struct {
		Kind, Path, Lang, Markup, MediaType, Ext string
		Outputs                                  []string
		Params                                   maps.Params
		Cascade                                  []map[string]any
		PagesFromData                            bool
	}
	pcs := []pcIn{
		{Kind: "page", Path: "/a", Ext: "md"},
		{Kind: "page", Ext: "html"},
		{Kind: "page", Ext: "adoc"},
		{Kind: "page", Ext: "unknownext"},
		{Kind: "page", Ext: ""},
		{Kind: "page", Markup: "markdown", Ext: "html"},
		{Kind: "page", Markup: "HTML"},
		{Kind: "page", Markup: "bogus"},
		{Kind: "page", MediaType: "text/markdown"},
		{Kind: "page", MediaType: "text/nope"},
		{Kind: "page", MediaType: "text/html", Markup: "md"},
		{Kind: "page", Outputs: []string{"html", "JSON", "rss"}, Ext: "md"},
		{Kind: "page", Outputs: []string{"html", "nope"}, Ext: "md"},
		{Kind: "page", Params: maps.Params{"Title": "X", "Nested": maps.Params{"A": 1}}, Ext: "md"},
		{Kind: "page", Cascade: []map[string]any{{"a": 1}}, Ext: "md"},
		{Kind: "section", Cascade: []map[string]any{{"a": 1}}, Ext: "md"},
		{Kind: "page", Path: "/x", PagesFromData: true},
		{Kind: "page", Path: "", PagesFromData: true},
		{Kind: "home", Path: "/", PagesFromData: true},
		{Kind: "page", Path: "/x", Lang: "en", PagesFromData: true},
		{Kind: "page", Path: "/x", Markup: "md", PagesFromData: true},
	}
	for _, in := range pcs {
		pc := &pagemeta.PageConfig{}
		pc.Kind, pc.Path, pc.Lang = in.Kind, in.Path, in.Lang
		pc.Content.Markup, pc.Content.MediaType = in.Markup, in.MediaType
		pc.Outputs = in.Outputs
		pc.Params = in.Params
		pc.Cascade = in.Cascade
		inDump := map[string]any{
			"kind": in.Kind, "path": in.Path, "lang": in.Lang, "markup": in.Markup, "mediaType": in.MediaType, "ext": in.Ext,
			"outputs": in.Outputs, "params": goval.Encode(in.Params), "cascade": in.Cascade != nil, "pagesFromData": in.PagesFromData,
		}
		want := map[string]any{}
		if err := pc.Init(in.PagesFromData); err != nil {
			want["initErr"] = err.Error()
		}
		if err := pc.Compile(in.Ext, loggers.NewDefault(), output.DefaultFormats, media.DefaultTypes); err != nil {
			want["compileErr"] = err.Error()
		}
		var names []string
		for _, f := range pc.ConfiguredOutputFormats {
			names = append(names, f.Name)
		}
		want["path"] = pc.Path
		want["markup"] = pc.Content.Markup
		want["contentMediaType"] = psupport.MediaTypeDump(pc.ContentMediaType)
		want["outputs"] = names
		want["params"] = goval.Encode(pc.Params)
		add(map[string]any{"fn": "pageConfig", "in": inDump, "want": want})
	}

	metaNames := []string{"kind", "bundletype", "mediatype", "section", "lang", "aliases", "name", "keywords", "description", "title", "linktitle", "slug", "date", "publishdate", "expirydate", "lastmod", "draft", "type", "layout", "weight", "custom", "a.b"}
	sort.Strings(metaNames)
	for _, n := range metaNames {
		v, found, err := page.NamedPageMetaValue(page.NopPage, n)
		want := map[string]any{"found": found}
		if err != nil {
			want["err"] = err.Error()
		}
		if mt, ok := v.(media.Type); ok {
			want["value"] = psupport.MediaTypeDump(mt)
		} else {
			want["value"] = goval.Encode(v)
		}
		add(map[string]any{"fn": "namedPageMetaValue", "name": n, "want": want})
	}

	if err := goval.WriteCasesGz(filepath.Join(*out, "misc.json.gz"), map[string]any{}, cases); err != nil {
		log.Fatal(err)
	}
	log.Printf("misc: %d cases", len(cases))
}
