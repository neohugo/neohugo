//go:build go1.27

package main

import (
	"bytes"
	"flag"
	"fmt"
	"os"
	"strings"
	"unicode"
	"unicode/utf8"
)

// The "adversarial" mode writes tests/fixtures/adversarial_hashes.txt: hash
// lines (same format as unicode_hashes.txt) for families the "fixtures" mode
// does not cover exhaustively:
//
//   - struct/...: the exact structure (R16, R32, LatinOffset) of every
//     RangeTable, CaseRanges, TurkishCase, GraphicRanges and PrintRanges, so
//     the generated Rust tables are checked field by field, not only through
//     membership.
//   - orbit/...: every SimpleFold orbit, walked from every code point.
//   - <domain>/<func>: ~55 strings/bytes functions (case mapping, Title,
//     EqualFold, Fields, TrimSpace, ToValidUTF8, Map, Trim*, Index*, Split)
//     applied to every string of a domain: every code point (string(r)),
//     code points in ASCII/space/invalid-UTF-8 contexts, and every short byte
//     string (all of length <= 2, byte classes for lengths 3 and 4).
//   - foldpair/...: EqualFold on every code point against its whole fold
//     orbit, its case mappings and near neighbours, and on all pairs of a set
//     of short (valid and invalid) strings.
//
// The Rust test tests/adversarial.rs recomputes every line.

// blob hashes a length-prefixed byte string.
func (h *fnv64) blob(p []byte) {
	h.i32(int32(len(p)))
	h.bytes(p)
}

// ---------------------------------------------------------------------------
// struct/...

func structTable(h *fnv64, t *unicode.RangeTable) {
	h.i32(int32(len(t.R16)))
	for _, r := range t.R16 {
		h.i32(int32(r.Lo))
		h.i32(int32(r.Hi))
		h.i32(int32(r.Stride))
	}
	h.i32(int32(len(t.R32)))
	for _, r := range t.R32 {
		h.i32(int32(r.Lo))
		h.i32(int32(r.Hi))
		h.i32(int32(r.Stride))
	}
	h.i32(int32(t.LatinOffset))
}

func structCaseRanges(h *fnv64, crs []unicode.CaseRange) {
	h.i32(int32(len(crs)))
	for _, cr := range crs {
		h.i32(int32(cr.Lo))
		h.i32(int32(cr.Hi))
		for _, d := range cr.Delta {
			h.i32(d)
		}
	}
}

func structLines() []hashLine {
	var lines []hashLine
	one := func(name string, t *unicode.RangeTable) {
		h := newFNV()
		structTable(&h, t)
		lines = append(lines, hashLine{"struct/" + name, len(t.R16) + len(t.R32), h})
	}
	for _, m := range []struct {
		name string
		m    map[string]*unicode.RangeTable
	}{
		{"Categories", unicode.Categories},
		{"Scripts", unicode.Scripts},
		{"Properties", unicode.Properties},
		{"FoldCategory", unicode.FoldCategory},
		{"FoldScript", unicode.FoldScript},
	} {
		for _, k := range sortedKeys(m.m) {
			one(m.name+"/"+k, m.m[k])
		}
	}
	for _, k := range sortedKeys(exportedTables) {
		one("exported/"+k, exportedTables[k])
	}
	for _, l := range []struct {
		name string
		l    []*unicode.RangeTable
	}{{"GraphicRanges", unicode.GraphicRanges}, {"PrintRanges", unicode.PrintRanges}} {
		h := newFNV()
		for _, t := range l.l {
			structTable(&h, t)
		}
		lines = append(lines, hashLine{"struct/" + l.name, len(l.l), h})
	}
	{
		h := newFNV()
		structCaseRanges(&h, unicode.CaseRanges)
		lines = append(lines, hashLine{"struct/CaseRanges", len(unicode.CaseRanges), h})
	}
	{
		h := newFNV()
		structCaseRanges(&h, unicode.TurkishCase)
		lines = append(lines, hashLine{"struct/TurkishCase", len(unicode.TurkishCase), h})
	}
	{
		h := newFNV()
		structCaseRanges(&h, unicode.AzeriCase)
		lines = append(lines, hashLine{"struct/AzeriCase", len(unicode.AzeriCase), h})
	}
	return lines
}

