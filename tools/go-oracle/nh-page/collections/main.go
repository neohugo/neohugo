// Command collections is the Go oracle for the page collection functions of
// resources/page (pages_sort.go, pages_cache.go, pagegroup.go,
// pages_prev_next.go, pages_sort_search.go, pages_language_merge.go,
// taxonomy.go, weighted.go) in crates/nh-page (Wave B task T12).
//
//	go run ./tools/go-oracle/nh-page/collections [-root .] [-out crates/nh-page/tests/fixtures/collections]
//
// It runs itself again with `go run -overlay` (csupport.Patches) to be able
// to set hugolib's current site, whose language gives the collator of the
// title sorts and tie-breaks and the time formatter of the date groups.
//
// The inputs are the page lists of real builds (csupport.Sites: this
// repository's docs/ and hugolib/testsite, two psupport synthetic sites and
// two sites built to stress ties): the sites' collections, every node's
// Pages/RegularPages, GetTerms lists (ordinals), taxonomy entries (weight0),
// deterministic shuffles and cross-language lists. For every list and every
// site as the current site, after page.Clear():
//
//   - ByWeight, ByTitle, ByLinkTitle, ByDate, ByPublishDate, ByExpiryDate,
//     ByLastmod, ByLanguage, ByLength, Reverse, ByParam (several keys),
//     Limit, SortByDefault, SortByLanguage;
//   - GroupBy (method keys of every result kind, errors), GroupByParam,
//     GroupByDate/PublishDate/ExpiryDate/Lastmod/ParamDate, in every order;
//   - Next/Prev; MergeByLanguage;
//   - cache sequences (a hit returns the order of the first computation).
//
// Taxonomies: Site.Taxonomies (terms, weighted entries), Alphabetical,
// ByCount, Get, Count, Page, WeightedPages.Sort/Next/Prev.
//
// Go orders that come from map iteration (GroupBy keys of kinds other than
// int and string, TaxonomyArray) are random in Go: those results are written
// in a canonical order and marked "unordered"; runs of keys the collator
// ranks equal are marked "ties". Everything else is written as Go returned it.
//
// Output: <site>.json.gz per site and methods.json.gz (the page.Page method
// set as GroupBy sees it). Nothing depends on the platform.
package main

import (
	"context"
	"flag"
	"fmt"
	"log"
	"path/filepath"
	"reflect"
	"sort"
	"time"

	"github.com/neohugo/neohugo/langs"
	"github.com/neohugo/neohugo/resources/page"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-page/csupport"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-page/psupport"
)

func main() {
	root := flag.String("root", ".", "neohugo module root")
	out := flag.String("out", "crates/nh-page/tests/fixtures/collections", "output directory")
	flag.Parse()
	if !psupport.IsChild() {
		if err := psupport.RunOverlaid(*root, "./tools/go-oracle/nh-page/collections", csupport.Patches, csupport.HookName, csupport.HookFile, []string{"-root", *root, "-out", *out}); err != nil {
			log.Fatal(err)
		}
		return
	}
	if csupport.SetCurrent == nil {
		log.Fatal("the current site hook is not installed")
	}
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
	if err := goval.WriteCasesGz(filepath.Join(outDir, "methods.json.gz"), map[string]any{}, methodCases()); err != nil {
		log.Fatal(err)
	}
	log.Printf("collections: %d cases", total)
}

// methodCases describes the page.Page interface's methods as GroupBy uses
// them.
func methodCases() []map[string]any {
	t := reflect.TypeOf((*page.Page)(nil)).Elem()
	ctxT := reflect.TypeOf((*context.Context)(nil)).Elem()
	var out []map[string]any
	for i := range t.NumMethod() {
		m := t.Method(i)
		mt := m.Type
		o := mt.Out(0)
		key := "other"
		switch {
		case !o.Comparable():
			key = "nc:" + o.String()
		case o.Kind() >= reflect.Int && o.Kind() <= reflect.Int64:
			key = "int"
		case o.Kind() == reflect.String:
			key = "string"
		}
		call := "ok"
		switch {
		case mt.NumIn() > 1:
			call = "notsupported"
		case mt.NumIn() == 1 && mt.In(0) != ctxT && !mt.IsVariadic():
			call = "toofew"
		}
		out = append(out, map[string]any{"name": m.Name, "key": key, "call": call})
	}
	return out
}

