// Command go-hashstructure is the Go oracle for the Rust crate
// crates/go-hashstructure.
//
//	go run ./tools/go-oracle/go-hashstructure -out <dir> [-n 20000] [-site <seeksnack>] [-cache <getresource dir>]
//
// It writes:
//
//   - random.fixture: random typed Go values (built with reflect: all
//     scalar kinds, time.Time in several locations, slices, arrays, maps,
//     pointers, interfaces, unnamed structs with tags), one per line:
//     desc TAB xx TAB fnv TAB sets TAB zeronil TAB ignorezero TAB stringer TAB multi
//     where desc is a description the Rust test parses back into a
//     HashValue, and each result is the hex hash or "err:<message>":
//     xx = hashstructure.Hash(v, &HashOptions{Hasher: xxhash.New()}),
//     fnv = hashstructure.Hash(v, nil), sets/zeronil/ignorezero/stringer =
//     xx with SlicesAsSets/ZeroNil/IgnoreZeroValue/UseStringer, multi =
//     hashing.HashUint64(v, v) (neohugo's variadic form).
//   - named.fixture: hand-written cases with named struct types, methods
//     (Hashable, Includable, IncludableMap, fmt.Stringer, Key()), blank
//     fields, the exact neohugo image filter structs, maps.Params, and
//     neohugo hashing helpers: name TAB result.
//   - remote.fixture: GetRemote cache keys hashing.HashString(uri,
//     map[string]any(nil)) for every youtube_video of the site, checked
//     against the file names of the getresource file cache (contains the
//     site's API key: scratch only); remote-public.fixture: the same URIs
//     with the placeholder key "API_KEY" (checked in).
//   - tzdata: the TZif bytes of the zones used (hex), so that Rust loads
//     exactly the same Location data.
package main

import (
	"bufio"
	"encoding/hex"
	"errors"
	"flag"
	"fmt"
	"image"
	"io/fs"
	"log"
	"math"
	"os"
	"path/filepath"
	"reflect"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/cespare/xxhash/v2"
	"github.com/gohugoio/hashstructure"
	"github.com/neohugo/neohugo/common/hashing"
	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/parser/pageparser"
	"github.com/neohugo/neohugo/resources/images"
)

// ---------------------------------------------------------------------------
// Time zones: loaded from TZif bytes so that the Rust side can load the
// same bytes.

var tzNames = []string{"Asia/Bangkok", "America/New_York", "Europe/Dublin", "Australia/Lord_Howe"}
var tzData = map[string][]byte{}
var tzLocs = map[string]*time.Location{}

func loadZones() {
	for _, n := range tzNames {
		b, err := os.ReadFile(filepath.Join("/usr/share/zoneinfo", n))
		if err != nil {
			log.Fatal(err)
		}
		loc, err := time.LoadLocationFromTZData(n, b)
		if err != nil {
			log.Fatal(err)
		}
		tzData[n] = b
		tzLocs[n] = loc
	}
}

// ---------------------------------------------------------------------------
// splitmix64

type rng struct{ s uint64 }

func (r *rng) next() uint64 {
	r.s += 0x9e3779b97f4a7c15
	z := r.s
	z = (z ^ (z >> 30)) * 0xbf58476d1ce4e5b9
	z = (z ^ (z >> 27)) * 0x94d049bb133111eb
	return z ^ (z >> 31)
}

func (r *rng) intn(n int) int { return int(r.next() % uint64(n)) }

// ---------------------------------------------------------------------------
// Random types and values

