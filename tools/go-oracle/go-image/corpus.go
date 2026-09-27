package main

// Corpus-based differential records (added by the adversarial verification
// pass). Inputs are stored in a simple binary corpus file so that the Rust
// tests do not need to re-implement the generators:
//
//	"GOIMGC1\n" { u32le len(name) name u32le len(data) data }*
//
// Commands:
//
//	go-image pack <out.bin> <root> <listfile>      # pack files into a corpus
//	go-image record <corpus.bin>                   # one TSV record per entry
//	go-image mkjpeg <seed0> <n> <outdir>           # random-structure baseline JPEGs
//	go-image mutate <n> <seed0> <out.bin> <in.bin> # structure-aware mutations
//	go-image encsweep <corpus.bin>                 # encode decoded images at q1..100

import (
	"bytes"
	"encoding/binary"
	"fmt"
	"image"
	"image/color/palette"
	"image/draw"
	"image/jpeg"
	"os"
	"path/filepath"
	"strings"
)

const corpusMagic = "GOIMGC1\n"

type corpusEntry struct {
	name string
	data []byte
}

func readCorpus(path string) []corpusEntry {
	b, err := os.ReadFile(path)
	if err != nil {
		panic(err)
	}
	if !bytes.HasPrefix(b, []byte(corpusMagic)) {
		panic("bad corpus magic: " + path)
	}
	b = b[len(corpusMagic):]
	var es []corpusEntry
	for len(b) > 0 {
		n := int(binary.LittleEndian.Uint32(b))
		name := string(b[4 : 4+n])
		b = b[4+n:]
		m := int(binary.LittleEndian.Uint32(b))
		data := b[4 : 4+m]
		b = b[4+m:]
		es = append(es, corpusEntry{name, data})
	}
	return es
}

func writeCorpus(path string, es []corpusEntry) {
	var buf bytes.Buffer
	buf.WriteString(corpusMagic)
	for _, e := range es {
		buf.Write(binary.LittleEndian.AppendUint32(nil, uint32(len(e.name))))
		buf.WriteString(e.name)
		buf.Write(binary.LittleEndian.AppendUint32(nil, uint32(len(e.data))))
		buf.Write(e.data)
	}
	if err := os.WriteFile(path, buf.Bytes(), 0o644); err != nil {
		panic(err)
	}
}

func packCorpus(out, root, listfile string) {
	list, err := os.ReadFile(listfile)
	if err != nil {
		panic(err)
	}
	var es []corpusEntry
	for _, rel := range strings.Split(strings.TrimSpace(string(list)), "\n") {
		if rel == "" {
			continue
		}
		data, err := os.ReadFile(filepath.Join(root, rel))
		if err != nil {
			panic(err)
		}
		es = append(es, corpusEntry{rel, data})
	}
	writeCorpus(out, es)
}

// imgDigest16 is imgDigest with 16-hex-digit hashes.
func imgDigest16(m image.Image) string {
	switch m := m.(type) {
	case *image.Gray:
		return fmt.Sprintf("Gray %s %d %d %s", rectStr(m.Rect), m.Stride, len(m.Pix), sha16(m.Pix))
	case *image.RGBA:
		return fmt.Sprintf("RGBA %s %d %d %s", rectStr(m.Rect), m.Stride, len(m.Pix), sha16(m.Pix))
	case *image.CMYK:
		return fmt.Sprintf("CMYK %s %d %d %s", rectStr(m.Rect), m.Stride, len(m.Pix), sha16(m.Pix))
	case *image.YCbCr:
		return fmt.Sprintf("YCbCr %s %d %d %d %d %d %d %s %s %s", rectStr(m.Rect), int(m.SubsampleRatio),
			m.YStride, m.CStride, len(m.Y), len(m.Cb), len(m.Cr), sha16(m.Y), sha16(m.Cb), sha16(m.Cr))
	case nil:
		return "nil"
	}
	return fmt.Sprintf("other %T", m)
}

func encSha16(m image.Image, q int) string {
	var buf bytes.Buffer
	if err := jpeg.Encode(&buf, m, &jpeg.Options{Quality: q}); err != nil {
		return "err:" + err.Error()
	}
	return sha16(buf.Bytes())
}

type subImager interface {
	SubImage(r image.Rectangle) image.Image
}

