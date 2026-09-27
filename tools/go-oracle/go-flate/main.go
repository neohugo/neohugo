// Command go-flate is the Go oracle for the Rust crate crates/go-flate.
//
// It runs Go's compress/flate and compress/zlib (from the Go toolchain that
// builds it; the golden build uses go1.27.1) over deterministic inputs and
// write/flush/reset/close programs, and prints one fixture line per case with
// the expected output hash, length, and the sequence of Write calls the
// compressor issued to its underlying writer.
//
// Usage:
//
//	go run ./tools/go-oracle/go-flate cases -profile small > crates/go-flate/tests/fixtures/cases_small.txt
//	go run ./tools/go-oracle/go-flate cases -profile big > $SCRATCH/cases_big.txt
//	go run ./tools/go-oracle/go-flate files -levels all FILE... > files.txt
//	go run ./tools/go-oracle/go-flate png -out DIR IMAGE...
//	go run ./tools/go-oracle/go-flate inflate-cases -n 3000 > crates/go-flate/tests/fixtures/inflate_cases.txt
//
// Fixture line format (whitespace separated):
//
//	id wrapper level data dict ops outlen outfnv wlogfnv results
//
// wrapper is "flate" or "zlib". data is "gen:<kind>:<size>:<seed>" or
// "file:<name>". dict is "none" (flate.NewWriter / zlib nil dict), "empty",
// or "gen:<kind>:<size>:<seed>". ops is a comma separated program over the
// data: W<n> writes the next n bytes, F flushes, R resets to the same sink,
// C closes. outfnv is FNV-1a 64 of all bytes written to the sink, wlogfnv is
// FNV-1a 64 over the little-endian uint64 lengths of each Write call, and
// results has one character per op ('o' ok, 'e' error).
package main

import (
	"bufio"
	"bytes"
	"compress/flate"
	"compress/zlib"
	"encoding/binary"
	"flag"
	"fmt"
	"hash/fnv"
	"image"
	"image/draw"
	_ "image/gif"
	_ "image/jpeg"
	"image/png"
	"io"
	"os"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"
)

// rng is splitmix64; the Rust test uses the identical generator.
type rng struct{ s uint64 }

func (r *rng) next() uint64 {
	r.s += 0x9e3779b97f4a7c15
	z := r.s
	z = (z ^ (z >> 30)) * 0xbf58476d1ce4e5b9
	z = (z ^ (z >> 27)) * 0x94d049bb133111eb
	return z ^ (z >> 31)
}

func (r *rng) intn(n int) int { return int(r.next() % uint64(n)) }

var words = []string{
	"the", "of", "and", "to", "in", "is", "you", "that", "it", "he",
	"was", "for", "on", "are", "as", "with", "his", "they", "at", "be",
	"this", "have", "from", "or", "one", "had", "by", "word", "but", "not",
	"what", "all", "were", "we", "when", "your", "can", "said", "there", "use",
	"an", "each", "which", "she", "do", "how", "their", "if", "will", "up",
	"compression", "snack", "company", "Thailand", "<div class=\"card\">", "</div>", "{{ .Title }}", "https://example.com/",
	"\t", "0123456789", "ประเทศไทย", "日本語", "naïve", "café",
}

