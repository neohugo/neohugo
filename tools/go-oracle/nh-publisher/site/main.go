// Command site is the Go oracle for the publisher (publisher/publisher.go:
// the absURL → [livereload] → [generator] → minify transformer chain, the
// write, and the hugo_stats.json HTML elements collector) in
// crates/nh-publisher (Wave B task T07), recorded from real in-process
// hugolib builds.
//
// The seeksnack site is private, so the builds are this repository's docs/
// site (patched to build offline and deterministically: no GetRemote, no npm
// packages, no Tailwind CLI, a fixed clock) and hugolib/testsite (with
// synthetic layouts full of absURL, minifier and collector edge cases,
// aliases, two languages, RSS/sitemap/JSON/robots outputs), each in several
// config variants: canonifyURLs and relativeURLs on/off, a baseURL with and
// without a path, minify off / default / customised, and buildStats configs.
//
// Two phases:
//
//	# 1. build (this command, native): records every Publish call (descriptor,
//	#    source bytes, written bytes), the minifier client's media types/output
//	#    formats/config and the build's hugo_stats.json into $FULL/<variant>.jsonl.gz
//	go run ./tools/go-oracle/nh-publisher/site -full $FULL
//
//	# 2. fixtures (tools/go-oracle/nh-publisher/sitefix, linux/arm64 under qemu:
//	#    the minifiers' FMA-sensitive colour conversions; no hugolib, so no cgo)
//	NH_T07_ORACLE_ARCH=arm64 go run ./tools/go-oracle/nh-publisher/sitefix -full $FULL
//
// The checked-in subset is every record of the testsite builds and, for the
// docs builds, every non-HTML output plus 16 HTML pages per variant (the
// written bytes as a SHA-256, with the bytes when small), with the Go
// collector's elements of exactly those pages. The full records are kept
// outside the repository (crates/nh-publisher/tests/site.rs has an ignored
// test that reads them through NH_T07_SITE_FULL).
package main

import (
	"bytes"
	"encoding/json"
	"flag"
	"fmt"
	"io/fs"
	"log"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"sync"
	"time"

	"github.com/bep/clocks"
	"github.com/bep/logg"
	"github.com/spf13/afero"

	"github.com/neohugo/neohugo/common/htime"
	"github.com/neohugo/neohugo/common/loggers"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/config/allconfig"
	"github.com/neohugo/neohugo/deps"
	"github.com/neohugo/neohugo/hugofs"
	"github.com/neohugo/neohugo/hugolib"
	"github.com/neohugo/neohugo/media"
	"github.com/neohugo/neohugo/minifiers"
	"github.com/neohugo/neohugo/output"
	"github.com/neohugo/neohugo/publisher"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-publisher/osupport"
)

// hooks is set by the overlay-added hook file.
type oracleHooks struct {
	setRecord  func(func(d publisher.Descriptor, in, out []byte))
	setNewHook func(func(media.Types, output.Formats, minifiers.MinifyConfig))
}

var hooks *oracleHooks

const hookFile = `package main

import (
	"github.com/neohugo/neohugo/media"
	"github.com/neohugo/neohugo/minifiers"
	"github.com/neohugo/neohugo/output"
	"github.com/neohugo/neohugo/publisher"
)

func init() {
	hooks = &oracleHooks{
		setRecord:  func(f func(d publisher.Descriptor, in, out []byte)) { publisher.OracleRecord = f },
		setNewHook: func(f func(media.Types, output.Formats, minifiers.MinifyConfig)) { minifiers.OracleNewHook = f },
	}
}
`

// newHookPatch records the arguments of every minifiers.New call.
var newHookPatch = osupport.Patch{
	File:   "minifiers/minifiers.go",
	After:  "\tconf := cfg.GetConfigSection(\"minify\").(MinifyConfig)\n",
	Insert: "\tif OracleNewHook != nil {\n\t\tOracleNewHook(mediaTypes, outputFormats, conf)\n\t}\n",
	Append: "\n// OracleNewHook is set by the T07 site oracle.\nvar OracleNewHook func(media.Types, output.Formats, MinifyConfig)\n",
}

