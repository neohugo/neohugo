package main

// Hash-checked differential tests. Each test is split into chunks; every chunk
// uses its own generator (chunkRng(seed, c)) and produces one FNV-1a hash over
// all results. The Rust side (crates/go-strconv/tests/common/mod.rs) mirrors
// these functions exactly.

import (
	"encoding/binary"
	"fmt"
	"io"
	"math"
	"strconv"
	"unicode/utf8"
)

type sink struct {
	h    uint64
	dump io.Writer
}

func newSink(dump io.Writer) *sink { return &sink{h: 14695981039346656037, dump: dump} }

func (s *sink) byte1(c byte) {
	s.h ^= uint64(c)
	s.h *= 1099511628211
}

// put hashes len(b) (4 bytes LE) followed by b.
func (s *sink) put(b []byte) {
	var l [4]byte
	binary.LittleEndian.PutUint32(l[:], uint32(len(b)))
	for _, c := range l {
		s.byte1(c)
	}
	for _, c := range b {
		s.byte1(c)
	}
	if s.dump != nil {
		fmt.Fprintf(s.dump, "  %x\n", b)
	}
}

func (s *sink) putStr(str string) { s.put([]byte(str)) }

func (s *sink) putU64(v uint64) {
	var b [8]byte
	binary.LittleEndian.PutUint64(b[:], v)
	s.put(b[:])
}

func (s *sink) putBool(v bool) {
	if v {
		s.put([]byte{1})
	} else {
		s.put([]byte{0})
	}
}

func (s *sink) putErr(err error) {
	if err == nil {
		s.putStr("nil")
		return
	}
	s.putStr(err.Error())
}

func (s *sink) ctx(format string, args ...any) {
	if s.dump != nil {
		fmt.Fprintf(s.dump, format+"\n", args...)
	}
}

type testDef struct {
	name   string
	seed   uint64
	chunks int
	run    func(s *sink, r *rng, c int)
	slow   bool // run with internal/strconv.optimize = false
}

const chunkN = 1 << 14 // values per chunk for the float tests

var tests = []testDef{
	{"ftoa64", 1, 64, runFtoa64, false},        // 64 * 16384 = 1,048,576 float64 values
	{"ftoa32", 2, 64, runFtoa32, false},        // 1,048,576 float32 values
	{"ftoa64as32", 3, 4, runFtoa64as32, false}, // float64 values formatted with bitSize 32
	{"ftoagrid64", 4, 4, runGrid64, false},     // dense fmt x prec grid
	{"ftoagrid32", 5, 4, runGrid32, false},
	{"atofgen", 6, 64, runAtofGen, false}, // 1,048,576 generated float strings
	{"atoigen", 7, 32, runAtoiGen, false}, // 524,288 generated integer strings
	{"quoteall", 8, 0x111, runQuoteAll, false},
	{"quotegen", 9, 32, runQuoteGen, false},
	{"nanconv", 10, 1, runNaNConv, false},
	{"ftoaslow64", 11, 16, runFtoaSlow64, true}, // bignum (decimal) formatting path
	{"ftoaslow32", 12, 8, runFtoaSlow32, true},
	{"atofslow", 13, 16, runAtofSlow, true}, // decimal parsing path
}

const fmts = "beEfgGxX"

func ftoaOps(s *sink, r *rng, f float64, bitSize int) {
	for i := 0; i < len(fmts); i++ {
		fm := fmts[i]
		var precs []int
		switch fm {
		case 'b':
			precs = []int{-1}
		case 'e', 'E', 'g', 'G':
			p1 := r.intn(25)
			p2 := r.intn(18)
			precs = []int{-1, p1, p2}
		case 'f':
			p1 := r.intn(30)
			p2 := r.intn(8)
			precs = []int{-1, p1, p2}
		default:
			p1 := r.intn(18) - 1
			p2 := r.intn(16)
			precs = []int{-1, p1, p2}
		}
		for _, prec := range precs {
			s.ctx("fmt %c %d", fm, prec)
			s.putStr(strconv.FormatFloat(f, fm, prec, bitSize))
		}
	}
	// round trips
	p1 := r.intn(40)
	p2 := r.intn(20)
	p3 := r.intn(16) - 1
	strs := [4]string{
		strconv.FormatFloat(f, 'g', -1, bitSize),
		strconv.FormatFloat(f, 'e', p1, bitSize),
		strconv.FormatFloat(f, 'f', p2, bitSize),
		strconv.FormatFloat(f, 'x', p3, bitSize),
	}
	for _, str := range strs {
		for _, bs := range [2]int{64, 32} {
			v, err := strconv.ParseFloat(str, bs)
			s.ctx("parse %s %d", str, bs)
			s.putU64(math.Float64bits(v))
			s.putErr(err)
		}
	}
}

