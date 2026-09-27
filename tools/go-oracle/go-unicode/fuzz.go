//go:build go1.27

package main

import (
	"bytes"
	"io/fs"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"unicode"
	"unicode/utf8"
)

// Rich fuzz generator (fixtures -rich) and real-text corpus (fixtures
// -corpus) for the strings/bytes vectors.

// caseRunes are the code points with a case mapping or a non-trivial
// SimpleFold orbit (built once).
var caseRunes = func() []rune {
	var rs []rune
	for r := rune(0); r <= unicode.MaxRune; r++ {
		if unicode.SimpleFold(r) != r || unicode.ToUpper(r) != r || unicode.ToLower(r) != r ||
			unicode.ToTitle(r) != r || unicode.TurkishCase.ToUpper(r) != r || unicode.TurkishCase.ToLower(r) != r {
			rs = append(rs, r)
		}
	}
	return rs
}()

// spaceLikeRunes are White_Space, other Z/Cf runes and the separators Title
// and Fields care about.
var spaceLikeRunes = []rune{
	'\t', '\n', '\v', '\f', '\r', ' ', 0x85, 0xA0, 0x1680, 0x2000, 0x2001, 0x2002, 0x2003, 0x2004,
	0x2005, 0x2006, 0x2007, 0x2008, 0x2009, 0x200A, 0x2028, 0x2029, 0x202F, 0x205F, 0x3000,
	0x180E, 0x200B, 0x200C, 0x200D, 0x2060, 0xFEFF, 0x1C, 0x1D, 0x1E, 0x1F, '_', '-', '\'', '.',
	'0', '9', 0x660, 0xFF10, 0x2E3A, 0x00AD, 0x061C,
}

// badSeqs are invalid UTF-8 sequences of every kind.
var badSeqs = []string{
	"\x80", "\xbf", "\xc0\x80", "\xc1\xbf", "\xc2", "\xdf", "\xe0\x80\x80", "\xe0\x9f\xbf",
	"\xe0\xa0", "\xed\xa0\x80", "\xed\xbf\xbf", "\xed\x9f", "\xef\xbf", "\xf0\x80\x80\x80",
	"\xf0\x8f\xbf\xbf", "\xf0\x90\x80", "\xf4\x90\x80\x80", "\xf4\x8f\xbf", "\xf5\x80\x80\x80",
	"\xf7\xbf\xbf\xbf", "\xf8\x88\x80\x80\x80", "\xfc\x84\x80\x80\x80\x80", "\xfe", "\xff",
	"\xc0\xaf", "\xe0\x80\xaf", "\xf0\x80\x80\xaf", "\xef\xbf\xbd",
}

// richStr is the rich counterpart of str: random code points from all
// planes, case-mapping runes, spaces/separators, random bytes, truncated and
// invalid encodings, mixed with the default pieces.
func (g *strGen) richStr(maxPieces int) string {
	n := g.r.IntN(maxPieces + 1)
	var b strings.Builder
	for range n {
		switch g.r.IntN(10) {
		case 0:
			b.WriteString(pieces[g.r.IntN(len(pieces))])
		case 1, 2:
			b.WriteRune(caseRunes[g.r.IntN(len(caseRunes))])
		case 3:
			b.WriteRune(rune(g.r.IntN(unicode.MaxRune + 1)))
		case 4:
			b.WriteRune(spaceLikeRunes[g.r.IntN(len(spaceLikeRunes))])
		case 5:
			b.WriteByte(byte(g.r.IntN(256)))
		case 6:
			// Truncated encoding of a random multi-byte rune.
			s := string(rune(0x80 + g.r.IntN(unicode.MaxRune+1-0x80)))
			b.WriteString(s[:1+g.r.IntN(len(s)-1)])
		case 7:
			b.WriteString(badSeqs[g.r.IntN(len(badSeqs))])
		case 8:
			b.WriteByte(byte('A' + g.r.IntN(58)))
		default:
			// A case rune followed by a continuation byte or a lead byte.
			b.WriteRune(caseRunes[g.r.IntN(len(caseRunes))])
			b.WriteByte(byte(0x80 + g.r.IntN(0x80)))
		}
	}
	return b.String()
}

// mutate returns s with a few random byte-level edits (insert an invalid or
// case-mapping sequence, flip a byte, cut a byte).
func (g *strGen) mutate(s string) string {
	b := []byte(s)
	for range 1 + g.r.IntN(3) {
		pos := 0
		if len(b) > 0 {
			pos = g.r.IntN(len(b) + 1)
		}
		switch g.r.IntN(4) {
		case 0:
			ins := badSeqs[g.r.IntN(len(badSeqs))]
			b = append(b[:pos], append([]byte(ins), b[pos:]...)...)
		case 1:
			ins := string(caseRunes[g.r.IntN(len(caseRunes))])
			b = append(b[:pos], append([]byte(ins), b[pos:]...)...)
		case 2:
			if pos < len(b) {
				b[pos] ^= byte(1 << g.r.IntN(8))
			}
		default:
			if pos < len(b) {
				b = append(b[:pos], b[pos+1:]...)
			}
		}
	}
	return string(b)
}

// loadCorpus returns up to max distinct lines (at most 400 bytes; longer lines
// are cut into chunks) of the text files under dir, in a deterministic order.
func loadCorpus(dir string, max int) ([]string, error) {
	exts := map[string]bool{".md": true, ".html": true, ".toml": true, ".yaml": true, ".yml": true,
		".json": true, ".txt": true, ".xml": true, ".css": true, ".scss": true, ".js": true}
	var files []string
	err := filepath.WalkDir(dir, func(p string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if d.IsDir() {
			switch d.Name() {
			case "node_modules", ".git", "public", "resources":
				return filepath.SkipDir
			}
			return nil
		}
		if exts[strings.ToLower(filepath.Ext(p))] {
			files = append(files, p)
		}
		return nil
	})
	if err != nil {
		return nil, err
	}
	sort.Strings(files)
	seen := map[string]bool{}
	var lines []string
	for _, f := range files {
		data, err := os.ReadFile(f)
		if err != nil {
			return nil, err
		}
		for _, l := range bytes.Split(data, []byte("\n")) {
			for len(l) > 0 {
				k := min(len(l), 400)
				// Do not cut inside a rune: the corpus is real text.
				for k < len(l) && k > 0 && !utf8.RuneStart(l[k]) {
					k--
				}
				if k == 0 {
					k = min(len(l), 400)
				}
				c := string(l[:k])
				l = l[k:]
				if strings.TrimSpace(c) == "" || seen[c] {
					continue
				}
				seen[c] = true
				lines = append(lines, c)
			}
		}
	}
	// Keep an evenly spread subset.
	if len(lines) > max {
		step := float64(len(lines)) / float64(max)
		sub := make([]string, 0, max)
		for i := range max {
			sub = append(sub, lines[int(float64(i)*step)])
		}
		lines = sub
	}
	return lines, nil
}
