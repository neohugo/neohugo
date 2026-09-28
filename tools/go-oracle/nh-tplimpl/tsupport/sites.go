package tsupport

import (
	"encoding/json"
	"io/fs"
	"os"
	"path/filepath"
	"sort"
	"strings"
)

// The sites every nh-tplimpl oracle builds. The seeksnack site is private, so
// they are this repository's docs/ site (its real layouts, English content;
// templates are resolved for every page and output format but not executed,
// because they fetch remote data) and hugolib/testsite (embedded templates
// only), plus synthetic layout trees that are fully rendered:
//
//   - legacy: pre-0.146 paths (_default/, partials/, shortcodes/, index.html,
//     section/<name>.html, taxonomy/terms files, <x>-baseof.html, type and
//     layout front matter), language and output-format suffixes;
//   - modern: the 0.146 tree (home/page/section/taxonomy/term/list/single/all,
//     _partials, _shortcodes, _markup render hooks per section and output
//     format, baseof variants per section/kind/format), inline partials,
//     partial returns, templates.Defer, $_hugo_config, .Inner, plain text
//     output formats;
//   - themes: a project over two themes, the project in the new layout and the
//     themes in the old one (issue 13715), shadowed partials and shortcodes,
//     useEmbedded render hook settings.

// ReadTree reads every file below root/rel into a map keyed by prefix + the
// slash path relative to root/rel.
func ReadTree(root, rel, prefix string) (map[string]string, error) {
	out := map[string]string{}
	base := filepath.Join(root, rel)
	err := filepath.WalkDir(base, func(p string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if d.IsDir() {
			return nil
		}
		r, err := filepath.Rel(base, p)
		if err != nil {
			return err
		}
		b, err := os.ReadFile(p)
		if err != nil {
			return err
		}
		out[prefix+filepath.ToSlash(r)] = string(b)
		return nil
	})
	return out, err
}

// RepoSites are this repository's sites: docs/ (docs/hugo.toml, the English
// content, the real layouts) and hugolib/testsite (en + nn, no layouts).
func RepoSites(root string) ([]Site, error) {
	docs, err := ReadTree(root, "docs/content/en", "content/en/")
	if err != nil {
		return nil, err
	}
	layouts, err := ReadTree(root, "docs/layouts", "layouts/")
	if err != nil {
		return nil, err
	}
	for k, v := range layouts {
		docs[k] = v
	}
	docsTOML, err := os.ReadFile(filepath.Join(root, "docs", "hugo.toml"))
	if err != nil {
		return nil, err
	}
	// A mount source (module.mounts) that must exist.
	docs["hugo_stats.json"] = "{}\n"
	for k := range docs {
		// Content adapters that fetch remote data (the news section).
		if strings.HasSuffix(k, "_content.gotmpl") {
			delete(docs, k)
		}
	}
	toml := string(docsTOML)
	// The build stats writer writes hugo_stats.json through the OS working
	// directory; the oracle builds in memory.
	toml = strings.Replace(toml, "disableIDs = true\n    enable     = true", "disableIDs = true\n    enable     = false", 1)

	ts, err := ReadTree(root, filepath.Join("hugolib", "testsite"), "")
	if err != nil {
		return nil, err
	}
	tsTOML := `baseURL = "https://example.org/"
defaultContentLanguage = "en"
[languages.en]
weight = 1
contentDir = "content"
[languages.nn]
weight = 2
contentDir = "content_nn"
`
	return []Site{
		{Name: "docs", TOML: toml, Files: docs, Render: false},
		{Name: "testsite", TOML: tsTOML, Files: ts, Render: true},
	}, nil
}