var (
	anyType  = reflect.TypeOf((*any)(nil)).Elem()
	timeType = reflect.TypeOf(time.Time{})
	scalars  = []reflect.Type{
		reflect.TypeOf(false), reflect.TypeOf(int(0)), reflect.TypeOf(int8(0)), reflect.TypeOf(int16(0)),
		reflect.TypeOf(int32(0)), reflect.TypeOf(int64(0)), reflect.TypeOf(uint(0)), reflect.TypeOf(uint8(0)),
		reflect.TypeOf(uint16(0)), reflect.TypeOf(uint32(0)), reflect.TypeOf(uint64(0)),
		reflect.TypeOf(float32(0)), reflect.TypeOf(float64(0)), reflect.TypeOf(complex64(0)),
		reflect.TypeOf(complex128(0)), reflect.TypeOf(""), reflect.TypeOf(""), reflect.TypeOf(""), timeType,
	}
	keyTypes = []reflect.Type{reflect.TypeOf(""), reflect.TypeOf(int(0)), anyType, reflect.TypeOf(false), reflect.TypeOf(float64(0)), reflect.TypeOf(uint8(0))}
	tags     = []string{"", "", "", `hash:"ignore"`, `hash:"-"`, `hash:"set"`, `json:"x"`, `json:"y" hash:"set"`, `hash:"string"`, `hash:""`}
)

func genType(r *rng, depth int) reflect.Type {
	k := r.intn(20)
	if depth > 3 {
		k = 0
	}
	switch {
	case k < 9:
		return scalars[r.intn(len(scalars))]
	case k < 11:
		return anyType
	case k < 13:
		return reflect.SliceOf(genType(r, depth+1))
	case k < 14:
		return reflect.ArrayOf(r.intn(4), genType(r, depth+1))
	case k < 16:
		return reflect.MapOf(keyTypes[r.intn(len(keyTypes))], genType(r, depth+1))
	case k < 18:
		return reflect.PointerTo(genType(r, depth+1))
	default:
		n := r.intn(5)
		fields := make([]reflect.StructField, n)
		for i := range fields {
			fields[i] = reflect.StructField{
				Name: fmt.Sprintf("F%d", i),
				Type: genType(r, depth+1),
				Tag:  reflect.StructTag(tags[r.intn(len(tags))]),
			}
		}
		return reflect.StructOf(fields)
	}
}

var specialFloats = []float64{0, math.Copysign(0, -1), 1, -1, 0.5, math.Inf(1), math.Inf(-1), math.NaN(), 5e-324, math.MaxFloat64, 1e300, 42}

func genFloat(r *rng) float64 {
	if r.intn(3) == 0 {
		return specialFloats[r.intn(len(specialFloats))]
	}
	return math.Float64frombits(r.next())
}

func genString(r *rng) string {
	switch r.intn(6) {
	case 0:
		return ""
	case 1:
		return []string{"foo", "bar", "resize", "600x480", "webp", "a", "b", "_merge", "none"}[r.intn(9)]
	case 2:
		return "日本語é"
	}
	b := make([]byte, r.intn(12))
	for i := range b {
		b[i] = byte(r.next())
	}
	return string(b)
}

func genTime(r *rng) time.Time {
	var t time.Time
	switch r.intn(6) {
	case 0:
		return time.Time{}
	case 1:
		t = time.Unix(int64(r.next()%4000000000)-1000000000, int64(r.next()%1000000000))
	case 2:
		t = time.Unix(int64(r.next()%80000000000)-60000000000, 0)
	default:
		t = time.Date(1900+r.intn(250), time.Month(1+r.intn(12)), 1+r.intn(28), r.intn(24), r.intn(60), r.intn(60), r.intn(1000000000), time.UTC)
	}
	switch r.intn(6) {
	case 0, 1:
		return t.UTC()
	case 2:
		offs := []int{3600, -18000, 25200, 3661, -45, 0, 19800, -60}
		return t.In(time.FixedZone([]string{"X", "", "ICT", "LMT"}[r.intn(4)], offs[r.intn(len(offs))]))
	default:
		return t.In(tzLocs[tzNames[r.intn(len(tzNames))]])
	}
}

