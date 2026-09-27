// Command go-png is the Go oracle for the Rust crate crates/go-png.
//
// It exercises the go1.27.1 image/png package (decoder and encoder) and
// prints digests that the Rust differential tests compare against.
// Pseudo-random synthetic images come from a splitmix64 generator that the
// Rust tests re-implement call for call; PNG corpora for the decoder are
// generated here and stored as files (see pack/mkpng/mutate).
//
// Usage:
//
//	go-png files <root> <listfile>         # decode/encode real PNG files
//	go-png synth <seed0> <n> [maxdim]      # synthetic images, all levels
//	go-png mkpng <seed0> <n> <out.bin>     # generate a PNG corpus
//	go-png mutate <seed0> <n> <in.bin> <out.bin>  # mutate a corpus
//	go-png pack <out.bin> <files...>       # pack files into a corpus
//	go-png record <corpus.bin>             # decode records for a corpus
package main

import (
	"bufio"
	"bytes"
	"compress/zlib"
	"crypto/sha256"
	"encoding/binary"
	"encoding/hex"
	"errors"
	"fmt"
	"hash/crc32"
	"image"
	"image/color"
	"image/color/palette"
	"image/draw"
	"image/png"
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

func sha16(b []byte) string {
	s := sha256.Sum256(b)
	return hex.EncodeToString(s[:])[:16]
}

var out = bufio.NewWriterSize(os.Stdout, 1<<20)

func main() {
	defer out.Flush()
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "usage: go-png <cmd> ...")
		os.Exit(2)
	}
	atoi := func(s string) int {
		n, err := strconv.Atoi(s)
		if err != nil {
			panic(err)
		}
		return n
	}
	switch os.Args[1] {
	case "files":
		files(os.Args[2], os.Args[3])
	case "synth":
		maxDim := 200
		if len(os.Args) > 4 {
			maxDim = atoi(os.Args[4])
		}
		synth(atoi(os.Args[2]), atoi(os.Args[3]), maxDim)
	case "mkpng":
		mkpngCmd(atoi(os.Args[2]), atoi(os.Args[3]), os.Args[4])
	case "mutate":
		mutateCmd(atoi(os.Args[2]), atoi(os.Args[3]), os.Args[4], os.Args[5])
	case "pack":
		packCmd(os.Args[2], os.Args[3:])
	case "record":
		record(os.Args[2])
	default:
		fmt.Fprintln(os.Stderr, "unknown command", os.Args[1])
		os.Exit(2)
	}
}

// ---------------------------------------------------------------------------
// Digests.

func rectStr(r image.Rectangle) string {
	return fmt.Sprintf("%d,%d,%d,%d", r.Min.X, r.Min.Y, r.Max.X, r.Max.Y)
}

// paletteBytes serialises a palette: a tag byte per entry plus its values.
func paletteBytes(p color.Palette) []byte {
	var b []byte
	for _, c := range p {
		switch c := c.(type) {
		case color.RGBA:
			b = append(b, 'R', c.R, c.G, c.B, c.A)
		case color.NRGBA:
			b = append(b, 'N', c.R, c.G, c.B, c.A)
		default:
			r, g, bb, a := c.RGBA()
			b = append(b, 'O', byte(r>>8), byte(r), byte(g>>8), byte(g), byte(bb>>8), byte(bb), byte(a>>8), byte(a))
		}
	}
	return b
}

func palStr(p color.Palette) string {
	return fmt.Sprintf("%d:%s", len(p), sha16(paletteBytes(p)))
}

// imgDigest describes a decoded image: concrete type, geometry, full Pix
// buffer and palette.
func imgDigest(m image.Image) string {
	switch m := m.(type) {
	case *image.Gray:
		return fmt.Sprintf("Gray %s %d %d %s", rectStr(m.Rect), m.Stride, len(m.Pix), sha16(m.Pix))
	case *image.Gray16:
		return fmt.Sprintf("Gray16 %s %d %d %s", rectStr(m.Rect), m.Stride, len(m.Pix), sha16(m.Pix))
	case *image.RGBA:
		return fmt.Sprintf("RGBA %s %d %d %s", rectStr(m.Rect), m.Stride, len(m.Pix), sha16(m.Pix))
	case *image.RGBA64:
		return fmt.Sprintf("RGBA64 %s %d %d %s", rectStr(m.Rect), m.Stride, len(m.Pix), sha16(m.Pix))
	case *image.NRGBA:
		return fmt.Sprintf("NRGBA %s %d %d %s", rectStr(m.Rect), m.Stride, len(m.Pix), sha16(m.Pix))
	case *image.NRGBA64:
		return fmt.Sprintf("NRGBA64 %s %d %d %s", rectStr(m.Rect), m.Stride, len(m.Pix), sha16(m.Pix))
	case *image.Paletted:
		return fmt.Sprintf("Paletted %s %d %d %s pal=%s", rectStr(m.Rect), m.Stride, len(m.Pix), sha16(m.Pix), palStr(m.Palette))
	case nil:
		return "nil"
	}
	return fmt.Sprintf("other %T", m)
}

