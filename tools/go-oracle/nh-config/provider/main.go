// Command provider is the Go oracle for the config provider of crates/nh-config
// (Wave B task T04): scripted Set/Get/Merge/SetDefaults/SetDefaultMergeStrategy/
// WalkParams/IsSet/Keys sequences on config.New() and config.NewFrom(), and the
// hash of config sections (the imaging SourceHash).
//
//	go run ./tools/go-oracle/nh-config/provider [-root .] [-out crates/nh-config/tests/fixtures/provider] [-scripts 1500] [-seed 1]
//
// Inputs: the decoded seeksnack config dumps
// (docs/rust-port/specs/architecture-core-data/config-{en,th}.json, with whole
// floats as int64 like TOML gives them), the seeksnack [imaging] section (the
// SourceHash vector 4bf645f71319dd1d), docs/hugo.toml, the scripts of
// config/defaultConfigProvider_test.go, and seeded random scripts (nested maps
// with case-mixed keys, _merge strategies, dotted keys, typed maps, every
// value kind). Each op records its input (before the call, which may mutate
// it), its result and, for mutating ops, the whole root afterwards.
package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"log"
	"math/rand"
	"os"
	"path/filepath"
	"runtime"
	"sort"
	"strings"

	"github.com/neohugo/neohugo/common/hashing"
	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/parser/metadecoders"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
)

type op struct {
	Op string
	K  string
	V  any
}

type script struct {
	name string
	from maps.Params // nil: config.New()
	ops  []op
}

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "crates/nh-config/tests/fixtures/provider", "output directory")
	nScripts := flag.Int("scripts", 1500, "number of random scripts")
	seed := flag.Int64("seed", 1, "random seed")
	flag.Parse()

	var scripts []script
	scripts = append(scripts, goTestScripts()...)
	scripts = append(scripts, imagingScript())
	for _, f := range []string{"config-en.json", "config-th.json"} {
		scripts = append(scripts, seeksnackScripts(*root, f)...)
	}
	scripts = append(scripts, docsScripts(*root)...)

	var cases []map[string]any
	nops := 0
	for _, s := range scripts {
		c := run(s)
		nops += len(s.ops)
		cases = append(cases, c)
	}

	// Random scripts are run several times (each time from a fresh copy): Go's results can
	// depend on its random map order (Merge visits the merged keys in map order, and panics
	// on the first bad one). From the first op whose record differs between runs, the
	// script is marked nondeterministic ("nondet": op index) and its later records are
	// dropped.
	nondet := 0
	for i := 0; i < *nScripts; i++ {
		gen := func() script {
			return randomScript(rand.New(rand.NewSource(*seed*1000003+int64(i))), i)
		}
		s := gen()
		c := run(s)
		nops += len(s.ops)
		first, _ := json.Marshal(c["ops"])
		firstOps := c["ops"].([]any)
		for run2 := 0; run2 < 100; run2++ {
			c2 := run(gen())
			again, _ := json.Marshal(c2["ops"])
			if string(again) == string(first) {
				continue
			}
			for j, rec := range c2["ops"].([]any) {
				a, _ := json.Marshal(rec)
				b, _ := json.Marshal(firstOps[j])
				if string(a) != string(b) {
					if prev, ok := c["nondet"].(int); !ok || j < prev {
						c["nondet"] = j
					}
					break
				}
			}
		}
		if j, ok := c["nondet"].(int); ok {
			nondet++
			// The records from the nondeterministic op on are those of one random run: drop
			// them so that the fixture regenerates byte for byte.
			c["ops"] = firstOps[:j]
		}
		cases = append(cases, c)
	}
	fmt.Fprintf(os.Stderr, "provider: %d random scripts nondeterministic\n", nondet)

	header := map[string]any{"oracle": "nh-config/provider", "goarch": runtime.GOARCH}
	if err := os.MkdirAll(*out, 0o755); err != nil {
		log.Fatal(err)
	}
	if err := goval.WriteCasesGz(filepath.Join(*out, "provider.json.gz"), header, cases); err != nil {
		log.Fatal(err)
	}
	fmt.Fprintf(os.Stderr, "provider: %d scripts, %d ops\n", len(cases), nops)
}

