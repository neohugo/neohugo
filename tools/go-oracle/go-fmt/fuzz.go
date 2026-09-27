package main

// Randomized (fuzz-style) differential vectors, written by -mode fuzz.
//
// Every generated operand is a value spec (spec.go), so the Rust side
// rebuilds exactly the same operand. Outputs that are not deterministic in
// Go (pointer addresses: the operand is decoded twice and formatted twice)
// are dropped.
//
// Files (all under -dir):
//   - fuzz_cases.txt: "fn \t format \t operands \t output [\t wrapped]" lines
//     (the cases.txt format, operands are inline specs): random nested
//     operands with random multi-directive formats, Sprint/Sprintln operand
//     lists and Errorf calls.
//   - fuzz_formats.txt + fuzz_matrix.txt: a dense single-operand format
//     matrix (flags x widths x precisions incl. values beyond fmt's 68-byte
//     intbuf) and, per random operand (floats, float32s, integers, runes,
//     strings, byte slices), the FNV-1a hash over all its outputs, as in
//     matrix.txt.

import (
	"bufio"
	"encoding/binary"
	"errors"
	"fmt"
	"hash/fnv"
	"log"
	"math"
	"math/rand/v2"
	"strconv"
	"strings"
)

type fuzzGen struct {
	r *rand.Rand
}

func (g *fuzzGen) pick(xs []string) string { return xs[g.r.IntN(len(xs))] }

// Random string pieces: ASCII, control characters, quoting-sensitive bytes,
// multi-byte UTF-8, non-printable and non-graphic runes, invalid UTF-8
// (lone continuation bytes, overlongs, surrogates, > U+10FFFF, truncated
// sequences).
var fuzzStrAtoms = []string{
	"a", "b", "Z", "0", "9", " ", "  ", "ab", "xyz", "Hello",
	"\t", "\n", "\r", "\x00", "\x01", "\x1b", "\x1f", "\x7f", "\a", "\b", "\f", "\v",
	"\x80", "\xbf", "\xff", "\xfe", "\xc0\x80", "\xc2", "\xe2\x82", "\xed\xa0\x80", "\xed\xbf\xbf",
	"\xf4\x90\x80\x80", "\xf0\x9f\x99", "\xf8\x88\x80\x80\x80",
	"\u00e9", "\u00c9", "\u00df", "\u65e5", "\u672c\u8a9e", "\u263a", "\U0001f642", "\U0001f1f9\U0001f1ed", "\u00ad", "\ufeff", "\u2028", "\u2029",
	"\u200b", "\u200d", "\U0010ffff", "\U000e0001", "\ue000", "\u0301", "\u0378", "\u00a0",
	"\u3000", "\ufffd", "\U0001f600", "\u061c", "\u0600", "\U000f0000", "\ufdd0",
	"\"", "'", "`", "\\", "%", "%d", "%!", "<", ">", "&", "{", "}", "[", "]", ":", ",",
}

func (g *fuzzGen) str() string {
	n := g.r.IntN(9)
	if g.r.IntN(12) == 0 {
		n += 10 + g.r.IntN(30)
	}
	var b strings.Builder
	for i := 0; i < n; i++ {
		b.WriteString(g.pick(fuzzStrAtoms))
	}
	return b.String()
}

// errStr is a random string for the message of an error operand. It is
// valid UTF-8: the value model's Object::go_error returns a Rust String.
func (g *fuzzGen) errStr() string {
	return strings.ToValidUTF8(g.str(), "\ufffd")
}

var intBounds = map[string][2]int64{
	"int8":  {math.MinInt8, math.MaxInt8},
	"int16": {math.MinInt16, math.MaxInt16},
	"int32": {math.MinInt32, math.MaxInt32},
	"int":   {math.MinInt64, math.MaxInt64},
	"int64": {math.MinInt64, math.MaxInt64},
}

var uintBits = map[string]uint{"uint8": 8, "uint16": 16, "uint32": 32, "uint": 64, "uint64": 64, "uintptr": 64}

