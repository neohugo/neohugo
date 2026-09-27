// Command go-image is the Go oracle for the Rust crate crates/go-image.
//
// It exercises the go1.27.1 standard library packages image, image/color,
// image/color/palette, image/draw and image/jpeg and prints digests that the
// Rust differential tests compare against. All pseudo-random inputs come from
// a splitmix64 generator that the Rust tests re-implement call for call.
//
// Usage:
//
//	go-image decode-files <root> <listfile>   # decode/encode real JPEGs
//	go-image synth <n>                        # synthetic images, encode q1..100
//	go-image draw <n>                         # random draw.DrawMask / FloydSteinberg ops
//	go-image color                            # colour conversions over full input spaces
//	go-image fuzz <n> <files...>              # decode mutated JPEGs
//	go-image dct <n>                          # fdct/idct on random blocks
//
// synth, draw, fuzz and dct use seeds 0..n-1, or $GO_IMAGE_SEED0.. when that
// environment variable is set.
//
// and the corpus commands documented in corpus.go.
package main

import (
	"bufio"
	"bytes"
	"crypto/sha256"
	"encoding/binary"
	"encoding/hex"
	"fmt"
	"hash"
	"image"
	"image/color"
	"image/color/palette"
	"image/draw"
	"image/jpeg"
	"os"
	"path/filepath"
	"strconv"
	"strings"
)

// rng is splitmix64.
type rng struct{ s uint64 }

func (r *rng) next() uint64 {
	r.s += 0x9e3779b97f4a7c15
	z := r.s
	z = (z ^ (z >> 30)) * 0xbf58476d1ce4e5b9
	z = (z ^ (z >> 27)) * 0x94d049bb133111eb
	return z ^ (z >> 31)
}

func (r *rng) intn(n int) int { return int(r.next() % uint64(n)) }
func (r *rng) byte() byte     { return byte(r.next()) }
func (r *rng) u16() uint16    { return uint16(r.next()) }

func sha(b []byte) string {
	s := sha256.Sum256(b)
	return hex.EncodeToString(s[:])
}

func sha16(b []byte) string { return sha(b)[:16] }

// seedOffset returns the first seed for synth, draw, fuzz and dct: 0 (the
// checked-in fixtures), or $GO_IMAGE_SEED0 for out-of-repo campaigns over
// fresh seeds. Rows carry their seed, so the Rust tests replay any range.
func seedOffset() int {
	s := os.Getenv("GO_IMAGE_SEED0")
	if s == "" {
		return 0
	}
	n, err := strconv.Atoi(s)
	if err != nil || n < 0 {
		panic("bad GO_IMAGE_SEED0: " + s)
	}
	return n
}

var out = bufio.NewWriterSize(os.Stdout, 1<<20)

func main() {
	defer func() { _ = out.Flush() }()
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "usage: go-image <cmd> ...")
		os.Exit(2)
	}
	switch os.Args[1] {
	case "decode-files":
		decodeFiles(os.Args[2], os.Args[3])
	case "synth":
		n, _ := strconv.Atoi(os.Args[2])
		synth(n)
	case "draw":
		n, _ := strconv.Atoi(os.Args[2])
		drawOps(n)
	case "color":
		colorSpace()
	case "fuzz":
		n, _ := strconv.Atoi(os.Args[2])
		fuzz(n, os.Args[3:])
	case "dct":
		n, _ := strconv.Atoi(os.Args[2])
		dctBlocks(n)
	case "draw2":
		s0, _ := strconv.Atoi(os.Args[2])
		n, _ := strconv.Atoi(os.Args[3])
		drawOps2(s0, n)
	case "pack":
		packCorpus(os.Args[2], os.Args[3], os.Args[4])
	case "record":
		recordCorpus(os.Args[2])
	case "encsweep":
		encsweep(os.Args[2])
	case "mkjpeg":
		s0, _ := strconv.Atoi(os.Args[2])
		n, _ := strconv.Atoi(os.Args[3])
		mkjpegCmd(s0, n, os.Args[4])
	case "mutate":
		n, _ := strconv.Atoi(os.Args[2])
		s0, _ := strconv.Atoi(os.Args[3])
		mutateCmd(n, s0, os.Args[4], os.Args[5])
	default:
		fmt.Fprintln(os.Stderr, "unknown command", os.Args[1])
		os.Exit(2)
	}
}