// genData must stay in sync with gen_data in crates/go-flate/tests/common/mod.rs.
func genData(kind, size int, seed uint64) []byte {
	r := &rng{s: seed ^ uint64(kind)<<56}
	out := make([]byte, 0, size+64)
	switch kind {
	case 0: // zeros
		out = out[:size]
	case 1: // uniform random
		for len(out) < size {
			out = append(out, byte(r.next()))
		}
	case 2: // small alphabet
		k := 2 + r.intn(14)
		base := byte(r.next())
		for len(out) < size {
			out = append(out, base+byte(r.intn(k)))
		}
	case 3: // text
		for len(out) < size {
			out = append(out, words[r.intn(len(words))]...)
			switch r.intn(12) {
			case 0:
				out = append(out, ".\n"...)
			case 1:
				out = append(out, ", "...)
			default:
				out = append(out, ' ')
			}
		}
	case 4: // repeated pattern with mutations
		l := 1 + r.intn(64)
		pat := make([]byte, l)
		for i := range pat {
			pat[i] = byte(r.next())
		}
		for i := 0; len(out) < size; i++ {
			b := pat[i%l]
			if r.intn(64) == 0 {
				b = byte(r.next())
			}
			out = append(out, b)
		}
	case 5: // runs
		for len(out) < size {
			b := byte(r.next())
			n := 1 + r.intn(300)
			for j := 0; j < n; j++ {
				out = append(out, b)
			}
		}
	case 6: // mixed segments of kinds 0-5, 7-9
		for len(out) < size {
			k := r.intn(9)
			if k == 6 {
				k = 9
			}
			n := 1 + r.intn(20000)
			out = append(out, genData(k, n, r.next())...)
		}
	case 7: // skewed (geometric) byte distribution
		for len(out) < size {
			v := 0
			for r.intn(3) != 0 && v < 255 {
				v++
			}
			out = append(out, byte(v))
		}
	case 8: // PNG-filtered-row-like data
		rowLen := 1 + 3*(1+r.intn(400))
		for len(out) < size {
			out = append(out, byte(r.intn(5)))
			for j := 1; j < rowLen && len(out) < size; j++ {
				if r.intn(16) == 0 {
					out = append(out, byte(r.next()))
				} else {
					out = append(out, byte(r.intn(7)-3))
				}
			}
		}
	case 9: // long-distance repeats around the 32 KiB window
		chunk := make([]byte, 1+r.intn(40000))
		for i := range chunk {
			chunk[i] = byte(r.next())
		}
		for len(out) < size {
			switch r.intn(3) {
			case 0:
				out = append(out, chunk...)
			case 1:
				n := 1 + r.intn(40000)
				for j := 0; j < n; j++ {
					out = append(out, byte(r.next()))
				}
			default:
				start := r.intn(len(chunk))
				out = append(out, chunk[start:]...)
			}
		}
	default:
		out = genDataExt(r, kind, size, out)
	}
	return out[:size]
}

// recorder is the sink: it keeps all bytes and the length of every Write.
// With mode 'b' it accepts at most limit bytes in total (a partial Write
// returns errFail); with mode 'c' the limit-th Write call fails with
// (0, errFail) and later calls succeed again; with mode 'p' every call from
// the limit-th on fails.
type recorder struct {
	buf   bytes.Buffer
	wlog  []int
	mode  byte
	limit int
	calls int
}

func (r *recorder) Write(p []byte) (int, error) {
	r.wlog = append(r.wlog, len(p))
	r.calls++
	switch r.mode {
	case 'b':
		if room := r.limit - r.buf.Len(); len(p) > room {
			r.buf.Write(p[:room])
			return room, errFail
		}
	case 'c':
		if r.calls == r.limit {
			return 0, errFail
		}
	case 'p':
		if r.calls >= r.limit {
			return 0, errFail
		}
	}
	return r.buf.Write(p)
}

func fnv64(b []byte) uint64 {
	h := fnv.New64a()
	h.Write(b)
	return h.Sum64()
}

func wlogHash(wlog []int) uint64 {
	h := fnv.New64a()
	var tmp [8]byte
	for _, n := range wlog {
		binary.LittleEndian.PutUint64(tmp[:], uint64(n))
		h.Write(tmp[:])
	}
	return h.Sum64()
}

type caseSpec struct {
	wrapper string
	level   int
	data    string
	dict    string
	ops     []string
}

func resolveData(spec string, files map[string][]byte) []byte {
	switch {
	case spec == "none":
		return nil
	case spec == "empty":
		return []byte{}
	case strings.HasPrefix(spec, "gen:"):
		p := strings.Split(spec, ":")
		kind, _ := strconv.Atoi(p[1])
		size, _ := strconv.Atoi(p[2])
		seed, _ := strconv.ParseUint(p[3], 10, 64)
		return genData(kind, size, seed)
	case strings.HasPrefix(spec, "go:"):
		return goTestData(spec)
	case strings.HasPrefix(spec, "file:"):
		b, ok := files[spec[5:]]
		if !ok {
			panic("unknown file " + spec)
		}
		return b
	}
	panic("bad data spec " + spec)
}

