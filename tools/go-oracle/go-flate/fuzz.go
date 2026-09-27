// Adversarial / randomized oracle modes added by the go-flate verifier:
//
//	fuzz          randomized compressor cases (new data kinds 10-15, block-boundary
//	              sizes, flush-heavy / boundary / lifecycle programs, failing sinks)
//	inflate-gen   random *valid* DEFLATE streams (stored / fixed / dynamic blocks
//	              with random prefix codes, RLE code-length encodings, degenerate
//	              and empty distance trees, preset dictionaries), optionally
//	              mutated, decoded by Go; written as a binary corpus + results.
//
// genDataExt must stay in sync with gen_data_ext in
// crates/go-flate/tests/common/mod.rs.

package main

import (
	"bufio"
	"bytes"
	"compress/flate"
	"compress/zlib"
	"encoding/binary"
	"errors"
	"flag"
	"fmt"
	"hash/adler32"
	"hash/fnv"
	"io"
	"os"
	"sort"
	"strconv"
	"strings"
)

var errFail = errors.New("oracle: sink failure")

// genDataExt implements the data kinds >= 10.
func genDataExt(r *rng, kind, size int, out []byte) []byte {
	switch kind {
	case 10: // Fibonacci-skewed symbol counts, shuffled: deep Huffman trees
		nsym := 3 + r.intn(30)
		syms := make([]byte, nsym)
		for i := range syms {
			syms[i] = byte(r.next())
		}
		fib := make([]uint64, nsym)
		var sum uint64
		a, b := uint64(1), uint64(1)
		for i := range fib {
			fib[i] = a
			sum += a
			a, b = b, a+b
		}
		for i := range fib {
			n := int(fib[i] * uint64(size) / sum)
			for j := 0; j < n; j++ {
				out = append(out, syms[i])
			}
		}
		for len(out) < size {
			out = append(out, syms[nsym-1])
		}
		for i := len(out) - 1; i > 0; i-- {
			j := r.intn(i + 1)
			out[i], out[j] = out[j], out[i]
		}
	case 11: // exact period repeats, optional rare mutations
		periods := []int{1, 2, 3, 4, 5, 7, 8, 16, 255, 256, 257, 258, 259, 1000,
			32766, 32767, 32768, 32769, 32770, 65535, 65536}
		p := periods[r.intn(len(periods))]
		pat := make([]byte, p)
		for i := range pat {
			pat[i] = byte(r.next())
		}
		mut := r.intn(4)
		for i := 0; len(out) < size; i++ {
			c := pat[i%p]
			if mut > 0 && r.intn(1<<(6+2*mut)) == 0 {
				c = byte(r.next())
			}
			out = append(out, c)
		}
	case 12: // sparse: zero runs with isolated random bytes
		gaps := []int{4, 64, 2000, 70000}
		for len(out) < size {
			g := gaps[r.intn(len(gaps))]
			gap := r.intn(1 + g)
			for j := 0; j < gap && len(out) < size; j++ {
				out = append(out, 0)
			}
			out = append(out, byte(r.next()))
		}
	case 13: // counters / binary tables
		mode := r.intn(4)
		k := uint32(1 + r.intn(7))
		for i := uint32(0); len(out) < size; i++ {
			switch mode {
			case 0:
				out = append(out, byte(i*k))
			case 1:
				v := uint16(i * k)
				out = append(out, byte(v), byte(v>>8))
			case 2:
				v := i * k * 2654435761
				out = append(out, byte(v), byte(v>>8), byte(v>>16), byte(v>>24))
			default:
				v := i * k
				out = append(out, byte(v), byte(v>>8), byte(v>>16), byte(v>>24))
			}
		}
	case 14: // segments sized around block / window boundaries
		kinds := []int{0, 1, 3, 5, 11, 12}
		lens := []int{65535, 65536, 32768, 65274, 128, 32}
		for len(out) < size {
			k := kinds[r.intn(len(kinds))]
			n := lens[r.intn(len(lens))] + r.intn(9) - 4
			if n < 1 {
				n = 1
			}
			out = append(out, genData(k, n, r.next())...)
		}
	case 15: // PNG filtered scanlines of a synthetic image
		bpps := []int{1, 2, 3, 4, 6, 8}
		bpp := bpps[r.intn(len(bpps))]
		width := 1 + r.intn(700)
		rowLen := width * bpp
		prev := make([]byte, rowLen)
		cur := make([]byte, rowLen)
		noise := r.intn(4)
		fx := 1 + r.intn(5)
		fy := 1 + r.intn(5)
		flat := r.intn(3) == 0
		for y := 0; len(out) < size; y++ {
			for x := 0; x < width; x++ {
				for c := 0; c < bpp; c++ {
					v := x*fx + y*fy + c*37
					if flat {
						v = (x/64)*fx + (y/64)*fy + c
					}
					if noise > 0 {
						v += r.intn(1 << noise)
					}
					cur[x*bpp+c] = byte(v)
				}
			}
			ft := r.intn(5)
			out = append(out, byte(ft))
			for i := 0; i < rowLen; i++ {
				var a, b, c byte
				if i >= bpp {
					a = cur[i-bpp]
					c = prev[i-bpp]
				}
				b = prev[i]
				var f byte
				switch ft {
				case 0:
					f = cur[i]
				case 1:
					f = cur[i] - a
				case 2:
					f = cur[i] - b
				case 3:
					f = cur[i] - byte((int(a)+int(b))/2)
				default:
					f = cur[i] - paeth(a, b, c)
				}
				out = append(out, f)
			}
			prev, cur = cur, prev
		}
	default:
		panic("bad kind")
	}
	return out
}

