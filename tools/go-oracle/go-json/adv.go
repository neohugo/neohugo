package main

// Adversarial generators (-mode advtext / advencode), written by the
// independent verifier of crates/go-json. They reuse the record formats of
// -mode text (textCase) and -mode encode, so the Rust replay code is shared,
// but draw inputs from distributions aimed at the risky corners:
//
//   - number literals: long mantissas, exact halfway points between adjacent
//     float64 values (and one digit either side), subnormals, overflow and
//     underflow boundaries, huge exponents, many leading/trailing zeros;
//   - float formatting: Go's TestMarshalFloat inputs (every prefix of
//     1.2345678901234567890123 times 10^-30..10^30, ±, 32/64 bit, and the
//     neighbours), values next to the 1e-6/1e21 cut-offs, random bits;
//   - strings: \u escapes of every interesting code unit (upper and lower
//     case hex, lone/reversed/valid surrogate pairs), raw invalid UTF-8,
//     encoded surrogates, overlong forms, control bytes, U+2028/U+2029 and
//     HTML-sensitive characters, both as values and as object names;
//   - every prefix (truncation) of a set of complex documents;
//   - non-JSON whitespace, BOMs, literal typos, deep nesting near the limit,
//     long strings and numbers across the decoder's buffer boundaries;
//   - random Indent/MarshalIndent/SetIndent prefix and indent strings
//     (spaces, tabs, newlines, CR, letters, HTML characters, invalid UTF-8,
//     U+2028, NUL).

import (
	"bytes"
	"encoding/json"
	"fmt"
	"html/template"
	"io"
	"log"
	"math"
	"math/big"
	"math/rand/v2"
	"strconv"
	"strings"
)

// advIndentPieces are glued together into random prefix/indent strings.
var advIndentPieces = []string{" ", " ", "\t", "  ", "\n", "\r", "a", "<", ">", "&", "\xff", "é", "\u2028", "--", "\t "}

func genIndentString(r *rand.Rand, allowNUL bool) string {
	var b strings.Builder
	if r.IntN(3) == 0 {
		// Only spaces and tabs (the non-placeholder path).
		for k := r.IntN(6); k > 0; k-- {
			b.WriteByte(" \t"[r.IntN(2)])
		}
		return b.String()
	}
	for k := r.IntN(4); k > 0; k-- {
		b.WriteString(advIndentPieces[r.IntN(len(advIndentPieces))])
	}
	if allowNUL && r.IntN(10) == 0 {
		b.WriteByte(0)
	}
	return b.String()
}

// indentHangs reports whether Go's appendIndent (go1.27.1
// src/encoding/json/v2_indent.go) loops forever on src with these arguments.
//
// When prefix or indent holds a byte other than space or tab, appendIndent
// formats with placeholder spaces, and a deferred function then walks the
// bytes it appended: after each '\n' it overwrites the run of spaces with the
// prefix and then with copies of the indent. With an empty indent,
// "for len(spaces) > 0 { spaces = spaces[copy(spaces, invalidIndent):] }"
// never ends once a run is longer than the prefix. The bytes it walks are:
//
//   - on success, the formatted value plus the trailing whitespace of src
//     (only that whitespace can hold such a run);
//   - on a syntax error, src itself, because jsontext.AppendFormat returns
//     append(dst, src...) together with the error. The deferred function
//     still runs although appendIndent then returns dst[:dstLen]. So any
//     invalid src with a '\n' followed by more than len(prefix) spaces
//     hangs, for example Indent(dst, []byte("[\n  1,]"), "a", "").
//
// The earlier version of this check looked only at the trailing whitespace
// and missed the error case, so -mode advtext could hang.
func indentHangs(src []byte, prefix, indent string) bool {
	if len(strings.Trim(prefix, " \t"))+len(strings.Trim(indent, " \t")) == 0 || indent != "" {
		return false
	}
	// The same call with the placeholder spaces does not take the
	// placeholder path, and returns the bytes the deferred function walks
	// (or the error, in which case it walks src).
	walked, err := indentBytes(src, strings.Repeat(" ", len(prefix)), "")
	if err != nil {
		walked = src
	}
	for b := walked; ; {
		i := bytes.IndexByte(b, '\n')
		if i < 0 {
			return false
		}
		b = b[i+1:]
		n := len(b) - len(bytes.TrimLeft(b, " "))
		if n > len(prefix) {
			return true
		}
		b = b[n:]
	}
}