// intVal returns a random value that fits the kind.
func (g *fuzzGen) intVal(kind string) int64 {
	b := intBounds[kind]
	switch g.r.IntN(8) {
	case 0:
		return b[0]
	case 1:
		return b[1]
	case 2:
		return int64(g.r.IntN(21)) - 10
	case 3:
		// Rune-ish values.
		cands := []int64{'a', 'x', '\u65e5', '\u263a', 0x7f, 0x80, 0xa0, 0xad, 0xd7ff, 0xd800, 0xdfff, 0xe000, 0xfffd, 0xfffe, 0x10ffff, 0x110000, 0x1f600, 0xe0001, -1}
		v := cands[g.r.IntN(len(cands))]
		if v < b[0] || v > b[1] {
			return 0
		}
		return v
	}
	// Random magnitude.
	bits := 1 + g.r.IntN(64)
	v := int64(g.r.Uint64() >> (64 - bits))
	if g.r.IntN(2) == 0 {
		v = -v
	}
	if v < b[0] {
		v = b[0]
	}
	if v > b[1] {
		v = b[1]
	}
	return v
}

func (g *fuzzGen) uintVal(kind string) uint64 {
	bits := uintBits[kind]
	max := uint64(math.MaxUint64) >> (64 - bits)
	switch g.r.IntN(6) {
	case 0:
		return max
	case 1:
		return uint64(g.r.IntN(20))
	case 2:
		cands := []uint64{'a', '\u65e5', 0xd800, 0x10ffff, 0x110000, 0x1f600, 1 << 63}
		v := cands[g.r.IntN(len(cands))]
		if v > max {
			return max
		}
		return v
	}
	nb := 1 + g.r.IntN(int(bits))
	return g.r.Uint64() >> (64 - nb)
}

// f64 returns a random float64 from several distributions.
func (g *fuzzGen) f64() float64 {
	switch g.r.IntN(10) {
	case 0:
		return math.Float64frombits(g.r.Uint64())
	case 1:
		specials := []float64{0, math.Copysign(0, -1), math.NaN(), math.Inf(1), math.Inf(-1), math.MaxFloat64, -math.MaxFloat64,
			math.SmallestNonzeroFloat64, 0x1p-1022, 0x1.fffffffffffffp-1023, 1e21, 1e20, 999999.5, 1e-4, 1e-5, 0.5, 1.5, 2.5, 1e23, 5e-324, 1 << 53, 1<<53 + 1}
		return specials[g.r.IntN(len(specials))]
	case 2:
		// Exact binary fractions (rounding ties in decimal).
		return float64(g.r.IntN(2001)-1000) / float64(uint64(1)<<g.r.IntN(12))
	case 3:
		// Integers.
		return float64(int64(g.r.Uint64() >> g.r.IntN(64)))
	case 4:
		// Short decimals: d.ddd x 10^e.
		digits := strconv.Itoa(g.r.IntN(100000))
		s := digits + "e" + strconv.Itoa(g.r.IntN(80)-40)
		f, _ := strconv.ParseFloat(s, 64)
		if g.r.IntN(2) == 0 {
			f = -f
		}
		return f
	case 5:
		// Values straddling the %e/%f switch of %g and the 1e21 boundary.
		e := g.r.IntN(50) - 25
		f := (1 + g.r.Float64()) * math.Pow(10, float64(e))
		return f
	case 6:
		// Near powers of ten.
		e := g.r.IntN(600) - 300
		f := math.Pow(10, float64(e))
		switch g.r.IntN(3) {
		case 0:
			return math.Nextafter(f, math.Inf(1))
		case 1:
			return math.Nextafter(f, math.Inf(-1))
		}
		return f
	case 7:
		// Subnormals.
		return math.Float64frombits(g.r.Uint64() >> (12 + g.r.IntN(52)))
	}
	return g.r.NormFloat64() * math.Pow(10, float64(g.r.IntN(40)-20))
}

