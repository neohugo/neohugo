package main

import (
	"io/fs"
	"log"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"unicode/utf8"

	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/corpus"
)

// input is one Parse(component, path) call, run with the named parsers.
type input struct {
	set     string // where the input comes from
	c       string // component
	s       string // path
	parsers []string
}

var components = []string{"content", "layouts", "assets", "data", "i18n", "static", "archetypes", ""}

// repoTrees maps this repository's site directories to their component.
var repoTrees = []struct{ dir, comp string }{
	{"docs/content/en", "content"},
	{"docs/layouts", "layouts"},
	{"docs/assets", "assets"},
	{"docs/data", "data"},
	{"docs/static", "static"},
	{"docs/archetypes", "archetypes"},
	{"hugolib/testsite/content", "content"},
	{"hugolib/testsite/content_nn", "content"},
	{"tpl/tplimpl/embedded/templates", "layouts"},
}

// sweepDirs and sweepNames are crossed into the synthetic sweep.
var sweepDirs = []string{
	"", "a/b", "_partials/sub", "foo/_shortcodes", // also run with the test parser
	"a", "A B/C d", "posts", "_default", "_markup", "ภาษาไทย/ข่าว", "Ünï.cödé/x.y", "..hidden/.x",
}

var sweepNames = []string{
	// bundles, every case
	"index.md", "_index.md", "INDEX.MD", "_Index.Md", "Index.md", "index.en.md", "_index.en.md",
	"index.th.md", "_index.th.md", "index.EN.md", "_index.TH.md", "index.fr.md", "_index.de.md",
	"index.no.md", "index.xx.md", "index.en.th.md", "index.th.en.md", "index.md.md",
	"index.html", "_index.html", "index.htm", "index", "_index", "index.", "_index.",
	"index.md.bak", "index.markdown", "index.adoc", "index.en.markdown", "_index.org",
	// singles
	"post.md", "post.en.md", "post.th.md", "Post Title.md", "Post  Title.EN.md", "a.b.c.d.md",
	"a.b.c.d.th.md", "เลย์.md", "ข่าว.th.md", "日本語.md", "😀.md", "café.en.md",
	"UPPER.MD", "x.MD", "baseof.md", "list.md", "section.md", "term.md", "home.th.md",
	// layouts: identifiers
	"baseof.html", "baseof.th.html", "baseof.en.amp.html", "baseof.list.html",
	"baseof.list.section.th.amp.html", "single.html", "single.html.html", "single.th.html",
	"list.amp.html", "list.html.amp", "list.th.amp.html", "list.amp.th.html", "list.no.amp.not.html",
	"section.html", "section.th.html", "taxonomy.html", "term.html", "home.html",
	"page.html", "taxonomyterm.html", "Section.HTML", "home.json", "list.json.json",
	"index.json", "index.rss.xml", "list.rss.xml", "rss.xml", "sitemap.xml",
	"sitemapindex.xml", "robots.txt", "robotstxt.txt", "404.html", "alias.html",
	"render-table.html.html", "render-table.json.json", "render-link.html",
	"render-image.th.html", "myshortcode.html", "myshortcode.list.html",
	"myshortcode.th.html", "myshortcode.list.th.html", "no.html", "en.html", "th.html",
	"mylayout.list.section.th.html", "styles.css", "styles.css.html", "css.html",
	"calendar.ics", "index.calendar.ics", "list.calendar.ics", "manifest.webmanifest",
	"single.markdown.md", "list.gotmpl.gotmpl",
	// resources and data
	"main.scss", "data.yaml", "data.th.yaml", "en.toml", "th.yaml", "image.jpg",
	"IMAGE.JPG", "image.th.jpg", "image.en.th.jpg", "file.tar.gz", "README", "Makefile",
	"_content.gotmpl", "_content.th.gotmpl", "_content.yaml", "_content", "_CONTENT.GOTMPL",
	// dots, spaces, odd names
	".gitkeep", ".", "..", "...", "a.", ".md", "..md", "a..b.md", "name with space.txt",
	"trailing/", "double//slash.md", "tab\there.md", "%20.md", "a%2Fb.md",
	"_shortcodes", "_partials", "_markup",
}

