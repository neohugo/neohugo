// Command tocss is the toCSS (LibSass) oracle of Wave B task T16
// (js-css-pipeline): neohugo's resource_transformers/tocss/scss client with
// golibsass (cgo) over a hermetic copy of the synthetic site
// rust/testdata/oracle/resource-transformers/t16site. The Rust port runs
// the same scripts through libsass-sys (crates/nh-resource-transformers/tests/tocss.rs).
//
// LibSass formats numbers with C++ floating point, which the arm64 compiler
// contracts into FMA (the golden build is darwin/arm64), so the fixtures come
// from a linux/arm64 build run under qemu-aarch64-static, with zig as the C
// and C++ cross-compiler (see regen.sh next to this file).
//
// Covered: includePaths (the seeksnack order [assets/scss, node_modules,
// assets/scss], none, a missing dir), partials and underscore/index/.sass
// resolution through Hugo's importer and through LibSass's include paths
// (a package in node_modules whose own imports go back through the
// node_modules -> assets/vendor mount), the entry file's `@import "x.css"`
// protection, hugo:vars of every value kind (strings, units, colors, CSS
// functions, numbers, css.Quoted/Unquoted, bools, `$`-prefixed names),
// outputStyle nested/expanded/compact/compressed (any case, unknown ->
// nested), precision 0 (-> 8)/3/10, enableSourceMap (published map),
// sourceMapIncludeSources (not a LibSass option), targetPath, .sass entries,
// errors in the entry file (stdin -> real filename) and in partials, missing
// imports, syntax errors, and the final chain toCSS | minify | fingerprint.
package main

import (
	"flag"
	"log"
	"os"
	"path/filepath"
	"runtime"

	"github.com/neohugo/neohugo/common/types/css"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-resource-transformers/t16support"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-resources/rsupport"
)

type m = map[string]any

var (
	get   = t16support.Get
	tocss = t16support.ToCSS
)

func c(name string, links bool, steps ...t16support.Step) t16support.Case {
	return t16support.Case{Name: name, Steps: steps, Links: links}
}

// Cases returns the tocss scripts.
func Cases() []t16support.Case {
	seeksnackPaths := []string{"assets/scss", "node_modules", "assets/scss"}
	vars := m{
		"brand": "#3a7bd5", "primary": "#ff0000", "size": "24px", "font": "Helvetica Neue",
		"n": 3, "f": 1.5, "calc": "calc(10px + 2px)", "url": "url(a.png)",
		"quoted": css.QuotedString("Hello \"World\""), "unquoted": css.UnquotedString("12px"),
	}
	main := func(extra m) m {
		o := m{"includePaths": seeksnackPaths, "vars": vars}
		for k, v := range extra {
			o[k] = v
		}
		return o
	}
	return []t16support.Case{
		c("simple-nil", false, get("scss/simple.scss"), tocss(nil)),
		c("simple-nested", false, get("scss/simple.scss"), tocss(m{"outputStyle": "nested"})),
		c("simple-expanded", false, get("scss/simple.scss"), tocss(m{"outputStyle": "expanded"})),
		c("simple-compact", false, get("scss/simple.scss"), tocss(m{"outputStyle": "compact"})),
		c("simple-compressed", false, get("scss/simple.scss"), tocss(m{"outputStyle": "COMPRESSED"})),
		c("simple-unknown-style", false, get("scss/simple.scss"), tocss(m{"outputStyle": "fancy"})),
		c("main-nested", false, get("scss/main.scss"), tocss(main(nil))),
		c("main-expanded", false, get("scss/main.scss"), tocss(main(m{"outputStyle": "expanded"}))),
		c("main-compact", false, get("scss/main.scss"), tocss(main(m{"outputStyle": "compact"}))),
		c("main-compressed", false, get("scss/main.scss"), tocss(main(m{"outputStyle": "compressed"}))),
		c("main-precision-3", false, get("scss/main.scss"), tocss(main(m{"precision": 3}))),
		c("main-precision-10", false, get("scss/main.scss"), tocss(main(m{"precision": "10", "outputStyle": "compressed"}))),
		c("main-node-modules-only", false, get("scss/main.scss"), tocss(m{"includePaths": []string{"node_modules"}, "vars": vars})),
		c("main-missing-include", false, get("scss/main.scss"), tocss(m{"includePaths": []string{"nope", "node_modules"}, "vars": vars, "outputStyle": "compact"})),
		c("main-sourcemap", true, get("scss/main.scss"), tocss(main(m{"enableSourceMap": true}))),
		c("main-sourcemap-target", true, get("scss/main.scss"), tocss(main(m{"enableSourceMap": true, "targetPath": "/css/styles.css", "outputStyle": "compressed", "sourceMapIncludeSources": true}))),
		c("main-target", true, get("scss/main.scss"), tocss(main(m{"targetPath": "css/out.css"}))),
		c("main-final", true, get("scss/main.scss"), tocss(main(m{"outputStyle": "compressed"})), t16support.Minify(), t16support.Fingerprint()),
		c("vars", false, get("scss/vars.scss"), tocss(m{"vars": vars})),
		c("vars-expanded", false, get("scss/vars.scss"), tocss(m{"vars": vars, "outputStyle": "expanded"})),
		c("indented-sass", false, get("scss/indented.sass"), tocss(m{"outputStyle": "expanded"})),
		c("plain-css", false, get("css/plain.css"), tocss(nil)),
		// Errors.
		c("err-no-vars", false, get("scss/main.scss"), tocss(m{"includePaths": seeksnackPaths})),
		c("err-no-include", false, get("scss/main.scss"), tocss(m{"vars": vars})),
		c("err-undefined", false, get("scss/errors/undefined.scss"), tocss(nil)),
		c("err-in-partial", false, get("scss/errors/in-partial.scss"), tocss(nil)),
		c("err-missing", false, get("scss/errors/missing.scss"), tocss(nil)),
		c("err-syntax", false, get("scss/errors/syntax.scss"), tocss(nil)),
		c("err-vars-bool", false, get("scss/vars-bool.scss"), tocss(m{"vars": m{"$bool": true}})),
		c("vars-bool-typed", false, get("scss/vars-bool.scss"), tocss(m{"vars": m{"bool": css.UnquotedString("true")}})),
		c("err-decode", false, get("scss/simple.scss"), tocss(m{"precision": "x"})),
	}
}

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "rust/testdata/oracle/resource-transformers/tocss", "fixture dir")
	flag.Parse()

	absRoot, err := filepath.Abs(*root)
	if err != nil {
		log.Fatal(err)
	}
	absOut, err := filepath.Abs(*out)
	if err != nil {
		log.Fatal(err)
	}
	src := "rust/testdata/oracle/resource-transformers/t16site"
	dir, err := t16support.CopySite(filepath.Join(absRoot, src), nil, nil)
	if err != nil {
		log.Fatal(err)
	}
	// LibSass makes source map paths relative to the process working
	// directory; the golden build runs from the site dir.
	if err := os.Chdir(dir); err != nil {
		log.Fatal(err)
	}
	s, err := t16support.LoadSite(dir)
	if err != nil {
		log.Fatal(err)
	}
	cases := Cases()
	res := map[string]any{
		"dir":     src,
		"arch":    runtime.GOARCH,
		"cases":   cases,
		"results": s.Run(cases),
	}
	s.Close()
	_ = os.RemoveAll(filepath.Dir(dir))
	if err := rsupport.WriteGz(filepath.Join(absOut, "synth.json.gz"), res); err != nil {
		log.Fatal(err)
	}
	log.Printf("tocss: %d cases (%s)", len(cases), runtime.GOARCH)
}
