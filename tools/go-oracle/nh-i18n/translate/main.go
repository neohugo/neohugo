// Command translate is the Go oracle for neohugo's langs/i18n
// (TranslationProvider.NewResource/CloneResource and the translate funcs of
// i18n.Translator) as crates/nh-i18n ports it (Wave B task T17).
//
//	go run ./tools/go-oracle/nh-i18n/translate [-out rust/testdata/oracle/i18n/translate]
//
// The seeksnack site is private and this repository has no i18n files of its
// own, so the sites are synthetic: en + th like seeksnack (messages with
// `{{ .Context }}`, plurals, HTML, custom delimiters, template errors),
// fr/pl/ar/ja/ru to reach every plural category, TOML/YAML/JSON files with
// nested namespaces and v1 id/translation arrays, theme overlays, the
// i18nTests and TestPlural tables of langs/i18n/i18n_test.go (one site per
// case), the i18n_integration_test.go sites, unknown language codes (art-x-),
// regional tags that the x/text matcher maps to another bundle tag, missing
// keys with enableMissingTranslationPlaceholders and printI18nWarnings on and
// off, an ignoreFiles pattern, a broken file and a site without i18n.
//
// Each site is written to a temporary directory and loaded with
// allconfig.LoadConfig; the first language's Deps is built like
// testconfig.GetTestDeps (hugofs.NewFrom(OS) + Init) with a logger that
// records every entry (INFO and up), NewResource runs on it and every other
// language gets CloneResource. Every call is Deps.Translate(ctx, id, arg); the
// output (or the recovered panic) and the log entries of the call are
// recorded. Arguments are described by specs that the Rust test rebuilds:
// Go ints of several kinds, floats (1.5), strings, template.HTML, bools, nil,
// map[string]any (with Count/count/Context keys), maps.Params, []any, structs
// with a Count field or method (value and pointer), a nil *struct and
// time.Month.
//
// Output: translate.json.gz. Nothing here depends on the platform.
package main

import (
	"context"
	"flag"
	"fmt"
	"html/template"
	"io"
	"log"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"

	"github.com/bep/logg"
	"github.com/neohugo/neohugo/common/loggers"
	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/config/allconfig"
	"github.com/neohugo/neohugo/deps"
	"github.com/neohugo/neohugo/hugofs"
	"github.com/neohugo/neohugo/langs/i18n"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/spf13/afero"
)

type countField struct {
	Count any
}

type noCountField struct {
	Counts int
}

type countMethod struct{}

func (c countMethod) Count() any {
	return 32.5
}

type wordCount struct {
	WordCount int
}

// spec describes a Go value; the Rust test rebuilds it (tests/common/mod.rs).
type spec = map[string]any

func sp(t string, v any) spec { return spec{"t": t, "v": v} }

func build(s spec) any {
	v := s["v"]
	switch s["t"] {
	case "nil":
		return nil
	case "int":
		return v.(int)
	case "int64":
		return int64(v.(int))
	case "int8":
		return int8(v.(int))
	case "uint":
		return uint(v.(int))
	case "float64":
		return v.(float64)
	case "float32":
		return float32(v.(float64))
	case "string":
		return v.(string)
	case "html":
		return template.HTML(v.(string))
	case "bool":
		return v.(bool)
	case "map":
		m := map[string]any{}
		for k, x := range v.(map[string]spec) {
			m[k] = build(x)
		}
		return m
	case "params":
		m := maps.Params{}
		for k, x := range v.(map[string]spec) {
			m[k] = build(x)
		}
		return m
	case "slice":
		var l []any
		for _, x := range v.([]spec) {
			l = append(l, build(x))
		}
		return l
	case "month":
		return time.Month(v.(int))
	case "nilptr":
		return (*countField)(nil)
	case "struct":
		ptr, _ := s["ptr"].(bool)
		switch s["type"] {
		case "main.countField":
			c := countField{Count: build(s["fields"].(map[string]spec)["Count"])}
			if ptr {
				return &c
			}
			return c
		case "main.noCountField":
			return noCountField{Counts: build(s["fields"].(map[string]spec)["Counts"]).(int)}
		case "main.countMethod":
			if ptr {
				return &countMethod{}
			}
			return countMethod{}
		case "main.wordCount":
			return wordCount{WordCount: build(s["fields"].(map[string]spec)["WordCount"]).(int)}
		}
	}
	panic(fmt.Sprintf("unknown spec %v", s))
}

