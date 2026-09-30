// Command output is the Go oracle for output/* in crates/nh-media (Wave B
// task T04): the built-in output formats and output.DefaultFormats (sorted),
// DecodeConfig (with the seeksnack config, docs/hugo.toml and adversarial
// inputs: case-mixed keys, _merge keys, bad media types, zero, equal and
// negative weights, seeded random inputs) with its sort order, SourceHash and
// config dump (`hugo config` JSON), and the Formats and Format methods.
//
//	go run ./tools/go-oracle/nh-media/output [-root .] [-out rust/testdata/oracle/media/output] [-random 1500] [-seed 1]
//
// DecodeConfig visits its input map in Go's random order: new formats are
// appended in that order before sort.Sort (not stable, and Formats.Less is not
// a strict weak order with negative weights), and the first bad key met is
// reported; a format renamed by a "name" field can also be matched by a later
// key. Each decode is run 100 times (Go starts the iteration of a small map at
// a random one of its 8 slots, so one order of two keys is favoured 7 to 1);
// when the records differ the case is marked "nondet" and not compared.
// Inputs set through a provider have no keys that fold to each other (the
// provider keeps a random one of them).
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

	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/media"
	"github.com/neohugo/neohugo/output"
	"github.com/neohugo/neohugo/parser"
	"github.com/neohugo/neohugo/parser/metadecoders"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-config/cval"
)

const reruns = 100

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "rust/testdata/oracle/media/output", "output directory")
	nRandom := flag.Int("random", 1500, "number of random DecodeConfig inputs")
	seed := flag.Int64("seed", 1, "random seed")
	flag.Parse()

	var cases []map[string]any
	add := func(c map[string]any) { cases = append(cases, c) }

	customTypes, err := media.DecodeTypes(customTypesInput())
	if err != nil {
		log.Fatal(err)
	}
	typesByName := map[string]media.Types{"default": media.DefaultTypes, "custom": customTypes.Config}

	builtins := []output.Format{
		output.AMPFormat, output.CalendarFormat, output.CSSFormat, output.CSVFormat, output.HTMLFormat,
		output.AliasHTMLFormat, output.MarkdownFormat, output.JSONFormat, output.WebAppManifestFormat,
		output.RobotsTxtFormat, output.RSSFormat, output.SitemapFormat, output.SitemapIndexFormat,
		output.GotmplFormat, output.HTTPStatus404HTMLFormat,
	}
	add(map[string]any{"op": "Builtin", "formats": cval.Dump(builtins)})
	add(map[string]any{"op": "DefaultFormats", "formats": cval.Dump(output.DefaultFormats), "methods": formatMethods(output.DefaultFormats)})
	formatsQueryCases("default", output.DefaultFormats, add)

	for _, in := range inputs(*root) {
		for _, tn := range []string{"default", "custom"} {
			add(decodeCase(in.name, in.gen, in.viaProvider, tn, typesByName[tn]))
		}
	}
	for i := 0; i < *nRandom; i++ {
		via := i%3 == 0
		gen := func() any {
			v := randInput(rand.New(rand.NewSource(*seed*1000003 + int64(i))))
			if via {
				v = noFoldDup(v)
			}
			return v
		}
		tn := []string{"default", "custom"}[i%2]
		add(decodeCase(fmt.Sprintf("random:%d", i), gen, via, tn, typesByName[tn]))
	}

	// The Formats methods on a decoded custom set.
	ns, err := output.DecodeConfig(customTypes.Config, customFormatsInput())
	if err != nil {
		log.Fatal(err)
	}
	add(map[string]any{"op": "CustomFormats", "formats": cval.Dump(ns.Config), "methods": formatMethods(ns.Config)})
	formatsQueryCases("custom", ns.Config, add)

	header := map[string]any{
		"oracle": "nh-media/output", "goarch": runtime.GOARCH, "reruns": reruns,
		"customTypesInput": goval.Encode(customTypesInput()), "customFormatsInput": goval.Encode(customFormatsInput()),
	}
	if err := os.MkdirAll(*out, 0o755); err != nil {
		log.Fatal(err)
	}
	if err := goval.WriteCasesGz(filepath.Join(*out, "output.json.gz"), header, cases); err != nil {
		log.Fatal(err)
	}
	nondet := 0
	for _, c := range cases {
		if c["nondet"] == true {
			nondet++
		}
	}
	fmt.Fprintf(os.Stderr, "output: %d cases (%d nondet)\n", len(cases), nondet)
}

