package main

// More generators in the text record format (-mode numtext / bigtext),
// written by the second independent verifier of crates/go-json:
//
//   numtext: number literals with extreme shapes: up to 20000 mantissa
//     digits, runs of zeros and nines, halfway patterns, long fractions,
//     exponents with leading zeros and exponents near and past the
//     10000 cap of strconv's readFloat, alone, in arrays and in objects.
//   bigtext: documents of 4 KB to 400 KB (arrays of adversarial documents,
//     long strings with escapes at random offsets, deep nesting mixed with
//     long values), streamed with chunk sizes around the decoder's buffer
//     sizes (64, 4096, 8192 and their neighbours).
//   vartext: advtext inputs (and some bigtext documents) streamed through
//     readers whose read sizes follow a random cycle, recorded as a third
//     field of the streamargs record.

import (
	"io"
	"math/rand/v2"
	"strconv"
	"strings"
)

var numDigitCounts = []int{0, 1, 2, 15, 16, 17, 18, 19, 20, 21, 39, 40, 300, 767, 768, 769, 799, 800, 801, 1000, 5000, 20000}

func pickDigitCount(r *rand.Rand) int {
	n := numDigitCounts[r.IntN(len(numDigitCounts))]
	if n >= 5000 && r.IntN(4) != 0 {
		n = numDigitCounts[r.IntN(len(numDigitCounts)-2)]
	}
	return n
}

// digitRun returns n digits following one of a few patterns.
func digitRun(r *rand.Rand, n int) string {
	switch r.IntN(6) {
	case 0:
		return strings.Repeat("0", n)
	case 1:
		return strings.Repeat("9", n)
	case 2:
		if n == 0 {
			return ""
		}
		return "5" + strings.Repeat("0", n-1)
	case 3:
		if n == 0 {
			return ""
		}
		return strings.Repeat("0", n-1) + "1"
	default:
		return digitsN(r, n, false)
	}
}

var numExponents = []string{
	"0", "1", "9", "10", "22", "23", "300", "307", "308", "309", "310", "323", "324", "325", "326", "340", "400",
	"999", "1000", "9999", "10000", "10001", "19999", "20000", "20001", "20767", "99999", "100000",
	"2147483647", "2147483648", "9223372036854775807", "9223372036854775808", "18446744073709551616",
	"1000000000000000000000000000000",
}

func genExtremeNumber(r *rand.Rand) string {
	var b strings.Builder
	if r.IntN(3) == 0 {
		b.WriteByte('-')
	}
	if r.IntN(3) == 0 {
		b.WriteByte('0')
	} else {
		b.WriteByte(byte('1' + r.IntN(9)))
		b.WriteString(digitRun(r, pickDigitCount(r)))
	}
	if r.IntN(2) == 0 {
		b.WriteByte('.')
		b.WriteString(strings.Repeat("0", []int{0, 0, 1, 5, 300, 323, 324, 325, 400, 1000, 5000}[r.IntN(11)]))
		k := pickDigitCount(r)
		if k == 0 {
			k = 1
		}
		b.WriteString(digitRun(r, k))
	}
	if r.IntN(3) != 0 {
		b.WriteByte("eE"[r.IntN(2)])
		b.WriteString([]string{"", "+", "-", "-"}[r.IntN(4)])
		b.WriteString(strings.Repeat("0", []int{0, 0, 0, 1, 30}[r.IntN(5)]))
		b.WriteString(numExponents[r.IntN(len(numExponents))])
	}
	return b.String()
}

func writeNumText(w io.Writer, r *rand.Rand, n int) {
	advIndent = true
	for i := 0; i < n; i++ {
		var in string
		switch r.IntN(4) {
		case 0:
			in = genExtremeNumber(r)
		case 1:
			k := 1 + r.IntN(6)
			nums := make([]string, k)
			for j := range nums {
				nums[j] = genExtremeNumber(r)
			}
			in = "[" + strings.Join(nums, ",") + "]"
		case 2:
			k := 1 + r.IntN(4)
			var b strings.Builder
			b.WriteString("{")
			for j := 0; j < k; j++ {
				if j > 0 {
					b.WriteString(",")
				}
				b.WriteString(strconv.Quote(strconv.Itoa(j)) + ":" + genExtremeNumber(r))
			}
			b.WriteString("}")
			in = b.String()
		default:
			in = genExtremeNumber(r) + " " + genExtremeNumber(r)
		}
		textCase(w, r, []byte(in))
	}
}

