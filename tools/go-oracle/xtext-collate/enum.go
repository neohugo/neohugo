package main

// Exhaustive enumerations for crates/xtext-collate: every string of length
// 0..maxlen over a small alphabet (Thai block, Latin contraction letters,
// marks, ignorables), keyed and compared under many configs. The Rust test
// regenerates the same enumeration from the alphabet written in the file.

import (
	"bufio"
	"crypto/sha256"
	"encoding/binary"
	"encoding/hex"
	"flag"
	"fmt"
	"os"
	"strings"

	"golang.org/x/text/collate"
)

var enumAlphabets = map[string][]string{
	// The whole Thai block (assigned in Unicode 6.2) plus a few neighbours.
	"thai": func() []string {
		var a []string
		for r := rune(0x0E01); r <= 0x0E5B; r++ {
			if r >= 0x0E3B && r <= 0x0E3E {
				continue
			}
			a = append(a, string(r))
		}
		return append(a, "a", "A", " ", "1", "́", "-", "\u200b")
	}(),
	// Letters that start or end contractions in some locale, marks that
	// interact with them (discontiguous matches), ignorables, digits.
	"latin": {"a", "A", "c", "C", "h", "H", "l", "L", "·", "ŀ", "d", "z", "ž", "s", "n", "y", "g", "j",
		"o", "e", "i", "I", "ı", "İ", "́", "̈", "̊", "̧", "̌", "̣", " ", "-", "'", "1", "0", "\u200b", "\u00ad", "å"},
	// Combining-mark ordering: starters and marks with various ccc values
	// (doNorm), including Thai and Tibetan.
	"marks": {"a", "o", "ก", "เ", "ཀ", "́", "̣", "̧", "̈", "̛", "ͅ",
		"ุ", "่", "้", "ั", "์", "ཱ", "ི", "ྀ", "ུ", "ྵ", "ྷ", "゙", "्", "़"},
}

func enumStrings(alpha []string, maxLen int) [][]byte {
	var out [][]byte
	var rec func(prefix []byte, n int)
	rec = func(prefix []byte, n int) {
		if n == 0 {
			out = append(out, append([]byte{}, prefix...))
			return
		}
		for _, u := range alpha {
			rec(append(prefix, u...), n-1)
		}
	}
	for l := 0; l <= maxLen; l++ {
		rec(nil, l)
	}
	return out
}

func enumDigests(c *collate.Collator, strs [][]byte) (string, string) {
	hk := sha256.New()
	hc := sha256.New()
	var buf collate.Buffer
	var lb [binary.MaxVarintLen64]byte
	for _, s := range strs {
		buf.Reset()
		k := c.Key(&buf, s)
		hk.Write(lb[:binary.PutUvarint(lb[:], uint64(len(k)))])
		hk.Write(k)
	}
	n := len(strs)
	res := make([]byte, 0, 4096)
	for i := 0; i < n; i++ {
		a := strs[i]
		b := strs[(i+1)%n]
		d := strs[(i*7919+13)%n]
		res = append(res, byte(c.Compare(a, b)+1), byte(c.Compare(b, a)+1), byte(c.Compare(a, d)+1))
		if len(res) >= 4096 {
			hc.Write(res)
			res = res[:0]
		}
	}
	hc.Write(res)
	return hex.EncodeToString(hk.Sum(nil)), hex.EncodeToString(hc.Sum(nil))
}

// cmdEnum writes
//
//	E	<maxlen>	<hex unit>,<hex unit>,...
//	C	<name>	<tag>	<opts>	<keys digest>	<compare digest>
//	[H	<config name>	<i>	<key hash>]   with -dump
func cmdEnum(args []string) error {
	fs := flag.NewFlagSet("enum", flag.ExitOnError)
	out := fs.String("out", "", "output file")
	alpha := fs.String("alpha", "thai", "alphabet: thai|latin|marks")
	maxLen := fs.Int("maxlen", 3, "maximum string length (in alphabet units)")
	locales := fs.Bool("locales", false, "also run every supported locale")
	dump := fs.String("dump", "", "config name whose per-string key hashes are written")
	_ = fs.Parse(args)
	a, ok := enumAlphabets[*alpha]
	if !ok {
		return fmt.Errorf("unknown alphabet %q", *alpha)
	}
	strs := enumStrings(a, *maxLen)
	f, err := os.Create(*out)
	if err != nil {
		return err
	}
	w := bufio.NewWriter(f)
	var hx []string
	for _, u := range a {
		hx = append(hx, hex.EncodeToString([]byte(u)))
	}
	_, _ = fmt.Fprintf(w, "E\t%d\t%s\n", *maxLen, strings.Join(hx, ","))
	cfgs := pairConfigs()
	if *locales {
		cfgs = append(cfgs, localeConfigs()...)
	}
	for _, cfg := range cfgs {
		c := cfg.collator()
		kd, cd := enumDigests(c, strs)
		_, _ = fmt.Fprintf(w, "C\t%s\t%s\t%s\n", cfg.line(), kd, cd)
		if cfg.name == *dump {
			var buf collate.Buffer
			for i, s := range strs {
				buf.Reset()
				_, _ = fmt.Fprintf(w, "H\t%s\t%d\t%s\n", cfg.name, i, keyHash(c.Key(&buf, s)))
			}
		}
	}
	if err := w.Flush(); err != nil {
		return err
	}
	return f.Close()
}

// pairConfigs: mainConfigs plus the combinations TestNonDigits /
// TestNumericCompare use and a few more Hugo-relevant tags.
func pairConfigs() []config {
	cs := mainConfigs()
	cs = append(cs,
		config{"en+loose,numeric", "en", []string{"loose", "numeric"}},
		config{"th+loose,numeric", "th", []string{"loose", "numeric"}},
		config{"en+numeric", "en", []string{"numeric"}},
		config{"th-TH", "th-TH", nil},
		config{"en-US", "en-US", nil},
		config{"th-u-ks-level2", "th-u-ks-level2", nil},
		config{"th-u-ka-shifted-ks-level4", "th-u-ka-shifted-ks-level4", nil},
		config{"th-u-ka-posix-ks-level4", "th-u-ka-posix-ks-level4", nil},
		config{"th-u-kb-true", "th-u-kb-true", nil},
		config{"th-u-kc-true-ks-level1", "th-u-kc-true-ks-level1", nil},
	)
	return cs
}
