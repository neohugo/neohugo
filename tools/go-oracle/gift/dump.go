package main

// Raw image dumps (gzip-compressed) of decoded images, read back by the Rust
// tests (crates/gift/tests/common/mod.rs: read_dump), and the real-image op
// tables.

import (
	"bufio"
	"bytes"
	"compress/gzip"
	"encoding/binary"
	"fmt"
	"image"
	"image/color"
	"image/draw"
	_ "image/jpeg"
	_ "image/png"
	"io"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"github.com/disintegration/gift"
)

const dumpMagic = "GIFTIMG1"

// dumpWriter only writes to a gzip.Writer over a bytes.Buffer, whose writes
// cannot fail; saveDump checks the final Close.
type dumpWriter struct{ w io.Writer }

func (d dumpWriter) u8(v uint8)   { _, _ = d.w.Write([]byte{v}) }
func (d dumpWriter) i64(v int)    { _ = binary.Write(d.w, binary.LittleEndian, int64(v)) }
func (d dumpWriter) u32(v uint32) { _ = binary.Write(d.w, binary.LittleEndian, v) }
func (d dumpWriter) buf(b []byte) {
	_ = binary.Write(d.w, binary.LittleEndian, uint64(len(b)))
	_, _ = d.w.Write(b)
}

func (d dumpWriter) color(c color.Color) {
	switch c := c.(type) {
	case color.RGBA:
		d.u8(1)
		_, _ = d.w.Write([]byte{c.R, c.G, c.B, c.A})
	case color.NRGBA:
		d.u8(2)
		_, _ = d.w.Write([]byte{c.R, c.G, c.B, c.A})
	case color.Gray:
		d.u8(3)
		_, _ = d.w.Write([]byte{c.Y})
	case color.RGBA64:
		d.u8(4)
		_ = binary.Write(d.w, binary.BigEndian, []uint16{c.R, c.G, c.B, c.A})
	case color.NRGBA64:
		d.u8(5)
		_ = binary.Write(d.w, binary.BigEndian, []uint16{c.R, c.G, c.B, c.A})
	case color.Alpha:
		d.u8(6)
		_, _ = d.w.Write([]byte{c.A})
	default:
		panic(fmt.Sprintf("dump: palette colour %T", c))
	}
}

func writeDump(w io.Writer, img image.Image) {
	d := dumpWriter{w}
	_, _ = d.w.Write([]byte(dumpMagic))
	rect := func(r image.Rectangle) {
		d.i64(r.Min.X)
		d.i64(r.Min.Y)
		d.i64(r.Max.X)
		d.i64(r.Max.Y)
	}
	switch m := img.(type) {
	case *image.NRGBA:
		d.u8(1)
		rect(m.Rect)
		d.i64(m.Stride)
		d.buf(m.Pix)
	case *image.NRGBA64:
		d.u8(2)
		rect(m.Rect)
		d.i64(m.Stride)
		d.buf(m.Pix)
	case *image.RGBA:
		d.u8(3)
		rect(m.Rect)
		d.i64(m.Stride)
		d.buf(m.Pix)
	case *image.RGBA64:
		d.u8(4)
		rect(m.Rect)
		d.i64(m.Stride)
		d.buf(m.Pix)
	case *image.Gray:
		d.u8(5)
		rect(m.Rect)
		d.i64(m.Stride)
		d.buf(m.Pix)
	case *image.Gray16:
		d.u8(6)
		rect(m.Rect)
		d.i64(m.Stride)
		d.buf(m.Pix)
	case *image.YCbCr:
		d.u8(7)
		rect(m.Rect)
		d.u8(uint8(m.SubsampleRatio))
		d.i64(m.YStride)
		d.i64(m.CStride)
		d.buf(m.Y)
		d.buf(m.Cb)
		d.buf(m.Cr)
	case *image.Paletted:
		d.u8(8)
		rect(m.Rect)
		d.i64(m.Stride)
		d.buf(m.Pix)
		d.u32(uint32(len(m.Palette)))
		for _, c := range m.Palette {
			d.color(c)
		}
	case *image.CMYK:
		d.u8(9)
		rect(m.Rect)
		d.i64(m.Stride)
		d.buf(m.Pix)
	default:
		panic(fmt.Sprintf("dump: %T", img))
	}
}

