// Oracle modes added by the second go-flate red-team pass:
//
//	specs          read "wrapper level data dict ops" lines from stdin and print
//	               fixture lines (regression cases, FMA-sensitive cases)
//	enc-sweep      systematic compressor cases: -part flushpos (Flush at every
//	               position), closepos (Close at every position, then use after
//	               close and Reset), dicts (every dictionary size class, incl.
//	               dictionaries that are a prefix of the data), errsweep (the
//	               underlying writer fails at every Write call / byte count),
//	               sizes (exact multiples of 32 KiB / 64 KiB and byte-at-a-time
//	               writes)
//	inflate-exh    every truncation and every single-bit flip of small base
//	               streams (Go-compressed, random valid, hand-made structural
//	               defects) decoded with random per-call Read sizes; one
//	               aggregated hash per base stream (-detail N prints the cases)
//	inflate-reset  one flate / zlib reader reused through Reset over chains of
//	               streams, dictionaries, partial reads and errors
//	zlib-hdr       every 2-byte zlib header, with and without a dictionary
//	adler          hash/adler32 at the 5552-byte NMAX boundaries, one-shot and
//	               chunked
//
// The Rust side is crates/go-flate/tests/redteam.rs; readSize, decodeSched
// and the case enumeration must stay in sync with it.

package main

import (
	"bufio"
	"bytes"
	"compress/flate"
	"compress/zlib"
	"encoding/binary"
	"flag"
	"fmt"
	"hash/adler32"
	"hash/fnv"
	"io"
	"os"
	"runtime"
	"strconv"
	"strings"
)

func archVersion() string {
	return goVersion() + " " + runtime.GOARCH
}

// ---- specs ----

func cmdSpecs(args []string) {
	fs := flag.NewFlagSet("specs", flag.ExitOnError)
	note := fs.String("note", "", "text for the header line")
	_ = fs.Parse(args)
	sc := bufio.NewScanner(os.Stdin)
	sc.Buffer(make([]byte, 1<<20), 1<<26)
	w := bufio.NewWriter(os.Stdout)
	defer func() { _ = w.Flush() }()
	_, _ = fmt.Fprintf(w, "# go-flate oracle (%s) specs %s\n", archVersion(), *note)
	id := 0
	for sc.Scan() {
		line := strings.TrimSpace(sc.Text())
		if line == "" || line[0] == '#' {
			continue
		}
		f := strings.Fields(line)
		level, err := strconv.Atoi(f[1])
		if err != nil {
			panic(line)
		}
		_, _ = fmt.Fprintln(w, formatCase(id, caseSpec{f[0], level, f[2], f[3], strings.Split(f[4], ",")}, nil))
		id++
	}
	if err := sc.Err(); err != nil {
		panic(err)
	}
}

// ---- enc-sweep ----

func genSpec(kind, size int, seed uint64) string {
	return fmt.Sprintf("gen:%d:%d:%d", kind, size, seed)
}

func wOp(n int) string { return "W" + strconv.Itoa(n) }

// chunkOpsX is chunkOps written with the W<n>x<k> repeat form.
func chunkOpsX(size, c int) []string {
	var ops []string
	if k := size / c; k > 0 {
		ops = append(ops, fmt.Sprintf("W%dx%d", c, k))
	}
	if size%c > 0 {
		ops = append(ops, wOp(size%c))
	}
	return append(ops, "C")
}

// genSpecExt resolves the extended generator spec
// "gen:K:S:SEED[+gen:K:S:SEED...][/POS=VAL...]": the concatenation of the
// generated parts, then byte assignments. Only called for specs that
// contain '+' or '/'.
func genSpecExt(spec string) []byte {
	parts := strings.Split(spec, "/")
	var out []byte
	for _, p := range strings.Split(parts[0], "+") {
		f := strings.Split(p, ":")
		if len(f) != 4 || f[0] != "gen" {
			panic("bad spec " + spec)
		}
		kind, _ := strconv.Atoi(f[1])
		size, _ := strconv.Atoi(f[2])
		seed, _ := strconv.ParseUint(f[3], 10, 64)
		out = append(out, genData(kind, size, seed)...)
	}
	for _, m := range parts[1:] {
		pos, val, ok := strings.Cut(m, "=")
		if !ok {
			panic("bad spec " + spec)
		}
		p, _ := strconv.Atoi(pos)
		v, _ := strconv.Atoi(val)
		out[p] = byte(v)
	}
	return out
}

// countWrites runs a case against a plain recorder and returns the number
// of Write calls and bytes the underlying writer received.
func countWrites(c caseSpec) (calls, nbytes int) {
	out, wlog, _ := run(c, nil)
	return len(wlog), len(out)
}

