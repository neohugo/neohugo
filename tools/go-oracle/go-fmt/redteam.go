package main

// Red-team vectors (third pass), written by -mode redteam and -mode
// flagperm into the fuzz_*.txt file names, so crates/go-fmt/tests/fuzz.rs
// runs them unchanged (GO_FMT_FUZZ_DIR=<dir>).
//
// -mode redteam (per -seed):
//   - fuzz_cases.txt: -n cases, a third Sprint/Sprintln (the gotemplate
//     print/println spacing rule), a tenth Errorf, the rest Sprintf with
//     formats of 1-6 pieces: 0-8 flags in any order (duplicated), widths
//     with leading zeros, star and indexed-star widths/precisions, good,
//     zero, negative, out-of-range and malformed argument indexes, 9-13
//     digit widths (parsenum overflow), truncated UTF-8 and control-byte
//     verbs; operands nested up to -depth levels (default 6, at most
//     rtMaxLeaves leaves) including the nil-receiver hook types below,
//     named slice/map types with String and Error methods, named basic
//     types (nb nodes), and times in fixed zones with arbitrary valid UTF-8
//     names; star values at fmt's 1e6 limit only with scalar operands;
//   - fuzz_formats.txt + fuzz_matrix.txt: formatMatrix() (25 verbs incl.
//     %T %p %w and bad verbs x 32 flag sets x 4 widths x 6 precisions,
//     plus the malformed formats) over -nmatrix random nested operands.
//
// -mode flagperm: every flag string of length 0-4 over "+-# 0" (781, in
// every order and duplicated) x 25 verbs x widths {"", 7} x precisions
// {"", .3} (78,100 formats) over valueCorpus() plus rtExtraCorpus().

import (
	"bufio"
	"encoding/binary"
	"fmt"
	"hash/fnv"
	"log"
	"math/rand/v2"
	"os"
	"reflect"
	"strconv"
	"strings"
	"time"
)

// NilPanicStr is a pointer-receiver Stringer that dereferences its
// receiver: String on a nil *NilPanicStr panics and fmt's catchPanic
// prints "<nil>".
type NilPanicStr struct{ S string }

func (p *NilPanicStr) String() string { return p.S }

// NilOKStr is a pointer-receiver Stringer that handles a nil receiver.
type NilOKStr struct{ S string }

func (p *NilOKStr) String() string {
	if p == nil {
		return "nil-ok"
	}
	return "OK(" + p.S + ")"
}

// NilPanicErr is a pointer-receiver error that panics on a nil receiver.
type NilPanicErr struct{ Msg string }

func (p *NilPanicErr) Error() string { return p.Msg }

// NilOKErr is a pointer-receiver error that handles a nil receiver.
type NilOKErr struct{ Msg string }

func (p *NilOKErr) Error() string {
	if p == nil {
		return "nil-err"
	}
	return "E(" + p.Msg + ")"
}

// SS is a named slice type with a value-receiver String method whose
// result needs quoting.
type SS []any

func (s SS) String() string { return "SS<" + strconv.Itoa(len(s)) + "\xff\"\t>" }

// SM is a named map type with a value-receiver Error method.
type SM map[string]any

func (m SM) Error() string { return "SM(" + strconv.Itoa(len(m)) + ")" }

// Named basic types (Rust: objects with Object::underlying), with and
// without methods, next to time.Month, time.Weekday and time.Duration.

// HStr is a named string type with a String method (like hstring.HTML).
type HStr string

func (s HStr) String() string { return string(s) }

// Celsius is a named float64 type with a String method.
type Celsius float64

func (c Celsius) String() string { return fmt.Sprintf("%.1f\u00b0C", float64(c)) }

// NStrErr is a named string type with an Error method.
type NStrErr string

func (e NStrErr) Error() string { return "NE:" + string(e) }

// NInt, NUint, NF32, NBool and NStr are named basic types without methods.
type (
	NInt  int8
	NUint uint16
	NF32  float32
	NBool bool
	NStr  string
)

func init() {
	reg(
		(*[]*NilPanicStr)(nil), (*[]*NilOKErr)(nil), (**NilPanicStr)(nil), (**NilOKStr)(nil),
		(**NilPanicErr)(nil), (**NilOKErr)(nil), (*SS)(nil), (*SM)(nil), (*[]SS)(nil),
		(*map[string]fmt.Stringer)(nil), (*map[string]error)(nil), (*map[string]SM)(nil),
		(*time.Month)(nil), (*time.Weekday)(nil), (*time.Duration)(nil), (*HStr)(nil), (*Celsius)(nil),
		(*NStrErr)(nil), (*NInt)(nil), (*NUint)(nil), (*NF32)(nil), (*NBool)(nil), (*NStr)(nil),
		(*[]time.Month)(nil), (*[]HStr)(nil),
	)
	decodeHook = decodeRedTeam
}

