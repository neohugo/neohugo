// Command resources is the Go oracle for resources/resource (Resources.Get,
// GetMatch, Match, ByType, Mount, MergeByLanguage, NewCachedResourceGetter,
// Param, GetParam) in crates/nh-resource (Wave B task T11). resources/internal
// cannot be imported from here (Go's internal package rule); its tests are
// ported by hand in the Rust test.
//
//	go run ./tools/go-oracle/nh-resource/resources [-out crates/nh-resource/tests/fixtures/resources]
//
// The resources are test resources (a name, a normalized name that may
// differ, a resource type, an optional translation key) over names with and
// without leading slashes, sub directories, mixed case, Thai and accented
// characters; the arguments are exact, relative ("./"), case-folded,
// normalized-only and missing names, non-string arguments (cast
// conversions and errors), and globs (every gobwas feature, bad patterns).
// Go's panics are recorded ({"panic": ...}).
//
// Output: resources.json.gz. Nothing here depends on the platform.
package main

import (
	"flag"
	"html/template"
	"log"
	"path/filepath"
	"time"

	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/resources/resource"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
)

// testResource is a resource.Resource with a name, a normalized name and a
// resource type (the other methods are never called).
type testResource struct {
	resource.Resource
	name, normalized, typ string
	params                maps.Params
}

func (r testResource) Name() string           { return r.name }
func (r testResource) NameNormalized() string { return r.normalized }
func (r testResource) ResourceType() string   { return r.typ }
func (r testResource) Params() maps.Params    { return r.params }

// translatedResource adds a translation key.
type translatedResource struct {
	testResource
	key string
}

func (r translatedResource) TranslationKey() string { return r.key }

// plainResource has no NameNormalized method.
type plainResource struct {
	resource.Resource
	name, typ string
}

func (r plainResource) Name() string         { return r.name }
func (r plainResource) ResourceType() string { return r.typ }

type res struct {
	Name       string `json:"name"`
	Normalized string `json:"normalized,omitempty"`
	Type       string `json:"type"`
	Key        string `json:"key,omitempty"`
	Plain      bool   `json:"plain,omitempty"`
}

func build(rs []res) resource.Resources {
	var out resource.Resources
	for _, r := range rs {
		switch {
		case r.Plain:
			out = append(out, plainResource{name: r.Name, typ: r.Type})
		case r.Key != "":
			out = append(out, translatedResource{testResource{name: r.Name, normalized: r.Normalized, typ: r.Type}, r.Key})
		default:
			out = append(out, testResource{name: r.Name, normalized: r.Normalized, typ: r.Type})
		}
	}
	return out
}

func nameOf(r resource.Resource) any {
	if r == nil {
		return nil
	}
	return r.Name()
}

func names(rs resource.Resources) any {
	if rs == nil {
		return nil
	}
	out := []any{}
	for _, r := range rs {
		out = append(out, r.Name())
	}
	return out
}

var sets = [][]res{
	{
		{Name: "/foo/theme.css", Normalized: "/foo/theme.css", Type: "text"},
	},
	{
		{Name: "/a/b/c/d.txt", Normalized: "/a/b/c/d.txt", Type: "text"},
		{Name: "/a/b/c/e/f.txt", Normalized: "/a/b/c/e/f.txt", Type: "text"},
		{Name: "/a/b/d.txt", Normalized: "/a/b/d.txt", Type: "text"},
		{Name: "/a/b/e.txt", Normalized: "/a/b/e.txt", Type: "text"},
	},
	{
		{Name: "a/b/c/d.txt", Normalized: "a/b/c/d.txt", Type: "text"},
		{Name: "a/b/c/e/f.txt", Normalized: "a/b/c/e/f.txt", Type: "text"},
		{Name: "a/b/d.txt", Normalized: "a/b/d.txt", Type: "text"},
		{Name: "a/b/e.txt", Normalized: "a/b/e.txt", Type: "text"},
		{Name: "n.txt", Normalized: "n.txt", Type: "text"},
	},
	{
		{Name: "Logo", Normalized: "images/logo.png", Type: "image", Key: "logo"},
		{Name: "images/Sunset.JPG", Normalized: "images/sunset.jpg", Type: "image", Key: "sunset"},
		{Name: "data.json", Normalized: "data.json", Type: "application"},
		{Name: "ภาพ/ขนม.png", Normalized: "ภาพ/ขนม.png", Type: "image", Key: "thai"},
		{Name: "docs/Café Menu.pdf", Normalized: "docs/café-menu.pdf", Type: "application", Key: "menu"},
		{Name: "index.md", Type: "page", Plain: true},
		{Name: "1", Normalized: "one", Type: "text"},
		{Name: "", Normalized: "empty", Type: "text"},
	},
	{
		{Name: "a.png", Normalized: "a.png", Type: "image", Key: "a"},
		{Name: "b.png", Normalized: "b.png", Type: "image", Key: "b"},
		{Name: "c.png", Normalized: "c.png", Type: "image"},
	},
}