func rectStr(r image.Rectangle) string {
	return fmt.Sprintf("%d,%d,%d,%d", r.Min.X, r.Min.Y, r.Max.X, r.Max.Y)
}

// imgDigest describes an image's concrete type, geometry and full buffers
// (including any padding beyond Rect that Go keeps in Pix).
func imgDigest(m image.Image) string {
	switch m := m.(type) {
	case *image.Gray:
		return fmt.Sprintf("Gray %s %d %d %s", rectStr(m.Rect), m.Stride, len(m.Pix), sha(m.Pix))
	case *image.RGBA:
		return fmt.Sprintf("RGBA %s %d %d %s", rectStr(m.Rect), m.Stride, len(m.Pix), sha(m.Pix))
	case *image.CMYK:
		return fmt.Sprintf("CMYK %s %d %d %s", rectStr(m.Rect), m.Stride, len(m.Pix), sha(m.Pix))
	case *image.YCbCr:
		return fmt.Sprintf("YCbCr %s %d %d %d %d %d %d %s %s %s", rectStr(m.Rect), int(m.SubsampleRatio),
			m.YStride, m.CStride, len(m.Y), len(m.Cb), len(m.Cr), sha(m.Y), sha(m.Cb), sha(m.Cr))
	case nil:
		return "nil"
	}
	return fmt.Sprintf("other %T", m)
}

func modelName(m color.Model) string {
	switch m {
	case color.GrayModel:
		return "Gray"
	case color.YCbCrModel:
		return "YCbCr"
	case color.RGBAModel:
		return "RGBA"
	case color.CMYKModel:
		return "CMYK"
	}
	return "other"
}

func encSha(m image.Image, q int) string {
	var buf bytes.Buffer
	if err := jpeg.Encode(&buf, m, &jpeg.Options{Quality: q}); err != nil {
		return "err:" + err.Error()
	}
	return sha(buf.Bytes())
}

// decodeFiles decodes every file in listfile (paths relative to root) and
// prints: path, DecodeConfig, image digest, q75 encoding of the decoded
// image, the RGBA conversion via draw.Draw(Src) and its q75 encoding.
func decodeFiles(root, listfile string) {
	list, err := os.ReadFile(listfile)
	if err != nil {
		panic(err)
	}
	for _, rel := range strings.Split(strings.TrimSpace(string(list)), "\n") {
		data, err := os.ReadFile(filepath.Join(root, rel))
		if err != nil {
			panic(err)
		}
		cfgStr := ""
		cfg, err := jpeg.DecodeConfig(bytes.NewReader(data))
		if err != nil {
			cfgStr = "err:" + err.Error()
		} else {
			cfgStr = fmt.Sprintf("%s,%d,%d", modelName(cfg.ColorModel), cfg.Width, cfg.Height)
		}
		m, err := jpeg.Decode(bytes.NewReader(data))
		if err != nil {
			_, _ = fmt.Fprintf(out, "%s\t%s\terr:%s\n", rel, cfgStr, err.Error())
			continue
		}
		b := m.Bounds()
		rgba := image.NewRGBA(b)
		draw.Draw(rgba, b, m, b.Min, draw.Src)
		_, _ = fmt.Fprintf(out, "%s\t%s\t%s\t%s\t%s\t%s\n", rel, cfgStr, imgDigest(m), encSha(m, 75), sha(rgba.Pix), encSha(rgba, 75))
	}
}

// ---------------------------------------------------------------------------
// Synthetic images.

