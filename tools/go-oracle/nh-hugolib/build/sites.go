package main

import (
	"compress/gzip"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"github.com/neohugo/neohugo/tools/go-oracle/nh-hugolib/hsupport"
)

// The sites of the build oracle: the 17 sites of the assemble oracle (read
// from its checked-in fixtures, so every hugolib oracle builds the same
// trees; the ones without page layouts get the layouts of buildLayer), and
// the build sites below: aliases (absolute, relative, ugly sections, output
// formats with a path, on the home page and sections, colliding with a
// page, invalid on Windows), a custom alias.html, disableAliases and
// pagination.disableAliases, multihost, the main-language redirect with and
// without defaultContentLanguageInSubdir, term collisions, paginators
// (page/N), 404, sitemaps, robots.txt, RSS, JSON, build stats with the
// tags/classes/ids toggles, resources.PostProcess and js.Build (jsconfig).

func fm(lines ...string) string {
	return "---\n" + strings.Join(lines, "\n") + "\n---\n"
}

func inline(m map[string]string) map[string]hsupport.File {
	out := map[string]hsupport.File{}
	for k, v := range m {
		out[k] = hsupport.File{Content: v}
	}
	return out
}

// assembleFixtureSites reads the site descriptions of the assemble oracle's
// fixtures (crates/nh-hugolib/tests/fixtures/assemble/*.json.gz).
func assembleFixtureSites(root string) ([]hsupport.Site, error) {
	dir := filepath.Join(root, "crates", "nh-hugolib", "tests", "fixtures", "assemble")
	entries, err := os.ReadDir(dir)
	if err != nil {
		return nil, err
	}
	var names []string
	for _, e := range entries {
		if strings.HasSuffix(e.Name(), ".json.gz") {
			names = append(names, e.Name())
		}
	}
	sort.Strings(names)
	var sites []hsupport.Site
	for _, n := range names {
		f, err := os.Open(filepath.Join(dir, n))
		if err != nil {
			return nil, err
		}
		zr, err := gzip.NewReader(f)
		if err != nil {
			_ = f.Close()
			return nil, err
		}
		var fx struct {
			Site struct {
				Name  string `json:"name"`
				TOML  string `json:"toml"`
				Files []struct {
					Path    string `json:"path"`
					Content string `json:"content"`
					Repo    string `json:"repo"`
					FNV     string `json:"fnv"`
				} `json:"files"`
			} `json:"site"`
		}
		err = json.NewDecoder(zr).Decode(&fx)
		_ = f.Close()
		if err != nil {
			return nil, fmt.Errorf("%s: %w", n, err)
		}
		files := map[string]hsupport.File{}
		for _, ff := range fx.Site.Files {
			if ff.Repo != "" {
				b, err := os.ReadFile(filepath.Join(root, filepath.FromSlash(ff.Repo)))
				if err != nil {
					return nil, err
				}
				if hsupport.FNV(string(b)) != ff.FNV {
					return nil, fmt.Errorf("%s: %s changed since the assemble fixture was recorded", n, ff.Repo)
				}
				files[ff.Path] = hsupport.File{Content: string(b), Repo: ff.Repo}
			} else {
				files[ff.Path] = hsupport.File{Content: ff.Content}
			}
		}
		sites = append(sites, hsupport.Site{Name: fx.Site.Name, TOML: fx.Site.TOML, Files: files})
	}
	return sites, nil
}

const pagerBlock = `{{ with .Paginator }}<nav class="pager" id="pager-{{ .PageNumber }}"><span class="pn">{{ .PageNumber }}/{{ .TotalPages }}</span>{{ range .Pages }}<a class="item" href="{{ .RelPermalink }}">{{ .Title }}</a>{{ end }}</nav>{{ end }}`

// buildLayer holds the page layouts added to the assemble sites that have
// none: nodes and the 404 page call .Paginator (like seeksnack's head.html).
var buildLayer = map[string]string{
	"layouts/home.html":   `<!doctype html><html><head><title>{{ .Title }}</title></head><body class="home"><h1 id="title">{{ .Title }}</h1>` + pagerBlock + `</body></html>`,
	"layouts/list.html":   `<!doctype html><html><head><title>{{ .Title }}</title></head><body class="list {{ .Kind }}"><h1 id="title">{{ .Title }}</h1>` + pagerBlock + `</body></html>`,
	"layouts/single.html": `<!doctype html><html><head><title>{{ .Title }}</title></head><body class="single"><article id="main">{{ .Title }}</article></body></html>`,
	"layouts/404.html":    `<!doctype html><html><head><title>404</title></head><body class="nf">` + pagerBlock + `</body></html>`,
}

