// Command go-sort is the Go oracle for the Rust crate crates/go-sort.
//
//	go run ./tools/go-oracle/go-sort -out crates/go-sort/tests/fixtures/sort.txt [-maxn 2000] [-seeds 1]
//	go run ./tools/go-oracle/go-sort -out dense.txt -dense -maxn 3000 [-idsmax 0]
//
// For every combination of (variant, comparator kind, input pattern, size,
// number of distinct keys, seed) it sorts deterministic pseudo-random input
// (splitmix64, reproduced bit-for-bit by the Rust test) with the real Go
// sort/slices functions and records:
//
//   - the number of comparator calls,
//   - an FNV-style hash of the full call trace (less/cmp arguments and, for
//     sort.Interface, every Swap),
//   - a hash of the resulting permutation (element ids, or float bits),
//   - the resulting permutation itself for small inputs.
//
// Line format (space separated):
//
//	variant kind pattern n distinct seed ncalls trace result [ids...]
package main

import (
	"bufio"
	"cmp"
	"flag"
	"fmt"
	"log"
	"math"
	"os"
	"slices"
	"sort"
	"strconv"
	"strings"
)

// ---- deterministic generator shared with the Rust test ----

type splitmix struct{ s uint64 }

func (r *splitmix) next() uint64 {
	r.s += 0x9e3779b97f4a7c15
	z := r.s
	z = (z ^ (z >> 30)) * 0xbf58476d1ce4e5b9
	z = (z ^ (z >> 27)) * 0x94d049bb133111eb
	return z ^ (z >> 31)
}

func (r *splitmix) intn(n int) int { return int(r.next() % uint64(n)) }

const fnvPrime = 0x100000001b3
const fnvBasis = 0xcbf29ce484222325

func mix(h *uint64, v uint64) {
	*h ^= v
	*h *= fnvPrime
}

// pairHash is a deterministic but inconsistent comparator source: its answer
// depends on the two element identities, not on the call order.
func pairHash(a, b int) uint64 {
	h := uint64(a)*0x9e3779b97f4a7c15 ^ uint64(b)*0xc2b2ae3d27d4eb4f
	h ^= h >> 29
	h *= 0xbf58476d1ce4e5b9
	h ^= h >> 32
	return h
}

type elem struct {
	key int
	id  int
}

// genKeys produces n keys in [0, distinct) following pattern p.
func genKeys(p, n, distinct int, r *splitmix) []int {
	keys := make([]int, n)
	for i := range keys {
		switch p {
		case 0: // random
			keys[i] = r.intn(distinct)
		case 1: // ascending with ties
			keys[i] = i * distinct / n
		case 2: // descending with ties
			keys[i] = (n - 1 - i) * distinct / n
		case 3: // sawtooth
			keys[i] = i % distinct
		case 4: // organ pipe
			keys[i] = min(i, n-1-i) % distinct
		case 5: // all equal
			keys[i] = 0
		case 6: // nearly sorted: ascending, patched below
			keys[i] = i * distinct / n
		case 7: // random runs
			if i == 0 || r.intn(8) == 0 {
				keys[i] = r.intn(distinct)
			} else {
				keys[i] = keys[i-1]
			}
		case 8: // descending then ascending (V shape)
			if i < n/2 {
				keys[i] = (n/2 - i) % distinct
			} else {
				keys[i] = (i - n/2) % distinct
			}
		}
	}
	if p == 6 && n > 1 {
		for k := 0; k < 1+n/50; k++ {
			a, b := r.intn(n), r.intn(n)
			keys[a], keys[b] = keys[b], keys[a]
		}
	}
	return keys
}

const numPatterns = 9

// ---- traced sort.Interface ----

type traced struct {
	d      []elem
	kind   int
	r      *splitmix
	trace  uint64
	ncalls int
	// adversary state (kind 2)
	nsolid, candidate, gas int
}

func (t *traced) Len() int { return len(t.d) }