func genValue(r *rng, t reflect.Type, depth int) reflect.Value {
	v := reflect.New(t).Elem()
	if t == timeType {
		v.Set(reflect.ValueOf(genTime(r)))
		return v
	}
	switch t.Kind() {
	case reflect.Bool:
		v.SetBool(r.intn(2) == 1)
	case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
		x := int64(r.next())
		if r.intn(2) == 0 {
			x = int64(r.intn(5)) - 2
		}
		v.SetInt(x) // truncates like a conversion
		v.SetInt(v.Int())
	case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64, reflect.Uintptr:
		x := r.next()
		if r.intn(2) == 0 {
			x = uint64(r.intn(3))
		}
		v.SetUint(x)
	case reflect.Float32:
		v.SetFloat(float64(float32(genFloat(r))))
	case reflect.Float64:
		v.SetFloat(genFloat(r))
	case reflect.Complex64:
		v.SetComplex(complex(float64(float32(genFloat(r))), float64(float32(genFloat(r)))))
	case reflect.Complex128:
		v.SetComplex(complex(genFloat(r), genFloat(r)))
	case reflect.String:
		v.SetString(genString(r))
	case reflect.Interface:
		if r.intn(5) == 0 || depth > 4 {
			return v // nil
		}
		v.Set(genValue(r, genType(r, depth+1), depth+1))
	case reflect.Slice:
		if r.intn(8) == 0 {
			return v // nil
		}
		n := r.intn(5)
		s := reflect.MakeSlice(t, n, n)
		for i := 0; i < n; i++ {
			s.Index(i).Set(genValue(r, t.Elem(), depth+1))
		}
		v.Set(s)
	case reflect.Array:
		for i := 0; i < t.Len(); i++ {
			v.Index(i).Set(genValue(r, t.Elem(), depth+1))
		}
	case reflect.Map:
		if r.intn(8) == 0 {
			return v // nil
		}
		m := reflect.MakeMap(t)
		n := r.intn(5)
		for i := 0; i < n; i++ {
			var k reflect.Value
			if t.Key() == anyType {
				k = reflect.New(anyType).Elem()
				kt := []reflect.Type{reflect.TypeOf(""), reflect.TypeOf(int(0)), reflect.TypeOf(false), reflect.TypeOf(float64(0))}[r.intn(4)]
				k.Set(genValue(r, kt, depth+1))
			} else {
				k = genValue(r, t.Key(), depth+1)
			}
			m.SetMapIndex(k, genValue(r, t.Elem(), depth+1))
		}
		v.Set(m)
	case reflect.Pointer:
		if r.intn(4) == 0 {
			return v // nil
		}
		p := reflect.New(t.Elem())
		p.Elem().Set(genValue(r, t.Elem(), depth+1))
		v.Set(p)
	case reflect.Struct:
		for i := 0; i < t.NumField(); i++ {
			v.Field(i).Set(genValue(r, t.Field(i).Type, depth+1))
		}
	}
	return v
}

// ---------------------------------------------------------------------------
// Descriptions (parsed by crates/go-hashstructure/tests/oracle.rs)

func hx(s string) string { return hex.EncodeToString([]byte(s)) }

func descLoc(t time.Time) string {
	loc := t.Location()
	if loc == time.UTC {
		return "utc"
	}
	for n, l := range tzLocs {
		if l == loc {
			return "tz:" + n
		}
	}
	name, off := t.Zone()
	// Fixed zone: its (only) zone is the one in effect.
	return fmt.Sprintf("fixed:%s:%d", hx(name), off)
}

func descZero(t reflect.Type) string {
	if t.Kind() == reflect.Interface {
		return "zeroiface"
	}
	return desc(reflect.Zero(t))
}

