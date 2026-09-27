package main

import (
	"bufio"
	"compress/gzip"
	"fmt"
	"io"
	"math/rand"
	"os"
	"sort"
	"strconv"
	"strings"

	"github.com/tdewolff/parse/v2"
	"github.com/tdewolff/parse/v2/js"
)

// genTables writes OUT (gzip text) with exhaustive checks of the small
// API surface that the fuzzers reach only sparsely but the minifier relies
// on: TokenType.String for all 65536 values, OpPrec/DeclType strings,
// IsIdentifierStart/Continue/End and the lexer's view of every code point
// (range-compressed), AsIdentifierName/AsDecimalLiteral over all short
// strings of an interesting alphabet, and sort.Sort(VarsByUses) tie order.
func genTables(out string, seed int64) {
	f, err := os.Create(out)
	if err != nil {
		panic(err)
	}
	gz, _ := gzip.NewWriterLevel(f, gzip.BestCompression)
	w := bufio.NewWriter(gz)

	// TokenType.String: only the values with a name (all others are Invalid(N))
	fmt.Fprintln(w, "#tokens")
	for i := 0; i < 1<<16; i++ {
		tt := js.TokenType(i)
		if tt.Bytes() != nil {
			fmt.Fprintf(w, "%d %s\n", i, tt.String())
		}
	}

	fmt.Fprintln(w, "#opprec")
	for i := -3; i < 30; i++ {
		fmt.Fprintf(w, "%d %s\n", i, js.OpPrec(i).String())
	}
	fmt.Fprintln(w, "#decltype")
	for i := 0; i < 12; i++ {
		fmt.Fprintf(w, "%d %s\n", i, js.DeclType(i).String())
	}

	// every code point (surrogates encoded as raw 3-byte sequences, which Go's
	// utf8 rejects but parse.Input.PeekRune decodes)
	fmt.Fprintln(w, "#runes")
	start, prev := 0, ""
	for r := 0; r <= 0x10FFFF+1; r++ {
		sig := ""
		if r <= 0x10FFFF {
			sig = runeSig(r)
		}
		if r == 0 {
			prev = sig
			continue
		}
		if sig != prev {
			fmt.Fprintf(w, "%x %x %s\n", start, r-1, prev)
			start, prev = r, sig
		}
	}

	// invalid and truncated UTF-8 sequences
	fmt.Fprintln(w, "#bytes")
	for _, b := range invalidSeqs() {
		fmt.Fprintf(w, "%x %s\n", b, bytesSig(b))
	}

	fmt.Fprintln(w, "#asname")
	alpha := []byte{'0', '1', '9', '.', 'a', 'Z', '$', '_', 'e', 'x', ' ', 0x80, 0xC3, '-', 'n'}
	var cur []byte
	var rec func(n int)
	rec = func(n int) {
		fmt.Fprintf(w, "%x %s%s\n", cur, b01(js.AsIdentifierName(cur)), b01(js.AsDecimalLiteral(cur)))
		if n == 0 {
			return
		}
		for _, c := range alpha {
			cur = append(cur, c)
			rec(n - 1)
			cur = cur[:len(cur)-1]
		}
	}
	rec(3)

	// sort.Sort(VarsByUses) with many ties: the renamer's order of names
	fmt.Fprintln(w, "#sort")
	rnd := rand.New(rand.NewSource(seed))
	for k := 0; k < 3000; k++ {
		n := rnd.Intn(8)
		if rnd.Intn(3) == 0 {
			n = rnd.Intn(400)
		}
		maxUses := 1 + rnd.Intn(6)
		if rnd.Intn(4) == 0 {
			maxUses = 1 + rnd.Intn(70000)
		}
		vs := make(js.VarArray, n)
		uses := make([]string, n)
		for i := range vs {
			vs[i] = &js.Var{Data: []byte(strconv.Itoa(i)), Uses: uint16(rnd.Intn(maxUses))}
			uses[i] = strconv.Itoa(int(vs[i].Uses))
		}
		sort.Sort(js.VarsByUses(vs))
		perm := make([]string, n)
		for i, v := range vs {
			perm[i] = string(v.Data)
		}
		fmt.Fprintf(w, "%s|%s\n", strings.Join(uses, ","), strings.Join(perm, ","))
	}

	if err := w.Flush(); err != nil {
		panic(err)
	}
	if err := gz.Close(); err != nil {
		panic(err)
	}
	if err := f.Close(); err != nil {
		panic(err)
	}
}