const commonTOML = `
baseURL = "https://example.org/"
title = "Oracle"
enableRobotsTXT = true
defaultContentLanguage = "en"
[pagination]
pagerSize = 20
[taxonomies]
tag = "tags"
category = "categories"
[languages.en]
languageName = "English"
weight = 1
[languages.th]
languageName = "Thai"
weight = 2
[mediaTypes."text/netlify"]
delimiter = ""
[outputFormats.plain]
mediaType = "text/plain"
baseName = "plain"
isPlainText = true
[outputFormats.redir]
mediaType = "text/netlify"
baseName = "_redirects"
isPlainText = true
notAlternative = true
[outputFormats.fancy]
mediaType = "text/html"
path = "fancy"
isHTML = true
`

func fm(lines ...string) string {
	return "---\n" + strings.Join(lines, "\n") + "\n---\n"
}

// markdownBody exercises every render hook kind.
const markdownBody = `
# Heading One

Some [a link](https://example.org/x "T") and ![an image](/img.png "IT") and <https://auto.org>.

## Heading Two {#custom}

` + "```go" + `
package main
` + "```" + `

` + "```plain" + `
text
` + "```" + `

` + "```" + `
no lang
` + "```" + `

| a | b |
|---|---|
| 1 | 2 |

> [!NOTE]
> An alert.

> A quote.
`

// syntheticContent is the content of the synthetic sites.
func syntheticContent(shortcodes string) map[string]string {
	return map[string]string{
		"content/_index.md":            fm(`title: Home`) + "Home content.\n" + markdownBody,
		"content/_index.th.md":         fm(`title: Home TH`) + "Home TH.\n",
		"content/about.md":             fm(`title: About`, `layout: mylayout`) + "About.\n" + markdownBody,
		"content/contact.md":           fm(`title: Contact`, `layout: nosuchlayout`) + "Contact.\n",
		"content/blog/_index.md":       fm(`title: Blog`, `outputs: ["html", "rss", "plain"]`) + "Blog.\n",
		"content/blog/post1.md":        fm(`title: Post 1`, `tags: [a, b]`, `categories: [c1]`) + "Post 1.\n" + markdownBody + shortcodes,
		"content/blog/post1.th.md":     fm(`title: Post 1 TH`, `tags: [a]`) + "Post 1 TH.\n" + markdownBody,
		"content/blog/post2.md":        fm(`title: Post 2`, `type: posts`, `tags: [b]`) + "Post 2.\n" + markdownBody,
		"content/blog/sub/_index.md":   fm(`title: Sub`) + "Sub.\n",
		"content/blog/sub/deep.md":     fm(`title: Deep`, `layout: mylayout`, `outputs: ["html", "json", "fancy", "plain"]`) + "Deep.\n" + markdownBody,
		"content/docs/_index.md":       fm(`title: Docs`) + "Docs.\n",
		"content/docs/d1.md":           fm(`title: D1`, `categories: [c2]`) + "D1.\n" + markdownBody + shortcodes,
		"content/docs/d2/index.md":     fm(`title: D2 bundle`, `outputs: ["html", "plain", "redir"]`) + "D2.\n" + markdownBody,
		"content/posts/_index.md":      fm(`title: Posts`) + "Posts.\n",
		"content/posts/p1.md":          fm(`title: P1`, `layout: special`) + "P1.\n" + markdownBody,
		"content/news/n1.md":           fm(`title: N1`, `type: blog`) + "N1.\n" + markdownBody,
		"content/tags/_index.md":       fm(`title: Tags`) + "Tags.\n",
		"content/tags/a/_index.md":     fm(`title: Tag A`) + "Tag A.\n",
		"content/categories/_index.md": fm(`title: Categories`, `outputs: ["html", "json"]`) + "Cats.\n",
	}
}

const siteOutputs = `
[outputs]
home = ["html", "rss", "json", "plain", "redir", "fancy"]
section = ["html", "rss", "json"]
page = ["html", "json", "fancy"]
taxonomy = ["html", "rss"]
term = ["html", "rss", "json"]
`

