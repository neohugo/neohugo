// Command walkmut is the Go oracle for walks over a crates/nh-doctree NodeShiftTree whose handle
// modifies the walked tree, as Hugo's assembly walks do (assembleTerms and
// addMissingRootSections insert, applyAggregatesToTaxonomiesAndTerms deletes the visited term,
// DeleteAll deletes while walking).
//
//	go run ./tools/go-oracle/nh-doctree/walkmut [-root .] [-out crates/nh-doctree/tests/fixtures/walkmut/walkmut.json.gz]
//
// What such a walk visits depends on armon/go-radix's node structure (keys inserted behind the
// walk's position are skipped, some inserted ahead are visited, deleting the visited key can
// make the walk visit a sibling again), which is why the Rust port ports the radix tree. Every
// scenario records the initial tree, the walk configuration, and per visit the key, the value,
// the match flag and the action the handle took; then the walk's result (a Go panic is
// recorded) and the final tree.
package main

import (
	"context"
	"errors"
	"flag"
	"fmt"
	"log"
	"path"
	"strings"

	"github.com/neohugo/neohugo/hugolib/doctree"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-doctree/dtcommon"
)

type node = dtcommon.Node

type scenario struct {
	Name    string  `json:"name"`
	Init    [][]any `json:"init"` // [key, def]
	Lang    int     `json:"d"`
	Prefix  string  `json:"k"`
	NoShift bool    `json:"ns"`
	Exact   bool    `json:"x"`
	Visits  [][]any `json:"visits"`  // [key, repr, flag]
	Actions [][]any `json:"actions"` // one per visit
	Result  string  `json:"result"`
	Final   [][]any `json:"final"` // WalkPrefixRaw("") after the walk: [key, repr]
	Len     int     `json:"len"`
}

type builder struct {
	tree *doctree.NodeShiftTree[node]
	seq  int
	sc   scenario
}

func newBuilder(name string) *builder {
	return &builder{
		tree: doctree.New(doctree.Config[node]{Shifter: &dtcommon.Shifter{}}),
		sc:   scenario{Name: name},
	}
}

func (b *builder) tv(lang int, branch bool) *dtcommon.TV {
	b.seq++
	return &dtcommon.TV{ID: fmt.Sprintf("p%d", b.seq), Lang: lang, Branch: branch}
}

func (b *builder) init(k string, v *dtcommon.TV) {
	b.tree.InsertIntoValuesDimension(k, v)
	b.sc.Init = append(b.sc.Init, []any{k, v.Def()})
}

// action is what the handle does at one visit; it returns (terminate, err).
type action func(w *doctree.NodeShiftTreeWalker[node], t *doctree.NodeShiftTree[node], s string) (bool, error)

var errBoom = errors.New("boom")

// run walks the tree; decide picks the action for each visit and records it.
func (b *builder) run(lang int, prefix string, noShift, exact bool, decide func(s string, n node) ([]any, action)) scenario {
	b.sc.Lang, b.sc.Prefix, b.sc.NoShift, b.sc.Exact = lang, prefix, noShift, exact
	b.sc.Visits = [][]any{}
	b.sc.Actions = [][]any{}
	t := b.tree.Shape(0, lang)
	limit := 10*t.Len() + 20
	var w *doctree.NodeShiftTreeWalker[node]
	w = &doctree.NodeShiftTreeWalker[node]{
		Tree:    t,
		Prefix:  prefix,
		NoShift: noShift,
		Exact:   exact,
		Handle: func(s string, n node, match doctree.DimensionFlag) (bool, error) {
			b.sc.Visits = append(b.sc.Visits, []any{s, dtcommon.Repr(n), int(match)})
			if len(b.sc.Visits) > limit {
				b.sc.Actions = append(b.sc.Actions, []any{"cap"})
				return true, nil
			}
			rec, act := decide(s, n)
			b.sc.Actions = append(b.sc.Actions, rec)
			return act(w, t, s)
		},
	}
	b.sc.Result = "ok"
	func() {
		defer func() {
			if r := recover(); r != nil {
				b.sc.Result = fmt.Sprintf("panic:%v", r)
			}
		}()
		if err := w.Walk(context.Background()); err != nil {
			b.sc.Result = "err:" + err.Error()
		}
	}()
	b.sc.Final = [][]any{}
	b.tree.WalkPrefixRaw("", func(s string, n node) bool {
		b.sc.Final = append(b.sc.Final, []any{s, dtcommon.Repr(n)})
		return false
	})
	b.sc.Len = b.tree.Len()
	return b.sc
}

