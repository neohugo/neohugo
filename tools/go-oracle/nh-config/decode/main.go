// Command decode is the Go oracle for the mapstructure port of crates/nh-config
// (Wave B task T04): mitchellh/mapstructure decoding (WeakDecode, Decode and
// the DecoderConfig options Hugo uses) into a kitchen-sink struct of every
// field kind, and the config decoders built on it: config.DecodeBuildConfig,
// DecodeSitemap, DecodeServer, Pagination, PageConfig, security.DecodeConfig,
// services.DecodeConfig, privacy.DecodeConfig, the minifiers config struct and
// a RootConfig-like struct with the squashed config.CommonDirs.
//
//	go run ./tools/go-oracle/nh-config/decode [-root .] [-out crates/nh-config/tests/fixtures/decode] [-n 400] [-seed 1]
//
// Inputs: the sections of the decoded seeksnack config dumps
// (docs/rust-port/specs/architecture-core-data/config-{en,th}.json) and of
// docs/hugo.toml, and seeded random maps (known keys in exact, lower, upper and
// mixed case, unknown keys, values of every kind: ints of every size at their
// limits, floats incl. ±Inf/NaN/±1e20, numeric/bool strings, slices, maps,
// Params, typed nils, named strings, template.HTML, time.Time).
//
// mapstructure converts floats with int64(f)/uint64(f), which differ between
// arm64 and amd64 for out-of-range values, so the checked-in fixture comes from
// an arm64 build run under qemu-aarch64-static (see crates/nh-config/PORTING.md).
package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"html/template"
	"log"
	"math"
	"math/rand"
	"os"
	"path/filepath"
	"reflect"
	"runtime"
	"sort"
	"strings"
	"time"

	"github.com/mitchellh/mapstructure"
	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/config/privacy"
	"github.com/neohugo/neohugo/config/security"
	"github.com/neohugo/neohugo/config/services"
	"github.com/neohugo/neohugo/minifiers"
	"github.com/neohugo/neohugo/parser/metadecoders"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-config/cval"
)

// Inner is a nested struct.
type Inner struct {
	A string
	B int
	C []string
}

// Embedded is squashed into Kitchen.
type Embedded struct {
	E1 string
	E2 bool
	E3 *Inner
}

// Kitchen has a field of every kind mapstructure decodes.
type Kitchen struct {
	S         string
	B         bool
	I         int
	I8        int8
	I16       int16
	I32       int32
	I64       int64
	U         uint
	U8        uint8
	U16       uint16
	U32       uint32
	U64       uint64
	F32       float32
	F64       float64
	SS        []string
	IS        []int
	BS        []byte
	FS        []float64
	AS        []any
	MSA       map[string]any
	MSS       map[string]string
	MSI       map[string]int
	MSIn      map[string]Inner
	Any       any
	In        Inner
	InP       *Inner
	Ins       []Inner
	InPs      []*Inner
	Arr       [2]string
	ArrI      [3]int
	Embedded  `mapstructure:",squash"`
	Tagged    string `mapstructure:"renamed"`
	TaggedOpt string `mapstructure:"other,omitempty"`
	Dash      string `mapstructure:"-"`
	Dur       time.Duration
	Params    maps.Params
	unexp     string
}

// Kitchen.unexp is never set: it checks that mapstructure skips unexported fields.
var _ = Kitchen{}.unexp

// Remain collects unknown keys.
type Remain struct {
	Name string
	Rest map[string]any `mapstructure:",remain"`
}

// EmbedNoTag embeds Inner without a squash tag (squashed only with DecoderConfig.Squash).
type EmbedNoTag struct {
	Inner
	X int
}

// BadSquash squashes a non-struct.
type BadSquash struct {
	S string `mapstructure:",squash"`
	T int
}

// RootLike is a RootConfig-like struct: the squashed config.CommonDirs and fields of the
// kinds allconfig.RootConfig has.
type RootLike struct {
	BaseURL                string
	Title                  string
	Timeout                string
	UglyURLs               any
	DisableKinds           []string
	Taxonomies             map[string]string
	SummaryLength          int
	BuildDrafts            bool
	DefaultContentLanguage string
	config.CommonDirs      `mapstructure:",squash"`
	Internal               string `mapstructure:"-"`
}

var decoderConfigs = []string{"weak", "strict", "unused", "unset", "zero", "squash", "duration"}