// ---------------------------------------------------------------------------
// orbit/...

// orbit returns the SimpleFold orbit of r starting after r (at most 16 steps).
func orbit(r rune) []rune {
	var o []rune
	x := unicode.SimpleFold(r)
	for x != r && len(o) < 16 {
		o = append(o, x)
		x = unicode.SimpleFold(x)
	}
	return o
}

func orbitLines() []hashLine {
	h := newFNV()
	n := 0
	forAllRunes(func(r rune) {
		o := orbit(r)
		n += len(o)
		h.i32(int32(len(o)))
		for _, x := range o {
			h.i32(x)
		}
	})
	return []hashLine{{"orbit/all", n, h}}
}

// ---------------------------------------------------------------------------
// String domains.

// advBytes3 / advBytes4 are the byte classes the length-3 and length-4 byte
// strings are built from: ASCII (spaces, case pairs, separators), continuation
// bytes at the accept-range boundaries, and every kind of lead byte.
var advBytes3 = []byte{
	0x00, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x20, 0x30, 0x41, 0x49, 0x4B, 0x53, 0x5A, 0x5F, 0x61, 0x69,
	0x6B, 0x73, 0x7A, 0x7F, 0x80, 0x84, 0x85, 0x89, 0x8F, 0x90, 0x9E, 0x9F, 0xA0, 0xAA, 0xB0, 0xBF,
	0xC0, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC7, 0xCE, 0xCF, 0xD0, 0xDF, 0xE0, 0xE1, 0xE2, 0xE3, 0xED,
	0xEF, 0xF0, 0xF4, 0xF5, 0xFF,
}

var advBytes4 = []byte{
	0x0A, 0x20, 0x41, 0x61, 0x80, 0x85, 0x9F, 0xA0, 0xBF, 0xC0, 0xC2, 0xC4, 0xCE, 0xE0, 0xE1, 0xE2,
	0xED, 0xF0, 0xF4, 0xFF,
}

// ctxRune reports whether r is in the context domains: everything below
// U+20000 (all case mappings live there), every 97th code point above, and
// the invalid runes.
func ctxRune(r rune) bool {
	return r < 0 || r > unicode.MaxRune || r < 0x20000 || r%97 == 0
}

// forDomain calls f for every string of the named domain.
func forDomain(name string, f func(s string)) {
	switch name {
	case "rune":
		forAllRunes(func(r rune) { f(string(r)) })
	case "ctx1":
		forAllRunes(func(r rune) {
			if ctxRune(r) {
				s := string(r)
				f(" " + s + "a" + s + " ")
			}
		})
	case "ctx2":
		forAllRunes(func(r rune) {
			if ctxRune(r) {
				s := string(r)
				f(" " + s + "\xff" + s + "_" + s + "　")
			}
		})
	case "ctx3":
		// Truncated encodings of r next to letters.
		forAllRunes(func(r rune) {
			if ctxRune(r) {
				s := string(r)
				f("A" + s[:len(s)-1] + "b" + s[1:] + "Z" + s)
			}
		})
	case "bytes":
		f("")
		for a := range 256 {
			f(string([]byte{byte(a)}))
		}
		for a := range 256 {
			for b := range 256 {
				f(string([]byte{byte(a), byte(b)}))
			}
		}
		for _, a := range advBytes3 {
			for _, b := range advBytes3 {
				for _, c := range advBytes3 {
					f(string([]byte{a, b, c}))
				}
			}
		}
		for _, a := range advBytes4 {
			for _, b := range advBytes4 {
				for _, c := range advBytes4 {
					for _, d := range advBytes4 {
						f(string([]byte{a, b, c, d}))
					}
				}
			}
		}
	default:
		panic("unknown domain " + name)
	}
}