// indentHangsAvoided counts the prefix/indent pairs pickAdvIndent rejected.
var indentHangsAvoided int

func pickAdvIndent(r *rand.Rand, src []byte) (string, string) {
	for {
		p, in := genIndentString(r, false), genIndentString(r, true)
		if src == nil || !indentHangs(src, p, in) {
			return p, in
		}
		indentHangsAvoided++
	}
}

// ---------------------------------------------------------------------------
// Numbers

func digitsN(r *rand.Rand, n int, leadNonZero bool) string {
	b := make([]byte, n)
	for i := range b {
		b[i] = byte('0' + r.IntN(10))
	}
	if leadNonZero && n > 0 {
		b[0] = byte('1' + r.IntN(9))
	}
	return string(b)
}

// exactDecimal returns the exact decimal expansion of the rational x
// (whose denominator is a power of two) in plain notation.
func exactDecimal(x *big.Rat) string {
	// A power-of-two denominator 2^k needs k fraction digits.
	k := x.Denom().BitLen() - 1
	if k < 0 {
		k = 0
	}
	s := x.FloatString(k)
	if strings.Contains(s, ".") {
		s = strings.TrimRight(s, "0")
		s = strings.TrimSuffix(s, ".")
	}
	return s
}

// toSci rewrites a plain decimal "123.456" / "0.000123" as d.ddde±x with
// all significant digits kept.
func toSci(s string, upper bool) string {
	neg := strings.HasPrefix(s, "-")
	s = strings.TrimPrefix(s, "-")
	intp, frac, _ := strings.Cut(s, ".")
	digits := intp + frac
	exp := len(intp) - 1
	i := 0
	for i < len(digits)-1 && digits[i] == '0' {
		i++
		exp--
	}
	digits = digits[i:]
	digits = strings.TrimRight(digits, "0")
	if digits == "" {
		digits = "0"
	}
	var b strings.Builder
	if neg {
		b.WriteByte('-')
	}
	b.WriteByte(digits[0])
	if len(digits) > 1 {
		b.WriteByte('.')
		b.WriteString(digits[1:])
	}
	if upper {
		b.WriteByte('E')
	} else {
		b.WriteByte('e')
	}
	b.WriteString(strconv.Itoa(exp))
	return b.String()
}

// halfway returns the exact midpoint between x and its successor, possibly
// nudged by one unit in a far digit, in plain or scientific notation.
func halfway(r *rand.Rand, x float64) string {
	y := math.Nextafter(x, math.Inf(1))
	if math.IsInf(y, 0) {
		y = x
	}
	a := new(big.Rat).SetFloat64(x)
	b := new(big.Rat).SetFloat64(y)
	m := new(big.Rat).Add(a, b)
	m.Quo(m, big.NewRat(2, 1))
	s := exactDecimal(m)
	switch r.IntN(4) {
	case 0:
		// Just above the midpoint.
		if strings.Contains(s, ".") {
			s += "000000000000000000001"
		} else {
			s += ".000000000000000000001"
		}
	case 1:
		// Just below: truncate the last significant digit.
		if strings.Contains(s, ".") && len(s) > 3 {
			s = s[:len(s)-1]
			s = strings.TrimSuffix(s, ".")
		}
	}
	if r.IntN(2) == 0 {
		s = toSci(s, r.IntN(2) == 0)
	}
	return s
}

