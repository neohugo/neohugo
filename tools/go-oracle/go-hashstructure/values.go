package main

// Values of the shared Rust value model (go_value::Value) — the Go types
// the Rust port maps them to — hashed with hashstructure and neohugo's
// hashing helpers. The Rust test (tests/oracle.rs, values_model) parses the
// description into a go_value::Value and hashes it through
// HashValue::Value, which exercises the go_value -> reflect mapping
// (from_value) that downstream crates use.
//
// Description language:
//
//	nil                      untyped nil (Value::Invalid)
//	tnil:<hex type>          typed nil of the Go type (Value::TypedNil)
//	bool:0|1, int:<kind>:<n>, uint:<kind>:<n>, f32:<bits>, f64:<bits>
//	str:<hex>, html:<hex>    string, template.HTML
//	time:<sec>:<nsec>:<loc>  as in desc
//	list:<ty>(v,...)         ty: any string int int64 float64 bool uint8 mapany
//	map:<ty>(<hex key>=v,...) ty: any params strstr

import (
	"fmt"
	"html/template"
	"math"
	"reflect"
	"sort"
	"strings"
	"time"

	"github.com/cespare/xxhash/v2"
	"github.com/gohugoio/hashstructure"
	"github.com/neohugo/neohugo/common/hashing"
	"github.com/neohugo/neohugo/common/maps"
)

var modelTypedNils = []any{
	[]string(nil), []any(nil), map[string]any(nil), maps.Params(nil), map[string]string(nil),
	(*int)(nil), (*time.Time)(nil), (func())(nil), []int(nil), []map[string]any(nil),
}

func modelScalar(r *rng) (any, string) {
	switch r.intn(16) {
	case 0:
		return nil, "nil"
	case 1:
		v := modelTypedNils[r.intn(len(modelTypedNils))]
		return v, "tnil:" + hx(reflect.TypeOf(v).String())
	case 2:
		b := r.intn(2) == 1
		if b {
			return b, "bool:1"
		}
		return b, "bool:0"
	case 3, 4:
		x := int64(r.next())
		if r.intn(2) == 0 {
			x = int64(r.intn(5)) - 2
		}
		switch r.intn(5) {
		case 0:
			return int(x), fmt.Sprintf("int:int:%d", x)
		case 1:
			return int8(x), fmt.Sprintf("int:int8:%d", int8(x))
		case 2:
			return int16(x), fmt.Sprintf("int:int16:%d", int16(x))
		case 3:
			return int32(x), fmt.Sprintf("int:int32:%d", int32(x))
		}
		return x, fmt.Sprintf("int:int64:%d", x)
	case 5:
		x := r.next()
		if r.intn(2) == 0 {
			x = uint64(r.intn(3))
		}
		switch r.intn(7) {
		case 0:
			return uint(x), fmt.Sprintf("uint:uint:%d", x)
		case 1:
			return uint8(x), fmt.Sprintf("uint:uint8:%d", uint8(x))
		case 2:
			return uint16(x), fmt.Sprintf("uint:uint16:%d", uint16(x))
		case 3:
			return uint32(x), fmt.Sprintf("uint:uint32:%d", uint32(x))
		case 4:
			if r.intn(4) == 0 {
				return uintptr(x), fmt.Sprintf("uint:uintptr:%d", x)
			}
		}
		return x, fmt.Sprintf("uint:uint64:%d", x)
	case 6:
		f := float32(genFloat(r))
		return f, fmt.Sprintf("f32:%x", math.Float32bits(f))
	case 7, 8:
		f := genFloat(r)
		return f, fmt.Sprintf("f64:%x", math.Float64bits(f))
	case 9:
		s := genString(r)
		return template.HTML(s), "html:" + hx(s)
	case 10:
		t := genTime(r)
		return t, fmt.Sprintf("time:%d:%d:%s", t.Unix(), t.Nanosecond(), descLoc(t))
	}
	s := genString(r)
	return s, "str:" + hx(s)
}