// decodeRedTeam decodes the spec nodes added by the red-team pass.
func decodeRedTeam(n *specNode) (any, bool, error) {
	switch n.name {
	case "obj_nps", "obj_nos", "obj_npe", "obj_noe":
		s, err := unhex(n.payload)
		if err != nil {
			return nil, true, err
		}
		switch n.name {
		case "obj_nps":
			return &NilPanicStr{s}, true, nil
		case "obj_nos":
			return &NilOKStr{s}, true, nil
		case "obj_npe":
			return &NilPanicErr{s}, true, nil
		}
		return &NilOKErr{s}, true, nil
	case "nb":
		// nb:HEXTYPE(child): the child converted to a named basic type.
		t, err := regType(n.payload)
		if err != nil {
			return nil, true, err
		}
		if len(n.children) != 1 {
			return nil, true, fmt.Errorf("nb needs 1 child")
		}
		c, err := decodeNode(n.children[0])
		if err != nil {
			return nil, true, err
		}
		cv := reflect.ValueOf(c)
		if cv.Kind() != t.Kind() {
			return nil, true, fmt.Errorf("nb: %s of %s", t, cv.Type())
		}
		return cv.Convert(t).Interface(), true, nil
	case "nest":
		v, err := decodeNest(n)
		return v, true, err
	}
	return nil, false, nil
}

// sNB builds a named-basic-type node.
func sNB(t, child string) string { return "nb:" + hexs(t) + "(" + child + ")" }

// rtNamedBasic returns a value of a named basic type.
func (g *fuzzGen) rtNamedBasic() string {
	switch g.r.IntN(11) {
	case 0:
		return sNB("time.Month", sInt("int", g.pick64([]int64{-1, 0, 1, 9, 12, 13}, g.intVal("int"))))
	case 1:
		return sNB("time.Weekday", sInt("int", g.pick64([]int64{-1, 0, 3, 6, 7}, g.intVal("int"))))
	case 2:
		d := g.intVal("int64")
		if g.r.IntN(2) == 0 {
			d = int64(g.r.IntN(2e6)-1e6) * []int64{1, 1e3, 1e6, 1e9, 60e9}[g.r.IntN(5)]
		}
		return sNB("time.Duration", sInt("int64", d))
	case 3:
		return sNB("main.HStr", sStr(g.rtStr()))
	case 4:
		return sNB("main.Celsius", sF64(g.f64()))
	case 5:
		return sNB("main.NStrErr", sStr(g.errStr()))
	case 6:
		return sNB("main.NInt", sInt("int8", g.intVal("int8")))
	case 7:
		return sNB("main.NUint", sUint("uint16", g.uintVal("uint16")))
	case 8:
		return sNB("main.NF32", sF32(g.f32()))
	case 9:
		return sNB("main.NBool", g.pick([]string{"bool:1", "bool:0"}))
	}
	return sNB("main.NStr", sStr(g.rtStr()))
}

// pick64 returns one of xs, or other one time in three.
func (g *fuzzGen) pick64(xs []int64, other int64) int64 {
	if g.r.IntN(3) == 0 {
		return other
	}
	return xs[g.r.IntN(len(xs))]
}

// Random zone names for "fixedhex" times: valid UTF-8 (the Rust
// Location name is a String) with quoting-sensitive characters.
var rtZoneAtoms = []string{"A", "Z", "x", "+", "-", "0", "9", "_", "/", "\"", "\\", "`", "'", "\t", "\n", "\x00", "\x7f",
	"\u00e9", "\u65e5", "\U0001f600", "\u200b", "\u00ad", "\ufffd", "%", "(", ")", ",", ";", "=", " "}

func (g *fuzzGen) rtTime() string {
	if g.r.IntN(3) != 0 {
		return g.timeSpec()
	}
	var name strings.Builder
	for i := g.r.IntN(6); i > 0; i-- {
		name.WriteString(g.pick(rtZoneAtoms))
	}
	off := g.r.IntN(2*86400) - 86400
	if name.Len() == 0 && off%3600 == 0 {
		// time.FixedZone("", whole hours) is a shared cached pointer, so
		// the time.Time{loc} field prints a deterministic Go address.
		off++
	}
	sec := int64(g.r.IntN(4e9)) - 1e9
	return "time:" + strconv.FormatInt(sec, 10) + ";" + strconv.Itoa(g.r.IntN(1e9)) + ";fixedhex=" + hexs(name.String()) + "=" + strconv.Itoa(off)
}

// rtStr is a random string: the fuzz atoms, or fully random bytes, or
// random runes from the whole code space (surrogates and out-of-range
// values encode as U+FFFD).
func (g *fuzzGen) rtStr() string {
	switch g.r.IntN(5) {
	case 0:
		b := make([]byte, g.r.IntN(12))
		for i := range b {
			b[i] = byte(g.r.IntN(256))
		}
		return string(b)
	case 1:
		var b strings.Builder
		for i := g.r.IntN(8); i > 0; i-- {
			switch g.r.IntN(4) {
			case 0:
				b.WriteRune(rune(g.r.IntN(0x110000)))
			case 1:
				b.WriteRune(rune(g.r.IntN(0x3000)))
			case 2:
				b.WriteRune(rune(0xe0000 + g.r.IntN(0x200)))
			default:
				b.WriteRune(rune(g.r.IntN(0x80)))
			}
		}
		return b.String()
	}
	return g.str()
}

