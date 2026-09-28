package main

// Red-team generators (the independent adversarial pass). They print
// synth-format lines (main.go: synthCase.line + digest) that
// crates/gift/tests/redteam.rs replays; the Rust side only needs the image
// spec grammar below and the filter parser, not these generators.
//
// Image spec types (imgSpec.typ) accepted by genImageX, on top of genImage's:
//
//	w.<typ>                 the image behind a wrapper that hides its concrete
//	                        type (gift's generic getter/setter paths; no Opaque)
//	<typ>@x0/y0/x1/y1       a SubImage (rect = the spec rect) of a <typ> image
//	                        generated over the parent rect x0,y0,x1,y1
//	nycbcra422 … 410        NYCbCrA in every subsample ratio
//	palettedx               Paletted with genPalette2 colours (all colour kinds,
//	                        up to 300 entries)
//	palettedoor             like palettedx, pixel indexes in 0..255 (may be out
//	                        of the palette's range: Go panics when gift reads one)
//	palettedempty           Paletted with an empty palette
//	rectimg                 image.Rectangle used as an image
//	grayseq                 Gray whose Pix[i] = byte(i) (the witness source)
//
// Modes (gift rt <mode> <n> <seed>): synth2, params, resample, drawat, hugo,
// far, panic; gift rtbounds <n> <seed> for Bounds of chains with extreme
// parameters; gift rtwitness for the witness() cases.
//
// Go panics inside gift's parallelize goroutines cannot be recovered, and
// every mode can reach one (e.g. image.Paletted.Opaque of a palette with
// more than 256 entries), so runs generate cases with
// `gift rtgen <mode> <n> <seed>` and run them with `gift rtrun <file>
// <start>`, which prints one flushed result line per case, through the
// driver rtrun.py: it restarts the oracle after a crash and records the
// crashing case as "panic".

import (
	"bufio"
	"fmt"
	"image"
	"image/color"
	"image/draw"
	"math"
	"os"
	"runtime"
	"strconv"
	"strings"

	"github.com/disintegration/gift"
)

// wrapImg hides the concrete type of an image (gift's generic getter).
type wrapImg struct{ image.Image }

// wrapDraw hides the concrete type of a settable image (gift's generic
// getter and setter).
type wrapDraw struct{ draw.Image }

type subImager interface {
	SubImage(r image.Rectangle) image.Image
}

func parseSlashRect(s string) image.Rectangle {
	f := strings.Split(s, "/")
	if len(f) != 4 {
		panic("bad rect " + s)
	}
	return image.Rect(pi(f[0]), pi(f[1]), pi(f[2]), pi(f[3]))
}

func slashRect(r image.Rectangle) string {
	return fmt.Sprintf("%d/%d/%d/%d", r.Min.X, r.Min.Y, r.Max.X, r.Max.Y)
}

// genImageX is genImage plus the red-team spec grammar (see above).
func genImageX(s imgSpec) image.Image {
	typ := s.typ
	wrap := strings.HasPrefix(typ, "w.")
	if wrap {
		typ = typ[2:]
	}
	var img image.Image
	if at := strings.IndexByte(typ, '@'); at >= 0 {
		ps := s
		ps.typ = typ[:at]
		ps.rect = parseSlashRect(typ[at+1:])
		img = genImageBase(ps).(subImager).SubImage(s.rect)
	} else {
		ps := s
		ps.typ = typ
		img = genImageBase(ps)
	}
	if wrap {
		if d, ok := img.(draw.Image); ok {
			return wrapDraw{d}
		}
		return wrapImg{img}
	}
	return img
}

func genImageBase(s imgSpec) image.Image {
	r := &rng{s: s.seed}
	switch s.typ {
	case "nycbcra422", "nycbcra440", "nycbcra411", "nycbcra410":
		m := image.NewNYCbCrA(s.rect, ycbcrRatio(s.typ))
		if !s.blank {
			fill8(r, m.Y, s.vmode)
			fill8(r, m.Cb, s.vmode)
			fill8(r, m.Cr, s.vmode)
			for i := range m.A {
				m.A[i] = genAlpha8(r, s.amode)
			}
		}
		return m
	case "palettedx", "palettedoor":
		p := genPalette2(r, s.vmode, s.amode, s.valid)
		m := image.NewPaletted(s.rect, p)
		if !s.blank {
			for i := range m.Pix {
				if s.typ == "palettedoor" {
					m.Pix[i] = r.byte()
				} else {
					m.Pix[i] = byte(r.intn(len(p)))
				}
			}
		}
		return m
	case "palettedempty":
		return image.NewPaletted(s.rect, color.Palette{})
	case "rectimg":
		return s.rect
	case "grayseq":
		m := image.NewGray(s.rect)
		for i := range m.Pix {
			m.Pix[i] = byte(i)
		}
		return m
	}
	return genImage(s)
}

// genColor2 returns a colour of any standard kind (genColor's six plus
// Gray16, Alpha16, CMYK, YCbCr and NYCbCrA).
func genColor2(r *rng, vmode, amode int, valid bool) color.Color {
	switch k := r.intn(11); k {
	case 6:
		return color.Gray16{genVal16(r, vmode)}
	case 7:
		return color.Alpha16{genAlpha16(r, amode)}
	case 8:
		return color.CMYK{genVal(r, vmode), genVal(r, vmode), genVal(r, vmode), genVal(r, vmode)}
	case 9:
		return color.YCbCr{genVal(r, vmode), genVal(r, vmode), genVal(r, vmode)}
	case 10:
		return color.NYCbCrA{YCbCr: color.YCbCr{Y: genVal(r, vmode), Cb: genVal(r, vmode), Cr: genVal(r, vmode)}, A: genAlpha8(r, amode)}
	default:
		return genColorK(r, k, vmode, amode, valid)
	}
}

func genPalette2(r *rng, vmode, amode int, valid bool) color.Palette {
	var n int
	switch r.intn(4) {
	case 0:
		n = 1 + r.intn(2)
	case 1:
		n = 1 + r.intn(16)
	case 2:
		n = 1 + r.intn(256)
	default:
		n = 257 + r.intn(44)
	}
	p := make(color.Palette, n)
	for i := range p {
		p[i] = genColor2(r, vmode, amode, valid)
	}
	return p
}

// colorStr2 extends colorStr with the extra colour kinds.
func colorStr2(c color.Color) string {
	switch c := c.(type) {
	case color.Gray16:
		return fmt.Sprintf("gray16/%d", c.Y)
	case color.Alpha16:
		return fmt.Sprintf("alpha16/%d", c.A)
	case color.CMYK:
		return fmt.Sprintf("cmyk/%d/%d/%d/%d", c.C, c.M, c.Y, c.K)
	case color.YCbCr:
		return fmt.Sprintf("ycbcr/%d/%d/%d", c.Y, c.Cb, c.Cr)
	case color.NYCbCrA:
		return fmt.Sprintf("nycbcra/%d/%d/%d/%d", c.Y, c.Cb, c.Cr, c.A)
	}
	return colorStr(c)
}