func run(s script) map[string]any {
	c := map[string]any{"name": s.name}
	var cfg config.Provider
	if s.from != nil {
		c["from"] = goval.Encode(s.from)
		cfg = config.NewFrom(s.from)
	} else {
		cfg = config.New()
	}
	var recs []any
	for _, o := range s.ops {
		rec := map[string]any{"op": o.Op, "k": goval.Str(o.K)}
		if o.V != nil || o.Op == "Set" || o.Op == "Merge" {
			rec["v"] = goval.Encode(o.V)
		}
		func() {
			defer func() {
				if r := recover(); r != nil {
					rec["panic"] = goval.Str(fmt.Sprint(r))
				}
			}()
			rec["r"] = apply(cfg, o)
		}()
		switch o.Op {
		case "Set", "Merge", "SetDefaults", "SetDefaultMergeStrategy":
			rec["root"] = goval.Encode(cfg.Get(""))
		}
		recs = append(recs, rec)
	}
	c["ops"] = recs
	return c
}

func apply(cfg config.Provider, o op) any {
	switch o.Op {
	case "Set":
		cfg.Set(o.K, o.V)
	case "Merge":
		cfg.Merge(o.K, o.V)
	case "SetDefaults":
		cfg.SetDefaults(o.V.(maps.Params))
	case "SetDefaultMergeStrategy":
		cfg.SetDefaultMergeStrategy()
	case "Get":
		return goval.Encode(cfg.Get(o.K))
	case "GetString":
		return goval.Str(cfg.GetString(o.K))
	case "GetInt":
		return cfg.GetInt(o.K)
	case "GetBool":
		return cfg.GetBool(o.K)
	case "GetParams":
		return goval.Encode(cfg.GetParams(o.K))
	case "GetStringMap":
		return goval.Encode(cfg.GetStringMap(o.K))
	case "GetStringMapString":
		return goval.Encode(cfg.GetStringMapString(o.K))
	case "GetStringSlice":
		return goval.Encode(cfg.GetStringSlice(o.K))
	case "GetStringSlicePreserveString":
		return goval.Encode(config.GetStringSlicePreserveString(cfg, o.K))
	case "IsSet":
		return cfg.IsSet(o.K)
	case "Keys":
		keys := cfg.Keys()
		sort.Strings(keys)
		out := []any{}
		for _, k := range keys {
			out = append(out, goval.Str(k))
		}
		return out
	case "Walk":
		// The visited Params (Go visits them in random map order): key path and merge strategy.
		var visited []string
		cfg.WalkParams(func(params ...maps.KeyParams) bool {
			var path []string
			for _, p := range params {
				path = append(path, p.Key)
			}
			s, found := params[len(params)-1].Params.GetMergeStrategy()
			visited = append(visited, fmt.Sprintf("%s|%s|%t", strings.Join(path, "/"), s, found))
			return false
		})
		sort.Strings(visited)
		out := []any{}
		for _, v := range visited {
			out = append(out, goval.Str(v))
		}
		return out
	case "Hash":
		return hashing.HashStringHex(cfg.GetStringMap(o.K))
	default:
		panic("unknown op " + o.Op)
	}
	return nil
}

func m(kv ...any) map[string]any {
	out := map[string]any{}
	for i := 0; i < len(kv); i += 2 {
		out[kv[i].(string)] = kv[i+1]
	}
	return out
}

