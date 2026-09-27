//go:build go1.27

package main

import (
	"bufio"
	"bytes"
	"compress/gzip"
	"encoding/binary"
	"flag"
	"fmt"
	"math/rand/v2"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"unicode"
	"unicode/utf16"
	"unicode/utf8"
)

// ---------------------------------------------------------------------------
// FNV-1a 64, mirrored in the Rust tests.

type fnv64 uint64

func newFNV() fnv64 { return 0xcbf29ce484222325 }

func (h *fnv64) byte(b byte) {
	*h ^= fnv64(b)
	*h *= 0x100000001b3
}

func (h *fnv64) bytes(p []byte) {
	for _, b := range p {
		h.byte(b)
	}
}

func (h *fnv64) i32(v int32) {
	var b [4]byte
	binary.LittleEndian.PutUint32(b[:], uint32(v))
	h.bytes(b[:])
}

func (h *fnv64) bool(v bool) {
	if v {
		h.byte(1)
	} else {
		h.byte(0)
	}
}

// ---------------------------------------------------------------------------
// Domains.

// extraRunes are the out-of-range runes appended to 0..=MaxRune.
var extraRunes = []rune{-1, -2, -0x80000000, 0x110000, 0x110001, 0x1FFFFF, 0x7FFFFFFF, -0xFFFD, 0xFFFFFF, 0x10FFFF + 0x20}

func forAllRunes(f func(r rune)) {
	for r := rune(0); r <= unicode.MaxRune; r++ {
		f(r)
	}
	for _, r := range extraRunes {
		f(r)
	}
}

type hashLine struct {
	name  string
	count int
	hash  fnv64
}

func predLine(name string, p func(rune) bool) hashLine {
	h := newFNV()
	n := 0
	forAllRunes(func(r rune) {
		v := p(r)
		if v {
			n++
		}
		h.bool(v)
	})
	return hashLine{name, n, h}
}

func mapLine(name string, m func(rune) rune) hashLine {
	h := newFNV()
	n := 0
	forAllRunes(func(r rune) {
		v := m(r)
		if v != r {
			n++
		}
		h.i32(v)
	})
	return hashLine{name, n, h}
}

func sortedKeys(m map[string]*unicode.RangeTable) []string {
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	return keys
}

