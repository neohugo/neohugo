// Command menus is the Go oracle for the navigation package
// (navigation/menu.go, menu_cache.go, pagemenus.go) in crates/nh-page (Wave B
// task T12).
//
//	go run ./tools/go-oracle/nh-page/menus [-root .] [-out rust/testdata/oracle/page/menus]
//
// The sites are the ones of hugolib/menu_test.go (section pages menus, front
// matter menus as a name, a list and a map, multiple output formats, children
// sorted by date, empty params, params from config and front matter, shadowed
// members with pageRef and url, HasMenuCurrent of sections, pre/post, a
// baseURL with a path, name and title fallbacks), this repository's docs/
// site (its global menu) and a site with nested menus, equal weights, zero
// and negative weights, identifiers, Thai names and two languages.
//
// Recorded per site and language:
//
//   - navigation.DecodeConfig of the language's menus config (its source
//     structure), with the source hash, and of adversarial inputs;
//   - the assembled Site.Menus trees (every entry: MenuConfig fields, Menu,
//     ConfiguredURL, page, URL, KeyName, HasChildren, children);
//   - PageMenusFromPage of every page's front matter `menus`/`menu` value;
//   - IsMenuCurrent and HasMenuCurrent of every page for every entry of
//     every menu (the page menus and the site menus are the recorded
//     entries, as hugolib shares them);
//   - Menu.Sort, ByWeight, ByName, Reverse, Limit of every menu and of
//     shuffled copies.
//
// Output: <site>.json.gz per site and decode.json.gz. Nothing depends on the
// platform.
package main

import (
	"bytes"
	"flag"
	"fmt"
	"log"
	"path/filepath"
	"sort"
	"strings"

	"github.com/neohugo/neohugo/navigation"
	"github.com/neohugo/neohugo/parser/pageparser"
	"github.com/neohugo/neohugo/resources/page"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-page/csupport"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-page/psupport"
)

func fm(lines ...string) string {
	return "---\n" + strings.Join(lines, "\n") + "\n---\n"
}

func site(name, toml string, files map[string]string) psupport.Site {
	if files == nil {
		files = map[string]string{}
	}
	for k, v := range psupport.Layouts() {
		if _, ok := files[k]; !ok {
			files[k] = v
		}
	}
	return psupport.Site{Name: name, TOML: toml, Files: files}
}

func menuPage(title string, weight int, menu, mtitle string, mweight int) string {
	return fmt.Sprintf("---\ntitle: %q\nweight: %d\nmenu:\n  %s:\n    title: %s\n    weight: %d\n---\n# Doc Menu\n", title, weight, menu, mtitle, mweight)
}

func testPage(title, date string, weight int) string {
	return fmt.Sprintf("---\ntitle: \"%s\"\npublishdate: \"%s\"\nweight: %d\n---\n# Doc %s\n", title, date, weight, title)
}

