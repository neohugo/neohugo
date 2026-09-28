// Command gift is the Go oracle for the Rust crate crates/gift, a port of
// github.com/disintegration/gift v1.2.1 plus the extra resampling filters of
// neohugo resources/images/resampling.go.
//
// It prints digests of gift results that the Rust differential tests compare
// against. Synthetic inputs come from a splitmix64 generator that the Rust
// tests re-implement call for call (gen.go); real images are dumped as raw
// decoded pixel buffers (dump.go) so the Rust tests never need a codec.
//
// Usage:
//
//	gift synth <n> <seed>             # random filter chains over generated images
//	gift one <line>                   # re-run one synth line, print all pixels
//	gift math <n>                     # math.Exp/Log/Pow/Sin/Cos/Sincos bit vectors
//	gift kernels <n>                  # every resampling kernel over float32 inputs
//	gift dump <in> <out.gz> [crop]    # decode an image file and write a raw dump
//	gift realfix <siteRoot> <giftTestdata> <outDir>  # checked-in real-image fixtures
//	gift realops <dumpDir>            # real.tsv again, from the checked-in dumps
//	gift gotestdata <giftTestdata> <outDir>  # dumps of gift's testdata/*.png (TestGolden)
//	gift site <siteRoot> <outDir>     # dump all site images + run the Hugo ops
//	gift setter                       # synth lines: every setter over the setter table, big Over cases
//	gift mathdigest <scale>           # chunked digests of dense math sweeps
//	gift kerneldigest <stride>        # chunked digests of every kernel over float32 sweeps
//	gift weights                      # chunked digests of 1-row resizes over many size pairs
//	gift rotbounds                    # chunked digest of Rotate bounds
//	gift rt <mode> <n> <seed>         # red-team cases (redteam.go) with digests
//	gift rtgen <mode> <n> <seed>      # red-team cases without running them
//	gift rtrun <file> <start>         # run case lines (crash-tolerant driver)
//	gift rtone <line>                 # re-run one red-team line, print all pixels
//	gift rtbounds <n> <seed>          # Bounds of chains with extreme parameters
//	gift rtwitness                    # the witness() setter-rounding cases
//	gift mathin <file>                # math rows for listed x, y bit patterns
package main

import (
	"bufio"
	"fmt"
	"image"
	"image/draw"
	"math"
	"os"
	"strconv"
	"strings"

	"github.com/disintegration/gift"
	"github.com/neohugo/neohugo/resources/images"
)

var out = bufio.NewWriterSize(os.Stdout, 1<<20)

func main() {
	defer func() { _ = out.Flush() }()
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "usage: gift <cmd> ...")
		os.Exit(2)
	}
	switch os.Args[1] {
	case "synth":
		n, _ := strconv.Atoi(os.Args[2])
		seed, _ := strconv.ParseUint(os.Args[3], 10, 64)
		synth(n, seed)
	case "one":
		one(os.Args[2])
	case "math":
		n, _ := strconv.Atoi(os.Args[2])
		mathVectors(n)
	case "kernels":
		n, _ := strconv.Atoi(os.Args[2])
		kernels(n)
	case "dump":
		crop := ""
		if len(os.Args) > 4 {
			crop = os.Args[4]
		}
		dumpFile(os.Args[2], os.Args[3], crop)
	case "realfix":
		realFixtures(os.Args[2], os.Args[3], os.Args[4])
	case "realops":
		realOps(os.Args[2])
	case "gotestdata":
		goTestdata(os.Args[2], os.Args[3])
	case "site":
		site(os.Args[2], os.Args[3])
	case "setter":
		setterCases()
	case "mathdigest":
		n, _ := strconv.Atoi(os.Args[2])
		mathDigest(n)
	case "kerneldigest":
		n, _ := strconv.Atoi(os.Args[2])
		kernelDigest(n)
	case "weights":
		weightsDigest()
	case "rotbounds":
		rotBoundsDigest()
	case "rt", "rtgen", "rtrun", "rtone", "rtbounds", "rtwitness", "mathin":
		rtMain(os.Args[1:])
	default:
		fmt.Fprintln(os.Stderr, "unknown command", os.Args[1])
		os.Exit(2)
	}
}