func parseColor2(s string) color.Color {
	f := strings.Split(s, "/")
	u := func(i int) int { return pi(f[i]) }
	switch f[0] {
	case "gray16":
		return color.Gray16{uint16(u(1))}
	case "alpha16":
		return color.Alpha16{uint16(u(1))}
	case "cmyk":
		return color.CMYK{uint8(u(1)), uint8(u(2)), uint8(u(3)), uint8(u(4))}
	case "ycbcr":
		return color.YCbCr{uint8(u(1)), uint8(u(2)), uint8(u(3))}
	case "nycbcra":
		return color.NYCbCrA{YCbCr: color.YCbCr{Y: uint8(u(1)), Cb: uint8(u(2)), Cr: uint8(u(3))}, A: uint8(u(4))}
	}
	return parseColor(s)
}

// digestX is digest for genImageX images (wrappers are unwrapped).
func digestX(img image.Image) string {
	if w, ok := img.(wrapDraw); ok {
		return digest(w.Image)
	}
	return digest(img)
}

// witnessTab holds float32 pixels (r, g, b, a bits) for which a pixel
// setter's fused `v*k + 0.5` (FMADDS) truncates differently from the
// unfused `float32(v*k) + 0.5`: the RGBA setter's f32u8(r*fa) and the RGBA64
// setter's f32u16(r*fa), fa = a*255 or a*65535 (found by exact search; see
// crates/gift/PORTING.md). Entry 0 is an ordinary pixel.
var witnessTab = [][4]uint32{
	{0x3f000000, 0x3e800000, 0x3f400000, 0x3f800000},
	{0x3d2026c5, 0x3d2026c5, 0x3d2026c5, 0x3d4d68a1}, // RGBA f32u8: fused 0, unfused 1
	{0x3d20086b, 0x3d20086b, 0x00000000, 0x3d4d8f96}, // RGBA f32u8
	{0x391fc3d7, 0x391fc3d7, 0x391fc3d7, 0x3d4d1ab7}, // RGBA64 f32u16: fused 0, unfused 1
	{0x391f873d, 0x3d2026c5, 0x391f873d, 0x3d4d68a1}, // RGBA64 r/b f32u16, RGBA g f32u8
}

// witnessFunc is the ColorFunc of the witness() filter: the source is a
// grayseq image, pixel value k selects witnessTab[k % len].
func witnessFunc(r0, g0, b0, a0 float32) (r, g, b, a float32) {
	k := int(math.Round(float64(r0) * 255))
	w := witnessTab[k%len(witnessTab)]
	return math.Float32frombits(w[0]), math.Float32frombits(w[1]), math.Float32frombits(w[2]), math.Float32frombits(w[3])
}

// parseFilterX is parseFilter with the extended colour syntax for rotate,
// witness() (see witnessTab) and overlayg(typ,x0,y0,x1,y1,seed,vmode,x,y):
// Hugo's overlayFilter with a generated overlay image (amode 2, valid).
func parseFilterX(s string) Filter {
	if s == "witness()" {
		return gift.ColorFunc(witnessFunc)
	}
	if strings.HasPrefix(s, "rotate(") {
		inner := s[len("rotate(") : len(s)-1]
		a := strings.Split(inner, ",")
		return gift.Rotate(pf(a[0]), parseColor2(a[1]), gift.Interpolation(pi(a[2])))
	}
	if strings.HasPrefix(s, "overlayg(") {
		a := strings.Split(s[len("overlayg("):len(s)-1], ",")
		seed, err := strconv.ParseUint(a[5], 10, 64)
		if err != nil {
			panic(err)
		}
		spec := imgSpec{typ: a[0], rect: image.Rect(pi(a[1]), pi(a[2]), pi(a[3]), pi(a[4])), seed: seed, vmode: pi(a[6]), amode: 2, valid: true}
		return overlayFilter{src: genImageX(spec), x: pi(a[7]), y: pi(a[8])}
	}
	return parseFilter(s)
}

func parseFiltersX(s string) []Filter {
	if s == "" || s == "-" {
		return nil
	}
	var fs []Filter
	for _, p := range strings.Split(s, "|") {
		fs = append(fs, parseFilterX(p))
	}
	return fs
}

func runSynthX(c synthCase) image.Image {
	src := genImageX(c.src)
	g := gift.New(parseFiltersX(c.filters)...)
	dst := genImageX(c.dst).(draw.Image)
	if c.op == "drawat" {
		f := strings.Split(c.pt, ",")
		g.DrawAt(dst, src, image.Pt(pi(f[0]), pi(f[1])), gift.Operator(pi(f[2])))
	} else {
		g.Draw(dst, src)
	}
	return dst
}

// ---------------------------------------------------------------------------
// parameter pickers

var (
	f32NaN  = float32(math.NaN())
	f32Inf  = float32(math.Inf(1))
	f32NInf = float32(math.Inf(-1))
)

// extremeF are float32 parameters at and beyond every filter's bounds.
var extremeF = []float32{
	f32NaN, f32Inf, f32NInf, math.MaxFloat32, -math.MaxFloat32, math.SmallestNonzeroFloat32,
	-math.SmallestNonzeroFloat32, 1e-38, float32(math.Copysign(0, -1)), 0, 1, -1, 1e10, -1e10,
	0.5, 2, 100, -100, 500, -500, 1e-7, 16777217,
}

func pickX(r *rng, specials []float32, lo, hi float32) float32 {
	switch r.intn(4) {
	case 0:
		return extremeF[r.intn(len(extremeF))]
	case 1:
		return specials[r.intn(len(specials))]
	default:
		return rf(r, lo, hi)
	}
}

// pickXSafe is pickX without the values that make gift allocate unbounded
// memory or run for unbounded time (blur sigmas).
func pickXSafe(r *rng, specials []float32, lo, hi float32) float32 {
	for {
		v := pickX(r, specials, lo, hi)
		if math.IsInf(float64(v), 0) || math.Abs(float64(v)) > float64(hi)*4+1 {
			continue
		}
		return v
	}
}

var bigInts = []int{math.MaxInt64, math.MinInt64, math.MaxInt64 - 1, math.MinInt64 + 1, math.MaxInt32, math.MinInt32, 1 << 40, -(1 << 40), 1 << 62}

var primes = []int{2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97, 101, 127, 131, 251, 257, 509, 521, 997, 1009, 2003, 4001, 7919}