type runner struct {
	t     *csupport.Table
	cases []map[string]any
}

func (r *runner) add(c map[string]any) { r.cases = append(r.cases, c) }

// res encodes a Pages result.
func (r *runner) pages(ps page.Pages) any { return r.t.List(ps) }

func (r *runner) call(f func() (any, error)) map[string]any {
	return goval.CallRaw(f)
}

// keyEnc encodes a group key.
func (r *runner) keyEnc(k any) any {
	if p, ok := k.(page.Page); ok {
		return map[string]any{"t": "page", "i": r.t.Add(p)}
	}
	if k == nil {
		return goval.Encode(nil)
	}
	switch k.(type) {
	case bool, string, int, int64, float64, time.Time:
		return goval.Encode(k)
	}
	v := reflect.ValueOf(k)
	if v.Kind() == reflect.String {
		return map[string]any{"t": fmt.Sprintf("%T", k), "s": goval.Str(v.String())}
	}
	if v.Kind() == reflect.Ptr && v.IsNil() {
		return map[string]any{"t": "nil:" + fmt.Sprintf("%T", k)}
	}
	return map[string]any{"t": fmt.Sprintf("%T", k), "opaque": true}
}

// groups encodes a PagesGroup; unordered sorts the groups canonically.
func (r *runner) groups(g page.PagesGroup, kind string, cur *langs.Language) any {
	if g == nil {
		return nil
	}
	type enc struct {
		key   any
		sk    string
		pages any
	}
	var es []enc
	for _, pg := range g {
		k := r.keyEnc(pg.Key)
		sk := fmt.Sprint(k)
		if p, ok := pg.Key.(page.Page); ok {
			// Independent of the table numbering (Go's order is random here).
			sk = p.Lang() + "|" + p.Kind() + "|" + p.Path()
		}
		es = append(es, enc{k, sk, r.t.List(pg.Pages)})
	}
	res := map[string]any{}
	switch kind {
	case "other":
		sort.SliceStable(es, func(i, j int) bool { return es[i].sk < es[j].sk })
		res["unordered"] = true
	case "string":
		coll := langs.GetCollator2(cur)
		coll.Lock()
		keys := map[string]string{}
		for i, pg := range g {
			keys[es[i].sk] = reflect.ValueOf(pg.Key).String()
		}
		if sortTieRuns(es, func(a, b enc) bool { return coll.CompareStrings(keys[a.sk], keys[b.sk]) == 0 }, func(e enc) string { return e.sk }) {
			res["ties"] = true
		}
		coll.Unlock()
	}
	var out []any
	for _, e := range es {
		out = append(out, map[string]any{"key": e.key, "pages": e.pages})
	}
	res["groups"] = out
	return res
}

func keyKind(v any) string {
	switch reflect.ValueOf(v).Kind() {
	case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
		return "int"
	case reflect.String:
		return "string"
	}
	return "other"
}

var groupByKeys = []string{
	"Section", "Kind", "Type", "Weight", "Title", "LinkTitle", "Lang", "Layout",
	"BundleType", "Slug", "Draft", "IsPage", "IsNode", "Date", "Parent",
	"Params", "Aliases", "Eq", "Param", "Paginate", "Nope", "title",
}

var paramKeys = []string{"rating", "mixed", "score", "author", "missing", "flag", "Rating", "event", "tags", "weight"}

