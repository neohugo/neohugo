package main

// Deterministic input generators shared (line by line) with the Rust tests in
// crates/go-strconv/tests/common/mod.rs. Any change here must be mirrored
// there, otherwise the chunk hashes stop matching.

import (
	"math"
	"unicode/utf8"
)

// rng is SplitMix64.
type rng struct{ s uint64 }

func newRng(seed uint64) *rng { return &rng{s: seed} }

func (r *rng) next() uint64 {
	r.s += 0x9E3779B97F4A7C15
	z := r.s
	z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9
	z = (z ^ (z >> 27)) * 0x94D049BB133111EB
	return z ^ (z >> 31)
}

// intn returns a value in [0, n).
func (r *rng) intn(n int) int { return int(r.next() % uint64(n)) }

// chunkRng returns the generator for chunk c of test seed.
func chunkRng(seed uint64, c int) *rng {
	return newRng(seed*1000003 + uint64(c))
}

var pow10u = [20]uint64{
	1, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9,
	1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16, 1e17, 1e18, 1e19,
}

var pow10f = [23]float64{
	1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9,
	1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16, 1e17, 1e18, 1e19,
	1e20, 1e21, 1e22,
}

var pow10f32 = [11]float32{1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10}

// genF64 returns float64 bits from a mix of distributions.
func genF64(r *rng) uint64 {
	switch r.intn(10) {
	case 0, 1, 2:
		return r.next()
	case 3: // moderate magnitude
		sign := r.next() & 1
		exp := uint64(1023 - 70 + r.intn(141))
		mant := r.next() & (1<<52 - 1)
		return sign<<63 | exp<<52 | mant
	case 4: // subnormal or tiny
		sign := r.next() & 1
		sh := uint(r.intn(64))
		mant := (r.next() >> sh) & (1<<52 - 1)
		exp := uint64(r.intn(3))
		return sign<<63 | exp<<52 | mant
	case 5: // short decimals n*10^e, n/10^e
		k := 1 + r.intn(17)
		n := r.next() % pow10u[k]
		e := r.intn(23)
		f := float64(n)
		if r.next()&1 == 0 {
			f = f / pow10f[e]
		} else {
			f = f * pow10f[e]
		}
		if r.next()&1 == 1 {
			f = -f
		}
		return math.Float64bits(f)
	case 6: // few mantissa bits
		sign := r.next() & 1
		exp := uint64(r.intn(2047))
		a := r.next()
		b := r.next()
		c := r.next()
		mant := a & b & c & (1<<52 - 1)
		sh := uint(r.intn(53))
		mant &= ^uint64(0) << sh
		return sign<<63 | exp<<52 | mant
	case 7: // boundary mantissas, all exponents (incl. Inf/NaN)
		sign := r.next() & 1
		exp := uint64(r.intn(2048))
		mants := [8]uint64{0, 1, 2, 1<<52 - 1, 1<<52 - 2, 1 << 51, 1<<51 - 1, 1<<51 + 1}
		return sign<<63 | exp<<52 | mants[r.intn(8)]
	case 8: // integers
		sh := uint(r.intn(64))
		n := r.next() >> sh
		f := float64(n)
		if r.next()&1 == 1 {
			f = -f
		}
		return math.Float64bits(f)
	default: // widened float32
		return math.Float64bits(float64(math.Float32frombits(uint32(r.next()))))
	}
}