func countFieldSpec(c spec, ptr bool) spec {
	return spec{"t": "struct", "type": "main.countField", "ptr": ptr, "fields": map[string]spec{"Count": c}}
}

func mapSpec(kv ...any) spec {
	m := map[string]spec{}
	for i := 0; i < len(kv); i += 2 {
		m[kv[i].(string)] = kv[i+1].(spec)
	}
	return sp("map", m)
}

// fullArgs is the argument set of the main sites.
func fullArgs() []spec {
	args := []spec{sp("nil", nil)}
	for _, i := range []int{0, 1, 2, 3, 5, 11, 21, 101, -1, 1000000} {
		args = append(args, sp("int", i))
	}
	for _, f := range []float64{1.5, 22.5, 100.0, 1.0, 0.5, 2.5} {
		args = append(args, sp("float64", f))
	}
	for _, s := range []string{"1", "1.5", "abc", "", "100.0", "0", "-1", "2.50", " 1", "1e3"} {
		args = append(args, sp("string", s))
	}
	args = append(args,
		sp("int64", 21), sp("int8", 3), sp("uint", 3), sp("float32", 1.5), sp("bool", true), sp("bool", false),
		sp("html", "5"), sp("html", "<b>x</b>"), sp("month", 3), spec{"t": "nilptr", "type": "*main.countField"},
		sp("slice", []spec{sp("int", 1)}),
		mapSpec("Count", sp("int", 1)), mapSpec("Count", sp("int", 21)), mapSpec("Count", sp("float64", 1.5)),
		mapSpec("Count", sp("string", "2")), mapSpec("count", sp("int", 5)), mapSpec("COUNT", sp("string", "1")),
		mapSpec("Counts", sp("int", 5)), mapSpec("Count", sp("nil", nil)), mapSpec("Count", sp("string", "")),
		mapSpec("Count", sp("bool", true)), mapSpec("Count", sp("slice", []spec{})),
		mapSpec("Context", sp("float64", 0.5)), mapSpec("Context", sp("int", 2)), mapSpec("Context", sp("string", "1/3 (Pack) (30 g)")),
		mapSpec("Context", sp("string", "<b>&amp;</b>")), mapSpec("Context", sp("nil", nil)), mapSpec(),
		mapSpec("Count", sp("int", 3), "Context", sp("string", "ctx")),
		sp("params", map[string]spec{"count": sp("int", 5)}),
		countFieldSpec(sp("int", 22), false), countFieldSpec(sp("float64", 1.5), false), countFieldSpec(sp("int", 1), true),
		countFieldSpec(sp("nil", nil), false),
		spec{"t": "struct", "type": "main.noCountField", "fields": map[string]spec{"Counts": sp("int", 23)}},
		spec{"t": "struct", "type": "main.countMethod", "methods": map[string]spec{"Count": sp("float64", 32.5)}},
		spec{"t": "struct", "type": "main.countMethod", "ptr": true, "methods": map[string]spec{"Count": sp("float64", 32.5)}},
		spec{"t": "struct", "type": "main.wordCount", "fields": map[string]spec{"WordCount": sp("int", 50)}},
	)
	return args
}

func pluralArgs() []spec {
	var args []spec
	for _, i := range []int{0, 1, 2, 3, 4, 5, 6, 7, 10, 11, 12, 14, 19, 20, 21, 22, 25, 100, 101, 102, 103, 111, 1000000, -1} {
		args = append(args, sp("int", i))
	}
	args = append(args, sp("float64", 1.5), sp("float64", 1.0), sp("float64", 0.5), sp("string", "1"), sp("string", "1.0"),
		sp("string", "0.0"), sp("string", "2.5"), sp("string", "-1"), mapSpec("Count", sp("int", 2)), sp("nil", nil))
	return args
}

