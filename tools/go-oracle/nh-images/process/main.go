// Command process is the Go oracle for the image pipeline of crates/nh-images (Wave B task
// T10): image.Decode of a source, DecodeImageConfig + ApplyFiltersFromConfig (Resize, Fit,
// Fill, Crop, rotations, format conversions), ImageProcessor.Filter with the images.* filters,
// and Image.EncodeTo (JPEG, PNG, WebP through cgo libwebp), including the chains of the
// seeksnack templates (resize, watermark overlay decoded from its encoded PNG, webp) with the
// encode/decode round trips between the steps.
//
// The sources are the raster images of this repository and synthetic images (every Go image
// type the PNG and JPEG decoders return, sizes from 1x1 to 2048x1536) whose pixels come from
// a splitmix64 generator that the Rust test reproduces. Every output is recorded as its
// SHA-256 (plus type, bounds and length); the outputs of the small synthetic sources are also
// stored in full (bytes.json.gz).
//
// gift's resampling and blending, and the flate encoder behind PNG, fuse float operations on
// arm64, and the golden build is darwin/arm64: run the linux/arm64 build under
// qemu-aarch64-static (tools/go-oracle/nh-images/regen.sh).
package main

import (
	"bytes"
	"crypto/sha256"
	"encoding/base64"
	"encoding/hex"
	"flag"
	"fmt"
	"image"
	"image/color"
	"image/jpeg"
	"image/png"
	"log"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"github.com/disintegration/gift"
	"github.com/neohugo/neohugo/common/hashing"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/resources/images"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"

	_ "golang.org/x/image/webp"
)

type ns = config.ConfigNamespace[images.ImagingConfig, images.ImagingConfigInternal]

var (
	root  string
	cfgs  map[string]*ns
	procs map[string]*images.ImageProcessor
)

func main() {
	out := flag.String("out", "crates/nh-images/tests/fixtures/process", "output directory")
	flag.StringVar(&root, "root", ".", "repository root")
	only := flag.String("only", "", "only sources whose id contains this (debugging; do not use for fixtures)")
	flag.Parse()

	cfgs = map[string]*ns{}
	procs = map[string]*images.ImageProcessor{}
	for name, m := range map[string]map[string]any{
		"d":  {},
		"s":  {"_merge": "none", "exif": map[string]any{"_merge": "none", "excludefields": ".*"}},
		"q0": {"quality": 0},
		"lz": {"resampleFilter": "lanczos", "anchor": "topleft", "bgColor": "#abc123"},
	} {
		c, err := images.DecodeConfig(m)
		if err != nil {
			log.Fatal(err)
		}
		cfgs[name] = c
		p, err := images.NewImageProcessor(nil, c)
		if err != nil {
			log.Fatal(err)
		}
		procs[name] = p
	}

	wm = encodeSynth("nrgbaa", 600, 480, "png")
	maskBytes = mustRead("resources/testdata/mask.png")

	var cases []map[string]any
	var full []map[string]any
	sources := allSources()
	for si, src := range sources {
		if *only != "" && !strings.Contains(src.id, *only) {
			continue
		}
		c, f := runSource(si, src)
		cases = append(cases, c...)
		full = append(full, f...)
		log.Printf("%d/%d %s: %d ops", si+1, len(sources), src.id, len(c))
	}
	header := map[string]any{"oracle": "nh-images/process", "wmsha": sha(wm)}
	if err := goval.WriteCasesGz(filepath.Join(*out, "process.json.gz"), header, cases); err != nil {
		log.Fatal(err)
	}
	if err := goval.WriteCasesGz(filepath.Join(*out, "bytes.json.gz"), map[string]any{}, full); err != nil {
		log.Fatal(err)
	}
}

func mustRead(rel string) []byte {
	b, err := os.ReadFile(filepath.Join(root, rel))
	if err != nil {
		log.Fatal(err)
	}
	return b
}

func sha(b []byte) string {
	h := sha256.Sum256(b)
	return hex.EncodeToString(h[:])
}

// ---------------------------------------------------------------------------
// Synthetic images

func splitmix64(x uint64) uint64 {
	x += 0x9e3779b97f4a7c15
	z := x
	z = (z ^ (z >> 30)) * 0xbf58476d1ce4e5b9
	z = (z ^ (z >> 27)) * 0x94d049bb133111eb
	return z ^ (z >> 31)
}

