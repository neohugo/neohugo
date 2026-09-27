package main

import (
	"bufio"
	"fmt"
	"math/rand"
	"os"
	"path/filepath"
	"strings"
)

// Differential fuzzing:
//
//	tdewolff-parse fuzz OUTDIR N SEED EXHAUST MAXWIN CORPUSROOT...
//
// writes OUTDIR/<group>.fz (groups html, css, xml, json) with a header line
// "#kinds K1 K2 ..." and one "hex(input)\tfnv64(K1 stream)\t..." record per
// input. Inputs come from exhaustive enumeration of all strings up to length
// EXHAUST (-1: per-group default; 100+k: length k over all 256 byte values)
// over small per-group alphabets after
// interesting prefixes, random strings over those alphabets, arbitrary random
// bytes, fragment splices and mutated windows (at most MAXWIN bytes) of real
// corpus files. The Rust side (tests/fuzz.rs) recomputes every digest; a
// small set is checked in (tests/fixtures/fuzz), large ones stay outside the
// repository.

type kindGen struct {
	kinds    []string
	alpha    string
	prefixes []string
	frags    []string
	exts     []string
	exhaust  int // max length of the exhaustive suffixes
}

var fuzzKinds = map[string]kindGen{
	"html": {
		kinds: []string{"html", "htmltmpl", "htmlphp"},
		alpha: "<>/!-?a=\"' \x00[{}%sS",
		prefixes: []string{"", "<script>", "<style>", "<textarea>", "<svg>", "<math>", "<plaintext>", "<script><!--",
			"<script><!--<script>", "<a ", "<a b", "<a b=", "<!--", "<![CDATA[", "<!doctype", "<!DOCTYPE ", "</", "<?", "{{",
			"<a {{", "<a b={{", "<p>x", "<xmp>", "<title>", "<iframe>", "<a b='", "<?php", "<svg><![CDATA[", "<svg a=\""},
		frags: append(append([]string{}, htmlExtra...), "<!--", "-->", "--!>", "<!-", "</script>", "</style", "</SCRIPT >",
			"<script>", "</svg>", "</math>", "<svg", "<math", "{{", "}}", "{{\"}}\"}}", "<%", "%>", "<?", "?>", "<!", "<![CDATA[", "]]>",
			"<!doctype html>", "<!DOCTYPE", "\x00", "\f", "\r\n", "&amp;", "<a href=x>", "<A HREF=X>", "<b/>", "</ b>", "</\t>"),
		exts:    []string{".html"},
		exhaust: 3,
	},
	"css": {
		kinds: []string{"csslex", "css", "cssinline"},
		alpha: "-\\:;{}()u+.e1\"' /*@a,!%\n\x00",
		prefixes: []string{"", "a{", "a{b:", "@media ", "@media x{", "@font-face{", "@x{", "url(", "u+", "\\", "--x:", "*",
			"a{--x:", "@-webkit-", "#", "\"", "/*", "a{*b:", "a[", "1", "-", "--"},
		frags: append(append([]string{}, cssExtra...), "url(", "url( ", ")", "U+", "u+0-f", "u+??", "\\\n", "\\\r\n", "\\\f",
			"\\30", "\\1F600 ", "\\é", "é", "\"\\\n\"", "'\n", "--", "-->", "<!--", "@-moz-document", "@page", "@keyframes",
			"@supports", "@font-face", "!important", "1e+5", "1.e5", ".5e-3", "+.5", "-.5", "1e", "1ex", "2%", "--a:{};", "url(\"a\" )",
			"url(a b)", "url(a\\)b)", "URL(a)", "u\\rl(a)", "\\75rl(x)", "*zoom", "_a", ";", "}", "{", "(", "[", "]", "\x00", "\x7f", "\x1f"),
		exts:    []string{".css", ".scss", ".less", ".in", ".out"},
		exhaust: 3,
	},
	"xml": {
		kinds: []string{"xml"},
		alpha: "<>/!-?a=\"' \x00[]\t\nD",
		prefixes: []string{"", "<a ", "<a b=", "<a b=\"", "<!--", "<![CDATA[", "<!DOCTYPE", "<!DOCTYPE x [", "<?", "<?xml ", "</",
			"<a>", "<!", "<!DOCTYPE \"", "<a b='"},
		frags: append(append([]string{}, xmlExtra...), "<!--", "-->", "<![CDATA[", "]]>", "<!DOCTYPE", "[", "]", "\"", "?>", "/>",
			"\x00", "\t", "\r\n", "<a", "</a>", "b=c", "b=\"c\td\"", "<?xml-stylesheet href=\"a\"?>"),
		exts:    []string{".xml", ".svg"},
		exhaust: 4,
	},
	"json": {
		kinds:    []string{"json"},
		alpha:    "{}[],:\"\\-0.1eE+tn \x00",
		prefixes: []string{"", "{", "[", "{\"a\":", "[1,", "{\"a\":1,", "\"", "-", "0", "1.", "1e", "{\"a\"", "[[", "{\"a\":{"},
		frags: append(append([]string{}, jsonExtra...), "true", "false", "null", "\"\\\"\"", "\"\\\\\"", "\"\\\\\\\"", "-0",
			"1.5e+10", "1E-2", "0.", ".5", "00", "\x00", "é", "{\"@type\":\"x\"}"),
		exts:    []string{".json"},
		exhaust: 3,
	},
}

