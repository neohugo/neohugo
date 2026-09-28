// Command config is the Go oracle for the configuration side of crates/nh-images (Wave B
// task T10): images.DecodeConfig (the [imaging] section: SourceHash, decoded config, errors),
// NewImageProcessor (the exif regexps), DecodeImageConfig (spec strings: every ImageConfig
// field and the Key), Reanchor, the image formats, the filter keys (hashing.HashString of the
// images.* filters, the file name part of every filtered image) and the colour helpers.
//
// Luminance is float64 arithmetic that the arm64 compiler fuses, so the fixture must come
// from an arm64 build (see crates/nh-images/PORTING.md, "Regenerating fixtures"):
//
//	GOARCH=arm64 CGO_ENABLED=1 CC=zcc go build -o /tmp/nhi-config ./tools/go-oracle/nh-images/config
//	qemu-aarch64-static /tmp/nhi-config -out crates/nh-images/tests/fixtures/config
package main

import (
	"flag"
	"fmt"
	"image"
	"image/color"
	"log"
	"math"
	"path/filepath"
	"reflect"
	"strings"

	"github.com/disintegration/gift"
	"github.com/neohugo/neohugo/common/hashing"
	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/resources/images"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
)

func main() {
	out := flag.String("out", "crates/nh-images/tests/fixtures/config", "output directory")
	_ = flag.String("root", ".", "repository root (unused by this topic)")
	flag.Parse()

	var cases []map[string]any
	add := func(c map[string]any) { cases = append(cases, c) }

	decodeConfigCases(add)
	imageConfigCases(add)
	formatCases(add)
	filterKeyCases(add)
	colorCases(add)

	header := map[string]any{"oracle": "nh-images/config"}
	if err := goval.WriteCasesGz(filepath.Join(*out, "config.json.gz"), header, cases); err != nil {
		log.Fatal(err)
	}
}

// colorDesc describes a color.Color: its Go type and RGBA().
func colorDesc(c color.Color) any {
	if c == nil {
		return nil
	}
	r, g, b, a := c.RGBA()
	return map[string]any{"t": fmt.Sprintf("%T", c), "v": fmt.Sprintf("%v", c), "rgba": []uint32{r, g, b, a}}
}

func resamplingName(r gift.Resampling) any {
	if r == nil {
		return nil
	}
	if s, ok := r.(fmt.Stringer); ok {
		return s.String()
	}
	return fmt.Sprintf("%T", r)
}

type namedMap struct {
	name string
	m    func() map[string]any
}

func p(kv ...any) maps.Params {
	m := maps.Params{}
	for i := 0; i < len(kv); i += 2 {
		m[kv[i].(string)] = kv[i+1]
	}
	return m
}

func sm(kv ...any) map[string]any {
	m := map[string]any{}
	for i := 0; i < len(kv); i += 2 {
		m[kv[i].(string)] = kv[i+1]
	}
	return m
}