// pix returns the 8 generator bytes of pixel (x, y): a gradient plus a little noise.
func pix(seed uint64, w, x, y int) [8]uint8 {
	h := splitmix64(seed*1000003 + uint64(y)*uint64(w) + uint64(x))
	var v [8]uint8
	for c := 0; c < 8; c++ {
		v[c] = uint8(x*(c+1)*3 + y*(c+2)*5 + int((h>>(8*c))&0x1f))
	}
	return v
}

func kindSeed(kind string) uint64 {
	var s uint64
	for _, c := range kind {
		s = s*131 + uint64(c)
	}
	return s
}

func alpha(v uint8) uint8 {
	if v < 40 {
		return 0
	}
	if v > 200 {
		return 255
	}
	return v
}

func synth(kind string, w, h int) image.Image {
	r := image.Rect(0, 0, w, h)
	seed := kindSeed(kind) + uint64(w)*7919 + uint64(h)
	switch kind {
	case "nrgba", "nrgbaa":
		m := image.NewNRGBA(r)
		for y := 0; y < h; y++ {
			for x := 0; x < w; x++ {
				v := pix(seed, w, x, y)
				a := uint8(255)
				if kind == "nrgbaa" {
					a = alpha(v[3])
				}
				m.SetNRGBA(x, y, color.NRGBA{v[0], v[1], v[2], a})
			}
		}
		return m
	case "rgba":
		m := image.NewRGBA(r)
		for y := 0; y < h; y++ {
			for x := 0; x < w; x++ {
				v := pix(seed, w, x, y)
				m.SetRGBA(x, y, color.RGBA{v[0], v[1], v[2], 255})
			}
		}
		return m
	case "gray":
		m := image.NewGray(r)
		for y := 0; y < h; y++ {
			for x := 0; x < w; x++ {
				m.SetGray(x, y, color.Gray{pix(seed, w, x, y)[0]})
			}
		}
		return m
	case "gray16":
		m := image.NewGray16(r)
		for y := 0; y < h; y++ {
			for x := 0; x < w; x++ {
				v := pix(seed, w, x, y)
				m.SetGray16(x, y, color.Gray16{uint16(v[0])<<8 | uint16(v[1])})
			}
		}
		return m
	case "nrgba64":
		m := image.NewNRGBA64(r)
		for y := 0; y < h; y++ {
			for x := 0; x < w; x++ {
				v := pix(seed, w, x, y)
				m.SetNRGBA64(x, y, color.NRGBA64{uint16(v[0])<<8 | uint16(v[4]), uint16(v[1])<<8 | uint16(v[5]), uint16(v[2])<<8 | uint16(v[6]), uint16(alpha(v[3]))<<8 | uint16(v[7])})
			}
		}
		return m
	case "rgba64":
		m := image.NewRGBA64(r)
		for y := 0; y < h; y++ {
			for x := 0; x < w; x++ {
				v := pix(seed, w, x, y)
				m.SetRGBA64(x, y, color.RGBA64{uint16(v[0])<<8 | uint16(v[4]), uint16(v[1])<<8 | uint16(v[5]), uint16(v[2])<<8 | uint16(v[6]), 0xffff})
			}
		}
		return m
	case "paletted":
		var p color.Palette
		for i := 0; i < 40; i++ {
			v := pix(seed, 1, i, 0)
			if i%3 == 0 {
				p = append(p, color.NRGBA{v[0], v[1], v[2], alpha(v[3])})
			} else {
				p = append(p, color.RGBA{v[0], v[1], v[2], 255})
			}
		}
		m := image.NewPaletted(r, p)
		for y := 0; y < h; y++ {
			for x := 0; x < w; x++ {
				m.SetColorIndex(x, y, pix(seed, w, x, y)[0]%40)
			}
		}
		return m
	case "ycbcr444", "ycbcr420":
		ratio := image.YCbCrSubsampleRatio444
		if kind == "ycbcr420" {
			ratio = image.YCbCrSubsampleRatio420
		}
		m := image.NewYCbCr(r, ratio)
		for y := 0; y < h; y++ {
			for x := 0; x < w; x++ {
				v := pix(seed, w, x, y)
				m.Y[m.YOffset(x, y)] = v[0]
				ci := m.COffset(x, y)
				m.Cb[ci] = v[1]
				m.Cr[ci] = v[2]
			}
		}
		return m
	}
	log.Fatalf("unknown synthetic kind %s", kind)
	return nil
}

