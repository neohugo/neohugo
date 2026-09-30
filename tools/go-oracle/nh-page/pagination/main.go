// Command pagination is the Go oracle for page.Paginate, the Pager and
// Paginator methods, the pagination URL factory, page.ResolvePagerSize and
// page.ToPagesGroup (resources/page/pagination.go, pagegroup.go) in
// crates/nh-page (Wave B task T12).
//
//	go run ./tools/go-oracle/nh-page/pagination [-root .] [-out rust/testdata/oracle/page/pagination]
//
// It runs itself again with `go run -overlay`: a hook in page.Paginate
// records every call of the real builds (csupport.Sites; the list layouts
// paginate every node page, so every list page of every site and output
// format is recorded: its TargetPathDescriptor, the paginated pages and the
// pager size; the sites use the default pager size and path, `pagerSize = 3`
// with `path = "seite"`, and a Thai path with a space). Each recorded call is
// replayed with its own size and with sizes 1, 3, 10 and 100, and with the
// pages grouped (GroupByDate, GroupBy Section, GroupByParam); every pager of
// every result is dumped (PageNumber, URL, Pages, PageGroups,
// NumberOfElements, HasPrev/HasNext, Prev/Next, First/Last, Pagers,
// PagerSize, TotalPages, TotalNumberOfElements). Errors: bad sizes, bad
// sequences. ResolvePagerSize with adversarial options.
//
// Descriptors are encoded like the paths oracle (the Rust test rebuilds the
// PathSpec, output formats and paths with the T11 test support). Output:
// <site>.json.gz per site and resolve.json.gz. Nothing depends on the
// platform.
package main

import (
	"context"
	"encoding/json"
	"flag"
	"fmt"
	"log"
	"path/filepath"
	"sync"

	"github.com/neohugo/neohugo/helpers"
	"github.com/neohugo/neohugo/output"
	"github.com/neohugo/neohugo/resources/page"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-page/csupport"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-page/psupport"
)

// installHooks is set by the overlay-added hook file.
var installHooks func(record func(page.TargetPathDescriptor, any, int))

const hookName = "zz_paginate_overlay.go"

const hookFile = `package main

import (
	"github.com/neohugo/neohugo/hugolib"
	"github.com/neohugo/neohugo/resources/page"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-page/csupport"
)

func init() {
	csupport.SetCurrent = hugolib.OracleSetCurrentSite
	installHooks = func(record func(page.TargetPathDescriptor, any, int)) {
		page.OracleHookPaginate = record
	}
}
`

var patches = append([]psupport.Patch{{
	File:   "resources/page/pagination.go",
	After:  "func Paginate(td TargetPathDescriptor, seq any, pagerSize int) (*Paginator, error) {",
	Insert: "\n\tif OracleHookPaginate != nil {\n\t\tOracleHookPaginate(td, seq, pagerSize)\n\t}\n",
	Append: "\n// OracleHookPaginate is set by the nh-page pagination oracle.\nvar OracleHookPaginate func(TargetPathDescriptor, any, int)\n",
}}, csupport.Patches...)

type call struct {
	td   page.TargetPathDescriptor
	seq  any
	size int
}

var (
	mu        sync.Mutex
	recording bool
	calls     []call
)

func record(td page.TargetPathDescriptor, seq any, size int) {
	mu.Lock()
	defer mu.Unlock()
	if recording {
		calls = append(calls, call{td, seq, size})
	}
}

func main() {
	root := flag.String("root", ".", "neohugo module root")
	out := flag.String("out", "rust/testdata/oracle/page/pagination", "output directory")
	flag.Parse()
	if !psupport.IsChild() {
		if err := psupport.RunOverlaid(*root, "./tools/go-oracle/nh-page/pagination", patches, hookName, hookFile, []string{"-root", *root, "-out", *out}); err != nil {
			log.Fatal(err)
		}
		return
	}
	if installHooks == nil {
		log.Fatal("the recording hook is not installed")
	}
	installHooks(record)
	outDir := psupport.OutDir(*root, *out)

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
	rc := resolveCases()
	if err := goval.WriteCasesGz(filepath.Join(outDir, "resolve.json.gz"), map[string]any{}, rc); err != nil {
		log.Fatal(err)
	}
	log.Printf("pagination: %d cases, %d resolve cases", total, len(rc))
}

