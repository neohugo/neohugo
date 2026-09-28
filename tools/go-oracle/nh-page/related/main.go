// Command related is the Go oracle for related.InvertedIndex, related.Config
// decoding and Pages.Related (related/inverted_index.go,
// resources/page/pages_related.go) in crates/nh-page (Wave B task T12).
//
//	go run ./tools/go-oracle/nh-page/related [-root .] [-out crates/nh-page/tests/fixtures/related]
//
// Over the builds of csupport.Sites (docs/ and hugolib/testsite, two psupport
// synthetic sites, the ties sites with keywords, tags, categories, authors,
// dates, fragments/headings, Thai terms):
//
//   - Pages.Related(ctx, p) with the site's own related config (docs:
//     toLower, includeNewer, keywords only; ties: toLower, includeNewer,
//     threshold 20, cardinalityThreshold, a date pattern and a fragments
//     index with applyFilter; the others the default config plus tags) for
//     EVERY page of every site's RegularPages, plus option maps (indices,
//     fragments, namedSlices-free document searches) and bad arguments;
//   - InvertedIndex built directly with the default config and five custom
//     configs (includeNewer, toLower, thresholds 0..100, cardinality
//     thresholds, date indices with patterns, fragments indices with and
//     without applyFilter, negative and zero weights, param indices of mixed
//     types): Search with a document for every page, with index subsets,
//     with named slices and with fragments;
//   - DecodeConfig of those configs and of invalid ones.
//
// Results are page table entries (a *hugolib.pageHeadingsFiltered result is
// its page and the filtered heading IDs). Go collects the candidates in a map
// and then stable-sorts them by (weight, publish date, name): runs of results
// with the same publish date and name keep a random order in Go, so they are
// written sorted by entry and marked "ties".
//
// Output: <site>.json.gz per site and decode.json.gz. Nothing depends on the
// platform.
package main

import (
	"context"
	"flag"
	"fmt"
	"log"
	"path/filepath"
	"sort"

	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/common/types"
	"github.com/neohugo/neohugo/markup/tableofcontents"
	"github.com/neohugo/neohugo/related"
	"github.com/neohugo/neohugo/resources/page"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-page/csupport"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-page/psupport"
)

// Custom configs (maps.Params as in a site config).
var customConfigs = []map[string]any{
	{
		"includeNewer": true, "toLower": true, "threshold": 0,
		"indices": []any{
			map[string]any{"name": "keywords", "weight": 100},
			map[string]any{"name": "tags", "weight": 80, "cardinalityThreshold": 50},
			map[string]any{"name": "date", "weight": 10, "pattern": "2006-01"},
		},
	},
	{
		"threshold": 50,
		"indices": []any{
			map[string]any{"name": "Tags", "weight": 100, "toLower": true},
			map[string]any{"name": "categories", "weight": 50},
			map[string]any{"name": "author", "weight": 30},
			map[string]any{"name": "publishDate", "weight": 20, "pattern": "2006"},
			map[string]any{"name": "fragments", "type": "fragments", "applyFilter": true, "weight": 60},
		},
	},
	{
		"threshold": 10, "includeNewer": "true",
		"indices": []any{
			map[string]any{"name": "keywords", "weight": 100},
			map[string]any{"name": "date", "weight": -10},
			map[string]any{"name": "section", "weight": 20},
			map[string]any{"name": "title", "weight": 5, "toLower": true},
			map[string]any{"name": "rating", "weight": 7},
		},
	},
	{
		"threshold": 100, "toLower": true,
		"indices": []any{
			map[string]any{"name": "keywords", "weight": 0},
			map[string]any{"name": "tags", "weight": 30, "cardinalityThreshold": 100},
			map[string]any{"name": "lastmod", "weight": 30, "pattern": "2006-01-02"},
			map[string]any{"name": "fragments", "type": "fragments", "weight": 30},
		},
	},
	{
		"threshold": 80,
		"indices": []any{
			map[string]any{"name": "tags", "weight": 100, "cardinalityThreshold": 1},
			map[string]any{"name": "kind", "weight": 50},
			map[string]any{"name": "lang", "weight": 10},
			map[string]any{"name": "slug", "weight": 10},
			map[string]any{"name": "draft", "weight": 10},
		},
	},
}