func encodeSynth(kind string, w, h int, enc string) []byte {
	m := synth(kind, w, h)
	var buf bytes.Buffer
	var err error
	if enc == "png" {
		err = png.Encode(&buf, m)
	} else {
		err = jpeg.Encode(&buf, m, &jpeg.Options{Quality: 90})
	}
	if err != nil {
		log.Fatal(err)
	}
	return buf.Bytes()
}

// ---------------------------------------------------------------------------
// Sources

type source struct {
	id     string // "file:<path>" or "gen:<kind>:<w>x<h>:<png|jpg>"
	format images.Format
	b      []byte
	small  bool // store the outputs in full
	extra  bool // run the systematic spec set
	filter bool // run the filter chains
	chains bool // run the seeksnack template chains
}

var (
	wm        []byte
	maskBytes []byte
)

func allSources() []source {
	var out []source
	var files []string
	for _, dir := range []string{
		"resources/testdata", "resources/images/testdata", "docs", "media/testdata", "hugolib/testdata",
		"tpl/images/testdata", "snap", "crates/go-image/tests/fixtures/gotestdata", "crates/go-image/tests/fixtures/site",
		"crates/go-png/tests/fixtures/gotestdata", "crates/go-png/tests/fixtures/golden", "crates/go-png/tests/fixtures/repo",
	} {
		err := filepath.WalkDir(filepath.Join(root, dir), func(path string, d os.DirEntry, err error) error {
			if err != nil {
				return err
			}
			if d.IsDir() {
				return nil
			}
			switch strings.ToLower(filepath.Ext(path)) {
			case ".jpg", ".jpeg", ".png":
				rel, err := filepath.Rel(root, path)
				if err != nil {
					return err
				}
				files = append(files, filepath.ToSlash(rel))
			}
			return nil
		})
		if err != nil {
			log.Fatal(err)
		}
	}
	sort.Strings(files)
	for i, f := range files {
		ext := strings.ToLower(filepath.Ext(f))
		format := images.JPEG
		if ext == ".png" {
			format = images.PNG
		}
		b := mustRead(f)
		big := len(b) > 300000
		out = append(out, source{
			id: "file:" + f, format: format, b: b,
			extra:  !big && i%5 == 0,
			filter: !big && i%5 == 2,
			chains: strings.Contains(f, "fixtures/site/") || strings.Contains(f, "fixtures/golden/") || strings.HasSuffix(f, "gopher-hero8.png") || strings.HasSuffix(f, "sunset.jpg"),
		})
	}
	kinds := []struct{ kind, enc string }{
		{"nrgba", "png"}, {"nrgbaa", "png"}, {"rgba", "png"}, {"gray", "png"}, {"gray16", "png"},
		{"nrgba64", "png"}, {"rgba64", "png"}, {"paletted", "png"}, {"ycbcr444", "jpg"}, {"ycbcr420", "jpg"},
	}
	for _, k := range kinds {
		for _, sz := range [][2]int{{1, 1}, {2, 3}, {7, 5}, {33, 17}, {640, 480}} {
			format := images.PNG
			if k.enc == "jpg" {
				format = images.JPEG
			}
			small := sz[0]*sz[1] < 1000
			out = append(out, source{
				id:     fmt.Sprintf("gen:%s:%dx%d:%s", k.kind, sz[0], sz[1], k.enc),
				format: format, b: encodeSynth(k.kind, sz[0], sz[1], k.enc),
				small: small, extra: true, filter: true, chains: sz[0] == 640,
			})
		}
	}
	for _, k := range []struct{ kind, enc string }{{"nrgba", "png"}, {"ycbcr420", "jpg"}} {
		format := images.PNG
		if k.enc == "jpg" {
			format = images.JPEG
		}
		out = append(out, source{
			id:     fmt.Sprintf("gen:%s:2048x1536:%s", k.kind, k.enc),
			format: format, b: encodeSynth(k.kind, 2048, 1536, k.enc), chains: true,
		})
	}
	return out
}

// ---------------------------------------------------------------------------
// Operations