type compressor interface {
	Write([]byte) (int, error)
	Flush() error
	Close() error
	Reset(io.Writer)
}

func run(c caseSpec, files map[string][]byte) (out []byte, wlog []int, results string) {
	data := resolveData(c.data, files)
	dict := resolveData(c.dict, files)
	rec := &recorder{}
	wrapper := c.wrapper
	if i := strings.IndexByte(wrapper, '!'); i >= 0 {
		rec.mode = wrapper[i+1]
		rec.limit, _ = strconv.Atoi(wrapper[i+2:])
		wrapper = wrapper[:i]
	}
	var w compressor
	switch wrapper {
	case "flate":
		var fw *flate.Writer
		var err error
		if c.dict == "none" {
			fw, err = flate.NewWriter(rec, c.level)
		} else {
			fw, err = flate.NewWriterDict(rec, c.level, dict)
		}
		if err != nil {
			panic(err)
		}
		w = fw
	case "zlib":
		zw, err := zlib.NewWriterLevelDict(rec, c.level, dict)
		if err != nil {
			panic(err)
		}
		w = zw
	default:
		panic("bad wrapper")
	}
	pos := 0
	var res strings.Builder
	for _, op := range c.ops {
		var err error
		switch op[0] {
		case 'W':
			// W<n> writes the next n bytes; W<n>x<k> does that k times,
			// stopping at the first error (like io.Copy).
			spec, reps, _ := strings.Cut(op[1:], "x")
			n, _ := strconv.Atoi(spec)
			k := 1
			if reps != "" {
				k, _ = strconv.Atoi(reps)
			}
			for j := 0; j < k && err == nil; j++ {
				_, err = w.Write(data[pos : pos+n])
				pos += n
			}
		case 'F':
			err = w.Flush()
		case 'C':
			err = w.Close()
		case 'R':
			w.Reset(rec)
		default:
			panic("bad op " + op)
		}
		if err != nil {
			res.WriteByte('e')
		} else {
			res.WriteByte('o')
		}
	}
	return rec.buf.Bytes(), rec.wlog, res.String()
}

func formatCase(id int, c caseSpec, files map[string][]byte) string {
	out, wlog, results := run(c, files)
	return fmt.Sprintf("%d %s %d %s %s %s %d %016x %016x %s",
		id, c.wrapper, c.level, c.data, c.dict, strings.Join(c.ops, ","),
		len(out), fnv64(out), wlogHash(wlog), results)
}

var allLevels = []int{-2, -1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9}

// chunkOps writes size bytes in chunks of c, then closes.
func chunkOps(size, c int) []string {
	var ops []string
	for pos := 0; pos < size; pos += c {
		ops = append(ops, "W"+strconv.Itoa(min(c, size-pos)))
	}
	return append(ops, "C")
}

// randomOps builds a random program over size bytes.
func randomOps(r *rng, size int, flushProb, resetProb int, maxChunk int) []string {
	var ops []string
	pos := 0
	for pos < size {
		var n int
		switch r.intn(4) {
		case 0:
			n = r.intn(16)
		case 1:
			n = r.intn(1000)
		case 2:
			n = r.intn(maxChunk)
		default:
			n = []int{65535, 65536, 32768, 128, 129, 32, 33, 4096}[r.intn(8)]
		}
		n = min(n, size-pos)
		ops = append(ops, "W"+strconv.Itoa(n))
		pos += n
		if flushProb > 0 && r.intn(flushProb) == 0 {
			ops = append(ops, "F")
			if r.intn(4) == 0 {
				ops = append(ops, "F")
			}
		}
		if resetProb > 0 && r.intn(resetProb) == 0 {
			if r.intn(2) == 0 {
				ops = append(ops, "C")
			}
			ops = append(ops, "R")
		}
	}
	if r.intn(3) == 0 {
		ops = append(ops, "F")
	}
	ops = append(ops, "C")
	if r.intn(8) == 0 {
		// Use after close: errors, then reset and reuse.
		ops = append(ops, "W0", "F", "C", "R", "C")
	}
	return ops
}

