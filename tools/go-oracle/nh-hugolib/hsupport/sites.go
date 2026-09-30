package hsupport

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/neohugo/neohugo/tools/go-oracle/nh-page/psupport"
)

// The sites of the nh-hugolib oracles. The seeksnack site is private, so
// they are: this repository's docs/ site and hugolib/testsite (as the nh-page
// oracles build them), nh-page's synthetic en/th site, the reconstructed
// seeksnack config (rust/testdata/oracle/allconfig/load/seeksnack) with a
// synthetic en/th content tree covering what capture depends on, and small
// sites that stress page-tree edge cases.

// fm is a YAML front matter block.
func fm(lines ...string) string {
	return "---\n" + strings.Join(lines, "\n") + "\n---\n"
}

func inline(m map[string]string) map[string]File {
	out := map[string]File{}
	for k, v := range m {
		out[k] = File{Content: v}
	}
	return out
}

// RepoSites are this repository's sites as psupport.RepoSites builds them;
// the files taken unchanged from the repository are recorded as references.
func RepoSites(root string) ([]Site, error) {
	sites, err := psupport.RepoSites(root)
	if err != nil {
		return nil, err
	}
	var out []Site
	for _, s := range sites {
		files := map[string]File{}
		for k, v := range s.Files {
			var repo string
			switch s.Name {
			case "docs":
				if strings.HasPrefix(k, "content/en/") {
					repo = "docs/" + k
				}
			case "testsite":
				repo = "hugolib/testsite/" + k
			}
			if repo != "" {
				b, err := os.ReadFile(filepath.Join(root, filepath.FromSlash(repo)))
				if err != nil || string(b) != v {
					repo = ""
				}
			}
			files[k] = File{Content: v, Repo: repo}
		}
		toml := s.TOML
		if s.Name == "docs" {
			// The goldmark passthrough extension is not ported (an explicit error in
			// nh-markup); capture does not depend on it.
			const pt = "[markup.goldmark.extensions.passthrough]\n        enable = true"
			if !strings.Contains(toml, pt) {
				return nil, fmt.Errorf("docs/hugo.toml: passthrough anchor not found")
			}
			toml = strings.Replace(toml, pt, "[markup.goldmark.extensions.passthrough]\n        enable = false", 1)
			// Likewise goldmark-emoji (enableEmoji).
			const emoji = "enableEmoji            = true"
			if !strings.Contains(toml, emoji) {
				return nil, fmt.Errorf("docs/hugo.toml: enableEmoji anchor not found")
			}
			toml = strings.Replace(toml, emoji, "enableEmoji            = false", 1)
		}
		out = append(out, Site{Name: s.Name, TOML: toml, Files: files})
	}
	return out, nil
}

// SyntheticSite is nh-page's synthetic en/th site (psupport's "seeksnack"
// configuration).
func SyntheticSite() Site {
	s := psupport.SyntheticSites()[0]
	files := map[string]File{}
	for k, v := range s.Files {
		files[k] = File{Content: v}
	}
	return Site{Name: "synthetic", TOML: s.TOML, Files: files}
}

// Shortcodes are the shortcode templates of the synthetic sites: some use
// .Inner (they must be closed), one is a version 1 shortcode (its {{% %}}
// output is inserted after markdown rendering).
func Shortcodes() map[string]string {
	return map[string]string{
		"layouts/_shortcodes/note.html":   `<div class="note">{{ .Inner }}</div>`,
		"layouts/_shortcodes/box.html":    `<div class="box {{ .Get "class" }}">{{ .Inner | markdownify }}</div>`,
		"layouts/_shortcodes/badge.html":  `<span>{{ .Get 0 }}{{ with .Get 1 }}-{{ . }}{{ end }}</span>`,
		"layouts/_shortcodes/img.html":    `<img src="{{ .Get "src" }}" alt="{{ .Get "alt" }}" width="{{ .Get "width" }}">`,
		"layouts/_shortcodes/v1.html":     "{{ $_hugo_config := `{ \"version\": 1 }` }}<em>{{ .Inner }}</em>",
		"layouts/_shortcodes/quote.html":  `<blockquote>{{ .Inner }}</blockquote>`,
		"layouts/_shortcodes/Mixed.html":  `mixed`,
		"layouts/_shortcodes/thai.html":   `{{ .Get "คำ" }}`,
		"layouts/_shortcodes/empty.html":  `{{- .Inner -}}`,
		"layouts/_shortcodes/nested.html": `<section>{{ .Inner }}</section>`,
	}
}