func pickColor(r *rng) string {
	return colorStr2(genColor2(r, r.intn(2), r.intn(3), r.intn(2) == 0))
}

// genFilterX generates one filter for a source of bounds b. level: 0 = safe
// parameters (bounded memory and time), 1 = also extreme parameters that may
// make Go panic (only the panic mode uses it).
func genFilterX(r *rng, b image.Rectangle, allowLut bool, level int) string {
	w, h := b.Dx(), b.Dy()
	sz := func(n int) int {
		switch r.intn(10) {
		case 0:
			return 0
		case 1:
			return -1 - r.intn(3)
		case 2:
			return 1
		case 3:
			return primes[r.intn(len(primes))] % (4*n + 8)
		case 4:
			return 1 + r.intn(3)
		default:
			return 1 + r.intn(2*n+4)
		}
	}
	res := func() string { return resamplingNames[r.intn(len(resamplingNames))] }
	anchor := func() int { return r.intn(12) - 1 }
	ff := func(specials []float32, lo, hi float32) string { return fstr(pickX(r, specials, lo, hi)) }
	for {
		switch k := r.intn(46); {
		case k < 6:
			return fmt.Sprintf("resize(%d,%d,%s)", sz(w), sz(h), res())
		case k < 8:
			return fmt.Sprintf("resizetofit(%d,%d,%s)", sz(w), sz(h), res())
		case k < 10:
			return fmt.Sprintf("resizetofill(%d,%d,%s,%d)", sz(w), sz(h), res(), anchor())
		case k == 10:
			x0 := b.Min.X - 3 + r.intn(w+6)
			y0 := b.Min.Y - 3 + r.intn(h+6)
			x1 := x0 - 2 + r.intn(w+6)
			y1 := y0 - 2 + r.intn(h+6)
			if r.intn(5) == 0 {
				x1 = bigInts[r.intn(len(bigInts))]
			}
			if r.intn(5) == 0 {
				y0 = bigInts[r.intn(len(bigInts))]
			}
			return fmt.Sprintf("crop(%d,%d,%d,%d)", x0, y0, x1, y1)
		case k == 11:
			cw, ch := r.intn(w+4)-1, r.intn(h+4)-1
			if r.intn(6) == 0 {
				cw = bigInts[r.intn(len(bigInts))]
			}
			if r.intn(6) == 0 {
				ch = bigInts[r.intn(len(bigInts))]
			}
			return fmt.Sprintf("croptosize(%d,%d,%d)", cw, ch, anchor())
		case k == 12:
			return []string{"rotate90()", "rotate180()", "rotate270()", "fliph()", "flipv()", "transpose()", "transverse()"}[r.intn(7)]
		case k < 16:
			ang := pickX(r, []float32{0, 90, 180, 270, -90, 30, 45, -45, 360, 1e-3, 89.99999, 90.00001, 1e9, -1e20, 3e38, 720.5}, -400, 400)
			return fmt.Sprintf("rotate(%s,%s,%d)", fstr(ang), pickColor(r), r.intn(5)-1)
		case k == 16:
			return []string{"invert()", "grayscale()", "sobel()"}[r.intn(3)]
		case k < 19:
			if !allowLut {
				continue
			}
			switch r.intn(4) {
			case 0:
				return "srgbtolinear()"
			case 1:
				return "lineartosrgb()"
			case 2:
				return fmt.Sprintf("gamma(%s)", ff([]float32{0, 0.5, 1, 1.5, 2.2, -1, 1e-6, 1e-5, 9.99e-6}, 0, 5))
			default:
				return fmt.Sprintf("sigmoid(%s,%s)", ff([]float32{0, 0.5, 1, -0.5, 1.5}, -0.5, 1.5), ff([]float32{0, 7, -7, 1e-3, -1e-3, 88, -88, 200}, -15, 15))
			}
		case k == 19:
			return fmt.Sprintf("contrast(%s)", ff([]float32{0, -100, 100, 30, -30, 150, -150, 99.99, 99.999, -99.999, 50}, -150, 150))
		case k == 20:
			return fmt.Sprintf("brightness(%s)", ff([]float32{0, -100, 100, 30, -30, 150, -150, 0.001}, -150, 150))
		case k == 21:
			return fmt.Sprintf("sepia(%s)", ff([]float32{0, 100, 50, -10, 120}, -20, 120))
		case k == 22:
			return fmt.Sprintf("hue(%s)", ff([]float32{0, 45, -45, 180, 360, -360, 720, 1e-4, 359.9999, -0.0001, 1e9}, -720, 720))
		case k == 23:
			return fmt.Sprintf("saturation(%s)", ff([]float32{0, 50, -50, -100, 500, 600, -150, 1e-5}, -150, 600))
		case k == 24:
			return fmt.Sprintf("colorize(%s,%s,%s)", ff([]float32{0, 240, 360, -30, 1e9}, -400, 400), ff([]float32{0, 50, 100, 110}, -10, 110), ff([]float32{0, 100, 50, -10, 1e-5}, -10, 110))
		case k == 25:
			return fmt.Sprintf("colorbalance(%s,%s,%s)", ff([]float32{0, 10, -10, -100, 500}, -150, 600), ff([]float32{0, 10, -10, -100, 500}, -150, 600), ff([]float32{0, 10, -10, -100, 500}, -150, 600))
		case k == 26:
			return fmt.Sprintf("threshold(%s)", ff([]float32{0, 50, 100, -10, 110, 49.99}, -10, 110))
		case k == 27:
			return fmt.Sprintf("colorfunc(%d)", r.intn(numSynthColorFuncs))
		case k < 31:
			n := r.intn(52)
			var ks []string
			for i := 0; i < n; i++ {
				v := float32(0)
				if r.intn(4) != 0 {
					if r.intn(8) == 0 {
						v = extremeF[r.intn(len(extremeF))]
					} else {
						v = pickF(r, []float32{1, -1, 2, -2, 0.5}, -3, 3)
					}
				}
				ks = append(ks, fstr(v))
			}
			delta := float32(0)
			if r.intn(3) == 0 {
				delta = pickX(r, []float32{0.5, -0.5, 1}, -0.5, 0.5)
			}
			return fmt.Sprintf("convolution(%s,%d,%d,%d,%s)", strings.Join(ks, "/"), r.intn(2), r.intn(2), r.intn(2), fstr(delta))
		case k < 33:
			sig := pickXSafe(r, []float32{0, -1, 0.3, 1, 2.5, 1.5, 1e-30, 1e-20, 0.1666, 0.1667, 1e-3}, 0, 6)
			if level > 0 && r.intn(4) == 0 {
				// Sigmas whose kernel size overflows int: Go panics at
				// once in makeslice (a large finite sigma would instead
				// allocate and loop for hours).
				sig = []float32{f32Inf, 1e20, math.MaxFloat32}[r.intn(3)]
			}
			return fmt.Sprintf("gaussianblur(%s)", fstr(sig))
		case k < 35:
			sig := pickXSafe(r, []float32{0, 1, 0.5, 2, 1e-30}, 0, 3)
			return fmt.Sprintf("unsharpmask(%s,%s,%s)", fstr(sig), ff([]float32{0, 1, 1.5, -1, 1e20}, -1, 3), ff([]float32{0, 0.05, -0.05, 1}, -0.1, 0.2))
		case k == 35:
			ks := r.intn(14) - 2
			if b.Dx()*b.Dy() > 4096 {
				ks = r.intn(8) - 1
			}
			return fmt.Sprintf("mean(%d,%d)", ks, r.intn(2))
		case k == 36:
			ks := r.intn(12) - 2
			if b.Dx()*b.Dy() > 4096 {
				ks = r.intn(7) - 1
			}
			return fmt.Sprintf("%s(%d,%d)", []string{"median", "minimum", "maximum"}[r.intn(3)], ks, r.intn(2))
		case k == 37:
			sz := r.intn(20) - 2
			if r.intn(5) == 0 {
				sz = bigInts[r.intn(len(bigInts))]
			}
			return fmt.Sprintf("pixelate(%d)", sz)
		case k == 38:
			return "copy()"
		default:
			// Hugo's exposed filters with template-like parameters.
			switch r.intn(4) {
			case 0:
				return fmt.Sprintf("resize(%d,%d,%s)", sz(w), sz(h), res())
			case 1:
				return fmt.Sprintf("resizetofill(%d,%d,%s,%d)", sz(w), sz(h), res(), r.intn(9))
			case 2:
				return fmt.Sprintf("croptosize(%d,%d,%d)", r.intn(w+4), r.intn(h+4), r.intn(9))
			default:
				return fmt.Sprintf("rotate(%s,alpha/0,0)", fstr(float32(r.intn(721)-360)))
			}
		}
	}
}

