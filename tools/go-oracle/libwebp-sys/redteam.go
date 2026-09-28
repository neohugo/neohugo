package main

// Red-team corpus (-mode redteam): adversarial geometry, pixel-buffer
// lengths and option values for the gowebp wrapper, with an out-of-bounds
// read detector.
//
// Every case gives gowebp an image whose Pix is a window [G, G+len) of a
// larger buffer with G guard bytes on both sides, and encodes it with zero
// guard bytes, then with random non-zero ones and (when the widest possible
// read leaves Pix) with 0xff and 0x55/0xaa. If any output differs, libwebp
// read outside Pix (Go reads out of bounds there and crates/libwebp-sys
// must refuse the input with Error::PixOutOfRange); otherwise the Rust
// output must equal Go's. (A read whose bytes never reach the output, e.g.
// the import of a picture that WebPEncode then rejects for its size, is not
// detected, and must not be refused.) G is chosen from the widest
// read libwebp could make for the geometry, so the probe never leaves the
// allocation. Cases in the "early" class have unbounded theoretical reads
// but libwebp fails before reading (the Rust guard's claim); they run with a
// small guard, so a wrong claim would crash this oracle.
//
// The fixture stores the parameters, the length and FNV-1a-64 hash of the
// Pix window (whose bytes crates/libwebp-sys/tests/redteam.rs regenerates
// from the seed), the outcome and whether the guard bytes were read.

import (
	"bufio"
	"bytes"
	"fmt"
	"image"
	"math"
	"os"
	"path/filepath"

	"github.com/bep/gowebp/libwebp/webpoptions"
)

type rtParams struct {
	class                  string
	kind                   string
	minX, minY, maxX, maxY int
	stride                 int
	plen                   int
	quality, preset        int
	sharp                  bool
	guard                  int
}

// rtPixSeed derives the seed of the Pix bytes from the case seed.
func rtPixSeed(seed uint64) uint64 { return seed ^ 0x6a09e667f3bcc908 }

// cReadExtent returns the byte range [lo, hi) relative to &Pix[0] that
// libwebp reads if it reads the image at all (C int truncation of the
// width, height and stride, as in gowebp's C.int conversions), and whether
// it reads anything (positive dimensions).
func cReadExtent(kind string, maxX, maxY, stride int) (lo, hi int64, reads bool) {
	w, h, s := int64(int32(maxX)), int64(int32(maxY)), int64(int32(stride))
	if w <= 0 || h <= 0 {
		return 0, 0, false
	}
	bpp := int64(4)
	if kind == "gray" {
		bpp = 1
	}
	last := (h - 1) * s
	lo, hi = min(last, 0), max(last, 0)+w*bpp
	return lo, hi, true
}

func pickInt(r *rng, xs ...int) int { return xs[r.intn(len(xs))] }