// imagingConfigs are the [imaging] sections: the defaults, the seeksnack section (with the
// `_merge` keys the config loader adds; its SourceHash 4bf645f71319dd1d names every processed
// image of the golden build), the config dumps' variants, and every option with valid and
// invalid values.
func imagingConfigs() []namedMap {
	var out []namedMap
	addm := func(name string, f func() map[string]any) { out = append(out, namedMap{name, f}) }

	addm("empty", func() map[string]any { return map[string]any{} })
	addm("defaults", func() map[string]any {
		return sm("resampleFilter", "box", "bgColor", "#ffffff", "hint", "photo", "quality", 75)
	})
	addm("seeksnack", func() map[string]any {
		return map[string]any(p("_merge", "none", "exif", p("_merge", "none", "disabledate", false, "disablelatlong", false, "excludefields", ".*", "includefields", "")))
	})
	addm("seeksnack-plain", func() map[string]any {
		return map[string]any(p("exif", p("excludefields", ".*")))
	})
	addm("dump-en", func() map[string]any {
		return map[string]any(p("bgcolor", "#ffffff", "exif", p("excludefields", ".*"), "hint", "photo", "quality", 75, "resamplefilter", "box"))
	})
	addm("dump-en-printzero", func() map[string]any {
		return map[string]any(p("bgcolor", "#ffffff", "exif", p("disabledate", false, "disablelatlong", false, "excludefields", ".*", "includefields", ""), "hint", "photo", "quality", 75, "resamplefilter", "box"))
	})
	for _, q := range []any{0, 1, 50, 90, 100, 101, -1, int64(80), 60.0, 60.7, "75", true, uint8(33)} {
		q := q
		addm(fmt.Sprintf("quality-%T-%v", q, q), func() map[string]any { return sm("quality", q) })
	}
	for _, f := range []string{"NearestNeighbor", "Box", "Linear", "Hermite", "MitchellNetravali", "CatmullRom", "BSpline", "Gaussian", "Lanczos", "Hann", "Hamming", "Blackman", "Bartlett", "Welch", "Cosine", "LANCZOS", "cubic", "", "bogus"} {
		f := f
		addm("resample-"+f, func() map[string]any { return sm("resampleFilter", f) })
	}
	for _, a := range []string{"Center", "TopLeft", "Top", "TopRight", "Left", "Right", "BottomLeft", "Bottom", "BottomRight", "Smart", "smart", "", "middle"} {
		a := a
		addm("anchor-"+a, func() map[string]any { return sm("anchor", a) })
	}
	for _, h := range []string{"picture", "photo", "drawing", "icon", "text", "PHOTO", "bogus", ""} {
		h := h
		addm("hint-"+h, func() map[string]any { return sm("hint", h) })
	}
	for _, c := range []string{"#fff", "fff", "#FFFFFF", "#000", "000000", "#abc123", "ABC123", "#abcd", "#12345678", "ffffffff", "#12345", "xyz", "#ggg", "", "#", "#é1", "#1é"} {
		c := c
		addm("bg-"+c, func() map[string]any { return sm("bgColor", c) })
	}
	exifs := []map[string]any{
		{},
		{"includeFields": ".*"},
		{"excludeFields": ".*"},
		{"includeFields": "Date|Orientation", "excludeFields": ""},
		{"includeFields": "(?i)make", "excludeFields": "GPS"},
		{"includeFields": " ", "excludeFields": " "},
		{"disableDate": true, "disableLatLong": true},
		{"disableDate": "true"},
		{"excludeFields": "("},
		{"includeFields": "[a-"},
	}
	for i, e := range exifs {
		e := e
		addm(fmt.Sprintf("exif-%d", i), func() map[string]any {
			m := map[string]any{}
			for k, v := range e {
				m[k] = v
			}
			return sm("exif", m)
		})
	}
	addm("case-keys", func() map[string]any {
		return sm("QUALITY", 88, "ResampleFilter", "Lanczos", "ANCHOR", "topleft", "BgColor", "#ABC")
	})
	addm("full", func() map[string]any {
		return sm("quality", 90, "resampleFilter", "CatmullRom", "hint", "drawing", "anchor", "BottomRight", "bgColor", "#abc123", "exif", sm("includeFields", "Orientation", "disableLatLong", true))
	})
	addm("unknown-key", func() map[string]any { return sm("foo", "bar", "quality", 70) })
	addm("bad-exif-type", func() map[string]any { return sm("exif", "none") })
	addm("bad-quality-slice", func() map[string]any { return sm("quality", []any{1}) })
	return out
}

