package main

import (
	"strings"

	"github.com/neohugo/neohugo/tools/go-oracle/nh-hugolib/hsupport"
)

// The assembly-focused synthetic sites: auto sections, taxonomies and terms
// with case and slug collisions, disableKinds, build options, drafts/future/
// expired with and without the build flags, cascades with _target matchers,
// translations and translationKey, headless bundles, aliases, custom output
// formats per kind, permalinks for every kind, uglyURLs, multihost and
// defaultContentLanguageInSubdir.

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

func merge(ms ...map[string]string) map[string]string {
	out := map[string]string{}
	for _, m := range ms {
		for k, v := range m {
			out[k] = v
		}
	}
	return out
}

// AssembleSites returns the synthetic sites of this oracle.
func AssembleSites() []hsupport.Site {
	taxo := map[string]string{
		"content/about.md": fm(`title: "About"`, `linkTitle: "About us"`, `aliases: ["/old-about/", "old2"]`, `keywords: [a, b]`, `weight: 3`,
			`outputs: [html, JSON]`, `markup: md`, `sitemap: {changefreq: weekly, priority: 0.9}`, `summary: "Sum"`, `description: "Desc"`) + "\nAbout.\n",
		"content/contact.md": fm(`title: "Contact"`, `url: "/custom/:year/:slug/"`, `slug: "reach-us"`, `date: 2022-02-03`) + "\n",
		"content/direct.md":  fm(`title: "Direct"`, `url: "/direct/path/"`) + "\n",
		"content/posts/p1.md": fm(`title: "Post One"`, `date: 2021-03-04T05:06:07Z`, `slug: "first"`,
			`tags: ["Lays", "Lay's", "lays", "LAYS", " spaced ", "a/b", "INS 322(i)", "INS 322i"]`, `categories: "Cat One"`, `series: [1, true, 2.5]`, `tags_weight: 5`) + "\nOne.\n",
		"content/posts/p2.md":      fm(`title: "Post Two"`, `date: 2021-03-04T05:06:07Z`, `tags: ["Blue Sky", null, "lays"]`, `categories: ["Cat One", "cat one", "Cat Two"]`, `categories_weight: 2`) + "\nTwo.\n",
		"content/posts/p3.md":      fm(`title: "post three"`, `date: 2021-03-04T05:06:07Z`, `categories: [["nested"], "x"]`, `tags: []`, `series: []`, `tags_weight: "abc"`) + "\nThree.\n",
		"content/docs/x.md":        fm(`title: "Doc X"`, `date: 2020-01-01T00:00:00Z`, `tags: ["Lays"]`) + "\n",
		"content/docs/a/_index.md": fm(`title: "Docs A"`) + "\n",
		"content/docs/a/b/deep.md": fm(`title: "Deep"`, `date: 2020-06-01T00:00:00Z`) + "\n",
		"content/docs/a/c.md":      fm(`title: "C"`, `date: 2019-06-01T00:00:00Z`) + "\n",
		"content/news/n1.md":       fm(`title: "News 1"`, `date: 2018-01-01T00:00:00Z`) + "\n",
		"content/news/n2/index.md": fm(`title: "News 2"`, `date: 2018-01-02T00:00:00Z`,
			"resources:", "  - src: \"*.jpg\"", "    name: \"img-:counter\"", "    title: \"Image :counter\"", "    params:", "      k: v", "  - src: \"doc.pdf\"", "    title: \"The doc\"") + "\n",
		"content/news/n2/b.jpg":            "jpeg b",
		"content/news/n2/a.jpg":            "jpeg a",
		"content/news/n2/doc.pdf":          "pdf",
		"content/news/n2/sub/data.json":    `{"x": 1}`,
		"content/tags/lays/_index.md":      fm(`title: "Lays (content)"`, `date: 2019-05-05T00:00:00Z`) + "\n",
		"content/tags/unused/_index.md":    fm(`title: "Unused term"`) + "\n",
		"content/categories/_index.md":     fm(`title: "All categories"`) + "\n",
		"content/series/_index.md":         "",
		"content/Upper Case/Title Page.md": fm(`title: "Upper"`, `date: 2017-01-01T00:00:00Z`, `tags: ["Upper"]`) + "\n",
		"layouts/_shortcodes/x.html":       "x",
		// A page whose key prefixes a leaf bundle's: its (non-branch) resource walks also
		// reach the bundle's files and bundled page (setMetaPost runs twice on it).
		"content/leafy.md":             fm(`title: "Leafy page"`, `date: 2016-01-01T00:00:00Z`) + "\n",
		"content/leafy/b/index.md":     fm(`title: "Leafy bundle"`, `date: 2016-02-01T00:00:00Z`) + "\n",
		"content/leafy/b/img.jpg":      "jpeg",
		"content/leafy/b/sub/index.md": fm(`title: "Bundled in leafy"`, `date: 2016-03-01T00:00:00Z`) + "\n",
	}
	build := map[string]string{
		"content/_index.md":                    fm(`title: "Home"`) + "\n",
		"content/drafts/_index.md":             fm(`title: "Draft section"`, `draft: true`) + "\n",
		"content/drafts/p.md":                  fm(`title: "In draft section"`, `tags: [d]`) + "\n",
		"content/s/draft.md":                   fm(`title: "Draft"`, `draft: true`, `tags: [draft]`) + "\n",
		"content/s/published-false.md":         fm(`title: "Published false"`, `published: false`) + "\n",
		"content/s/future.md":                  fm(`title: "Future"`, `publishDate: 2999-01-01T00:00:00Z`, `tags: [future]`) + "\n",
		"content/s/expired.md":                 fm(`title: "Expired"`, `expiryDate: 2000-01-01T00:00:00Z`, `tags: [expired]`) + "\n",
		"content/s/ok.md":                      fm(`title: "OK"`, `date: 2020-02-02T00:00:00Z`, `tags: [ok, future]`) + "\n",
		"content/s/never.md":                   fm(`title: "List never"`, `build: {list: never}`, `tags: [ok]`) + "\n",
		"content/s/local.md":                   fm(`title: "List local"`, `build: {list: local}`, `tags: [ok]`) + "\n",
		"content/s/always.md":                  fm(`title: "List always"`, `build: {list: always, render: always}`) + "\n",
		"content/s/render-never.md":            fm(`title: "Render never"`, `build: {render: never}`, `tags: [rn]`) + "\n",
		"content/s/render-link.md":             fm(`title: "Render link"`, `build: {render: link}`) + "\n",
		"content/s/b/index.md":                 fm(`title: "Bundle"`, `build: {publishResources: false}`) + "\n",
		"content/s/b/r.txt":                    "resource",
		"content/s/b/img.png":                  "png",
		"content/s/headless/index.md":          fm(`title: "Headless"`, `headless: true`) + "\n",
		"content/s/headless/h.jpg":             "jpeg",
		"content/s/old-build/index.md":         fm(`title: "Old _build"`, `_build: {list: false, render: false}`) + "\n",
		"content/tags/future/_index.md":        fm(`title: "Future term"`, `publishDate: 2999-01-01T00:00:00Z`) + "\n",
		"content/tags/expired-term/_index.md":  fm(`title: "Expired term"`, `expiryDate: 2000-01-01T00:00:00Z`) + "\n",
		"content/tags/ok/_index.md":            fm(`title: "OK term"`, `date: 2019-01-01T00:00:00Z`) + "\n",
		"content/future-section/_index.md":     fm(`title: "Future section"`, `publishDate: 2999-01-01T00:00:00Z`) + "\n",
		"content/future-section/child.md":      fm(`title: "Child of future section"`) + "\n",
		"content/expired-leaf/index.md":        fm(`title: "Expired leaf"`, `expiryDate: 2000-01-01T00:00:00Z`) + "\n",
		"content/expired-leaf/img.jpg":         "jpeg",
		"content/expired-leaf/nested/index.md": fm(`title: "Nested in expired"`) + "\n",
	}
	buildTOML := `baseURL = "https://example.org/"
title = "Asm Build"
disableKinds = ["taxonomy", "RSS"]
[outputs]
home = ["html", "rss", "json"]
section = ["html", "rss"]
page = ["html", "json"]
term = ["html"]
[taxonomies]
tag = "tags"
`
	cascade := map[string]string{
		"content/_index.md": fm(`title: "Home"`, "cascade:", "  - params:", "      a: home", "    _target:", "      kind: \"{page,section}\"",
			"  - title: \"Cascaded title\"", "    date: 2020-01-01T00:00:00Z", "    _target:", "      path: \"/docs/**\"",
			"  - params:", "      prod: yes", "    _target:", "      environment: production",
			"  - params:", "      dev: yes", "    _target:", "      environment: development") + "\n",
		"content/docs/_index.md": fm(`title: "Docs"`, "cascade:", "  params:", "    b: docs", "    a: docs-override", "  _target:", "    kind: page") + "\n",
		"content/docs/p.md":      fm(`params: {own: 1}`, `a: page-own`) + "\n",
		"content/docs/q.md":      fm(`title: "Q has title"`, `tags: [c]`) + "\n",
		"content/docs/sub/_index.md": fm(`title: "Sub"`, "cascade:", "  - params:", "      b: sub", "      c: sub", "    _target:", "      path: \"/docs/sub/**\"",
			"  - params:", "      th: only", "    _target:", "      lang: th") + "\n",
		"content/docs/sub/r.md":          fm(`title: "R"`) + "\n",
		"content/docs/sub/r.th.md":       fm(`title: "R th"`) + "\n",
		"content/docs/sub/leaf/index.md": fm(`title: "Leaf"`) + "\n",
		"content/docs/sub/leaf/inner.md": fm(`title: "Bundled content"`) + "\n",
		"content/other/o.md":             fm(`title: "Other"`, `date: 2021-01-01T00:00:00Z`) + "\n",
		"content/tags/_index.md":         fm(`title: "Tags"`, "cascade:", "  params:", "    fromtags: true") + "\n",
		"content/tags/c/_index.md":       fm(`title: "C term"`) + "\n",
	}
	cascadeTOML := `baseURL = "https://example.org/"
title = "Asm Cascade"
[[cascade]]
[cascade.params]
sitewide = "yes"
[cascade._target]
kind = "page"
[[cascade]]
[cascade.params]
thonly = "x"
[cascade._target]
lang = "th"
[languages.en]
weight = 1
[languages.th]
weight = 2
[taxonomies]
tag = "tags"
`
	i18n := map[string]string{
		"content/_index.en.md":             fm(`title: "Home en"`) + "\n",
		"content/blog/a.en.md":             fm(`title: "A en"`, `translationKey: k1`, `tags: [shared, en]`, `date: 2020-01-01T00:00:00Z`) + "\n",
		"content/blog/b.th.md":             fm(`title: "B th"`, `translationKey: k1`, `tags: [shared]`) + "\n",
		"content/blog/c.fr.md":             fm(`title: "C fr"`, `aliases: [/fr-alias/]`) + "\n",
		"content/blog/c.md":                fm(`title: "C default"`) + "\n",
		"content/bundle1/index.en.md":      fm(`title: "Bundle 1 en"`) + "\n",
		"content/bundle1/index.th.md":      fm(`title: "Bundle 1 th"`) + "\n",
		"content/bundle1/img.jpg":          "jpeg shared",
		"content/bundle1/img.th.jpg":       "jpeg th",
		"content/bundle1/only-en.txt":      "en",
		"content/bundle2/index.fr.md":      fm(`title: "Bundle 2 fr"`, `translationKey: k2`) + "\n",
		"content/bundle2/fr.txt":           "fr text",
		"content/bundle3/index.en.md":      fm(`title: "Bundle 3 en"`, `translationKey: k2`) + "\n",
		"content/bundle3/en.txt":           "en text",
		"content/bundle3/fr.txt":           "en's fr text",
		"content/shared/index.md":          fm(`title: "Headless shared"`, `headless: true`) + "\n",
		"content/shared/s1.jpg":            "jpeg",
		"content/printable/p.md":           fm(`title: "P"`, `outputs: [print, html, feed]`) + "\n",
		"content/printable/q.md":           fm(`title: "Q"`, `outputs: [feed]`) + "\n",
		"content/tags/shared/_index.th.md": fm(`title: "Shared th"`) + "\n",
	}
	i18nTOML := `baseURL = "https://example.org/"
title = "Asm I18n"
defaultContentLanguage = "en"
defaultContentLanguageInSubdir = true
[outputFormats.print]
mediaType = "text/html"
path = "print"
baseName = "printable"
isHTML = true
permalinkable = true
[outputFormats.feed]
mediaType = "application/json"
baseName = "feed"
isPlainText = true
noUgly = true
weight = 5
[outputs]
page = ["html", "print"]
section = ["html", "feed", "rss"]
home = ["html", "feed", "rss", "print"]
[languages.en]
weight = 1
title = "English"
[languages.th]
weight = 2
title = "ไทย"
[languages.fr]
weight = 3
[taxonomies]
tag = "tags"
`
	multihost := map[string]string{
		"content/_index.md":          fm(`title: "Home"`) + "\n",
		"content/s/b/index.md":       fm(`title: "B en"`) + "\n",
		"content/s/b/index.th.md":    fm(`title: "B th"`) + "\n",
		"content/s/b/img.jpg":        "jpeg",
		"content/s/p.md":             fm(`title: "P"`, `tags: [x]`) + "\n",
		"content/s/p.th.md":          fm(`title: "P th"`, `tags: [x, y]`) + "\n",
		"content/leaf/index.th.md":   fm(`title: "TH only"`) + "\n",
		"content/leaf/th.jpg":        "jpeg",
		"content/docs/_index.th.md":  fm(`title: "Docs th"`) + "\n",
		"layouts/_shortcodes/x.html": "x",
	}
	multihostTOML := `title = "Asm Multihost"
defaultContentLanguage = "en"
enableRobotsTXT = true
[languages.en]
baseURL = "https://en.example.org/"
weight = 1
[languages.th]
baseURL = "https://th.example.org/sub/"
weight = 2
[taxonomies]
tag = "tags"
`
	ugly := map[string]string{
		"content/Section/_index.md":             fm(`title: ""`) + "\n",
		"content/Section/Page One.md":           fm(`title: "Page One"`, `tags: [Big, "big"]`, `date: 2020-03-03T00:00:00Z`) + "\n",
		"content/Section/Nested/Deep.md":        fm(`title: "Deep"`, `slug: "Deep-Slug-"`) + "\n",
		"content/Section/Bundle/index.md":       fm(`title: "Bundle"`) + "\n",
		"content/Section/Bundle/Image.JPG":      "jpeg",
		"content/other/x.md":                    fm(`title: "X"`) + "\n",
		"content/other/y.md":                    fm(`title: "Y"`, `url: "/y-url.html"`) + "\n",
		"content/tags/Big/_index.md":            fm(`title: "Big term"`) + "\n",
		"content/categories/Some Cat/_index.md": "",
	}
	uglyTOML := `baseURL = "https://example.org/"
title = "Asm Ugly"
uglyURLs = true
disablePathToLower = true
capitalizeListTitles = false
pluralizeListTitles = false
[permalinks.page]
other = "/o/:slugorcontentbasename/"
[permalinks.section]
other = "/o-section/"
[taxonomies]
tag = "tags"
category = "categories"
`

	return []hsupport.Site{
		{Name: "asm-taxo", TOML: `baseURL = "https://example.org/sub/"
title = "Asm Taxo"
enableRobotsTXT = true
[taxonomies]
tag = "tags"
category = "categories"
series = "series"
[permalinks]
posts = "/:year/:month/:slug/"
[permalinks.section]
posts = "/articles/"
[permalinks.taxonomy]
tags = "/topics/"
[permalinks.term]
tags = "/topic/:slug/"
categories = "/cat/:title/"
[permalinks.page]
docs = "/:sections/:filename/"
news = "/:section/:contentbasename/"
[uglyURLs]
news = true
[sitemap]
filename = "map.xml"
changefreq = "daily"
priority = 0.3
`, Files: inline(taxo)},
		{Name: "asm-build", TOML: buildTOML, Files: inline(build)},
		{Name: "asm-flags", TOML: strings.Replace(buildTOML, "title = \"Asm Build\"\n", "title = \"Asm Flags\"\nbuildDrafts = true\nbuildFuture = true\nbuildExpired = true\n", 1), Files: inline(build)},
		{Name: "asm-cascade", TOML: cascadeTOML, Files: inline(merge(cascade, map[string]string{
			"content/docs/p.th.md":        fm(`title: "P th"`, `tags: [c]`) + "\n",
			"content/_index.th.md":        fm(`title: "Home th"`, "cascade:", "  params:", "    a: home-th") + "\n",
			"content/tags/c/_index.th.md": fm(`title: "C th"`) + "\n",
		}))},
		{Name: "asm-i18n", TOML: i18nTOML, Files: inline(i18n)},
		{Name: "asm-multihost", TOML: multihostTOML, Files: inline(multihost)},
		{Name: "asm-ugly", TOML: uglyTOML, Files: inline(ugly)},
	}
}