func sweepCases(part string, r *rng) []caseSpec {
	var cases []caseSpec
	add := func(wrapper string, level int, data, dict string, ops ...string) {
		cases = append(cases, caseSpec{wrapper, level, data, dict, ops})
	}
	switch part {
	case "flushpos":
		// Flush after every prefix of small inputs (the <=32 stored / <128
		// Huffman-only / fast-encoder / lazy sync paths), sometimes twice.
		for _, level := range allLevels {
			for _, kind := range []int{1, 3, 11} {
				const n = 300
				data := genSpec(kind, n, r.next()%1000000)
				for p := 0; p <= n; p++ {
					ops := []string{wOp(p), "F"}
					if p%7 == 3 {
						ops = append(ops, "F")
					}
					add("flate", level, data, "none", append(ops, wOp(n-p), "C")...)
				}
			}
			// Two flushes at random positions in 3000 bytes, zlib too.
			for i := 0; i < 60; i++ {
				const n = 3000
				a := r.intn(n + 1)
				b := a + r.intn(n-a+1)
				wrapper := []string{"flate", "zlib"}[i%2]
				add(wrapper, level, genSpec(r.intn(16), n, r.next()%1000000), "none",
					wOp(a), "F", wOp(b-a), "F", wOp(n-b), "C")
			}
			// Flush near the block / window boundaries of a 140000-byte input.
			for _, base := range []int{32768, 65274, 65535, 65536, 98304, 131070, 131072} {
				for d := -2; d <= 2; d++ {
					const n = 140000
					p := base + d
					add("flate", level, genSpec([]int{3, 6, 9, 14, 15}[r.intn(5)], n, r.next()%1000000), "none",
						wOp(p), "F", wOp(n-p), "C")
				}
			}
		}
	case "closepos":
		// Close after every prefix; then Write, Flush and Close on the
		// closed writer, Reset and compress the rest.
		for _, level := range allLevels {
			for _, kind := range []int{1, 3} {
				const n = 300
				data := genSpec(kind, n, r.next()%1000000)
				for p := 0; p <= n; p++ {
					wrapper := []string{"flate", "zlib"}[p%2]
					rest := n - p
					if rest == 0 {
						add(wrapper, level, data, "none", wOp(p), "C", "W0", "F", "C", "R", "C")
						continue
					}
					add(wrapper, level, data, "none", wOp(p), "C", "W1", "F", "C", "R", wOp(rest-1), "C")
				}
			}
		}
	case "dicts":
		dsizes := []int{0, 1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129,
			255, 256, 257, 258, 259, 1023, 1024, 1025, 4095, 4096, 4097, 32505, 32506, 32767, 32768,
			32769, 32770, 65535, 65536, 65537, 100000}
		for _, level := range allLevels {
			for _, ds := range dsizes {
				for variant := 0; variant < 4; variant++ {
					var dict, data string
					var size int
					switch variant {
					case 0: // unrelated random dictionary, text data
						size = []int{0, 1, 4, 5, 100, 4000}[r.intn(6)]
						dict = genSpec(1, ds, r.next()%1000000)
						data = genSpec(3, size, r.next()%1000000)
					case 1: // text dictionary that shares vocabulary with the data
						size = []int{3, 50, 1000, 70000}[r.intn(4)]
						dict = genSpec(3, ds, r.next()%1000000)
						data = genSpec(3, size, r.next()%1000000)
					case 2: // data starts with the dictionary (kind 1 is a prefix stream)
						size = ds + []int{0, 1, 3, 4, 100, 5000}[r.intn(6)]
						seed := r.next() % 1000000
						dict = genSpec(1, ds, seed)
						data = genSpec(1, size, seed)
					default: // repeated pattern data, same kind of dictionary
						size = []int{10, 300, 40000}[r.intn(3)]
						seed := r.next() % 1000000
						dict = genSpec(11, ds, seed)
						data = genSpec(11, size, seed)
					}
					if ds == 0 {
						dict = "empty"
					}
					wrapper := "flate"
					if r.intn(3) == 0 {
						wrapper = "zlib"
					}
					var ops []string
					switch r.intn(4) {
					case 0:
						ops = []string{wOp(size), "C"}
					case 1:
						a := r.intn(size + 1)
						ops = []string{wOp(a), "F", wOp(size - a), "C"}
					case 2:
						// Reset re-applies the dictionary.
						a := r.intn(size + 1)
						ops = []string{wOp(a), "C", "R", wOp(size - a), "C"}
					default:
						ops = chunkOpsX(size, 1+r.intn(5000))
					}
					add(wrapper, level, data, dict, ops...)
				}
			}
		}
	case "errsweep":
		var progs []caseSpec
		for _, level := range allLevels {
			for i := 0; i < 5; i++ {
				size := []int{0, 20, 500, 5000, 30000}[i]
				data := genSpec([]int{3, 1, 6, 8, 14}[r.intn(5)], size, r.next()%1000000)
				wrapper := []string{"flate", "zlib"}[r.intn(2)]
				dict := "none"
				if r.intn(3) == 0 {
					dict = genSpec(3, 1+r.intn(2000), r.next()%1000000)
				}
				a := r.intn(size + 1)
				b := a + r.intn(size-a+1)
				var ops []string
				switch r.intn(3) {
				case 0:
					ops = []string{wOp(a), "F", wOp(b - a), "F", wOp(size - b), "C", "R", "W0", "C"}
				case 1:
					ops = []string{wOp(a), wOp(b - a), "C", "C", "F", "R", wOp(size - b), "C"}
				default:
					// Write and Flush after a (possibly failed) Close.
					ops = []string{wOp(a), "C", wOp(b - a), "F", wOp(size - b), "C", "R", "W0", "C"}
				}
				progs = append(progs, caseSpec{wrapper, level, data, dict, ops})
			}
		}
		for _, p := range progs {
			calls, nb := countWrites(p)
			for k := 1; k <= calls+1; k++ {
				add(p.wrapper+"!c"+strconv.Itoa(k), p.level, p.data, p.dict, p.ops...)
				add(p.wrapper+"!p"+strconv.Itoa(k), p.level, p.data, p.dict, p.ops...)
			}
			for j := 0; j < 12; j++ {
				lim := r.intn(nb + 2)
				if j < 4 {
					lim = j
				}
				add(p.wrapper+"!b"+strconv.Itoa(lim), p.level, p.data, p.dict, p.ops...)
			}
		}
	case "sizes":
		for _, level := range allLevels {
			for _, base := range []int{32768, 65535, 65536} {
				for _, k := range []int{1, 2, 3, 5, 8} {
					for d := -1; d <= 1; d++ {
						size := base*k + d
						kind := []int{1, 3, 6, 9, 11, 13, 14, 15}[r.intn(8)]
						data := genSpec(kind, size, r.next()%1000000)
						switch r.intn(3) {
						case 0:
							add("flate", level, data, "none", wOp(size), "C")
						case 1:
							add("flate", level, data, "none", chunkOpsX(size, base)...)
						default:
							add("flate", level, data, "none", chunkOpsX(size, 1+r.intn(1000))...)
						}
					}
				}
			}
			// Byte-at-a-time and tiny writes versus one Write.
			for _, c := range []int{1, 2, 3, 4, 5, 7, 31, 127, 257} {
				size := 1 + r.intn(20000)
				data := genSpec(r.intn(16), size, r.next()%1000000)
				ops := []string{fmt.Sprintf("W%dx%d", c, size/c)}
				if size%c > 0 {
					ops = append(ops, wOp(size%c))
				}
				add("flate", level, data, "none", append(ops, "C")...)
				add("flate", level, data, "none", wOp(size), "C")
			}
		}
	default:
		panic("bad part " + part)
	}
	return cases
}