func saveDump(path string, img image.Image) {
	var b bytes.Buffer
	zw, _ := gzip.NewWriterLevel(&b, gzip.BestCompression)
	writeDump(zw, img)
	if err := zw.Close(); err != nil {
		panic(err)
	}
	if err := os.WriteFile(path, b.Bytes(), 0o644); err != nil {
		panic(err)
	}
}

// dumpReader reads the format of writeDump. The oracle only reads its own
// dumps, so a short read panics.
type dumpReader struct{ r io.Reader }

func (d dumpReader) read(n int) []byte {
	b := make([]byte, n)
	if _, err := io.ReadFull(d.r, b); err != nil {
		panic(err)
	}
	return b
}

func (d dumpReader) u8() uint8   { return d.read(1)[0] }
func (d dumpReader) i64() int    { return int(int64(binary.LittleEndian.Uint64(d.read(8)))) }
func (d dumpReader) u32() uint32 { return binary.LittleEndian.Uint32(d.read(4)) }
func (d dumpReader) buf() []byte { return d.read(d.i64()) }

func (d dumpReader) rect() image.Rectangle {
	var r image.Rectangle
	r.Min.X = d.i64()
	r.Min.Y = d.i64()
	r.Max.X = d.i64()
	r.Max.Y = d.i64()
	return r
}

func (d dumpReader) color() color.Color {
	switch k := d.u8(); k {
	case 1:
		v := d.read(4)
		return color.RGBA{v[0], v[1], v[2], v[3]}
	case 2:
		v := d.read(4)
		return color.NRGBA{v[0], v[1], v[2], v[3]}
	case 3:
		return color.Gray{d.u8()}
	case 4, 5:
		v := d.read(8)
		w := func(i int) uint16 { return uint16(v[i])<<8 | uint16(v[i+1]) }
		if k == 4 {
			return color.RGBA64{w(0), w(2), w(4), w(6)}
		}
		return color.NRGBA64{w(0), w(2), w(4), w(6)}
	case 6:
		return color.Alpha{d.u8()}
	default:
		panic(fmt.Sprintf("dump: palette colour kind %d", k))
	}
}

// loadDump reads a gzip-compressed dump written by saveDump (the Rust tests'
// crates/gift/tests/common/mod.rs:load_dump is the same reader).
func loadDump(path string) image.Image {
	raw, err := os.ReadFile(path)
	if err != nil {
		panic(err)
	}
	zr, err := gzip.NewReader(bytes.NewReader(raw))
	if err != nil {
		panic(err)
	}
	d := dumpReader{zr}
	if string(d.read(len(dumpMagic))) != dumpMagic {
		panic("dump: bad magic in " + path)
	}
	switch k := d.u8(); k {
	case 1:
		r := d.rect()
		return &image.NRGBA{Rect: r, Stride: d.i64(), Pix: d.buf()}
	case 2:
		r := d.rect()
		return &image.NRGBA64{Rect: r, Stride: d.i64(), Pix: d.buf()}
	case 3:
		r := d.rect()
		return &image.RGBA{Rect: r, Stride: d.i64(), Pix: d.buf()}
	case 4:
		r := d.rect()
		return &image.RGBA64{Rect: r, Stride: d.i64(), Pix: d.buf()}
	case 5:
		r := d.rect()
		return &image.Gray{Rect: r, Stride: d.i64(), Pix: d.buf()}
	case 6:
		r := d.rect()
		return &image.Gray16{Rect: r, Stride: d.i64(), Pix: d.buf()}
	case 7:
		m := &image.YCbCr{Rect: d.rect()}
		m.SubsampleRatio = image.YCbCrSubsampleRatio(d.u8())
		m.YStride = d.i64()
		m.CStride = d.i64()
		m.Y = d.buf()
		m.Cb = d.buf()
		m.Cr = d.buf()
		return m
	case 8:
		m := &image.Paletted{Rect: d.rect()}
		m.Stride = d.i64()
		m.Pix = d.buf()
		m.Palette = make(color.Palette, d.u32())
		for i := range m.Palette {
			m.Palette[i] = d.color()
		}
		return m
	case 9:
		r := d.rect()
		return &image.CMYK{Rect: r, Stride: d.i64(), Pix: d.buf()}
	default:
		panic(fmt.Sprintf("dump: kind %d in %s", k, path))
	}
}