func (t *traced) Less(i, j int) bool {
	t.ncalls++
	mix(&t.trace, 1)
	mix(&t.trace, uint64(i))
	mix(&t.trace, uint64(j))
	switch t.kind {
	case 0:
		return t.d[i].key < t.d[j].key
	case 1:
		return t.r.next()&1 == 1
	case 3:
		return t.d[i].key <= t.d[j].key
	case 4:
		return pairHash(t.d[i].id, t.d[j].id)&1 == 1
	case 2:
		// Go sort_test.go adversaryTestingData.Less (McIlroy's antiqsort).
		if t.d[i].key == t.gas && t.d[j].key == t.gas {
			if i == t.candidate {
				t.d[i].key = t.nsolid
				t.nsolid++
			} else {
				t.d[j].key = t.nsolid
				t.nsolid++
			}
		}
		if t.d[i].key == t.gas {
			t.candidate = i
		} else if t.d[j].key == t.gas {
			t.candidate = j
		}
		return t.d[i].key < t.d[j].key
	}
	panic("kind")
}

func (t *traced) Swap(i, j int) {
	mix(&t.trace, 2)
	mix(&t.trace, uint64(i))
	mix(&t.trace, uint64(j))
	t.d[i], t.d[j] = t.d[j], t.d[i]
}

// ---- variants ----

const (
	vSort           = 0 // sort.Sort(Interface)
	vStable         = 1 // sort.Stable(Interface)
	vSlice          = 2 // sort.Slice
	vSliceStable    = 3 // sort.SliceStable
	vSortFunc       = 4 // slices.SortFunc
	vSortStableFunc = 5 // slices.SortStableFunc
	vSlicesFloat    = 6 // slices.Sort([]float64) (== sort.Float64s)
	vFloat64Slice   = 7 // sort.Sort(sort.Float64Slice)
	vReverse        = 8 // sort.Sort(sort.Reverse(Interface))
	vReverseStable  = 9 // sort.Stable(sort.Reverse(Interface))
	vSlicesInt      = 10
	vSlicesString   = 11
	vSlicesMin      = 12 // slices.Min([]float64): arm64 FMIN NaN/±0 semantics
	vSlicesMax      = 13 // slices.Max([]float64)
)

type result struct {
	ncalls int
	trace  uint64
	res    uint64
	ids    []uint64
}

func floatVal(k, id int) float64 {
	switch k % 8 {
	case 0:
		return math.Float64frombits(0x7ff8000000000000 | uint64(id)) // NaN with payload id
	case 1:
		return math.Copysign(0, -1)
	case 2:
		return 0
	case 3:
		return math.Inf(1)
	case 4:
		return math.Inf(-1)
	case 5:
		return 1
	case 6:
		return -1
	default:
		return float64(k) / 4
	}
}

// mmVal produces values for the min/max variants: quiet and signaling NaNs
// with distinct payloads, signed zeros, infinities and ordinary values.
func mmVal(k, id int) float64 {
	switch k % 10 {
	case 0:
		return math.Float64frombits(0x7ff8000000000000 | uint64(id)) // quiet NaN
	case 1:
		return math.Float64frombits(0x7ff0000000000000 | uint64(id+1)) // signaling NaN
	case 2:
		return math.Copysign(0, -1)
	case 3:
		return 0
	case 4:
		return math.Inf(1)
	case 5:
		return math.Inf(-1)
	case 6:
		return 1
	case 7:
		return -1
	default:
		return float64(k) / 3
	}
}