var advDomains = []string{"rune", "ctx1", "ctx2", "ctx3", "bytes"}

// mapFold is the Map function of the MapFold entries: drops spaces, grows
// RuneError to a 4-byte rune, folds everything else.
func mapFold(r rune) rune {
	if unicode.IsSpace(r) {
		return -1
	}
	if r == utf8.RuneError {
		return unicode.MaxRune
	}
	return unicode.SimpleFold(r)
}

const (
	advAny      = " \xffAk"
	advTrimL    = "  \xffA"
	advTrimR    = " \xff z"
	advTrimBoth = "\xff\x80　"
)

type strFunc struct {
	name string
	f    func(s string) []byte
}

var advFuncs = []strFunc{
	{"ToUpper", func(s string) []byte { return []byte(strings.ToUpper(s)) }},
	{"ToLower", func(s string) []byte { return []byte(strings.ToLower(s)) }},
	{"ToTitle", func(s string) []byte { return []byte(strings.ToTitle(s)) }},
	{"Title", func(s string) []byte { return []byte(strings.Title(s)) }},
	{"ToUpperSpecialTR", func(s string) []byte { return []byte(strings.ToUpperSpecial(unicode.TurkishCase, s)) }},
	{"ToLowerSpecialTR", func(s string) []byte { return []byte(strings.ToLowerSpecial(unicode.TurkishCase, s)) }},
	{"ToTitleSpecialTR", func(s string) []byte { return []byte(strings.ToTitleSpecial(unicode.TurkishCase, s)) }},
	{"TrimSpace", func(s string) []byte { return []byte(strings.TrimSpace(s)) }},
	{"Fields", func(s string) []byte { return encList(strings.Fields(s)) }},
	{"FieldsFuncPunct", func(s string) []byte { return encList(strings.FieldsFunc(s, unicode.IsPunct)) }},
	{"EqualFoldUpper", func(s string) []byte { return encBool(strings.EqualFold(s, strings.ToUpper(s))) }},
	{"EqualFoldLower", func(s string) []byte { return encBool(strings.EqualFold(strings.ToLower(s), s)) }},
	{"EqualFoldTitle", func(s string) []byte { return encBool(strings.EqualFold(s, strings.ToTitle(s))) }},
	{"EqualFoldSimpleFold", func(s string) []byte {
		return encBool(strings.EqualFold(s, strings.Map(unicode.SimpleFold, s)))
	}},
	{"EqualFoldTurkish", func(s string) []byte {
		return encBool(strings.EqualFold(strings.ToUpperSpecial(unicode.TurkishCase, s), s))
	}},
	{"ToValidUTF8", func(s string) []byte { return []byte(strings.ToValidUTF8(s, "�")) }},
	{"ToValidUTF8Empty", func(s string) []byte { return []byte(strings.ToValidUTF8(s, "")) }},
	{"MapIdentity", func(s string) []byte { return []byte(strings.Map(func(r rune) rune { return r }, s)) }},
	{"MapFold", func(s string) []byte { return []byte(strings.Map(mapFold, s)) }},
	{"TrimFuncLetter", func(s string) []byte { return []byte(strings.TrimFunc(s, unicode.IsLetter)) }},
	{"TrimLeftSet", func(s string) []byte { return []byte(strings.TrimLeft(s, advTrimL)) }},
	{"TrimRightSet", func(s string) []byte { return []byte(strings.TrimRight(s, advTrimR)) }},
	{"TrimSet", func(s string) []byte { return []byte(strings.Trim(s, advTrimBoth)) }},
	{"IndexRuneError", func(s string) []byte { return encInt(strings.IndexRune(s, utf8.RuneError)) }},
	{"IndexFuncUpper", func(s string) []byte { return encInt(strings.IndexFunc(s, unicode.IsUpper)) }},
	{"LastIndexFuncSpace", func(s string) []byte { return encInt(strings.LastIndexFunc(s, unicode.IsSpace)) }},
	{"SplitEmpty", func(s string) []byte { return encList(strings.Split(s, "")) }},
	{"SplitN2Empty", func(s string) []byte { return encList(strings.SplitN(s, "", 2)) }},
	{"IndexAny", func(s string) []byte { return encInt(strings.IndexAny(s, advAny)) }},
	{"LastIndexAny", func(s string) []byte { return encInt(strings.LastIndexAny(s, advAny)) }},
	{"RuneCount", func(s string) []byte { return encInt(utf8.RuneCountInString(s)) }},
	{"Runes", func(s string) []byte { return encRunes([]rune(s)) }},
	{"bytes.ToUpper", func(s string) []byte { return bytes.ToUpper([]byte(s)) }},
	{"bytes.ToLower", func(s string) []byte { return bytes.ToLower([]byte(s)) }},
	{"bytes.ToTitle", func(s string) []byte { return bytes.ToTitle([]byte(s)) }},
	{"bytes.Title", func(s string) []byte { return bytes.Title([]byte(s)) }},
	{"bytes.ToUpperSpecialTR", func(s string) []byte { return bytes.ToUpperSpecial(unicode.TurkishCase, []byte(s)) }},
	{"bytes.TrimSpace", func(s string) []byte { return bytes.TrimSpace([]byte(s)) }},
	{"bytes.Fields", func(s string) []byte { return encList(bytes.Fields([]byte(s))) }},
	{"bytes.EqualFoldUpper", func(s string) []byte {
		return encBool(bytes.EqualFold([]byte(s), bytes.ToUpper([]byte(s))))
	}},
	{"bytes.EqualFoldLower", func(s string) []byte {
		return encBool(bytes.EqualFold(bytes.ToLower([]byte(s)), []byte(s)))
	}},
	{"bytes.ToValidUTF8", func(s string) []byte { return bytes.ToValidUTF8([]byte(s), []byte("�")) }},
	{"bytes.MapIdentity", func(s string) []byte { return bytes.Map(func(r rune) rune { return r }, []byte(s)) }},
	{"bytes.MapFold", func(s string) []byte { return bytes.Map(mapFold, []byte(s)) }},
	{"bytes.Runes", func(s string) []byte { return encRunes(bytes.Runes([]byte(s))) }},
	{"bytes.IndexAny", func(s string) []byte { return encInt(bytes.IndexAny([]byte(s), advAny)) }},
	{"bytes.LastIndexAny", func(s string) []byte { return encInt(bytes.LastIndexAny([]byte(s), advAny)) }},
	{"bytes.TrimFuncLetter", func(s string) []byte { return bytes.TrimFunc([]byte(s), unicode.IsLetter) }},
	{"bytes.TrimSet", func(s string) []byte { return bytes.Trim([]byte(s), advTrimBoth) }},
	{"bytes.TrimRightSet", func(s string) []byte { return bytes.TrimRight([]byte(s), advTrimR) }},
	{"bytes.IndexRuneError", func(s string) []byte { return encInt(bytes.IndexRune([]byte(s), utf8.RuneError)) }},
	{"bytes.LastIndexFuncSpace", func(s string) []byte {
		return encInt(bytes.LastIndexFunc([]byte(s), unicode.IsSpace))
	}},
	{"bytes.SplitEmpty", func(s string) []byte { return encList(bytes.Split([]byte(s), nil)) }},
	{"bytes.SplitN2Empty", func(s string) []byte { return encList(bytes.SplitN([]byte(s), nil, 2)) }},
}

