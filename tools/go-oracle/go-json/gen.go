package main

import (
	"encoding/json"
	"fmt"
	"html/template"
	"math"
	"math/rand/v2"
	"reflect"
	"slices"
	"strings"
	"time"
	"unicode"
)

func sortedKeys[V any](m map[string]V) []string {
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	slices.Sort(keys)
	return keys
}

// strPieces are glued together to build adversarial strings: every control
// byte, the escaped ASCII set, U+2028/U+2029 (also split and next to
// invalid bytes), multi-byte UTF-8 and malformed UTF-8.
var strPieces = func() []string {
	p := []string{
		"a", "Z", "0", " ", "hello", "Lay's", "/", "'", "\"", "\\", "<", ">", "&", "&amp;", "</script>",
		"\x7f", "\u0080", "\u00a0", "é", "ก", "ปาร์ตี้", "日本", "😀", "\u2028", "\u2029", "\u2027", "\u202a",
		"\ufffd", "\ufeff", "\U0010ffff",
		"\xff", "\xfe", "\x80", "\xbf", "\xc0\xaf", "\xc3", "\xed\xa0\x80", "\xed\xbf\xbf", "\xe2\x80", "\xe2",
		"\xe2\x80\xa8\xff", "\xf4\x90\x80\x80", "\xf0\x9f\x98", "\xf8\x88\x80\x80\x80", "\xe2\x80\xa9",
	}
	for c := 0; c < 0x20; c++ {
		p = append(p, string([]byte{byte(c)}))
	}
	return p
}()

func genString(r *rand.Rand, maxPieces int) string {
	var b strings.Builder
	n := r.IntN(maxPieces + 1)
	for i := 0; i < n; i++ {
		switch r.IntN(10) {
		case 0:
			// random bytes
			for k := r.IntN(4); k >= 0; k-- {
				b.WriteByte(byte(r.IntN(256)))
			}
		case 1, 2:
			// random printable ASCII run
			for k := r.IntN(8); k >= 0; k-- {
				b.WriteByte(byte(0x20 + r.IntN(0x5f)))
			}
		default:
			b.WriteString(strPieces[r.IntN(len(strPieces))])
		}
	}
	return b.String()
}

func float64Specials() []float64 {
	f := []float64{
		0, math.Copysign(0, -1), 1, -1, 0.1, 0.2, 0.3, 1.0 / 3, 2.0 / 3, 100, 1e6, 1e7, 123456789, 1234567.5,
		1e-6, -1e-6, 1e-7, 1.5e-7, 12e-8, 9.999999e-7, 1e21, -1e21, 1e20, 9.999999999999999e20, 1e22,
		5e-324, -5e-324, math.MaxFloat64, -math.MaxFloat64, math.SmallestNonzeroFloat64, 2.2250738585072014e-308,
		1 << 53, 1<<53 + 2, 1 << 62, 1e15, 1e16, 1e17, 0.000001, 0.0000011, 1e-5, 1e-10, 1e-100, 1e100, 1e300,
		3.5, 4.5, 0.5, 2.5, 15000000000, 1.7976931348623157e308, 4.35, 0.07, 1e-323, 123e-20, 1e-9,
	}
	for _, x := range []float64{1e-6, 1e21, 1e-5, 1e20} {
		f = append(f, math.Nextafter(x, 0), math.Nextafter(x, math.Inf(1)), -math.Nextafter(x, 0))
	}
	return f
}

func float32Specials() []float32 {
	f := []float32{
		0, float32(math.Copysign(0, -1)), 1, -1, 0.1, 1.0 / 3, 16777216, 16777217, 1e-6, 1e21, 1e-7, 1e20,
		math.MaxFloat32, -math.MaxFloat32, math.SmallestNonzeroFloat32, 1.17549435e-38, 3.4028235e38, 0.3, 100.25,
	}
	for _, x := range []float32{1e-6, 1e21} {
		f = append(f, math.Nextafter32(x, 0), math.Nextafter32(x, float32(math.Inf(1))))
	}
	return f
}

