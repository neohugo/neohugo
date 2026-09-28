// Command exif is the Go oracle for the EXIF decoder of crates/nh-images (Wave B task T10):
// resources/images/exif.Decoder.Decode over bep/imagemeta v0.12.0 (EXIF source), for the
// decoder configurations Hugo builds from [imaging.exif], on
//
//   - every JPEG, PNG and WebP file of this repository (the camera JPEGs of
//     resources/testdata/issue10738 and exif/orientation6.jpg carry EXIF),
//   - synthetic files with hand-built EXIF blocks (both byte orders, EXIF orientations 1-8,
//     every value type, the converters, GPS, IFD1 thumbnails, the panicking type assertions of
//     GetLatLong/GetDateTime) embedded in JPEG APP1, PNG eXIf, WebP EXIF chunks and bare TIFF,
//   - mutations of those (truncations, byte and word overwrites) for the EOF and error paths.
//
// The synthetic inputs are stored in the fixture. Dates are parsed in time.Local, so the
// oracle runs with TZ=UTC; math.Pow (APEX values) is float code, so it runs as a linux/arm64
// build under qemu (tools/go-oracle/nh-images/regen.sh).
package main

import (
	"bytes"
	"encoding/base64"
	"encoding/binary"
	"flag"
	"fmt"
	"image"
	"image/color"
	"image/jpeg"
	"image/png"
	"log"
	"math"
	"math/rand/v2"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"

	"github.com/bep/imagemeta"
	"github.com/neohugo/neohugo/resources/images/exif"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
)

var root string

type decoderCfg struct {
	name                   string
	include, exclude       string
	disableDate, disableLL bool
}

// The decoder configurations: Hugo's default exclude list (ImagingConfig.init), seeksnack's
// excludeFields '.*', everything, dates and lat/long disabled, an include list.
var cfgs = []decoderCfg{
	{name: "default", exclude: "GPS|Exif|Exposure[M|P|B]|Contrast|Resolution|Sharp|JPEG|Metering|Sensing|Saturation|ColorSpace|Flash|WhiteBalance"},
	{name: "none", exclude: ".*"},
	{name: "all", include: ".*"},
	{name: "nodate", include: ".*", disableDate: true, disableLL: true},
	{name: "incl", include: "Orientation|Date|GPS|Model", exclude: "Offset"},
}

func main() {
	out := flag.String("out", "crates/nh-images/tests/fixtures/exif", "output directory")
	flag.StringVar(&root, "root", ".", "repository root")
	flag.Parse()

	decoders := map[string]*exif.Decoder{}
	for _, c := range cfgs {
		d, err := exif.NewDecoder(
			exif.WithDateDisabled(c.disableDate),
			exif.WithLatLongDisabled(c.disableLL),
			exif.ExcludeFields(c.exclude),
			exif.IncludeFields(c.include),
		)
		if err != nil {
			log.Fatal(err)
		}
		decoders[c.name] = d
	}

	type input struct {
		id     string
		format imagemeta.ImageFormat
		b      []byte
		stored bool
	}
	var inputs []input
	for _, f := range repoFiles() {
		format := imagemeta.JPEG
		switch strings.ToLower(filepath.Ext(f)) {
		case ".png":
			format = imagemeta.PNG
		case ".webp":
			format = imagemeta.WebP
		}
		b, err := os.ReadFile(filepath.Join(root, f))
		if err != nil {
			log.Fatal(err)
		}
		inputs = append(inputs, input{"file:" + f, format, b, false})
	}
	synth := synthInputs()
	for _, s := range synth {
		inputs = append(inputs, input{s.id, s.format, s.b, true})
	}
	r := rand.New(rand.NewPCG(1, 2))
	for i := 0; i < 700; i++ {
		base := synth[r.IntN(len(synth))]
		b := mutate(r, base.b)
		inputs = append(inputs, input{fmt.Sprintf("mut%d:%s", i, base.id), base.format, b, true})
	}

	var cases []map[string]any
	for _, in := range inputs {
		c := map[string]any{"src": in.id, "format": int(in.format)}
		if in.stored {
			c["b"] = base64.StdEncoding.EncodeToString(in.b)
		}
		res := map[string]any{}
		for _, dc := range cfgs {
			if strings.HasPrefix(in.id, "mut") && dc.name != "all" && dc.name != "default" {
				continue
			}
			res[dc.name] = decode(decoders[dc.name], in.format, in.b)
		}
		c["res"] = res
		cases = append(cases, c)
	}
	if err := goval.WriteCasesGz(filepath.Join(*out, "exif.json.gz"), map[string]any{"oracle": "nh-images/exif"}, cases); err != nil {
		log.Fatal(err)
	}
}

