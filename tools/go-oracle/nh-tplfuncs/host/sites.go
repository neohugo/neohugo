package main

import (
	"bytes"
	"encoding/hex"
	"fmt"
	"image"
	"image/color"
	"image/gif"
	"image/jpeg"
	"image/png"
	"path/filepath"
	"sort"
	"strings"
	"unicode/utf8"

	"github.com/bep/logg"
	"github.com/neohugo/neohugo/common/loggers"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/config/allconfig"
	"github.com/neohugo/neohugo/deps"
	"github.com/neohugo/neohugo/hugofs"
	"github.com/neohugo/neohugo/hugolib"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-page/psupport"
	"github.com/spf13/afero"
)

// site is one site of the oracle: hugo.toml plus files (relative to the site
// directory). Binary files are given as bytes.
type site struct {
	Name  string
	TOML  string
	Files map[string]string
	Bin   map[string][]byte
}

// builtSite is a site built with hugolib (process and assemble, no render),
// like the Rust test does (nh-hugolib process + assemble + freeze).
type builtSite struct {
	s   site
	h   *hugolib.HugoSites
	afs afero.Fs
	log *bytes.Buffer
	// errs receives the errors sent to the error handler by the cases.
	errs chan error
}

func fm(lines ...string) string {
	return "---\n" + strings.Join(lines, "\n") + "\n---\n"
}

// images returns small deterministic images in the formats images.Config
// decodes.
func testImages() map[string][]byte {
	img := image.NewNRGBA(image.Rect(0, 0, 12, 8))
	for y := range 8 {
		for x := range 12 {
			img.Set(x, y, color.NRGBA{R: uint8(x * 20), G: uint8(y * 30), B: uint8((x + y) * 10), A: 255})
		}
	}
	out := map[string][]byte{}
	var b bytes.Buffer
	if err := png.Encode(&b, img); err != nil {
		panic(err)
	}
	out["png"] = append([]byte(nil), b.Bytes()...)
	b.Reset()
	if err := jpeg.Encode(&b, img, &jpeg.Options{Quality: 80}); err != nil {
		panic(err)
	}
	out["jpg"] = append([]byte(nil), b.Bytes()...)
	b.Reset()
	pal := image.NewPaletted(image.Rect(0, 0, 5, 3), color.Palette{color.Black, color.White})
	if err := gif.Encode(&b, pal, nil); err != nil {
		panic(err)
	}
	out["gif"] = append([]byte(nil), b.Bytes()...)
	wm := image.NewNRGBA(image.Rect(0, 0, 4, 4))
	for y := range 4 {
		for x := range 4 {
			wm.Set(x, y, color.NRGBA{R: 255, A: uint8(60 * (x + 1))})
		}
	}
	b.Reset()
	if err := png.Encode(&b, wm); err != nil {
		panic(err)
	}
	out["wm"] = append([]byte(nil), b.Bytes()...)
	return out
}

// partials are the layouts of the partial tests (every site has them).
var partials = map[string]string{
	"layouts/_partials/hello.html":              `Hello {{ . }}!`,
	"layouts/_partials/hello.txt":               `Text {{ . }}`,
	"layouts/_partials/ret.html":                `{{ $x := add 1 2 }}ignored output{{ return (dict "v" . "x" $x) }}`,
	"layouts/_partials/retnil.html":             `{{ if false }}{{ return 1 }}{{ end }}nothing returned`,
	"layouts/_partials/retstr.html":             `{{ return (printf "%s-%s" . "r") }}`,
	"layouts/_partials/nested.html":             `[{{ partial "hello.html" . }}|{{ partialCached "hello.html" . "v" }}]`,
	"layouts/_partials/cached.html":             `{{ math.Counter }}`,
	"layouts/_partials/current.html":            `{{ with templates.Current }}{{ .Name }}|{{ .Filename }}|{{ .Level }}|{{ with .Parent }}{{ .Name }}{{ end }}|{{ len .Ancestors }}{{ end }}`,
	"layouts/_partials/outer.html":              `outer:{{ partial "current.html" . }}`,
	"layouts/_partials/page.html":               `{{ with page }}{{ .Title }}|{{ .Kind }}{{ else }}no page{{ end }}`,
	"layouts/_partials/loop.html":               `{{ partialCached "loop.html" . }}`,
	"layouts/_partials/err.html":                `{{ .Foo.Bar }}`,
	"layouts/_partials/sub/deep.html":           `deep {{ . }}`,
	"layouts/_partials/dot.html":                `{{ printf "%T" . }}`,
	"layouts/_partials/exists.html":             `{{ templates.Exists "_partials/hello.html" }}{{ templates.Exists "partials/hello.html" }}`,
	"layouts/shortcodes/sc.html":                `sc`,
	"layouts/single.html":                       `{{ .Title }}`,
	"layouts/list.html":                         `{{ .Title }}`,
	"layouts/_default/_markup/render-link.html": `<a href="{{ .Destination }}">{{ .Text }}</a>`,
}