func patches() []osupport.Patch {
	return append([]osupport.Patch{osupport.MinifiersPatch, newHookPatch}, osupport.PublisherPatches...)
}

// ---------------------------------------------------------------------------
// Sites and variants.

type variant struct {
	name   string
	site   string // "docs" or "testsite"
	edit   func(toml string) string
	subset bool // check in a subset only
}

func setTop(toml, line string) string {
	return line + "\n" + toml
}

func replaceOnce(s, old, new string) string {
	if strings.Count(s, old) != 1 {
		log.Fatalf("config edit: %q found %d times", old, strings.Count(s, old))
	}
	return strings.Replace(s, old, new, 1)
}

const minifyDefault = "\n[minify]\nminifyOutput = true\n"

const minifyCustom = `
[minify]
minifyOutput = true
[minify.tdewolff.html]
keepWhitespace = true
keepComments = true
keepQuotes = true
keepEndTags = false
keepDefaultAttrVals = false
keepDocumentTags = false
keepConditionalComments = true
[minify.tdewolff.css]
precision = 3
keepCSS2 = false
[minify.tdewolff.js]
precision = 3
keepVarNames = true
version = 2015
[minify.tdewolff.json]
precision = 2
[minify.tdewolff.svg]
precision = 3
keepComments = true
[minify.tdewolff.xml]
keepWhitespace = true
`

var variants = []variant{
	{name: "docs-default", site: "docs", subset: true, edit: func(t string) string { return t + minifyDefault }},
	{name: "docs-canon", site: "docs", subset: true, edit: func(t string) string {
		t = replaceOnce(t, "disableIDs = true", "disableIDs = false")
		return setTop(t, "canonifyURLs = true") + minifyDefault
	}},
	{name: "docs-rel", site: "docs", subset: true, edit: func(t string) string { return setTop(t, "relativeURLs = true") + minifyDefault }},
	{name: "docs-canon-nomin", site: "docs", subset: true, edit: func(t string) string { return setTop(t, "canonifyURLs = true") }},
	{name: "docs-canon-custommin", site: "docs", subset: true, edit: func(t string) string { return setTop(t, "canonifyURLs = true") + minifyCustom }},
	{name: "docs-subpath-canon", site: "docs", subset: true, edit: func(t string) string {
		t = replaceOnce(t, `baseURL                = "https://gohugo.io/"`, `baseURL                = "https://example.org/docs/"`)
		return setTop(t, "canonifyURLs = true") + minifyDefault
	}},
	{name: "ts-plain", site: "testsite", edit: func(t string) string { return tsConfig("https://example.org/", "", "enable = true") }},
	{name: "ts-canon-min", site: "testsite", edit: func(t string) string {
		return tsConfig("https://example.org/", "canonifyURLs = true", "enable = true") + minifyDefault
	}},
	{name: "ts-rel-min", site: "testsite", edit: func(t string) string {
		return tsConfig("https://example.org/", "relativeURLs = true", "enable = true\ndisableTags = true") + minifyDefault
	}},
	{name: "ts-sub-canon-custommin", site: "testsite", edit: func(t string) string {
		return tsConfig("https://example.org/sub/", "canonifyURLs = true", "enable = true") + minifyCustom
	}},
	{name: "ts-sub-rel-nomin", site: "testsite", edit: func(t string) string {
		return tsConfig("https://example.org/sub/", "relativeURLs = true", "enable = true\ndisableClasses = true")
	}},
	{name: "ts-sub-canonrel-min", site: "testsite", edit: func(t string) string {
		return tsConfig("https://example.org/sub", "canonifyURLs = true\nrelativeURLs = true", "enable = true\ndisableIDs = true") + minifyDefault
	}},
}