// resamplingNames lists every resampling filter: gift's five and neohugo's
// eleven extra ones (resources/images/resampling.go), by their neohugo names.
var resamplingNames = []string{
	"nearestneighbor", "box", "linear", "cubic", "lanczos",
	"hermite", "mitchellnetravali", "catmullrom", "bspline", "gaussian",
	"hann", "hamming", "blackman", "bartlett", "welch", "cosine",
}

var hugoResampling = map[string]gift.Resampling{}

func resampling(name string) gift.Resampling {
	switch name {
	case "nearestneighbor":
		return gift.NearestNeighborResampling
	case "box":
		return gift.BoxResampling
	case "linear":
		return gift.LinearResampling
	case "cubic":
		return gift.CubicResampling
	case "lanczos":
		return gift.LanczosResampling
	}
	if r, ok := hugoResampling[name]; ok {
		return r
	}
	ns, err := images.DecodeConfig(map[string]any{"resampleFilter": name})
	if err != nil {
		panic(err)
	}
	r := ns.Config.ResampleFilter
	if r == nil {
		panic("no resampling " + name)
	}
	hugoResampling[name] = r
	return r
}

// ---------------------------------------------------------------------------
// synth

var settableTypes = []string{"nrgba", "nrgba", "nrgba", "rgba", "rgba", "nrgba64", "rgba64", "gray", "gray16", "paletted", "cmyk", "alpha", "alpha16"}

var srcTypes = []string{
	"nrgba", "nrgba", "nrgba", "rgba", "rgba", "nrgba64", "rgba64", "gray", "gray16",
	"ycbcr444", "ycbcr422", "ycbcr420", "ycbcr440", "ycbcr411", "ycbcr410",
	"paletted", "paletted", "cmyk", "alpha", "alpha16", "nycbcra444", "nycbcra420",
}

func rf(r *rng, lo, hi float32) float32 {
	// A float32 in [lo, hi] with a non-trivial fraction.
	u := float32(r.next()%1000003) / 1000003
	return lo + (hi-lo)*u
}

func pickF(r *rng, specials []float32, lo, hi float32) float32 {
	if r.intn(3) == 0 {
		return specials[r.intn(len(specials))]
	}
	return rf(r, lo, hi)
}

func genSize(r *rng) (int, int) {
	switch k := r.intn(100); {
	case k < 2:
		return r.intn(2), r.intn(3)
	case k < 70:
		return 1 + r.intn(16), 1 + r.intn(16)
	case k < 92:
		return 1 + r.intn(64), 1 + r.intn(64)
	default:
		return 1 + r.intn(160), 1 + r.intn(160)
	}
}

