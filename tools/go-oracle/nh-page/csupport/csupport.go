// Package csupport holds what the nh-page collection oracles (collections,
// pagination, related, menus; Wave B task T12) share: the sites they build
// (psupport's repository and synthetic sites plus sites that stress ties),
// the page table that records every page value a case refers to (with the
// attributes the Rust fake pages answer from), and the overlay hook that sets
// the site hugolib considers "current" (the collator and time formatter of
// page sorts and date groups come from it).
package csupport

import (
	"context"
	"fmt"
	"reflect"
	"sort"
	"strings"

	"github.com/neohugo/neohugo/hugolib"
	"github.com/neohugo/neohugo/markup/tableofcontents"
	"github.com/neohugo/neohugo/resources/page"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-page/psupport"
)

// SetCurrent is installed by the overlay hook (hugolib has no exported way to
// set the current site).
var SetCurrent func(h *hugolib.HugoSites, i int)

// HookName is the file name of the overlay-added hook file.
const HookName = "zz_current_overlay.go"

// HookFile installs SetCurrent.
const HookFile = `package main

import (
	"github.com/neohugo/neohugo/hugolib"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-page/csupport"
)

func init() {
	csupport.SetCurrent = hugolib.OracleSetCurrentSite
}
`

// Patches adds hugolib.OracleSetCurrentSite.
var Patches = []psupport.Patch{{
	File:   "hugolib/site.go",
	After:  "func (s *Site) Current() page.Site {",
	Append: "\n// OracleSetCurrentSite is added by the nh-page collection oracles.\nfunc OracleSetCurrentSite(h *HugoSites, i int) { h.currentSite = h.Sites[i] }\n",
}}

// Table records page values. Every distinct page.Page value (the wrappers
// pageWithWeight0 and *pageWithOrdinal are distinct from the page they wrap)
// is one entry; "pid" identifies the underlying page.
type Table struct {
	H       *hugolib.HugoSites
	Entries []map[string]any
	index   map[page.Page]int
	pids    map[uintptr]int
	// Headings holds each entry's headings in walk order (for the related
	// fragments filter).
	Headings [][]string
	pages    []page.Page
}

// Pages returns the recorded page values in entry order.
func (t *Table) Pages() []page.Page { return t.pages }

// NewTable returns an empty table over the built sites.
func NewTable(h *hugolib.HugoSites) *Table {
	return &Table{H: h, index: map[page.Page]int{}, pids: map[uintptr]int{}}
}

// Underlying returns the address of the *hugolib.pageState behind p.
func Underlying(p page.Page) uintptr {
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
	panic(fmt.Sprintf("csupport: no pageState in %T", p))
}

// SiteIndex returns the index of the site of language lang.
func (t *Table) SiteIndex(lang string) int {
	for i, s := range t.H.Sites {
		if s.Language().Lang == lang {
			return i
		}
	}
	panic("csupport: no site for " + lang)
}

// SitesHeader describes the sites (language, language weight, site params).
func (t *Table) SitesHeader() []any {
	var out []any
	for _, s := range t.H.Sites {
		out = append(out, map[string]any{
			"lang":   s.Language().Lang,
			"weight": s.Language().Weight,
			"params": goval.Encode(map[string]any(s.Params())),
		})
	}
	return out
}

func timeEnc(v any) any { return goval.Encode(v) }