// ---------------------------------------------------------------------------
// image specs

var rtSrcTypes = []string{
	"nrgba", "rgba", "nrgba64", "rgba64", "gray", "gray16",
	"ycbcr444", "ycbcr422", "ycbcr420", "ycbcr440", "ycbcr411", "ycbcr410",
	"nycbcra444", "nycbcra422", "nycbcra420", "nycbcra440", "nycbcra411", "nycbcra410",
	"paletted", "palettedx", "cmyk", "alpha", "alpha16", "rectimg",
}

var rtDstTypes = []string{"nrgba", "rgba", "nrgba64", "rgba64", "gray", "gray16", "paletted", "palettedx", "cmyk", "alpha", "alpha16"}

// subImageTypes are the types whose SubImage returns a draw.Image or image
// (everything but rectimg and the synthetic sources).
func canSub(typ string) bool {
	return typ != "rectimg" && typ != "grayseq" && typ != "palettedempty"
}

func isYCbCr(typ string) bool {
	return strings.HasPrefix(typ, "ycbcr") || strings.HasPrefix(typ, "nycbcra")
}

// genSpec returns an image spec over rect with random decorations: a
// sub-image of a larger parent (1/4) and a type-hiding wrapper (1/6).
func genSpec(r *rng, typ string, rect image.Rectangle, blank bool, allowNegParent bool) imgSpec {
	s := imgSpec{typ: typ, rect: rect, seed: r.next(), vmode: r.intn(2), amode: r.intn(3), valid: r.intn(8) != 0, blank: blank}
	if blank {
		s.valid = true
	}
	if canSub(typ) && r.intn(4) == 0 {
		// Margins that do not overflow (far origins, genFar).
		sub := func(a, d int) int {
			if a < math.MinInt64+d {
				return a
			}
			return a - d
		}
		add := func(a, d int) int {
			if a > math.MaxInt64-d {
				return a
			}
			return a + d
		}
		pr := image.Rect(sub(rect.Min.X, r.intn(6)), sub(rect.Min.Y, r.intn(6)), add(rect.Max.X, r.intn(6)), add(rect.Max.Y, r.intn(6)))
		if isYCbCr(typ) && !allowNegParent {
			pr.Min.X = max(pr.Min.X, 0)
			pr.Min.Y = max(pr.Min.Y, 0)
		}
		s.typ = typ + "@" + slashRect(pr)
		if r.intn(8) == 0 {
			// A sub-image rect partly or wholly outside the parent.
			s.rect = rect.Add(image.Pt(r.intn(21)-10, r.intn(21)-10))
		}
	}
	// No wrapper around an empty palette: Go's Paletted.At returns a nil
	// color.Color there and gift's pixelFromColor(nil) panics; the Rust
	// colour model cannot express nil (known divergence, PORTING.md).
	if r.intn(6) == 0 && typ != "palettedempty" {
		s.typ = "w." + s.typ
	}
	return s
}

// lutUnsafeX: may a source give channel values > 1 (LUT index panic)?
func lutUnsafeX(s imgSpec) bool {
	if s.valid {
		return false
	}
	t := strings.TrimPrefix(s.typ, "w.")
	if at := strings.IndexByte(t, '@'); at >= 0 {
		t = t[:at]
	}
	return t == "rgba" || t == "rgba64" || strings.HasPrefix(t, "paletted")
}

func genOrigin(r *rng, typ string, level int) (int, int) {
	if isYCbCr(typ) && level == 0 {
		return r.intn(9), r.intn(9)
	}
	switch r.intn(12) {
	case 0:
		return 1<<40 + r.intn(7), -(1 << 41) - r.intn(7)
	case 1:
		return r.intn(2001) - 1000, r.intn(2001) - 1000
	default:
		return r.intn(15) - 7, r.intn(15) - 7
	}
}

func genSizeX(r *rng) (int, int) {
	switch k := r.intn(100); {
	case k < 3:
		return r.intn(2), r.intn(3)
	case k < 8:
		return 1, 1 + r.intn(300)
	case k < 13:
		return 1 + r.intn(300), 1
	case k < 70:
		return 1 + r.intn(16), 1 + r.intn(16)
	case k < 92:
		return 1 + r.intn(64), 1 + r.intn(64)
	default:
		return 1 + r.intn(200), 1 + r.intn(200)
	}
}

// areaOK bounds the memory and time of a case. (Dimensions can be negative
// on amd64, where int(NaN) is MinInt64, e.g. Rotate by ±Inf.)
func areaOK(b image.Rectangle) bool {
	return b.Dx() >= 0 && b.Dy() >= 0 && b.Dx() <= 4000 && b.Dy() <= 4000 && b.Dx()*b.Dy() <= 250000
}