func strFamilyLines() []hashLine {
	var lines []hashLine
	for _, d := range advDomains {
		for _, fn := range advFuncs {
			h := newFNV()
			n := 0
			forDomain(d, func(s string) {
				out := fn.f(s)
				n += len(out)
				h.blob(out)
			})
			lines = append(lines, hashLine{d + "/" + fn.name, n, h})
		}
	}
	return lines
}

// ---------------------------------------------------------------------------
// foldpair/...

// foldPartners returns the runes EqualFold is checked against for r.
func foldPartners(r rune) []rune {
	p := orbit(r)
	p = append(p,
		unicode.ToUpper(r), unicode.ToLower(r), unicode.ToTitle(r),
		unicode.TurkishCase.ToUpper(r), unicode.TurkishCase.ToLower(r),
		r+1, r-1, r^0x20, r^1, 0x212A, 0x17F, utf8.RuneError, 'k', 's')
	return p
}

// foldStrings is the set of short strings whose pairs are all checked.
func foldStrings() []string {
	var ss []string
	for a := range 256 {
		ss = append(ss, string([]byte{byte(a)}))
	}
	runes := []rune{
		0xB5, 0xC5, 0xDF, 0xE5, 0xFF, 0x130, 0x131, 0x149, 0x17F, 0x178, 0x1C4, 0x1C5, 0x1C6, 0x1C7,
		0x1C8, 0x1C9, 0x1F0, 0x1F1, 0x1F2, 0x1F3, 0x345, 0x390, 0x392, 0x398, 0x399, 0x39A, 0x39C,
		0x3A0, 0x3A1, 0x3A3, 0x3A6, 0x3A9, 0x3B0, 0x3B2, 0x3B8, 0x3B9, 0x3BA, 0x3BC, 0x3C0, 0x3C1,
		0x3C2, 0x3C3, 0x3C6, 0x3C9, 0x3D0, 0x3D1, 0x3D5, 0x3D6, 0x3F0, 0x3F1, 0x3F4, 0x3F5, 0x412,
		0x432, 0x1C80, 0x1C88, 0x1E60, 0x1E61, 0x1E9B, 0x1E9E, 0x1FBE, 0x1FD3, 0x1FE3, 0x2126, 0x212A,
		0x212B, 0x2C2F, 0x2C5F, 0xA64A, 0xA64B, 0xA7AE, 0xAB53, 0xAB70, 0x13A0, 0xFB05, 0xFB06, 0xFF21,
		0xFF41, 0xFFFD, 0x10400, 0x10428, 0x1E900, 0x1E922, 0x10FFFF, 0x2000, 0x3000, 0x85, 0xA0,
	}
	for _, r := range runes {
		ss = append(ss, string(r))
	}
	ss = append(ss,
		"\xc0\xaf", "\xe0\x80\xaf", "\xed\xa0\x80", "\xf4\x90\x80\x80", "\xe2\x84", "\xf0\x9f\x98",
		"\xc4", "\xce", "\xe1", "\xef\xbf", "Aa", "aA", "kK", "Kk", "KK", "sſ",
		"ſS", "\xffa", "a\xff", "İi", "iı",
	)
	return ss
}

