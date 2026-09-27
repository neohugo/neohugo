package main

// The "vdump" format: a binary-safe, typed serialisation of Go values shared
// with the Rust tests (crates/go-json/tests/common/mod.rs). It is used in both
// directions: random Go values are written as vdumps so the Rust side can
// rebuild the same go_value::Value, and decoded values are written as vdumps
// so the Rust decoder's output can be compared exactly.
//
//	N                         nil interface
//	Z <str>                   typed nil of the named Go type
//	T | F                     bool
//	i <k> <dec> ;             int kinds: 0 int 1 int8 2 int16 3 int32 4 int64
//	u <k> <dec> ;             uint kinds: 0 uint 1 uint8 2 uint16 3 uint32 4 uint64 5 uintptr
//	d <16 hex>                float64 bits
//	e <8 hex>                 float32 bits
//	s <str>                   string
//	h <k> <str>               html/template string types: 0 HTML 1 HTMLAttr 2 CSS 3 JS 4 JSStr 5 URL 6 Srcset
//	t <sec> ; <nsec> ; U      time.Time in UTC
//	t <sec> ; <nsec> ; F <str> <off> ;   time.Time in a FixedZone(name, off)
//	a <t> <count> [ ... ]     slices: A []any S []string I []int L []int64 D []float64 B []bool Y []uint8 M []map[string]any
//	m <t> <count> { (<str> value)* }   maps: A map[string]any S map[string]string P maps.Params
//	n <str>                   json.Number
//	J <str>                   main.jm: MarshalJSON returns the bytes
//	E <str>                   main.jerr: MarshalJSON returns an error with this message
//	X <str>                   main.tm: MarshalText returns the bytes
//	W <str>                   main.tmerr: MarshalText returns an error with this message
//	R <count> { (<str> value)* }        struct with exported interface{} fields (names in order)
//	P <count> { (<flags> <str> value)* } struct with json tags: flags bits 1 omitempty 2 omitzero
//	                                     4 string 8 interface-typed; <str> is the resolved JSON name
//
// <str> is <len> ':' <bytes>; <dec>, <count>, <len>, <sec>, <nsec>, <off>
// are decimal; <k>, <t>, <flags> are single characters.

import (
	"encoding/json"
	"errors"
	"fmt"
	"html/template"
	"math"
	"reflect"
	"strconv"
	"time"

	"github.com/neohugo/neohugo/common/maps"
)

type jm struct{ b string }

func (m jm) MarshalJSON() ([]byte, error) { return []byte(m.b), nil }

type jerr struct{ msg string }

func (m jerr) MarshalJSON() ([]byte, error) { return nil, errors.New(m.msg) }

type tm struct{ b string }

func (m tm) MarshalText() ([]byte, error) { return []byte(m.b), nil }

type tmerr struct{ msg string }

func (m tmerr) MarshalText() ([]byte, error) { return nil, errors.New(m.msg) }

func putStr(b []byte, s string) []byte {
	b = strconv.AppendInt(b, int64(len(s)), 10)
	b = append(b, ':')
	return append(b, s...)
}

