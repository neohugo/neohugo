// Command media is the Go oracle for media/* in crates/nh-media (Wave B task
// T04): media.Builtin and media.DefaultTypes, DecodeTypes and
// DecodeContentTypes (with the seeksnack config, docs/hugo.toml and
// adversarial inputs: case-mixed keys, _merge keys, invalid keys and values,
// seeded random inputs), their SourceHash and config dump (`hugo config`
// JSON), the Types and Type methods, FromString, FromStringAndExt, the
// ContentTypes methods, http.DetectContentType and FromContent over a byte
// corpus.
//
//	go run ./tools/go-oracle/nh-media/media [-root .] [-out rust/testdata/oracle/media/media] [-random 1500] [-seed 1]
//
// Go iterates maps in random order, so a decode whose result depends on that
// order (several bad keys: the first one met is reported) is run 100 times
// (Go starts the iteration of a small map at a random one of its 8 slots, so
// one order of two keys is favoured 7 to 1); when the records differ the case
// is marked "nondet" and not compared. Inputs set through a provider have no
// keys that fold to each other (the provider keeps a random one of them).
package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"log"
	"math/rand"
	"net/http"
	"os"
	"path/filepath"
	"reflect"
	"runtime"
	"sort"
	"strings"

	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/media"
	"github.com/neohugo/neohugo/parser"
	"github.com/neohugo/neohugo/parser/metadecoders"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-config/cval"
)

const reruns = 100

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "rust/testdata/oracle/media/media", "output directory")
	nRandom := flag.Int("random", 1500, "number of random DecodeTypes/DecodeContentTypes inputs")
	seed := flag.Int64("seed", 1, "random seed")
	flag.Parse()

	var cases []map[string]any
	add := func(c map[string]any) { cases = append(cases, c) }

	builtinCases(add)
	for _, in := range typesInputs(*root) {
		add(decodeTypesCase(in.name, in.gen, in.viaProvider))
	}
	for i := 0; i < *nRandom; i++ {
		via := i%3 == 0
		gen := func() any {
			v := randTypesInput(rand.New(rand.NewSource(*seed*1000003 + int64(i))))
			if via {
				v = noFoldDup(v)
			}
			return v
		}
		add(decodeTypesCase(fmt.Sprintf("random:%d", i), gen, via))
	}
	customTypes := mustTypes(customTypesInput())
	for _, in := range contentTypesInputs() {
		for _, tn := range []string{"default", "custom"} {
			types := media.DefaultTypes
			if tn == "custom" {
				types = customTypes
			}
			add(decodeContentTypesCase(in.name, in.gen, tn, types))
		}
	}
	for i := 0; i < *nRandom/3; i++ {
		gen := func() any { return randContentTypesInput(rand.New(rand.NewSource(*seed*7919 + int64(i)))) }
		add(decodeContentTypesCase(fmt.Sprintf("random:%d", i), gen, "custom", customTypes))
	}
	for _, tn := range []string{"default", "custom"} {
		types := media.DefaultTypes
		if tn == "custom" {
			types = customTypes
		}
		typesMethodCases(tn, types, add)
		fromContentCases(tn, types, add)
	}
	fromStringCases(add)
	detectCases(*seed, add)

	header := map[string]any{
		"oracle": "nh-media/media", "goarch": runtime.GOARCH, "reruns": reruns,
		"customTypesInput": goval.Encode(customTypesInput()),
	}
	if err := os.MkdirAll(*out, 0o755); err != nil {
		log.Fatal(err)
	}
	if err := goval.WriteCasesGz(filepath.Join(*out, "media.json.gz"), header, cases); err != nil {
		log.Fatal(err)
	}
	nondet := 0
	for _, c := range cases {
		if c["nondet"] == true {
			nondet++
		}
	}
	fmt.Fprintf(os.Stderr, "media: %d cases (%d nondet)\n", len(cases), nondet)
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

func merge(a, b map[string]any) map[string]any {
	for k, v := range b {
		a[k] = v
	}
	return a
}

func errOut(err error) any {
	if err == nil {
		return nil
	}
	return goval.Str(err.Error())
}

func str(s string) any { return goval.Str(s) }

func strs(ss []string) any {
	if ss == nil {
		return nil
	}
	out := []any{}
	for _, s := range ss {
		out = append(out, goval.Str(s))
	}
	return out
}