func none(*doctree.NodeShiftTreeWalker[node], *doctree.NodeShiftTree[node], string) (bool, error) {
	return false, nil
}

func (b *builder) ins(k string, v *dtcommon.TV) ([]any, action) {
	return []any{"ins", k, v.Def()}, func(_ *doctree.NodeShiftTreeWalker[node], t *doctree.NodeShiftTree[node], _ string) (bool, error) {
		t.InsertIntoValuesDimension(k, v)
		return false, nil
	}
}

func (b *builder) insc(k string, v *dtcommon.TV) ([]any, action) {
	return []any{"insc", k, v.Def()}, func(_ *doctree.NodeShiftTreeWalker[node], t *doctree.NodeShiftTree[node], _ string) (bool, error) {
		t.InsertIntoCurrentDimension(k, v)
		return false, nil
	}
}

func delcur() ([]any, action) {
	return []any{"delcur"}, func(_ *doctree.NodeShiftTreeWalker[node], t *doctree.NodeShiftTree[node], s string) (bool, error) {
		t.Delete(s)
		return false, nil
	}
}

func (b *builder) del(k string, lang int) ([]any, action) {
	return []any{"del", k, lang}, func(_ *doctree.NodeShiftTreeWalker[node], _ *doctree.NodeShiftTree[node], _ string) (bool, error) {
		b.tree.Shape(0, lang).Delete(k)
		return false, nil
	}
}

func (b *builder) delall(k string) ([]any, action) {
	return []any{"delall", k}, func(_ *doctree.NodeShiftTreeWalker[node], _ *doctree.NodeShiftTree[node], _ string) (bool, error) {
		b.tree.DeleteAll(k)
		return false, nil
	}
}

func skip(p string) ([]any, action) {
	return []any{"skip", p}, func(w *doctree.NodeShiftTreeWalker[node], _ *doctree.NodeShiftTree[node], _ string) (bool, error) {
		w.SkipPrefix(p)
		return false, nil
	}
}

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "crates/nh-doctree/tests/fixtures/walkmut/walkmut.json.gz", "output file (gzip)")
	flag.Parse()

	cfs, err := dtcommon.ContentFiles(*root)
	if err != nil {
		log.Fatal(err)
	}
	var pages []dtcommon.ContentFile
	for _, cf := range cfs {
		if cf.Page && cf.Site == "docs" {
			pages = append(pages, cf)
		}
	}

	var scenarios []scenario
	for lang := range dtcommon.NumLanguages {
		scenarios = append(scenarios, assembleTerms(pages, lang), addMissingRootSections(pages, lang))
	}
	for seed := uint64(1); seed <= 40; seed++ {
		scenarios = append(scenarios, deleteTerms(seed))
	}
	for seed := uint64(1); seed <= 3000; seed++ {
		scenarios = append(scenarios, random(seed))
	}
	panics := 0
	for _, s := range scenarios {
		if strings.HasPrefix(s.Result, "panic:") {
			panics++
		}
	}
	log.Printf("%d scenarios, %d end in a Go panic", len(scenarios), panics)
	if err := dtcommon.WriteJSONGz(*out, map[string]any{"scenarios": scenarios}); err != nil {
		log.Fatal(err)
	}
}

