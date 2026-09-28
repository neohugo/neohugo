// Command probe is the Go oracle for template execution through the store
// of crates/nh-tplimpl (Wave B task T13): the probe templates of the
// template-engine spec (Appendices A and B: printing, truthiness, and/or,
// len, range, with/else with, break/continue, eq/ne/not, every html/template
// escaping context, safe types, template calls, trim markers, try, printf of
// nil) executed through tplimpl.TemplateStore.ExecuteWithContext with a
// minimal function map (safeHTML, safeHTMLAttr, safeCSS, safeJS, safeURL,
// printf, jsonify, eq, ne, not, dict, slice, try) and a stub page.
//
//	GOTOOLCHAIN=go1.27.1 go run ./tools/go-oracle/nh-tplimpl/probe [-root .] [-out crates/nh-tplimpl/tests/fixtures/probe]
//
// The oracle runs itself again with `go run -overlay` (tsupport.RunOverlaid)
// to create a store with the minimal function map (hugolib.OracleNewStore).
// It writes probe.json.gz: the probe site (layouts, modules, configuration,
// in the store oracle's format) and the output of the home page's html and
// json templates. The Rust test builds the same store with the same function
// map and a stub page with the same methods and values.
package main

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"html/template"
	"log"
	"math"
	"reflect"
	"time"

	"github.com/neohugo/neohugo/common/hreflect"
	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/common/types"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-tplimpl/tsupport"
	"github.com/neohugo/neohugo/tpl/tplimpl"
	"github.com/spf13/cast"
)

// probePage is the stub page.
type probePage struct{}

func (probePage) Title() string       { return `Home "Q" & 'A' <b>` }
func (probePage) Description() string { return "desc + plus / slash" }
func (probePage) Kind() string        { return "home" }
func (probePage) IsHome() bool        { return true }
func (probePage) Date() time.Time {
	return time.Date(2020, 9, 6, 15, 46, 26, 955000000, time.UTC)
}
func (probePage) Params() maps.Params { return probeParams }
func (probePage) Site() probeSite     { return probeSite{} }

// probeSite is the stub site.
type probeSite struct{}

func (probeSite) Title() string       { return "Site" }
func (probeSite) Params() maps.Params { return siteParams }

var probeParams = maps.Params{
	"yint":       5,
	"yfloat":     4.5,
	"yfloat0":    5.0,
	"ybig":       12345678901,
	"ystr":       "007",
	"ybool":      true,
	"yfalse":     false,
	"ynull":      nil,
	"yempty":     "",
	"yzero":      0,
	"ylist":      []any{"a", "b", "c"},
	"ylistmixed": []any{1, "x", 2.5},
	"yemptylist": []any{},
	"ymap":       maps.Params{"b": 2, "a": 1, "c": []any{1, 2}},
	"yemptymap":  maps.Params{},
	"ydate":      time.Date(2021, 1, 2, 0, 0, 0, 0, time.UTC),
	"yneg":       -3,
	"yexp":       1.5e10,
	"mixed_case": "mc",
	"yhtml":      "<em>h</em>",
}

var siteParams = maps.Params{
	"intv":   3,
	"floatv": 1.0,
	"arr":    []any{"x", "y"},
	"nested": maps.Params{"key": "v"},
}

// eq is Hugo's compare.Eq for the probe's kinds (tpl/compare/compare.go):
// numbers normalized to int64/float64, strings by value, then DeepEqual.
func eq(first any, others ...any) bool {
	normalize := func(v any) any {
		if types.IsNil(v) {
			return nil
		}
		vv := reflect.ValueOf(v)
		switch vv.Kind() {
		case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
			return vv.Int()
		case reflect.Float32, reflect.Float64:
			return vv.Float()
		case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64:
			i := vv.Uint()
			if i <= math.MaxInt64 {
				return int64(i)
			}
			return i
		case reflect.String:
			return vv.String()
		default:
			return v
		}
	}
	normFirst := normalize(first)
	for _, other := range others {
		if reflect.DeepEqual(normFirst, normalize(other)) {
			return true
		}
	}
	return false
}