func genDst(r *rng, c *synthCase, b image.Rectangle, level int) {
	dtyp := rtDstTypes[r.intn(len(rtDstTypes))]
	if r.intn(3) == 0 {
		c.op = "drawat"
		dw, dh := 1+r.intn(40), 1+r.intn(40)
		dx, dy := genOrigin(r, dtyp, level)
		dr := image.Rect(dx, dy, dx+dw, dy+dh)
		c.dst = genSpec(r, dtyp, dr, false, true)
		c.dst.valid = true
		var px, py int
		switch r.intn(6) {
		case 0:
			px, py = dx, dy
		case 1:
			px, py = dx+r.intn(dw), dy+r.intn(dh)
		case 2:
			px, py = r.intn(2001)-1000, r.intn(2001)-1000
		default:
			px, py = dx-8-b.Dx()+r.intn(dw+16+b.Dx()), dy-8-b.Dy()+r.intn(dh+16+b.Dy())
		}
		c.pt = fmt.Sprintf("%d,%d,%d", px, py, r.intn(2))
	} else {
		c.op = "draw"
		var dr image.Rectangle
		if r.intn(7) == 0 {
			dx, dy := genOrigin(r, dtyp, level)
			dr = image.Rect(dx, dy, dx+r.intn(30), dy+r.intn(30))
		} else {
			ox, oy := 0, 0
			if r.intn(2) == 0 {
				ox, oy = r.intn(7)-3, r.intn(7)-3
			}
			dr = b.Add(image.Pt(ox, oy))
		}
		c.dst = genSpec(r, dtyp, dr, true, true)
	}
}

func genSrc(r *rng, types []string, level int) imgSpec {
	typ := types[r.intn(len(types))]
	w, h := genSizeX(r)
	ox, oy := genOrigin(r, typ, level)
	return genSpec(r, typ, image.Rect(ox, oy, ox+w, oy+h), false, level > 0)
}

func genChain(r *rng, c *synthCase, nf int, level int) image.Rectangle {
	var fs []string
	b := genImageX(c.src).Bounds()
	for i := 0; i < nf; i++ {
		for tries := 0; ; tries++ {
			f := genFilterX(r, b, i > 0 || !lutUnsafeX(c.src) || level > 0, level)
			nb := parseFilterX(f).Bounds(b)
			if !areaOK(nb) && tries < 50 {
				continue
			}
			if !areaOK(nb) {
				f = "copy()"
				nb = parseFilterX(f).Bounds(b)
			}
			fs = append(fs, f)
			b = nb
			break
		}
	}
	c.filters = strings.Join(fs, "|")
	return b
}

func genSynth2(r *rng) synthCase {
	var c synthCase
	c.src = genSrc(r, rtSrcTypes, 0)
	nf := []int{0, 1, 1, 1, 2, 2, 3, 3, 4, 5, 6}[r.intn(11)]
	b := genChain(r, &c, nf, 0)
	genDst(r, &c, b, 0)
	return c
}

// genParams: one or two filters with extreme parameters over small images.
func genParams(r *rng) synthCase {
	var c synthCase
	typ := rtSrcTypes[r.intn(len(rtSrcTypes))]
	w, h := 1+r.intn(12), 1+r.intn(12)
	ox, oy := genOrigin(r, typ, 0)
	c.src = genSpec(r, typ, image.Rect(ox, oy, ox+w, oy+h), false, false)
	b := genChain(r, &c, 1+r.intn(2), 0)
	genDst(r, &c, b, 0)
	return c
}

// genResample: Resize / ResizeToFit / ResizeToFill / CropToSize with
// extreme ratios and every kernel and anchor.
func genResample(r *rng) synthCase {
	var c synthCase
	typ := rtSrcTypes[r.intn(len(rtSrcTypes))]
	res := resamplingNames[r.intn(len(resamplingNames))]
	var sw, sh, dw, dh int
	pick := func() int { return primes[r.intn(len(primes))] }
	switch r.intn(9) {
	case 0: // 1→N horizontally
		sw, sh = 1+r.intn(3), 1+r.intn(4)
		dw, dh = 1+r.intn(3000), sh
	case 1: // N→1 horizontally
		sw, sh = 1+r.intn(20000), 1+r.intn(2)
		dw, dh = 1+r.intn(3), sh
	case 2: // 1→N vertically
		sw, sh = 1+r.intn(4), 1+r.intn(3)
		dw, dh = sw, 1+r.intn(3000)
	case 3: // N→1 vertically
		sw, sh = 1+r.intn(2), 1+r.intn(20000)
		dw, dh = sw, 1+r.intn(3)
	case 4: // primes
		sw, sh = pick()%300+1, pick()%300+1
		dw, dh = pick()%400+1, pick()%400+1
	case 5: // strips to strips
		if r.intn(2) == 0 {
			sw, sh, dw, dh = 1, 1+r.intn(500), 1+r.intn(500), 1
		} else {
			sw, sh, dw, dh = 1+r.intn(500), 1, 1, 1+r.intn(500)
		}
	case 6: // one side 0 (keep aspect)
		sw, sh = 1+r.intn(200), 1+r.intn(200)
		if r.intn(2) == 0 {
			dw, dh = 0, 1+r.intn(400)
		} else {
			dw, dh = 1+r.intn(400), 0
		}
	default:
		sw, sh = 1+r.intn(120), 1+r.intn(120)
		dw, dh = r.intn(250)-2, r.intn(250)-2
	}
	for sw*sh > 60000 {
		sh = (sh + 1) / 2
	}
	ox, oy := genOrigin(r, typ, 0)
	c.src = genSpec(r, typ, image.Rect(ox, oy, ox+sw, oy+sh), false, false)
	var f string
	switch r.intn(6) {
	case 0:
		f = fmt.Sprintf("resizetofit(%d,%d,%s)", dw, dh, res)
	case 1:
		if fillTmpArea(sw, sh, dw, dh) > 1<<20 {
			f = fmt.Sprintf("resizetofit(%d,%d,%s)", dw, dh, res)
			break
		}
		f = fmt.Sprintf("resizetofill(%d,%d,%s,%d)", dw, dh, res, r.intn(12)-1)
	case 2:
		f = fmt.Sprintf("resize(%d,%d,%s)|croptosize(%d,%d,%d)", dw, dh, res, r.intn(dw+3)-1, r.intn(dh+3)-1, r.intn(12)-1)
	default:
		f = fmt.Sprintf("resize(%d,%d,%s)", dw, dh, res)
	}
	sb := genImageX(c.src).Bounds()
	b := sb
	for _, p := range strings.Split(f, "|") {
		b = parseFilterX(p).Bounds(b)
	}
	if !areaOK(b) {
		f = fmt.Sprintf("resize(%d,%d,%s)", min(dw, 300), min(dh, 300), res)
		b = parseFilterX(f).Bounds(sb)
	}
	c.filters = f
	if r.intn(3) == 0 {
		b = genChainTail(r, &c, b)
	}
	genDst(r, &c, b, 0)
	return c
}