func cmdEncSweep(args []string) {
	fs := flag.NewFlagSet("enc-sweep", flag.ExitOnError)
	part := fs.String("part", "flushpos", "flushpos, closepos, dicts, errsweep or sizes")
	seed := fs.Uint64("seed", 1, "seed")
	_ = fs.Parse(args)
	r := &rng{s: *seed}
	w := bufio.NewWriter(os.Stdout)
	defer func() { _ = w.Flush() }()
	_, _ = fmt.Fprintf(w, "# go-flate oracle (%s) enc-sweep -part %s -seed %d\n", archVersion(), *part, *seed)
	for i, c := range sweepCases(*part, r) {
		_, _ = fmt.Fprintln(w, formatCase(i, c, nil))
	}
}

// ---- decoding with a per-call Read size schedule ----

// readSize returns the size of the next Read call.
func readSize(r *rng, mode int) int {
	switch mode {
	case 0:
		return 1
	case 1:
		return 7
	case 2:
		return 4096
	case 3:
		return 70000
	}
	switch r.intn(8) {
	case 0:
		return 0
	case 1:
		return 1
	case 2:
		return 1 + r.intn(16)
	case 3:
		return 1 + r.intn(300)
	case 4:
		return []int{4095, 4096, 4097, 32767, 32768, 32769}[r.intn(6)]
	case 5:
		return 1 + r.intn(70000)
	default:
		return 1 + r.intn(3000)
	}
}

const maxReadCalls = 1 << 16

var decodeBuf [70000]byte

// decodeSched decodes stream with Read sizes from the schedule and returns
// the output, a hash of the per-call (n, err) log, the input bytes
// consumed and the final Read / Close errors.
func decodeSched(zl bool, stream, dict []byte, mode int, seed uint64) (out []byte, callLog uint64, consumed int, errStr string) {
	br := bytes.NewReader(stream)
	var rd io.ReadCloser
	if !zl {
		rd = flate.NewReaderDict(br, dict)
	} else {
		var err error
		rd, err = zlib.NewReaderDict(br, dict)
		if err != nil {
			return nil, 0, len(stream) - br.Len(), "new: " + errString(err)
		}
	}
	r := &rng{s: seed}
	h := fnv.New64a()
	buf := decodeBuf[:]
	var outBuf bytes.Buffer
	final := "<none>"
	var rec []byte
	for i := 0; i < maxReadCalls; i++ {
		n, err := rd.Read(buf[:readSize(r, mode)])
		outBuf.Write(buf[:n])
		// The log entry is "<n>:<err>;".
		rec = strconv.AppendInt(rec[:0], int64(n), 10)
		rec = append(rec, ':')
		rec = append(rec, errString(err)...)
		rec = append(rec, ';')
		_, _ = h.Write(rec)
		if err != nil {
			final = errString(err)
			break
		}
	}
	closeErr := rd.Close()
	return outBuf.Bytes(), h.Sum64(), len(stream) - br.Len(), final + " | close: " + errString(closeErr)
}

