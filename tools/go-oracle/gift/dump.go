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

type dumpWriter struct{ w io.Writer }

func (d dumpWriter) u8(v uint8)   { d.w.Write([]byte{v}) }
func (d dumpWriter) i64(v int)    { binary.Write(d.w, binary.LittleEndian, int64(v)) }
func (d dumpWriter) u32(v uint32) { binary.Write(d.w, binary.LittleEndian, v) }
func (d dumpWriter) buf(b []byte) {
	binary.Write(d.w, binary.LittleEndian, uint64(len(b)))
	d.w.Write(b)
}

func (d dumpWriter) color(c color.Color) {
	switch c := c.(type) {
	case color.RGBA:
		d.u8(1)
		d.w.Write([]byte{c.R, c.G, c.B, c.A})
	case color.NRGBA:
		d.u8(2)
		d.w.Write([]byte{c.R, c.G, c.B, c.A})
	case color.Gray:
		d.u8(3)
		d.w.Write([]byte{c.Y})
	case color.RGBA64:
		d.u8(4)
		binary.Write(d.w, binary.BigEndian, []uint16{c.R, c.G, c.B, c.A})
	case color.NRGBA64:
		d.u8(5)
		binary.Write(d.w, binary.BigEndian, []uint16{c.R, c.G, c.B, c.A})
	case color.Alpha:
		d.u8(6)
		d.w.Write([]byte{c.A})
	default:
		panic(fmt.Sprintf("dump: palette colour %T", c))
	}
}

func writeDump(w io.Writer, img image.Image) {
	d := dumpWriter{w}
	d.w.Write([]byte(dumpMagic))
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
	zw.Close()
	if err := os.WriteFile(path, b.Bytes(), 0o644); err != nil {
		panic(err)
	}
}

func decodeFile(path string) image.Image {
	f, err := os.Open(path)
	if err != nil {
		panic(err)
	}
	defer f.Close()
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
	fmt.Fprintf(out, "%s\t%T\t%v\n", outPath, img, img.Bounds())
}

func loadDump(path string) image.Image {
	b, err := os.ReadFile(path)
	if err != nil {
		panic(err)
	}
	zr, err := gzip.NewReader(bytes.NewReader(b))
	if err != nil {
		panic(err)
	}
	raw, err := io.ReadAll(zr)
	if err != nil {
		panic(err)
	}
	rd := bytes.NewReader(raw)
	magic := make([]byte, 8)
	io.ReadFull(rd, magic)
	if string(magic) != dumpMagic {
		panic("bad dump " + path)
	}
	u8 := func() uint8 { v, _ := rd.ReadByte(); return v }
	i64 := func() int { var v int64; binary.Read(rd, binary.LittleEndian, &v); return int(v) }
	buf := func() []byte {
		var n uint64
		binary.Read(rd, binary.LittleEndian, &n)
		b := make([]byte, n)
		io.ReadFull(rd, b)
		return b
	}
	kind := u8()
	r := image.Rect(i64(), i64(), i64(), i64())
	switch kind {
	case 1:
		s := i64()
		return &image.NRGBA{Rect: r, Stride: s, Pix: buf()}
	case 2:
		s := i64()
		return &image.NRGBA64{Rect: r, Stride: s, Pix: buf()}
	case 3:
		s := i64()
		return &image.RGBA{Rect: r, Stride: s, Pix: buf()}
	case 4:
		s := i64()
		return &image.RGBA64{Rect: r, Stride: s, Pix: buf()}
	case 5:
		s := i64()
		return &image.Gray{Rect: r, Stride: s, Pix: buf()}
	case 6:
		s := i64()
		return &image.Gray16{Rect: r, Stride: s, Pix: buf()}
	case 7:
		ratio := image.YCbCrSubsampleRatio(u8())
		ys := i64()
		cs := i64()
		return &image.YCbCr{Rect: r, SubsampleRatio: ratio, YStride: ys, CStride: cs, Y: buf(), Cb: buf(), Cr: buf()}
	case 8:
		s := i64()
		pix := buf()
		var n uint32
		binary.Read(rd, binary.LittleEndian, &n)
		pal := make(color.Palette, n)
		for i := range pal {
			switch u8() {
			case 1:
				pal[i] = color.RGBA{u8(), u8(), u8(), u8()}
			case 2:
				pal[i] = color.NRGBA{u8(), u8(), u8(), u8()}
			case 3:
				pal[i] = color.Gray{u8()}
			case 4:
				var v [4]uint16
				binary.Read(rd, binary.BigEndian, &v)
				pal[i] = color.RGBA64{v[0], v[1], v[2], v[3]}
			case 5:
				var v [4]uint16
				binary.Read(rd, binary.BigEndian, &v)
				pal[i] = color.NRGBA64{v[0], v[1], v[2], v[3]}
			case 6:
				pal[i] = color.Alpha{u8()}
			}
		}
		return &image.Paletted{Rect: r, Stride: s, Pix: pix, Palette: pal}
	case 9:
		s := i64()
		return &image.CMYK{Rect: r, Stride: s, Pix: buf()}
	}
	panic("bad dump kind")
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
		filepath.Walk(filepath.Join(siteRoot, dir), func(p string, fi os.FileInfo, err error) error {
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
			fmt.Fprintf(out, "%s\t%s\t%s\t%s\n", p.name, op.kind, op.arg, digest(runRealOp(img, op, dumps)))
		}
	}
}
