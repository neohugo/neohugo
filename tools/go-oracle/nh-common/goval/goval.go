// Package goval encodes Go values as typed JSON for the nh-common oracles
// (values, math, hashing, compare). The Rust tests decode the same format into
// go_value::Value (crates/nh-common/tests/support/mod.rs) and encode their
// results back, so every Go type distinction that reaches output (int vs int64,
// []string vs []interface {}, maps.Params vs map[string]interface {}, typed
// nils) is compared exactly.
//
// Format ({"t": <Go type>, ...}):
//
//	nil                      {"t":"nil"}
//	bool                     {"t":"bool","v":true}
//	int*/uint*               {"t":"int64","v":"-12"}          (decimal string)
//	float32/float64          {"t":"float64","v":"<16 hex digits of the float64 bits>"}
//	string                   {"t":"string","s":<string or {"hex":...}>}
//	html/template types      {"t":"template.HTML","s":...}
//	time.Time                {"t":"time.Time","unix":..,"nsec":..,"loc":..,"abbr":..,"off":..}
//	named basic types        {"t":"named","name":"maps.ParamsMergeStrategy","under":<value>}
//	slices                   {"t":"[]string","items":[...]}  / {"t":"nil:[]string"}
//	maps (string keys)       {"t":"maps.Params","entries":[[key, value], ...]} (sorted keys) / {"t":"nil:..."}
//	go-toml local dates      {"t":"toml.LocalDate","s":"2020-01-02"}
//	test objects             {"t":"*main.pg","id":"..."}      (see Obj)
//
// Shallow encodes a map by its keys ({"t":..,"keys":[..]}) and a slice by its
// length ({"t":..,"len":n}). Results are {"ok": v}, {"err": msg} or
// {"panic": msg}.
package goval

import (
	"bytes"
	"compress/gzip"
	"encoding/hex"
	"fmt"
	"html/template"
	"math"
	"os"
	"reflect"
	"sort"
	"time"
	"unicode/utf8"

	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/corpus"
)

type asTimeStringer interface {
	AsTime(*time.Location) time.Time
	String() string
}

// Obj is implemented by the test objects of an oracle (encoded as {"t": Type(), "id": ID()}).
type Obj interface {
	GoType() string
	ID() string
}

// Str returns s as a JSON-able value: the string itself when it is valid
// UTF-8, else {"hex": ...}.
func Str(s string) any {
	if utf8.ValidString(s) {
		return s
	}
	return map[string]string{"hex": hex.EncodeToString([]byte(s))}
}

// FBits returns the IEEE bits of f as 16 hex digits.
func FBits(f float64) string { return fmt.Sprintf("%016x", math.Float64bits(f)) }

// Encode returns v in the typed JSON format.
func Encode(v any) any {
	switch x := v.(type) {
	case nil:
		return map[string]any{"t": "nil"}
	case Obj:
		if reflect.ValueOf(x).Kind() == reflect.Ptr && reflect.ValueOf(x).IsNil() {
			return map[string]any{"t": "nil:" + x.GoType()}
		}
		return map[string]any{"t": x.GoType(), "id": x.ID()}
	case bool:
		return map[string]any{"t": "bool", "v": x}
	case int, int8, int16, int32, int64, uint, uint8, uint16, uint32, uint64, uintptr:
		return map[string]any{"t": reflect.TypeOf(v).String(), "v": fmt.Sprint(v)}
	case float64:
		return map[string]any{"t": "float64", "v": FBits(x)}
	case float32:
		return map[string]any{"t": "float32", "v": FBits(float64(x))}
	case string:
		return map[string]any{"t": "string", "s": Str(x)}
	case template.HTML, template.URL, template.JS, template.CSS, template.HTMLAttr, template.JSStr, template.Srcset:
		return map[string]any{"t": reflect.TypeOf(v).String(), "s": Str(reflect.ValueOf(v).String())}
	case time.Time:
		abbr, off := x.Zone()
		return map[string]any{"t": "time.Time", "unix": x.Unix(), "nsec": x.Nanosecond(), "loc": x.Location().String(), "abbr": abbr, "off": off}
	case maps.Params:
		if x == nil {
			return map[string]any{"t": "nil:maps.Params"}
		}
		return encodeMap("maps.Params", reflect.ValueOf(v))
	case asTimeStringer:
		// go-toml's LocalDate, LocalTime, LocalDateTime (front matter).
		return map[string]any{"t": reflect.TypeOf(v).String(), "s": x.String()}
	}
	rv := reflect.ValueOf(v)
	t := rv.Type().String()
	switch rv.Kind() {
	case reflect.Slice:
		if rv.IsNil() {
			return map[string]any{"t": "nil:" + t}
		}
		items := []any{}
		for i := 0; i < rv.Len(); i++ {
			items = append(items, Encode(rv.Index(i).Interface()))
		}
		return map[string]any{"t": t, "items": items}
	case reflect.Map:
		if rv.IsNil() {
			return map[string]any{"t": "nil:" + t}
		}
		return encodeMap(t, rv)
	case reflect.Ptr:
		if rv.IsNil() {
			return map[string]any{"t": "nil:" + t}
		}
	case reflect.Bool, reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64,
		reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64,
		reflect.Float32, reflect.Float64, reflect.String:
		// A named basic type.
		var u any
		switch rv.Kind() {
		case reflect.Bool:
			u = rv.Bool()
		case reflect.Int:
			u = int(rv.Int())
		case reflect.Int8:
			u = int8(rv.Int())
		case reflect.Int16:
			u = int16(rv.Int())
		case reflect.Int32:
			u = int32(rv.Int())
		case reflect.Int64:
			u = rv.Int()
		case reflect.Uint:
			u = uint(rv.Uint())
		case reflect.Uint8:
			u = uint8(rv.Uint())
		case reflect.Uint16:
			u = uint16(rv.Uint())
		case reflect.Uint32:
			u = uint32(rv.Uint())
		case reflect.Uint64:
			u = rv.Uint()
		case reflect.Float32:
			u = float32(rv.Float())
		case reflect.Float64:
			u = rv.Float()
		case reflect.String:
			u = rv.String()
		}
		return map[string]any{"t": "named", "name": t, "under": Encode(u)}
	}
	panic(fmt.Sprintf("goval: cannot encode %T", v))
}

