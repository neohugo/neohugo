package main

import (
	"html/template"
	"sort"

	"github.com/neohugo/neohugo/common/collections"
	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
)

func sortStrings(s []string) { sort.Strings(s) }

// appendTos are the `to` values of collections.Append.
func appendTos() []any {
	var nilStrings []string
	var nilAny []any
	var nilPgs pgs
	return []any{
		nil, []any{}, []any{"a"}, []any{"a", nil}, nilAny, []string{}, []string{"a", "b"}, nilStrings,
		[]int{1}, []int64{1}, []float64{1.5}, [][]string{{"a", "b"}}, [][]string{}, []map[string]any{{"k": 1}},
		[]map[string]any{}, pgs{p1}, pgs{}, nilPgs, []template.HTML{"<a>"}, []maps.Params{{"a": 1}},
		"", "ab", 3, maps.Params{}, map[string]any{}, true,
	}
}

// appendFroms are the `from ...any` argument lists of collections.Append.
func appendFroms() [][]any {
	var nilStrings []string
	return [][]any{
		{}, {"c"}, {"c", "d"}, {[]string{"c", "d"}}, {[]string{}}, {nilStrings}, {nil}, {nil, "d", nil},
		{template.HTML("c")}, {"b", template.HTML("c")}, {1}, {1, 2}, {int64(1)}, {[]int{1, 2}}, {[]any{"x", 1}},
		{[]any{}}, {map[string]any{"k": 2}}, {maps.Params{"p": 1}}, {map[string]any{"k": 2}, maps.Params{"p": 1}},
		{p2}, {p2, p3}, {pgs{p2}}, {p2, "x"}, {[]string{"c"}, []string{"d"}}, {[]int{1}, []int{2}},
		{[][]string{{"z"}}}, {1.5}, {true}, {[]template.HTML{"<b>"}},
	}
}

// sliceArgs are the argument lists of collections.Slice.
func sliceArgs() [][]any {
	var nilStrings []string
	return [][]any{
		{}, {nil}, {nil, "a"}, {"a"}, {"a", "b"}, {"a", 1}, {1, 2}, {1, int64(2)}, {int64(1)}, {1.5, 2.5},
		{true}, {uint8(1)}, {template.HTML("a"), template.HTML("b")}, {template.HTML("a"), "b"},
		{map[string]any{"k": 1}, map[string]any{}}, {maps.Params{"a": 1}}, {maps.Params{"a": 1}, map[string]any{}},
		{[]string{"a"}, []string{"b"}}, {[]string{"a"}, nilStrings}, {[]any{}}, {p1}, {p1, p2}, {p1, "x"}, {"x", p1},
		{pgs{p1}}, {map[string]string{"a": "b"}}, {float32(1)}, {[]int{1}},
	}
}

func collectionsCases() []map[string]any {
	var cases []map[string]any
	for _, to := range appendTos() {
		for _, from := range appendFroms() {
			var enc []any
			for _, f := range from {
				enc = append(enc, goval.Encode(f))
			}
			cases = append(cases, map[string]any{
				"op": "append", "to": goval.Encode(to), "from": enc,
				"r": goval.Call(func() (any, error) { return collections.Append(to, from...) }),
			})
		}
	}
	for _, args := range sliceArgs() {
		var enc []any
		for _, a := range args {
			enc = append(enc, goval.Encode(a))
		}
		cases = append(cases, map[string]any{
			"op": "slice", "args": enc,
			"r": goval.Call(func() (any, error) { return collections.Slice(args...), nil }),
		})
	}
	return cases
}