// funcs is the minimal function map of the probe (the Rust test defines
// the same functions).
var funcs = map[string]any{
	"safeHTML":     func(s any) template.HTML { return template.HTML(cast.ToString(s)) },
	"safeHTMLAttr": func(s any) template.HTMLAttr { return template.HTMLAttr(cast.ToString(s)) },
	"safeCSS":      func(s any) template.CSS { return template.CSS(cast.ToString(s)) },
	"safeJS":       func(s any) template.JS { return template.JS(cast.ToString(s)) },
	"safeURL":      func(s any) template.URL { return template.URL(cast.ToString(s)) },
	"printf":       fmt.Sprintf,
	"jsonify": func(v any) (template.HTML, error) {
		b, err := json.Marshal(v)
		if err != nil {
			return "", err
		}
		return template.HTML(b), nil
	},
	"eq":  eq,
	"ne":  func(a, b any) bool { return !eq(a, b) },
	"not": func(v any) bool { return !hreflect.IsTruthful(v) },
	"dict": func(values ...any) (map[string]any, error) {
		if len(values)%2 != 0 {
			return nil, errors.New("invalid dictionary call")
		}
		m := map[string]any{}
		for i := 0; i < len(values); i += 2 {
			k, ok := values[i].(string)
			if !ok {
				return nil, errors.New("dictionary keys must be strings")
			}
			m[k] = values[i+1]
		}
		return m, nil
	},
	"slice": func(args ...any) any { return args },
	// Hugo's try (tpl/safe): the identity; the engine wraps errors by name.
	"try": func(v any) (any, error) { return v, nil },
}

// goBuiltins are text/template's builtin functions.
var goBuiltins = map[string]bool{
	"and": true, "call": true, "html": true, "index": true, "slice": true, "js": true, "len": true,
	"not": true, "or": true, "print": true, "printf": true, "println": true, "urlquery": true,
	"eq": true, "ge": true, "gt": true, "le": true, "lt": true, "ne": true,
}

func main() {
	root := flag.String("root", ".", "neohugo module root")
	out := flag.String("out", "crates/nh-tplimpl/tests/fixtures/probe", "output directory")
	flag.Parse()

	if !tsupport.IsChild() {
		if err := tsupport.RunOverlaid(*root, "./tools/go-oracle/nh-tplimpl/probe", []string{"-root", *root, "-out", *out}); err != nil {
			log.Fatal(err)
		}
		return
	}
	if tsupport.Oracle == nil {
		log.Fatal("the overlay is not installed")
	}

	site := tsupport.ProbeSite()
	b, err := tsupport.New(site)
	if err != nil {
		log.Fatal(err)
	}
	// The embedded templates are parsed too: every other Hugo function name
	// gets a stub (the probe templates call only the minimal functions).
	full, err := tsupport.Oracle.StoreDump(b.H.Sites[0].TemplateStore)
	if err != nil {
		log.Fatal(err)
	}
	all := map[string]any{}
	for _, n := range full["funcNames"].([]string) {
		if goBuiltins[n] {
			// Hugo overrides these; the probe uses text/template's own.
			continue
		}
		all[n] = func(...any) (any, error) { return nil, errors.New("stub function called") }
	}
	for k, v := range funcs {
		all[k] = v
	}
	store, err := tsupport.Oracle.NewStore(b.H, all)
	if err != nil {
		log.Fatal(err)
	}
	outputs := map[string]any{}
	for _, f := range []struct{ name, mediaType string }{{"html", "text/html"}, {"json", "application/json"}, {"plain", "text/plain"}} {
		ti := store.LookupPagesLayout(tplimpl.TemplateQuery{
			Category: tplimpl.CategoryLayout,
			Desc: tplimpl.TemplateDescriptor{
				Kind:         "home",
				OutputFormat: f.name,
				MediaType:    f.mediaType,
				IsPlainText:  f.name != "html",
			},
		})
		if ti == nil {
			log.Fatalf("no %s template", f.name)
		}
		var buf bytes.Buffer
		res := map[string]any{"template": tsupport.Oracle.TemplID(store, ti)}
		if err := store.ExecuteWithContext(context.Background(), ti, &buf, probePage{}); err != nil {
			res["err"] = err.Error()
		}
		res["output"] = buf.String()
		outputs[f.name] = res
		log.Printf("probe: %s: %d bytes", f.name, buf.Len())
	}

	dump, err := tsupport.Oracle.StoreDump(store)
	if err != nil {
		log.Fatal(err)
	}
	files := map[string]string{}
	for k, v := range site.Files {
		files[k] = v
	}
	if err := tsupport.WriteFixture(tsupport.OutDir(*root, *out), "probe.json.gz", map[string]any{
		"site":    site.Name,
		"siteDir": tsupport.SiteDir(site.Name),
		"files":   files,
		"modules": b.Modules(),
		"config":  tsupport.Oracle.SiteConfig(b.H),
		"store":   map[string]any{"funcNames": dump["funcNames"]},
		"outputs": outputs,
	}); err != nil {
		log.Fatal(err)
	}
}
