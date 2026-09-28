// Command compare is the Go oracle for crates/nh-common/src/compare.rs, the
// port of neohugo's compare package (Strings, LessStrings, Eq, ProbablyEq).
//
//	go run ./tools/go-oracle/nh-common/compare [-root .] [-out crates/nh-common/tests/fixtures/compare/compare.json.gz]
//
// Over the nh-common string corpus (../corpus) it records:
//   - the corpus sorted with sort.SliceStable and LessStrings (as indexes);
//   - compare.Strings for every string against its neighbours in that order,
//     against its upper- and lower-cased forms, and for a deterministic sample
//     of other pairs;
//
// and compare.Eq/ProbablyEq over pairs of typed values.
package main

import (
	"flag"
	"html/template"
	"log"
	"math/rand/v2"
	"sort"
	"strings"
	"unicode"

	"github.com/neohugo/neohugo/common/types/hstring"
	"github.com/neohugo/neohugo/compare"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/corpus"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
)

// eqer implements compare.Eqer and compare.ProbablyEqer.
type eqer struct{ id string }

func (e *eqer) GoType() string { return "*main.eqer" }
func (e *eqer) ID() string     { return e.id }

// Eq is true for another *eqer with the same id.
func (e *eqer) Eq(other any) bool {
	o, ok := other.(*eqer)
	return ok && o.id == e.id
}

// ProbablyEq is true for any *eqer or string with the same id.
func (e *eqer) ProbablyEq(other any) bool {
	if s, ok := other.(string); ok {
		return s == e.id
	}
	_, ok := other.(*eqer)
	return ok
}

// titleWords upper-cases the first rune of every space-separated word.
func titleWords(s string) string {
	var b strings.Builder
	start := true
	for _, r := range s {
		if start {
			b.WriteRune(unicode.ToUpper(r))
		} else {
			b.WriteRune(r)
		}
		start = r == ' '
	}
	return b.String()
}

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "crates/nh-common/tests/fixtures/compare/compare.json.gz", "output file (gzip)")
	flag.Parse()

	strs, err := corpus.Strings(*root)
	if err != nil {
		log.Fatal(err)
	}
	// Extra case-folding edge cases (Kelvin sign, long s, dotless i, final sigma, ß/ẞ, titlecase digraphs).
	strs = append(strs, "k", "K", "K", "s", "S", "ſ", "i", "I", "ı", "İ", "σ", "ς", "Σ",
		"ß", "ẞ", "ǆ", "ǅ", "Ǆ", "aKb", "AKB", "straße", "STRASSE", "Ab\xffc", "AB\xffC", "ab\xfe")
	var encs []any
	for _, s := range strs {
		encs = append(encs, goval.Str(s))
	}

	order := make([]int, len(strs))
	for i := range order {
		order[i] = i
	}
	sort.SliceStable(order, func(i, j int) bool { return compare.LessStrings(strs[order[i]], strs[order[j]]) })

	var cases []map[string]any
	pair := func(a, b int) {
		cases = append(cases, map[string]any{"a": a, "b": b, "r": compare.Strings(strs[a], strs[b])})
	}
	for k := 0; k+1 < len(order); k++ {
		pair(order[k], order[k+1])
		pair(order[k+1], order[k])
	}
	idx := map[string]int{}
	for i, s := range strs {
		idx[s] = i
	}
	for i, s := range strs {
		for _, v := range []string{strings.ToUpper(s), strings.ToLower(s), titleWords(strings.ToLower(s))} {
			j, ok := idx[v]
			if !ok {
				strs = append(strs, v)
				encs = append(encs, goval.Str(v))
				j = len(strs) - 1
				idx[v] = j
			}
			pair(i, j)
			pair(j, i)
		}
	}
	rng := rand.New(rand.NewPCG(1, 2))
	for range 20000 {
		pair(rng.IntN(len(strs)), rng.IntN(len(strs)))
	}

	// Eq / ProbablyEq over typed values.
	e1, e1b, e2 := &eqer{"x"}, &eqer{"x"}, &eqer{"y"}
	vals := []any{
		nil, 1, int64(1), 1.0, "x", "1", template.HTML("x"), hstring.HTML("x"), true, uint(1), e1, e1b, e2,
	}
	var venc []any
	for _, v := range vals {
		venc = append(venc, goval.Encode(v))
	}
	var eqs []any
	for i, a := range vals {
		for j, b := range vals {
			eqs = append(eqs, []any{i, j, compare.Eq(a, b), compare.ProbablyEq(a, b)})
		}
	}

	header := map[string]any{"strings": encs, "sorted": order, "values": venc, "eq": eqs}
	if err := goval.WriteCasesGz(*out, header, cases); err != nil {
		log.Fatal(err)
	}
	log.Printf("%s: %d cases", *out, len(cases))
}