// ---- inflate-exh ----

type exhBase struct {
	zl     bool
	dict   []byte
	stream []byte
	mode   int
	seed   uint64
	origin string
}

// fixedLitCodes are the fixed literal/length codes for all 288 symbols.
var fixedLitCodes, fixedLitLens = func() ([]uint32, []int) {
	l := make([]int, 288)
	for i := range l {
		switch {
		case i < 144:
			l[i] = 8
		case i < 256:
			l[i] = 9
		case i < 280:
			l[i] = 7
		default:
			l[i] = 8
		}
	}
	return canonical(l), l
}()

// randBits writes nb random bits, at most 16 at a time.
func randBits(bw *bitw, r *rng, nb int) {
	for nb > 0 {
		k := min(nb, 16)
		bw.bits(r.next()&(1<<uint(k)-1), uint(k))
		nb -= k
	}
}

func putFixedLit(bw *bitw, sym int) {
	bw.bits(revBits(fixedLitCodes[sym], fixedLitLens[sym]), uint(fixedLitLens[sym]))
}

func putFixedDist(bw *bitw, code int) {
	bw.bits(revBits(uint32(code), 5), 5)
}

// putFixedMatch writes a length/distance pair in a fixed block.
func putFixedMatch(bw *bitw, length, dist int) {
	lc, le := lengthSym(length, false)
	putFixedLit(bw, lc)
	if e := lenExtra[lc-257]; e > 0 {
		bw.bits(uint64(le), uint(e))
	}
	dc, de := distSym(dist)
	putFixedDist(bw, dc)
	if e := distExtra[dc]; e > 0 {
		bw.bits(uint64(de), uint(e))
	}
}

// putDynHeader writes a dynamic block header from explicit literal and
// distance code lengths (which may be invalid), coding the code lengths
// with plain symbols 0-15 and the given code-length code lengths.
func putDynHeader(bw *bitw, final bool, ll, dl []int, cl []int, nclen int) {
	bf := uint64(0)
	if final {
		bf = 1
	}
	bw.bits(bf, 1)
	bw.bits(2, 2)
	bw.bits(uint64(len(ll)-257), 5)
	bw.bits(uint64(len(dl)-1), 5)
	bw.bits(uint64(nclen-4), 4)
	for i := 0; i < nclen; i++ {
		bw.bits(uint64(cl[clOrder[i]]), 3)
	}
	codes := canonical(cl)
	for _, l := range append(append([]int(nil), ll...), dl...) {
		if cl[l] == 0 {
			// Not encodable: write a zero-length code (which huffSym rejects).
			continue
		}
		bw.bits(revBits(codes[l], cl[l]), uint(cl[l]))
	}
}

