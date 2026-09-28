// Command site is the Go oracle of nh-hugolib's template API (T23): for each
// site (the assemble oracle's 17 sites, read from its fixtures, and the
// menus/refs sites of sites.go) it creates the HugoSites like a build, runs
// process and assemble (the overlay-added NHOracleSite in package hugolib),
// and calls the template-visible methods of every site and page through
// reflect, as text/template would:
//
//   - the `page.Site` (`*page.siteWrapper`) and `*hugolib.Site` methods
//     (BaseURL, Params and its identity, Taxonomies, Menus, Lastmod,
//     SitemapAbsURL, GetPage, Param, the promoted PathSpec helpers, ...);
//   - every page method that does not render content, with arguments where
//     they take some (Eq, GetPage, GetTerms, Param, Ref/RelRef with every
//     argument form and error, IsMenuCurrent/HasMenuCurrent, ...), on the
//     page and on its wrappers (pageForShortcode, pageForRenderHooks with the
//     nop content methods, pageWithWeight0, pageWithOrdinal);
//   - the paginators of every node (first call wins, reset by a
//     rendering-site shift);
//   - probe templates without template functions (nil results: `.Parent` of
//     home in a Scratch, `.GetPage` misses, `with`/`if` on them, the
//     `mainsections` special case);
//
// with every result's Go type. It also writes the method sets of every type
// templates receive (methodsets.json.gz).
//
// Each site is written to a temporary directory; repository files are
// recorded by hash, so the Rust test recreates the site.
//
//	go run ./tools/go-oracle/nh-hugolib/site -root .
package main

import (
	_ "embed"
	"flag"
	"fmt"
	"log"
	"os"
	"path/filepath"

	"github.com/neohugo/neohugo/hugolib"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-hugolib/hsupport"
)

//go:embed overlay_site.go.txt
var overlaySite string

// Set by the overlay-added hook file (hookFile).
var (
	oracle     func(h *hugolib.HugoSites, norm func(string) string, enc func(any) any, probes [][2]string) (map[string]any, error)
	methodSets func(h *hugolib.HugoSites) []any
)

const hookFile = `package main

import "github.com/neohugo/neohugo/hugolib"

func init() {
	oracle = hugolib.NHOracleSite
	methodSets = hugolib.NHOracleMethodSets
}
`