func genFilter(r *rng, b image.Rectangle, allowLut bool) string {
	w, h := b.Dx(), b.Dy()
	sz := func(n int) int {
		switch r.intn(6) {
		case 0:
			return 0
		case 1:
			return 1 + r.intn(3)
		default:
			return 1 + r.intn(2*n+4)
		}
	}
	res := func() string { return resamplingNames[r.intn(len(resamplingNames))] }
	for {
		switch k := r.intn(40); {
		case k < 10:
			return fmt.Sprintf("resize(%d,%d,%s)", sz(w), sz(h), res())
		case k < 12:
			return fmt.Sprintf("resizetofit(%d,%d,%s)", sz(w), sz(h), res())
		case k < 14:
			return fmt.Sprintf("resizetofill(%d,%d,%s,%d)", sz(w), sz(h), res(), r.intn(10))
		case k == 14:
			x0 := b.Min.X - 3 + r.intn(w+6)
			y0 := b.Min.Y - 3 + r.intn(h+6)
			return fmt.Sprintf("crop(%d,%d,%d,%d)", x0, y0, x0-2+r.intn(w+6), y0-2+r.intn(h+6))
		case k == 15:
			return fmt.Sprintf("croptosize(%d,%d,%d)", r.intn(w+4)-1, r.intn(h+4)-1, r.intn(10))
		case k == 16:
			return []string{"rotate90()", "rotate180()", "rotate270()", "fliph()", "flipv()", "transpose()", "transverse()"}[r.intn(7)]
		case k < 20:
			ang := pickF(r, []float32{0, 90, 180, 270, -90, 30, 45, -45, 360, 1e-3}, -400, 400)
			c := genColor(r, 1, 2, true)
			return fmt.Sprintf("rotate(%s,%s,%d)", fstr(ang), colorStr(c), r.intn(4))
		case k == 20:
			return []string{"invert()", "grayscale()", "sobel()"}[r.intn(3)]
		case k == 21:
			if !allowLut {
				continue
			}
			switch r.intn(4) {
			case 0:
				return "srgbtolinear()"
			case 1:
				return "lineartosrgb()"
			case 2:
				return fmt.Sprintf("gamma(%s)", fstr(pickF(r, []float32{0, 0.5, 1, 1.5, 2.2, -1, 1e-6}, 0, 5)))
			default:
				return fmt.Sprintf("sigmoid(%s,%s)", fstr(pickF(r, []float32{0, 0.5, 1, -0.5, 1.5}, -0.5, 1.5)), fstr(pickF(r, []float32{0, 7, -7, 1e-3, -1e-3}, -15, 15)))
			}
		case k == 22:
			return fmt.Sprintf("contrast(%s)", fstr(pickF(r, []float32{0, -100, 100, 30, -30, 150, -150, 99.99}, -150, 150)))
		case k == 23:
			return fmt.Sprintf("brightness(%s)", fstr(pickF(r, []float32{0, -100, 100, 30, -30, 150, -150}, -150, 150)))
		case k == 24:
			return fmt.Sprintf("sepia(%s)", fstr(pickF(r, []float32{0, 100, 50, -10, 120}, -20, 120)))
		case k == 25:
			return fmt.Sprintf("hue(%s)", fstr(pickF(r, []float32{0, 45, -45, 180, 360, -360, 720, 1e-4}, -720, 720)))
		case k == 26:
			return fmt.Sprintf("saturation(%s)", fstr(pickF(r, []float32{0, 50, -50, -100, 500, 600, -150}, -150, 600)))
		case k == 27:
			return fmt.Sprintf("colorize(%s,%s,%s)", fstr(pickF(r, []float32{0, 240, 360, -30}, -400, 400)), fstr(pickF(r, []float32{0, 50, 100, 110}, -10, 110)), fstr(pickF(r, []float32{0, 100, 50, -10}, -10, 110)))
		case k == 28:
			return fmt.Sprintf("colorbalance(%s,%s,%s)", fstr(pickF(r, []float32{0, 10, -10, -100, 500}, -150, 600)), fstr(pickF(r, []float32{0, 10, -10, -100, 500}, -150, 600)), fstr(pickF(r, []float32{0, 10, -10, -100, 500}, -150, 600)))
		case k == 29:
			return fmt.Sprintf("threshold(%s)", fstr(pickF(r, []float32{0, 50, 100, -10, 110}, -10, 110)))
		case k == 30:
			// Only the three synth callbacks: colorFuncs[3] (extra.go's
			// setter table) was added after synth.tsv was generated, and
			// r.intn(len(colorFuncs)) would change the checked-in cases.
			return fmt.Sprintf("colorfunc(%d)", r.intn(numSynthColorFuncs))
		case k < 33:
			n := r.intn(30)
			var ks []string
			for i := 0; i < n; i++ {
				v := float32(0)
				if r.intn(4) != 0 {
					v = pickF(r, []float32{1, -1, 2, -2, 0.5}, -3, 3)
				}
				ks = append(ks, fstr(v))
			}
			delta := float32(0)
			if r.intn(3) == 0 {
				delta = rf(r, -0.5, 0.5)
			}
			return fmt.Sprintf("convolution(%s,%d,%d,%d,%s)", strings.Join(ks, "/"), r.intn(2), r.intn(2), r.intn(2), fstr(delta))
		case k == 33:
			return fmt.Sprintf("gaussianblur(%s)", fstr(pickF(r, []float32{0, -1, 0.3, 1, 2.5, 1.5}, 0, 4)))
		case k == 34:
			return fmt.Sprintf("unsharpmask(%s,%s,%s)", fstr(pickF(r, []float32{0, 1, 0.5, 2}, 0, 3)), fstr(pickF(r, []float32{0, 1, 1.5, -1}, -1, 3)), fstr(pickF(r, []float32{0, 0.05, -0.05}, -0.1, 0.2)))
		case k == 35:
			return fmt.Sprintf("mean(%d,%d)", r.intn(11)-1, r.intn(2))
		case k == 36:
			return fmt.Sprintf("%s(%d,%d)", []string{"median", "minimum", "maximum"}[r.intn(3)], r.intn(8), r.intn(2))
		case k == 37:
			return fmt.Sprintf("pixelate(%d)", r.intn(14)-1)
		default:
			return "copy()"
		}
	}
}