func paeth(a, b, c byte) byte {
	p := int(a) + int(b) - int(c)
	pa, pb, pc := absInt(p-int(a)), absInt(p-int(b)), absInt(p-int(c))
	if pa <= pb && pa <= pc {
		return a
	}
	if pb <= pc {
		return b
	}
	return c
}

func absInt(x int) int {
	if x < 0 {
		return -x
	}
	return x
}

// ---- fuzz ----

func fuzzSize(r *rng, maxSize int) int {
	bases := []int{0, 1, 13, 32, 33, 127, 128, 129, 4096, 32768, 65274, 65535, 65536,
		98304, 130548, 131070, 131072, 196605, 262144}
	switch r.intn(3) {
	case 0:
		return r.intn(maxSize + 1)
	case 1:
		return r.intn(1 + r.intn(3000))
	default:
		n := bases[r.intn(len(bases))] + r.intn(41) - 20
		if n < 0 {
			n = 0
		}
		return min(n, maxSize)
	}
}

func fuzzOps(r *rng, size int) []string {
	w := func(n int) string { return "W" + strconv.Itoa(n) }
	var ops []string
	pos := 0
	switch r.intn(6) {
	case 0:
		return []string{w(size), "C"}
	case 1:
		chunks := []int{1, 2, 3, 5, 13, 127, 128, 129, 255, 256, 4095, 32767, 32768,
			65273, 65274, 65534, 65535, 65536, 65537, 131071}
		c := chunks[r.intn(len(chunks))]
		if c < 13 && size > 30000 {
			c = 13
		}
		return chunkOps(size, c)
	case 2:
		return randomOps(r, size, []int{0, 1, 2, 5}[r.intn(4)], []int{0, 0, 4, 20}[r.intn(4)],
			[]int{16, 300, 70000}[r.intn(3)])
	case 3: // flush heavy
		sizes := []int{0, 1, 2, 3, 4, 5, 12, 13, 31, 32, 33, 127, 128, 129, 258, 1000,
			32768, 65535, 65536}
		for pos < size {
			n := min(sizes[r.intn(len(sizes))], size-pos)
			ops = append(ops, w(n))
			pos += n
			if r.intn(3) != 0 {
				ops = append(ops, "F")
			}
			if len(ops) > 4000 {
				ops = append(ops, w(size-pos))
				pos = size
			}
		}
		if r.intn(2) == 0 {
			ops = append(ops, "F")
		}
		return append(ops, "C")
	case 4: // land exactly on window / block boundaries
		steps := []int{65535, 65536, 32768, 65274, 131070, 1}
		for pos < size {
			n := steps[r.intn(len(steps))] + r.intn(3) - 1
			n = max(0, min(n, size-pos))
			ops = append(ops, w(n))
			pos += n
			if r.intn(2) == 0 {
				ops = append(ops, "F")
			}
		}
		return append(ops, "C")
	default: // lifecycle: close/flush/reset in odd orders
		nops := 5 + r.intn(40)
		for i := 0; i < nops; i++ {
			switch r.intn(8) {
			case 0, 1, 2, 3:
				n := min(r.intn(1+[]int{10, 1000, 70000}[r.intn(3)]), size-pos)
				ops = append(ops, w(n))
				pos += n
			case 4:
				ops = append(ops, "F")
			case 5:
				ops = append(ops, "C")
			case 6:
				ops = append(ops, "R")
			default:
				ops = append(ops, []string{"C,C", "F,F,F", "R,R", "C,F", "C,W0", "R,C", "W0,F"}[r.intn(7)])
			}
		}
		if pos < size {
			ops = append(ops, w(size-pos))
		}
		ops = append(ops, "C")
		if r.intn(3) == 0 {
			ops = append(ops, "C")
		}
		// Split the composite entries.
		return strings.Split(strings.Join(ops, ","), ",")
	}
}