var invalidConfigs = []any{
	nil,
	map[string]any{},
	map[string]any{"threshold": 101},
	map[string]any{"threshold": -1},
	map[string]any{"indices": []any{map[string]any{"name": "x", "type": "nope"}}},
	map[string]any{"indices": []any{map[string]any{"name": "x", "cardinalityThreshold": -1}}},
	map[string]any{"indices": []any{map[string]any{"name": "x", "cardinalityThreshold": 101}}},
	map[string]any{"indices": "not a list"},
	map[string]any{"threshold": "abc"},
	map[string]any{"indices": []any{map[string]any{"name": "Mixed", "weight": "12", "applyFilter": "1"}}, "toLower": 1},
}

func main() {
	root := flag.String("root", ".", "neohugo module root")
	out := flag.String("out", "crates/nh-page/tests/fixtures/related", "output directory")
	flag.Parse()
	outDir := psupport.OutDir(*root, *out)

	var decodeCases []map[string]any
	for _, c := range customConfigs {
		decodeCases = append(decodeCases, decodeCase(c))
	}
	for _, c := range invalidConfigs {
		decodeCases = append(decodeCases, decodeCase(c))
	}
	if err := goval.WriteCasesGz(filepath.Join(outDir, "decode.json.gz"), map[string]any{}, decodeCases); err != nil {
		log.Fatal(err)
	}

	sites, err := csupport.Sites(*root)
	if err != nil {
		log.Fatal(err)
	}
	total := 0
	for _, s := range sites {
		n, err := runSite(s, outDir)
		if err != nil {
			log.Fatal(err)
		}
		total += n
	}
	log.Printf("related: %d cases, %d decode cases", total, len(decodeCases))
}

func cfgEnc(c related.Config) map[string]any {
	var idx []any
	for _, i := range c.Indices {
		idx = append(idx, map[string]any{
			"name": goval.Str(i.Name), "type": i.Type, "applyFilter": i.ApplyFilter, "pattern": i.Pattern,
			"weight": i.Weight, "cardinalityThreshold": i.CardinalityThreshold, "toLower": i.ToLower,
		})
	}
	return map[string]any{"threshold": c.Threshold, "includeNewer": c.IncludeNewer, "toLower": c.ToLower, "indices": idx}
}

func decodeCase(in any) map[string]any {
	var p maps.Params
	if m, ok := in.(map[string]any); ok {
		p = maps.Params(m)
		maps.PrepareParams(p)
	}
	return map[string]any{"in": goval.Encode(p), "res": goval.CallRaw(func() (any, error) {
		c, err := related.DecodeConfig(p)
		if err != nil {
			return nil, err
		}
		return cfgEnc(c), nil
	})}
}

type runner struct {
	t       *csupport.Table
	byUnder map[uintptr]int
}

