package main

import (
	"bytes"
	"encoding/json"
	"hash/fnv"
	"html/template"
	"io/fs"
	"log"
	"math"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"

	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/parser/pageparser"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/corpus"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
)

type input struct {
	name string
	m    map[string]any
}

// configFiles are the decoded seeksnack config dumps (hugo config --format json).
var configFiles = []string{"config-en", "config-th", "config-mounts", "config-en-printzero"}

func paramsInputs(root string) []input {
	var in []input
	dir := filepath.Join(root, "docs/rust-port/specs/architecture-core-data")
	for _, name := range configFiles {
		b, err := os.ReadFile(filepath.Join(dir, name+".json"))
		if err != nil {
			log.Fatal(err)
		}
		var m map[string]any
		if err := json.Unmarshal(b, &m); err != nil {
			log.Fatal(err)
		}
		in = append(in, input{name, m})
	}
	// Case-mixed variants (the dumps are already lower-cased).
	for _, c := range in[:len(configFiles)] {
		in = append(in, input{c.name + "-mixed", mix(c.m, 0).(map[string]any)})
	}
	in = append(in, frontMatter(root)...)
	in = append(in, adversarialMaps()...)
	return in
}

// titleWords upper-cases the first ASCII letter of every "_"- or "-"-separated word.
func titleWords(s string) string {
	b := []byte(s)
	start := true
	for i, c := range b {
		if start && c >= 'a' && c <= 'z' {
			b[i] = c - 'a' + 'A'
		}
		start = c == '_' || c == '-'
	}
	return string(b)
}

// mix upper-cases keys deterministically (by an FNV hash of key and depth) and
// turns some all-string nested maps into map[string]string.
func mix(v any, depth int) any {
	switch vv := v.(type) {
	case map[string]any:
		m := map[string]any{}
		allStrings := len(vv) > 0
		for k, x := range vv {
			if _, ok := x.(string); !ok {
				allStrings = false
			}
			h := fnv.New32a()
			_, _ = h.Write([]byte(k))
			_, _ = h.Write([]byte{byte(depth)})
			nk := k
			switch h.Sum32() % 4 {
			case 1:
				if k != "" {
					nk = strings.ToUpper(k[:1]) + k[1:]
				}
			case 2:
				nk = strings.ToUpper(k)
			case 3:
				nk = titleWords(k)
			}
			m[nk] = mix(x, depth+1)
		}
		if allStrings && depth%2 == 1 {
			ms := map[string]string{}
			for k, x := range m {
				ms[k] = x.(string)
			}
			return ms
		}
		return m
	case []any:
		out := make([]any, len(vv))
		for i, x := range vv {
			out[i] = mix(x, depth+1)
		}
		return out
	default:
		return v
	}
}

// frontMatter decodes the front matter of every content file.
func frontMatter(root string) []input {
	var in []input
	seen := map[string]bool{}
	for _, r := range corpus.Roots {
		dir := filepath.Join(root, r)
		err := filepath.WalkDir(dir, func(path string, d fs.DirEntry, err error) error {
			if err != nil {
				return err
			}
			if d.IsDir() {
				return nil
			}
			switch filepath.Ext(path) {
			case ".md", ".markdown", ".html":
			default:
				return nil
			}
			b, err := os.ReadFile(path)
			if err != nil {
				return err
			}
			cf, err := pageparser.ParseFrontMatterAndContent(bytes.NewReader(b))
			if err != nil || len(cf.FrontMatter) == 0 {
				return nil
			}
			// Skip exact duplicates (many pages share their front matter).
			eb, err := json.Marshal(goval.Encode(cf.FrontMatter))
			if err != nil {
				return err
			}
			if seen[string(eb)] {
				return nil
			}
			seen[string(eb)] = true
			rel, _ := filepath.Rel(root, path)
			in = append(in, input{"fm:" + filepath.ToSlash(rel), cf.FrontMatter})
			return nil
		})
		if err != nil {
			log.Fatal(err)
		}
	}
	return in
}