func tsConfig(baseURL, urls, stats string) string {
	return fmt.Sprintf(`baseURL = %q
title = "Testsite"
%s
enableRobotsTXT = true
defaultContentLanguage = "en"
[languages.en]
languageName = "English"
contentDir = "content"
weight = 1
[languages.nn]
languageName = "Nynorsk"
contentDir = "content_nn"
weight = 2
[markup.goldmark.renderer]
unsafe = true
[outputs]
home = ["html", "rss", "json"]
section = ["html", "rss"]
[build.buildStats]
%s
`, baseURL, urls, stats)
}

// The testsite's synthetic files (added to hugolib/testsite's content).
var tsFiles = map[string]string{
	"layouts/_partials/page.html": `<!DOCTYPE html>
<html lang="{{ .Lang }}">
<head>
<meta charset="utf-8">
<title>{{ .Title }}</title>
<link rel="stylesheet" href="/css/site.css">
<link rel='icon' href='/favicon.ico'>
<script src="/js/app.js"></script>
<script>var u = "/x"; var s = 'src="/y"'; if (a < b) { f( 1 ) }</script>
<style>.a { background : url(/img/a.png) } .b{color:rgb(10%,20%,30%);margin:0px 0px}</style>
<meta http-equiv="refresh" content="0; url=/redirect/">
</head>
<body class="kind-{{ .Kind }} lang-{{ .Lang }}" id="top">
<nav class="nav main"><a href="/">Home</a> <a href='{{ .RelPermalink }}'>Self</a> <a href=/unquoted/>U</a> <a href="//cdn.example.com/x.js">cdn</a> <a HREF="/upper">Up</a>
<a href="/sub/in-sub/">sub</a> <a href="/subx/">subx</a> <a href="{{ "about/" | relURL }}">rel</a> <a href="{{ "about/" | absURL }}">abs</a>
<form action="/search" method="get"><input type="text" name="q" class="search-input" value=""></form>
<img src="/img/a.png" srcset="/img/a.png 1x, /img/a@2x.png 2x" alt="a" class="img responsive">
<img srcset='/img/b.png 100w,   /img/b-large.png   200w' src='/img/b.png'>
<img srcset=/img/c.png src=/img/c.png>
<picture><source srcset="/img/d.webp" type="image/webp"><img src="/img/d.jpg" id="pic-img"></picture>
</nav>
<main>{{ .Content }}{{ range .Pages }}<article class="summary"><a href="{{ .RelPermalink }}">{{ .Title }}</a>{{ .Summary }}</article>{{ end }}</main>
<table class="tbl"><thead class="th-head"><tr class="row"><th class="hdr" scope="col">H</th></tr></thead><tbody><tr><td class="cell" id="c1">1</td></tr></tbody><caption class="cap">C</caption></table>
<div x-data="{open:false}" :class="{ 'is-open': open, 'is-closed': !open }" x-bind:class="open ? 'a-open' : 'a-closed'" x-transition:enter="t-enter t-enter-2">x</div>
<svg class="icon" viewBox="0 0 24 24"><use xlink:href="/icons.svg#i"></use><path class="p" d="M 1.000 2.000 L 3 4"/></svg>
<pre class="code"><code class="language-html">&lt;div class="not-collected"&gt;</code><span class="not-collected-either"></span></pre>
<textarea class="ta"><b class="tx-inner"></b></textarea>
<!-- <div class="commented-out"> -->
<p>Text with href="/in-text" and src='/in-text-2' and url=/in-text-3 and action=/in-text-4.</p>
<footer class="footer" id="footer">&copy; {{ now.Year }} <span class="d{{ .Lang }}">{{ .Date.Format "2006" }}</span></footer>
</body>
</html>
`,
	"layouts/single.html": `{{ partial "page.html" . }}`,
	"layouts/list.html":   `{{ partial "page.html" . }}`,
	"layouts/404.html":    `<html><body class="nf"><a href="/">home</a><img src='/img/404.png' class="nf-img"></body></html>`,
	"layouts/home.json":   `{"title": {{ .Title | jsonify }}, "n": 1.50000, "pages": [{{ range $i, $p := .Site.RegularPages }}{{ if $i }}, {{ end }}{ "url" : {{ .RelPermalink | jsonify }} }{{ end }}]}`,
	"static/css/site.css": `a { color : red }`,
	"static/static.html":  `<div class="static-not-collected"></div>`,
	"content/_index.md":   "---\ntitle: Home\n---\nHome with [a link](/about/) and ![img](/img/x.png).\n",
	"content/about.md": "---\ntitle: About\naliases: [/old-about/, /older/about.html]\n---\n" +
		"About <span class=\"raw-html\" id=\"raw\">raw</span>. [Rel](/docs/a/) [Abs](https://example.org/x) [Proto](//x.org/y)\n\n" +
		"<div class=\"md-div\" x-bind:class=\"{ 'k1': a, 'k2 k3': b }\">div</div>\n\n```html\n<div class=\"fenced\">\n```\n",
	"content/posts/_index.md": "---\ntitle: Posts\n---\n",
	"content/posts/one.md": "---\ntitle: \"One & <Two>\"\ndate: 2021-01-02\ntags: [a, b]\n---\n" +
		"Post with <img src=\"/img/p.png\" srcset=\"/img/p.png 1x, /img/p2.png 2x\"> and <a href='/posts/two/'>two</a>. Ünïcödé.\n",
	"content/posts/two.md":  "---\ntitle: Two\ndate: 2021-01-03\naliases: [/posts/2/]\n---\nSecond <q class=\"quote\">q</q>.\n",
	"content_nn/_index.md":  "---\ntitle: Heim\n---\nHeim [lenkje](/nn/om/).\n",
	"content_nn/om.md":      "---\ntitle: Om\naliases: [/nn/gamal/]\n---\nOm <b class=\"nn-b\">oss</b>.\n",
	"content/sub/in-sub.md": "---\ntitle: In sub\n---\n[x](/sub/y/)\n",
}