// specPaths are seeksnack paths quoted in docs/rust-port/specs (component,
// path relative to the component root).
var specPaths = [][2]string{
	// content (content-model.md, output-publishing.md, images.md)
	{"content", "_index.md"}, {"content", "latesturl.md"}, {"content", "privacy.md"},
	{"content", "terms.md"}, {"content", "disclaimer.md"}, {"content", "search.md"},
	{"content", "brands/alice/_index.md"}, {"content", "brands/_index.md"},
	{"content", "companies/frito-lay/_index.th.md"},
	{"content", "companies/hanami-foods-co-ltd/_index.th.md"},
	{"content", "companies/berli-jucker-foods-ltd.berli-jucker-plc/_index.md"},
	{"content", "companies/tohato-inc./_index.md"},
	{"content", "biscuit/koalas-march-chocolate/index.md"},
	{"content", "biscuit/koalas-march-chocolate/index.th.md"},
	{"content", "biscuit/hello-panda-biscuits-with-chocolate-flavoured-filling/index.th.md"},
	{"content", "biscuit-stick/pocky-biscuit-sticks-white-peach-strawberry/index.en.md"},
	{"content", "biscuit-stick/pocky-biscuit-sticks-almond-crush-thailand/index.th.md"},
	{"content", "biscuit-roll/collon-cream/collon_cream_package.jpg"},
	{"content", "biscuit-roll/collon-cream/index.md"},
	{"content", "potato-chips/herrs-salt-&-vinegar-potato-chips/index.md"},
	{"content", "potato-chips/wise-chili-olé-chili-&-spice-flavor-potato-chips/index.md"},
	{"content", "potato-chips/lays-flat-potato-chip-seaweed-gochujang-sauce-flavor/index.md"},
	{"content", "corn-chips/party-crispy-pie-butter-caramel/index.th.md"},
	{"content", "rice-chips/zeni-zeni/index.md"},
	{"content", "pie/chocky-banana-pie/index.md"},
	{"content", "crepe/thai-crepe/index.th.md"},
	{"content", "sponge-cake/yubari-melon-steam-cake/index.th.md"},
	{"content", "ice-cream/vanilla-and-milk-flavoured-ice-cream-in-chewy-mochi/index.md"},
	{"content", "pastry/x/index.en.md"}, {"content", "section/bundle/img.jpg"},
	{"content", "posts/_index.md"}, {"content", "tags/คริสปี้พาย /_index.md"},
	{"content", "tags/lays/_index.md"}, {"content", "ingredients/ins-322i/_index.md"},
	{"content", "categories/no-salt/low-salt-chips/_index.md"},
	// assets (resources-pipeline.md, images.md)
	{"assets", "_jsconfig/package.json"}, {"assets", "_jsconfig/postcss.config.js"},
	{"assets", "images/companies/kee-wee-hup-kee.gif"}, {"assets", "images/logo.png"},
	{"assets", "images/watermark.png"}, {"assets", "jsconfig.json"},
	{"assets", "scss/website.scss"}, {"assets", "ts/comment.ts"}, {"assets", "ts/search.ts"},
	{"assets", "ts/themeset.ts"}, {"assets", "ts/themeswitch.ts"},
	{"assets", "vendor/bootstrap/dist/js/bootstrap.bundle.min.js"},
	{"static", "css/seeksnack.css"}, {"static", "favicon.ico"}, {"static", "images/og.png"},
	{"i18n", "en.yaml"}, {"i18n", "th.yaml"}, {"data", "companies.json"},
	{"archetypes", "default.md"},
}

// seeksnackLayouts are the site's 58 layout files (templates-inventory.md §1.1).
var seeksnackLayouts = []string{
	"_default/baseof.html", "_default/single.html", "_default/list.html", "index.html",
	"_default/index.json", "_default/rss.xml", "404.html", "_default/simple.html",
	"_default/latesturl.html", "taxonomy/list.html", "term/term.html", "robots.txt",
	"sitemap.xml", "_default/_markup/render-heading.html", "_default/_markup/render-image.html",
	"partials/head.html", "partials/header.html", "partials/footer.html",
	"partials/marketing/jsonLd.html", "partials/comments.html", "partials/breadcrumb.html",
	"partials/pagination.html", "partials/ads/adsensehead.html", "partials/ads/adsensemanual.html",
	"partials/marketing/google/googleGtag.html",
	"partials/marketing/google/googleTagManagerHead.html", "partials/carousel.html",
	"partials/related.html", "partials/rating/rating.html", "partials/single/socialshare.html",
	"partials/single/nutritionfacts.html", "partials/single/ingredientslist.html",
	"partials/single/author.html", "partials/single/date.html", "partials/single/whenseen.html",
	"partials/single/taste.html", "partials/taxonomy/tags.html",
	"partials/taxonomy/categories.html", "partials/taxonomy/countries.html",
	"partials/taxonomy/companies.html", "partials/taxonomy/brands.html",
	"partials/taxonomy/ingredients.html", "partials/taxonomy/company/website.html",
	"partials/taxonomy/company/facebook.html", "partials/taxonomy/company/twitter.html",
	"partials/taxonomy/company/instagram.html", "partials/taxonomy/company/youtube.html",
	"partials/marketing/google/googleTagManagerBody.html",
	"partials/marketing/facebook/likeshare.html", "partials/marketing/twitter/share.html",
	"posts/list.html", "shortcodes/modalImage.html", "shortcodes/url.html", "alias.html",
	"sitemapindex.xml", "_markup/render-link.html", "_markup/render-table.html.html",
	"_markup/render-table.json.json", "_shortcodes/ref.html",
	// fromLegacyPath forms (templates-inventory.md)
	"_partials/head.html", "_partials/marketing/google/googlegtag.html",
	"_shortcodes/modalimage.html", "baseof.html", "single.html", "list.html",
	"simple.html", "latesturl.html",
}

