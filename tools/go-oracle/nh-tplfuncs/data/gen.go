package main

import (
	"html/template"
	"math"
	"time"

	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/common/types/hstring"
)

// genScalar: every unary function over the corpus.
func genScalar(o *oracle) {
	unary := []struct{ ns, m string }{
		{"cast", "ToInt"}, {"cast", "ToFloat"}, {"cast", "ToString"},
		{"crypto", "MD5"}, {"crypto", "SHA1"}, {"crypto", "SHA256"}, {"crypto", "FNV32a"},
		{"hash", "FNV32a"}, {"hash", "XxHash"},
		{"encoding", "Base64Encode"}, {"encoding", "Base64Decode"}, {"encoding", "Jsonify"},
		{"reflect", "IsMap"}, {"reflect", "IsSlice"},
		{"safe", "CSS"}, {"safe", "HTML"}, {"safe", "HTMLAttr"}, {"safe", "JS"}, {"safe", "JSStr"}, {"safe", "URL"},
		{"math", "Abs"}, {"math", "Acos"}, {"math", "Asin"}, {"math", "Atan"}, {"math", "Ceil"}, {"math", "Cos"},
		{"math", "Floor"}, {"math", "Log"}, {"math", "Round"}, {"math", "Sin"}, {"math", "Sqrt"}, {"math", "Tan"},
		{"math", "ToDegrees"}, {"math", "ToRadians"}, {"math", "Max"}, {"math", "Min"}, {"math", "Sum"}, {"math", "Product"},
		{"fmt", "Print"}, {"fmt", "Println"},
		{"collections", "Reverse"}, {"collections", "Uniq"}, {"collections", "Seq"}, {"collections", "Slice"},
		{"collections", "Querify"}, {"collections", "Sort"}, {"collections", "Index"}, {"collections", "Dictionary"},
		{"collections", "Merge"}, {"collections", "Complement"}, {"collections", "Append"}, {"collections", "KeyVals"},
		{"collections", "Shuffle"},
		{"compare", "Default"}, {"compare", "Eq"},
	}
	for _, f := range unary {
		for _, v := range scalars() {
			o.add("scalar_"+f.ns, f.ns, f.m, v)
		}
	}
	// No-argument calls.
	for _, f := range []struct{ ns, m string }{
		{"math", "Pi"}, {"math", "MaxInt64"}, {"math", "Add"}, {"math", "Max"}, {"fmt", "Print"}, {"fmt", "Println"},
		{"collections", "Slice"}, {"collections", "Seq"}, {"collections", "Querify"}, {"collections", "Dictionary"},
		{"collections", "Merge"}, {"collections", "NewScratch"}, {"encoding", "Jsonify"}, {"compare", "Eq"}, {"compare", "Lt"},
		{"cast", "ToInt"}, {"collections", "Where"}, {"collections", "Sort"},
	} {
		o.add("scalar_"+f.ns, f.ns, f.m)
	}
	for range 3 {
		o.add("rand", "math", "Rand")
	}
}

// genCompare: the comparison functions over every pair of the corpus.
func genCompare(o *oracle) {
	vs := scalars()
	for _, m := range []string{"Eq", "Ne", "Lt", "Le", "Gt", "Ge"} {
		for _, a := range vs {
			for _, b := range vs {
				o.add("compare_"+m, "compare", m, a, b)
			}
		}
	}
	for _, a := range vs {
		for _, b := range small() {
			o.add("compare_Default", "compare", "Default", a, b)
		}
		o.add("compare_Conditional", "compare", "Conditional", a, "yes", "no")
	}
	sm := small()
	for _, a := range sm {
		for _, b := range sm {
			for _, c := range []any{1, "a", nil, 2.5} {
				o.add("compare_multi", "compare", "Eq", a, b, c)
				o.add("compare_multi", "compare", "Ne", a, b, c)
				o.add("compare_multi", "compare", "Lt", a, b, c)
				o.add("compare_multi", "compare", "Ge", a, b, c)
			}
		}
	}
	o.add("compare_multi", "compare", "Eq", 1)
	o.add("compare_multi", "compare", "Lt", 1)
	o.add("compare_multi", "compare", "Default", 1, 2, 3)
	o.add("compare_multi", "compare", "Conditional", true, 1)
	o.add("compare_multi", "compare", "LtCollate", nil, "a", "b")
}

