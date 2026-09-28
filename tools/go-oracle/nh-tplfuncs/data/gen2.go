package main

import (
	"html/template"
	"math"

	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/resources/page"
)

var whereOps = []any{"", "=", "==", "eq", "!=", "<>", "ne", ">", ">=", "ge", "gt", "<", "<=", "lt", "le", "in", "not in", "intersect", "like", "bogus", " IN "}

// genWhere: where over maps, params, test objects and mixed lists, with
// nested keys, every operator and match values of every kind
// (where_test.go).
func genWhere(o *oracle) {
	seqs := []any{
		[]map[string]any{
			{"a": 1, "b": "x", "c": map[string]any{"d": 1}, "tags": []string{"a", "b"}, "n": []int{1, 2}, "flag": true, "d": t1},
			{"a": 2, "b": "y", "c": map[string]any{"d": 2}, "tags": []string{"c"}, "n": []int{3}, "flag": false, "d": t3},
			{"a": 3, "b": "x", "tags": []any{"a", 1}, "n": []any{2, "x"}, "d": t2},
			{"b": nil, "c": nil},
			{"a": 2.5, "b": template.HTML("x"), "c": maps.Params{"d": "2"}},
			{"a": "2", "b": 2, "c": "d"},
			{"a": int64(2), "b": uint(2), "flag": "true"},
		},
		[]maps.Params{{"a": 1, "b": maps.Params{"c": "x"}}, {"a": 2, "B": "y"}, {}, {"a": nil}},
		[]any{map[string]any{"a": 1}, maps.Params{"a": 2}, nil, obj1, "str", 3, []int{1}},
		[]*tstObj{obj1, obj2, obj3},
		[]any{obj1, obj2, obj3, nil},
		[]map[string]int{{"a": 1, "b": 2}, {"a": 3, "b": 4}, {"a": 5, "x": 4}},
		[]map[string]float64{{"a": 1, "b": 2}, {"a": 3, "b": 4}, {"a": 5, "x": 4}},
		map[string]any{"x": []any{map[string]any{"a": 1}}, "y": []map[string]any{{"a": 2}}, "z": 3, "w": nil, "v": []map[string]any{}},
		[]string{"a", "b"},
	}
	keys := []any{"a", "b", "c.d", ".a", "a.", "missing", "tags", "n", "flag", "d", "A", "B", "GetA", "Double", "Fail", "WithArg", "P.k", "c.missing", "", 1, template.HTML("a"), nil}
	vals := []any{1, 2, 2.0, int64(2), uint(2), "x", "2", nil, true, []string{"a", "x"}, []any{1, "x"}, []int{1, 2}, []string{}, t1, template.HTML("x"), "^x", []float64{2}}
	for _, s := range seqs {
		for _, k := range keys {
			for _, v := range vals {
				o.add("where", "collections", "Where", s, k, v)
			}
			for _, op := range whereOps {
				for _, v := range vals {
					o.add("where_ops", "collections", "Where", s, k, op, v)
				}
			}
		}
	}
	o.add("where", "collections", "Where", seqs[0], "a")
	o.add("where", "collections", "Where", seqs[0], "a", "=", 1, 2)
	o.add("where", "collections", "Where", seqs[0], "a", 1, 1)
	o.add("where", "collections", "Where", nil, "a", 1)
	o.add("where", "collections", "Where", (*tstObj)(nil), "a", 1)
	o.add("where", "collections", "Where", "abc", "a", 1)
	o.add("where", "collections", "Where", []map[string]any(nil), "a", 1)
	o.add("where", "collections", "Where", seqs[0])
}

// genSort: sort by value, key paths and methods, asc/desc, with the en and
// th collators (sort_test.go).
func genSort(o *oracle) {
	lists := []any{
		[]int{3, 1, 2, 1},
		[]string{"b", "A", "a", "ข้าว", "ขนม", "กล้วย", "ไก่", "เค้ก", "é", "e", "10", "9", "1e2", "Inf", "NaN", "", "B"},
		[]any{3, "b", 1.5, nil, "a", true, t1, "10", uint(2), int64(-1)},
		[]map[string]any{{"k": 2, "s": "b", "m": map[string]any{"x": 2}}, {"k": 1, "s": "a"}, {"s": "c", "m": map[string]any{"x": 1}}, {"k": "2", "s": "ข"}, {"k": nil, "s": "ก"}, {"k": 1.5, "s": "B"}},
		[]maps.Params{{"a": maps.Params{"b": 2}, "t": "x"}, {"a": maps.Params{"b": 1}, "t": "Y"}, {}},
		map[string]any{"z": 1, "a": 3, "m": 2},
		map[string]int{"b": 2, "a": 1, "c": 3},
		map[string]string{"x": "ข้าว", "y": "ขนม", "z": "a"},
		[]*tstObj{obj2, obj1, obj3},
		[]any{obj2, obj1, obj3},
		[]float64{2.5, math.NaN(), -1, math.Inf(1), 0},
		[]any{t2, t1, t3},
		[]string{},
		[]string(nil),
		"abc",
		(*tstObj)(nil),
	}
	keys := []any{nil, "value", "k", "s", "m.x", "a.b", "t", "A", "B", "GetA", "Double", "Fail", "WithArg", "missing", "missing.x", "k.x", ".s.", 1, template.HTML("s"), "P.k"}
	orders := []any{nil, "asc", "desc", "DESC", 1}
	for _, lang := range []string{"collections", "collections@th"} {
		for _, l := range lists {
			o.add("sort", lang, "Sort", l)
			for _, k := range keys {
				for _, ord := range orders {
					if ord == nil {
						o.add("sort", lang, "Sort", l, k)
					} else {
						o.add("sort", lang, "Sort", l, k, ord)
					}
				}
			}
		}
	}
	o.add("sort", "collections", "Sort", nil)
	o.add("sort", "collections", "Sort", []int{1}, "value", "desc", "x")
}