// Add returns the entry index of p (recording it on first use); -1 for nil.
func (t *Table) Add(p page.Page) int {
	if p == nil {
		return -1
	}
	if i, ok := t.index[p]; ok {
		return i
	}
	u := Underlying(p)
	pid, ok := t.pids[u]
	if !ok {
		pid = len(t.pids)
		t.pids[u] = pid
	}
	ctx := context.Background()
	e := map[string]any{
		"pid":            pid,
		"type":           fmt.Sprintf("%T", p),
		"site":           t.SiteIndex(p.Lang()),
		"kind":           p.Kind(),
		"title":          goval.Str(p.Title()),
		"linkTitle":      goval.Str(p.LinkTitle()),
		"description":    goval.Str(p.Description()),
		"weight":         p.Weight(),
		"date":           timeEnc(p.Date()),
		"lastmod":        timeEnc(p.Lastmod()),
		"publishDate":    timeEnc(p.PublishDate()),
		"expiryDate":     timeEnc(p.ExpiryDate()),
		"isHome":         p.IsHome(),
		"isNode":         p.IsNode(),
		"isPage":         p.IsPage(),
		"isSection":      p.IsSection(),
		"section":        goval.Str(p.Section()),
		"pageType":       goval.Str(p.Type()),
		"layout":         goval.Str(p.Layout()),
		"lang":           p.Lang(),
		"path":           goval.Str(p.Path()),
		"pathInfo":       goval.Str(p.PathInfo().Path()),
		"slug":           goval.Str(p.Slug()),
		"draft":          p.Draft(),
		"aliases":        goval.Encode(p.Aliases()),
		"keywords":       goval.Encode(p.Keywords()),
		"bundleType":     goval.Str(p.BundleType()),
		"name":           goval.Str(p.Name()),
		"translationKey": goval.Str(p.TranslationKey()),
		"relPermalink":   goval.Str(p.RelPermalink()),
		"params":         goval.Encode(map[string]any(p.Params())),
		"len":            p.Len(ctx),
	}
	if f := p.File(); f != nil {
		e["filename"] = goval.Str(f.Filename())
	}
	if w, ok := p.(interface{ Weight0() int }); ok {
		e["weight0"] = w.Weight0()
	}
	if o, ok := p.(interface{ Ordinal() int }); ok {
		e["ordinal"] = o.Ordinal()
	}
	var headings []string
	if fr := p.Fragments(ctx); fr != nil {
		ids := []any{}
		for _, id := range fr.Identifiers {
			ids = append(ids, goval.Str(id))
		}
		e["fragments"] = ids
		for _, h := range fr.Headings.FilterBy(func(*tableofcontents.Heading) bool { return true }) {
			headings = append(headings, h.ID)
		}
		hs := []any{}
		for _, id := range headings {
			hs = append(hs, goval.Str(id))
		}
		e["headings"] = hs
	}
	t.index[p] = len(t.Entries)
	t.Entries = append(t.Entries, e)
	t.Headings = append(t.Headings, headings)
	t.pages = append(t.pages, p)
	i := len(t.Entries) - 1
	e["parent"] = t.Add(p.Parent())
	return i
}

// List returns the entry indices of ps (nil for a nil list, [] for empty).
func (t *Table) List(ps page.Pages) any {
	if ps == nil {
		return nil
	}
	out := make([]int, len(ps))
	for i, p := range ps {
		out[i] = t.Add(p)
	}
	return out
}

// Shuffle returns a deterministic permutation of ps (an LCG seeded with seed).
func Shuffle(ps page.Pages, seed uint64) page.Pages {
	out := make(page.Pages, len(ps))
	copy(out, ps)
	s := seed*6364136223846793005 + 1442695040888963407
	for i := len(out) - 1; i > 0; i-- {
		s = s*6364136223846793005 + 1442695040888963407
		j := int((s >> 33) % uint64(i+1))
		out[i], out[j] = out[j], out[i]
	}
	return out
}

// NamedList is an input page list of a site.
type NamedList struct {
	Name  string
	Pages page.Pages
}

// Lists returns the page lists of a build the oracles use as inputs: the
// sites' page collections, every section's and term's pages, GetTerms lists
// (pages with ordinals), shuffles and a cross-language list.
func Lists(h *hugolib.HugoSites) []NamedList {
	var out []NamedList
	add := func(name string, ps page.Pages) {
		if len(ps) == 0 {
			return
		}
		out = append(out, NamedList{name, ps})
	}
	var allRegular page.Pages
	for si, s := range h.Sites {
		pre := fmt.Sprintf("s%d:", si)
		add(pre+"RegularPages", s.RegularPages())
		add(pre+"Pages", s.Pages())
		add(pre+"AllPages", s.AllPages())
		add(pre+"shuffle(RegularPages,1)", Shuffle(s.RegularPages(), 1))
		add(pre+"shuffle(RegularPages,2)", Shuffle(s.RegularPages(), 2))
		add(pre+"shuffle(AllPages,3)", Shuffle(s.AllPages(), 3))
		allRegular = append(allRegular, s.RegularPages()...)
		var nodes page.Pages
		for _, p := range s.Pages() {
			if p.IsNode() {
				nodes = append(nodes, p)
			}
		}
		sort.SliceStable(nodes, func(i, j int) bool { return nodes[i].Path() < nodes[j].Path() })
		for _, p := range nodes {
			add(pre+"node:"+p.Path()+":Pages", p.Pages())
			add(pre+"node:"+p.Path()+":RegularPages", p.RegularPages())
			if len(p.Pages()) > 12 {
				add(pre+"node:"+p.Path()+":shuffle(Pages,4)", Shuffle(p.Pages(), 4))
			}
		}
		var taxos []string
		for k := range s.Taxonomies() {
			taxos = append(taxos, k)
		}
		sort.Strings(taxos)
		n := 0
		for _, p := range s.RegularPages() {
			for _, tx := range taxos {
				if ts := p.GetTerms(tx); len(ts) > 1 && n < 12 {
					add(pre+"GetTerms:"+p.Path()+":"+tx, ts)
					n++
				}
			}
		}
	}
	if len(h.Sites) > 1 {
		add("all:RegularPages", allRegular)
		add("all:shuffle(RegularPages,5)", Shuffle(allRegular, 5))
	}
	return out
}