func randFiniteFloat(r *rand.Rand) float64 {
	switch r.IntN(6) {
	case 0:
		return math.Float64frombits(uint64(r.Int64N(1 << 52))) // subnormal
	case 1:
		return math.MaxFloat64 / float64(1+r.IntN(4))
	case 2:
		return math.Ldexp(1, r.IntN(2098)-1074)
	default:
		for {
			f := math.Float64frombits(r.Uint64() &^ (1 << 63))
			if !math.IsNaN(f) && !math.IsInf(f, 0) {
				return f
			}
		}
	}
}

// genNumberLit returns a syntactically valid JSON number.
func genNumberLit(r *rand.Rand) string {
	var s string
	switch r.IntN(12) {
	case 0, 1:
		return halfway(r, randFiniteFloat(r))
	case 2:
		// Near the overflow boundary.
		lits := []string{
			"1.7976931348623157e308", "1.7976931348623158e308", "1.7976931348623159e308",
			"1.797693134862315708145274237317043567981e+308", "1.797693134862315808e308",
			"179769313486231580793728971405303415079934132710037826936173778980444968292764750946649017977587207096330286416692887910946555547851940402630657488671505820681908902000708383676273854845817711531764475730270069855571366959622842914819860834936475292719074168444365510704342711559699508093042880177904174497791",
			"-1.7976931348623159e308", "2e308", "1e309", "0.1e310", "100000000000000000000000e300",
			"4.9406564584124654e-324", "2.4703282292062327e-324", "2.4703282292062328e-324", "2.470328229206232720882e-324",
			"1e-324", "3e-324", "5e-325", "2.2250738585072011e-308", "2.2250738585072012e-308", "2.225073858507201136057409796709131975934819546351645648e-308",
			"0.0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000049406564584124654",
			"9007199254740993", "9007199254740992.5", "9007199254740993.0000000000000000001", "18014398509481985", "123456789012345678901234567890e-10",
			"0e0", "0E+0", "-0e-0", "0.0", "-0.000", "0e999999999999999999999", "0e-99999999999", "1e-99999999999999999999", "1e99999999999999999999",
			"1e0000000000000000000000000000000000000001", "1.0000000000000000000000000000000000000000000000001", "0.9999999999999999999999999999999999",
			"7.038531e-26", "8.41e21", "5e-324", "1e23", "8.533e+68", "4.1006e-184", "9.998e+307", "9.9538452227e-280", "6.47660115e-260", "7.4e+47", "5.92e+48", "7.35e+66", "8.32116e+55",
		}
		return lits[r.IntN(len(lits))]
	case 3:
		// Long mantissa, moderate exponent.
		s = digitsN(r, 1+r.IntN(40), true)
		if r.IntN(2) == 0 {
			s += "." + digitsN(r, 1+r.IntN(40), false)
		}
		s += "e" + strconv.Itoa(r.IntN(700)-350)
	case 4:
		// Very long mantissa (beyond the 800-digit decimal buffer).
		s = digitsN(r, 1+r.IntN(900), true)
		if r.IntN(2) == 0 {
			s += "." + digitsN(r, r.IntN(900)+1, false)
		}
		if r.IntN(2) == 0 {
			s += "e" + []string{"", "+", "-"}[r.IntN(3)] + strconv.Itoa(r.IntN(1200))
		}
	case 5:
		// Leading zeros in the fraction, trailing zeros in the mantissa.
		s = "0." + strings.Repeat("0", r.IntN(400)) + digitsN(r, 1+r.IntN(20), true)
		if r.IntN(2) == 0 {
			s += strings.Repeat("0", r.IntN(50))
		}
		if r.IntN(2) == 0 {
			s += "E" + strconv.Itoa(r.IntN(400))
		}
	case 6:
		// Integers around 2^53, 2^63, 2^64.
		base := []uint64{1 << 53, 1 << 63, math.MaxUint64, 1<<62 + 1, 1e15, 1e16, 1e17}[r.IntN(7)]
		s = strconv.FormatUint(base+uint64(r.IntN(9))-4, 10)
	case 7:
		// Exponents padded with zeros, extreme exponents.
		s = digitsN(r, 1+r.IntN(3), true) + "e" + []string{"", "+", "-"}[r.IntN(3)] + strings.Repeat("0", r.IntN(30)) + strconv.Itoa(r.IntN(400))
	case 8:
		// Shortest representations of random floats (round trip).
		f := randFiniteFloat(r)
		s = strconv.FormatFloat(f, "eEfg"[r.IntN(4)], -1, 64)
		s = strings.TrimPrefix(s, "+")
	case 9:
		// Decimal values near the 1e-6 and 1e21 cut-offs.
		s = digitsN(r, 1+r.IntN(17), true) + "e" + strconv.Itoa([]int{-6, -7, -5, 20, 21, 22, -22, -23}[r.IntN(8)]-r.IntN(17))
	default:
		s = digitsN(r, 1+r.IntN(6), true)
		if r.IntN(2) == 0 {
			s = "0"
		}
		if r.IntN(2) == 0 {
			s += "." + digitsN(r, 1+r.IntN(20), false)
		}
	}
	if r.IntN(3) == 0 {
		s = "-" + s
	}
	return s
}