// craftStream builds a small stream with a specific structural property
// (most are invalid). It returns the DEFLATE body, the dictionary and a
// label.
func craftStream(r *rng) ([]byte, []byte, string) {
	bw := &bitw{}
	var dict []byte
	if r.intn(4) == 0 {
		dict = genData(3, 1+r.intn(200), r.next())
	}
	kind := r.intn(16)
	if kind == 14 && r.intn(4) != 0 {
		kind = 13 // kind 14 has 36 KB of output: keep it rare
	}
	label := "craft" + strconv.Itoa(kind)
	lits := func(n int) {
		for i := 0; i < n; i++ {
			putFixedLit(bw, int('a')+r.intn(26))
		}
	}
	switch kind {
	case 0: // distance around the available history (dict + output)
		bw.bits(1, 1)
		bw.bits(1, 2)
		n := 1 + r.intn(10)
		lits(n)
		d := len(dict) + n + r.intn(4) - 1
		putFixedMatch(bw, 3+r.intn(20), max(1, min(d, 32768)))
		putFixedLit(bw, 256)
	case 1: // fixed distance codes 30 and 31
		bw.bits(1, 1)
		bw.bits(1, 2)
		lits(1 + r.intn(5))
		putFixedLit(bw, 257+r.intn(8))
		putFixedDist(bw, 30+r.intn(2))
		randBits(bw, r, 13)
		putFixedLit(bw, 256)
	case 2: // fixed literal/length codes 286 and 287
		bw.bits(1, 1)
		bw.bits(1, 2)
		lits(r.intn(5))
		putFixedLit(bw, 286+r.intn(2))
		putFixedDist(bw, 0)
		putFixedLit(bw, 256)
	case 3: // HLIT / HDIST beyond the maximum
		bw.bits(1, 1)
		bw.bits(2, 2)
		bw.bits(uint64(28+r.intn(4)), 5)
		bw.bits(uint64(28+r.intn(4)), 5)
		bw.bits(uint64(r.intn(16)), 4)
		randBits(bw, r, 60)
	case 4, 5, 6, 7, 8, 9: // dynamic headers with chosen (often invalid) code lengths
		ll := make([]int, 258+r.intn(29))
		dl := make([]int, 2+r.intn(29))
		// A complete base tree: literals 0-253, EOB and length code 257
		// at 8 bits (256 codes); distance codes 0 and 1 at 1 bit.
		for i := 0; i < 254; i++ {
			ll[i] = 8
		}
		ll[256] = 8
		ll[257] = 8
		dl[0] = 1
		dl[1] = 1
		switch kind {
		case 4: // EOB without a code (tree still complete)
			ll[256] = 0
			ll[255] = 8
		case 5: // incomplete literal tree
			ll[r.intn(254)] = 0
		case 6: // oversubscribed literal tree
			ll[254] = 8
		case 7: // empty distance tree
			dl[0] = 0
			dl[1] = 0
		case 8: // degenerate single-code distance tree
			dl[1] = 0
		default: // skewed frequencies: codes up to 15 bits (link tables)
			freq := make([]int, len(ll))
			a, b := 1, 1
			for i := 0; i < 40; i++ {
				freq[i] = a
				a, b = b, a+b
			}
			freq[256] = 1
			freq[257] = 3
			copy(ll, huffLengths(freq, 15))
		}
		cfreq := make([]int, 19)
		for _, l := range append(append([]int(nil), ll...), dl...) {
			cfreq[l]++
		}
		cl := huffLengths(cfreq, 7)
		nclen := 19
		for nclen > 4 && cl[clOrder[nclen-1]] == 0 {
			nclen--
		}
		switch r.intn(6) {
		case 0:
			cl[r.intn(19)] = r.intn(8) // perturb the code-length code
		case 1:
			nclen = 4 + r.intn(16) // drop or add code-length entries
		}
		putDynHeader(bw, true, ll, dl, cl, nclen)
		// Body: some literals / matches with the (maybe invalid) codes.
		codes := canonical(ll)
		for i := 0; i < r.intn(20); i++ {
			s := r.intn(len(ll))
			if ll[s] > 0 {
				bw.bits(revBits(codes[s], ll[s]), uint(ll[s]))
			}
			if s > 256 {
				randBits(bw, r, 1+r.intn(20))
			}
		}
		if ll[256] > 0 {
			bw.bits(revBits(codes[256], ll[256]), uint(ll[256]))
		}
	case 10: // reserved block type 3
		bw.bits(uint64(r.intn(2)), 1)
		bw.bits(3, 2)
		randBits(bw, r, 20)
	case 11: // stored blocks with LEN / NLEN variations
		for b := 0; b < 1+r.intn(3); b++ {
			final := b == 2 || r.intn(3) == 0
			bf := uint64(0)
			if final {
				bf = 1
			}
			bw.bits(bf, 1)
			bw.bits(0, 2)
			bw.align()
			n := []int{0, 1, 5, 300}[r.intn(4)]
			nn := ^uint16(n)
			if r.intn(3) == 0 {
				nn ^= uint16(1) << uint(r.intn(16))
			}
			bw.out = append(bw.out, byte(n), byte(n>>8), byte(nn), byte(nn>>8))
			for j := 0; j < n; j++ {
				bw.out = append(bw.out, byte(r.next()))
			}
			if final {
				break
			}
		}
	case 12: // no final block
		for b := 0; b < 1+r.intn(3); b++ {
			bw.bits(0, 1)
			bw.bits(1, 2)
			lits(r.intn(4))
			putFixedLit(bw, 256)
		}
	case 13: // sync markers (empty stored blocks) before the final block
		for b := 0; b < r.intn(4); b++ {
			bw.bits(0, 1)
			bw.bits(0, 2)
			bw.align()
			bw.out = append(bw.out, 0, 0, 0xff, 0xff)
		}
		bw.bits(1, 1)
		bw.bits(1, 2)
		lits(r.intn(10))
		putFixedLit(bw, 256)
	case 14: // matches that wrap the 32 KiB history (long output)
		bw.bits(1, 1)
		bw.bits(1, 2)
		lits(3)
		for i := 0; i < 140+r.intn(20); i++ {
			putFixedMatch(bw, 258, 1+r.intn(3))
		}
		putFixedMatch(bw, 3+r.intn(256), min(32768, len(dict)+3+258*140-r.intn(10)))
		putFixedLit(bw, 256)
	default: // empty final fixed block, garbage after
		bw.bits(1, 1)
		bw.bits(1, 2)
		putFixedLit(bw, 256)
		bw.align()
		bw.out = append(bw.out, byte(r.next()), byte(r.next()))
	}
	bw.align()
	return bw.out, dict, label
}