func b01(b bool) string {
	if b {
		return "1"
	}
	return "0"
}

// encRune encodes r as UTF-8 without validation (surrogates too).
func encRune(r int) []byte {
	switch {
	case r < 0x80:
		return []byte{byte(r)}
	case r < 0x800:
		return []byte{0xC0 | byte(r>>6), 0x80 | byte(r)&0x3F}
	case r < 0x10000:
		return []byte{0xE0 | byte(r>>12), 0x80 | byte(r>>6)&0x3F, 0x80 | byte(r)&0x3F}
	}
	return []byte{0xF0 | byte(r>>18), 0x80 | byte(r>>12)&0x3F, 0x80 | byte(r>>6)&0x3F, 0x80 | byte(r)&0x3F}
}

func runeSig(r int) string {
	return bytesSig(encRune(r))
}

// bytesSig: IsIdentifierStart/Continue/End of b, then the token streams of
// b, "a"+b+"a", " "+b, "#"+b, "\n"+b and "/x/"+b (with RegExp).
func bytesSig(b []byte) string {
	var sb strings.Builder
	sb.WriteString(b01(js.IsIdentifierStart(b)))
	sb.WriteString(b01(js.IsIdentifierContinue(b)))
	sb.WriteString(b01(js.IsIdentifierEnd(b)))
	cat := func(parts ...[]byte) []byte {
		var o []byte
		for _, p := range parts {
			o = append(o, p...)
		}
		return o
	}
	for _, in := range [][]byte{b, cat([]byte("a"), b, []byte("a")), cat([]byte(" "), b), cat([]byte("#"), b), cat([]byte("\n-->"), b), cat([]byte("'"), b, []byte("'")), cat([]byte("`"), b, []byte("`"))} {
		sb.WriteByte('|')
		sb.WriteString(tokSig(in, false))
	}
	sb.WriteByte('|')
	sb.WriteString(tokSig(cat([]byte("/x/"), b), true))
	return sb.String()
}

// tokSig lexes src and returns "tt:len" per token (at most 6), "E" for an
// error token with a non-EOF error.
func tokSig(src []byte, re bool) (s string) {
	defer func() {
		if r := recover(); r != nil {
			s += "P"
		}
	}()
	l := js.NewLexer(parse.NewInputBytes(append([]byte(nil), src...)))
	var parts []string
	for i := 0; i < 6; i++ {
		tt, data := l.Next()
		if re && i == 0 && tt == js.DivToken {
			tt, data = l.RegExp()
		}
		p := strconv.Itoa(int(tt)) + ":" + strconv.Itoa(len(data))
		if tt == js.ErrorToken {
			if l.Err() == io.EOF {
				parts = append(parts, p)
				break
			}
			p += "E"
		}
		parts = append(parts, p)
	}
	return strings.Join(parts, ",")
}

func invalidSeqs() [][]byte {
	var out [][]byte
	lead := []byte{0x80, 0xBF, 0xC0, 0xC1, 0xC2, 0xC3, 0xDF, 0xE0, 0xE2, 0xED, 0xEF, 0xF0, 0xF4, 0xF5, 0xF8, 0xFF}
	cont := []byte{0x00, 0x20, 0x41, 0x7F, 0x80, 0x9F, 0xA0, 0xA8, 0xBF, 0xC0, 0xFF}
	for _, a := range lead {
		out = append(out, []byte{a})
		for _, b := range cont {
			out = append(out, []byte{a, b})
			for _, c := range cont {
				out = append(out, []byte{a, b, c})
			}
		}
	}
	return out
}
