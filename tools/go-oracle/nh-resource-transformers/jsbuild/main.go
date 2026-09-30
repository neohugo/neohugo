// Command jsbuild is the js.Build oracle of Wave B task T16 (js-css-pipeline):
// neohugo's real resource_transformers/js client (esbuild v0.25.6 linked
// in, as in a Hugo build) over a hermetic copy of the synthetic site
// rust/testdata/oracle/resource-transformers/t16site and of the
// repository's docs site assets. The Rust port runs the same scripts through
// the pinned esbuild binary over --service and must produce the same bytes
// (crates/nh-resource-transformers/tests/jsbuild.rs).
//
// Covered: TS/JS/JSX/TSX entries with cross-file imports resolved by Hugo's
// resolver (relative and /assets-relative), node_modules package resolution
// (main/module/exports, CommonJS, a package whose own relative imports go
// through the node_modules -> assets/vendor mount), JSON/text/dataurl
// loaders, @params and @params/config, defines, targets es2015/es2020/esnext,
// formats iife/esm/cjs, minify, source maps inline/external/linked with and
// without sourcesContent, externals, shims, inject, JSX/JSXImportSource,
// targetPath, avoidTDZ, drop, platforms, CSS imports, option errors, esbuild
// errors, and the seeksnack chains (level-1 es2015 builds, a Concat with a
// vendor file, the level-2 targetPath+minify build and fingerprint).
package main

import (
	"flag"
	"log"
	"os"
	"path/filepath"

	"github.com/neohugo/neohugo/tools/go-oracle/nh-resource-transformers/t16support"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-resources/rsupport"
)

type m = map[string]any

var (
	get    = t16support.Get
	js     = t16support.JS
	concat = t16support.Concat
)

func c(name string, links bool, steps ...t16support.Step) t16support.Case {
	return t16support.Case{Name: name, Steps: steps, Links: links}
}

func synthCases() []t16support.Case {
	return []t16support.Case{
		c("main-nil", false, get("js/main.js"), js(nil)),
		c("main-empty", false, get("js/main.js"), js(m{})),
		c("main-es2015", false, get("js/main.js"), js(m{"target": "es2015"})),
		c("main-es6", false, get("js/main.js"), js(m{"target": "ES6"})),
		c("main-es2020-esm", false, get("js/main.js"), js(m{"target": "es2020", "format": "esm"})),
		c("main-cjs-minify", false, get("js/main.js"), js(m{"format": "CJS", "minify": true})),
		c("main-minify-str", false, get("js/main.js"), js(m{"minify": "true", "TARGET": "esnext"})),
		c("main-params", false, get("js/main.js"), js(m{"params": m{"api": "https://api.example.org", "n": 3, "nested": m{"a": []any{1, "two", 3.5, true, nil}}}})),
		c("params-config", false, get("js/params.js"), js(m{"params": m{"x": "<&>"}})),
		c("params-nil", false, get("js/params.js"), js(m{"minify": true})),
		c("main-inline-map", false, get("js/main.js"), js(m{"sourceMap": "inline"})),
		c("main-inline-map-nosources", false, get("js/main.js"), js(m{"sourceMap": "inline", "sourcesContent": false, "minify": true})),
		c("main-external-map", true, get("js/main.js"), js(m{"sourceMap": "external", "targetPath": "out/main.bundle.js"})),
		c("main-linked-map", true, get("js/main.js"), js(m{"sourceMap": "linked", "targetPath": "/out/linked.js"})),
		c("main-linked-map-min", true, get("js/main.js"), js(m{"sourceMap": "linked", "sourcesContent": false, "minify": true, "targetPath": "out/linked-min.js"})),
		c("main-none-map", false, get("js/main.js"), js(m{"sourceMap": "none"})),
		c("app-ts", false, get("ts/app.ts"), js(m{"target": "es2015"})),
		c("app-ts-esnext", true, get("ts/app.ts"), js(nil)),
		c("app-ts-es5", false, get("ts/app.ts"), js(m{"target": "es5"})),
		c("comp-tsx-automatic", false, get("ts/comp.tsx"), js(m{"JSX": "automatic", "JSXImportSource": "fakejsx"})),
		c("comp-tsx-inject", false, get("ts/comp.tsx"), js(m{"jsxFactory": "h", "jsxFragment": "Frag", "inject": []string{"js/inject/h.js"}})),
		c("comp-tsx-preserve", false, get("ts/comp.tsx"), js(m{"jsx": "preserve", "format": "esm"})),
		c("comp-jsx", false, get("js/comp.jsx"), js(m{"minify": true})),
		c("define", false, get("js/define.js"), js(m{"defines": m{"process.env.NODE_ENV": `"production"`, "__DEV__": false, "VERSION_STR": 42}, "minify": true})),
		c("define-none", false, get("js/define.js"), js(nil)),
		c("externals", false, get("js/ext.js"), js(m{"externals": []string{"extpkg", "extpkg/sub"}, "format": "esm"})),
		c("externals-missing", false, get("js/ext.js"), js(m{"externals": []string{"extpkg"}, "format": "esm"})),
		c("shims", false, get("js/shim.js"), js(m{"shims": m{"react": "js/shims/react.js"}})),
		c("pkg", false, get("js/pkg.js"), js(nil)),
		c("pkg-node", false, get("js/pkg.js"), js(m{"platform": "node", "format": "cjs"})),
		c("pkg-neutral", false, get("js/pkg.js"), js(m{"platform": "neutral", "format": "esm"})),
		c("json", false, get("js/json.js"), js(nil)),
		c("loaders", false, get("js/loaders.js"), js(m{"loaders": m{".svg": "dataurl", ".txt": "base64"}})),
		c("loaders-text", false, get("js/loaders.js"), js(m{"loaders": m{".svg": "text", ".txt": "text"}, "minify": true})),
		c("drop-console", false, get("js/drop.js"), js(m{"drop": "console"})),
		c("drop-debugger", false, get("js/drop.js"), js(m{"drop": "debugger"})),
		c("tdz", false, get("js/tdz.js"), js(m{"avoidTDZ": true, "target": "es2015"})),
		c("modern-es2015", false, get("js/modern.js"), js(m{"target": "es2015"})),
		c("modern-es2020", false, get("js/modern.js"), js(m{"target": "es2020"})),
		c("modern-esnext-min", false, get("js/modern.js"), js(m{"minify": true})),
		c("err-tla-iife", false, get("js/tla.js"), js(nil)),
		c("tla-esm", false, get("js/tla.js"), js(m{"format": "esm", "target": "es2022"})),
		c("style-css", false, get("js/style.js"), js(nil)),
		// Errors.
		c("err-target", false, get("js/main.js"), js(m{"target": "es3"})),
		c("err-format", false, get("js/main.js"), js(m{"format": "umd"})),
		c("err-jsx", false, get("js/main.js"), js(m{"jsx": "react"})),
		c("err-platform", false, get("js/main.js"), js(m{"platform": "deno"})),
		c("err-sourcemap", false, get("js/main.js"), js(m{"sourceMap": "both"})),
		c("err-loader", false, get("js/loaders.js"), js(m{"loaders": m{".svg": "wat"}})),
		c("err-drop", false, get("js/drop.js"), js(m{"drop": "everything"})),
		c("err-syntax", false, get("js/err.js"), js(nil)),
		c("err-missing-import", false, get("js/missing.js"), js(nil)),
		c("err-inject-missing", false, get("ts/comp.tsx"), js(m{"inject": []string{"js/inject/nope.js"}})),
		c("err-inject-abs", false, get("ts/comp.tsx"), js(m{"inject": []string{"/abs/h.js"}})),
		c("err-mediatype", false, get("bad.json"), js(nil)),
		c("err-decode", false, get("js/main.js"), js(m{"minify": []string{"x"}})),
		// Seeksnack-shaped chains.
		c("themeswitch", false, get("ts/themeswitch.ts"), js(m{"target": "es2015"})),
		c("search", false, get("ts/search.ts"), js(m{"target": "es2015"})),
		c("jquery", false, get("js/jquery-lite.min.js")),
		c("website", true, concat("js/website.js", "jquery", "search", "themeswitch"), js(m{"targetPath": "js/website.js", "minify": true}), t16support.Fingerprint()),
		c("set-theme", true, get("ts/themeset.ts"), js(m{"targetPath": "js/set-theme.js", "minify": true}), t16support.Fingerprint()),
		c("vendor-direct", false, get("vendor/innerpkg/src/index.js"), js(nil)),
	}
}