var ratios = []image.YCbCrSubsampleRatio{
	image.YCbCrSubsampleRatio444, image.YCbCrSubsampleRatio422, image.YCbCrSubsampleRatio420,
	image.YCbCrSubsampleRatio440, image.YCbCrSubsampleRatio411, image.YCbCrSubsampleRatio410,
}

// fill fills b with either random bytes (mode 0) or a smooth gradient with
// small noise (mode 1), whose DCT has realistic zero runs.
func fill(r *rng, b []byte, stride int) {
	mode := r.intn(2)
	if mode == 0 {
		for i := range b {
			b[i] = r.byte()
		}
		return
	}
	k1 := r.intn(7)
	k2 := r.intn(7)
	c := r.intn(256)
	noise := r.intn(8)
	if stride <= 0 {
		stride = 1
	}
	for i := range b {
		v := (i%stride)*k1 + (i/stride)*k2 + c
		if noise > 0 {
			v += r.intn(noise)
		}
		b[i] = byte(v)
	}
}

func randColor(r *rng) color.Color {
	switch r.intn(11) {
	case 0:
		return color.RGBA{r.byte(), r.byte(), r.byte(), r.byte()}
	case 1:
		return color.NRGBA{r.byte(), r.byte(), r.byte(), r.byte()}
	case 2:
		return color.RGBA64{r.u16(), r.u16(), r.u16(), r.u16()}
	case 3:
		return color.NRGBA64{r.u16(), r.u16(), r.u16(), r.u16()}
	case 4:
		return color.Gray{r.byte()}
	case 5:
		return color.Gray16{r.u16()}
	case 6:
		return color.Alpha{r.byte()}
	case 7:
		return color.Alpha16{r.u16()}
	case 8:
		return color.YCbCr{r.byte(), r.byte(), r.byte()}
	case 9:
		return color.NYCbCrA{color.YCbCr{r.byte(), r.byte(), r.byte()}, r.byte()}
	}
	return color.CMYK{r.byte(), r.byte(), r.byte(), r.byte()}
}

func randPalette(r *rng) color.Palette {
	switch r.intn(3) {
	case 0:
		return palette.Plan9
	case 1:
		return palette.WebSafe
	}
	n := 1 + r.intn(20)
	p := make(color.Palette, n)
	for i := range p {
		p[i] = randColor(r)
	}
	return p
}

const numKinds = 17

// genImage creates an image of the given kind with bounds r0 and random or
// smooth content.
func genImage(r *rng, kind int, rect image.Rectangle) image.Image {
	switch kind {
	case 0:
		m := image.NewRGBA(rect)
		fill(r, m.Pix, m.Stride)
		return m
	case 1:
		m := image.NewNRGBA(rect)
		fill(r, m.Pix, m.Stride)
		return m
	case 2:
		m := image.NewGray(rect)
		fill(r, m.Pix, m.Stride)
		return m
	case 3, 4, 5, 6, 7, 8:
		// Go's chroma indexing (x/2 - Min.X/2) is only consistent for
		// non-negative coordinates (negative Min panics in Go too), so
		// YCbCr images are shifted into the positive quadrant.
		rect = rect.Add(image.Pt(16, 16))
		m := image.NewYCbCr(rect, ratios[kind-3])
		fill(r, m.Y, m.YStride)
		fill(r, m.Cb, m.CStride)
		fill(r, m.Cr, m.CStride)
		return m
	case 9:
		m := image.NewCMYK(rect)
		fill(r, m.Pix, m.Stride)
		return m
	case 10:
		m := image.NewRGBA64(rect)
		fill(r, m.Pix, m.Stride)
		return m
	case 11:
		m := image.NewNRGBA64(rect)
		fill(r, m.Pix, m.Stride)
		return m
	case 12:
		p := randPalette(r)
		m := image.NewPaletted(rect, p)
		for i := range m.Pix {
			m.Pix[i] = byte(r.intn(len(p)))
		}
		return m
	case 13:
		m := image.NewGray16(rect)
		fill(r, m.Pix, m.Stride)
		return m
	case 14:
		m := image.NewAlpha(rect)
		fill(r, m.Pix, m.Stride)
		return m
	case 15:
		rect = rect.Add(image.Pt(16, 16))
		m := image.NewNYCbCrA(rect, ratios[r.intn(len(ratios))])
		fill(r, m.Y, m.YStride)
		fill(r, m.Cb, m.CStride)
		fill(r, m.Cr, m.CStride)
		fill(r, m.A, m.AStride)
		return m
	}
	m := image.NewAlpha16(rect)
	fill(r, m.Pix, m.Stride)
	return m
}