func foldPairLines() []hashLine {
	var lines []hashLine
	type variant struct {
		name string
		f    func(a, b string) bool
	}
	variants := []variant{
		{"ab", func(a, b string) bool { return strings.EqualFold(a, b) }},
		{"ba", func(a, b string) bool { return strings.EqualFold(b, a) }},
		{"bytes", func(a, b string) bool { return bytes.EqualFold([]byte(a), []byte(b)) }},
		{"ctx", func(a, b string) bool { return strings.EqualFold("Ab"+a+"\xff", "aB"+b+"\xff") }},
		{"long", func(a, b string) bool { return strings.EqualFold(a+"x", b) }},
	}
	for _, v := range variants {
		h := newFNV()
		n := 0
		forAllRunes(func(r rune) {
			a := string(r)
			for _, x := range foldPartners(r) {
				e := v.f(a, string(x))
				if e {
					n++
				}
				h.bool(e)
			}
		})
		lines = append(lines, hashLine{"foldpair/rune/" + v.name, n, h})
	}
	ss := foldStrings()
	for _, v := range variants {
		h := newFNV()
		n := 0
		for _, a := range ss {
			for _, b := range ss {
				e := v.f(a, b)
				if e {
					n++
				}
				h.bool(e)
			}
		}
		lines = append(lines, hashLine{"foldpair/set/" + v.name, n, h})
	}
	return lines
}