func desc(v reflect.Value) string {
	if !v.IsValid() {
		return "nil"
	}
	t := v.Type()
	if t == timeType {
		tm := v.Interface().(time.Time)
		return fmt.Sprintf("time:%d:%d:%s", tm.Unix(), tm.Nanosecond(), descLoc(tm))
	}
	switch t.Kind() {
	case reflect.Bool:
		if v.Bool() {
			return "bool:1"
		}
		return "bool:0"
	case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
		return fmt.Sprintf("int:%s:%d", t.Kind(), v.Int())
	case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64, reflect.Uintptr:
		return fmt.Sprintf("uint:%s:%d", t.Kind(), v.Uint())
	case reflect.Float32:
		return fmt.Sprintf("f32:%x", math.Float32bits(float32(v.Float())))
	case reflect.Float64:
		return fmt.Sprintf("f64:%x", math.Float64bits(v.Float()))
	case reflect.Complex64:
		c := v.Complex()
		return fmt.Sprintf("c64:%x:%x", math.Float32bits(float32(real(c))), math.Float32bits(float32(imag(c))))
	case reflect.Complex128:
		c := v.Complex()
		return fmt.Sprintf("c128:%x:%x", math.Float64bits(real(c)), math.Float64bits(imag(c)))
	case reflect.String:
		return "str:" + hx(v.String())
	case reflect.Interface:
		if v.IsNil() {
			return "nil"
		}
		return "iface(" + desc(v.Elem()) + ")"
	case reflect.Slice:
		if v.IsNil() {
			return "slice:nil"
		}
		parts := make([]string, v.Len())
		for i := range parts {
			parts[i] = desc(v.Index(i))
		}
		return "slice(" + strings.Join(parts, ",") + ")"
	case reflect.Array:
		parts := make([]string, v.Len())
		for i := range parts {
			parts[i] = desc(v.Index(i))
		}
		return "array(" + strings.Join(parts, ",") + ")"
	case reflect.Map:
		if v.IsNil() {
			return "map:nil"
		}
		var parts []string
		it := v.MapRange()
		for it.Next() {
			parts = append(parts, desc(it.Key())+"="+desc(it.Value()))
		}
		sort.Strings(parts)
		return "map(" + strings.Join(parts, ",") + ")"
	case reflect.Pointer:
		if v.IsNil() {
			return "ptr:nil:" + descZero(t.Elem())
		}
		return "ptr(" + desc(v.Elem()) + ")"
	case reflect.Struct:
		parts := make([]string, t.NumField())
		for i := range parts {
			f := t.Field(i)
			exp := "1"
			if f.PkgPath != "" {
				exp = "0"
			}
			parts[i] = hx(f.Name) + ":" + exp + ":" + hx(string(f.Tag)) + ":" + desc(v.Field(i))
		}
		return "struct(" + hx(t.Name()) + ";" + strings.Join(parts, ";") + ")"
	}
	return "unsupported:" + t.Kind().String()
}

// ---------------------------------------------------------------------------
// Hashing

func res(h uint64, err error) string {
	if err != nil {
		return "err:" + hx(err.Error())
	}
	return strconv.FormatUint(h, 16)
}

func hashAll(v any) []string {
	protect := func(f func() (uint64, error)) (s string) {
		defer func() {
			if r := recover(); r != nil {
				s = "panic:" + hx(fmt.Sprint(r))
			}
		}()
		return res(f())
	}
	return []string{
		protect(func() (uint64, error) { return hashstructure.Hash(v, &hashstructure.HashOptions{Hasher: xxhash.New()}) }),
		protect(func() (uint64, error) { return hashstructure.Hash(v, nil) }),
		protect(func() (uint64, error) {
			return hashstructure.Hash(v, &hashstructure.HashOptions{Hasher: xxhash.New(), SlicesAsSets: true})
		}),
		protect(func() (uint64, error) {
			return hashstructure.Hash(v, &hashstructure.HashOptions{Hasher: xxhash.New(), ZeroNil: true})
		}),
		protect(func() (uint64, error) {
			return hashstructure.Hash(v, &hashstructure.HashOptions{Hasher: xxhash.New(), IgnoreZeroValue: true})
		}),
		protect(func() (uint64, error) {
			return hashstructure.Hash(v, &hashstructure.HashOptions{Hasher: xxhash.New(), UseStringer: true})
		}),
		protect(func() (uint64, error) { return hashing.HashUint64(v, v), nil }),
	}
}