func newDecoder(cfg string, out any) *mapstructure.Decoder {
	dc := &mapstructure.DecoderConfig{Result: out, WeaklyTypedInput: true}
	switch cfg {
	case "weak":
	case "strict":
		dc.WeaklyTypedInput = false
	case "unused":
		dc.ErrorUnused = true
	case "unset":
		dc.ErrorUnset = true
	case "zero":
		dc.ZeroFields = true
	case "squash":
		dc.Squash = true
	case "duration":
		dc.DecodeHook = mapstructure.StringToTimeDurationHookFunc()
	default:
		panic(cfg)
	}
	d, err := mapstructure.NewDecoder(dc)
	if err != nil {
		panic(err)
	}
	return d
}

// newTarget returns a pointer to a new target of the named type.
func newTarget(target string) any {
	switch target {
	case "Kitchen":
		return &Kitchen{}
	case "Remain":
		return &Remain{}
	case "EmbedNoTag":
		return &EmbedNoTag{}
	case "BadSquash":
		return &BadSquash{}
	case "RootLike":
		return &RootLike{}
	case "Pagination":
		return &config.Pagination{PagerSize: 10, Path: "page"}
	case "PageConfig":
		return &config.PageConfig{NextPrevSortOrder: "desc", NextPrevInSectionSortOrder: "desc"}
	case "SitemapConfig":
		return &config.SitemapConfig{Priority: -1, Filename: "sitemap.xml"}
	case "Minify":
		conf, err := minifiers.DecodeConfig(nil)
		if err != nil {
			panic(err)
		}
		return &conf
	}
	panic(target)
}

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "crates/nh-config/tests/fixtures/decode", "output directory")
	n := flag.Int("n", 400, "random inputs per target")
	seed := flag.Int64("seed", 1, "random seed")
	flag.Parse()

	r := rand.New(rand.NewSource(*seed))
	var cases []map[string]any

	// mapstructure over the oracle's structs, every decoder config.
	for _, target := range []string{"Kitchen", "Remain", "EmbedNoTag", "BadSquash", "RootLike"} {
		fields := fieldsOf(newTarget(target))
		for i := 0; i < *n*3; i++ {
			cfg := decoderConfigs[r.Intn(len(decoderConfigs))]
			in := randInput(r, fields, 2)
			var prefill any
			if r.Intn(3) == 0 {
				prefill = randPrefill(r, fields)
			}
			cases = append(cases, structCase(target, cfg, prefill, in))
		}
		// Every value of the pool into every field.
		for _, f := range fields {
			for _, v := range valuePool() {
				for _, cfg := range []string{"weak", "strict"} {
					cases = append(cases, structCase(target, cfg, nil, map[string]any{f.name: v}))
				}
			}
		}
	}
	// Non-map inputs to a struct and to the whole decode.
	for _, v := range valuePool() {
		cases = append(cases, structCase("Kitchen", "weak", nil, v))
	}

	// The config decoders.
	sections := configSections(*root)
	for _, target := range []string{"Pagination", "PageConfig", "SitemapConfig", "Minify", "RootLike"} {
		fields := fieldsOf(newTarget(target))
		for _, s := range sections[strings.ToLower(target)] {
			cases = append(cases, structCase(target, "weak", nil, s))
		}
		for i := 0; i < *n; i++ {
			cases = append(cases, structCase(target, "weak", nil, randInput(r, fields, 3)))
		}
	}
	for _, key := range []string{"build", "server", "security", "services", "privacy", "sitemap"} {
		var fields []field
		switch key {
		case "build":
			fields = fieldsOf(&config.BuildConfig{})
		case "server":
			fields = fieldsOf(&config.Server{})
		case "security":
			fields = fieldsOf(&security.Config{})
		case "services":
			fields = fieldsOf(&services.Config{})
		case "privacy":
			fields = fieldsOf(&privacy.Config{})
		case "sitemap":
			fields = fieldsOf(&config.SitemapConfig{})
		}
		var inputs []any
		inputs = append(inputs, sections[key]...)
		noFoldDup = true
		for i := 0; i < *n; i++ {
			inputs = append(inputs, randInput(r, fields, 3))
		}
		noFoldDup = false
		inputs = append(inputs, specialInputs(key)...)
		for i, in := range inputs {
			extra := map[string]any{}
			if i%3 == 1 {
				// Legacy root keys.
				extra = map[string]any{
					"enableInlineShortcodes": r.Intn(2) == 0, "googleAnalytics": "UA-legacy", "disqusShortname": "legacy",
					"rssLimit": []any{0, 5, "7", -1}[r.Intn(4)],
				}
			}
			cases = append(cases, providerCase(key, in, extra, i%2 == 0))
		}
	}

	header := map[string]any{"oracle": "nh-config/decode", "goarch": runtime.GOARCH}
	if err := os.MkdirAll(*out, 0o755); err != nil {
		log.Fatal(err)
	}
	if err := goval.WriteCasesGz(filepath.Join(*out, "decode.json.gz"), header, cases); err != nil {
		log.Fatal(err)
	}
	fmt.Fprintf(os.Stderr, "decode: %d cases\n", len(cases))
}