func m(kv ...any) map[string]any {
	out := map[string]any{}
	for i := 0; i < len(kv); i += 2 {
		out[kv[i].(string)] = kv[i+1]
	}
	return out
}

func str(s string) any { return goval.Str(s) }

func errOut(err error) any {
	if err == nil {
		return nil
	}
	return goval.Str(err.Error())
}

func merge(a, b map[string]any) map[string]any {
	for k, v := range b {
		a[k] = v
	}
	return a
}

// stable runs f reruns times; when the (JSON) records differ, it returns
// {"nondet": true}.
func stable(f func() map[string]any) map[string]any {
	first := f()
	b0, err := json.Marshal(first)
	if err != nil {
		log.Fatal(err)
	}
	for i := 1; i < reruns; i++ {
		b, err := json.Marshal(f())
		if err != nil {
			log.Fatal(err)
		}
		if string(b) != string(b0) {
			return map[string]any{"nondet": true}
		}
	}
	return first
}

// The same custom media types as the media oracle.
func customTypesInput() map[string]any {
	return m(
		"text/x-custom", m("suffixes", []any{"cst", "CUS"}),
		"application/x-thing+json", m("Suffixes", []string{"thing"}, "delimiter", "_"),
		"text/html", m("suffixes", []any{"html", "htm", "xhtml"}),
		"image/svg+xml", m("suffixes", "svg svgz"),
		"text/x-nosuffix", m(),
		"application/rss+xml", m("suffixes", []any{"xml", "rss", "feed"}),
		"text/x-md2", m("suffixes", []any{"md"}),
	)
}

func customFormatsInput() map[string]any {
	return m(
		"custom", m("mediaType", "text/x-custom", "baseName", "cbase", "isPlainText", true, "weight", 3),
		"thing", m("mediaType", "application/x-thing+json", "path", "things", "rel", "thing"),
		"html", m("weight", 20, "baseName", "home"),
		"nosuffix", m("mediaType", "text/x-nosuffix", "weight", 1),
		"feed", m("mediaType", "application/rss+xml", "noUgly", true),
		"md2", m("mediatype", "text/x-md2"),
	)
}

// ---------------------------------------------------------------------------
// Format and Formats methods

func formatMethods(fs output.Formats) []any {
	var out []any
	for _, f := range fs {
		b, err := f.MarshalJSON()
		out = append(out, map[string]any{
			"BaseFilename": str(f.BaseFilename()), "IsZero": f.IsZero(), "MarshalJSON": []any{str(string(b)), errOut(err)},
		})
	}
	return out
}

func found(f output.Format, ok bool) any {
	return []any{cval.Dump(f), ok}
}

var nameProbes = []string{
	"html", "HTML", "Html", "rss", "amp", "json", "css", "csv", "calendar", "404", "alias", "markdown", "robots",
	"sitemap", "sitemapindex", "gotmpl", "webappmanifest", "", "nope", "custom", "thing", "feed", "md2", "nosuffix",
	"ics", "xml", "md", "txt", "webmanifest", "cst", "CUS", "htm", "_redirects", "redirects",
}

var fileProbes = []string{
	"mytemplate.amp.html", "mytemplate.html", "mytemplate", "list.rss.xml", "index.xml", "single.json", "a.b.c.d",
	"robots.txt", "_redirects", "x.redirects", "x.calendar", "x.ics", "x.HTML", "x.md", "baseof.custom.cst",
	"x.nope", "x.", ".html", "", "a..html", "x.cst", "x.thing", "x.webmanifest", "x.404.html", "sitemap.xml",
}