func zlibWrap(body, dict []byte, withDict bool, output []byte) []byte {
	hdr := []byte{0x78, 0x9c}
	if withDict {
		hdr[1] = 0xbb
	}
	s := append([]byte(nil), hdr...)
	if withDict {
		s = binary.BigEndian.AppendUint32(s, adler32.Checksum(dict))
	}
	s = append(s, body...)
	return binary.BigEndian.AppendUint32(s, adler32.Checksum(output))
}

func genExhBase(r *rng, maxLen int) exhBase {
	var b exhBase
	b.mode = r.intn(5)
	b.seed = r.next()
	b.zl = r.intn(3) == 0
	switch r.intn(3) {
	case 0: // Go's compressor
		level := allLevels[r.intn(len(allLevels))]
		data := genData(r.intn(16), r.intn(1+[]int{10, 100, 600, 3000}[r.intn(4)]), r.next())
		var dict []byte
		if r.intn(4) == 0 {
			dict = genData(3, 1+r.intn(300), r.next())
		}
		var buf bytes.Buffer
		var w compressor
		if b.zl {
			zw, err := zlib.NewWriterLevelDict(&buf, level, dict)
			if err != nil {
				panic(err)
			}
			w = zw
		} else {
			fw, err := flate.NewWriterDict(&buf, level, dict)
			if err != nil {
				panic(err)
			}
			w = fw
		}
		if len(data) > 1 && r.intn(3) == 0 {
			a := r.intn(len(data))
			_, _ = w.Write(data[:a])
			_ = w.Flush()
			_, _ = w.Write(data[a:])
		} else {
			_, _ = w.Write(data)
		}
		_ = w.Close()
		b.stream, b.dict = buf.Bytes(), dict
		b.origin = fmt.Sprintf("go%d", level)
	case 1: // random valid stream
		var dict []byte
		if r.intn(4) == 0 {
			dict = genData(r.intn(16), 1+r.intn(300), r.next())
		}
		body, output := genStream(r, dict, 400)
		if b.zl {
			body = zlibWrap(body, dict, dict != nil, output)
		}
		b.stream, b.dict = body, dict
		b.origin = "gen"
	default: // structural defects
		body, dict, label := craftStream(r)
		if b.zl {
			// The trailer is the Adler-32 of nothing in particular; the
			// defects make most of these fail before it is checked.
			body = zlibWrap(body, dict, dict != nil, nil)
		}
		b.stream, b.dict = body, dict
		b.origin = label
	}
	if len(b.stream) > maxLen {
		b.stream = b.stream[:maxLen]
		b.origin += "-cut"
	}
	return b
}

// exhCases returns the number of cases of a base stream of length l: the
// stream itself, every truncation and every single-bit flip.
func exhCases(l int) int { return 1 + l + 8*l }

// exhStream returns case k of the base stream.
func exhStream(s []byte, k int) []byte {
	l := len(s)
	switch {
	case k == 0:
		return s
	case k <= l:
		return s[:k-1]
	default:
		k -= l + 1
		m := append([]byte(nil), s...)
		m[k/8] ^= 1 << uint(k%8)
		return m
	}
}

func exhSeed(seed uint64, k int) uint64 {
	return seed + uint64(k)*0x9e3779b97f4a7c15
}

func exhRecord(b exhBase, k int) string {
	out, callLog, consumed, errStr := decodeSched(b.zl, exhStream(b.stream, k), b.dict, b.mode, exhSeed(b.seed, k))
	return fmt.Sprintf("%d:%d:%016x:%016x:%d:%s\n", k, len(out), fnv64(out), callLog, consumed, errStr)
}

func putBin(bin *bufio.Writer, b []byte) {
	var tmp [4]byte
	binary.LittleEndian.PutUint32(tmp[:], uint32(len(b)))
	_, _ = bin.Write(tmp[:])
	_, _ = bin.Write(b)
}

func cmdInflateExh(args []string) {
	fs := flag.NewFlagSet("inflate-exh", flag.ExitOnError)
	n := fs.Int("n", 200, "number of base streams")
	seed := fs.Uint64("seed", 1, "seed")
	maxLen := fs.Int("maxlen", 400, "maximum base stream length")
	binPath := fs.String("bin", "", "binary corpus output")
	detail := fs.Int("detail", -1, "print every case of this base instead")
	_ = fs.Parse(args)
	r := &rng{s: *seed}
	var bin *bufio.Writer
	if *binPath != "" {
		bf, err := os.Create(*binPath)
		if err != nil {
			panic(err)
		}
		defer func() {
			if err := bin.Flush(); err != nil {
				panic(err)
			}
			if err := bf.Close(); err != nil {
				panic(err)
			}
		}()
		bin = bufio.NewWriter(bf)
	}
	w := bufio.NewWriter(os.Stdout)
	defer func() { _ = w.Flush() }()
	_, _ = fmt.Fprintf(w, "# go-flate oracle (%s) inflate-exh -n %d -seed %d -maxlen %d\n", archVersion(), *n, *seed, *maxLen)
	total := 0
	for i := 0; i < *n; i++ {
		b := genExhBase(r, *maxLen)
		nc := exhCases(len(b.stream))
		if *detail >= 0 {
			if i == *detail {
				for k := 0; k < nc; k++ {
					_, _ = w.WriteString(exhRecord(b, k))
				}
			}
			continue
		}
		h := fnv.New64a()
		for k := 0; k < nc; k++ {
			_, _ = h.Write([]byte(exhRecord(b, k)))
		}
		total += nc
		wr := "f"
		if b.zl {
			wr = "z"
		}
		if bin != nil {
			_, _ = bin.WriteString(wr)
			var tmp [9]byte
			tmp[0] = byte(b.mode)
			binary.LittleEndian.PutUint64(tmp[1:], b.seed)
			_, _ = bin.Write(tmp[:])
			putBin(bin, b.dict)
			putBin(bin, b.stream)
		}
		_, _ = fmt.Fprintf(w, "%d %s %s %d %d %d %d %016x\n", i, wr, b.origin, len(b.dict), len(b.stream), b.mode, nc, h.Sum64())
	}
	_, _ = fmt.Fprintf(w, "# total cases %d\n", total)
}

