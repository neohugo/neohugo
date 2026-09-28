// Command nodeshift is the Go oracle for crates/nh-doctree's NodeShiftTree with a shifter that
// mirrors Hugo's contentNodeShifter (dtcommon.Shifter), languages en and th.
//
//	go run ./tools/go-oracle/nh-doctree/nodeshift [-root .] [-out crates/nh-doctree/tests/fixtures/nodeshift/nodeshift.json.gz]
//
// Every scenario is a sequence of operations with their results: InsertIntoValuesDimension,
// InsertIntoCurrentDimension, InsertRawWithLock, Delete, DeletePrefix, DeleteAll,
// DeletePrefixAll, Get/Has, GetRaw, LongestPrefix (exact or not, with Hugo's isBranch predicate
// or none, with the path.Dir retry), LongestPrefixAll, ForEeachInDimension, walks (per
// language, prefix, NoShift, Exact), WalkPrefixRaw and Len.
//
// Scenarios: the pages and the resources of this repository's sites (with synthetic Thai
// translations) queried the way Hugo queries them, and random operation sequences over small
// key pools.
package main

import (
	"context"
	"flag"
	"fmt"
	"log"
	"path"
	"strings"

	"github.com/neohugo/neohugo/hugolib/doctree"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-doctree/dtcommon"
)

type node = dtcommon.Node

type op = map[string]any

type recorder struct {
	tree *doctree.NodeShiftTree[node]
	ops  []op
	seq  int
}

func newRecorder() *recorder {
	return &recorder{tree: doctree.New(doctree.Config[node]{Shifter: &dtcommon.Shifter{}})}
}

func (r *recorder) newTV(lang int, res, isPage, branch bool) *dtcommon.TV {
	r.seq++
	kind := "p"
	if res {
		kind = "r"
	}
	return &dtcommon.TV{ID: fmt.Sprintf("%s%d", kind, r.seq), Lang: lang, Res: res, IsPage: isPage, Branch: branch}
}

func (r *recorder) shape(lang int) *doctree.NodeShiftTree[node] {
	return r.tree.Shape(0, lang)
}

func (r *recorder) ins(k string, v *dtcommon.TV) {
	u, e, updated := r.tree.InsertIntoValuesDimensionWithLock(k, v)
	r.ops = append(r.ops, op{"o": "ins", "k": k, "v": v.Def(), "r": []any{dtcommon.Repr(u), dtcommon.Repr(e), updated}})
}

func (r *recorder) insc(k string, lang int, v *dtcommon.TV) {
	u, e, updated := r.shape(lang).InsertIntoCurrentDimension(k, v)
	r.ops = append(r.ops, op{"o": "insc", "k": k, "d": lang, "v": v.Def(), "r": []any{dtcommon.Repr(u), dtcommon.Repr(e), updated}})
}

func (r *recorder) insr(k string, v *dtcommon.TV) {
	old, existed := r.tree.InsertRawWithLock(k, v)
	o, _ := old.(node)
	r.ops = append(r.ops, op{"o": "insr", "k": k, "v": v.Def(), "r": []any{dtcommon.Repr(o), existed}})
}

func (r *recorder) del(k string, lang int) {
	d, ok := r.shape(lang).Delete(k)
	r.ops = append(r.ops, op{"o": "del", "k": k, "d": lang, "r": []any{dtcommon.Repr(d), ok}})
}

func (r *recorder) delp(k string, lang int) {
	n := r.shape(lang).DeletePrefix(k)
	r.ops = append(r.ops, op{"o": "delp", "k": k, "d": lang, "n": n})
}

func (r *recorder) delall(k string) {
	r.tree.DeleteAll(k)
	r.ops = append(r.ops, op{"o": "delall", "k": k})
}

func (r *recorder) delpall(k string) {
	n := r.tree.DeletePrefixAll(k)
	r.ops = append(r.ops, op{"o": "delpall", "k": k, "n": n})
}

