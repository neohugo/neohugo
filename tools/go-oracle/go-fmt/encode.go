package main

// This file is also copied (via go:embed) into the temporary program that
// runs Go's own fmt_test.go tables (see fmttests.go), so it must only use
// the standard library and must not depend on the rest of the oracle.

import (
	"encoding/hex"
	"fmt"
	"math"
	"reflect"
	"sort"
	"strconv"
	"strings"
	"time"
)

var encTimeType = reflect.TypeOf(time.Time{})

var encSafeNames = map[string]string{
	"template.HTML":     "html",
	"template.HTMLAttr": "htmlattr",
	"template.CSS":      "css",
	"template.JS":       "js",
	"template.JSStr":    "jsstr",
	"template.URL":      "url",
	"template.Srcset":   "srcset",
}

func encHex(s string) string { return hex.EncodeToString([]byte(s)) }

// encSpec encodes an `any` as a value spec (see spec.go), or reports that
// the value model cannot express it.
func encSpec(a any) (string, bool) {
	if a == nil {
		return "nil", true
	}
	return encValue(reflect.ValueOf(a))
}

func encValue(v reflect.Value) (string, bool) {
	t := v.Type()
	if t.PkgPath() == "" && t.Name() != "" {
		// Predeclared (unnamed-package) basic types only.
		switch t.Kind() {
		case reflect.Bool:
			if v.Bool() {
				return "bool:1", true
			}
			return "bool:0", true
		case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
			return t.Name() + ":" + strconv.FormatInt(v.Int(), 10), true
		case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64, reflect.Uintptr:
			return t.Name() + ":" + strconv.FormatUint(v.Uint(), 10), true
		case reflect.Float32:
			return fmt.Sprintf("f32:%08x", math.Float32bits(float32(v.Float()))), true
		case reflect.Float64:
			return fmt.Sprintf("f64:%016x", math.Float64bits(v.Float())), true
		case reflect.String:
			return "str:" + encHex(v.String()), true
		}
		return "", false
	}
	if t == encTimeType {
		tm := v.Interface().(time.Time)
		loc := tm.Location()
		var ls string
		switch {
		case loc == time.UTC:
			ls = "nil"
		case strings.Contains(loc.String(), "/"):
			ls = "zone=" + loc.String()
		default:
			name, off := tm.Zone()
			if name != loc.String() {
				return "", false
			}
			ls = "fixed=" + name + "=" + strconv.Itoa(off)
		}
		// The value model has no monotonic clock reading.
		if tm != tm.Round(0) {
			return "", false
		}
		return "time:" + strconv.FormatInt(tm.Unix(), 10) + ";" + strconv.Itoa(tm.Nanosecond()) + ";" + ls, true
	}
	if t.Kind() == reflect.String {
		if n, ok := encSafeNames[t.String()]; ok {
			return n + ":" + encHex(v.String()), true
		}
		return "", false
	}
	if encHook != nil {
		if s, ok := encHook(v); ok {
			return s, true
		}
	}
	switch t.Kind() {
	case reflect.Slice:
		if v.IsNil() {
			return "tnil:" + encHex(t.String()), true
		}
		if t.String() == "[]uint8" {
			return "bytes:" + hex.EncodeToString(v.Bytes()), true
		}
		var parts []string
		for i := 0; i < v.Len(); i++ {
			s, ok := encElem(v.Index(i))
			if !ok {
				return "", false
			}
			parts = append(parts, s)
		}
		return "list:" + encHex(t.String()) + "(" + strings.Join(parts, ",") + ")", true
	case reflect.Map:
		if t.Key() != reflect.TypeOf("") {
			return "", false
		}
		if v.IsNil() {
			return "tnil:" + encHex(t.String()), true
		}
		keys := v.MapKeys()
		sort.Slice(keys, func(i, j int) bool { return keys[i].String() < keys[j].String() })
		var parts []string
		for _, k := range keys {
			s, ok := encElem(v.MapIndex(k))
			if !ok {
				return "", false
			}
			parts = append(parts, "kv:"+encHex(k.String())+"("+s+")")
		}
		return "map:" + encHex(t.String()) + "(" + strings.Join(parts, ",") + ")", true
	case reflect.Pointer, reflect.Func, reflect.Chan, reflect.Interface:
		if v.IsNil() {
			return "tnil:" + encHex(t.String()), true
		}
	}
	return "", false
}

func encElem(v reflect.Value) (string, bool) {
	if v.Kind() == reflect.Interface {
		if v.IsNil() {
			return "nil", true
		}
		return encValue(v.Elem())
	}
	return encValue(v)
}

// encHook, when set, encodes the oracle's own types.
var encHook func(v reflect.Value) (string, bool)
