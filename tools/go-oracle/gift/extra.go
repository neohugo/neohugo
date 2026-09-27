package main

// Dense sweeps reported as chunked FNV-1a digests (the Rust tests recompute
// the same sweeps): Go math functions, resampling kernels, resample weights
// (1-row resizes over many size pairs) and Rotate bounds; plus the setter
// table used by colorfunc(3).

import (
	"fmt"
	"image"
	"image/color"
	"math"
	"strconv"

	"github.com/disintegration/gift"
)

type fnv64 struct {
	h uint64
	n int
}

func newFnv() *fnv64 { return &fnv64{h: 14695981039346656037} }

func (f *fnv64) u64(v uint64) {
	for i := 0; i < 8; i++ {
		f.h ^= (v >> (8 * i)) & 0xff
		f.h *= 1099511628211
	}
	f.n++
}

const chunk = 1 << 20

// digester prints one line per chunk of 1<<20 values.
type digester struct {
	name string
	cur  *fnv64
	idx  int
}

func (d *digester) add(v uint64) {
	if d.cur == nil {
		d.cur = newFnv()
	}
	d.cur.u64(v)
	if d.cur.n == chunk {
		d.flush()
	}
}

func (d *digester) flush() {
	if d.cur == nil || d.cur.n == 0 {
		return
	}
	_, _ = fmt.Fprintf(out, "%s\t%d\t%d\t%016x\n", d.name, d.idx, d.cur.n, d.cur.h)
	d.idx++
	d.cur = nil
}

// sweep64 yields n float64 values whose bit patterns are evenly spaced
// between the bits of lo and hi (same sign).
func sweep64(lo, hi float64, n int, f func(x float64)) {
	a, b := math.Float64bits(lo), math.Float64bits(hi)
	step := (b - a) / uint64(n)
	for i := 0; i < n; i++ {
		f(math.Float64frombits(a + uint64(i)*step))
	}
}

func mathDigest(scale int) {
	n := scale << 20
	d := &digester{name: "exp"}
	sweep64(1e-12, 720, 2*n, func(x float64) { d.add(math.Float64bits(math.Exp(x))) })
	sweep64(-1e-12, -750, 2*n, func(x float64) { d.add(math.Float64bits(math.Exp(x))) })
	d.flush()

	d = &digester{name: "log"}
	sweep64(1e-300, 1e300, 2*n, func(x float64) { d.add(math.Float64bits(math.Log(x))) })
	sweep64(1e-6, 1e6, 2*n, func(x float64) { d.add(math.Float64bits(math.Log(x))) })
	d.flush()

	d = &digester{name: "pow"}
	var ys []float64
	for _, g := range []float32{0.5, 1.5, 2.2, 0.1, 3, 1e-5, 0.7, 1.3} {
		ys = append(ys, float64(1/max(g, 1.0e-5)))
	}
	ys = append(ys, 2.4, 1/2.4)
	for k := 0; k < 22; k++ {
		ys = append(ys, float64(math.Float32frombits(0x3e000000+uint32(k)*0x01234567%0x02000000)))
	}
	for _, y := range ys {
		for k := 0; k < 65536; k++ {
			x := float64(float32(k) * (1.0 / 0xffff))
			d.add(math.Float64bits(math.Pow(x, y)))
			d.add(math.Float64bits(math.Pow(float64((float32(x)+0.055)/1.055), y)))
		}
	}
	sweep64(1e-6, 1e6, n, func(x float64) { d.add(math.Float64bits(math.Pow(x, 0.3))) })
	d.flush()

	for _, fn := range []string{"sin", "cos", "sincos"} {
		d = &digester{name: fn}
		f := func(x float64) {
			switch fn {
			case "sin":
				d.add(math.Float64bits(math.Sin(x)))
			case "cos":
				d.add(math.Float64bits(math.Cos(x)))
			default:
				s, c := math.Sincos(x)
				d.add(math.Float64bits(s))
				d.add(math.Float64bits(c))
			}
		}
		sweep64(1e-9, 20, 2*n, f)
		sweep64(-1e-9, -20, n, f)
		sweep64(20, 1e15, n/4, f)
		d.flush()
	}
}

