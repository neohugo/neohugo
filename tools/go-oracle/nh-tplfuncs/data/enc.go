package main

import (
	"encoding/json"
	"fmt"
	"html/template"
	"math"
	"reflect"
	"sort"
	"time"
	"unicode/utf8"

	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/common/types"
	"github.com/neohugo/neohugo/resources/page"
)

// str returns s as a JSON-able value: the string itself when it is valid
// UTF-8, else {"hex": ...}.
func str(s string) any {
	if utf8.ValidString(s) {
		return s
	}
	return map[string]string{"hex": fmt.Sprintf("%x", s)}
}

func fbits(f float64) string { return fmt.Sprintf("%016x", math.Float64bits(f)) }

// encoder encodes values in the typed JSON format of the nh-common oracles
// (goval), extended with pages ({"t": %T, "page": entry}), page groups,
// types.KeyValues and *maps.Scratch.
type encoder struct {
	pages *pageTable
}

func (e *encoder) enc(v any) any {
	switch x := v.(type) {
	case nil:
		return map[string]any{"t": "nil"}
	case page.Page:
		if reflect.ValueOf(x).Kind() == reflect.Ptr && reflect.ValueOf(x).IsNil() {
			return map[string]any{"t": "nil:" + reflect.TypeOf(x).String()}
		}
		return map[string]any{"t": fmt.Sprintf("%T", x), "page": e.pages.add(x)}
	case page.PageGroup:
		return map[string]any{"t": "page.PageGroup", "key": e.enc(x.Key), "pages": e.enc(x.Pages)}
	case types.KeyValues:
		return map[string]any{"t": "types.KeyValues", "key": e.enc(x.Key), "values": e.enc(x.Values)}
	case *maps.Scratch:
		return map[string]any{"t": "*maps.Scratch"}
	case bool:
		return map[string]any{"t": "bool", "v": x}
	case int, int8, int16, int32, int64, uint, uint8, uint16, uint32, uint64, uintptr:
		return map[string]any{"t": reflect.TypeOf(v).String(), "v": fmt.Sprint(v)}
	case float64:
		return map[string]any{"t": "float64", "v": fbits(x)}
	case float32:
		return map[string]any{"t": "float32", "v": fbits(float64(x))}
	case string:
		return map[string]any{"t": "string", "s": str(x)}
	case template.HTML, template.URL, template.JS, template.CSS, template.HTMLAttr, template.JSStr, template.Srcset:
		return map[string]any{"t": reflect.TypeOf(v).String(), "s": str(reflect.ValueOf(v).String())}
	case time.Time:
		abbr, off := x.Zone()
		return map[string]any{"t": "time.Time", "unix": x.Unix(), "nsec": x.Nanosecond(), "loc": x.Location().String(), "abbr": abbr, "off": off}
	case maps.Params:
		if x == nil {
			return map[string]any{"t": "nil:maps.Params"}
		}
		return e.encMap("maps.Params", reflect.ValueOf(v))
	case *tstObj:
		if x == nil {
			return map[string]any{"t": "nil:*main.tstObj"}
		}
		return map[string]any{"t": "*main.tstObj", "id": x.ID}
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
			items = append(items, e.enc(rv.Index(i).Interface()))
		}
		return map[string]any{"t": t, "items": items}
	case reflect.Map:
		if rv.IsNil() {
			return map[string]any{"t": "nil:" + t}
		}
		return e.encMap(t, rv)
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
		return map[string]any{"t": "named", "name": t, "under": e.enc(u)}
	}
	panic(fmt.Sprintf("enc: cannot encode %T", v))
}

func (e *encoder) encMap(t string, rv reflect.Value) any {
	if rv.Type().Key().Kind() != reflect.String {
		panic(fmt.Sprintf("enc: map key type %s", rv.Type().Key()))
	}
	var keys []string
	for _, k := range rv.MapKeys() {
		keys = append(keys, k.String())
	}
	sort.Strings(keys)
	entries := []any{}
	for _, k := range keys {
		entries = append(entries, []any{str(k), e.enc(rv.MapIndex(reflect.ValueOf(k).Convert(rv.Type().Key())).Interface())})
	}
	return map[string]any{"t": t, "entries": entries}
}

// key returns the canonical JSON text of an encoded value.
func key(v any) string {
	b, err := json.Marshal(v)
	if err != nil {
		panic(err)
	}
	return string(b)
}