// lutUnsafe reports whether a source spec may produce channel values > 1
// (invalid premultiplied colours), which make gift's LUT lookup panic.
func lutUnsafe(s imgSpec) bool {
	return !s.valid && (s.typ == "rgba" || s.typ == "rgba64" || s.typ == "paletted")
}

type synthCase struct {
	op      string // draw | drawat
	src     imgSpec
	dst     imgSpec
	pt      string // x,y,op for drawat
	filters string
}

func (c synthCase) line(i int) string {
	pt := c.pt
	if pt == "" {
		pt = "-"
	}
	f := c.filters
	if f == "" {
		f = "-"
	}
	return fmt.Sprintf("%d\t%s\t%s\t%s\t%s\t%s", i, c.op, c.src, c.dst, pt, f)
}

func genSynthCase(r *rng) synthCase {
	var c synthCase
	typ := srcTypes[r.intn(len(srcTypes))]
	w, h := genSize(r)
	var ox, oy int
	if strings.HasPrefix(typ, "ycbcr") || strings.HasPrefix(typ, "nycbcra") {
		ox, oy = r.intn(7), r.intn(7)
	} else {
		ox, oy = r.intn(11)-5, r.intn(11)-5
	}
	c.src = imgSpec{typ: typ, rect: image.Rect(ox, oy, ox+w, oy+h), seed: r.next(), vmode: r.intn(2), amode: r.intn(3), valid: r.intn(8) != 0}

	var nf int
	switch k := r.intn(10); {
	case k == 0:
		nf = 0
	case k < 7:
		nf = 1
	case k < 9:
		nf = 2
	default:
		nf = 3
	}
	var fs []string
	b := c.src.rect
	for i := 0; i < nf; i++ {
		f := genFilter(r, b, i > 0 || !lutUnsafe(c.src))
		fs = append(fs, f)
		b = parseFilter(f).Bounds(b)
	}
	c.filters = strings.Join(fs, "|")

	dtyp := settableTypes[r.intn(len(settableTypes))]
	if r.intn(4) == 0 {
		c.op = "drawat"
		dw, dh := 1+r.intn(40), 1+r.intn(40)
		dx, dy := r.intn(11)-5, r.intn(11)-5
		c.dst = imgSpec{typ: dtyp, rect: image.Rect(dx, dy, dx+dw, dy+dh), seed: r.next(), vmode: r.intn(2), amode: r.intn(3), valid: true}
		px := dx - 8 + r.intn(dw+16)
		py := dy - 8 + r.intn(dh+16)
		if r.intn(4) == 0 {
			px, py = dx, dy
		}
		c.pt = fmt.Sprintf("%d,%d,%d", px, py, r.intn(2))
	} else {
		c.op = "draw"
		var dr image.Rectangle
		if r.intn(7) == 0 {
			dx, dy := r.intn(11)-5, r.intn(11)-5
			dr = image.Rect(dx, dy, dx+r.intn(30), dy+r.intn(30))
		} else {
			ox, oy := 0, 0
			if r.intn(2) == 0 {
				ox, oy = r.intn(7)-3, r.intn(7)-3
			}
			dr = b.Add(image.Pt(ox, oy))
		}
		c.dst = imgSpec{typ: dtyp, rect: dr, seed: r.next(), vmode: r.intn(2), amode: r.intn(3), valid: true, blank: true}
	}
	return c
}

func runSynth(c synthCase) image.Image {
	src := genImage(c.src)
	g := gift.New(parseFilters(c.filters)...)
	dst := genImage(c.dst).(draw.Image)
	if c.op == "drawat" {
		f := strings.Split(c.pt, ",")
		g.DrawAt(dst, src, image.Pt(pi(f[0]), pi(f[1])), gift.Operator(pi(f[2])))
	} else {
		g.Draw(dst, src)
	}
	return dst
}

