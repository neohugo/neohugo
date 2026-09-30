// Command cast is the Go oracle for crates/nh-common/src/cast, the port of
// github.com/spf13/cast (the version in go.mod).
//
//	go run ./tools/go-oracle/nh-common/cast [-out rust/testdata/oracle/common/cast/cast.json]
//
// It runs every exported E-function neohugo reaches (ToStringE, ToBoolE, the
// integer, unsigned and float conversions, ToDurationE, ToTimeE,
// ToTimeInDefaultLocationE, ToSliceE, the typed slice conversions and the
// string-keyed map conversions) over a table of inputs of every Go kind the
// value model has: nil, bool, int..int64, uint..uint64, uintptr, float32/64,
// string (numbers in every base and form, booleans, durations, JSON, dates in
// all 24 layouts and edge dates), json.Number, time.Time, time.Duration,
// time.Month, time.Weekday, the html/template string types, []byte, named
// basic types, fmt.Stringer, error, Float64() providers, slices, maps (incl.
// maps.Params), typed nil pointers, plus the inputs of cast's own *_test.go
// tables. Results are {"ok": value} or {"err": message}; values are encoded
// with their Go type (encode below).
//
// Float to integer conversions of out-of-range values are platform dependent
// (arm64 saturates, amd64 does not), so the checked-in fixture comes from an
// arm64 build (GOARCH=arm64, run under qemu-aarch64-static).
//
// time.Local is set to UTC so time.Unix results and zone-abbreviation parsing
// do not depend on the machine.
package main

import (
	"archive/zip"
	"encoding/hex"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"go/scanner"
	"go/token"
	"html/template"
	"io"
	"log"
	"math"
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"runtime"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/corpus"
	"github.com/spf13/cast"
)

// Named types (resolveAlias), a Stringer, an error and Float64 providers.
type (
	MyInt     int
	MyInt8    int8
	MyInt64   int64
	MyUint    uint
	MyUint8   uint8
	MyUintptr uintptr
	MyFloat   float64
	MyFloat32 float32
	MyString  string
	MyBool    bool
)

type stringer struct{ s string }

func (s stringer) String() string { return s.s }

type myErr struct{ msg string }

func (e myErr) Error() string { return e.msg }

type f64p struct{ v float64 }

func (p f64p) Float64() float64 { return p.v }

type f64ep struct {
	v    float64
	fail bool
}

func (p f64ep) Float64() (float64, error) {
	if p.fail {
		return 0, errors.New("no float")
	}
	return p.v, nil
}

// zones used by time inputs and ToTimeInDefaultLocationE.
type zoneSpec struct {
	Name   string `json:"name"`
	Kind   string `json:"kind"` // "utc", "fixed", "tzdata"
	Offset int    `json:"offset,omitempty"`
	TZData string `json:"tzdata,omitempty"`
	loc    *time.Location
}

var zones []*zoneSpec