// docs encodes a search result.
func (r *runner) docs(ds []related.Document) any {
	if ds == nil {
		return nil
	}
	type enc struct {
		i    int
		h    []any
		date int64
		nsec int
		name string
	}
	var es []enc
	for _, d := range ds {
		p := d.(page.Page)
		i, ok := r.byUnder[csupport.Underlying(p)]
		if !ok {
			panic(fmt.Sprintf("result %T not in the index", d))
		}
		e := enc{i: i, date: d.PublishDate().Unix(), nsec: d.PublishDate().Nanosecond(), name: d.Name()}
		if hf, ok := d.(interface {
			HeadingsFiltered(context.Context) tableofcontents.Headings
		}); ok && fmt.Sprintf("%T", d) == "*hugolib.pageHeadingsFiltered" {
			e.h = []any{}
			for _, h := range hf.HeadingsFiltered(context.Background()) {
				e.h = append(e.h, goval.Str(h.ID))
			}
		}
		es = append(es, e)
	}
	ties := false
	for i := 0; i < len(es); {
		j := i + 1
		for j < len(es) && es[j].date == es[i].date && es[j].nsec == es[i].nsec && es[j].name == es[i].name {
			j++
		}
		if j-i > 1 {
			ties = true
			run := es[i:j]
			sort.SliceStable(run, func(a, b int) bool { return run[a].i < run[b].i })
		}
		i = j
	}
	out := []any{}
	for _, e := range es {
		m := map[string]any{"i": e.i}
		if e.h != nil {
			m["h"] = e.h
		}
		out = append(out, m)
	}
	res := map[string]any{"docs": out}
	if ties {
		res["ties"] = true
	}
	return res
}

func (r *runner) pagesDocs(ps page.Pages) any {
	if ps == nil {
		return nil
	}
	ds := make([]related.Document, len(ps))
	for i, p := range ps {
		ds[i] = p
	}
	return r.docs(ds)
}