// adversarialMaps are hand-written edge cases.
func adversarialMaps() []input {
	t1 := time.Date(2024, 2, 29, 13, 14, 15, 123456789, time.UTC)
	t2 := time.Date(1999, 12, 31, 23, 59, 59, 0, time.FixedZone("ICT", 7*3600))
	var nilParams maps.Params
	var nilMap map[string]any
	var nilSMap map[string]string
	var nilSlice []string
	return []input{
		{"adv:kinds", map[string]any{
			"Bool": true, "False": false, "Int": 1, "Int8": int8(-8), "Int16": int16(16), "Int32": int32(-32),
			"Int64": int64(1 << 40), "Uint": uint(7), "Uint8": uint8(255), "Uint16": uint16(65535), "Uint32": uint32(1 << 31),
			"Uint64": uint64(math.MaxUint64), "Float32": float32(1.5), "Float64": 2.25, "NaN": math.NaN(), "Inf": math.Inf(-1),
			"NegZero": math.Copysign(0, -1), "String": "Value", "Empty": "", "HTML": template.HTML("<b>x</b>"),
			"URL": template.URL("http://x"), "Time": t1, "TimeICT": t2, "Nil": nil, "Strings": []string{"A", "b"},
			"Ints": []int{1, 2}, "Any": []any{"x", 1, map[string]any{"InSlice": 1}}, "NilSlice": nilSlice,
			"Bytes": []byte("ab"), "SliceOfMaps": []map[string]any{{"K": "v"}},
		}},
		{"adv:nested", map[string]any{
			"A":         map[string]any{"B": map[string]any{"C": "deep", "D": 1}, "E": []any{map[string]any{"F": 1}}},
			"StrMap":    map[string]string{"X": "1", "y": "2"},
			"Params":    maps.Params{"Upper": "kept", "lower": 1},
			"NilParams": nilParams, "NilMap": nilMap, "NilStrMap": nilSMap,
			"EmptyMap": map[string]any{}, "EmptyStrMap": map[string]string{},
			"Mixed.Dot": map[string]any{"Inner.Key": "dotted"},
			"Menu":      map[string]any{"Main": []any{map[string]any{"Name": "Home", "Weight": 1}}},
		}},
		{"adv:merge", map[string]any{
			"_merge": "deep", "A": map[string]any{"_merge": "none", "b": 1, "C": map[string]any{"_MERGE": "shallow", "d": 2}},
			"B": map[string]any{"_merge": "bogus", "x": 1}, "C": map[string]any{"_merge": 3}, "D": map[string]any{"_Merge": ""},
			"E": map[string]any{"_merge": maps.ParamsMergeStrategyShallow}, "F": map[string]string{"_merge": "deep", "k": "v"},
			"G": map[string]any{"_merge": nil}, "H": map[string]any{"_merge": template.HTML("none"), "i": 1},
		}},
		{"adv:mergeonly", map[string]any{"_merge": "none"}},
		{"adv:mergeonly2", map[string]any{"_Merge": "deep", "a": map[string]any{"_merge": "none"}}},
		{"adv:shallow", map[string]any{"_merge": "shallow", "a": map[string]any{"new": 1}, "n": "new"}},
		{"adv:empty", map[string]any{}},
		{"adv:collide", map[string]any{
			// One non-lower-case key and its lower-case form: Go's result is the
			// non-lower-case key's value whatever the map order.
			"Title": "upper", "title": "lower", "A": map[string]any{"x": 1}, "a": map[string]any{"y": 2},
			"lower": 1, "Only": map[string]string{"K": "v"},
		}},
		{"adv:collide3", map[string]any{
			// Two non-lower-case variants: Go's result depends on map order.
			"Foo": 1, "FOO": 2, "foo": 3, "bar": 4,
		}},
		{"adv:unicode", map[string]any{
			"ÄÖÜ": 1, "Straße": 2, "ǅ": 3, "ΣΑΣ": 4, "İstanbul": 5, "K": 6, "ﬁle": 7, "日本": 8, "\xff\xfe": 9, "": 10,
			"Ω": map[string]any{"Ωmega": "x"},
		}},
		{"adv:paths", map[string]any{
			"a":   map[string]any{"b": map[string]any{"c": "abc", "c.d": "dotted"}, "b.c": "flat"},
			"a.b": map[string]any{"c": "wrong"}, "x/y": "slash", "x": map[string]any{"y": "nested"},
			"sep": map[string]any{"": "emptykey"}, "list": []any{"0", "1"}, "str": "leaf",
		}},
		{"adv:menus", map[string]any{
			"Menu": map[string]any{"main": []any{}}, "menus": map[string]any{"footer": []any{}},
			"Languages": map[string]any{
				"en": map[string]any{"Menu": map[string]any{"main": 1}, "weight": 1},
				"th": map[string]any{"menu": "x", "Menus": "y"},
			},
			"params": map[string]any{"menu": "kept"},
		}},
		{"adv:floats", map[string]any{
			"a": 1.0, "b": 1.5, "c": 0.0, "d": math.Copysign(0, -1), "e": 1e300, "f": math.Pow(2, 63), "g": -math.Pow(2, 63),
			"h": math.NaN(), "i": math.Inf(1), "j": 9007199254740993.0, "k": float32(2), "l": int64(3), "m": 1e-300,
			"n": []any{1.0, 2.5, map[string]any{"o": 4.0, "p": []any{5.0}}, []any{6.0}},
			"q": map[string]any{"r": 7.0, "s": maps.Params{"t": 8.0}}, "u": []float64{9}, "v": 4611686018427387904.0,
		}},
	}
}

