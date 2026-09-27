package main

// Second-round (independent verifier) corpora for crates/go-yaml:
//
//	go run ./tools/go-oracle/go-yaml verify -out <dir> [-n N] [-seed S]
//
// writes verify-*.corpus files (same format as "corpus"), to be decoded
// with "run". The generators target areas the first-round corpora cover
// thinly: large maps with colliding keys (Go map key equality, NaN and
// signed-zero keys, key replacement), merges and anchors, numeric scalar
// resolution (ParseInt/ParseUint/ParseFloat on random numerals), key
// stringification (cast.ToStringE of floats), timestamps, Unicode and
// special line breaks, block scalars and token soup.

import (
	"flag"
	"fmt"
	"math"
	"path/filepath"
	"strconv"
	"strings"
)

func cmdVerify(args []string) {
	fset := flag.NewFlagSet("verify", flag.ExitOnError)
	outDir := fset.String("out", "", "output directory")
	n := fset.Int("n", 20000, "cases per generator")
	seed := fset.Uint64("seed", 0x2545f4914f6cdd1d, "seed")
	_ = fset.Parse(args)

	gens := []struct {
		name string
		f    func(r *rng) string
	}{
		{"maps", genMaps},
		{"numbers", genNumbers},
		{"floatkeys", genFloatKeys},
		{"timestamps", genTimestamps},
		{"unicode", genUnicode},
		{"blocks", genBlocks},
		{"soup", genSoup},
		{"anchors", genAnchors},
		{"encodings", genEncodings},
	}
	for gi, g := range gens {
		c := newCorpus()
		r := &rng{state: *seed + uint64(gi)*0x9e3779b97f4a7c15}
		for i := 0; len(c.names) < *n && i < *n*5; i++ {
			c.add("v-"+g.name, []byte(g.f(r)))
		}
		c.write(filepath.Join(*outDir, "verify-"+g.name+".corpus"))
	}
}

// collidingKeys are scalars that are equal (or stringify equally) under
// Go's interface equality / cast.ToStringE in various combinations.
var collidingKeys = []string{
	"a", `"a"`, "'a'", "!!str a", "b", "c", "1", `"1"`, "!!str 1", "1.0", "!!float 1", "01", "0x1",
	"+1", "1_0", "10", "0o1", "0b1", "0.0", "-0.0", "0", "-0", "+0.0", ".nan", ".NaN", ".inf",
	"-.inf", "~", "null", `""`, "!!null ''", "true", "y", "yes", "on", "True", `"true"`, "false",
	"n", "no", "off", "2001-01-01", `"2001-01-01"`, "!!binary YQ==", "!!binary MQ==",
	"18446744073709551615", "9223372036854775807", "9223372036854775808", "1e3", "1000",
	"1000.0", "1e21", "1e20", "0.1", "1e-7", "123456789012345678", "<<X", "k1", "k2", "k3",
	"k4", "k5", "k6", "k7", "k8", "k9", "k10",
}

func genMaps(r *rng) string {
	n := 1 + r.intn(30)
	flow := r.intn(2) == 0
	var parts []string
	for i := 0; i < n; i++ {
		k := r.pick(collidingKeys)
		var v string
		switch r.intn(6) {
		case 0:
			v = strconv.Itoa(r.intn(100))
		case 1:
			v = r.pick(collidingKeys)
		case 2:
			v = "{" + r.pick(collidingKeys) + ": " + strconv.Itoa(i) + "}"
		case 3:
			v = "[" + r.pick(collidingKeys) + ", " + strconv.Itoa(i) + "]"
		default:
			v = "v" + strconv.Itoa(i)
		}
		if r.intn(15) == 0 {
			k = "<<"
			v = "{" + r.pick(collidingKeys) + ": m" + strconv.Itoa(i) + ", " + r.pick(collidingKeys) + ": n}"
			if r.intn(2) == 0 {
				v = "[" + v + ", {" + r.pick(collidingKeys) + ": s}]"
			}
		}
		parts = append(parts, k+": "+v)
	}
	var body string
	if flow {
		body = "{" + strings.Join(parts, ", ") + "}"
	} else {
		body = "\n  " + strings.Join(parts, "\n  ")
	}
	switch r.intn(4) {
	case 0:
		// Top level (map[string]interface{} target keys are the source text).
		if flow {
			return body + "\n"
		}
		return strings.Join(parts, "\n") + "\n"
	case 1:
		return "- " + strings.TrimPrefix(body, "\n  ") + "\n"
	default:
		return "m: " + body + "\n"
	}
}