// genMath: arithmetic over every pair of the corpus.
func genMath(o *oracle) {
	vs := scalars()
	for _, m := range []string{"Add", "Sub", "Mul", "Div", "Mod", "ModBool", "Pow"} {
		for _, a := range vs {
			for _, b := range vs {
				o.add("math_"+m, "math", m, a, b)
			}
		}
	}
	for _, m := range []string{"Atan2", "Max", "Min", "Sum", "Product"} {
		for _, a := range vs {
			for _, b := range small() {
				o.add("math_"+m, "math", m, a, b)
				o.add("math_"+m, "math", m, b, a)
			}
		}
	}
	nums := []any{0, 1, -1, 2, 3, 7, -7, int64(10), uint(4), 0.5, -0.5, 2.5, 1e-310, 1e308, 1.0000000000000002, math.Pi, -math.Pi, 100.123, 1e20, math.Inf(1), math.NaN(), "4", "-2.5"}
	for _, m := range []string{"Add", "Sub", "Mul", "Div", "Pow", "Atan2"} {
		for _, a := range nums {
			for _, b := range nums {
				o.add("math_nums", "math", m, a, b)
			}
		}
	}
	for _, m := range []string{"Log", "Sqrt", "Sin", "Cos", "Tan", "Asin", "Acos", "Atan", "Round", "Floor", "Ceil", "Abs", "ToDegrees", "ToRadians"} {
		for _, a := range nums {
			o.add("math_nums", "math", m, a)
		}
		for i := -40; i <= 40; i++ {
			o.add("math_nums", "math", m, float64(i)*0.37+0.013)
			o.add("math_nums", "math", m, float64(i)*123456.789)
		}
		for _, x := range []float64{1e-10, 0.7, 0.66, 0.6600000000000001, 2.41421356237309504880, 1e6, 1 << 29, 1<<29 + 0.5, 1e17, 0.49999999999999994, 2.5, -2.5, 4503599627370495.5} {
			o.add("math_nums", "math", m, x)
			o.add("math_nums", "math", m, -x)
		}
	}
	for _, m := range []string{"Add", "Sub", "Mul", "Div", "Sum", "Product", "Max", "Min"} {
		o.add("math_nums", "math", m, 1, 2, 3.5)
		o.add("math_nums", "math", m, []any{1, 2}, 3)
		o.add("math_nums", "math", m, []int{1, 2}, []float64{3, 0.5})
		o.add("math_nums", "math", m, 1, "a", 3)
		o.add("math_nums", "math", m, []any{}, []string{})
	}
}