// legacySite uses the pre-0.146 layout paths.
func legacySite() Site {
	files := syntheticContent("\n{{< note >}}Inner **md**{{< /note >}}\n{{% box %}}Box{{% /box %}}\n{{< simple >}}\n{{< tweet user=\"x\" id=\"1\" >}}\n")
	layouts := map[string]string{
		"layouts/_default/baseof.html":                 `<html>{{ block "main" . }}base main{{ end }}|{{ partial "footer.html" . }}|{{ partialCached "head/meta.html" . }}</html>`,
		"layouts/_default/baseof.fancy.html":           `<fancy>{{ block "main" . }}fancy base{{ end }}</fancy>`,
		"layouts/_default/list.html":                   `{{ define "main" }}list {{ .Title }}{{ range .Pages }}{{ .Title }}{{ end }}{{ end }}`,
		"layouts/_default/single.html":                 `{{ define "main" }}single {{ .Title }} {{ .Content }}{{ end }}`,
		"layouts/_default/single.json":                 `{{ .Title | jsonify }}`,
		"layouts/_default/list.json":                   `{{ range .Pages }}{{ .Title | jsonify }}{{ end }}`,
		"layouts/_default/single.plain.txt":            `plain single {{ .Title }}`,
		"layouts/_default/list.plain.txt":              `plain list {{ .Title }}{{ range .Pages }} {{ .Title }}{{ end }}`,
		"layouts/_default/mylayout.html":               `{{ define "main" }}mylayout {{ .Title }}{{ .Content }}{{ end }}`,
		"layouts/_default/terms.html":                  `{{ define "main" }}terms {{ .Title }}{{ range .Data.Terms }}{{ .Page.Title }}{{ end }}{{ end }}`,
		"layouts/_default/taxonomy.html":               `{{ define "main" }}taxonomy-legacy {{ .Title }}{{ end }}`,
		"layouts/_default/single.th.html":              `{{ define "main" }}single th {{ .Title }}{{ .Content }}{{ end }}`,
		"layouts/_default/list.fancy.html":             `{{ define "main" }}fancy list {{ .Title }}{{ end }}`,
		"layouts/_default/_markup/render-link.html":    `<a href="{{ .Destination | safeURL }}">{{ .Text }}</a>`,
		"layouts/_default/_markup/render-heading.html": `<h{{ .Level }} id="{{ .Anchor }}">{{ .Text }}</h{{ .Level }}>`,
		"layouts/index.html":                           `{{ define "main" }}home {{ .Title }}{{ range .Site.RegularPages }}{{ .RelPermalink }}{{ end }}{{ .Content }}{{ end }}`,
		"layouts/index.json":                           `{"title": {{ .Title | jsonify }}}`,
		"layouts/index.redir":                          `/old /new 301`,
		"layouts/section/blog.html":                    `{{ define "main" }}section blog {{ .Title }}{{ end }}`,
		"layouts/section/section.html":                 `{{ define "main" }}section section {{ .Title }}{{ end }}`,
		"layouts/blog/single.html":                     `{{ define "main" }}blog single {{ .Title }}{{ .Content }}{{ end }}`,
		"layouts/docs/list-baseof.html":                `<docs>{{ block "main" . }}docs base{{ end }}</docs>`,
		"layouts/docs/list.html":                       `{{ define "main" }}docs list {{ .Title }}{{ end }}`,
		"layouts/docs/baseof.html":                     `<docsbase>{{ block "main" . }}docs single base{{ end }}</docsbase>`,
		"layouts/posts/single.html":                    `{{ define "main" }}posts single {{ .Title }}{{ .Content }}{{ end }}`,
		"layouts/posts/special.html":                   `{{ define "main" }}posts special {{ .Title }}{{ end }}`,
		"layouts/taxonomy/tag.html":                    `{{ define "main" }}taxonomy tag {{ .Title }}{{ end }}`,
		"layouts/taxonomy/tag.terms.html":              `{{ define "main" }}tag terms {{ .Title }}{{ end }}`,
		"layouts/tags/term.html":                       `{{ define "main" }}tags term {{ .Title }}{{ end }}`,
		"layouts/categories/list.html":                 `{{ define "main" }}categories list {{ .Title }}{{ end }}`,
		"layouts/partials/footer.html":                 `footer {{ .Title }}`,
		"layouts/partials/head/meta.html":              `<meta name="t" content="{{ .Title }}">`,
		"layouts/partials/ret.html":                    `{{ $x := printf "r-%s" . }}{{ return $x }}`,
		"layouts/shortcodes/note.html":                 `<div class="note">{{ .Inner | markdownify }}</div>`,
		"layouts/shortcodes/box.html":                  `{{ $_hugo_config := ` + "`" + `{ "version": 1 }` + "`" + ` }}<box>{{ .Inner }}</box>`,
		"layouts/shortcodes/simple.html":               `simple {{ partial "ret.html" "x" }}`,
		"layouts/shortcodes/simple.json":               `"simple"`,
		"layouts/shortcodes/simple.plain.txt":          `simple plain`,
		"layouts/404.html":                             `404 {{ .Title }}`,
		"layouts/robots.txt":                           "User-agent: *\n",
		"layouts/_default/rss.xml":                     `<rss>{{ .Title }}</rss>`,
		"layouts/.hidden.html":                         `hidden`,
		"layouts/_default/backup.html~":                `backup`,
	}
	for k, v := range layouts {
		files[k] = v
	}
	return Site{
		Name:   "legacy",
		TOML:   commonTOML + siteOutputs,
		Files:  files,
		Render: true,
	}
}