func unicodeLines() []hashLine {
	var lines []hashLine
	preds := []struct {
		name string
		f    func(rune) bool
	}{
		{"IsLetter", unicode.IsLetter},
		{"IsDigit", unicode.IsDigit},
		{"IsNumber", unicode.IsNumber},
		{"IsMark", unicode.IsMark},
		{"IsSpace", unicode.IsSpace},
		{"IsPunct", unicode.IsPunct},
		{"IsSymbol", unicode.IsSymbol},
		{"IsPrint", unicode.IsPrint},
		{"IsGraphic", unicode.IsGraphic},
		{"IsControl", unicode.IsControl},
		{"IsUpper", unicode.IsUpper},
		{"IsLower", unicode.IsLower},
		{"IsTitle", unicode.IsTitle},
		{"utf16.IsSurrogate", utf16.IsSurrogate},
		{"utf8.ValidRune", utf8.ValidRune},
		{"In/Graphic", func(r rune) bool { return unicode.In(r, unicode.GraphicRanges...) }},
		{"In/Print", func(r rune) bool { return unicode.In(r, unicode.PrintRanges...) }},
		{"In/Lu,Nd,Han", func(r rune) bool { return unicode.In(r, unicode.Lu, unicode.Nd, unicode.Han) }},
		{"IsOneOf/Graphic", func(r rune) bool { return unicode.IsOneOf(unicode.GraphicRanges, r) }},
		{"IsOneOf/empty", func(r rune) bool { return unicode.IsOneOf(nil, r) }},
	}
	for _, p := range preds {
		lines = append(lines, predLine("pred/"+p.name, p.f))
	}
	maps := []struct {
		name string
		m    map[string]*unicode.RangeTable
	}{
		{"Categories", unicode.Categories},
		{"Scripts", unicode.Scripts},
		{"Properties", unicode.Properties},
		{"FoldCategory", unicode.FoldCategory},
		{"FoldScript", unicode.FoldScript},
	}
	for _, m := range maps {
		for _, k := range sortedKeys(m.m) {
			t := m.m[k]
			lines = append(lines, predLine(m.name+"/"+k, func(r rune) bool { return unicode.Is(t, r) }))
		}
	}
	for _, k := range sortedKeys(exportedTables) {
		t := exportedTables[k]
		lines = append(lines, predLine("exported/"+k, func(r rune) bool { return unicode.Is(t, r) }))
	}
	rmaps := []struct {
		name string
		f    func(rune) rune
	}{
		{"ToUpper", unicode.ToUpper},
		{"ToLower", unicode.ToLower},
		{"ToTitle", unicode.ToTitle},
		{"SimpleFold", unicode.SimpleFold},
		{"To/-1", func(r rune) rune { return unicode.To(-1, r) }},
		{"To/0", func(r rune) rune { return unicode.To(0, r) }},
		{"To/1", func(r rune) rune { return unicode.To(1, r) }},
		{"To/2", func(r rune) rune { return unicode.To(2, r) }},
		{"To/3", func(r rune) rune { return unicode.To(3, r) }},
		{"TurkishCase.ToUpper", unicode.TurkishCase.ToUpper},
		{"TurkishCase.ToLower", unicode.TurkishCase.ToLower},
		{"TurkishCase.ToTitle", unicode.TurkishCase.ToTitle},
		{"AzeriCase.ToUpper", unicode.AzeriCase.ToUpper},
		{"utf8.RuneLen", func(r rune) rune { return rune(utf8.RuneLen(r)) }},
		{"utf16.RuneLen", func(r rune) rune { return rune(utf16.RuneLen(r)) }},
		{"utf16.EncodeRune.1", func(r rune) rune { a, _ := utf16.EncodeRune(r); return a }},
		{"utf16.EncodeRune.2", func(r rune) rune { _, b := utf16.EncodeRune(r); return b }},
	}
	for _, m := range rmaps {
		lines = append(lines, mapLine("map/"+m.name, m.f))
	}
	// Encoders: hash the produced bytes.
	{
		h := newFNV()
		n := 0
		forAllRunes(func(r rune) {
			b := utf8.AppendRune(nil, r)
			var p [4]byte
			k := utf8.EncodeRune(p[:], r)
			if !bytes.Equal(b, p[:k]) {
				panic("EncodeRune != AppendRune")
			}
			n += len(b)
			h.byte(byte(len(b)))
			h.bytes(b)
		})
		lines = append(lines, hashLine{"enc/utf8.AppendRune", n, h})
	}
	{
		h := newFNV()
		n := 0
		forAllRunes(func(r rune) {
			a := utf16.AppendRune(nil, r)
			e := utf16.Encode([]rune{r})
			if len(a) != len(e) || a[0] != e[0] {
				panic("utf16 Encode != AppendRune")
			}
			n += len(a)
			h.byte(byte(len(a)))
			for _, u := range a {
				h.byte(byte(u))
				h.byte(byte(u >> 8))
			}
		})
		lines = append(lines, hashLine{"enc/utf16.AppendRune", n, h})
	}
	{
		// string(rune) conversion.
		h := newFNV()
		n := 0
		forAllRunes(func(r rune) {
			s := string(r)
			n += len(s)
			h.byte(byte(len(s)))
			h.bytes([]byte(s))
		})
		lines = append(lines, hashLine{"enc/string(rune)", n, h})
	}
	{
		// utf16.DecodeRune over a window around the surrogate blocks.
		h := newFNV()
		n := 0
		var vals []rune
		for r := rune(0xD700); r < 0xE100; r += 3 {
			vals = append(vals, r)
		}
		vals = append(vals, -1, 0, 0x41, 0xD800, 0xDBFF, 0xDC00, 0xDFFF, 0xE000, 0x10000, 0x10FFFF, 0x110000)
		for _, a := range vals {
			for _, b := range vals {
				v := utf16.DecodeRune(a, b)
				if v != unicode.ReplacementChar {
					n++
				}
				h.i32(v)
			}
		}
		lines = append(lines, hashLine{"utf16.DecodeRune/window3", n, h})
	}
	return lines
}

// ---------------------------------------------------------------------------
// utf8 decoding, exhaustive over short byte sequences.

// decodeHash folds every utf8 decoding function over p into h.
func decodeHash(h *fnv64, p []byte) {
	r, size := utf8.DecodeRune(p)
	h.i32(r)
	h.byte(byte(size))
	r2, size2 := utf8.DecodeRuneInString(string(p))
	if r2 != r || size2 != size {
		panic("DecodeRuneInString mismatch")
	}
	r, size = utf8.DecodeLastRune(p)
	h.i32(r)
	h.byte(byte(size))
	h.bool(utf8.FullRune(p))
	h.bool(utf8.Valid(p))
	h.byte(byte(utf8.RuneCount(p)))
	// for range (runtime decoderune).
	for i, c := range string(p) {
		h.byte(byte(i))
		h.i32(c)
	}
	h.byte(0xAA)
}

// boundary values for the 4th byte class in the 4-byte sweep.
var byteClasses = []byte{0x00, 0x41, 0x7F, 0x80, 0x8F, 0x90, 0x9F, 0xA0, 0xBF, 0xC0, 0xC2, 0xDF, 0xE0, 0xED, 0xF0, 0xF4, 0xF5, 0xFF}