func genCases(profile string) []caseSpec {
	var cases []caseSpec
	r := &rng{s: 12345}
	add := func(c caseSpec) { cases = append(cases, c) }
	gen := func(kind, size int, seed uint64) string {
		return fmt.Sprintf("gen:%d:%d:%d", kind, size, seed)
	}

	var sizes []int
	var bigSizes []int
	nRandom := 0
	switch profile {
	case "small":
		sizes = []int{0, 1, 2, 3, 4, 5, 8, 11, 12, 13, 16, 31, 32, 33, 100, 127, 128, 129, 255, 256, 300,
			1000, 1023, 1024, 1025, 2000, 4096, 5000, 8191, 8192, 8193, 16384, 32767, 32768, 32769,
			40000, 65534, 65535, 65536, 65537, 70000, 100000, 131071}
		bigSizes = []int{200000, 300000}
		nRandom = 700
	case "big":
		sizes = []int{0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 31, 32, 33, 63, 64, 65,
			100, 127, 128, 129, 200, 255, 256, 257, 300, 500, 1000, 1023, 1024, 1025, 2000, 3000, 4095, 4096,
			4097, 5000, 8191, 8192, 8193, 10000, 16383, 16384, 16385, 20000, 32767, 32768, 32769, 40000,
			50000, 65533, 65534, 65535, 65536, 65537, 65538, 70000, 98303, 98304, 100000, 131070, 131071,
			131072, 131073, 196605, 196606, 200000, 262144}
		bigSizes = []int{300000, 500000, 1 << 20, 2 << 20, 4 << 20}
		nRandom = 6000
	default:
		panic("bad profile")
	}

	// 1. Every level x kind x size, single write.
	for _, level := range allLevels {
		for kind := 0; kind < 10; kind++ {
			for _, size := range sizes {
				add(caseSpec{"flate", level, gen(kind, size, r.next()%1000000), "none", []string{"W" + strconv.Itoa(size), "C"}})
			}
		}
	}
	// 2. Large inputs, fewer kinds.
	for _, level := range allLevels {
		for _, kind := range []int{1, 3, 6, 8, 9} {
			for _, size := range bigSizes {
				add(caseSpec{"flate", level, gen(kind, size, r.next()%1000000), "none", chunkOps(size, 1<<20)})
			}
		}
	}
	// 3. Fixed chunkings.
	for _, level := range allLevels {
		for _, c := range []int{1, 7, 100, 1000, 4096, 32768, 65535, 65536, 100000} {
			kind := r.intn(10)
			size := 1 + r.intn(300000)
			if c == 1 {
				size = r.intn(5000)
			}
			add(caseSpec{"flate", level, gen(kind, size, r.next()%1000000), "none", chunkOps(size, c)})
		}
	}
	// 4. Random programs (flush / reset / close), flate and zlib, with dicts.
	for i := 0; i < nRandom; i++ {
		level := allLevels[r.intn(len(allLevels))]
		kind := r.intn(10)
		var size int
		switch r.intn(4) {
		case 0:
			size = r.intn(300)
		case 1:
			size = r.intn(70000)
		case 2:
			size = r.intn(300000)
		default:
			size = r.intn(700000)
		}
		wrapper := "flate"
		if r.intn(3) == 0 {
			wrapper = "zlib"
		}
		dict := "none"
		switch r.intn(4) {
		case 0:
			dict = "empty"
		case 1:
			dsize := []int{1, 3, 4, 5, 100, 1000, 32767, 32768, 32769, 40000, 65536, 70000}[r.intn(12)]
			dict = gen(r.intn(10), dsize, r.next()%1000000)
		}
		maxChunk := []int{100, 5000, 70000, 200000}[r.intn(4)]
		ops := randomOps(r, size, []int{0, 2, 5, 20}[r.intn(4)], []int{0, 0, 3, 10}[r.intn(4)], maxChunk)
		add(caseSpec{wrapper, level, gen(kind, size, r.next()%1000000), dict, ops})
	}
	// 5. zlib header for every level, with nil / empty / real dict.
	for _, level := range allLevels {
		for _, dict := range []string{"none", "empty", gen(3, 1000, 7)} {
			size := 1 + r.intn(100000)
			add(caseSpec{"zlib", level, gen(r.intn(10), size, r.next()%1000000), dict, chunkOps(size, 1<<20)})
			add(caseSpec{"zlib", level, gen(0, 0, 0), dict, []string{"C"}})
			add(caseSpec{"zlib", level, gen(0, 0, 0), dict, []string{"F", "C"}})
		}
	}
	return cases
}