func main() {
	out := flag.String("out", "rust/testdata/oracle/common/cast/cast.json", "output file")
	zipPath := flag.String("zoneinfo", filepath.Join(runtime.GOROOT(), "lib", "time", "zoneinfo.zip"), "zoneinfo.zip")
	flag.Parse()

	time.Local = time.UTC

	ny := readZone(*zipPath, "America/New_York")
	nyLoc, err := time.LoadLocationFromTZData("America/New_York", ny)
	if err != nil {
		log.Fatal(err)
	}
	zones = []*zoneSpec{
		{Name: "UTC", Kind: "utc", loc: time.UTC},
		{Name: "", Kind: "fixed", Offset: 7 * 3600, loc: time.FixedZone("", 7*3600)},
		{Name: "ICT", Kind: "fixed", Offset: 7 * 3600, loc: time.FixedZone("ICT", 7*3600)},
		{Name: "America/New_York", Kind: "tzdata", TZData: hex.EncodeToString(ny), loc: nyLoc},
	}

	type fn struct {
		name string
		f    func(any) (any, error)
	}
	w := func(f func(any) (any, error)) func(any) (any, error) { return f }
	fns := []fn{
		{"ToStringE", w(func(i any) (any, error) { return cast.ToStringE(i) })},
		{"ToBoolE", w(func(i any) (any, error) { return cast.ToBoolE(i) })},
		{"ToIntE", w(func(i any) (any, error) { return cast.ToIntE(i) })},
		{"ToInt8E", w(func(i any) (any, error) { return cast.ToInt8E(i) })},
		{"ToInt16E", w(func(i any) (any, error) { return cast.ToInt16E(i) })},
		{"ToInt32E", w(func(i any) (any, error) { return cast.ToInt32E(i) })},
		{"ToInt64E", w(func(i any) (any, error) { return cast.ToInt64E(i) })},
		{"ToUintE", w(func(i any) (any, error) { return cast.ToUintE(i) })},
		{"ToUint8E", w(func(i any) (any, error) { return cast.ToUint8E(i) })},
		{"ToUint16E", w(func(i any) (any, error) { return cast.ToUint16E(i) })},
		{"ToUint32E", w(func(i any) (any, error) { return cast.ToUint32E(i) })},
		{"ToUint64E", w(func(i any) (any, error) { return cast.ToUint64E(i) })},
		{"ToFloat32E", w(func(i any) (any, error) { return cast.ToFloat32E(i) })},
		{"ToFloat64E", w(func(i any) (any, error) { return cast.ToFloat64E(i) })},
		{"ToDurationE", w(func(i any) (any, error) { return cast.ToDurationE(i) })},
		{"ToTimeE", w(func(i any) (any, error) { return cast.ToTimeE(i) })},
		{"ToTimeInDefaultLocationE:1", w(func(i any) (any, error) { return cast.ToTimeInDefaultLocationE(i, zones[1].loc) })},
		{"ToTimeInDefaultLocationE:3", w(func(i any) (any, error) { return cast.ToTimeInDefaultLocationE(i, zones[3].loc) })},
		{"ToSliceE", w(func(i any) (any, error) { return cast.ToSliceE(i) })},
		{"ToStringSliceE", w(func(i any) (any, error) { return cast.ToStringSliceE(i) })},
		{"ToIntSliceE", w(func(i any) (any, error) { return cast.ToIntSliceE(i) })},
		{"ToInt64SliceE", w(func(i any) (any, error) { return cast.ToInt64SliceE(i) })},
		{"ToUintSliceE", w(func(i any) (any, error) { return cast.ToUintSliceE(i) })},
		{"ToFloat64SliceE", w(func(i any) (any, error) { return cast.ToFloat64SliceE(i) })},
		{"ToBoolSliceE", w(func(i any) (any, error) { return cast.ToBoolSliceE(i) })},
		{"ToDurationSliceE", w(func(i any) (any, error) { return cast.ToDurationSliceE(i) })},
		{"ToStringMapE", w(func(i any) (any, error) { return cast.ToStringMapE(i) })},
		{"ToStringMapStringE", w(func(i any) (any, error) { return cast.ToStringMapStringE(i) })},
		{"ToStringMapBoolE", w(func(i any) (any, error) { return cast.ToStringMapBoolE(i) })},
	}

	var cases []map[string]any
	seen := map[string]bool{}
	for _, in := range inputs() {
		enc := encode(in)
		key, err := json.Marshal(enc)
		if err != nil {
			log.Fatal(err)
		}
		if seen[string(key)] {
			continue
		}
		seen[string(key)] = true
		c := map[string]any{"in": enc}
		for _, f := range fns {
			if !collectionFn[f.name] || wantsCollectionFns(in) {
				c[f.name] = call(f.f, in)
			}
		}
		cases = append(cases, c)
	}

	header := map[string]any{
		"source": "tools/go-oracle/nh-common/cast",
		"goarch": runtime.GOARCH,
		"zones":  zones,
	}
	if err := corpus.WriteCases(*out, header, cases); err != nil {
		log.Fatal(err)
	}
	log.Printf("%d inputs x %d functions -> %s", len(cases), len(fns), *out)
}

// collectionFn are the typed slice and map conversions. For scalar inputs
// they only format an error (the same one the scalar conversions exercise),
// so they run on a sample of scalars only, to keep the fixture small.
var collectionFn = map[string]bool{
	"ToSliceE": true, "ToIntSliceE": true, "ToInt64SliceE": true, "ToUintSliceE": true,
	"ToFloat64SliceE": true, "ToBoolSliceE": true, "ToDurationSliceE": true,
	"ToStringMapStringE": true, "ToStringMapBoolE": true,
}

