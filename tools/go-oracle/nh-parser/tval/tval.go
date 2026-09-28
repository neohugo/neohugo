// Package tval encodes decoded front matter, config and data values as typed
// JSON for the nh-parser and nh-langs oracles (Wave B task T03). It extends the
// format of ../../nh-common/goval with go-toml's local date and time types:
//
//	nil                 {"t":"nil"}
//	bool                {"t":"bool","v":true}
//	int*/uint*          {"t":"int64","v":"-12"}                (decimal string)
//	float32/float64     {"t":"float64","v":"<16 hex digits of the float64 bits>"}
//	string              {"t":"string","s":<string or {"hex":...}>}
//	time.Time           {"t":"time.Time","unix":..,"nsec":..,"loc":..,"abbr":..,"off":..}
//	toml.LocalDate      {"t":"toml.LocalDate","s":String(),"f":[y,m,d],"utc":<AsTime(UTC)>,"ict":<AsTime(ICT)>}
//	toml.LocalTime      {"t":"toml.LocalTime","s":String(),"f":[h,m,s,ns,precision]}
//	toml.LocalDateTime  {"t":"toml.LocalDateTime","s":..,"f":[y,m,d,h,m,s,ns,precision],"utc":..,"ict":..}
//	slices              {"t":"[]interface {}","items":[...]}  / {"t":"nil:[]interface {}"}
//	maps (string keys)  {"t":"map[string]interface {}","entries":[[key, value], ...]} (sorted keys)
//
// ICT is time.FixedZone("ICT", 7*3600) (no tzdata needed).
package tval

import (
	"fmt"
	"reflect"
	"sort"
	"time"

	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	toml "github.com/pelletier/go-toml/v2"
)

// ICT is the fixed zone the local dates are also converted to.
var ICT = time.FixedZone("ICT", 7*3600)

func encodeTime(x time.Time) any {
	abbr, off := x.Zone()
	return map[string]any{"t": "time.Time", "unix": x.Unix(), "nsec": x.Nanosecond(), "loc": x.Location().String(), "abbr": abbr, "off": off}
}

// Encode returns v in the typed JSON format.
func Encode(v any) any {
	switch x := v.(type) {
	case nil:
		return map[string]any{"t": "nil"}
	case bool:
		return map[string]any{"t": "bool", "v": x}
	case int, int8, int16, int32, int64, uint, uint8, uint16, uint32, uint64, uintptr:
		return map[string]any{"t": reflect.TypeOf(v).String(), "v": fmt.Sprint(v)}
	case float64:
		return map[string]any{"t": "float64", "v": goval.FBits(x)}
	case float32:
		return map[string]any{"t": "float32", "v": goval.FBits(float64(x))}
	case string:
		return map[string]any{"t": "string", "s": goval.Str(x)}
	case time.Time:
		return encodeTime(x)
	case toml.LocalDate:
		return map[string]any{
			"t": "toml.LocalDate", "s": x.String(), "f": []int{x.Year, x.Month, x.Day},
			"utc": encodeTime(x.AsTime(time.UTC)), "ict": encodeTime(x.AsTime(ICT)),
		}
	case toml.LocalTime:
		return map[string]any{
			"t": "toml.LocalTime", "s": x.String(),
			"f": []int{x.Hour, x.Minute, x.Second, x.Nanosecond, x.Precision},
		}
	case toml.LocalDateTime:
		return map[string]any{
			"t": "toml.LocalDateTime", "s": x.String(),
			"f":   []int{x.Year, x.Month, x.Day, x.Hour, x.Minute, x.Second, x.Nanosecond, x.Precision},
			"utc": encodeTime(x.AsTime(time.UTC)), "ict": encodeTime(x.AsTime(ICT)),
		}
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
		if rv.Type().Key().Kind() != reflect.String {
			panic(fmt.Sprintf("tval: map key type %s", rv.Type().Key()))
		}
		var keys []string
		for _, k := range rv.MapKeys() {
			keys = append(keys, k.String())
		}
		sort.Strings(keys)
		entries := []any{}
		for _, k := range keys {
			entries = append(entries, []any{goval.Str(k), Encode(rv.MapIndex(reflect.ValueOf(k).Convert(rv.Type().Key())).Interface())})
		}
		return map[string]any{"t": t, "entries": entries}
	}
	panic(fmt.Sprintf("tval: cannot encode %T", v))
}
