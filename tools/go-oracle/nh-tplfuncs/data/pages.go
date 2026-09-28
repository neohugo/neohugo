package main

import (
	"errors"
	"fmt"
	"reflect"
	"strings"

	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/hugolib"
	"github.com/neohugo/neohugo/resources/page"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-page/psupport"
)

// pageTable records every page value a case refers to; the Rust test
// rebuilds each entry as a fake page with the same template methods. A
// wrapper (hugolib.pageWithWeight0, *hugolib.pageWithOrdinal) is its own
// entry; "pid" identifies the *hugolib.pageState behind it.
type pageTable struct {
	enc     *encoder
	entries []map[string]any
	index   map[page.Page]int
	pids    map[uintptr]int
	// byPtr maps the address of every *pageState of the build to it.
	byPtr map[uintptr]page.Page
}

func newPageTable(all page.Pages) *pageTable {
	t := &pageTable{index: map[page.Page]int{}, pids: map[uintptr]int{}, byPtr: map[uintptr]page.Page{}}
	for _, p := range all {
		t.byPtr[underlying(p)] = p
	}
	return t
}

// underlying returns the address of the *hugolib.pageState behind p.
func underlying(p page.Page) uintptr {
	v := reflect.ValueOf(p)
	for range 4 {
		if v.Kind() == reflect.Ptr {
			if v.Type().Elem().Name() == "pageState" {
				return v.Pointer()
			}
			v = v.Elem()
			continue
		}
		if v.Kind() == reflect.Struct {
			f := v.FieldByName("pageState")
			if f.IsValid() {
				v = f
				continue
			}
		}
		break
	}
	panic(fmt.Sprintf("no pageState in %T", p))
}

func (t *pageTable) add(p page.Page) int {
	if i, ok := t.index[p]; ok {
		return i
	}
	u := underlying(p)
	pid, ok := t.pids[u]
	if !ok {
		pid = len(t.pids)
		t.pids[u] = pid
	}
	e := map[string]any{
		"pid":          pid,
		"type":         fmt.Sprintf("%T", p),
		"kind":         p.Kind(),
		"title":        str(p.Title()),
		"linkTitle":    str(p.LinkTitle()),
		"weight":       p.Weight(),
		"date":         t.enc.enc(p.Date()),
		"lastmod":      t.enc.enc(p.Lastmod()),
		"publishDate":  t.enc.enc(p.PublishDate()),
		"section":      str(p.Section()),
		"pageType":     str(p.Type()),
		"lang":         p.Lang(),
		"path":         str(p.Path()),
		"isPage":       p.IsPage(),
		"isNode":       p.IsNode(),
		"isHome":       p.IsHome(),
		"isSection":    p.IsSection(),
		"name":         str(p.Name()),
		"relPermalink": str(p.RelPermalink()),
		"params":       t.enc.enc(maps.Params(p.Params())),
	}
	if w, ok := p.(interface{ Weight0() int }); ok {
		e["weight0"] = w.Weight0()
	}
	if o, ok := p.(interface{ Ordinal() int }); ok {
		e["ordinal"] = o.Ordinal()
	}
	t.index[p] = len(t.entries)
	t.entries = append(t.entries, e)
	i := len(t.entries) - 1
	// A wrapper's *pageState is recorded too (the fake wrapper unwraps to it).
	if base, ok := t.byPtr[u]; ok && base != p {
		t.add(base)
	}
	return i
}

func fm(lines ...string) string {
	return "---\n" + strings.Join(lines, "\n") + "\n---\n"
}