var kernelNames = resamplingNames

func kernelDigest(stride int) {
	for _, name := range kernelNames {
		r := resampling(name)
		d := &digester{name: name}
		// every stride-th float32 in [0, 4.5], then a coarser negative sweep.
		for b := uint32(0); b <= 0x40900000; b += uint32(stride) {
			d.add(uint64(math.Float32bits(r.Kernel(math.Float32frombits(b)))))
		}
		for b := uint32(0x80000000); b <= 0xc0900000; b += uint32(stride * 16) {
			d.add(uint64(math.Float32bits(r.Kernel(math.Float32frombits(b)))))
		}
		d.flush()
	}
}

// weightPairs returns the (src, dst) sizes of the weights sweep.
func weightPairs() [][2]int {
	var ps [][2]int
	seen := map[[2]int]bool{}
	add := func(s, d int) {
		if s < 1 || d < 1 || seen[[2]int{s, d}] {
			return
		}
		seen[[2]int{s, d}] = true
		ps = append(ps, [2]int{s, d})
	}
	for s := 1; s <= 160; s++ {
		for d := 1; d <= 12; d++ {
			add(s, d)
		}
		for d := s - 2; d <= s+2; d++ {
			add(s, d)
		}
		for _, d := range []int{2 * s, 3 * s, s / 2, s / 3, 2 * s / 3, 3 * s / 2, 5 * s / 4, 4 * s / 5} {
			add(s, d)
		}
	}
	for _, s := range []int{250, 480, 599, 600, 640, 1000, 1500, 2047} {
		for _, d := range []int{1, 7, 32, 100, 128, 200, 240, 300, 480, 600, 999} {
			add(s, d)
		}
	}
	return ps
}

// weightsDigest resizes 1-row (and 1-column) NRGBA64 images between many
// size pairs with every kernel.
func weightsDigest() {
	pairs := weightPairs()
	for _, name := range kernelNames {
		r := resampling(name)
		d := &digester{name: name}
		for _, p := range pairs {
			s, n := p[0], p[1]
			for _, vertical := range []bool{false, true} {
				sr := image.Rect(0, 0, s, 1)
				dr := image.Rect(0, 0, n, 1)
				if vertical {
					sr = image.Rect(0, 0, 1, s)
					dr = image.Rect(0, 0, 1, n)
				}
				src := genImage(imgSpec{typ: "nrgba64", rect: sr, seed: uint64(s*7919 + n), vmode: 0, amode: 2, valid: true})
				dst := image.NewNRGBA64(dr)
				gift.New(gift.Resize(dr.Dx(), dr.Dy(), r)).Draw(dst, src)
				for i := 0; i+7 < len(dst.Pix); i += 8 {
					var v uint64
					for k := 0; k < 8; k++ {
						v = v<<8 | uint64(dst.Pix[i+k])
					}
					d.add(v)
				}
			}
		}
		d.flush()
	}
	_, _ = fmt.Fprintf(out, "pairs\t%d\n", len(pairs))
}

// rotBoundsDigest digests gift.Rotate(angle, ...).Bounds over many sizes
// and angles.
func rotBoundsDigest() {
	d := &digester{name: "rotbounds"}
	var angles []float32
	for a := -360; a <= 360; a++ {
		angles = append(angles, float32(a))
	}
	r := &rng{s: 99}
	for i := 0; i < 1500; i++ {
		angles = append(angles, rf(r, -400, 400))
	}
	for _, a := range angles {
		f := gift.Rotate(a, color.Transparent, gift.CubicInterpolation)
		for w := 1; w <= 64; w++ {
			for h := 1; h <= 64; h += 3 {
				b := f.Bounds(image.Rect(0, 0, w, h))
				d.add(uint64(b.Dx())<<32 | uint64(b.Dy()))
			}
		}
		for _, wh := range [][2]int{{600, 480}, {640, 480}, {1500, 994}, {97, 1033}, {2000, 3}} {
			b := f.Bounds(image.Rect(0, 0, wh[0], wh[1]))
			d.add(uint64(b.Dx())<<32 | uint64(b.Dy()))
		}
	}
	d.flush()
}