// structCase decodes in into a new target (after decoding prefill, when set, with WeakDecode).
func structCase(target, cfg string, prefill, in any) map[string]any {
	c := map[string]any{"target": target, "cfg": cfg, "in": goval.Encode(in)}
	t := newTarget(target)
	if prefill != nil {
		enc := goval.Encode(prefill)
		if err := mapstructure.WeakDecode(prefill, t); err != nil {
			// Only valid prefills are used.
			t = newTarget(target)
		} else {
			c["prefill"] = enc
		}
	}
	res := cval.Call(func() (any, error) {
		err := newDecoder(cfg, t).Decode(in)
		return t, err
	})
	for k, v := range res {
		c[k] = v
	}
	return c
}

// providerCase runs a config decoder on a provider holding in under key (and extra root keys).
func providerCase(key string, in any, extra map[string]any, mergeStrategy bool) map[string]any {
	root := map[string]any{key: in}
	for k, v := range extra {
		root[k] = v
	}
	c := map[string]any{"target": "provider:" + key, "in": goval.Encode(root), "mergeStrategy": mergeStrategy}
	cfg := config.New()
	cfg.Set("", root)
	if mergeStrategy {
		cfg.SetDefaultMergeStrategy()
	}
	res := cval.Call(func() (any, error) {
		switch key {
		case "build":
			return config.DecodeBuildConfig(cfg), nil
		case "server":
			return config.DecodeServer(cfg)
		case "security":
			return security.DecodeConfig(cfg)
		case "services":
			return services.DecodeConfig(cfg)
		case "privacy":
			return privacy.DecodeConfig(cfg)
		case "sitemap":
			return config.DecodeSitemap(config.SitemapConfig{Priority: -1, Filename: "sitemap.xml"}, cfg.GetStringMap(key))
		}
		panic(key)
	})
	for k, v := range res {
		c[k] = v
	}
	return c
}

type field struct {
	name string
	kind string // "struct", "slice", "map", "ptr" or a basic kind
	sub  []field
}

// fieldsOf returns the mapstructure field names of a struct (squashed structs flattened).
func fieldsOf(v any) []field {
	return fieldsOfType(reflect.TypeOf(v).Elem())
}

const (
	kindStruct = reflect.Struct
	kindPtr    = reflect.Ptr
	kindSlice  = reflect.Slice
	kindArray  = reflect.Array
	kindMap    = reflect.Map
)

func fieldsOfType(t reflect.Type) []field {
	var out []field
	for i := 0; i < t.NumField(); i++ {
		f := t.Field(i)
		if f.PkgPath != "" {
			continue
		}
		tag := f.Tag.Get("mapstructure")
		name := f.Name
		if p := strings.SplitN(tag, ",", 2)[0]; p != "" {
			name = p
		}
		if strings.Contains(tag, "squash") && f.Type.Kind() == kindStruct {
			out = append(out, fieldsOfType(f.Type)...)
			continue
		}
		fd := field{name: name, kind: f.Type.Kind().String()}
		ft := f.Type
		if ft.Kind() == kindPtr {
			ft = ft.Elem()
		}
		if ft.Kind() == kindSlice || ft.Kind() == kindArray || ft.Kind() == kindMap {
			if et := ft.Elem(); et.Kind() == kindStruct {
				fd.sub = fieldsOfType(et)
			} else if et.Kind() == kindPtr && et.Elem().Kind() == kindStruct {
				fd.sub = fieldsOfType(et.Elem())
			}
		}
		if ft.Kind() == kindStruct && ft.String() != "time.Time" {
			fd.kind = "struct"
			fd.sub = fieldsOfType(ft)
		}
		out = append(out, fd)
	}
	return out
}