// record computes the differential record for one input. The fields are:
// name, sha16(input), jpeg.DecodeConfig, image.DecodeConfig, jpeg.Decode
// digest, image.Decode digest, and (when decoding succeeded) derived values:
// encodings of the decoded image, draw conversions (fast paths and generic
// paths), a sub-image with odd offsets, and a Floyd-Steinberg paletted
// conversion for small images.
func record(name string, data []byte) string {
	f := []string{name, sha16(data)}
	cfg, err := jpeg.DecodeConfig(bytes.NewReader(data))
	large := false
	if err != nil {
		f = append(f, "err:"+err.Error())
	} else {
		f = append(f, fmt.Sprintf("%s,%d,%d", modelName(cfg.ColorModel), cfg.Width, cfg.Height))
		large = cfg.Width*cfg.Height > 4<<20
	}
	cfg2, fname, err := image.DecodeConfig(bytes.NewReader(data))
	if err != nil {
		f = append(f, "err:"+err.Error())
	} else {
		f = append(f, fmt.Sprintf("%s:%s,%d,%d", fname, modelName(cfg2.ColorModel), cfg2.Width, cfg2.Height))
	}
	if large {
		f = append(f, "skip")
		return strings.Join(f, "\t")
	}
	m, err := jpeg.Decode(bytes.NewReader(data))
	if err != nil {
		f = append(f, "err:"+err.Error())
	} else {
		f = append(f, imgDigest16(m))
	}
	m2, fname2, err2 := image.Decode(bytes.NewReader(data))
	if err2 != nil {
		f = append(f, "err:"+err2.Error())
	} else {
		f = append(f, fname2+":"+imgDigest16(m2))
	}
	if err != nil {
		return strings.Join(f, "\t")
	}
	b := m.Bounds()

	// Encodings of the decoded image.
	var qs []string
	for _, q := range []int{1, 50, 75, 100} {
		qs = append(qs, encSha16(m, q))
	}
	f = append(f, strings.Join(qs, ","))

	// draw.Draw into RGBA (Src): DrawYCbCr / drawGray / drawCMYK / copy
	// fast paths, or drawRGBA for 4:1:1 and 4:1:0.
	rgba := image.NewRGBA(b)
	draw.Draw(rgba, b, m, b.Min, draw.Src)
	f = append(f, sha16(rgba.Pix), encSha16(rgba, 75))

	// DrawMask Over with an Alpha mask onto a patterned RGBA:
	// drawGrayMaskOver / drawRGBA64ImageMaskOver / drawRGBAMaskOver.
	pat := image.NewRGBA(b)
	for i := range pat.Pix {
		pat.Pix[i] = byte(i*7 + 3)
	}
	mask := image.NewAlpha(b)
	for i := range mask.Pix {
		mask.Pix[i] = byte(i*13 + 1)
	}
	draw.DrawMask(pat, b, m, b.Min, mask, b.Min, draw.Over)
	f = append(f, sha16(pat.Pix))

	// draw.Draw into NRGBA (Src): the FALLBACK1.17 generic path.
	nrgba := image.NewNRGBA(b)
	draw.Draw(nrgba, b, m, b.Min, draw.Src)
	f = append(f, sha16(nrgba.Pix))

	// A sub-image with odd offsets: encoded, and drawn into RGBA.
	r2 := image.Rect(b.Min.X+1, b.Min.Y+3, b.Max.X-2, b.Max.Y)
	sub := m.(subImager).SubImage(r2)
	sb := sub.Bounds()
	subRGBA := image.NewRGBA(sb)
	draw.Draw(subRGBA, sb, sub, sb.Min, draw.Src)
	f = append(f, rectStr(sb), encSha16(sub, 75), sha16(subRGBA.Pix))

	// Floyd-Steinberg onto a Plan9 paletted image (small images only).
	if b.Dx()*b.Dy() <= 64*64 {
		p := image.NewPaletted(b, palette.Plan9)
		draw.FloydSteinberg.Draw(p, b, m, b.Min)
		f = append(f, sha16(p.Pix))
	} else {
		f = append(f, "-")
	}
	return strings.Join(f, "\t")
}

func recordCorpus(path string) {
	for _, e := range readCorpus(path) {
		fmt.Fprintln(out, record(e.name, e.data))
	}
}

// encsweep encodes every successfully decoded image at every quality
// 1..100 (odd sizes and every decoder output type) and prints the hashes.
func encsweep(path string) {
	for _, e := range readCorpus(path) {
		m, err := jpeg.Decode(bytes.NewReader(e.data))
		if err != nil {
			continue
		}
		b := m.Bounds()
		if b.Dx()*b.Dy() > 1<<20 {
			continue
		}
		var hs []string
		for q := 1; q <= 100; q++ {
			hs = append(hs, encSha16(m, q)[:8])
		}
		fmt.Fprintf(out, "%s\t%s\n", e.name, strings.Join(hs, ","))
	}
}