func encodeInputs(in []input) []map[string]any {
	var out []map[string]any
	for _, i := range in {
		out = append(out, map[string]any{"name": goval.Str(i.name), "v": goval.Encode(i.m)})
	}
	return out
}

// deepCopy copies every map and slice (Go mutates maps in place).
func deepCopy(v any) any {
	switch vv := v.(type) {
	case map[string]any:
		if vv == nil {
			return vv
		}
		m := make(map[string]any, len(vv))
		for k, x := range vv {
			m[k] = deepCopy(x)
		}
		return m
	case maps.Params:
		if vv == nil {
			return vv
		}
		m := make(maps.Params, len(vv))
		for k, x := range vv {
			m[k] = deepCopy(x)
		}
		return m
	case map[string]string:
		if vv == nil {
			return vv
		}
		m := make(map[string]string, len(vv))
		for k, x := range vv {
			m[k] = x
		}
		return m
	case []any:
		if vv == nil {
			return vv
		}
		s := make([]any, len(vv))
		for i, x := range vv {
			s[i] = deepCopy(x)
		}
		return s
	case []map[string]any:
		if vv == nil {
			return vv
		}
		s := make([]map[string]any, len(vv))
		for i, x := range vv {
			s[i] = deepCopy(x).(map[string]any)
		}
		return s
	case []string:
		if vv == nil {
			return vv
		}
		return append([]string{}, vv...)
	default:
		return v
	}
}

func copyMap(m map[string]any) map[string]any { return deepCopy(m).(map[string]any) }

// keyPaths returns the "."-joined key paths of m up to depth 3 (sorted).
func keyPaths(m map[string]any) []string {
	var out []string
	var walk func(prefix string, v any, depth int)
	walk = func(prefix string, v any, depth int) {
		if depth > 3 {
			return
		}
		var keys []string
		var get func(string) any
		switch vv := v.(type) {
		case map[string]any:
			for k := range vv {
				keys = append(keys, k)
			}
			get = func(k string) any { return vv[k] }
		case maps.Params:
			for k := range vv {
				keys = append(keys, k)
			}
			get = func(k string) any { return vv[k] }
		case map[string]string:
			for k := range vv {
				keys = append(keys, k)
			}
			get = func(k string) any { return vv[k] }
		default:
			return
		}
		sort.Strings(keys)
		for _, k := range keys {
			p := k
			if prefix != "" {
				p = prefix + "." + k
			}
			out = append(out, p)
			walk(p, get(k), depth+1)
		}
	}
	walk("", m, 1)
	return out
}

// lookupKeys are the keys looked up in a map: its key paths (original case and
// upper case), a few misses and separators.
func lookupKeys(m map[string]any, limit int) []string {
	paths := keyPaths(m)
	if len(paths) > limit {
		// Keep a deterministic spread.
		step := float64(len(paths)) / float64(limit)
		var sel []string
		for i := range limit {
			sel = append(sel, paths[int(float64(i)*step)])
		}
		paths = sel
	}
	keys := []string{"", ".", "missing", "missing.key", "a..b"}
	for _, p := range paths {
		keys = append(keys, p, strings.ToUpper(p), p+".missing")
	}
	return keys
}