// hasPageLayouts reports whether the site has page layouts of its own.
func hasPageLayouts(s hsupport.Site) bool {
	for k := range s.Files {
		switch k {
		case "layouts/home.html", "layouts/list.html", "layouts/single.html", "layouts/page.html", "layouts/section.html",
			"layouts/_default/list.html", "layouts/_default/single.html", "layouts/index.html":
			return true
		}
	}
	return false
}

// BuildSites returns every site of the oracle.
func BuildSites(root string) ([]hsupport.Site, error) {
	sites, err := assembleFixtureSites(root)
	if err != nil {
		return nil, err
	}
	for i, s := range sites {
		if hasPageLayouts(s) {
			continue
		}
		files := map[string]hsupport.File{}
		for k, v := range s.Files {
			files[k] = v
		}
		for k, v := range buildLayer {
			files[k] = hsupport.File{Content: v}
		}
		sites[i].Files = files
	}
	return append(sites, buildSites()...), nil
}

// layouts returns the build layer plus the given layouts.
func layouts(extra map[string]string) map[string]string {
	m := map[string]string{}
	for k, v := range buildLayer {
		m[k] = v
	}
	for k, v := range extra {
		m[k] = v
	}
	return m
}

func merge(ms ...map[string]string) map[string]string {
	out := map[string]string{}
	for _, m := range ms {
		for k, v := range m {
			out[k] = v
		}
	}
	return out
}