var f64s = float64Specials()
var f32s = float32Specials()

func genFloat64(r *rand.Rand, allowBad bool) float64 {
	switch r.IntN(8) {
	case 0, 1:
		return f64s[r.IntN(len(f64s))]
	case 2:
		if allowBad && r.IntN(10) == 0 {
			return []float64{math.NaN(), math.Inf(1), math.Inf(-1)}[r.IntN(3)]
		}
		return float64(r.Int64N(2000000) - 1000000)
	case 3:
		// decimal-looking values across the cut-offs
		m := float64(r.IntN(100000))
		e := r.IntN(60) - 30
		return m * math.Pow(10, float64(e))
	case 4:
		return r.NormFloat64() * math.Pow(10, float64(r.IntN(50)-25))
	default:
		for {
			f := math.Float64frombits(r.Uint64())
			if !math.IsNaN(f) && !math.IsInf(f, 0) {
				return f
			}
		}
	}
}

func genFloat32(r *rand.Rand, allowBad bool) float32 {
	switch r.IntN(5) {
	case 0, 1:
		return f32s[r.IntN(len(f32s))]
	case 2:
		if allowBad && r.IntN(10) == 0 {
			return float32([]float64{math.NaN(), math.Inf(1), math.Inf(-1)}[r.IntN(3)])
		}
		return float32(r.NormFloat64() * math.Pow(10, float64(r.IntN(50)-25)))
	default:
		for {
			f := math.Float32frombits(r.Uint32())
			if !math.IsNaN(float64(f)) && !math.IsInf(float64(f), 0) {
				return f
			}
		}
	}
}

func genInt64(r *rand.Rand) int64 {
	switch r.IntN(4) {
	case 0:
		return []int64{0, 1, -1, math.MaxInt64, math.MinInt64, math.MaxInt32, math.MinInt32, 127, -128, 255}[r.IntN(10)]
	case 1:
		return int64(r.Uint64())
	default:
		return r.Int64N(20000) - 10000
	}
}

func genUint64(r *rand.Rand) uint64 {
	if r.IntN(3) == 0 {
		return []uint64{0, 1, math.MaxUint64, math.MaxUint32, 255, 1 << 63}[r.IntN(6)]
	}
	return r.Uint64() >> r.IntN(64)
}

var zoneNames = []string{"", "UTC", "ICT", "+07", "CET", "X<y>&z", "\xff"}

func genTime(r *rand.Rand, allowBad bool) time.Time {
	var sec int64
	switch r.IntN(6) {
	case 0:
		sec = -62135596800 // 0001-01-01
	case 1:
		if allowBad {
			sec = []int64{-62135596800 - 86400*366*2, 253402300800 + 86400, -62167219200 - 1}[r.IntN(3)]
		} else {
			sec = 1600000000
		}
	default:
		sec = r.Int64N(253402300799+62135596800) - 62135596800
	}
	nsec := int64(0)
	switch r.IntN(4) {
	case 0:
		nsec = r.Int64N(1e9)
	case 1:
		nsec = r.Int64N(1000) * 1e6
	case 2:
		nsec = r.Int64N(1000) * 1e3
	}
	t := time.Unix(sec, nsec)
	if r.IntN(2) == 0 {
		return t.UTC()
	}
	offs := []int{0, 7 * 3600, -5 * 3600, 5*3600 + 1800, -(9*3600 + 30*60), 3723, -61, 14 * 3600}
	if allowBad {
		offs = append(offs, 24*3600, -25*3600, 100*3600)
	}
	return t.In(time.FixedZone(zoneNames[r.IntN(len(zoneNames))], offs[r.IntN(len(offs))]))
}