func decodeConfigCases(add func(map[string]any)) {
	for _, nm := range imagingConfigs() {
		in := nm.m()
		enc := goval.Encode(in)
		res := goval.CallRaw(func() (any, error) {
			ns, err := images.DecodeConfig(in)
			if err != nil {
				return nil, err
			}
			c := ns.Config
			js, err := ns.MarshalJSON()
			if err != nil {
				return nil, err
			}
			r := map[string]any{
				"hash": ns.SourceHash,
				"cfg": map[string]any{
					"quality":        c.Imaging.Quality,
					"resampleFilter": c.Imaging.ResampleFilter,
					"hint":           c.Imaging.Hint,
					"anchor":         c.Imaging.Anchor,
					"bgColor":        c.Imaging.BgColor,
					"exif": map[string]any{
						"includeFields":  c.Imaging.Exif.IncludeFields,
						"excludeFields":  c.Imaging.Exif.ExcludeFields,
						"disableDate":    c.Imaging.Exif.DisableDate,
						"disableLatLong": c.Imaging.Exif.DisableLatLong,
					},
				},
				"internal": map[string]any{
					"bgColor":  colorDesc(c.BgColor),
					"hint":     int(c.Hint),
					"resample": resamplingName(c.ResampleFilter),
					"anchor":   int(c.Anchor),
				},
				"json": string(js),
			}
			if _, err := images.NewImageProcessor(nil, ns); err != nil {
				r["procErr"] = err.Error()
			}
			return r, nil
		})
		add(map[string]any{"kind": "decodeconfig", "name": nm.name, "in": enc, "res": res})
	}
}

// specs are option lists as Hugo builds them: the action and strings.Fields(strings.ToLower(spec))
// (Resize, Crop, Fit, Fill), strings.Fields(spec) (Process), and raw lists with case and
// spaces for DecodeImageConfig's own cleaning.
func specs() [][]string {
	var out [][]string
	actionSpecs := map[string][]string{
		"resize": {
			"600x480", "300x240", "600x480 webp", "640x480 webp", "600x200", "128x128", "32x32 webp", "128x128 webp",
			"600x", "x480", "0x0", "x", "600x480x3", "abcx480", "600xabc", "-10x20", "600x480 q1", "600x480 q75", "600x480 q100",
			"600x480 q0", "600x480 q101", "600x480 q", "600x480 qabc", "600x480 r90", "600x480 r180", "600x480 r270", "600x480 r-90",
			"600x480 rabc", "600x480 #fff", "600x480 #abc123", "600x480 #xyz", "600x480 #", "600x480 png", "600x480 jpg", "600x480 jpeg",
			"600x480 gif", "600x480 tif", "600x480 tiff", "600x480 bmp", "600x480 webp picture", "600x480 webp photo",
			"600x480 webp drawing", "600x480 webp icon", "600x480 webp text", "600x480 webp q90 drawing", "600x480 lanczos",
			"600x480 box", "600x480 nearestneighbor", "600x480 catmullrom", "600x480 hermite", "600x480 topleft",
			"600x480 center smart", "600x480 bogus", "600x480 jpg #fff q50", "", "webp", "600x480 1234", "600X480",
		},
		"crop":  {"600x480", "600x480 topleft", "600x480 top", "600x480 topright", "600x480 left", "600x480 center", "600x480 right", "600x480 bottomleft", "600x480 bottom", "600x480 bottomright", "600x480 smart", "600x", "x480"},
		"fit":   {"600x480", "600x480 lanczos", "600x", "100x100 webp q10"},
		"fill":  {"600x480", "600x480 topleft", "600x480 bottomright smart", "600x480 center lanczos", "x480", "600x480 png #abc123"},
		"":      {"600x480", "png", "webp q80", "jpg #abc", "r90", "resize 600x480", "fill 100x100 center", "crop 10x10", "fit 10x10 png", "", "q50"},
		"bogus": {"600x480"},
	}
	for _, action := range []string{"resize", "crop", "fit", "fill", "", "bogus"} {
		for _, spec := range actionSpecs[action] {
			if action == "" {
				out = append(out, strings.Fields(spec))
			} else {
				out = append(out, append([]string{action}, strings.Fields(strings.ToLower(spec))...))
			}
		}
	}
	out = append(out,
		[]string{"RESIZE", " 600x480 ", "", "WEBP", "  "},
		[]string{"resize", "600x480", "Q90"},
		[]string{"resize", "600x480", "#ABC"},
		[]string{},
		[]string{"resize"},
		[]string{"fill", "1x1"},
	)
	return out
}