func decode(d *exif.Decoder, format imagemeta.ImageFormat, b []byte) map[string]any {
	ex, err := d.Decode("x", format, bytes.NewReader(b))
	if err != nil {
		return map[string]any{"err": err.Error()}
	}
	var keys []string
	for k := range ex.Tags {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	var tags [][]any
	for _, k := range keys {
		v := ex.Tags[k]
		tags = append(tags, []any{k, fmt.Sprintf("%T", v), goval.Str(fmt.Sprintf("%v", v))})
	}
	return map[string]any{
		"lat":  goval.FBits(ex.Lat),
		"long": goval.FBits(ex.Long),
		"date": ex.Date.Format(time.RFC3339Nano),
		"tags": tags,
	}
}

func repoFiles() []string {
	var files []string
	for _, dir := range []string{"resources", "docs", "media", "hugolib", "tpl", "crates/go-image/tests/fixtures", "crates/go-png/tests/fixtures"} {
		err := filepath.WalkDir(filepath.Join(root, dir), func(path string, d os.DirEntry, err error) error {
			if err != nil {
				return err
			}
			if d.IsDir() {
				return nil
			}
			switch strings.ToLower(filepath.Ext(path)) {
			case ".jpg", ".jpeg", ".png", ".webp":
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
	return files
}

// ---------------------------------------------------------------------------
// EXIF (TIFF structure) builder

type entry struct {
	tag   uint16
	typ   uint16
	count uint32
	data  []byte // the value bytes in the block's byte order
	child *ifd   // an IFD pointer (LONG)
}

type ifd struct {
	entries []entry
	next    *ifd // IFD1
}

type enc struct{ bo binary.ByteOrder }

func (e enc) ascii(tag uint16, s string) entry {
	b := append([]byte(s), 0)
	return entry{tag: tag, typ: 2, count: uint32(len(b)), data: b}
}

func (e enc) shorts(tag uint16, vs ...uint16) entry {
	b := make([]byte, 2*len(vs))
	for i, v := range vs {
		e.bo.PutUint16(b[2*i:], v)
	}
	return entry{tag: tag, typ: 3, count: uint32(len(vs)), data: b}
}

func (e enc) longs(tag uint16, typ uint16, vs ...uint32) entry {
	b := make([]byte, 4*len(vs))
	for i, v := range vs {
		e.bo.PutUint32(b[4*i:], v)
	}
	return entry{tag: tag, typ: typ, count: uint32(len(vs)), data: b}
}

func (e enc) rats(tag uint16, typ uint16, vs ...uint32) entry {
	b := make([]byte, 4*len(vs))
	for i, v := range vs {
		e.bo.PutUint32(b[4*i:], v)
	}
	return entry{tag: tag, typ: typ, count: uint32(len(vs) / 2), data: b}
}

func (e enc) raw(tag uint16, typ uint16, b []byte) entry {
	return entry{tag: tag, typ: typ, count: uint32(len(b)), data: b}
}

func (e enc) f32(tag uint16, f float32) entry {
	b := make([]byte, 4)
	e.bo.PutUint32(b, math.Float32bits(f))
	return entry{tag: tag, typ: 11, count: 1, data: b}
}

func (e enc) f64(tag uint16, f float64) entry {
	b := make([]byte, 8)
	e.bo.PutUint64(b, math.Float64bits(f))
	return entry{tag: tag, typ: 12, count: 1, data: b}
}

func ptr(tag uint16, child *ifd) entry {
	return entry{tag: tag, typ: 4, count: 1, child: child}
}

// tiff lays out a TIFF block: the header, then each IFD followed by its out-of-line values.
func tiff(bo binary.ByteOrder, ifd0 *ifd) []byte {
	var order []*ifd
	var visit func(d *ifd)
	visit = func(d *ifd) {
		order = append(order, d)
		for _, en := range d.entries {
			if en.child != nil {
				visit(en.child)
			}
		}
		if d.next != nil {
			visit(d.next)
		}
	}
	visit(ifd0)
	offsets := map[*ifd]uint32{}
	pos := uint32(8)
	for _, d := range order {
		offsets[d] = pos
		pos += 2 + 12*uint32(len(d.entries)) + 4
		for _, en := range d.entries {
			if en.child == nil && len(en.data) > 4 {
				pos += uint32(len(en.data))
			}
		}
	}
	out := make([]byte, 8)
	if bo == binary.LittleEndian {
		copy(out, "II")
	} else {
		copy(out, "MM")
	}
	bo.PutUint16(out[2:], 42)
	bo.PutUint32(out[4:], 8)
	for _, d := range order {
		start := offsets[d]
		dataPos := start + 2 + 12*uint32(len(d.entries)) + 4
		var data []byte
		b := make([]byte, 2)
		bo.PutUint16(b, uint16(len(d.entries)))
		for _, en := range d.entries {
			e := make([]byte, 12)
			bo.PutUint16(e, en.tag)
			bo.PutUint16(e[2:], en.typ)
			bo.PutUint32(e[4:], en.count)
			switch {
			case en.child != nil:
				bo.PutUint32(e[8:], offsets[en.child])
			case len(en.data) > 4:
				bo.PutUint32(e[8:], dataPos+uint32(len(data)))
				data = append(data, en.data...)
			default:
				copy(e[8:], en.data)
			}
			b = append(b, e...)
		}
		nb := make([]byte, 4)
		if d.next != nil {
			bo.PutUint32(nb, offsets[d.next])
		}
		b = append(b, nb...)
		out = append(out, b...)
		out = append(out, data...)
	}
	return out
}

type variant struct {
	name        string
	orientation uint16
	gpsASCII    bool // GPSLatitude as an ASCII string (parseDegrees)
	badRef      bool // GPSLatitudeRef as a SHORT (GetLatLong panics)
	badDate     bool // DateTimeOriginal as a SHORT (GetDateTime panics)
	oddDate     bool // an unparsable date
	comment     string
}

func block(bo binary.ByteOrder, v variant) []byte {
	e := enc{bo}
	gps := &ifd{entries: []entry{
		e.raw(0x0000, 1, []byte{2, 3, 0, 0}),
		e.ascii(0x0001, "S"),
		e.rats(0x0002, 5, 13, 1, 45, 1, 3033, 100),
		e.ascii(0x0003, "W"),
		e.rats(0x0004, 5, 100, 1, 30, 1, 0, 1),
		e.raw(0x0005, 1, []byte{0}),
		e.rats(0x0006, 5, 12345, 10),
		e.rats(0x0007, 5, 13, 1, 3, 1, 4279, 100),
		e.ascii(0x0008, " 7 "),
		e.ascii(0x000a, "3"),
		e.ascii(0x0012, "WGS-84"),
		e.ascii(0x001d, "2021:05:06"),
	}}
	if v.gpsASCII {
		gps.entries[2] = e.ascii(0x0002, "13,45,30.33")
		gps.entries[4] = e.ascii(0x0004, " 100.5,30,0")
	}
	if v.badRef {
		gps.entries[1] = e.shorts(0x0001, 83)
	}
	interop := &ifd{entries: []entry{e.ascii(0x0001, "R98"), e.raw(0x0002, 7, []byte("0100"))}}
	comment := []byte("ASCII\x00\x00\x00Hello, world \x00\x00")
	switch v.comment {
	case "unicode":
		comment = []byte("UNICODE\x00H\x00i\x00\x01 ")
	case "zeros":
		comment = []byte("\x00\x00\x00\x00\x00\x00\x00\x00plain text   ")
	case "short":
		comment = []byte("ASC")
	case "nonascii":
		comment = []byte("ASCII\x00\x00\x00caf\xc3\xa9")
	}
	exifIFD := &ifd{entries: []entry{
		e.rats(0x829a, 5, 1, 200),
		e.rats(0x829d, 5, 28, 10),
		e.shorts(0x8822, 2),
		e.shorts(0x8827, 400),
		e.raw(0x9000, 7, []byte("0230")),
		e.ascii(0x9003, "2021:05:06 07:08:09"),
		e.ascii(0x9004, "2021:05:06 07:08:10"),
		e.raw(0x9101, 7, []byte{1, 2, 3, 0}),
		e.rats(0x9201, 10, 0xfffffff6, 0xfffffffe, 0, 0),
		e.rats(0x9202, 5, 30, 10),
		e.rats(0x9204, 10, 0xffffffff, 3),
		e.rats(0x9205, 5, 0, 0),
		e.rats(0x9206, 5, 7, 0),
		e.shorts(0x9207, 5),
		e.rats(0x920a, 5, 50, 1),
		e.shorts(0x9214, 100, 200, 30, 40),
		e.raw(0x9286, 7, comment),
		e.ascii(0x9290, "042"),
		e.ascii(0x9291, "x9"),
		e.raw(0x927c, 7, []byte("Nikon\x00maker")),
		e.raw(0xa000, 7, []byte("0100")),
		e.shorts(0xa001, 1),
		e.longs(0xa002, 4, 640),
		e.longs(0xa003, 3, 480),
		ptr(0xa005, interop),
		e.raw(0xa302, 7, []byte{0, 2, 0, 2, 0, 1, 1, 2}),
		e.rats(0xa432, 5, 18, 1, 55, 1, 35, 10, 56, 10),
		e.ascii(0xa434, "Lens\x01 18-55mm "),
		e.f32(0xfe01, 1.5),
		e.f64(0xfe02, -2.25),
		e.f32(0xfe03, float32(math.Inf(1))),
		e.longs(0xfe04, 9, 0xffffff85, 7),
		e.raw(0xfe05, 6, []byte{0xff}),
		e.shorts(0xfe06),
		e.raw(0xfe07, 1, []byte{1, 2, 3, 4, 5, 6, 7, 8, 9}),
		e.shorts(0x0102, 8, 8, 8),
	}}
	if v.badDate {
		exifIFD.entries[5] = e.shorts(0x9003, 2021)
	}
	if v.oddDate {
		exifIFD.entries[5] = e.ascii(0x9003, "2021-05-06T07:08:09")
	}
	ifd1 := &ifd{entries: []entry{
		e.shorts(0x0103, 6),
		e.longs(0x0201, 4, 1234),
		e.longs(0x0202, 4, 99),
		e.ascii(0x0132, "2019:01:02 03:04:05"),
		e.ascii(0x010f, "ThumbMake"),
	}}
	ifd0 := &ifd{entries: []entry{
		e.ascii(0x010e, "  A description\x7f "),
		e.ascii(0x010f, "Canon"),
		e.ascii(0x0110, "EOS 5D"),
		e.shorts(0x0112, v.orientation),
		e.rats(0x011a, 5, 72, 1),
		e.rats(0x011b, 5, 144, 2),
		e.shorts(0x0128, 2),
		e.ascii(0x0131, "neohugo oracle"),
		e.ascii(0x0132, "2020:01:02 03:04:05"),
		e.raw(0x02bc, 1, []byte("<x:xmpmeta/>")),
		ptr(0x8769, exifIFD),
		ptr(0x8825, gps),
		e.longs(0x83bb, 4, 1, 2, 3),
		e.shorts(0x0213, 1),
	}, next: ifd1}
	return tiff(bo, ifd0)
}

type synthInput struct {
	id     string
	format imagemeta.ImageFormat
	b      []byte
}

func smallJPEG() []byte {
	m := image.NewNRGBA(image.Rect(0, 0, 8, 8))
	for i := range m.Pix {
		m.Pix[i] = uint8(i * 7)
	}
	var buf bytes.Buffer
	if err := jpeg.Encode(&buf, m, &jpeg.Options{Quality: 50}); err != nil {
		log.Fatal(err)
	}
	return buf.Bytes()
}

func smallPNG() []byte {
	m := image.NewGray(image.Rect(0, 0, 4, 4))
	for i := range m.Pix {
		m.Pix[i] = uint8(i * 16)
	}
	m.Set(0, 0, color.Gray{1})
	var buf bytes.Buffer
	if err := png.Encode(&buf, m); err != nil {
		log.Fatal(err)
	}
	return buf.Bytes()
}

func withJPEGApp1(jpg, payload []byte) []byte {
	seg := []byte{0xff, 0xe1, 0, 0}
	binary.BigEndian.PutUint16(seg[2:], uint16(len(payload)+2))
	out := append([]byte{0xff, 0xd8}, seg...)
	out = append(out, payload...)
	return append(out, jpg[2:]...)
}

func pngChunk(typ string, data []byte) []byte {
	out := make([]byte, 4)
	binary.BigEndian.PutUint32(out, uint32(len(data)))
	out = append(out, typ...)
	out = append(out, data...)
	return append(out, 0, 0, 0, 0) // imagemeta does not check the CRC
}

func withPNGChunks(p []byte, chunks ...[]byte) []byte {
	// Insert after the signature and IHDR (8 + 25 bytes).
	out := append([]byte(nil), p[:33]...)
	for _, c := range chunks {
		out = append(out, c...)
	}
	return append(out, p[33:]...)
}

func webpWithEXIF(payload []byte, flags byte) []byte {
	var body []byte
	body = append(body, "WEBP"...)
	vp8x := make([]byte, 10)
	vp8x[0] = flags
	body = append(body, "VP8X"...)
	body = binary.LittleEndian.AppendUint32(body, 10)
	body = append(body, vp8x...)
	body = append(body, "ICCP"...)
	body = binary.LittleEndian.AppendUint32(body, 4)
	body = append(body, 1, 2, 3, 4)
	body = append(body, "EXIF"...)
	body = binary.LittleEndian.AppendUint32(body, uint32(len(payload)))
	body = append(body, payload...)
	out := append([]byte("RIFF"), 0, 0, 0, 0)
	binary.LittleEndian.PutUint32(out[4:], uint32(len(body)))
	return append(out, body...)
}

func synthInputs() []synthInput {
	var out []synthInput
	jpg := smallJPEG()
	pngb := smallPNG()
	for _, bo := range []binary.ByteOrder{binary.LittleEndian, binary.BigEndian} {
		bon := "II"
		if bo == binary.BigEndian {
			bon = "MM"
		}
		for o := uint16(1); o <= 8; o++ {
			b := block(bo, variant{orientation: o})
			out = append(out, synthInput{fmt.Sprintf("jpeg:%s:o%d", bon, o), imagemeta.JPEG, withJPEGApp1(jpg, append([]byte("Exif\x00\x00"), b...))})
		}
		for _, v := range []variant{
			{name: "gpsascii", orientation: 6, gpsASCII: true},
			{name: "badref", orientation: 1, badRef: true},
			{name: "baddate", orientation: 1, badDate: true},
			{name: "odddate", orientation: 3, oddDate: true},
			{name: "unicode", orientation: 1, comment: "unicode"},
			{name: "zeros", orientation: 1, comment: "zeros"},
			{name: "short", orientation: 1, comment: "short"},
			{name: "nonascii", orientation: 1, comment: "nonascii"},
		} {
			b := block(bo, v)
			out = append(out, synthInput{fmt.Sprintf("jpeg:%s:%s", bon, v.name), imagemeta.JPEG, withJPEGApp1(jpg, append([]byte("Exif\x00\x00"), b...))})
		}
		b := block(bo, variant{orientation: 8})
		out = append(out,
			synthInput{"png:" + bon, imagemeta.PNG, withPNGChunks(pngb, pngChunk("tEXt", []byte("k\x00v")), pngChunk("zTXt", []byte("Raw profile type exif\x00\x00xx")), pngChunk("eXIf", b))},
			synthInput{"webp:" + bon, imagemeta.WebP, webpWithEXIF(b, 1<<3)},
			synthInput{"webp-noflag:" + bon, imagemeta.WebP, webpWithEXIF(b, 0)},
			synthInput{"tiff:" + bon, imagemeta.TIFF, b},
		)
	}
	// Structural odds and ends.
	b := block(binary.LittleEndian, variant{orientation: 2})
	out = append(out,
		synthInput{"jpeg:notexif", imagemeta.JPEG, withJPEGApp1(jpg, append([]byte("Exix\x00\x00"), b...))},
		synthInput{"jpeg:empty-app1", imagemeta.JPEG, withJPEGApp1(jpg, nil)},
		synthInput{"jpeg:plain", imagemeta.JPEG, jpg},
		synthInput{"png:plain", imagemeta.PNG, pngb},
		synthInput{"jpeg:not-jpeg", imagemeta.JPEG, []byte("GIF89a")},
		synthInput{"tiff:bad", imagemeta.TIFF, []byte("XX*\x00\x08\x00\x00\x00")},
		synthInput{"webp:bad", imagemeta.WebP, []byte("RIFX\x00\x00\x00\x00WEBP")},
		synthInput{"jpeg:zero-marker", imagemeta.JPEG, append([]byte{0xff, 0xd8, 0, 0}, jpg[2:]...)},
	)
	return out
}

func mutate(r *rand.Rand, in []byte) []byte {
	b := append([]byte(nil), in...)
	for n := 1 + r.IntN(3); n > 0; n-- {
		if len(b) == 0 {
			break
		}
		switch r.IntN(5) {
		case 0:
			b = b[:r.IntN(len(b))]
		case 1:
			b[r.IntN(len(b))] = byte(r.IntN(256))
		case 2:
			i := r.IntN(len(b))
			specials := []uint32{0, 1, 2, 7, 8, 13, 0xffff, 0x10001, 0xffffffff, 0x7fffffff, 30000}
			v := specials[r.IntN(len(specials))]
			for k := 0; k < 4 && i+k < len(b); k++ {
				b[i+k] = byte(v >> (8 * k))
			}
		case 3:
			// Corrupt a byte inside the EXIF block (the first 600 bytes).
			if len(b) > 20 {
				i := 12 + r.IntN(min(len(b), 600)-12)
				b[i] ^= byte(1 << r.IntN(8))
			}
		case 4:
			i := r.IntN(len(b))
			b = append(b[:i:i], b[min(len(b), i+1+r.IntN(8)):]...)
		}
	}
	return b
}