// Sites returns the sites of the collection oracles: the repository's docs/
// and hugolib/testsite, psupport's seeksnack-like and multihost synthetic
// sites, and the ties sites below.
func Sites(root string) ([]psupport.Site, error) {
	sites, err := psupport.RepoSites(root)
	if err != nil {
		return nil, err
	}
	for _, s := range psupport.SyntheticSites() {
		if s.Name == "seeksnack" || s.Name == "multihost" {
			sites = append(sites, s)
		}
	}
	sites = append(sites, TiesSite(), TiesThSite())
	return sites, nil
}

func fm(lines ...string) string {
	return "---\n" + strings.Join(lines, "\n") + "\n---\n"
}

func q(s string) string { return fmt.Sprintf("%q", s) }

// tiesContent is content with many ties in every sort key: titles, link
// titles, weights (zero, negative, equal), dates (equal, missing), params of
// mixed types and missing params, Thai titles, translations, weighted terms,
// and sections with more than 12 pages (pdqsort paths).
func tiesContent() map[string]string {
	titles := []string{"Alpha", "alpha", "Beta", "Beta", "gamma", "Gamma", "Éclair", "eclair", "Zulu", "10 Things", "2 Things", "ขนม", "ข้าว", "กล้วย", "ไก่ย่าง", "เค้ก", "Beta", "", "Ωmega", "a-b", "a b", "ab", "ฯลฯ", "ํา"}
	linkTitles := []string{"", "", "Same Link", "Same Link", "", "zeta", "ขนม"}
	weights := []int{0, 1, 1, -1, 2, 0, 5, -3, 1}
	dates := []string{"2021-01-01T00:00:00Z", "2021-01-01T00:00:00Z", "", "2020-06-15T12:00:00+07:00", "2020-06-15T05:00:00Z", "2019-12-31T23:59:59Z", "2022-02-02"}
	tags := []string{"go", "rust", "Go", "ขนม", "snacks", "a b", "ฯลฯ"}
	c := map[string]string{
		"content/_index.md":          fm(`title: "Ties Home"`),
		"content/_index.th.md":       fm(`title: "หน้าแรก"`),
		"content/items/_index.md":    fm(`title: "Items"`, `weight: 3`),
		"content/thai/_index.md":     fm(`title: "ไทย"`, `weight: 3`),
		"content/nodates/_index.md":  fm(`title: "No Dates"`),
		"content/nodates/b.md":       fm(`title: "Same"`),
		"content/nodates/a.md":       fm(`title: "Same"`),
		"content/nodates/c/index.md": fm(`title: "Same"`),
		"content/nodates/d.md":       fm(`title: "Same"`, `weight: 0`),
		"content/nodates/e.md":       fm(`title: "same"`),
	}
	for i := range 40 {
		lines := []string{"title: " + q(titles[i%len(titles)])}
		if lt := linkTitles[i%len(linkTitles)]; lt != "" {
			lines = append(lines, "linkTitle: "+q(lt))
		}
		lines = append(lines, fmt.Sprintf("weight: %d", weights[i%len(weights)]))
		if d := dates[i%len(dates)]; d != "" {
			lines = append(lines, "date: "+d)
		}
		if i%5 == 0 {
			lines = append(lines, "publishDate: 2020-03-03T03:03:03Z")
		}
		if i%6 == 1 {
			lines = append(lines, "expiryDate: 2099-01-01T00:00:00Z")
		}
		if i%4 == 2 {
			lines = append(lines, fmt.Sprintf("lastmod: 2023-0%d-01T00:00:00Z", 1+i%9))
		}
		switch i % 5 {
		case 0:
			lines = append(lines, fmt.Sprintf("rating: %d", i%4))
		case 1:
			lines = append(lines, fmt.Sprintf("rating: %d.5", i%3))
		case 2:
			lines = append(lines, fmt.Sprintf("rating: %q", fmt.Sprint(i%7)))
		case 3:
			lines = append(lines, "rating: -2")
		}
		switch i % 6 {
		case 0:
			lines = append(lines, "mixed: 7")
		case 1:
			lines = append(lines, "mixed: \"seven\"")
		case 2:
			lines = append(lines, "mixed: true")
		case 3:
			lines = append(lines, "mixed: 7")
		case 4:
			lines = append(lines, "mixed: 2021-05-05T00:00:00Z")
		}
		lines = append(lines, fmt.Sprintf("score: %d", (i*7)%5))
		if i%3 != 0 {
			lines = append(lines, "author: "+q([]string{"Bob", "alice", "Alice", "บ๊อบ", "Émile"}[i%5]))
		}
		switch i % 4 {
		case 0:
			lines = append(lines, "event: 2020-0"+fmt.Sprint(1+i%9)+"-10")
		case 1:
			lines = append(lines, "event: 2020-05-10T10:00:00Z")
		case 2:
			lines = append(lines, "event: \"not a date\"")
		}
		lines = append(lines, "flag: "+fmt.Sprint(i%2 == 0))
		lines = append(lines, "keywords: ["+q(tags[i%len(tags)])+", "+q(tags[(i+2)%len(tags)])+"]")
		lines = append(lines, "tags: ["+q(tags[i%len(tags)])+", "+q(tags[(i+1)%len(tags)])+"]")
		lines = append(lines, fmt.Sprintf("tags_weight: %d", (i%4)*10))
		lines = append(lines, "categories: ["+q([]string{"one", "two", "Three"}[i%3])+"]")
		c[fmt.Sprintf("content/items/p%02d.md", i)] = fm(lines...) + fmt.Sprintf("\n## Heading %d\n\nText %d.\n\n## Common\n\nMore text for %d.\n", i%3, i, i)
		if i%4 == 0 {
			c[fmt.Sprintf("content/items/p%02d.th.md", i)] = fm("title: "+q(titles[(i+11)%len(titles)]), fmt.Sprintf("weight: %d", weights[(i+1)%len(weights)]), "date: 2021-01-01T00:00:00Z") + "\nไทย\n"
		}
	}
	thai := []string{"ขนมปัง", "ขนม", "ข้าวเหนียว", "ข้าว", "กล้วยทอด", "กล้วย", "ไก่", "ไข่", "เค้ก", "เค็ม", "แกง", "แก้ว", "โจ๊ก", "ใบ", "ไม้", "ก", "ข", "ฯลฯ", "ฯ", "ํา", "Latin"}
	for i, t := range thai {
		c[fmt.Sprintf("content/thai/t%02d.md", i)] = fm("title: "+q(t), fmt.Sprintf("weight: %d", i%3), "date: 2021-03-03") + "\nText.\n"
		c[fmt.Sprintf("content/thai/t%02d.th.md", i)] = fm("title: "+q(thai[len(thai)-1-i]), fmt.Sprintf("weight: %d", i%2), "date: 2021-03-03") + "\nข้อความ\n"
	}
	return c
}