func randDigits(r *rng, n int) string {
	var b strings.Builder
	for i := 0; i < n; i++ {
		b.WriteByte(byte('0' + r.intn(10)))
	}
	return b.String()
}

func sprinkleUnderscores(r *rng, s string) string {
	if r.intn(4) != 0 || len(s) == 0 {
		return s
	}
	var b strings.Builder
	for i := 0; i < len(s); i++ {
		if r.intn(5) == 0 {
			b.WriteByte('_')
		}
		b.WriteByte(s[i])
	}
	if r.intn(6) == 0 {
		b.WriteByte('_')
	}
	return b.String()
}

func randNumeral(r *rng) string {
	sign := r.pick([]string{"", "", "", "-", "+"})
	switch r.intn(12) {
	case 0:
		return sign + r.pick([]string{"0x", "0X", "0o", "0O", "0b", "0B", "0"}) + sprinkleUnderscores(r, r.pick([]string{randDigits(r, 1+r.intn(20)), "7fffffffffffffff", "8000000000000000", "ffffffffffffffff", "10000000000000000", "1f", "DEADbeef", "1_0"}))
	case 1:
		// Near int64/uint64 bounds.
		base := r.pick([]string{"9223372036854775807", "9223372036854775808", "18446744073709551615", "18446744073709551616", "9223372036854775806", "18446744073709551614"})
		return sign + sprinkleUnderscores(r, base)
	case 2:
		// Hex floats and other ParseFloat-only syntax.
		return sign + r.pick([]string{"0x1p-2", "0x1.8p3", "0x.8p1", "0x1P+2", "inf", "Inf", "infinity", "nan", "NaN", "1e", "e5", "1.e5", ".e5", "1e+", "0x1p", "1p5"})
	case 3:
		// Integer with leading zeros (octal / float fallback).
		return sign + "0" + sprinkleUnderscores(r, randDigits(r, 1+r.intn(22)))
	}
	intPart := sprinkleUnderscores(r, randDigits(r, r.intn(25)))
	s := sign + intPart
	if r.intn(2) == 0 {
		s += "." + sprinkleUnderscores(r, randDigits(r, r.intn(25)))
	}
	if r.intn(3) == 0 {
		exp := r.intn(700) - 350
		if r.intn(4) == 0 {
			exp = r.intn(40) - 20
		}
		e := r.pick([]string{"e", "E"})
		if exp >= 0 && r.intn(2) == 0 {
			e += "+"
		}
		s += e + strconv.Itoa(exp)
	}
	if s == "" || s == sign {
		s += "0"
	}
	return s
}

func genNumbers(r *rng) string {
	var b strings.Builder
	for i := 0; i < 1+r.intn(6); i++ {
		num := randNumeral(r)
		switch r.intn(8) {
		case 0:
			fmt.Fprintf(&b, "k%d: !!float %s\n", i, num)
		case 1:
			fmt.Fprintf(&b, "k%d: !!int %s\n", i, num)
		case 2:
			fmt.Fprintf(&b, "k%d: [%s, %s]\n", i, num, randNumeral(r))
		case 3:
			fmt.Fprintf(&b, "k%d: {%s: x}\n", i, num)
		default:
			fmt.Fprintf(&b, "k%d: %s\n", i, num)
		}
	}
	return b.String()
}