// fillTmpArea is the area of the temporary image ResizeToFill(w, h) draws
// before cropping (resize.go:354-377); it is not bounded by the filter's
// Bounds.
func fillTmpArea(srcw, srch, w, h int) int {
	if w <= 0 || h <= 0 || srcw <= 0 || srch <= 0 {
		return 0
	}
	wratio := float64(srcw) / float64(w)
	hratio := float64(srch) / float64(h)
	if wratio < hratio {
		return w * max(int(float64(srch)/wratio+0.5), h)
	}
	return h * max(int(float64(srcw)/hratio+0.5), w)
}

func genChainTail(r *rng, c *synthCase, b image.Rectangle) image.Rectangle {
	f := genFilterX(r, b, true, 0)
	nb := parseFilterX(f).Bounds(b)
	if areaOK(nb) {
		c.filters += "|" + f
		return nb
	}
	return b
}

// genDrawAt: DrawAt with both operators into every destination kind.
func genDrawAt(r *rng) synthCase {
	var c synthCase
	c.src = genSrc(r, rtSrcTypes, 0)
	nf := []int{0, 0, 0, 1, 1, 2}[r.intn(6)]
	b := genChain(r, &c, nf, 0)
	for {
		genDst(r, &c, b, 0)
		if c.op == "drawat" {
			break
		}
	}
	return c
}

// genHugo: the sources Hugo's decoders return, Hugo's filters and the
// doFilter destination rule, plus the overlay/padding DrawAt(Over) shape.
func genHugo(r *rng) synthCase {
	var c synthCase
	typ := []string{"ycbcr444", "ycbcr420", "ycbcr422", "nrgba", "rgba", "paletted", "gray", "gray16", "nrgba64", "rgba64", "cmyk"}[r.intn(11)]
	w, h := 1+r.intn(120), 1+r.intn(120)
	c.src = imgSpec{typ: typ, rect: image.Rect(0, 0, w, h), seed: r.next(), vmode: r.intn(2), amode: r.intn(3), valid: true}
	b := c.src.rect
	var fs []string
	nf := 1 + r.intn(3)
	for i := 0; i < nf; i++ {
		var f string
		res := resamplingNames[r.intn(len(resamplingNames))]
		tw, th := 1+r.intn(200), 1+r.intn(200)
		switch r.intn(20) {
		case 0:
			f = fmt.Sprintf("resize(%d,0,%s)", tw, res)
		case 1:
			f = fmt.Sprintf("resize(0,%d,%s)", th, res)
		case 2:
			f = fmt.Sprintf("resize(%d,%d,%s)", tw, th, res)
		case 3:
			f = fmt.Sprintf("resizetofit(%d,%d,%s)", tw, th, res)
		case 4:
			f = fmt.Sprintf("resizetofill(%d,%d,%s,%d)", tw, th, res, r.intn(9))
		case 5:
			f = fmt.Sprintf("croptosize(%d,%d,%d)", tw, th, r.intn(9))
		case 6:
			f = fmt.Sprintf("rotate(%s,alpha/0,0)", fstr(float32(r.intn(721)-360)))
		case 7:
			f = fmt.Sprintf("brightness(%s)", fstr(rf(r, -100, 100)))
		case 8:
			f = fmt.Sprintf("colorbalance(%s,%s,%s)", fstr(rf(r, -100, 500)), fstr(rf(r, -100, 500)), fstr(rf(r, -100, 500)))
		case 9:
			f = fmt.Sprintf("colorize(%s,%s,%s)", fstr(float32(r.intn(361))), fstr(float32(r.intn(101))), fstr(float32(r.intn(101))))
		case 10:
			f = fmt.Sprintf("contrast(%s)", fstr(float32(r.intn(201)-100)))
		case 11:
			f = fmt.Sprintf("gamma(%s)", fstr(rf(r, 0, 3)))
		case 12:
			f = fmt.Sprintf("gaussianblur(%s)", fstr(rf(r, 0, 4)))
		case 13:
			f = []string{"grayscale()", "invert()"}[r.intn(2)]
		case 14:
			f = fmt.Sprintf("hue(%s)", fstr(float32(r.intn(361)-180)))
		case 15:
			f = fmt.Sprintf("pixelate(%d)", r.intn(12))
		case 16:
			f = fmt.Sprintf("saturation(%s)", fstr(float32(r.intn(601)-100)))
		case 17:
			f = fmt.Sprintf("sepia(%s)", fstr(float32(r.intn(101))))
		case 18:
			f = fmt.Sprintf("sigmoid(%s,%s)", fstr(rf(r, 0, 1)), fstr(rf(r, -10, 10)))
		default:
			f = fmt.Sprintf("unsharpmask(%s,%s,%s)", fstr(rf(r, 0, 3)), fstr(rf(r, 0, 2)), fstr(rf(r, 0, 0.1)))
		}
		nb := parseFilterX(f).Bounds(b)
		if !areaOK(nb) {
			continue
		}
		fs = append(fs, f)
		b = nb
	}
	if r.intn(3) == 0 {
		// images.Overlay: an overlay of an arbitrary NRGBA/RGBA/Paletted image.
		otyp := []string{"nrgba", "rgba", "paletted", "nrgba64"}[r.intn(4)]
		ow, oh := 1+r.intn(60), 1+r.intn(60)
		fs = append(fs, fmt.Sprintf("overlayg(%s,0,0,%d,%d,%d,%d,%d,%d)", otyp, ow, oh, r.next(), r.intn(2), r.intn(81)-40, r.intn(81)-40))
	}
	c.filters = strings.Join(fs, "|")
	dtyp := "nrgba"
	switch typ {
	case "rgba":
		dtyp = "rgba"
	case "gray":
		dtyp = "gray"
	}
	c.op = "draw"
	c.dst = imgSpec{typ: dtyp, rect: b, valid: true, blank: true}
	return c
}