// goTestScripts are the scripts of config/defaultConfigProvider_test.go.
func goTestScripts() []script {
	var s []script
	s = append(s, script{name: "go:set-and-get", ops: []op{
		{"Set", "foo", "bar"}, {"Get", "foo", nil}, {"Get", "FOO", nil}, {"GetString", "foo", nil},
		{"Set", "foo", 42}, {"Get", "foo", nil}, {"GetInt", "foo", nil}, {"Get", "", nil},
	}})
	s = append(s, script{name: "go:set-and-get-map", ops: []op{
		{"Set", "foo", m("bar", "baz")}, {"Get", "foo", nil}, {"GetStringMap", "foo", nil}, {"GetStringMapString", "foo", nil},
	}})
	s = append(s, script{name: "go:nested", ops: []op{
		{"Set", "a", m("B", "bv")}, {"Set", "a.c", "cv"}, {"Get", "a", nil}, {"Get", "a.c", nil},
		{"Set", "b.a", "av"}, {"Get", "b", nil}, {"Set", "b", m("b", "bv")}, {"Get", "b", nil},
	}})
	s = append(s, script{name: "go:nested2", ops: []op{{"Set", "a", "av"}, {"Set", "", m("a", "av2", "b", "bv2")}, {"Get", "", nil}}})
	s = append(s, script{name: "go:nested3", ops: []op{{"Set", "a", "av"}, {"Set", "", m("b", "bv2")}, {"Get", "", nil}}})
	s = append(s, script{name: "go:nested4", ops: []op{{"Set", "", m("foo", m("a", "av"))}, {"Set", "", m("foo", m("b", "bv2"))}, {"Get", "foo", nil}}})
	s = append(s, script{name: "go:merge-default", ops: []op{{"Set", "a", m("B", "bv")}, {"Merge", "a", m("B", "bv2", "c", "cv2")}, {"Get", "a", nil}}})
	s = append(s, script{name: "go:merge-default2", ops: []op{{"Set", "a", "av"}, {"Merge", "", m("a", "av2", "b", "bv2")}, {"Get", "", nil}}})
	s = append(s, script{name: "go:merge-shallow", ops: []op{
		{"Set", "a", m("_merge", "shallow", "B", "bv", "c", m("b", "bv"))},
		{"Merge", "a", m("c", m("d", "dv2"), "e", "ev2")}, {"Get", "a", nil},
	}})
	for i, left := range []any{map[string]string{"c": "cv1"}, m("c", "cv1")} {
		s = append(s, script{name: fmt.Sprintf("go:merge-typed-%d", i), ops: []op{
			{"Set", "", m("b", left)}, {"Merge", "", maps.Params{"b": maps.Params{"c": "cv2", "d": "dv2"}}}, {"Get", "", nil},
		}})
	}
	for i, left := range []func() any{func() any { return map[string]string{"b": "bv1"} }, func() any { return m("b", "bv1") }} {
		for j, right := range []func() any{func() any { return map[string]string{"b": "bv2", "c": "cv2"} }, func() any { return m("b", "bv2", "c", "cv2") }} {
			s = append(s, script{name: fmt.Sprintf("go:merge-typed2-%d-%d", i, j), ops: []op{
				{"Set", "a", left()}, {"Merge", "a", right()}, {"Get", "", nil},
			}})
		}
	}
	s = append(s, script{name: "go:merge-only-maps", ops: []op{
		{"Set", "", m("B", "bv")}, {"Merge", "", m("c", m("_merge", "shallow", "d", "dv2"))}, {"Get", "", nil},
	}})
	s = append(s, script{name: "go:isset", ops: []op{{"Set", "a", m("B", "bv")}, {"IsSet", "A", nil}, {"IsSet", "a.b", nil}, {"IsSet", "z", nil}}})
	s = append(s, script{name: "go:getbool", ops: []op{{"Set", "foo", true}, {"Get", "foo", nil}, {"GetBool", "foo", nil}}})
	s = append(s, script{name: "go:getparams", ops: []op{{"Set", "foo", maps.Params{"foo": true}}, {"GetParams", "foo", nil}, {"GetParams", "bar", nil}}})
	s = append(s, script{name: "go:keys", ops: []op{{"Set", "foo", maps.Params{"foo": 1}}, {"Set", "bar", maps.Params{"bar": 2}}, {"Keys", "", nil}}})
	s = append(s, script{name: "go:walk", ops: []op{{"Set", "x", maps.Params{}}, {"Set", "y", maps.Params{}}, {"Walk", "", nil}}})
	s = append(s, script{name: "go:setdefaults", ops: []op{{"SetDefaults", "", maps.Params{"foo": "bar", "bar": "baz"}}, {"Get", "foo", nil}, {"Get", "bar", nil}}})
	// Panics.
	s = append(s, script{name: "panic:merge-root-string", ops: []op{{"Merge", "", "not a map"}}})
	s = append(s, script{name: "panic:getparams-string", ops: []op{{"Set", "a", "x"}, {"GetParams", "a", nil}}})
	s = append(s, script{name: "panic:merge-root-nonparams", ops: []op{{"Set", "a", "x"}, {"Merge", "", m("a", m("b", 1))}}})
	s = append(s, script{name: "newfrom", from: maps.Params{"Title": "T", "Params": m("A", 1, "b", m("C", "d")), "_merge": "deep"}, ops: []op{
		{"Get", "", nil}, {"Get", "params.a", nil}, {"Get", "params.b.c", nil}, {"Walk", "", nil}, {"SetDefaultMergeStrategy", "", nil}, {"Walk", "", nil},
	}})
	return s
}