func (g *fuzzGen) f32() float32 {
	switch g.r.IntN(6) {
	case 0:
		return math.Float32frombits(g.r.Uint32())
	case 1:
		specials := []float32{0, float32(math.Copysign(0, -1)), float32(math.NaN()), float32(math.Inf(1)), float32(math.Inf(-1)),
			math.MaxFloat32, math.SmallestNonzeroFloat32, 0x1p-126, 16777216, 16777217, 1e10, 0.1, 1.1, 3.4e38, 1e-45}
		return specials[g.r.IntN(len(specials))]
	case 2:
		return float32(g.r.IntN(2001)-1000) / float32(uint32(1)<<g.r.IntN(12))
	case 3:
		// Subnormals.
		return math.Float32frombits(g.r.Uint32() >> (9 + g.r.IntN(23)))
	}
	return float32(g.f64())
}

var fuzzZones = []string{"nil", "utc", "fixed=MST=-25200", "fixed=IST=19800", "fixed=X=-1", "fixed=LMT=-17762",
	"fixed=ABCDEFGH=50400", "fixed=+0545=20700", "zone=Asia/Bangkok", "zone=America/New_York", "zone=Europe/Berlin",
	"zone=Australia/Lord_Howe", "zone=Pacific/Chatham", "zone=America/Sao_Paulo", "zone=Asia/Tokyo", "zone=Europe/London"}

func (g *fuzzGen) timeSpec() string {
	var sec int64
	switch g.r.IntN(4) {
	case 0:
		sec = int64(g.r.IntN(4e9)) - 1e9
	case 1:
		sec = int64(g.r.Uint64()>>20) - 1<<43
	case 2:
		sec = []int64{-62135596800, -62135596801, 253402300799, 253402300800, 0, -1, 1790510400}[g.r.IntN(7)]
	default:
		sec = 1700000000 + int64(g.r.IntN(1e8))
	}
	nsec := int64(0)
	switch g.r.IntN(3) {
	case 0:
		nsec = int64(g.r.IntN(1e9))
	case 1:
		nsec = int64(g.r.IntN(1000)) * 1e6
	}
	return "time:" + strconv.FormatInt(sec, 10) + ";" + strconv.FormatInt(nsec, 10) + ";" + g.pick(fuzzZones)
}

var fuzzListTypes = []string{"[]interface {}", "[]interface {}", "[]interface {}", "[]string", "[]int", "[]int8", "[]int16",
	"[]int32", "[]int64", "[]uint", "[]uint8", "[]uint16", "[]uint32", "[]uint64", "[]uintptr", "[]float32", "[]float64",
	"[]bool", "[]map[string]interface {}", "[][]string", "[][]interface {}", "[][]uint8", "[]*main.Plain", "[]main.Iface",
	"[]*int", "[]template.HTML", "[]error", "[]fmt.Stringer", "main.NS"}

var fuzzMapTypes = []string{"map[string]interface {}", "map[string]interface {}", "map[string]string", "map[string]int",
	"map[string]int64", "map[string]float64", "map[string]bool", "map[string][]string", "map[string]uint8",
	"map[string]*main.Plain", "map[string]main.Iface", "map[string][]uint8", "maps.Params", "main.NM"}

// Typed nils whose kind the Rust side can recover from the type string.
var fuzzTnils = []string{"[]string", "[]interface {}", "[]int", "[]uint8", "[]float64", "map[string]interface {}",
	"map[string]string", "maps.Params", "*int", "*string", "*main.Plain", "func()", "chan int",
	"main.Iface", "error", "fmt.Stringer", "[][]string", "map[string]int", "*time.Location"}

// elem returns a random element spec for the element type of a container
// type (the Go element type as in the registry).
func (g *fuzzGen) elem(elemType string, depth int) string {
	switch elemType {
	case "interface {}":
		if g.r.IntN(10) == 0 {
			return "nil"
		}
		return g.value(depth)
	case "string":
		return sStr(g.str())
	case "int", "int8", "int16", "int32", "int64":
		return sInt(elemType, g.intVal(elemType))
	case "uint", "uint8", "uint16", "uint32", "uint64", "uintptr":
		return sUint(elemType, g.uintVal(elemType))
	case "float32":
		return sF32(g.f32())
	case "float64":
		return sF64(g.f64())
	case "bool":
		return []string{"bool:1", "bool:0"}[g.r.IntN(2)]
	case "map[string]interface {}":
		if g.r.IntN(5) == 0 {
			return "nil"
		}
		return g.mapSpec("map[string]interface {}", depth)
	case "[]string", "[]interface {}", "[]byte":
		if g.r.IntN(5) == 0 {
			return "nil"
		}
		if elemType == "[]byte" {
			return sBytes(g.str())
		}
		return g.listSpec(elemType, depth)
	case "*main.Plain", "main.Iface", "*int":
		return "nil"
	case "template.HTML":
		return sSafe("html", g.str())
	case "error":
		return []string{"nil", sObj("obj_err", g.errStr()), sObj("obj_both", g.errStr())}[g.r.IntN(3)]
	case "fmt.Stringer":
		return []string{"nil", sObj("obj_str", g.str()), sObj("obj_sv", g.str()), sObj("obj_both", g.errStr())}[g.r.IntN(4)]
	}
	log.Fatalf("fuzz: no element generator for %q", elemType)
	return ""
}