// walkRepo returns every file and directory of the repository site trees.
func walkRepo(root string) []input {
	var out []input
	add := func(set, comp, rel string) {
		ps := []string{"seeksnack"}
		if comp == "content" || comp == "layouts" {
			ps = append(ps, "test")
		}
		out = append(out, input{set: set, c: comp, s: rel, parsers: ps})
	}
	for _, t := range repoTrees {
		dir := filepath.Join(root, t.dir)
		if _, err := os.Stat(dir); err != nil {
			continue
		}
		err := filepath.WalkDir(dir, func(p string, d fs.DirEntry, err error) error {
			if err != nil {
				return err
			}
			rel, err := filepath.Rel(dir, p)
			if err != nil {
				return err
			}
			rel = filepath.ToSlash(rel)
			if rel == "." {
				return nil
			}
			if d.IsDir() {
				add("repo-dir", t.comp, rel)
				add("repo-dir", t.comp, "/"+rel+"/")
				return nil
			}
			add("repo", t.comp, rel)
			return nil
		})
		if err != nil {
			log.Fatal(err)
		}
	}
	// create/skeletons/{site,theme}/<component>/...
	for _, sk := range []string{"site", "theme"} {
		for _, comp := range components {
			if comp == "" {
				continue
			}
			dir := filepath.Join(root, "create/skeletons", sk, comp)
			if _, err := os.Stat(dir); err != nil {
				continue
			}
			err := filepath.WalkDir(dir, func(p string, d fs.DirEntry, err error) error {
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
				add("skeleton", comp, filepath.ToSlash(rel))
				return nil
			})
			if err != nil {
				log.Fatal(err)
			}
		}
	}
	return out
}

func pathInputs(root string) []input {
	var in []input
	in = append(in, walkRepo(root)...)

	for _, sp := range specPaths {
		in = append(in, input{set: "spec", c: sp[0], s: sp[1], parsers: []string{"seeksnack"}})
		in = append(in, input{set: "spec", c: sp[0], s: "/" + sp[1], parsers: []string{"seeksnack"}})
	}
	for _, l := range seeksnackLayouts {
		in = append(in, input{set: "spec-layout", c: "layouts", s: l, parsers: []string{"seeksnack", "test"}})
		in = append(in, input{set: "spec-layout", c: "layouts", s: "/" + l, parsers: []string{"seeksnack"}})
	}

	// The sweep: dirs × names for content and layouts, a few dirs for the
	// other components.
	for i, d := range sweepDirs {
		ps := []string{"seeksnack"}
		if i < 4 {
			ps = append(ps, "test")
		}
		for _, n := range sweepNames {
			s := n
			if d != "" {
				s = d + "/" + n
			}
			for _, c := range []string{"content", "layouts"} {
				in = append(in, input{set: "sweep", c: c, s: s, parsers: ps})
			}
		}
	}
	for _, d := range []string{"", "a/b"} {
		for _, n := range sweepNames {
			s := n
			if d != "" {
				s = "/" + d + "/" + n
			}
			for _, c := range components {
				if c != "content" && c != "layouts" {
					in = append(in, input{set: "sweep-other", c: c, s: s, parsers: []string{"seeksnack"}})
				}
			}
			for _, c := range []string{"content", "layouts", "assets"} {
				in = append(in, input{set: "sweep-nolang", c: c, s: s, parsers: []string{"nolang"}})
			}
		}
	}
	// Slash forms.
	for _, s := range []string{"", "/", "//", "///", "a//", "/a/b/", "a/b//", "/_index.md/", "./a.md", "../a.md", "a/./b.md", "a/../b.md"} {
		for _, c := range components {
			in = append(in, input{set: "slashes", c: c, s: s, parsers: []string{"seeksnack", "test", "nolang"}})
		}
	}

	// The string corpus as term keys and page names (valid UTF-8 only: Rust
	// paths are str).
	strs, err := corpus.Strings(root)
	if err != nil {
		log.Fatal(err)
	}
	for _, s := range strs {
		if !utf8.ValidString(s) {
			continue
		}
		in = append(in, input{set: "term", c: "content", s: "/tags/" + s + "/_index.md", parsers: []string{"seeksnack"}})
		if strings.ContainsAny(s, "./_") {
			in = append(in, input{set: "corpus", c: "layouts", s: s, parsers: []string{"seeksnack"}})
		}
	}

	sort.SliceStable(in, func(i, j int) bool { return in[i].set < in[j].set })
	return in
}