// genF32 returns float32 bits from a mix of distributions.
func genF32(r *rng) uint32 {
	switch r.intn(8) {
	case 0, 1, 2:
		return uint32(r.next())
	case 3:
		sign := uint32(r.next() & 1)
		exp := uint32(127 - 30 + r.intn(61))
		mant := uint32(r.next()) & (1<<23 - 1)
		return sign<<31 | exp<<23 | mant
	case 4:
		sign := uint32(r.next() & 1)
		sh := uint(r.intn(64))
		mant := uint32(r.next()>>sh) & (1<<23 - 1)
		exp := uint32(r.intn(3))
		return sign<<31 | exp<<23 | mant
	case 5:
		k := 1 + r.intn(9)
		n := r.next() % pow10u[k]
		e := r.intn(11)
		f := float32(n)
		if r.next()&1 == 0 {
			f = f / pow10f32[e]
		} else {
			f = f * pow10f32[e]
		}
		if r.next()&1 == 1 {
			f = -f
		}
		return math.Float32bits(f)
	case 6:
		sign := uint32(r.next() & 1)
		exp := uint32(r.intn(255))
		a := r.next()
		b := r.next()
		c := r.next()
		mant := uint32(a&b&c) & (1<<23 - 1)
		sh := uint(r.intn(24))
		mant &= ^uint32(0) << sh
		return sign<<31 | exp<<23 | mant
	default:
		sign := uint32(r.next() & 1)
		exp := uint32(r.intn(256))
		mants := [8]uint32{0, 1, 2, 1<<23 - 1, 1<<23 - 2, 1 << 22, 1<<22 - 1, 1<<22 + 1}
		return sign<<31 | exp<<23 | mants[r.intn(8)]
	}
}

const decDigits = "0123456789"
const hexDigits = "0123456789abcdefABCDEF"

// genDigits appends n digits using mode 0..3.
func genDigits(r *rng, b []byte, n int) []byte {
	mode := r.intn(4)
	for i := 0; i < n; i++ {
		var c byte
		switch mode {
		case 0:
			c = decDigits[r.intn(10)]
		case 1: // mostly 9s
			if r.intn(8) == 0 {
				c = decDigits[r.intn(10)]
			} else {
				c = '9'
			}
		case 2: // mostly 0s
			if r.intn(8) == 0 {
				c = decDigits[r.intn(10)]
			} else {
				c = '0'
			}
		default: // leading digits, zeros, random tail
			if i == 0 || i >= n-2 {
				c = decDigits[r.intn(10)]
			} else {
				c = '0'
			}
		}
		b = append(b, c)
	}
	return b
}

func genSign(r *rng, b []byte) []byte {
	switch r.intn(3) {
	case 0:
		b = append(b, '+')
	case 1:
		b = append(b, '-')
	}
	return b
}

func genExp(r *rng, b []byte) []byte {
	b = genSign(r, b)
	var n int
	switch r.intn(4) {
	case 0:
		n = 1
	case 1:
		n = 2
	case 2:
		n = 3
	default:
		n = 1 + r.intn(22)
	}
	for i := 0; i < n; i++ {
		b = append(b, decDigits[r.intn(10)])
	}
	return b
}

var specialWords = [10]string{"inf", "infinity", "nan", "infin", "infinit", "infinityx", "na", "in", "i", "n"}

const junkChars = "x.e+-_ pP0i"

func mutate(r *rng, b []byte) []byte {
	if r.intn(10) == 0 {
		k := 1 + r.intn(3)
		for j := 0; j < k; j++ {
			pos := r.intn(len(b) + 1)
			b = append(b[:pos], append([]byte{'_'}, b[pos:]...)...)
		}
	}
	if r.intn(20) == 0 {
		pos := r.intn(len(b) + 1)
		c := junkChars[r.intn(len(junkChars))]
		b = append(b[:pos], append([]byte{c}, b[pos:]...)...)
	}
	if r.intn(30) == 0 {
		b = b[:r.intn(len(b)+1)]
	}
	return b
}

