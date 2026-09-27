package main

// Deterministic image generation and spec strings shared with the Rust tests
// (crates/gift/tests/common/mod.rs re-implements genImage call for call).

import (
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"image"
	"image/color"
	"image/draw"
	"math"
	"strconv"
	"strings"

	"github.com/disintegration/gift"
)

// Filter is gift.Filter.
type Filter = gift.Filter

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

// genVal returns a channel byte. vmode 0: uniform; 1: biased to edge values.
func genVal(r *rng, vmode int) byte {
	if vmode == 0 {
		return r.byte()
	}
	switch r.intn(8) {
	case 0:
		return 0
	case 1:
		return 1
	case 2:
		return 127
	case 3:
		return 128
	case 4:
		return 254
	case 5:
		return 255
	default:
		return r.byte()
	}
}

// genAlpha8 returns an alpha byte. amode 0: uniform; 1: opaque; 2: mixed.
func genAlpha8(r *rng, amode int) byte {
	switch amode {
	case 1:
		return 0xff
	case 2:
		switch r.intn(3) {
		case 0:
			return 0
		case 1:
			return 0xff
		default:
			return r.byte()
		}
	default:
		return r.byte()
	}
}

func genAlpha16(r *rng, amode int) uint16 {
	switch amode {
	case 1:
		return 0xffff
	case 2:
		switch r.intn(3) {
		case 0:
			return 0
		case 1:
			return 0xffff
		default:
			hi := r.byte()
			lo := r.byte()
			return uint16(hi)<<8 | uint16(lo)
		}
	default:
		hi := r.byte()
		lo := r.byte()
		return uint16(hi)<<8 | uint16(lo)
	}
}

func genVal16(r *rng, vmode int) uint16 {
	hi := genVal(r, vmode)
	lo := genVal(r, vmode)
	return uint16(hi)<<8 | uint16(lo)
}

// genColor returns a palette colour. valid: premultiplied colours are valid.
func genColor(r *rng, vmode, amode int, valid bool) color.Color {
	switch r.intn(6) {
	case 0:
		c := color.RGBA{genVal(r, vmode), genVal(r, vmode), genVal(r, vmode), genAlpha8(r, amode)}
		if valid {
			c.R = min(c.R, c.A)
			c.G = min(c.G, c.A)
			c.B = min(c.B, c.A)
		}
		return c
	case 1:
		return color.NRGBA{genVal(r, vmode), genVal(r, vmode), genVal(r, vmode), genAlpha8(r, amode)}
	case 2:
		return color.Gray{genVal(r, vmode)}
	case 3:
		c := color.RGBA64{genVal16(r, vmode), genVal16(r, vmode), genVal16(r, vmode), genAlpha16(r, amode)}
		if valid {
			c.R = min(c.R, c.A)
			c.G = min(c.G, c.A)
			c.B = min(c.B, c.A)
		}
		return c
	case 4:
		return color.NRGBA64{genVal16(r, vmode), genVal16(r, vmode), genVal16(r, vmode), genAlpha16(r, amode)}
	default:
		return color.Alpha{genAlpha8(r, amode)}
	}
}

func genPalette(r *rng, vmode, amode int, valid bool) color.Palette {
	var n int
	switch r.intn(3) {
	case 0:
		n = 1 + r.intn(2)
	case 1:
		n = 1 + r.intn(16)
	default:
		n = 1 + r.intn(256)
	}
	p := make(color.Palette, n)
	for i := range p {
		p[i] = genColor(r, vmode, amode, valid)
	}
	return p
}

// imgSpec describes a generated image.
type imgSpec struct {
	typ   string
	rect  image.Rectangle
	seed  uint64
	vmode int
	amode int
	valid bool
	blank bool // only allocate (NewXxx), no content; paletted still gets a palette
}

