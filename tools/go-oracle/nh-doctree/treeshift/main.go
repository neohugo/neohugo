// Command treeshift is the Go oracle for crates/nh-doctree's TreeShiftTree (one
// SimpleThreadSafeTree per language, Hugo's treeTaxonomyEntries) and SimpleTree walks.
//
//	go run ./tools/go-oracle/nh-doctree/treeshift [-root .] [-out crates/nh-doctree/tests/fixtures/treeshift/treeshift.json.gz]
//
// Random operation sequences over small key pools: Insert, Get, LongestPrefix, WalkPrefix and
// WalkPath (stopping or failing at a given visit), WalkPrefixRaw, All, LenRaw, Delete,
// DeletePrefix (which leaves empty radix nodes behind, as Go's does) and DeleteAllFunc. Plus
// Hugo's taxonomy entry keys (term key + page key) for the docs site.
package main

import (
	"errors"
	"flag"
	"fmt"
	"log"
	"strings"

	"github.com/neohugo/neohugo/hugolib/doctree"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-doctree/dtcommon"
)

type op = map[string]any

type recorder struct {
	tree *doctree.TreeShiftTree[string]
	ops  []op
	seq  int
}

var errBoom = errors.New("boom")

func (r *recorder) val() string {
	r.seq++
	return fmt.Sprintf("v%d", r.seq)
}

// walkFn records the visits and stops (terminate) at visit stop or fails at visit fail.
func walkFn(res *[][]string, stop, fail int) func(s, v string) (bool, error) {
	return func(s, v string) (bool, error) {
		*res = append(*res, []string{s, v})
		if len(*res) == fail {
			return false, errBoom
		}
		return len(*res) == stop, nil
	}
}

func errString(err error) string {
	if err != nil {
		return err.Error()
	}
	return ""
}

func (r *recorder) apply(rnd *dtcommon.Rand, pool, queries []string) {
	v := rnd.Intn(dtcommon.NumLanguages)
	t := r.tree.Shape(0, v)
	k := dtcommon.Pick(rnd, queries)
	switch n := rnd.Intn(100); {
	case n < 30:
		k := dtcommon.Pick(rnd, pool)
		val := r.val()
		r.ops = append(r.ops, op{"o": "ins", "d": v, "k": k, "v": val, "r": t.Insert(k, val)})
	case n < 42:
		r.ops = append(r.ops, op{"o": "get", "d": v, "k": k, "r": t.Get(k)})
	case n < 54:
		lk, lv := t.LongestPrefix(k)
		r.ops = append(r.ops, op{"o": "lp", "d": v, "k": k, "r": []string{lk, lv}})
	case n < 66:
		res := [][]string{}
		stop, fail := rnd.Intn(6), rnd.Intn(8)
		err := t.WalkPrefix(doctree.LockTypeRead, k, walkFn(&res, stop, fail))
		r.ops = append(r.ops, op{"o": "wp", "d": v, "k": k, "s": stop, "f": fail, "r": res, "e": errString(err)})
	case n < 72:
		res := [][]string{}
		stop, fail := rnd.Intn(6), rnd.Intn(8)
		err := t.WalkPath(doctree.LockTypeRead, k, walkFn(&res, stop, fail))
		r.ops = append(r.ops, op{"o": "wpath", "d": v, "k": k, "s": stop, "f": fail, "r": res, "e": errString(err)})
	case n < 78:
		res := [][]string{}
		stop, fail := rnd.Intn(4), rnd.Intn(12)
		err := t.WalkPrefixRaw(doctree.LockTypeRead, k, walkFn(&res, stop, fail))
		r.ops = append(r.ops, op{"o": "wpr", "k": k, "s": stop, "f": fail, "r": res, "e": errString(err)})
	case n < 82:
		res := [][]string{}
		for s, val := range t.All(doctree.LockTypeRead) {
			res = append(res, []string{s, val})
		}
		r.ops = append(r.ops, op{"o": "all", "d": v, "r": res})
	case n < 85:
		r.ops = append(r.ops, op{"o": "len", "r": t.LenRaw()})
	case n < 90:
		t.Delete(k)
		r.ops = append(r.ops, op{"o": "del", "k": k})
	case n < 95:
		r.ops = append(r.ops, op{"o": "delp", "k": k, "r": t.DeletePrefix(k)})
	default:
		// Delete the values with an odd number.
		res := [][]string{}
		t.DeleteAllFunc(k, func(s, val string) bool {
			res = append(res, []string{s, val})
			var i int
			_, _ = fmt.Sscanf(val, "v%d", &i)
			return i%2 == 1
		})
		r.ops = append(r.ops, op{"o": "delf", "k": k, "r": res})
	}
}

