package main

import (
	"fmt"
	"reflect"
	"strings"

	"github.com/neohugo/neohugo/common/hreflect"
	"github.com/neohugo/neohugo/common/types"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
)

// redact replaces error texts that print pointer addresses (a %v/%#v of a
// slice of pointers), which differ from run to run.
func redact(v any, r map[string]any) map[string]any {
	if _, ok := r["err"]; ok && strings.Contains(fmt.Sprintf("%#v", v), "(0x") {
		return map[string]any{"err": "<redacted>"}
	}
	return r
}

// reflectCases runs the hreflect predicates and the common/types conversions
// over values of every kind.
func reflectCases() []map[string]any {
	var cases []map[string]any
	for _, v := range values() {
		c := map[string]any{"v": goval.Encode(v)}
		c["truthful"] = hreflect.IsTruthful(v)
		c["isSlice"] = hreflect.IsSlice(v)
		c["isMap"] = hreflect.IsMap(v)
		c["isNil"] = types.IsNil(v)
		c["isValid"] = hreflect.IsValid(reflect.ValueOf(v))
		c["isNumber"] = hreflect.IsNumber(reflect.ValueOf(v).Kind())
		c["toSliceAny"] = goval.Call(func() (any, error) {
			s, ok := hreflect.ToSliceAny(v)
			return []any{s, ok}, nil
		})
		c["toStringSlicePreserveString"] = redact(v, goval.Call(func() (any, error) { return types.ToStringSlicePreserveStringE(v) }))
		c["toString"] = redact(v, goval.Call(func() (any, error) { return types.ToStringE(v) }))
		c["typeToString"] = goval.Call(func() (any, error) {
			s, ok := types.TypeToString(v)
			return []any{s, ok}, nil
		})
		c["toDuration"] = redact(v, goval.Call(func() (any, error) { return types.ToDurationE(v) }))
		cases = append(cases, c)
	}
	return cases
}