// sites ports the hugolib/menu_test.go sites and adds a nested one.
func sites(root string) ([]psupport.Site, error) {
	var out []psupport.Site
	out = append(out, site("section-pages-menu", `baseURL = "http://example.com/"
title = "Section Menu"
sectionPagesMenu = "sect"
`, map[string]string{
		"content/sect1/p1.md":     menuPage("p1", 1, "main", "atitle1", 40),
		"content/sect1/p2.md":     menuPage("p2", 2, "main", "atitle2", 30),
		"content/sect2/p3.md":     menuPage("p3", 3, "main", "atitle3", 20),
		"content/sect2/p4.md":     menuPage("p4", 4, "main", "atitle4", 10),
		"content/sect3/p5.md":     menuPage("p5", 5, "main", "atitle5", 5),
		"content/sect1/_index.md": testPage("Section One", "2017-01-01", 100),
		"content/sect5/_index.md": testPage("Section Five", "2017-01-01", 10),
	}))
	out = append(out, site("front-matter", `baseURL = "https://example.org/"
`, map[string]string{
		"content/blog/page1.md": fm(`title: "P1"`, `menu: main`),
		"content/blog/page2.md": fm(`title: "P2"`, `menu: [main,other]`),
		"content/blog/page3.md": fm(`title: "P3"`, "menu:\n  main:\n    weight: 30"),
		"content/blog/page4.md": fm(`title: "P4"`, "menus:\n  main:\n    weight: 30\n    identifier: p4\n    pre: \"<i>\"\n    post: \"</i>\"\n  other:\n  third:\n    name: Third\n    parent: nope"),
		"content/blog/page5.md": fm(`title: "P5"`, `menu: 42`),
		"content/blog/page6.md": fm(`title: "P6"`, "menus:\n  main: [1, 2]"),
	}))
	out = append(out, site("multiple-output-formats", `baseURL = "https://example.com"
[outputFormats]
[outputFormats.damp]
mediaType = "text/html"
path = "damp"
`, map[string]string{
		"content/_index.md":        fm(`Title: Home Sweet Home`, `outputs: [ "html", "amp" ]`, `menu: "main"`),
		"content/blog/html-amp.md": fm(`Title: AMP and HTML`, `outputs: [ "html", "amp" ]`, `menu: "main"`),
		"content/blog/html.md":     fm(`Title: HTML only`, `outputs: [ "html" ]`, `menu: "main"`),
		"content/blog/amp.md":      fm(`Title: AMP only`, `outputs: [ "amp" ]`, `menu: "main"`),
	}))
	out = append(out, site("sort-by-date", `baseURL = "https://example.org/"
`, map[string]string{
		"content/blog/a.md": fm(`Title: A`, `date: 2019-01-01`, "menu:\n  main:\n    identifier: \"a\"\n    weight: 1"),
		"content/blog/b.md": fm(`Title: B`, `date: 2018-01-02`, "menu:\n  main:\n    parent: \"a\"\n    weight: 100"),
		"content/blog/c.md": fm(`Title: C`, `date: 2019-01-03`, "menu:\n  main:\n    parent: \"a\"\n    weight: 10"),
	}))
	out = append(out, site("params", `baseURL = "https://example.org/"
[[menus.main]]
identifier = "contact"
title = "Contact Us"
url = "mailto:noreply@example.com"
weight = 300
[menus.main.params]
foo = "foo_config"
key2 = "key2_config"
camelCase = "camelCase_config"
`, map[string]string{
		"content/_index.md": fm(`title: "Home"`, "menu:\n  main:\n    weight: 10\n    params:\n      foo: \"foo_content\"\n      key2: \"key2_content\"\n      camelCase: \"camelCase_content\""),
		"content/p1.md":     fm("menus:\n  main:\n    identity: journal\n    weight: 2\n    params:"),
	}))
	out = append(out, site("shadow-members", `baseURL = "https://example.org/"
[[menus.main]]
identifier = "contact"
pageRef = "contact"
title = "Contact Us"
url = "mailto:noreply@example.com"
weight = 1
[[menus.main]]
pageRef = "/blog/post3"
title = "My Post 3"
url = "/blog/post3"
`, map[string]string{
		"content/_index.md":      fm(`title: "Home"`, "menu:\n  main:\n    weight: 10"),
		"content/blog/_index.md": fm(`title: "Blog"`, "menu:\n  main:\n    weight: 20"),
		"content/blog/post1.md":  fm(`title: "My Post 1: With  No Menu Defined"`),
		"content/blog/post2.md":  fm(`title: "My Post 2: With Menu Defined"`, "menu:\n  main:\n    weight: 30"),
		"content/blog/post3.md":  fm(`title: "My Post 2: With  No Menu Defined"`),
		"content/contact.md":     fm(`title: "Contact: With  No Menu Defined"`),
	}))
	out = append(out, site("has-menu-current-section", `baseURL = "https://example.org/"
disableKinds = ['RSS','sitemap','taxonomy','term']
[[menu.main]]
name = 'Home'
pageRef = '/'
weight = 1
[[menu.main]]
name = 'Tests'
pageRef = '/tests'
weight = 2
[[menu.main]]
name = 'Test 1'
pageRef = '/tests/test-1'
parent = 'Tests'
weight = 1
`, map[string]string{
		"content/tests/test-1.md": fm(`title: "Test 1"`),
	}))
	out = append(out, site("pre-post-subdir", `baseURL = "https://example.com/foo/"
title = "Hugo Menu Test"
[menus]
[[menus.main]]
name = "Home"
url = "/"
pre = "<span>"
post = "</span>"
weight = 1
[[menus.main]]
name = "Posts"
url = "/posts"
weight = 1
`, nil))
	out = append(out, site("section-pages-multilingual", `baseURL = "https://example.org/"
disableKinds = ['section','rss','sitemap','taxonomy','term']
defaultContentLanguageInSubdir = true
sectionPagesMenu = "main"
[languages.en]
[languages.fr]
`, map[string]string{
		"content/p1.en.md": fm(`title: p1`, `menu: main`),
		"content/p1.fr.md": fm(`title: p1`, `menu: main`),
		"content/p2.en.md": fm(`title: p2`, `menu: main`),
	}))
	out = append(out, site("section-pages-12399", `baseURL = "https://example.org/"
disableKinds = ['rss','sitemap','taxonomy','term']
capitalizeListTitles = false
pluralizeListTitles = false
sectionPagesMenu = 'main'
`, map[string]string{
		"content/p1.md":    fm(`title: p1`),
		"content/s1/p2.md": fm(`title: p2`, `menus: main`),
		"content/s1/p3.md": fm(`title: p3`),
	}))
	out = append(out, site("name-title-fallback", `baseURL = "https://example.org/"
disableKinds = ['rss','sitemap','taxonomy','term']
[[menus.main]]
name = 'P1_ME_Name'
title = 'P1_ME_Title'
pageRef = '/p1'
weight = 10
[[menus.main]]
pageRef = '/p2'
weight = 20
[[menus.main]]
pageRef = '/p3'
weight = 30
[[menus.main]]
name = 'S1_ME_Name'
title = 'S1_ME_Title'
pageRef = '/s1'
weight = 40
[[menus.main]]
pageRef = '/s2'
weight = 50
[[menus.main]]
pageRef = '/s3'
weight = 60
`, map[string]string{
		"content/p1.md":          fm(`title: P1_Title`),
		"content/p2.md":          fm(`title: P2_Title`),
		"content/p3.md":          fm(`title: P3_Title`, `linkTitle: P3_LinkTitle`),
		"content/s1/_index.md":   fm(`title: S1_Title`),
		"content/s2/_index.md":   fm(`title: S2_Title`),
		"content/s3/_index.md":   fm(`title: S3_Title`, `linkTitle: S3_LinkTitle`),
		"content/s3/child.md":    fm(`title: S3 Child`, "menu:\n  main:\n    parent: S3_LinkTitle\n    weight: 1"),
		"content/s3/child2.md":   fm(`title: S3 Child 2`, "menu:\n  main:\n    parent: S3_LinkTitle"),
		"content/s1/nested.md":   fm(`title: Nested`, "menu:\n  main:\n    parent: S1_ME_Name\n    identifier: nested"),
		"content/s1/deeper.md":   fm(`title: Deeper`, "menu:\n  main:\n    parent: nested\n    weight: -5"),
		"content/s1/deeper-2.md": fm(`title: Deeper 2`, "menu:\n  main:\n    parent: nested\n    weight: -5"),
	}))
	nested := map[string]string{
		"content/_index.md":          fm(`title: Home`),
		"content/_index.th.md":       fm(`title: หน้าแรก`),
		"content/docs/_index.md":     fm(`title: Docs`, "menu:\n  main:\n    identifier: docs\n    weight: 5"),
		"content/docs/a.md":          fm(`title: Alpha`, "menu:\n  main:\n    parent: docs"),
		"content/docs/b.md":          fm(`title: Beta`, "menu:\n  main:\n    parent: docs\n    weight: 0"),
		"content/docs/c.md":          fm(`title: alpha`, "menu:\n  main:\n    parent: docs\n    name: Alpha\n    identifier: c"),
		"content/docs/d.md":          fm(`title: Delta`, "menu:\n  main:\n    parent: docs\n    weight: 2"),
		"content/docs/e.md":          fm(`title: Echo`, "menu:\n  main:\n    parent: docs\n    weight: 2\n    name: Delta\n    identifier: a-e"),
		"content/docs/sub/_index.md": fm(`title: Sub`, "menu:\n  main:\n    parent: docs\n    identifier: sub\n    weight: -1"),
		"content/docs/sub/x.md":      fm(`title: X`, "menu:\n  main:\n    parent: sub"),
		"content/docs/sub/y.md":      fm(`title: ขนม`, "menu:\n  main:\n    parent: sub"),
		"content/docs/sub/z.md":      fm(`title: ข้าว`, "menu:\n  [main, footer]"),
		"content/docs/a.th.md":       fm(`title: ก`, "menu:\n  main:\n    parent: docs\n    weight: 3"),
		"content/blog/_index.md":     fm(`title: Blog`, "menu: main"),
		"content/blog/p.md":          fm(`title: Post`, "menu:\n  footer:\n    weight: 7\n    pre: <b>\n    params:\n      Icon: star\n      nested: {A: 1}"),
	}
	out = append(out, site("nested", `baseURL = "https://example.org/"
defaultContentLanguage = "en"
[languages.en]
weight = 1
[languages.th]
weight = 2
[[languages.th.menus.main]]
name = "ภายนอก"
url = "https://example.com/th"
weight = 1
[[menus.main]]
name = "External"
url = "https://example.com/"
weight = 100
[[menus.main]]
name = "Docs Home"
pageRef = "/docs"
parent = "docs"
weight = -3
[[menus.footer]]
name = "Zero"
url = "/zero/"
[[menus.footer]]
name = "zero"
url = "/zero2/"
[[menus.footer]]
name = "Neg"
url = "/neg/"
weight = -1
identifier = "neg"
`, nested))
	repo, err := psupport.RepoSites(root)
	if err != nil {
		return nil, err
	}
	for _, s := range repo {
		if s.Name == "docs" {
			out = append(out, s)
		}
	}
	return out, nil
}

