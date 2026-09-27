package main

// Hugo-shaped values in the encode record format (-mode hugoencode),
// written by the second independent verifier of crates/go-json. jsonify,
// debug.Dump, Remarshal and the html/template JS escaper marshal template
// values: front matter and site params (maps.Params, whose IsZero method
// omitzero calls), data files and dicts (map[string]any, []any, float64),
// Scratch-built []map[string]any (index.json), page content
// (template.HTML with markup, entities, U+2028 and emoji) and dates.

import (
	"fmt"
	"html/template"
	"io"
	"math"
	"math/rand/v2"
	"reflect"
	"strings"

	"github.com/neohugo/neohugo/common/maps"
)

var hugoKeys = []string{"title", "tags", "_merge", "date", "draft", "weight", "params", "images", "description",
	"", "é", "<k>", "a&b", "permalink", "relpermalink", "summary", "content", "10", "9", "_MERGE", "Title"}

var hugoText = []string{
	"<p>Hello &amp; welcome to <em>Hugo</em></p>\n", "Lay's", "Tom & Jerry", "a < b > c", "  ", "😀 ปาร์ตี้",
	"line1\nline2\ttab", `quote " and \ backslash`, "</script><script>alert(1)</script>", "&lt;escaped&gt;", "",
	"\x00\x1f\x7f", "caf\xe9", "https://example.com/?a=1&b=2", "   ", strings.Repeat("long text ", 50),
}

func genHugoLeaf(r *rand.Rand) any {
	switch r.IntN(12) {
	case 0, 1, 2:
		return hugoText[r.IntN(len(hugoText))]
	case 3:
		return template.HTML(hugoText[r.IntN(len(hugoText))])
	case 4:
		return []float64{0, 1, 0.5, 1.5, 100, 2024, 1e21, 1e20, 1e-7, 1e-6, 0.1, 3.14159, 2.5, math.Copysign(0, -1), 12345678901234567890}[r.IntN(15)]
	case 5:
		return genInt64(r)
	case 6:
		return r.IntN(1000)
	case 7:
		return r.IntN(2) == 0
	case 8:
		return nil
	case 9:
		return genTime(r, false)
	case 10:
		return genAdvString(r)
	default:
		return advFloat64(r)
	}
}

func genParams(r *rand.Rand, depth int) maps.Params {
	switch r.IntN(8) {
	case 0:
		return nil
	case 1:
		return maps.Params{}
	case 2:
		return maps.Params{"_merge": []string{"deep", "shallow", "none"}[r.IntN(3)]}
	}
	n := r.IntN(6)
	p := make(maps.Params, n)
	for i := 0; i < n; i++ {
		p[hugoKeys[r.IntN(len(hugoKeys))]] = genHugoValue(r, depth-1)
	}
	return p
}

func genHugoValue(r *rand.Rand, depth int) any {
	top := 11
	if depth <= 0 {
		top = 4
	}
	switch r.IntN(top) {
	case 0, 1, 2, 3:
		return genHugoLeaf(r)
	case 4, 5:
		return genParams(r, depth)
	case 6:
		n := r.IntN(5)
		m := make(map[string]any, n)
		for i := 0; i < n; i++ {
			m[hugoKeys[r.IntN(len(hugoKeys))]] = genHugoValue(r, depth-1)
		}
		return m
	case 7:
		n := r.IntN(5)
		s := make([]any, n)
		for i := range s {
			s[i] = genHugoValue(r, depth-1)
		}
		return s
	case 8:
		n := r.IntN(5)
		s := make([]string, n)
		for i := range s {
			s[i] = hugoText[r.IntN(len(hugoText))]
		}
		return s
	case 9:
		// Scratch-built search index entries.
		n := r.IntN(4)
		s := make([]map[string]any, n)
		for i := range s {
			s[i] = map[string]any{
				"title":        hugoText[r.IntN(len(hugoText))],
				"contents":     genHugoLeaf(r),
				"relpermalink": "/posts/" + fmt.Sprint(i) + "/",
				"tags":         []any{"a", "b&c"},
			}
		}
		return s
	default:
		return genParamsStruct(r, depth)
	}
}

// genParamsStruct builds a struct with maps.Params-typed and any-typed
// fields carrying omitzero/omitempty options.
func genParamsStruct(r *rand.Rand, depth int) genStruct {
	n := r.IntN(5)
	var fields []reflect.StructField
	var vals []any
	var body []byte
	for i := 0; i < n; i++ {
		name := fmt.Sprintf("f%d", i)
		var opts []string
		flags := 0
		if r.IntN(2) == 0 {
			opts = append(opts, "omitempty")
			flags |= 1
		}
		if r.IntN(2) == 0 {
			opts = append(opts, "omitzero")
			flags |= 2
		}
		p := genParams(r, depth)
		typ := reflect.TypeFor[maps.Params]()
		var v any = p
		if r.IntN(3) == 0 {
			typ = anyType
			flags |= 8
			if r.IntN(3) == 0 {
				v = nil
			}
		}
		tag := name
		if len(opts) > 0 {
			tag += "," + strings.Join(opts, ",")
		}
		fields = append(fields, reflect.StructField{Name: fmt.Sprintf("G%d", i), Type: typ, Tag: reflect.StructTag(fmt.Sprintf("json:%q", tag))})
		vals = append(vals, unwrap(v))
		body = append(body, byte('0'+flags))
		body = putStr(body, name)
		body = dump(body, v)
	}
	d := fmt.Appendf(nil, "P%d{", n)
	d = append(d, body...)
	d = append(d, '}')
	sv := reflect.New(reflect.StructOf(fields)).Elem()
	for i, v := range vals {
		if v != nil {
			sv.Field(i).Set(reflect.ValueOf(v))
		}
	}
	return genStruct{value: sv.Interface(), d: d}
}

func writeHugoEncode(w io.Writer, r *rand.Rand, n int) {
	advIndent = true
	for i := 0; i < n; i++ {
		writeEncodeCase(w, r, genHugoValue(r, 4))
	}
}
