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

// The sites of the site oracle: the 17 sites of the assemble oracle (read
// from its checked-in fixtures, so both oracles build the same trees), and
// the sites of this oracle: menus (config entries with pageRef, URLs,
// children, a missing parent, the section pages menu, front matter menus as
// a name, a list and a map, duplicates), pagination sizes and next/prev
// orders, refs across languages and output formats.

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

// AssembleFixtureSites reads the site descriptions of the assemble oracle's
// fixtures (crates/nh-hugolib/tests/fixtures/assemble/*.json.gz).
func AssembleFixtureSites(root string) ([]hsupport.Site, error) {
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

// SiteSites are the sites of this oracle.
func SiteSites() []hsupport.Site {
	menusTOML := `baseURL = "https://example.org/sub/"
title = "Menus"
sectionPagesMenu = "main"
disableKinds = ["rss", "sitemap", "robotsTXT"]
[params]
author = "A"
mainSections = ["blog"]
[pagination]
pagerSize = 2
[page]
nextPrevSortOrder = "asc"
nextPrevInSectionSortOrder = "desc"
[menus]
[[menus.main]]
name = "Home"
pageRef = "/"
weight = 1
[[menus.main]]
name = "About"
pageRef = "/about"
weight = 2
[[menus.main]]
name = "External"
url = "https://gohugo.io/"
weight = 3
pre = "<i>"
post = "</i>"
[[menus.main]]
name = "Local URL"
url = "/local/path/"
weight = 3
[[menus.main]]
identifier = "parent"
name = "Parent"
weight = 4
[[menus.main]]
name = "Child one"
parent = "parent"
pageRef = "/blog/b1"
weight = 1
[[menus.main]]
name = "Child two"
parent = "parent"
url = "/c2/"
weight = 1
[[menus.main]]
name = "Orphan"
parent = "missing"
url = "/orphan/"
[[menus.main]]
name = "Missing pageRef"
pageRef = "/does/not/exist"
url = "/fallback/"
[[menus.footer]]
name = "Blog"
pageRef = "/blog"
[menus.footer.params]
class = "x"
`
	menusFiles := map[string]string{
		"content/_index.md":              fm(`title: "Home"`),
		"content/about.md":               fm(`title: "About"`, `menu: footer`, `weight: 5`),
		"content/contact.md":             fm(`title: "Contact"`, `menus: ["main", "footer"]`, `linkTitle: "Reach"`),
		"content/blog/_index.md":         fm(`title: "Blog"`, `weight: 2`),
		"content/blog/b1.md":             fm(`title: "B1"`, `date: 2021-01-01`, `menus: {main: {parent: "parent", weight: 3, identifier: "b1id"}}`, `tags: [x, y]`),
		"content/blog/b2.md":             fm(`title: "B2"`, `date: 2021-01-02`, `menu: {footer: {name: "Custom B2", params: {k: v}}}`, `tags: [y]`),
		"content/blog/b3.md":             fm(`title: "B3"`, `date: 2021-01-03`, `menu: {main: {identifier: "parent"}}`),
		"content/blog/b4.md":             fm(`title: "B4"`, `date: 2021-01-03`, `weight: 1`),
		"content/blog/sub/_index.md":     fm(`title: "Sub"`),
		"content/blog/sub/s1.md":         fm(`title: "S1"`, `date: 2020-05-05`),
		"content/blog/sub/s2.md":         fm(`title: "S2"`, `date: 2020-05-06`),
		"content/docs/d1.md":             fm(`title: "D1"`, `menu: main`),
		"content/docs/d2.md":             fm(`title: "D2"`, `draft: true`),
		"content/news/n1.md":             fm(`title: "N1"`, `date: 2019-01-01`),
		"content/news/n2.md":             fm(`title: "N2"`, `date: 2019-01-02`),
		"content/news/n3.md":             fm(`title: "N3"`, `date: 2019-01-03`),
		"content/news/n4.md":             fm(`title: "N4"`, `date: 2019-01-04`),
		"content/news/n5.md":             fm(`title: "N5"`, `date: 2019-01-05`),
		"layouts/_shortcodes/badge.html": `x`,
	}

	refsTOML := `baseURL = "https://example.org/"
title = "Refs"
defaultContentLanguage = "en"
refLinksErrorLevel = "warning"
refLinksNotFoundURL = "/notfound/"
[languages.en]
weight = 1
title = "Refs EN"
[languages.th]
weight = 2
title = "Refs TH"
[languages.th.params]
greeting = "สวัสดี"
[params]
greeting = "Hello"
[params.nested]
Deep = "d"
[outputs]
home = ["html", "json", "rss"]
page = ["html", "json"]
`
	refsFiles := map[string]string{
		"content/_index.md":     fm(`title: "Home"`),
		"content/_index.th.md":  fm(`title: "หน้าแรก"`),
		"content/a.md":          fm(`title: "A"`, `translationKey: "ka"`) + "\n## Heading\n{{< ref \"b.md\" >}}\n",
		"content/a.th.md":       fm(`title: "เอ"`, `translationKey: "ka"`),
		"content/b.md":          fm(`title: "B"`, `aliases: ["/old-b/"]`, `keywords: [k1, k2]`),
		"content/c/index.md":    fm(`title: "C bundle"`, `resources: [{src: "*.txt", title: "Text :counter"}]`),
		"content/c/one.txt":     "one",
		"content/c/two.txt":     "two",
		"content/c/page.md":     fm(`title: "Bundled"`),
		"content/th-only.th.md": fm(`title: "ไทยเท่านั้น"`),
	}

	return []hsupport.Site{
		{Name: "site-menus", TOML: menusTOML, Files: inline(menusFiles)},
		{Name: "site-refs", TOML: refsTOML, Files: inline(refsFiles)},
	}
}