// Adversarial DecodeConfig inputs.
var decodeInputs = []any{
	nil,
	map[string]any{},
	map[string]any{"main": []any{
		map[string]any{"name": "A", "weight": "10", "pre": 3, "post": true, "url": "/a/", "params": map[string]any{"Key": "v", "Nested": map[string]any{"X": 1}}},
		map[string]any{"name": "B", "weight": 10, "identifier": "b"},
		map[string]any{"name": "B", "weight": 10, "identifier": "a"},
		map[string]any{"Name": "C", "Weight": 0},
		map[string]any{"name": "D", "weight": -2, "pageRef": "/d", "Title": "T", "parent": "A"},
	}},
	map[string]any{"Main": []any{map[string]any{"name": "x"}}, "main": []any{map[string]any{"name": "y"}}},
	map[string]any{"main": map[string]any{"name": "not a list"}},
	map[string]any{"main": []any{"not a map"}},
	map[string]any{"main": []any{map[string]any{"weight": "heavy"}}},
	"not a map",
	map[string]any{"main": []any{}},
}

func main() {
	root := flag.String("root", ".", "neohugo module root")
	out := flag.String("out", "rust/testdata/oracle/page/menus", "output directory")
	flag.Parse()
	outDir := psupport.OutDir(*root, *out)

	var dc []map[string]any
	for _, in := range decodeInputs {
		dc = append(dc, map[string]any{"in": goval.Encode(in), "res": decodeRes(in)})
	}
	if err := goval.WriteCasesGz(filepath.Join(outDir, "decode.json.gz"), map[string]any{}, dc); err != nil {
		log.Fatal(err)
	}

	ss, err := sites(*root)
	if err != nil {
		log.Fatal(err)
	}
	total := 0
	for _, s := range ss {
		n, err := runSite(s, outDir)
		if err != nil {
			log.Fatal(err)
		}
		total += n
	}
	log.Printf("menus: %d cases, %d decode cases", total, len(dc))
}