func encodeMap(t string, rv reflect.Value) any {
	if rv.Type().Key().Kind() != reflect.String {
		panic(fmt.Sprintf("goval: map key type %s", rv.Type().Key()))
	}
	var keys []string
	for _, k := range rv.MapKeys() {
		keys = append(keys, k.String())
	}
	sort.Strings(keys)
	entries := []any{}
	for _, k := range keys {
		entries = append(entries, []any{Str(k), Encode(rv.MapIndex(reflect.ValueOf(k).Convert(rv.Type().Key())).Interface())})
	}
	return map[string]any{"t": t, "entries": entries}
}

// Shallow encodes v like Encode, except that a non-nil map becomes
// {"t": type, "keys": [sorted keys]} and a non-nil slice {"t": type, "len": n}
// (for results that are a map or slice taken unchanged from a larger value
// whose full encoding is checked elsewhere).
func Shallow(v any) any {
	if v == nil {
		return Encode(v)
	}
	rv := reflect.ValueOf(v)
	switch rv.Kind() {
	case reflect.Map:
		if rv.IsNil() || rv.Type().Key().Kind() != reflect.String {
			return Encode(v)
		}
		var keys []string
		for _, k := range rv.MapKeys() {
			keys = append(keys, k.String())
		}
		sort.Strings(keys)
		enc := make([]any, len(keys))
		for i, k := range keys {
			enc[i] = Str(k)
		}
		return map[string]any{"t": typeName(v), "keys": enc}
	case reflect.Slice:
		if rv.IsNil() {
			return Encode(v)
		}
		return map[string]any{"t": typeName(v), "len": rv.Len()}
	}
	return Encode(v)
}

func typeName(v any) string {
	if _, ok := v.(maps.Params); ok {
		return "maps.Params"
	}
	return reflect.TypeOf(v).String()
}

// CallRaw runs f, which returns an already encoded value, as {"ok": v},
// {"err": message} or {"panic": message}.
func CallRaw(f func() (any, error)) (out map[string]any) {
	defer func() {
		if r := recover(); r != nil {
			out = map[string]any{"panic": fmt.Sprint(r)}
		}
	}()
	v, err := f()
	if err != nil {
		return map[string]any{"err": err.Error()}
	}
	return map[string]any{"ok": v}
}

// WriteCasesGz writes corpus.WriteCases' output gzip-compressed (best
// compression, no header name or time) to path.
func WriteCasesGz(path string, header map[string]any, cases []map[string]any) error {
	tmp := path + ".tmp"
	if err := corpus.WriteCases(tmp, header, cases); err != nil {
		return err
	}
	b, err := os.ReadFile(tmp)
	if err != nil {
		return err
	}
	if err := os.Remove(tmp); err != nil {
		return err
	}
	var buf bytes.Buffer
	zw, err := gzip.NewWriterLevel(&buf, gzip.BestCompression)
	if err != nil {
		return err
	}
	if _, err := zw.Write(b); err != nil {
		return err
	}
	if err := zw.Close(); err != nil {
		return err
	}
	return os.WriteFile(path, buf.Bytes(), 0o644)
}

// Result encodes (v, err) as {"ok": v} or {"err": message}.
func Result(v any, err error) map[string]any {
	if err != nil {
		return map[string]any{"err": err.Error()}
	}
	return map[string]any{"ok": Encode(v)}
}

// Call runs f and encodes its result, turning a panic into {"panic": message}.
func Call(f func() (any, error)) (out map[string]any) {
	defer func() {
		if r := recover(); r != nil {
			out = map[string]any{"panic": fmt.Sprint(r)}
		}
	}()
	return Result(f())
}