func utf8Lines() []hashLine {
	var lines []hashLine
	{
		h := newFNV()
		for a := range 256 {
			decodeHash(&h, []byte{byte(a)})
		}
		lines = append(lines, hashLine{"decode/len1", 256, h})
	}
	{
		h := newFNV()
		for a := range 256 {
			for b := range 256 {
				decodeHash(&h, []byte{byte(a), byte(b)})
			}
		}
		lines = append(lines, hashLine{"decode/len2", 65536, h})
	}
	{
		// len3: first byte >= 0x80 only (ASCII first bytes are covered by len2
		// and keep the Rust debug-mode test fast), all second and third bytes.
		h := newFNV()
		n := 0
		for a := 0x80; a < 256; a++ {
			for b := range 256 {
				for c := range 256 {
					decodeHash(&h, []byte{byte(a), byte(b), byte(c)})
					n++
				}
			}
		}
		lines = append(lines, hashLine{"decode/len3hi", n, h})
	}
	{
		h := newFNV()
		n := 0
		for a := 0xC0; a < 256; a++ {
			for b := range 256 {
				for _, c := range byteClasses {
					for _, d := range byteClasses {
						decodeHash(&h, []byte{byte(a), byte(b), c, d})
						n++
					}
				}
			}
		}
		lines = append(lines, hashLine{"decode/len4", n, h})
	}
	{
		h := newFNV()
		n := 0
		for a := 0xC0; a < 256; a++ {
			for _, b := range byteClasses {
				for _, c := range byteClasses {
					for _, d := range byteClasses {
						for _, e := range byteClasses {
							decodeHash(&h, []byte{byte(a), b, c, d, e})
							n++
						}
					}
				}
			}
		}
		lines = append(lines, hashLine{"decode/len5", n, h})
	}
	return lines
}

func writeHashLines(path string, lines []hashLine) error {
	var b bytes.Buffer
	for _, l := range lines {
		fmt.Fprintf(&b, "%s\t%d\t%016x\n", l.name, l.count, uint64(l.hash))
	}
	return os.WriteFile(path, b.Bytes(), 0o644)
}

// ---------------------------------------------------------------------------
// strings/bytes vectors on random input.

// pieces random strings are made of: ASCII, spaces, case-mapping edge cases,
// multi-byte runes, U+FFFD and invalid/truncated/overlong/surrogate bytes.
var pieces = []string{
	"a", "b", "z", "A", "B", "Z", "i", "I", "k", "K", "s", "S", "x", "y", "0", "7", "9", "_", "-",
	".", ",", "!", "'", "(", " ", " ", "\t", "\n", "\v", "\f", "\r", "  ", "\u0085", "\u00a0",
	"\u1680", "\u2000", "\u200a", "\u2028", "\u3000", "\u180e", "\u200b", "\u00e9", "\u00c9",
	"\u00df", "\u1e9e", "\u0130", "\u0131", "\u017f", "\u212a", "\u212b", "\u00c5", "\u00e5",
	"\u00b5", "\u039c", "\u03bc", "\u03a3", "\u03c3", "\u03c2", "\u01c5", "\u01c6", "\u01c4",
	"\u01c8", "\ufb05", "\ufb06", "\u1ffc", "\u1fb3", "\u03a9", "\u2126", "\u03c9", "\u0416",
	"\u0436", "\u0490", "\u0531", "\u0561", "\u10d0", "\u1c90", "\ua64b", "\u1c88", "\U00010400",
	"\U00010428", "\U0001e900", "\U0001e922", "\u65e5", "\u672c", "\u4e2d", "\u0e01", "\u0e44",
	"\u0e31", "\u0301", "\u0308", "\u20dd", "\u0661", "\u0663", "\U0001d7d8", "\u216b", "\u00bd",
	"\u00b2", "\u20ac", "$", "\u00a9", "\u2211", "\U0001f600", "\U0001f44d\U0001f3fd", "\U0010ffff",
	"\U000e0001", "\ufffd", "\ufeff", "\u00ad", "\x00", "\x1f", "\x7f", "\u0080", "\u009f", "\x80",
	"\xbf", "\xc0", "\xc1", "\xc2", "\xdf", "\xe0", "\xed", "\xef", "\xf0", "\xf4", "\xf5", "\xff",
	"\xc0\xaf", "\xe0\x80\xaf", "\xed\xa0\x80", "\xed\xbf\xbf", "\xf4\x90\x80\x80",
	"\xf0\x80\x80\xaf", "\xe2\x82", "\xf0\x9f\x98", "\xf0\x9f", "\xe2", "\xcf",
}

type strGen struct {
	r *rand.Rand
	// rich selects richStr (the -rich fuzz corpus) instead of the default
	// piece-based generator.
	rich bool
}

func (g *strGen) str(maxPieces int) string {
	if g.rich {
		return g.richStr(maxPieces)
	}
	n := g.r.IntN(maxPieces + 1)
	var b strings.Builder
	for range n {
		// Bias towards ASCII so ASCII fast paths are exercised too.
		switch g.r.IntN(4) {
		case 0:
			b.WriteString(pieces[g.r.IntN(24)])
		default:
			b.WriteString(pieces[g.r.IntN(len(pieces))])
		}
	}
	return b.String()
}