// genFloatKeys puts float-valued keys into nested maps: stringifyMapKeys
// formats them with cast.ToStringE (FormatFloat 'f', -1).
func genFloatKeys(r *rng) string {
	var parts []string
	for i := 0; i < 1+r.intn(5); i++ {
		var k string
		switch r.intn(4) {
		case 0:
			f := math.Float64frombits(r.next())
			if math.IsNaN(f) || math.IsInf(f, 0) {
				f = 1.5
			}
			k = strconv.FormatFloat(f, r.pick([]string{"e", "g"})[0], -1, 64)
		case 1:
			f := float64(r.intn(1000000)) / math.Pow(10, float64(r.intn(30)))
			k = strconv.FormatFloat(f, 'e', r.intn(20), 64)
		case 2:
			k = strconv.FormatFloat(math.Pow(10, float64(r.intn(600)-300))*(1+float64(r.intn(9))), 'g', -1, 64)
		default:
			k = randNumeral(r)
		}
		if !strings.ContainsAny(k, ".eE") && r.intn(2) == 0 {
			k += ".0"
		}
		parts = append(parts, k+": "+strconv.Itoa(i))
	}
	return "m: {" + strings.Join(parts, ", ") + "}\n"
}

func genTimestamps(r *rng) string {
	num := func(n int) string {
		if r.intn(5) == 0 {
			return strconv.Itoa(r.intn(100))
		}
		return fmt.Sprintf("%0*d", n, r.intn(int(math.Pow10(n))))
	}
	s := num(4) + r.pick([]string{"-", "-", "-", "/", ""}) + num(2) + r.pick([]string{"-", "-", "."}) + num(2)
	if r.intn(3) != 0 {
		s += r.pick([]string{"T", "t", " ", "  ", "\t", "x"}) + num(2) + ":" + num(2) + ":" + num(2)
		if r.intn(2) == 0 {
			s += r.pick([]string{".", ",", "."}) + randDigits(r, r.intn(14))
		}
		switch r.intn(5) {
		case 0:
			s += "Z"
		case 1:
			s += r.pick([]string{"+", "-", " +", "*"}) + num(2) + r.pick([]string{":", ":", ""}) + num(2)
		case 2:
			s += r.pick([]string{"z", " Z", "+05", "-5", "UTC"})
		}
	}
	switch r.intn(5) {
	case 0:
		return "k: " + s + "\n" + s + ": v\nm: {" + s + ": x}\n"
	case 1:
		return "k: !!timestamp " + s + "\n"
	case 2:
		return "k: \"" + s + "\"\n"
	case 3:
		return "- " + s + "\n- !!str " + s + "\n"
	}
	return "k: " + s + "\n"
}

var unicodeRunes = []rune{
	'a', 'b', ' ', ' ', '\t', '\n', '\r', ':', '-', '#', '"', '\'', '\\', '[', ']', '{', '}', ',', '&', '*', '!', '|', '>', '?', '%', '@', '`',
	0x80, 0x85, 0x9f, 0xa0, 0xe9, 0xff, 0x100, 0x2028, 0x2029, 0xfeff, 0xfffe, 0xfffd, 0xffff,
	0xd7ff, 0xe000, 0x3000, 0x65e5, 0x1f600, 0x10ffff, 0x7f, 0x1, 0x0, 0x1b, 0x200b, 0x2060,
}