func (s imgSpec) String() string {
	v := 0
	if s.valid {
		v = 1
	}
	b := 0
	if s.blank {
		b = 1
	}
	return fmt.Sprintf("%s:%d,%d,%d,%d:%d:%d:%d:%d:%d", s.typ, s.rect.Min.X, s.rect.Min.Y, s.rect.Max.X, s.rect.Max.Y, s.seed, s.vmode, s.amode, v, b)
}

func parseImgSpec(str string) imgSpec {
	f := strings.Split(str, ":")
	if len(f) != 7 {
		panic("bad img spec " + str)
	}
	var s imgSpec
	s.typ = f[0]
	var r [4]int
	for i, v := range strings.Split(f[1], ",") {
		r[i], _ = strconv.Atoi(v)
	}
	s.rect = image.Rect(r[0], r[1], r[2], r[3])
	s.seed, _ = strconv.ParseUint(f[2], 10, 64)
	s.vmode, _ = strconv.Atoi(f[3])
	s.amode, _ = strconv.Atoi(f[4])
	s.valid = f[5] == "1"
	s.blank = f[6] == "1"
	return s
}

func ycbcrRatio(typ string) image.YCbCrSubsampleRatio {
	switch typ[len(typ)-3:] {
	case "444":
		return image.YCbCrSubsampleRatio444
	case "422":
		return image.YCbCrSubsampleRatio422
	case "420":
		return image.YCbCrSubsampleRatio420
	case "440":
		return image.YCbCrSubsampleRatio440
	case "411":
		return image.YCbCrSubsampleRatio411
	case "410":
		return image.YCbCrSubsampleRatio410
	}
	panic(typ)
}

func fill8(r *rng, pix []byte, vmode int) {
	for i := range pix {
		pix[i] = genVal(r, vmode)
	}
}

// fillRGBA8 fills 4-byte pixels; alpha is the last byte; premul clamps c<=a.
func fillRGBA8(r *rng, pix []byte, s imgSpec, premul bool) {
	for i := 0; i+3 < len(pix); i += 4 {
		pix[i+0] = genVal(r, s.vmode)
		pix[i+1] = genVal(r, s.vmode)
		pix[i+2] = genVal(r, s.vmode)
		a := genAlpha8(r, s.amode)
		pix[i+3] = a
		if premul && s.valid {
			pix[i+0] = min(pix[i+0], a)
			pix[i+1] = min(pix[i+1], a)
			pix[i+2] = min(pix[i+2], a)
		}
	}
}

func fillRGBA16(r *rng, pix []byte, s imgSpec, premul bool) {
	for i := 0; i+7 < len(pix); i += 8 {
		var c [3]uint16
		for k := 0; k < 3; k++ {
			c[k] = genVal16(r, s.vmode)
		}
		a := genAlpha16(r, s.amode)
		if premul && s.valid {
			for k := 0; k < 3; k++ {
				c[k] = min(c[k], a)
			}
		}
		for k := 0; k < 3; k++ {
			pix[i+2*k] = byte(c[k] >> 8)
			pix[i+2*k+1] = byte(c[k])
		}
		pix[i+6] = byte(a >> 8)
		pix[i+7] = byte(a)
	}
}