// modernSite uses the 0.146 layout tree.
func modernSite() Site {
	files := syntheticContent("\n{{< note >}}Inner **md**{{< /note >}}\n{{% box %}}Box{{% /box %}}\n{{< simple >}}\n{{< local >}}\n{{< x user=\"u\" id=\"1\" >}}\n")
	layouts := map[string]string{
		"layouts/baseof.html":             `<html>{{ block "main" . }}base{{ end }}|{{ partial "footer.html" . }}|{{ partial "inline.html" . }}{{ define "_partials/inline.html" }}inline {{ .Title }}{{ end }}</html>`,
		"layouts/baseof.fancy.html":       `<fancy>{{ block "main" . }}fancy{{ end }}</fancy>`,
		"layouts/baseof.plain.txt":        `PLAIN[{{ block "main" . }}plain base{{ end }}]`,
		"layouts/blog/baseof.html":        `<blog>{{ block "main" . }}blog base{{ end }}</blog>`,
		"layouts/docs/baseof.list.html":   `<docslist>{{ block "main" . }}docs list base{{ end }}</docslist>`,
		"layouts/home.html":               `{{ define "main" }}home {{ .Title }}{{ .Content }}{{ with (templates.Defer (dict "key" "home")) }}deferred {{ now.Year }}{{ end }}{{ end }}`,
		"layouts/home.th.html":            `{{ define "main" }}home th {{ .Title }}{{ end }}`,
		"layouts/home.json":               `{"home": {{ .Title | jsonify }}}`,
		"layouts/home.plain.txt":          `{{ define "main" }}home plain {{ .Title }}{{ end }}`,
		"layouts/home.redir":              `/a /b 301`,
		"layouts/page.html":               `{{ define "main" }}page {{ .Title }}{{ .Content }}{{ end }}`,
		"layouts/page.json":               `{{ .Title | jsonify }}`,
		"layouts/single.fancy.html":       `{{ define "main" }}single fancy {{ .Title }}{{ end }}`,
		"layouts/section.html":            `{{ define "main" }}section {{ .Title }}{{ end }}`,
		"layouts/section.plain.txt":       `{{ define "main" }}section plain {{ .Title }}{{ end }}`,
		"layouts/taxonomy.html":           `{{ define "main" }}taxonomy {{ .Title }}{{ end }}`,
		"layouts/term.html":               `{{ define "main" }}term {{ .Title }}{{ end }}`,
		"layouts/list.html":               `{{ define "main" }}list {{ .Title }}{{ end }}`,
		"layouts/list.json":               `{{ range .Pages }}{{ .Title | jsonify }}{{ end }}`,
		"layouts/all.html":                `{{ define "main" }}all {{ .Title }}{{ end }}`,
		"layouts/all.plain.txt":           `all plain {{ .Title }}`,
		"layouts/mylayout.html":           `{{ define "main" }}mylayout {{ .Title }}{{ end }}`,
		"layouts/blog/mylayout.html":      `{{ define "main" }}blog mylayout {{ .Title }}{{ .Content }}{{ end }}`,
		"layouts/blog/page.html":          `{{ define "main" }}blog page {{ .Title }}{{ .Content }}{{ end }}`,
		"layouts/blog/page.th.html":       `{{ define "main" }}blog page th {{ .Title }}{{ .Content }}{{ end }}`,
		"layouts/blog/section.html":       `{{ define "main" }}blog section {{ .Title }}{{ end }}`,
		"layouts/blog/sub/list.html":      `{{ define "main" }}blog sub list {{ .Title }}{{ end }}`,
		"layouts/posts/page.html":         `{{ define "main" }}posts page {{ .Title }}{{ .Content }}{{ end }}`,
		"layouts/posts/special.html":      `{{ define "main" }}posts special {{ .Title }}{{ end }}`,
		"layouts/docs/list.html":          `{{ define "main" }}docs list {{ .Title }}{{ end }}`,
		"layouts/docs/page.html":          `{{ define "main" }}docs page {{ .Title }}{{ .Content }}{{ end }}`,
		"layouts/tags/term.html":          `{{ define "main" }}tags term {{ .Title }}{{ end }}`,
		"layouts/tags/taxonomy.html":      `{{ define "main" }}tags taxonomy {{ .Title }}{{ end }}`,
		"layouts/404.html":                `{{ define "main" }}404{{ end }}`,
		"layouts/rss.xml":                 `<rss>{{ .Title }}</rss>`,
		"layouts/list.rss.xml":            `<rss list>{{ .Title }}</rss>`,
		"layouts/section.rss.xml":         `<rss section>{{ .Title }}</rss>`,
		"layouts/_partials/footer.html":   `footer {{ .Title }}{{ partial "_funcs/f.html" . }}`,
		"layouts/_partials/_funcs/f.html": `{{ $r := "" }}{{ with . }}{{ $r = .Title }}{{ end }}{{ return $r }}`,
		"layouts/_partials/ret.html":      `{{ $r := printf "%s-r" . }}{{ if eq . "x" }}{{ $r = "rx" }}{{ end }}{{ return $r }}`,
		// Not executed: the command after a return stays in the pipeline (Go quirk).
		"layouts/_partials/quirk.html":                  `{{ $r := 1 }}{{ return $r | upper }}{{ return 2 }}`,
		"layouts/_partials/json.json":                   `{"p": 1}`,
		"layouts/_partials/text.txt":                    `text partial`,
		"layouts/_partials/tpl.html":                    `{{ template "shared" . }}{{ define "shared" }}shared {{ . }}{{ end }}`,
		"layouts/_shortcodes/note.html":                 `<div class="note">{{ .Inner | markdownify }}</div>`,
		"layouts/_shortcodes/note.th.html":              `<div class="note th">{{ .Inner }}</div>`,
		"layouts/_shortcodes/box.html":                  `{{ $_hugo_config := ` + "`" + `{ "version": 1 }` + "`" + ` }}<box>{{ .Inner }}</box>`,
		"layouts/_shortcodes/simple.html":               `simple {{ partial "ret.html" "x" }}{{ partial "ret.html" "y" }}`,
		"layouts/_shortcodes/simple.json":               `"simple"`,
		"layouts/_shortcodes/simple.fancy.html":         `simple fancy`,
		"layouts/_shortcodes/local.html":                `local root`,
		"layouts/docs/_shortcodes/local.html":           `local docs`,
		"layouts/_markup/render-link.html":              `<a href="{{ .Destination | safeURL }}">{{ .Text }}</a>`,
		"layouts/_markup/render-link.json.json":         `{{ .Destination }}`,
		"layouts/_markup/render-image.html":             `<img src="{{ .Destination | safeURL }}" alt="{{ .PlainText }}">`,
		"layouts/_markup/render-heading.html":           `<h{{ .Level }} id="{{ .Anchor }}">{{ .Text }}</h{{ .Level }}>`,
		"layouts/_markup/render-codeblock.html":         `<pre>{{ .Inner }}</pre>`,
		"layouts/_markup/render-codeblock-go.html":      `<pre class="go">{{ .Inner }}</pre>`,
		"layouts/_markup/render-blockquote.html":        `<blockquote>{{ .Text }}</blockquote>`,
		"layouts/_markup/render-blockquote-alert.html":  `<div class="alert">{{ .Text }}</div>`,
		"layouts/docs/_markup/render-link.html":         `<a class="docs" href="{{ .Destination | safeURL }}">{{ .Text }}</a>`,
		"layouts/docs/_markup/render-table.html":        `<table class="docs"></table>`,
		"layouts/blog/_markup/render-heading.plain.txt": `H: {{ .Text }}`,
	}
	for k, v := range layouts {
		files[k] = v
	}
	return Site{
		Name:   "modern",
		TOML:   commonTOML + siteOutputs,
		Files:  files,
		Render: true,
	}
}