// keyVariant returns name in a random case.
func keyVariant(r *rand.Rand, name string) string {
	switch r.Intn(5) {
	case 0, 1:
		return name
	case 2:
		return strings.ToLower(name)
	case 3:
		return strings.ToUpper(name)
	default:
		b := []rune(name)
		for i := range b {
			if r.Intn(2) == 0 {
				b[i] = []rune(strings.ToUpper(string(b[i])))[0]
			} else {
				b[i] = []rune(strings.ToLower(string(b[i])))[0]
			}
		}
		return string(b)
	}
}

func randInput(r *rand.Rand, fields []field, depth int) map[string]any {
	m := map[string]any{}
	seen := map[string]bool{}
	for _, f := range fields {
		if r.Intn(2) == 0 {
			continue
		}
		k := keyVariant(r, f.name)
		if seen[strings.ToLower(k)] {
			continue
		}
		seen[strings.ToLower(k)] = true
		m[k] = randFieldValue(r, f, depth)
		if !noFoldDup && r.Intn(25) == 0 && k != f.name {
			// The exact name too: it wins over the case-insensitive match.
			m[f.name] = randFieldValue(r, f, depth)
		}
	}
	for i := r.Intn(3); i > 0; i-- {
		k := []string{"unknown", "extra", "_merge", "Nope", "x.y"}[r.Intn(5)]
		if !seen[strings.ToLower(k)] {
			seen[strings.ToLower(k)] = true
			m[k] = randAny(r, depth-1)
		}
	}
	switch r.Intn(10) {
	case 0:
		return maps.Params(lowerKeys(m))
	}
	return m
}

// noFoldDup disables keys that differ only in case (the config provider lower-cases keys and
// keeps a random one of them).
var noFoldDup bool

// lowerKeys lower-cases the keys of m (of keys that fold to the same key, the smallest wins).
func lowerKeys(m map[string]any) map[string]any {
	var keys []string
	for k := range m {
		keys = append(keys, k)
	}
	sort.Sort(sort.Reverse(sort.StringSlice(keys)))
	out := map[string]any{}
	for _, k := range keys {
		out[strings.ToLower(k)] = m[k]
	}
	return out
}

// randPrefill returns a valid (WeakDecode-able) input.
func randPrefill(r *rand.Rand, fields []field) map[string]any {
	m := map[string]any{}
	for _, f := range fields {
		if r.Intn(2) == 0 {
			continue
		}
		if v := goodValue(r, f, 2); v != nil {
			m[f.name] = v
		}
	}
	return m
}

func goodValue(r *rand.Rand, f field, depth int) any {
	switch f.kind {
	case "string":
		return []string{"", "p", "prefilled"}[r.Intn(3)]
	case "bool":
		return r.Intn(2) == 0
	case "int", "int8", "int16", "int32", "int64", "uint", "uint8", "uint16", "uint32", "uint64":
		return r.Intn(100)
	case "float32", "float64":
		return 0.5 * float64(r.Intn(10))
	case "slice", "array":
		if f.sub != nil {
			var out []any
			for i := r.Intn(3); i > 0; i-- {
				out = append(out, randPrefill(r, f.sub))
			}
			return out
		}
		if f.name == "ArrI" || f.name == "IS" {
			return []any{r.Intn(9), r.Intn(9)}
		}
		if f.name == "FS" {
			return []any{1.5}
		}
		return []any{"p1", "p2"}[:1+r.Intn(2)]
	case "map":
		if f.sub != nil {
			return map[string]any{"k": randPrefill(r, f.sub)}
		}
		if f.name == "MSI" {
			return map[string]any{"a": 1, "b": 2}
		}
		return map[string]any{"a": "1", "b": "2"}
	case "struct", "ptr":
		if f.sub != nil && depth > 0 {
			return randPrefill(r, f.sub)
		}
		return nil
	case "interface":
		return []any{"s", 3, true, []any{"x"}, map[string]any{"q": 1}, 2.5, int64(7), uint8(9)}[r.Intn(8)]
	}
	return nil
}