const tiesTOML = `baseURL = "https://ties.example.org/"
title = "Ties"
defaultContentLanguage = "en"
[pagination]
pagerSize = 3
path = "seite"
[taxonomies]
tag = "tags"
category = "categories"
[languages.en]
weight = 1
[languages.th]
weight = 2
[params]
rating = 100
author = "Site Author"
[related]
threshold = 20
includeNewer = true
toLower = true
[[related.indices]]
name = "keywords"
weight = 100
[[related.indices]]
name = "tags"
weight = 80
cardinalityThreshold = 60
[[related.indices]]
name = "date"
weight = 10
pattern = "2006-01"
[[related.indices]]
name = "fragments"
type = "fragments"
applyFilter = true
weight = 50
`

// TiesSite is the ties content with en as the default language.
func TiesSite() psupport.Site {
	files := tiesContent()
	for k, v := range psupport.Layouts() {
		files[k] = v
	}
	return psupport.Site{Name: "ties", TOML: tiesTOML, Files: files}
}

// TiesThSite is the ties content with th first (weight 1) and the default
// related config.
func TiesThSite() psupport.Site {
	files := tiesContent()
	for k, v := range psupport.Layouts() {
		files[k] = v
	}
	toml := `baseURL = "https://th.example.org/sub/"
title = "Ties TH"
defaultContentLanguage = "th"
defaultContentLanguageInSubdir = true
[pagination]
pagerSize = 10
path = "หน้า Ü"
[taxonomies]
tag = "tags"
[languages.th]
weight = 1
[languages.en]
weight = 0
`
	return psupport.Site{Name: "ties-th", TOML: toml, Files: files}
}