// bigChunks makes textCase draw its streaming chunk size from
// bigChunkSizes (only -mode bigtext sets it, so the other modes consume
// the random stream exactly as before).
var bigChunks bool

var bigChunkSizes = []int{1, 7, 63, 64, 65, 127, 128, 1000, 2047, 2048, 2049, 4095, 4096, 4097, 8191, 8192, 8193, 65536, 1 << 20}

func genBigDoc(r *rand.Rand) string {
	target := 4096 + r.IntN(1<<(12+r.IntN(7)))
	var b strings.Builder
	switch r.IntN(4) {
	case 0:
		// An array of adversarial documents.
		b.WriteString("[")
		for b.Len() < target {
			if b.Len() > 1 {
				b.WriteString(ws(r) + "," + ws(r))
			}
			b.WriteString(genAdvDoc(r, 3, r.IntN(20) == 0))
		}
		b.WriteString("]")
	case 1:
		// One long string with escapes and multi-byte runes at random offsets.
		b.WriteString(`"`)
		for b.Len() < target {
			switch r.IntN(8) {
			case 0:
				b.WriteString(escUnit(r, advUnits[r.IntN(len(advUnits))]))
			case 1:
				b.WriteString(advJSONStringPieces[r.IntN(18)])
			case 2:
				b.WriteString("é😀 ")
			default:
				b.WriteString(strings.Repeat("x", r.IntN(200)))
			}
		}
		b.WriteString(`"`)
	case 2:
		// An object with many members and long names.
		b.WriteString("{")
		for i := 0; b.Len() < target; i++ {
			if i > 0 {
				b.WriteString(",")
			}
			b.WriteString(`"` + strings.Repeat("k", r.IntN(300)) + strconv.Itoa(i) + `":` + genAdvDoc(r, 2, false))
		}
		b.WriteString("}")
	default:
		// Deep nesting around long values, then a stream of more values.
		d := 1 + r.IntN(200)
		b.WriteString(strings.Repeat("[", d))
		for b.Len() < target {
			b.WriteString(genNumberLit(r) + ",")
		}
		b.WriteString("0" + strings.Repeat("]", d))
		for k := r.IntN(4); k > 0; k-- {
			b.WriteString(ws(r) + genAdvDoc(r, 2, false))
		}
	}
	s := b.String()
	if r.IntN(5) == 0 {
		s = string(mutate(r, []byte(s)))
	}
	if r.IntN(4) == 0 {
		s = s[:r.IntN(len(s)+1)]
	}
	return s
}

func writeBigText(w io.Writer, r *rand.Rand, n int) {
	advIndent = true
	bigChunks = true
	for i := 0; i < n; i++ {
		textCase(w, r, []byte(genBigDoc(r)))
	}
}

// varChunks makes textCase give its streaming readers a random cycle of
// read sizes (only -mode vartext sets it).
var varChunks bool

func varChunkSize(r *rand.Rand) int {
	if r.IntN(2) == 0 {
		return bigChunkSizes[r.IntN(len(bigChunkSizes))]
	}
	return 1 + r.IntN(100)
}

// writeVarText writes advtext inputs, and every 20th a bigtext document,
// streamed with a cycle of read sizes.
func writeVarText(w io.Writer, r *rand.Rand, n int) {
	advIndent = true
	varChunks = true
	for i := 0; i < n; i++ {
		in := genAdvTextInput(r)
		if r.IntN(20) == 0 {
			in = genBigDoc(r)
		}
		textCase(w, r, []byte(in))
	}
}