// asciiStr makes pure-ASCII strings (the ASCII fast paths).
func (g *strGen) asciiStr(maxLen int) string {
	n := g.r.IntN(maxLen + 1)
	b := make([]byte, n)
	for i := range b {
		switch g.r.IntN(5) {
		case 0:
			b[i] = " \t\n\v\f\r"[g.r.IntN(6)]
		default:
			b[i] = byte(0x20 + g.r.IntN(0x5f))
		}
	}
	return string(b)
}

// sub returns a random substring of s (possibly cutting runes in half).
func (g *strGen) sub(s string, maxLen int) string {
	if len(s) == 0 {
		return ""
	}
	a := g.r.IntN(len(s))
	l := g.r.IntN(min(maxLen, len(s)-a) + 1)
	return s[a : a+l]
}

type vecWriter struct {
	w   *bufio.Writer
	n   int
	err error
}

func (v *vecWriter) bytes(b []byte) {
	var l [4]byte
	binary.LittleEndian.PutUint32(l[:], uint32(len(b)))
	v.w.Write(l[:])
	v.w.Write(b)
}

// rec writes one record: name, args, result.
func (v *vecWriter) rec(name string, args []string, result []byte) {
	v.bytes([]byte(name))
	v.w.WriteByte(byte(len(args)))
	for _, a := range args {
		v.bytes([]byte(a))
	}
	v.bytes(result)
	v.n++
}

func encInt(i int) []byte { return []byte(strconv.Itoa(i)) }

func encBool(b bool) []byte {
	if b {
		return []byte("true")
	}
	return []byte("false")
}

func encList[T string | []byte](l []T) []byte {
	var b bytes.Buffer
	var n [4]byte
	binary.LittleEndian.PutUint32(n[:], uint32(len(l)))
	b.Write(n[:])
	for _, e := range l {
		binary.LittleEndian.PutUint32(n[:], uint32(len(e)))
		b.Write(n[:])
		b.WriteString(string(e))
	}
	return b.Bytes()
}

func encRunes(rs []rune) []byte {
	var b bytes.Buffer
	for _, r := range rs {
		var n [4]byte
		binary.LittleEndian.PutUint32(n[:], uint32(r))
		b.Write(n[:])
	}
	return b.Bytes()
}

// Named rune functions shared with the Rust test.
var predFuncs = map[string]func(rune) bool{
	"IsSpace":  unicode.IsSpace,
	"IsPunct":  unicode.IsPunct,
	"IsLetter": unicode.IsLetter,
	"IsDigit":  unicode.IsDigit,
	"IsUpper":  unicode.IsUpper,
	"IsError":  func(r rune) bool { return r == utf8.RuneError },
}

var predNames = []string{"IsSpace", "IsPunct", "IsLetter", "IsDigit", "IsUpper", "IsError"}

var mapFuncs = map[string]func(rune) rune{
	"identity": func(r rune) rune { return r },
	"weird": func(r rune) rune {
		switch {
		case '0' <= r && r <= '9':
			return -1
		case r == 'a':
			return 0xE4
		case r == utf8.RuneError:
			return '?'
		case r == 'x':
			return 0xD800
		case r == 'y':
			return 0x110000
		case r == 'b':
			return utf8.RuneError
		case r == ' ':
			return -5
		}
		return unicode.ToUpper(r)
	},
	"toError": func(r rune) rune {
		if r >= 0x80 {
			return utf8.RuneError
		}
		return r
	},
}

var mapNames = []string{"identity", "weird", "toError"}