var getArgs = []any{
	"/foo/theme.css", "foo/theme.css", "./foo/theme.css", "./theme.css", "/FOO/THEME.CSS",
	"a/b/d.txt", "/a/b/d.txt", "./a/b/d.txt", "A/B/D.TXT", "n.txt", "./n.txt", "/n.txt",
	"logo", "Logo", "/logo", "images/logo.png", "./images/logo.png", "IMAGES/SUNSET.jpg",
	"images/sunset.jpg", "data.json", "ภาพ/ขนม.png", "docs/café-menu.pdf", "docs/Café Menu.pdf",
	"index.md", "missing", "", "/", "./", "1", 1, int64(1), 1.5, true, nil, template.HTML("data.json"),
	[]string{"x"}, "one", "empty",
}

var patterns = []any{
	"*", "**", "*.txt", "**.txt", "a/b/*", "a/b/**", "/a/b/*.txt", "*.PNG", "images/*",
	"images/**.jpg", "**/*.png", "{a,b}.png", "[ab].png", "[!a].png", "?.png", "ภาพ/*",
	"docs/*.pdf", "logo", "Logo", "*logo*", "[", "a[", "{a,b", "", "/", "\\*", 42, nil,
}

var mounts = [][2]string{
	{"/foo", "."}, {"/a/b/c", "z"}, {"/a/b", ""}, {"/a/b", "."}, {"/a/b", "./"},
	{"/a/b/c", "/z"}, {"/a/b", "/z"}, {"", ""}, {"/a/b", "/a/b"}, {"a/b", "z"},
	{"images", "img"}, {"", "/x"}, {"x", ""},
}

var mountArgs = []any{
	"./theme.css", "z/d.txt", "z/e/f.txt", "d.txt", "./d.txt", "/z/d.txt", "/z/e/f.txt",
	"/z/f.txt", "/z/c/d.txt", "/z/c/e/f.txt", "/a/b/c/d.txt", "/a/b/f.txt", "img/logo",
	"img/Sunset.JPG", "/x/data.json", "n.txt", 7,
}