func synth(n int, seed uint64) {
	r := &rng{s: seed}
	for i := 0; i < n; i++ {
		c := genSynthCase(r)
		dst := runSynth(c)
		_, _ = fmt.Fprintf(out, "%s\t%s\n", c.line(i), digest(dst))
	}
}

func parseSynthLine(line string) synthCase {
	f := strings.Split(line, "\t")
	c := synthCase{op: f[1], src: parseImgSpec(f[2]), dst: parseImgSpec(f[3])}
	if f[4] != "-" {
		c.pt = f[4]
	}
	if f[5] != "-" {
		c.filters = f[5]
	}
	return c
}

func one(line string) {
	c := parseSynthLine(line)
	dst := runSynth(c)
	_, _ = fmt.Fprintf(out, "digest %s\n", digest(dst))
	b := dst.Bounds()
	for y := b.Min.Y; y < b.Max.Y; y++ {
		for x := b.Min.X; x < b.Max.X; x++ {
			_, _ = fmt.Fprintf(out, "(%d,%d) %v\n", x, y, dst.At(x, y))
		}
	}
}

// ---------------------------------------------------------------------------
// math vectors: the float64 functions gift and resampling.go call.

func mathVectors(n int) {
	r := &rng{s: 12345}
	f64 := func(v float64) string { return strconv.FormatUint(math.Float64bits(v), 16) }
	special := []float64{0, math.Copysign(0, -1), 1, -1, 0.5, -0.5, 2, 3, 1e-300, 5e-324, -5e-324, 1e300, -1e300,
		math.Inf(1), math.Inf(-1), math.NaN(), 709.78, 709.79, -745.13, -745.14, -708.4, -720, -740, 1e-10, 1 << 28, 1 << 29, 1<<29 + 0.5, 1e19, 1e20,
		math.Pi, math.Pi / 2, math.Pi / 4, 3 * math.Pi / 4, 1e6 * math.Pi, 1e15, 12.92, 2.4, 1 / 2.4, 0.055, 1.055}
	gen := func() float64 {
		switch r.intn(6) {
		case 0:
			return special[r.intn(len(special))]
		case 1:
			return math.Float64frombits(r.next())
		case 2:
			return float64(float32(r.next()%2000001)/1000000 - 1)
		case 3:
			return (float64(r.next()%2000001)/1000000 - 1) * 800
		case 4:
			return float64(math.Float32frombits(uint32(r.next())))
		default:
			return (float64(r.next()%2000001)/1000000 - 1) * 1e6
		}
	}
	for i := 0; i < n; i++ {
		x := gen()
		y := gen()
		if r.intn(3) == 0 {
			y = float64(r.intn(21) - 10)
		}
		if r.intn(4) == 0 {
			x = math.Abs(x)
		}
		s, c := math.Sincos(x)
		_, _ = fmt.Fprintf(out, "%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n", f64(x), f64(y),
			f64(math.Exp(x)), f64(math.Log(x)), f64(math.Pow(x, y)), f64(math.Sin(x)), f64(math.Cos(x)), f64(s), f64(c))
	}
}

// kernels evaluates every resampling kernel over float32 inputs.
func kernels(n int) {
	r := &rng{s: 777}
	for _, name := range resamplingNames {
		rs := resampling(name)
		_, _ = fmt.Fprintf(out, "support\t%s\t%08x\n", name, math.Float32bits(rs.Support()))
	}
	for i := 0; i < n; i++ {
		var x float32
		switch r.intn(4) {
		case 0:
			x = float32(r.intn(4001)-2000) / 500
		case 1:
			x = math.Float32frombits(uint32(r.next()))
		default:
			x = rf(r, -4, 4)
		}
		var parts []string
		for _, name := range resamplingNames {
			parts = append(parts, fmt.Sprintf("%08x", math.Float32bits(resampling(name).Kernel(x))))
		}
		_, _ = fmt.Fprintf(out, "k\t%08x\t%s\n", math.Float32bits(x), strings.Join(parts, ","))
	}
}