const seeksnackBody = `
First paragraph with {{< badge "new" "hot" >}} and {{< img src="/a.jpg" alt="An image" width=300 >}}.

<!--more-->

{{% box class="wide" %}}
Some **markdown** in a box with {{< badge 42 >}} nested and {{< badge 3.14 true >}}.
{{% /box %}}

  {{< note >}}
  Indented note with {{< nested >}}deep {{< badge "x" >}}{{< /nested >}}
  {{< /note >}}

See [ref]({{< ref "/biscuit/koalas-march-chocolate" >}}) and {{< relref "disclaimer.md" >}}.

{{% v1 %}}version one{{% /v1 %}} {{< v1 >}}*v1 no markup*{{< /v1 >}}
{{</* escaped "shortcode" */>}}
{{% quote %}}quoted{{% /quote %}} {{< Mixed >}} {{< thai คำ="ขนม" >}} {{< empty />}}
`

// SeeksnackSite is the reconstructed seeksnack config with a synthetic
// content tree.
func SeeksnackSite(root string) (Site, error) {
	toml, err := os.ReadFile(filepath.Join(root, "crates", "nh-allconfig", "tests", "fixtures", "load", "seeksnack", "hugo.toml"))
	if err != nil {
		return Site{}, err
	}
	c := map[string]string{
		"content/_index.md":     "",
		"content/about.md":      fm(`title: "About"`, `date: 2020-01-02T03:04:05.678Z`, `type: seo`, `layout: simple`) + seeksnackBody,
		"content/disclaimer.md": fm(`title: "Disclaimer"`, `Description: "Capital D"`) + "\nText.\n",
		"content/search.md":     fm(`title: "Search"`, `type: search`, `layout: search`, `sitemap: {priority: 0.1}`, `output: [html]`) + "\n",
		"content/latesturl.md":  "+++\ntitle = \"Latest\"\ntype = \"seo\"\nprivate = true\ndate = 2019-12-31T07:06:21.671Z\n+++\nTOML page.\n",
		"content/biscuit/koalas-march-chocolate/index.en.md":                             fm(`title: "Koala's March Chocolate"`, `date: "2020-12-10T12:45:19.622Z"`, `image: koala.jpg`, `brands: Lotte`, `companies: ["Lotte Co., Ltd."]`, `categories: ["biscuit", ["biscuit"]]`, `tags: ["Koala", "Chocolate", null]`, `ingredients: []`, `rating: {Taste: 4, Smell: 3.5}`, `ingredients_percentage: [{Name: Flour, Value: 30}, {Name: Sugar, Value: null}]`) + seeksnackBody,
		"content/biscuit/koalas-march-chocolate/index.th.md":                             fm(`title: "โคอะลา มาร์ช"`, `date: "2020-12-10T12:45:19.622Z"`, `brands: Lotte`, `tags: ['ปาร์ตี้','คริสปี้พาย ', null]`) + "\nขนม {{< badge \"ไทย\" >}}\n",
		"content/biscuit/koalas-march-chocolate/koala.jpg":                               "jpeg",
		"content/biscuit/koalas-march-chocolate/koala_pack.jpg":                          "jpeg2",
		"content/biscuit/koalas-march-chocolate/sub/nested.txt":                          "nested resource",
		"content/biscuit/koalas-march-chocolate/sub/index.md":                            fm(`title: "Bundled page"`) + "\nA content resource.\n",
		"content/biscuit/koalas-march-chocolate/notes.th.md":                             fm(`title: "Thai content resource"`) + "\nNotes.\n",
		"content/biscuit/.gitkeep":                                                       "",
		"content/biscuit/hello-panda/index.en.md":                                        fm(`title: "Hello Panda"`, `draft: true`, `categories: biscuit`) + "\nDraft.\n",
		"content/biscuit/hello-panda/index.th.md":                                        fm(`description: "no title"`) + "\nไม่มีชื่อเรื่อง\n",
		"content/biscuit/hello-panda/panda.png":                                          "png",
		"content/biscuit/future/index.md":                                                fm(`title: "Future"`, `publishDate: 2999-01-01T00:00:00Z`) + "\nLater.\n",
		"content/biscuit/expired/index.md":                                               fm(`title: "Expired"`, `expiryDate: 2000-01-01T00:00:00Z`) + "\nGone.\n",
		"content/potato-chips/herrs-salt-&-vinegar-potato-chips/index.en.md":             fm(`title: "Herr's Salt & Vinegar"`, `brands: ["Herr's"]`, `tags: ["Lay's", "Lays"]`) + "\nChips.\n",
		"content/potato-chips/wise-chili-olé-chili-&-spice-flavor-potato-chips/index.md": fm(`title: "Wise Chili Olé"`) + "\nOlé.\n",
		"content/potato-chips/wise-chili-olé-chili-&-spice-flavor-potato-chips/olé.jpg":  "jpeg",
		"content/seafood/squid/index.md":                                                 fm(`title: "Squid"`, `Description: "Upper case key"`, `Rating_smell: 2.5`) + "\nSquid.\n",
		"content/companies/Berli-Jucker-Foods-Ltd.Berli-Jucker-PLC/_index.en.md":         fm(`title: "Berli Jucker"`, `website: "https://example.com"`) + "\nCompany.\n",
		"content/companies/Berli-Jucker-Foods-Ltd.Berli-Jucker-PLC/logo.png":             "png",
		"content/companies/frito-lay/_index.en.md":                                       fm(`title: "Frito Lay"`) + "\n",
		"content/companies/frito-lay/_index.th.md":                                       fm(`references: ["a", "b"]`) + "\n",
		"content/brands/le-pan/_index.md":                                                fm(`title: "Le Pan"`) + "\n",
		"content/ingredients/ins-124/_index.en.md":                                       fm(`title: "INS 124"`) + "\n",
		"content/ingredients/ins-124/_index.th.md":                                       fm(`title: "INS 124 ไทย"`) + "\n",
		"content/ingredients/ins-124/ins124.jpg":                                         "jpeg",
		"content/tags/_index.md":                                                         fm(`title: "All Tags"`, `cascade: {params: {fromtags: true}}`) + "\n",
		"content/ขนม/_index.th.md":                                                       fm(`title: "ขนมไทย"`) + "\n",
		"content/ขนม/ข้าวเกรียบ/index.th.md":                                             fm(`title: "ข้าวเกรียบ"`, `tags: ["ขนม"]`) + "\nข้าวเกรียบ {{% box %}}**ไทย**{{% /box %}}\n",
		"content/ขนม/ข้าวเกรียบ/รูป.jpg":                                                 "jpeg",
		"content/blog/_index.md":                                                         fm(`title: "Blog"`, "cascade:", "  - params:", "      banner: blog.jpg", "    _target:", "      kind: page", "      path: /blog/**", "  - draft: false", "    _target:", "      lang: th") + "\n",
		"content/blog/nested/_index.md":                                                  fm(`title: "Nested section"`) + "\n",
		"content/blog/nested/deeper/_index.md":                                           fm(`title: "Deeper"`) + "\n",
		"content/blog/nested/deeper/post.md":                                             fm(`title: "Deep post"`, `slug: deep`) + seeksnackBody,
		"content/blog/nested/deeper/post.th.md":                                          fm(`title: "โพสต์"`) + "\nไทย\n",
		"content/blog/nested/leaf/index.md":                                              fm(`title: "Leaf in nested"`) + "\nLeaf.\n",
		"content/blog/nested/leaf/data.json":                                             `{"a": 1}`,
		"content/blog/branch-res/_index.md":                                              fm(`title: "Branch with resources"`) + "\n",
		"content/blog/branch-res/file.pdf":                                               "pdf",
		"content/blog/branch-res/child.md":                                               fm(`title: "Child"`, `build: {list: never, render: always}`) + "\nChild.\n",
		"content/blog/branch-res/local.md":                                               fm(`title: "Local"`, `build: {list: local, render: link, publishResources: false}`) + "\nLocal.\n",
		"content/blog/headless/index.md":                                                 fm(`title: "Headless"`, `headless: true`) + "\nHeadless.\n",
		"content/blog/headless/h.jpg":                                                    "jpeg",
		"content/blog/headless-build/index.md":                                           fm(`title: "Headless build"`, `build: {render: never, list: never}`) + "\nHeadless.\n",
		"content/blog/summary.md":                                                        fm(`title: "Summary"`, `summary: "FM summary"`) + "\nIntro.\n\n<!--more-->\n\nRest.\n",
		"content/blog/summary-lead.md":                                                   fm(`title: "Lead"`) + "<!--more-->\nAfter divider only.\n",
		"content/blog/html-page.html":                                                    fm(`title: "HTML"`) + "<p>HTML <!--more--> page</p>\n",
		"content/blog/json.md":                                                           "{\n\"title\": \"JSON\",\n\"tags\": [\"json\"]\n}\nJSON front matter.\n",
		"content/blog/nofm.md":                                                           "No front matter {{< badge \"x\" >}}.\n",
		"content/blog/nullfm.md":                                                         "---\n---\nEmpty YAML.\n",
		"content/blog/path-override.md":                                                  fm(`title: "Path override"`, `path: /custom/moved`) + "\nMoved.\n",
		"content/blog/lang-override.md":                                                  fm(`title: "Lang override"`, `lang: TH`) + "\nThai by front matter.\n",
		"content/blog/lang-disabled.md":                                                  fm(`title: "Lang disabled"`, `lang: fr`) + "\nFrench.\n",
		"content/blog/disabled.fr.md":                                                    fm(`title: "French"`) + "\n",
		"content/blog/kind-override.md":                                                  fm(`title: "Kind override"`, `kind: section`) + "\n",
		"content/blog/#ignored.md":                                                       fm(`title: "Ignored"`) + "\n",
		"content/blog/backup.md~":                                                        "backup",
	}
	for k, v := range Shortcodes() {
		c[k] = v
	}
	return Site{Name: "seeksnack", TOML: string(toml), Files: inline(c)}, nil
}