func (r *recorder) get(k string, lang int) {
	t := r.shape(lang)
	r.ops = append(r.ops, op{"o": "get", "k": k, "d": lang, "r": dtcommon.Repr(t.Get(k)), "h": t.Has(k)})
}

func (r *recorder) raw(k string) {
	v, ok := r.tree.GetRaw(k)
	r.ops = append(r.ops, op{"o": "raw", "k": k, "r": dtcommon.Repr(v), "f": ok})
}

func (r *recorder) lp(s string, lang int, exact bool, pred int) {
	var p func(node) bool
	switch pred {
	case 1:
		p = dtcommon.IsBranch
	case 2:
		p = func(n node) bool { return !dtcommon.IsBranch(n) }
	}
	k, v := r.shape(lang).LongestPrefix(s, exact, p)
	var res any
	if v != nil {
		res = []any{k, dtcommon.Repr(v)}
	} else if k != "" {
		log.Fatalf("LongestPrefix(%q) returned key %q without a value", s, k)
	}
	r.ops = append(r.ops, op{"o": "lp", "k": s, "d": lang, "x": exact, "p": pred, "r": res})
}

func (r *recorder) lpa(s string) {
	k, found := r.tree.LongestPrefixAll(s)
	var res any
	if found {
		res = k
	}
	r.ops = append(r.ops, op{"o": "lpa", "k": s, "r": res})
}

func (r *recorder) fe(k string, stopAt int) {
	res := []string{}
	r.tree.ForEeachInDimension(k, doctree.DimensionLanguage.Index(), func(n node) bool {
		res = append(res, dtcommon.Repr(n))
		return len(res) == stopAt
	})
	r.ops = append(r.ops, op{"o": "fe", "k": k, "n": stopAt, "r": res})
}

func (r *recorder) walk(lang int, prefix string, noShift, exact bool) {
	res := [][]any{}
	w := &doctree.NodeShiftTreeWalker[node]{
		Tree:    r.shape(lang),
		Prefix:  prefix,
		NoShift: noShift,
		Exact:   exact,
		Handle: func(s string, n node, match doctree.DimensionFlag) (bool, error) {
			res = append(res, []any{s, dtcommon.Repr(n), int(match)})
			return false, nil
		},
	}
	if err := w.Walk(context.Background()); err != nil {
		log.Fatal(err)
	}
	r.ops = append(r.ops, op{"o": "walk", "d": lang, "k": prefix, "ns": noShift, "x": exact, "r": res})
}

func (r *recorder) wpr(prefix string) {
	res := [][]any{}
	r.tree.WalkPrefixRaw(prefix, func(s string, n node) bool {
		res = append(res, []any{s, dtcommon.Repr(n)})
		return false
	})
	r.ops = append(r.ops, op{"o": "wpr", "k": prefix, "r": res})
}

func (r *recorder) length() {
	r.ops = append(r.ops, op{"o": "len", "n": r.tree.Len()})
}

type scenario struct {
	Name string `json:"name"`
	Ops  []op   `json:"ops"`
}

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "crates/nh-doctree/tests/fixtures/nodeshift/nodeshift.json.gz", "output file (gzip)")
	flag.Parse()

	cfs, err := dtcommon.ContentFiles(*root)
	if err != nil {
		log.Fatal(err)
	}

	var scenarios []scenario
	scenarios = append(scenarios, hugoPages(cfs), hugoResources(cfs))
	for seed := uint64(1); seed <= 60; seed++ {
		scenarios = append(scenarios, random(seed))
	}
	if err := dtcommon.WriteJSONGz(*out, map[string]any{"scenarios": scenarios}); err != nil {
		log.Fatal(err)
	}
}

// rootSections returns the distinct first path segments of keys, as "/section".
func rootSections(keys []string) []string {
	var out []string
	seen := map[string]bool{}
	for _, k := range keys {
		if k == "" {
			continue
		}
		s := "/" + strings.SplitN(k[1:], "/", 2)[0]
		if !seen[s] {
			seen[s] = true
			out = append(out, s)
		}
	}
	return out
}