// ---------------------------------------------------------------------------
// Named cases (mirrored by hand in the Rust test)

type Foo struct {
	Name string
	Age  int
}

type withTags struct {
	A string `hash:"ignore"`
	B string `hash:"-"`
	C []int  `hash:"set"`
	D []int
	e int
	F string `json:"f" hash:"set"`
}

type blanks struct {
	_ int
	A int
	_ string
}

type stringerT struct{ v string }

func (s stringerT) String() string { return "S:" + s.v }

type withStringer struct {
	A stringerT `hash:"string"`
	B stringerT
	C int
}

type badStringer struct {
	C int `hash:"string"`
}

type hashableV struct{ X int }

func (h hashableV) Hash() (uint64, error) { return 500 + uint64(h.X), nil }

type hashableP struct{ X int }

func (h *hashableP) Hash() (uint64, error) { return 700 + uint64(h.X), nil }

type hashableErr struct{}

func (hashableErr) Hash() (uint64, error) { return 0, errors.New("boom") }

type includable struct {
	Value  string
	Ignore string
}

func (t includable) HashInclude(field string, v any) (bool, error) { return field != "Ignore", nil }

type includableP struct {
	Value  string
	Ignore string
}

func (t *includableP) HashInclude(field string, v any) (bool, error) { return field != "Ignore", nil }

type includableMap struct {
	Map map[string]string
}

func (t includableMap) HashIncludeMap(field string, k, v any) (bool, error) {
	if field != "Map" {
		return true, nil
	}
	if s, ok := k.(string); ok && s == "ignore" {
		return false, nil
	}
	return true, nil
}

type mapIncl map[string]string

func (t mapIncl) HashIncludeMap(_ string, k, _ any) (bool, error) { return k.(string) != "ignore", nil }

type keyer struct{ key string }

func (k keyer) Key() string { return k.key }

type withFunc struct {
	F func()
}

type withChan struct {
	C chan int
}

type withUintptr struct {
	P uintptr
}

type nested struct {
	Foo  Foo
	PFoo *Foo
	Any  any
	M    map[string]any
}

type wmSource struct{ key string }

func (w wmSource) Key() string                       { return w.key }
func (w wmSource) DecodeImage() (image.Image, error) { return nil, nil }