func runFtoa64(s *sink, r *rng, _ int) {
	for i := 0; i < chunkN; i++ {
		b := genF64(r)
		s.ctx("value %016x", b)
		ftoaOps(s, r, math.Float64frombits(b), 64)
	}
}

func runFtoa32(s *sink, r *rng, _ int) {
	for i := 0; i < chunkN; i++ {
		b := genF32(r)
		s.ctx("value %08x", b)
		ftoaOps(s, r, float64(math.Float32frombits(b)), 32)
	}
}

func runFtoa64as32(s *sink, r *rng, _ int) {
	for i := 0; i < chunkN; i++ {
		b := genF64(r)
		s.ctx("value %016x", b)
		ftoaOps(s, r, math.Float64frombits(b), 32)
	}
}

var precGrid = func() []int {
	var p []int
	for i := -1; i <= 40; i++ {
		p = append(p, i)
	}
	return append(p, 50, 60, 70, 80, 100, 150, 200, 300, 400, 500, 767, 800, 1074, 1100)
}()

const gridN = 1 << 10

func gridOps(s *sink, f float64, bitSize int) {
	for i := 0; i < len(fmts); i++ {
		for _, prec := range precGrid {
			s.ctx("fmt %c %d", fmts[i], prec)
			s.putStr(strconv.FormatFloat(f, fmts[i], prec, bitSize))
		}
	}
}

func runGrid64(s *sink, r *rng, _ int) {
	for i := 0; i < gridN; i++ {
		b := genF64(r)
		s.ctx("value %016x", b)
		gridOps(s, math.Float64frombits(b), 64)
	}
}

func runGrid32(s *sink, r *rng, _ int) {
	for i := 0; i < gridN; i++ {
		b := genF32(r)
		s.ctx("value %08x", b)
		gridOps(s, float64(math.Float32frombits(b)), 32)
	}
}

func runAtofGen(s *sink, r *rng, _ int) {
	for i := 0; i < chunkN; i++ {
		str := genNumString(r)
		s.ctx("input %x", str)
		for _, bs := range [2]int{64, 32} {
			v, err := strconv.ParseFloat(string(str), bs)
			s.putU64(math.Float64bits(v))
			s.putErr(err)
		}
		if i%4 == 0 {
			// complex
			str2 := genNumString(r)
			var c []byte
			paren := r.intn(4) == 0
			if paren {
				c = append(c, '(')
			}
			c = append(c, str...)
			switch r.intn(3) {
			case 0:
				c = append(c, '+')
			case 1:
				c = append(c, '-')
			}
			c = append(c, str2...)
			if r.intn(5) != 0 {
				c = append(c, 'i')
			}
			if paren {
				c = append(c, ')')
			}
			s.ctx("complex %x", c)
			for _, bs := range [2]int{128, 64} {
				v, err := strconv.ParseComplex(string(c), bs)
				s.putU64(math.Float64bits(real(v)))
				s.putU64(math.Float64bits(imag(v)))
				s.putErr(err)
			}
		}
	}
}

var intBaseBits = [9][2]int{{0, 0}, {0, 64}, {10, 0}, {10, 32}, {16, 64}, {2, 64}, {8, 16}, {36, 64}, {0, 8}}

func runAtoiGen(s *sink, r *rng, _ int) {
	for i := 0; i < chunkN; i++ {
		str := genIntString(r)
		s.ctx("input %x", str)
		rb := r.intn(40) - 1
		rbits := r.intn(70) - 2
		for j := 0; j <= len(intBaseBits); j++ {
			var base, bits int
			if j < len(intBaseBits) {
				base, bits = intBaseBits[j][0], intBaseBits[j][1]
			} else {
				base, bits = rb, rbits
			}
			v, err := strconv.ParseInt(string(str), base, bits)
			s.putU64(uint64(v))
			s.putErr(err)
			u, err := strconv.ParseUint(string(str), base, bits)
			s.putU64(u)
			s.putErr(err)
		}
		v, err := strconv.Atoi(string(str))
		s.putU64(uint64(v))
		s.putErr(err)
		b, err := strconv.ParseBool(string(str))
		s.putBool(b)
		s.putErr(err)
		// formatting of a random integer in a random base
		n := r.next() >> uint(r.intn(64))
		base := 2 + r.intn(35)
		s.putStr(strconv.FormatInt(int64(n), base))
		s.putStr(strconv.FormatUint(n, base))
		s.putStr(strconv.FormatInt(int64(n), 10))
		s.putStr(strconv.Itoa(-int(n)))
	}
}