// ---------------------------------------------------------------------------
// mkjpeg: a baseline/extended-sequential JPEG writer with random structure
// (component count, sampling factors, component ids, table ids, quantization
// precision, restart intervals, scan layout, APP0/APP14 markers). The
// entropy-coded data holds random coefficients laid out exactly in the order
// Go's decoder reads them, so most files decode successfully.

var mkUnzig = [64]int{
	0, 1, 8, 16, 9, 2, 3, 10,
	17, 24, 32, 25, 18, 11, 4, 5,
	12, 19, 26, 33, 40, 48, 41, 34,
	27, 20, 13, 6, 7, 14, 21, 28,
	35, 42, 49, 56, 57, 50, 43, 36,
	29, 22, 15, 23, 30, 37, 44, 51,
	58, 59, 52, 45, 38, 31, 39, 46,
	53, 60, 61, 54, 47, 55, 62, 63,
}

type mkSpec struct {
	count [16]byte
	value []byte
}

// The K.3 tables (as in image/jpeg/writer.go).
var mkSpecs = [4]mkSpec{
	{[16]byte{0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0},
		[]byte{0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11}},
	{[16]byte{0, 2, 1, 3, 3, 2, 4, 3, 5, 5, 4, 4, 0, 0, 1, 125},
		[]byte{
			0x01, 0x02, 0x03, 0x00, 0x04, 0x11, 0x05, 0x12, 0x21, 0x31, 0x41, 0x06, 0x13, 0x51, 0x61, 0x07,
			0x22, 0x71, 0x14, 0x32, 0x81, 0x91, 0xa1, 0x08, 0x23, 0x42, 0xb1, 0xc1, 0x15, 0x52, 0xd1, 0xf0,
			0x24, 0x33, 0x62, 0x72, 0x82, 0x09, 0x0a, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x25, 0x26, 0x27, 0x28,
			0x29, 0x2a, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49,
			0x4a, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69,
			0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89,
			0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7,
			0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3, 0xc4, 0xc5,
			0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xe1, 0xe2,
			0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8,
			0xf9, 0xfa,
		}},
	{[16]byte{0, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0},
		[]byte{0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11}},
	{[16]byte{0, 2, 1, 2, 4, 4, 3, 4, 7, 5, 4, 4, 0, 1, 2, 119},
		[]byte{
			0x00, 0x01, 0x02, 0x03, 0x11, 0x04, 0x05, 0x21, 0x31, 0x06, 0x12, 0x41, 0x51, 0x07, 0x61, 0x71,
			0x13, 0x22, 0x32, 0x81, 0x08, 0x14, 0x42, 0x91, 0xa1, 0xb1, 0xc1, 0x09, 0x23, 0x33, 0x52, 0xf0,
			0x15, 0x62, 0x72, 0xd1, 0x0a, 0x16, 0x24, 0x34, 0xe1, 0x25, 0xf1, 0x17, 0x18, 0x19, 0x1a, 0x26,
			0x27, 0x28, 0x29, 0x2a, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48,
			0x49, 0x4a, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68,
			0x69, 0x6a, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87,
			0x88, 0x89, 0x8a, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x98, 0x99, 0x9a, 0xa2, 0xa3, 0xa4, 0xa5,
			0xa6, 0xa7, 0xa8, 0xa9, 0xaa, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xc2, 0xc3,
			0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda,
			0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8,
			0xf9, 0xfa,
		}},
}

// mkLUT maps a value to nBits<<24 | code.
func mkLUT(s mkSpec) [256]uint32 {
	var h [256]uint32
	code, k := uint32(0), 0
	for i := 0; i < 16; i++ {
		nBits := uint32(i+1) << 24
		for j := byte(0); j < s.count[i]; j++ {
			h[s.value[k]] = nBits | code
			code++
			k++
		}
		code <<= 1
	}
	return h
}

type mkBits struct {
	buf   []byte
	bits  uint32
	nBits uint32
}