func randFieldValue(r *rand.Rand, f field, depth int) any {
	if r.Intn(10) < 4 {
		return randAny(r, depth-1)
	}
	switch f.kind {
	case "struct", "ptr":
		if f.sub != nil && depth > 0 {
			if r.Intn(4) == 0 {
				return maps.Params(lowerKeys(randInput(r, f.sub, depth-1)))
			}
			return randInput(r, f.sub, depth-1)
		}
	case "slice", "array":
		var out []any
		for i := r.Intn(4); i > 0; i-- {
			if f.sub != nil && depth > 0 {
				out = append(out, randInput(r, f.sub, depth-1))
			} else {
				out = append(out, randScalar(r))
			}
		}
		if r.Intn(5) == 0 {
			ss := []string{}
			for _, x := range out {
				ss = append(ss, fmt.Sprint(x))
			}
			return ss
		}
		return out
	case "map":
		m := map[string]any{}
		for i := r.Intn(4); i > 0; i-- {
			k := []string{"a", "B", "c", "D", "_merge", "x.y", ""}[r.Intn(7)]
			if f.sub != nil && depth > 0 {
				m[k] = randInput(r, f.sub, depth-1)
			} else {
				m[k] = randScalar(r)
			}
		}
		return m
	}
	return randScalar(r)
}

func randScalar(r *rand.Rand) any {
	p := scalarPool()
	return p[r.Intn(len(p))]
}

func randAny(r *rand.Rand, depth int) any {
	if depth <= 0 || r.Intn(3) > 0 {
		return randScalar(r)
	}
	switch r.Intn(3) {
	case 0:
		return []any{randAny(r, depth-1), randAny(r, depth-1)}
	case 1:
		return map[string]any{"a": randAny(r, depth-1), "B": randAny(r, depth-1)}
	default:
		return maps.Params{"a": randAny(r, depth-1)}
	}
}

func scalarPool() []any {
	return []any{
		nil, true, false,
		0, 1, -1, 42, 300, -70000, math.MaxInt64, math.MinInt64,
		int8(-5), int8(127), int16(-300), int32(-70000), int64(1 << 40), int64(-1),
		uint(7), uint8(255), uint16(65535), uint32(1 << 31), uint64(math.MaxUint64),
		0.0, 1.5, -2.5, 2.0, 1e20, -1e20, math.Inf(1), math.Inf(-1), math.NaN(), float32(3.25), 1e-7,
		"", "0", "1", "true", "false", "t", "yes", "abc", "0x1F", "0b101", "0o17", "017", "1_000",
		"1e3", "-12", " 7", "3.9", "18446744073709551615", "18446744073709551616",
		"9223372036854775808", "-9223372036854775809", "NaN", "Inf", "-0", "Ünïcode",
		template.HTML("<b>html</b>"), maps.ParamsMergeStrategyDeep,
	}
}

func valuePool() []any {
	p := scalarPool()
	p = append(p,
		[]any{}, []any{"a"}, []any{"a", 1, true}, []string{"x", "y"}, []int{1, 2}, []byte("hi"), []float64{1.5},
		map[string]any{}, map[string]any{"a": 1, "B": "x"}, maps.Params{"x": "y"}, map[string]string{"k": "v"},
		map[string]any{"a": map[string]any{"b": 1}},
		map[string]any(nil), []string(nil), []any(nil), maps.Params(nil),
		time.Date(2024, 2, 29, 12, 30, 0, 5, time.UTC),
	)
	return p
}