// assembleTerms mirrors sitePagesAssembler.assembleTerms: the docs pages (every third also in
// Thai), taxonomy nodes /tags and /categories, and a walk of the language that inserts every
// missing term page it finds in a page's (synthetic) front matter.
func assembleTerms(pages []dtcommon.ContentFile, lang int) scenario {
	b := newBuilder(fmt.Sprintf("assembleTerms-%d", lang))
	rnd := dtcommon.NewRand(uint64(100 + lang))
	for i, cf := range pages {
		b.init(cf.Key, b.tv(0, cf.Branch))
		if i%3 == 0 {
			b.init(cf.Key, b.tv(1, cf.Branch))
		}
	}
	b.init("/tags", b.tv(lang, true))
	b.init("/categories", b.tv(0, true))
	b.init("/categories", b.tv(1, true))
	terms := []string{"go", "hugo", "templates", "a", "zzz", "ขนม", "go-lang", "go.lang", "t1", "t10", "t2"}
	return b.run(lang, "", false, false, func(s string, n node) ([]any, action) {
		if strings.HasPrefix(s, "/tags/") || strings.HasPrefix(s, "/categories/") {
			return []any{"none"}, none
		}
		if rnd.Intn(3) != 0 {
			return []any{"none"}, none
		}
		tax := dtcommon.Pick(rnd, []string{"/tags", "/categories"})
		k := tax + "/" + dtcommon.Pick(rnd, terms)
		if b.tree.Shape(0, lang).Get(k) != nil {
			return []any{"none"}, none
		}
		return b.ins(k, b.tv(lang, true))
	})
}

// addMissingRootSections mirrors sitePagesAssembler.addMissingRootSections: sections without an
// _index file are created while walking, and deeper paths are skipped.
func addMissingRootSections(pages []dtcommon.ContentFile, lang int) scenario {
	b := newBuilder(fmt.Sprintf("addMissingRootSections-%d", lang))
	for i, cf := range pages {
		if cf.Branch && strings.Count(cf.Key, "/") == 1 && i%2 == 0 {
			// Drop every other root section so that the walk has to add it.
			continue
		}
		b.init(cf.Key, b.tv(lang, cf.Branch))
	}
	seen := map[string]bool{}
	return b.run(lang, "", false, false, func(s string, n node) ([]any, action) {
		if s == "" {
			return []any{"none"}, none
		}
		section := strings.SplitN(s[1:], "/", 2)[0]
		if seen[section] {
			return []any{"none"}, none
		}
		seen[section] = true
		if b.tree.Shape(0, lang).Get("/"+section) == nil {
			rec, act := b.ins("/"+section, b.tv(lang, true))
			if strings.Count(s, "/") > 1 {
				return append(rec, "skip", s+"/"), func(w *doctree.NodeShiftTreeWalker[node], t *doctree.NodeShiftTree[node], ss string) (bool, error) {
					_, _ = act(w, t, ss)
					w.SkipPrefix(ss + "/")
					return false, nil
				}
			}
			return rec, act
		}
		if strings.Count(s, "/") > 1 {
			return skip(s + "/")
		}
		return []any{"none"}, none
	})
}

// deleteTerms mirrors applyAggregatesToTaxonomiesAndTerms: a walk of /tags that deletes the
// term pages that should not be built, in a tree where terms exist in one or both languages.
func deleteTerms(seed uint64) scenario {
	rnd := dtcommon.NewRand(seed)
	b := newBuilder(fmt.Sprintf("deleteTerms-%d", seed))
	lang := rnd.Intn(dtcommon.NumLanguages)
	b.init("", b.tv(lang, true))
	b.init("/posts", b.tv(lang, true))
	b.init("/posts/p1", b.tv(lang, false))
	b.init("/tags", b.tv(lang, true))
	b.init("/tagsx", b.tv(lang, true))
	for range 3 + rnd.Intn(12) {
		k := "/tags/" + dtcommon.Pick(rnd, []string{"a", "ab", "abc", "b", "ba", "c", "go", "go-x", "go.x", "gox", "ข"})
		if rnd.Intn(4) == 0 {
			k += "/" + dtcommon.Pick(rnd, []string{"a", "b"})
		}
		b.init(k, b.tv(lang, true))
		if rnd.Intn(2) == 0 {
			b.init(k, b.tv(1-lang, true))
		}
	}
	prefix := dtcommon.Pick(rnd, []string{"/tags", "/tags/", ""})
	return b.run(lang, prefix, false, false, func(s string, n node) ([]any, action) {
		if strings.HasPrefix(s, "/tags/") && rnd.Intn(2) == 0 {
			return delcur()
		}
		return []any{"none"}, none
	})
}