func entryDump(me *navigation.MenuEntry) map[string]any {
	params := goval.Encode(me.Params)
	if me.Params == nil {
		params = map[string]any{"t": "nil:maps.Params"}
	}
	return map[string]any{
		"identifier": goval.Str(me.Identifier), "parent": goval.Str(me.Parent), "name": goval.Str(me.Name),
		"pre": goval.Str(string(me.Pre)), "post": goval.Str(string(me.Post)), "url": goval.Str(me.MenuConfig.URL),
		"pageRef": goval.Str(me.PageRef), "weight": me.Weight, "title": goval.Str(me.Title), "params": params,
		"menu": goval.Str(me.Menu), "configuredURL": goval.Str(me.ConfiguredURL),
	}
}

func decodeRes(in any) any {
	return goval.CallRaw(func() (any, error) {
		ns, err := navigation.DecodeConfig(in)
		if err != nil {
			return nil, err
		}
		menus := map[string]any{}
		for name, m := range ns.Config {
			var es []any
			for _, me := range m {
				es = append(es, entryDump(me))
			}
			menus[name] = es
		}
		return map[string]any{"menus": menus, "hash": ns.SourceHash, "source": goval.Encode(ns.SourceStructure)}, nil
	})
}

type runner struct {
	t       *csupport.Table
	entries []map[string]any
	index   map[*navigation.MenuEntry]int
}