func docsCases() []t16support.Case {
	externals := []string{"alpinejs", "@alpinejs/persist", "@alpinejs/focus"}
	return []t16support.Case{
		c("head-early", false, get("js/head-early.js"), js(nil)),
		c("head-early-min", false, get("js/head-early.js"), js(m{"minify": true, "targetPath": "js/head.js"})),
		c("body-start", false, get("js/body-start.js"), js(m{"minify": true})),
		c("main-externals", false, get("js/main.js"), js(m{"externals": externals, "target": "es2020", "format": "esm"})),
		c("main-unresolved", false, get("js/main.js"), js(nil)),
		c("turbo", false, get("js/turbo.js"), js(m{"externals": []string{"@hotwired/turbo"}, "format": "esm", "sourceMap": "linked"})),
	}
}

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "rust/testdata/oracle/resource-transformers/jsbuild", "fixture dir")
	flag.Parse()

	absRoot, err := filepath.Abs(*root)
	if err != nil {
		log.Fatal(err)
	}
	for _, sd := range []struct {
		name, dir string
		only      []string
		cases     []t16support.Case
	}{
		{"synth", "rust/testdata/oracle/resource-transformers/t16site", nil, synthCases()},
		{"docs", "docs", []string{"hugo.toml", "assets", "package.json", "package.hugo.json", "hugo_stats.json"}, docsCases()},
	} {
		dir, err := t16support.CopySite(filepath.Join(absRoot, sd.dir), sd.only, nil)
		if err != nil {
			log.Fatal(err)
		}
		if err := os.MkdirAll(filepath.Join(dir, "content", "en"), 0o755); err != nil {
			log.Fatal(err)
		}
		s, err := t16support.LoadSite(dir)
		if err != nil {
			log.Fatal(err)
		}
		res := map[string]any{
			"dir":     sd.dir,
			"only":    sd.only,
			"cases":   sd.cases,
			"results": s.Run(sd.cases),
		}
		s.Close()
		_ = os.RemoveAll(filepath.Dir(dir))
		if err := rsupport.WriteGz(filepath.Join(*out, sd.name+".json.gz"), res); err != nil {
			log.Fatal(err)
		}
		log.Printf("jsbuild: %s: %d cases", sd.name, len(sd.cases))
	}
}