// ---------------------------------------------------------------------------
// long/...: long haystacks built from small alphabets (many partial matches),
// which drive Go's IndexByte/IndexRune cutovers, Rabin-Karp, Boyer-Moore and
// the Replacer algorithms. The PRNG is a xorshift64* mirrored in Rust.

type xorshift struct{ s uint64 }

func (x *xorshift) next() uint64 {
	x.s ^= x.s >> 12
	x.s ^= x.s << 25
	x.s ^= x.s >> 27
	return x.s * 2685821657736338717
}

func (x *xorshift) intn(n int) int { return int((x.next() >> 33) % uint64(n)) }

var longPieces = []string{
	"a", "b", "ab", "aab", "A", "K", "k", " ", "\t", "\u00a0", "\u3000", "ц", "ӆ", "Ꙁ", "Ꚁ", "䚀",
	"𡋀", "𡌀", "𣌀", "\xe2\x98", "☺", "\xff", "\xed\xa0\x80", "\u212a", "ſ", "ß", "İ", "ı", "Σ",
	"ς", "σ", "鄄", "\U000bc104", "x", "_", ".",
}

type longCase struct {
	h       string
	alpha   []string
	needles []string
}

func longCases() []longCase {
	rng := &xorshift{s: 0x9E3779B97F4A7C15}
	var cs []longCase
	for k := range 300 {
		m := 2 + rng.intn(4)
		alpha := make([]string, m)
		for i := range alpha {
			alpha[i] = longPieces[rng.intn(len(longPieces))]
		}
		maxN := 400
		if k >= 250 {
			maxN = 6000
		}
		n := 1 + rng.intn(maxN)
		var b strings.Builder
		for range n {
			b.WriteString(alpha[rng.intn(m)])
		}
		h := b.String()
		a := rng.intn(len(h))
		l := 2 + rng.intn(11)
		needles := []string{
			alpha[0] + alpha[1],
			h[a:min(len(h), a+l)],
			alpha[rng.intn(m)] + alpha[rng.intn(m)] + alpha[rng.intn(m)],
			"Q" + alpha[0],
		}
		cs = append(cs, longCase{h, alpha, needles})
	}
	return cs
}

// longRunes are the runes searched for with IndexRune in every haystack.
func longRunes() []rune {
	var rs []rune
	for _, p := range longPieces {
		r, _ := utf8.DecodeRuneInString(p)
		rs = append(rs, r)
	}
	return append(rs, -1, 0xD800, utf8.MaxRune+1, 'Q')
}

