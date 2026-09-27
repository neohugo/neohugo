package main

import (
	"bufio"
	"bytes"
	"compress/zlib"
	"fmt"
	"image"
	"os"
	"path/filepath"
	"strings"
)

// pack writes a fixture pack read by crates/libwebp-sys/tests:
//
//	manifest.tsv  one case per line (see header line)
//	inputs.bin    per-input zlib blocks (inputs are deduplicated)
//	outputs.bin   concatenated expected WebP bytes
type pack struct {
	dir      string
	manifest *bufio.Writer
	mf       *os.File
	inputs   *os.File
	outputs  *os.File
	inOff    int64
	outOff   int64
	seen     map[string][3]int64 // raw input -> (offset, zlen, rawlen)
	n        int
}

func newPack(dir string) *pack {
	if err := os.MkdirAll(dir, 0o755); err != nil {
		panic(err)
	}
	mf, err := os.Create(filepath.Join(dir, "manifest.tsv"))
	if err != nil {
		panic(err)
	}
	in, err := os.Create(filepath.Join(dir, "inputs.bin"))
	if err != nil {
		panic(err)
	}
	out, err := os.Create(filepath.Join(dir, "outputs.bin"))
	if err != nil {
		panic(err)
	}
	p := &pack{dir: dir, mf: mf, manifest: bufio.NewWriter(mf), inputs: in, outputs: out, seen: map[string][3]int64{}}
	fmt.Fprintln(p.manifest, "#name\tkind\tstride\tminx\tminy\tmaxx\tmaxy\tquality\tpreset\tsharp\tin_off\tin_zlen\tin_len\tout_off\tout_len\texpect")
	return p
}

// imageParts returns the gowebp-relevant parts of img. For image kinds that
// gowebp converts itself (ConvertToNRGBA), the converted NRGBA is recorded:
// the Rust API expects callers to do that conversion.
func imageParts(img image.Image) (kind string, pix []byte, stride int, r image.Rectangle) {
	switch v := img.(type) {
	case *image.RGBA:
		return "rgba", v.Pix, v.Stride, v.Rect
	case *image.NRGBA:
		return "nrgba", v.Pix, v.Stride, v.Rect
	case *image.Gray:
		return "gray", v.Pix, v.Stride, v.Rect
	default:
		n := convertToNRGBA(img)
		return "nrgba", n.Pix, n.Stride, n.Rect
	}
}

func (p *pack) add(name string, img image.Image, o encOpts, out []byte, expect string) {
	if strings.ContainsAny(name, "\t\n") {
		panic(name)
	}
	kind, pix, stride, r := imageParts(img)
	key := string(pix)
	loc, ok := p.seen[key]
	if !ok {
		var zb bytes.Buffer
		zw, _ := zlib.NewWriterLevel(&zb, zlib.BestCompression)
		zw.Write(pix)
		zw.Close()
		if _, err := p.inputs.Write(zb.Bytes()); err != nil {
			panic(err)
		}
		loc = [3]int64{p.inOff, int64(zb.Len()), int64(len(pix))}
		p.inOff += int64(zb.Len())
		p.seen[key] = loc
	}
	outOff := p.outOff
	if _, err := p.outputs.Write(out); err != nil {
		panic(err)
	}
	p.outOff += int64(len(out))
	sharp := 0
	if o.UseSharpYuv {
		sharp = 1
	}
	fmt.Fprintf(p.manifest, "%s\t%s\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%s\n",
		name, kind, stride, r.Min.X, r.Min.Y, r.Max.X, r.Max.Y, o.Quality, int(o.EncodingPreset), sharp,
		loc[0], loc[1], loc[2], outOff, len(out), expect)
	p.n++
}

func (p *pack) close() {
	p.manifest.Flush()
	p.mf.Close()
	p.inputs.Close()
	p.outputs.Close()
}