var scalarSample int

func wantsCollectionFns(in any) bool {
	switch x := in.(type) {
	case string:
		if strings.HasPrefix(strings.TrimSpace(x), "{") || strings.HasPrefix(strings.TrimSpace(x), "[") || x == "null" {
			return true
		}
	case nil, []byte:
		return true
	}
	switch reflect.ValueOf(in).Kind() {
	case reflect.Slice, reflect.Map, reflect.Pointer:
		return true
	}
	scalarSample++
	return scalarSample%10 == 1
}

func call(f func(any) (any, error), in any) (res map[string]any) {
	defer func() {
		if r := recover(); r != nil {
			res = map[string]any{"panic": fmt.Sprint(r)}
		}
	}()
	v, err := f(in)
	if err != nil {
		return map[string]any{"err": err.Error()}
	}
	return map[string]any{"ok": encode(v)}
}

func fbits(f float64) string { return fmt.Sprintf("%016x", math.Float64bits(f)) }

// encode describes a Go value with its type; the Rust test builds the same
// go_value::Value from it (inputs) or encodes its results the same way.
func encode(v any) any {
	switch x := v.(type) {
	case nil:
		return map[string]any{"t": "nil"}
	case bool:
		return map[string]any{"t": "bool", "v": x}
	case int, int8, int16, int32, int64, uint, uint8, uint16, uint32, uint64, uintptr:
		return map[string]any{"t": reflect.TypeOf(v).String(), "v": fmt.Sprint(v)}
	case float64:
		return map[string]any{"t": "float64", "v": fbits(x)}
	case float32:
		return map[string]any{"t": "float32", "v": fbits(float64(x))}
	case string:
		return map[string]any{"t": "string", "s": corpus.Encode(x)}
	case json.Number:
		return map[string]any{"t": "json.Number", "s": string(x)}
	case time.Time:
		return map[string]any{"t": "time.Time", "unix": x.Unix(), "nsec": x.Nanosecond(), "zone": zoneOf(x), "str": x.String()}
	case time.Duration:
		return map[string]any{"t": "time.Duration", "v": fmt.Sprint(int64(x))}
	case time.Month:
		return map[string]any{"t": "time.Month", "v": fmt.Sprint(int(x))}
	case time.Weekday:
		return map[string]any{"t": "time.Weekday", "v": fmt.Sprint(int(x))}
	case template.HTML, template.URL, template.JS, template.CSS, template.HTMLAttr, template.JSStr, template.Srcset:
		return map[string]any{"t": reflect.TypeOf(v).String(), "s": reflect.ValueOf(v).String()}
	case []byte:
		if x == nil {
			return map[string]any{"t": "nil:[]uint8"}
		}
		return map[string]any{"t": "[]uint8", "hex": hex.EncodeToString(x)}
	case MyInt, MyInt8, MyInt64, MyUint, MyUint8, MyUintptr, MyFloat, MyFloat32, MyString, MyBool:
		rv := reflect.ValueOf(v)
		var u any
		switch rv.Kind() {
		case reflect.Int:
			u = int(rv.Int())
		case reflect.Int8:
			u = int8(rv.Int())
		case reflect.Int64:
			u = rv.Int()
		case reflect.Uint:
			u = uint(rv.Uint())
		case reflect.Uint8:
			u = uint8(rv.Uint())
		case reflect.Uintptr:
			u = uintptr(rv.Uint())
		case reflect.Float64:
			u = rv.Float()
		case reflect.Float32:
			u = float32(rv.Float())
		case reflect.String:
			u = rv.String()
		case reflect.Bool:
			u = rv.Bool()
		}
		return map[string]any{"t": "named", "name": reflect.TypeOf(v).String(), "under": encode(u)}
	case stringer:
		return map[string]any{"t": "stringer", "s": x.s}
	case myErr:
		return map[string]any{"t": "error", "s": x.msg}
	case f64p:
		return map[string]any{"t": "f64p", "v": fbits(x.v)}
	case f64ep:
		return map[string]any{"t": "f64ep", "v": fbits(x.v), "fail": x.fail}
	case *int:
		if x == nil {
			return map[string]any{"t": "nil:*int"}
		}
	case *time.Time:
		if x == nil {
			return map[string]any{"t": "nil:*time.Time"}
		}
	case maps.Params:
		return encodeMap("maps.Params", reflect.ValueOf(v))
	}
	rv := reflect.ValueOf(v)
	switch rv.Kind() {
	case reflect.Slice:
		t := rv.Type().String()
		if rv.IsNil() {
			return map[string]any{"t": "nil:" + t}
		}
		items := []any{}
		for i := 0; i < rv.Len(); i++ {
			items = append(items, encode(rv.Index(i).Interface()))
		}
		return map[string]any{"t": t, "items": items}
	case reflect.Map:
		t := rv.Type().String()
		if rv.IsNil() {
			return map[string]any{"t": "nil:" + t}
		}
		return encodeMap(t, rv)
	}
	log.Fatalf("cannot encode %T", v)
	return nil
}