func quoteOps(s *sink, str []byte) {
	s.putStr(strconv.Quote(string(str)))
	s.putStr(strconv.QuoteToASCII(string(str)))
	s.putStr(strconv.QuoteToGraphic(string(str)))
	s.putBool(strconv.CanBackquote(string(str)))
	for _, q := range [4]string{"", "\"", "'", "`"} {
		u, err := strconv.Unquote(q + string(str) + q)
		s.putStr(u)
		s.putErr(err)
		p, err := strconv.QuotedPrefix(q + string(str) + q + "tail")
		s.putStr(p)
		s.putErr(err)
	}
	for _, q := range [3]byte{0, '\'', '"'} {
		v, mb, tail, err := strconv.UnquoteChar(string(str), q)
		s.putU64(uint64(int64(v)))
		s.putBool(mb)
		s.putU64(uint64(len(tail)))
		s.putErr(err)
	}
}

func runQuoteAll(s *sink, _ *rng, c int) {
	var runes []rune
	if c < 0x110 {
		for x := c << 12; x < (c+1)<<12; x++ {
			runes = append(runes, rune(x))
		}
	} else {
		runes = []rune{-1, -2, math.MinInt32, 0x110000, 0x110001, math.MaxInt32, 0x7fffffff - 1, -0x10000}
	}
	for _, x := range runes {
		s.ctx("rune %d", x)
		s.putStr(strconv.QuoteRune(x))
		s.putStr(strconv.QuoteRuneToASCII(x))
		s.putStr(strconv.QuoteRuneToGraphic(x))
		s.putBool(strconv.IsPrint(x))
		s.putBool(strconv.IsGraphic(x))
		str := utf8.AppendRune(nil, x)
		quoteOps(s, str)
		u, err := strconv.Unquote(strconv.QuoteRune(x))
		s.putStr(u)
		s.putErr(err)
		u, err = strconv.Unquote(strconv.QuoteToASCII(string(str)))
		s.putStr(u)
		s.putErr(err)
	}
}

func runQuoteGen(s *sink, r *rng, _ int) {
	for i := 0; i < chunkN; i++ {
		str := genBytes(r)
		s.ctx("input %x", str)
		quoteOps(s, str)
	}
}

func runNaNConv(s *sink, r *rng, _ int) {
	for i := 0; i < chunkN; i++ {
		sign := r.next() & 1
		mant := r.next() & (1<<52 - 1)
		if mant == 0 {
			mant = 1
		}
		f := math.Float64frombits(sign<<63 | 0x7ff<<52 | mant)
		f32 := float32(f)
		s.putU64(uint64(math.Float32bits(f32)))
		s.putU64(math.Float64bits(float64(f32)))
		b32 := uint32(r.next())
		b32 |= 0x7f800000
		if b32&(1<<23-1) == 0 {
			b32 |= 1
		}
		s.putU64(math.Float64bits(float64(math.Float32frombits(b32))))
		// ordinary values f64 -> f32 rounding
		g := math.Float64frombits(genF64(r))
		s.putU64(uint64(math.Float32bits(float32(g))))
	}
}

const slowN = 1 << 12

func slowFtoaOps(s *sink, r *rng, f float64, bitSize int) {
	for i := 0; i < len(fmts); i++ {
		fm := fmts[i]
		if fm == 'b' || fm == 'x' || fm == 'X' {
			continue
		}
		p1 := r.intn(25)
		p2 := r.intn(18)
		p3 := precGrid[r.intn(len(precGrid))]
		for _, prec := range [3]int{p1, p2, p3} {
			s.ctx("fmt %c %d", fm, prec)
			s.putStr(strconv.FormatFloat(f, fm, prec, bitSize))
		}
	}
}

func runFtoaSlow64(s *sink, r *rng, _ int) {
	for i := 0; i < slowN; i++ {
		b := genF64(r)
		s.ctx("value %016x", b)
		slowFtoaOps(s, r, math.Float64frombits(b), 64)
	}
}

func runFtoaSlow32(s *sink, r *rng, _ int) {
	for i := 0; i < slowN; i++ {
		b := genF32(r)
		s.ctx("value %08x", b)
		slowFtoaOps(s, r, float64(math.Float32frombits(b)), 32)
	}
}

func runAtofSlow(s *sink, r *rng, _ int) {
	for i := 0; i < slowN; i++ {
		str := genNumString(r)
		s.ctx("input %x", str)
		for _, bs := range [2]int{64, 32} {
			v, err := strconv.ParseFloat(string(str), bs)
			s.putU64(math.Float64bits(v))
			s.putErr(err)
		}
		f := math.Float64frombits(genF64(r))
		p := r.intn(30)
		str2 := strconv.FormatFloat(f, 'e', p, 64)
		s.ctx("input2 %s", str2)
		for _, bs := range [2]int{64, 32} {
			v, err := strconv.ParseFloat(str2, bs)
			s.putU64(math.Float64bits(v))
			s.putErr(err)
		}
	}
}