// genPanic: cases where Go may panic (out-of-range palette indexes, empty
// palettes, negative YCbCr origins, LUTs over invalid premultiplied input,
// infinite blur sigmas) next to cases that do not.
func genPanic(r *rng) synthCase {
	var c synthCase
	types := append([]string{"palettedoor", "palettedoor", "palettedempty", "ycbcr422", "ycbcr420", "ycbcr411", "ycbcr410", "ycbcr440", "nycbcra420", "nycbcra411"}, rtSrcTypes...)
	c.src = genSrc(r, types, 1)
	nf := []int{0, 1, 1, 2, 3}[r.intn(5)]
	b := genChain(r, &c, nf, 1)
	dtyp := []string{"nrgba", "paletted", "palettedoor", "palettedempty", "nrgba64", "gray"}[r.intn(6)]
	if r.intn(2) == 0 {
		c.op = "drawat"
		dr := image.Rect(r.intn(9)-4, r.intn(9)-4, r.intn(9)+5, r.intn(9)+5)
		c.dst = genSpec(r, dtyp, dr, false, true)
		c.pt = fmt.Sprintf("%d,%d,%d", r.intn(13)-6, r.intn(13)-6, r.intn(2))
	} else {
		c.op = "draw"
		c.dst = genSpec(r, dtyp, b, true, true)
	}
	return c
}

// farCoord returns a coordinate near the ends of the int range (images with
// such origins are valid; gift's coordinate arithmetic wraps in Go).
func farCoord(r *rng, span int) int {
	switch r.intn(7) {
	case 0:
		return math.MaxInt64 - span - r.intn(20)
	case 1:
		return math.MinInt64 + r.intn(20)
	case 2:
		return 1<<62 - r.intn(20)
	case 3:
		return -(1 << 62) + r.intn(20)
	case 4:
		return math.MaxInt64/2 + r.intn(20)
	case 5:
		return math.MinInt64/2 - r.intn(20)
	default:
		return r.intn(21) - 10
	}
}

// genFar: sources, destinations and DrawAt points near the ends of the int
// range.
func genFar(r *rng) synthCase {
	var c synthCase
	typ := rtSrcTypes[r.intn(len(rtSrcTypes))]
	w, h := 1+r.intn(12), 1+r.intn(12)
	ox, oy := farCoord(r, w+40), farCoord(r, h+40)
	if isYCbCr(typ) {
		ox, oy = max(ox, 0), max(oy, 0)
	}
	c.src = genSpec(r, typ, image.Rect(ox, oy, ox+w, oy+h), false, false)
	nf := []int{0, 0, 1, 1, 2}[r.intn(5)]
	b := genChain(r, &c, nf, 0)
	dtyp := rtDstTypes[r.intn(len(rtDstTypes))]
	if r.intn(2) == 0 {
		c.op = "drawat"
		dw, dh := 1+r.intn(20), 1+r.intn(20)
		dx, dy := farCoord(r, dw+40), farCoord(r, dh+40)
		c.dst = genSpec(r, dtyp, image.Rect(dx, dy, dx+dw, dy+dh), false, true)
		c.dst.valid = true
		px, py := dx+r.intn(dw+10)-5, dy+r.intn(dh+10)-5
		if r.intn(3) == 0 {
			px, py = farCoord(r, 40), farCoord(r, 40)
		}
		c.pt = fmt.Sprintf("%d,%d,%d", px, py, r.intn(2))
	} else {
		c.op = "draw"
		dr := b
		if r.intn(2) == 0 {
			dx, dy := farCoord(r, b.Dx()+40), farCoord(r, b.Dy()+40)
			dr = image.Rect(dx, dy, dx+b.Dx(), dy+b.Dy())
		}
		c.dst = genSpec(r, dtyp, dr, true, true)
	}
	return c
}

var rtModes = map[string]func(*rng) synthCase{
	"far":      genFar,
	"synth2":   genSynth2,
	"params":   genParams,
	"resample": genResample,
	"drawat":   genDrawAt,
	"hugo":     genHugo,
	"panic":    genPanic,
}

func rtMode(name string) func(*rng) synthCase {
	g, ok := rtModes[name]
	if !ok {
		panic("unknown redteam mode " + name)
	}
	return g
}

// redteam prints n cases of a mode with their digests.
func redteam(mode string, n int, seed uint64) {
	gen := rtMode(mode)
	r := &rng{s: seed}
	for i := 0; i < n; i++ {
		// Run the printed line (as rtrun and the Rust tests do), so that a
		// spec rect that is not canonical cannot make the two differ.
		line := gen(r).line(i)
		_, _ = fmt.Fprintf(out, "%s\t%s\n", line, digestX(runSynthX(parseSynthLine(line))))
	}
}

// redteamGen prints n cases of a mode without running them.
func redteamGen(mode string, n int, seed uint64) {
	gen := rtMode(mode)
	r := &rng{s: seed}
	for i := 0; i < n; i++ {
		_, _ = fmt.Fprintf(out, "%s\n", gen(r).line(i))
	}
}

// redteamRun runs the case lines of a file from line index start on,
// printing each result line immediately (a crash loses no finished line).
//
// A panic inside gift's parallelize goroutines first runs their deferred
// wg.Done, so the main goroutine could print a (partial) digest before the
// runtime exits. The driver therefore runs this with GOMAXPROCS=1, where the
// panicking goroutine is not descheduled before the exit, and the yield
// below lets any still-runnable goroutine finish first.
func redteamRun(path string, start int) {
	f, err := os.Open(path)
	if err != nil {
		panic(err)
	}
	defer func() { _ = f.Close() }()
	sc := bufio.NewScanner(f)
	sc.Buffer(make([]byte, 1<<20), 1<<24)
	for i := 0; sc.Scan(); i++ {
		if i < start {
			continue
		}
		// A fixture line carries its old result as a 7th field: drop it, so
		// that running a fixture through here regenerates it.
		f := strings.Split(sc.Text(), "\t")
		if len(f) > 6 {
			f = f[:6]
		}
		line := strings.Join(f, "\t")
		var d string
		if f[1] == "bounds" {
			d = boundsResult(f[2], f[5])
		} else {
			d = digestX(runSynthX(parseSynthLine(line)))
		}
		for k := 0; k < 4; k++ {
			runtime.Gosched()
		}
		_, _ = fmt.Fprintf(out, "%s\t%s\n", line, d)
		_ = out.Flush()
	}
}

// rtOne re-runs one red-team line and prints every pixel.
func rtOne(line string) {
	c := parseSynthLine(line)
	dst := runSynthX(c)
	_, _ = fmt.Fprintf(out, "digest %s\n", digestX(dst))
	b := dst.Bounds()
	for y := b.Min.Y; y < b.Max.Y; y++ {
		for x := b.Min.X; x < b.Max.X; x++ {
			_, _ = fmt.Fprintf(out, "(%d,%d) %v\n", x, y, dst.At(x, y))
		}
	}
}

// ---------------------------------------------------------------------------
// Bounds of chains with extreme parameters (no drawing).