var numberLits = []string{"0", "-0", "1", "-1.5", "1e10", "1E+2", "0.000001", "123456789012345678901234567890", "3.14e-10",
	"", "01", "1.", ".5", "-", "+1", "0x10", "1e", "NaN", "1_000", " 1"}

// genValue returns a random Go value built from the vdump types. bad allows
// values that make Marshal fail (NaN, bad times, failing marshalers).
func genValue(r *rand.Rand, depth int, bad bool) any {
	top := 30
	if depth <= 0 {
		top = 21
	}
	switch k := r.IntN(top); k {
	case 0:
		return nil
	case 1:
		return r.IntN(2) == 0
	case 2:
		switch r.IntN(5) {
		case 0:
			return int(genInt64(r))
		case 1:
			return int8(genInt64(r))
		case 2:
			return int16(genInt64(r))
		case 3:
			return int32(genInt64(r))
		default:
			return genInt64(r)
		}
	case 3:
		switch r.IntN(6) {
		case 0:
			return uint(genUint64(r))
		case 1:
			return uint8(genUint64(r))
		case 2:
			return uint16(genUint64(r))
		case 3:
			return uint32(genUint64(r))
		case 4:
			return uintptr(genUint64(r))
		default:
			return genUint64(r)
		}
	case 4, 5, 6:
		return genFloat64(r, bad)
	case 7:
		return genFloat32(r, bad)
	case 8, 9, 10:
		return genString(r, 8)
	case 11:
		s := genString(r, 5)
		switch r.IntN(7) {
		case 0:
			return template.HTML(s)
		case 1:
			return template.HTMLAttr(s)
		case 2:
			return template.CSS(s)
		case 3:
			return template.JS(s)
		case 4:
			return template.JSStr(s)
		case 5:
			return template.URL(s)
		default:
			return template.Srcset(s)
		}
	case 12:
		return genTime(r, bad)
	case 13:
		switch r.IntN(6) {
		case 0:
			return (*int)(nil)
		case 1:
			return []string(nil)
		case 2:
			return map[string]any(nil)
		case 3:
			return []any(nil)
		case 4:
			return []byte(nil)
		default:
			return (*time.Time)(nil)
		}
	case 14:
		if !bad {
			return json.Number(numberLits[r.IntN(9)])
		}
		return json.Number(numberLits[r.IntN(len(numberLits))])
	case 15:
		return jm{genJSONText(r, bad)}
	case 16:
		if bad && r.IntN(3) == 0 {
			return jerr{strings.ToValidUTF8(genString(r, 3), "?")}
		}
		return jm{genJSONText(r, false)}
	case 17:
		if bad && r.IntN(4) == 0 {
			return tmerr{strings.ToValidUTF8(genString(r, 3), "?")}
		}
		return tm{genString(r, 6)}
	case 18:
		n := r.IntN(12)
		b := make([]byte, n)
		for i := range b {
			b[i] = byte(r.IntN(256))
		}
		return b
	case 19, 20:
		n := r.IntN(4)
		switch r.IntN(6) {
		case 0:
			s := make([]string, n)
			for i := range s {
				s[i] = genString(r, 4)
			}
			return s
		case 1:
			s := make([]int, n)
			for i := range s {
				s[i] = int(genInt64(r))
			}
			return s
		case 2:
			s := make([]int64, n)
			for i := range s {
				s[i] = genInt64(r)
			}
			return s
		case 3:
			s := make([]float64, n)
			for i := range s {
				s[i] = genFloat64(r, bad)
			}
			return s
		case 4:
			s := make([]bool, n)
			for i := range s {
				s[i] = r.IntN(2) == 0
			}
			return s
		default:
			m := make(map[string]string, n)
			for i := 0; i < n; i++ {
				m[genString(r, 3)] = genString(r, 4)
			}
			return m
		}
	case 21, 22, 23, 24:
		n := r.IntN(6)
		s := make([]any, n)
		for i := range s {
			s[i] = genValue(r, depth-1, bad)
		}
		return s
	case 25, 26, 27:
		return genMap(r, depth, bad)
	case 28:
		n := r.IntN(3)
		s := make([]map[string]any, n)
		for i := range s {
			s[i] = genMap(r, depth, bad)
		}
		return s
	default:
		if r.IntN(2) == 0 {
			return genPlainStruct(r, depth, bad)
		}
		return genTaggedStruct(r, depth, bad)
	}
}