// genNumString returns a float-literal-like string.
func genNumString(r *rng) []byte {
	var b []byte
	switch r.intn(20) {
	case 0: // specials
		w := specialWords[r.intn(len(specialWords))]
		b = genSign(r, b)
		for i := 0; i < len(w); i++ {
			c := w[i]
			if r.next()&1 == 1 {
				c -= 'a' - 'A'
			}
			b = append(b, c)
		}
		return mutate(r, b)
	case 1, 2: // hex
		b = genSign(r, b)
		b = append(b, '0')
		if r.next()&1 == 0 {
			b = append(b, 'x')
		} else {
			b = append(b, 'X')
		}
		nd := r.intn(20)
		for i := 0; i < nd; i++ {
			b = append(b, hexDigits[r.intn(len(hexDigits))])
		}
		if r.intn(4) != 0 {
			b = append(b, '.')
			nf := r.intn(20)
			for i := 0; i < nf; i++ {
				b = append(b, hexDigits[r.intn(len(hexDigits))])
			}
		}
		if r.intn(8) != 0 {
			if r.next()&1 == 0 {
				b = append(b, 'p')
			} else {
				b = append(b, 'P')
			}
			b = genExp(r, b)
		}
		return mutate(r, b)
	default: // decimal
		b = genSign(r, b)
		var nd int
		switch {
		case r.intn(50) == 0:
			nd = 700 + r.intn(400)
		case r.intn(4) == 0:
			nd = r.intn(40)
		default:
			nd = r.intn(20)
		}
		b = genDigits(r, b, nd)
		if r.intn(3) != 0 {
			b = append(b, '.')
			b = genDigits(r, b, r.intn(25))
		}
		if r.intn(3) != 0 {
			if r.next()&1 == 0 {
				b = append(b, 'e')
			} else {
				b = append(b, 'E')
			}
			b = genExp(r, b)
		}
		return mutate(r, b)
	}
}

const intChars = "0123456789abcdefghijklmnopqrstuvwxyzABCXYZ"

// genIntString returns an integer-literal-like string.
func genIntString(r *rng) []byte {
	var b []byte
	b = genSign(r, b)
	switch r.intn(8) {
	case 0:
		b = append(b, '0', "xX"[r.intn(2)])
	case 1:
		b = append(b, '0', "bB"[r.intn(2)])
	case 2:
		b = append(b, '0', "oO"[r.intn(2)])
	case 3:
		b = append(b, '0')
	}
	var n int
	if r.intn(8) == 0 {
		n = r.intn(70)
	} else {
		n = r.intn(22)
	}
	mode := r.intn(4)
	for i := 0; i < n; i++ {
		var c byte
		switch mode {
		case 0:
			c = decDigits[r.intn(10)]
		case 1:
			c = hexDigits[r.intn(len(hexDigits))]
		case 2:
			c = intChars[r.intn(len(intChars))]
		default:
			c = "01"[r.intn(2)]
		}
		b = append(b, c)
	}
	return mutate(r, b)
}

var escapeSnippets = [...]string{
	`\x41`, `\xff`, `\x4`, `\xg0`, `é`, `\u12`, `\ud800`, `\U0001F600`, `\U00110000`, `\U0010ffff`,
	`\377`, `\400`, `\12`, `\8`, `\n`, `\t`, `\a`, `\b`, `\f`, `\r`, `\v`, `\\`, `\'`, `\"`, `\q`, `\`,
}

var interestingRunes = [...]rune{
	0xa0, 0xad, 0x85, 0x2000, 0x2028, 0x2029, 0x3000, 0xfeff, 0xe000, 0xfffd, 0xffff,
	0x10ffff, 0x1f600, 0x1680, 0x180e, 0x200b, 0x0378, 0x061c, 0xd7ff, 0xe0001, 0x10000,
}

// genBytes returns a random byte string for quoting tests.
func genBytes(r *rng) []byte {
	var b []byte
	var n int
	if r.intn(10) == 0 {
		n = r.intn(200)
	} else {
		n = r.intn(20)
	}
	for i := 0; i < n; i++ {
		switch r.intn(8) {
		case 0, 1:
			b = append(b, byte(32+r.intn(95)))
		case 2:
			if r.intn(10) == 0 {
				b = append(b, 0x7f)
			} else {
				b = append(b, byte(r.intn(32)))
			}
		case 3:
			b = append(b, byte(r.intn(256)))
		case 4:
			b = utf8.AppendRune(b, rune(r.intn(0x110000)))
		case 5:
			b = append(b, "\"'`\\"[r.intn(4)])
		case 6:
			b = append(b, escapeSnippets[r.intn(len(escapeSnippets))]...)
		default:
			b = utf8.AppendRune(b, interestingRunes[r.intn(len(interestingRunes))])
		}
	}
	return b
}