// themesSite is a project over two themes.
func themesSite() Site {
	files := syntheticContent("\n{{< note >}}Inner{{< /note >}}\n{{< themed >}}\n")
	layouts := map[string]string{
		// Project: new layout.
		"layouts/single.html":           `{{ define "main" }}project single {{ .Title }}{{ .Content }}{{ end }}`,
		"layouts/_partials/footer.html": `project footer`,
		"layouts/_shortcodes/note.html": `project note {{ .Inner }}`,
		"layouts/home.html":             `{{ define "main" }}project home {{ .Content }}{{ end }}`,
		// Theme mytheme: old layout.
		"themes/mytheme/layouts/_default/baseof.html":               `<theme>{{ block "main" . }}theme base{{ end }}|{{ partial "footer.html" . }}|{{ partial "themeonly.html" . }}</theme>`,
		"themes/mytheme/layouts/_default/single.html":               `{{ define "main" }}theme single {{ .Title }}{{ end }}`,
		"themes/mytheme/layouts/_default/list.html":                 `{{ define "main" }}theme list {{ .Title }}{{ end }}`,
		"themes/mytheme/layouts/index.html":                         `{{ define "main" }}theme home{{ end }}`,
		"themes/mytheme/layouts/partials/footer.html":               `theme footer`,
		"themes/mytheme/layouts/partials/themeonly.html":            `theme only`,
		"themes/mytheme/layouts/shortcodes/note.html":               `theme note {{ .Inner }}`,
		"themes/mytheme/layouts/shortcodes/themed.html":             `themed`,
		"themes/mytheme/layouts/_default/_markup/render-image.html": `<img theme src="{{ .Destination | safeURL }}">`,
		"themes/mytheme/layouts/_default/_markup/render-link.html":  `<a theme href="{{ .Destination | safeURL }}">{{ .Text }}</a>`,
		"themes/mytheme/layouts/blog/list.html":                     `{{ define "main" }}theme blog list{{ end }}`,
		"themes/mytheme/layouts/taxonomy/tag.html":                  `{{ define "main" }}theme tag{{ end }}`,
		"themes/mytheme/theme.toml":                                 `name = "mytheme"`,
		// Theme other.
		"themes/other/layouts/_default/single.html":    `{{ define "main" }}other single{{ end }}`,
		"themes/other/layouts/_default/terms.html":     `{{ define "main" }}other terms{{ end }}`,
		"themes/other/layouts/_shortcodes/themed.html": `other themed`,
		"themes/other/layouts/_partials/footer.html":   `other footer`,
		"themes/other/layouts/docs/single.html":        `{{ define "main" }}other docs single{{ end }}`,
		"themes/other/theme.toml":                      `name = "other"`,
	}
	for k, v := range layouts {
		files[k] = v
	}
	toml := `theme = ["mytheme", "other"]
` + commonTOML + siteOutputs + `
[markup.goldmark.renderHooks.image]
useEmbedded = "always"
[markup.goldmark.renderHooks.link]
useEmbedded = "fallback"
`
	return Site{
		Name:   "themes",
		TOML:   toml,
		Files:  files,
		Render: true,
	}
}