var rtErrKinds = []string{"obj_err", "obj_both", "obj_npe", "obj_noe"}
var rtStrKinds = []string{"obj_str", "obj_sv", "obj_both", "obj_gs", "obj_nps", "obj_nos"}
var rtNilHooks = []string{"*main.NilPanicStr", "*main.NilOKStr", "*main.NilPanicErr", "*main.NilOKErr"}

// rtHook returns a hook-implementing operand.
func (g *fuzzGen) rtHook() string {
	switch g.r.IntN(4) {
	case 0:
		return sTnil(g.pick(rtNilHooks))
	case 1:
		k := g.pick(rtErrKinds)
		return sObj(k, g.errStr())
	}
	k := g.pick(rtStrKinds)
	if k == "obj_both" {
		return sObj(k, g.errStr())
	}
	return sObj(k, g.rtStr())
}

var rtListTypes = []string{"[]interface {}", "[]interface {}", "main.NS", "main.SS", "[]fmt.Stringer", "[]error",
	"[]*main.NilPanicStr", "[]*main.NilOKErr", "[][]interface {}", "[]main.SS", "[]string", "[]uint8", "[]float32", "[]int8"}
var rtMapTypes = []string{"map[string]interface {}", "map[string]interface {}", "maps.Params", "main.NM", "main.SM",
	"map[string]fmt.Stringer", "map[string]error", "map[string]main.SM", "map[string]string"}

// rtElem returns an element spec for a container element type.
func (g *fuzzGen) rtElem(t string, depth int) string {
	switch t {
	case "interface {}":
		if g.r.IntN(8) == 0 {
			return "nil"
		}
		return g.rtValue(depth)
	case "fmt.Stringer":
		if g.r.IntN(6) == 0 {
			return "nil"
		}
		return g.pick([]string{sObj(g.pick(rtStrKinds), g.errStr()), sTnil("*main.NilPanicStr"), sTnil("*main.NilOKStr"), "list:" + hexs("main.SS") + "()"})
	case "error":
		if g.r.IntN(6) == 0 {
			return "nil"
		}
		return g.pick([]string{sObj(g.pick(rtErrKinds), g.errStr()), sTnil("*main.NilPanicErr"), sTnil("*main.NilOKErr"), sMap("main.SM")})
	case "*main.NilPanicStr":
		if g.r.IntN(2) == 0 {
			return "nil"
		}
		return sObj("obj_nps", g.rtStr())
	case "*main.NilOKErr":
		if g.r.IntN(2) == 0 {
			return "nil"
		}
		return sObj("obj_noe", g.errStr())
	case "[]interface {}", "main.SS":
		if g.r.IntN(4) == 0 {
			return "nil"
		}
		return g.rtList(t, depth)
	case "main.SM":
		if g.r.IntN(4) == 0 {
			return "nil"
		}
		return g.rtMap(t, depth)
	case "string":
		return sStr(g.rtStr())
	}
	return g.elem(t, depth)
}

func rtListElem(t string) string {
	switch t {
	case "main.NS", "main.SS":
		return "interface {}"
	}
	return strings.TrimPrefix(t, "[]")
}

func (g *fuzzGen) rtList(t string, depth int) string {
	if t == "[]uint8" {
		return sBytes(g.rtStr())
	}
	var es []string
	for i := g.r.IntN(5); i > 0; i-- {
		es = append(es, g.rtElem(rtListElem(t), depth+1))
	}
	return sList(t, es...)
}

func rtMapElem(t string) string {
	switch t {
	case "maps.Params", "main.NM", "main.SM":
		return "interface {}"
	}
	return strings.TrimPrefix(t, "map[string]")
}

func (g *fuzzGen) rtMap(t string, depth int) string {
	var kvs []string
	seen := map[string]bool{}
	for i := g.r.IntN(5); i > 0; i-- {
		k := g.rtStr()
		if seen[k] {
			continue
		}
		seen[k] = true
		kvs = append(kvs, k, g.rtElem(rtMapElem(t), depth+1))
	}
	return sMap(t, kvs...)
}

// rtMaxDepth bounds the nesting of rtValue.
var rtMaxDepth = 6

// rtBudget bounds the number of rtValue calls of one operand (-1: no
// bound); once it is spent, rtValue returns scalars.
var rtBudget = -1