func stringsVectors(path string, nRandom int, seed uint64, rich bool, corpus []string) (int, error) {
	f, err := os.Create(path)
	if err != nil {
		return 0, err
	}
	defer f.Close()
	gz, err := gzip.NewWriterLevel(f, gzip.BestCompression)
	if err != nil {
		return 0, err
	}
	bw := bufio.NewWriter(gz)
	v := &vecWriter{w: bw}

	g := &strGen{r: rand.New(rand.NewPCG(seed, seed^0x9e3779b97f4a7c15)), rich: rich}

	inputs := []string{"", " ", "\t \n", "a", "A", "\x80", "\xff", "\xe2\x82", "\ufffd", "\u0130", "\u01c5\u01c6\u01c4", "Hello, World", "  x  ", "\u00a0x\u00a0", "\u0085", "a\xffb", "\xed\xa0\x80", "\u00ff", "\u212a", "\ufb05\ufb06", "\u0130stanbul \u0131\u0049"}
	for range nRandom {
		switch g.r.IntN(5) {
		case 0:
			inputs = append(inputs, g.asciiStr(24))
		default:
			inputs = append(inputs, g.str(14))
		}
	}
	// Real text (-corpus), each line also with a random mutation.
	for _, c := range corpus {
		inputs = append(inputs, c, g.mutate(c))
	}

	for idx, s := range inputs {
		// A second operand: substring of s, a random string, or ASCII.
		var t string
		switch g.r.IntN(3) {
		case 0:
			t = g.sub(s, 6)
		case 1:
			t = g.str(2)
		default:
			t = g.asciiStr(3)
		}
		u := g.str(2)
		n := g.r.IntN(5) - 1
		pred := predNames[idx%len(predNames)]
		pf := predFuncs[pred]
		mname := mapNames[idx%len(mapNames)]
		mf := mapFuncs[mname]
		r := []rune(g.str(1) + "a")[0]
		switch g.r.IntN(6) {
		case 0:
			r = utf8.RuneError
		case 1:
			r = []rune{-1, 0xD800, 0x110000, 0x10FFFF, 0}[g.r.IntN(5)]
		}
		c := byte(g.r.IntN(256))
		if g.r.IntN(2) == 0 && len(s) > 0 {
			c = s[g.r.IntN(len(s))]
		}
		cs := string([]byte{c})
		ns := strconv.Itoa(n)
		rs := strconv.Itoa(int(r))

		// utf8
		v.rec("utf8.RuneCount", []string{s}, encInt(utf8.RuneCount([]byte(s))))
		v.rec("utf8.RuneCountInString", []string{s}, encInt(utf8.RuneCountInString(s)))
		v.rec("utf8.Valid", []string{s}, encBool(utf8.Valid([]byte(s))))
		v.rec("utf8.ValidString", []string{s}, encBool(utf8.ValidString(s)))
		v.rec("[]rune", []string{s}, encRunes([]rune(s)))
		v.rec("string([]rune)", []string{s, rs}, []byte(string([]rune(s+string(r)))))
		v.rec("string(runes)", []string{s, rs}, []byte(string([]rune{r, 'x', r})))
		{
			var b []byte
			for i, c := range s {
				b = append(b, encInt(i)...)
				b = append(b, ':')
				b = append(b, encInt(int(c))...)
				b = append(b, ' ')
			}
			v.rec("range", []string{s}, b)
		}
		{
			// utf16 round trip.
			u16 := utf16.Encode([]rune(s))
			var b []byte
			for _, x := range u16 {
				b = append(b, byte(x), byte(x>>8))
			}
			v.rec("utf16.Encode", []string{s}, b)
			if len(u16) > 1 && g.r.IntN(2) == 0 {
				u16 = u16[1:] // leading lone low surrogate sometimes
			}
			if len(u16) > 2 && g.r.IntN(2) == 0 {
				u16 = u16[:len(u16)-1]
			}
			var in []byte
			for _, x := range u16 {
				in = append(in, byte(x), byte(x>>8))
			}
			v.rec("utf16.Decode", []string{string(in)}, encRunes(utf16.Decode(u16)))
		}

		// strings
		v.rec("strings.Count", []string{s, t}, encInt(strings.Count(s, t)))
		v.rec("strings.Contains", []string{s, t}, encBool(strings.Contains(s, t)))
		v.rec("strings.ContainsAny", []string{s, t}, encBool(strings.ContainsAny(s, t)))
		v.rec("strings.ContainsRune", []string{s, rs}, encBool(strings.ContainsRune(s, r)))
		v.rec("strings.ContainsFunc", []string{s, pred}, encBool(strings.ContainsFunc(s, pf)))
		v.rec("strings.Index", []string{s, t}, encInt(strings.Index(s, t)))
		v.rec("strings.LastIndex", []string{s, t}, encInt(strings.LastIndex(s, t)))
		v.rec("strings.IndexByte", []string{s, cs}, encInt(strings.IndexByte(s, c)))
		v.rec("strings.LastIndexByte", []string{s, cs}, encInt(strings.LastIndexByte(s, c)))
		v.rec("strings.IndexRune", []string{s, rs}, encInt(strings.IndexRune(s, r)))
		v.rec("strings.IndexAny", []string{s, t}, encInt(strings.IndexAny(s, t)))
		v.rec("strings.LastIndexAny", []string{s, t}, encInt(strings.LastIndexAny(s, t)))
		v.rec("strings.SplitN", []string{s, t, ns}, encList(strings.SplitN(s, t, n)))
		v.rec("strings.SplitAfterN", []string{s, t, ns}, encList(strings.SplitAfterN(s, t, n)))
		v.rec("strings.Split", []string{s, t}, encList(strings.Split(s, t)))
		v.rec("strings.SplitAfter", []string{s, t}, encList(strings.SplitAfter(s, t)))
		v.rec("strings.Fields", []string{s}, encList(strings.Fields(s)))
		v.rec("strings.FieldsFunc", []string{s, pred}, encList(strings.FieldsFunc(s, pf)))
		v.rec("strings.Join", []string{s, t, u}, []byte(strings.Join(strings.Split(s, u), t)))
		v.rec("strings.HasPrefix", []string{s, t}, encBool(strings.HasPrefix(s, t)))
		v.rec("strings.HasSuffix", []string{s, t}, encBool(strings.HasSuffix(s, t)))
		v.rec("strings.Map", []string{s, mname}, []byte(strings.Map(mf, s)))
		if n >= 0 {
			v.rec("strings.Repeat", []string{s, ns}, []byte(strings.Repeat(s, n)))
		}
		v.rec("strings.ToUpper", []string{s}, []byte(strings.ToUpper(s)))
		v.rec("strings.ToLower", []string{s}, []byte(strings.ToLower(s)))
		v.rec("strings.ToTitle", []string{s}, []byte(strings.ToTitle(s)))
		v.rec("strings.ToUpperSpecial", []string{s}, []byte(strings.ToUpperSpecial(unicode.TurkishCase, s)))
		v.rec("strings.ToLowerSpecial", []string{s}, []byte(strings.ToLowerSpecial(unicode.TurkishCase, s)))
		v.rec("strings.ToTitleSpecial", []string{s}, []byte(strings.ToTitleSpecial(unicode.TurkishCase, s)))
		v.rec("strings.ToValidUTF8", []string{s, t}, []byte(strings.ToValidUTF8(s, t)))
		v.rec("strings.Title", []string{s}, []byte(strings.Title(s)))
		v.rec("strings.TrimLeftFunc", []string{s, pred}, []byte(strings.TrimLeftFunc(s, pf)))
		v.rec("strings.TrimRightFunc", []string{s, pred}, []byte(strings.TrimRightFunc(s, pf)))
		v.rec("strings.TrimFunc", []string{s, pred}, []byte(strings.TrimFunc(s, pf)))
		v.rec("strings.IndexFunc", []string{s, pred}, encInt(strings.IndexFunc(s, pf)))
		v.rec("strings.LastIndexFunc", []string{s, pred}, encInt(strings.LastIndexFunc(s, pf)))
		v.rec("strings.Trim", []string{s, t}, []byte(strings.Trim(s, t)))
		v.rec("strings.TrimLeft", []string{s, t}, []byte(strings.TrimLeft(s, t)))
		v.rec("strings.TrimRight", []string{s, t}, []byte(strings.TrimRight(s, t)))
		v.rec("strings.TrimSpace", []string{s}, []byte(strings.TrimSpace(s)))
		v.rec("strings.TrimPrefix", []string{s, t}, []byte(strings.TrimPrefix(s, t)))
		v.rec("strings.TrimSuffix", []string{s, t}, []byte(strings.TrimSuffix(s, t)))
		v.rec("strings.Replace", []string{s, t, u, ns}, []byte(strings.Replace(s, t, u, n)))
		v.rec("strings.ReplaceAll", []string{s, t, u}, []byte(strings.ReplaceAll(s, t, u)))
		v.rec("strings.EqualFold", []string{s, t}, encBool(strings.EqualFold(s, t)))
		v.rec("strings.EqualFold/upper", []string{s}, encBool(strings.EqualFold(s, strings.ToUpper(s))))
		v.rec("strings.EqualFold/lower", []string{s}, encBool(strings.EqualFold(strings.ToLower(s), s)))
		{
			b, a, found := strings.Cut(s, t)
			v.rec("strings.Cut", []string{s, t}, encList([]string{b, a, string(encBool(found))}))
			b, a, found = strings.CutLast(s, t)
			v.rec("strings.CutLast", []string{s, t}, encList([]string{b, a, string(encBool(found))}))
			a, found = strings.CutPrefix(s, t)
			v.rec("strings.CutPrefix", []string{s, t}, encList([]string{a, string(encBool(found))}))
			b, found = strings.CutSuffix(s, t)
			v.rec("strings.CutSuffix", []string{s, t}, encList([]string{b, string(encBool(found))}))
		}
		v.rec("strings.Compare", []string{s, t}, encInt(strings.Compare(s, t)))

		// bytes
		bs, bt, bu := []byte(s), []byte(t), []byte(u)
		v.rec("bytes.Count", []string{s, t}, encInt(bytes.Count(bs, bt)))
		v.rec("bytes.Contains", []string{s, t}, encBool(bytes.Contains(bs, bt)))
		v.rec("bytes.ContainsAny", []string{s, t}, encBool(bytes.ContainsAny(bs, t)))
		v.rec("bytes.ContainsRune", []string{s, rs}, encBool(bytes.ContainsRune(bs, r)))
		v.rec("bytes.ContainsFunc", []string{s, pred}, encBool(bytes.ContainsFunc(bs, pf)))
		v.rec("bytes.Index", []string{s, t}, encInt(bytes.Index(bs, bt)))
		v.rec("bytes.LastIndex", []string{s, t}, encInt(bytes.LastIndex(bs, bt)))
		v.rec("bytes.IndexByte", []string{s, cs}, encInt(bytes.IndexByte(bs, c)))
		v.rec("bytes.LastIndexByte", []string{s, cs}, encInt(bytes.LastIndexByte(bs, c)))
		v.rec("bytes.IndexRune", []string{s, rs}, encInt(bytes.IndexRune(bs, r)))
		v.rec("bytes.IndexAny", []string{s, t}, encInt(bytes.IndexAny(bs, t)))
		v.rec("bytes.LastIndexAny", []string{s, t}, encInt(bytes.LastIndexAny(bs, t)))
		v.rec("bytes.SplitN", []string{s, t, ns}, encList(bytes.SplitN(bs, bt, n)))
		v.rec("bytes.SplitAfterN", []string{s, t, ns}, encList(bytes.SplitAfterN(bs, bt, n)))
		v.rec("bytes.Split", []string{s, t}, encList(bytes.Split(bs, bt)))
		v.rec("bytes.SplitAfter", []string{s, t}, encList(bytes.SplitAfter(bs, bt)))
		v.rec("bytes.Fields", []string{s}, encList(bytes.Fields(bs)))
		v.rec("bytes.FieldsFunc", []string{s, pred}, encList(bytes.FieldsFunc(bs, pf)))
		v.rec("bytes.Join", []string{s, t, u}, bytes.Join(bytes.Split(bs, bu), bt))
		v.rec("bytes.Map", []string{s, mname}, bytes.Map(mf, bs))
		if n >= 0 {
			v.rec("bytes.Repeat", []string{s, ns}, bytes.Repeat(bs, n))
		}
		v.rec("bytes.ToUpper", []string{s}, bytes.ToUpper(bs))
		v.rec("bytes.ToLower", []string{s}, bytes.ToLower(bs))
		v.rec("bytes.ToTitle", []string{s}, bytes.ToTitle(bs))
		v.rec("bytes.ToUpperSpecial", []string{s}, bytes.ToUpperSpecial(unicode.TurkishCase, bs))
		v.rec("bytes.ToLowerSpecial", []string{s}, bytes.ToLowerSpecial(unicode.TurkishCase, bs))
		v.rec("bytes.ToTitleSpecial", []string{s}, bytes.ToTitleSpecial(unicode.TurkishCase, bs))
		v.rec("bytes.ToValidUTF8", []string{s, t}, bytes.ToValidUTF8(bs, bt))
		v.rec("bytes.Title", []string{s}, bytes.Title(bs))
		v.rec("bytes.TrimLeftFunc", []string{s, pred}, bytes.TrimLeftFunc(bs, pf))
		v.rec("bytes.TrimRightFunc", []string{s, pred}, bytes.TrimRightFunc(bs, pf))
		v.rec("bytes.TrimFunc", []string{s, pred}, bytes.TrimFunc(bs, pf))
		v.rec("bytes.IndexFunc", []string{s, pred}, encInt(bytes.IndexFunc(bs, pf)))
		v.rec("bytes.LastIndexFunc", []string{s, pred}, encInt(bytes.LastIndexFunc(bs, pf)))
		v.rec("bytes.Trim", []string{s, t}, bytes.Trim(bs, t))
		v.rec("bytes.TrimLeft", []string{s, t}, bytes.TrimLeft(bs, t))
		v.rec("bytes.TrimRight", []string{s, t}, bytes.TrimRight(bs, t))
		v.rec("bytes.TrimSpace", []string{s}, bytes.TrimSpace(bs))
		v.rec("bytes.TrimPrefix", []string{s, t}, bytes.TrimPrefix(bs, bt))
		v.rec("bytes.TrimSuffix", []string{s, t}, bytes.TrimSuffix(bs, bt))
		v.rec("bytes.Runes", []string{s}, encRunes(bytes.Runes(bs)))
		v.rec("bytes.Replace", []string{s, t, u, ns}, bytes.Replace(bs, bt, bu, n))
		v.rec("bytes.ReplaceAll", []string{s, t, u}, bytes.ReplaceAll(bs, bt, bu))
		v.rec("bytes.EqualFold", []string{s, t}, encBool(bytes.EqualFold(bs, bt)))
		v.rec("bytes.EqualFold/upper", []string{s}, encBool(bytes.EqualFold(bs, bytes.ToUpper(bs))))
		v.rec("bytes.Equal", []string{s, t}, encBool(bytes.Equal(bs, bt)))
		v.rec("bytes.Compare", []string{s, t}, encInt(bytes.Compare(bs, bt)))
		{
			b, a, found := bytes.Cut(bs, bt)
			v.rec("bytes.Cut", []string{s, t}, encList([][]byte{b, a, encBool(found)}))
			b, a, found = bytes.CutLast(bs, bt)
			v.rec("bytes.CutLast", []string{s, t}, encList([][]byte{b, a, encBool(found)}))
			a, found = bytes.CutPrefix(bs, bt)
			v.rec("bytes.CutPrefix", []string{s, t}, encList([][]byte{a, encBool(found)}))
			b, found = bytes.CutSuffix(bs, bt)
			v.rec("bytes.CutSuffix", []string{s, t}, encList([][]byte{b, encBool(found)}))
		}

		// strings.Replacer with pairs taken from the inputs.
		{
			var old []string
			np := g.r.IntN(4) + 1
			for range np {
				switch g.r.IntN(4) {
				case 0:
					old = append(old, g.sub(s, 3), g.str(1))
				case 1:
					old = append(old, g.asciiStr(1), g.asciiStr(2))
				case 2:
					old = append(old, g.sub(s, 1), g.sub(s, 2))
				default:
					old = append(old, g.str(1), g.str(2))
				}
			}
			args := append([]string{s}, old...)
			v.rec("strings.NewReplacer", args, []byte(strings.NewReplacer(old...).Replace(s)))
			// Single byte replacer / byte string replacer shapes.
			var bo []string
			for range g.r.IntN(4) + 1 {
				bo = append(bo, g.asciiStr(1)+"x"[:0], g.asciiStr(2))
			}
			var bo1 []string
			for i := 0; i+1 < len(bo); i += 2 {
				if len(bo[i]) == 1 {
					bo1 = append(bo1, bo[i], bo[i+1])
				}
			}
			if len(bo1) > 0 {
				args = append([]string{s}, bo1...)
				v.rec("strings.NewReplacer", args, []byte(strings.NewReplacer(bo1...).Replace(s)))
				var bo2 []string
				for i := 0; i+1 < len(bo1); i += 2 {
					nv := bo1[i+1]
					if len(nv) > 1 {
						nv = nv[:1]
					}
					bo2 = append(bo2, bo1[i], nv)
				}
				args = append([]string{s}, bo2...)
				v.rec("strings.NewReplacer", args, []byte(strings.NewReplacer(bo2...).Replace(s)))
			}
		}
	}
	if err := bw.Flush(); err != nil {
		return 0, err
	}
	if err := gz.Close(); err != nil {
		return 0, err
	}
	return v.n, nil
}