// The docs site's edits for an offline, deterministic build.
var docsRemove = []string{
	"content/en/news/_content.gotmpl",     // GetRemote of GitHub releases
	"content/en/functions/images/Text.md", // GetRemote of a font
}

var docsReplace = map[string][2]string{
	"layouts/baseof.html": {"css.TailwindCSS $opts", "minify"},
	"assets/js/main.js": {
		"import Alpine from 'alpinejs';",
		"const Alpine = { plugin() {}, data() {}, store() {}, magic() {}, start() {}, directive() {} };",
	},
	"assets/js/turbo.js": {"import * as Turbo from '@hotwired/turbo';", "window.Turbo = { session: {} };"},
}

var docsReplaceMore = map[string][2]string{
	"assets/js/main.js": {"import persist from '@alpinejs/persist';\nimport focus from '@alpinejs/focus';", "const persist = {};\nconst focus = {};"},
}

const githubInfo = `{{ return dict "html_url" "https://github.com/gohugoio/hugo" "stargazers_url" "https://api.github.com/repos/gohugoio/hugo/stargazers" "watchers_count" 1234 "stargazers_count" 76543 "forks_count" 7890 "contributors_url" "https://api.github.com/repos/gohugoio/hugo/contributors" "releases_url" "https://api.github.com/repos/gohugoio/hugo/releases{/id}" }}`

func copyTree(src, dst string) error {
	return filepath.WalkDir(src, func(p string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		rel, err := filepath.Rel(src, p)
		if err != nil {
			return err
		}
		if rel == "public" || rel == "resources" || rel == "node_modules" || strings.HasPrefix(rel, ".") && rel != "." {
			if d.IsDir() {
				return filepath.SkipDir
			}
			return nil
		}
		t := filepath.Join(dst, rel)
		if d.IsDir() {
			return os.MkdirAll(t, 0o755)
		}
		b, err := os.ReadFile(p)
		if err != nil {
			return err
		}
		return os.WriteFile(t, b, 0o644)
	})
}