// specialInputs are hand-written inputs for a config key.
func specialInputs(key string) []any {
	switch key {
	case "build":
		return []any{
			map[string]any{"writeStats": true}, map[string]any{"writestats": false, "buildStats": map[string]any{"disableTags": true}},
			map[string]any{"writestats": "true"}, map[string]any{"useResourceCacheWhen": "ALWAYS"}, map[string]any{"useResourceCacheWhen": "bogus"},
			map[string]any{"useResourceCacheWhen": 1}, map[string]any{"cacheBusters": []any{map[string]any{"source": "x"}}},
			map[string]any{"cacheBusters": []any{map[string]any{"source": "a", "target": "b"}, map[string]any{"target": "c"}}},
			map[string]any{"cacheBusters": []any{}}, map[string]any{"cacheBusters": "notalist"}, map[string]any{"noJSConfigInAssets": "1"},
			map[string]any{"buildstats": map[string]any{"enable": true, "disableIDs": 1}}, "notamap", nil,
		}
	case "server":
		return []any{
			map[string]any{"headers": []any{map[string]any{"for": "/**", "values": map[string]any{"X-Frame-Options": "DENY", "N": 1}}}},
			map[string]any{"redirects": []any{map[string]any{"from": "/a/**", "to": "/b/index.html", "status": 301}, map[string]any{"fromRe": "^/c/(.*)$", "to": "/d/$1", "force": true, "fromHeaders": map[string]any{"Accept": "*"}}}},
			map[string]any{"redirects": []any{}}, map[string]any{"redirects": "x"}, map[string]any{"redirects": []any{map[string]any{"status": "abc"}}},
		}
	case "security":
		return []any{
			map[string]any{"exec": map[string]any{"allow": []any{"^a$", "b"}, "osEnv": "PATH"}},
			map[string]any{"exec": map[string]any{"allow": "none"}}, map[string]any{"exec": map[string]any{"allow": []any{}}},
			map[string]any{"exec": map[string]any{"allow": []any{"", "  "}}}, map[string]any{"exec": map[string]any{"allow": []any{"a", "none", "b"}}},
			map[string]any{"funcs": map[string]any{"getenv": []any{"^HUGO_"}}}, map[string]any{"http": map[string]any{"urls": []any{"https://.*"}, "methods": "GET", "mediaTypes": []any{"application/json"}}},
			map[string]any{"enableInlineShortcodes": true}, map[string]any{"enableInlineShortcodes": "yes"}, map[string]any{"exec": "x"},
			map[string]any{"exec": map[string]any{"allow": 5}}, map[string]any{"exec": map[string]any{"allow": map[string]any{"a": 1}}},
			map[string]any{"exec": map[string]any{"allow": []any{1, 2.5, true}}},
		}
	case "services":
		return []any{
			map[string]any{"rss": map[string]any{"limit": 10}}, map[string]any{"rss": map[string]any{"limit": "20"}}, map[string]any{"rss": map[string]any{"limit": 0}},
			map[string]any{"googleAnalytics": map[string]any{"id": "G-1"}, "disqus": map[string]any{"shortname": "s"}},
			map[string]any{"instagram": map[string]any{"disableInlineCSS": true, "accessToken": "t"}, "twitter": map[string]any{"disableInlineCSS": "1"}, "x": map[string]any{"disableInlineCSS": 0}},
			map[string]any{"rss": "x"},
		}
	case "privacy":
		return []any{
			map[string]any{"youtube": map[string]any{"privacyEnhanced": true, "disable": "true"}, "vimeo": map[string]any{"enableDNT": 1, "simple": true}},
			map[string]any{"disqus": map[string]any{"disable": true, "simple": true}}, map[string]any{"googleAnalytics": map[string]any{"respectDoNotTrack": true, "service": map[string]any{"disable": true}}},
			map[string]any{"x": map[string]any{"enableDNT": "abc"}}, map[string]any{"twitter": "x"},
		}
	case "sitemap":
		return []any{
			map[string]any{"changefreq": "weekly", "priority": 0.5, "filename": "s.xml", "disable": true},
			map[string]any{"priority": "0.3"}, map[string]any{"priority": "abc"}, map[string]any{"changeFreq": 1},
		}
	}
	return nil
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

// configSections returns the config sections of the seeksnack dumps and docs/hugo.toml by
// lower-cased key ("rootlike" = the whole config).
func configSections(root string) map[string][]any {
	out := map[string][]any{}
	add := func(cfg map[string]any) {
		var keys []string
		for k := range cfg {
			keys = append(keys, k)
		}
		sort.Strings(keys)
		for _, k := range keys {
			out[strings.ToLower(k)] = append(out[strings.ToLower(k)], cfg[k])
		}
		out["rootlike"] = append(out["rootlike"], cfg)
		if p, ok := cfg["pagination"]; ok {
			out["pagination"] = append(out["pagination"], p)
		}
		if p, ok := cfg["page"]; ok {
			out["pageconfig"] = append(out["pageconfig"], p)
		}
		if p, ok := cfg["sitemap"]; ok {
			out["sitemapconfig"] = append(out["sitemapconfig"], p)
		}
	}
	for _, f := range []string{"config-en.json", "config-th.json", "config-en-printzero.json"} {
		add(readJSON(root, f))
	}
	b, err := os.ReadFile(filepath.Join(root, "docs/hugo.toml"))
	if err != nil {
		log.Fatal(err)
	}
	m, err := metadecoders.Default.UnmarshalToMap(b, metadecoders.TOML)
	if err != nil {
		log.Fatal(err)
	}
	add(m)
	return out
}