// hugoPages builds treePages from the repository's content pages (plus Thai translations of
// every third page and Thai-only pages) and runs Hugo's page queries in both languages.
func hugoPages(cfs []dtcommon.ContentFile) scenario {
	r := newRecorder()
	var keys, resKeys []string
	i := 0
	for _, cf := range cfs {
		if !cf.Page {
			resKeys = append(resKeys, cf.Key)
			continue
		}
		keys = append(keys, cf.Key)
		r.ins(cf.Key, r.newTV(cf.Lang, false, false, cf.Branch))
		if cf.Site == "docs" {
			if i%3 == 0 {
				r.ins(cf.Key, r.newTV(1, false, false, cf.Branch))
			}
			if i%7 == 0 && cf.Key != "" {
				k := cf.Key + "-th"
				keys = append(keys, k)
				r.ins(k, r.newTV(1, false, false, false))
			}
			i++
		}
	}
	// A page inserted twice in the same language (Hugo warns about duplicate content paths).
	r.ins(keys[5], r.newTV(0, false, false, false))
	r.length()

	sections := rootSections(keys)
	for lang := range dtcommon.NumLanguages {
		for _, ns := range []bool{false, true} {
			for _, x := range []bool{false, true} {
				r.walk(lang, "", ns, x)
			}
		}
		for _, s := range sections {
			r.walk(lang, s+"/", false, false)
			r.walk(lang, s, false, true)
		}
		for _, k := range keys {
			r.get(k, lang)
			if k != "" {
				// CurrentSection / Parent lookups: the directory, branch predicate or none.
				r.lp(path.Dir(k), lang, true, 1)
				r.lp(path.Dir(k), lang, true, 0)
			}
			r.lp(k+"/x", lang, false, 2)
		}
		for _, k := range resKeys {
			// Resource ownership (applyAggregates, forEachResourceInPage).
			r.lp(k, lang, true, 0)
			r.lp(path.Dir(k), lang, false, 1)
		}
		r.get("/", lang)
		r.get("/does-not-exist", lang)
	}
	for _, k := range resKeys {
		for s := path.Dir(k); ; s = path.Dir(s) {
			r.lpa(s)
			if s == "/" {
				break
			}
		}
	}
	for _, k := range keys {
		r.fe(k, 0)
		r.fe(k, 1)
		r.raw(k)
	}
	for _, s := range sections {
		r.wpr(s + "/")
	}

	// Deletes: Thai from every fifth page, English from every eleventh, a section in Thai.
	for j, k := range keys {
		if j%5 == 0 {
			r.del(k, 1)
		}
		if j%11 == 0 {
			r.del(k, 0)
		}
	}
	if len(sections) > 2 {
		r.delp(sections[2]+"/", 1)
		r.delp(sections[1], 0)
	}
	r.length()
	r.wpr("")
	for lang := range dtcommon.NumLanguages {
		r.walk(lang, "", false, false)
		for _, k := range keys {
			r.get(k, lang)
		}
	}
	return scenario{Name: "hugo-pages", Ops: r.ops}
}

// hugoResources builds treeResources from the repository's resources (files in English, every
// fourth also in Thai; content files in leaf bundles as page resources in both languages) and
// runs the per-page resource walks of forEachResourceInPage in both languages, then Hugo's
// assembleResources insertion of the alternative language version into the current dimension.
func hugoResources(cfs []dtcommon.ContentFile) scenario {
	r := newRecorder()
	var keys, owners []string
	seen := map[string]bool{}
	for j, cf := range cfs {
		if cf.Page {
			owners = append(owners, cf.Key)
			continue
		}
		keys = append(keys, cf.Key)
		r.ins(cf.Key, r.newTV(cf.Lang, true, cf.IsPage, false))
		if cf.IsPage || j%4 == 0 {
			r.ins(cf.Key, r.newTV(1-cf.Lang, true, cf.IsPage, false))
		}
		seen[cf.Key] = true
	}
	r.length()
	for lang := range dtcommon.NumLanguages {
		for _, x := range []bool{false, true} {
			r.walk(lang, "", false, x)
			for _, o := range owners {
				r.walk(lang, o+"/", false, x)
			}
		}
		for _, k := range keys {
			r.get(k, lang)
		}
	}
	// assembleResources: resources that matched from another language are cloned into the
	// current dimension.
	for lang := range dtcommon.NumLanguages {
		for _, k := range keys {
			if !r.shape(lang).Has(k) {
				r.insc(k, lang, r.newTV(lang, true, false, false))
			}
		}
		r.walk(lang, "", false, true)
	}
	r.wpr("")
	for _, k := range keys {
		r.fe(k, 0)
	}
	r.length()
	return scenario{Name: "hugo-resources", Ops: r.ops}
}