func genUnicode(r *rng) string {
	var b strings.Builder
	randText := func(n int) string {
		var t strings.Builder
		for i := 0; i < n; i++ {
			if r.intn(4) == 0 {
				t.WriteRune(unicodeRunes[r.intn(len(unicodeRunes))])
			} else {
				t.WriteByte(byte('a' + r.intn(26)))
			}
		}
		return t.String()
	}
	for i := 0; i < 1+r.intn(4); i++ {
		t := randText(r.intn(12))
		switch r.intn(8) {
		case 0:
			fmt.Fprintf(&b, "k%d: \"%s\"\n", i, strings.ReplaceAll(t, `"`, `\"`))
		case 1:
			fmt.Fprintf(&b, "k%d: '%s'\n", i, strings.ReplaceAll(t, "'", "''"))
		case 2:
			fmt.Fprintf(&b, "k%d: |\n  %s\n  %s\n", i, t, randText(5))
		case 3:
			fmt.Fprintf(&b, "%s: v%d\n", t, i)
		case 4:
			esc := r.pick([]string{`\x41`, `\x85`, `\u2028`, `\u00e9`, `\U0001F600`, `\N`, `\_`, `\L`, `\P`, `\0`, `\e`, `\ud800`, `\U00110000`, `\x`, `\u12`, `\/`, `\	`, "\\\n"})
			fmt.Fprintf(&b, "k%d: \"a%sb\"\n", i, esc)
		case 5:
			fmt.Fprintf(&b, "k%d: >\n  %s\n\n  %s\n", i, t, randText(4))
		default:
			fmt.Fprintf(&b, "k%d: %s\n", i, t)
		}
	}
	s := b.String()
	if r.intn(10) == 0 {
		s = "\ufeff" + s
	}
	return s
}

func genBlocks(r *rng) string {
	ind := r.pick([]string{"|", ">", "|-", "|+", ">-", ">+", "|1", "|2", "|9", ">1-", "|+2", "|-3", "|0", "|10", "| #c", ">  #c", "|x"})
	var b strings.Builder
	b.WriteString(r.pick([]string{"k: ", "- ", "? ", "--- ", "k: !!str ", "k: &a "}) + ind + "\n")
	lines := 1 + r.intn(6)
	for i := 0; i < lines; i++ {
		switch r.intn(7) {
		case 0:
			b.WriteString("\n")
		case 1:
			b.WriteString(strings.Repeat(" ", r.intn(6)) + "\n")
		case 2:
			b.WriteString(strings.Repeat(" ", 1+r.intn(5)) + "\ttab\n")
		case 3:
			b.WriteString(strings.Repeat(" ", 2+r.intn(4)) + "more " + strconv.Itoa(i) + "  \n")
		case 4:
			b.WriteString(r.pick([]string{"---\n", "...\n", "# c\n", "x: y\n", " - z\n", "\r\n", "  a\r\n"}))
		default:
			b.WriteString("  line " + strconv.Itoa(i) + r.pick([]string{"", " ", "  ", " # not comment", "\t"}) + "\n")
		}
	}
	if r.intn(2) == 0 {
		b.WriteString(r.pick([]string{"next: 1\n", "- n\n", "...\n", "---\nb\n", ""}))
	}
	s := b.String()
	if r.intn(4) == 0 {
		s = strings.TrimRight(s, "\n")
	}
	return s
}

var soupTokens = []string{
	"a", "b", "1", "-", "- ", ":", ": ", "? ", ",", "[", "]", "{", "}", "#", " #c", "&x ", "*x", "!!str ", "!t ", "|", ">", "\"q\"", "'s'",
	"\"", "'", " ", "  ", "\t", "\n", "\n  ", "\n    ", "\n- ", "---", "...", "%YAML 1.1", "<<", "~", "\r\n", "\\", "@", "`",
	"\u0085", "\u2028", "\ufeff", "\u00e9",
}

func genSoup(r *rng) string {
	var b strings.Builder
	for i := 0; i < 1+r.intn(25); i++ {
		b.WriteString(r.pick(soupTokens))
	}
	return b.String()
}