func encodeMap(t string, rv reflect.Value) any {
	var keys []string
	for _, k := range rv.MapKeys() {
		keys = append(keys, k.String())
	}
	sort.Strings(keys)
	entries := []any{}
	for _, k := range keys {
		entries = append(entries, []any{corpus.Encode(k), encode(rv.MapIndex(reflect.ValueOf(k).Convert(rv.Type().Key())).Interface())})
	}
	return map[string]any{"t": t, "entries": entries}
}

// zoneOf returns the index of t's location in zones, or -1 for a location
// the parser made up (fixed zones from numeric offsets and abbreviations),
// recorded by name and offset through "str".
func zoneOf(t time.Time) any {
	for i, z := range zones {
		if z.loc == t.Location() {
			return i
		}
	}
	name, off := t.Zone()
	return map[string]any{"name": t.Location().String(), "abbr": name, "offset": off}
}

func readZone(zipPath, name string) []byte {
	zr, err := zip.OpenReader(zipPath)
	if err != nil {
		log.Fatal(err)
	}
	var data []byte
	for _, f := range zr.File {
		if f.Name != name {
			continue
		}
		rc, err := f.Open()
		if err != nil {
			log.Fatal(err)
		}
		data, err = io.ReadAll(rc)
		if err != nil {
			log.Fatal(err)
		}
		if err := rc.Close(); err != nil {
			log.Fatal(err)
		}
		break
	}
	if err := zr.Close(); err != nil {
		log.Fatal(err)
	}
	if data == nil {
		log.Fatalf("zone %s not in %s", name, zipPath)
	}
	return data
}

// timeFormats are the layouts of cast's internal.TimeFormats (an internal
// package, so copied here).
var timeFormats = []string{
	"2006-01-02", time.RFC3339, "2006-01-02T15:04:05", time.RFC1123Z, time.RFC1123, time.RFC822Z,
	time.RFC822, time.RFC850, "2006-01-02 15:04:05.999999999 -0700 MST", "2006-01-02T15:04:05-0700",
	"2006-01-02 15:04:05Z0700", "2006-01-02 15:04:05", time.ANSIC, time.UnixDate, time.RubyDate,
	"2006-01-02 15:04:05Z07:00", "02 Jan 2006", "2006-01-02 15:04:05 -07:00", "2006-01-02 15:04:05 -0700",
	time.Kitchen, time.Stamp, time.StampMilli, time.StampMicro, time.StampNano,
}

// upstreamLiterals returns the string literals of spf13/cast's *_test.go files.
func upstreamLiterals() []string {
	cmd := exec.Command("go", "list", "-m", "-f", "{{.Dir}}", "github.com/spf13/cast")
	cmd.Stderr = os.Stderr
	dir, err := cmd.Output()
	if err != nil {
		log.Fatal(err)
	}
	files, err := filepath.Glob(filepath.Join(strings.TrimSpace(string(dir)), "*_test.go"))
	if err != nil {
		log.Fatal(err)
	}
	set := map[string]bool{}
	for _, f := range files {
		b, err := os.ReadFile(f)
		if err != nil {
			log.Fatal(err)
		}
		fset := token.NewFileSet()
		var sc scanner.Scanner
		sc.Init(fset.AddFile(f, fset.Base(), len(b)), b, nil, 0)
		for {
			_, tok, lit := sc.Scan()
			if tok == token.EOF {
				break
			}
			if tok != token.STRING {
				continue
			}
			if s, err := strconv.Unquote(lit); err == nil {
				set[s] = true
			}
		}
	}
	var out []string
	for s := range set {
		out = append(out, s)
	}
	sort.Strings(out)
	return out
}