func editFile(fn string, old, new string) {
	b, err := os.ReadFile(fn)
	if err != nil {
		log.Fatal(err)
	}
	s := string(b)
	if strings.Count(s, old) != 1 {
		log.Fatalf("%s: %q found %d times", fn, old, strings.Count(s, old))
	}
	if err := os.WriteFile(fn, []byte(strings.Replace(s, old, new, 1)), 0o644); err != nil {
		log.Fatal(err)
	}
}

// prepare copies the site of v into a new temp dir and applies the edits and
// the variant's config. It returns the dir.
func prepare(root string, v variant) string {
	dir, err := os.MkdirTemp("", "nh-t07-site-")
	if err != nil {
		log.Fatal(err)
	}
	switch v.site {
	case "docs":
		if err := copyTree(filepath.Join(root, "docs"), dir); err != nil {
			log.Fatal(err)
		}
		for _, f := range docsRemove {
			if err := os.Remove(filepath.Join(dir, f)); err != nil {
				log.Fatal(err)
			}
		}
		for f, r := range docsReplace {
			editFile(filepath.Join(dir, f), r[0], r[1])
		}
		for f, r := range docsReplaceMore {
			editFile(filepath.Join(dir, f), r[0], r[1])
		}
		if err := os.WriteFile(filepath.Join(dir, "layouts/_partials/helpers/funcs/get-github-info.html"), []byte(githubInfo), 0o644); err != nil {
			log.Fatal(err)
		}
		_ = os.Remove(filepath.Join(dir, "hugo_stats.json"))
	case "testsite":
		if err := copyTree(filepath.Join(root, "hugolib", "testsite"), dir); err != nil {
			log.Fatal(err)
		}
		for f, c := range tsFiles {
			fn := filepath.Join(dir, filepath.FromSlash(f))
			if err := os.MkdirAll(filepath.Dir(fn), 0o755); err != nil {
				log.Fatal(err)
			}
			if err := os.WriteFile(fn, []byte(c), 0o644); err != nil {
				log.Fatal(err)
			}
		}
	}
	cfgFile := filepath.Join(dir, "hugo.toml")
	b, _ := os.ReadFile(cfgFile)
	if err := os.WriteFile(cfgFile, []byte(v.edit(string(b))), 0o644); err != nil {
		log.Fatal(err)
	}
	return dir
}

// ---------------------------------------------------------------------------
// Records.

type pubRec struct {
	T      string `json:"t"`
	Path   string `json:"path"`
	Format string `json:"format"`
	MT     string `json:"mt"`
	HTML   bool   `json:"html"`
	Abs    string `json:"abs"`
	Gen    bool   `json:"gen"`
	LR     string `json:"lr,omitempty"`
	In     any    `json:"in"`
	Out    any    `json:"out,omitempty"`
	Sha    string `json:"sha,omitempty"`
	N      int    `json:"n"`
}

type buildRec struct {
	T          string   `json:"t"`
	Site       string   `json:"site"`
	Variant    string   `json:"variant"`
	Types      [][2]any `json:"types"`
	Formats    [][3]any `json:"formats"`
	Minify     any      `json:"minify"`
	BuildStats [4]bool  `json:"buildStats"` // enable, disableTags, disableClasses, disableIDs
	Stats      *string  `json:"stats"`
	Arch       string   `json:"arch"`
	Pubs       int      `json:"pubs"`
	Extra      []string `json:"extra,omitempty"`
	Elements   any      `json:"elements,omitempty"`
}