func (w *mkBits) emit(bits, nBits uint32) {
	nBits += w.nBits
	bits <<= 32 - nBits
	bits |= w.bits
	for nBits >= 8 {
		b := uint8(bits >> 24)
		w.buf = append(w.buf, b)
		if b == 0xff {
			w.buf = append(w.buf, 0x00)
		}
		bits <<= 8
		nBits -= 8
	}
	w.bits, w.nBits = bits, nBits
}

// flush pads the last partial byte with 1 bits.
func (w *mkBits) flush() {
	if w.nBits > 0 {
		k := 8 - w.nBits
		w.emit(1<<k-1, k)
	}
	w.bits, w.nBits = 0, 0
}

func (w *mkBits) huff(lut *[256]uint32, v int) {
	x := lut[v]
	if x == 0 {
		// Symbol not in the table: emit nothing (only happens with
		// deliberately permuted tables; keeps the stream deterministic).
		return
	}
	w.emit(x&(1<<24-1), x>>24)
}

func mkBitCount(a int32) uint32 {
	n := uint32(0)
	for a > 0 {
		n++
		a >>= 1
	}
	return n
}

func (w *mkBits) huffRLE(lut *[256]uint32, run int, v int32) {
	a, b := v, v
	if a < 0 {
		a, b = -v, v-1
	}
	nBits := mkBitCount(a)
	w.huff(lut, run<<4|int(nBits))
	if nBits > 0 {
		w.emit(uint32(b)&(1<<nBits-1), nBits)
	}
}

type mkComp struct {
	id     byte
	h, v   int
	tq     int
	td, ta int
}

type mkGen struct {
	r       *rng
	buf     []byte
	comps   []mkComp
	w, h    int
	maxH    int
	maxV    int
	ri      int
	luts    [2][4][256]uint32 // [class][id]
	defined [2][4]bool
}

func (g *mkGen) marker(m byte, payload []byte) {
	g.buf = append(g.buf, 0xff, m)
	n := len(payload) + 2
	g.buf = append(g.buf, byte(n>>8), byte(n))
	g.buf = append(g.buf, payload...)
}

func (g *mkGen) dqt(tq int, sixteen bool, vals [64]int) {
	var p []byte
	if sixteen {
		p = append(p, 0x10|byte(tq))
		for _, v := range vals {
			p = append(p, byte(v>>8), byte(v))
		}
	} else {
		p = append(p, byte(tq))
		for _, v := range vals {
			p = append(p, byte(v))
		}
	}
	g.marker(0xdb, p)
}

func (g *mkGen) dht(class, id int, s mkSpec) {
	p := []byte{byte(class<<4 | id)}
	p = append(p, s.count[:]...)
	p = append(p, s.value...)
	g.marker(0xc4, p)
	g.luts[class][id] = mkLUT(s)
	g.defined[class][id] = true
}

// permuteSpec keeps the code lengths of s but shuffles which value gets
// which code.
func (g *mkGen) permuteSpec(s mkSpec) mkSpec {
	v := append([]byte(nil), s.value...)
	for i := len(v) - 1; i > 0; i-- {
		j := g.r.intn(i + 1)
		v[i], v[j] = v[j], v[i]
	}
	return mkSpec{s.count, v}
}

func (g *mkGen) block(w *mkBits, dc, ac *[256]uint32, pred *int32, density int) {
	r := g.r
	var val int32
	switch r.intn(4) {
	case 0:
		val = *pred
	case 1:
		val = int32(r.intn(64)) - 32
	default:
		val = int32(r.intn(2047)) - 1023
	}
	diff := val - *pred
	*pred = val
	w.huffRLE(dc, 0, diff)
	run := 0
	for zig := 1; zig < 64; zig++ {
		nz := false
		switch density {
		case 0:
		case 1:
			nz = r.intn(20) == 0
		case 2:
			nz = r.intn(4) == 0
		case 3:
			nz = zig < 10 && r.intn(2) == 0
		default:
			nz = r.intn(3) != 0
		}
		if !nz {
			run++
			continue
		}
		size := 1 + r.intn(10)
		if r.intn(3) != 0 {
			size = 1 + r.intn(4)
		}
		mag := int32(1<<(size-1)) + int32(r.intn(1<<(size-1)))
		if r.intn(2) == 0 {
			mag = -mag
		}
		for run > 15 {
			w.huff(ac, 0xf0)
			run -= 16
		}
		w.huffRLE(ac, run, mag)
		run = 0
	}
	if run > 0 {
		if run > 16 && r.intn(8) == 0 {
			// A ZRL immediately followed by EOB.
			w.huff(ac, 0xf0)
		}
		w.huff(ac, 0x00)
	}
}