func decodeFile(path string) image.Image {
	f, err := os.Open(path)
	if err != nil {
		panic(err)
	}
	defer func() { _ = f.Close() }()
	img, _, err := image.Decode(bufio.NewReader(f))
	if err != nil {
		panic(fmt.Sprintf("%s: %v", path, err))
	}
	return img
}

// compactCopy returns an image of the same concrete type holding only the
// pixels of r (keeping r's non-zero origin), with minimal buffers.
func compactCopy(img image.Image, r image.Rectangle) image.Image {
	r = r.Intersect(img.Bounds())
	switch m := img.(type) {
	case *image.YCbCr:
		n := image.NewYCbCr(r, m.SubsampleRatio)
		for y := r.Min.Y; y < r.Max.Y; y++ {
			for x := r.Min.X; x < r.Max.X; x++ {
				n.Y[n.YOffset(x, y)] = m.Y[m.YOffset(x, y)]
				n.Cb[n.COffset(x, y)] = m.Cb[m.COffset(x, y)]
				n.Cr[n.COffset(x, y)] = m.Cr[m.COffset(x, y)]
			}
		}
		return n
	case *image.Paletted:
		n := image.NewPaletted(r, m.Palette)
		for y := r.Min.Y; y < r.Max.Y; y++ {
			for x := r.Min.X; x < r.Max.X; x++ {
				n.Pix[n.PixOffset(x, y)] = m.Pix[m.PixOffset(x, y)]
			}
		}
		return n
	}
	var n draw.Image
	switch img.(type) {
	case *image.NRGBA:
		n = image.NewNRGBA(r)
	case *image.NRGBA64:
		n = image.NewNRGBA64(r)
	case *image.RGBA:
		n = image.NewRGBA(r)
	case *image.RGBA64:
		n = image.NewRGBA64(r)
	case *image.Gray:
		n = image.NewGray(r)
	case *image.Gray16:
		n = image.NewGray16(r)
	case *image.CMYK:
		n = image.NewCMYK(r)
	default:
		panic(fmt.Sprintf("compactCopy %T", img))
	}
	draw.Draw(n, r, img, r.Min, draw.Src)
	return n
}

func dumpFile(in, outPath, crop string) {
	img := decodeFile(in)
	if crop != "" {
		f := strings.Split(crop, ",")
		img = compactCopy(img, image.Rect(pi(f[0]), pi(f[1]), pi(f[2]), pi(f[3])))
	}
	saveDump(outPath, img)
	_, _ = fmt.Fprintf(out, "%s\t%T\t%v\n", outPath, img, img.Bounds())
}

// doFilter is neohugo resources/images/image.go:(*ImageProcessor).doFilter
// (non-GIF path): the destination type follows the source type.
func doFilter(src image.Image, filters ...gift.Filter) image.Image {
	g := gift.New(filters...)
	bounds := g.Bounds(src.Bounds())
	var dst draw.Image
	switch src.(type) {
	case *image.RGBA:
		dst = image.NewRGBA(bounds)
	case *image.NRGBA:
		dst = image.NewNRGBA(bounds)
	case *image.Gray:
		dst = image.NewGray(bounds)
	default:
		dst = image.NewNRGBA(bounds)
	}
	g.Draw(dst, src)
	return dst
}