func modelStr(m color.Model) string {
	switch m {
	case color.GrayModel:
		return "Gray"
	case color.Gray16Model:
		return "Gray16"
	case color.RGBAModel:
		return "RGBA"
	case color.RGBA64Model:
		return "RGBA64"
	case color.NRGBAModel:
		return "NRGBA"
	case color.NRGBA64Model:
		return "NRGBA64"
	}
	if p, ok := m.(color.Palette); ok {
		return "Palette:" + palStr(p)
	}
	return "other"
}

func cfgStr(data []byte) string {
	cfg, err := png.DecodeConfig(bytes.NewReader(data))
	if err != nil {
		return "err:" + err.Error()
	}
	return fmt.Sprintf("%s,%d,%d", modelStr(cfg.ColorModel), cfg.Width, cfg.Height)
}

func decStr(data []byte) (image.Image, string) {
	m, err := png.Decode(bytes.NewReader(data))
	if err != nil {
		return nil, "err:" + err.Error()
	}
	return m, imgDigest(m)
}

func regStr(data []byte) string {
	m, name, err := image.Decode(bytes.NewReader(data))
	if err != nil {
		return "err:" + err.Error()
	}
	return name + " " + imgDigest(m)
}

// encStr encodes m at the given level: "sha16:len" or "err:<msg>:sha16:len".
func encStr(m image.Image, level png.CompressionLevel) (string, []byte) {
	var buf bytes.Buffer
	enc := png.Encoder{CompressionLevel: level}
	err := enc.Encode(&buf, m)
	if err != nil {
		return fmt.Sprintf("err:%s:%s:%d", err.Error(), sha16(buf.Bytes()), buf.Len()), nil
	}
	return fmt.Sprintf("%s:%d", sha16(buf.Bytes()), buf.Len()), buf.Bytes()
}

// countWriter counts Write calls.
type countWriter struct{ calls int }

func (c *countWriter) Write(p []byte) (int, error) {
	c.calls++
	return len(p), nil
}

// failWriter accepts Write calls until call index k (0-based) and fails
// that call and every later one.
type failWriter struct {
	k, calls int
	buf      bytes.Buffer
}

func (f *failWriter) Write(p []byte) (int, error) {
	f.calls++
	if f.calls > f.k {
		return 0, errors.New("boom")
	}
	f.buf.Write(p)
	return len(p), nil
}

// failStr encodes m at the default level with a writer that fails at a
// pseudo-random call index.
func failStr(m image.Image, seed uint64) string {
	var cw countWriter
	if err := png.Encode(&cw, m); err != nil {
		return "-"
	}
	rk := &rng{s: seed ^ 0xabcdef}
	k := rk.intn(cw.calls + 1)
	fw := &failWriter{k: k}
	err := png.Encode(fw, m)
	es := "nil"
	if err != nil {
		es = err.Error()
	}
	return fmt.Sprintf("k:%d calls:%d %s:%d err:%s", k, fw.calls, sha16(fw.buf.Bytes()), fw.buf.Len(), es)
}

// ---------------------------------------------------------------------------
// Real files.