func namedCases() [][2]string {
	var out [][2]string
	add := func(name string, v any) {
		for i, r := range hashAll(v) {
			out = append(out, [2]string{fmt.Sprintf("%s#%d", name, i), r})
		}
	}
	add("foo", Foo{Name: "x", Age: 3})
	add("foo-ptr", &Foo{Name: "x", Age: 3})
	add("foo-zero", Foo{})
	add("foo-nilptr", (*Foo)(nil))
	add("tags", withTags{A: "a", B: "b", C: []int{1, 2, 3}, D: []int{1, 2, 3}, e: 5, F: "f"})
	add("tags-ptr", &withTags{A: "a", C: []int{3, 2, 1}, D: []int{3, 2, 1}})
	add("blanks", blanks{A: 1})
	add("blanks-ptr", &blanks{A: 1})
	add("stringer", withStringer{A: stringerT{"a"}, B: stringerT{"b"}, C: 1})
	add("stringer-ptr", &withStringer{A: stringerT{"a"}, B: stringerT{"b"}, C: 1})
	add("badstringer", badStringer{C: 1})
	add("hashable-v", hashableV{X: 1})
	add("hashable-v-ptr", &hashableV{X: 1})
	add("hashable-p", hashableP{X: 1})
	add("hashable-p-ptr", &hashableP{X: 1})
	add("hashable-p-slice", []hashableP{{X: 2}})
	add("hashable-p-iface-slice", []any{hashableP{X: 2}})
	add("hashable-p-map", map[string]hashableP{"a": {X: 3}})
	add("hashable-err", hashableErr{})
	add("includable", includable{Value: "v", Ignore: "i"})
	add("includable-p", includableP{Value: "v", Ignore: "i"})
	add("includable-p-ptr", &includableP{Value: "v", Ignore: "i"})
	add("includable-map", includableMap{Map: map[string]string{"foo": "bar", "ignore": "x"}})
	add("map-incl", mapIncl{"foo": "bar", "ignore": "x"})
	add("keyer", keyer{key: "k"})
	add("func", withFunc{F: func() {}})
	add("func-nil", withFunc{})
	add("chan", withChan{C: make(chan int)})
	add("uintptr", withUintptr{P: 5})
	add("nested", nested{Foo: Foo{"a", 1}, PFoo: &Foo{"b", 2}, Any: Foo{"c", 3}, M: map[string]any{"x": Foo{"d", 4}, "y": []any{1, "z"}}})
	add("nested-ptr", &nested{PFoo: &Foo{"b", 2}, Any: &Foo{"c", 3}})
	add("pptr-nil", func() **Foo { var p *Foo; return &p }())
	add("ptr-iface-nil", func() *any { var a any; return &a }())
	add("anon", struct {
		A int
		B string `hash:"set"`
	}{1, "x"})
	add("time-zero", time.Time{})
	add("time-utc", time.Date(2021, 1, 2, 3, 4, 5, 6, time.UTC))
	add("time-fixed", time.Date(2021, 1, 2, 3, 4, 5, 6, time.FixedZone("ICT", 7*3600)))
	add("time-fixed-sec", time.Date(2021, 1, 2, 3, 4, 5, 6, time.FixedZone("LMT", 3661)))
	add("time-fixed-bad", time.Date(2021, 1, 2, 3, 4, 5, 6, time.FixedZone("X", -60)))
	add("time-bangkok", time.Date(1900, 1, 2, 3, 4, 5, 6, tzLocs["Asia/Bangkok"]))
	add("time-newyork-2030", time.Date(2030, 7, 2, 3, 4, 5, 6, tzLocs["America/New_York"]))
	// Addressability contexts (independent verifier additions): pointer
	// receivers and blank fields depend on whether the struct is
	// addressable where it is visited.
	blankAt := func(a int) blanks { b := blanks{}; b.A = a; return b }
	add("hashable-p-array", [1]hashableP{{X: 4}})
	add("hashable-p-array-ptr", &[1]hashableP{{X: 4}})
	add("hashable-p-field", struct{ H hashableP }{hashableP{X: 5}})
	add("hashable-p-field-ptr", &struct{ H hashableP }{hashableP{X: 5}})
	add("hashable-p-array-in-slice", [][1]hashableP{{{X: 6}}})
	add("blanks-array", [2]blanks{blankAt(1), blankAt(2)})
	add("blanks-array-ptr", &[2]blanks{blankAt(1), blankAt(2)})
	add("blanks-slice", []blanks{blankAt(1)})
	add("blanks-iface-slice", []any{blankAt(1)})
	add("blanks-map", map[string]blanks{"k": blankAt(1)})
	add("blanks-field", struct{ B blanks }{blankAt(3)})
	add("blanks-field-ptr", &struct{ B blanks }{blankAt(3)})
	add("blanks-iface-field-ptr", &struct{ B any }{blankAt(3)})
	add("includable-p-slice", []includableP{{Value: "v", Ignore: "i"}})
	add("includable-p-field", struct{ I includableP }{includableP{Value: "v", Ignore: "i"}})
	add("includable-p-field-ptr", &struct{ I includableP }{includableP{Value: "v", Ignore: "i"}})
	tm := time.Date(2021, 1, 2, 3, 4, 5, 6, time.UTC)
	add("stringer-time-field", struct {
		T time.Time `hash:"string"`
	}{tm})
	add("stringer-ptrtime-field", struct {
		T *time.Time `hash:"string"`
	}{&tm})
	add("stringer-ptrtime-nil", struct {
		T *time.Time `hash:"string"`
	}{nil})
	add("stringer-any-time", struct {
		T any `hash:"string"`
	}{tm})
	add("stringer-any-int", struct {
		T any `hash:"string"`
	}{1})
	add("ignorezero-mixed", struct {
		A int
		B *int
		C []int
		D map[string]int
		E any
		F blanks
		G [2]float64
		H time.Time
		I string
		J [0]int
	}{G: [2]float64{math.Copysign(0, -1), 0}, C: []int{}, D: map[string]int{}})
	add("params", maps.Params{"_merge": "none", "exif": maps.Params{"_merge": "none", "disabledate": false, "disablelatlong": false, "excludefields": ".*", "includefields": ""}})

	// neohugo image filter keys with the real resources/images types.
	f := images.Filters{}
	wm := wmSource{key: "/images/watermark_hu_3bf49ff914f6e68c.png_6519743917224815147"}
	add("overlay", []any{f.Overlay(wm, 0, 0)})
	out = append(out, [2]string{"overlay-key", hashing.HashString([]any{f.Overlay(wm, 0, 0)})})
	out = append(out, [2]string{"overlay-key-float", hashing.HashString([]any{f.Overlay(wm, 0.0, 1)})})
	out = append(out, [2]string{"opacity-key", hashing.HashString([]any{f.Opacity(0.5)})})
	out = append(out, [2]string{"process-key", hashing.HashString([]any{f.Process("resize 100x100")})})
	out = append(out, [2]string{"resize-key", hashing.HashStringHex([]string{"resize", "600x480", "webp"})})
	out = append(out, [2]string{"hashstring-ab", hashing.HashString("a", "b")})
	out = append(out, [2]string{"hashstring-keyer", hashing.HashString("a", "b", keyer{"c"})})
	out = append(out, [2]string{"hashstring-none", hashing.HashString()})
	out = append(out, [2]string{"hashstring-nil", hashing.HashString(nil)})
	out = append(out, [2]string{"hashstring-nilmap", hashing.HashString("u", map[string]any(nil))})
	out = append(out, [2]string{"xx-hex", hashing.XxHashFromStringHexEncoded("hello")})
	out = append(out, [2]string{"md5", hashing.MD5FromStringHexEncoded("hello")})
	h, n, _ := hashing.XXHashFromReader(strings.NewReader("hello world"))
	out = append(out, [2]string{"reader-strings", fmt.Sprintf("%d:%d", h, n)})
	tf, _ := os.CreateTemp("", "hs")
	_, _ = tf.WriteString("hello world")
	_, _ = tf.Seek(0, 0)
	h, n, _ = hashing.XXHashFromReader(tf)
	_ = tf.Close()
	_ = os.Remove(tf.Name())
	out = append(out, [2]string{"reader-file", fmt.Sprintf("%d:%d", h, n)})
	return out
}