// setterTable returns 65536 float32 values around the 8-bit and 16-bit
// rounding boundaries of the pixel setters, out-of-range and special values.
// Only integer operations and FMA-free float expressions are used, so the
// Rust tests rebuild the identical table.
func setterTable() []float32 {
	tab := make([]float32, 65536)
	for i := range tab {
		t := i % 1024
		var v float64
		kind := (i / 1024) % 8
		switch kind {
		case 0:
			v = (float64(t%256) + 0.5) / 255
		case 1:
			v = (float64((t*61)%65535) + 0.5) / 65535
		case 2:
			v = float64(t) / 1023
		case 3:
			v = (float64(t%256) + 0.5) / 255 * 0.5
		case 4:
			v = -float64(t) / 4096
		case 5:
			v = 1 + float64(t)/512
		case 6:
			v = (float64(t%256) + 0.5) / 65535 * 257
		}
		var bits uint32
		if kind == 7 {
			bits = []uint32{0x7fc00000, 0x7f800000, 0xff800000, 0, 0x80000000, 0x3f800000, 0x00000001, 0x7f7fffff}[t%8]
		} else {
			bits = math.Float32bits(float32(v))
			dlt := int32((i/8192)%9) - 4
			bits = uint32(int32(bits) + dlt)
		}
		tab[i] = math.Float32frombits(bits)
	}
	return tab
}

var setterTab = setterTable()

func init() {
	colorFuncs = append(colorFuncs, func(r0, g0, b0, a0 float32) (r, g, b, a float32) {
		k := int(math.Round(float64(r0) * 65535))
		return setterTab[k&0xffff], setterTab[(k*7+1)&0xffff], setterTab[(k*13+2)&0xffff], setterTab[(k*29+3)&0xffff]
	})
}

// setterCases prints synth-format lines that push every setterTable value
// through the setter of every settable image type (source "gray16all", a
// 256x256 Gray16 holding all 65536 values), and large DrawAt Over cases.
func setterCases() {
	i := 0
	for _, typ := range []string{"nrgba", "nrgba64", "rgba", "rgba64", "gray", "gray16", "paletted", "cmyk", "alpha", "alpha16"} {
		for seed := uint64(1); seed <= 3; seed++ {
			c := synthCase{
				op:      "draw",
				src:     imgSpec{typ: "gray16all", rect: image.Rect(0, 0, 256, 256)},
				dst:     imgSpec{typ: typ, rect: image.Rect(0, 0, 256, 256), seed: seed, vmode: int(seed % 2), amode: 2, valid: true, blank: true},
				filters: "colorfunc(3)",
			}
			if typ != "paletted" && seed > 1 {
				continue
			}
			_, _ = fmt.Fprintf(out, "%s\t%s\n", c.line(i), digest(runSynth(c)))
			i++
		}
	}
	r := &rng{s: 4242}
	for _, typ := range []string{"nrgba", "nrgba64", "rgba", "rgba64", "gray", "gray16", "paletted", "cmyk"} {
		for k := 0; k < 6; k++ {
			srcTyp := []string{"nrgba", "nrgba64", "rgba", "paletted", "gray", "ycbcr420"}[k]
			c := synthCase{
				op:  "drawat",
				src: imgSpec{typ: srcTyp, rect: image.Rect(k, 2*k, k+100+k*7, 2*k+90), seed: r.next(), vmode: k % 2, amode: 2, valid: true},
				dst: imgSpec{typ: typ, rect: image.Rect(-3, 5, 125, 133), seed: r.next(), vmode: 1 - k%2, amode: 2, valid: true},
				pt:  fmt.Sprintf("%d,%d,1", k*5-7, k*3),
			}
			if k%3 == 1 {
				c.filters = "resize(" + strconv.Itoa(60+k) + ",0,box)"
			}
			_, _ = fmt.Fprintf(out, "%s\t%s\n", c.line(i), digest(runSynth(c)))
			i++
		}
	}
}