// genPages: the functions over page values (and the wrapped pages of taxonomy
// terms and GetTerms).
func genPages(o *oracle, pages map[string]page.Pages) {
	regular := pages["regular"]
	weighted := pages["weighted"]
	ordinals := pages["ordinals"]
	nodes := pages["nodes"]

	lists := []any{regular, weighted, ordinals, nodes, pagesAny(regular), page.Pages{}}
	whereKeys := []any{"Section", "Type", "Kind", "Title", "Weight", "Date", "Params.x", "Params.s", "Params.f", "Params.nested.k", "Params.NESTED.K", "Params.nested.n", "Params.flag", "Params.list", "Params.when", "params.x", "IsPage", "Lang", "Missing", ".Params.type", "Params", "Params.missing.x", "Weight0", "Ordinal", "Path"}
	whereVals := []any{"snack", "a", "b", 1, 2, 2.5, "3", true, []string{"go", "a"}, []string{"a", "b"}, t1, nil, "v1", 0}
	for _, l := range lists[:5] {
		for _, k := range whereKeys {
			for _, v := range whereVals {
				o.add("pages_where", "collections", "Where", l, k, v)
				for _, op := range []any{"!=", ">", "<=", "in", "not in", "intersect"} {
					o.add("pages_where", "collections", "Where", l, k, op, v)
				}
			}
		}
	}
	sortKeys := []any{nil, "Title", "Weight", "Date", "Params.x", "Params.s", "Params.nested.n", "Params.f", "LinkTitle", "Section", "Params", "Weight0", "Missing"}
	for _, lang := range []string{"collections", "collections@th"} {
		for _, l := range lists {
			for _, k := range sortKeys {
				o.add("pages_sort", lang, "Sort", l, k)
				o.add("pages_sort", lang, "Sort", l, k, "desc")
			}
		}
	}

	var all []any
	all = append(all, pagesAny(regular)...)
	all = append(all, pagesAny(weighted)...)
	all = append(all, pagesAny(ordinals)...)
	all = append(all, pagesAny(nodes)...)
	others := []any{nil, "Alpha", 1, obj1, []any{}}
	for _, a := range all {
		for _, b := range append(append([]any{}, all...), others...) {
			for _, m := range []string{"Eq", "Ne", "Lt", "Ge"} {
				o.add("pages_compare", "compare", m, a, b)
			}
		}
		o.add("pages_compare", "compare", "Default", "x", a)
		o.add("pages_compare", "compare", "Conditional", a, 1, 2)
		for _, l := range lists {
			o.add("pages_sets", "collections", "In", l, a)
		}
		o.add("pages_misc", "collections", "KeyVals", "k", a)
		o.add("pages_misc", "collections", "Group", "k", a)
		o.add("pages_misc", "fmt", "Printf", "%T", a)
		o.add("pages_misc", "reflect", "IsMap", a)
		o.add("pages_misc", "collections", "Index", a, 0)
	}
	for _, l1 := range lists {
		o.add("pages_sets", "collections", "Uniq", l1)
		o.add("pages_sets", "collections", "Reverse", l1)
		o.add("pages_misc", "collections", "Group", "k", l1)
		o.add("pages_misc", "collections", "Index", l1, 1)
		o.add("pages_misc", "collections", "IsSet", l1, 2)
		for _, n := range []any{0, 1, 3, 100} {
			o.add("pages_misc", "collections", "First", n, l1)
			o.add("pages_misc", "collections", "Last", n, l1)
			o.add("pages_misc", "collections", "After", n, l1)
		}
		for _, l2 := range lists {
			for _, m := range []string{"Union", "Intersect", "SymDiff", "Complement", "Append"} {
				o.add("pages_sets", "collections", m, l1, l2)
			}
		}
		o.add("pages_misc", "collections", "Append", regular[0], l1)
		o.add("pages_misc", "collections", "Append", weighted[0], regular[1], l1)
	}
	o.add("pages_sets", "collections", "Uniq", []any{regular[0], weighted[0], regular[0], ordinals[0], ordinals[1]})
	o.add("pages_misc", "collections", "Slice", regular[0], regular[1])
	o.add("pages_misc", "collections", "Slice", weighted[0], weighted[1])
	o.add("pages_misc", "collections", "Slice", regular[0], "a")
	o.add("pages_misc", "collections", "Slice", ordinals[0])
	o.add("pages_misc", "collections", "Append", regular[0], []any{})
	o.add("pages_misc", "collections", "Append", regular[0], nil)
	o.add("pages_misc", "collections", "Group", "k", []any{regular[0]})
}