// hasFoldDup reports whether m or a map nested in it has two keys that are
// equal under simple case folding: only then do the map operations depend on
// Go's randomized map order.
func hasFoldDup(v any) bool {
	var keys []string
	var vals []any
	switch vv := v.(type) {
	case map[string]any:
		for k, x := range vv {
			keys, vals = append(keys, k), append(vals, x)
		}
	case maps.Params:
		for k, x := range vv {
			keys, vals = append(keys, k), append(vals, x)
		}
	case map[string]string:
		for k := range vv {
			keys = append(keys, k)
		}
	case []any:
		for _, x := range vv {
			if hasFoldDup(x) {
				return true
			}
		}
		return false
	default:
		return false
	}
	for i, a := range keys {
		for _, b := range keys[i+1:] {
			if strings.EqualFold(a, b) {
				return true
			}
		}
	}
	for _, x := range vals {
		if hasFoldDup(x) {
			return true
		}
	}
	return false
}

// repsFor is how often stable runs an operation on m: many times when the
// result may depend on map order (so that a 1-in-8 order is seen with
// near certainty and the fixture regenerates identically), else a few.
func repsFor(ms ...map[string]any) int {
	for _, m := range ms {
		if hasFoldDup(m) {
			return 400
		}
	}
	return 25
}

// stable runs f n times (Go randomizes map iteration on every range) and
// returns its encoded result, or {"nondet": true} when the runs differ, i.e.
// when Go's own result depends on map order.
func stable(n int, f func() (any, error)) map[string]any {
	first := goval.Call(f)
	fb, err := json.Marshal(first)
	if err != nil {
		log.Fatal(err)
	}
	for range n - 1 {
		b, err := json.Marshal(goval.Call(f))
		if err != nil {
			log.Fatal(err)
		}
		if !bytes.Equal(b, fb) {
			return map[string]any{"nondet": true}
		}
	}
	return first
}

// stableRaw is stable for functions that return an already encoded value.
func stableRaw(n int, f func() (any, error)) map[string]any {
	first := goval.CallRaw(f)
	fb, err := json.Marshal(first)
	if err != nil {
		log.Fatal(err)
	}
	for range n - 1 {
		b, err := json.Marshal(goval.CallRaw(f))
		if err != nil {
			log.Fatal(err)
		}
		if !bytes.Equal(b, fb) {
			return map[string]any{"nondet": true}
		}
	}
	return first
}

type ref struct {
	name string
	enc  any
}

// same replaces a result that equals a reference encoding by {"same": name}.
func same(v map[string]any, refs ...ref) map[string]any {
	vb, err := json.Marshal(v)
	if err != nil {
		log.Fatal(err)
	}
	for _, r := range refs {
		rb, err := json.Marshal(map[string]any{"ok": r.enc})
		if err != nil {
			log.Fatal(err)
		}
		if bytes.Equal(vb, rb) {
			return map[string]any{"same": r.name}
		}
	}
	return v
}

func renamer(patternKeys ...string) func(m map[string]any) (any, error) {
	return func(m map[string]any) (any, error) {
		r, err := maps.NewKeyRenamer(patternKeys...)
		if err != nil {
			return nil, err
		}
		r.Rename(m)
		return m, nil
	}
}

var (
	rename1 = renamer("{menu,languages/*/menu}", "menus")
	// Each pattern renames at most one key per map (unless keys differ only in
	// case), so the result does not depend on map order.
	rename2 = renamer("{ren1,sub/*/ren1}", "new1", "{Ren2,sub/ren2}", "new2", "**/b", "B", "a/?/c", "x",
		"l[a-j]st", "listed", "st[!a]", "sTR", "x/{y,yy}", "xy", "\\{}", "braces")
)