func runSite(s psupport.Site, outDir string) (int, error) {
	b, err := psupport.Build(s)
	if err != nil {
		return 0, err
	}
	h := b.H
	t := csupport.NewTable(h)
	r := &runner{t: t}
	ctx := context.Background()

	lists := csupport.Lists(h)
	var listsHeader []any
	// Every listed page is in the table before any case runs, so that no
	// Go map order reaches the numbering.
	for _, l := range lists {
		listsHeader = append(listsHeader, map[string]any{"name": l.Name, "pages": t.List(l.Pages)})
	}
	for li, l := range lists {
		for ci := range h.Sites {
			if ci > 0 && len(h.Sites[ci].RegularPages()) == 0 {
				continue
			}
			_ = page.Clear()
			csupport.SetCurrent(h, ci)
			cur := h.Sites[ci].Language()
			p := l.Pages
			base := func(op string, args ...any) map[string]any {
				c := map[string]any{"list": li, "current": ci, "op": op}
				if len(args) > 0 {
					c["args"] = args
				}
				return c
			}
			sorts := map[string]func() page.Pages{
				"ByWeight": p.ByWeight, "ByTitle": p.ByTitle, "ByLinkTitle": p.ByLinkTitle,
				"ByDate": p.ByDate, "ByPublishDate": p.ByPublishDate, "ByExpiryDate": p.ByExpiryDate,
				"ByLastmod": p.ByLastmod, "ByLanguage": p.ByLanguage, "Reverse": p.Reverse,
				"ByLength": func() page.Pages { return p.ByLength(ctx) },
			}
			var names []string
			for k := range sorts {
				names = append(names, k)
			}
			sort.Strings(names)
			for _, op := range names {
				c := base(op)
				c["res"] = r.pages(sorts[op]())
				r.add(c)
			}
			for _, k := range paramKeys {
				c := base("ByParam", k)
				c["res"] = r.pages(p.ByParam(k))
				r.add(c)
			}
			for _, n := range []int{0, 1, 5} {
				c := base("Limit", n)
				c["res"] = r.pages(p.Limit(n))
				r.add(c)
			}
			{
				cp := append(page.Pages(nil), p...)
				page.SortByDefault(cp)
				c := base("SortByDefault")
				c["res"] = r.pages(cp)
				r.add(c)
				cp = append(page.Pages(nil), p...)
				page.SortByLanguage(cp)
				c = base("SortByLanguage")
				c["res"] = r.pages(cp)
				r.add(c)
			}
			for _, key := range groupByKeys {
				for _, order := range [][]string{nil, {"desc"}, {"REV"}} {
					c := base("GroupBy", append([]any{key}, anys(order)...)...)
					c["res"] = r.call(func() (any, error) {
						g, err := p.GroupBy(ctx, key, order...)
						if err != nil {
							return nil, err
						}
						return r.groups(g, methodKind(key), cur), nil
					})
					r.add(c)
				}
			}
			for _, key := range paramKeys {
				for _, order := range [][]string{nil, {"desc"}} {
					c := base("GroupByParam", append([]any{key}, anys(order)...)...)
					c["res"] = r.call(func() (any, error) {
						g, err := p.GroupByParam(key, order...)
						if err != nil {
							return nil, err
						}
						kind := "other"
						if len(g) > 0 {
							kind = keyKind(g[0].Key)
						}
						return r.groups(g, kind, cur), nil
					})
					r.add(c)
				}
			}
			if cur.Lang == "en" || cur.Lang == "th" {
				dateOrders := [][]string{nil, {"asc"}, {"desc"}, {"rev"}, {"Reverse"}}
				for _, format := range []string{"2006", "January 2006", "2006-01-02", ":date_long", "Mon"} {
					for _, order := range dateOrders {
						c := base("GroupByDate", append([]any{format}, anys(order)...)...)
						c["res"] = r.call(func() (any, error) {
							g, err := p.GroupByDate(format, order...)
							return r.groups(g, "", cur), err
						})
						r.add(c)
					}
				}
				for _, op := range []string{"GroupByPublishDate", "GroupByExpiryDate", "GroupByLastmod"} {
					for _, order := range dateOrders {
						c := base(op, append([]any{"2006-01"}, anys(order)...)...)
						c["res"] = r.call(func() (any, error) {
							var g page.PagesGroup
							var err error
							switch op {
							case "GroupByPublishDate":
								g, err = p.GroupByPublishDate("2006-01", order...)
							case "GroupByExpiryDate":
								g, err = p.GroupByExpiryDate("2006-01", order...)
							default:
								g, err = p.GroupByLastmod("2006-01", order...)
							}
							return r.groups(g, "", cur), err
						})
						r.add(c)
					}
				}
				for _, key := range []string{"event", "date", "missing", "rating"} {
					for _, order := range [][]string{nil, {"asc"}} {
						c := base("GroupByParamDate", append([]any{key, "2006-01"}, anys(order)...)...)
						c["res"] = r.call(func() (any, error) {
							g, err := p.GroupByParamDate(key, "2006-01", order...)
							return r.groups(g, "", cur), err
						})
						r.add(c)
					}
				}
			}
			for _, i := range []int{0, 1, len(p) / 2, len(p) - 1} {
				if i < 0 || i >= len(p) {
					continue
				}
				c := base("NextPrev", t.Add(p[i]))
				c["res"] = []any{t.Add(p.Next(p[i])), t.Add(p.Prev(p[i]))}
				r.add(c)
			}
		}
	}

	// Cache sequences: the first computation wins until page.Clear.
	if len(h.Sites) > 1 {
		for li, l := range lists {
			if li%3 != 0 {
				continue
			}
			p := l.Pages
			_ = page.Clear()
			csupport.SetCurrent(h, 0)
			a := p.ByTitle()
			b := p.ByLinkTitle()
			csupport.SetCurrent(h, 1)
			a2 := p.ByTitle()
			b2 := p.ByLinkTitle()
			_ = page.Clear()
			a3 := p.ByTitle()
			b3 := p.ByLinkTitle()
			r.add(map[string]any{"list": li, "op": "cache", "res": []any{r.pages(a), r.pages(b), r.pages(a2), r.pages(b2), r.pages(a3), r.pages(b3)}})
		}
	}

	// MergeByLanguage.
	if len(h.Sites) > 1 {
		for i := range h.Sites {
			for j := range h.Sites {
				if i == j {
					continue
				}
				for _, sh := range []uint64{0, 7} {
					p1, p2 := h.Sites[i].RegularPages(), h.Sites[j].RegularPages()
					if sh > 0 {
						p1, p2 = csupport.Shuffle(p1, sh), csupport.Shuffle(p2, sh+1)
					}
					_ = page.Clear()
					csupport.SetCurrent(h, i)
					r.add(map[string]any{
						"op": "MergeByLanguage", "current": i,
						"p1": t.List(p1), "p2": t.List(p2),
						"res": t.List(p1.MergeByLanguage(p2)),
					})
				}
			}
		}
	}

	// Taxonomies.
	var taxonomies []any
	for si, site := range h.Sites {
		tl := site.Taxonomies()
		var plurals []string
		for k := range tl {
			plurals = append(plurals, k)
		}
		sort.Strings(plurals)
		for _, plural := range plurals {
			tax := tl[plural]
			var terms []string
			for k := range tax {
				terms = append(terms, k)
			}
			sort.Strings(terms)
			var te []any
			for _, term := range terms {
				wp := tax[term]
				var entries []any
				for _, w := range wp {
					entries = append(entries, []any{t.Add(w.Page), w.Weight})
				}
				owner := -1
				if pg := wp.Page(); pg != nil {
					owner = t.Add(pg)
				}
				te = append(te, map[string]any{"term": goval.Str(term), "entries": entries, "owner": owner})
			}
			tc := map[string]any{"site": si, "plural": plural, "terms": te}
			tc["page"] = t.Add(tax.Page())
			tc["byCount"] = orderedNames(tax.ByCount())
			var alpha []any
			for ci := range h.Sites {
				csupport.SetCurrent(h, ci)
				al := tax.Alphabetical()
				coll := langs.GetCollator1(h.Sites[ci].Language())
				coll.Lock()
				ties := sortTieRuns(al, func(a, b page.OrderedTaxonomyEntry) bool { return coll.CompareStrings(a.Name, b.Name) == 0 }, func(e page.OrderedTaxonomyEntry) string { return e.Name })
				coll.Unlock()
				alpha = append(alpha, map[string]any{"current": ci, "names": orderedNames(al), "ties": ties})
			}
			tc["alphabetical"] = alpha
			var gets []any
			for _, term := range terms {
				for _, k := range []string{term, upper(term)} {
					gets = append(gets, []any{goval.Str(k), len(tax.Get(k)), tax.Count(k)})
				}
			}
			tc["get"] = gets
			// WeightedPages.Sort of a reversed copy and Next/Prev.
			var sorts []any
			for _, term := range terms {
				wp := tax[term]
				cp := make(page.WeightedPages, len(wp))
				for i := range wp {
					cp[i] = wp[len(wp)-1-i]
				}
				cp.Sort()
				var order []any
				for _, w := range cp {
					order = append(order, t.Add(w.Page))
				}
				var np []any
				for _, w := range wp {
					np = append(np, []any{t.Add(wp.Next(w.Page)), t.Add(wp.Prev(w.Page))})
				}
				sorts = append(sorts, map[string]any{"term": goval.Str(term), "sorted": order, "nextPrev": np})
			}
			tc["weighted"] = sorts
			taxonomies = append(taxonomies, tc)
		}
	}

	header := map[string]any{
		"site":       s.Name,
		"sites":      t.SitesHeader(),
		"lists":      listsHeader,
		"taxonomies": taxonomies,
	}
	header["pages"] = t.Entries
	if err := goval.WriteCasesGz(filepath.Join(outDir, s.Name+".json.gz"), header, r.cases); err != nil {
		return 0, err
	}
	return len(r.cases), nil
}