// dump writes the vdump of v (only the types listed above).
func dump(b []byte, v any) []byte {
	switch x := v.(type) {
	case nil:
		return append(b, 'N')
	case bool:
		if x {
			return append(b, 'T')
		}
		return append(b, 'F')
	case int:
		return appendInt(b, '0', int64(x))
	case int8:
		return appendInt(b, '1', int64(x))
	case int16:
		return appendInt(b, '2', int64(x))
	case int32:
		return appendInt(b, '3', int64(x))
	case int64:
		return appendInt(b, '4', x)
	case uint:
		return appendUint(b, '0', uint64(x))
	case uint8:
		return appendUint(b, '1', uint64(x))
	case uint16:
		return appendUint(b, '2', uint64(x))
	case uint32:
		return appendUint(b, '3', uint64(x))
	case uint64:
		return appendUint(b, '4', x)
	case uintptr:
		return appendUint(b, '5', uint64(x))
	case float64:
		return fmt.Appendf(b, "d%016x", math.Float64bits(x))
	case float32:
		return fmt.Appendf(b, "e%08x", math.Float32bits(x))
	case string:
		return putStr(append(b, 's'), x)
	case template.HTML:
		return putStr(append(b, 'h', '0'), string(x))
	case template.HTMLAttr:
		return putStr(append(b, 'h', '1'), string(x))
	case template.CSS:
		return putStr(append(b, 'h', '2'), string(x))
	case template.JS:
		return putStr(append(b, 'h', '3'), string(x))
	case template.JSStr:
		return putStr(append(b, 'h', '4'), string(x))
	case template.URL:
		return putStr(append(b, 'h', '5'), string(x))
	case template.Srcset:
		return putStr(append(b, 'h', '6'), string(x))
	case time.Time:
		b = fmt.Appendf(b, "t%d;%d;", x.Unix(), x.Nanosecond())
		name, off := x.Zone()
		if x.Location() == time.UTC {
			return append(b, 'U')
		}
		b = putStr(append(b, 'F'), name)
		return fmt.Appendf(b, "%d;", off)
	case json.Number:
		return putStr(append(b, 'n'), string(x))
	case jm:
		return putStr(append(b, 'J'), x.b)
	case jerr:
		return putStr(append(b, 'E'), x.msg)
	case tm:
		return putStr(append(b, 'X'), x.b)
	case tmerr:
		return putStr(append(b, 'W'), x.msg)
	case []any:
		if x == nil {
			return putStr(append(b, 'Z'), "[]interface {}")
		}
		b = fmt.Appendf(b, "aA%d[", len(x))
		for _, e := range x {
			b = dump(b, e)
		}
		return append(b, ']')
	case []string:
		if x == nil {
			return putStr(append(b, 'Z'), "[]string")
		}
		b = fmt.Appendf(b, "aS%d[", len(x))
		for _, e := range x {
			b = dump(b, e)
		}
		return append(b, ']')
	case []int:
		b = fmt.Appendf(b, "aI%d[", len(x))
		for _, e := range x {
			b = dump(b, e)
		}
		return append(b, ']')
	case []int64:
		b = fmt.Appendf(b, "aL%d[", len(x))
		for _, e := range x {
			b = dump(b, e)
		}
		return append(b, ']')
	case []float64:
		b = fmt.Appendf(b, "aD%d[", len(x))
		for _, e := range x {
			b = dump(b, e)
		}
		return append(b, ']')
	case []bool:
		b = fmt.Appendf(b, "aB%d[", len(x))
		for _, e := range x {
			b = dump(b, e)
		}
		return append(b, ']')
	case []byte:
		if x == nil {
			return putStr(append(b, 'Z'), "[]uint8")
		}
		b = fmt.Appendf(b, "aY%d[", len(x))
		for _, e := range x {
			b = dump(b, e)
		}
		return append(b, ']')
	case []map[string]any:
		b = fmt.Appendf(b, "aM%d[", len(x))
		for _, e := range x {
			b = dump(b, e)
		}
		return append(b, ']')
	case map[string]any:
		if x == nil {
			return putStr(append(b, 'Z'), "map[string]interface {}")
		}
		b = fmt.Appendf(b, "mA%d{", len(x))
		for _, k := range sortedKeys(x) {
			b = putStr(b, k)
			b = dump(b, x[k])
		}
		return append(b, '}')
	case maps.Params:
		if x == nil {
			return putStr(append(b, 'Z'), "maps.Params")
		}
		b = fmt.Appendf(b, "mP%d{", len(x))
		for _, k := range sortedKeys(x) {
			b = putStr(b, k)
			b = dump(b, x[k])
		}
		return append(b, '}')
	case map[string]string:
		b = fmt.Appendf(b, "mS%d{", len(x))
		for _, k := range sortedKeys(x) {
			b = putStr(b, k)
			b = dump(b, x[k])
		}
		return append(b, '}')
	case *int:
		if x == nil {
			return putStr(append(b, 'Z'), "*int")
		}
	case *time.Time:
		if x == nil {
			return putStr(append(b, 'Z'), "*time.Time")
		}
	case genStruct:
		return x.dump(b)
	}
	panic(fmt.Sprintf("dump: unsupported %T", v))
}

func appendInt(b []byte, k byte, i int64) []byte {
	b = append(b, 'i', k)
	b = strconv.AppendInt(b, i, 10)
	return append(b, ';')
}

func appendUint(b []byte, k byte, u uint64) []byte {
	b = append(b, 'u', k)
	b = strconv.AppendUint(b, u, 10)
	return append(b, ';')
}

// genStruct is a generated struct value (built with reflect.StructOf) and
// its dump. Marshal is called on value.
type genStruct struct {
	value any
	d     []byte
}

func (g genStruct) dump(b []byte) []byte { return append(b, g.d...) }

// MarshalJSON is not defined on genStruct: the oracle always marshals
// g.value (see unwrap).

// unwrap replaces genStruct wrappers by their reflect-built struct values,
// recursively, so the value handed to encoding/json has the real types.
func unwrap(v any) any {
	switch x := v.(type) {
	case genStruct:
		return x.value
	case []any:
		if x == nil {
			return x
		}
		out := make([]any, len(x))
		for i, e := range x {
			out[i] = unwrap(e)
		}
		return out
	case map[string]any:
		if x == nil {
			return x
		}
		out := make(map[string]any, len(x))
		for k, e := range x {
			out[k] = unwrap(e)
		}
		return out
	case maps.Params:
		if x == nil {
			return x
		}
		out := make(maps.Params, len(x))
		for k, e := range x {
			out[k] = unwrap(e)
		}
		return out
	case []map[string]any:
		out := make([]map[string]any, len(x))
		for i, e := range x {
			out[i] = unwrap(e).(map[string]any)
		}
		return out
	}
	return v
}

var anyType = reflect.TypeFor[any]()