// entry records me (children first) and returns its index.
func (r *runner) entry(me *navigation.MenuEntry) int {
	if i, ok := r.index[me]; ok {
		return i
	}
	var children []any
	for _, c := range me.Children {
		children = append(children, r.entry(c))
	}
	d := entryDump(me)
	d["children"] = children
	d["hasChildren"] = me.HasChildren()
	d["keyName"] = goval.Str(me.KeyName())
	d["urlResult"] = goval.Str(me.URL())
	d["page"] = -1
	if p, ok := me.Page.(page.Page); ok && p != nil {
		d["page"] = r.t.Add(p)
	}
	r.index[me] = len(r.entries)
	r.entries = append(r.entries, d)
	return len(r.entries) - 1
}

func (r *runner) menu(m navigation.Menu) any {
	if m == nil {
		return nil
	}
	out := []any{}
	for _, me := range m {
		out = append(out, r.entry(me))
	}
	return out
}

func walk(m navigation.Menu, f func(*navigation.MenuEntry)) {
	for _, me := range m {
		f(me)
		walk(me.Children, f)
	}
}

func frontMatterMenus(s psupport.Site, p page.Page) (any, bool) {
	if p.File() == nil {
		return nil, false
	}
	fn := filepath.ToSlash(p.File().Filename())
	rel := strings.TrimPrefix(fn, psupport.SiteDir(s.Name)+"/")
	content, ok := s.Files[rel]
	if !ok {
		return nil, false
	}
	cf, err := pageparser.ParseFrontMatterAndContent(bytes.NewReader([]byte(content)))
	if err != nil {
		return nil, false
	}
	var menus any
	for _, key := range []string{"menus", "menu"} {
		for k, v := range cf.FrontMatter {
			if strings.ToLower(k) == key {
				menus = v
			}
		}
		if menus != nil {
			break
		}
	}
	return menus, true
}