// random runs a random operation sequence over a small pool of keys, so that the same keys are
// inserted, merged across languages, deleted and queried many times.
func random(seed uint64) scenario {
	rnd := dtcommon.NewRand(seed)
	r := newRecorder()
	segs := []string{"a", "b", "ab", "a-b", "a.b", "ขนม", "c"}
	var pool []string
	pool = append(pool, "", "/")
	for range 10 + rnd.Intn(20) {
		k := ""
		for range 1 + rnd.Intn(4) {
			k += "/" + dtcommon.Pick(rnd, segs)
		}
		pool = append(pool, k)
	}
	pool = dtcommon.Dedupe(pool)
	queries := append([]string{}, pool...)
	for _, k := range pool {
		queries = append(queries, k+"/x", k+"x", k+"/", k+"//")
	}
	res := seed%3 == 0
	for range 150 + rnd.Intn(150) {
		k := dtcommon.Pick(rnd, pool)
		lang := rnd.Intn(dtcommon.NumLanguages)
		switch n := rnd.Intn(100); {
		case n < 25:
			r.ins(k, r.newTV(lang, res, res && rnd.Intn(3) == 0, rnd.Intn(2) == 0))
		case n < 32:
			if old, ok := r.tree.GetRaw(k); ok {
				if _, one := old.(*dtcommon.TV); one || rnd.Intn(2) == 0 {
					r.insc(k, lang, r.newTV(rnd.Intn(dtcommon.NumLanguages), res, false, rnd.Intn(2) == 0))
				}
			} else {
				r.insc(k, lang, r.newTV(lang, res, false, false))
			}
		case n < 34:
			r.insr(k, r.newTV(lang, res, false, false))
		case n < 44:
			r.del(dtcommon.Pick(rnd, queries), lang)
		case n < 46:
			r.delp(dtcommon.Pick(rnd, queries), lang)
		case n < 47:
			r.delall(dtcommon.Pick(rnd, queries))
		case n < 48:
			r.delpall(dtcommon.Pick(rnd, queries))
		case n < 60:
			r.get(dtcommon.Pick(rnd, queries), lang)
		case n < 62:
			r.raw(dtcommon.Pick(rnd, queries))
		case n < 75:
			// Go's retry loop never ends for a relative string that finds nothing.
			if q := dtcommon.Pick(rnd, queries); q == "" || strings.HasPrefix(q, "/") {
				r.lp(q, lang, rnd.Intn(2) == 0, rnd.Intn(3))
			}
		case n < 79:
			r.lpa(dtcommon.Pick(rnd, queries))
		case n < 83:
			r.fe(dtcommon.Pick(rnd, queries), rnd.Intn(3))
		case n < 93:
			p := ""
			if rnd.Intn(2) == 0 {
				p = dtcommon.Pick(rnd, queries)
			}
			r.walk(lang, p, rnd.Intn(4) == 0, rnd.Intn(2) == 0)
		case n < 97:
			r.wpr(dtcommon.Pick(rnd, queries))
		default:
			r.length()
		}
	}
	r.wpr("")
	return scenario{Name: fmt.Sprintf("random-%d", seed), Ops: r.ops}
}