func randRect(r *rng, maxSize, maxOff int) image.Rectangle {
	w := 1 + r.intn(maxSize)
	h := 1 + r.intn(maxSize)
	x0 := r.intn(2*maxOff+1) - maxOff
	y0 := r.intn(2*maxOff+1) - maxOff
	return image.Rect(x0, y0, x0+w, y0+h)
}

// synth encodes n synthetic images at every quality 1..100.
func synth(n int) {
	s0 := seedOffset()
	for seed := s0; seed < s0+n; seed++ {
		r := &rng{uint64(seed)*1000003 + 17}
		kind := r.intn(numKinds)
		maxSize := 40
		if r.intn(5) == 0 {
			maxSize = 300
		}
		m := genImage(r, kind, randRect(r, maxSize, 10))
		var hs []string
		var q75 []byte
		for q := 1; q <= 100; q++ {
			var buf bytes.Buffer
			if err := jpeg.Encode(&buf, m, &jpeg.Options{Quality: q}); err != nil {
				hs = append(hs, "err")
				continue
			}
			hs = append(hs, sha16(buf.Bytes()))
			if q == 75 {
				q75 = buf.Bytes()
			}
		}
		// Round trip: decode the q75 output.
		rt := "none"
		if q75 != nil {
			d, err := jpeg.Decode(bytes.NewReader(q75))
			if err != nil {
				rt = "err:" + err.Error()
			} else {
				rt = imgDigest(d)
			}
		}
		// nil options (default quality) and out-of-range qualities.
		var bnil, b0, b200 bytes.Buffer
		_ = jpeg.Encode(&bnil, m, nil)
		_ = jpeg.Encode(&b0, m, &jpeg.Options{Quality: -5})
		_ = jpeg.Encode(&b200, m, &jpeg.Options{Quality: 200})
		_, _ = fmt.Fprintf(out, "%d\t%d\t%s\t%s\t%s\t%s\t%s\t%s\n", seed, kind, rectStr(m.Bounds()),
			strings.Join(hs, ","), rt, sha16(bnil.Bytes()), sha16(b0.Bytes()), sha16(b200.Bytes()))
	}
}

// ---------------------------------------------------------------------------
// Draw.

// slowDst is a draw.Image that implements neither image.RGBA64Image nor
// draw.RGBA64Image, so DrawMask takes its FALLBACK1.0 path.
type slowDst struct{ d draw.Image }

func (s *slowDst) ColorModel() color.Model     { return s.d.ColorModel() }
func (s *slowDst) Bounds() image.Rectangle     { return s.d.Bounds() }
func (s *slowDst) At(x, y int) color.Color     { return s.d.At(x, y) }
func (s *slowDst) Set(x, y int, c color.Color) { s.d.Set(x, y, c) }

// slowSrc is an image.Image that does not implement image.RGBA64Image, so
// drawRGBA and DrawMask take their image.Image (Go 1.0) fallbacks.
type slowSrc struct{ m image.Image }

func (s *slowSrc) ColorModel() color.Model { return s.m.ColorModel() }
func (s *slowSrc) Bounds() image.Rectangle { return s.m.Bounds() }
func (s *slowSrc) At(x, y int) color.Color { return s.m.At(x, y) }