var coreSpecs = []string{
	"d:resize 600x480", "d:resize 300x240", "d:resize 600x480 webp", "d:resize 640x480 webp",
	"s:resize 600x480", "d:resize 100x png", "d:resize x77 jpg q90",
}

var extraSpecs = []string{
	"d:resize 128x128", "d:resize 32x32 webp", "d:resize 128x128 webp", "d:fit 120x90", "d:fit 1000x1000",
	"d:fill 90x60 topleft", "d:fill 90x60 center", "d:fill 90x60 bottomright", "d:fill 60x90 top",
	"d:crop 50x40 topleft", "d:crop 50x40 top", "d:crop 50x40 topright", "d:crop 50x40 left", "d:crop 50x40 center",
	"d:crop 50x40 right", "d:crop 50x40 bottomleft", "d:crop 50x40 bottom", "d:crop 50x40 bottomright",
	"d:resize 64x jpg q1", "d:resize 64x jpg q100", "d:resize 64x webp picture", "d:resize 64x webp photo",
	"d:resize 64x webp drawing", "d:resize 64x webp icon", "d:resize 64x webp text", "d:resize 64x webp q10",
	"d:resize 64x webp q100", "d:resize 64x jpg #fff", "d:resize 64x jpg #abc123", "d:resize 64x png #abc123",
	"d:resize 50x r90", "d:resize 50x r180", "d:resize 50x r270", "d:resize 50x r45", "d:resize 64x png",
	"d:resize 64x gif", "d:resize 20x gif", "d:gif", "d:r90 gif", "d:resize 64x tif", "d:resize 64x bmp",
	"d:png", "d:jpg", "d:webp", "d:r90", "d:", "d:webp q50 drawing",
	"d:resize 70x45 nearestneighbor", "d:resize 70x45 box", "d:resize 70x45 linear", "d:resize 70x45 hermite",
	"d:resize 70x45 mitchellnetravali", "d:resize 70x45 catmullrom", "d:resize 70x45 bspline",
	"d:resize 70x45 gaussian", "d:resize 70x45 lanczos", "d:resize 70x45 hann", "d:resize 70x45 hamming",
	"d:resize 70x45 blackman", "d:resize 70x45 bartlett", "d:resize 70x45 welch", "d:resize 70x45 cosine",
	"d:resize 1000x", "d:resize 3x2",
	"lz:fill 90x60", "lz:resize 64x jpg", "lz:resize 64x png", "q0:resize 64x webp", "q0:webp",
	"d:fill 90x60 smart", "d:crop 50x40", "d:fill SRCWxSRCH smart", "d:crop SRCWxSRCH",
}

// filterChains are named images.Filter argument lists (the Rust test has the same table).
func filterChains(f images.Filters, wmSrc, maskSrc images.ImageSource) []struct {
	name string
	fs   []gift.Filter
} {
	return []struct {
		name string
		fs   []gift.Filter
	}{
		{"overlay", []gift.Filter{f.Overlay(wmSrc, 0, 0)}},
		{"overlay-off", []gift.Filter{f.Overlay(wmSrc, 10, -5)}},
		{"brightness", []gift.Filter{f.Brightness(30)}},
		{"contrast", []gift.Filter{f.Contrast(-20)}},
		{"gamma", []gift.Filter{f.Gamma(1.5)}},
		{"blur", []gift.Filter{f.GaussianBlur(1.2)}},
		{"gray", []gift.Filter{f.Grayscale()}},
		{"hue", []gift.Filter{f.Hue(-45)}},
		{"invert", []gift.Filter{f.Invert()}},
		{"pixelate", []gift.Filter{f.Pixelate(5)}},
		{"saturation", []gift.Filter{f.Saturation(50)}},
		{"sepia", []gift.Filter{f.Sepia(70)}},
		{"sigmoid", []gift.Filter{f.Sigmoid(0.5, 7)}},
		{"unsharp", []gift.Filter{f.UnsharpMask(1, 0.8, 0.02)}},
		{"colorize", []gift.Filter{f.Colorize(180, 50, 30)}},
		{"colorbalance", []gift.Filter{f.ColorBalance(10, -10, 20)}},
		{"opacity", []gift.Filter{f.Opacity(0.6)}},
		{"opacity-over", []gift.Filter{f.Opacity(1.7)}},
		{"padding", []gift.Filter{f.Padding(10, 20, "#abc123")}},
		{"padding-neg", []gift.Filter{f.Padding(-2)}},
		{"padding-bad", []gift.Filter{f.Padding(-1000)}},
		{"mask", []gift.Filter{f.Mask(maskSrc)}},
		{"process", []gift.Filter{f.Process("resize 50x webp")}},
		{"chain", []gift.Filter{f.Brightness(10), f.Process("resize 40x"), f.Overlay(wmSrc, 0, 0)}},
	}
}