// ---------------------------------------------------------------------------
// GetRemote keys

func remoteKeys(site, cacheDir, apiKey string) [][2]string {
	var ids []string
	seen := map[string]bool{}
	_ = filepath.WalkDir(filepath.Join(site, "content"), func(path string, d fs.DirEntry, err error) error {
		if err != nil || d.IsDir() || !strings.HasSuffix(path, ".md") {
			return nil
		}
		b, _ := os.ReadFile(path)
		cf, err := pageparser.ParseFrontMatterAndContent(strings.NewReader(string(b)))
		if err != nil {
			return nil
		}
		if id, ok := cf.FrontMatter["youtube_video"].(string); ok && id != "" && !seen[id] {
			seen[id] = true
			ids = append(ids, id)
		}
		return nil
	})
	sort.Strings(ids)
	cached := map[string]bool{}
	if entries, err := os.ReadDir(cacheDir); err == nil {
		for _, e := range entries {
			cached[e.Name()] = true
		}
	}
	// The API key comes from the site config (params.api.youtube).
	cfg, _ := os.ReadFile(filepath.Join(site, "hugo.toml"))
	key := ""
	for _, l := range strings.Split(string(cfg), "\n") {
		if strings.HasPrefix(strings.TrimSpace(l), "youtube") {
			_, v, _ := strings.Cut(l, "=")
			key = strings.Trim(strings.TrimSpace(v), `"`)
		}
	}
	if apiKey != "" {
		key = apiKey
	}
	var out [][2]string
	matched := 0
	for _, id := range ids {
		uri := "https://www.googleapis.com/youtube/v3/videos?key=" + key + "&part=snippet,contentDetails,statistics&id=" + id
		k := hashing.HashString(uri, map[string]any(nil))
		if cached[k] {
			matched++
		}
		out = append(out, [2]string{uri, k})
	}
	log.Printf("remote keys: %d ids, %d found in %s (%d files)", len(ids), matched, cacheDir, len(cached))
	return out
}