func listElemType(t string) string {
	switch t {
	case "main.NS":
		return "interface {}"
	case "[][]uint8":
		return "[]byte"
	}
	return strings.TrimPrefix(t, "[]")
}

func (g *fuzzGen) listSpec(t string, depth int) string {
	n := g.r.IntN(5)
	var es []string
	for i := 0; i < n; i++ {
		es = append(es, g.elem(listElemType(t), depth+1))
	}
	if t == "[]uint8" {
		return sBytes(g.str())
	}
	return sList(t, es...)
}

func mapElemType(t string) string {
	switch t {
	case "maps.Params", "main.NM":
		return "interface {}"
	case "map[string][]uint8":
		return "[]byte"
	}
	return strings.TrimPrefix(t, "map[string]")
}

func (g *fuzzGen) mapSpec(t string, depth int) string {
	n := g.r.IntN(5)
	var kvs []string
	seen := map[string]bool{}
	for i := 0; i < n; i++ {
		k := g.str()
		if seen[k] {
			continue
		}
		seen[k] = true
		kvs = append(kvs, k, g.elem(mapElemType(t), depth+1))
	}
	return sMap(t, kvs...)
}

// value returns a random operand spec. Deeper levels favour scalars.
func (g *fuzzGen) value(depth int) string {
	n := 27
	if depth >= 3 {
		n = 14
	}
	switch g.r.IntN(n) {
	case 0:
		return "nil"
	case 1:
		return []string{"bool:1", "bool:0"}[g.r.IntN(2)]
	case 2, 3:
		k := []string{"int", "int8", "int16", "int32", "int64"}[g.r.IntN(5)]
		return sInt(k, g.intVal(k))
	case 4:
		k := []string{"uint", "uint8", "uint16", "uint32", "uint64", "uintptr"}[g.r.IntN(6)]
		return sUint(k, g.uintVal(k))
	case 5, 6:
		return sF64(g.f64())
	case 7:
		return sF32(g.f32())
	case 8, 9:
		return sStr(g.str())
	case 10:
		return sSafe(g.pick([]string{"html", "htmlattr", "css", "js", "jsstr", "url", "srcset"}), g.str())
	case 11:
		return sBytes(g.str())
	case 12:
		return sTnil(g.pick(fuzzTnils))
	case 13:
		return g.timeSpec()
	case 14, 15, 16:
		return g.listSpec(g.pick(fuzzListTypes), depth)
	case 17, 18:
		return g.mapSpec(g.pick(fuzzMapTypes), depth)
	case 19:
		k := g.pick([]string{"obj_str", "obj_err", "obj_both", "obj_sv"})
		if k == "obj_err" || k == "obj_both" {
			return sObj(k, g.errStr())
		}
		return sObj(k, g.str())
	case 20, 21:
		name := "obj_plain"
		if depth == 0 && g.r.IntN(2) == 0 {
			name = "obj_pplain"
		}
		return name + "(" + g.elem("interface {}", depth+1) + "," + g.elem("interface {}", depth+1) + ")"
	case 22:
		n := g.r.IntN(4)
		var kvs []string
		seen := map[string]bool{}
		for i := 0; i < n; i++ {
			k := g.str()
			if seen[k] {
				continue
			}
			seen[k] = true
			kvs = append(kvs, k, g.elem("interface {}", depth+1))
		}
		return sObjNM(kvs...)
	case 23:
		n := g.r.IntN(4)
		var es []string
		for i := 0; i < n; i++ {
			es = append(es, g.elem("interface {}", depth+1))
		}
		return sObjNS(es...)
	case 24:
		// neohugo named collection types with a String method.
		switch g.r.IntN(4) {
		case 0:
			return "pages:" + strconv.Itoa(g.r.IntN(4))
		case 1:
			return "taxlist:" + strconv.Itoa(g.r.IntN(3))
		case 2:
			return sTnil("page.Pages")
		}
		return sTnil("page.TaxonomyList")
	case 25:
		return sObj("obj_gs", g.str())
	}
	return sStr(g.str())
}