func formatsQueryCases(fn string, fs output.Formats, add func(map[string]any)) {
	for _, q := range nameProbes {
		fs2, err := fs.GetByNames(q, "html")
		fs3, err3 := fs.GetByNames("rss", q)
		add(map[string]any{
			"op": "FormatsQuery", "formats": fn, "q": str(q),
			"GetBySuffix": found(fs.GetBySuffix(q)), "GetByName": found(fs.GetByName(q)),
			"GetByNames": []any{cval.Dump(fs2), errOut(err)}, "GetByNames2": []any{cval.Dump(fs3), errOut(err3)},
		})
	}
	for _, f := range fileProbes {
		add(map[string]any{"op": "FromFilename", "formats": fn, "q": str(f), "out": found(fs.FromFilename(f))})
	}
}

// ---------------------------------------------------------------------------
// DecodeConfig

type input struct {
	name        string
	gen         func() any
	viaProvider bool // Set the input as "outputFormats" in a config.Provider and Get it (like allconfig)
}

func readTOMLSection(root, file, key string) any {
	b, err := os.ReadFile(filepath.Join(root, file))
	if err != nil {
		log.Fatal(err)
	}
	cfg, err := metadecoders.Default.UnmarshalToMap(b, metadecoders.TOML)
	if err != nil {
		log.Fatal(err)
	}
	for k, v := range cfg {
		if strings.EqualFold(k, key) {
			return v
		}
	}
	return nil
}

func readJSONSection(root, file, key string) any {
	b, err := os.ReadFile(filepath.Join(root, "docs/rust-port/specs/architecture-core-data", file))
	if err != nil {
		log.Fatal(err)
	}
	var cfg map[string]any
	if err := json.Unmarshal(b, &cfg); err != nil {
		log.Fatal(err)
	}
	return cfg[key]
}