func genFixtures(args []string) error {
	fs := flag.NewFlagSet("fixtures", flag.ExitOnError)
	dir := fs.String("dir", "crates/go-unicode/tests/fixtures", "output directory")
	nRandom := fs.Int("n", 1500, "number of random strings for strings/bytes vectors")
	seed := fs.Uint64("seed", 20260927, "PRNG seed")
	vecName := fs.String("vec", "strings_vectors.bin.gz", "strings/bytes vector file name")
	only := fs.String("only", "", "only generate: unicode, utf8, vectors")
	rich := fs.Bool("rich", false, "use the rich fuzz generator (random runes/bytes, fold orbits, truncated encodings) for the vectors")
	corpusDir := fs.String("corpus", "", "directory of real text files whose lines are added to the vector inputs")
	maxCorpus := fs.Int("maxcorpus", 20000, "maximum number of corpus lines")
	if err := fs.Parse(args); err != nil {
		return err
	}
	if err := os.MkdirAll(*dir, 0o755); err != nil {
		return err
	}
	if *only == "" || *only == "unicode" {
		lines := unicodeLines()
		// CategoryAliases and Version, as extra lines.
		if err := writeHashLines(filepath.Join(*dir, "unicode_hashes.txt"), lines); err != nil {
			return err
		}
		var b bytes.Buffer
		fmt.Fprintf(&b, "Version\t%s\n", unicode.Version)
		keys := make([]string, 0, len(unicode.CategoryAliases))
		for k := range unicode.CategoryAliases {
			keys = append(keys, k)
		}
		sort.Strings(keys)
		for _, k := range keys {
			fmt.Fprintf(&b, "CategoryAliases\t%s\t%s\n", k, unicode.CategoryAliases[k])
		}
		for _, m := range []struct {
			name string
			m    map[string]*unicode.RangeTable
		}{{"Categories", unicode.Categories}, {"Scripts", unicode.Scripts}, {"Properties", unicode.Properties}, {"FoldCategory", unicode.FoldCategory}, {"FoldScript", unicode.FoldScript}} {
			fmt.Fprintf(&b, "%s\t%d\n", m.name, len(m.m))
		}
		fmt.Fprintf(&b, "exported\t%d\n", len(exportedTables))
		fmt.Fprintf(&b, "CaseRanges\t%d\n", len(unicode.CaseRanges))
		if err := os.WriteFile(filepath.Join(*dir, "unicode_meta.txt"), b.Bytes(), 0o644); err != nil {
			return err
		}
	}
	if *only == "" || *only == "utf8" {
		if err := writeHashLines(filepath.Join(*dir, "utf8_hashes.txt"), utf8Lines()); err != nil {
			return err
		}
	}
	if *only == "" || *only == "vectors" {
		var corpus []string
		if *corpusDir != "" {
			var err error
			if corpus, err = loadCorpus(*corpusDir, *maxCorpus); err != nil {
				return err
			}
		}
		n, err := stringsVectors(filepath.Join(*dir, *vecName), *nRandom, *seed, *rich, corpus)
		if err != nil {
			return err
		}
		fmt.Fprintf(os.Stderr, "%s: %d records\n", *vecName, n)
	}
	return nil
}