// realOp is one line of a real-image op table:
//
//	dofilter <filters>        dst = doFilter(src, filters...)
//	overlay  <wm>,<x>,<y>     wmR = doFilter(wm, resize(W,H,box)) (W,H = src size);
//	                          dst = doFilter(src, overlay(wmR, x, y))
//	overlaya <wm>,<w>,<h>     A = doFilter(src, resize(w,h,box)); wmR = doFilter(wm, resize(w,h,box));
//	                          dst = doFilter(A, overlay(wmR, 0, 0))
type realOp struct{ kind, arg string }

func runRealOp(src image.Image, op realOp, dumps func(string) image.Image) image.Image {
	switch op.kind {
	case "dofilter":
		return doFilter(src, parseFilters(op.arg)...)
	case "overlay":
		f := strings.Split(op.arg, ",")
		b := src.Bounds()
		wm := doFilter(dumps(f[0]), gift.Resize(b.Dx(), b.Dy(), gift.BoxResampling))
		return doFilter(src, overlayFilter{src: wm, x: pi(f[1]), y: pi(f[2])})
	case "overlaya":
		f := strings.Split(op.arg, ",")
		w, h := pi(f[1]), pi(f[2])
		a := doFilter(src, gift.Resize(w, h, gift.BoxResampling))
		wm := doFilter(dumps(f[0]), gift.Resize(w, h, gift.BoxResampling))
		return doFilter(a, overlayFilter{src: wm, x: 0, y: 0})
	}
	panic("bad op " + op.kind)
}