func modelValue(r *rng, depth int) (any, string) {
	k := r.intn(10)
	if depth > 3 {
		k = 0
	}
	switch {
	case k < 5:
		return modelScalar(r)
	case k < 8:
		n := r.intn(5)
		ty := []string{"any", "any", "string", "int", "int64", "float64", "bool", "uint8", "mapany"}[r.intn(9)]
		parts := make([]string, n)
		var out any
		switch ty {
		case "any":
			s := make([]any, n)
			for i := range s {
				s[i], parts[i] = modelValue(r, depth+1)
			}
			out = s
		case "string":
			s := make([]string, n)
			for i := range s {
				s[i] = genString(r)
				parts[i] = "str:" + hx(s[i])
			}
			out = s
		case "int":
			s := make([]int, n)
			for i := range s {
				s[i] = int(r.next()) >> r.intn(64)
				parts[i] = fmt.Sprintf("int:int:%d", s[i])
			}
			out = s
		case "int64":
			s := make([]int64, n)
			for i := range s {
				s[i] = int64(r.next()) >> r.intn(64)
				parts[i] = fmt.Sprintf("int:int64:%d", s[i])
			}
			out = s
		case "float64":
			s := make([]float64, n)
			for i := range s {
				s[i] = genFloat(r)
				parts[i] = fmt.Sprintf("f64:%x", math.Float64bits(s[i]))
			}
			out = s
		case "bool":
			s := make([]bool, n)
			for i := range s {
				s[i] = r.intn(2) == 1
				parts[i] = "bool:0"
				if s[i] {
					parts[i] = "bool:1"
				}
			}
			out = s
		case "uint8":
			s := make([]uint8, n)
			for i := range s {
				s[i] = uint8(r.next())
				parts[i] = fmt.Sprintf("uint:uint8:%d", s[i])
			}
			out = s
		case "mapany":
			s := make([]map[string]any, n)
			for i := range s {
				if r.intn(6) == 0 {
					parts[i] = "tnil:" + hx("map[string]interface {}")
					continue
				}
				m, d := modelMap(r, "any", depth+1)
				s[i] = m.(map[string]any)
				parts[i] = d
			}
			out = s
		}
		return out, "list:" + ty + "(" + strings.Join(parts, ",") + ")"
	}
	return modelMap(r, []string{"any", "any", "params", "strstr"}[r.intn(4)], depth)
}

func modelMap(r *rng, ty string, depth int) (any, string) {
	n := r.intn(5)
	type kv struct{ k, d string }
	var parts []kv
	var out any
	switch ty {
	case "any", "params":
		m := map[string]any{}
		for i := 0; i < n; i++ {
			k := genString(r)
			v, d := modelValue(r, depth+1)
			if _, dup := m[k]; dup {
				continue
			}
			m[k] = v
			parts = append(parts, kv{k, d})
		}
		if ty == "params" {
			out = maps.Params(m)
		} else {
			out = m
		}
	case "strstr":
		m := map[string]string{}
		for i := 0; i < n; i++ {
			k, v := genString(r), genString(r)
			if _, dup := m[k]; dup {
				continue
			}
			m[k] = v
			parts = append(parts, kv{k, "str:" + hx(v)})
		}
		out = m
	}
	sort.Slice(parts, func(i, j int) bool { return parts[i].k < parts[j].k })
	ss := make([]string, len(parts))
	for i, p := range parts {
		ss[i] = hx(p.k) + "=" + p.d
	}
	return out, "map:" + ty + "(" + strings.Join(ss, ",") + ")"
}

// valueLines returns n lines: desc TAB xx TAB fnv TAB HashString(v) TAB
// HashString("k", v, v) TAB HashStringHex([]any{v}).
func valueLines(seed uint64, n int) []string {
	r := &rng{s: seed ^ 0xa0761d6478bd642f}
	protect := func(f func() string) (s string) {
		defer func() {
			if rec := recover(); rec != nil {
				s = "panic:" + hx(fmt.Sprint(rec))
			}
		}()
		return f()
	}
	var lines []string
	for i := 0; i < n; i++ {
		v, d := modelValue(r, 0)
		cols := []string{
			d,
			protect(func() string { return res(hashstructure.Hash(v, &hashstructure.HashOptions{Hasher: xxhash.New()})) }),
			protect(func() string { return res(hashstructure.Hash(v, nil)) }),
			protect(func() string { return hashing.HashString(v) }),
			protect(func() string { return hashing.HashString("k", v, v) }),
			protect(func() string { return hashing.HashStringHex([]any{v}) }),
		}
		lines = append(lines, strings.Join(cols, "\t"))
	}
	return lines
}