func dstImage(r *rng) draw.Image {
	rect := randRect(r, 24, 8)
	switch r.intn(12) {
	case 0:
		return genImage(r, 0, rect).(draw.Image)
	case 1:
		return genImage(r, 1, rect).(draw.Image)
	case 2:
		return genImage(r, 11, rect).(draw.Image)
	case 3:
		return genImage(r, 10, rect).(draw.Image)
	case 4:
		return genImage(r, 12, rect).(draw.Image)
	case 5:
		return genImage(r, 2, rect).(draw.Image)
	case 6:
		return genImage(r, 13, rect).(draw.Image)
	case 7:
		return genImage(r, 14, rect).(draw.Image)
	case 8:
		return genImage(r, 16, rect).(draw.Image)
	case 9:
		return genImage(r, 9, rect).(draw.Image)
	case 10:
		return &slowDst{genImage(r, 0, rect).(draw.Image)}
	}
	return &slowDst{genImage(r, 12, rect).(draw.Image)}
}

func srcImage(r *rng) image.Image {
	k := r.intn(numKinds + 3)
	if k == numKinds {
		return image.NewUniform(randColor(r))
	}
	if k == numKinds+1 {
		return randRect(r, 30, 10)
	}
	if k == numKinds+2 {
		k2 := r.intn(numKinds)
		return &slowSrc{genImage(r, k2, randRect(r, 30, 10))}
	}
	return genImage(r, k, randRect(r, 30, 10))
}

func maskImage(r *rng) image.Image {
	switch r.intn(8) {
	case 0, 1:
		return nil
	case 2:
		return genImage(r, 14, randRect(r, 30, 10))
	case 3:
		return genImage(r, 16, randRect(r, 30, 10))
	case 4:
		return image.NewUniform(randColor(r))
	case 5:
		return genImage(r, 0, randRect(r, 30, 10))
	case 6:
		return &slowSrc{genImage(r, 14, randRect(r, 30, 10))}
	}
	return genImage(r, r.intn(numKinds), randRect(r, 30, 10))
}

func pixOf(m draw.Image) []byte {
	switch m := m.(type) {
	case *slowDst:
		return pixOf(m.d)
	case *image.RGBA:
		return m.Pix
	case *image.NRGBA:
		return m.Pix
	case *image.NRGBA64:
		return m.Pix
	case *image.RGBA64:
		return m.Pix
	case *image.Paletted:
		return m.Pix
	case *image.Gray:
		return m.Pix
	case *image.Gray16:
		return m.Pix
	case *image.Alpha:
		return m.Pix
	case *image.Alpha16:
		return m.Pix
	case *image.CMYK:
		return m.Pix
	}
	panic("pixOf")
}

func drawOps(n int) {
	s0 := seedOffset()
	for seed := s0; seed < s0+n; seed++ {
		r := &rng{uint64(seed)*7919 + 3}
		dst := dstImage(r)
		src := srcImage(r)
		mask := maskImage(r)
		db := dst.Bounds()
		rr := image.Rect(db.Min.X-4+r.intn(12), db.Min.Y-4+r.intn(12), db.Max.X-6+r.intn(12), db.Max.Y-6+r.intn(12))
		sp := image.Pt(r.intn(41)-20, r.intn(41)-20)
		mp := image.Pt(r.intn(41)-20, r.intn(41)-20)
		op := draw.Op(r.intn(2))
		how := r.intn(5)
		switch how {
		case 0:
			draw.FloydSteinberg.Draw(dst, rr, src, sp)
		case 1:
			draw.Draw(dst, rr, src, sp, op)
		default:
			draw.DrawMask(dst, rr, src, sp, mask, mp, op)
		}
		_, _ = fmt.Fprintf(out, "%d\t%T\t%T\t%s\t%d\t%s\n", seed, dst, src, rectStr(dst.Bounds()), how, sha16(pixOf(dst)))
	}
}

// ---------------------------------------------------------------------------
// Colour conversions.

// hasher streams little-endian values into SHA-256.
type hasher struct {
	h   hash.Hash
	buf []byte
}

func newHasher() *hasher { return &hasher{h: sha256.New(), buf: make([]byte, 0, 1<<16)} }

func (h *hasher) flushIfFull() {
	if len(h.buf) >= 1<<16-16 {
		h.h.Write(h.buf)
		h.buf = h.buf[:0]
	}
}