// scan writes one SOS segment and its entropy-coded data, traversing blocks
// exactly as Go's decoder (image/jpeg/scan.go) reads them.
func (g *mkGen) scan(sc []int, rstMode int) {
	r := g.r
	p := []byte{byte(len(sc))}
	for _, ci := range sc {
		c := g.comps[ci]
		p = append(p, c.id, byte(c.td<<4|c.ta))
	}
	if r.intn(10) == 0 {
		p = append(p, byte(r.intn(256)), byte(r.intn(256)), byte(r.intn(256)))
	} else {
		p = append(p, 0x00, 0x3f, 0x00)
	}
	g.marker(0xda, p)

	var w mkBits
	nComp := len(g.comps)
	maxH, maxV := g.maxH, g.maxV
	if nComp == 1 {
		maxH, maxV = 1, 1
	}
	mxx := (g.w + 8*maxH - 1) / (8 * maxH)
	myy := (g.h + 8*maxV - 1) / (8 * maxV)
	var preds [4]int32
	density := r.intn(5)
	mcu, rst := 0, 0
	blockCount := 0
	for my := 0; my < myy; my++ {
		for mx := 0; mx < mxx; mx++ {
			for _, ci := range sc {
				c := g.comps[ci]
				hi, vi := c.h, c.v
				if nComp == 1 {
					hi, vi = 1, 1
				}
				for j := 0; j < hi*vi; j++ {
					if len(sc) == 1 {
						q := mxx * hi
						bx := blockCount % q
						by := blockCount / q
						blockCount++
						if bx*8 >= g.w || by*8 >= g.h {
							continue
						}
					}
					g.block(&w, &g.luts[0][c.td], &g.luts[1][c.ta], &preds[ci], density)
				}
			}
			mcu++
			if g.ri > 0 && mcu%g.ri == 0 && mcu < mxx*myy {
				w.flush()
				switch {
				case rstMode == 1 && r.intn(30) == 0:
					// Junk before the marker: findRST resynchronizes.
					w.buf = append(w.buf, byte(r.intn(255)), 0xff, 0x00, 0xff, 0xff)
					w.buf = append(w.buf, 0xff, byte(0xd0+rst%8))
				case rstMode == 1 && r.intn(40) == 0:
					// Wrong RST number.
					w.buf = append(w.buf, 0xff, byte(0xd0+(rst+1+r.intn(7))%8))
				case rstMode == 1 && r.intn(40) == 0:
					// Missing marker.
				default:
					w.buf = append(w.buf, 0xff, byte(0xd0+rst%8))
				}
				rst++
				preds = [4]int32{}
			}
		}
	}
	w.flush()
	g.buf = append(g.buf, w.buf...)
}