// realOpsFor returns the op table for one real image. level 0: Hugo pipeline
// only; 1: + every resampling filter; 2: + the other filters.
func realOpsFor(b image.Rectangle, idx int, wm string, level int) []realOp {
	w, h := b.Dx(), b.Dy()
	var ops []realOp
	add := func(kind, arg string) { ops = append(ops, realOp{kind, arg}) }
	for _, sz := range [][2]int{{600, 480}, {300, 240}, {600, 200}, {128, 128}, {32, 32}} {
		add("dofilter", fmt.Sprintf("resize(%d,%d,box)", sz[0], sz[1]))
	}
	add("dofilter", fmt.Sprintf("resize(%d,%d,box)", w, h)) // same size: copyimage
	if wm != "" {
		add("overlay", wm+",0,0")
		add("overlaya", wm+",600,480")
		add("overlaya", wm+",300,240")
		add("overlay", fmt.Sprintf("%s,%d,%d", wm, w/3, -h/5))
	}
	if level < 1 {
		return ops
	}
	for i, name := range resamplingNames {
		add("dofilter", fmt.Sprintf("resize(%d,0,%s)", max(1, w*2/3+(idx+i)%5), name))
		add("dofilter", fmt.Sprintf("resize(0,%d,%s)", max(1, h*4/3-(idx+i)%3), name))
		if level >= 2 {
			add("dofilter", fmt.Sprintf("resize(%d,%d,%s)", max(1, w/3+i), max(1, h*5/4), name))
			add("dofilter", fmt.Sprintf("resizetofill(%d,%d,%s,%d)", max(1, w/2), max(1, w/2), name, (idx+i)%9))
			add("dofilter", fmt.Sprintf("resizetofit(%d,%d,%s)", max(1, w/2+1), max(1, h/3+2), name))
		}
	}
	if level < 2 {
		return ops
	}
	emboss := strings.Join([]string{fstr(-1), fstr(-1), fstr(0), fstr(-1), fstr(1), fstr(1), fstr(0), fstr(1), fstr(1)}, "/")
	for _, f := range []string{
		"croptosize(100,100,4)", "rotate180()", "rotate90()", "transverse()",
		fmt.Sprintf("rotate(%s,rgba/0/0/0/0,2)", fstr(30)),
		fmt.Sprintf("rotate(%s,nrgba/255/0/0/128,1)", fstr(-17.5)),
		fmt.Sprintf("rotate(%s,gray/200,0)", fstr(100)),
		fmt.Sprintf("brightness(%s)", fstr(30)), fmt.Sprintf("brightness(%s)", fstr(-30)),
		fmt.Sprintf("contrast(%s)", fstr(30)), fmt.Sprintf("contrast(%s)", fstr(-30)),
		fmt.Sprintf("saturation(%s)", fstr(50)), fmt.Sprintf("saturation(%s)", fstr(-50)),
		fmt.Sprintf("gamma(%s)", fstr(1.5)), fmt.Sprintf("gamma(%s)", fstr(0.5)),
		fmt.Sprintf("gaussianblur(%s)", fstr(1)), fmt.Sprintf("gaussianblur(%s)", fstr(2.5)),
		fmt.Sprintf("unsharpmask(%s,%s,%s)", fstr(1), fstr(1), fstr(0)),
		fmt.Sprintf("sigmoid(%s,%s)", fstr(0.5), fstr(7)), fmt.Sprintf("sigmoid(%s,%s)", fstr(0.5), fstr(-7)),
		"pixelate(5)",
		fmt.Sprintf("colorize(%s,%s,%s)", fstr(240), fstr(50), fstr(100)),
		"grayscale()", fmt.Sprintf("sepia(%s)", fstr(100)), "invert()",
		"mean(5,1)", "median(5,1)", "minimum(5,1)", "maximum(5,1)", "median(3,0)",
		fmt.Sprintf("hue(%s)", fstr(45)),
		fmt.Sprintf("colorbalance(%s,%s,%s)", fstr(10), fstr(-10), fstr(-10)),
		"colorfunc(0)",
		fmt.Sprintf("convolution(%s,0,0,0,%s)", emboss, fstr(0)),
		"sobel()", "srgbtolinear()", "lineartosrgb()",
		fmt.Sprintf("threshold(%s)", fstr(50)),
	} {
		add("dofilter", f)
	}
	return ops
}