func (h *hasher) u8(v uint8) {
	h.buf = append(h.buf, v)
	h.flushIfFull()
}

func (h *hasher) u16(v uint16) {
	h.buf = binary.LittleEndian.AppendUint16(h.buf, v)
	h.flushIfFull()
}

func (h *hasher) u32(v uint32) {
	h.buf = binary.LittleEndian.AppendUint32(h.buf, v)
	h.flushIfFull()
}

func (h *hasher) sum() string {
	h.h.Write(h.buf)
	h.buf = h.buf[:0]
	return hex.EncodeToString(h.h.Sum(nil))
}

func colorDigest(h *hasher, c color.Color) {
	switch c := c.(type) {
	case color.RGBA:
		h.u8(0)
		h.u8(c.R)
		h.u8(c.G)
		h.u8(c.B)
		h.u8(c.A)
	case color.RGBA64:
		h.u8(1)
		h.u16(c.R)
		h.u16(c.G)
		h.u16(c.B)
		h.u16(c.A)
	case color.NRGBA:
		h.u8(2)
		h.u8(c.R)
		h.u8(c.G)
		h.u8(c.B)
		h.u8(c.A)
	case color.NRGBA64:
		h.u8(3)
		h.u16(c.R)
		h.u16(c.G)
		h.u16(c.B)
		h.u16(c.A)
	case color.Alpha:
		h.u8(4)
		h.u8(c.A)
	case color.Alpha16:
		h.u8(5)
		h.u16(c.A)
	case color.Gray:
		h.u8(6)
		h.u8(c.Y)
	case color.Gray16:
		h.u8(7)
		h.u16(c.Y)
	case color.YCbCr:
		h.u8(8)
		h.u8(c.Y)
		h.u8(c.Cb)
		h.u8(c.Cr)
	case color.NYCbCrA:
		h.u8(9)
		h.u8(c.Y)
		h.u8(c.Cb)
		h.u8(c.Cr)
		h.u8(c.A)
	case color.CMYK:
		h.u8(10)
		h.u8(c.C)
		h.u8(c.M)
		h.u8(c.Y)
		h.u8(c.K)
	default:
		panic(fmt.Sprintf("colorDigest %T", c))
	}
}

func rgbaDigest(h *hasher, c color.Color) {
	r, g, b, a := c.RGBA()
	h.u32(r)
	h.u32(g)
	h.u32(b)
	h.u32(a)
}