// ---------------------------------------------------------------------------
// Strings

var advUnits = []uint16{
	0x0000, 0x0001, 0x0008, 0x0009, 0x000a, 0x000c, 0x000d, 0x001f, 0x0020, 0x0022, 0x0026, 0x002f, 0x003c, 0x003e,
	0x005c, 0x007f, 0x0080, 0x009f, 0x00a0, 0x00e9, 0x00ff, 0x0100, 0x0e01, 0x2027, 0x2028, 0x2029, 0x202a,
	0xd7ff, 0xd800, 0xd83d, 0xdbff, 0xdc00, 0xde00, 0xdfff, 0xe000, 0xfeff, 0xfffd, 0xfffe, 0xffff,
}

func escUnit(r *rand.Rand, u uint16) string {
	h := fmt.Sprintf("%04x", u)
	switch r.IntN(3) {
	case 0:
		h = strings.ToUpper(h)
	case 1:
		b := []byte(h)
		for i := range b {
			if r.IntN(2) == 0 {
				b[i] = strings.ToUpper(string(b[i]))[0]
			}
		}
		h = string(b)
	}
	return `\u` + h
}

// advJSONStringPieces are raw pieces of the inside of a JSON string literal
// (they may be invalid).
var advJSONStringPieces = []string{
	"a", " ", "hello", "<", ">", "&", "&amp;", "</script>", "/", `\/`, `\\`, `\"`, `\b`, `\f`, `\n`, `\r`, `\t`,
	"\u2028", "\u2029", "é", "ปาร์ตี้", "😀", "\ufffd", "\ufeff", "\x7f",
	"\xff", "\xfe", "\x80", "\xc0\xaf", "\xc3", "\xed\xa0\x80", "\xed\xbf\xbf", "\xe2\x80", "\xf4\x90\x80\x80", "\xf0\x9f\x98", "\xf8\x88\x80\x80\x80",
	"\x00", "\x01", "\x1f", "\t", "\n",
	`\x`, `\'`, `\a`, `\U0041`, `\u12`, `\u12g4`, `\u`, `\`,
}

// genAdvJSONString returns a JSON string literal (quoted), mostly valid.
func genAdvJSONString(r *rand.Rand, bad bool) string {
	var b strings.Builder
	b.WriteByte('"')
	for k := r.IntN(8); k > 0; k-- {
		switch r.IntN(6) {
		case 0, 1:
			u := advUnits[r.IntN(len(advUnits))]
			b.WriteString(escUnit(r, u))
		case 2:
			// Surrogate pair combinations.
			hi := uint16(0xd800 + r.IntN(0x400))
			lo := uint16(0xdc00 + r.IntN(0x400))
			switch r.IntN(4) {
			case 0:
				b.WriteString(escUnit(r, hi) + escUnit(r, lo))
			case 1:
				b.WriteString(escUnit(r, lo) + escUnit(r, hi))
			case 2:
				b.WriteString(escUnit(r, hi) + escUnit(r, hi))
			default:
				b.WriteString(escUnit(r, hi) + "x")
			}
		case 3:
			b.WriteString(escUnit(r, uint16(r.IntN(0x10000))))
		default:
			p := advJSONStringPieces[r.IntN(len(advJSONStringPieces))]
			if !bad {
				for strings.ContainsAny(p, "\x00\x01\x1f\t\n") || p == `\x` || p == `\'` || p == `\a` || p == `\U0041` || p == `\u12` || p == `\u12g4` || p == `\u` || p == `\` {
					p = advJSONStringPieces[r.IntN(len(advJSONStringPieces))]
				}
			}
			b.WriteString(p)
		}
	}
	b.WriteByte('"')
	return b.String()
}

