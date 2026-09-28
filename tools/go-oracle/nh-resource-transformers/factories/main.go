// Command factories is the Go oracle of
// crates/nh-resource-transformers/tests/factories.rs (Wave B task T15): the
// tpl resources namespace's factories over the assets file system of the
// synthetic site (crates/nh-resource-transformers/tests/fixtures/site) and the
// repository's docs site: Get (every asset, path variants, case, bad
// arguments), GetMatch and Match (globs, case insensitivity), ByType,
// FromString and Copy (first writer wins per target path). Every returned
// resource is recorded (attributes, content, links, which publish), and so are
// the published files.
//
//	GOTOOLCHAIN=go1.27.1 go run ./tools/go-oracle/nh-resource-transformers/factories -root .
package main

import (
	"flag"
	"fmt"
	"log"
	"path/filepath"
	"strings"

	"github.com/neohugo/neohugo/tools/go-oracle/nh-resource-transformers/rtsupport"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-resources/rsupport"
)

type (
	step = rtsupport.Step
	cse  = rtsupport.Case
)

var (
	s = rtsupport.S
	v = rtsupport.V
)

var patterns = []string{
	"*", "**", "*.js", "**.js", "**/*.js", "js/*", "js/**", "js/*.js", "JS/*.JS", "js/*.JS",
	"**.{js,css}", "css/[ab].css", "css/[!a].css", "js/?.js", "**/sub/*", "/js/a.js",
	"js/a.js", "./js/a.js", "js/../js/a.js", "upper/*", "Upper/MixedCase.TXT", "a b/*",
	"**.txt", "**.TXT", "images/*", "**.png", "**.svg", "missing/*", "", "[", "js/{a,b", "**/",
	"images/**", "**/*", "*/*", "opengraph/*", "css/**.css", "js/alpinejs/**",
}

func cases(dir string) []cse {
	files := rtsupport.AssetFiles(dir)
	var cs []cse

	var get []step
	for i, f := range files {
		get = append(get, step{Op: "Get", Args: []any{s(f)}, As: fmt.Sprintf("f%d", i)})
	}
	cs = append(cs, cse{Name: "get-all", Steps: get})

	first := files[0]
	var variants []step
	for _, p := range []string{
		"", "/" + first, "./" + first, strings.ToUpper(first), strings.ToLower(first), "missing.js",
		"js//a.js", "js/../js/a.js", "/js/./a.js", "a b/space ü.txt", "Upper/MixedCase.TXT",
		"upper/mixedcase.txt", "js", "js/", "/", ".",
	} {
		variants = append(variants, step{Op: "Get", Args: []any{s(p)}})
	}
	variants = append(variants,
		step{Op: "Get", Args: []any{v(nil)}},
		step{Op: "Get", Args: []any{v(3)}},
		step{Op: "Get", Args: []any{v(map[string]any{"a": "b"})}},
		step{Op: "Get", Args: []any{v([]string{"js/a.js"})}},
	)
	cs = append(cs, cse{Name: "get-variants", Steps: variants})

	var gm, m []step
	for _, p := range patterns {
		gm = append(gm, step{Op: "GetMatch", Args: []any{s(p)}})
		m = append(m, step{Op: "Match", Args: []any{s(p)}})
	}
	for _, x := range []any{nil, 3, map[string]any{"a": "b"}} {
		gm = append(gm, step{Op: "GetMatch", Args: []any{v(x)}})
		m = append(m, step{Op: "Match", Args: []any{v(x)}})
	}
	cs = append(cs, cse{Name: "getmatch", Steps: gm}, cse{Name: "match", Steps: m})

	var bt []step
	for _, x := range []any{"image", "text", "application", "font", "video", "", "IMAGE", nil, 3} {
		bt = append(bt, step{Op: "ByType", Args: []any{v(x)}})
	}
	cs = append(cs, cse{Name: "bytype", Steps: bt})

	cs = append(cs, cse{Name: "fromstring", Steps: []step{
		{Op: "FromString", Args: []any{s("x/a.txt"), s("hello")}, As: "fs1"},
		{Op: "FromString", Args: []any{s("x/a.txt"), s("hello")}},
		{Op: "FromString", Args: []any{s("/x/a.txt"), s("hello")}},
		{Op: "FromString", Args: []any{s("x/a.txt"), s("other")}},
		{Op: "FromString", Args: []any{s("X/A.TXT"), s("hello")}},
		{Op: "FromString", Args: []any{s("x/b.js"), s("var b = 1;")}},
		{Op: "FromString", Args: []any{s("x/c.css"), s("")}},
		{Op: "FromString", Args: []any{s("x/d.json"), v(42)}},
		{Op: "FromString", Args: []any{s("x/e.svg"), s("<svg/>")}},
		{Op: "FromString", Args: []any{s("x/noext"), s("data")}},
		{Op: "FromString", Args: []any{s("x/../y/f.html"), s("<p>f</p>")}},
		{Op: "FromString", Args: []any{v(3), s("num")}},
		// (No empty target path: it publishes a file over the publish dir itself, and
		// afero's MemMapFs then keeps listing the dir's children; nh-hugofs does not.)
		{Op: "FromString", Args: []any{s("x/m.txt"), v(map[string]any{"a": 1})}},
		{Op: "FromString", Args: []any{v(map[string]any{"a": 1}), s("x")}},
	}})

	cs = append(cs, cse{Name: "copy", Steps: []step{
		{Op: "Copy", Args: []any{s("copies/first.txt"), rtsupport.Ref("f0")}, As: "c1"},
		{Op: "Copy", Args: []any{s("copies/first.txt"), rtsupport.Ref("f1")}},
		{Op: "Copy", Args: []any{s("/copies/first.txt"), rtsupport.Ref("f1")}},
		{Op: "Copy", Args: []any{s("copies/second.css"), rtsupport.Ref("f1")}},
		{Op: "Copy", Args: []any{s("copies/fs.txt"), rtsupport.Ref("fs1")}},
		{Op: "Copy", Args: []any{v(3), rtsupport.Ref("f0")}},
		{Op: "Copy", Args: []any{s("copies/str.txt"), s("notaresource")}},
		{Op: "Copy", Args: []any{s("copies/nested/c1.txt"), rtsupport.Ref("c1")}},
	}})
	return cs
}

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "crates/nh-resource-transformers/tests/fixtures/factories", "fixture dir")
	flag.Parse()

	absRoot, err := filepath.Abs(*root)
	if err != nil {
		log.Fatal(err)
	}
	for _, sd := range []struct{ name, dir string }{
		{"synth", "crates/nh-resource-transformers/tests/fixtures/site"},
		{"docs", "docs"},
	} {
		dir := filepath.Join(absRoot, sd.dir)
		res := rtsupport.RunSite(dir, nil, cases(dir))
		res["dir"] = sd.dir
		if err := rsupport.WriteGz(filepath.Join(*out, sd.name+".json.gz"), res); err != nil {
			log.Fatal(err)
		}
		log.Printf("factories: %s: %d cases", sd.name, len(res["cases"].([]any)))
	}
}