// resolveCases runs ResolvePagerSize with options (the configured size is
// read without options).
func resolveCases() []map[string]any {
	opts := [][]any{
		{3}, {"4"}, {0}, {-1}, {"x"}, {1, 2}, {3.7}, {int64(9)}, {"0x10"}, {nil}, {true}, {uint8(5)}, {"  8"}, {1e3}, {-2.5},
	}
	var out []map[string]any
	for _, o := range opts {
		enc := []any{}
		for _, v := range o {
			enc = append(enc, goval.Encode(v))
		}
		out = append(out, map[string]any{"options": enc, "res": goval.Call(func() (any, error) {
			return page.ResolvePagerSize(nil, o...)
		})})
	}
	return out
}

type tables struct {
	t         *csupport.Table
	psIndex   map[*helpers.PathSpec]int
	pathspecs []any
	formats   []any
	fmtIndex  map[string]int
	paths     *psupport.PathTable
}

func (t *tables) format(f output.Format) int {
	d := psupport.FormatDump(f)
	b, err := json.Marshal(d)
	if err != nil {
		panic(err)
	}
	if i, ok := t.fmtIndex[string(b)]; ok {
		return i
	}
	t.fmtIndex[string(b)] = len(t.formats)
	t.formats = append(t.formats, d)
	return len(t.formats) - 1
}

// descriptor encodes d like the paths oracle.
func (t *tables) descriptor(d page.TargetPathDescriptor) map[string]any {
	ps, ok := t.psIndex[d.PathSpec]
	if !ok {
		panic("descriptor with an unknown PathSpec")
	}
	return map[string]any{
		"ps":                ps,
		"type":              t.format(d.Type),
		"kind":              d.Kind,
		"path":              t.paths.Add(d.Path),
		"section":           t.paths.Add(d.Section),
		"baseName":          goval.Str(d.BaseName),
		"prefixFilePath":    goval.Str(d.PrefixFilePath),
		"prefixLink":        goval.Str(d.PrefixLink),
		"forcePrefix":       d.ForcePrefix,
		"url":               goval.Str(d.URL),
		"addends":           goval.Str(d.Addends),
		"expandedPermalink": goval.Str(d.ExpandedPermalink),
		"uglyURLs":          d.UglyURLs,
	}
}

func pagerNum(p *page.Pager) int {
	if p == nil {
		return -1
	}
	return p.PageNumber()
}

func (t *tables) groups(g page.PagesGroup) any {
	if g == nil {
		return nil
	}
	out := []any{}
	for _, pg := range g {
		k := any(nil)
		switch v := pg.Key.(type) {
		case page.Page:
			k = map[string]any{"t": "page", "i": t.t.Add(v)}
		default:
			k = goval.Encode(v)
		}
		out = append(out, map[string]any{"key": k, "pages": t.t.List(pg.Pages)})
	}
	return out
}

// dump encodes a paginator through every Pager method.
func (t *tables) dump(p *page.Paginator) any {
	if p == nil {
		return nil
	}
	first := p.Pagers()[0]
	var pagers []any
	for _, pg := range p.Pagers() {
		pagers = append(pagers, map[string]any{
			"number":           pg.PageNumber(),
			"url":              goval.Str(pg.URL()),
			"pages":            t.t.List(pg.Pages()),
			"groups":           t.groups(pg.PageGroups()),
			"numberOfElements": pg.NumberOfElements(),
			"hasPrev":          pg.HasPrev(),
			"hasNext":          pg.HasNext(),
			"prev":             pagerNum(pg.Prev()),
			"next":             pagerNum(pg.Next()),
			"first":            pagerNum(pg.First()),
			"last":             pagerNum(pg.Last()),
			"string":           pg.String(),
		})
	}
	return map[string]any{
		"pagerSize":             first.PagerSize(),
		"totalPages":            first.TotalPages(),
		"totalNumberOfElements": first.TotalNumberOfElements(),
		"pagers":                pagers,
	}
}

func (t *tables) seq(v any) any {
	switch s := v.(type) {
	case nil:
		return map[string]any{"t": "nil"}
	case page.Pages:
		return map[string]any{"t": "pages", "pages": t.t.List(s)}
	case page.PagesGroup:
		return map[string]any{"t": "groups", "groups": t.groups(s)}
	case []any:
		var items []any
		for _, it := range s {
			switch x := it.(type) {
			case page.Page:
				items = append(items, map[string]any{"t": "page", "i": t.t.Add(x)})
			case page.PageGroup:
				items = append(items, map[string]any{"t": "group", "group": t.groups(page.PagesGroup{x})})
			default:
				items = append(items, goval.Encode(x))
			}
		}
		return map[string]any{"t": "any", "items": items}
	}
	return map[string]any{"t": "value", "v": goval.Encode(v)}
}