func genImage(s imgSpec) image.Image {
	r := &rng{s: s.seed}
	switch s.typ {
	case "nrgba":
		m := image.NewNRGBA(s.rect)
		if !s.blank {
			fillRGBA8(r, m.Pix, s, false)
		}
		return m
	case "rgba":
		m := image.NewRGBA(s.rect)
		if !s.blank {
			fillRGBA8(r, m.Pix, s, true)
		}
		return m
	case "nrgba64":
		m := image.NewNRGBA64(s.rect)
		if !s.blank {
			fillRGBA16(r, m.Pix, s, false)
		}
		return m
	case "rgba64":
		m := image.NewRGBA64(s.rect)
		if !s.blank {
			fillRGBA16(r, m.Pix, s, true)
		}
		return m
	case "gray":
		m := image.NewGray(s.rect)
		if !s.blank {
			fill8(r, m.Pix, s.vmode)
		}
		return m
	case "gray16":
		m := image.NewGray16(s.rect)
		if !s.blank {
			fill8(r, m.Pix, s.vmode)
		}
		return m
	case "cmyk":
		m := image.NewCMYK(s.rect)
		if !s.blank {
			fill8(r, m.Pix, s.vmode)
		}
		return m
	case "alpha":
		m := image.NewAlpha(s.rect)
		if !s.blank {
			for i := range m.Pix {
				m.Pix[i] = genAlpha8(r, s.amode)
			}
		}
		return m
	case "alpha16":
		m := image.NewAlpha16(s.rect)
		if !s.blank {
			for i := 0; i+1 < len(m.Pix); i += 2 {
				a := genAlpha16(r, s.amode)
				m.Pix[i] = byte(a >> 8)
				m.Pix[i+1] = byte(a)
			}
		}
		return m
	case "paletted":
		p := genPalette(r, s.vmode, s.amode, s.valid)
		m := image.NewPaletted(s.rect, p)
		if !s.blank {
			for i := range m.Pix {
				m.Pix[i] = byte(r.intn(len(p)))
			}
		}
		return m
	case "gray16all":
		// Every 16-bit value in order (setterCases).
		m := image.NewGray16(s.rect)
		for i := 0; i+1 < len(m.Pix); i += 2 {
			v := (i / 2) & 0xffff
			m.Pix[i] = byte(v >> 8)
			m.Pix[i+1] = byte(v)
		}
		return m
	case "ycbcr444", "ycbcr422", "ycbcr420", "ycbcr440", "ycbcr411", "ycbcr410":
		m := image.NewYCbCr(s.rect, ycbcrRatio(s.typ))
		if !s.blank {
			fill8(r, m.Y, s.vmode)
			fill8(r, m.Cb, s.vmode)
			fill8(r, m.Cr, s.vmode)
		}
		return m
	case "nycbcra444", "nycbcra420":
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
	}
	panic("unknown image type " + s.typ)
}

// digest returns the SHA-256 (hex) of the image's pixel buffers.
func digest(img image.Image) string {
	h := sha256.New()
	switch m := img.(type) {
	case *image.NRGBA:
		h.Write(m.Pix)
	case *image.NRGBA64:
		h.Write(m.Pix)
	case *image.RGBA:
		h.Write(m.Pix)
	case *image.RGBA64:
		h.Write(m.Pix)
	case *image.Gray:
		h.Write(m.Pix)
	case *image.Gray16:
		h.Write(m.Pix)
	case *image.CMYK:
		h.Write(m.Pix)
	case *image.Alpha:
		h.Write(m.Pix)
	case *image.Alpha16:
		h.Write(m.Pix)
	case *image.Paletted:
		h.Write(m.Pix)
	default:
		panic(fmt.Sprintf("digest: %T", img))
	}
	fmt.Fprintf(h, "|%d,%d,%d,%d", img.Bounds().Min.X, img.Bounds().Min.Y, img.Bounds().Max.X, img.Bounds().Max.Y)
	s := h.Sum(nil)
	return hex.EncodeToString(s[:])
}

// ---------------------------------------------------------------------------
// Float parameters are printed as the hex bits of the float32 value.

func fstr(f float32) string { return fmt.Sprintf("f%08x", math.Float32bits(f)) }

func pf(s string) float32 {
	if !strings.HasPrefix(s, "f") {
		panic("bad float " + s)
	}
	u, err := strconv.ParseUint(s[1:], 16, 32)
	if err != nil {
		panic(err)
	}
	return math.Float32frombits(uint32(u))
}

func pi(s string) int {
	v, err := strconv.Atoi(s)
	if err != nil {
		panic(err)
	}
	return v
}