type site struct {
	name  string
	toml  string
	files map[string]string
	calls []call
}

type call struct {
	lang, id string
	arg      spec
}

func cross(langs, ids []string, args []spec) []call {
	var cs []call
	for _, l := range langs {
		for _, id := range ids {
			for _, a := range args {
				cs = append(cs, call{l, id, a})
			}
		}
	}
	return cs
}

const enToml = `
[categories]
other = "Categories"
[servings]
other = "{{ .Context }} servings per container"
[readingTime]
one = "One minute to read"
other = "{{ .Count }} minutes to read"
[dot]
one = "one {{ . }}"
other = "other {{ . }}"
[allforms]
zero = "zero {{ . }}"
one = "one {{ . }}"
two = "two {{ . }}"
few = "few {{ . }}"
many = "many {{ . }}"
other = "other {{ . }}"
[html]
other = "<b>{{ .Count }}</b> & 'quotes' \"dq\""
[delims]
leftDelim = "<<"
rightDelim = ">>"
other = "delims << .Count >> {{ .Count }}"
[onlyone]
one = "only one"
[onlyen]
other = "English only {{ . }}"
[badexec]
other = "bad {{ .Count.Foo }}"
[badparse]
other = "bad {{ if }}"
[withif]
other = "{{ if . }}truthy{{ else }}falsy{{ end }} {{ printf \"%v|%T\" . . }}"
[emptyother]
one = "one"
other = ""
[nested.key]
other = "nested key"
flat = "flat value"
`

const thToml = `
[categories]
other = "หมวดหมู่"
[servings]
other = "จำนวนหน่วยบริโภคต่อ{{ .Context }}"
[readingTime]
other = "อ่าน {{ .Count }} นาที"
[dot]
other = "th {{ . }}"
[allforms]
zero = "zero {{ . }}"
one = "one {{ . }}"
two = "two {{ . }}"
few = "few {{ . }}"
many = "many {{ . }}"
other = "other {{ . }}"
[html]
other = "<i>{{ .Count }}</i>"
[delims]
leftDelim = "<<"
rightDelim = ">>"
other = "th << .Count >>"
[onlyone]
one = "only one th"
[badexec]
other = "bad {{ .Count.Foo }}"
[badparse]
other = "bad {{ if }}"
[withif]
other = "th {{ if . }}truthy{{ else }}falsy{{ end }}"
[emptyother]
one = "one"
other = ""
flat = "ค่า"
`

var mainIDs = []string{"categories", "servings", "readingTime", "dot", "allforms", "html", "delims", "onlyone", "onlyen",
	"badexec", "badparse", "withif", "emptyother", "nested.key", "flat", "missing", "Categories", ""}

const allForms = `
[allforms]
zero = "zero {{ . }}"
one = "one {{ . }}"
two = "two {{ . }}"
few = "few {{ . }}"
many = "many {{ . }}"
other = "other {{ . }}"
`