func dumpConf(c minifiers.MinifyConfig) map[string]any {
	t := c.Tdewolff
	return map[string]any{
		"MinifyOutput": c.MinifyOutput, "DisableHTML": c.DisableHTML, "DisableCSS": c.DisableCSS, "DisableJS": c.DisableJS,
		"DisableJSON": c.DisableJSON, "DisableSVG": c.DisableSVG, "DisableXML": c.DisableXML,
		"HTML": map[string]any{
			"KeepComments": t.HTML.KeepComments, "KeepConditionalComments": t.HTML.KeepConditionalComments,
			"KeepSpecialComments": t.HTML.KeepSpecialComments, "KeepDefaultAttrVals": t.HTML.KeepDefaultAttrVals,
			"KeepDocumentTags": t.HTML.KeepDocumentTags, "KeepEndTags": t.HTML.KeepEndTags, "KeepQuotes": t.HTML.KeepQuotes,
			"KeepWhitespace": t.HTML.KeepWhitespace, "TemplateDelims": []string{t.HTML.TemplateDelims[0], t.HTML.TemplateDelims[1]},
		},
		"CSS":  map[string]any{"KeepCSS2": t.CSS.KeepCSS2, "Precision": t.CSS.Precision, "Inline": t.CSS.Inline},
		"JS":   map[string]any{"Precision": t.JS.Precision, "KeepVarNames": t.JS.KeepVarNames, "Version": t.JS.Version},
		"JSON": map[string]any{"Precision": t.JSON.Precision, "KeepNumbers": t.JSON.KeepNumbers},
		"SVG":  map[string]any{"KeepComments": t.SVG.KeepComments, "Precision": t.SVG.Precision, "Inline": t.SVG.Inline},
		"XML":  map[string]any{"KeepWhitespace": t.XML.KeepWhitespace},
	}
}

func typesRec(ts media.Types) [][2]any {
	var out [][2]any
	for _, t := range ts {
		out = append(out, [2]any{t.Type, t.Suffixes()})
	}
	return out
}

func formatsRec(fs output.Formats) [][3]any {
	var out [][3]any
	for _, f := range fs {
		out = append(out, [3]any{f.Name, f.MediaType.Type, f.IsHTML})
	}
	return out
}

// ---------------------------------------------------------------------------
// Phase 1: build.