func runSite(s psupport.Site, outDir string) (int, error) {
	mu.Lock()
	calls = nil
	recording = true
	mu.Unlock()
	b, err := psupport.Build(s)
	mu.Lock()
	recording = false
	recs := calls
	calls = nil
	mu.Unlock()
	if err != nil {
		return 0, err
	}
	h := b.H
	t := &tables{
		t:        csupport.NewTable(h),
		psIndex:  map[*helpers.PathSpec]int{},
		fmtIndex: map[string]int{},
		paths:    psupport.NewPathTable(h.Configs.ContentPathParser),
	}
	for i, st := range h.Sites {
		t.psIndex[st.PathSpec] = i
		d := psupport.PathSpecDump(st.Conf, st.PathSpec)
		pg := st.Conf.Pagination()
		d["pagination"] = map[string]any{"pagerSize": pg.PagerSize, "path": goval.Str(pg.Path), "disableAliases": pg.DisableAliases}
		t.pathspecs = append(t.pathspecs, d)
	}
	// The table numbering follows the recorded calls (deterministic).
	for _, r := range recs {
		if ps, ok := r.seq.(page.Pages); ok {
			t.t.List(ps)
		}
	}

	var cases []map[string]any
	seen := map[string]bool{}
	add := func(c map[string]any) {
		k, err := json.Marshal(c)
		if err != nil {
			panic(err)
		}
		if !seen[string(k)] {
			seen[string(k)] = true
			cases = append(cases, c)
		}
	}
	run := func(src string, td page.TargetPathDescriptor, seq any, size int) {
		c := map[string]any{"src": src, "td": t.descriptor(td), "seq": t.seq(seq), "size": size}
		c["res"] = goval.CallRaw(func() (any, error) {
			p, err := page.Paginate(td, seq, size)
			if err != nil {
				return nil, err
			}
			return t.dump(p), nil
		})
		add(c)
	}
	for ri, r := range recs {
		run("build", r.td, r.seq, r.size)
		for _, size := range []int{1, 3, 10, 100} {
			run("size", r.td, r.seq, size)
		}
		ps, ok := r.seq.(page.Pages)
		if !ok || len(ps) == 0 || ri%4 != 0 {
			continue
		}
		si := t.t.SiteIndex(ps[0].Lang())
		csupport.SetCurrent(h, si)
		cur := h.Sites[si].Language().Lang
		if cur == "en" || cur == "th" {
			if g, err := ps.GroupByDate("2006"); err == nil {
				run("groups", r.td, g, 3)
				run("groups", r.td, g, 2)
			}
		}
		if g, err := ps.GroupBy(context.Background(), "Section"); err == nil {
			run("groups", r.td, g, 4)
		}
		if g, err := ps.GroupByParam("rating"); err == nil && g != nil {
			run("groups", r.td, g, 2)
		}
		// Groups with a nil key: every page starts a group.
		run("groups", r.td, page.PagesGroup{{Key: nil, Pages: ps}}, 3)
		if ri == 0 {
			run("err", r.td, ps, 0)
			run("err", r.td, ps, -1)
			run("err", r.td, "not pages", 3)
			run("err", r.td, nil, 3)
			run("err", r.td, []any{ps[0], ps[0]}, 3)
			run("err", r.td, []any{page.PageGroup{Key: "a", Pages: ps}, "x"}, 3)
			run("err", r.td, []any{page.PageGroup{Key: "a", Pages: ps}}, 3)
			run("err", r.td, []any{}, 3)
			run("err", r.td, page.Pages{}, 3)
		}
	}
	header := map[string]any{
		"site":      s.Name,
		"sites":     t.t.SitesHeader(),
		"pages":     t.t.Entries,
		"pathspecs": t.pathspecs,
		"formats":   t.formats,
		"paths":     t.paths.Entries,
		"parser":    t.paths.Describe(h.Configs.ContentPathParser),
	}
	if t.paths.Mismatches > 0 {
		return 0, fmt.Errorf("%s: %d paths do not re-parse to the same path", s.Name, t.paths.Mismatches)
	}
	log.Printf("%s: %d recorded calls, %d cases", s.Name, len(recs), len(cases))
	return len(cases), goval.WriteCasesGz(filepath.Join(outDir, s.Name+".json.gz"), header, cases)
}