var fuzzVerbs = []string{"v", "v", "v", "v", "s", "s", "d", "d", "q", "q", "x", "X", "t", "b", "o", "O", "c", "U",
	"e", "E", "f", "F", "g", "G", "T", "w", "%", "z", "\u00e9", "!", "\xff", "\u65e5"}

func (g *fuzzGen) width() string {
	switch g.r.IntN(10) {
	case 0, 1, 2, 3, 4:
		return ""
	case 5:
		return strconv.Itoa(40 + g.r.IntN(60))
	}
	return strconv.Itoa(g.r.IntN(25))
}

func (g *fuzzGen) format(nargs int) string {
	var b strings.Builder
	pieces := 1 + g.r.IntN(4)
	idx := func() string { return "[" + strconv.Itoa(g.r.IntN(nargs+2)) + "]" }
	for p := 0; p < pieces; p++ {
		if g.r.IntN(6) == 0 {
			b.WriteString(g.pick([]string{"ab", " ", "\u65e5", "%%", "\xff", "x=", "[", "]", "*", "\n"}))
			continue
		}
		b.WriteByte('%')
		for i := 0; i < 3; i++ {
			if g.r.IntN(3) == 0 {
				b.WriteByte("+-# 0"[g.r.IntN(5)])
			}
		}
		if g.r.IntN(8) == 0 {
			b.WriteString(idx())
		}
		switch g.r.IntN(8) {
		case 0:
			b.WriteByte('*')
		case 1:
			b.WriteString(idx() + "*")
		default:
			b.WriteString(g.width())
		}
		if g.r.IntN(3) == 0 {
			b.WriteByte('.')
			switch g.r.IntN(6) {
			case 0:
				b.WriteByte('*')
			case 1:
				b.WriteString(idx() + "*")
			case 2:
			default:
				b.WriteString(g.width())
			}
		}
		if g.r.IntN(10) == 0 {
			b.WriteString(idx())
		}
		if g.r.IntN(40) == 0 {
			break
		}
		b.WriteString(g.pick(fuzzVerbs))
	}
	return b.String()
}

var fuzzStarArgs = []string{"int:3", "int:-4", "int:0", "int:12", "int:-30", "int:70", "int:1000001", "int:-1000001", "uint:5",
	"uint8:2", "uint64:18446744073709551615", "int64:-9223372036854775808", "int32:7", "int8:-3", "uintptr:9", "str:35", "nil",
	"f64:4014000000000000", "bool:1"}

func (g *fuzzGen) args(n int, depth int) []string {
	var as []string
	for i := 0; i < n; i++ {
		if g.r.IntN(4) == 0 {
			as = append(as, g.pick(fuzzStarArgs))
		} else {
			as = append(as, g.value(depth))
		}
	}
	return as
}

func decodeSpecs(specs []string) []any {
	var vs []any
	for _, s := range specs {
		vs = append(vs, mustDecode(s))
	}
	return vs
}