func colorSpace() {
	emit := func(name string, h *hasher) { _, _ = fmt.Fprintf(out, "%s\t%s\n", name, h.sum()) }

	h := newHasher()
	for i := 0; i < 1<<24; i++ {
		y, cb, cr := color.RGBToYCbCr(uint8(i>>16), uint8(i>>8), uint8(i))
		h.u8(y)
		h.u8(cb)
		h.u8(cr)
	}
	emit("RGBToYCbCr", h)

	h = newHasher()
	for i := 0; i < 1<<24; i++ {
		r, g, b := color.YCbCrToRGB(uint8(i>>16), uint8(i>>8), uint8(i))
		h.u8(r)
		h.u8(g)
		h.u8(b)
	}
	emit("YCbCrToRGB", h)

	h = newHasher()
	for i := 0; i < 1<<24; i++ {
		rgbaDigest(h, color.YCbCr{uint8(i >> 16), uint8(i >> 8), uint8(i)})
	}
	emit("YCbCr.RGBA", h)

	h = newHasher()
	for i := 0; i < 1<<24; i++ {
		c, m, y, k := color.RGBToCMYK(uint8(i>>16), uint8(i>>8), uint8(i))
		h.u8(c)
		h.u8(m)
		h.u8(y)
		h.u8(k)
	}
	emit("RGBToCMYK", h)

	for _, k := range []uint8{0, 1, 77, 128, 254, 255} {
		h = newHasher()
		for i := 0; i < 1<<24; i++ {
			r, g, b := color.CMYKToRGB(uint8(i>>16), uint8(i>>8), uint8(i), k)
			h.u8(r)
			h.u8(g)
			h.u8(b)
			rgbaDigest(h, color.CMYK{uint8(i >> 16), uint8(i >> 8), uint8(i), k})
		}
		emit(fmt.Sprintf("CMYKToRGB.k%d", k), h)
	}

	for _, a := range []uint8{0, 1, 127, 128, 255} {
		h = newHasher()
		for i := 0; i < 1<<24; i += 7 {
			rgbaDigest(h, color.NYCbCrA{color.YCbCr{uint8(i >> 16), uint8(i >> 8), uint8(i)}, a})
		}
		emit(fmt.Sprintf("NYCbCrA.RGBA.a%d", a), h)
	}

	// Per-channel functions of (c, a): exhaustive over 2^16.
	h = newHasher()
	for c := 0; c < 256; c++ {
		for a := 0; a < 256; a++ {
			rgbaDigest(h, color.NRGBA{uint8(c), uint8(255 - c), uint8(c ^ 0x5a), uint8(a)})
			rgbaDigest(h, color.RGBA{uint8(c), uint8(255 - c), uint8(c ^ 0x5a), uint8(a)})
			rgbaDigest(h, color.Alpha{uint8(a)})
			rgbaDigest(h, color.Gray{uint8(c)})
		}
	}
	emit("8bit.RGBA", h)

	// All models over RGBA / NRGBA inputs (c, a) exhaustive.
	models := []color.Model{color.RGBAModel, color.RGBA64Model, color.NRGBAModel, color.NRGBA64Model,
		color.AlphaModel, color.Alpha16Model, color.GrayModel, color.Gray16Model, color.YCbCrModel,
		color.NYCbCrAModel, color.CMYKModel}
	for mi, m := range models {
		h = newHasher()
		for c := 0; c < 256; c++ {
			for a := 0; a < 256; a++ {
				colorDigest(h, m.Convert(color.RGBA{uint8(c), uint8(255 - c), uint8(c ^ 0x5a), uint8(a)}))
				colorDigest(h, m.Convert(color.NRGBA{uint8(c), uint8(255 - c), uint8(c ^ 0x5a), uint8(a)}))
			}
		}
		emit(fmt.Sprintf("model%d.8bit", mi), h)
	}

	// All models over random colours of every type.
	for mi, m := range models {
		h = newHasher()
		r := &rng{uint64(mi) + 99}
		for i := 0; i < 200000; i++ {
			c := randColor(r)
			colorDigest(h, m.Convert(c))
			rgbaDigest(h, c)
		}
		emit(fmt.Sprintf("model%d.random", mi), h)
	}

	// Gray models over the full RGB cube (opaque).
	h = newHasher()
	for i := 0; i < 1<<24; i++ {
		c := color.RGBA{uint8(i >> 16), uint8(i >> 8), uint8(i), 0xff}
		colorDigest(h, color.GrayModel.Convert(c))
		colorDigest(h, color.Gray16Model.Convert(c))
		colorDigest(h, color.YCbCrModel.Convert(c))
	}
	emit("GrayYCbCrModels.cube", h)

	// NRGBA64 (c, a) sampled.
	h = newHasher()
	r := &rng{7}
	for i := 0; i < 1000000; i++ {
		c := color.NRGBA64{r.u16(), r.u16(), r.u16(), r.u16()}
		rgbaDigest(h, c)
		colorDigest(h, color.NRGBA64Model.Convert(color.RGBA64(c)))
		colorDigest(h, color.NRGBAModel.Convert(color.RGBA64(c)))
	}
	emit("NRGBA64.random", h)

	// Palette.Index / Convert.
	for pi, p := range []color.Palette{palette.Plan9, palette.WebSafe} {
		h = newHasher()
		r := &rng{uint64(pi) + 5}
		for i := 0; i < 100000; i++ {
			c := randColor(r)
			h.u32(uint32(p.Index(c)))
			colorDigest(h, p.Convert(c))
		}
		emit(fmt.Sprintf("palette%d.index", pi), h)
	}
	h = newHasher()
	r = &rng{11}
	for i := 0; i < 20000; i++ {
		p := randPalette(r)
		c := randColor(r)
		h.u32(uint32(p.Index(c)))
	}
	emit("randpalette.index", h)
}