// rtValue returns a random operand spec, nested up to rtMaxDepth levels.
func (g *fuzzGen) rtValue(depth int) string {
	n := 24
	if depth >= rtMaxDepth || rtBudget == 0 {
		n = 8
	}
	if rtBudget > 0 {
		rtBudget--
	}
	switch g.r.IntN(n) {
	case 0, 1, 2, 3:
		// Scalars, typed nils and times of the fuzz generator.
		return g.value(99)
	case 4:
		return sStr(g.rtStr())
	case 5:
		return g.rtHook()
	case 6:
		return g.rtTime()
	case 7:
		return sSafe(g.pick([]string{"html", "htmlattr", "css", "js", "jsstr", "url", "srcset"}), g.rtStr())
	case 8, 9, 10, 11:
		return g.rtList(g.pick(rtListTypes), depth)
	case 12, 13, 14:
		return g.rtMap(g.pick(rtMapTypes), depth)
	case 15, 16:
		name := "obj_plain"
		if depth == 0 && g.r.IntN(2) == 0 {
			name = "obj_pplain"
		}
		return name + "(" + g.rtElem("interface {}", depth+1) + "," + g.rtElem("interface {}", depth+1) + ")"
	case 17:
		var es []string
		for i := g.r.IntN(4); i > 0; i-- {
			es = append(es, g.rtElem("interface {}", depth+1))
		}
		return sObjNS(es...)
	case 18:
		var kvs []string
		seen := map[string]bool{}
		for i := g.r.IntN(4); i > 0; i-- {
			k := g.rtStr()
			if !seen[k] {
				seen[k] = true
				kvs = append(kvs, k, g.rtElem("interface {}", depth+1))
			}
		}
		return sObjNM(kvs...)
	case 19:
		return g.pick([]string{"pages:" + strconv.Itoa(g.r.IntN(4)), "taxlist:" + strconv.Itoa(g.r.IntN(3)),
			sTnil("page.Pages"), sTnil("page.TaxonomyList"), sTnil("main.SS"), sTnil("main.SM"), sTnil("*time.Location")})
	case 20:
		return g.rtNamedBasic()
	case 21:
		var es []string
		t := g.pick([]string{"[]time.Month", "[]main.HStr"})
		for i := g.r.IntN(4); i > 0; i-- {
			if t == "[]time.Month" {
				es = append(es, sNB("time.Month", sInt("int", int64(g.r.IntN(15)-1))))
			} else {
				es = append(es, sNB("main.HStr", sStr(g.rtStr())))
			}
		}
		return sList(t, es...)
	}
	return g.value(depth)
}

// Format generation.

var rtVerbs = []string{"v", "v", "v", "v", "s", "s", "s", "d", "d", "q", "q", "x", "X", "t", "b", "o", "O", "c", "U",
	"e", "E", "f", "F", "g", "G", "T", "T", "w", "%", "z", "y", "a", "h", "i", "j", "k", "l", "m", "n", "r", "u", "A", "B",
	"D", "H", "S", "V", "Z", "!", "#", "$", "&", "(", ")", ",", ".", "[", "]", "{", "}", "~", "\"", "'", "/", ":", ";",
	"<", "=", ">", "?", "@", "^", "_", "`", "|", "\\", "\x00", "\x01", "\x1f", "\x7f", "\u00e9", "\u65e5",
	"\U0001f600", "\xff", "\x80", "\xe6\x97", "\xf0\x9f", "\xed\xa0\x80", "\u2028"}

func (g *fuzzGen) rtIndex(nargs int) string {
	switch g.r.IntN(12) {
	case 0:
		return "[0]"
	case 1:
		return g.pick([]string{"[", "[]", "[x]", "[-1]", "[+1]", "[01]", "[1", "[ 1]", "[1]]", "[99999999999]", "[2*]", "[.1]"})
	case 2:
		return "[" + strconv.Itoa(nargs+1+g.r.IntN(3)) + "]"
	}
	return "[" + strconv.Itoa(1+g.r.IntN(nargs+1)) + "]"
}

func (g *fuzzGen) rtNum() string {
	switch g.r.IntN(14) {
	case 0:
		return "0" + strconv.Itoa(g.r.IntN(30))
	case 1:
		return strconv.Itoa(30 + g.r.IntN(90))
	case 2:
		// Too large: parsenum gives up and swallows the rest of the format.
		return g.pick([]string{"100000000", "999999999", "1000000000000"})
	case 3:
		return "00"
	}
	return strconv.Itoa(g.r.IntN(25))
}

// rtFormat returns a random format with 1-6 pieces.
func (g *fuzzGen) rtFormat(nargs int) string {
	var b strings.Builder
	for p := 1 + g.r.IntN(6); p > 0; p-- {
		if g.r.IntN(7) == 0 {
			b.WriteString(g.pick([]string{"ab", " ", "\u65e5", "%%", "%5%", "%-%", "%[1]%", "%.%", "\xff", "\xe6\x97", "x=",
				"[", "]", "*", "\n", "%!", "%\x00", "\x00", "%%%%"}))
			continue
		}
		b.WriteByte('%')
		nf := 0
		switch g.r.IntN(4) {
		case 0:
			nf = 1 + g.r.IntN(8)
		case 1:
			nf = g.r.IntN(3)
		}
		for i := 0; i < nf; i++ {
			b.WriteByte("+-# 0"[g.r.IntN(5)])
		}
		if g.r.IntN(7) == 0 {
			b.WriteString(g.rtIndex(nargs))
		}
		switch g.r.IntN(9) {
		case 0:
			b.WriteByte('*')
		case 1:
			b.WriteString(g.rtIndex(nargs) + "*")
		case 2, 3, 4:
			b.WriteString(g.rtNum())
		}
		if g.r.IntN(3) == 0 {
			b.WriteByte('.')
			switch g.r.IntN(7) {
			case 0:
				b.WriteByte('*')
			case 1:
				b.WriteString(g.rtIndex(nargs) + "*")
			case 2:
				b.WriteString(g.rtIndex(nargs))
			case 3:
			default:
				b.WriteString(g.rtNum())
			}
		}
		if g.r.IntN(8) == 0 {
			b.WriteString(g.rtIndex(nargs))
		}
		if g.r.IntN(60) == 0 {
			break
		}
		b.WriteString(g.pick(rtVerbs))
	}
	return b.String()
}