// genFmt: printf verbs over every value, print/println.
func genFmt(o *oracle) {
	verbs := []string{
		"%v", "%+v", "%#v", "%T", "%d", "%s", "%q", "%x", "%X", "%o", "%O", "%b", "%e", "%E", "%f", "%F",
		"%.2f", "%g", "%G", "%t", "%c", "%U", "%5s", "%-5d|", "%05d", "% x", "%#x", "%+d", "%8.3f", "%.0f", "%.3g",
		"%10.4v|", "%-8q|", "%x %X", "%!", "%z", "%", "%d %d", "%[2]v %[1]v", "%[3]v", "%*d", "%.*f", "%v%%",
	}
	for _, f := range verbs {
		for _, v := range scalars() {
			if tm, ok := v.(time.Time); ok && tm.Location() != time.UTC {
				// Other verbs print the *time.Location internals.
				switch f {
				case "%v", "%+v", "%#v", "%s", "%q", "%T":
				default:
					continue
				}
			}
			o.add("fmt_printf", "fmt", "Printf", f, v)
		}
	}
	for _, v := range scalars() {
		o.add("fmt_printf", "fmt", "Printf", "%*d", v, 5)
		o.add("fmt_printf", "fmt", "Printf", v)
	}
	o.add("fmt_printf", "fmt", "Printf", "%s %d", "a")
	o.add("fmt_printf", "fmt", "Printf", "%s", "a", "b")
	o.add("fmt_printf", "fmt", "Printf", nil)
	o.add("fmt_printf", "fmt", "Printf", template.HTML("%s"), "x")
	sm := small()
	for _, a := range sm {
		for _, b := range sm {
			o.add("fmt_print", "fmt", "Print", a, b)
			o.add("fmt_print", "fmt", "Println", a, b)
			o.add("fmt_print", "fmt", "Print", a, "x", b)
		}
	}
	for _, m := range []string{"Errorf", "Warnf"} {
		o.add("fmt_log", "fmt", m, "msg %s", "x")
		o.add("fmt_log", "fmt", m, 1)
	}
	for _, m := range []string{"Erroridf", "Warnidf"} {
		o.add("fmt_log", "fmt", m, "id", "msg %d", 3)
		o.add("fmt_log", "fmt", m, "id")
	}
	for _, m := range []string{"Errormf", "Warnmf"} {
		o.add("fmt_log", "fmt", m, map[string]any{"a": 1}, "msg %d", 3)
		o.add("fmt_log", "fmt", m, nil, 1)
	}
}