func imageConfigCases(add func(map[string]any)) {
	cfgs := []namedMap{
		{"defaults", func() map[string]any { return map[string]any{} }},
		{"seeksnack", imagingConfigs()[2].m},
		{"topleft-lanczos-q90", func() map[string]any {
			return sm("anchor", "topleft", "resampleFilter", "lanczos", "quality", 90, "bgColor", "#abc123")
		}},
		{"quality0", func() map[string]any { return sm("quality", 0) }},
	}
	for _, nc := range cfgs {
		ns, err := images.DecodeConfig(nc.m())
		if err != nil {
			log.Fatal(err)
		}
		for _, opts := range specs() {
			for f := images.JPEG; f <= images.WEBP; f++ {
				in := append([]string(nil), opts...)
				res := goval.CallRaw(func() (any, error) {
					c, err := images.DecodeImageConfig(in, ns, f)
					if err != nil {
						return nil, err
					}
					rv := reflect.ValueOf(c)
					r := map[string]any{
						"target":     int(c.TargetFormat),
						"action":     c.Action,
						"key":        c.Key,
						"quality":    c.Quality,
						"qualitySet": rv.FieldByName("qualitySetForImage").Bool(),
						"rotate":     c.Rotate,
						"bgColor":    colorDesc(c.BgColor),
						"hint":       int(c.Hint),
						"width":      c.Width,
						"height":     c.Height,
						"filter":     resamplingName(c.Filter),
						"anchor":     int(c.Anchor),
						"reanchor":   c.Reanchor(gift.CenterAnchor).Key,
					}
					return r, nil
				})
				add(map[string]any{"kind": "imageconfig", "cfg": nc.name, "fmt": int(f), "opts": opts, "res": res})
			}
		}
	}
}

func formatCases(add func(map[string]any)) {
	for _, ext := range []string{".jpg", ".jpeg", ".jpe", ".jif", ".jfif", ".png", ".tif", ".tiff", ".bmp", ".gif", ".webp", ".JPG", "jpg", ".svg", ""} {
		f, ok := images.ImageFormatFromExt(ext)
		add(map[string]any{"kind": "fromext", "ext": ext, "fmt": int(f), "ok": ok})
	}
	for _, sub := range []string{"jpeg", "png", "tiff", "bmp", "gif", "webp", "jpg", "svg+xml", ""} {
		f, ok := images.ImageFormatFromMediaSubType(sub)
		add(map[string]any{"kind": "fromsubtype", "sub": sub, "fmt": int(f), "ok": ok})
	}
	for f := images.JPEG; f <= images.WEBP; f++ {
		add(map[string]any{
			"kind": "format", "fmt": int(f),
			"ext":          f.DefaultExtension(),
			"mediaType":    f.MediaType().Type,
			"defaultQ":     f.RequiresDefaultQuality(),
			"transparency": f.SupportsTransparency(),
			"imagemeta":    int(f.ToImageMetaImageFormatFormat()),
		})
	}
}

// keySource is an images.ImageSource with a given Key (the filters only hash the key).
type keySource string

func (k keySource) DecodeImage() (image.Image, error) { return nil, fmt.Errorf("not decodable") }
func (k keySource) Key() string                       { return string(k) }

type filterCase struct {
	fn   string
	args []any
}