var rtStarArgs = []string{"int:3", "int:-4", "int:0", "int:12", "int:-30", "int:70", "int:1000001", "uint:5", "uint8:2",
	"uint16:300", "uint64:18446744073709551615", "uint64:9223372036854775808", "int64:-9223372036854775808", "int32:7",
	"int8:-3", "int16:-32768", "uintptr:9", "str:35", "nil", "f64:4014000000000000", "f32:40400000", "bool:1",
	"tnil:2a696e74", "html:35"}

// rtHugeStarArgs are widths and precisions at fmt's 1e6 limit. A width
// applies to every element of a container, so cases that use them only
// have scalar operands.
var rtHugeStarArgs = []string{"int:1000000", "int:-1000000", "uint64:1000000", "int64:999999", "int16:32767", "uint16:65535"}

// rtMaxLeaves bounds the number of scalar leaves of an operand (a width of
// n pads every leaf: see rtHugeStarArgs and formatMatrix's %1000001v).
const rtMaxLeaves = 200

// leafCount counts the scalar leaves of a decoded operand, up to limit+1.
func leafCount(v reflect.Value, limit int) int {
	switch v.Kind() {
	case reflect.Interface, reflect.Pointer:
		if v.IsNil() {
			return 1
		}
		return leafCount(v.Elem(), limit)
	case reflect.Slice:
		if v.Type().Elem().Kind() == reflect.Uint8 {
			return v.Len() + 1
		}
		n := 1
		for i := 0; i < v.Len() && n <= limit; i++ {
			n += leafCount(v.Index(i), limit)
		}
		return n
	case reflect.Map:
		n := 1
		for it := v.MapRange(); it.Next() && n <= limit; {
			n += 1 + leafCount(it.Value(), limit)
		}
		return n
	case reflect.Struct:
		n := 1
		for i := 0; i < v.NumField() && n <= limit; i++ {
			n += leafCount(v.Field(i), limit)
		}
		return n
	}
	return 1
}

// rtBoundedValue is rtValue(0) with at most limit leaves.
func (g *fuzzGen) rtBoundedValue(limit int) string {
	for {
		rtBudget = 2 * limit
		spec := g.rtValue(0)
		rtBudget = -1
		if leafCount(reflect.ValueOf(mustDecode(spec)), limit) <= limit {
			return spec
		}
	}
}

func (g *fuzzGen) rtArgs(n int) []string {
	var as []string
	if g.r.IntN(25) == 0 {
		// Huge widths and precisions with scalar operands only.
		for i := 0; i < n; i++ {
			if g.r.IntN(2) == 0 {
				as = append(as, g.pick(rtHugeStarArgs))
				continue
			}
			for {
				// A scalar: not a []byte, whose bytes are padded one by
				// one, nor a time.Time, whose bad-verb form prints its
				// *time.Location's fields (hundreds of transitions).
				if v := g.value(99); !strings.HasPrefix(v, "bytes:") && !strings.HasPrefix(v, "time:") {
					as = append(as, v)
					break
				}
			}
		}
		return as
	}
	for i := 0; i < n; i++ {
		if g.r.IntN(4) == 0 {
			if g.r.IntN(5) == 0 {
				// Named integer types are integers for '*'.
				as = append(as, g.pick([]string{sNB("time.Month", "int:5"), sNB("main.NInt", "int8:-3"), sNB("main.NUint", "uint16:7"),
					sNB("time.Duration", "int64:12"), sNB("main.NStr", "str:35")}))
				continue
			}
			as = append(as, g.pick(rtStarArgs))
		} else {
			as = append(as, g.rtOperand())
		}
	}
	return as
}

// rtOperand is a top-level operand of a case: at most rtMaxLeaves leaves,
// and not an integer that a '*' could turn into a width over 2000 (a
// width pads every leaf of a container, and a bad verb on a time.Time
// prints its location's hundreds of fields).
func (g *fuzzGen) rtOperand() string {
	for {
		spec := g.rtBoundedValue(rtMaxLeaves)
		kind, num, ok := strings.Cut(spec, ":")
		if !ok || !strings.HasPrefix(kind, "int") && !strings.HasPrefix(kind, "uint") {
			return spec
		}
		if n, err := strconv.ParseInt(num, 10, 64); err == nil && (n < -2000 || n > 2000) && n >= -1e6 && n <= 1e6 {
			continue
		}
		if n, err := strconv.ParseUint(num, 10, 64); err == nil && n > 2000 && n <= 1e6 {
			continue
		}
		return spec
	}
}

// anyPointerLike reports whether an operand prints an address under %p.
func anyPointerLike(vs []any) bool {
	for _, v := range vs {
		if pointerLike(v) {
			return true
		}
	}
	return false
}