// imagingScript builds the seeksnack [imaging] config and hashes it (the imaging
// SourceHash that names every processed image: 4bf645f71319dd1d).
func imagingScript() script {
	cfg := m(
		"baseURL", "https://seeksnack.com/",
		"imaging", m("exif", m("disableDate", false, "disableLatLong", false, "excludeFields", ".*", "includeFields", "")),
		"params", m("author", "SeekSnack"),
		"outputFormats", m(),
		"menus", m("main", []any{m("name", "Snack Companies", "url", "/companies/", "weight", int64(1))}),
	)
	return script{name: "seeksnack:imaging", ops: []op{
		{"Set", "", cfg}, {"Hash", "imaging", nil}, {"SetDefaultMergeStrategy", "", nil},
		{"Get", "imaging", nil}, {"GetStringMap", "imaging", nil}, {"Hash", "imaging", nil},
		{"Walk", "", nil}, {"Hash", "params", nil}, {"Hash", "menus", nil},
	}}
}

// toInt64 converts whole float64 values to int64 recursively (TOML decodes integers as int64).
func toInt64(v any) any {
	switch vv := v.(type) {
	case map[string]any:
		for k, x := range vv {
			vv[k] = toInt64(x)
		}
		return vv
	case []any:
		for i, x := range vv {
			vv[i] = toInt64(x)
		}
		return vv
	case float64:
		if vv == float64(int64(vv)) {
			return int64(vv)
		}
	}
	return v
}

func readJSON(root, f string) map[string]any {
	b, err := os.ReadFile(filepath.Join(root, "docs/rust-port/specs/architecture-core-data", f))
	if err != nil {
		log.Fatal(err)
	}
	var cfg map[string]any
	if err := json.Unmarshal(b, &cfg); err != nil {
		log.Fatal(err)
	}
	return cfg
}