// bytesSource is an images.ImageSource over encoded bytes (the watermark, the mask).
type bytesSource struct {
	key string
	b   []byte
}

func (s bytesSource) DecodeImage() (image.Image, error) {
	m, _, err := image.Decode(bytes.NewReader(s.b))
	return m, err
}
func (s bytesSource) Key() string { return s.key }

type outcome struct {
	b   []byte
	img image.Image
	err error
}

func describe(o outcome) map[string]any {
	if o.err != nil {
		return map[string]any{"err": o.err.Error()}
	}
	return map[string]any{
		"type": fmt.Sprintf("%T", o.img), "bounds": o.img.Bounds().String(),
		"len": len(o.b), "sha": sha(o.b),
	}
}

func decode(b []byte) (image.Image, error) {
	m, _, err := image.Decode(bytes.NewReader(b))
	return m, err
}

// specOp runs "<cfg>:<spec>": Hugo's processOptions (DecodeImageConfig, ApplyFiltersFromConfig)
// and EncodeTo.
func specOp(src []byte, format images.Format, op string) outcome {
	name, spec, _ := strings.Cut(op, ":")
	var opts []string
	fields := strings.Fields(spec)
	opts = append(opts, fields...)
	conf, err := images.DecodeImageConfig(opts, cfgs[name], format)
	if err != nil {
		return outcome{err: err}
	}
	img, err := decode(src)
	if err != nil {
		return outcome{err: err}
	}
	converted, err := procs[name].ApplyFiltersFromConfig(img, conf)
	if err != nil {
		return outcome{err: err}
	}
	return encode(conf, converted)
}

func encode(conf images.ImageConfig, img image.Image) (o outcome) {
	defer func() {
		if r := recover(); r != nil {
			o = outcome{err: fmt.Errorf("panic: %v", r)}
		}
	}()
	var buf bytes.Buffer
	i := images.Image{}
	if err := i.EncodeTo(conf, img, &buf); err != nil {
		return outcome{err: err}
	}
	return outcome{b: buf.Bytes(), img: img}
}

// filterOp runs Hugo's imageResource.Filter: the key and config from the process filters,
// the process filters expanded to their gift filters, ImageProcessor.Filter, EncodeTo.
func filterOp(src []byte, format images.Format, fs []gift.Filter) (o outcome, key string) {
	defer func() {
		if r := recover(); r != nil {
			o = outcome{err: fmt.Errorf("panic: %v", r)}
		}
	}()
	var options []string
	for _, f := range fs {
		f = images.UnwrapFilter(f)
		if sp, ok := f.(images.ImageProcessSpecProvider); ok {
			options = append(options, strings.Fields(sp.ImageProcessSpec())...)
		}
	}
	confMain, err := images.DecodeImageConfig(options, cfgs["d"], format)
	if err != nil {
		return outcome{err: err}, ""
	}
	confMain.Action = "filter"
	confMain.Key = hashing.HashString(fs)
	key = confMain.Key // kept when a filter panics (the recover above)
	img, err := decode(src)
	if err != nil {
		return outcome{err: err}, confMain.Key
	}
	var filters []gift.Filter
	for _, f := range fs {
		f = images.UnwrapFilter(f)
		if sp, ok := f.(images.ImageProcessSpecProvider); ok {
			conf, err := images.DecodeImageConfig(strings.Fields(sp.ImageProcessSpec()), cfgs["d"], format)
			if err != nil {
				return outcome{err: err}, confMain.Key
			}
			pf, err := procs["d"].FiltersFromConfig(img, conf)
			if err != nil {
				return outcome{err: err}, confMain.Key
			}
			filters = append(filters, pf...)
		} else {
			filters = append(filters, f)
		}
	}
	converted, err := procs["d"].Filter(img, filters...)
	if err != nil {
		return outcome{err: err}, confMain.Key
	}
	return encode(confMain, converted), confMain.Key
}