func sites() []site {
	var ss []site
	enth := `
baseURL = "https://example.org/"
defaultContentLanguage = "en"
[languages.en]
weight = 1
[languages.th]
weight = 2
`
	files := map[string]string{"i18n/en.toml": enToml, "i18n/th.toml": thToml}
	ss = append(ss,
		site{"enth", enth, files, cross([]string{"en", "th"}, mainIDs, fullArgs())},
		site{"enth-placeholders", enth + "enableMissingTranslationPlaceholders = true\nprintI18nWarnings = true\n", files,
			cross([]string{"en", "th"}, mainIDs, []spec{sp("nil", nil), sp("int", 1), sp("int", 2), sp("string", ""), sp("float64", 1.5),
				mapSpec("Count", sp("nil", nil)), mapSpec("Context", sp("float64", 0.5))})},
		site{"thdefault", `
defaultContentLanguage = "th"
[languages.th]
weight = 1
[languages.en]
weight = 2
`, files, cross([]string{"en", "th"}, mainIDs, []spec{sp("nil", nil), sp("int", 1), sp("int", 3), mapSpec("Context", sp("int", 2))})},
	)

	// Every plural category.
	pl := `
defaultContentLanguage = "en"
[languages.en]
weight = 1
[languages.th]
[languages.fr]
[languages.pl]
[languages.ar]
[languages.ja]
[languages.ru]
[languages.cy]
[languages.ga]
[languages.pt]
`
	pfiles := map[string]string{}
	for _, l := range []string{"en", "th", "fr", "pl", "ar", "ja", "ru", "cy", "ga", "pt"} {
		pfiles["i18n/"+l+".toml"] = allForms
	}
	ss = append(ss, site{"plurals", pl, pfiles,
		cross([]string{"en", "th", "fr", "pl", "ar", "ja", "ru", "cy", "ga", "pt"}, []string{"allforms"}, pluralArgs())})

	// Formats: TOML, YAML, JSON, nested namespaces, v1 arrays.
	ss = append(ss, site{"formats", `
defaultContentLanguage = "en"
[languages.en]
[languages.fr]
[languages.de]
[languages.nl]
`, map[string]string{
		"i18n/en.toml": "[a]\nother = \"en a\"\n[ns.b]\nother = \"en ns.b {{ .Count }}\"\none = \"en ns.b one\"\n",
		"i18n/fr.yaml": "a: fr a\nns:\n  b:\n    one: \"fr one {{ . }}\"\n    other: \"fr other {{ . }}\"\n  c: fr c\n",
		"i18n/de.json": `[{"id": "a", "translation": "de a"}, {"id": "ns.b", "translation": {"one": "de one", "other": "de other {{ .Count }}"}}]`,
		"i18n/nl.yml":  "- id: a\n  translation: nl a\n- id: ns.c\n  translation: nl c\n",
	}, cross([]string{"en", "fr", "de", "nl"}, []string{"a", "ns.b", "ns.c", "b", "c"},
		[]spec{sp("nil", nil), sp("int", 1), sp("int", 2), sp("float64", 1.5), mapSpec("Count", sp("int", 1))})})

	// Themes (TestI18nFromTheme).
	ss = append(ss, site{"theme", `
[module]
[[module.imports]]
path = "mytheme"
`, map[string]string{
		"i18n/en.toml":                "[l1]\nother = 'l1main'\n[l2]\nother = 'l2main'\n",
		"themes/mytheme/i18n/en.toml": "[l1]\nother = 'l1theme'\n[l2]\nother = 'l2theme'\n[l3]\nother = 'l3theme'\n",
		"themes/mytheme/i18n/fr.toml": "[l1]\nother = 'l1fr'\n",
	}, cross([]string{"en"}, []string{"l1", "l2", "l3", "l4"}, []spec{sp("nil", nil)})})

	// Issue 9216 (TestI18nDefaultContentLanguage).
	ss = append(ss, site{"esdefault", `
disableKinds = ['RSS','sitemap','taxonomy','term','page','section']
defaultContentLanguage = 'es'
defaultContentLanguageInSubdir = true
[languages.es]
[languages.fr]
`, map[string]string{
		"i18n/es.toml": "cat = 'gato'\n",
		"i18n/fr.toml": "# this file intentionally empty\n",
	}, cross([]string{"es", "fr"}, []string{"cat", "dog"}, []spec{sp("nil", nil)})})

	// Unknown language codes and art-x- tags.
	ss = append(ss, site{"art", `
defaultContentLanguage = "klingon"
[languages.klingon]
weight = 1
[languages.a1]
[languages.a2]
[languages.en]
[languages.oc]
[languages.x1]
`, map[string]string{
		"i18n/en.toml":      "[readingTime]\none =\"one minute read\"\nother = \"{{.Count}} minutes read\"",
		"i18n/klingon.toml": "[readingTime]\none =  \"eitt minutt med lesing\"\nother = \"{{ .Count }} minuttar lesing\"",
		"i18n/a1.toml":      "[readingTime]\none =  \"a1 one\"\nother = \"a1 count {{ .Count }}\"",
		"i18n/a2.toml":      "[readingTime]\none =  \"a2 one\"\nother = \"a2 count {{ .Count }}\"\n[onlya2]\nother = \"a2 only\"",
		"i18n/oc.toml":      "[oc]\none =  \"abc\"\n[readingTime]\none = \"oc one\"",
	}, cross([]string{"klingon", "a1", "a2", "en", "oc", "x1"}, []string{"readingTime", "oc", "onlya2", "missing"},
		[]spec{sp("nil", nil), sp("int", 1), sp("int", 3), sp("string", "1")})})

	// Regional variants: the x/text matcher resolves en to en-US (golang/go#49176) and pt to pt-BR.
	ss = append(ss, site{"regions", `
defaultContentLanguage = "en"
[languages.en]
weight = 1
[languages.en-us]
[languages.en-gb]
[languages.pt]
[languages.pt-br]
[languages.zh-cn]
[languages.zh-tw]
[languages.zh]
[languages.sr]
`, map[string]string{
		"i18n/en-us.toml":   "[m]\nother = \"en-US m\"\n[onlyus]\nother = \"only us\"\n",
		"i18n/en.toml":      "[m]\nother = \"en m\"\n[onlyen]\nother = \"only en\"\n",
		"i18n/en-gb.toml":   "[m]\nother = \"en-GB m\"\n",
		"i18n/pt-br.toml":   "[m]\nother = \"pt-BR m\"\n",
		"i18n/pt.toml":      "[m]\nother = \"pt m\"\n",
		"i18n/zh-cn.json":   `{"m": "zh-CN m"}`,
		"i18n/zh-TW.yaml":   "m: zh-TW m\n",
		"i18n/zh.toml":      "m = \"zh m\"\n",
		"i18n/sr-latn.toml": "m = \"sr-Latn m\"\n",
		"i18n/sr.toml":      "m = \"sr m\"\n",
	}, cross([]string{"en", "en-us", "en-gb", "pt", "pt-br", "zh-cn", "zh-tw", "zh", "sr"}, []string{"m", "onlyus", "onlyen", "missing"},
		[]spec{sp("nil", nil)})})

	// ignoreFiles, and hidden/backup files the SourceSpec ignores.
	ss = append(ss, site{"ignore", `
ignoreFiles = ['\.bak$', 'ignored']
`, map[string]string{
		"i18n/en.toml":      "a = \"en a\"\n",
		"i18n/en.toml.bak":  "a = \"bak\"\n",
		"i18n/ignored.toml": "a = \"ignored\"\n",
		"i18n/.hidden.toml": "a = \"hidden\"\n",
		"i18n/sub/en.toml":  "b = \"sub b\"\n",
		"i18n/en.yaml":      "c: yaml c\n",
	}, cross([]string{"en"}, []string{"a", "b", "c"}, []spec{sp("nil", nil)})})

	// No i18n at all.
	ss = append(ss, site{"noi18n", "[languages.en]\n[languages.de]\n", nil,
		cross([]string{"en", "de"}, []string{"a"}, []spec{sp("nil", nil), sp("int", 1)})})

	// Broken files: NewResource fails.
	for i, f := range []map[string]string{
		{"i18n/en.toml": "[a\nother = 1"},
		{"i18n/en.toml": "[a]\nother = \"x\"\none = 5\n"},
		{"i18n/en.yaml": "a: [\n"},
		{"i18n/en.json": "{"},
		{"i18n/en.txt": "a = \"b\"\n"},
		{"i18n/zh.Hans.toml": "a = \"b\"\n"},
		{"i18n/en.toml": "a = 1\n"},
	} {
		ss = append(ss, site{fmt.Sprintf("broken%d", i), "", f, cross([]string{"en"}, []string{"a"}, []spec{sp("nil", nil)})})
	}

	// langs/i18n/i18n_test.go i18nTests, with placeholders off and on.
	for _, t := range i18nTests {
		for _, ph := range []bool{false, true} {
			cfg := fmt.Sprintf("enableMissingTranslationPlaceholders = %t\n[languages.en]\nweight = 1\n", ph)
			if t.lang != "en" {
				cfg += fmt.Sprintf("[languages.%s]\nweight = 2\n", t.lang)
			}
			files := map[string]string{}
			for k, v := range t.data {
				files["i18n/"+k] = v
			}
			ss = append(ss, site{fmt.Sprintf("gotest-%s-%t", t.name, ph), cfg, files, []call{{t.lang, t.id, t.args}}})
		}
	}

	// langs/i18n/i18n_test.go TestPlural.
	for _, t := range pluralTests {
		cfg := "enableMissingTranslationPlaceholders = true\n[languages.en]\nweight = 1\n"
		if t.lang != "en" {
			cfg += fmt.Sprintf("[languages.%s]\nweight = 2\n", t.lang)
		}
		var cs []call
		for _, a := range t.args {
			cs = append(cs, call{t.lang, t.id, a})
		}
		ss = append(ss, site{"gotestplural-" + t.name, cfg, map[string]string{"i18n/" + t.lang + ".toml": t.templ}, cs})
	}
	return ss
}