func upper(s string) string {
	b := []byte(s)
	for i, c := range b {
		if c >= 'a' && c <= 'z' {
			b[i] = c - 32
		}
	}
	return string(b)
}

func orderedNames(o page.OrderedTaxonomy) []any {
	out := []any{}
	for _, e := range o {
		out = append(out, []any{goval.Str(e.Name), e.Count()})
	}
	return out
}

func anys(ss []string) []any {
	var out []any
	for _, s := range ss {
		out = append(out, s)
	}
	return out
}

// methodKind is the sortKeys kind of the result type of the page.Page
// method name ("other" when there is no such method).
func methodKind(name string) string {
	m, ok := reflect.TypeOf((*page.Page)(nil)).Elem().MethodByName(name)
	if !ok {
		return "other"
	}
	switch k := m.Type.Out(0).Kind(); {
	case k >= reflect.Int && k <= reflect.Int64:
		return "int"
	case k == reflect.String:
		return "string"
	}
	return "other"
}

// sortTieRuns sorts the runs of adjacent entries that less reports equal
// (both directions false) by their canonical key sk (Go's order within such
// runs is random); it reports whether there was a run.
func sortTieRuns[T any](xs []T, equal func(a, b T) bool, sk func(T) string) bool {
	ties := false
	for i := 0; i < len(xs); {
		j := i + 1
		for j < len(xs) && equal(xs[j-1], xs[j]) {
			j++
		}
		if j-i > 1 {
			ties = true
			run := xs[i:j]
			sort.SliceStable(run, func(a, b int) bool { return sk(run[a]) < sk(run[b]) })
		}
		i = j
	}
	return ties
}