func runSite(s psupport.Site, outDir string) (int, error) {
	b, err := psupport.Build(s)
	if err != nil {
		return 0, err
	}
	h := b.H
	t := csupport.NewTable(h)
	r := &runner{t: t, index: map[*navigation.MenuEntry]int{}}
	var cases []map[string]any

	var siteMenus []any
	for si, st := range h.Sites {
		lang := st.Language().Lang
		cfg := h.Configs.LanguageConfigMap[lang].Menus
		cases = append(cases, map[string]any{"op": "decode", "site": si, "in": goval.Encode(cfg.SourceStructure), "res": decodeRes(cfg.SourceStructure)})

		menus := st.Menus()
		var names []string
		for k := range menus {
			names = append(names, k)
		}
		sort.Strings(names)
		sm := map[string]any{}
		for _, name := range names {
			sm[name] = r.menu(menus[name])
		}
		siteMenus = append(siteMenus, sm)

		pages := st.Pages()
		if s.Name == "docs" {
			// Every node and every 25th page.
			var sel page.Pages
			for i, p := range pages {
				if p.IsNode() || i%25 == 0 {
					sel = append(sel, p)
				}
			}
			pages = sel
		}
		for _, p := range pages {
			pi := t.Add(p)
			// Page menus from the front matter.
			if ms, ok := frontMatterMenus(s, p); ok && ms != nil {
				c := map[string]any{"op": "PageMenusFromPage", "site": si, "page": pi, "in": goval.Encode(ms)}
				c["res"] = goval.CallRaw(func() (any, error) {
					pm, err := navigation.PageMenusFromPage(ms, p)
					if err != nil {
						return nil, err
					}
					out := map[string]any{}
					for k, me := range pm {
						d := entryDump(me)
						d["page"] = t.Add(me.Page.(page.Page))
						out[k] = d
					}
					return out, nil
				})
				cases = append(cases, c)
			}
			pm := map[string]any{}
			pms := p.Menus()
			var pmKeys []string
			for k := range pms {
				pmKeys = append(pmKeys, k)
			}
			sort.Strings(pmKeys)
			for _, k := range pmKeys {
				pm[k] = r.entry(pms[k])
			}
			var cur []any
			for _, name := range names {
				walk(menus[name], func(me *navigation.MenuEntry) {
					cur = append(cur, []any{name, r.entry(me), p.IsMenuCurrent(name, me), p.HasMenuCurrent(name, me)})
				})
			}
			cases = append(cases, map[string]any{"op": "current", "site": si, "page": pi, "pageMenus": pm, "res": cur})
		}

		// Sorting.
		for _, name := range names {
			var lists []navigation.Menu
			walk(navigation.Menu{&navigation.MenuEntry{Children: menus[name]}}, func(me *navigation.MenuEntry) {
				if len(me.Children) > 0 {
					lists = append(lists, me.Children)
				}
			})
			for li, m := range lists {
				for _, sh := range []uint64{0, 1, 2} {
					in := m.Clone()
					if sh > 0 {
						in = shuffle(in, sh)
					}
					c := map[string]any{"op": "sort", "site": si, "menu": name, "list": li, "in": r.menu(in)}
					sorted := in.Clone()
					sorted.Sort()
					c["sort"] = r.menu(sorted)
					c["byWeight"] = r.menu(in.ByWeight())
					c["byName"] = r.menu(in.ByName())
					c["reverse"] = r.menu(in.Reverse())
					c["limit2"] = r.menu(in.Limit(2))
					cases = append(cases, c)
				}
			}
		}
	}

	// IsAncestor answers for the menu entry pages (HasMenuCurrent asks them).
	all := append([]page.Page(nil), t.Pages()...)
	for _, e := range r.entries {
		pi := e["page"].(int)
		if pi < 0 {
			continue
		}
		p := all[pi]
		var anc []any
		for j, q := range all {
			if p.IsAncestor(q) {
				anc = append(anc, j)
			}
		}
		t.Entries[pi]["ancestorOf"] = anc
	}

	header := map[string]any{
		"site":      s.Name,
		"sites":     t.SitesHeader(),
		"pages":     t.Entries,
		"entries":   r.entries,
		"siteMenus": siteMenus,
	}
	log.Printf("%s: %d cases, %d menu entries", s.Name, len(cases), len(r.entries))
	return len(cases), goval.WriteCasesGz(filepath.Join(outDir, s.Name+".json.gz"), header, cases)
}

func shuffle(m navigation.Menu, seed uint64) navigation.Menu {
	out := m.Clone()
	s := seed*6364136223846793005 + 1442695040888963407
	for i := len(out) - 1; i > 0; i-- {
		s = s*6364136223846793005 + 1442695040888963407
		j := int((s >> 33) % uint64(i+1))
		out[i], out[j] = out[j], out[i]
	}
	return out
}