// ---------------------------------------------------------------------------

func writeLines(path string, lines []string) {
	f, err := os.Create(path)
	if err != nil {
		log.Fatal(err)
	}
	w := bufio.NewWriter(f)
	for _, l := range lines {
		fmt.Fprintln(w, l)
	}
	if err := w.Flush(); err != nil {
		log.Fatal(err)
	}
	if err := f.Close(); err != nil {
		log.Fatal(err)
	}
	log.Printf("%d lines -> %s", len(lines), path)
}

func main() {
	out := flag.String("out", "", "output directory")
	n := flag.Int("n", 20000, "number of random values")
	seed := flag.Uint64("seed", 1, "random seed")
	site := flag.String("site", "", "seeksnack site (read only)")
	cache := flag.String("cache", "", "getresource file cache dir")
	nvalues := flag.Int("values", 0, "number of go_value-model values (values.fixture)")
	flag.Parse()
	loadZones()

	var tz []string
	for _, n := range tzNames {
		tz = append(tz, n+"\t"+hex.EncodeToString(tzData[n]))
	}
	writeLines(filepath.Join(*out, "tzdata"), tz)

	r := &rng{s: *seed}
	var lines []string
	for i := 0; i < *n; i++ {
		t := genType(r, 0)
		v := genValue(r, t, 0)
		var iv any
		if v.IsValid() {
			iv = v.Interface()
		}
		d := desc(reflect.ValueOf(iv))
		lines = append(lines, d+"\t"+strings.Join(hashAll(iv), "\t"))
	}
	writeLines(filepath.Join(*out, "random.fixture"), lines)

	if *nvalues > 0 {
		writeLines(filepath.Join(*out, "values.fixture"), valueLines(*seed, *nvalues))
	}

	var named []string
	for _, c := range namedCases() {
		named = append(named, c[0]+"\t"+c[1])
	}
	writeLines(filepath.Join(*out, "named.fixture"), named)

	if *site != "" {
		// remote.fixture uses the site's real API key (keep it out of the
		// repository); remote-public.fixture the placeholder "API_KEY".
		var rk, pub []string
		for _, c := range remoteKeys(*site, *cache, "") {
			rk = append(rk, c[0]+"\t"+c[1])
		}
		for _, c := range remoteKeys(*site, *cache, "API_KEY") {
			pub = append(pub, c[0]+"\t"+c[1])
		}
		writeLines(filepath.Join(*out, "remote.fixture"), rk)
		writeLines(filepath.Join(*out, "remote-public.fixture"), pub)
	}
}