func runSite(s psupport.Site, outDir string) (int, error) {
	b, err := psupport.Build(s)
	if err != nil {
		return 0, err
	}
	h := b.H
	ctx := context.Background()
	t := csupport.NewTable(h)
	r := &runner{t: t, byUnder: map[uintptr]int{}}
	var cases []map[string]any

	var lists []any
	var siteCfgs []any
	for si, st := range h.Sites {
		ps := st.RegularPages()
		lists = append(lists, t.List(ps))
		for _, p := range ps {
			r.byUnder[csupport.Underlying(p)] = t.Add(p)
		}
		siteCfgs = append(siteCfgs, cfgEnc(h.Configs.LanguageConfigMap[st.Language().Lang].Related))

		// The site's own config through Pages.Related.
		for _, p := range ps {
			cases = append(cases, map[string]any{"op": "Related", "list": si, "doc": t.Add(p),
				"res": goval.CallRaw(func() (any, error) {
					rel, err := ps.Related(ctx, p)
					return r.pagesDocs(rel), err
				})})
		}
		for i, p := range ps {
			if i%7 != 0 {
				continue
			}
			for oi, opts := range []map[string]any{
				{"document": p, "indices": []string{"keywords"}},
				{"Document": p, "indices": []any{"tags", "date"}},
				{"document": p, "fragments": []string{"common", "heading-1"}},
				{"fragments": []any{"common"}},
				{"indices": []string{"nope"}, "document": p},
				{"document": "not a page"},
			} {
				cases = append(cases, map[string]any{"op": "RelatedOpts", "list": si, "doc": t.Add(p), "opts": oi,
					"res": goval.CallRaw(func() (any, error) {
						rel, err := ps.Related(ctx, opts)
						return r.pagesDocs(rel), err
					})})
			}
		}
		if len(ps) > 0 {
			cases = append(cases, map[string]any{"op": "RelatedBadArg", "list": si,
				"res": goval.CallRaw(func() (any, error) {
					rel, err := ps.Related(ctx, "a string")
					return r.pagesDocs(rel), err
				})})
		}
	}

	// Direct inverted indices with the default and custom configs.
	var cfgs []related.Config
	cfgs = append(cfgs, related.DefaultConfig)
	for _, c := range customConfigs {
		p := maps.Params(c)
		maps.PrepareParams(p)
		cfg, err := related.DecodeConfig(p)
		if err != nil {
			return 0, err
		}
		cfgs = append(cfgs, cfg)
	}
	var cfgHeader []any
	for _, c := range cfgs {
		cfgHeader = append(cfgHeader, cfgEnc(c))
	}
	for ci, cfg := range cfgs {
		for si, st := range h.Sites {
			ps := st.RegularPages()
			if len(ps) == 0 {
				continue
			}
			if cfg.HasType(related.TypeFragments) && anyNilFragments(ctx, ps) {
				// Go dereferences the nil *Fragments of such a page (a panic).
				continue
			}
			idx := related.NewInvertedIndex(cfg)
			var addErr []any
			for _, p := range ps {
				if err := idx.Add(ctx, p); err != nil {
					addErr = append(addErr, []any{t.Add(p), err.Error()})
				}
			}
			if err := idx.Finalize(ctx); err != nil {
				return 0, err
			}
			cases = append(cases, map[string]any{"op": "Add", "cfg": ci, "list": si, "errs": addErr})
			for pi, p := range ps {
				c := map[string]any{"op": "Search", "cfg": ci, "list": si, "doc": t.Add(p)}
				c["res"] = goval.CallRaw(func() (any, error) {
					ds, err := idx.Search(ctx, related.SearchOpts{Document: p})
					return r.docs(ds), err
				})
				cases = append(cases, c)
				if pi%5 != 0 {
					continue
				}
				for _, ix := range [][]string{{cfg.Indices[0].Name}, {cfg.Indices[len(cfg.Indices)-1].Name, cfg.Indices[0].Name}, {"missing"}} {
					c := map[string]any{"op": "Search", "cfg": ci, "list": si, "doc": t.Add(p), "indices": ix}
					c["res"] = goval.CallRaw(func() (any, error) {
						ds, err := idx.Search(ctx, related.SearchOpts{Document: p, Indices: ix})
						return r.docs(ds), err
					})
					cases = append(cases, c)
				}
				for _, fr := range [][]string{{"common"}, {"heading-0", "heading-2"}} {
					c := map[string]any{"op": "Search", "cfg": ci, "list": si, "doc": t.Add(p), "fragments": fr}
					c["res"] = goval.CallRaw(func() (any, error) {
						ds, err := idx.Search(ctx, related.SearchOpts{Document: p, Fragments: fr})
						return r.docs(ds), err
					})
					cases = append(cases, c)
				}
			}
			// Named slices (no document: no date filter, no self skip).
			for _, ns := range []types.KeyValues{
				{Key: "tags", Values: []any{"go", "Rust"}},
				{Key: "keywords", Values: []any{"hugo", []string{"go", "templates"}, []any{"snacks", 3}}},
				{Key: "date", Values: []any{p0date(ps)}},
				{Key: "nope", Values: []any{"x"}},
				{Key: "", Values: []any{"x"}},
				{Key: "tags", Values: []any{map[string]any{"x": 1}}},
				{Key: "fragments", Values: []any{"common"}},
			} {
				c := map[string]any{"op": "SearchNamed", "cfg": ci, "list": si, "key": goval.Encode(ns.Key), "values": goval.Encode(ns.Values)}
				c["res"] = goval.CallRaw(func() (any, error) {
					ds, err := idx.Search(ctx, related.SearchOpts{NamedSlices: []types.KeyValues{ns}})
					return r.docs(ds), err
				})
				cases = append(cases, c)
			}
		}
	}

	header := map[string]any{
		"site":        s.Name,
		"sites":       t.SitesHeader(),
		"pages":       t.Entries,
		"lists":       lists,
		"siteConfigs": siteCfgs,
		"configs":     cfgHeader,
	}
	log.Printf("%s: %d cases", s.Name, len(cases))
	return len(cases), goval.WriteCasesGz(filepath.Join(outDir, s.Name+".json.gz"), header, cases)
}

func p0date(ps page.Pages) any {
	if len(ps) == 0 {
		return nil
	}
	return ps[0].Date()
}

func anyNilFragments(ctx context.Context, ps page.Pages) bool {
	for _, p := range ps {
		if p.Fragments(ctx) == nil {
			return true
		}
	}
	return false
}