// random walks a small dense tree with a handle that randomly inserts (children, siblings,
// prefixes of the visited key, other keys), deletes (the visited key, other keys, whole
// subtrees), skips prefixes, stops or fails.
func random(seed uint64) scenario {
	rnd := dtcommon.NewRand(seed)
	b := newBuilder(fmt.Sprintf("random-%d", seed))
	segs := []string{"a", "b", "ab", "a-b", "a.b", "c", "ข", "b/a"}
	var pool []string
	for range 3 + rnd.Intn(18) {
		k := ""
		for range 1 + rnd.Intn(3) {
			k += "/" + dtcommon.Pick(rnd, segs)
		}
		pool = append(pool, k)
	}
	if rnd.Intn(2) == 0 {
		pool = append(pool, "")
	}
	pool = dtcommon.Dedupe(pool)
	for _, k := range pool {
		l := rnd.Intn(3)
		if l < 2 {
			b.init(k, b.tv(l, rnd.Intn(2) == 0))
		} else {
			b.init(k, b.tv(0, false))
			b.init(k, b.tv(1, false))
		}
	}
	lang := rnd.Intn(dtcommon.NumLanguages)
	prefix := ""
	if rnd.Intn(3) == 0 {
		prefix = dtcommon.Pick(rnd, pool)
		if rnd.Intn(2) == 0 && prefix != "" {
			prefix = dtcommon.DropLastRune(prefix)
		}
	}
	newKey := func(s string) string {
		switch rnd.Intn(6) {
		case 0:
			return s + "/" + dtcommon.Pick(rnd, segs)
		case 1:
			if s == "" {
				return "/" + dtcommon.Pick(rnd, segs)
			}
			return path.Dir(s) + "/" + dtcommon.Pick(rnd, segs)
		case 2:
			if i := strings.LastIndex(s, "/"); i > 0 {
				return s[:i]
			}
			return ""
		case 3:
			return dtcommon.Pick(rnd, pool)
		default:
			k := ""
			for range 1 + rnd.Intn(3) {
				k += "/" + dtcommon.Pick(rnd, segs)
			}
			return k
		}
	}
	return b.run(lang, prefix, rnd.Intn(5) == 0, rnd.Intn(2) == 0, func(s string, n node) ([]any, action) {
		clean := func(k string) string {
			if k == "/" {
				return ""
			}
			return strings.TrimSuffix(k, "/")
		}
		switch x := rnd.Intn(100); {
		case x < 40:
			return []any{"none"}, none
		case x < 60:
			return b.ins(clean(newKey(s)), b.tv(rnd.Intn(dtcommon.NumLanguages), rnd.Intn(2) == 0))
		case x < 65:
			return b.insc(clean(newKey(s)), b.tv(rnd.Intn(dtcommon.NumLanguages), false))
		case x < 75:
			return delcur()
		case x < 85:
			return b.del(newKey(s), rnd.Intn(dtcommon.NumLanguages))
		case x < 88:
			return b.delall(newKey(s))
		case x < 96:
			p := dtcommon.Pick(rnd, []string{s + "/", path.Dir(s) + "/", newKey(s)})
			return skip(p)
		case x < 98:
			return []any{"stop"}, func(*doctree.NodeShiftTreeWalker[node], *doctree.NodeShiftTree[node], string) (bool, error) {
				return true, nil
			}
		default:
			return []any{"err"}, func(*doctree.NodeShiftTreeWalker[node], *doctree.NodeShiftTree[node], string) (bool, error) {
				return false, errBoom
			}
		}
	})
}