var allBytes = func() string {
	b := make([]byte, 256)
	for i := range b {
		b[i] = byte(i)
	}
	return string(b)
}()

func loadCorpus(roots []string, exts []string, maxFiles int, r *rand.Rand) [][]byte {
	var files []string
	for _, root := range roots {
		if _, err := os.Stat(root); err != nil {
			continue
		}
		files = append(files, listFiles(root, exts)...)
	}
	r.Shuffle(len(files), func(i, j int) { files[i], files[j] = files[j], files[i] })
	if len(files) > maxFiles {
		files = files[:maxFiles]
	}
	var out [][]byte
	for _, f := range files {
		b, err := os.ReadFile(f)
		if err == nil && len(b) > 0 {
			out = append(out, b)
		}
	}
	return out
}

// enumerate calls f with every string of length 0..maxLen over alpha.
func enumerate(alpha string, maxLen int, f func([]byte)) {
	var rec func(prefix []byte, n int)
	rec = func(prefix []byte, n int) {
		f(prefix)
		if n == 0 {
			return
		}
		for i := 0; i < len(alpha); i++ {
			rec(append(prefix, alpha[i]), n-1)
		}
	}
	rec(nil, maxLen)
}

func mutate(r *rand.Rand, b []byte, g kindGen) []byte {
	b = append([]byte(nil), b...)
	k := 1 + r.Intn(6)
	for i := 0; i < k; i++ {
		pos := 0
		if len(b) > 0 {
			pos = r.Intn(len(b) + 1)
		}
		switch r.Intn(9) {
		case 0, 1: // insert alphabet byte
			b = append(b[:pos], append([]byte{g.alpha[r.Intn(len(g.alpha))]}, b[pos:]...)...)
		case 2: // insert random byte
			b = append(b[:pos], append([]byte{byte(r.Intn(256))}, b[pos:]...)...)
		case 3: // delete a range
			if len(b) > 0 {
				p := r.Intn(len(b))
				q := p + r.Intn(min(len(b)-p, 16)+1)
				b = append(b[:p], b[q:]...)
			}
		case 4: // overwrite a byte
			if len(b) > 0 {
				b[r.Intn(len(b))] = g.alpha[r.Intn(len(g.alpha))]
			}
		case 5, 6: // insert a fragment
			f := g.frags[r.Intn(len(g.frags))]
			b = append(b[:pos], append([]byte(f), b[pos:]...)...)
		case 7: // duplicate a range
			if len(b) > 0 {
				p := r.Intn(len(b))
				q := p + r.Intn(min(len(b)-p, 32)+1)
				d := append([]byte(nil), b[p:q]...)
				b = append(b[:pos], append(d, b[pos:]...)...)
			}
		case 8: // truncate
			b = b[:pos]
		}
	}
	return b
}