// sites returns the sites of the oracle.
func sites() []site {
	imgs := testImages()
	bin := map[string][]byte{
		"assets/img.png":       imgs["png"],
		"assets/img.jpg":       imgs["jpg"],
		"assets/img.gif":       imgs["gif"],
		"assets/wm.png":        imgs["wm"],
		"files/img.png":        imgs["png"],
		"content/b/q1/cat.png": imgs["png"],
	}
	common := map[string]string{
		"content/_index.md":     fm(`title: "Home"`) + "Home *content*.\n",
		"content/a/_index.md":   fm(`title: "Section A"`),
		"content/a/p1.md":       fm(`title: "Alpha"`, `date: 2021-01-01T00:00:00Z`, `tags: ["go", "rust"]`) + "Alpha [link](/a/p2/).\n",
		"content/a/p2.md":       fm(`title: "ข้าว beta"`, `date: 2022-02-02T00:00:00Z`) + "Beta.\n",
		"content/b/q1/index.md": fm(`title: "Bundle"`, `date: 2020-03-03T00:00:00Z`) + "Bundle.\n",
		"assets/data.json":      `{"a": 1, "b": [1, 2.5, "x"], "c": {"d": true}}`,
		"assets/data.toml":      "a = 1\nb = [1, 2]\n[c]\nd = 2021-01-01T00:00:00Z\n",
		"assets/data.yaml":      "a: 1\nb: [x, 2]\nc:\n  d: null\n",
		"assets/data.csv":       "a;b\n1;2\n",
		"assets/data.xml":       "<root><a>1</a></root>",
		"assets/data.txt":       "plain text",
		"assets/style.scss":     "$c: #333;\nbody { color: $c; a { color: red; } }\n",
		"assets/js/main.js":     "import { v } from './dep.js';\nconsole.log(v * 2);\n",
		"assets/js/dep.js":      "export const v = 21;\n",
		"files/hello.txt":       "Hello, file!\n",
		"files/sub/x.md":        "# x\n",
		"i18n/en.toml":          "[hello]\nother = \"Hello {{ .Name }}\"\n[apples]\none = \"One apple\"\nother = \"{{ .Count }} apples\"\n[html]\nother = \"<b>bold</b>\"\n",
		"i18n/th.toml":          "[hello]\nother = \"สวัสดี {{ .Name }}\"\n[apples]\nother = \"แอปเปิ้ล {{ .Count }} ผล\"\n",
	}
	for k, v := range partials {
		common[k] = v
	}
	mk := func(name, toml string, extra map[string]string) site {
		files := map[string]string{}
		for k, v := range common {
			files[k] = v
		}
		for k, v := range extra {
			files[k] = v
		}
		b := map[string][]byte{}
		for k, v := range bin {
			b[k] = v
		}
		return site{Name: name, TOML: toml, Files: files, Bin: b}
	}
	const base = `title = "T19"
disableKinds = ["RSS", "sitemap", "robotsTXT", "404", "taxonomy", "term"]
[taxonomies]
tag = "tags"
[security.funcs]
getenv = ["^HUGO_", "^NEOHUGO_T19_"]
`
	multiContent := map[string]string{
		"content/_index.th.md": fm(`title: "หน้าแรก"`),
		"content/a/p1.th.md":   fm(`title: "อัลฟา"`, `date: 2021-01-01T00:00:00Z`),
		"content/a/p3.th.md":   fm(`title: "Only Thai"`, `date: 2021-05-05T00:00:00Z`),
	}
	return []site{
		mk("en", `baseURL = "https://example.org/"
defaultContentLanguage = "en"
`+base, nil),
		mk("th", `baseURL = "https://example.org/"
defaultContentLanguage = "th"
languageCode = "th"
`+base, nil),
		mk("sub", `baseURL = "https://example.org/sub/"
`+base, nil),
		mk("canon", `baseURL = "https://example.org/blog/"
canonifyURLs = true
`+base, nil),
		mk("multi", `baseURL = "https://example.org/"
defaultContentLanguage = "en"
`+base+`[languages.en]
weight = 1
languageName = "English"
[languages.th]
weight = 2
languageName = "ไทย"
languageCode = "th-TH"
`, multiContent),
		mk("multisub", `baseURL = "https://example.org/docs/"
defaultContentLanguage = "en"
defaultContentLanguageInSubdir = true
`+base+`[languages.en]
weight = 1
[languages.th]
weight = 2
`, multiContent),
		mk("multihost", `defaultContentLanguage = "en"
`+base+`[languages.en]
weight = 1
baseURL = "https://en.example.org/"
[languages.th]
weight = 2
baseURL = "https://th.example.org/sub/"
`, multiContent),
		mk("ap", `baseURL = "https://example.org/"
titleCaseStyle = "ap"
`+base, nil),
		mk("chicago", `baseURL = "https://example.org/"
titleCaseStyle = "chicago"
`+base, nil),
		mk("gostyle", `baseURL = "https://example.org/"
titleCaseStyle = "go"
`+base, nil),
		mk("firstupper", `baseURL = "https://example.org/"
titleCaseStyle = "firstupper"
`+base, nil),
	}
}