func mkjpeg(seed uint64) []byte {
	r := &rng{seed*2654435761 + 12345}
	g := &mkGen{r: r}
	g.buf = []byte{0xff, 0xd8}

	nComp := 3
	switch k := r.intn(10); {
	case k < 3:
		nComp = 1
	case k < 8:
		nComp = 3
	default:
		nComp = 4
	}
	dim := func() int {
		switch r.intn(40) {
		case 0:
			return 0
		case 1, 2, 3, 4, 5:
			return 1 + r.intn(300)
		}
		return 1 + r.intn(70)
	}
	g.w, g.h = dim(), dim()

	// Sampling factors.
	facs := []int{1, 1, 1, 2, 2, 4}
	hv := make([][2]int, nComp)
	switch nComp {
	case 1:
		hv[0] = [2]int{facs[r.intn(len(facs))], facs[r.intn(len(facs))]}
	case 3:
		switch r.intn(4) {
		case 0: // Standard ratios.
			std := [][2]int{{1, 1}, {2, 1}, {2, 2}, {1, 2}, {4, 1}, {4, 2}}
			s := std[r.intn(len(std))]
			hv[0] = s
			hv[1] = [2]int{1, 1}
			hv[2] = [2]int{1, 1}
		default:
			// Random factors, usually within the 10-blocks-per-MCU limit.
			for try := 0; try < 8; try++ {
				total := 0
				for i := range hv {
					hv[i] = [2]int{facs[r.intn(len(facs))], facs[r.intn(len(facs))]}
					total += hv[i][0] * hv[i][1]
				}
				if total <= 10 || r.intn(5) == 0 {
					break
				}
			}
		}
	case 4:
		switch r.intn(4) {
		case 0:
			hv = [][2]int{{1, 1}, {1, 1}, {1, 1}, {1, 1}}
		case 1, 2:
			hv = [][2]int{{2, 2}, {1, 1}, {1, 1}, {2, 2}}
		default:
			for i := range hv {
				hv[i] = [2]int{facs[r.intn(len(facs))], facs[r.intn(len(facs))]}
			}
		}
	}
	if r.intn(50) == 0 {
		hv[r.intn(nComp)] = [2]int{3, 1 + r.intn(2)}
	}

	// Component ids.
	ids := []byte{1, 2, 3, 4}
	switch r.intn(8) {
	case 0:
		ids = []byte{'R', 'G', 'B', 'A'}
	case 1:
		ids = []byte{0, 1, 2, 3}
	case 2:
		ids = []byte{byte(r.intn(256)), byte(r.intn(256)), byte(r.intn(256)), byte(r.intn(256))}
	case 3:
		ids = []byte{'C', 'M', 'Y', 'K'}
	}

	extended := r.intn(3) == 0
	nq := 1 + r.intn(4)
	g.comps = make([]mkComp, nComp)
	for i := range g.comps {
		c := &g.comps[i]
		c.id = ids[i]
		c.h, c.v = hv[i][0], hv[i][1]
		c.tq = r.intn(nq)
		if extended {
			c.td, c.ta = r.intn(4), r.intn(4)
		} else if i > 0 && r.intn(3) != 0 {
			c.td, c.ta = 1, 1
		}
		if c.h > g.maxH {
			g.maxH = c.h
		}
		if c.v > g.maxV {
			g.maxV = c.v
		}
	}

	// APP markers.
	if r.intn(2) == 0 {
		g.marker(0xe0, []byte("JFIF\x00\x01\x02\x00\x00\x01\x00\x01\x00\x00"))
	}
	if r.intn(3) == 0 || (nComp == 4 && r.intn(6) != 0) {
		t := byte(r.intn(3))
		if r.intn(10) == 0 {
			t = byte(r.intn(256))
		}
		g.marker(0xee, []byte{'A', 'd', 'o', 'b', 'e', 0, 100, 0, 0, 0, 0, t})
	}
	if r.intn(4) == 0 {
		g.marker(0xfe, []byte("comment"))
	}
	if r.intn(6) == 0 {
		g.marker(0xe1, []byte("Exif\x00\x00junk"))
	}

	// Quantization tables.
	for tq := 0; tq < nq; tq++ {
		var vals [64]int
		mode := r.intn(6)
		for i := range vals {
			switch mode {
			case 0:
				vals[i] = 1
			case 1:
				vals[i] = 1 + r.intn(255)
			case 2:
				vals[i] = 1 + r.intn(16)
			case 3:
				vals[i] = r.intn(65536)
			default:
				vals[i] = 2 + i/2
			}
		}
		g.dqt(tq, mode == 3 || r.intn(8) == 0, vals)
	}

	if r.intn(3) == 0 {
		g.ri = 1 + r.intn(8)
		if r.intn(4) == 0 {
			g.ri = 1 + r.intn(100)
		}
		g.marker(0xdd, []byte{byte(g.ri >> 8), byte(g.ri)})
	}

	// SOF.
	sof := []byte{8, byte(g.h >> 8), byte(g.h), byte(g.w >> 8), byte(g.w), byte(nComp)}
	for _, c := range g.comps {
		sof = append(sof, c.id, byte(c.h<<4|c.v), byte(c.tq))
	}
	if extended {
		g.marker(0xc1, sof)
	} else {
		g.marker(0xc0, sof)
	}

	// Huffman tables: define every (class, id) the components use.
	for _, c := range g.comps {
		for class, id := range []int{c.td, c.ta} {
			if g.defined[class][id] {
				continue
			}
			s := mkSpecs[2*(id&1)+class]
			if r.intn(5) == 0 {
				s = g.permuteSpec(s)
			}
			g.dht(class, id, s)
		}
	}

	// Scans.
	rstMode := 0
	if r.intn(4) == 0 {
		rstMode = 1
	}
	all := make([]int, nComp)
	for i := range all {
		all[i] = i
	}
	switch k := r.intn(6); {
	case nComp == 1 || k < 3:
		g.scan(all, rstMode)
	case k < 5:
		perm := r.perm(nComp)
		for _, ci := range perm {
			g.scan([]int{ci}, rstMode)
		}
	default:
		g.scan(all[:2], rstMode)
		for _, ci := range all[2:] {
			g.scan([]int{ci}, rstMode)
		}
	}
	if r.intn(3) == 0 {
		// A second pass over the first component overwrites it.
		g.scan([]int{0}, 0)
	}

	switch r.intn(12) {
	case 0: // Missing EOI.
	case 1:
		g.buf = append(g.buf, 0xff, 0xd0, 0xff, 0xd9)
	case 2:
		g.buf = append(g.buf, 0x00, 0x12, 0xff, 0x00, 0xff, 0xd9, 0x55)
	default:
		g.buf = append(g.buf, 0xff, 0xd9)
	}
	return g.buf
}