// leafPaths returns the dotted paths of every value in m (maps are descended into).
func leafPaths(prefix string, m map[string]any, out *[]string) {
	var keys []string
	for k := range m {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	for _, k := range keys {
		p := k
		if prefix != "" {
			p = prefix + "." + k
		}
		*out = append(*out, p)
		if mm, ok := m[k].(map[string]any); ok {
			leafPaths(p, mm, out)
		}
	}
}

func seeksnackScripts(root, f string) []script {
	var out []script
	for _, variant := range []string{"float64", "int64"} {
		cfg := readJSON(root, f)
		if variant == "int64" {
			toInt64(cfg)
		}
		var paths []string
		leafPaths("", cfg, &paths)
		ops := []op{{"Set", "", cfg}, {"SetDefaultMergeStrategy", "", nil}, {"Walk", "", nil}, {"Keys", "", nil}}
		for i, p := range paths {
			ops = append(ops, op{"Get", p, nil}, op{"IsSet", p, nil})
			switch i % 4 {
			case 0:
				ops = append(ops, op{"GetString", strings.ToUpper(p), nil}, op{"GetStringSlice", p, nil})
			case 1:
				ops = append(ops, op{"GetInt", p, nil}, op{"GetStringMap", p, nil})
			case 2:
				ops = append(ops, op{"GetBool", p, nil}, op{"GetStringMapString", p, nil})
			case 3:
				ops = append(ops, op{"GetStringSlicePreserveString", p, nil}, op{"IsSet", p + ".nope", nil})
			}
		}
		var top []string
		for k := range cfg {
			top = append(top, k)
		}
		sort.Strings(top)
		for _, k := range top {
			ops = append(ops, op{"Hash", k, nil})
		}
		// Merge the other language's params and a theme-like config.
		other := readJSON(root, "config-th.json")
		ops = append(ops,
			op{"Merge", "", m("params", other["params"], "newsection", m("a", int64(1)), "menus", m("footer", []any{}))},
			op{"Merge", "languages.th", m("params", m("x", "y"), "menus", m("side", []any{}))},
			op{"Get", "params", nil}, op{"Get", "languages", nil}, op{"Walk", "", nil},
		)
		out = append(out, script{name: "seeksnack:" + f + ":" + variant, ops: ops})
	}
	return out
}

func docsScripts(root string) []script {
	b, err := os.ReadFile(filepath.Join(root, "docs/hugo.toml"))
	if err != nil {
		log.Fatal(err)
	}
	cfg, err := metadecoders.Default.UnmarshalToMap(b, metadecoders.TOML)
	if err != nil {
		log.Fatal(err)
	}
	config.RenameKeys(cfg)
	var paths []string
	leafPaths("", cfg, &paths)
	ops := []op{{"Set", "", cfg}, {"SetDefaultMergeStrategy", "", nil}, {"Walk", "", nil}}
	for _, p := range paths {
		ops = append(ops, op{"Get", p, nil}, op{"GetString", p, nil}, op{"IsSet", strings.ToUpper(p), nil})
	}
	return []script{{name: "docs:hugo.toml", ops: ops}}
}

// Random scripts.

var keyPool = []string{
	"a", "b", "c", "A", "B", "Title", "params", "Params", "menus", "menu", "languages", "en", "th",
	"outputformats", "outputFormats", "mediatypes", "mediaTypes", "imaging", "exif", "x.y", "", "Ä", "ß",
}

var strPool = []string{"", "1", "0", "true", "false", "x", "x y", "a,b", "Ünïcode", "12abc", "-7", "3.5", "none", "shallow", "deep"}

func randKey(r *rand.Rand) string {
	return keyPool[r.Intn(len(keyPool))]
}

func randMergeStrategy(r *rand.Rand) any {
	switch r.Intn(6) {
	case 0:
		return maps.ParamsMergeStrategyDeep
	case 1:
		return maps.ParamsMergeStrategyShallow
	case 2:
		return maps.ParamsMergeStrategyNone
	case 3:
		return "bogus"
	default:
		return []string{"none", "shallow", "deep"}[r.Intn(3)]
	}
}

func randValue(r *rand.Rand, depth int) any {
	n := 10
	if depth <= 0 {
		n = 7
	}
	switch r.Intn(n) {
	case 0:
		return strPool[r.Intn(len(strPool))]
	case 1:
		return r.Intn(200) - 100
	case 2:
		return int64(r.Intn(1 << 20))
	case 3:
		return r.Intn(2) == 0
	case 4:
		return []float64{0, 1.5, 2, -3.25, 1e10}[r.Intn(5)]
	case 5:
		if r.Intn(4) == 0 {
			return nil
		}
		return []string{"a", "b"}
	case 6:
		return []any{"x", r.Intn(5), true}
	case 7:
		return map[string]string{"k": "v", "K2": "v2"}
	case 8:
		return randMap(r, depth-1, false)
	default:
		return maps.Params(randMap(r, depth-1, true))
	}
}

// randMap returns a map whose keys are unique case-insensitively (Go keeps a random one of
// several keys that fold to the same lower-case key).
func randMap(r *rand.Rand, depth int, lowerOnly bool) map[string]any {
	out := map[string]any{}
	seen := map[string]bool{}
	for i := r.Intn(4); i >= 0; i-- {
		k := randKey(r)
		if lowerOnly {
			k = strings.ToLower(k)
		}
		lk := strings.ToLower(k)
		if seen[lk] {
			continue
		}
		seen[lk] = true
		out[k] = randValue(r, depth)
	}
	if r.Intn(5) == 0 && !seen["_merge"] {
		out["_merge"] = randMergeStrategy(r)
	}
	return out
}

func randPath(r *rand.Rand) string {
	if r.Intn(12) == 0 {
		return ""
	}
	n := 1 + r.Intn(3)
	var parts []string
	for i := 0; i < n; i++ {
		parts = append(parts, randKey(r))
	}
	return strings.Join(parts, ".")
}

func randomScript(r *rand.Rand, i int) script {
	s := script{name: fmt.Sprintf("random#%d", i)}
	if r.Intn(4) == 0 {
		s.from = maps.Params(randMap(r, 3, false))
	}
	for j := 3 + r.Intn(10); j >= 0; j-- {
		switch r.Intn(14) {
		case 0, 1:
			s.ops = append(s.ops, op{"Set", randPath(r), randValue(r, 3)})
		case 2:
			s.ops = append(s.ops, op{"Set", "", randMap(r, 3, false)})
		case 3:
			s.ops = append(s.ops, op{"Merge", "", randMap(r, 3, false)})
		case 4:
			s.ops = append(s.ops, op{"Merge", randPath(r), randValue(r, 3)})
		case 5:
			s.ops = append(s.ops, op{"SetDefaults", "", maps.Params(randMap(r, 2, false))})
		case 6:
			s.ops = append(s.ops, op{"SetDefaultMergeStrategy", "", nil})
		case 7:
			s.ops = append(s.ops, op{"Walk", "", nil}, op{"Keys", "", nil})
		case 8:
			s.ops = append(s.ops, op{"GetParams", randPath(r), nil})
		default:
			get := []string{"Get", "GetString", "GetInt", "GetBool", "GetStringMap", "GetStringMapString", "GetStringSlice", "GetStringSlicePreserveString", "IsSet", "Hash"}
			s.ops = append(s.ops, op{get[r.Intn(len(get))], randPath(r), nil})
		}
	}
	s.ops = append(s.ops, op{"Get", "", nil})
	return s
}