func genRTCase(seed uint64) rtParams {
	r := &rng{s: seed}
	var p rtParams
	p.kind = []string{"nrgba", "rgba", "gray"}[r.intn(3)]
	bpp := 4
	if p.kind == "gray" {
		bpp = 1
	}
	switch k := r.intn(20); {
	case k < 11:
		p.class = "small"
		w, h := r.rangeIn(1, 40), r.rangeIn(1, 40)
		if r.intn(3) == 0 {
			p.minX, p.minY = r.rangeIn(-3, 4), r.rangeIn(-3, 4)
		}
		p.maxX, p.maxY = p.minX+w, p.minY+h
	case k < 13:
		p.class = "strip"
		if r.intn(2) == 0 {
			p.maxX, p.maxY = pickInt(r, 16381, 16382, 16383, 16384, 16385, 20000), r.rangeIn(1, 2)
		} else {
			p.maxX, p.maxY = r.rangeIn(1, 2), pickInt(r, 16381, 16382, 16383, 16384, 16385, 20000)
		}
	case k < 15:
		p.class = "degenerate"
		p.minX, p.minY = r.rangeIn(-6, 2), r.rangeIn(-6, 2)
		p.maxX, p.maxY = r.rangeIn(-5, 12), r.rangeIn(-5, 12)
		if p.maxX < p.minX {
			p.minX, p.maxX = p.maxX, p.minX
		}
		if p.maxY < p.minY {
			p.minY, p.maxY = p.maxY, p.minY
		}
	case k < 17:
		p.class = "tiny"
		p.maxX, p.maxY = r.rangeIn(1, 4), r.rangeIn(1, 4)
	default:
		p.class = "early"
		// Geometries on which libwebp fails before reading any pixel.
		switch r.intn(5) {
		case 0: // |stride| < 4*width (RGBA only; checked in Import)
			if p.kind == "gray" {
				p.kind = "nrgba"
			}
			p.maxX, p.maxY = pickInt(r, 1<<20, 1<<28, 1<<29-1), pickInt(r, 1, 2, 1000, 1<<20)
			p.stride = pickInt(r, 4, 16, 400, -400, 1<<20, -(1 << 20), 1<<32+400)
		case 4: // 4*width wraps in C int; the allocation then fails
			if p.kind == "gray" {
				p.kind = "rgba"
			}
			p.maxX, p.maxY = pickInt(r, 1<<30+1, math.MaxInt32, 1<<30), pickInt(r, 1<<20, 1<<24)
			p.stride = pickInt(r, 4, 16, -16, 1<<20)
		case 1: // width*height too large to allocate (> 2^34 bytes)
			if p.kind == "gray" {
				p.kind = "rgba"
			}
			p.maxX, p.maxY = pickInt(r, 100000, 1<<20, 1<<24), pickInt(r, 1<<20, 1<<24, 100000)
			p.stride = p.maxX * 4
		case 2: // gray above WEBP_MAX_DIMENSION
			p.kind = "gray"
			p.maxX, p.maxY = pickInt(r, 16384, 20000, 1<<20, 1), pickInt(r, 1, 3, 16384, 1<<24)
			if p.maxX <= 16383 && p.maxY <= 16383 {
				p.maxY = 16384
			}
			p.stride = pickInt(r, p.maxX, 1, 0, -p.maxX)
		default: // non-positive width or height with a huge other side
			p.maxX, p.maxY = pickInt(r, 0, -1, -(1<<20), 1<<30), pickInt(r, 0, -3, 1<<30)
			if p.maxX > 0 && p.maxY > 0 {
				p.maxY = 0
			}
			p.stride = pickInt(r, 4, 1<<20, -(1 << 20), 0)
		}
		p.plen = r.rangeIn(1, 64)
	}
	if p.class != "early" {
		exact := p.maxX * bpp
		switch r.intn(12) {
		case 0, 1, 2, 3:
			p.stride = exact
		case 4:
			p.stride = exact + r.rangeIn(1, 9)
		case 5:
			p.stride = exact - r.rangeIn(1, 3)
		case 6:
			p.stride = 0
		case 7:
			p.stride = -exact - r.intn(4)
		case 8:
			p.stride = (p.maxX - p.minX) * bpp
		case 9:
			p.stride = r.rangeIn(-exact-8, exact+8)
		case 10:
			// Go int -> C.int truncation.
			p.stride = pickInt(r, 1<<32, 1<<33) + exact + r.intn(3)
		default:
			p.stride = pickInt(r, 1<<31, 1<<31-1, -(1 << 31), 1<<32-1, -(1<<32)-exact)
		}
		lo, hi, reads := cReadExtent(p.kind, p.maxX, p.maxY, p.stride)
		need := int64(1)
		if reads {
			need = max(hi, 1)
		}
		switch r.intn(10) {
		case 0:
			p.plen = 0
		case 1:
			p.plen = 1
		case 2:
			p.plen = int(need) + r.rangeIn(1, 64)
		case 3:
			p.plen = r.rangeIn(0, int(min(need, 1<<20))+8)
		case 4, 5:
			p.plen = max(int(need)-r.rangeIn(1, 3), 0)
		default:
			p.plen = int(need)
		}
		if reads {
			g := max(-lo, hi-int64(p.plen), 0) + 64
			if g > 64<<20 || int64(p.plen) > 64<<20 {
				// Unbounded probe: fall back to a harmless geometry.
				p.stride = exact
				p.plen = max(exact*p.maxY, 1)
				g = 64
			}
			p.guard = int(g)
		} else {
			p.guard = 64
		}
	} else {
		p.guard = 64
	}

	switch q := r.intn(12); {
	case q < 4:
		p.quality, p.preset, p.sharp = 75, 2, true
	case q < 6:
		p.quality, p.preset = 0, r.rangeIn(0, 5)
	case q < 8:
		p.quality, p.preset = r.rangeIn(1, 100), r.rangeIn(0, 5)
	default:
		p.quality = pickInt(r, 100, 1, 99, 50, 0, 75)
		p.preset = pickInt(r, 0, 1, 2, 3, 4, 5)
	}
	if r.intn(3) == 0 {
		p.sharp = !p.sharp
	}
	return p
}

// rtImage builds the gowebp input on the Pix window.
func rtImage(p rtParams, pix []byte) image.Image {
	rect := image.Rectangle{Min: image.Point{X: p.minX, Y: p.minY}, Max: image.Point{X: p.maxX, Y: p.maxY}}
	switch p.kind {
	case "nrgba":
		return &image.NRGBA{Pix: pix, Stride: p.stride, Rect: rect}
	case "rgba":
		return &image.RGBA{Pix: pix, Stride: p.stride, Rect: rect}
	default:
		return &image.Gray{Pix: pix, Stride: p.stride, Rect: rect}
	}
}

// optionCases returns the option sweep: every quality around and past the
// bounds and every preset around the enum range and past int32/uint32.
func optionCases() []webpoptions.EncodingOptions {
	var opts []webpoptions.EncodingOptions
	qualities := []int{math.MinInt64, math.MinInt32, -1 << 24, -101, -1, 0, 100, 101, 102, 1000, 1 << 24, 1<<24 + 1, math.MaxInt32, 1 << 32, math.MaxInt64}
	for q := 1; q < 100; q += 7 {
		qualities = append(qualities, q)
	}
	presets := []int{-1 << 32, math.MinInt64, -(1 << 31), -2, -1, 0, 1, 2, 3, 4, 5, 6, 7, 100, 1 << 31, 1<<32 - 1, 1 << 32, 1<<32 + 2, 1<<32 + 5, math.MaxInt64}
	for _, q := range qualities {
		for _, pr := range presets {
			for _, sharp := range []bool{false, true} {
				opts = append(opts, webpoptions.EncodingOptions{Quality: q, EncodingPreset: webpoptions.EncodingPreset(pr), UseSharpYuv: sharp})
			}
		}
	}
	return opts
}