// SyntheticSites are the synthetic layout trees.
func SyntheticSites() []Site {
	return []Site{legacySite(), modernSite(), themesSite()}
}

// AllSites are the repository sites, the synthetic ones and the integration
// test archives that build (IntegrationSites).
func AllSites(root string) ([]Site, error) {
	sites, err := RepoSites(root)
	if err != nil {
		return nil, err
	}
	sites = append(sites, SyntheticSites()...)
	its, err := IntegrationSites(root)
	if err != nil {
		return nil, err
	}
	for _, s := range its {
		b, err := New(s)
		if err == nil {
			err = b.Build()
		}
		if err != nil {
			continue
		}
		sites = append(sites, s)
	}
	return sites, nil
}

// IsIntegration reports whether the site is an integration test archive
// (their fixtures are written to one file per topic, integration.json.gz).
func IsIntegration(s Site) bool {
	return strings.HasPrefix(s.Name, "it_")
}

// Fixtures collects the fixtures of one oracle topic.
type Fixtures struct {
	dir         string
	integration map[string]any
}

// NewFixtures writes to dir.
func NewFixtures(dir string) *Fixtures {
	return &Fixtures{dir: dir, integration: map[string]any{}}
}

// Add writes the fixture of a site (<site>.json.gz), or keeps an
// integration site's for Close.
func (f *Fixtures) Add(s Site, v any) error {
	if IsIntegration(s) {
		f.integration[s.Name] = v
		return nil
	}
	return WriteFixture(f.dir, s.Name+".json.gz", v)
}