func cmdCases(args []string) {
	fs := flag.NewFlagSet("cases", flag.ExitOnError)
	profile := fs.String("profile", "small", "small or big")
	fs.Parse(args)
	w := bufio.NewWriter(os.Stdout)
	defer w.Flush()
	fmt.Fprintf(w, "# go-flate oracle (%s) profile=%s\n", goVersion(), *profile)
	for i, c := range genCases(*profile) {
		fmt.Fprintln(w, formatCase(i, c, nil))
	}
}

// cmdFiles emits cases over real files; file names are recorded by base name.
func cmdFiles(args []string) {
	fs := flag.NewFlagSet("files", flag.ExitOnError)
	programs := fs.Int("programs", 2, "random programs per file and level")
	fs.Parse(args)
	files := map[string][]byte{}
	var names []string
	for _, p := range fs.Args() {
		b, err := os.ReadFile(p)
		if err != nil {
			panic(err)
		}
		name := filepath.Base(p)
		if _, dup := files[name]; dup {
			name = strings.ReplaceAll(strings.TrimPrefix(p, "/"), "/", "__")
		}
		files[name] = b
		names = append(names, name)
	}
	w := bufio.NewWriter(os.Stdout)
	defer w.Flush()
	fmt.Fprintf(w, "# go-flate oracle (%s) files\n", goVersion())
	r := &rng{s: 777}
	id := 0
	for _, name := range names {
		size := len(files[name])
		for _, level := range allLevels {
			for _, wrapper := range []string{"flate", "zlib"} {
				fmt.Fprintln(w, formatCase(id, caseSpec{wrapper, level, "file:" + name, "none", chunkOps(size, 1<<30)}, files))
				id++
			}
			for i := 0; i < *programs; i++ {
				ops := randomOps(r, size, []int{0, 3, 20}[r.intn(3)], []int{0, 10}[r.intn(2)], 70000)
				fmt.Fprintln(w, formatCase(id, caseSpec{"flate", level, "file:" + name, "none", ops}, files))
				id++
			}
		}
	}
}

// cmdPNG encodes each image with image/png at DefaultCompression (the path
// neohugo uses) and writes the PNG to -out. The Rust test re-derives the
// filtered rows from the IDAT stream and must reproduce the zlib stream and
// the IDAT chunk boundaries.
func cmdPNG(args []string) {
	fs := flag.NewFlagSet("png", flag.ExitOnError)
	outDir := fs.String("out", "", "output directory")
	fs.Parse(args)
	if err := os.MkdirAll(*outDir, 0o755); err != nil {
		panic(err)
	}
	n := 0
	for _, p := range fs.Args() {
		f, err := os.Open(p)
		if err != nil {
			panic(err)
		}
		img, _, err := image.Decode(f)
		f.Close()
		if err != nil {
			fmt.Fprintf(os.Stderr, "skip %s: %v\n", p, err)
			continue
		}
		base := strings.TrimSuffix(filepath.Base(p), filepath.Ext(p))
		variants := map[string]image.Image{"orig": img}
		// NRGBA and a paletted-free RGBA copy exercise other color types.
		b := img.Bounds()
		nrgba := image.NewNRGBA(b)
		draw.Draw(nrgba, b, img, b.Min, draw.Src)
		variants["nrgba"] = nrgba
		gray := image.NewGray(b)
		draw.Draw(gray, b, img, b.Min, draw.Src)
		variants["gray"] = gray
		for vname, vimg := range variants {
			for _, lvl := range []png.CompressionLevel{png.DefaultCompression, png.BestSpeed, png.BestCompression, png.NoCompression} {
				var buf bytes.Buffer
				enc := png.Encoder{CompressionLevel: lvl}
				if err := enc.Encode(&buf, vimg); err != nil {
					panic(err)
				}
				name := fmt.Sprintf("%s.%s.l%d.png", base, vname, -int(lvl))
				if err := os.WriteFile(filepath.Join(*outDir, name), buf.Bytes(), 0o644); err != nil {
					panic(err)
				}
				n++
			}
		}
	}
	fmt.Fprintf(os.Stderr, "wrote %d PNGs\n", n)
}