// redteamCases writes <out>/redteam.tsv: n generated cases from seed, and,
// when withOpts is set, the option sweep over three fixed images first.
func redteamCases(out string, seed uint64, n int, withOpts bool) {
	if err := os.MkdirAll(out, 0o755); err != nil {
		panic(err)
	}
	f, err := os.Create(filepath.Join(out, "redteam.tsv"))
	if err != nil {
		panic(err)
	}
	w := bufio.NewWriter(f)
	_, _ = fmt.Fprintln(w, "#seed\tclass\tkind\tminx\tminy\tmaxx\tmaxy\tstride\tpix_len\tquality\tpreset\tsharp\tpix_fnv\tout_len\tout_fnv\tread_outside\texpect")
	counts := map[string]int{}
	emit := func(seed uint64, p rtParams, pixFnv uint64, out []byte, sensitive bool, exp string) {
		sharp, sens := 0, 0
		if p.sharp {
			sharp = 1
		}
		if sensitive {
			sens = 1
		}
		_, _ = fmt.Fprintf(w, "%d\t%s\t%s\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%016x\t%d\t%016x\t%d\t%s\n",
			seed, p.class, p.kind, p.minX, p.minY, p.maxX, p.maxY, p.stride, p.plen, p.quality, p.preset, sharp,
			pixFnv, len(out), fnv64(out), sens, exp)
		key := p.class + "/" + exp
		if sensitive {
			key += "/read-outside"
		}
		if len(key) > 60 {
			key = key[:60]
		}
		counts[key]++
	}
	run := func(seed uint64, p rtParams) {
		buf := make([]byte, 2*p.guard+p.plen)
		pr := &rng{s: rtPixSeed(seed)}
		pix := buf[p.guard : p.guard+p.plen : p.guard+p.plen]
		for i := range pix {
			pix[i] = byte(pr.next())
		}
		o := webpoptions.EncodingOptions{Quality: p.quality, EncodingPreset: webpoptions.EncodingPreset(p.preset), UseSharpYuv: p.sharp}
		img := rtImage(p, pix)
		outA, expA := encodeCase(img, o)
		// Other guard contents: random non-zero bytes, and (when the
		// widest possible read leaves Pix) 0xff and 0x55/0xaa, since a
		// single out-of-bounds pixel can encode the same for two values.
		fills := []func(i int) byte{nil}
		lo, hi, reads := cReadExtent(p.kind, p.maxX, p.maxY, p.stride)
		if reads && (lo < 0 || hi > int64(p.plen)) {
			fills = append(fills, func(int) byte { return 0xff }, func(i int) byte { return 0x55 << (i & 1) })
		}
		sensitive := false
		for _, fill := range fills {
			gr := &rng{s: seed ^ 0xbb67ae8584caa73b}
			for i := range buf {
				if i >= p.guard && i < p.guard+p.plen {
					continue
				}
				if fill == nil {
					buf[i] = byte(gr.next()) | 1
				} else {
					buf[i] = fill(i)
				}
			}
			outB, expB := encodeCase(img, o)
			if expA != expB || !bytes.Equal(outA, outB) {
				sensitive = true
				break
			}
		}
		emit(seed, p, fnv64(pix), outA, sensitive, expA)
	}
	if withOpts {
		// Seeds 1..3 of the option sweep use fixed images: NRGBA with alpha,
		// opaque RGBA, gray.
		bases := []rtParams{
			{class: "opts", kind: "nrgba", maxX: 23, maxY: 17, stride: 23 * 4, plen: 23 * 4 * 17, guard: 64},
			{class: "opts", kind: "rgba", maxX: 16, maxY: 9, stride: 16 * 4, plen: 16 * 4 * 9, guard: 64},
			{class: "opts", kind: "gray", maxX: 9, maxY: 13, stride: 9, plen: 9 * 13, guard: 64},
		}
		for bi, b := range bases {
			for _, o := range optionCases() {
				p := b
				p.quality, p.preset, p.sharp = o.Quality, int(o.EncodingPreset), o.UseSharpYuv
				run(uint64(bi+1), p)
			}
		}
	}
	for i := 0; i < n; i++ {
		s := seed + uint64(i)
		run(s, genRTCase(s))
	}
	fmt.Fprintf(os.Stderr, "redteam: %d cases\n", n)
	for k, v := range counts {
		fmt.Fprintf(os.Stderr, "  %-60s %d\n", k, v)
	}
	if err := w.Flush(); err != nil {
		panic(err)
	}
	if err := f.Close(); err != nil {
		panic(err)
	}
}