// genCollections: the collection functions over the corpus.
func genCollections(o *oracle) {
	vs := scalars()
	sm := small()
	for _, m := range []string{"In", "Intersect", "Union", "SymDiff", "Complement", "Index", "Append", "Slice"} {
		for _, a := range vs {
			for _, b := range vs {
				o.add("coll_"+m, "collections", m, a, b)
			}
		}
	}
	for _, m := range []string{"After", "First", "Last", "IsSet", "Delimit", "Querify", "Merge", "Dictionary", "Seq", "Group", "KeyVals", "Sort", "Uniq", "Apply", "Where"} {
		ns := "collections"
		if m == "Apply" {
			ns = "collections@site"
		}
		for _, a := range vs {
			for _, b := range sm {
				o.add("coll_"+m, ns, m, a, b)
				o.add("coll_"+m, ns, m, b, a)
			}
		}
	}
	for _, m := range []string{"Delimit", "Append", "Slice", "Querify", "Merge", "Dictionary", "Seq", "Index", "Complement", "KeyVals", "Sort"} {
		for _, a := range sm {
			for _, b := range sm {
				for _, c := range sm {
					o.add("coll3_"+m, "collections", m, a, b, c)
				}
			}
		}
	}
	// Seq forms and limits (collections_test.go TestSeq).
	seqArgs := [][]any{
		{3}, {1, 2, 4}, {1, 4}, {-3}, {-1, -3}, {1, -2}, {0}, {5, 5}, {5, 1}, {1, 0, 5}, {5, 2, 1}, {1, -2, 5},
		{1, 2, 1, 2}, {"3"}, {"a"}, {1.9}, {int64(3)}, {uint(3)}, {nil}, {-100001}, {-99999}, {2000}, {2001},
		{1, 2000}, {1, 2001}, {0, 1999}, {-1000, 1, 1000}, {1, 1, 1}, {9223372036854775800, 1, 9223372036854775807},
		{-9223372036854775808, 1, -9223372036854775805}, {9223372036854775807, -1, 9223372036854775805},
		{1, 3, 2000}, {2000, -3, 1}, {-5, -5}, {true}, {[]int{1, 2}}, {"1", "2", "3"}, {math.MaxInt64}, {-math.MaxInt64},
	}
	for _, a := range seqArgs {
		o.add("coll_seq", "collections", "Seq", a...)
	}
	// Dictionary with nested keys (collections_test.go TestDictionary).
	dicts := [][]any{
		{"a", 1, "b", 2},
		{[]string{"a", "b"}, 1, []string{"a", "c"}, 2},
		{[]string{"a", "b", "c"}, 1, "d", 2},
		{[]string{"a"}, 1},
		{[]string{}, 1},
		{"a", 1, []string{"a", "b"}, 2},
		{[]string{"a", "b"}, 1, "a", 2},
		{[]string{"a", "b"}, 1, []string{"a", "b", "c"}, 2},
		{template.HTML("a"), 1},
		{1, 2},
		{"a"},
		{"a", map[string]any{"x": 1}, []string{"a", "y"}, 2},
		{"a", maps.Params{"x": 1}, []string{"a", "y"}, 2},
		{"a", map[string]any(nil), []string{"a", "y"}, 2},
		{[]any{"a"}, 1},
	}
	for _, d := range dicts {
		o.add("coll_dict", "collections", "Dictionary", d...)
	}
	// Querify (querify_test.go).
	for _, q := range [][]any{
		{"a", 1, "b", 2}, {"a", "b c", "d", "e&f"}, {"", 1}, {"a", 1, "b"}, {[]string{"a", "b", "c", "d"}}, {[]string{"a"}},
		{[]any{"a", 1, "b", 2.5}}, {[]any{"a", map[string]any{}}}, {map[string]any{"z": 1, "a": "x y", "é": true}},
		{maps.Params{"a": 1, "b": 2}}, {map[string]any{"": 1}}, {map[string]any{"a": []int{1}}}, {1, 2}, {"a", nil},
	} {
		o.add("coll_querify", "collections", "Querify", q...)
	}
	// Merge (merge_test.go).
	for _, m := range [][]any{
		{map[string]any{"a": 1}, map[string]any{"b": 2}},
		{map[string]any{"a": 1, "c": map[string]any{"x": 1}}, map[string]any{"A": 2, "c": map[string]any{"y": 2}}},
		{maps.Params{"a": 1, "c": maps.Params{"x": 1}}, maps.Params{"b": 2, "c": maps.Params{"y": 2}}},
		{map[string]any{"B": 1, "c": map[string]any{"X": 1}}, maps.Params{"a": 2, "c": maps.Params{"y": 2}}},
		{maps.Params{"a": 1}, map[string]any{"a": 2, "B": 3}},
		{map[string]any{"a": map[string]any{"b": 1}}, map[string]any{"a": "x"}},
		{map[string]any{"a": "x"}, map[string]any{"a": map[string]any{"b": 1}}},
		{map[string]any{"a": 1}, map[string]any{"b": 2}, map[string]any{"c": 3}},
		{nil, map[string]any{"a": 1}},
		{map[string]any{}, map[string]any{"a": 1}},
		{"a", map[string]any{"a": 1}},
		{map[string]any{"a": 1}, "b"},
		{map[string]any{"a": 1}, map[string]string{"b": "x"}},
		{map[string]string{"a": "y"}, map[string]string{"a": "x"}},
		{map[string]any{"Foo": map[string]any{"x": 1}}, map[string]any{"foo": map[string]any{"y": 1}}},
		{map[string]any{"a": 1}},
	} {
		o.add("coll_merge", "collections", "Merge", m...)
	}
	// Apply (apply_test.go) through the site's func map.
	for _, a := range [][]any{
		{[]any{"a", 1, 2.5}, "print", "."},
		{[]string{"a", "b"}, "print", "x", "."},
		{[]int{1, 2}, "add", ".", 10},
		{[]string{"a", "b"}, "safeHTML", "."},
		{[]string{"a", "b"}, "md5", "."},
		{[]string{"a", "b"}, "fmt.Printf", "%s!", "."},
		{[]string{"a", "b"}, "collections.First", 1, "."},
		{[]string{"a", "b"}, "string", "."},
		{[]string{"a", "b"}, "apply", "."},
		{[]string{"a", "b"}, "nosuchfunc", "."},
		{[]string{"a", "b"}, "fmt.Nope", "."},
		{[]string{"a", "b"}, "nons.Nope", "."},
		{"a", "print", "."},
		{nil, "print", "."},
		{(*tstObj)(nil), "print", "."},
		{[]string{}, "print", "."},
		{[]any{1, "x"}, "int", "."},
		{map[string]any{"a": 1}, "print", "."},
	} {
		o.add("coll_apply", "collections@site", "Apply", a...)
	}
	// Index (index_test.go).
	for _, a := range [][]any{
		{[]int{0, 1}, 0}, {[]int{0, 1}, 9}, {[]int{0, 1}, -1}, {[]uint{0, 1}, uint8(1)}, {[]int{0, 1}, int64(1)},
		{[][]int{{1, 2}, {3, 4}}, 1, 0}, {map[string]int{"a": 1, "b": 2}, "a"}, {map[string]int{"a": 1}, "x"},
		{map[string]map[string]int{"a": {"x": 1}}, "a", "x"}, {map[string]any{"a": nil}, "a"}, {[]any{1, nil}, 1, 0},
		{maps.Params{"a": maps.Params{"b": 1}}, "A", "B"}, {maps.Params{"a": maps.Params{"b": 1}}, []string{"a", "b"}},
		{maps.Params{"a": 1}, 1}, {map[string]any{"a": 1}, []any{"a"}}, {"abc", 1}, {"abc", 3}, {[]int{1}, nil},
		{map[string]any{"a": 1}, nil}, {map[string]any{"a": 1}, 1}, {nil, 1}, {(*tstObj)(nil), 1}, {1, 1},
		{[]int{1, 2}, []int{1}}, {[]int{1, 2}, "1"}, {map[string]any{"a": 1}, template.HTML("a")},
		{map[string]string{"a": "x"}, "b"}, {[]any{"a"}, 1.0}, {[]string{"a"}},
	} {
		o.add("coll_index", "collections", "Index", a...)
	}
	// IsSet.
	for _, a := range [][]any{
		{[]int{1, 2}, 1}, {[]int{1, 2}, 2}, {[]int{1, 2}, "1"}, {[]int{1, 2}, "a"}, {map[string]any{"a": 1}, "a"},
		{map[string]any{"a": nil}, "a"}, {map[string]any{"a": 1}, "b"}, {maps.Params{"a": 1}, "A"}, {maps.Params{"a": 1}, "a"},
		{map[string]any{"a": 1}, template.HTML("a")}, {map[string]int{"a": 1}, "a"}, {"abc", 1}, {nil, 1}, {obj1, "A"},
	} {
		o.add("coll_isset", "collections", "IsSet", a...)
	}
	// First, Last, After (collections_test.go).
	for _, m := range []string{"First", "Last", "After"} {
		for _, n := range []any{0, 1, 2, 3, 10, -1, "2", 1.9, nil, "a", int64(1), uint(2)} {
			for _, l := range []any{[]int{1, 2, 3}, []string{"a", "b", "c"}, "abcdef", template.HTML("<b>"), []any{}, nil, []string(nil), (*tstObj)(nil), 5, map[string]any{"a": 1}} {
				o.add("coll_firstlast", "collections", m, n, l)
			}
		}
	}
	// Delimit (collections_test.go TestDelimit).
	for _, a := range [][]any{
		{[]string{"a", "b", "c"}, ", "}, {[]string{"a", "b", "c"}, ", ", " and "}, {[]int{1, 2, 3}, "-"},
		{map[string]any{"b": 2, "a": 1}, ", "}, {map[string]int{"b": 2, "a": 1, "c": 3}, ", ", " & "}, {"abc", ","},
		{[]any{1, "a", nil, true}, "|"}, {[]string{"a"}, ", ", " and "}, {[]string{}, ", "}, {[]string{"a", "b"}, 1, 2},
		{[]string{"a", "b"}, []int{1}}, {[]string{"a", "b"}, ", ", map[string]any{}}, {nil, ","}, {(*tstObj)(nil), ","},
	} {
		o.add("coll_delimit", "collections", "Delimit", a...)
	}
	// Uniq, Union, Intersect, SymDiff, Complement, In (collections_test.go, complement_test.go, symdiff_test.go).
	lists := []any{
		[]any{}, []any{1, 2, 2, 3}, []any{"a", "b", "a"}, []any{1, 1.0, "1", int64(1), uint(1)}, []any{nil, nil, 1},
		[]int{1, 2, 3}, []int{3, 4}, []int64{2, 3}, []float64{1, 2.5}, []string{"a", "b"}, []string{"b", "c"},
		[]any{"a", template.HTML("a"), hstring.HTML("a")}, []any{t1, t1, t2}, []any{obj1, obj2, obj1}, []any{[]int{1}, []int{1}},
		[]any{map[string]any{"a": 1}, map[string]any{"a": 1}}, []string(nil), []any{math.NaN(), math.NaN()},
		[]any{int8(1), int16(1), uint8(1)}, []uint8{1, 2}, []float32{1.5},
	}
	for _, a := range lists {
		o.add("coll_sets", "collections", "Uniq", a)
		for _, b := range lists {
			for _, m := range []string{"Union", "Intersect", "SymDiff", "Complement"} {
				o.add("coll_sets", "collections", m, a, b)
			}
		}
		for _, v := range []any{1, 1.0, "1", "a", int64(1), uint(1), template.HTML("a"), nil, t1, obj1, []int{1}, math.NaN()} {
			o.add("coll_sets", "collections", "In", a, v)
		}
	}
	o.add("coll_sets", "collections", "In", "abc", "b")
	o.add("coll_sets", "collections", "In", "abc", 1)
	o.add("coll_sets", "collections", "In", template.HTML("abc"), "bc")
	o.add("coll_sets", "collections", "Complement", []int{1}, []int{2}, []int{1, 2, 3})
	o.add("coll_sets", "collections", "Complement", []int{1})
	o.add("coll_sets", "collections", "Complement", "a", []int{1})
	// Append (append_test.go).
	for _, a := range [][]any{
		{"c", []string{"a", "b"}}, {"c", "d", []string{"a", "b"}}, {[]string{"c", "d"}, []string{"a", "b"}},
		{1, []string{"a"}}, {1, []any{"a"}}, {map[string]any{"a": 1}, []any{}}, {map[string]any{"a": 1}, []any(nil)},
		{"a", nil}, {"a", []int(nil)}, {1, 2, []int{0}}, {[]int{1}, []int{0}}, {[]any{1}, []any{0}}, {"a"},
		{t1, []any{}}, {obj1, []any{}}, {"a", "b"},
	} {
		o.add("coll_append", "collections", "Append", a...)
	}
	// Group, KeyVals, NewScratch, Slice.
	o.add("coll_misc", "collections", "KeyVals", "k", 1, "a", nil)
	o.add("coll_misc", "collections", "KeyVals", nil)
	o.add("coll_misc", "collections", "Group", "k", []string{"a"})
	o.add("coll_misc", "collections", "Group", nil, []string{"a"})
	o.add("coll_misc", "collections", "Group", "k", "a")
	o.add("coll_misc", "collections", "Group", "k", nil)
	o.add("coll_misc", "collections", "NewScratch")
	for _, a := range [][]any{
		{1, 2}, {"a", "b"}, {1, "a"}, {nil, 1}, {1, nil}, {t1, t2}, {template.HTML("a"), template.HTML("b")},
		{map[string]any{"a": 1}, map[string]any{}}, {obj1, obj2}, {[]int{1}, []int{2}}, {int64(1), 2},
	} {
		o.add("coll_misc", "collections", "Slice", a...)
	}
}
