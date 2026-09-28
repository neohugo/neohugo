package main

import (
	"io/fs"
	"os"
	"path/filepath"
	"sort"
	"strings"
)

// node is one entry of a recorded directory tree: a file ("f", with its
// content), a directory ("d") or a symlink ("l", with its target).
type node struct {
	P string `json:"p"`
	T string `json:"t"`
	C string `json:"c,omitempty"`
	L string `json:"l,omitempty"`
}

// site is one input tree with its configuration file.
type site struct {
	name string
	tree []node
	// Extra flags set before loading the config.
	flags map[string]any
}

func f(p, c string) node { return node{P: p, T: "f", C: c} }
func d(p string) node    { return node{P: p, T: "d"} }
func l(p, t string) node { return node{P: p, T: "l", L: t} }

// materialize writes the tree below root. Entries are created in sorted order
// so that directory order is reproducible for a given filesystem.
func materialize(root string, tree []node) error {
	sorted := append([]node(nil), tree...)
	sort.Slice(sorted, func(i, j int) bool { return sorted[i].P < sorted[j].P })
	for _, n := range sorted {
		p := filepath.Join(root, filepath.FromSlash(n.P))
		if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
			return err
		}
		switch n.T {
		case "d":
			if err := os.MkdirAll(p, 0o755); err != nil {
				return err
			}
		case "f":
			if err := os.WriteFile(p, []byte(n.C), 0o644); err != nil {
				return err
			}
		case "l":
			if err := os.Symlink(n.L, p); err != nil {
				return err
			}
		}
	}
	return nil
}

// configFiles are the files whose content is recorded for the repository
// sites (everything else is written empty: the filesystem layer never reads
// file contents).
var configFiles = map[string]bool{
	"hugo.toml": true,
	"go.mod":    true,
	"hugo.work": true,
}

// repoTree records the files, directories and symlinks below dir (skipping
// the named top-level entries).
func repoTree(dir string, skip ...string) ([]node, error) {
	var tree []node
	err := filepath.WalkDir(dir, func(p string, de fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		rel, err := filepath.Rel(dir, p)
		if err != nil {
			return err
		}
		if rel == "." {
			return nil
		}
		rel = filepath.ToSlash(rel)
		for _, s := range skip {
			if rel == s {
				if de.IsDir() {
					return filepath.SkipDir
				}
				return nil
			}
		}
		switch {
		case de.Type()&fs.ModeSymlink != 0:
			t, err := os.Readlink(p)
			if err != nil {
				return err
			}
			tree = append(tree, l(rel, t))
		case de.IsDir():
			tree = append(tree, d(rel))
		default:
			c := ""
			if configFiles[rel] {
				b, err := os.ReadFile(p)
				if err != nil {
					return err
				}
				c = string(b)
			}
			tree = append(tree, f(rel, c))
		}
		return nil
	})
	return tree, err
}

const nfdCafe = "café"

