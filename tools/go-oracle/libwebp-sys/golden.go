package main

// Reproduction of every processed WebP in the golden seeksnack output,
// adapted from the research harness $S/work/images/repro/full (which
// reproduces all golden *_hu_* images byte for byte). For every golden WebP
// the exact image handed to libwebp.Encode is captured, so the Rust crate
// can be checked against the golden bytes.

import (
	"bytes"
	"fmt"
	"image"
	"image/draw"
	"image/jpeg"
	"image/png"
	"os"
	"path/filepath"
	"runtime"
	"sort"
	"strings"
	"sync"
	"sync/atomic"

	"github.com/bep/gowebp/libwebp"
	"github.com/bep/gowebp/libwebp/webpoptions"
	"github.com/disintegration/gift"
)

var cfgHash string

// hugoWebpOptions are the options resources/images/image.go EncodeTo uses
// for the seeksnack imaging config (quality 75, hint photo).
var hugoWebpOptions = webpoptions.EncodingOptions{
	Quality: 75, EncodingPreset: webpoptions.EncodingPresetPhoto, UseSharpYuv: true,
}

// capturedWebp is the input and output of one golden WebP encode.
type capturedWebp struct {
	img image.Image
	out []byte
}

var (
	capturedMu sync.Mutex
	captured   = map[string]capturedWebp{}
)

// node is a lazily processed image in a chain.
type node struct {
	dir     string // output dir relative to publish root
	base    string
	hu      string
	format  string
	srcHash uint64

	parent *node
	op     func(parentImg []byte, parentFormat string) []byte

	once sync.Once
	data []byte
	raw  []byte // for originals
}

func (n *node) fileName() string {
	if n.hu == "" {
		return n.base + "." + n.format
	}
	return n.base + "_hu_" + n.hu + "." + n.format
}

func (n *node) relPath() string { return filepath.Join(n.dir, n.fileName()) }

// Key() for an image created in a cold build (includeHashInKey && !sourceFilenameIsHash)
func (n *node) key() string { return fmt.Sprintf("/%s_%d", filepath.ToSlash(n.relPath()), n.srcHash) }

func (n *node) bytes() []byte {
	n.once.Do(func() {
		if n.parent == nil {
			n.data = n.raw
			return
		}
		n.data = n.op(n.parent.bytes(), n.parent.format)
	})
	return n.data
}

func decode(b []byte, format string) image.Image {
	var img image.Image
	var err error
	switch format {
	case "jpg":
		img, err = jpeg.Decode(bytes.NewReader(b))
	case "png":
		img, err = png.Decode(bytes.NewReader(b))
	default:
		panic(format)
	}
	if err != nil {
		panic(err)
	}
	return img
}

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

func isOpaque(img image.Image) bool {
	if o, ok := img.(interface{ Opaque() bool }); ok {
		return o.Opaque()
	}
	return false
}

func postProcess(src, converted image.Image, target string) image.Image {
	hasAlpha := !isOpaque(converted)
	if target == "jpg" && hasAlpha {
		panic("unexpected alpha fill")
	}
	if target == "png" {
		if paletted, ok := src.(*image.Paletted); ok {
			tmp := image.NewPaletted(converted.Bounds(), paletted.Palette)
			draw.FloydSteinberg.Draw(tmp, tmp.Bounds(), converted, converted.Bounds().Min)
			converted = tmp
		}
	}
	return converted
}

func encodeImage(format string, img image.Image) []byte {
	var buf bytes.Buffer
	var err error
	switch format {
	case "jpg":
		if n, ok := img.(*image.NRGBA); ok && n.Opaque() {
			img = &image.RGBA{Pix: n.Pix, Stride: n.Stride, Rect: n.Rect}
		}
		err = jpeg.Encode(&buf, img, &jpeg.Options{Quality: 75})
	case "png":
		enc := png.Encoder{CompressionLevel: png.DefaultCompression}
		err = enc.Encode(&buf, img)
	case "webp":
		err = libwebp.Encode(&buf, img, hugoWebpOptions)
	}
	if err != nil {
		panic(err)
	}
	return buf.Bytes()
}

func resize(p *node, w, h int, target string) *node {
	opts := []string{"resize", fmt.Sprintf("%dx%d", w, h)}
	if target == "" {
		target = p.format
	} else {
		opts = append(opts, target)
	}
	key := processKey(opts...)
	n := &node{dir: p.dir, base: p.base, format: target, srcHash: p.srcHash, parent: p}
	n.hu = targetHash(p.hu, p.srcHash, key, cfgHash)
	n.op = func(pb []byte, pf string) []byte {
		src := decode(pb, pf)
		conv := doFilter(src, gift.Resize(w, h, gift.BoxResampling))
		conv = postProcess(src, conv, target)
		out := encodeImage(target, conv)
		if target == "webp" {
			capturedMu.Lock()
			captured[n.relPath()] = capturedWebp{img: conv, out: out}
			capturedMu.Unlock()
		}
		return out
	}
	return n
}

type overlayDrawFilter struct{ wm image.Image }

func (f overlayDrawFilter) Draw(dst draw.Image, src image.Image, options *gift.Options) {
	gift.New().Draw(dst, src)
	gift.New().DrawAt(dst, f.wm, image.Pt(0, 0), gift.OverOperator)
}