func (r *rng) perm(n int) []int {
	p := make([]int, n)
	for i := range p {
		p[i] = i
	}
	for i := n - 1; i > 0; i-- {
		j := r.intn(i + 1)
		p[i], p[j] = p[j], p[i]
	}
	return p
}

func mkjpegCmd(seed0, n int, outdir string) {
	if err := os.MkdirAll(outdir, 0o755); err != nil {
		panic(err)
	}
	for s := seed0; s < seed0+n; s++ {
		b := mkjpeg(uint64(s))
		if err := os.WriteFile(filepath.Join(outdir, fmt.Sprintf("mk-%06d.jpg", s)), b, 0o644); err != nil {
			panic(err)
		}
	}
}

// ---------------------------------------------------------------------------
// Structure-aware mutations.

type segment struct {
	marker     byte
	start, end int // payload bytes [start, end)
}

// segments lists the marker segments before the first SOS payload ends
// (and every later SOS header).
func segments(b []byte) []segment {
	var segs []segment
	i := 2
	for i+4 <= len(b) {
		if b[i] != 0xff {
			i++
			continue
		}
		m := b[i+1]
		if m == 0x00 || m == 0xff || (m >= 0xd0 && m <= 0xd9) {
			i += 2
			continue
		}
		n := int(b[i+2])<<8 | int(b[i+3])
		s := segment{m, i + 4, i + 2 + n}
		if s.end > len(b) {
			s.end = len(b)
		}
		segs = append(segs, s)
		i = s.end
	}
	return segs
}

var specialBytes = []byte{0, 1, 2, 3, 4, 8, 12, 16, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x21, 0x22, 0x24,
	0x31, 0x33, 0x41, 0x42, 0x44, 0x3f, 0x40, 0x7f, 0x80, 0xfe, 0xff, 'R', 'G', 'B'}

func mutate1(r *rng, b []byte) []byte {
	if len(b) == 0 {
		return b
	}
	segs := segments(b)
	switch r.intn(12) {
	case 0, 1: // Header byte set to a special value.
		if len(segs) > 0 {
			s := segs[r.intn(len(segs))]
			if s.end > s.start {
				b[s.start+r.intn(s.end-s.start)] = specialBytes[r.intn(len(specialBytes))]
				return b
			}
		}
	case 2: // Header byte flipped.
		if len(segs) > 0 {
			s := segs[r.intn(len(segs))]
			if s.end > s.start {
				b[s.start+r.intn(s.end-s.start)] ^= byte(1 << r.intn(8))
				return b
			}
		}
	case 3: // Marker-ish pair inserted anywhere.
		i := r.intn(len(b))
		pairs := [][]byte{{0xff, 0x00}, {0xff, 0xff}, {0xff, 0xd9}, {0xff, byte(0xd0 + r.intn(8))},
			{0xff, byte(r.intn(256))}}
		ins := pairs[r.intn(len(pairs))]
		return append(b[:i], append(append([]byte(nil), ins...), b[i:]...)...)
	case 4: // Truncation (often near the end).
		if r.intn(2) == 0 && len(b) > 64 {
			return b[:len(b)-1-r.intn(64)]
		}
		return b[:r.intn(len(b))]
	case 5: // Deletion.
		i := r.intn(len(b))
		m := 1 + r.intn(8)
		if i+m > len(b) {
			m = len(b) - i
		}
		return append(b[:i], b[i+m:]...)
	case 6: // Duplicate a segment.
		if len(segs) > 0 {
			s := segs[r.intn(len(segs))]
			if s.start >= 4 {
				seg := append([]byte(nil), b[s.start-4:s.end]...)
				i := s.start - 4
				return append(b[:i], append(seg, b[i:]...)...)
			}
		}
	case 7: // Drop a segment.
		if len(segs) > 0 {
			s := segs[r.intn(len(segs))]
			if s.start >= 4 {
				return append(b[:s.start-4], b[s.end:]...)
			}
		}
	case 8: // Change a marker code.
		if len(segs) > 0 {
			s := segs[r.intn(len(segs))]
			if s.start >= 3 {
				codes := []byte{0xc0, 0xc1, 0xc2, 0xc3, 0xc4, 0xc9, 0xcc, 0xdb, 0xdd, 0xda, 0xe0, 0xee, 0xfe, 0x01, 0xbf}
				b[s.start-3] = codes[r.intn(len(codes))]
				return b
			}
		}
	case 9: // Segment length changed.
		if len(segs) > 0 {
			s := segs[r.intn(len(segs))]
			if s.start >= 2 {
				b[s.start-1] += byte(r.intn(5)) - 2
				return b
			}
		}
	}
	// Random byte change anywhere.
	i := r.intn(len(b))
	b[i] ^= byte(1 + r.intn(255))
	return b
}