// colorSpec: rgba/r/g/b/a, nrgba/r/g/b/a, gray/y, rgba64/r/g/b/a, nrgba64/r/g/b/a, alpha/a.
func colorStr(c color.Color) string {
	switch c := c.(type) {
	case color.RGBA:
		return fmt.Sprintf("rgba/%d/%d/%d/%d", c.R, c.G, c.B, c.A)
	case color.NRGBA:
		return fmt.Sprintf("nrgba/%d/%d/%d/%d", c.R, c.G, c.B, c.A)
	case color.Gray:
		return fmt.Sprintf("gray/%d", c.Y)
	case color.RGBA64:
		return fmt.Sprintf("rgba64/%d/%d/%d/%d", c.R, c.G, c.B, c.A)
	case color.NRGBA64:
		return fmt.Sprintf("nrgba64/%d/%d/%d/%d", c.R, c.G, c.B, c.A)
	case color.Alpha:
		return fmt.Sprintf("alpha/%d", c.A)
	}
	panic(fmt.Sprintf("colorStr %T", c))
}

func parseColor(s string) color.Color {
	f := strings.Split(s, "/")
	u := func(i int) int { return pi(f[i]) }
	switch f[0] {
	case "rgba":
		return color.RGBA{uint8(u(1)), uint8(u(2)), uint8(u(3)), uint8(u(4))}
	case "nrgba":
		return color.NRGBA{uint8(u(1)), uint8(u(2)), uint8(u(3)), uint8(u(4))}
	case "gray":
		return color.Gray{uint8(u(1))}
	case "rgba64":
		return color.RGBA64{uint16(u(1)), uint16(u(2)), uint16(u(3)), uint16(u(4))}
	case "nrgba64":
		return color.NRGBA64{uint16(u(1)), uint16(u(2)), uint16(u(3)), uint16(u(4))}
	case "alpha":
		return color.Alpha{uint8(u(1))}
	}
	panic("bad color " + s)
}

// colorFuncs are the ColorFunc callbacks the Rust tests mirror.
var colorFuncs = []func(r0, g0, b0, a0 float32) (r, g, b, a float32){
	func(r0, g0, b0, a0 float32) (r, g, b, a float32) {
		// gift_test.go TestGolden "color_func"
		r = 1 - r0
		g = g0 + 0.1
		b = 0
		a = a0
		return r, g, b, a
	},
	func(r0, g0, b0, a0 float32) (r, g, b, a float32) {
		return b0, r0, g0, a0 / 2
	},
	func(r0, g0, b0, a0 float32) (r, g, b, a float32) {
		return r0 + g0, g0 - b0, -b0, a0 + 0.25
	},
}