func genBoundsInt(r *rng) int {
	switch r.intn(6) {
	case 0:
		return bigInts[r.intn(len(bigInts))]
	case 1:
		return r.intn(11) - 5
	case 2:
		return r.intn(1<<31) - 1<<30
	default:
		return r.intn(3000) - 100
	}
}

func genBoundsFilter(r *rng) string {
	I := func() int { return genBoundsInt(r) }
	res := resamplingNames[r.intn(len(resamplingNames))]
	switch r.intn(14) {
	case 0, 1:
		return fmt.Sprintf("resize(%d,%d,%s)", I(), I(), res)
	case 2:
		return fmt.Sprintf("resize(%d,0,%s)", I(), res)
	case 3:
		return fmt.Sprintf("resize(0,%d,%s)", I(), res)
	case 4:
		return fmt.Sprintf("resizetofit(%d,%d,%s)", I(), I(), res)
	case 5:
		return fmt.Sprintf("resizetofill(%d,%d,%s,%d)", I(), I(), res, r.intn(12)-1)
	case 6:
		return fmt.Sprintf("crop(%d,%d,%d,%d)", I(), I(), I(), I())
	case 7, 8:
		return fmt.Sprintf("croptosize(%d,%d,%d)", I(), I(), r.intn(12)-1)
	case 9, 10:
		ang := pickX(r, []float32{0, 90, 45, 1e-3, 1e9, 3e38, -1e30, 180.00002}, -100000, 100000)
		return fmt.Sprintf("rotate(%s,alpha/0,%d)", fstr(ang), r.intn(3))
	case 11:
		return []string{"rotate90()", "rotate180()", "transpose()", "transverse()", "fliph()", "invert()"}[r.intn(6)]
	case 12:
		return fmt.Sprintf("pixelate(%d)", I())
	default:
		return fmt.Sprintf("gaussianblur(%s)", fstr(pickX(r, []float32{1, 1e30}, 0, 10)))
	}
}

// boundsResult is the result field of a bounds line: GIFT.Bounds of the
// chain over the rect "x0,y0,x1,y1".
func boundsResult(rect, filters string) string {
	r := strings.Split(rect, ",")
	b := gift.New(parseFiltersX(filters)...).Bounds(image.Rect(pi(r[0]), pi(r[1]), pi(r[2]), pi(r[3])))
	return fmt.Sprintf("%d,%d,%d,%d", b.Min.X, b.Min.Y, b.Max.X, b.Max.Y)
}

func rtBounds(n int, seed uint64) {
	r := &rng{s: seed}
	for i := 0; i < n; i++ {
		var rect image.Rectangle
		switch r.intn(4) {
		case 0:
			rect = image.Rect(genBoundsInt(r), genBoundsInt(r), genBoundsInt(r), genBoundsInt(r))
		default:
			x, y := r.intn(41)-20, r.intn(41)-20
			rect = image.Rect(x, y, x+r.intn(5000), y+r.intn(5000))
		}
		nf := 1 + r.intn(4)
		var fs []string
		for k := 0; k < nf; k++ {
			fs = append(fs, genBoundsFilter(r))
		}
		f := strings.Join(fs, "|")
		b := gift.New(parseFiltersX(f)...).Bounds(rect)
		_, _ = fmt.Fprintf(out, "%d\tbounds\t%d,%d,%d,%d\t-\t-\t%s\t%d,%d,%d,%d\n", i, rect.Min.X, rect.Min.Y, rect.Max.X, rect.Max.Y, f,
			b.Min.X, b.Min.Y, b.Max.X, b.Max.Y)
	}
}

// rtWitness prints the witness() cases: every witnessTab pixel through the
// setter of every destination kind (the RGBA and RGBA64 lines kill the
// unfused f32u8/f32u16 mutants), plain and as DrawAt(Over).
func rtWitness() {
	i := 0
	src := imgSpec{typ: "grayseq", rect: image.Rect(0, 0, len(witnessTab), 1)}
	for _, typ := range []string{"rgba", "rgba64", "nrgba", "nrgba64", "gray", "gray16", "cmyk", "alpha", "alpha16", "paletted", "w.rgba", "w.rgba64"} {
		for _, op := range []string{"draw", "drawat"} {
			c := synthCase{op: op, src: src, filters: "witness()"}
			c.dst = imgSpec{typ: typ, rect: src.rect, seed: uint64(i + 1), vmode: 1, amode: 2, valid: true, blank: op == "draw"}
			if op == "drawat" {
				c.pt = "0,0,1"
			}
			line := c.line(i)
			_, _ = fmt.Fprintf(out, "%s\t%s\n", line, digestX(runSynthX(parseSynthLine(line))))
			i++
		}
	}
}

// mathIn prints math-vector rows (mathVectors' format) for the x, y bit
// patterns of a file (two hex fields per line): the FMA witness inputs of
// Go's Log and Sin (crates/gift/tests/fixtures/mathwitness.tsv).
func mathIn(path string) {
	data, err := os.ReadFile(path)
	if err != nil {
		panic(err)
	}
	f64 := func(v float64) string { return strconv.FormatUint(math.Float64bits(v), 16) }
	for _, line := range strings.Split(string(data), "\n") {
		fs := strings.Fields(line)
		if len(fs) < 2 || strings.HasPrefix(fs[0], "#") {
			continue
		}
		xb, err1 := strconv.ParseUint(fs[0], 16, 64)
		yb, err2 := strconv.ParseUint(fs[1], 16, 64)
		if err1 != nil || err2 != nil {
			panic("bad line " + line)
		}
		x, y := math.Float64frombits(xb), math.Float64frombits(yb)
		s, c := math.Sincos(x)
		_, _ = fmt.Fprintf(out, "%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n", f64(x), f64(y),
			f64(math.Exp(x)), f64(math.Log(x)), f64(math.Pow(x, y)), f64(math.Sin(x)), f64(math.Cos(x)), f64(s), f64(c))
	}
}

func rtMain(args []string) {
	switch args[0] {
	case "mathin":
		mathIn(args[1])
	case "rtwitness":
		rtWitness()
	case "rt":
		n, _ := strconv.Atoi(args[2])
		seed, _ := strconv.ParseUint(args[3], 10, 64)
		redteam(args[1], n, seed)
	case "rtgen":
		n, _ := strconv.Atoi(args[2])
		seed, _ := strconv.ParseUint(args[3], 10, 64)
		redteamGen(args[1], n, seed)
	case "rtrun":
		start, _ := strconv.Atoi(args[2])
		redteamRun(args[1], start)
	case "rtone":
		rtOne(args[1])
	case "rtbounds":
		n, _ := strconv.Atoi(args[1])
		seed, _ := strconv.ParseUint(args[2], 10, 64)
		rtBounds(n, seed)
	}
}