// ---- inflate-reset ----

type resetSeg struct {
	stream   []byte
	dict     []byte
	maxReads int // Read calls before the next Reset (0: until an error)
	mode     int
	seed     uint64
}

func genResetChain(r *rng, zl bool) []resetSeg {
	var segs []resetSeg
	for i := 0; i < 2+r.intn(6); i++ {
		var s resetSeg
		var b exhBase
		for {
			b = genExhBase(r, 3000)
			if b.zl == zl {
				break
			}
		}
		s.stream, s.dict = b.stream, b.dict
		switch r.intn(4) {
		case 0: // a dictionary the stream was not written with
			s.dict = genData(r.intn(16), r.intn(400), r.next())
		case 1:
			if len(s.stream) > 0 && r.intn(2) == 0 {
				s.stream = exhStream(s.stream, r.intn(exhCases(len(s.stream))))
			}
		}
		if r.intn(3) == 0 {
			s.maxReads = 1 + r.intn(5)
		}
		s.mode = r.intn(5)
		s.seed = r.next()
		segs = append(segs, s)
	}
	return segs
}

// runResetChain decodes the segments with one reader (Reset between
// segments) and returns one record per segment.
func runResetChain(zl bool, segs []resetSeg) string {
	var sb strings.Builder
	var fr io.ReadCloser
	buf := make([]byte, 70000)
	for i, s := range segs {
		br := bytes.NewReader(s.stream)
		resetErr := "-"
		if !zl {
			if fr == nil {
				fr = flate.NewReaderDict(br, s.dict)
			} else {
				resetErr = errString(fr.(flate.Resetter).Reset(br, s.dict))
			}
		} else {
			if fr == nil {
				z, err := zlib.NewReaderDict(br, s.dict)
				if err != nil {
					_, _ = fmt.Fprintf(&sb, "%d new:%s consumed=%d;", i, errString(err), len(s.stream)-br.Len())
					continue
				}
				fr = z
			} else {
				resetErr = errString(fr.(zlib.Resetter).Reset(br, s.dict))
			}
		}
		r := &rng{s: s.seed}
		h := fnv.New64a()
		var out bytes.Buffer
		reads := 0
		for ; reads < maxReadCalls; reads++ {
			if s.maxReads > 0 && reads == s.maxReads {
				break
			}
			n, err := fr.Read(buf[:readSize(r, s.mode)])
			out.Write(buf[:n])
			_, _ = fmt.Fprintf(h, "%d:%s;", n, errString(err))
			if err != nil {
				break
			}
		}
		_, _ = fmt.Fprintf(&sb, "%d reset=%s reads=%d out=%d:%016x log=%016x consumed=%d close=%s;", i, resetErr, reads,
			out.Len(), fnv64(out.Bytes()), h.Sum64(), len(s.stream)-br.Len(), errString(fr.Close()))
	}
	return sb.String()
}

func cmdInflateReset(args []string) {
	fs := flag.NewFlagSet("inflate-reset", flag.ExitOnError)
	n := fs.Int("n", 300, "number of chains")
	seed := fs.Uint64("seed", 1, "seed")
	binPath := fs.String("bin", "", "binary corpus output")
	detail := fs.Bool("detail", false, "print the full record instead of its hash")
	_ = fs.Parse(args)
	r := &rng{s: *seed}
	bf, err := os.Create(*binPath)
	if err != nil {
		panic(err)
	}
	bin := bufio.NewWriter(bf)
	w := bufio.NewWriter(os.Stdout)
	defer func() { _ = w.Flush() }()
	_, _ = fmt.Fprintf(w, "# go-flate oracle (%s) inflate-reset -n %d -seed %d\n", archVersion(), *n, *seed)
	for i := 0; i < *n; i++ {
		zl := r.intn(2) == 0
		segs := genResetChain(r, zl)
		wr := "f"
		if zl {
			wr = "z"
		}
		_, _ = bin.WriteString(wr)
		_, _ = bin.Write([]byte{byte(len(segs))})
		for _, s := range segs {
			var tmp [13]byte
			binary.LittleEndian.PutUint32(tmp[0:], uint32(s.maxReads))
			tmp[4] = byte(s.mode)
			binary.LittleEndian.PutUint64(tmp[5:], s.seed)
			_, _ = bin.Write(tmp[:])
			putBin(bin, s.dict)
			putBin(bin, s.stream)
		}
		rec := runResetChain(zl, segs)
		if *detail {
			_, _ = fmt.Fprintf(w, "%d %s %d %s\n", i, wr, len(segs), rec)
		} else {
			_, _ = fmt.Fprintf(w, "%d %s %d %016x\n", i, wr, len(segs), fnv64([]byte(rec)))
		}
	}
	if err := bin.Flush(); err != nil {
		panic(err)
	}
	if err := bf.Close(); err != nil {
		panic(err)
	}
}