// parseFilter parses one filter spec: name(arg,arg,...).
func parseFilter(s string) Filter {
	open := strings.IndexByte(s, '(')
	if open < 0 || !strings.HasSuffix(s, ")") {
		panic("bad filter " + s)
	}
	name := s[:open]
	var a []string
	if inner := s[open+1 : len(s)-1]; inner != "" {
		a = strings.Split(inner, ",")
	}
	b := func(i int) bool { return a[i] == "1" }
	switch name {
	case "resize":
		return gift.Resize(pi(a[0]), pi(a[1]), resampling(a[2]))
	case "resizetofit":
		return gift.ResizeToFit(pi(a[0]), pi(a[1]), resampling(a[2]))
	case "resizetofill":
		return gift.ResizeToFill(pi(a[0]), pi(a[1]), resampling(a[2]), gift.Anchor(pi(a[3])))
	case "crop":
		return gift.Crop(image.Rect(pi(a[0]), pi(a[1]), pi(a[2]), pi(a[3])))
	case "croptosize":
		return gift.CropToSize(pi(a[0]), pi(a[1]), gift.Anchor(pi(a[2])))
	case "rotate90":
		return gift.Rotate90()
	case "rotate180":
		return gift.Rotate180()
	case "rotate270":
		return gift.Rotate270()
	case "fliph":
		return gift.FlipHorizontal()
	case "flipv":
		return gift.FlipVertical()
	case "transpose":
		return gift.Transpose()
	case "transverse":
		return gift.Transverse()
	case "rotate":
		return gift.Rotate(pf(a[0]), parseColor(a[1]), gift.Interpolation(pi(a[2])))
	case "invert":
		return gift.Invert()
	case "srgbtolinear":
		return gift.ColorspaceSRGBToLinear()
	case "lineartosrgb":
		return gift.ColorspaceLinearToSRGB()
	case "gamma":
		return gift.Gamma(pf(a[0]))
	case "sigmoid":
		return gift.Sigmoid(pf(a[0]), pf(a[1]))
	case "contrast":
		return gift.Contrast(pf(a[0]))
	case "brightness":
		return gift.Brightness(pf(a[0]))
	case "grayscale":
		return gift.Grayscale()
	case "sepia":
		return gift.Sepia(pf(a[0]))
	case "hue":
		return gift.Hue(pf(a[0]))
	case "saturation":
		return gift.Saturation(pf(a[0]))
	case "colorize":
		return gift.Colorize(pf(a[0]), pf(a[1]), pf(a[2]))
	case "colorbalance":
		return gift.ColorBalance(pf(a[0]), pf(a[1]), pf(a[2]))
	case "threshold":
		return gift.Threshold(pf(a[0]))
	case "colorfunc":
		return gift.ColorFunc(colorFuncs[pi(a[0])])
	case "convolution":
		var k []float32
		if a[0] != "" {
			for _, v := range strings.Split(a[0], "/") {
				k = append(k, pf(v))
			}
		}
		return gift.Convolution(k, b(1), b(2), b(3), pf(a[4]))
	case "gaussianblur":
		return gift.GaussianBlur(pf(a[0]))
	case "unsharpmask":
		return gift.UnsharpMask(pf(a[0]), pf(a[1]), pf(a[2]))
	case "mean":
		return gift.Mean(pi(a[0]), b(1))
	case "sobel":
		return gift.Sobel()
	case "median":
		return gift.Median(pi(a[0]), b(1))
	case "minimum":
		return gift.Minimum(pi(a[0]), b(1))
	case "maximum":
		return gift.Maximum(pi(a[0]), b(1))
	case "pixelate":
		return gift.Pixelate(pi(a[0]))
	case "copy":
		return copyFilter{}
	case "overlay":
		// overlay(<imgspec with ; for :>,x,y): Hugo's overlayFilter.
		return overlayFilter{src: genImage(parseImgSpec(strings.ReplaceAll(a[0], ";", ":"))), x: pi(a[1]), y: pi(a[2])}
	}
	panic("unknown filter " + name)
}

func parseFilters(s string) []Filter {
	if s == "" || s == "-" {
		return nil
	}
	var fs []Filter
	for _, p := range strings.Split(s, "|") {
		fs = append(fs, parseFilter(p))
	}
	return fs
}

// copyFilter draws the source unchanged through gift.New().Draw (copyimage).
type copyFilter struct{}

func (copyFilter) Draw(dst draw.Image, src image.Image, options *gift.Options) {
	gift.New().Draw(dst, src)
}

func (copyFilter) Bounds(b image.Rectangle) image.Rectangle {
	return image.Rect(0, 0, b.Dx(), b.Dy())
}

// overlayFilter is neohugo resources/images/overlay.go:overlayFilter with an
// in-memory overlay image.
type overlayFilter struct {
	src  image.Image
	x, y int
}

func (f overlayFilter) Draw(dst draw.Image, src image.Image, options *gift.Options) {
	gift.New().Draw(dst, src)
	gift.New().DrawAt(dst, f.src, image.Pt(f.x, f.y), gift.OverOperator)
}

func (f overlayFilter) Bounds(srcBounds image.Rectangle) image.Rectangle {
	return image.Rect(0, 0, srcBounds.Dx(), srcBounds.Dy())
}