// errorfWrapped returns the indexes of the operands that err unwraps to.
// Operands identical to another operand (e.g. two nil *NilOKErr) are
// ambiguous, so ok is false when a wrapped error matches several operands.
func errorfWrapped(err error, a []any) (idx []string, ok bool) {
	var wrapped []error
	switch u := err.(type) {
	case interface{ Unwrap() []error }:
		wrapped = u.Unwrap()
	case interface{ Unwrap() error }:
		if e := u.Unwrap(); e != nil {
			wrapped = []error{e}
		}
	}
	for _, we := range wrapped {
		found := -1
		for k, a := range a {
			if ae, isErr := a.(error); isErr && sameErr(ae, we) {
				if found >= 0 {
					return nil, false
				}
				found = k
			}
		}
		if found < 0 {
			return nil, false
		}
		idx = append(idx, strconv.Itoa(found))
	}
	return idx, true
}

// sameErr reports whether two errors are the same value: == for the
// pointer types, the map pointer for SM (maps are not comparable).
func sameErr(a, b error) bool {
	va, vb := reflect.ValueOf(a), reflect.ValueOf(b)
	if va.Type() != vb.Type() {
		return false
	}
	if va.Kind() == reflect.Map {
		return va.UnsafePointer() == vb.UnsafePointer()
	}
	return a == b
}