// files decodes every file in listfile (paths relative to root) and prints
// the config, the decoded image, the registry decode, encodings of the
// decoded image at every level and encodings of conversions of it.
func files(root, listfile string) {
	list, err := os.ReadFile(listfile)
	if err != nil {
		panic(err)
	}
	for _, rel := range strings.Split(strings.TrimSpace(string(list)), "\n") {
		data, err := os.ReadFile(filepath.Join(root, rel))
		if err != nil {
			panic(err)
		}
		cols := []string{rel, sha16(data), cfgStr(data)}
		m, d := decStr(data)
		cols = append(cols, d, regStr(data))
		if m == nil {
			fmt.Fprintln(out, strings.Join(cols, "\t"))
			continue
		}
		for _, lvl := range []png.CompressionLevel{png.DefaultCompression, png.NoCompression, png.BestSpeed, png.BestCompression} {
			s, _ := encStr(m, lvl)
			cols = append(cols, s)
		}
		b := m.Bounds()
		nrgba := image.NewNRGBA(b)
		draw.Draw(nrgba, b, m, b.Min, draw.Src)
		rgba := image.NewRGBA(b)
		draw.Draw(rgba, b, m, b.Min, draw.Src)
		gray := image.NewGray(b)
		draw.Draw(gray, b, m, b.Min, draw.Src)
		pal8 := image.NewPaletted(b, palette.WebSafe)
		draw.Draw(pal8, b, m, b.Min, draw.Src)
		pal4 := image.NewPaletted(b, palette.Plan9[:16])
		draw.Draw(pal4, b, m, b.Min, draw.Src)
		pal1 := image.NewPaletted(b, color.Palette{color.Black, color.Transparent})
		draw.Draw(pal1, b, m, b.Min, draw.Src)
		for _, c := range []image.Image{nrgba, rgba, gray, pal8, pal4, pal1} {
			s, _ := encStr(c, png.DefaultCompression)
			cols = append(cols, s)
		}
		// A sub-image with odd offsets (Pix not starting at Rect.Min).
		sub := "-"
		if si, ok := m.(interface {
			SubImage(image.Rectangle) image.Image
		}); ok {
			r := image.Rect(b.Min.X+b.Dx()/3, b.Min.Y+b.Dy()/5, b.Max.X-b.Dx()/7, b.Max.Y-b.Dy()/9)
			sub, _ = encStr(si.SubImage(r), png.DefaultCompression)
		}
		cols = append(cols, sub, failStr(m, uint64(len(data))))
		fmt.Fprintln(out, strings.Join(cols, "\t"))
	}
}

// ---------------------------------------------------------------------------
// Synthetic images.

// fill fills b with random, smooth, few-valued or run-heavy content.
func fill(r *rng, b []byte, stride int) {
	switch r.intn(4) {
	case 0:
		for i := range b {
			b[i] = r.byte()
		}
	case 1:
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
	case 2:
		vals := [4]byte{r.byte(), r.byte(), r.byte(), r.byte()}
		for i := range b {
			b[i] = vals[r.intn(4)]
		}
	default:
		c := r.byte()
		for i := range b {
			if r.intn(64) == 0 {
				c = r.byte()
			}
			b[i] = c
		}
	}
}