func buildSites() []hsupport.Site {
	var sites []hsupport.Site

	// Aliases in an en/th site with canonifyURLs and minification.
	sites = append(sites, hsupport.Site{
		Name: "build-aliases",
		TOML: `baseURL = "https://example.org/"
title = "Aliases"
defaultContentLanguage = "en"
canonifyURLs = true
enableRobotsTXT = true
[languages.en]
weight = 1
languageCode = "en-US"
[languages.th]
weight = 2
languageCode = "th"
[pagination]
pagerSize = 2
[minify]
minifyOutput = true
[outputs]
home = ["html", "rss", "json"]
page = ["html", "print"]
[outputFormats.print]
mediaType = "text/html"
path = "print"
isHTML = true
baseName = "index"
permalinkable = true
[uglyURLs]
ugly = true
[taxonomies]
tag = "tags"
[build.buildStats]
enable = true
`,
		Files: inline(merge(layouts(map[string]string{
			"layouts/home.json":         `{"title": {{ .Title | jsonify }}}`,
			"layouts/robots.txt":        `User-agent: *`,
			"layouts/single.html":       `<!doctype html><html><head><title>{{ .Title }}</title></head><body class="single {{ .Section }}"><article id="main-{{ .Lang }}">{{ .Title }} <a href="/abs/link/">abs</a></article></body></html>`,
			"layouts/single.print.html": `<!doctype html><html><body class="print">{{ .Title }}</body></html>`,
		}), map[string]string{
			"content/_index.md":         fm(`title: "Home"`, `aliases: ["/old-home/", "/home.html"]`),
			"content/_index.th.md":      fm(`title: "หน้าแรก"`, `aliases: ["/old-home/"]`),
			"content/a.md":              fm(`title: "A"`, `aliases: ["/old/a", "relative-a", "sub/rel.html", "/b/", "/Mixed/Case/", "/bad:name/", "/dir/./x/../y/"]`, `tags: ["x", "y"]`),
			"content/a.th.md":           fm(`title: "เอ"`, `aliases: ["/th-old/a", "relative-th"]`, `tags: ["x"]`),
			"content/b.md":              fm(`title: "B"`, `tags: ["y"]`),
			"content/posts/_index.md":   fm(`title: "Posts"`, `aliases: ["/old-posts/", "rel-posts"]`),
			"content/posts/p1.md":       fm(`title: "P1"`, `date: 2020-01-01`, `aliases: ["/p/one/", "p1-rel"]`, `tags: ["x"]`),
			"content/posts/p2.md":       fm(`title: "P2"`, `date: 2020-01-02`, `tags: ["x", "z"]`),
			"content/posts/p3.md":       fm(`title: "P3"`, `date: 2020-01-03`, `tags: ["x"]`),
			"content/posts/p4.md":       fm(`title: "P4"`, `date: 2020-01-04`),
			"content/posts/p5.md":       fm(`title: "P5"`, `date: 2020-01-05`, `outputs: ["html"]`, `aliases: ["/only-html/"]`),
			"content/posts/p1.th.md":    fm(`title: "พี1"`, `date: 2020-01-01`),
			"content/ugly/_index.md":    fm(`title: "Ugly"`),
			"content/ugly/u.md":         fm(`title: "U"`, `aliases: ["/ugly-old", "/ugly-old2.html", "rel-u"]`),
			"content/bundle/index.md":   fm(`title: "Bundle"`, `aliases: ["/old-bundle/"]`),
			"content/bundle/data.txt":   "bundle resource",
			"content/bundle/img.png":    "png",
			"content/draft.md":          fm(`title: "Draft"`, `draft: true`, `aliases: ["/draft-alias/"]`),
			"content/headless/index.md": fm(`title: "Headless"`, `headless: true`, `aliases: ["/headless-alias/"]`),
			"content/nolist.md":         fm(`title: "No render"`, `build: {render: never}`, `aliases: ["/norender-alias/"]`),
		})),
	})

	// A custom alias.html; defaultContentLanguageInSubdir (the redirect is
	// written at the root); relativeURLs.
	sites = append(sites, hsupport.Site{
		Name: "build-custom-alias",
		TOML: `baseURL = "https://example.com/docs/"
title = "Custom alias"
defaultContentLanguage = "en"
defaultContentLanguageInSubdir = true
relativeURLs = true
[pagination]
pagerSize = 1
[outputs]
home = ["html", "rss"]
`,
		Files: inline(merge(layouts(map[string]string{
			"layouts/alias.html": `<!DOCTYPE html><html><head><title>{{ .Permalink }}</title><link rel="canonical" href="{{ .Permalink }}"><meta http-equiv="refresh" content="0; url={{ .Permalink }}"></head><body>{{ with .Page }}<p class="kind">{{ .Kind }} {{ .Title }}</p>{{ else }}<p class="root">root</p>{{ end }}<p class="lang">{{ site.Language.Lang }}</p></body></html>`,
		}), map[string]string{
			"content/_index.md":   fm(`title: "Home"`),
			"content/one.md":      fm(`title: "One"`, `aliases: ["/uno/", "../../escape"]`),
			"content/two.md":      fm(`title: "Two"`, `aliases: ["dos"]`),
			"content/s/_index.md": fm(`title: "S"`, `aliases: ["/ess/"]`),
			"content/s/s1.md":     fm(`title: "S1"`),
			"content/s/s2.md":     fm(`title: "S2"`),
			"content/s/s3.md":     fm(`title: "S3"`),
		})),
	})

	// disableAliases, pagination.disableAliases,
	// disableDefaultLanguageRedirect.
	sites = append(sites, hsupport.Site{
		Name: "build-disable",
		TOML: `baseURL = "https://example.org/"
title = "Disabled"
disableAliases = true
disableDefaultLanguageRedirect = true
[languages.en]
weight = 1
[languages.fr]
weight = 2
[pagination]
pagerSize = 1
disableAliases = true
`,
		Files: inline(merge(buildLayer, map[string]string{
			"content/_index.md": fm(`title: "Home"`, `aliases: ["/old-home/"]`),
			"content/a.md":      fm(`title: "A"`, `aliases: ["/old-a/"]`),
			"content/b.md":      fm(`title: "B"`),
			"content/a.fr.md":   fm(`title: "A fr"`, `aliases: ["/old-a-fr/"]`),
			"content/b.fr.md":   fm(`title: "B fr"`),
		})),
	})

	// Multihost: aliases in the language roots, robots.txt and 404 per
	// site, no main-language redirect.
	sites = append(sites, hsupport.Site{
		Name: "build-multihost",
		TOML: `title = "Multihost"
defaultContentLanguage = "fr"
enableRobotsTXT = true
[languages.en]
baseURL = "https://example.com/"
weight = 2
[languages.fr]
baseURL = "https://example.fr/"
weight = 1
[pagination]
pagerSize = 1
`,
		Files: inline(merge(buildLayer, map[string]string{
			"content/_index.md":    fm(`title: "Home"`),
			"content/a.md":         fm(`title: "A"`, `aliases: ["/old-a/", "rel-a", "/en/already/"]`),
			"content/b.md":         fm(`title: "B"`),
			"content/a.fr.md":      fm(`title: "A fr"`, `aliases: ["/old-a-fr/", "/fr/deja/"]`),
			"content/b.fr.md":      fm(`title: "B fr"`),
			"content/s/_index.md":  fm(`title: "S"`, `aliases: ["/old-s/"]`),
			"content/s/x/index.md": fm(`title: "X"`),
			"content/s/x/pic.png":  "png",
		})),
	})

	// Term collisions (several term pages sanitised to the same target:
	// the last in tree-key order wins), term paginators, 404, sitemaps,
	// robots, RSS; build stats without ids.
	colTags := `tags: ["Lay's", "Lays", "INS 322(i)", "ins-322i", "Disodium 5'-Guanylate", "disodium-5-guanylate"]`
	sites = append(sites, hsupport.Site{
		Name: "build-collide",
		TOML: `baseURL = "https://example.net/"
title = "Collide"
defaultContentLanguage = "en"
enableRobotsTXT = true
[languages.en]
weight = 1
[languages.th]
weight = 2
[pagination]
pagerSize = 1
[taxonomies]
tag = "tags"
category = "categories"
[minify]
minifyOutput = true
[build.buildStats]
enable = true
disableIDs = true
`,
		Files: inline(merge(buildLayer, map[string]string{
			"layouts/robots.txt":  `User-agent: *{{ range .Site.Pages }}{{ end }}`,
			"layouts/sitemap.xml": `<?xml version="1.0" encoding="utf-8" standalone="yes" ?><urlset>{{ range .Data.Pages }}<url><loc>{{ .Permalink }}</loc></url>{{ end }}</urlset>`,
			"content/_index.md":   fm(`title: "Home"`),
			"content/p1.md":       fm(`title: "P1"`, colTags, `categories: ["Snack", "snack"]`),
			"content/p2.md":       fm(`title: "P2"`, `tags: ["Lay's", "Lays"]`),
			"content/p3.md":       fm(`title: "P3"`, `tags: ["Lay's"]`),
			"content/p1.th.md":    fm(`title: "พี1"`, `tags: ["ไดโซเดียม 5'-กัวไนเลต", "ไดโซเดียม-5-กัวไนเลต", "INS 322(i)"]`),
			"content/p2.th.md":    fm(`title: "พี2"`, `tags: ["ไดโซเดียม 5'-กัวไนเลต"]`),
		})),
	})

	// Build stats toggles.
	statsLayouts := map[string]string{
		"layouts/home.html":   `<!doctype html><html><head><title>{{ .Title }}</title></head><body class="home b-{{ .Kind }}"><div id="app" class="x  y z-{{ .Lang }}"><span class='single'>{{ .Title }}</span><svg class="icon"><use href="#i"></use></svg><i class="md:w-1/2 hover:bg-[#fff] a&b"></i></div><script>var s = "<div class='no'>";</script>` + pagerBlock + `</body></html>`,
		"layouts/single.html": `<!doctype html><html><body class="single"><p id="p-{{ .File.BaseFileName }}" class="{{ .Params.cls }}">{{ .Title }}</p><!-- <div class="comment"> --></body></html>`,
	}
	sites = append(sites, hsupport.Site{
		Name: "build-stats-notags",
		TOML: `baseURL = "https://example.org/"
title = "Stats"
[pagination]
pagerSize = 1
[build.buildStats]
enable = true
disableTags = true
`,
		Files: inline(merge(layouts(statsLayouts), map[string]string{
			"content/_index.md": fm(`title: "Home"`),
			"content/a.md":      fm(`title: "A"`, `cls: "alpha beta"`),
			"content/b.md":      fm(`title: "B"`, `cls: "beta gamma"`, `aliases: ["/old-b/"]`),
		})),
	})
	sites = append(sites, hsupport.Site{
		Name: "build-stats-classes",
		TOML: `baseURL = "https://example.org/"
title = "Stats"
[pagination]
pagerSize = 1
[minify]
minifyOutput = true
[build.buildStats]
enable = true
disableClasses = true
disableIDs = true
`,
		Files: inline(merge(layouts(statsLayouts), map[string]string{
			"content/_index.md": fm(`title: "Home"`),
			"content/a.md":      fm(`title: "A"`, `cls: "alpha beta"`),
			"content/b.md":      fm(`title: "B"`, `cls: "beta gamma"`),
		})),
	})

	// resources.PostProcess placeholders (HTML and JSON) and js.Build with
	// an import from the project (jsconfig.json).
	css := `{{ $css := resources.Get "css/main.css" | minify | fingerprint | resources.PostProcess }}`
	sites = append(sites, hsupport.Site{
		Name: "build-postprocess",
		TOML: `baseURL = "https://example.org/"
title = "Post"
[pagination]
pagerSize = 1
[outputs]
home = ["html", "json"]
[build.buildStats]
enable = true
`,
		Files: inline(merge(layouts(map[string]string{
			"layouts/home.html":   css + `{{ $js := resources.Get "js/main.js" | js.Build }}<!doctype html><html><head><link rel="stylesheet" href="{{ $css.RelPermalink }}" integrity="{{ $css.Data.Integrity }}"><style>{{ $css.Content | safeCSS }}</style><script src="{{ $js.RelPermalink }}"></script></head><body data-name="{{ $css.Name }}" data-title="{{ $css.Title }}" data-rt="{{ $css.ResourceType }}" data-mt="{{ $css.MediaType.Type }}" data-sub="{{ $css.MediaType.SubType }}" data-pl="{{ $css.Permalink }}">` + pagerBlock + `</body></html>`,
			"layouts/home.json":   css + `{"css": {{ $css.RelPermalink | jsonify }}, "again": {{ $css.RelPermalink | jsonify }}}`,
			"layouts/list.html":   `{{ $o := resources.Get "css/other.css" | resources.PostProcess }}<!doctype html><html><head><link rel="stylesheet" href="{{ $o.RelPermalink }}"></head><body class="list">` + pagerBlock + `</body></html>`,
			"layouts/single.html": css + `<!doctype html><html><head><link rel="stylesheet" href="{{ $css.RelPermalink }}"></head><body class="single">{{ .Title }}</body></html>`,
		}), map[string]string{
			"assets/css/main.css":  "body {\n  color: red;\n}\n.a { margin: 0 }\n",
			"assets/css/other.css": ".other { padding: 1px; }\n",
			"assets/js/main.js":    "import { hello } from \"js/util\";\nconsole.log(hello());\n",
			"assets/js/util.js":    "export function hello() { return \"hello\"; }\n",
			"content/_index.md":    fm(`title: "Home"`),
			"content/a.md":         fm(`title: "A"`),
			"content/s/_index.md":  fm(`title: "S"`),
			"content/s/b.md":       fm(`title: "B"`),
		})),
	})

	// Render errors (two pages: the render pass goes on, one error is
	// picked and the other logged), an empty output (no file), a missing
	// layout.
	sites = append(sites, hsupport.Site{
		Name: "build-errors",
		TOML: `baseURL = "https://example.org/"
title = "Errors"
[pagination]
pagerSize = 1
[outputs]
home = ["html", "rss", "json"]
`,
		Files: inline(merge(layouts(map[string]string{
			"layouts/single.html": `{{ if eq .Title "Bad" }}{{ .NoSuchMethod }}{{ else if eq .Title "Empty" }}{{ else }}<p>{{ .Title }}</p>{{ end }}`,
		}), map[string]string{
			"content/_index.md": fm(`title: "Home"`),
			"content/a.md":      fm(`title: "A"`),
			"content/bad.md":    fm(`title: "Bad"`),
			"content/bad2.md":   fm(`title: "Bad"`),
			"content/empty.md":  fm(`title: "Empty"`, `aliases: ["/was-empty/"]`),
			"content/z.md":      fm(`title: "Z"`),
		})),
	})

	return sites
}