// ---------------------------------------------------------------------------
// JPEG fuzzing.

func fuzz(n int, files []string) {
	var bases [][]byte
	for _, f := range files {
		b, err := os.ReadFile(f)
		if err != nil {
			panic(err)
		}
		bases = append(bases, b)
	}
	s0 := seedOffset()
	for seed := s0; seed < s0+n; seed++ {
		r := &rng{uint64(seed)*31 + 1}
		bi := r.intn(len(bases))
		b := append([]byte(nil), bases[bi]...)
		nm := 1 + r.intn(4)
		for k := 0; k < nm; k++ {
			if len(b) == 0 {
				break
			}
			switch r.intn(5) {
			case 0:
				i := r.intn(len(b))
				b[i] ^= byte(1 + r.intn(255))
			case 1:
				i := r.intn(len(b))
				b[i] = 0xff
			case 2:
				b = b[:r.intn(len(b))]
			case 3:
				i := r.intn(len(b))
				m := r.intn(16)
				if i+m > len(b) {
					m = len(b) - i
				}
				b = append(b[:i], b[i+m:]...)
			case 4:
				i := r.intn(len(b) + 1)
				m := 1 + r.intn(4)
				ins := make([]byte, m)
				for j := range ins {
					ins[j] = r.byte()
				}
				b = append(b[:i], append(ins, b[i:]...)...)
			}
		}
		cfgStr := ""
		cfg, err := jpeg.DecodeConfig(bytes.NewReader(b))
		if err != nil {
			cfgStr = "err:" + err.Error()
		} else {
			cfgStr = fmt.Sprintf("%s,%d,%d", modelName(cfg.ColorModel), cfg.Width, cfg.Height)
			if cfg.Width*cfg.Height > 4<<20 {
				_, _ = fmt.Fprintf(out, "%d\t%d\t%s\t%s\tskip\n", seed, bi, sha16(b), cfgStr)
				continue
			}
		}
		res := ""
		m, err := jpeg.Decode(bytes.NewReader(b))
		if err != nil {
			res = "err:" + err.Error()
		} else {
			res = imgDigest(m)
		}
		_, _ = fmt.Fprintf(out, "%d\t%d\t%s\t%s\t%s\n", seed, bi, sha16(b), cfgStr, res)
	}
}

// ---------------------------------------------------------------------------
// DCT.

func dctBlocks(n int) {
	s0 := seedOffset()
	for seed := s0; seed < s0+n; seed++ {
		r := &rng{uint64(seed)*101 + 7}
		var b block
		class := r.intn(5)
		for i := range b {
			switch class {
			case 0: // FDCT domain: pixels.
				b[i] = int32(r.byte())
			case 1: // Typical dequantized coefficients, sparse.
				if r.intn(4) == 0 {
					b[i] = int32(r.intn(4096)) - 2048
				}
			case 2: // Large coefficients (quant up to 65535 * coef up to 32767).
				b[i] = int32(r.intn(1<<24)) - 1<<23
			case 3: // Arbitrary int32 (wrapping).
				b[i] = int32(uint32(r.next()))
			default: // Small coefficients.
				b[i] = int32(r.intn(64)) - 32
			}
		}
		f, iv := b, b
		fdct(&f)
		idct(&iv)
		var fb, ib []byte
		for i := range f {
			fb = binary.LittleEndian.AppendUint32(fb, uint32(f[i]))
			ib = binary.LittleEndian.AppendUint32(ib, uint32(iv[i]))
		}
		_, _ = fmt.Fprintf(out, "%d\t%d\t%s\t%s\n", seed, class, sha16(fb), sha16(ib))
	}
}