func genMap(r *rand.Rand, depth int, bad bool) map[string]any {
	n := r.IntN(6)
	m := make(map[string]any, n)
	for i := 0; i < n; i++ {
		var k string
		switch r.IntN(4) {
		case 0:
			k = genString(r, 3)
		default:
			k = []string{"title", "tags", "a", "b", "A", "Z", "", "é", "<x>", "a&b", "_", "10", "9", "contents", "relpermalink"}[r.IntN(15)]
		}
		m[k] = genValue(r, depth-1, bad)
	}
	return m
}

// genPlainStruct builds a struct with exported interface{} fields and no tags.
func genPlainStruct(r *rand.Rand, depth int, bad bool) genStruct {
	n := r.IntN(5)
	var fields []reflect.StructField
	var vals []any
	d := fmt.Appendf(nil, "R%d{", n)
	for i := 0; i < n; i++ {
		name := fmt.Sprintf("F%d", i)
		if r.IntN(4) == 0 {
			name = fmt.Sprintf("Ωmega%d", i)
		}
		v := genValue(r, depth-1, bad)
		fields = append(fields, reflect.StructField{Name: name, Type: anyType})
		vals = append(vals, unwrap(v))
		d = putStr(d, name)
		d = dump(d, v)
	}
	d = append(d, '}')
	sv := reflect.New(reflect.StructOf(fields)).Elem()
	for i, v := range vals {
		if v != nil {
			sv.Field(i).Set(reflect.ValueOf(v))
		}
	}
	return genStruct{value: sv.Interface(), d: d}
}

var tagNames = []string{"", "", "name", "title", "a<b", "x&y", "Ünï", "with space", "dash-ed", "p.q", "1st", "$ref", "@id", "_u"}

func isValidTag(s string) bool {
	if s == "" {
		return false
	}
	for _, c := range s {
		switch {
		case strings.ContainsRune("!#$%&()*+-./:;<=>?@[]^_{|}~ ", c):
		case !unicode.IsLetter(c) && !unicode.IsDigit(c):
			return false
		}
	}
	return true
}

// genTaggedStruct builds a struct whose fields carry json tags with
// omitempty/omitzero/string options, typed either as the concrete type of
// their value or as interface{}.
func genTaggedStruct(r *rand.Rand, depth int, bad bool) genStruct {
	n := r.IntN(6)
	var fields []reflect.StructField
	var vals []any
	used := map[string]bool{}
	var body []byte
	count := 0
	for i := 0; i < n; i++ {
		goName := fmt.Sprintf("G%d", i)
		tagName := tagNames[r.IntN(len(tagNames))]
		name := goName
		if isValidTag(tagName) {
			name = tagName
		}
		if used[name] {
			continue
		}
		used[name] = true
		var opts []string
		flags := 0
		if r.IntN(2) == 0 {
			opts = append(opts, "omitempty")
			flags |= 1
		}
		if r.IntN(3) == 0 {
			opts = append(opts, "omitzero")
			flags |= 2
		}
		if r.IntN(3) == 0 {
			opts = append(opts, "string")
			flags |= 4
		}
		var v any
		if r.IntN(3) == 0 {
			// Values that are empty/zero more often.
			v = []any{"", 0, int8(0), uint(0), 0.0, math.Copysign(0, -1), float32(0), false, []string{}, map[string]any{},
				[]byte{}, time.Time{}, template.HTML(""), json.Number(""), nil, (*int)(nil), []string(nil)}[r.IntN(17)]
		} else {
			v = genValue(r, depth-1, bad)
		}
		typ := anyType
		iface := true
		switch v.(type) {
		case nil, jm, jerr, tm, tmerr, genStruct:
		default:
			if r.IntN(3) != 0 {
				typ = reflect.TypeOf(v)
				iface = false
			}
		}
		if iface {
			flags |= 8
		}
		tag := tagName
		if len(opts) > 0 {
			tag += "," + strings.Join(opts, ",")
		}
		sf := reflect.StructField{Name: goName, Type: typ}
		if tag != "" {
			sf.Tag = reflect.StructTag(fmt.Sprintf("json:%q", tag))
		}
		fields = append(fields, sf)
		vals = append(vals, unwrap(v))
		body = append(body, byte('0'+flags))
		body = putStr(body, name)
		body = dump(body, v)
		count++
	}
	d := fmt.Appendf(nil, "P%d{", count)
	d = append(d, body...)
	d = append(d, '}')
	sv := reflect.New(reflect.StructOf(fields)).Elem()
	for i, v := range vals {
		if v != nil {
			sv.Field(i).Set(reflect.ValueOf(v))
		}
	}
	return genStruct{value: sv.Interface(), d: d}
}