func goVersion() string {
	return runtime.Version()
}

// ---- inflate oracle ----

// inflateCase describes a compressed stream (built with Go's writer from
// generated data), mutations applied to it, and how it is read back.
type inflateCase struct {
	wrapper   string // flate or zlib
	level     int
	data      string // gen:kind:size:seed
	flushAt   int    // -1: no Flush; otherwise Flush after this many bytes
	dictW     string // none or gen:... (dictionary used by the writer)
	dictR     string // none, empty or gen:... (dictionary given to the reader)
	mutations []string
	readSize  int
}

// buildStream compresses the case's data exactly as the Rust test does.
func buildStream(c inflateCase) []byte {
	data := resolveData(c.data, nil)
	dictW := resolveData(c.dictW, nil)
	var buf bytes.Buffer
	var w compressor
	if c.wrapper == "flate" {
		var fw *flate.Writer
		var err error
		if c.dictW == "none" {
			fw, err = flate.NewWriter(&buf, c.level)
		} else {
			fw, err = flate.NewWriterDict(&buf, c.level, dictW)
		}
		if err != nil {
			panic(err)
		}
		w = fw
	} else {
		zw, err := zlib.NewWriterLevelDict(&buf, c.level, dictW)
		if err != nil {
			panic(err)
		}
		w = zw
	}
	if c.flushAt >= 0 {
		w.Write(data[:c.flushAt])
		w.Flush()
		w.Write(data[c.flushAt:])
	} else {
		w.Write(data)
	}
	w.Close()
	stream := buf.Bytes()
	for _, m := range c.mutations {
		switch m[0] {
		case 't':
			n, _ := strconv.Atoi(m[1:])
			stream = stream[:n]
		case 'x':
			p := strings.Split(m[1:], ":")
			pos, _ := strconv.Atoi(p[0])
			v, _ := strconv.Atoi(p[1])
			stream[pos] ^= byte(v)
		case 'a':
			n, _ := strconv.Atoi(m[1:])
			stream = append(stream, genData(1, n, uint64(n))...)
		default:
			panic("bad mutation")
		}
	}
	return stream
}

func errString(err error) string {
	if err == nil {
		return "<nil>"
	}
	return err.Error()
}

// runInflate reads the stream back and returns: output, per-call log hash,
// bytes consumed from the input, and the final error string.
func runInflate(c inflateCase) (out []byte, callLog uint64, consumed int, errStr string) {
	stream := buildStream(c)
	br := bytes.NewReader(stream)
	var r io.ReadCloser
	if c.wrapper == "flate" {
		if c.dictR == "none" {
			r = flate.NewReader(br)
		} else {
			r = flate.NewReaderDict(br, resolveData(c.dictR, nil))
		}
	} else {
		var err error
		r, err = zlib.NewReaderDict(br, resolveData(c.dictR, nil))
		if err != nil {
			return nil, 0, len(stream) - br.Len(), "new: " + errString(err)
		}
	}
	h := fnv.New64a()
	buf := make([]byte, c.readSize)
	var outBuf bytes.Buffer
	var final error
	for i := 0; i < 1<<20; i++ {
		n, err := r.Read(buf)
		outBuf.Write(buf[:n])
		fmt.Fprintf(h, "%d:%s;", n, errString(err))
		if err != nil {
			final = err
			break
		}
	}
	closeErr := r.Close()
	return outBuf.Bytes(), h.Sum64(), len(stream) - br.Len(), errString(final) + " | close: " + errString(closeErr)
}