// fixAlpha rewrites the alpha samples of pix (psize bytes per pixel, alpha
// at aoff, asize bytes wide) and optionally clamps premultiplied colour
// samples (the csize-byte samples before the alpha) to the alpha.
func fixAlpha(r *rng, pix []byte, psize, aoff, asize int, premul bool) {
	n := len(pix) / psize
	mode := r.intn(5)
	setA := func(p int, v uint16) {
		if asize == 1 {
			pix[p*psize+aoff] = byte(v >> 8)
		} else {
			pix[p*psize+aoff] = byte(v >> 8)
			pix[p*psize+aoff+1] = byte(v)
		}
	}
	getA := func(p int) uint16 {
		if asize == 1 {
			v := uint16(pix[p*psize+aoff])
			return v<<8 | v
		}
		return uint16(pix[p*psize+aoff])<<8 | uint16(pix[p*psize+aoff+1])
	}
	switch mode {
	case 0:
		for p := 0; p < n; p++ {
			setA(p, 0xffff)
		}
	case 1:
		// Leave the random alpha.
	case 2:
		for p := 0; p < n; p++ {
			if getA(p)&0x100 != 0 {
				setA(p, 0xffff)
			} else {
				setA(p, 0)
			}
		}
	case 3:
		for p := 0; p < n; p++ {
			setA(p, 0xffff)
		}
		if n > 0 {
			p := r.intn(n)
			setA(p, r.u16())
		}
	default:
		for p := 0; p < n; p++ {
			setA(p, 0)
		}
	}
	if premul && r.intn(2) == 0 {
		for p := 0; p < n; p++ {
			a := getA(p)
			for c := 0; c < aoff; c += asize {
				o := p*psize + c
				if asize == 1 {
					if uint16(pix[o]) > a>>8 {
						pix[o] = byte(a >> 8)
					}
				} else {
					v := uint16(pix[o])<<8 | uint16(pix[o+1])
					if v > a {
						pix[o] = byte(a >> 8)
						pix[o+1] = byte(a)
					}
				}
			}
		}
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

func genPaletted(r *rng, rect image.Rectangle) *image.Paletted {
	var np int
	switch r.intn(8) {
	case 0:
		np = 1 + r.intn(2)
	case 1:
		np = 3 + r.intn(2)
	case 2:
		np = 5 + r.intn(12)
	case 3:
		np = 256
	case 4:
		if r.intn(4) == 0 {
			if r.intn(2) == 0 {
				np = 0
			} else {
				np = 257 + r.intn(10)
			}
		} else {
			np = 17 + r.intn(239)
		}
	default:
		np = 1 + r.intn(256)
	}
	p := make(color.Palette, np)
	for i := range p {
		switch r.intn(4) {
		case 0:
			p[i] = color.RGBA{r.byte(), r.byte(), r.byte(), 0xff}
		case 1:
			p[i] = color.NRGBA{r.byte(), r.byte(), r.byte(), r.byte()}
		case 2:
			p[i] = randColor(r)
		default:
			p[i] = color.NRGBA{r.byte(), r.byte(), r.byte(), 0xff}
		}
	}
	m := image.NewPaletted(rect, p)
	mode := r.intn(4)
	if np == 0 {
		mode = 1
	}
	switch mode {
	case 0:
		for i := range m.Pix {
			m.Pix[i] = byte(r.intn(np))
		}
	case 1:
		for i := range m.Pix {
			m.Pix[i] = r.byte()
		}
	case 2:
		fill(r, m.Pix, m.Stride)
		for i := range m.Pix {
			m.Pix[i] = byte(int(m.Pix[i]) % np)
		}
	default:
		for i := range m.Pix {
			if r.intn(8) == 0 {
				m.Pix[i] = byte(r.intn(np))
			}
		}
	}
	return m
}

var ratios = []image.YCbCrSubsampleRatio{
	image.YCbCrSubsampleRatio444, image.YCbCrSubsampleRatio422, image.YCbCrSubsampleRatio420,
	image.YCbCrSubsampleRatio440, image.YCbCrSubsampleRatio411, image.YCbCrSubsampleRatio410,
}

const nSynthKinds = 14

// genBase creates an image of kind 0..9 with bounds rect.
func genBase(r *rng, kind int, rect image.Rectangle) image.Image {
	switch kind {
	case 0:
		m := image.NewRGBA(rect)
		fill(r, m.Pix, m.Stride)
		fixAlpha(r, m.Pix, 4, 3, 1, true)
		return m
	case 1:
		m := image.NewNRGBA(rect)
		fill(r, m.Pix, m.Stride)
		fixAlpha(r, m.Pix, 4, 3, 1, false)
		return m
	case 2:
		m := image.NewRGBA64(rect)
		fill(r, m.Pix, m.Stride)
		fixAlpha(r, m.Pix, 8, 6, 2, true)
		return m
	case 3:
		m := image.NewNRGBA64(rect)
		fill(r, m.Pix, m.Stride)
		fixAlpha(r, m.Pix, 8, 6, 2, false)
		return m
	case 4:
		m := image.NewAlpha(rect)
		fill(r, m.Pix, m.Stride)
		fixAlpha(r, m.Pix, 1, 0, 1, false)
		return m
	case 5:
		m := image.NewAlpha16(rect)
		fill(r, m.Pix, m.Stride)
		fixAlpha(r, m.Pix, 2, 0, 2, false)
		return m
	case 6:
		m := image.NewGray(rect)
		fill(r, m.Pix, m.Stride)
		return m
	case 7:
		m := image.NewGray16(rect)
		fill(r, m.Pix, m.Stride)
		return m
	case 8:
		m := image.NewCMYK(rect)
		fill(r, m.Pix, m.Stride)
		return m
	}
	return genPaletted(r, rect)
}

func genSynth(r *rng, maxDim int) (image.Image, string) {
	w, h := 1+r.intn(24), 1+r.intn(24)
	if r.intn(6) == 0 {
		w, h = 1+r.intn(maxDim), 1+r.intn(maxDim)
	}
	if r.intn(40) == 0 {
		if r.intn(2) == 0 {
			w = 0
		} else {
			h = 0
		}
	}
	ox, oy := r.intn(7)-3, r.intn(7)-3
	rect := image.Rect(ox, oy, ox+w, oy+h)
	kind := r.intn(nSynthKinds)
	desc := fmt.Sprintf("k%d %s", kind, rectStr(rect))
	switch kind {
	case 10:
		// Go's chroma indexing is only consistent for non-negative
		// coordinates, so YCbCr images are shifted into the positive quadrant.
		rect = rect.Add(image.Pt(16, 16))
		m := image.NewYCbCr(rect, ratios[r.intn(len(ratios))])
		fill(r, m.Y, m.YStride)
		fill(r, m.Cb, m.CStride)
		fill(r, m.Cr, m.CStride)
		return m, desc
	case 11:
		rect = rect.Add(image.Pt(16, 16))
		m := image.NewNYCbCrA(rect, ratios[r.intn(len(ratios))])
		fill(r, m.Y, m.YStride)
		fill(r, m.Cb, m.CStride)
		fill(r, m.Cr, m.CStride)
		fill(r, m.A, m.AStride)
		fixAlpha(r, m.A, 1, 0, 1, false)
		return m, desc
	case 12:
		pk := r.intn(10)
		l, t, rr, b := r.intn(4), r.intn(4), r.intn(4), r.intn(4)
		parent := genBase(r, pk, image.Rect(rect.Min.X-l, rect.Min.Y-t, rect.Max.X+rr, rect.Max.Y+b))
		sub := parent.(interface {
			SubImage(image.Rectangle) image.Image
		}).SubImage(rect)
		return sub, fmt.Sprintf("%s sub%d", desc, pk)
	case 13:
		return rect, desc
	}
	return genBase(r, kind, rect), desc
}

func synth(seed0, n, maxDim int) {
	for i := seed0; i < seed0+n; i++ {
		r := &rng{s: uint64(i)*0x9e3779b97f4a7c15 + 12345}
		m, desc := genSynth(r, maxDim)
		cols := []string{strconv.Itoa(i), desc}
		var def []byte
		for _, lvl := range []png.CompressionLevel{png.DefaultCompression, png.NoCompression, png.BestSpeed, png.BestCompression, 7} {
			s, b := encStr(m, lvl)
			if lvl == png.DefaultCompression {
				def = b
			}
			cols = append(cols, s)
		}
		rt := "-"
		if def != nil {
			_, rt = decStr(def)
		}
		cols = append(cols, rt, failStr(m, uint64(i)))
		fmt.Fprintln(out, strings.Join(cols, "\t"))
	}
}

// ---------------------------------------------------------------------------
// Corpora.

func readCorpus(path string) [][]byte {
	data, err := os.ReadFile(path)
	if err != nil {
		panic(err)
	}
	var files [][]byte
	for len(data) > 0 {
		n := binary.LittleEndian.Uint32(data)
		files = append(files, data[4:4+n])
		data = data[4+n:]
	}
	return files
}

func writeCorpus(path string, files [][]byte) {
	var buf bytes.Buffer
	for _, f := range files {
		var l [4]byte
		binary.LittleEndian.PutUint32(l[:], uint32(len(f)))
		buf.Write(l[:])
		buf.Write(f)
	}
	if err := os.WriteFile(path, buf.Bytes(), 0o644); err != nil {
		panic(err)
	}
}

func packCmd(outPath string, paths []string) {
	var files [][]byte
	for _, p := range paths {
		b, err := os.ReadFile(p)
		if err != nil {
			panic(err)
		}
		files = append(files, b)
	}
	writeCorpus(outPath, files)
}

// record prints, for every corpus entry: DecodeConfig, Decode, image.Decode
// and the default-level encoding of the decoded image.
func record(path string) {
	for i, data := range readCorpus(path) {
		cols := []string{strconv.Itoa(i), cfgStr(data)}
		// Refuse to decode images whose allocation would exhaust memory
		// (Go's own fuzz test does the same).
		if cfg, err := png.DecodeConfig(bytes.NewReader(data)); err == nil && cfg.Width*cfg.Height > 4e6 {
			cols = append(cols, "skip")
			fmt.Fprintln(out, strings.Join(cols, "\t"))
			continue
		}
		m, d := decStr(data)
		cols = append(cols, d, regStr(data))
		if m != nil {
			s, _ := encStr(m, png.DefaultCompression)
			cols = append(cols, s)
		}
		fmt.Fprintln(out, strings.Join(cols, "\t"))
	}
}

func writeChunk(w *bytes.Buffer, name string, data []byte) {
	var l [4]byte
	binary.BigEndian.PutUint32(l[:], uint32(len(data)))
	w.Write(l[:])
	w.WriteString(name)
	w.Write(data)
	crc := crc32.NewIEEE()
	crc.Write([]byte(name))
	crc.Write(data)
	binary.BigEndian.PutUint32(l[:], crc.Sum32())
	w.Write(l[:])
}

func channels(ct int) int {
	switch ct {
	case 2:
		return 3
	case 4:
		return 2
	case 6:
		return 4
	}
	return 1
}

var interlacing = []struct{ xf, yf, xo, yo int }{
	{8, 8, 0, 0}, {8, 8, 4, 0}, {4, 8, 0, 4}, {4, 4, 2, 0}, {2, 4, 0, 2}, {2, 2, 1, 0}, {1, 2, 0, 1},
}

// mkpng writes a random PNG file: every colour type and bit depth,
// interlacing, random filters, PLTE/tRNS variants, IDAT splitting,
// ancillary chunks and a sprinkling of structural errors.
func mkpng(r *rng) []byte {
	type combo struct{ ct, depth int }
	valid := []combo{{0, 1}, {0, 2}, {0, 4}, {0, 8}, {0, 16}, {2, 8}, {2, 16}, {3, 1}, {3, 2}, {3, 4}, {3, 8}, {4, 8}, {4, 16}, {6, 8}, {6, 16}}
	c := valid[r.intn(len(valid))]
	if r.intn(30) == 0 {
		c = combo{r.intn(8), []int{0, 1, 2, 3, 4, 5, 8, 16, 32}[r.intn(9)]}
	}
	w, h := 1+r.intn(12), 1+r.intn(12)
	if r.intn(8) == 0 {
		w, h = 1+r.intn(70), 1+r.intn(70)
	}
	interlace := r.intn(2)
	if r.intn(50) == 0 {
		interlace = 2 + r.intn(3)
	}
	var buf bytes.Buffer
	buf.WriteString("\x89PNG\r\n\x1a\n")
	ihdr := make([]byte, 13)
	binary.BigEndian.PutUint32(ihdr[0:], uint32(w))
	binary.BigEndian.PutUint32(ihdr[4:], uint32(h))
	if r.intn(60) == 0 {
		switch r.intn(4) {
		case 0:
			binary.BigEndian.PutUint32(ihdr[0:], 0)
		case 1:
			binary.BigEndian.PutUint32(ihdr[4:], 0x80000000)
		case 2:
			binary.BigEndian.PutUint32(ihdr[0:], 0x7fffffff)
			binary.BigEndian.PutUint32(ihdr[4:], 0x7fffffff)
		default:
			binary.BigEndian.PutUint32(ihdr[0:], 0x40000)
			binary.BigEndian.PutUint32(ihdr[4:], 0x40000)
		}
	}
	ihdr[8] = byte(c.depth)
	ihdr[9] = byte(c.ct)
	ihdr[12] = byte(interlace)
	if r.intn(80) == 0 {
		ihdr[10] = 1
	}
	if r.intn(80) == 0 {
		ihdr[11] = 1
	}
	ihdrData := ihdr
	if r.intn(80) == 0 {
		ihdrData = append(ihdrData, 0)
	}
	writeChunk(&buf, "IHDR", ihdrData)
	if r.intn(4) == 0 {
		writeChunk(&buf, "gAMA", []byte{r.byte(), r.byte(), r.byte(), r.byte()})
	}

	// Raw (filtered) image data.
	bitsPP := c.depth * channels(c.ct)
	noFilter := r.intn(3) == 0
	var raw []byte
	type pass struct{ w, h int }
	var passes []pass
	if interlace == 1 {
		for _, p := range interlacing {
			pw := (w - p.xo + p.xf - 1) / p.xf
			ph := (h - p.yo + p.yf - 1) / p.yf
			passes = append(passes, pass{pw, ph})
		}
	} else {
		passes = []pass{{w, h}}
	}
	np := 0
	if c.ct == 3 {
		max := 256
		if c.depth < 8 {
			max = 1 << c.depth
		}
		np = 1 + r.intn(max)
	}
	content := r.intn(3)
	badFilter := r.intn(40) == 0
	for _, p := range passes {
		if p.w <= 0 || p.h <= 0 {
			continue
		}
		rowBytes := (bitsPP*p.w + 7) / 8
		for y := 0; y < p.h; y++ {
			ft := byte(0)
			if !noFilter {
				ft = byte(r.intn(5))
				if badFilter && r.intn(4) == 0 {
					ft = r.byte()
				}
			}
			raw = append(raw, ft)
			for x := 0; x < rowBytes; x++ {
				var v byte
				switch content {
				case 0:
					v = r.byte()
				case 1:
					v = byte(x*3 + y*5)
				default:
					if r.intn(6) == 0 {
						v = r.byte()
					}
				}
				if c.ct == 3 && c.depth == 8 && np > 0 && r.intn(20) != 0 {
					v = byte(int(v) % np)
				}
				raw = append(raw, v)
			}
		}
	}
	if len(raw) > 1 && r.intn(20) == 0 {
		raw = raw[:r.intn(len(raw))]
	}
	if r.intn(25) == 0 {
		raw = append(raw, 0, r.byte(), r.byte())
	}

	// PLTE and tRNS.
	var plte []byte
	if c.ct == 3 || ((c.ct == 2 || c.ct == 6) && r.intn(5) == 0) || r.intn(40) == 0 {
		n := np
		if n == 0 {
			n = 1 + r.intn(256)
		}
		if r.intn(30) == 0 {
			n += 1 + r.intn(300)
		}
		plte = make([]byte, 3*n)
		for i := range plte {
			plte[i] = r.byte()
		}
		if r.intn(30) == 0 {
			plte = plte[:len(plte)-1]
		}
	}
	var trns []byte
	if r.intn(3) == 0 && (c.ct <= 3 || r.intn(8) == 0) {
		switch c.ct {
		case 0:
			trns = []byte{r.byte(), r.byte()}
			if noFilter && len(raw) > 2 && r.intn(2) == 0 {
				switch {
				case c.depth < 8:
					trns = []byte{0, raw[1] >> (8 - c.depth)}
				case c.depth == 8:
					trns = []byte{0, raw[1]}
				default:
					trns = []byte{raw[1], raw[2]}
				}
			}
		case 2:
			trns = make([]byte, 6)
			for i := range trns {
				trns[i] = r.byte()
			}
			if noFilter && len(raw) > 6 && r.intn(2) == 0 {
				if c.depth == 8 {
					trns = []byte{0, raw[1], 0, raw[2], 0, raw[3]}
				} else {
					trns = append([]byte(nil), raw[1:7]...)
				}
			}
		case 3:
			n := r.intn(np + 3)
			if r.intn(20) == 0 {
				n = 250 + r.intn(10)
			}
			trns = make([]byte, n)
			for i := range trns {
				trns[i] = r.byte()
			}
		default:
			trns = make([]byte, r.intn(8))
		}
		if r.intn(20) == 0 && len(trns) > 0 {
			trns = trns[:len(trns)-1]
		}
	}
	trnsPos := 0
	if r.intn(20) == 0 {
		trnsPos = 1 + r.intn(2)
	}
	if trns != nil && trnsPos == 1 {
		writeChunk(&buf, "tRNS", trns)
	}
	if plte != nil && !(c.ct == 3 && r.intn(40) == 0) {
		writeChunk(&buf, "PLTE", plte)
	}
	if trns != nil && trnsPos == 0 {
		writeChunk(&buf, "tRNS", trns)
	}
	if r.intn(5) == 0 {
		writeChunk(&buf, "tEXt", []byte("Comment\x00go-png oracle"))
	}

	// zlib stream split into IDAT chunks.
	var z bytes.Buffer
	zw, err := zlib.NewWriterLevel(&z, r.intn(11)-1)
	if err != nil {
		panic(err)
	}
	zw.Write(raw)
	zw.Close()
	zb := z.Bytes()
	if r.intn(30) == 0 {
		zb = append(zb, r.byte(), r.byte(), r.byte())
	}
	if r.intn(40) == 0 && len(zb) > 4 {
		zb[len(zb)-1-r.intn(4)] ^= 1 << r.intn(8)
	}
	if r.intn(30) != 0 {
		for len(zb) > 0 {
			n := len(zb)
			if r.intn(2) == 0 {
				n = 1 + r.intn(len(zb))
			}
			writeChunk(&buf, "IDAT", zb[:n])
			zb = zb[n:]
			if r.intn(40) == 0 {
				writeChunk(&buf, "IDAT", nil)
			}
			if len(zb) > 0 && r.intn(60) == 0 {
				writeChunk(&buf, "tEXt", []byte("x\x00y"))
			}
		}
	}
	switch r.intn(20) {
	case 0:
		writeChunk(&buf, "IDAT", nil)
	case 1:
		writeChunk(&buf, "IDAT", []byte{r.byte(), r.byte()})
	case 2:
		writeChunk(&buf, "tRNS", []byte{0, 0})
	}
	if trns != nil && trnsPos == 2 {
		writeChunk(&buf, "tRNS", trns)
	}
	switch r.intn(30) {
	case 0:
		// No IEND.
	case 1:
		writeChunk(&buf, "IEND", []byte{0})
	default:
		writeChunk(&buf, "IEND", nil)
	}
	if r.intn(20) == 0 {
		buf.Write([]byte("trailing garbage"))
	}
	return buf.Bytes()
}

func mkpngCmd(seed0, n int, outPath string) {
	var files [][]byte
	for i := seed0; i < seed0+n; i++ {
		r := &rng{s: uint64(i)*0x9e3779b97f4a7c15 + 777}
		files = append(files, mkpng(r))
	}
	writeCorpus(outPath, files)
}

type chunkPos struct{ off, length int } // off of the length field; data length

func chunks(b []byte) []chunkPos {
	var cs []chunkPos
	off := 8
	for off+12 <= len(b) {
		l := int(binary.BigEndian.Uint32(b[off:]))
		if l < 0 || off+12+l > len(b) {
			break
		}
		cs = append(cs, chunkPos{off, l})
		off += 12 + l
	}
	return cs
}

func fixCRC(b []byte, c chunkPos) {
	crc := crc32.ChecksumIEEE(b[c.off+4 : c.off+8+c.length])
	binary.BigEndian.PutUint32(b[c.off+8+c.length:], crc)
}

func mutate(r *rng, in []byte) []byte {
	b := append([]byte(nil), in...)
	nm := 1 + r.intn(3)
	for k := 0; k < nm; k++ {
		if len(b) == 0 {
			break
		}
		cs := chunks(b)
		op := r.intn(10)
		if len(cs) == 0 && op >= 2 {
			op = r.intn(2)
		}
		switch op {
		case 0:
			b[r.intn(len(b))] ^= 1 << r.intn(8)
		case 1:
			b = b[:r.intn(len(b))]
		case 2:
			c := cs[r.intn(len(cs))]
			if c.length > 0 {
				b[c.off+8+r.intn(c.length)] = r.byte()
				fixCRC(b, c)
			}
		case 3:
			c := cs[r.intn(len(cs))]
			binary.BigEndian.PutUint32(b[c.off:], uint32(c.length+r.intn(5)-2))
		case 4:
			c := cs[r.intn(len(cs))]
			b = append(b[:c.off:c.off], b[c.off+12+c.length:]...)
		case 5:
			c := cs[r.intn(len(cs))]
			dup := append([]byte(nil), b[c.off:c.off+12+c.length]...)
			b = append(b[:c.off:c.off], append(dup, b[c.off:]...)...)
		case 6:
			if len(cs) >= 2 {
				i := r.intn(len(cs) - 1)
				c0, c1 := cs[i], cs[i+1]
				a := append([]byte(nil), b[c0.off:c0.off+12+c0.length]...)
				bb := append([]byte(nil), b[c1.off:c1.off+12+c1.length]...)
				rest := append([]byte(nil), b[c1.off+12+c1.length:]...)
				b = append(append(append(b[:c0.off:c0.off], bb...), a...), rest...)
			}
		case 7:
			names := []string{"IHDR", "PLTE", "tRNS", "IDAT", "IEND", "tEXt", "gAMA", "IDAT"}
			var nb bytes.Buffer
			data := make([]byte, r.intn(8))
			for i := range data {
				data[i] = r.byte()
			}
			writeChunk(&nb, names[r.intn(len(names))], data)
			c := cs[r.intn(len(cs))]
			b = append(b[:c.off:c.off], append(nb.Bytes(), b[c.off:]...)...)
		case 8:
			c := cs[0]
			if string(b[c.off+4:c.off+8]) == "IHDR" && c.length >= 13 {
				d := b[c.off+8:]
				switch r.intn(5) {
				case 0:
					d[8] = []byte{1, 2, 4, 8, 16}[r.intn(5)]
				case 1:
					d[9] = []byte{0, 2, 3, 4, 6}[r.intn(5)]
				case 2:
					d[12] ^= 1
				case 3:
					d[3] = byte(int(d[3]) + r.intn(5) - 2)
				default:
					d[7] = byte(int(d[7]) + r.intn(5) - 2)
				}
				fixCRC(b, c)
			}
		default:
			var idats []chunkPos
			for _, c := range cs {
				if string(b[c.off+4:c.off+8]) == "IDAT" && c.length > 0 {
					idats = append(idats, c)
				}
			}
			if len(idats) > 0 {
				c := idats[r.intn(len(idats))]
				b[c.off+8+r.intn(c.length)] ^= 1 << r.intn(8)
				fixCRC(b, c)
			}
		}
	}
	return b
}

func mutateCmd(seed0, n int, inPath, outPath string) {
	seeds := readCorpus(inPath)
	var files [][]byte
	for i := seed0; i < seed0+n; i++ {
		r := &rng{s: uint64(i)*0x9e3779b97f4a7c15 + 4242}
		files = append(files, mutate(r, seeds[r.intn(len(seeds))]))
	}
	writeCorpus(outPath, files)
}