func mutateCmd(n, seed0 int, outPath, inPath string) {
	bases := readCorpus(inPath)
	var es []corpusEntry
	for s := seed0; s < seed0+n; s++ {
		r := &rng{uint64(s)*0x9e3779b1 + 77}
		bi := r.intn(len(bases))
		b := append([]byte(nil), bases[bi].data...)
		nm := 1 + r.intn(3)
		for k := 0; k < nm; k++ {
			b = mutate1(r, b)
		}
		es = append(es, corpusEntry{fmt.Sprintf("mut-%d-%s", s, bases[bi].name), b})
	}
	writeCorpus(outPath, es)
}

// ---------------------------------------------------------------------------
// draw2: like draw, but destinations, sources and masks are sometimes
// sub-images (Pix not starting at Rect.Min, Stride larger than the row,
// odd chroma offsets for YCbCr), which exercises the stride and offset
// arithmetic of every fast path.

func maybeSub(r *rng, m image.Image) image.Image {
	if r.intn(3) != 0 {
		return m
	}
	b := m.Bounds()
	x0 := b.Min.X - 1 + r.intn(b.Dx()+2)
	y0 := b.Min.Y - 1 + r.intn(b.Dy()+2)
	x1 := x0 + r.intn(b.Dx()+2)
	y1 := y0 + r.intn(b.Dy()+2)
	return m.(subImager).SubImage(image.Rect(x0, y0, x1, y1))
}

var dst2Kinds = []int{0, 1, 11, 10, 12, 2, 13, 14, 16, 9, 0, 12}

func dstImage2(r *rng) draw.Image {
	rect := randRect(r, 24, 8)
	k := r.intn(len(dst2Kinds))
	g := genImage(r, dst2Kinds[k], rect)
	m := maybeSub(r, g).(draw.Image)
	if k >= 10 {
		return &slowDst{m}
	}
	return m
}

func srcImage2(r *rng) image.Image {
	k := r.intn(numKinds + 3)
	if k == numKinds {
		return image.NewUniform(randColor(r))
	}
	if k == numKinds+1 {
		return randRect(r, 30, 10)
	}
	if k == numKinds+2 {
		k2 := r.intn(numKinds)
		rr := randRect(r, 30, 10)
		g := genImage(r, k2, rr)
		return &slowSrc{maybeSub(r, g)}
	}
	rr := randRect(r, 30, 10)
	g := genImage(r, k, rr)
	return maybeSub(r, g)
}

func maskImage2(r *rng) image.Image {
	sub := func(kind int) image.Image {
		rr := randRect(r, 30, 10)
		g := genImage(r, kind, rr)
		return maybeSub(r, g)
	}
	switch r.intn(8) {
	case 0, 1:
		return nil
	case 2:
		return sub(14)
	case 3:
		return sub(16)
	case 4:
		return image.NewUniform(randColor(r))
	case 5:
		return sub(0)
	case 6:
		return &slowSrc{sub(14)}
	}
	return sub(r.intn(numKinds))
}

func drawOps2(seed0, n int) {
	for seed := seed0; seed < seed0+n; seed++ {
		r := &rng{uint64(seed)*104729 + 11}
		dst := dstImage2(r)
		src := srcImage2(r)
		mask := maskImage2(r)
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
		fmt.Fprintf(out, "%d\t%T\t%T\t%s\t%d\t%s\n", seed, dst, src, rectStr(dst.Bounds()), how, sha16(pixOf(dst)))
	}
}