func cmdInflateCases(args []string) {
	fs := flag.NewFlagSet("inflate-cases", flag.ExitOnError)
	n := fs.Int("n", 3000, "number of cases")
	seed := fs.Uint64("seed", 99, "seed")
	fs.Parse(args)
	r := &rng{s: *seed}
	w := bufio.NewWriter(os.Stdout)
	defer w.Flush()
	fmt.Fprintf(w, "# go-flate oracle (%s) inflate-cases\n", goVersion())
	for i := 0; i < *n; i++ {
		c := inflateCase{flushAt: -1}
		c.wrapper = "flate"
		if r.intn(3) == 0 {
			c.wrapper = "zlib"
		}
		c.level = allLevels[r.intn(len(allLevels))]
		size := []int{0, 1, 10, 100, 1000, 5000, 20000, 70000, 150000}[r.intn(9)]
		size = r.intn(size + 1)
		c.data = fmt.Sprintf("gen:%d:%d:%d", r.intn(10), size, r.next()%1000000)
		if size > 0 && r.intn(4) == 0 {
			c.flushAt = r.intn(size)
		}
		c.dictW, c.dictR = "none", "none"
		if r.intn(5) == 0 {
			c.dictW = fmt.Sprintf("gen:3:%d:%d", 1+r.intn(40000), r.next()%1000)
			c.dictR = c.dictW
			switch r.intn(4) {
			case 0:
				c.dictR = "none"
			case 1:
				c.dictR = fmt.Sprintf("gen:3:%d:%d", 1+r.intn(40000), r.next()%1000)
			}
		} else if c.wrapper == "zlib" && r.intn(6) == 0 {
			c.dictR = "empty"
		}
		streamLen := len(buildStream(c))
		nm := r.intn(4)
		if r.intn(3) == 0 {
			nm = 0
		}
		cur := streamLen
		for j := 0; j < nm && cur > 0; j++ {
			switch r.intn(4) {
			case 0:
				cur = r.intn(cur)
				c.mutations = append(c.mutations, "t"+strconv.Itoa(cur))
			case 1, 2:
				pos := r.intn(cur)
				if r.intn(2) == 0 {
					pos = r.intn(min(cur, 40))
				}
				c.mutations = append(c.mutations, fmt.Sprintf("x%d:%d", pos, 1+r.intn(255)))
			default:
				k := 1 + r.intn(20)
				c.mutations = append(c.mutations, "a"+strconv.Itoa(k))
				cur += k
			}
		}
		muts := "-"
		if len(c.mutations) > 0 {
			muts = strings.Join(c.mutations, ",")
		}
		c.readSize = []int{1, 7, 100, 4096, 32768, 70000}[r.intn(6)]
		out, callLog, consumed, errStr := runInflate(c)
		fmt.Fprintf(w, "%d %s %d %s %d %s %s %s %d %d %016x %016x %d %s\n",
			i, c.wrapper, c.level, c.data, c.flushAt, c.dictW, c.dictR, muts, c.readSize,
			len(out), fnv64(out), callLog, consumed, errStr)
	}
}

// ---- long streams (fastGen.cur wraparound, >2 GiB through one writer) ----

// hashWriter hashes everything written to it (FNV-1a 64) and the Write sizes.
type hashWriter struct {
	n    int64
	h    uint64
	wlog uint64
	nw   int64
}

func newHashWriter() *hashWriter {
	return &hashWriter{h: 0xcbf29ce484222325, wlog: 0xcbf29ce484222325}
}

func (w *hashWriter) Write(p []byte) (int, error) {
	for _, c := range p {
		w.h ^= uint64(c)
		w.h *= 0x100000001b3
	}
	var tmp [8]byte
	binary.LittleEndian.PutUint64(tmp[:], uint64(len(p)))
	for _, c := range tmp {
		w.wlog ^= uint64(c)
		w.wlog *= 0x100000001b3
	}
	w.n += int64(len(p))
	w.nw++
	return len(p), nil
}