// site is the page source of the page cases: two sections, tags with
// weights, params of every kind (missing in some pages), Thai titles.
func site() psupport.Site {
	files := map[string]string{
		"content/_index.md":                         fm(`title: "Home"`),
		"content/a/_index.md":                       fm(`title: "Section A"`, `weight: 2`),
		"content/b/_index.md":                       fm(`title: "Section B"`, `weight: 1`),
		"content/a/p1.md":                           fm(`title: "Alpha"`, `weight: 3`, `date: 2021-01-01T00:00:00Z`, `tags: ["go", "rust"]`, `tags_weight: 10`, `x: 1`, `s: "b"`, `f: 1.5`, `flag: true`, `nested: {k: "v1", n: 2}`, `list: ["a", "b"]`, `type: snack`, `when: 2020-05-05T00:00:00Z`),
		"content/a/p2.md":                           fm(`title: "beta"`, `weight: 1`, `date: 2022-02-02T00:00:00Z`, `tags: ["go"]`, `tags_weight: 5`, `x: 2`, `s: "a"`, `f: 2`, `flag: false`, `nested: {k: "v2", n: 1}`, `list: ["c"]`, `type: drink`),
		"content/a/p3.md":                           fm(`title: "Gamma"`, `weight: 2`, `date: 2020-03-03T00:00:00Z`, `x: "3"`, `s: "c"`, `nested: {k: "v1"}`, `type: snack`),
		"content/a/p4.md":                           fm(`title: "ข้าว"`, `weight: 0`, `date: 2021-01-01T00:00:00Z`, `tags: ["rust"]`, `x: 10`, `s: "ข"`, `f: -1`, `type: snack`),
		"content/b/q1.md":                           fm(`title: "ขนม"`, `weight: 5`, `date: 2019-09-09T00:00:00Z`, `tags: ["go", "ขนม"]`, `x: 2.5`, `s: "B"`, `flag: true`, `nested: {k: "V1", n: 3}`),
		"content/b/q2.md":                           fm(`title: "alpha"`, `weight: 4`, `date: 2023-03-03T00:00:00Z`, `x: -1`, `s: "10"`, `f: 0`, `list: []`),
		"content/b/q3/index.md":                     fm(`title: "Éclair"`, `weight: 3`, `date: 2021-06-06T00:00:00Z`, `tags: ["rust", "go"]`, `x: 0`, `s: "2"`, `type: snack`),
		"layouts/list.html":                         `{{ .Title }}`,
		"layouts/single.html":                       `{{ .Title }}`,
		"layouts/taxonomy.html":                     `{{ .Title }}`,
		"layouts/term.html":                         `{{ .Title }}`,
		"layouts/_default/_markup/render-link.html": `{{ .Text }}`,
	}
	toml := `baseURL = "https://example.org/"
title = "T18"
disableKinds = ["RSS", "sitemap", "robotsTXT", "404"]
[taxonomies]
tag = "tags"
`
	return psupport.Site{Name: "t18", TOML: toml, Files: files}
}

// buildPages builds the site and returns its page values: the regular
// pages, the sections, home, a term's pages (pageWithWeight0 wrappers) and
// GetTerms results (*pageWithOrdinal wrappers).
func buildPages() (*hugolib.HugoSites, map[string]page.Pages, error) {
	b, err := psupport.Build(site())
	if err != nil {
		return nil, nil, err
	}
	h := b.H
	s := h.Sites[0]
	out := map[string]page.Pages{}
	out["regular"] = s.RegularPages()
	var sections page.Pages
	for _, p := range s.Pages() {
		if p.Kind() == "section" || p.Kind() == "home" {
			sections = append(sections, p)
		}
	}
	out["nodes"] = sections
	var goTerm page.Page
	for _, p := range s.Pages() {
		if p.Kind() == "term" && p.Path() == "/tags/go" {
			goTerm = p
		}
	}
	if goTerm == nil {
		return nil, nil, errors.New("no /tags/go term")
	}
	out["weighted"] = goTerm.Pages()
	var ordinals page.Pages
	for _, p := range s.RegularPages() {
		ordinals = append(ordinals, p.GetTerms("tags")...)
	}
	out["ordinals"] = ordinals
	return h, out, nil
}