// writeFuzzCases writes n Sprintf cases plus Sprint/Sprintln/Errorf cases.
func writeFuzzCases(w *bufio.Writer, seed uint64, n int, maxOut int) (int, int) {
	g := &fuzzGen{r: rand.New(rand.NewPCG(seed, seed^0x9e3779b97f4a7c15))}
	count, dropped := 0, 0
	emit := func(fn, format string, args []string, out string, extra string) {
		_, _ = fmt.Fprintf(w, "%s\t%s\t%s\t%s%s\n", fn, strconv.Quote(format), strings.Join(args, " "), strconv.Quote(out), extra)
		count++
	}
	for i := 0; i < n; i++ {
		switch i % 10 {
		case 0:
			args := g.args(g.r.IntN(5), 0)
			fn := "sprint"
			if i%20 == 0 {
				fn = "sprintln"
			}
			var o1, o2 string
			if fn == "sprint" {
				o1, o2 = fmt.Sprint(decodeSpecs(args)...), fmt.Sprint(decodeSpecs(args)...)
			} else {
				o1, o2 = fmt.Sprintln(decodeSpecs(args)...), fmt.Sprintln(decodeSpecs(args)...)
			}
			if o1 != o2 || len(o1) > maxOut {
				dropped++
				continue
			}
			emit(fn, "", args, o1, "")
		case 1:
			// Errorf: %w with errors, non-errors, reordering.
			nargs := g.r.IntN(4)
			var args []string
			for j := 0; j < nargs; j++ {
				if g.r.IntN(2) == 0 {
					args = append(args, sObj(g.pick([]string{"obj_err", "obj_both", "obj_str"}), g.errStr()))
				} else {
					args = append(args, g.value(1))
				}
			}
			format := g.format(nargs)
			if g.r.IntN(2) == 0 {
				format = strings.ReplaceAll(format, "v", "w")
			}
			if strings.Contains(format, "p") {
				dropped++
				continue
			}
			a1 := decodeSpecs(args)
			err := fmt.Errorf(format, a1...)
			e2 := fmt.Errorf(format, decodeSpecs(args)...)
			if err.Error() != e2.Error() || len(err.Error()) > maxOut {
				dropped++
				continue
			}
			var wrapped []error
			switch u := err.(type) {
			case interface{ Unwrap() []error }:
				wrapped = u.Unwrap()
			case interface{ Unwrap() error }:
				if e := u.Unwrap(); e != nil {
					wrapped = []error{e}
				}
			}
			var idx []string
			for _, we := range wrapped {
				for k, a := range a1 {
					if ae, ok := a.(error); ok && errors.Is(ae, we) && ae == we {
						idx = append(idx, strconv.Itoa(k))
						break
					}
				}
			}
			emit("errorf", format, args, err.Error(), "\t"+strings.Join(idx, ","))
		default:
			nargs := g.r.IntN(5)
			args := g.args(nargs, 0)
			format := g.format(nargs)
			if strings.Contains(format, "p") {
				dropped++
				continue
			}
			o1, o2 := fmt.Sprintf(format, decodeSpecs(args)...), fmt.Sprintf(format, decodeSpecs(args)...)
			if o1 != o2 || len(o1) > maxOut {
				dropped++
				continue
			}
			emit("sprintf", format, args, o1, "")
		}
	}
	return count, dropped
}

// fuzzFormatMatrix is the dense single-operand format list.
func fuzzFormatMatrix() []string {
	verbs := []string{"v", "d", "s", "q", "x", "X", "b", "o", "O", "c", "U", "e", "E", "f", "F", "g", "G", "t"}
	flagSets := []string{"", "+", "-", "#", " ", "0", "+0", "#0", "-0", "# ", "+#", "-#", " 0", "+-#0 "}
	widths := []string{"", "3", "9", "24", "66", "70"}
	precs := []string{"", ".", ".1", ".3", ".6", ".9", ".13", ".17", ".20", ".31", ".64", ".66"}
	var fs []string
	for _, v := range verbs {
		for _, fl := range flagSets {
			for _, w := range widths {
				for _, p := range precs {
					fs = append(fs, "%"+fl+w+p+v)
				}
			}
		}
	}
	// Big precisions and widths (exact decimal expansions, buffer growth).
	for _, v := range []string{"e", "f", "g", "x", "d", "b", "U", "s", "q", "X"} {
		for _, p := range []string{".100", ".330", ".767", ".1074", ".1100"} {
			fs = append(fs, "%"+p+v, "%#"+p+v, "%+0200"+p+v)
		}
		fs = append(fs, "%0300"+v, "%-300"+v, "%#0300"+v)
	}
	return fs
}