func genFuzzCase(r *rng, maxSize int) caseSpec {
	level := allLevels[r.intn(len(allLevels))]
	kind := r.intn(16)
	size := fuzzSize(r, maxSize)
	wrapper := "flate"
	if r.intn(5) < 2 {
		wrapper = "zlib"
	}
	switch r.intn(10) {
	case 0:
		lim := []int{0, 1, 2, 5, 10, 100, 1000, 5000, 70000}[r.intn(9)] + r.intn(100)
		wrapper += "!b" + strconv.Itoa(lim)
	case 1:
		wrapper += "!c" + strconv.Itoa(1+r.intn(50))
	}
	dict := "none"
	switch r.intn(5) {
	case 0:
		dict = "empty"
	case 1:
		dsize := []int{1, 4, 12, 13, 100, 32767, 32768, 32769, 70000}[r.intn(9)]
		dict = fmt.Sprintf("gen:%d:%d:%d", r.intn(16), dsize, r.next()%1000000)
	}
	ops := fuzzOps(r, size)
	return caseSpec{wrapper, level, fmt.Sprintf("gen:%d:%d:%d", kind, size, r.next()%1000000), dict, ops}
}

func cmdFuzz(args []string) {
	fs := flag.NewFlagSet("fuzz", flag.ExitOnError)
	n := fs.Int("n", 5000, "number of cases")
	seed := fs.Uint64("seed", 1, "seed")
	maxSize := fs.Int("max", 400000, "maximum input size")
	_ = fs.Parse(args)
	r := &rng{s: *seed}
	w := bufio.NewWriter(os.Stdout)
	defer func() { _ = w.Flush() }()
	_, _ = fmt.Fprintf(w, "# go-flate oracle (%s) fuzz n=%d seed=%d max=%d\n", goVersion(), *n, *seed, *maxSize)
	for i := 0; i < *n; i++ {
		_, _ = fmt.Fprintln(w, formatCase(i, genFuzzCase(r, *maxSize), nil))
	}
}

// ---- random valid DEFLATE streams ----

type bitw struct {
	out []byte
	acc uint64
	n   uint
}