func buildFilter(fc filterCase) gift.Filter {
	var f images.Filters
	a := fc.args
	switch fc.fn {
	case "Process":
		return f.Process(a[0])
	case "Overlay":
		return f.Overlay(keySource(a[0].(string)), a[1], a[2])
	case "Mask":
		return f.Mask(keySource(a[0].(string)))
	case "Opacity":
		return f.Opacity(a[0])
	case "Text":
		return f.Text(a[0].(string), a[1:]...)
	case "Padding":
		return f.Padding(a...)
	case "Dither":
		return f.Dither(a...)
	case "AutoOrient":
		return f.AutoOrient()
	case "Brightness":
		return f.Brightness(a[0])
	case "ColorBalance":
		return f.ColorBalance(a[0], a[1], a[2])
	case "Colorize":
		return f.Colorize(a[0], a[1], a[2])
	case "Contrast":
		return f.Contrast(a[0])
	case "Gamma":
		return f.Gamma(a[0])
	case "GaussianBlur":
		return f.GaussianBlur(a[0])
	case "Grayscale":
		return f.Grayscale()
	case "Hue":
		return f.Hue(a[0])
	case "Invert":
		return f.Invert()
	case "Pixelate":
		return f.Pixelate(a[0])
	case "Saturation":
		return f.Saturation(a[0])
	case "Sepia":
		return f.Sepia(a[0])
	case "Sigmoid":
		return f.Sigmoid(a[0], a[1])
	case "UnsharpMask":
		return f.UnsharpMask(a[0], a[1], a[2])
	}
	panic("unknown filter " + fc.fn)
}

// wmKey is the cold-cache key of the golden build's resized watermark (specs/images.md §4.5):
// Overlay(wm600x480, 0, 0) hashes to 1682858112077426900.
const wmKey = "/images/watermark_hu_3bf49ff914f6e68c.png_6519743917224815147"

func filterCases() []filterCase {
	nums := []any{0, 1, -1, 30, 100, 150, 0.5, 30.0, int64(30), "30", float32(0.25), uint8(7), -100, 1e10}
	var out []filterCase
	for _, x := range []any{0, 1, 10, -5, 0.0, 1.5, int64(0), "0", "7", uint(3)} {
		out = append(out, filterCase{"Overlay", []any{wmKey, x, 0}}, filterCase{"Overlay", []any{wmKey, 0, x}})
	}
	out = append(out,
		filterCase{"Overlay", []any{wmKey, 0, 0}},
		filterCase{"Overlay", []any{"/images/watermark_hu_3bf49ff914f6e68c.png", 0, 0}},
		filterCase{"Overlay", []any{"", 20, 30}},
		filterCase{"Mask", []any{"/images/mask.png_123"}},
		filterCase{"Process", []any{"resize 600x480 webp"}},
		filterCase{"Process", []any{"Fill 100x100 TopLeft"}},
		filterCase{"Process", []any{42}},
		filterCase{"AutoOrient", nil},
		filterCase{"Grayscale", nil},
		filterCase{"Invert", nil},
		filterCase{"Text", []any{"Hello", map[string]any{"size": 30, "x": 5}}},
		filterCase{"Dither", nil},
		filterCase{"Dither", []any{map[string]any{"method": "Atkinson", "colors": []any{"000", "fff"}}}},
		filterCase{"ColorBalance", []any{10, -20, 30.5}},
		filterCase{"Colorize", []any{180, 50, 20}},
		filterCase{"Sigmoid", []any{0.5, 5}},
		filterCase{"Sigmoid", []any{0.5, -5}},
		filterCase{"UnsharpMask", []any{1.5, 1, 0.05}},
		filterCase{"UnsharpMask", []any{0, 0, 0}},
	)
	for _, fn := range []string{"Opacity", "Brightness", "Contrast", "Gamma", "GaussianBlur", "Hue", "Pixelate", "Saturation", "Sepia"} {
		for _, n := range nums {
			out = append(out, filterCase{fn, []any{n}})
		}
	}
	for _, args := range [][]any{
		{10}, {10, 20}, {10, 20, 30}, {10, 20, 30, 40}, {10, 20, 30, 40, 50}, {10, "#fff"}, {10, "ff000080"},
		{"#abc"}, {10, "xyz"}, {10, "12"}, {-5, 3, "#abc123"}, {5001}, {5000}, {}, {1, 2, 3, 4, 5, 6},
		{10.5, "20"}, {int64(3)},
	} {
		out = append(out, filterCase{"Padding", args})
	}
	return out
}