type i18nTest struct {
	name     string
	data     map[string]string
	args     spec
	lang, id string
}

var i18nTests = []i18nTest{
	{"all-present", map[string]string{"en.toml": "[hello]\nother = \"Hello, World!\"", "es.toml": "[hello]\nother = \"¡Hola, Mundo!\""}, sp("nil", nil), "es", "hello"},
	{"present-in-default", map[string]string{"en.toml": "[hello]\nother = \"Hello, World!\"", "es.toml": "[goodbye]\nother = \"¡Adiós, Mundo!\""}, sp("nil", nil), "es", "hello"},
	{"present-in-current", map[string]string{"en.toml": "[goodbye]\nother = \"Goodbye, World!\"", "es.toml": "[hello]\nother = \"¡Hola, Mundo!\""}, sp("nil", nil), "es", "hello"},
	{"missing", map[string]string{"en.toml": "[goodbye]\nother = \"Goodbye, World!\"", "es.toml": "[goodbye]\nother = \"¡Adiós, Mundo!\""}, sp("nil", nil), "es", "hello"},
	{"file-missing", map[string]string{"en.toml": ""}, sp("nil", nil), "es", "hello"},
	{"context-provided", map[string]string{"en.toml": "[wordCount]\nother = \"Hello, {{.WordCount}} people!\"", "es.toml": "[wordCount]\nother = \"¡Hola, {{.WordCount}} gente!\""},
		spec{"t": "struct", "type": "main.wordCount", "fields": map[string]spec{"WordCount": sp("int", 50)}}, "es", "wordCount"},
	{"readingTime-one", map[string]string{"en.toml": "[readingTime]\none = \"One minute to read\"\nother = \"{{ .Count }} minutes to read\"\n"}, sp("int", 1), "en", "readingTime"},
	{"readingTime-many-dot", map[string]string{"en.toml": "[readingTime]\none = \"One minute to read\"\nother = \"{{ . }} minutes to read\"\n"}, sp("int", 21), "en", "readingTime"},
	{"readingTime-many", map[string]string{"en.toml": "[readingTime]\none = \"One minute to read\"\nother = \"{{ .Count }} minutes to read\"\n"}, sp("int", 21), "en", "readingTime"},
	{"readingTime-map-one", map[string]string{"en.toml": "[readingTime]\none = \"One minute to read\"\nother = \"{{ .Count }} minutes to read\"\n"}, mapSpec("Count", sp("int", 1)), "en", "readingTime"},
	{"readingTime-string-one", map[string]string{"en.toml": "[readingTime]\none = \"One minute to read\"\nother = \"{{ . }} minutes to read\"\n"}, sp("string", "1"), "en", "readingTime"},
	{"readingTime-map-many", map[string]string{"en.toml": "[readingTime]\none = \"One minute to read\"\nother = \"{{ .Count }} minutes to read\"\n"}, mapSpec("Count", sp("int", 21)), "en", "readingTime"},
	{"argument-float", map[string]string{"en.toml": "[float]\nother = \"Number is {{ . }}\"\n"}, sp("float64", 22.5), "en", "float"},
	{"same-id-and-translation", map[string]string{"es.toml": "[hello]\nother = \"hello\"", "en.toml": "[hello]\nother = \"hi\""}, sp("nil", nil), "es", "hello"},
	{"same-id-and-translation-default", map[string]string{"es.toml": "[bye]\nother = \"bye\"", "en.toml": "[hello]\nother = \"hello\""}, sp("nil", nil), "es", "hello"},
	{"unknown-language-code", map[string]string{"en.toml": "[readingTime]\none =\"one minute read\"\nother = \"{{.Count}} minutes read\"",
		"klingon.toml": "[readingTime]\none =  \"eitt minutt med lesing\"\nother = \"{{ .Count }} minuttar lesing\""}, sp("int", 3), "klingon", "readingTime"},
	{"unknown-language-codes", map[string]string{"en.toml": "[readingTime]\none =\"en one\"\nother = \"en count {{.Count}}\"",
		"a1.toml": "[readingTime]\none =  \"a1 one\"\nother = \"a1 count {{ .Count }}\"",
		"a2.toml": "[readingTime]\none =  \"a2 one\"\nother = \"a2 count {{ .Count }}\""}, sp("int", 3), "a2", "readingTime"},
	{"known-language-missing-plural", map[string]string{"oc.toml": "[oc]\none =  \"abc\""}, sp("int", 1), "oc", "oc"},
	{"dotted-bare-key", map[string]string{"en.toml": "\"shop_nextPage.one\" = \"Show Me The Money\"\n"}, sp("nil", nil), "en", "shop_nextPage.one"},
	{"lang-with-hyphen", map[string]string{"pt-br.toml": "foo.one =  \"abc\""}, sp("int", 1), "pt-br", "foo"},
}