func (w *bitw) bits(v uint64, nb uint) {
	w.acc |= v << w.n
	w.n += nb
	for w.n >= 8 {
		w.out = append(w.out, byte(w.acc))
		w.acc >>= 8
		w.n -= 8
	}
}

func (w *bitw) align() {
	if w.n > 0 {
		w.out = append(w.out, byte(w.acc))
		w.acc, w.n = 0, 0
	}
}

func revBits(c uint32, l int) uint64 {
	var v uint64
	for i := 0; i < l; i++ {
		v = v<<1 | uint64(c>>uint(i)&1)
	}
	return v
}

// canonical assigns RFC 1951 canonical codes to the given lengths.
func canonical(lengths []int) []uint32 {
	var blCount [17]int
	for _, l := range lengths {
		if l > 0 {
			blCount[l]++
		}
	}
	var next [17]uint32
	code := uint32(0)
	for b := 1; b <= 16; b++ {
		code = (code + uint32(blCount[b-1])) << 1
		next[b] = code
	}
	codes := make([]uint32, len(lengths))
	for i, l := range lengths {
		if l > 0 {
			codes[i] = next[l]
			next[l]++
		}
	}
	return codes
}

// huffLengths returns length-limited Huffman code lengths for freq (a
// complete code when at least two symbols are used; length 1 for a single
// symbol). Limiting is done by flattening the frequencies and retrying.
func huffLengths(freq []int, limit int) []int {
	f := append([]int(nil), freq...)
	for {
		lengths := make([]int, len(f))
		type node struct {
			w     int
			syms  []int
			order int
		}
		var nodes []node
		for i, v := range f {
			if v > 0 {
				nodes = append(nodes, node{v, []int{i}, i})
			}
		}
		if len(nodes) == 0 {
			return lengths
		}
		if len(nodes) == 1 {
			lengths[nodes[0].syms[0]] = 1
			return lengths
		}
		ord := len(f)
		for len(nodes) > 1 {
			sort.Slice(nodes, func(i, j int) bool {
				if nodes[i].w != nodes[j].w {
					return nodes[i].w < nodes[j].w
				}
				return nodes[i].order < nodes[j].order
			})
			a, b := nodes[0], nodes[1]
			for _, s := range a.syms {
				lengths[s]++
			}
			for _, s := range b.syms {
				lengths[s]++
			}
			m := node{a.w + b.w, append(append([]int(nil), a.syms...), b.syms...), ord}
			ord++
			nodes = append([]node{m}, nodes[2:]...)
		}
		mx := 0
		for _, l := range lengths {
			mx = max(mx, l)
		}
		if mx <= limit {
			return lengths
		}
		for i := range f {
			if f[i] > 0 {
				f[i] = (f[i] + 1) / 2
			}
		}
	}
}

var (
	lenBase   = []int{3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258}
	lenExtra  = []int{0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0}
	distBase  = []int{1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577}
	distExtra = []int{0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13}
	clOrder   = []int{16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15}
)

type sym struct {
	lit    int // literal byte, 256 = EOB, -1 = match
	length int
	dist   int
	lcode  int
	lextra int
	dcode  int
	dextra int
}

func lengthSym(l int, alt bool) (code, extra int) {
	if l == 258 && alt {
		return 284, 31
	}
	for i := len(lenBase) - 1; i >= 0; i-- {
		if l >= lenBase[i] {
			return 257 + i, l - lenBase[i]
		}
	}
	panic("length")
}

func distSym(d int) (code, extra int) {
	for i := len(distBase) - 1; i >= 0; i-- {
		if d >= distBase[i] {
			return i, d - distBase[i]
		}
	}
	panic("dist")
}

