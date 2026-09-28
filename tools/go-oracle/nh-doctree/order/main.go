// Command order is the Go oracle for the walk order and the prefix lookups of
// crates/nh-doctree (armon/go-radix under doctree.SimpleTree and doctree.NodeShiftTree).
//
//	go run ./tools/go-oracle/nh-doctree/order [-root .] [-out crates/nh-doctree/tests/fixtures/order/order.json.gz]
//
// For every key set (tree keys of this repository's sites and templates, keys quoted in the
// specs, synthetic and random keys) it inserts the keys into a SimpleTree and a NodeShiftTree
// and records Walk, All and WalkPrefixRaw("") order, and for a set of query strings per key
// (the key, the key plus "/", "/x", "-", ".", "~", the key minus its last rune, path.Dir of the
// key): Get, LongestPrefix, WalkPrefix, WalkPath, NodeShiftTree.LongestPrefixAll and
// NodeShiftTree walks with a prefix. Keys are written once, results as indexes into them.
package main

import (
	"context"
	"flag"
	"fmt"
	"log"
	"path"
	"slices"

	"github.com/neohugo/neohugo/hugolib/doctree"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-doctree/dtcommon"
)

// echo is a shifter over plain strings (every value exists in every dimension).
type echo struct{}

func (echo) ForEeachInDimension(n string, d int, f func(string) bool) { f(n) }
func (echo) Insert(old, new string) (string, string, bool)            { return new, old, true }
func (echo) InsertInto(old, new string, _ doctree.Dimension) (string, string, bool) {
	return new, old, true
}
func (echo) Delete(n string, _ doctree.Dimension) (string, bool, bool) { return n, true, true }
func (echo) Shift(n string, _ doctree.Dimension, _ bool) (string, bool, doctree.DimensionFlag) {
	return n, true, doctree.DimensionLanguage
}

type query struct {
	Q      string `json:"q"`
	Get    int    `json:"g"`
	LP     int    `json:"l"`
	Prefix []int  `json:"p"`
	Path   []int  `json:"w"`
	LPA    int    `json:"a"`
	Walk   []int  `json:"n"`
}

type set struct {
	Name    string   `json:"name"`
	Keys    []string `json:"keys"`
	Walk    []int    `json:"walk"`
	All     []int    `json:"all"`
	Raw     []int    `json:"raw"`
	Queries []query  `json:"queries"`
}

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "crates/nh-doctree/tests/fixtures/order/order.json.gz", "output file (gzip)")
	flag.Parse()

	cfs, err := dtcommon.ContentFiles(*root)
	if err != nil {
		log.Fatal(err)
	}
	var pages, resources, all []string
	for _, cf := range cfs {
		if cf.Page {
			pages = append(pages, cf.Key)
		} else {
			resources = append(resources, cf.Key)
		}
		all = append(all, cf.Key)
	}
	tmpl, err := dtcommon.TemplateKeys(*root)
	if err != nil {
		log.Fatal(err)
	}
	spec, err := dtcommon.SpecKeys(*root)
	if err != nil {
		log.Fatal(err)
	}
	synth := dtcommon.SyntheticKeys()

	var mixed []string
	for _, ks := range [][]string{all, tmpl, spec, synth} {
		mixed = append(mixed, ks...)
	}

	sets := []struct {
		name string
		keys []string
	}{
		{"content-pages", pages},
		{"content-resources", resources},
		{"content-all", all},
		{"templates", tmpl},
		{"specs", spec},
		{"synthetic", synth},
		{"mixed", mixed},
	}
	// Dense random tries over a small alphabet.
	alphabet := []string{"/", "a", "b", "-", ".", "ข", "a/", "/b"}
	for seed := uint64(1); seed <= 6; seed++ {
		r := dtcommon.NewRand(seed)
		var keys []string
		for range 120 + r.Intn(120) {
			k := ""
			for range r.Intn(9) {
				k += dtcommon.Pick(r, alphabet)
			}
			keys = append(keys, k)
		}
		sets = append(sets, struct {
			name string
			keys []string
		}{fmt.Sprintf("random-%d", seed), keys})
	}

	var res []set
	for _, s := range sets {
		res = append(res, dump(s.name, dtcommon.Dedupe(s.keys)))
	}
	if err := dtcommon.WriteJSONGz(*out, map[string]any{"sets": res}); err != nil {
		log.Fatal(err)
	}
}

func dump(name string, keys []string) set {
	idx := map[string]int{}
	for i, k := range keys {
		idx[k] = i
	}
	index := func(k string) int {
		i, ok := idx[k]
		if !ok {
			log.Fatalf("%s: unknown key %q", name, k)
		}
		return i
	}

	st := doctree.NewSimpleTree[string]()
	nt := doctree.New(doctree.Config[string]{Shifter: echo{}})
	for _, k := range keys {
		st.Insert(k, "v"+k)
		nt.InsertRawWithLock(k, "v"+k)
	}

	// The radix tree is canonical: the reverse insertion order walks the same.
	rev := doctree.NewSimpleTree[string]()
	for i := len(keys) - 1; i >= 0; i-- {
		rev.Insert(keys[i], "v"+keys[i])
	}

	s := set{Name: name, Keys: keys}
	_ = st.Walk(func(k string, _ string) (bool, error) {
		s.Walk = append(s.Walk, index(k))
		return false, nil
	})
	var revWalk []int
	_ = rev.Walk(func(k string, _ string) (bool, error) {
		revWalk = append(revWalk, index(k))
		return false, nil
	})
	if !slices.Equal(s.Walk, revWalk) {
		log.Fatalf("%s: walk depends on the insertion order", name)
	}
	for k := range st.All() {
		s.All = append(s.All, index(k))
	}
	nt.WalkPrefixRaw("", func(k string, _ string) bool {
		s.Raw = append(s.Raw, index(k))
		return false
	})

	var qs []string
	qs = append(qs, "", "/", "//", "x", "~", "ÿ")
	for _, k := range keys {
		qs = append(qs, k, k+"/", k+"/x", k+"-", k+".", k+"~")
		if k != "" {
			qs = append(qs, dtcommon.DropLastRune(k))
		}
		qs = append(qs, path.Dir(k))
	}
	for _, q := range dtcommon.Dedupe(qs) {
		qr := query{Q: q, Get: -1, LP: -1, LPA: -1}
		if v := st.Get(q); v != "" {
			qr.Get = index(v[1:])
		}
		if k, v := st.LongestPrefix(q); v != "" {
			if v[1:] != k {
				log.Fatalf("%s: LongestPrefix(%q) = %q, %q", name, q, k, v)
			}
			qr.LP = index(k)
		}
		if k, found := nt.LongestPrefixAll(q); found {
			qr.LPA = index(k)
		}
		qr.Prefix = []int{}
		_ = st.WalkPrefix(q, func(k string, _ string) (bool, error) {
			qr.Prefix = append(qr.Prefix, index(k))
			return false, nil
		})
		qr.Path = []int{}
		_ = st.WalkPath(q, func(k string, _ string) (bool, error) {
			qr.Path = append(qr.Path, index(k))
			return false, nil
		})
		qr.Walk = []int{}
		w := &doctree.NodeShiftTreeWalker[string]{
			Tree:   nt,
			Prefix: q,
			Handle: func(k string, _ string, _ doctree.DimensionFlag) (bool, error) {
				qr.Walk = append(qr.Walk, index(k))
				return false, nil
			},
		}
		if err := w.Walk(context.Background()); err != nil {
			log.Fatal(err)
		}
		s.Queries = append(s.Queries, qr)
	}
	return s
}