func filterKeyCases(add func(map[string]any)) {
	var built []gift.Filter
	for _, fc := range filterCases() {
		var args []any
		for _, a := range fc.args {
			args = append(args, goval.Encode(a))
		}
		var f gift.Filter
		res := goval.CallRaw(func() (any, error) {
			f = buildFilter(fc)
			return map[string]any{
				"key":    hashing.HashString([]gift.Filter{f}),
				"single": hashing.HashString(f),
			}, nil
		})
		if f != nil {
			built = append(built, f)
		}
		add(map[string]any{"kind": "filter", "fn": fc.fn, "args": args, "res": res})
	}
	// Multi-filter lists (the Filter key of `images.Filter f1 f2 ... $img`).
	for i := 0; i+2 < len(built); i += 7 {
		add(map[string]any{"kind": "filterlist", "idx": []int{i, i + 1, i + 2}, "key": hashing.HashString([]gift.Filter{built[i], built[i+1], built[i+2]})})
	}
}

func colorCases(add func(map[string]any)) {
	hexes := []string{
		"fff", "#fff", "ffffff", "#FFFFFF", "000", "000000", "#abc123", "abcd", "#12345678", "ffffffff", "00000000",
		"#ff000080", "808080", "#010203", "fefefe", "0a0b0c", "#12345", "xyz", "", "#", "#é1", "g00", "12", "ÿÿÿ",
	}
	for _, h := range hexes {
		res := goval.CallRaw(func() (any, error) {
			cs := images.HexStringsToColors(h)
			c := cs[0]
			hh, _ := c.Hash()
			return map[string]any{
				"hex":       c.ColorHex(),
				"string":    c.String(),
				"luminance": goval.FBits(c.Luminance()),
				"color":     colorDesc(c.ColorGo()),
				"hash":      fmt.Sprint(hh),
			}, nil
		})
		add(map[string]any{"kind": "color", "in": h, "res": res})
	}
	// Luminance over a grid of colours (arm64 FMA).
	var lums []string
	for r := 0; r < 256; r += 15 {
		for g := 0; g < 256; g += 17 {
			for b := 0; b < 256; b += 51 {
				c := images.ColorGoToColor(color.RGBA{uint8(r), uint8(g), uint8(b), 255})
				lums = append(lums, c.ColorHex()+"="+goval.FBits(c.Luminance()))
			}
		}
	}
	add(map[string]any{"kind": "luminance", "values": lums})
	cols := []color.Color{
		color.RGBA{1, 2, 3, 255}, color.RGBA{1, 2, 3, 4}, color.NRGBA{200, 100, 50, 128}, color.White, color.Black,
		color.Transparent, color.Gray{77}, color.Gray16{0x1234}, color.Alpha{9}, color.RGBA64{0x1111, 0x2222, 0x3333, 0xffff},
		color.NRGBA64{0xffff, 0, 0x8000, 0x8000}, color.CMYK{10, 20, 30, 40}, color.YCbCr{100, 120, 140},
	}
	for _, c := range cols {
		cc := images.ColorGoToColor(c)
		add(map[string]any{
			"kind": "colorgo", "color": colorDesc(c),
			"hex": images.ColorGoToHexString(c), "luminance": goval.FBits(cc.Luminance()),
			"luminanceIsNaN": math.IsNaN(cc.Luminance()),
		})
	}
	pal := color.Palette{color.RGBA{0, 0, 0, 255}, color.RGBA{255, 255, 255, 255}, color.NRGBA{255, 0, 0, 128}}
	for _, c := range []color.Color{color.RGBA{255, 255, 255, 255}, color.White, color.RGBA{10, 20, 30, 255}, color.NRGBA{255, 0, 0, 128}} {
		p2 := images.AddColorToPalette(c, append(color.Palette(nil), pal...))
		var desc []any
		for _, pc := range p2 {
			desc = append(desc, colorDesc(pc))
		}
		p3 := append(color.Palette(nil), pal...)
		images.ReplaceColorInPalette(c, p3)
		var desc3 []any
		for _, pc := range p3 {
			desc3 = append(desc3, colorDesc(pc))
		}
		add(map[string]any{"kind": "palette", "color": colorDesc(c), "added": desc, "replaced": desc3})
	}
}