func main() {
	out := flag.String("out", "crates/nh-resource/tests/fixtures/resources", "output directory")
	flag.Parse()

	var cases []map[string]any
	add := func(c map[string]any) { cases = append(cases, c) }

	for si, set := range sets {
		r := build(set)
		for _, a := range getArgs {
			add(map[string]any{"fn": "Get", "set": si, "arg": goval.Encode(a), "want": goval.CallRaw(func() (any, error) { return nameOf(r.Get(a)), nil })})
		}
		for _, p := range patterns {
			add(map[string]any{"fn": "GetMatch", "set": si, "arg": goval.Encode(p), "want": goval.CallRaw(func() (any, error) { return nameOf(r.GetMatch(p)), nil })})
			add(map[string]any{"fn": "Match", "set": si, "arg": goval.Encode(p), "want": goval.CallRaw(func() (any, error) { return names(r.Match(p)), nil })})
		}
		for _, t := range []any{"image", "text", "page", "application", "", "IMAGE", 1, nil} {
			add(map[string]any{"fn": "ByType", "set": si, "arg": goval.Encode(t), "want": goval.CallRaw(func() (any, error) { return names(r.ByType(t)), nil })})
		}
		for _, m := range mounts {
			g := r.Mount(m[0], m[1])
			for _, a := range mountArgs {
				add(map[string]any{"fn": "Mount", "set": si, "base": m[0], "target": m[1], "arg": goval.Encode(a), "want": goval.CallRaw(func() (any, error) { return nameOf(g.Get(a)), nil })})
			}
		}
		for sj := range sets {
			r2 := build(sets[sj])
			add(map[string]any{"fn": "MergeByLanguage", "set": si, "set2": sj, "want": goval.CallRaw(func() (any, error) { return names(r.MergeByLanguage(r2)), nil })})
		}
	}
	// A nil Resources.
	add(map[string]any{"fn": "GetNil", "arg": goval.Encode(nil), "want": goval.CallRaw(func() (any, error) { return nameOf(resource.Resources(nil).Get(nil)), nil })})

	// NewCachedResourceGetter over two sets: first match wins.
	cg := resource.NewCachedResourceGetter(build(sets[3]), build(sets[2]))
	for _, a := range append(getArgs, mountArgs...) {
		add(map[string]any{"fn": "Cached", "arg": goval.Encode(a), "want": goval.CallRaw(func() (any, error) { return nameOf(cg.Get(a)), nil })})
	}

	// Param / GetParam.
	params := maps.Params{
		"s":      "Hello",
		"i":      42,
		"i64":    int64(7),
		"f":      1.5,
		"b":      true,
		"t":      time.Date(2020, 1, 2, 3, 4, 5, 0, time.UTC),
		"ss":     []string{"A", "b"},
		"sa":     []any{"A", "b"},
		"m":      map[string]any{"k": "V"},
		"p":      maps.Params{"nested": maps.Params{"deep": "D"}, "k": "v"},
		"nil":    nil,
		"html":   template.HTML("<b>"),
		"mixed":  "MiXeD",
		"u":      uint(3),
		"nilss":  []string(nil),
		"f32":    float32(2.5),
		"i8":     int8(-3),
		"dotted": "top",
	}
	fallback := maps.Params{"fb": "fallback", "s": "shadowed", "p": maps.Params{"other": "O"}}
	keys := []any{"s", "S", "i", "i64", "f", "b", "t", "ss", "sa", "m", "p", "p.nested", "p.nested.deep", "P.Nested.Deep", "p.k", "p.other", "nil", "html", "mixed", "u", "nilss", "f32", "i8", "missing", "fb", "p.x.y", "", 3, nil, "dotted.x"}
	tr := testResource{params: params}
	for _, k := range keys {
		add(map[string]any{"fn": "Param", "params": goval.Encode(params), "key": goval.Encode(k), "want": goval.CallRaw(func() (any, error) {
			v, err := resource.Param(tr, nil, k)
			return goval.Encode(v), err
		})})
		add(map[string]any{"fn": "ParamFallback", "params": goval.Encode(params), "fallback": goval.Encode(fallback), "key": goval.Encode(k), "want": goval.CallRaw(func() (any, error) {
			v, err := resource.Param(tr, fallback, k)
			return goval.Encode(v), err
		})})
		if ks, ok := k.(string); ok {
			add(map[string]any{"fn": "GetParam", "params": goval.Encode(params), "key": goval.Encode(k), "want": goval.CallRaw(func() (any, error) { return goval.Encode(resource.GetParam(tr, ks)), nil })})
			add(map[string]any{"fn": "GetParamToLower", "params": goval.Encode(params), "key": goval.Encode(k), "want": goval.CallRaw(func() (any, error) { return goval.Encode(resource.GetParamToLower(tr, ks)), nil })})
		}
	}

	if err := goval.WriteCasesGz(filepath.Join(*out, "resources.json.gz"), map[string]any{"sets": sets}, cases); err != nil {
		log.Fatal(err)
	}
	log.Printf("resources: %d cases", len(cases))
}