// genSyms produces the symbols of one Huffman block and appends the decoded
// bytes to hist.
func genSyms(r *rng, hist *[]byte, n int) []sym {
	var syms []sym
	alpha := 1 + r.intn(256)
	base := r.intn(256)
	matchProb := r.intn(4)
	for i := 0; i < n; i++ {
		if len(*hist) > 0 && matchProb > 0 && r.intn(4) < matchProb {
			maxd := min(len(*hist), 32768)
			var d int
			switch r.intn(4) {
			case 0:
				d = 1 + r.intn(min(maxd, 4))
			case 1:
				d = maxd - r.intn(min(maxd, 3))
			default:
				d = 1 + r.intn(maxd)
			}
			var l int
			switch r.intn(4) {
			case 0:
				l = 3 + r.intn(8)
			case 1:
				l = 258
			default:
				l = 3 + r.intn(256)
			}
			alt := l == 258 && r.intn(4) == 0
			lc, le := lengthSym(l, alt)
			dc, de := distSym(d)
			syms = append(syms, sym{lit: -1, length: l, dist: d, lcode: lc, lextra: le, dcode: dc, dextra: de})
			for j := 0; j < l; j++ {
				*hist = append(*hist, (*hist)[len(*hist)-d])
			}
			continue
		}
		c := byte(base + r.intn(alpha))
		syms = append(syms, sym{lit: int(c)})
		*hist = append(*hist, c)
	}
	return append(syms, sym{lit: 256})
}

func writeSyms(bw *bitw, syms []sym, lcodes []uint32, llens []int, dcodes []uint32, dlens []int) {
	for _, s := range syms {
		if s.lit >= 0 {
			bw.bits(revBits(lcodes[s.lit], llens[s.lit]), uint(llens[s.lit]))
			continue
		}
		bw.bits(revBits(lcodes[s.lcode], llens[s.lcode]), uint(llens[s.lcode]))
		if e := lenExtra[s.lcode-257]; e > 0 {
			bw.bits(uint64(s.lextra), uint(e))
		}
		bw.bits(revBits(dcodes[s.dcode], dlens[s.dcode]), uint(dlens[s.dcode]))
		if e := distExtra[s.dcode]; e > 0 {
			bw.bits(uint64(s.dextra), uint(e))
		}
	}
}