func inputs(root string) []input {
	c := func(v func() any) func() any { return v }
	return []input{
		{"nil", c(func() any { return nil }), false},
		{"empty", c(func() any { return map[string]any{} }), false},
		// The seeksnack build: no [outputFormats] (Get gives nil).
		{"seeksnack", c(func() any { return nil }), true},
		// The config dump of the seeksnack build (the defaults with mediaType strings), decoded again.
		{"seeksnack-dump-en", c(func() any { return readJSONSection(root, "config-en.json", "outputformats") }), true},
		{"seeksnack-dump-th", c(func() any { return readJSONSection(root, "config-th.json", "outputformats") }), true},
		{"docs-hugo.toml", c(func() any { return readTOMLSection(root, "docs/hugo.toml", "outputFormats") }), true},
		{"custom", c(func() any { return customFormatsInput() }), false},
		{"custom-provider", c(func() any { return customFormatsInput() }), true},
		{"case-mixed", c(func() any {
			return m("HTML", m("MediaType", "TEXT/HTML", "BaseName", "x"), "MyFormat", m("MEDIATYPE", "application/json", "IsPlainText", "true"))
		}), false},
		{"case-mixed-provider", c(func() any {
			return m("HTML", m("MediaType", "TEXT/HTML", "BaseName", "x"), "MyFormat", m("MEDIATYPE", "application/json", "IsPlainText", "true"))
		}), true},
		{"merge-key", c(func() any { return m("_merge", "deep", "html", m("_merge", "none", "weight", 5)) }), false},
		{"merge-key-provider", c(func() any { return m("_merge", "deep", "html", m("weight", 5)) }), true},
		{"override-html", c(func() any { return m("html", m("weight", 0, "rel", "x", "isHTML", false)) }), false},
		{"override-rss-mediatype", c(func() any { return m("rss", m("mediaType", "application/xml")) }), false},
		{"weights", c(func() any {
			return m("a", m("mediaType", "text/html", "weight", 5), "b", m("mediaType", "text/html", "weight", 5),
				"c", m("mediaType", "text/html", "weight", 1), "d", m("mediaType", "text/html"), "e", m("mediaType", "text/html", "weight", 11))
		}), false},
		{"negative-weight", c(func() any { return m("neg", m("mediaType", "text/html", "weight", -1)) }), false},
		{"negative-weights", c(func() any {
			return m("neg1", m("mediaType", "text/html", "weight", -1), "neg2", m("mediaType", "text/plain", "weight", -5))
		}), false},
		{"weight-string", c(func() any { return m("w", m("mediaType", "text/html", "weight", "7")) }), false},
		{"weight-float", c(func() any { return m("w", m("mediaType", "text/html", "weight", 2.5)) }), false},
		{"weight-bad", c(func() any { return m("w", m("mediaType", "text/html", "weight", "x")) }), false},
		{"bool-string", c(func() any { return m("b", m("mediaType", "text/html", "isPlainText", "1", "ugly", "false", "root", 1)) }), false},
		{"bool-bad", c(func() any { return m("b", m("isPlainText", "maybe")) }), false},
		{"no-mediatype", c(func() any { return m("nomt", m("baseName", "x")) }), false},
		{"unknown-mediatype", c(func() any { return m("x", m("mediaType", "text/x-nope")) }), false},
		{"mediatype-by-suffix", c(func() any { return m("x", m("mediaType", "html")) }), false},
		{"mediatype-main-sub", c(func() any { return m("x", m("mediaType", "text/HTML")) }), false},
		{"mediatype-int", c(func() any { return m("x", m("mediaType", 42)) }), false},
		{"mediatype-map", c(func() any { return m("x", m("mediaType", m("type", "text/html"))) }), false},
		{"mediatype-nil", c(func() any { return m("x", m("mediaType", nil)) }), false},
		{"mediatype-list", c(func() any { return m("x", m("mediaType", []any{"text/html"})) }), false},
		{"mediatype-twice", c(func() any { return m("x", m("mediaType", "text/html", "MEDIATYPE", "application/json")) }), false},
		{"name-field", c(func() any { return m("x", m("name", "other", "mediaType", "text/css")) }), false},
		{"unknown-field", c(func() any { return m("x", m("nope", 1)) }), false},
		{"value-string", c(func() any { return m("x", "html") }), false},
		{"value-nil", c(func() any { return m("x", nil) }), false},
		{"value-string-map", c(func() any { return m("x", map[string]string{"baseName": "b", "rel": "r"}) }), false},
		{"value-string-map-mediatype", c(func() any { return m("x", map[string]string{"mediaType": "text/html"}) }), false},
		{"input-string", c(func() any { return "html" }), false},
		{"input-list", c(func() any { return []any{m("a", 1)} }), false},
		{"input-string-map", c(func() any { return map[string]string{"a": "b"} }), false},
		{"input-params", c(func() any { return maps.Params(m("html", m("weight", 3))) }), false},
		{"input-string-provider", c(func() any { return "html" }), true},
		{"nested-mediatype-in-path", c(func() any { return m("x", m("path", m("mediaType", "text/html"))) }), false},
		{"empty-name", c(func() any { return m("", m("mediaType", "text/html")) }), false},
		{"upper-dup", c(func() any { return m("rss", m("weight", 2), "RSS", m("weight", 3)) }), false},
		{"custom-types-only", c(func() any { return m("cst", m("mediaType", "text/x-custom")) }), false},
	}
}

// decode runs DecodeConfig on in (via a provider when asked, like allconfig:
// p.Get("outputformats")).
func decode(in any, viaProvider bool, types media.Types) (*config.ConfigNamespace[map[string]output.OutputFormatConfig, output.Formats], error) {
	if viaProvider {
		p := config.New()
		if in != nil {
			p.Set("outputFormats", in)
		}
		return output.DecodeConfig(types, p.Get("outputformats"))
	}
	return output.DecodeConfig(types, in)
}

func decodeCase(name string, gen func() any, viaProvider bool, tn string, types media.Types) map[string]any {
	c := map[string]any{"op": "DecodeConfig", "name": name, "in": goval.Encode(gen()), "viaProvider": viaProvider, "types": tn}
	return merge(c, stable(func() map[string]any {
		return goval.CallRaw(func() (any, error) {
			ns, err := decode(gen(), viaProvider, types)
			if err != nil {
				return nil, err
			}
			out := map[string]any{"formats": cval.Dump(ns.Config), "hash": ns.SourceHash}
			b, err := ns.MarshalJSON()
			out["json"] = []any{str(string(b)), errOut(err)}
			for _, omit := range []bool{true, false} {
				b, err := parser.ReplacingJSONMarshaller{Value: ns, KeysToLower: true, OmitEmpty: omit}.MarshalJSON()
				out[fmt.Sprintf("dump:%v", omit)] = []any{str(string(b)), errOut(err)}
			}
			return out, nil
		})
	}))
}