// dumpJSON is the namespace's config dump: the JSON of `hugo config` (with and
// without zero values) and the plain MarshalJSON.
func dumpJSON(ns json.Marshaler) map[string]any {
	out := map[string]any{}
	b, err := ns.MarshalJSON()
	out["json"] = []any{str(string(b)), errOut(err)}
	for _, omit := range []bool{true, false} {
		b, err := parser.ReplacingJSONMarshaller{Value: ns, KeysToLower: true, OmitEmpty: omit}.MarshalJSON()
		out[fmt.Sprintf("dump:%v", omit)] = []any{str(string(b)), errOut(err)}
	}
	return out
}

// ---------------------------------------------------------------------------
// Builtin, DefaultTypes

func builtinCases(add func(map[string]any)) {
	v := reflect.ValueOf(media.Builtin)
	var fields []any
	for i := 0; i < v.NumField(); i++ {
		fields = append(fields, []any{v.Type().Field(i).Name, cval.Dump(v.Field(i).Interface())})
	}
	add(map[string]any{"op": "Builtin", "fields": fields})
	add(map[string]any{"op": "DefaultTypes", "types": cval.Dump(media.DefaultTypes)})
	dc := media.DefaultContentTypes
	add(map[string]any{"op": "DefaultContentTypes", "out": contentTypesDump(dc)})
}

// ---------------------------------------------------------------------------
// DecodeTypes

type input struct {
	name        string
	gen         func() any
	viaProvider bool // Set the input as "mediaTypes" in a config.Provider and GetStringMap it
}

func m(kv ...any) map[string]any {
	out := map[string]any{}
	for i := 0; i < len(kv); i += 2 {
		out[kv[i].(string)] = kv[i+1]
	}
	return out
}

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