type pluralTest struct {
	name, lang, id, templ string
	args                  []spec
}

var pluralTests = []pluralTest{
	{"English", "en", "hour", "\n[hour]\none = \"{{ . }} hour\"\nother = \"{{ . }} hours\"",
		[]spec{sp("int", 1), sp("string", "1"), sp("float64", 1.5), sp("string", "1.5"), sp("int", 2), sp("string", "2")}},
	{"Other only", "en", "hour", "\n[hour]\nother = \"{{ with . }}{{ . }}{{ end }} hours\"",
		[]spec{sp("int", 1), sp("string", "1"), sp("int", 2), sp("nil", nil)}},
	{"Polish", "pl", "day", "\n[day]\none = \"{{ . }} miesiąc\"\nfew = \"{{ . }} miesiące\"\nmany = \"{{ . }} miesięcy\"\nother = \"{{ . }} miesiąca\"\n",
		[]spec{sp("int", 1), sp("int", 2), sp("int", 100), sp("string", "100.0"), sp("float64", 100.0)}},
}

type entry struct {
	level, msg string
}

func runSite(tmp string, s site) map[string]any {
	res := map[string]any{"name": s.name, "toml": s.toml, "files": s.files}
	// Error texts and log messages name the site directory; the Rust test
	// replaces its own directory the same way.
	siteDir := filepath.Join(tmp, s.name)
	st, err := loadSite(tmp, s)
	if err != nil {
		res["loaderr"] = err.Error()
		return res
	}
	var entries []entry
	logger := loggers.New(loggers.Options{
		Level:  logg.LevelInfo,
		StdOut: io.Discard,
		StdErr: io.Discard,
		HandlerPost: func(e *logg.Entry) error {
			entries = append(entries, entry{e.Level.String(), strings.ReplaceAll(e.Message, siteDir, "$SITE")})
			return nil
		},
	})
	take := func() []any {
		var out []any
		for _, e := range entries {
			out = append(out, []string{e.level, e.msg})
		}
		entries = nil
		return out
	}
	conf0 := st.GetFirstLanguageConfig()
	d := &deps.Deps{Conf: conf0, Fs: hugofs.NewFrom(hugofs.Os, conf0.BaseConfig()), Log: logger}
	if err := d.Init(); err != nil {
		res["initerr"] = err.Error()
		return res
	}
	take()
	tp := i18n.NewTranslationProvider()
	if err := tp.NewResource(d); err != nil {
		res["err"] = strings.ReplaceAll(err.Error(), siteDir, "$SITE")
		res["log"] = take()
		return res
	}
	res["log"] = take()
	byLang := map[string]*deps.Deps{}
	var langs []string
	for _, c := range s.calls {
		if _, ok := byLang[c.lang]; !ok {
			langs = append(langs, c.lang)
			if c.lang == conf0.Language().Lang {
				byLang[c.lang] = d
				continue
			}
			conf := st.GetByLang(c.lang)
			if conf == nil {
				log.Fatalf("%s: no language %q", s.name, c.lang)
			}
			dl := &deps.Deps{Conf: conf}
			if err := tp.CloneResource(dl, d); err != nil {
				log.Fatal(err)
			}
			byLang[c.lang] = dl
		}
	}
	res["clonelog"] = take()
	var calls []any
	for _, c := range s.calls {
		out := translate(byLang[c.lang], c.id, build(c.arg))
		calls = append(calls, map[string]any{"lang": c.lang, "id": c.id, "arg": c.arg, "argtype": fmt.Sprintf("%T", build(c.arg)), "out": out, "log": take()})
	}
	res["langs"] = langs
	res["calls"] = calls
	return res
}