// sitesRoot is the temporary directory of the sites (set by main, removed
// when it exits).
var sitesRoot string

// siteDir is the working directory of a site.
func siteDir(name string) string {
	return filepath.Join(sitesRoot, name)
}

// buildSite writes s into its directory below sitesRoot and builds it with
// hugolib without rendering (Build with SkipRender: process and assemble).
func buildSite(s site) (*builtSite, error) {
	// The real OS filesystem (a temporary directory): the os namespace and the
	// resources see the same errors and listings as the Rust test.
	afs := afero.NewOsFs()
	dir := siteDir(s.Name)
	files := map[string][]byte{"hugo.toml": []byte(s.TOML)}
	for k, v := range s.Files {
		files[k] = []byte(v)
	}
	for k, v := range s.Bin {
		files[k] = v
	}
	keys := make([]string, 0, len(files))
	for k := range files {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	for _, k := range keys {
		fn := filepath.Join(dir, filepath.FromSlash(k))
		if err := afs.MkdirAll(filepath.Dir(fn), 0o777); err != nil {
			return nil, err
		}
		if err := afero.WriteFile(afs, fn, files[k], 0o666); err != nil {
			return nil, err
		}
		mt := psupport.ModTime(k)
		if err := afs.Chtimes(fn, mt, mt); err != nil {
			return nil, err
		}
	}

	var logBuf bytes.Buffer
	logger := loggers.New(loggers.Options{
		StdOut:        &logBuf,
		StdErr:        &logBuf,
		Level:         logg.LevelWarn,
		DistinctLevel: logg.LevelWarn,
	})

	flags := config.New()
	flags.Set("workingDir", dir)
	flags.Set("noBuildLock", true)
	flags.Set("cacheDir", filepath.Join(sitesRoot, "_cache"))

	res, err := allconfig.LoadConfig(allconfig.ConfigSourceDescriptor{
		Flags:    flags,
		Fs:       afs,
		Filename: filepath.Join(dir, "hugo.toml"),
		Logger:   logger,
		Environ:  []string{"NEOHUGO_ORACLE=1"},
	})
	if err != nil {
		return nil, fmt.Errorf("%s: load config: %w", s.Name, err)
	}

	hfs := hugofs.NewFrom(afs, res.LoadingInfo.BaseConfig)
	h, err := hugolib.NewHugoSites(deps.DepsCfg{Configs: res, Fs: hfs, LogLevel: logger.Level(), StdErr: &logBuf, StdOut: &logBuf})
	if err != nil {
		return nil, fmt.Errorf("%s: new sites: %w", s.Name, err)
	}
	if err := h.Build(hugolib.BuildCfg{SkipRender: true}); err != nil {
		return nil, fmt.Errorf("%s: build: %w\n%s", s.Name, err, logBuf.String())
	}
	return &builtSite{s: s, h: h, afs: afs, log: &logBuf}, nil
}

// encodeSite is the fixture form of a site: text files as strings, binary
// files as hex.
func encodeSite(s site) map[string]any {
	files := map[string]any{}
	for k, v := range s.Files {
		if !utf8.ValidString(v) {
			panic(k)
		}
		files[k] = v
	}
	bin := map[string]any{}
	for k, v := range s.Bin {
		bin[k] = hex.EncodeToString(v)
	}
	return map[string]any{"name": s.Name, "toml": s.TOML, "files": files, "bin": bin}
}