// realFixtures writes the checked-in real-image fixtures: compact crops of
// seeksnack images (plus gift's testdata/src.png) and their op table.
func realFixtures(siteRoot, giftTestdata, outDir string) {
	type pick struct{ name, path, crop string }
	picks := []pick{
		// YCbCr JPEGs with 4:4:4, 4:2:0 and 4:2:2 chroma (found below).
		{"ycc444", "", ""},
		{"ycc420", "", ""},
		{"ycc422", "", ""},
		// NRGBA watermark (mostly transparent), a crop and the full image.
		{"watermark", "assets/images/watermark.png", "252,196,348,268"},
		{"watermark_full", "assets/images/watermark.png", ""},
		// RGB PNG -> *image.RGBA.
		{"classic", "content/companies/classic-foods-inc/classic-foods-inc.png", ""},
		// Paletted PNG with tRNS.
		{"berli", "content/companies/Berli-Jucker-Foods-Ltd.Berli-Jucker-PLC/berli-jucker-foods-ltd.berli-jucker-plc.png", ""},
		// RGBA PNG -> *image.NRGBA.
		{"fritolay", "content/companies/frito-lay/frito-lay.png", ""},
		{"favicon32", "assets/images/favicon/favicon-32x32.png", ""},
	}
	// Find JPEGs with the wanted chroma subsampling in the site.
	var jpgs []string
	for _, dir := range []string{"content", "assets"} {
		_ = filepath.Walk(filepath.Join(siteRoot, dir), func(p string, fi os.FileInfo, err error) error {
			if err == nil && !fi.IsDir() && strings.HasSuffix(strings.ToLower(p), ".jpg") {
				jpgs = append(jpgs, p)
			}
			return nil
		})
	}
	sort.Strings(jpgs)
	for i := range picks {
		if picks[i].path != "" {
			picks[i].path = filepath.Join(siteRoot, picks[i].path)
			continue
		}
		want := ycbcrRatio(picks[i].name)
		for _, p := range jpgs {
			if y, ok := decodeFile(p).(*image.YCbCr); ok && y.SubsampleRatio == want {
				picks[i].path = p
				b := y.Bounds()
				picks[i].crop = fmt.Sprintf("%d,%d,%d,%d", b.Min.X+33, b.Min.Y+17, b.Min.X+33+89, b.Min.Y+17+67)
				break
			}
		}
	}
	picks = append(picks, pick{"giftsrc", filepath.Join(giftTestdata, "src.png"), ""})

	imgs := map[string]image.Image{}
	for _, p := range picks {
		if p.path == "" {
			panic("no image for " + p.name)
		}
		img := decodeFile(p.path)
		if p.crop != "" {
			f := strings.Split(p.crop, ",")
			img = compactCopy(img, image.Rect(pi(f[0]), pi(f[1]), pi(f[2]), pi(f[3])))
		}
		imgs[p.name] = img
		saveDump(filepath.Join(outDir, p.name+".gz"), img)
		rel, _ := filepath.Rel(siteRoot, p.path)
		fmt.Fprintf(os.Stderr, "%s\t%s\t%T\t%v\n", p.name, rel, img, img.Bounds())
	}
	dumps := func(n string) image.Image { return imgs[n] }
	for i, p := range picks {
		if p.name == "watermark_full" {
			continue
		}
		img := imgs[p.name]
		for _, op := range realOpsFor(img.Bounds(), i, "watermark_full", 2) {
			_, _ = fmt.Fprintf(out, "%s\t%s\t%s\t%s\n", p.name, op.kind, op.arg, digest(runRealOp(img, op, dumps)))
		}
	}
}

// realPickNames are the dumps realFixtures writes, in pick order (the index
// feeds realOpsFor).
var realPickNames = []string{"ycc444", "ycc420", "ycc422", "watermark", "watermark_full", "classic", "berli", "fritolay", "favicon32", "giftsrc"}

// realOps re-runs the realFixtures op table from the dumps in dumpDir
// (crates/gift/tests/fixtures/real), so real.tsv can be regenerated without
// the seeksnack site.
func realOps(dumpDir string) {
	imgs := map[string]image.Image{}
	for _, n := range realPickNames {
		imgs[n] = loadDump(filepath.Join(dumpDir, n+".gz"))
	}
	dumps := func(n string) image.Image { return imgs[n] }
	for i, n := range realPickNames {
		if n == "watermark_full" {
			continue
		}
		img := imgs[n]
		for _, op := range realOpsFor(img.Bounds(), i, "watermark_full", 2) {
			_, _ = fmt.Fprintf(out, "%s\t%s\t%s\t%s\n", n, op.kind, op.arg, digest(runRealOp(img, op, dumps)))
		}
	}
}

// goTestdata dumps the TestGolden images testdata/dst_*.png of gift and
// prints name, Go type and bounds (testdata/src.png is realFixtures'
// giftsrc dump). PNG decoding is integer-only, so the decoded pixels do not
// depend on the platform (the gzip bytes may: compress/flate's block choice
// uses float estimates).
func goTestdata(giftTestdata, outDir string) {
	names, err := filepath.Glob(filepath.Join(giftTestdata, "dst_*.png"))
	if err != nil {
		panic(err)
	}
	sort.Strings(names)
	for _, p := range names {
		img := decodeFile(p)
		n := strings.TrimSuffix(filepath.Base(p), ".png")
		saveDump(filepath.Join(outDir, n+".gz"), img)
		_, _ = fmt.Fprintf(out, "%s\t%T\t%v\n", n, img, img.Bounds())
	}
}