func (f overlayDrawFilter) Bounds(b image.Rectangle) image.Rectangle {
	return image.Rect(0, 0, b.Dx(), b.Dy())
}

func overlay(p *node, wm *node) *node {
	key := overlayFilterKey(wm.key(), 0, 0)
	n := &node{dir: p.dir, base: p.base, format: p.format, srcHash: p.srcHash, parent: p}
	n.hu = targetHash(p.hu, p.srcHash, key, cfgHash)
	n.op = func(pb []byte, pf string) []byte {
		src := decode(pb, pf)
		conv := doFilter(src, overlayDrawFilter{wm: decode(wm.bytes(), wm.format)})
		conv = postProcess(src, conv, pf)
		return encodeImage(pf, conv)
	}
	return n
}

func original(path, outDir string) *node {
	b, err := os.ReadFile(path)
	if err != nil {
		panic(err)
	}
	ext := strings.TrimPrefix(filepath.Ext(path), ".")
	return &node{
		dir: outDir, base: strings.TrimSuffix(filepath.Base(path), filepath.Ext(path)),
		format: ext, srcHash: xxFile(b), raw: b,
	}
}

// goldenWebps reproduces every golden *_hu_*.webp and returns the captured
// encoder inputs keyed by golden relative path, after checking that the Go
// encode is byte-identical to the golden file.
func goldenWebps(site, golden string) map[string]capturedWebp {
	cfgHash = hashStringHex(map[string]any{
		"_merge": "none",
		"exif": map[string]any{
			"_merge": "none", "disabledate": false, "disablelatlong": false,
			"excludefields": ".*", "includefields": "",
		},
	})

	goldenHu := map[string]bool{}
	dirs := map[string]bool{}
	filepath.Walk(golden, func(p string, fi os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		if !fi.IsDir() && strings.Contains(fi.Name(), "_hu_") && strings.HasSuffix(fi.Name(), ".webp") {
			rel, _ := filepath.Rel(golden, p)
			goldenHu[rel] = true
			dirs[filepath.Dir(rel)] = true
		}
		return nil
	})

	wmOrig := original(filepath.Join(site, "assets/images/watermark.png"), "images")
	wms := map[[2]int]*node{}
	var wmMu sync.Mutex
	wmFor := func(w, h int) *node {
		wmMu.Lock()
		defer wmMu.Unlock()
		k := [2]int{w, h}
		if n, ok := wms[k]; ok {
			return n
		}
		n := resize(wmOrig, w, h, "")
		wms[k] = n
		return n
	}

	cands := map[string]*node{}
	add := func(n *node) {
		if goldenHu[n.relPath()] {
			cands[n.relPath()] = n
		}
	}
	for dir := range dirs {
		entries, _ := os.ReadDir(filepath.Join(golden, dir))
		for _, e := range entries {
			name := e.Name()
			ext := filepath.Ext(name)
			if strings.Contains(name, "_hu_") || (ext != ".jpg" && ext != ".png") {
				continue
			}
			// The published original in golden is a byte-identical copy of the source.
			src := original(filepath.Join(golden, dir, name), dir)
			for _, sz := range [][2]int{{600, 480}, {300, 240}, {600, 200}, {128, 128}, {32, 32}} {
				w, h := sz[0], sz[1]
				a := resize(src, w, h, "")
				add(resize(a, w, h, "webp"))   // term.html / footer.html
				add(resize(src, w, h, "webp")) // header.html
				b := overlay(a, wmFor(w, h))
				add(resize(b, w, h, "webp"))
				if sz == [2]int{300, 240} {
					add(resize(b, 600, 480, "webp")) // index.json
				}
			}
			// render-image.html: filter original with a watermark resized to the original size
			var cfg image.Config
			var err error
			if ext == ".jpg" {
				cfg, err = jpeg.DecodeConfig(bytes.NewReader(src.raw))
			} else {
				cfg, err = png.DecodeConfig(bytes.NewReader(src.raw))
			}
			if err != nil {
				panic(err)
			}
			b := overlay(src, wmFor(cfg.Width, cfg.Height))
			add(resize(b, cfg.Width, cfg.Height, "webp"))
		}
	}
	var missing []string
	for p := range goldenHu {
		if cands[p] == nil {
			missing = append(missing, p)
		}
	}
	if len(missing) > 0 {
		sort.Strings(missing)
		panic(fmt.Sprintf("golden webps without a chain: %v", missing))
	}

	var keys []string
	for k := range cands {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	var bad int64
	work := make(chan string)
	var wg sync.WaitGroup
	for i := 0; i < runtime.GOMAXPROCS(0); i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for k := range work {
				g, err := os.ReadFile(filepath.Join(golden, k))
				if err != nil {
					panic(err)
				}
				if !bytes.Equal(g, cands[k].bytes()) {
					atomic.AddInt64(&bad, 1)
					fmt.Fprintln(os.Stderr, "BYTES DIFFER:", k)
				}
			}
		}()
	}
	for _, k := range keys {
		work <- k
	}
	close(work)
	wg.Wait()
	if bad != 0 {
		panic(fmt.Sprintf("%d golden webps not reproduced", bad))
	}
	res := map[string]capturedWebp{}
	for _, k := range keys {
		c, ok := captured[k]
		if !ok {
			panic("not captured: " + k)
		}
		res[k] = c
	}
	fmt.Fprintf(os.Stderr, "golden webps reproduced byte-identically: %d\n", len(res))
	return res
}