func genAnchors(r *rng) string {
	var b strings.Builder
	names := []string{"a", "b", "c", "a1", "x"}
	var defined []string
	for i := 0; i < 1+r.intn(8); i++ {
		nm := r.pick(names)
		var def string
		switch r.intn(5) {
		case 0:
			def = "&" + nm + " {k" + strconv.Itoa(r.intn(3)) + ": " + strconv.Itoa(i) + ", z: " + strconv.Itoa(i) + "}"
		case 1:
			def = "&" + nm + " [1, " + strconv.Itoa(i) + "]"
		case 2:
			def = "&" + nm + " s" + strconv.Itoa(i)
		case 3:
			def = "&" + nm + "\n  k" + strconv.Itoa(r.intn(3)) + ": " + strconv.Itoa(i) + "\n  y: " + strconv.Itoa(i)
		default:
			def = "&" + nm + " {}"
		}
		key := "e" + strconv.Itoa(i)
		if len(defined) > 0 && r.intn(2) == 0 {
			ref := r.pick(defined)
			switch r.intn(5) {
			case 0:
				def = "*" + ref
			case 1:
				def = "{<<: *" + ref + ", k0: over}"
			case 2:
				def = "{<<: [*" + ref + ", *" + r.pick(defined) + "], z: last}"
			case 3:
				def = "\n  k1: before\n  <<: *" + ref + "\n  k2: after"
			default:
				def = "[*" + ref + ", *" + ref + "]"
			}
			if r.intn(8) == 0 {
				key = "*" + ref + " "
			}
		}
		defined = append(defined, nm)
		fmt.Fprintf(&b, "%s: %s\n", key, def)
	}
	return b.String()
}

// genEncodings builds longer documents in UTF-8, UTF-16LE and UTF-16BE
// (with and without BOM), with multi-byte characters, surrogate pairs,
// lone surrogates and invalid bytes placed around the reader's 512-byte
// raw / 1536-byte decoded buffer boundaries, and simple keys around the
// 1024-character limit made of multi-byte characters.
func genEncodings(r *rng) string {
	chars := []rune{'a', 'b', ' ', 0xe9, 0x65e5, 0x1f600, 0x85, 0x2028, 0xfeff, 0x10000, 0xffff}
	var doc strings.Builder
	pad := func(n int) string {
		var b strings.Builder
		ch := chars[r.intn(len(chars))]
		if ch == ' ' || ch == 0x85 || ch == 0x2028 || ch == 0xfeff {
			ch = 'x'
		}
		for i := 0; i < n; i++ {
			b.WriteRune(ch)
		}
		return b.String()
	}
	switch r.intn(4) {
	case 0:
		// Long simple key made of multi-byte characters.
		n := 1015 + r.intn(20)
		k := pad(n)
		if r.intn(2) == 0 {
			doc.WriteString("{" + k + ": v}\n")
		} else {
			doc.WriteString(k + ": v\nz: 1\n")
		}
	default:
		for i := 0; i < 1+r.intn(4); i++ {
			fmt.Fprintf(&doc, "k%d: %s\n", i, pad(r.intn(700)))
			if r.intn(3) == 0 {
				fmt.Fprintf(&doc, "s%d: \"%s\"\n", i, pad(r.intn(300)))
			}
		}
	}
	s := doc.String()
	switch r.intn(4) {
	case 0:
		return s
	case 1:
		// Invalid UTF-8 byte at a random position.
		b := []byte(s)
		pos := r.intn(len(b) + 1)
		bad := []byte{0xff, 0xc3, 0x80, 0xed, 0xf4, 0x00, 0x01}[r.intn(7)]
		return string(append(b[:pos], append([]byte{bad}, b[pos:]...)...))
	}
	// UTF-16.
	be := r.intn(2) == 0
	var out []byte
	put := func(u uint16) {
		if be {
			out = append(out, byte(u>>8), byte(u))
		} else {
			out = append(out, byte(u), byte(u>>8))
		}
	}
	if r.intn(4) != 0 {
		put(0xfeff)
	}
	for _, c := range s {
		if c >= 0x10000 {
			c -= 0x10000
			put(uint16(0xd800 + (c >> 10)))
			put(uint16(0xdc00 + (c & 0x3ff)))
		} else {
			put(uint16(c))
		}
	}
	if r.intn(5) == 0 && len(out) > 2 {
		// Lone surrogate or truncated unit.
		pos := 2 * r.intn(len(out)/2)
		if r.intn(2) == 0 {
			out = out[:len(out)-1]
		} else {
			out[pos], out[pos+1] = 0xd8, 0xd8
		}
	}
	return string(out)
}