func runSource(si int, src source) (cases, full []map[string]any) {
	record := func(op string, o outcome, extra map[string]any) {
		c := map[string]any{"src": src.id, "op": op, "res": describe(o)}
		for k, v := range extra {
			c[k] = v
		}
		cases = append(cases, c)
		// Full bytes for every 4th small output of the small synthetic sources.
		if src.small && o.err == nil && len(o.b) < 2048 && len(cases)%4 == 0 {
			full = append(full, map[string]any{"src": src.id, "op": op, "b": base64.StdEncoding.EncodeToString(o.b)})
		}
	}

	img, err := decode(src.b)
	head := map[string]any{"srcsha": sha(src.b), "format": int(src.format)}
	if err != nil {
		record("decode", outcome{err: err}, head)
		return
	}
	record("decode", outcome{b: src.b, img: img}, head)
	b := img.Bounds()

	specs := append([]string(nil), coreSpecs...)
	if src.extra {
		specs = append(specs, extraSpecs...)
	}
	for _, op := range specs {
		op = strings.ReplaceAll(op, "SRCWxSRCH", fmt.Sprintf("%dx%d", b.Dx(), b.Dy()))
		record(op, specOp(src.b, src.format, op), nil)
	}

	if src.filter {
		var f images.Filters
		wmSrc := bytesSource{"/images/watermark_hu_3bf49ff914f6e68c.png_6519743917224815147", wm}
		maskSrc := bytesSource{"/mask.png_1", maskBytes}
		for _, fc := range filterChains(f, wmSrc, maskSrc) {
			o, key := filterOp(src.b, src.format, fc.fs)
			record("filter:"+fc.name, o, map[string]any{"key": key})
		}
	}

	if src.chains {
		chains(src, b, record)
	}
	_ = si
	return
}

// chains runs the seeksnack template chains (specs/images.md §2) with the round trips
// through the encoded bytes between the steps.
func chains(src source, b image.Rectangle, record func(string, outcome, map[string]any)) {
	var f images.Filters
	step := func(in []byte, format images.Format, spec string) outcome {
		return specOp(in, format, "s:"+spec)
	}
	overlay := func(in []byte, format images.Format, w []byte) outcome {
		o, _ := filterOpCfg(in, format, []gift.Filter{f.Overlay(bytesSource{"wm", w}, 0, 0)})
		return o
	}
	for _, sz := range []string{"600x480", "300x240", "600x200"} {
		a := step(src.b, src.format, "resize "+sz)
		record("chain:"+sz+":A", a, nil)
		w := step(wm, images.PNG, "resize "+sz)
		record("chain:"+sz+":W", w, nil)
		if a.err != nil || w.err != nil {
			continue
		}
		bb := overlay(a.b, src.format, w.b)
		record("chain:"+sz+":B", bb, nil)
		if bb.err != nil {
			continue
		}
		record("chain:"+sz+":C", step(bb.b, src.format, "resize "+sz+" webp"), nil)
		if sz == "300x240" {
			record("chain:"+sz+":C600", step(bb.b, src.format, "resize 600x480 webp"), nil)
		}
		if sz == "600x480" {
			record("chain:"+sz+":A-webp", step(a.b, src.format, "resize 600x480 webp"), nil)
		}
	}
	// render-image: the watermark resized to the original's size, overlaid on the original.
	dims := fmt.Sprintf("%dx%d", b.Dx(), b.Dy())
	w := step(wm, images.PNG, "resize "+dims)
	record("chain:render:W", w, nil)
	if w.err == nil {
		bb := overlay(src.b, src.format, w.b)
		record("chain:render:B", bb, nil)
		if bb.err == nil {
			record("chain:render:C", step(bb.b, src.format, "resize "+dims+" webp"), nil)
		}
	}
}

// filterOpCfg is filterOp with the seeksnack config.
func filterOpCfg(src []byte, format images.Format, fs []gift.Filter) (outcome, string) {
	saved := cfgs["d"]
	savedP := procs["d"]
	cfgs["d"], procs["d"] = cfgs["s"], procs["s"]
	defer func() { cfgs["d"], procs["d"] = saved, savedP }()
	return filterOp(src, format, fs)
}