func build(root, full string) {
	fixed := time.Date(2026, 9, 27, 12, 0, 0, 0, time.UTC)
	htime.Clock = clocks.Fixed(fixed)

	for _, v := range variants {
		dir := prepare(root, v)
		cacheDir, err := os.MkdirTemp("", "nh-t07-cache-")
		if err != nil {
			log.Fatal(err)
		}

		var mu sync.Mutex
		var pubs []pubRec
		var clientArgs []string
		var br buildRec
		hooks.setRecord(func(d publisher.Descriptor, in, out []byte) {
			mu.Lock()
			defer mu.Unlock()
			lr := ""
			if d.LiveReloadBaseURL != nil {
				lr = d.LiveReloadBaseURL.String()
			}
			pubs = append(pubs, pubRec{
				T: "pub", Path: filepath.ToSlash(d.TargetPath), Format: d.OutputFormat.Name, MT: d.OutputFormat.MediaType.Type,
				HTML: d.OutputFormat.IsHTML, Abs: d.AbsURLPath, Gen: d.AddHugoGeneratorTag, LR: lr,
				In: osupport.B(in), Out: osupport.B(out),
			})
		})
		hooks.setNewHook(func(ts media.Types, fs output.Formats, c minifiers.MinifyConfig) {
			mu.Lock()
			defer mu.Unlock()
			b, _ := json.Marshal([]any{typesRec(ts), formatsRec(fs), dumpConf(c)})
			sig := string(b)
			for _, s := range clientArgs {
				if s == sig {
					return
				}
			}
			clientArgs = append(clientArgs, sig)
			br.Types, br.Formats, br.Minify = typesRec(ts), formatsRec(fs), dumpConf(c)
		})

		var logBuf bytes.Buffer
		logger := loggers.New(loggers.Options{StdOut: &logBuf, StdErr: &logBuf, Level: logg.LevelWarn, DistinctLevel: logg.LevelWarn})
		flags := config.New()
		flags.Set("workingDir", dir)
		flags.Set("noBuildLock", true)
		flags.Set("cacheDir", cacheDir)
		res, err := allconfig.LoadConfig(allconfig.ConfigSourceDescriptor{
			Flags: flags, Fs: hugofs.Os, Filename: filepath.Join(dir, "hugo.toml"), Logger: logger,
			Environ: []string{"NEOHUGO_ORACLE=1"},
		})
		if err != nil {
			log.Fatalf("%s: load config: %v", v.name, err)
		}
		fsCfg := config.New()
		fsCfg.Set("workingDir", dir)
		fsCfg.Set("publishDir", res.LoadingInfo.BaseConfig.PublishDir)
		hfs := hugofs.NewFromSourceAndDestination(hugofs.Os, afero.NewMemMapFs(), fsCfg)
		h, err := hugolib.NewHugoSites(deps.DepsCfg{Configs: res, Fs: hfs, LogLevel: logger.Level(), StdErr: &logBuf, StdOut: &logBuf})
		if err != nil {
			log.Fatalf("%s: new sites: %v", v.name, err)
		}
		if err := h.Build(hugolib.BuildCfg{}); err != nil {
			log.Fatalf("%s: build: %v\n%s", v.name, err, logBuf.String())
		}
		hooks.setRecord(nil)
		hooks.setNewHook(nil)

		bs := res.Base.Build.BuildStats
		br.T, br.Site, br.Variant, br.Arch, br.Pubs = "build", v.site, v.name, osupport.Arch(), len(pubs)
		br.BuildStats = [4]bool{bs.Enable, bs.DisableTags, bs.DisableClasses, bs.DisableIDs}
		if len(clientArgs) != 1 {
			// Every publisher (and resource minifier) of the build must share one config.
			br.Extra = clientArgs
			log.Fatalf("%s: %d distinct minifier clients", v.name, len(clientArgs))
		}
		if b, err := os.ReadFile(filepath.Join(dir, "hugo_stats.json")); err == nil {
			s := string(b)
			br.Stats = &s
		}

		sort.SliceStable(pubs, func(i, j int) bool { return pubs[i].Path < pubs[j].Path })
		w, err := osupport.Create(filepath.Join(full, v.name+".jsonl.gz"))
		if err != nil {
			log.Fatal(err)
		}
		if err := w.Write(br); err != nil {
			log.Fatal(err)
		}
		for i := range pubs {
			pubs[i].N = i
			if err := w.Write(pubs[i]); err != nil {
				log.Fatal(err)
			}
		}
		if err := w.Close(); err != nil {
			log.Fatal(err)
		}
		log.Printf("site: %s: %d publishes, stats %v", v.name, len(pubs), br.Stats != nil)
		_ = os.RemoveAll(dir)
		_ = os.RemoveAll(cacheDir)
	}
}

// ---------------------------------------------------------------------------
// Phase 2: fixtures.

func main() {
	root := flag.String("root", ".", "neohugo module root")
	full := flag.String("full", "", "directory of the full (phase 1) records")
	flag.Parse()

	if *full == "" {
		log.Fatal("usage: site -full DIR")
	}
	absFull, err := filepath.Abs(*full)
	if err != nil {
		log.Fatal(err)
	}
	absRoot, err := filepath.Abs(*root)
	if err != nil {
		log.Fatal(err)
	}

	if !osupport.IsChild() {
		if err := osupport.RunOverlaid(absRoot, "./tools/go-oracle/nh-publisher/site", patches(), "hook_overlay.go", hookFile,
			[]string{"-root", absRoot, "-full", absFull}); err != nil {
			log.Fatal(err)
		}
		return
	}
	if hooks == nil {
		log.Fatal("site: hooks not installed (not run through the overlay)")
	}
	build(absRoot, absFull)
}