func run(variant, kind int, keys []int, seed uint64) result {
	n := len(keys)
	d := make([]elem, n)
	for i := range d {
		d[i] = elem{keys[i], i}
	}
	r := &splitmix{seed ^ 0x5eed}
	var res result
	switch variant {
	case vSort, vStable, vReverse, vReverseStable:
		t := &traced{d: d, kind: kind, r: r, trace: fnvBasis}
		if kind == 2 {
			t.gas = n - 1
			for i := range t.d {
				t.d[i].key = t.gas
			}
		}
		var data sort.Interface = t
		if variant == vReverse || variant == vReverseStable {
			data = sort.Reverse(t)
		}
		if variant == vSort || variant == vReverse {
			sort.Sort(data)
		} else {
			sort.Stable(data)
		}
		res.ncalls, res.trace = t.ncalls, t.trace
	case vSlice, vSliceStable:
		trace := uint64(fnvBasis)
		less := func(i, j int) bool {
			res.ncalls++
			mix(&trace, 1)
			mix(&trace, uint64(i))
			mix(&trace, uint64(j))
			switch kind {
			case 1:
				return r.next()&1 == 1
			case 3:
				return d[i].key <= d[j].key
			case 4:
				return pairHash(d[i].id, d[j].id)&1 == 1
			}
			return d[i].key < d[j].key
		}
		if variant == vSlice {
			sort.Slice(d, less)
		} else {
			sort.SliceStable(d, less)
		}
		res.trace = trace
	case vSortFunc, vSortStableFunc:
		trace := uint64(fnvBasis)
		cmpf := func(a, b elem) int {
			res.ncalls++
			mix(&trace, 3)
			mix(&trace, uint64(a.id))
			mix(&trace, uint64(b.id))
			switch kind {
			case 1:
				return int(r.next()%3) - 1
			case 3:
				if a.key <= b.key {
					return -1
				}
				return 1
			case 4:
				return int(pairHash(a.id, b.id)%3) - 1
			}
			return cmp.Compare(a.key, b.key)
		}
		if variant == vSortFunc {
			slices.SortFunc(d, cmpf)
		} else {
			slices.SortStableFunc(d, cmpf)
		}
		res.trace = trace
	case vSlicesFloat, vFloat64Slice:
		f := make([]float64, n)
		for i := range f {
			f[i] = floatVal(keys[i], i)
		}
		if variant == vSlicesFloat {
			slices.Sort(f)
		} else {
			sort.Sort(sort.Float64Slice(f))
		}
		res.res = fnvBasis
		for _, v := range f {
			mix(&res.res, math.Float64bits(v))
			res.ids = append(res.ids, math.Float64bits(v))
		}
		return res
	case vSlicesMin, vSlicesMax:
		f := make([]float64, n)
		for i := range f {
			f[i] = mmVal(keys[i], i)
		}
		var v float64
		if variant == vSlicesMin {
			v = slices.Min(f)
		} else {
			v = slices.Max(f)
		}
		res.res = math.Float64bits(v)
		return res
	case vSlicesInt:
		x := make([]int, n)
		for i := range x {
			x[i] = keys[i]*7919 - 3*i
		}
		slices.Sort(x)
		res.res = fnvBasis
		for _, v := range x {
			mix(&res.res, uint64(v))
			res.ids = append(res.ids, uint64(v))
		}
		return res
	case vSlicesString:
		x := make([]string, n)
		for i := range x {
			x[i] = strconv.Itoa(keys[i]*31 + i%3)
		}
		slices.Sort(x)
		res.res = fnvBasis
		for i, v := range x {
			for k := 0; k < len(v); k++ {
				mix(&res.res, uint64(v[k]))
			}
			mix(&res.res, 0x100)
			_ = i
		}
		return res
	}
	res.res = fnvBasis
	for _, e := range d {
		mix(&res.res, uint64(e.id))
		res.ids = append(res.ids, uint64(e.id))
	}
	return res
}

type combo struct{ variant, kind int }

func combos() []combo {
	var cs []combo
	for _, v := range []int{vSort, vStable, vSlice, vSliceStable, vSortFunc, vSortStableFunc, vReverse, vReverseStable} {
		cs = append(cs, combo{v, 0}, combo{v, 1})
	}
	cs = append(cs, combo{vSort, 2}, combo{vStable, 2},
		combo{vSlicesFloat, 0}, combo{vFloat64Slice, 0}, combo{vSlicesInt, 0}, combo{vSlicesString, 0},
		combo{vSlicesMin, 0}, combo{vSlicesMax, 0})
	return cs
}

// denseCombos adds the non-strict (<=) and identity-hash comparators
// (kinds 3 and 4) to every comparator-driven variant.
func denseCombos() []combo {
	cs := combos()
	for _, v := range []int{vSort, vStable, vSlice, vSliceStable, vSortFunc, vSortStableFunc, vReverse, vReverseStable} {
		cs = append(cs, combo{v, 3}, combo{v, 4})
	}
	return cs
}

