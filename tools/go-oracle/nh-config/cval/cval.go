// Package cval dumps decoded Go config values as JSON for the nh-config and
// nh-media oracles (Wave B task T04). Dynamic values (interface fields, maps
// of interface values, oracle inputs) use the typed format of
// ../../nh-common/goval; static struct values are dumped field by field:
//
//	struct              {"@": "<Go type>", "<Field>": <dump>, ...} (exported fields)
//	string              goval.Str
//	bool                true/false
//	int*/uint*          JSON number (exact)
//	float32/float64     {"f": "<16 hex digits of the float64 bits>"}
//	slice/array         [<dump>, ...] (a nil slice is [])
//	map[string]T        {"@map": "<Go type>", "entries": [[key, <dump>], ...]} (sorted keys)
//	pointer             null or <dump of the element>
//	interface           goval.Encode (typed)
//
// A few unexported fields that decide behaviour are added: security.Whitelist
// ("acceptNone", "patterns") and media.Type ("mimeSuffix").
package cval

import (
	"fmt"
	"reflect"
	"sort"

	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
)

// Dump returns v in the dump format.
func Dump(v any) any {
	return dump(reflect.ValueOf(v))
}

func dump(rv reflect.Value) any {
	if !rv.IsValid() {
		return nil
	}
	t := rv.Type()
	switch rv.Kind() {
	case reflect.Interface:
		if rv.IsNil() {
			return goval.Encode(nil)
		}
		if !rv.CanInterface() {
			return dump(rv.Elem())
		}
		return Enc(rv.Interface())
	case reflect.Ptr:
		if rv.IsNil() {
			return nil
		}
		return dump(rv.Elem())
	case reflect.String:
		return goval.Str(rv.String())
	case reflect.Bool:
		return rv.Bool()
	case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
		return rv.Int()
	case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64, reflect.Uintptr:
		return rv.Uint()
	case reflect.Float32, reflect.Float64:
		return map[string]any{"f": goval.FBits(rv.Float())}
	case reflect.Slice, reflect.Array:
		out := []any{}
		for i := 0; i < rv.Len(); i++ {
			out = append(out, dump(rv.Index(i)))
		}
		return out
	case reflect.Map:
		if t.Key().Kind() != reflect.String {
			panic(fmt.Sprintf("cval: map key type %s", t.Key()))
		}
		var keys []string
		for _, k := range rv.MapKeys() {
			keys = append(keys, k.String())
		}
		sort.Strings(keys)
		entries := []any{}
		for _, k := range keys {
			entries = append(entries, []any{goval.Str(k), dump(rv.MapIndex(reflect.ValueOf(k).Convert(t.Key())))})
		}
		return map[string]any{"@map": t.String(), "entries": entries}
	case reflect.Struct:
		out := map[string]any{"@": t.String()}
		for i := 0; i < t.NumField(); i++ {
			f := t.Field(i)
			if f.PkgPath != "" {
				continue
			}
			out[f.Name] = dump(rv.Field(i))
		}
		switch t.String() {
		case "security.Whitelist":
			out["acceptNone"] = rv.FieldByName("acceptNone").Bool()
			ps := rv.FieldByName("patternsStrings")
			if ps.IsNil() {
				out["patterns"] = nil
			} else {
				var p []any
				for i := 0; i < ps.Len(); i++ {
					p = append(p, goval.Str(ps.Index(i).String()))
				}
				if p == nil {
					p = []any{}
				}
				out["patterns"] = p
			}
		case "media.Type":
			out["mimeSuffix"] = goval.Str(rv.FieldByName("mimeSuffix").String())
		}
		return out
	case reflect.Func, reflect.Chan:
		return nil
	}
	panic(fmt.Sprintf("cval: cannot dump %s", t))
}

// Enc encodes a dynamic value like goval.Encode, and dumps struct values.
func Enc(v any) (out any) {
	if v == nil {
		return goval.Encode(nil)
	}
	rv := reflect.ValueOf(v)
	if rv.Kind() == reflect.Struct && rv.Type().String() != "time.Time" {
		return map[string]any{"t": "struct", "v": Dump(v)}
	}
	return goval.Encode(v)
}

// Result is {"ok": dump(v)} or {"err": message}.
func Result(v any, err error) map[string]any {
	if err != nil {
		return map[string]any{"err": goval.Str(err.Error()), "ok": Dump(v)}
	}
	return map[string]any{"ok": Dump(v)}
}

// Call runs f and returns Result, or {"panic": message}.
func Call(f func() (any, error)) (out map[string]any) {
	defer func() {
		if r := recover(); r != nil {
			out = map[string]any{"panic": goval.Str(fmt.Sprint(r))}
		}
	}()
	return Result(f())
}