func paramsCases(in []input) []map[string]any {
	var cases []map[string]any
	prepared := make([]maps.Params, len(in))
	for i, inp := range in {
		p := maps.Params(copyMap(inp.m))
		maps.PrepareParams(p)
		prepared[i] = p
	}
	isFM := func(i int) bool { return strings.HasPrefix(in[i].name, "fm:") }
	fmOrd := map[int]int{}
	for i := range in {
		if isFM(i) {
			fmOrd[i] = len(fmOrd)
		}
	}
	// sampled selects every config and adversarial input and every 10th front
	// matter for the heavier operations.
	sampled := func(i int) bool { return !isFM(i) || fmOrd[i]%10 == 0 }
	// Inputs whose prepared form depends on Go's map order (keys that differ
	// only in case): pairs and lookups skip them.
	nondet := map[int]bool{}

	for i, inp := range in {
		n := repsFor(inp.m)
		inRef := ref{"input", goval.Encode(inp.m)}
		prep := stable(n, func() (any, error) {
			p := maps.Params(copyMap(inp.m))
			maps.PrepareParams(p)
			return p, nil
		})
		if prep["ok"] == nil {
			nondet[i] = true
		}
		prepRef := ref{"prepare", prep["ok"]}
		c := map[string]any{"op": "single", "input": i, "prepare": prep}
		c["prepareClone"] = same(stable(n, func() (any, error) { return maps.PrepareParamsClone(copyMap(inp.m)), nil }), prepRef)
		c["toParams"] = same(stable(n, func() (any, error) { return maps.ToParamsAndPrepare(copyMap(inp.m)) }), prepRef)
		c["clean"] = same(stable(n, func() (any, error) { return maps.CleanConfigStringMap(copyMap(inp.m)), nil }), inRef)
		if prep["ok"] != nil {
			// CleanConfigStringMap's result type is map[string]interface {}.
			asMap := map[string]any{}
			for k, v := range prep["ok"].(map[string]any) {
				asMap[k] = v
			}
			asMap["t"] = "map[string]interface {}"
			c["cleanPrepared"] = same(stable(n, func() (any, error) {
				return maps.CleanConfigStringMap(deepCopy(prepared[i]).(maps.Params)), nil
			}), prepRef, ref{"prepareMap", asMap})
			c["isZero"] = prepared[i].IsZero()
		}
		c["convertFloat"] = same(stable(n, func() (any, error) {
			m := copyMap(inp.m)
			maps.ConvertFloat64WithNoDecimalsToInt(m)
			return m, nil
		}), inRef)
		c["rename"] = same(stable(n, func() (any, error) { return rename1(copyMap(inp.m)) }), inRef)
		c["rename2"] = same(stable(n, func() (any, error) { return rename2(copyMap(inp.m)) }), inRef)

		// The heavier operations run on every config and adversarial input and
		// on every 10th front matter.
		if !sampled(i) {
			cases = append(cases, c)
			continue
		}
		c["toStringMapString"] = stable(n, func() (any, error) { return maps.ToStringMapStringE(copyMap(inp.m)) })
		if prep["ok"] != nil {
			var lookups []any
			shallow := func(f func() (any, error)) map[string]any {
				return goval.CallRaw(func() (any, error) {
					v, err := f()
					return goval.Shallow(v), err
				})
			}
			for _, k := range lookupKeys(inp.m, 30) {
				param := shallow(func() (any, error) { return maps.GetNestedParam(k, ".", prepared[i]) })
				pref := ref{"param", param["ok"]}
				slash := same(shallow(func() (any, error) { return maps.GetNestedParam(k, "/", prepared[i]) }), pref)
				nested := same(shallow(func() (any, error) { return prepared[i].GetNested(strings.Split(k, ".")...), nil }), pref)
				fn := goval.CallRaw(func() (any, error) {
					v, key, owner, err := maps.GetNestedParamFn(k, ".", func(key string) any { return prepared[i][strings.ToLower(key)] })
					if err != nil {
						return nil, err
					}
					return []any{goval.Shallow(v), goval.Str(key), goval.Shallow(owner)}, nil
				})
				lookups = append(lookups, []any{goval.Str(k), param, slash, nested, fn})
			}
			c["lookups"] = lookups
		}
		cases = append(cases, c)
	}

	// Pairs: MergeParams, SetParams, MergeParamsWithStrategy, MergeShallow.
	type pair struct{ a, b int }
	idx := map[string]int{}
	for i, inp := range in {
		idx[inp.name] = i
	}
	var pairs []pair
	for _, p := range [][2]string{
		{"config-en", "config-th"}, {"config-th-mixed", "config-en-mixed"}, {"config-mounts", "config-en-printzero"},
	} {
		pairs = append(pairs, pair{idx[p[0]], idx[p[1]]})
	}
	var adv, fm []int
	for i, inp := range in {
		switch {
		case isFM(i):
			fm = append(fm, i)
		case strings.HasPrefix(inp.name, "adv:"):
			adv = append(adv, i)
		}
	}
	for _, a := range adv {
		for _, b := range adv {
			if a != b {
				pairs = append(pairs, pair{a, b})
			}
		}
	}
	for k := 0; k+10 < len(fm); k += 20 {
		pairs = append(pairs, pair{fm[k], fm[k+10]}, pair{fm[k+10], fm[k]})
	}
	for _, p := range pairs {
		if nondet[p.a] || nondet[p.b] {
			continue
		}
		c := map[string]any{"op": "pair", "a": p.a, "b": p.b}
		n := repsFor(in[p.a].m, in[p.b].m)
		a, b := prepared[p.a], prepared[p.b]
		aRef := ref{"a", goval.Encode(a)}
		merged := stable(n, func() (any, error) {
			dst := deepCopy(a).(maps.Params)
			maps.MergeParams(dst, deepCopy(b).(maps.Params))
			return dst, nil
		})
		mRef := ref{"merge", merged["ok"]}
		c["merge"] = same(merged, aRef)
		set := stable(n, func() (any, error) {
			dst := deepCopy(a).(maps.Params)
			maps.SetParams(dst, deepCopy(b).(maps.Params))
			return dst, nil
		})
		sRef := ref{"set", set["ok"]}
		c["set"] = same(set, aRef, mRef)
		var with []any
		for _, s := range []string{"", "none", "shallow", "deep", "bogus"} {
			with = append(with, same(stable(n, func() (any, error) {
				dst := deepCopy(a).(maps.Params)
				maps.MergeParamsWithStrategy(s, dst, deepCopy(b).(maps.Params))
				return dst, nil
			}), aRef, mRef, sRef))
		}
		c["mergeWith"] = with
		c["mergeShallow"] = same(stable(n, func() (any, error) {
			dst := copyMap(in[p.a].m)
			maps.MergeShallow(dst, copyMap(in[p.b].m))
			return dst, nil
		}), ref{"inputA", goval.Encode(in[p.a].m)})
		cases = append(cases, c)
	}

	// LookupEqualFold over key variants.
	for i, inp := range in {
		if !sampled(i) {
			continue
		}
		n := repsFor(inp.m)
		var ls []any
		for _, k := range append(topKeys(inp.m), "missing", "") {
			for _, kk := range []string{k, strings.ToUpper(k), strings.ToLower(k)} {
				ls = append(ls, []any{goval.Str(kk), stableRaw(n, func() (any, error) {
					v, key, found := maps.LookupEqualFold(inp.m, kk)
					return []any{goval.Shallow(v), goval.Str(key), found}, nil
				})})
			}
		}
		cases = append(cases, map[string]any{"op": "fold", "input": i, "lookups": ls})
	}

	// ToSliceStringMap, ToStringMapE and ToParamsAndPrepare over other shapes.
	for _, v := range []any{
		[]map[string]any{{"abc": 123}}, []any{map[string]any{"def": 456}, "skip", maps.Params{"p": 1}},
		maps.Params{"x": 1}, map[string]any{"x": 1}, map[string]string{"S": "s"}, "str", nil, []any{},
		[]map[string]any(nil), map[string]any(nil), maps.Params(nil),
	} {
		cases = append(cases, map[string]any{
			"op": "convert", "v": goval.Encode(v),
			"toSliceStringMap": goval.Call(func() (any, error) { return maps.ToSliceStringMap(v) }),
			"toStringMap":      goval.Call(func() (any, error) { return maps.ToStringMapE(v) }),
			"toParams":         goval.Call(func() (any, error) { return maps.ToParamsAndPrepare(v) }),
		})
	}
	return cases
}

func topKeys(m map[string]any) []string {
	var keys []string
	for k := range m {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	return keys
}