// fuzzMatrixKinds restricts the dense matrix operands: "" (all kinds),
// "float", "int" or "str".
var fuzzMatrixKinds = ""

// matrixOperand returns a random single operand for the dense matrix.
func (g *fuzzGen) matrixOperand() string {
	switch fuzzMatrixKinds {
	case "float":
		if g.r.IntN(2) == 0 {
			return sF32(g.f32())
		}
		return sF64(g.f64())
	case "int":
		if g.r.IntN(3) == 0 {
			k := []string{"uint", "uint8", "uint16", "uint32", "uint64", "uintptr"}[g.r.IntN(6)]
			return sUint(k, g.uintVal(k))
		}
		k := []string{"int", "int8", "int16", "int32", "int64"}[g.r.IntN(5)]
		return sInt(k, g.intVal(k))
	case "str":
		if g.r.IntN(3) == 0 {
			return sBytes(g.str())
		}
		return sStr(g.str())
	}
	switch g.r.IntN(12) {
	case 0, 1, 2:
		return sF64(g.f64())
	case 3, 4:
		return sF32(g.f32())
	case 5, 6:
		k := []string{"int", "int8", "int16", "int32", "int64"}[g.r.IntN(5)]
		return sInt(k, g.intVal(k))
	case 7:
		k := []string{"uint", "uint8", "uint16", "uint32", "uint64", "uintptr"}[g.r.IntN(6)]
		return sUint(k, g.uintVal(k))
	case 8, 9:
		return sStr(g.str())
	case 10:
		return sBytes(g.str())
	}
	return g.listSpec(g.pick([]string{"[]float64", "[]float32", "[]int", "[]string", "[]interface {}"}), 2)
}

func writeFuzzMatrix(dir string, seed uint64, n int) {
	formats := fuzzFormatMatrix()
	f, w := create(dir, "fuzz_formats.txt")
	for _, s := range formats {
		_, _ = fmt.Fprintln(w, strconv.Quote(s))
	}
	finish(f, w)

	g := &fuzzGen{r: rand.New(rand.NewPCG(seed, seed*3+1))}
	f, w = create(dir, "fuzz_matrix.txt")
	total := 0
	for i := 0; i < n; i++ {
		var spec string
		for {
			spec = g.matrixOperand()
			if !strings.HasPrefix(spec, "list:") || !strings.Contains(spec, "obj_pplain") {
				break
			}
		}
		v1, v2 := mustDecode(spec), mustDecode(spec)
		h := fnv.New64a()
		var nondet []int
		for fi, format := range formats {
			o1 := fmt.Sprintf(format, v1)
			if o2 := fmt.Sprintf(format, v2); o1 != o2 {
				nondet = append(nondet, fi)
				continue
			}
			var nb [8]byte
			binary.LittleEndian.PutUint64(nb[:], uint64(len(o1)))
			h.Write(nb[:])
			h.Write([]byte(o1))
			total++
		}
		_, _ = fmt.Fprintf(w, "%s\t%016x\t%s\n", spec, h.Sum64(), ranges(nondet))
	}
	finish(f, w)
	log.Printf("fuzz matrix: %d operands x %d formats: %d outputs hashed", n, len(formats), total)
}

func writeFuzz(dir string, seed uint64, nCases, nMatrix, maxOut int) {
	f, w := create(dir, "fuzz_cases.txt")
	count, dropped := writeFuzzCases(w, seed, nCases, maxOut)
	finish(f, w)
	log.Printf("fuzz cases: %d written, %d dropped (nondeterministic, %%p or longer than %d bytes)", count, dropped, maxOut)
	writeFuzzMatrix(dir, seed, nMatrix)
}