// genAdvString returns a Go string rich in bytes that need escaping.
func genAdvString(r *rand.Rand) string {
	var b strings.Builder
	for k := r.IntN(10); k > 0; k-- {
		switch r.IntN(8) {
		case 0:
			b.WriteByte(byte(r.IntN(256)))
		case 1:
			b.WriteByte(byte(0x80 + r.IntN(0x80)))
		case 2:
			b.WriteRune(rune(r.IntN(0x110000)))
		case 3:
			// Encoded surrogates and out-of-range code points.
			c := 0xd800 + r.IntN(0x800)
			b.Write([]byte{0xed, byte(0x80 | (c>>6)&0x3f), byte(0x80 | c&0x3f)})
		default:
			b.WriteString(strPieces[r.IntN(len(strPieces))])
		}
	}
	return b.String()
}

// ---------------------------------------------------------------------------
// Documents

var advWS = []string{"", "", " ", "\n", "\t", "\r\n", "  \n\t"}

func ws(r *rand.Rand) string { return advWS[r.IntN(len(advWS))] }

func genAdvDoc(r *rand.Rand, depth int, bad bool) string {
	top := 9
	if depth <= 0 {
		top = 6
	}
	switch r.IntN(top) {
	case 0, 1:
		return genNumberLit(r)
	case 2, 3:
		return genAdvJSONString(r, bad)
	case 4:
		return []string{"null", "true", "false"}[r.IntN(3)]
	case 5:
		return strconv.FormatFloat(randFiniteFloat(r), 'g', -1, 64)
	case 6, 7:
		n := r.IntN(5)
		var b strings.Builder
		b.WriteString("{" + ws(r))
		for i := 0; i < n; i++ {
			if i > 0 {
				b.WriteString(ws(r) + "," + ws(r))
			}
			b.WriteString(genAdvJSONString(r, bad) + ws(r) + ":" + ws(r) + genAdvDoc(r, depth-1, bad))
		}
		b.WriteString(ws(r) + "}")
		return b.String()
	default:
		n := r.IntN(5)
		var b strings.Builder
		b.WriteString("[" + ws(r))
		for i := 0; i < n; i++ {
			if i > 0 {
				b.WriteString(ws(r) + "," + ws(r))
			}
			b.WriteString(genAdvDoc(r, depth-1, bad))
		}
		b.WriteString(ws(r) + "]")
		return b.String()
	}
}