var longFuncs = []struct {
	name string
	f    func(c longCase, h *fnv64)
}{
	{"IndexRune", func(c longCase, h *fnv64) {
		for _, r := range longRunes() {
			h.i32(int32(strings.IndexRune(c.h, r)))
			h.i32(int32(bytes.IndexRune([]byte(c.h), r)))
		}
	}},
	{"Index", func(c longCase, h *fnv64) {
		for _, n := range c.needles {
			h.i32(int32(strings.Index(c.h, n)))
			h.i32(int32(strings.LastIndex(c.h, n)))
			h.i32(int32(strings.Count(c.h, n)))
			h.i32(int32(bytes.Index([]byte(c.h), []byte(n))))
			h.i32(int32(bytes.LastIndex([]byte(c.h), []byte(n))))
			h.i32(int32(bytes.Count([]byte(c.h), []byte(n))))
		}
	}},
	{"IndexAny", func(c longCase, h *fnv64) {
		set := strings.Join(c.alpha[1:], "")
		for _, cs := range []string{set, c.alpha[0], "Q\xff", "\u00a0\u3000"} {
			h.i32(int32(strings.IndexAny(c.h, cs)))
			h.i32(int32(strings.LastIndexAny(c.h, cs)))
			h.i32(int32(bytes.IndexAny([]byte(c.h), cs)))
			h.i32(int32(bytes.LastIndexAny([]byte(c.h), cs)))
		}
	}},
	{"Replace", func(c longCase, h *fnv64) {
		for _, n := range c.needles {
			h.blob([]byte(strings.Replace(c.h, n, "Z", -1)))
			h.blob([]byte(strings.Replace(c.h, n, "", 3)))
			h.blob(bytes.Replace([]byte(c.h), []byte(n), []byte("\u00e9"), -1))
		}
	}},
	{"Split", func(c longCase, h *fnv64) {
		for _, n := range c.needles {
			h.blob(encList(strings.SplitN(c.h, n, 5)))
			h.i32(int32(len(strings.Split(c.h, n))))
			h.i32(int32(len(strings.SplitAfter(c.h, n))))
			h.i32(int32(len(bytes.Split([]byte(c.h), []byte(n)))))
		}
	}},
	{"Replacer", func(c longCase, h *fnv64) {
		// generic (and byte / byte-string when the alphabet allows it)
		var on []string
		for i, a := range c.alpha {
			on = append(on, a, strings.Repeat("<", i))
		}
		h.blob([]byte(strings.NewReplacer(on...).Replace(c.h)))
		// single string (Boyer-Moore)
		if len(c.needles[1]) > 1 {
			h.blob([]byte(strings.NewReplacer(c.needles[1], "[m]").Replace(c.h)))
		}
		if len(c.needles[0]) > 1 {
			h.blob([]byte(strings.NewReplacer(c.needles[0], "").Replace(c.h)))
		}
		// generic with an empty old string
		h.blob([]byte(strings.NewReplacer("", "|", c.needles[0], "#").Replace(c.h[:min(len(c.h), 300)])))
	}},
	{"Case", func(c longCase, h *fnv64) {
		h.blob([]byte(strings.ToUpper(c.h)))
		h.blob([]byte(strings.ToLower(c.h)))
		h.blob([]byte(strings.Title(c.h)))
		h.blob(bytes.ToTitle([]byte(c.h)))
		h.bool(strings.EqualFold(c.h, strings.ToUpper(c.h)))
		h.bool(strings.EqualFold(strings.ToLower(c.h), c.h))
		h.bool(bytes.EqualFold([]byte(c.h), []byte(strings.ToTitle(c.h))))
	}},
	{"Space", func(c longCase, h *fnv64) {
		h.blob([]byte(strings.TrimSpace(c.h)))
		h.blob(encList(strings.Fields(c.h)))
		h.blob(bytes.TrimSpace([]byte(c.h)))
		h.blob(encList(bytes.Fields([]byte(c.h))))
		h.blob([]byte(strings.ToValidUTF8(c.h, "?")))
		h.blob([]byte(strings.Map(mapFold, c.h)))
	}},
}

func longLines() []hashLine {
	cs := longCases()
	var lines []hashLine
	for _, fn := range longFuncs {
		h := newFNV()
		n := 0
		for _, c := range cs {
			fn.f(c, &h)
			n += len(c.h)
		}
		lines = append(lines, hashLine{"long/" + fn.name, n, h})
	}
	return lines
}

// ---------------------------------------------------------------------------

func genAdversarial(args []string) error {
	fs := flag.NewFlagSet("adversarial", flag.ExitOnError)
	out := fs.String("out", "crates/go-unicode/tests/fixtures/adversarial_hashes.txt", "output file")
	if err := fs.Parse(args); err != nil {
		return err
	}
	var lines []hashLine
	lines = append(lines, structLines()...)
	lines = append(lines, orbitLines()...)
	lines = append(lines, strFamilyLines()...)
	lines = append(lines, foldPairLines()...)
	lines = append(lines, longLines()...)
	if err := writeHashLines(*out, lines); err != nil {
		return err
	}
	fmt.Fprintf(os.Stderr, "%s: %d lines\n", *out, len(lines))
	return nil
}