func translate(d *deps.Deps, id string, arg any) (out any) {
	defer func() {
		if r := recover(); r != nil {
			out = map[string]any{"panic": fmt.Sprint(r)}
		}
	}()
	return d.Translate(context.Background(), id, arg)
}

func loadSite(tmp string, s site) (*allconfig.Configs, error) {
	dir := filepath.Join(tmp, s.name)
	if err := os.MkdirAll(dir, 0o777); err != nil {
		return nil, err
	}
	if err := os.WriteFile(filepath.Join(dir, "hugo.toml"), []byte(s.toml), 0o666); err != nil {
		return nil, err
	}
	var keys []string
	for k := range s.files {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	for _, k := range keys {
		p := filepath.Join(dir, k)
		if err := os.MkdirAll(filepath.Dir(p), 0o777); err != nil {
			return nil, err
		}
		if err := os.WriteFile(p, []byte(s.files[k]), 0o666); err != nil {
			return nil, err
		}
	}
	flags := config.New()
	flags.Set("workingDir", dir)
	return allconfig.LoadConfig(allconfig.ConfigSourceDescriptor{
		Fs:       afero.NewOsFs(),
		Filename: filepath.Join(dir, "hugo.toml"),
		Flags:    flags,
		Environ:  []string{"NEOHUGO_ORACLE=1"},
		Logger:   loggers.NewDefault(),
	})
}

func main() {
	out := flag.String("out", "rust/testdata/oracle/i18n/translate", "output directory")
	flag.Parse()

	tmp, err := os.MkdirTemp("", "nh-i18n-translate")
	if err != nil {
		log.Fatal(err)
	}
	defer func() { _ = os.RemoveAll(tmp) }()

	var cases []map[string]any
	n := 0
	for _, s := range sites() {
		r := runSite(tmp, s)
		if c, ok := r["calls"].([]any); ok {
			n += len(c)
		}
		cases = append(cases, r)
	}
	if err := goval.WriteCasesGz(filepath.Join(*out, "translate.json.gz"), map[string]any{}, cases); err != nil {
		log.Fatal(err)
	}
	fmt.Println("wrote", len(cases), "sites,", n, "calls")
}
