package mdoracle

import (
	"bytes"
	"encoding/json"
	"fmt"
	"reflect"
	"sort"
	"strings"
	"unicode/utf8"

	"github.com/neohugo/neohugo/markup/markup_config"
)

// SortedKeys returns the keys of a string-keyed map in byte order.
func SortedKeys[V any](m map[string]V) []string {
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	return keys
}

// DumpMarkupConfig is the canonical text of a decoded markup config: one
// "<field path>=<value>" line per leaf (the Rust test writes the same
// lines field by field).
func DumpMarkupConfig(m markup_config.Config) string {
	var b strings.Builder
	dumpValue(&b, "Markup", reflect.ValueOf(m))
	return b.String()
}

func dumpValue(b *strings.Builder, path string, v reflect.Value) {
	switch v.Kind() {
	case reflect.Struct:
		t := v.Type()
		for i := 0; i < t.NumField(); i++ {
			dumpValue(b, path+"."+t.Field(i).Name, v.Field(i))
		}
	case reflect.Ptr:
		if v.IsNil() {
			fmt.Fprintf(b, "%s=nil\n", path)
			return
		}
		dumpValue(b, path+"*", v.Elem())
	case reflect.Slice, reflect.Array:
		fmt.Fprintf(b, "%s=len %d\n", path, v.Len())
		for i := 0; i < v.Len(); i++ {
			dumpValue(b, fmt.Sprintf("%s[%d]", path, i), v.Index(i))
		}
	case reflect.Map:
		keys := v.MapKeys()
		sort.Slice(keys, func(i, j int) bool { return keys[i].String() < keys[j].String() })
		fmt.Fprintf(b, "%s=len %d\n", path, v.Len())
		for _, k := range keys {
			dumpValue(b, fmt.Sprintf("%s[%s]", path, k.String()), v.MapIndex(k))
		}
	default:
		fmt.Fprintf(b, "%s=%v\n", path, v.Interface())
	}
}

// B is a byte string in the fixtures: a JSON string when it is valid UTF-8,
// else {"b64": "<base64>"} (Go's encoder would replace invalid bytes).
type B []byte

// MarshalJSON implements json.Marshaler.
func (b B) MarshalJSON() ([]byte, error) {
	if utf8.Valid(b) {
		var buf bytes.Buffer
		enc := json.NewEncoder(&buf)
		enc.SetEscapeHTML(false)
		if err := enc.Encode(string(b)); err != nil {
			return nil, err
		}
		return bytes.TrimRight(buf.Bytes(), "\n"), nil
	}
	return json.Marshal(map[string][]byte{"b64": b})
}