// genTables: the remaining inputs of the Go test tables (crypto_test.go,
// encoding_test.go, hash_test.go, safe_test.go, cast_test.go).
func genTables(o *oracle) {
	for _, h := range []any{"md5", "sha1", "sha256", "sha512", "sha384", "MD5", 1, nil} {
		for _, k := range []any{"Secret key", "", 1, nil, template.HTML("k")} {
			for _, m := range []any{"Hello world, gophers!", "", 42} {
				o.add("crypto_hmac", "crypto", "HMAC", h, k, m)
				o.add("crypto_hmac", "crypto", "HMAC", h, k, m, "binary")
				o.add("crypto_hmac", "crypto", "HMAC", h, k, m, "hex")
			}
		}
	}
	o.add("crypto_hmac", "crypto", "HMAC", "sha256", "k", "m", "base64")
	o.add("crypto_hmac", "crypto", "HMAC", "sha256", "k", "m", nil)
	o.add("crypto_hmac", "crypto", "HMAC", "sha256", "k", "m", 1)
	o.add("crypto_hmac", "crypto", "HMAC", "sha256", "k", "m", "hex", "x")
	o.add("crypto_hmac", "crypto", "HMAC", "sha256", "k")
	o.add("crypto_hmac", "crypto", "HMAC", "sha256", string(make([]byte, 200)), "m")
	o.add("crypto_hmac", "crypto", "HMAC", "sha512", string(make([]byte, 200)), "m")

	for _, s := range []any{
		"", "YQ==", "YWI=", "YWJj", "YWJjZA==", "YWJjZA=", "YWJjZA", "YWJjZA==x", "YWJjZA===", "YW Jj", "YWJj\n", "YW\r\nJj", "\n", "=",
		"Y", "YW", "YWJ", "Y===", "YW==", "YWJ=", "YQ==YQ==", "!!!!", "YWJjZGVmZ2hpams=", "YWJjZGVmZ2hpamts", "YWJjZGVmZ2hpamt=\n",
		"YWJjZGVm\nZ2hpamts", "YWJjZGVmZ2hpams==", "YWJjZGVmZ2hp!mts", "4pyTIMOgIGxhIG1vZGU=", "8J+YgA==", "/+/+", "-_-_", 1, template.HTML("YWJj"),
	} {
		o.add("encoding_base64", "encoding", "Base64Decode", s)
		o.add("encoding_base64", "encoding", "Base64Encode", s)
	}
	objs := []any{
		map[string]any{"a": "<b>", "b": []int{1, 2}, "c": map[string]any{"d": "e&f"}, "e": nil}, []any{"x", 1.5, true, nil, t1}, "<script>",
		maps.Params{"z": 1, "a": maps.Params{"b": "c"}}, template.HTML("<b>"), 1e21, float32(0.1), math.NaN(), map[string]any{}, []string(nil),
		obj1, json01(),
	}
	optsList := []any{
		map[string]any{"indent": "  "}, map[string]any{"prefix": "> ", "indent": "\t"}, map[string]any{"noHTMLEscape": true},
		map[string]any{"NoHTMLEscape": "true", "Indent": 2}, map[string]any{"indent": true}, map[string]any{"noHTMLEscape": "maybe"},
		map[string]any{"noHTMLEscape": ""}, map[string]any{"noHTMLEscape": 1, "prefix": 3.5, "indent": " "}, map[string]any{"indent": []int{1}},
		map[string]any{"indent": nil}, map[string]any{}, maps.Params{"indent": "  "}, "x", nil, map[string]any{"indent": []uint8{32, 32}},
		map[string]any{"noHTMLEscape": []int{1}, "indent": map[string]any{}},
	}
	for _, v := range objs {
		o.add("encoding_jsonify", "encoding", "Jsonify", v)
		for _, opts := range optsList {
			o.add("encoding_jsonify", "encoding", "Jsonify", opts, v)
		}
	}
	o.add("encoding_jsonify", "encoding", "Jsonify", 1, 2, 3)
}

// json01 is a nested value with every JSON kind.
func json01() any {
	return map[string]any{"s": "é ", "i": int64(-1), "u": uint8(3), "f": 0.1, "b": false, "n": nil, "l": []any{map[string]any{}}, "h": template.HTML("<>")}
}