func advHandTexts() []string {
	t := []string{
		"\ufeff{}", "\ufeff", "{}\ufeff", "\v1", "1\v", "\f[]", "[\u00a01]", "[1\u00a0]", "\u2028[]", "[]\x00", "\x001",
		"nul", "nulx", "null1", "true1", "[true false]", "tRue", "[nul]", "[tru", "{\"a\":fals", "{\"a\":nul}", "{\"a\" :t}",
		"-", "-a", "--1", "-.1", "-0.", "-0e", "0.e1", "0e+-1", "1e--1", "1.5e", "01.5", "00", "0x", "1_0", "1.2.3", "1e2e3", "[-]", "[1.]", "{\"a\":-}",
		`{"a":1,"a":2,"a":{"x":1}}`, `{"a":{"b":1},"a":null}`, `{"a":[1],"a":"x"}`, `{"\u0061":1,"a":2}`, `{"a":1,"\u0061":2}`,
		`{"\ud800":1,"\udc00":2,"\ufffd":3}`, "{\"\xff\":1,\"\ufffd\":2}", `{"\ud83d\ude00":1,"😀":2}`,
		`"\ud800\udc00"`, `"\udbff\udfff"`, `"\uDBFF\uDFFF"`, `"\ud800\uD800"`, `"\ud800\u0000"`, `"\ud800\\u0000"`, `"\ud800\n"`, `"\ud800`, `"\ud800\`, `"\ud800\u`, `"\ud800\ud`, `"\ud800\udc`, `"\ud800\udc0`, `"\ud800\udc00`,
		`"\u00`, `"\u`, `"\`, `"abc`, `"abc\"`, "\"\xe2\x80", "\"\xe2", "\"\xf0\x9f\x98", "\"\xf0\x9f\x98\x80", "\"\xc3\"", "\"\xed\xa0\x80\xed\xb0\x80\"",
		`["\u003c\u003e\u0026", "<>&", "\u2028\u2029", "` + "\u2028\u2029" + `"]`, `{"<a>":"&b","\u0026":"\u003C"}`, `"\/\/"`, `"\u002f"`, `"\u001F\u001f\u007F"`,
		strings.Repeat("[", 9999) + "1" + strings.Repeat("]", 9999),
		strings.Repeat("[", 10000) + "1" + strings.Repeat("]", 10000),
		strings.Repeat("[", 10001) + "1" + strings.Repeat("]", 10001),
		strings.Repeat(`{"":`, 9999) + "[]" + strings.Repeat("}", 9999),
		strings.Repeat(`{"":`, 10000) + "[]" + strings.Repeat("}", 10000),
		strings.Repeat(`[{"a":`, 5000) + "0" + strings.Repeat("}]", 5000),
		strings.Repeat(`[{"a":`, 5001) + "0" + strings.Repeat("}]", 5001),
		strings.Repeat("[", 10001),
		strings.Repeat("[", 10000) + strings.Repeat("]", 9999),
		`"` + strings.Repeat("x", 70000) + `"`,
		`"` + strings.Repeat(`\u00e9`, 12000) + `"`,
		`"` + strings.Repeat("é", 40000) + "\xff" + `"`,
		`["` + strings.Repeat("a", 4093) + `","` + strings.Repeat("b", 4096) + `"]`,
		strings.Repeat("1", 5000), "0." + strings.Repeat("0", 5000) + "1", "1" + strings.Repeat("0", 400), "1" + strings.Repeat("0", 400) + ".5e-400",
		"[" + strings.Repeat("1,", 3000) + "1]",
		strings.Repeat(" ", 5000) + "1" + strings.Repeat(" ", 5000),
		strings.Repeat("{}", 50), strings.Repeat("[] ", 50), strings.Repeat("1 ", 50), strings.Repeat(`"a"`, 50), "1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32 33 34 35 36 37 38 39 40 41 42",
		"[1]x", "[1] x", "{} ,", "{}:", "1,2", "[1],[2]", "\"a\":1", "null,",
		"  {\"a\" : [ 1 , 2 ] }  \n\n  ", "{\"a\":[1,2]}\n   ", "[]\r\n", "[] \t\n \t",
	}
	return t
}

// truncationDocs are documents whose every prefix is a test case.
func truncationDocs(r *rand.Rand) []string {
	docs := []string{
		`{"a":[1,-2.5e+10,true,false,null,"s\u00e9\n"],"b":{"c":{}},"d":[[],[{}]]}`,
		` [ "x\ud83d\ude00y" , 0.000001 , -0 , 1E400 , {"k" : "v"} ] `,
		"{\"\xff<\":\"\u2028&\",\"e\":1e-7}\n",
		`[1.7976931348623157e308,5e-324,"\/\b\f\n\r\t\"\\"]`,
		"1 [2] {\"3\":4} \"5\" true null",
	}
	for i := 0; i < 3; i++ {
		docs = append(docs, genAdvDoc(r, 3, false))
	}
	return docs
}