// ---- zlib-hdr ----

// zlibHdrBody is a stored block holding "hello" followed by its Adler-32.
var zlibHdrBody = []byte{0x01, 0x05, 0x00, 0xfa, 0xff, 'h', 'e', 'l', 'l', 'o', 0x06, 0x2c, 0x02, 0x15}

var zlibHdrDict = []byte("dictionary")

// zlibHdrRecord decodes every 2-byte header h with the given dictionary ID
// bytes (used when FDICT is set) and reader dictionary.
func zlibHdrRecord(h int, dictID []byte, dict []byte, mode int) string {
	s := []byte{byte(h >> 8), byte(h)}
	if h&0x20 != 0 {
		s = append(s, dictID...)
	}
	s = append(s, zlibHdrBody...)
	out, callLog, consumed, errStr := decodeSched(true, s, dict, mode, uint64(h))
	return fmt.Sprintf("%d:%d:%016x:%016x:%d:%s\n", h, len(out), fnv64(out), callLog, consumed, errStr)
}

func zlibHdrVariants() (ids [][]byte, dicts [][]byte) {
	var one [4]byte
	binary.BigEndian.PutUint32(one[:], 1)
	var id [4]byte
	binary.BigEndian.PutUint32(id[:], adler32.Checksum(zlibHdrDict))
	return [][]byte{id[:], one[:], {0, 0}}, [][]byte{nil, zlibHdrDict}
}

func cmdZlibHdr(args []string) {
	fs := flag.NewFlagSet("zlib-hdr", flag.ExitOnError)
	detail := fs.Int("detail", -1, "print every record of this first header byte")
	_ = fs.Parse(args)
	ids, dicts := zlibHdrVariants()
	w := bufio.NewWriter(os.Stdout)
	defer func() { _ = w.Flush() }()
	_, _ = fmt.Fprintf(w, "# go-flate oracle (%s) zlib-hdr\n", archVersion())
	for b0 := 0; b0 < 256; b0++ {
		h := fnv.New64a()
		for vi, id := range ids {
			for di, dict := range dicts {
				for b1 := 0; b1 < 256; b1++ {
					rec := fmt.Sprintf("%d.%d.", vi, di) + zlibHdrRecord(b0<<8|b1, id, dict, (b1+vi+di)%5)
					if b0 == *detail {
						_, _ = w.WriteString(rec)
					}
					_, _ = h.Write([]byte(rec))
				}
			}
		}
		if *detail < 0 {
			_, _ = fmt.Fprintf(w, "%d %016x\n", b0, h.Sum64())
		}
	}
}

// ---- adler ----

func adlerLengths() []int {
	var ls []int
	for i := 0; i <= 300; i++ {
		ls = append(ls, i)
	}
	for k := 1; k <= 200; k++ {
		for d := -5; d <= 5; d++ {
			ls = append(ls, 5552*k+d)
		}
	}
	for s := 9; s <= 22; s++ {
		for d := -1; d <= 1; d++ {
			ls = append(ls, 1<<s+d)
		}
	}
	return ls
}

func cmdAdler(args []string) {
	fs := flag.NewFlagSet("adler", flag.ExitOnError)
	_ = fs.Parse(args)
	ff := bytes.Repeat([]byte{0xff}, 1<<22+1)
	rnd := genData(1, 1<<22+1, 5)
	w := bufio.NewWriter(os.Stdout)
	defer func() { _ = w.Flush() }()
	_, _ = fmt.Fprintf(w, "# go-flate oracle (%s) adler\n", archVersion())
	r := &rng{s: 3}
	for _, l := range adlerLengths() {
		// Chunked: random piece sizes, some around NMAX.
		d := adler32.New()
		for pos := 0; pos < l; {
			c := min(l-pos, []int{1, 3, 4, 5551, 5552, 5553, 1 + r.intn(20000)}[r.intn(7)])
			_, _ = d.Write(rnd[pos : pos+c])
			pos += c
		}
		_, _ = fmt.Fprintf(w, "%d %08x %08x %08x\n", l, adler32.Checksum(ff[:l]), adler32.Checksum(rnd[:l]), d.Sum32())
	}
}