// Close writes integration.json.gz ({"sites": {name: fixture}, "pool":
// [...]}): the integration sites repeat much (the embedded templates, the
// descriptors), so every string of 32 bytes or more is stored once in the
// pool and referenced as {"$p": index} (in the order of first use, map keys
// sorted).
func (f *Fixtures) Close() error {
	b, err := json.Marshal(map[string]any{"sites": f.integration})
	if err != nil {
		return err
	}
	var v any
	if err := json.Unmarshal(b, &v); err != nil {
		return err
	}
	p := &pooler{index: map[string]int{}}
	v = p.walk(v)
	v.(map[string]any)["pool"] = p.pool
	return WriteFixture(f.dir, "integration.json.gz", v)
}

type pooler struct {
	index map[string]int
	pool  []string
}

func (p *pooler) walk(v any) any {
	switch v := v.(type) {
	case string:
		if len(v) < 32 {
			return v
		}
		i, ok := p.index[v]
		if !ok {
			i = len(p.pool)
			p.index[v] = i
			p.pool = append(p.pool, v)
		}
		return map[string]any{"$p": i}
	case []any:
		for i := range v {
			v[i] = p.walk(v[i])
		}
		return v
	case map[string]any:
		keys := make([]string, 0, len(v))
		for k := range v {
			keys = append(keys, k)
		}
		sort.Strings(keys)
		for _, k := range keys {
			v[k] = p.walk(v[k])
		}
		return v
	default:
		return v
	}
}