func writeAdvText(w io.Writer, r *rand.Rand, n int) {
	advIndent = true
	for _, s := range advHandTexts() {
		textCase(w, r, []byte(s))
	}
	for _, d := range truncationDocs(r) {
		for i := 0; i <= len(d); i++ {
			textCase(w, r, []byte(d[:i]))
		}
	}
	for i := 0; i < n; i++ {
		textCase(w, r, []byte(genAdvTextInput(r)))
	}
	log.Printf("advtext: re-drew %d Indent prefix/indent pairs that would hang", indentHangsAvoided)
}

// genAdvTextInput returns one random -mode advtext input.
func genAdvTextInput(r *rand.Rand) string {
	switch r.IntN(10) {
	case 0, 1:
		// Arrays of number literals.
		k := 1 + r.IntN(40)
		nums := make([]string, k)
		for j := range nums {
			nums[j] = genNumberLit(r)
		}
		return ws(r) + "[" + strings.Join(nums, ws(r)+","+ws(r)) + "]" + ws(r)
	case 2:
		// A lone number (terminated by EOF in streams).
		return genNumberLit(r) + ws(r)
	case 3:
		// Objects with adversarial names and string values.
		k := r.IntN(6)
		var b strings.Builder
		b.WriteString("{")
		for j := 0; j < k; j++ {
			if j > 0 {
				b.WriteString(",")
			}
			b.WriteString(genAdvJSONString(r, false) + ":" + genAdvJSONString(r, false))
		}
		b.WriteString("}")
		return b.String()
	case 4:
		// Possibly invalid strings.
		return "[" + genAdvJSONString(r, true) + "," + genAdvJSONString(r, true) + "]"
	case 5:
		// Streams of documents.
		var in string
		for k := 1 + r.IntN(4); k > 0; k-- {
			in += genAdvDoc(r, 2, false) + []string{" ", "\n", "", "\t"}[r.IntN(4)]
		}
		return in
	case 6:
		return string(mutate(r, []byte(genAdvDoc(r, 3, false))))
	default:
		return genAdvDoc(r, 3, r.IntN(4) == 0)
	}
}

// ---------------------------------------------------------------------------
// Encode

func goTestMarshalFloatInputs() (f64 []float64, f32 []float32) {
	digits := "1.2345678901234567890123"
	for i := len(digits); i >= 2; i-- {
		for exp := -30; exp <= 30; exp++ {
			for _, sign := range "+-" {
				for bits := 32; bits <= 64; bits += 32 {
					s := fmt.Sprintf("%c%se%d", sign, digits[:i], exp)
					f, err := strconv.ParseFloat(s, bits)
					if err != nil {
						panic(err)
					}
					if bits == 32 {
						g := float32(f)
						f32 = append(f32, g, math.Nextafter32(g, float32(math.Inf(1))), math.Nextafter32(g, float32(math.Inf(-1))))
					} else {
						f64 = append(f64, f, math.Nextafter(f, math.Inf(1)), math.Nextafter(f, math.Inf(-1)))
					}
				}
			}
		}
	}
	return f64, f32
}

func advFloat64(r *rand.Rand) float64 {
	switch r.IntN(8) {
	case 0:
		// k ulps away from the cut-offs and powers of ten.
		x := []float64{1e-6, 1e21, 1e-7, 1e20, 1e22, 1e-5, 1, 10, 0.1, 1e15, 1e16, 1e17, 1e23, 5e-324, math.MaxFloat64, 2.2250738585072014e-308}[r.IntN(16)]
		for k := r.IntN(5); k > 0; k-- {
			x = math.Nextafter(x, math.Inf(1))
		}
		for k := r.IntN(5); k > 0; k-- {
			x = math.Nextafter(x, 0)
		}
		return x
	case 1:
		return math.Float64frombits(uint64(r.Int64N(1 << 52)))
	case 2:
		return float64(r.Int64N(1<<54)) * math.Pow(2, float64(r.IntN(40)-20))
	case 3:
		f, _ := strconv.ParseFloat(genNumberLit(r), 64)
		if math.IsInf(f, 0) {
			return math.MaxFloat64
		}
		return f
	default:
		for {
			f := math.Float64frombits(r.Uint64())
			if !math.IsNaN(f) && !math.IsInf(f, 0) {
				return f
			}
		}
	}
}