// longStream must stay in sync with long_stream in crates/go-flate/tests/long_streams.rs.
func longStream(level int, variant string, total int64) string {
	hw := newHashWriter()
	w, err := flate.NewWriter(hw, level)
	if err != nil {
		panic(err)
	}
	// A pool of 64 generated 1 MiB-ish blocks, written in varying chunk sizes.
	var pool [][]byte
	for i := 0; i < 16; i++ {
		pool = append(pool, genData([]int{3, 6, 8, 9}[i%4], 1<<20+i*977, uint64(i)))
	}
	r := &rng{s: uint64(level)*1000 + 17}
	var written int64
	switch variant {
	case "stream":
		for written < total {
			b := pool[r.intn(len(pool))]
			n := 1 + r.intn(len(b))
			w.Write(b[:n])
			written += int64(n)
			if r.intn(500) == 0 {
				w.Flush()
			}
		}
	case "random":
		// Fresh incompressible bytes: long literal runs at levels 7-9 wrap
		// advancedState.literalCounter (uint16) and exercise skipLiterals.
		src := &rng{s: 99}
		buf := make([]byte, 1<<20)
		for written < total {
			n := 1 + r.intn(len(buf))
			for i := 0; i < n; i += 8 {
				v := src.next()
				for j := 0; j < 8 && i+j < n; j++ {
					buf[i+j] = byte(v >> (8 * j))
				}
			}
			w.Write(buf[:n])
			written += int64(n)
			if r.intn(500) == 0 {
				w.Flush()
			}
		}
	case "resets":
		for i := int64(0); i < total; i++ {
			b := pool[r.intn(len(pool))]
			n := r.intn(200)
			if r.intn(50) == 0 {
				n = r.intn(70000)
			}
			w.Write(b[:n])
			if r.intn(2) == 0 {
				w.Close()
			}
			w.Reset(hw)
		}
	default:
		panic("bad variant")
	}
	w.Close()
	return fmt.Sprintf("%d %s %d %d %016x %016x %d", level, variant, total, hw.n, hw.h, hw.wlog, hw.nw)
}

func cmdLongStreams(args []string) {
	fs := flag.NewFlagSet("longstreams", flag.ExitOnError)
	total := fs.Int64("total", 2400<<20, "bytes per stream")
	resets := fs.Int64("resets", 70000, "reset iterations")
	levels := fs.String("levels", "1,2,3,4,5,6", "comma separated levels")
	noResets := fs.Bool("noresets", false, "skip the resets variant")
	variant := fs.String("variant", "stream", "first variant: stream or random")
	fs.Parse(args)
	fmt.Printf("# go-flate oracle (%s) longstreams\n", goVersion())
	for _, ls := range strings.Split(*levels, ",") {
		level, err := strconv.Atoi(ls)
		if err != nil {
			panic(err)
		}
		fmt.Println(longStream(level, *variant, *total))
		if !*noResets {
			fmt.Println(longStream(level, "resets", *resets))
		}
	}
}

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "usage: go-flate cases|files|png|inflate-cases|longstreams|gendata|run|fuzz|inflate-gen ...")
		os.Exit(2)
	}
	switch os.Args[1] {
	case "cases":
		cmdCases(os.Args[2:])
	case "files":
		cmdFiles(os.Args[2:])
	case "png":
		cmdPNG(os.Args[2:])
	case "longstreams":
		cmdLongStreams(os.Args[2:])
	case "inflate-cases":
		cmdInflateCases(os.Args[2:])
	case "fuzz":
		cmdFuzz(os.Args[2:])
	case "gotests":
		cmdGoTests(os.Args[2:])
	case "inflate-gen":
		cmdInflateGen(os.Args[2:])
	case "genhash":
		// genhash: FNV-1a 64 of genData for every kind over a few sizes/seeds.
		for kind := 0; kind < 16; kind++ {
			for _, size := range []int{0, 1, 100, 5000, 70000, 300000} {
				for seed := uint64(0); seed < 3; seed++ {
					fmt.Printf("%d %d %d %016x\n", kind, size, seed, fnv64(genData(kind, size, seed)))
				}
			}
		}
	case "gendata":
		// gendata kind size seed: raw generator output (for debugging the Rust port).
		kind, _ := strconv.Atoi(os.Args[2])
		size, _ := strconv.Atoi(os.Args[3])
		seed, _ := strconv.ParseUint(os.Args[4], 10, 64)
		os.Stdout.Write(genData(kind, size, seed))
	case "run":
		// run "<fixture line fields 1..5>": raw output of one case (for debugging).
		f := strings.Fields(strings.Join(os.Args[2:], " "))
		level, _ := strconv.Atoi(f[1])
		out, _, _ := run(caseSpec{f[0], level, f[2], f[3], strings.Split(f[4], ",")}, nil)
		os.Stdout.Write(out)
	default:
		fmt.Fprintln(os.Stderr, "unknown command")
		os.Exit(2)
	}
}
