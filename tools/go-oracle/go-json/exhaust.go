package main

// Enumerated inputs (-mode exhausttext / exhaustencode), written by the
// second independent verifier of crates/go-json. Where adv.go draws random
// inputs, these modes walk small input spaces completely, so every error
// path of the scanner and of the string and number grammars is reached with
// every byte:
//
//   exhausttext (text record format):
//     - every input of 1 and 2 bytes, and every 3-byte input over an
//       alphabet of 48 syntax, digit, letter, control and non-ASCII bytes;
//     - every 1- and 2-byte string body, every "\X" escape (also in an
//       object name), every "\uXXXX" code unit, every third code unit after
//       three high surrogates, and 3-/4-byte UTF-8 sequences with
//       interesting continuation bytes;
//     - every number-like string of up to 4 bytes over "0123456789-+.eE",
//       and of 5 bytes over "01-+.eE9";
//     - every byte after each prefix of null, true and false.
//   exhaustencode (encode record format):
//     - every rune 0..U+10FFFF, surrogates included as their 3-byte
//       encodings, as string values and as map keys, 256 per case;
//     - every 1- and 2-byte string, as values and as keys, and every
//       2-byte template.HTML.
//
// The cases are numbered; -parts P -part K keeps the cases whose number is
// K modulo P, so a run can be split into corpora of manageable size.

import (
	"fmt"
	"html/template"
	"io"
	"math/rand/v2"
	"strings"
	"unicode/utf8"
)

// exhaustAlphabet3 is the alphabet of the 3-byte inputs.
var exhaustAlphabet3 = []byte("{}[],:\" \t\n\r0123456789-+.eEtrufalsn\\/\x00\x1f\x7f\x80\xbf\xc2\xe2\xed\xef\xf0\xf4\xff")

func exhaustTexts(emit func(string)) {
	for c := 0; c < 256; c++ {
		emit(string([]byte{byte(c)}))
	}
	for c := 0; c < 1<<16; c++ {
		emit(string([]byte{byte(c >> 8), byte(c)}))
	}
	for _, a := range exhaustAlphabet3 {
		for _, b := range exhaustAlphabet3 {
			for _, c := range exhaustAlphabet3 {
				emit(string([]byte{a, b, c}))
			}
		}
	}

	// String bodies.
	for c := 0; c < 1<<16; c++ {
		emit(`"` + string([]byte{byte(c >> 8), byte(c)}) + `"`)
	}
	for c := 0; c < 256; c++ {
		emit(`"\` + string([]byte{byte(c)}) + `"`)
		emit(`{"\` + string([]byte{byte(c)}) + `":1}`)
	}
	for u := 0; u < 1<<16; u++ {
		h := fmt.Sprintf("%04x", u)
		if u%7 == 0 {
			h = strings.ToUpper(h)
		}
		emit(`"\u` + h + `"`)
	}
	for _, hi := range []string{"d800", "DBFF", "d83d"} {
		for lo := 0; lo < 1<<16; lo += 3 {
			emit(fmt.Sprintf(`["\u%s\u%04x"]`, hi, lo))
		}
		emit(`["\u` + hi + `\udfff"]`)
	}
	conts := []byte{0x00, 0x22, 0x5c, 0x7f, 0x80, 0x8f, 0x90, 0x9f, 0xa0, 0xbf, 0xc0, 0xff}
	for lead := 0xe0; lead <= 0xef; lead++ {
		for _, b := range conts {
			for _, c := range conts {
				emit(`"` + string([]byte{byte(lead), b, c}) + `"`)
			}
		}
	}
	for lead := 0xf0; lead <= 0xf7; lead++ {
		for _, b := range conts {
			for _, c := range conts {
				for _, d := range conts[3:9] {
					emit(`"` + string([]byte{byte(lead), b, c, d}) + `"`)
				}
			}
		}
	}

	// Number-like strings.
	numAlpha := []byte("0123456789-+.eE")
	var rec func(prefix []byte, left int, alpha []byte)
	rec = func(prefix []byte, left int, alpha []byte) {
		if len(prefix) > 0 {
			emit(string(prefix))
		}
		if left == 0 {
			return
		}
		for _, c := range alpha {
			rec(append(prefix, c), left-1, alpha)
		}
	}
	rec(nil, 4, numAlpha)
	five := []byte("01-+.eE9")
	for i := 0; i < 8*8*8*8*8; i++ {
		b := make([]byte, 5)
		for j, k := 0, i; j < 5; j, k = j+1, k/8 {
			b[j] = five[k%8]
		}
		emit(string(b))
	}

	// Literal prefixes followed by every byte.
	for _, lit := range []string{"null", "true", "false"} {
		for i := 0; i <= len(lit); i++ {
			for c := 0; c < 256; c++ {
				emit(lit[:i] + string([]byte{byte(c)}))
			}
		}
	}
}

func writeExhaustText(w io.Writer, r *rand.Rand, part, parts int) {
	advIndent = true
	i := 0
	exhaustTexts(func(s string) {
		if i%parts == part {
			textCase(w, r, []byte(s))
		}
		i++
	})
}

// surrogateString returns the 3-byte encoding of a surrogate code point
// (invalid UTF-8, which Go strings can hold).
func surrogateString(c int) string {
	return string([]byte{0xe0 | byte(c>>12), 0x80 | byte(c>>6)&0x3f, 0x80 | byte(c)&0x3f})
}

func exhaustValues(emit func(any)) {
	const batch = 256
	var ss []string
	flush := func() {
		if len(ss) == 0 {
			return
		}
		emit(ss)
		m := make(map[string]any, len(ss))
		for i, s := range ss {
			m[s] = i
		}
		emit(m)
		ss = nil
	}
	add := func(s string) {
		ss = append(ss, s)
		if len(ss) == batch {
			flush()
		}
	}
	for c := 0; c <= utf8.MaxRune; c++ {
		if 0xd800 <= c && c < 0xe000 {
			add(surrogateString(c))
		} else {
			add(string(rune(c)))
		}
	}
	flush()
	for c := 0; c < 256; c++ {
		add(string([]byte{byte(c)}))
	}
	flush()
	for c := 0; c < 1<<16; c++ {
		add(string([]byte{byte(c >> 8), byte(c)}))
	}
	flush()
	for c := 0; c < 1<<16; c += 256 {
		h := make([]any, 256)
		for k := range h {
			h[k] = template.HTML([]byte{byte((c + k) >> 8), byte(c + k)})
		}
		emit(h)
	}
}

func writeExhaustEncode(w io.Writer, r *rand.Rand, part, parts int) {
	advIndent = true
	i := 0
	exhaustValues(func(v any) {
		if i%parts == part {
			writeEncodeCase(w, r, v)
		}
		i++
	})
}
