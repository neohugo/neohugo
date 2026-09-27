package main

// Adversarial inputs (-adv N): paths built from dot runs, slash runs,
// Thai/multi-byte and invalid UTF-8 segments (Clean/Join/Rel/Split/Ext/Base/
// Dir/IsLocal), and glob patterns with multi-byte and invalid-UTF-8
// character classes, escapes at every position and star runs (Match).

import (
	"math/rand/v2"
	"strings"
)

var (
	advPathParts = []string{"/", "/", "//", "///", ".", ".", "..", "...", "....", "./", "../", "/.", "/..", "a", "b",
		"abc", "ไทย", "ก", "\u0e01\u0e34", "é", "€", "😀", "\xff", "\xe0\xb8", "\xc3", "\x80", " ", "\t", "\x00",
		":", "::", "\\", "\\\\", "C:", "c:\\", "~", ".a", "a.", "a.b", ".a.b.", "index.html", "x.tar.gz", "..a",
		"a..", ". .", "%2F", "?", "*", "[", "]"}
	advPatParts = []string{"*", "**", "?", "??", "[", "]", "^", "-", "\\", "\\\\", "a", "b", "c", "/", "é", "ก",
		"[a-c]", "[^a]", "\\*", "\\?", "\\[", "[\\]]", "[é-ü]", "[ก-ฮ]", "[^ก-ฮ]", "[\\-]", "[a\\-z]", "[]a]",
		"[^]a]", "[]", "[^]", "[a-]", "[-a]", "[a--]", "[\\", "[a-\\", "[\xff]", "[\xff-a]", "[a-\xff]", "\xff",
		"\xe0\xb8", "[\xe0\xb8]", "[\xe0\xb8\x81-\xe0\xb8\xae]", "[^\x00-\x7f]", "[\x00-\xff]", "*/", "/*",
		"*.html", "[a-z]*", "ไทย", "*ไ*", "?ย", "[!a]", "[[]", "[]]", "[a]]"}
	advNameParts = []string{"a", "b", "c", "/", "é", "ü", "ก", "ข", "ฮ", "ไทย", "*", "?", "-", "]", "[", "^",
		"\\", "\xff", "\xe0\xb8", "\x80", "\x00", "ab", "index.html", ".", "..", " ", "z", "Z", "0"}
	// UTF-8 boundary and malformed sequences (overlong, surrogate, > U+10FFFF,
	// truncated), exercising every row of DecodeRune's acceptRanges.
	utf8Edges = []string{"\xc0\x80", "\xc1\xbf", "\xc2\x80", "\xdf\xbf", "\xe0\x80\x80", "\xe0\x9f\xbf",
		"\xe0\xa0\x80", "\xed\x9f\xbf", "\xed\xa0\x80", "\xed\xbf\xbf", "\xee\x80\x80", "\xef\xbf\xbf",
		"\xf0\x80\x80\x80", "\xf0\x8f\xbf\xbf", "\xf0\x90\x80\x80", "\xf3\xbf\xbf\xbf", "\xf4\x8f\xbf\xbf",
		"\xf4\x90\x80\x80", "\xf5\x80\x80\x80", "\xf0\x90\x80", "\xe1\x80", "\xc3", "\xe1\x80\x2f", "\xf0\x90\x2f\x80"}
)

func advJoin(rng *rand.Rand, parts []string, max int) string {
	var b strings.Builder
	for k := rng.IntN(max + 1); k > 0; k-- {
		b.WriteString(parts[rng.IntN(len(parts))])
	}
	return b.String()
}

func adversarial(w *writer, rng *rand.Rand, n int) {
	for i := 0; i < n; i++ {
		p := advJoin(rng, advPathParts, 10)
		single(w, p)
		elems := make([]string, rng.IntN(6))
		for j := range elems {
			elems[j] = advJoin(rng, advPathParts, 5)
		}
		join(w, elems)
		pair(w, advJoin(rng, advPathParts, 8), advJoin(rng, advPathParts, 8))
		// Rel between a path and a lexical variant of it.
		q := p + advJoin(rng, []string{"/", "/..", "/.", "/x", "/../y", "//", "/ไทย"}, 4)
		pair(w, p, q)
		pair(w, q, p)
		pair(w, "/"+p, "/"+q)
		// Match.
		for k := 0; k < 3; k++ {
			pat := advJoin(rng, advPatParts, 6)
			name := advJoin(rng, advNameParts, 7)
			args := []string{pat, name}
			m, err := pathMatch(pat, name)
			w.rec("path.Match", args, boolStr(m), errStr(err))
			m, err = filepathMatch(pat, name)
			w.rec("filepath.Match", args, boolStr(m), errStr(err))
		}
	}
	// UTF-8 edge sequences as names, as '?' targets and inside classes.
	for _, e := range utf8Edges {
		for _, f := range utf8Edges {
			for _, pat := range []string{"?", "??", "???", "????", "*", "[" + e + "]", "[^" + e + "]", "[" + e + "-" + f + "]",
				"[\\" + e + "]", "?" + f, "*" + f, "\\" + e, e + "*", "[a-" + e + "]"} {
				for _, name := range []string{e, e + f, f, e + "a", "a" + e} {
					args := []string{pat, name}
					m, err := pathMatch(pat, name)
					w.rec("path.Match", args, boolStr(m), errStr(err))
					m, err = filepathMatch(pat, name)
					w.rec("filepath.Match", args, boolStr(m), errStr(err))
				}
			}
		}
		single(w, e+"/."+e+"/../x")
	}
	// Every byte against single-character patterns and classes.
	for c := 0; c < 256; c++ {
		b := string([]byte{byte(c)})
		for _, pat := range []string{"?", "*", "[" + b + "]", "[^" + b + "]", "\\" + b, b, "[a-" + b + "]", "[" + b + "-z]", "*" + b, b + "*"} {
			for _, name := range []string{b, "a", "z", "é", "ก", "\xff", "/", "", b + b, "x" + b} {
				args := []string{pat, name}
				m, err := pathMatch(pat, name)
				w.rec("path.Match", args, boolStr(m), errStr(err))
				m, err = filepathMatch(pat, name)
				w.rec("filepath.Match", args, boolStr(m), errStr(err))
			}
		}
		single(w, "a"+b+"b/."+b+"/..")
	}
}