// genJSONText returns JSON text: valid compact or indented JSON with odd
// whitespace, or (when bad) possibly invalid JSON.
func genJSONText(r *rand.Rand, bad bool) string {
	v := unwrap(genValue(r, 2, false))
	b, err := json.Marshal(v)
	if err != nil {
		return "null"
	}
	switch r.IntN(4) {
	case 0:
		var out []byte
		ws := []string{"", " ", "\t", "\n", "\r\n", " \t "}
		out, err = appendIndentLike(b, ws[r.IntN(len(ws))], ws[r.IntN(len(ws))])
		if err == nil {
			b = out
		}
	case 1:
		// Raw HTML-sensitive characters inside strings.
		b = []byte(strings.ReplaceAll(string(b), `\u003c`, "<"))
		b = []byte(strings.ReplaceAll(string(b), `\u0026`, "&"))
		b = []byte(strings.ReplaceAll(string(b), `\u2028`, "\u2028"))
	}
	if bad && r.IntN(3) == 0 {
		b = mutate(r, b)
	}
	return string(b)
}

func appendIndentLike(b []byte, prefix, indent string) ([]byte, error) {
	var out strings.Builder
	var dst = []byte(" \n")
	tmp, err := indentBytes(b, prefix, indent)
	if err != nil {
		return nil, err
	}
	out.Write(dst[:1])
	out.Write(tmp)
	out.Write(dst[1:])
	return []byte(out.String()), nil
}

// mutate applies 1-3 random edits to b.
var mutateBytes = []byte(`{}[],:"\ tfnrue0123456789-+.eEu/'x` + "\x00\x1f\x80\xff\xe2")

func mutate(r *rand.Rand, b []byte) []byte {
	b = slices.Clone(b)
	for k := 1 + r.IntN(3); k > 0; k-- {
		switch r.IntN(5) {
		case 0:
			if len(b) > 0 {
				b[r.IntN(len(b))] = mutateBytes[r.IntN(len(mutateBytes))]
			}
		case 1:
			i := r.IntN(len(b) + 1)
			b = slices.Insert(b, i, mutateBytes[r.IntN(len(mutateBytes))])
		case 2:
			if len(b) > 0 {
				i := r.IntN(len(b))
				b = slices.Delete(b, i, i+1)
			}
		case 3:
			b = b[:r.IntN(len(b)+1)]
		default:
			if len(b) > 0 {
				i := r.IntN(len(b))
				j := i + r.IntN(len(b)-i)
				seg := slices.Clone(b[i:j])
				b = slices.Insert(b, r.IntN(len(b)+1), seg...)
			}
		}
	}
	return b
}