func mustTypes(in map[string]any) media.Types {
	ns, err := media.DecodeTypes(in)
	if err != nil {
		log.Fatal(err)
	}
	return ns.Config
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

func typesInputs(root string) []input {
	c := func(v func() any) func() any { return v }
	return []input{
		{"nil", c(func() any { return nil }), false},
		{"empty", c(func() any { return map[string]any{} }), false},
		// The seeksnack build: no [mediaTypes] (GetStringMap gives an empty map).
		{"seeksnack", c(func() any { return nil }), true},
		// The config dump of the seeksnack build (the defaults), decoded again.
		{"seeksnack-dump-en", c(func() any { return readJSONSection(root, "config-en.json", "mediatypes") }), true},
		{"seeksnack-dump-th", c(func() any { return readJSONSection(root, "config-th.json", "mediatypes") }), true},
		{"docs-hugo.toml", c(func() any { return readTOMLSection(root, "docs/hugo.toml", "mediaTypes") }), true},
		{"custom", c(func() any { return customTypesInput() }), false},
		{"custom-provider", c(func() any { return customTypesInput() }), true},
		{"case-mixed", c(func() any {
			return m("Text/X-Mixed", m("SUFFIXES", []any{" A ", "B"}, "Delimiter", ""), "TEXT/HTML", m("Suffixes", []any{"HTML"}))
		}), false},
		{"merge-key", c(func() any {
			return m("_merge", "deep", "text/x-a", m("_merge", "shallow", "suffixes", []any{"a"}))
		}), false},
		{"merge-key-provider", c(func() any {
			return m("_merge", "none", "text/x-a", m("suffixes", []any{"a"}))
		}), true},
		{"override-fields", c(func() any {
			return m("text/x-o", m("type", "x/y", "mainType", "zzz", "subType", "qqq", "suffixesCSV", "p,q", "firstSuffix", m("suffix", "f", "fullSuffix", ".f"), "mimeSuffix", "m"))
		}), false},
		{"suffix-string", c(func() any { return m("text/x-s", m("suffixes", "a,b c")) }), false},
		{"suffix-int", c(func() any { return m("text/x-s", m("suffixes", 42)) }), false},
		{"suffix-mixed-list", c(func() any { return m("text/x-s", m("suffixes", []any{1, true, 2.5, "x", nil})) }), false},
		{"suffix-empty-list", c(func() any { return m("text/x-s", m("suffixes", []any{})) }), false},
		{"suffix-blank", c(func() any { return m("text/x-s", m("suffixes", []any{" ", ""})) }), false},
		{"delimiter-only", c(func() any { return m("text/x-d", m("delimiter", "-")) }), false},
		{"delimiter-list", c(func() any { return m("text/x-d", m("delimiter", []any{"a"})) }), false},
		{"delimiter-int", c(func() any { return m("text/x-d", m("delimiter", 7, "suffixes", []any{"d"})) }), false},
		{"value-string", c(func() any { return m("text/x-v", "html") }), false},
		{"value-nil", c(func() any { return m("text/x-v", nil) }), false},
		{"value-list", c(func() any { return m("text/x-v", []any{"a"}) }), false},
		{"value-string-map", c(func() any { return m("text/x-v", map[string]string{"suffixes": "ss", "delimiter": "+"}) }), false},
		{"bad-key-noslash", c(func() any { return m("textplain", m()) }), false},
		{"bad-key-twoslash", c(func() any { return m("a/b/c", m()) }), false},
		{"key-params", c(func() any { return m("text/plain; charset=utf-8", m("suffixes", []any{"t"}), "a/b+c+d", m()) }), false},
		{"key-empty-parts", c(func() any { return m("/", m(), "text/", m("suffixes", []any{"e"})) }), false},
		{"two-bad-keys", c(func() any { return m("bad1", m(), "bad2", m()) }), false},
		{"input-string", c(func() any { return "text/html" }), true},
		{"input-list", c(func() any { return []any{m("a", 1)} }), true},
		{"input-params", c(func() any { return config.New().GetStringMap("x") }), false},
		{"dup-fold", c(func() any { return m("text/x-f", m("suffixes", []any{"a"}), "TEXT/X-F", m("suffixes", []any{"b"})) }), false},
		{"default-shadowed", c(func() any { return m("TEXT/CSS", m("suffixes", []any{"css2"})) }), false},
	}
}

// decodeTypes runs DecodeTypes on in (via a provider when asked, like
// allconfig: p.GetStringMap("mediatypes")).
func decodeTypes(in any, viaProvider bool) (*config.ConfigNamespace[map[string]media.MediaTypeConfig, media.Types], error) {
	if viaProvider {
		p := config.New()
		if in != nil {
			p.Set("mediaTypes", in)
		}
		return media.DecodeTypes(p.GetStringMap("mediatypes"))
	}
	x, _ := in.(map[string]any)
	return media.DecodeTypes(x)
}

func decodeTypesCase(name string, gen func() any, viaProvider bool) map[string]any {
	in0 := gen()
	c := map[string]any{"op": "DecodeTypes", "name": name, "in": goval.Encode(in0), "viaProvider": viaProvider}
	if _, ok := in0.(map[string]any); !ok && in0 != nil && !viaProvider {
		c["skip"] = "not a map[string]any"
		return c
	}
	return merge(c, stable(func() map[string]any {
		return goval.CallRaw(func() (any, error) {
			ns, err := decodeTypes(gen(), viaProvider)
			if err != nil {
				return nil, err
			}
			return merge(map[string]any{
				"types": cval.Dump(ns.Config), "hash": ns.SourceHash,
				"typeMethods": typeMethods(ns.Config),
			}, dumpJSON(ns)), nil
		})
	}))
}

var suffixPool = []string{"html", "htm", "xml", "rss", "json", "md", "css", "js", "a", "B", " c ", "", "svg", "txt", "x.y", "é", "HTML", "yaml"}
var keyPool = []string{
	"text/html", "text/x-a", "Text/X-A", "application/x-b+json", "image/png", "text/css", "application/rss+xml",
	"text/x-c", "font/x-d", "_merge", "text/x-a;charset=utf-8", "badkey", "a/b/c", "TEXT/PLAIN", "text/plain",
}
var fieldPool = []string{
	"suffixes", "Suffixes", "SUFFIXES", "delimiter", "Delimiter", "type", "mainType", "subtype", "suffixesCSV",
	"firstSuffix", "mimeSuffix", "_merge", "unknown",
}

func randScalar(r *rand.Rand) any {
	switch r.Intn(9) {
	case 0:
		return int64(r.Intn(5))
	case 1:
		return r.Intn(2) == 0
	case 2:
		return 1.5
	case 3:
		return nil
	default:
		return suffixPool[r.Intn(len(suffixPool))]
	}
}

func randFieldValue(r *rand.Rand, field string) any {
	switch strings.ToLower(field) {
	case "suffixes":
		switch r.Intn(6) {
		case 0:
			return suffixPool[r.Intn(len(suffixPool))]
		case 1:
			return randScalar(r)
		case 2:
			var ss []string
			for i := r.Intn(3); i > 0; i-- {
				ss = append(ss, suffixPool[r.Intn(len(suffixPool))])
			}
			return ss
		default:
			l := []any{}
			for i := r.Intn(4); i > 0; i-- {
				if r.Intn(6) == 0 {
					l = append(l, randScalar(r))
				} else {
					l = append(l, suffixPool[r.Intn(len(suffixPool))])
				}
			}
			return l
		}
	case "delimiter":
		if r.Intn(5) == 0 {
			return randScalar(r)
		}
		return []string{".", "", "-", "_", "+"}[r.Intn(5)]
	case "firstsuffix":
		if r.Intn(2) == 0 {
			return m("suffix", "s", "FullSuffix", ".s")
		}
		return randScalar(r)
	case "_merge":
		return []string{"deep", "shallow", "none", "x"}[r.Intn(4)]
	}
	return randScalar(r)
}

func randTypesInput(r *rand.Rand) any {
	if r.Intn(40) == 0 {
		return nil
	}
	out := map[string]any{}
	for i := r.Intn(5); i > 0; i-- {
		k := keyPool[r.Intn(len(keyPool))]
		if k == "_merge" {
			out[k] = randFieldValue(r, k)
			continue
		}
		var v any
		switch r.Intn(12) {
		case 0:
			v = randScalar(r)
		case 1:
			mm := map[string]string{}
			for j := r.Intn(3); j > 0; j-- {
				f := fieldPool[r.Intn(len(fieldPool))]
				mm[f] = suffixPool[r.Intn(len(suffixPool))]
			}
			v = mm
		default:
			mm := map[string]any{}
			for j := r.Intn(4); j > 0; j-- {
				f := fieldPool[r.Intn(len(fieldPool))]
				mm[f] = randFieldValue(r, f)
			}
			v = mm
		}
		out[k] = v
	}
	return out
}

// ---------------------------------------------------------------------------
// DecodeContentTypes

var suffixProbes = []string{
	"html", "htm", "md", "markdown", "mdown", "adoc", "asciidoc", "ad", "pdc", "rst", "org", "xml", "json", "",
	"HTML", "MD", "cst", "CUS", "cus", "xhtml", "txt", ".md", "pandoc", "mmark", "thing",
}

var fileProbes = []string{
	"a.md", "index.md", "_index.md", "x/y/index.adoc", "index", "index.", "a.MD", ".md", "a.b.org", "index.html",
	"_index.htm", "/abs/_index.rst", "c:\\x\\index.md", "index.md/", "indexes.md", "_indexx.md", "p/index.cst",
	"a.pdc", "a.txt", "",
}

func contentTypesDump(c media.ContentTypes) map[string]any {
	out := map[string]any{
		"HTML": cval.Dump(c.HTML), "Markdown": cval.Dump(c.Markdown), "AsciiDoc": cval.Dump(c.AsciiDoc),
		"Pandoc": cval.Dump(c.Pandoc), "ReStructuredText": cval.Dump(c.ReStructuredText), "EmacsOrgMode": cval.Dump(c.EmacsOrgMode),
		"Types": cval.Dump(c.Types()),
	}
	var sfx, html, file, index []any
	for _, s := range suffixProbes {
		sfx = append(sfx, c.IsContentSuffix(s))
		html = append(html, c.IsHTMLSuffix(s))
	}
	for _, f := range fileProbes {
		file = append(file, c.IsContentFile(f))
		index = append(index, c.IsIndexContentFile(f))
	}
	out["IsContentSuffix"] = sfx
	out["IsHTMLSuffix"] = html
	out["IsContentFile"] = file
	out["IsIndexContentFile"] = index
	return out
}

func contentTypesInputs() []input {
	c := func(v func() any) func() any { return v }
	return []input{
		{"nil", c(func() any { return nil }), false},
		{"empty", c(func() any { return map[string]any{} }), false},
		{"html-only", c(func() any { return m("text/html", m()) }), false},
		{"upper", c(func() any { return m("TEXT/HTML", m(), "Text/Markdown", map[string]any{}) }), false},
		{"custom", c(func() any { return m("text/x-custom", m(), "text/markdown", m()) }), false},
		{"merge", c(func() any { return m("_merge", "deep", "text/html", m()) }), false},
		{"only-merge", c(func() any { return m("_merge", "deep") }), false},
		{"unknown", c(func() any { return m("text/x-nope", m()) }), false},
		{"two-unknown", c(func() any { return m("text/x-nope", m(), "text/x-nope2", m()) }), false},
		{"value-string", c(func() any { return m("text/html", "x") }), false},
		{"value-nil", c(func() any { return m("text/html", nil) }), false},
		{"value-fields", c(func() any { return m("text/html", m("a", 1)) }), false},
		{"value-list", c(func() any { return m("text/html", []any{}) }), false},
		{"suffix-key", c(func() any { return m("html", m()) }), false},
		{"subtype-key", c(func() any { return m("text/x-md2", m(), "text/html", m()) }), false},
	}
}

func decodeContentTypesCase(name string, gen func() any, typesName string, types media.Types) map[string]any {
	in0 := gen()
	c := map[string]any{"op": "DecodeContentTypes", "name": name, "in": goval.Encode(in0), "types": typesName}
	return merge(c, stable(func() map[string]any {
		return goval.CallRaw(func() (any, error) {
			in, _ := gen().(map[string]any)
			ns, err := media.DecodeContentTypes(in, types)
			if err != nil {
				return nil, err
			}
			return merge(map[string]any{"out": contentTypesDump(ns.Config), "hash": ns.SourceHash}, dumpJSON(ns)), nil
		})
	}))
}

func randContentTypesInput(r *rand.Rand) any {
	pool := []string{
		"text/html", "TEXT/HTML", "text/markdown", "text/asciidoc", "text/pandoc", "text/rst", "text/org",
		"text/x-custom", "text/x-md2", "_merge", "text/x-nope", "application/json", "image/svg+xml",
	}
	out := map[string]any{}
	for i := r.Intn(4); i > 0; i-- {
		k := pool[r.Intn(len(pool))]
		switch r.Intn(8) {
		case 0:
			out[k] = randScalar(r)
		case 1:
			out[k] = m("x", randScalar(r))
		default:
			out[k] = map[string]any{}
		}
	}
	return out
}

// ---------------------------------------------------------------------------
// Types and Type methods

var queryProbes = []string{
	"text/html", "TEXT/HTML", "html", "HTML", ".html", "xml", "rss", "json", "application/json", "json;charset",
	"text/plain", "plain", "md", "markdown", "text/markdown", "image/svg+xml", "svg", "svg+xml",
	"application/rss+xml", "application/xml", "xml+rss", "yaml", "yml", "toml", "", "/", "a/b/c", "text/", "/html",
	"x", "js", "javascript", "text/javascript", "ts", "jsx", "tsx", "scss", "sass", "css", "woff2", "font/woff2",
	"image/jpeg", "jpg", "JPEG", "jpeg", "webp", "gif", "png", "ico", "bmp", "tiff", "tif", "avif", "heic", "mp4",
	"mpeg", "mp3", "ogg", "ogv", "webm", "wav", "pdf", "wasm", "csv", "ics", "calendar", "txt", "webmanifest",
	"manifest+json", "otf", "ttf", "org", "adoc", "pandoc", "rst", "gotmpl", "htm", "xhtml", "asciidoc", "Rss",
	"cst", "CUS", "thing", "x-custom", "text/x-custom", "application/x-thing+json", "application/x-thing",
	"x-nosuffix", "svgz", "feed", "text/x-md2", "x-md2", "octet-stream", "application/octet-stream", "image/x-icon",
	"video/3gpp", "3gpp", "audio/x-wav", "html,htm", ",", "text/html+html", "application/json+json",
}

func typeMethods(types media.Types) []any {
	var out []any
	for _, t := range types {
		mj, err := t.MarshalJSON()
		var has []any
		for _, s := range []string{"html", "xml", "md", "", "json", "cst", "HTML"} {
			has = append(has, t.HasSuffix(s))
		}
		out = append(out, map[string]any{
			"String": str(t.String()), "Suffixes": strs(t.Suffixes()), "IsText": t.IsText(), "IsHTML": t.IsHTML(),
			"IsMarkdown": t.IsMarkdown(), "IsZero": t.IsZero(), "HasSuffix": has,
			"MarshalJSON": []any{str(string(mj)), errOut(err)},
		})
	}
	return out
}

func found(t media.Type, ok bool) any {
	return []any{cval.Dump(t), ok}
}

func typesMethodCases(tn string, types media.Types, add func(map[string]any)) {
	for _, q := range queryProbes {
		c := map[string]any{"op": "TypesQuery", "types": tn, "q": str(q)}
		c["GetBestMatch"] = found(types.GetBestMatch(q))
		c["GetByType"] = found(types.GetByType(q))
		var by []any
		for _, t := range types.BySuffix(q) {
			by = append(by, t.Type)
		}
		c["BySuffix"] = by
		t, si, ok := types.GetFirstBySuffix(q)
		c["GetFirstBySuffix"] = []any{cval.Dump(t), cval.Dump(si), ok}
		t, si, ok = types.GetBySuffix(q)
		c["GetBySuffix"] = []any{cval.Dump(t), cval.Dump(si), ok}
		c["IsTextSuffix"] = types.IsTextSuffix(q)
		c["GetBySubType"] = found(types.GetBySubType(q))
		main, sub, _ := strings.Cut(q, "/")
		c["GetByMainSubType"] = found(types.GetByMainSubType(main, sub))
		add(c)
	}
	add(map[string]any{"op": "TypeMethods", "types": tn, "out": typeMethods(types)})
}

// ---------------------------------------------------------------------------
// FromString, FromStringAndExt

func fromStringCases(add func(map[string]any)) {
	probes := []string{
		"text/html", "text/html; charset=utf-8", "Application/RSS+XML", "a/b+c+d", "noslash", "a/b/c", "", "/",
		"text/", "/x", "text/plain;+x", "text/x+", "TEXT/X-Y", "image/svg+xml;q=1", "a/;b", "a/b;c+d", "É/Ü",
		"application/vnd.api+json", " text/html ", "text/html\t",
	}
	exts := [][]string{nil, {"html"}, {".HTML", "htm"}, {"", ""}, {"..x"}, {"a", ".b", "c"}}
	for _, p := range probes {
		t, err := media.FromString(p)
		add(map[string]any{"op": "FromString", "s": str(p), "out": cval.Dump(t), "err": errOut(err)})
		for _, e := range exts {
			ev := strs(e)
			t, err := media.FromStringAndExt(p, append([]string(nil), e...)...)
			mj, jerr := t.MarshalJSON()
			add(map[string]any{
				"op": "FromStringAndExt", "s": str(p), "ext": ev, "out": cval.Dump(t), "err": errOut(err),
				"json": []any{str(string(mj)), errOut(jerr)},
			})
		}
	}
}

// ---------------------------------------------------------------------------
// http.DetectContentType and FromContent

func corpus() [][]byte {
	s := func(v string) []byte { return []byte(v) }
	return [][]byte{
		s(""), s(" "), s("\n\t "), s("plain text"), s("hello, world\n"),
		s("<!DOCTYPE html><html><body>x</body></html>"), s("<!doctype HTML>"), s("  <HTML>"), s("<html"), s("<htmlx>"),
		s("<head>"), s("<script>"), s("<iframe "), s("<h1>"), s("<div>"), s("<font>"), s("<table>"), s("<a>"),
		s("<style>"), s("<title>"), s("<b>"), s("<body>"), s("<br>"), s("<p>"), s("<!-->"), s("<!-- c -->"),
		s("<?xml version=\"1.0\"?><rss/>"), s("<?xml"), s("<?XML"), s("<svg xmlns=\"x\"></svg>"),
		s("<rss version=\"2.0\">"), s("<feed>"), s("%PDF-1.4"), s("%!PS-Adobe-"), s("\xfe\xffx\x00"), s("\xff\xfex\x00"),
		s("\xef\xbb\xbfhello"), s("\xef\xbb\xbf<html>"), s("GIF87a"), s("GIF89a......"), s("\x89PNG\x0d\x0a\x1a\x0a...."),
		s("\xff\xd8\xff\xe0"), s("BM1234"), s("RIFF\x00\x00\x00\x00WEBPVP"), s("RIFF\x00\x00\x00\x00AVI "),
		s("RIFF\x00\x00\x00\x00WAVE"), s("\x00\x00\x01\x00"), s("\x00\x00\x02\x00"), s("ID3\x03"), s("OggS\x00"),
		s("OggS\x00\x02"), s("MThd\x00\x00\x00\x06"), s("FORM\x00\x00\x00\x00AIFF"), s("\xff\xfb\x90"),
		s("\x1aE\xdf\xa3"), s("\x00\x00\x00\x18ftypmp42"), s("\x00\x00\x00\x1cftypavif"), s("\x00\x00\x00\x14ftypqt  "),
		s("\x00asm\x01\x00\x00\x00"), s("wOFF"), s("wOF2"), s("\x00\x01\x00\x00"), s("OTTO"), s("ttcf"),
		s("PK\x03\x04"), s("\x1f\x8b\x08"), s("Rar!\x1a\x07\x00"), s("Rar!\x1a\x07\x01\x00"), s("BZh"),
		s("\x00\x00\x01\xba"), s("\x00\x00\x01\xb3"), s(".snd"), s("\x7fELF"), s("{\"a\": 1}"), s("[1, 2]"),
		s("a: b\nc: d\n"), s("title = \"x\"\n"), s("body { color: red }"), s("console.log(1)"),
		s("#!/bin/sh\necho\n"), s("a,b,c\n1,2,3\n"), s("BEGIN:VCALENDAR\r\n"), s("\x00\x01\x02binary"),
		s("text\x00with nul"), s("\x1b[31mred"), s("\x80\x81\x82"), s("héllo"), s("\xff"), s("# Title\n\ntext"),
		s("\x00\x00\x00\x0cjP  \r\n\x87\n"), s("II*\x00"), s("MM\x00*"), s("8BPS"), s("\x0a\x0d\x0d\x0a"),
		s("<svg"), s("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\"/>"),
	}
}

var hintProbes = [][]string{
	nil, {"html"}, {"js"}, {".svg"}, {"xml"}, {"json"}, {"txt"}, {"csv"}, {"md"}, {"png"}, {"unknown", "json"},
	{"yaml"}, {"toml"}, {"css"}, {".rss"}, {"ics"}, {"webp"}, {"jpg", "png"}, {"cst"}, {"svgz"},
}

func fromContentCases(tn string, types media.Types, add func(map[string]any)) {
	for _, b := range corpus() {
		var outs []any
		for _, h := range hintProbes {
			outs = append(outs, cval.Dump(media.FromContent(types, h, b)))
		}
		add(map[string]any{"op": "FromContent", "types": tn, "content": str(string(b)), "out": outs})
	}
}

func detectCases(seed int64, add func(map[string]any)) {
	var all []string
	for _, b := range corpus() {
		all = append(all, string(b))
	}
	r := rand.New(rand.NewSource(seed))
	c := corpus()
	for i := 0; i < 3000; i++ {
		base := c[r.Intn(len(c))]
		n := r.Intn(len(base) + 1)
		b := append([]byte{}, base[:n]...)
		for j := r.Intn(8); j > 0; j-- {
			switch r.Intn(4) {
			case 0:
				b = append(b, byte(r.Intn(256)))
			case 1:
				b = append(b, []byte(" \t\r\n\x0c")[r.Intn(5)])
			default:
				b = append(b, byte('a'+r.Intn(26)))
			}
		}
		if r.Intn(4) == 0 {
			b = append([]byte(strings.Repeat(" ", r.Intn(4))), b...)
		}
		all = append(all, string(b))
	}
	// Longer than the 512 sniffing bytes.
	all = append(all, strings.Repeat("a", 600), strings.Repeat(" ", 520)+"<html>", strings.Repeat("\x00", 513))
	sort.Strings(all)
	var ins, outs []any
	prev := "\x00never"
	for _, s := range all {
		if s == prev {
			continue
		}
		prev = s
		ins = append(ins, str(s))
		outs = append(outs, http.DetectContentType([]byte(s)))
	}
	add(map[string]any{"op": "DetectContentType", "in": ins, "out": outs})
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