func advFloat32(r *rand.Rand) float32 {
	switch r.IntN(4) {
	case 0:
		x := []float32{1e-6, 1e21, 1e-7, 1e20, 1e22, 1, 0.1, math.MaxFloat32, math.SmallestNonzeroFloat32, 16777216}[r.IntN(10)]
		for k := r.IntN(5); k > 0; k-- {
			x = math.Nextafter32(x, float32(math.Inf(1)))
		}
		for k := r.IntN(5); k > 0; k-- {
			x = math.Nextafter32(x, 0)
		}
		return x
	case 1:
		return float32(advFloat64(r))
	default:
		for {
			f := math.Float32frombits(r.Uint32())
			if !math.IsNaN(float64(f)) && !math.IsInf(float64(f), 0) {
				return f
			}
		}
	}
}

func genAdvValue(r *rand.Rand, depth int) any {
	top := 14
	if depth <= 0 {
		top = 9
	}
	switch r.IntN(top) {
	case 0:
		return advFloat64(r)
	case 1:
		return advFloat32(r)
	case 2, 3:
		return genAdvString(r)
	case 4:
		s := genAdvString(r)
		return []any{template.HTML(s), template.JS(s), template.URL(s)}[r.IntN(3)]
	case 5:
		if r.IntN(2) == 0 {
			return json.Number(genNumberLit(r))
		}
		return json.Number(numberLits[r.IntN(len(numberLits))])
	case 6:
		// MarshalJSON output with adversarial strings and numbers.
		return jm{genAdvDoc(r, 2, r.IntN(5) == 0)}
	case 7:
		return tm{genAdvString(r)}
	case 8:
		return genValue(r, 0, r.IntN(5) == 0)
	case 9, 10:
		n := r.IntN(6)
		m := make(map[string]any, n)
		for i := 0; i < n; i++ {
			m[genAdvString(r)] = genAdvValue(r, depth-1)
		}
		return m
	case 11:
		n := r.IntN(6)
		s := make([]any, n)
		for i := range s {
			s[i] = genAdvValue(r, depth-1)
		}
		return s
	case 12:
		n := r.IntN(6)
		s := make([]string, n)
		for i := range s {
			s[i] = genAdvString(r)
		}
		return s
	default:
		return genValue(r, depth-1, r.IntN(5) == 0)
	}
}

var advIndent bool

func writeAdvEncode(w io.Writer, r *rand.Rand, n int) {
	advIndent = true
	var vals []any
	f64, f32 := goTestMarshalFloatInputs()
	for len(f64) > 0 {
		k := min(len(f64), 64)
		vals = append(vals, f64[:k])
		f64 = f64[k:]
	}
	for len(f32) > 0 {
		k := min(len(f32), 64)
		s := make([]any, k)
		for i := range s {
			s[i] = f32[i]
		}
		vals = append(vals, s)
		f32 = f32[k:]
	}
	for i := 0; i < n; i++ {
		switch r.IntN(4) {
		case 0:
			s := make([]float64, 32)
			for j := range s {
				s[j] = advFloat64(r)
				if r.IntN(2) == 0 {
					s[j] = -s[j]
				}
			}
			vals = append(vals, s)
		case 1:
			s := make([]any, 32)
			for j := range s {
				f := advFloat32(r)
				if r.IntN(2) == 0 {
					f = -f
				}
				s[j] = f
			}
			vals = append(vals, s)
		default:
			vals = append(vals, genAdvValue(r, 3))
		}
	}
	for _, v := range vals {
		writeEncodeCase(w, r, v)
	}
}