func genOne(r *rand.Rand, g kindGen, corpus [][]byte, maxWin int) []byte {
	switch x := r.Intn(20); {
	case x < 4: // random string over the alphabet after a prefix
		b := []byte(g.prefixes[r.Intn(len(g.prefixes))])
		n := r.Intn(24)
		for i := 0; i < n; i++ {
			b = append(b, g.alpha[r.Intn(len(g.alpha))])
		}
		return b
	case x < 5: // random bytes
		n := r.Intn(24)
		b := make([]byte, n)
		for i := range b {
			b[i] = byte(r.Intn(256))
		}
		return b
	case x < 10: // fragment splices
		var b []byte
		n := 1 + r.Intn(8)
		for i := 0; i < n; i++ {
			f := g.frags[r.Intn(len(g.frags))]
			if len(f) > 0 && r.Intn(4) == 0 {
				a := r.Intn(len(f))
				f = f[a : a+r.Intn(len(f)-a+1)]
			}
			b = append(b, f...)
			if r.Intn(3) == 0 {
				b = append(b, g.alpha[r.Intn(len(g.alpha))])
			}
		}
		return mutate(r, b, g)
	default: // mutated window of a corpus file
		if len(corpus) == 0 {
			return mutate(r, []byte(g.prefixes[r.Intn(len(g.prefixes))]), g)
		}
		c := corpus[r.Intn(len(corpus))]
		p := r.Intn(len(c))
		if r.Intn(3) == 0 {
			p = 0
		}
		q := p + 1 + r.Intn(min(len(c)-p, 1+r.Intn(maxWin)))
		if r.Intn(10) == 0 && len(c) <= maxWin {
			q = len(c)
		}
		return mutate(r, c[p:q], g)
	}
}

func fuzzGen(outDir string, n int, seed int64, exhaust, maxWin int, roots []string) {
	if err := os.MkdirAll(outDir, 0o755); err != nil {
		panic(err)
	}
	for _, name := range []string{"html", "css", "xml", "json"} {
		g := fuzzKinds[name]
		r := rand.New(rand.NewSource(seed))
		corpus := loadCorpus(roots, g.exts, 400, r)
		ex, alpha := g.exhaust, g.alpha
		if exhaust >= 100 { // all 256 byte values, length exhaust-100
			ex, alpha = exhaust-100, allBytes
		} else if exhaust >= 0 {
			ex = exhaust
		}
		var inputs [][]byte
		for _, p := range g.prefixes {
			enumerate(alpha, ex, func(s []byte) {
				inputs = append(inputs, append([]byte(p), s...))
			})
		}
		for i := 0; i < n; i++ {
			inputs = append(inputs, genOne(r, g, corpus, maxWin))
		}
		f, err := os.Create(filepath.Join(outDir, name+".fz"))
		if err != nil {
			panic(err)
		}
		w := bufio.NewWriterSize(f, 1<<20)
		fmt.Fprintf(w, "#kinds %s\n", strings.Join(g.kinds, " "))
		for _, in := range inputs {
			w.WriteString(hx(in))
			for _, kind := range g.kinds {
				fmt.Fprintf(w, "\t%016x", fnv64a(streamFor(kind, in)))
			}
			w.WriteByte('\n')
		}
		w.Flush()
		f.Close()
		fmt.Fprintf(os.Stderr, "%s.fz: %d inputs x %d kinds (%d corpus files)\n", name, len(inputs), len(g.kinds), len(corpus))
	}
}