// Probes are templates without template functions (the Rust test runs them
// with a names-only func map), executed with each probed page as data.
var Probes = [][2]string{
	{"parent-scratch", `{{ $s := .Scratch }}{{ $s.Set "current" .Parent }}{{ with $s.Get "current" }}P:{{ .Parent }}{{ else }}none{{ end }}`},
	{"parent-print", `[{{ .Parent }}]`},
	{"parent-with", `{{ with .Parent }}{{ .Title }}{{ else }}no parent{{ end }}|{{ if .Parent }}y{{ else }}n{{ end }}`},
	{"parent-var", `{{ $p := .Parent }}{{ $p.Title }}`},
	{"parent-chain", `{{ .Parent.Title }}`},
	{"getpage-miss", `{{ with .GetPage "nope" }}found{{ else }}nop{{ end }}|{{ .GetPage "nope" }}|{{ (.GetPage "nope").Title }}|{{ (.GetPage "nope").IsDescendant . }}`},
	{"site-getpage-miss", `{{ .Site.GetPage "nope" }}|{{ with .Site.GetPage "nope" }}y{{ else }}n{{ end }}`},
	{"mainsections", `{{ .Site.Params.mainSections }}|{{ .Site.Params.MainSections }}|{{ .Site.MainSections }}`},
	{"params", `{{ .Params.title }}|{{ .Site.Params.author }}|{{ .Param "title" }}|{{ .Param "nope" }}`},
	{"file", `{{ with .File }}{{ .Path }}{{ else }}nofile{{ end }}|{{ .File }}`},
	{"sitemap", `{{ .Sitemap.ChangeFreq }}|{{ .Sitemap.Priority }}|{{ .Sitemap.Filename }}`},
	{"translations", `{{ range .Translations }}{{ .Lang }},{{ end }}|{{ .IsTranslated }}|{{ range .AllTranslations }}{{ .Lang }},{{ end }}`},
	{"meta", `{{ .Kind }}|{{ .Type }}|{{ .Section }}|{{ .RelPermalink }}|{{ .Permalink }}|{{ .Lang }}|{{ .Language }}|{{ .Language.LanguageName }}`},
	{"positions", `{{ .Next }}|{{ .Prev }}|{{ .NextInSection }}|{{ .PrevInSection }}|{{ with .Next }}{{ .Title }}{{ end }}`},
	{"collections", `{{ range .Pages }}{{ .Title }},{{ end }}|{{ range .RegularPages }}{{ .Title }},{{ end }}|{{ len .Resources }}`},
	{"site", `{{ .Site.Title }}|{{ .Site.BaseURL }}|{{ .Site.LanguageCode }}|{{ range .Site.Languages }}{{ .Lang }}{{ end }}|{{ .Site.Home.Title }}|{{ range $k, $v := .Site.Taxonomies }}{{ $k }}={{ len $v }};{{ end }}`},
	{"data", `{{ range $k, $v := .Data }}{{ $k }};{{ end }}|{{ .Data.Singular }}|{{ .Data.Plural }}`},
	{"paginator", `{{ with .Paginator }}{{ .PageNumber }}/{{ .TotalPages }}:{{ range .Pages }}{{ .Title }},{{ end }}{{ end }}`},
	{"outputformats", `{{ range .OutputFormats }}{{ .Name }}={{ .RelPermalink }};{{ end }}|{{ range .AlternativeOutputFormats }}{{ .Rel }};{{ end }}|{{ with .OutputFormats.Get "json" }}{{ .Permalink }}{{ end }}`},
	{"scratch", `{{ .Scratch.Set "x" 1 }}{{ .Scratch.Add "x" 2 }}{{ .Store.Get "x" }}|{{ .Scratch.Get "nope" }}`},
	{"menus", `{{ range $k, $v := .Site.Menus }}{{ $k }}:{{ range $v }}{{ .Name }}({{ .URL }}{{ range .Children }} {{ .Name }}{{ end }}),{{ end }};{{ end }}|{{ range $k, $v := .Menus }}{{ $k }}={{ $v.Name }};{{ end }}`},
	{"wrongargs", `{{ .Title 1 }}`},
	{"wrongargs-ctx", `{{ .Content 1 }}`},
	{"eq-method", `{{ .Eq . }}|{{ .Eq .Parent }}`},
}

func main() {
	root := flag.String("root", ".", "the neohugo module root")
	out := flag.String("out", "crates/nh-hugolib/tests/fixtures/site", "output directory (relative to -root)")
	only := flag.String("site", "", "only this site")
	flag.Parse()

	if !hsupport.IsChild() {
		files := map[string]string{
			"hugolib/zz_nh_oracle_site.go":                  overlaySite,
			"tools/go-oracle/nh-hugolib/site/zz_nh_hook.go": hookFile,
		}
		args := []string{"-root", *root, "-out", *out, "-site", *only}
		if err := hsupport.RunOverlaid(*root, "./tools/go-oracle/nh-hugolib/site", files, nil, args); err != nil {
			log.Fatal(err)
		}
		return
	}
	if oracle == nil || methodSets == nil {
		log.Fatal("oracle hook not installed")
	}

	sites, err := AssembleFixtureSites(*root)
	if err != nil {
		log.Fatal(err)
	}
	sites = append(sites, SiteSites()...)

	tmp, err := os.MkdirTemp("", "nh-hugolib-site")
	if err != nil {
		log.Fatal(err)
	}
	defer func() { _ = os.RemoveAll(tmp) }()

	outDir := *out
	if !filepath.IsAbs(outDir) {
		outDir = filepath.Join(*root, outDir)
	}
	for _, s := range sites {
		if *only != "" && s.Name != *only {
			continue
		}
		b, err := hsupport.New(s, tmp)
		if err != nil {
			log.Fatal(err)
		}
		if s.Name == "synthetic" && *only == "" {
			if err := hsupport.WriteJSONGz(filepath.Join(outDir, "methodsets.json.gz"), methodSets(b.H)); err != nil {
				log.Fatal(err)
			}
		}
		c := map[string]any{"site": s.Describe(), "probes": Probes}
		dump, err := oracle(b.H, b.Norm, goval.Encode, Probes)
		if err != nil {
			c["err"] = b.Norm(err.Error())
		} else {
			c["dump"] = dump
		}
		c["log"] = b.LogLines()
		if err := hsupport.WriteJSONGz(filepath.Join(outDir, s.Name+".json.gz"), c); err != nil {
			log.Fatal(err)
		}
		fmt.Printf("%s: ok\n", s.Name)
	}
}
