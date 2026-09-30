// Command postcss is the postCSS oracle of Wave B task T16 (js-css-pipeline):
// neohugo's resource_transformers/cssjs client running a real postcss-cli
// (node) over a hermetic copy of the synthetic site
// rust/testdata/oracle/resource-transformers/t16site, whose
// node_modules/.bin/postcss is linked to -postcss. The site's
// postcss.config.js has no dependencies: an inline plugin that rewrites a
// declaration and appends what the child process sees (HUGO_ENVIRONMENT, the
// working directory, NODE_ENV — which Hugo filters out of the environment, so
// postcss-load-config defaults it to "development" —
// and the HUGO_FILE_* names). The oracle runs with the site copy as its
// working directory and NODE_ENV set, like a Hugo build run from the site.
//
// Covered: the default config (assets/_jsconfig), a config in the working
// dir, a missing config, noMap/no-map, the @import inliner (quotes,
// indentation, url()/media/tailwindcss exclusions, repeated and nested
// imports, a missing import with and without skipInlineImportsNotFound),
// postcss syntax errors mapped back to the imported file, and the chains
// toCSS | postCSS and toCSS | postCSS | minify | fingerprint (the final
// CSS). Built for linux/arm64 like tocss (LibSass is in the chains).
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
	get     = t16support.Get
	postcss = t16support.PostCSS
	tocss   = t16support.ToCSS
)

func c(name string, links bool, steps ...t16support.Step) t16support.Case {
	return t16support.Case{Name: name, Steps: steps, Links: links}
}

// Cases returns the postcss scripts.
func Cases() []t16support.Case {
	vars := m{
		"brand": "#3a7bd5", "primary": "#ff0000", "size": "24px", "font": "Helvetica Neue",
		"n": 3, "f": 1.5, "calc": "calc(10px + 2px)", "url": "url(a.png)",
		"quoted": css.QuotedString("Hello"), "unquoted": css.UnquotedString("12px"),
	}
	seeksnack := m{"enableSourceMap": false, "includePaths": []string{"node_modules", "assets/scss"}, "outputStyle": "compressed", "vars": vars}
	return []t16support.Case{
		c("plain-nil", false, get("css/plain.css"), postcss(nil)),
		c("plain-empty", false, get("css/plain.css"), postcss(m{})),
		c("plain-alt-config", false, get("css/plain.css"), postcss(m{"config": "postcss-alt.config.js"})),
		c("plain-nomap", true, get("css/plain.css"), postcss(m{"noMap": true})),
		c("plain-no-map", false, get("css/plain.css"), postcss(m{"no-map": "true"})),
		c("err-missing-config", false, get("css/plain.css"), postcss(m{"config": "missing.config.js"})),
		c("imports-off", false, get("css/imports.css"), postcss(nil)),
		c("imports-inline", false, get("css/imports.css"), postcss(m{"inlineImports": true})),
		c("err-imports-comment", false, get("css/imports-comment.css"), postcss(m{"inlineImports": true})),
		c("err-imports-missing", false, get("css/imports-missing.css"), postcss(m{"inlineImports": true})),
		c("imports-missing-skip", false, get("css/imports-missing.css"), postcss(m{"inlineImports": true, "skipInlineImportsNotFound": true})),
		c("err-broken", false, get("css/broken.css"), postcss(nil)),
		c("err-imports-broken", false, get("css/imports-broken.css"), postcss(m{"inlineImports": true})),
		c("scss-postcss", false, get("scss/simple.scss"), tocss(nil), postcss(nil)),
		c("final", true, get("scss/main.scss"), tocss(seeksnack), postcss(nil), t16support.Minify(), t16support.Fingerprint()),
	}
}

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "rust/testdata/oracle/resource-transformers/postcss", "fixture dir")
	postcssBin := flag.String("postcss", "", "node_modules/.bin/postcss of postcss-cli")
	flag.Parse()
	if *postcssBin == "" {
		log.Fatal("-postcss is required")
	}

	absRoot, err := filepath.Abs(*root)
	if err != nil {
		log.Fatal(err)
	}
	absOut, err := filepath.Abs(*out)
	if err != nil {
		log.Fatal(err)
	}
	bin, err := filepath.Abs(*postcssBin)
	if err != nil {
		log.Fatal(err)
	}
	src := "rust/testdata/oracle/resource-transformers/t16site"
	dir, err := t16support.CopySite(filepath.Join(absRoot, src), nil, map[string]string{"node_modules/.bin/postcss": bin})
	if err != nil {
		log.Fatal(err)
	}
	// A Hugo build run from the site dir (the child inherits the working
	// directory), with NODE_ENV set in the environment (Hugo filters it).
	if err := os.Chdir(dir); err != nil {
		log.Fatal(err)
	}
	if err := os.Setenv("NODE_ENV", "staging"); err != nil {
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
	log.Printf("postcss: %d cases (%s)", len(cases), runtime.GOARCH)
}