func writeRedTeamCases(w *bufio.Writer, seed uint64, n, maxOut int) (int, int) {
	g := &fuzzGen{r: rand.New(rand.NewPCG(seed^0x5bd1e995, seed+0x632be59bd9b4e019))}
	count, dropped := 0, 0
	emit := func(fn, format string, args []string, out string, extra string) {
		_, _ = fmt.Fprintf(w, "%s\t%s\t%s\t%s%s\n", fn, strconv.Quote(format), strings.Join(args, " "), strconv.Quote(out), extra)
		count++
	}
	debug := os.Getenv("GO_FMT_RT_DEBUG") != ""
	for i := 0; i < n; i++ {
		if debug {
			log.Printf("case %d", i)
		}
		switch {
		case i%3 == 0:
			// print / println: long operand lists mixing strings, named
			// string types and everything else.
			args := g.rtArgs(g.r.IntN(8))
			for j := range args {
				if g.r.IntN(3) == 0 {
					args[j] = g.pick([]string{sStr(g.rtStr()), sStr(""), sSafe("html", g.rtStr()), sSafe("url", ""),
						sNB("main.HStr", sStr(g.rtStr())), sNB("main.NStr", sStr("")), sNB("main.NStrErr", sStr(g.errStr())),
						sNB("time.Month", "int:9"), sNB("main.NBool", "bool:1")})
				}
			}
			fn := "sprint"
			if i%9 == 0 {
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
		case i%10 == 1:
			nargs := g.r.IntN(5)
			var args []string
			for j := 0; j < nargs; j++ {
				switch g.r.IntN(3) {
				case 0:
					args = append(args, g.rtHook())
				case 1:
					args = append(args, g.pick([]string{sObj(g.pick(rtErrKinds), g.errStr()), sMap("main.SM", "k", "nil"), sTnil("*main.NilOKErr"), sTnil("error")}))
				default:
					args = append(args, g.rtOperand())
				}
			}
			format := g.rtFormat(nargs)
			switch g.r.IntN(3) {
			case 0:
				format = strings.ReplaceAll(format, "v", "w")
			case 1:
				// Plain %w-heavy formats: every directive is well formed.
				var b strings.Builder
				for p := 1 + g.r.IntN(4); p > 0; p-- {
					b.WriteString(g.pick([]string{"%w", "%w", "%w", "%v", "%s", "%d", "%#w", "%+w", "%10w", "%-8w", "%.2w",
						"%x", "%q", ": ", " ", "%%", "%[" + strconv.Itoa(1+g.r.IntN(nargs+1)) + "]w", "%[1]w"}))
				}
				format = b.String()
			}
			a1 := decodeSpecs(args)
			if strings.Contains(format, "p") && anyPointerLike(a1) {
				dropped++
				continue
			}
			err := fmt.Errorf(format, a1...)
			e2 := fmt.Errorf(format, decodeSpecs(args)...)
			idx, ok := errorfWrapped(err, a1)
			if !ok || err.Error() != e2.Error() || len(err.Error()) > maxOut {
				dropped++
				continue
			}
			emit("errorf", format, args, err.Error(), "\t"+strings.Join(idx, ","))
		default:
			nargs := g.r.IntN(6)
			args := g.rtArgs(nargs)
			format := g.rtFormat(nargs)
			if debug {
				log.Printf("sprintf %q %s", format, strings.Join(args, " "))
			}
			a1, a2 := decodeSpecs(args), decodeSpecs(args)
			if strings.Contains(format, "p") && anyPointerLike(a1) {
				dropped++
				continue
			}
			o1, o2 := fmt.Sprintf(format, a1...), fmt.Sprintf(format, a2...)
			if o1 != o2 || len(o1) > maxOut {
				dropped++
				continue
			}
			emit("sprintf", format, args, o1, "")
		}
	}
	return count, dropped
}

// writeMatrixOver writes fuzz_formats.txt and fuzz_matrix.txt for the
// given formats and operand specs.
func writeMatrixOver(dir string, formats []string, specs []string) {
	f, w := create(dir, "fuzz_formats.txt")
	for _, s := range formats {
		_, _ = fmt.Fprintln(w, strconv.Quote(s))
	}
	finish(f, w)
	writeHashes(dir, "fuzz_matrix.txt", formats, specs)
}

// writeHashes writes "spec \t fnv64 \t nondeterministic format ranges"
// lines (the matrix.txt format) for the given formats and operands.
func writeHashes(dir, name string, formats []string, specs []string) {
	f, w := create(dir, name)
	total, skipped := 0, 0
	for _, spec := range specs {
		v1, v2 := mustDecode(spec), mustDecode(spec)
		h := fnv.New64a()
		var nondet []int
		for fi, format := range formats {
			o1 := fmt.Sprintf(format, v1)
			o2 := fmt.Sprintf(format, v2)
			if o1 != o2 || (strings.ContainsRune(format, 'p') && pointerLike(v1)) {
				nondet = append(nondet, fi)
				skipped++
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
	log.Printf("%s: %d operands x %d formats: %d outputs hashed, %d nondeterministic skipped", name, len(specs), len(formats), total, skipped)
}

func writeRedTeam(dir string, seed uint64, nCases, nMatrix, maxOut int) {
	f, w := create(dir, "fuzz_cases.txt")
	count, dropped := writeRedTeamCases(w, seed, nCases, maxOut)
	finish(f, w)
	log.Printf("redteam cases: %d written, %d dropped (nondeterministic, %%p of a pointer, ambiguous %%w or longer than %d bytes)", count, dropped, maxOut)

	g := &fuzzGen{r: rand.New(rand.NewPCG(seed*7+3, seed^0xa0761d6478bd642f))}
	var specs []string
	for len(specs) < nMatrix {
		specs = append(specs, g.rtBoundedValue(24))
	}
	writeMatrixOver(dir, formatMatrix(), specs)
}

// rtExtraCorpus is the hook and named-type operands added to
// valueCorpus() by -mode flagperm.
func rtExtraCorpus() []string {
	var vs []string
	for _, t := range rtNilHooks {
		vs = append(vs, sTnil(t))
	}
	for _, k := range append(append([]string{}, rtStrKinds...), rtErrKinds...) {
		s := "a\xffb\"c"
		if k == "obj_both" || strings.HasSuffix(k, "err") || k == "obj_npe" || k == "obj_noe" {
			// Error messages are valid UTF-8 (Object::go_error is a String).
			s = "a\u00e9b\"c"
		}
		vs = append(vs, sObj(k, s))
	}
	vs = append(vs,
		sList("main.SS"), sList("main.SS", sInt("int", 1), sStr("x")), sTnil("main.SS"),
		sMap("main.SM"), sMap("main.SM", "k", sInt("int", 1)), sTnil("main.SM"),
		sList("[]fmt.Stringer", sTnil("*main.NilPanicStr"), sTnil("*main.NilOKStr"), "nil"),
		sList("[]error", sTnil("*main.NilPanicErr"), sTnil("*main.NilOKErr"), sMap("main.SM"), "nil"),
		sList("[]*main.NilPanicStr", "nil", sObj("obj_nps", "p")),
		sList("[]main.SS", "nil", sList("main.SS", "nil")),
		sMap("map[string]error", "a", sTnil("*main.NilPanicErr"), "b", "nil"),
		sMap("map[string]fmt.Stringer", "a", sTnil("*main.NilOKStr"), "b", sList("main.SS")),
		sMap("map[string]main.SM", "a", "nil", "b", sMap("main.SM")),
		"obj_plain("+sTnil("*main.NilPanicStr")+","+sList("main.SS")+")",
		"obj_pplain("+sTnil("*main.NilOKErr")+","+sMap("main.SM")+")",
		"time:1790510400;5;fixedhex="+hexs("\u65e5\"\\")+"=-3601",
		"time:0;0;fixedhex="+hexs("")+"=45",
		"pages:2", "taxlist:1", sTnil("page.Pages"), sTnil("*time.Location"),
		sNB("time.Month", "int:9"), sNB("time.Month", "int:13"), sNB("time.Weekday", "int:-1"),
		sNB("time.Duration", "int64:-5400000000123"), sNB("main.HStr", sStr("h\xff\"s")), sNB("main.Celsius", sF64(-0.05)),
		sNB("main.NStrErr", sStr("e")), sNB("main.NInt", "int8:-128"), sNB("main.NUint", "uint16:65535"),
		sNB("main.NF32", sF32(1.1)), sNB("main.NBool", "bool:0"), sNB("main.NStr", sStr("s")),
		sList("[]time.Month", sNB("time.Month", "int:1"), sNB("time.Month", "int:0")),
		sList("[]main.HStr", sNB("main.HStr", sStr("a"))),
		"obj_plain("+sNB("time.Weekday", "int:3")+","+sNB("main.NStr", sStr("x"))+")",
	)
	return vs
}

// flagPermFormats: every flag string of length 0-maxLen (in every order,
// with repeats) x 25 verbs x widths {"", 7} x precisions {"", .3}.
// crates/go-fmt/tests/redteam.rs builds the same list (maxLen 3).
func flagPermFormats(maxLen int) []string {
	flags := []string{""}
	level := []string{""}
	for n := 1; n <= maxLen; n++ {
		var next []string
		for _, p := range level {
			for _, c := range "+-# 0" {
				next = append(next, p+string(c))
			}
		}
		flags = append(flags, next...)
		level = next
	}
	verbs := []string{"v", "d", "s", "q", "x", "X", "t", "b", "o", "O", "c", "U", "e", "E", "f", "F", "g", "G", "T", "p", "%", "w", "z", "!", "\u00e9"}
	var fs []string
	for _, v := range verbs {
		for _, fl := range flags {
			for _, wd := range []string{"", "7"} {
				for _, p := range []string{"", ".3"} {
					fs = append(fs, "%"+fl+wd+p+v)
				}
			}
		}
	}
	return fs
}

func writeFlagPerm(dir string, seed uint64, nCases, maxOut int) {
	f, w := create(dir, "fuzz_cases.txt")
	count, dropped := writeRedTeamCases(w, seed, nCases, maxOut)
	finish(f, w)
	log.Printf("redteam cases: %d written, %d dropped", count, dropped)
	writeMatrixOver(dir, flagPermFormats(4), append(valueCorpus(), rtExtraCorpus()...))
}

// Deeply nested operands: nest:KIND;N(child) wraps child N times.

var deepSpecs = []string{
	"nest:list;10000(int:1)", "nest:list;100000(str:6869)", "nest:map;10000(nil)", "nest:plain;10000(f64:3ff8000000000000)",
	"nest:ns;10000(bool:1)", "nest:nm;10000(tnil:2a696e74)", "nest:objns;10000(int8:-5)", "nest:objnm;10000(bytes:6162)",
	"nest:mix;30000(obj_sv:6869)", "nest:mix;100000(nil)",
}

var deepFormats = []string{"%v", "%+v", "%#v", "%d", "%s", "%x", "%q", "%10.3v"}

// decodeNest builds a nest node iteratively (list, map, plain, ns and
// nm are []interface {}, map[string]interface {}, Plain, NS and NM; objns
// and objnm are the same Go types as ns and nm; mix cycles list, map,
// plain).
func decodeNest(n *specNode) (any, error) {
	kind, count, ok := strings.Cut(n.payload, ";")
	cnt, err := strconv.Atoi(count)
	if !ok || err != nil || len(n.children) != 1 {
		return nil, fmt.Errorf("bad nest node %q", n.payload)
	}
	v, err := decodeNode(n.children[0])
	if err != nil {
		return nil, err
	}
	for i := 0; i < cnt; i++ {
		k := kind
		if k == "mix" {
			k = []string{"list", "map", "plain"}[i%3]
		}
		switch k {
		case "list":
			v = []any{v}
		case "map":
			v = map[string]any{"k": v}
		case "plain":
			v = Plain{v, nil}
		case "ns", "objns":
			v = NS{v}
		case "nm", "objnm":
			v = NM{"k": v}
		default:
			return nil, fmt.Errorf("bad nest kind %q", k)
		}
	}
	return v, nil
}

// writeDeep writes deep.txt: "spec \t format \t len \t fnv64" of
// Sprintf(format, operand) for the deeply nested operands.
func writeDeep(dir string) {
	f, w := create(dir, "deep.txt")
	for _, spec := range deepSpecs {
		for _, format := range deepFormats {
			o1 := fmt.Sprintf(format, mustDecode(spec))
			if o2 := fmt.Sprintf(format, mustDecode(spec)); o1 != o2 {
				log.Fatalf("deep: %s %s is not deterministic", spec, format)
			}
			h := fnv.New64a()
			h.Write([]byte(o1))
			_, _ = fmt.Fprintf(w, "%s\t%s\t%d\t%016x\n", spec, strconv.Quote(format), len(o1), h.Sum64())
		}
	}
	finish(f, w)
}

// writeRedTeamFixtures writes the red-team pass's checked-in fixtures
// (-mode vectors): redteam_cases.txt (seed 1, the fuzz_cases.txt format),
// redteam_matrix.txt (formats.txt over 60 random nested operands),
// flagperm_matrix.txt (flagPermFormats(3) over valueCorpus() and
// rtExtraCorpus()) and deep.txt.
func writeRedTeamFixtures(dir string) {
	f, w := create(dir, "redteam_cases.txt")
	count, dropped := writeRedTeamCases(w, 1, 3200, 2000)
	finish(f, w)
	log.Printf("redteam cases: %d written, %d dropped", count, dropped)

	g := &fuzzGen{r: rand.New(rand.NewPCG(8, 9))}
	var specs []string
	for len(specs) < 60 {
		specs = append(specs, g.rtBoundedValue(24))
	}
	writeHashes(dir, "redteam_matrix.txt", formatMatrix(), specs)
	writeHashes(dir, "flagperm_matrix.txt", flagPermFormats(3), append(valueCorpus(), rtExtraCorpus()...))
	writeDeep(dir)
}