// genStream builds a random DEFLATE stream; hist starts as the dictionary.
func genStream(r *rng, dict []byte, maxBlock int) (stream []byte, output []byte) {
	bw := &bitw{}
	hist := append([]byte(nil), dict...)
	nblocks := 1 + r.intn(6)
	for bi := 0; bi < nblocks; bi++ {
		final := bi == nblocks-1
		bf := uint64(0)
		if final {
			bf = 1
		}
		size := []int{0, 1, 10, 300, 5000, 40000, 70000}[r.intn(7)]
		size = r.intn(min(size, maxBlock) + 1)
		switch r.intn(3) {
		case 0: // stored
			size = min(size, 65535)
			bw.bits(bf, 1)
			bw.bits(0, 2)
			bw.align()
			bw.out = append(bw.out, byte(size), byte(size>>8), ^byte(size), ^byte(size>>8))
			for j := 0; j < size; j++ {
				var c byte
				if len(hist) > 0 && r.intn(2) == 0 {
					c = hist[len(hist)-1-r.intn(min(len(hist), 100))]
				} else {
					c = byte(r.next())
				}
				bw.out = append(bw.out, c)
				hist = append(hist, c)
			}
		case 1: // fixed
			syms := genSyms(r, &hist, size/20)
			ll := make([]int, 288)
			for i := range ll {
				switch {
				case i < 144:
					ll[i] = 8
				case i < 256:
					ll[i] = 9
				case i < 280:
					ll[i] = 7
				default:
					ll[i] = 8
				}
			}
			dl := make([]int, 30)
			for i := range dl {
				dl[i] = 5
			}
			bw.bits(bf, 1)
			bw.bits(1, 2)
			writeSyms(bw, syms, canonical(ll), ll, canonical(dl), dl)
		default: // dynamic
			syms := genSyms(r, &hist, size/20)
			lfreq := make([]int, 286)
			dfreq := make([]int, 30)
			for _, s := range syms {
				if s.lit >= 0 {
					lfreq[s.lit]++
				} else {
					lfreq[s.lcode]++
					dfreq[s.dcode]++
				}
			}
			// Unused but coded symbols.
			for k := r.intn(20); k > 0; k-- {
				lfreq[r.intn(286)] += 1 + r.intn(3)
			}
			if r.intn(3) == 0 {
				for k := r.intn(5); k > 0; k-- {
					dfreq[r.intn(30)]++
				}
			}
			ll := huffLengths(lfreq, 15)
			dl := huffLengths(dfreq, 15)
			nlit := 286
			for nlit > 257 && ll[nlit-1] == 0 {
				nlit--
			}
			ndist := 30
			for ndist > 1 && dl[ndist-1] == 0 {
				ndist--
			}
			all := append(append([]int(nil), ll[:nlit]...), dl[:ndist]...)
			// RLE encode the code lengths.
			type clsym struct{ c, extra, nb int }
			var cls []clsym
			useRLE := r.intn(4) != 0
			for i := 0; i < len(all); {
				l := all[i]
				run := 1
				for i+run < len(all) && all[i+run] == l {
					run++
				}
				if useRLE && l == 0 && run >= 3 {
					n := min(run, 138)
					if n >= 11 {
						cls = append(cls, clsym{18, n - 11, 7})
					} else {
						cls = append(cls, clsym{17, n - 3, 3})
					}
					i += n
					continue
				}
				if useRLE && l != 0 && i > 0 && all[i-1] == l && run >= 3 {
					n := min(run, 6)
					cls = append(cls, clsym{16, n - 3, 2})
					i += n
					continue
				}
				cls = append(cls, clsym{l, 0, 0})
				i++
			}
			cfreq := make([]int, 19)
			for _, c := range cls {
				cfreq[c.c]++
			}
			cl := huffLengths(cfreq, 7)
			nclen := 19
			for nclen > 4 && cl[clOrder[nclen-1]] == 0 {
				nclen--
			}
			ccodes := canonical(cl)
			bw.bits(bf, 1)
			bw.bits(2, 2)
			bw.bits(uint64(nlit-257), 5)
			bw.bits(uint64(ndist-1), 5)
			bw.bits(uint64(nclen-4), 4)
			for i := 0; i < nclen; i++ {
				bw.bits(uint64(cl[clOrder[i]]), 3)
			}
			for _, c := range cls {
				bw.bits(revBits(ccodes[c.c], cl[c.c]), uint(cl[c.c]))
				if c.nb > 0 {
					bw.bits(uint64(c.extra), uint(c.nb))
				}
			}
			ll2 := append(append([]int(nil), ll[:nlit]...), make([]int, 286-nlit)...)
			dl2 := append(append([]int(nil), dl[:ndist]...), make([]int, 30-ndist)...)
			writeSyms(bw, syms, canonical(ll2), ll2, canonical(dl2), dl2)
		}
	}
	bw.align()
	return bw.out, hist[len(dict):]
}