func inputs() []any {
	var in []any
	add := func(v ...any) { in = append(in, v...) }

	add(nil, true, false)

	// Integers of every kind (values that fit the kind).
	ints := []int64{0, 1, -1, 8, -8, 127, 128, -128, -129, 255, 256, 32767, 32768, -32768, -32769, 65535, 65536,
		math.MaxInt32, math.MinInt32, math.MaxInt32 + 1, math.MaxUint32, 1 << 32, 1e15, math.MaxInt64, math.MinInt64}
	for _, i := range ints {
		add(int(i), int64(i))
		if i == int64(int8(i)) {
			add(int8(i))
		}
		if i == int64(int16(i)) {
			add(int16(i))
		}
		if i == int64(int32(i)) {
			add(int32(i))
		}
	}
	uints := []uint64{0, 1, 8, 127, 128, 255, 256, 65535, 65536, math.MaxUint32, 1 << 32, math.MaxInt64, math.MaxInt64 + 1, math.MaxUint64}
	for _, u := range uints {
		add(uint(u), uint64(u), uintptr(u))
		if u == uint64(uint8(u)) {
			add(uint8(u))
		}
		if u == uint64(uint16(u)) {
			add(uint16(u))
		}
		if u == uint64(uint32(u)) {
			add(uint32(u))
		}
	}

	// Floats, including out-of-range values for every integer kind.
	floats := []float64{0, math.Copysign(0, -1), 0.5, -0.5, 0.9999, 1, -1, 1.5, -1.5, 2.5, 8.3, -8.3, 127.9, 128,
		-128.5, -129, 255.5, 256, 32767.5, 32768, 65535.9, 65536, -65536, 1e10, -1e10, 2147483647.5, 2147483648,
		-2147483649, 4294967295.5, 4294967296, 1 << 53, 9007199254740993, 1 << 62, 1 << 63, -(1 << 63), 1 << 64,
		1e19, -1e19, 1e20, 1e300, -1e300, math.MaxFloat64, -math.MaxFloat64, math.SmallestNonzeroFloat64,
		math.MaxFloat32, 3.4028236e38, 16777217, 3.14159265358979, 0.1, 0.1 + 0.2, 1e-7, 123456789.125, 1e21,
		1e20 + 1, 5e-324, math.NaN(), math.Inf(1), math.Inf(-1)}
	for _, f := range floats {
		add(f, float32(f))
	}

	// Strings: numbers, booleans, durations, fields, JSON.
	add(
		"", "0", "1", "-1", "8", "-8", "8.3", "-8.3", "8.0", "08", "010", "09", "0x1F", "0X1f", "-0x10", "0o17",
		"0O17", "0b101", "0B11", "1_000", "1__0", "_1", "+5", "+-5", "--5", "++5", " 5", "5 ", "5x", "abc",
		"true", "false", "True", "TRUE", "t", "f", "T", "F", "yes", "no", "on", "1.", ".5", "-.5", "+.5", "+.",
		"-.", "-", "+", ".", "..", "1..2", "1.2.3", "1.5e3", "1e3", "1E3", "1e-3", "1e400", "-1e400", "1e-400",
		"NaN", "nan", "Inf", "+Inf", "-inf", "infinity", "-Infinity", "0x1p-2", "0x1.8p1", "9223372036854775807",
		"9223372036854775808", "-9223372036854775808", "-9223372036854775809", "18446744073709551615",
		"18446744073709551616", "127", "128", "-128", "-129", "255", "256", "32767", "32768", "65535", "65536",
		"2147483647", "2147483648", "4294967295", "4294967296", "3.4028235e38", "3.5e38", "1e-50", "-0", "+0",
		"0.0", "-0.0", "00", "007", "٣", "１２", "8.", "8.00", "+8.3", "-8.", "8.3.", "0.5x", "1,000",
		"5ns", "5us", "5µs", "5μs", "5ms", "5s", "5m", "5h", "1h30m", "1.5h", "-2m", "+2m", "300ms", "1d",
		"1 h", "h", "m", "µs", "5hx", "2h45m30.5s", "9223372036854775807ns", "9223372036854775808ns", "1e3ns",
		"a b  c", " x ", "\ta\nb\v", "one", "a b", "a b", "a\u0085b", "\xffab c",
		`{"a":1,"b":"x"}`, `{"a":"x","b":"y"}`, `{"a":true,"b":false}`, `{"a":null}`, `[1,2]`, `null`,
		`{"a":{"b":1}}`, `{"a":1}`, `{"a":"1"}`, `{"a":[1]}`, `{"7":1}`, `{invalid`, `{"a":1} x`, `{}`,
		`{"a":1,"a":2}`, `"str"`, `true`, `12`, `{"a":1e400}`, `{"a":1e400,"b":"x"}`, `{"b":true,"a":1e400}`,
		` {"a":"x"} `, `[{}]`, `{"a":"x"}{`,
	)
	// Dates in every layout of internal.TimeFormats, and edge dates.
	add(
		"2006-01-02", "2024-02-29", "2023-02-29", "2020-08-14", "0001-01-01", "0000-01-01", "9999-12-31",
		"10000-01-01", "2006-1-2", "2016-03-06T15:28:01Z", "2016-03-06T15:28:01-07:00",
		"2016-03-06T15:28:01+07:00", "2021-06-09T10:00:00+07:00", "2019-12-31T23:59:59.999Z",
		"2016-03-06T15:28:01.123456789Z", "2016-03-06T15:28:01", "2016-03-06T15:28:01.5",
		"Mon, 02 Jan 2006 15:04:05 -0700", "Tue, 02 Jan 2006 15:04:05 -0700", "Mon, 02 Jan 2006 15:04:05 MST",
		"Mon, 02 Jan 2006 15:04:05 UTC", "Mon, 02 Jan 2006 15:04:05 EST", "Mon, 02 Jan 2006 15:04:05 GMT",
		"02 Jan 06 15:04 -0700", "02 Jan 06 15:04 MST", "Monday, 02-Jan-06 15:04:05 MST",
		"2006-01-02 15:04:05.999999999 -0700 MST", "2006-01-02 15:04:05.123 +0700 +07",
		"2006-01-02 15:04:05 +0000 UTC", "2006-01-02T15:04:05-0700", "2006-01-02 15:04:05Z0700",
		"2006-01-02 15:04:05Z", "2006-01-02 15:04:05-0700", "2006-01-02 15:04:05", "2006-01-02 15:04:05.5",
		"Mon Jan  2 15:04:05 2006", "Mon Jan 2 15:04:05 2006", "Mon Jan  2 15:04:05 MST 2006",
		"Mon Jan 02 15:04:05 -0700 2006", "2006-01-02 15:04:05+07:00", "02 Jan 2006", "2 Jan 2006",
		"2006-01-02 15:04:05 -07:00", "2006-01-02 15:04:05 -0700", "3:04PM", "3:04pm", "12:00AM",
		"Jan  2 15:04:05", "Jan 2 15:04:05", "Jan  2 15:04:05.000", "Jan  2 15:04:05.000000",
		"Jan  2 15:04:05.000000000", "2021-03-14 02:30:00", "2021-11-07 01:30:00", "2021-03-14T02:30:00",
		"2021-11-07T01:30:00", "2006-01-02T24:00:00", "2006-13-02", "2006-00-10", "2006-01-32",
		"not a date", "2006-01-02 ", " 2006-01-02", "2006-01-02T15:04:05ZZ",
	)
	for m := time.January; m <= time.December; m++ {
		add(fmt.Sprintf("15 %s 2020", m.String()[:3]), fmt.Sprintf("15 %s 2020", m.String()))
	}
	for _, d := range []string{"Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "sun", "MON"} {
		add(fmt.Sprintf("%s, 02 Jan 2006 15:04:05 -0700", d))
	}

	// json.Number
	for _, s := range []string{"123", "123.0", "123.00", "123.5", "1e3", "-0", "abc", "", "9223372036854775808",
		"0x10", "1.000", "-8.30", "0.0", "1.10", "10"} {
		add(json.Number(s))
	}

	// time.Time
	add(time.Time{}, time.Date(2020, 8, 14, 12, 30, 45, 123, time.UTC),
		time.Date(2021, 6, 9, 10, 0, 0, 0, zones[1].loc), time.Date(2021, 6, 9, 10, 0, 0, 0, zones[2].loc),
		time.Date(2021, 3, 14, 3, 0, 0, 0, zones[3].loc), time.Unix(0, 0))

	// time.Duration, time.Month, time.Weekday
	add(time.Duration(0), time.Duration(5), time.Duration(-5), time.Hour, time.Duration(math.MaxInt64),
		time.Duration(math.MinInt64), time.March, time.December, time.Month(13), time.Month(0), time.Month(-1),
		time.Sunday, time.Tuesday, time.Weekday(7), time.Weekday(-1))

	// html/template string types
	add(template.HTML("5"), template.HTML("<b>x</b>"), template.URL("true"), template.JS("1.5"),
		template.CSS("x"), template.HTMLAttr("0"), template.JSStr("8"), template.Srcset("9"), template.HTML(""),
		template.JS("2h"), template.CSS(`{"a":1}`))

	// []byte
	add([]byte("hello"), []byte{}, []byte(nil), []byte("12"), []byte("\xff"))

	// Named basic types
	add(MyInt(8), MyInt(-8), MyInt8(-8), MyInt64(1<<40), MyUint(8), MyUint8(200), MyUintptr(5), MyFloat(1.5),
		MyFloat(-2.5), MyFloat32(1.5), MyString("5"), MyString("true"), MyString("1h"), MyString("a b"),
		MyString(""), MyBool(true), MyBool(false))

	// Stringer, error, Float64 providers
	add(stringer{"hello"}, stringer{"42"}, stringer{""}, myErr{"boom"}, myErr{"7"}, f64p{1.5}, f64p{-2},
		f64p{math.NaN()}, f64ep{3.5, false}, f64ep{-1, false}, f64ep{0, true})

	// typed nil pointers
	add((*int)(nil), (*time.Time)(nil))

	// Slices
	add([]any{}, []any{1, "2", 3.5, true, nil}, []any{"a", []any{"b"}}, []any{"a", map[string]any{}},
		[]string{"a", "b"}, []string{}, []string{"1", "2", "x"}, []string{"1", "2"}, []int{1, 2, 3},
		[]int64{1, 2}, []float64{1.5, 2}, []bool{true, false}, []any{"1", "2", "x"}, []any{"1", 2, 3.0},
		[]any{"true", 0, 1.5}, []any{"5s", 5, "1h"}, []map[string]any{{"a": 1}}, []time.Duration{1, 2},
		[]string(nil), []any(nil), []any{json.Number("5"), template.HTML("6")}, []any{[]byte("x")},
		[]any{stringer{"s"}, myErr{"e"}}, []uint8{1, 2}, []any{nil, nil}, []any{int8(-1), uint8(200)})

	// Every layout of internal.TimeFormats applied to reference times (as in
	// cast's TestTimeWithTimezones), and the string literals of cast's own
	// *_test.go files.
	refs := []time.Time{
		time.Date(2016, time.January, 1, 0, 0, 0, 0, zones[3].loc),
		time.Date(2021, time.July, 4, 15, 4, 5, 123456789, zones[3].loc),
		time.Date(2009, time.November, 10, 23, 0, 0, 0, time.UTC),
		time.Date(2020, time.August, 14, 9, 5, 0, 0, zones[1].loc),
		time.Date(2020, time.February, 29, 12, 0, 0, 0, zones[2].loc),
	}
	for _, layout := range timeFormats {
		for _, r := range refs {
			add(r.Format(layout))
		}
	}
	for _, lit := range upstreamLiterals() {
		add(lit)
	}
	add(json.Number("1234567890"), json.Number("123.4567890"), int32(1234567890), uint32(1234567890))

	// Maps
	add(map[string]any{"a": 1, "b": "x", "c": true, "d": nil, "e": 1.5, "f": []any{1}},
		map[string]any{"a": "true", "b": 0, "c": "no"}, map[string]string{"a": "x", "b": "true"},
		map[string]bool{"a": true, "b": false}, maps.Params{"a": 1, "b": "x"}, map[string]any{},
		map[string]any(nil), map[string]string(nil), map[string]bool(nil), map[string]int{"a": 1},
		map[string]any{"x": map[string]any{"y": 1}})

	return in
}