type scenario struct {
	Name string `json:"name"`
	Ops  []op   `json:"ops"`
}

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "crates/nh-doctree/tests/fixtures/treeshift/treeshift.json.gz", "output file (gzip)")
	flag.Parse()

	var scenarios []scenario

	// Hugo's taxonomy entries: term key + page key (home: term key + "/").
	cfs, err := dtcommon.ContentFiles(*root)
	if err != nil {
		log.Fatal(err)
	}
	{
		r := &recorder{tree: doctree.NewTreeShiftTree[string](doctree.DimensionLanguage.Index(), dtcommon.NumLanguages)}
		rnd := dtcommon.NewRand(7)
		terms := []string{"/tags/go", "/tags/hugo", "/tags/go-lang", "/categories/a", "/categories/ab", "/tags/ขนม"}
		var keys []string
		for _, cf := range cfs {
			if !cf.Page || cf.Site != "docs" {
				continue
			}
			s := cf.Key
			if s == "" {
				s = "/"
			}
			for range rnd.Intn(3) {
				k := dtcommon.Pick(rnd, terms) + s
				keys = append(keys, k)
				lang := rnd.Intn(dtcommon.NumLanguages)
				val := r.val()
				r.ops = append(r.ops, op{"o": "ins", "d": lang, "k": k, "v": val, "r": r.tree.Shape(0, lang).Insert(k, val)})
			}
		}
		for lang := range dtcommon.NumLanguages {
			t := r.tree.Shape(0, lang)
			for _, term := range terms {
				res := [][]string{}
				err := t.WalkPrefix(doctree.LockTypeNone, term+"/", walkFn(&res, -1, -1))
				r.ops = append(r.ops, op{"o": "wp", "d": lang, "k": term + "/", "s": -1, "f": -1, "r": res, "e": errString(err)})
			}
		}
		res := [][]string{}
		err := r.tree.WalkPrefixRaw(doctree.LockTypeRead, "", walkFn(&res, -1, -1))
		r.ops = append(r.ops, op{"o": "wpr", "k": "", "s": -1, "f": -1, "r": res, "e": errString(err)})
		for _, k := range keys[:len(keys)/3] {
			if strings.HasPrefix(k, "/tags/go") {
				r.ops = append(r.ops, op{"o": "delp", "k": k + "/", "r": r.tree.DeletePrefix(k + "/")})
			}
		}
		r.ops = append(r.ops, op{"o": "len", "r": r.tree.LenRaw()})
		scenarios = append(scenarios, scenario{Name: "taxonomy-entries", Ops: r.ops})
	}

	segs := []string{"a", "b", "ab", "a-b", "a.b", "c", "ข", ""}
	for seed := uint64(1); seed <= 300; seed++ {
		rnd := dtcommon.NewRand(seed)
		r := &recorder{tree: doctree.NewTreeShiftTree[string](0, dtcommon.NumLanguages)}
		var pool []string
		pool = append(pool, "", "/")
		for range 5 + rnd.Intn(15) {
			k := ""
			for range 1 + rnd.Intn(4) {
				k += "/" + dtcommon.Pick(rnd, segs)
			}
			pool = append(pool, k)
		}
		pool = dtcommon.Dedupe(pool)
		queries := append([]string{}, pool...)
		for _, k := range pool {
			queries = append(queries, k+"/", k+"x")
			if k != "" {
				queries = append(queries, dtcommon.DropLastRune(k))
			}
		}
		for range 100 + rnd.Intn(100) {
			r.apply(rnd, pool, queries)
		}
		scenarios = append(scenarios, scenario{Name: fmt.Sprintf("random-%d", seed), Ops: r.ops})
	}
	if err := dtcommon.WriteJSONGz(*out, map[string]any{"scenarios": scenarios}); err != nil {
		log.Fatal(err)
	}
}