func cmdInflateGen(args []string) {
	fs := flag.NewFlagSet("inflate-gen", flag.ExitOnError)
	n := fs.Int("n", 2000, "number of streams")
	seed := fs.Uint64("seed", 7, "seed")
	binPath := fs.String("bin", "", "binary corpus output")
	maxBlock := fs.Int("maxblock", 70000, "maximum block input size")
	maxDict := fs.Int("maxdict", 40000, "maximum dictionary size")
	_ = fs.Parse(args)
	r := &rng{s: *seed}
	bf, err := os.Create(*binPath)
	if err != nil {
		panic(err)
	}
	bin := bufio.NewWriter(bf)
	w := bufio.NewWriter(os.Stdout)
	defer func() { _ = w.Flush() }()
	_, _ = fmt.Fprintf(w, "# go-flate oracle (%s) inflate-gen n=%d seed=%d maxblock=%d maxdict=%d\n", goVersion(), *n, *seed, *maxBlock, *maxDict)
	put := func(b []byte) {
		var tmp [4]byte
		binary.LittleEndian.PutUint32(tmp[:], uint32(len(b)))
		_, _ = bin.Write(tmp[:])
		_, _ = bin.Write(b)
	}
	for i := 0; i < *n; i++ {
		wrapper := "flate"
		if r.intn(3) == 0 {
			wrapper = "zlib"
		}
		var dict []byte
		hasDict := r.intn(4) == 0
		if hasDict {
			dict = genData(r.intn(16), 1+r.intn(*maxDict), r.next())
		}
		body, output := genStream(r, dict, *maxBlock)
		stream := body
		if wrapper == "zlib" {
			hdr := []byte{0x78, 0x9c}
			if hasDict {
				hdr[1] = 0xbb // FDICT, FCHECK valid for 0x78
			}
			stream = append([]byte(nil), hdr...)
			if hasDict {
				stream = binary.BigEndian.AppendUint32(stream, adler32.Checksum(dict))
			}
			stream = append(stream, body...)
			stream = binary.BigEndian.AppendUint32(stream, adler32.Checksum(output))
		}
		muts := "-"
		if len(stream) > 0 && r.intn(3) == 0 {
			var ms []string
			for k := 1 + r.intn(3); k > 0 && len(stream) > 0; k-- {
				switch r.intn(3) {
				case 0:
					cut := r.intn(len(stream))
					stream = stream[:cut]
					ms = append(ms, "t"+strconv.Itoa(cut))
				default:
					pos := r.intn(len(stream))
					bit := byte(1) << uint(r.intn(8))
					stream[pos] ^= bit
					ms = append(ms, fmt.Sprintf("x%d:%d", pos, bit))
				}
			}
			muts = strings.Join(ms, ",")
		}
		if r.intn(8) == 0 {
			stream = append(stream, genData(1, 1+r.intn(20), uint64(i))...)
		}
		// Reader dictionary: usually the writer's, sometimes wrong / missing.
		dictR := dict
		dictMode := "same"
		if hasDict && r.intn(6) == 0 {
			dictR = nil
			dictMode = "none"
		}
		readSize := []int{1, 7, 100, 4096, 32768, 70000}[r.intn(6)]
		out, callLog, consumed, errStr := decodeGo(wrapper, stream, dictR, readSize)
		_, _ = bin.WriteString(wrapper[:1])
		put(dictR)
		put(stream)
		_, _ = fmt.Fprintf(w, "%d %s %d %d %s %s %d %016x %016x %d %s\n", i, wrapper, len(dictR), readSize, dictMode, muts,
			len(out), fnv64(out), callLog, consumed, errStr)
	}
	if err := bin.Flush(); err != nil {
		panic(err)
	}
	if err := bf.Close(); err != nil {
		panic(err)
	}
}

func decodeGo(wrapper string, stream, dict []byte, readSize int) (out []byte, callLog uint64, consumed int, errStr string) {
	br := bytes.NewReader(stream)
	var r io.ReadCloser
	if wrapper == "flate" {
		if dict == nil {
			r = flate.NewReader(br)
		} else {
			r = flate.NewReaderDict(br, dict)
		}
	} else {
		var err error
		r, err = zlib.NewReaderDict(br, dict)
		if err != nil {
			return nil, 0, len(stream) - br.Len(), "new: " + errString(err)
		}
	}
	h := fnv.New64a()
	buf := make([]byte, readSize)
	var outBuf bytes.Buffer
	var final error
	for i := 0; i < 1<<22; i++ {
		n, err := r.Read(buf)
		outBuf.Write(buf[:n])
		_, _ = fmt.Fprintf(h, "%d:%s;", n, errString(err))
		if err != nil {
			final = err
			break
		}
	}
	closeErr := r.Close()
	return outBuf.Bytes(), h.Sum64(), len(stream) - br.Len(), errString(final) + " | close: " + errString(closeErr)
}