func multilangSite() site {
	return site{
		name: "multilang",
		tree: []node{
			f("hugo.toml", `baseURL = "https://example.org/"
defaultContentLanguage = "en"
disableLanguages = ["fr"]
[languages.en]
weight = 1
contentDir = "content/en"
[languages.th]
weight = 2
contentDir = "content/th"
[languages.fr]
weight = 3
contentDir = "content/fr"
`),
			f("content/en/_index.md", ""),
			f("content/en/posts/_index.md", ""),
			f("content/en/posts/hello.md", ""),
			f("content/en/posts/hello.th.md", ""),
			f("content/en/posts/x.fr.md", ""),
			f("content/en/posts/y.de.md", ""),
			f("content/en/posts/bundle/index.md", ""),
			f("content/en/posts/bundle/index.th.md", ""),
			f("content/en/posts/bundle/img.jpg", ""),
			f("content/en/posts/bundle/data.json", ""),
			f("content/en/posts/bundle/sub/deep.txt", ""),
			f("content/en/café.md", ""),
			f("content/en/"+nfdCafe+".md", ""),
			f("content/en/.gitkeep", ""),
			f("content/en/.hidden/secret.md", ""),
			f("content/en/#notes.md", ""),
			f("content/en/backup.md~", ""),
			d("content/en/empty"),
			l("content/en/link.md", "posts/hello.md"),
			l("content/en/linkdir", "posts"),
			l("content/en/dangling.md", "nowhere.md"),
			f("content/en/a.b.c.md", ""),
			f("content/en/UPPER.MD", ""),
			f("content/en/page.html", ""),
			f("content/en/page.md", ""),
			f("content/en/page.th.html", ""),
			f("content/en/_content.gotmpl", ""),
			f("content/en/noext", ""),
			f("content/th/_index.md", ""),
			f("content/th/บทความ/ไทย.md", ""),
			f("content/th/บทความ/_index.md", ""),
			f("content/th/posts/hello.md", ""),
			f("content/th/posts/hello.en.md", ""),
			f("content/th/posts/only-th.md", ""),
			f("content/fr/_index.md", ""),
			f("content/fr/posts/bonjour.md", ""),
			f("layouts/_default/baseof.html", ""),
			f("layouts/_default/single.html", ""),
			f("layouts/_default/list.th.html", ""),
			f("layouts/index.th.html", ""),
			f("layouts/index.html", ""),
			f("layouts/index.json", ""),
			f("layouts/partials/head.html", ""),
			f("layouts/shortcodes/x.html", ""),
			f("layouts/_markup/render-link.html", ""),
			f("i18n/en.toml", ""),
			f("i18n/th.toml", ""),
			f("i18n/fr.toml", ""),
			f("data/a.toml", ""),
			f("data/sub/b.json", ""),
			f("data/sub/c.th.yaml", ""),
			f("static/.DS_Store", ""),
			f("static/robots.bak", ""),
			f("static/img/a.png", ""),
			f("static/img/ภาพ.png", ""),
			f("static/"+nfdCafe+".txt", ""),
			d("static/emptydir"),
			l("static/link.txt", "../hugo.toml"),
			l("static/linkeddir", "img"),
			f("archetypes/default.md", ""),
			f("archetypes/posts.md", ""),
			f("assets/css/main.css", ""),
			f("assets/js/a.js", ""),
			f("assets/js/a.th.js", ""),
		},
	}
}

func mountsSite() site {
	return site{
		name: "mounts",
		tree: []node{
			f("hugo.toml", `baseURL = "https://example.org/"
theme = "mytheme"
defaultContentLanguage = "en"
[languages.en]
weight = 1
[languages.th]
weight = 2
[module]
[[module.mounts]]
source = "content"
target = "content"
[[module.mounts]]
source = "content2"
target = "content"
lang = "th"
includeFiles = ["**/*.md", "images/*"]
excludeFiles = ["drafts/**"]
[[module.mounts]]
source = "content3"
target = "content/extra"
excludeFiles = "*.txt"
[[module.mounts]]
source = "static"
target = "static"
[[module.mounts]]
source = "static2"
target = "static"
[[module.mounts]]
source = "assets2"
target = "static/sub"
excludeFiles = "*.tmp"
[[module.mounts]]
source = "staticth"
target = "static"
lang = "th"
[[module.mounts]]
source = "staticlink"
target = "static/linked"
[[module.mounts]]
source = "node_modules"
target = "assets/vendor"
[[module.mounts]]
source = "assets"
target = "assets"
[[module.mounts]]
source = "README.md"
target = "assets/docs/readme.md"
[[module.mounts]]
source = "layouts"
target = "layouts"
[[module.mounts]]
source = "layouts-extra"
target = "layouts/partials/extra"
[[module.mounts]]
source = "data"
target = "data"
[[module.mounts]]
source = "data2"
target = "data"
[[module.mounts]]
source = "i18n"
target = "i18n"
[[module.mounts]]
source = "archetypes"
target = "archetypes"
[[module.mounts]]
source = "missing-dir"
target = "assets/missing"
`),
			f("package.json", "{}"),
			f("postcss.config.js", ""),
			f("README.md", "readme"),
			f("content/_index.md", ""),
			f("content/post.md", ""),
			f("content/same.md", ""),
			f("content/blog/one.md", ""),
			f("content/blog/two.th.md", ""),
			f("content2/same.md", ""),
			f("content2/th-only.md", ""),
			f("content2/notes.txt", ""),
			f("content2/drafts/d1.md", ""),
			f("content2/blog/three.md", ""),
			f("content2/images/pic.png", ""),
			f("content2/images/nested/deep.png", ""),
			f("content3/x.md", ""),
			f("content3/y.txt", ""),
			f("content3/sub/z.md", ""),
			f("static/a.txt", ""),
			f("static/same.txt", ""),
			f("static/dir/s1.txt", ""),
			f("static2/b.txt", ""),
			f("static2/same.txt", ""),
			f("static2/dir/s2.txt", ""),
			f("assets2/c.css", ""),
			f("assets2/skip.tmp", ""),
			f("staticth/th.txt", ""),
			f("staticth/same.txt", ""),
			d("realstatic"),
			f("realstatic/r.txt", ""),
			l("staticlink", "realstatic"),
			f("node_modules/bootstrap/scss/bootstrap.scss", ""),
			f("node_modules/bootstrap/package.json", ""),
			f("node_modules/jquery/dist/jquery.slim.js", ""),
			f("node_modules/.bin/tool", ""),
			f("assets/css/main.css", ""),
			f("assets/scss/app.scss", ""),
			f("assets/vendor/local.js", ""),
			f("assets/docs/other.md", ""),
			f("layouts/_default/single.html", ""),
			f("layouts/partials/head.html", ""),
			f("layouts/partials/extra/proj.html", ""),
			f("layouts-extra/e1.html", ""),
			f("layouts-extra/sub/e2.html", ""),
			f("data/shared.toml", ""),
			f("data/proj.toml", ""),
			f("data2/shared.toml", ""),
			f("data2/sub/d2.json", ""),
			f("i18n/en.toml", ""),
			f("archetypes/default.md", ""),
			f("themes/mytheme/hugo.toml", `[module]
[[module.mounts]]
source = "layouts"
target = "layouts"
[[module.mounts]]
source = "assets"
target = "assets"
[[module.mounts]]
source = "data"
target = "data"
[[module.mounts]]
source = "i18n"
target = "i18n"
[[module.mounts]]
source = "static"
target = "static"
[[module.mounts]]
source = "archetypes"
target = "archetypes"
[[module.mounts]]
source = "content"
target = "content"
`),
			f("themes/mytheme/layouts/_default/single.html", ""),
			f("themes/mytheme/layouts/_default/list.html", ""),
			f("themes/mytheme/layouts/partials/head.html", ""),
			f("themes/mytheme/layouts/partials/theme-only.html", ""),
			f("themes/mytheme/assets/css/main.css", ""),
			f("themes/mytheme/assets/css/theme.css", ""),
			f("themes/mytheme/data/shared.toml", ""),
			f("themes/mytheme/data/theme.toml", ""),
			f("themes/mytheme/i18n/en.toml", ""),
			f("themes/mytheme/i18n/th.toml", ""),
			f("themes/mytheme/static/a.txt", ""),
			f("themes/mytheme/static/theme.txt", ""),
			f("themes/mytheme/archetypes/default.md", ""),
			f("themes/mytheme/archetypes/theme.md", ""),
			f("themes/mytheme/content/post.md", ""),
			f("themes/mytheme/content/theme-post.md", ""),
		},
	}
}