// EdgeSites are small sites stressing the page tree: duplicate paths
// (page vs bundle, branch vs page), taxonomy prefix matches at the
// character level, translations by content dir, inline shortcodes, a home
// leaf bundle, disabled kinds.
func EdgeSites() []Site {
	tree := map[string]string{
		"content/foo.md":                  fm(`title: "foo page"`) + "\n",
		"content/foo/index.md":            fm(`title: "foo bundle"`) + "\n",
		"content/bar/_index.md":           fm(`title: "bar section"`) + "\n",
		"content/bar.md":                  fm(`title: "bar page"`) + "\n",
		"content/bar/a.md":                fm(`title: "bar a"`) + "\n",
		"content/bar/a/index.md":          fm(`title: "bar a bundle"`) + "\n",
		"content/bar/a/r.txt":             "r",
		"content/bar/a.txt":               "resource at section level",
		"content/tagsfoo/_index.md":       fm(`title: "not a term?"`) + "\n",
		"content/tagsfoo/p.md":            fm(`title: "under tagsfoo"`) + "\n",
		"content/tags/_index.md":          fm(`title: "Tags"`) + "\n",
		"content/tags/Blue Sky/_index.md": fm(`title: "Blue sky term"`) + "\n",
		"content/tags/a/b/_index.md":      fm(`title: "Nested term"`) + "\n",
		"content/tag/_index.md":           fm(`title: "singular dir"`) + "\n",
		"content/categories/x/_index.md":  fm(`title: "cat x"`) + "\n",
		"content/UPPER/Mixed Case.md":     fm(`title: "Mixed"`) + "\n",
		"content/sp ace/x y.md":           fm(`title: "spaces"`) + "\n",
		"content/a&b/c'd.md":              fm(`title: "punct"`) + "\n",
		"content/dots/v1.2.3.md":          fm(`title: "dots"`) + "\n",
		"content/dots/x.en.md":            fm(`title: "x en"`) + "\n",
		"content/dots/x.md":               fm(`title: "x default"`) + "\n",
		"content/bom.md":                  string([]byte{0xEF, 0xBB, 0xBF}) + fm(`title: "BOM"`) + "\n",
		"content/empty.md":                "",
		"content/only-divider.md":         "<!--more-->",
		"content/sc/inline.md":            fm(`title: "inline"`) + "\n{{< hello.inline >}}Hi {{ .Page.Title }}{{< /hello.inline >}} {{< hello.inline />}}\n{{< badge 1 2 3 >}}\n",
		"layouts/_shortcodes/badge.html":  `{{ .Get 0 }}`,
	}
	contentDir := map[string]string{
		"content/_index.md":          fm(`title: "Home en"`) + "\n",
		"content/s/_index.md":        fm(`title: "S en"`) + "\n",
		"content/s/p.md":             fm(`title: "P en"`) + "\n",
		"content/s/b/index.md":       fm(`title: "B en"`) + "\n",
		"content/s/b/img.jpg":        "jpeg en",
		"content_th/_index.md":       fm(`title: "Home th"`) + "\n",
		"content_th/s/p.md":          fm(`title: "P th"`) + "\n",
		"content_th/s/b/index.md":    fm(`title: "B th"`) + "\n",
		"content_th/s/b/img.jpg":     "jpeg th",
		"content_th/s/b/only-th.jpg": "jpeg th only",
		"content_th/s/p.en.md":       fm(`title: "en file in th dir"`) + "\n",
		"content_th/ไทย/หน้า.md":     fm(`title: "Thai path"`) + "\n{{< inl.inline \"a\" b >}}{{ .Get 0 }}{{< /inl.inline >}}\n",
		"content/s/sc.md":            fm(`title: "sc"`) + "\n{{< n >}}{{< n >}}{{< n />}}{{< /n >}}{{< /n >}}\n",
		"layouts/_shortcodes/n.html": `{{ .Inner }}`,
	}
	// Go's TestExtractShortcodes inputs (hugolib/shortcode_test.go), one page each.
	scTemplates := map[string]string{
		"layouts/_shortcodes/tag.html":       `tag`,
		"layouts/_shortcodes/legacytag.html": `{{ $_hugo_config := "{ \"version\": 1 }" }}tag`,
		"layouts/_shortcodes/sc1.html":       `sc1`,
		"layouts/_shortcodes/sc2.html":       `sc2`,
		"layouts/_shortcodes/inner.html":     `{{with .Inner }}{{ . }}{{ end }}`,
		"layouts/_shortcodes/inner2.html":    `{{.Inner}}`,
		"layouts/_shortcodes/inner3.html":    `{{.Inner}}`,
	}
	scInputs := []string{
		"{{< tag >}}",
		"{{% tag %}}",
		"{{% legacytag %}}",
		"{{% inner %}}{{< tag >}}{{% /inner %}}",
		"{{< inner >}}{{% tag %}}{{< /inner >}}",
		"{{% tag param1 %}}",
		"{{< tag param1 param2>}}",
		`{{% tag param1="value" %}}`,
		`{{< tag param1="value1" param2="value2" >}}`,
		`{{< inner >}}Inner Content{{< / inner >}}`,
		`{{< inner />}}`,
		`{{< inner >}}Inner Content->{{% inner2 param1 %}}inner2txt{{% /inner2 %}}Inner close->{{< / inner >}}`,
		`{{< inner >}}inner2->{{% inner2 param1 %}}inner2txt->inner3{{< inner3>}}inner3txt{{</ inner3 >}}{{% /inner2 %}}final close->{{< / inner >}}`,
		`{{< inner param1 >}}{{< / inner >}}`,
		`{{< my.inline >}}Hi{{< /my.inline >}}`,
		"text {{< tag \"quoted \\\"escaped\\\" value\" `raw` 1 -2 3.5 true false 0x10 1e3 >}} after",
		"{{< tag a=1 b=\"x\" c=true d=1.5 >}}\n\n  {{< tag >}}\n\t{{% tag %}}",
		"{{< tag >}}{{< tag >}}{{< sc1 >}}{{< sc2 >}}",
		"before <!--more--> after {{< tag >}}",
		"{{</* tag */>}} {{%/* tag */%}}",
	}
	scFiles := map[string]string{}
	for k, v := range scTemplates {
		scFiles[k] = v
	}
	for i, in := range scInputs {
		scFiles[fmt.Sprintf("content/p%02d.md", i)] = fm(fmt.Sprintf(`title: "p%02d"`, i)) + in + "\n"
	}
	errSite := func(name, content string) Site {
		files := map[string]string{"content/p.md": fm(`title: "p"`) + content + "\n"}
		for k, v := range scTemplates {
			files[k] = v
		}
		return Site{Name: name, TOML: "baseURL = \"https://example.org/\"\n", Files: inline(files)}
	}

	return []Site{
		{Name: "shortcodes", TOML: "baseURL = \"https://example.org/\"\n[security]\nenableInlineShortcodes = true\n", Files: inline(scFiles)},
		errSite("sc-err-notfound", "{{< nosuch >}}"),
		errSite("sc-err-closing", "{{< tag >}}x{{< /tag >}}"),
		errSite("sc-err-unclosed", "{{< inner >}}never closed"),
		errSite("sc-err-noname", "{{< >}}"),
		errSite("sc-err-mixed-params", `{{< tag a="1" b >}}`),
		{Name: "edge-tree", TOML: `baseURL = "https://example.org/"
title = "Edge"
printPathWarnings = true
[taxonomies]
tag = "tags"
category = "categories"
[security]
enableInlineShortcodes = true
`, Files: inline(tree)},
		{Name: "contentdir", TOML: `baseURL = "https://example.org/"
title = "ContentDir"
defaultContentLanguage = "th"
[security]
enableInlineShortcodes = true
[languages.en]
weight = 2
contentDir = "content"
[languages.th]
weight = 1
contentDir = "content_th"
`, Files: inline(contentDir)},
		{Name: "homeleaf", TOML: `baseURL = "https://example.org/"
title = "HomeLeaf"
`, Files: inline(map[string]string{
			"content/index.md":     "---\ntitle: Home leaf bundle\n---\nHome.\n",
			"content/home.jpg":     "jpeg",
			"content/sub/page.md":  fm(`title: "resource page"`) + "\n",
			"content/sub/index.md": fm(`title: "index in subdir"`) + "\n",
		})},
		{Name: "nokinds", TOML: `baseURL = "https://example.org/"
title = "NoKinds"
disableKinds = ["page", "taxonomy"]
[taxonomies]
tag = "tags"
`, Files: inline(map[string]string{
			"content/_index.md":        fm(`title: "Home"`) + "\n",
			"content/s/_index.md":      fm(`title: "S"`) + "\n",
			"content/s/p.md":           fm(`title: "P"`, `tags: [a]`) + "\n",
			"content/s/b/index.md":     fm(`title: "B"`) + "\n",
			"content/s/b/r.txt":        "r",
			"content/tags/_index.md":   fm(`title: "Tags"`) + "\n",
			"content/tags/a/_index.md": fm(`title: "A"`) + "\n",
		})},
	}
}