// writeModelCases writes model_cases.txt: hand-picked operands for the
// value-model mappings (named collection types with String methods, typed
// nils of interface types inside containers, GoStringer host objects, nil
// *time.Location, time.Time struct fields) under a fixed verb list.
func writeModelCases(dir string) int {
	operands := [][]string{
		{"pages:0"}, {"pages:3"}, {"taxlist:0"}, {"taxlist:2"},
		{sTnil("page.Pages")}, {sTnil("page.TaxonomyList")},
		{sList("[]interface {}", "pages:2", "taxlist:1", sTnil("page.Pages"))},
		{sMap("map[string]interface {}", "p", "pages:1", "t", sTnil("page.TaxonomyList"))},
		{"obj_plain(pages:2," + sTnil("page.TaxonomyList") + ")"},
		{sList("[]interface {}", sTnil("error"), sTnil("main.Iface"), sTnil("fmt.Stringer"), "nil")},
		{sList("[]error", sTnil("error"), "nil")},
		{sList("[]fmt.Stringer", sTnil("fmt.Stringer"), sTnil("error"))},
		{sMap("map[string]interface {}", "e", sTnil("error"), "i", sTnil("main.Iface"))},
		{sMap("map[string]main.Iface", "e", sTnil("main.Iface"))},
		{"obj_plain(" + sTnil("error") + "," + sTnil("main.Iface") + ")"},
		{sObj("obj_gs", "g\"s")},
		{sList("[]interface {}", sObj("obj_gs", "a"), sList("[]interface {}", sObj("obj_gs", "b")))},
		{sMap("map[string]interface {}", "k", sObj("obj_gs", "v"))},
		{"obj_plain(" + sObj("obj_gs", "f") + ",nil)"},
		{sTnil("*time.Location")},
		{sList("[]interface {}", sTnil("*time.Location"))},
		{"time:1790510400;120000000;utc"}, {"time:1790510400;7;nil"},
		{sList("[]interface {}", "time:0;0;utc")},
		{"obj_plain(time:1;2;utc,nil)"},
		{"pages:2", "taxlist:1", sStr("s"), "pages:0"},
	}
	verbs := []string{"%v", "%+v", "%#v", "%s", "%q", "%x", "%X", "%d", "%t", "%10v", "%-12s", "%.3v", "%T", "%w"}
	f, w := create(dir, "model_cases.txt")
	n := 0
	emit := func(fn, format string, args []string, out, extra string) {
		_, _ = fmt.Fprintf(w, "%s\t%s\t%s\t%s%s\n", fn, strconv.Quote(format), strings.Join(args, " "), strconv.Quote(out), extra)
		n++
	}
	for _, args := range operands {
		for _, verb := range verbs {
			format := strings.Repeat(verb+"|", len(args))
			o1 := fmt.Sprintf(format, decodeSpecs(args)...)
			if o2 := fmt.Sprintf(format, decodeSpecs(args)...); o1 != o2 {
				continue
			}
			emit("sprintf", format, args, o1, "")
		}
		emit("sprint", "", args, fmt.Sprint(decodeSpecs(args)...), "")
		emit("sprintln", "", args, fmt.Sprintln(decodeSpecs(args)...), "")
	}
	// Malformed argument indexes, flags after widths, stars in odd places.
	weird := []string{"%[01]d", "%[+1]d", "%[ 1]d", "%[1", "%[]d", "%[-1]d", "%[99999999999]d", "%[1]]d",
		"%[2]*[1]d", "%[3]*.[2]*[1]f", "%.[2]*d", "%[1]*.[1]*d|%d", "%-*d", "%0-5d|%-05d", "%5-d", "%.5.3d",
		"%*.*.*d", "%[1]%", "%[5]%", "%[0]%", "%.%", "%5%", "%-%", "%[1]*%", "%!", "%\\x00", "%\x00d", "%[",
		"%[1]", "%.[", "%.[1", "%.*[1]d", "%[1][2]d", "%[2][1]d", "%3[2]d", "%[2]3d", "%.[2]3d", "%#[2]x",
		"%+[2]q", "%v%[1]v%v%v", "%[3]v%v%v", "%[2]v%[2]v%[2]v", "%10.[2]v", "%[1]10v", "%1$d", "%*[2]d"}
	args := []string{sInt("int", 5), sStr("ab"), sInt("int", -3)}
	for _, format := range weird {
		emit("sprintf", format, args, fmt.Sprintf(format, decodeSpecs(args)...), "")
		emit("sprintf", format, args[:1], fmt.Sprintf(format, decodeSpecs(args[:1])...), "")
		emit("sprintf", format, nil, fmt.Sprintf(format, decodeSpecs(nil)...), "")
	}
	finish(f, w)
	return n
}