var namePool = []string{"html", "rss", "amp", "json", "404", "myformat", "MyFormat", "_merge", "x", "y", "z", "css", "calendar"}
var fieldPool = []string{
	"mediaType", "MediaType", "mediatype", "path", "baseName", "rel", "protocol", "isPlainText", "isHTML", "noUgly",
	"ugly", "notAlternative", "root", "permalinkable", "weight", "Weight", "name", "unknown",
}
var mediaTypePool = []string{
	"text/html", "TEXT/HTML", "application/json", "text/plain", "text/css", "application/rss+xml", "text/x-custom",
	"text/x-nope", "html", "application/x-thing+json", "text/x-md2", "", "text/calendar",
}

func randScalar(r *rand.Rand) any {
	switch r.Intn(8) {
	case 0:
		return int64(r.Intn(21) - 5)
	case 1:
		return r.Intn(2) == 0
	case 2:
		return 1.5
	case 3:
		return nil
	case 4:
		return []string{"true", "0", "1", "x", ""}[r.Intn(5)]
	default:
		return []string{"a", "b", "index", "webcal://", "", "amp"}[r.Intn(6)]
	}
}

func randField(r *rand.Rand, f string) any {
	switch strings.ToLower(f) {
	case "mediatype":
		if r.Intn(8) == 0 {
			return randScalar(r)
		}
		return mediaTypePool[r.Intn(len(mediaTypePool))]
	case "weight":
		if r.Intn(6) == 0 {
			return randScalar(r)
		}
		return int64(r.Intn(8) - 2)
	case "isplaintext", "ishtml", "nougly", "ugly", "notalternative", "root", "permalinkable":
		if r.Intn(6) == 0 {
			return randScalar(r)
		}
		return r.Intn(2) == 0
	}
	if r.Intn(8) == 0 {
		return randScalar(r)
	}
	return []string{"a", "b", "index", "webcal://", "", "amp"}[r.Intn(6)]
}

func randInput(r *rand.Rand) any {
	if r.Intn(40) == 0 {
		return nil
	}
	out := map[string]any{}
	for i := r.Intn(4); i > 0; i-- {
		k := namePool[r.Intn(len(namePool))]
		if k == "_merge" {
			out[k] = []string{"deep", "none", "shallow"}[r.Intn(3)]
			continue
		}
		if r.Intn(20) == 0 {
			out[k] = randScalar(r)
			continue
		}
		mm := map[string]any{}
		if r.Intn(3) > 0 {
			mm["mediaType"] = mediaTypePool[r.Intn(4)]
		}
		for j := r.Intn(4); j > 0; j-- {
			f := fieldPool[r.Intn(len(fieldPool))]
			mm[f] = randField(r, f)
		}
		out[k] = mm
	}
	return out
}

// noFoldDup drops, recursively, the map keys that fold to an earlier key (in
// sorted order): a provider lower-cases keys with Go's random map order, so it
// keeps a random one of such keys (nh-config's provider oracle covers that).
func noFoldDup(v any) any {
	switch x := v.(type) {
	case map[string]any:
		keys := make([]string, 0, len(x))
		for k := range x {
			keys = append(keys, k)
		}
		sort.Strings(keys)
		out := map[string]any{}
		seen := map[string]bool{}
		for _, k := range keys {
			if seen[strings.ToLower(k)] {
				continue
			}
			seen[strings.ToLower(k)] = true
			out[k] = noFoldDup(x[k])
		}
		return out
	case map[string]string:
		keys := make([]string, 0, len(x))
		for k := range x {
			keys = append(keys, k)
		}
		sort.Strings(keys)
		out := map[string]string{}
		seen := map[string]bool{}
		for _, k := range keys {
			if seen[strings.ToLower(k)] {
				continue
			}
			seen[strings.ToLower(k)] = true
			out[k] = x[k]
		}
		return out
	}
	return v
}