func multihostSite() site {
	return site{
		name: "multihost",
		tree: []node{
			f("hugo.toml", `defaultContentLanguage = "en"
[languages.en]
baseURL = "https://en.example.org/"
weight = 1
[languages.th]
baseURL = "https://th.example.org/"
weight = 2
[module]
[[module.mounts]]
source = "static"
target = "static"
[[module.mounts]]
source = "static_th"
target = "static"
lang = "th"
[[module.mounts]]
source = "static_en"
target = "static"
lang = "en"
[[module.mounts]]
source = "content"
target = "content"
`),
			f("static/common.txt", ""),
			f("static/dir/c.txt", ""),
			f("static_th/th.txt", ""),
			f("static_th/common.txt", ""),
			f("static_en/en.txt", ""),
			f("content/_index.md", ""),
			f("content/p.th.md", ""),
		},
	}
}

func emptySite() site {
	return site{
		name: "empty",
		tree: []node{
			f("hugo.toml", `baseURL = "https://example.org/"
`),
		},
	}
}

func testSite(root string) (site, error) {
	tree, err := repoTree(filepath.Join(root, "hugolib", "testsite"))
	if err != nil {
		return site{}, err
	}
	tree = append(tree, f("hugo.toml", `baseURL = "https://example.org/"
defaultContentLanguage = "en"
[languages.en]
weight = 1
[languages.nn]
weight = 2
contentDir = "content_nn"
`))
	return site{name: "testsite", tree: tree}, nil
}

func docsSite(root string) (site, error) {
	tree, err := repoTree(filepath.Join(root, "docs"), "rust-port")
	if err != nil {
		return site{}, err
	}
	// Keep only the paths the fixture needs (the tree is recorded in full, but
	// .DS_Store-like OS files would make the input machine dependent).
	var out []node
	for _, n := range tree {
		if strings.HasSuffix(n.P, ".DS_Store") {
			continue
		}
		out = append(out, n)
	}
	return site{name: "docs", tree: out}, nil
}