func main() {
	out := flag.String("out", "", "output file")
	maxn := flag.Int("maxn", 2000, "largest input size")
	dense := flag.Bool("dense", false, "every size 0..maxn, one rotating pattern per (size, combo), comparator kinds 0-4")
	sizeList := flag.String("sizes", "", "with -dense: comma-separated sizes instead of 0..maxn")
	every := flag.Int("every", 1, "with -dense: keep only every Nth case")
	nseeds := flag.Int("seeds", 1, "seeds per configuration")
	idsMax := flag.Int("idsmax", 48, "write full result ids for n <= idsmax")
	flag.Parse()
	if *out == "" {
		log.Fatal("-out required")
	}

	if *dense {
		var sizes []int
		if *sizeList != "" {
			for _, f := range strings.Split(*sizeList, ",") {
				v, err := strconv.Atoi(f)
				if err != nil {
					log.Fatal(err)
				}
				sizes = append(sizes, v)
			}
		} else {
			for n := 0; n <= *maxn; n++ {
				sizes = append(sizes, n)
			}
		}
		writeDense(*out, sizes, *idsMax, *every)
		return
	}

	var sizes []int
	for n := 0; n <= 80 && n <= *maxn; n++ {
		sizes = append(sizes, n)
	}
	for _, n := range []int{96, 100, 127, 128, 129, 150, 200, 255, 256, 257, 300, 400, 499, 500, 512, 700, 999, 1000, 1024, 1500, 1999, 2000} {
		if n <= *maxn {
			sizes = append(sizes, n)
		}
	}

	f, err := os.Create(*out)
	if err != nil {
		log.Fatal(err)
	}
	w := bufio.NewWriter(f)
	cases := 0
	for _, n := range sizes {
		distincts := []int{1, 2, 3, 10, n/2 + 1, n + 1}
		for _, c := range combos() {
			for p := 0; p < numPatterns; p++ {
				if c.kind == 2 && p != 0 {
					continue // adversary ignores the input
				}
				if (c.variant == vSlicesMin || c.variant == vSlicesMax) && n == 0 {
					continue // panics in Go
				}
				for s := 0; s < *nseeds; s++ {
					seed := uint64(n)*1_000_003 + uint64(p)*7919 + uint64(c.variant)*131 + uint64(c.kind)*17 + uint64(s)
					sel := &splitmix{seed}
					distinct := distincts[sel.intn(len(distincts))]
					keys := genKeys(p, n, distinct, &splitmix{seed * 31})
					res := run(c.variant, c.kind, keys, seed)
					_, _ = fmt.Fprintf(w, "%d %d %d %d %d %d %d %d %d", c.variant, c.kind, p, n, distinct, seed, res.ncalls, res.trace, res.res)
					if n <= *idsMax {
						for _, id := range res.ids {
							_, _ = fmt.Fprintf(w, " %d", id)
						}
					}
					_ = w.WriteByte('\n')
					cases++
				}
			}
		}
	}
	if err := w.Flush(); err != nil {
		log.Fatal(err)
	}
	if err := f.Close(); err != nil {
		log.Fatal(err)
	}
	fmt.Fprintf(os.Stderr, "wrote %d cases to %s\n", cases, *out)
}

// writeDense covers the given sizes (the -dense mode).
func writeDense(out string, sizes []int, idsMax, every int) {
	f, err := os.Create(out)
	if err != nil {
		log.Fatal(err)
	}
	w := bufio.NewWriter(f)
	cases, seen := 0, 0
	for _, n := range sizes {
		distincts := []int{1, 2, 3, 10, n/2 + 1, n + 1}
		for ci, c := range denseCombos() {
			if (c.variant == vSlicesMin || c.variant == vSlicesMax) && n == 0 {
				continue // panics in Go
			}
			seen++
			if every > 1 && (seen-1)%every != 0 {
				continue
			}
			p := (n + ci) % numPatterns
			if c.kind == 2 {
				p = 0 // adversary ignores the input
			}
			seed := uint64(n)*1_000_003 + uint64(p)*7919 + uint64(c.variant)*131 + uint64(c.kind)*17 + 0xd0e5e
			sel := &splitmix{seed}
			distinct := distincts[sel.intn(len(distincts))]
			keys := genKeys(p, n, distinct, &splitmix{seed * 31})
			res := run(c.variant, c.kind, keys, seed)
			_, _ = fmt.Fprintf(w, "%d %d %d %d %d %d %d %d %d", c.variant, c.kind, p, n, distinct, seed, res.ncalls, res.trace, res.res)
			if n <= idsMax {
				for _, id := range res.ids {
					_, _ = fmt.Fprintf(w, " %d", id)
				}
			}
			_ = w.WriteByte('\n')
			cases++
		}
	}
	if err := w.Flush(); err != nil {
		log.Fatal(err)
	}
	if err := f.Close(); err != nil {
		log.Fatal(err)
	}
	fmt.Fprintf(os.Stderr, "wrote %d cases to %s\n", cases, out)
}
