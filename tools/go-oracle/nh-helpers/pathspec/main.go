// Command pathspec is the Go oracle for helpers.PathSpec (url.go, path.go,
// pathspec.go) and helpers.ContentSpec in crates/nh-helpers (Wave B task T08).
//
//	go run ./tools/go-oracle/nh-helpers/pathspec [-root .] [-out rust/testdata/oracle/helpers/pathspec]
//
// The seeksnack site is private, so the inputs are this repository's strings:
// the nh-common corpus (every front matter title, taxonomy term, heading and
// file name of docs/content, hugolib/testsite/content and create/skeletons,
// plus adversarial strings), every content path of docs/content and
// hugolib/testsite/content, and adversarial URLs and paths (Thai, spaces,
// %-escapes, "..", trailing slashes, schemes, fragments, queries).
//
// Each setup is a real site configuration loaded with allconfig.LoadConfig
// (the seeksnack one mirrors docs/rust-port/specs/architecture-core-data/
// config-en.json: baseURL https://seeksnack.com/, canonifyURLs, en + th and 9
// disabled languages), and a PathSpec is created per language with
// helpers.NewPathSpec. The full string set runs through the seeksnack setups;
// the other setups (canonifyURLs off, baseURLs with and without a path, port
// or trailing slash, defaultContentLanguageInSubdir, disablePathToLower,
// removePathAccents, uglyURLs, multihost) get the paths and adversarial
// strings; the removePathAccents setup also gets accentStrings and every
// corpus string that is not ASCII. Strings that are not valid UTF-8 are left
// out (the Rust helpers take &str; nh-common deviation 21).
//
// Output: pathspec.json.gz. Nothing here depends on the platform.
package main

import (
	"flag"
	"fmt"
	"io/fs"
	"log"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"unicode/utf8"

	"github.com/neohugo/neohugo/common/loggers"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/helpers"
	"github.com/neohugo/neohugo/hugofs"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/corpus"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-helpers/hsupport"
	"github.com/spf13/afero"
)

type setup struct {
	name string
	toml string
	full bool
}

const langsEnTh = `
defaultContentLanguage = "en"
disableLanguages = ["de", "es", "fr", "ja", "nl", "pl", "pt", "zh-cn", "zh-tw"]
[languages.en]
languageCode = "en"
languageName = "English"
weight = 1
[languages.th]
languageCode = "th"
languageName = "ไทย"
weight = 2
[languages.de]
weight = 6
[languages.es]
weight = 7
[languages.fr]
weight = 3
[languages.ja]
weight = 10
[languages.nl]
weight = 4
[languages.pl]
weight = 5
[languages.pt]
weight = 8
[languages.zh-cn]
weight = 9
[languages.zh-tw]
weight = 11
`

var setups = []setup{
	{name: "seeksnack", full: true, toml: `baseURL = "https://seeksnack.com/"
canonifyURLs = true
titleCaseStyle = "AP"
` + langsEnTh},
	{name: "seeksnack-nocanon", full: true, toml: `baseURL = "https://seeksnack.com/"
` + langsEnTh},
	{name: "subpath", toml: `baseURL = "https://example.org/sub/"
` + langsEnTh},
	{name: "subpath-canon", toml: `baseURL = "https://example.org/sub/"
canonifyURLs = true
` + langsEnTh},
	{name: "subpath-noslash", toml: `baseURL = "https://example.org/a/b"
`},
	{name: "port", toml: `baseURL = "http://localhost:1313"
`},
	{name: "port-path", toml: `baseURL = "http://localhost:1313/docs/"
defaultContentLanguageInSubdir = true
` + langsEnTh},
	{name: "unicode-path", toml: `baseURL = "https://example.com/ภาษา/a%20b/"
`},
	{name: "no-baseurl", toml: `title = "x"
`},
	{name: "subdir", toml: `baseURL = "https://seeksnack.com/"
defaultContentLanguageInSubdir = true
` + langsEnTh},
	{name: "nolower", toml: `baseURL = "https://seeksnack.com/"
disablePathToLower = true
uglyURLs = true
` + langsEnTh},
	{name: "accents", toml: `baseURL = "https://seeksnack.com/"
removePathAccents = true
`},
	{name: "multihost", toml: `defaultContentLanguage = "en"
[languages.en]
baseURL = "https://en.example.com/"
weight = 1
[languages.th]
baseURL = "https://th.example.org/sub/"
weight = 2
`},
	{name: "multihost-canon", toml: `defaultContentLanguage = "th"
canonifyURLs = true
[languages.en]
baseURL = "http://localhost:1313/en"
weight = 2
[languages.th]
baseURL = "https://th.example.org:8443/"
weight = 1
`},
}

var adversarial = []string{
	"", "/", "//", "///", "//example.com/x", "http://example.com", "https://example.com/a/b/",
	"https://seeksnack.com/", "https://seeksnack.com/th/x/", "https://seeksnack.com/x",
	"https://seeksnack.comx/", "http://seeksnack.com/x", "https://example.org/sub/x",
	"https://example.org/sub", "http://localhost:1313/docs/x", "http://localhost:1313",
	"mailto:hugo@rules.com", "webcal://x/cal.ics", "ftp://x", "HTTP://UPPER.COM/X", "javascript:alert(1)",
	"th", "/th", "th/", "/th/", "th/x", "/th/x/", "thx", "/thx", "en", "/en", "/en/", "en/x",
	"x", "/x", "x/", "/x/", "x/y", "/x/y/", "x/y/z.html", "/x/y/z.html", "index.html", "/index.xml",
	"sitemap.xml", "/sitemap.xml", "robots.txt", "css/style.css", "/js/app.min.js",
	"images/a b.jpg", "/images/a b.jpg", "a%20b", "/a%20b/", "%", "%2", "%zz", "%e0%b8%81",
	"/%E0%B8%81/", "a?q=1", "/a?q=1&b=2", "a#frag", "/a/#frag", "?q", "#", "#x", "/a/b?c#d",
	"..", "../", "../x", "/../x", "x/../y", "/x/./y/", "./x", ".", "./", "x//y", "/x//y//",
	"ภาษาไทย", "/ภาษาไทย/", "th/ภาษาไทย/", "ขนม ไทย", "/ขนม-ไทย/หน้า 1/", "café", "/Café/Crème Brûlée/",
	"a b c", " leading", "trailing ", " both ", "a\tb", "a\nb", "UPPER/Case/Path", "/Tags/Lay's/",
	"tags/INS 160a (I)", "/brands/Pepsi-Cola (Thai) Trading Co.,Ltd./", "a+b", "a&b", "a=b", "a;b",
	"a:b", ":", "a:", ":a", "C:\\x", "\\x\\y", "a|b", "a<b>", "a\"b", "a'b", "a`b", "a^b", "a~b",
	"a@b", "a$b", "a!b", "a*b", "a(b)", "a[b]", "a{b}", "😀", "/😀/", "日本語/テスト", "x\x7fy",
	"http://[::1]:1313/x", "http://ex ample.com/", "http://x/%zz", "https://x.com:99999/",
	"//seeksnack.com/x", "/seeksnack.com/x", "seeksnack.com/x", "https:x", "http:/x",
}

// accentStrings are the extra inputs of the removePathAccents setup (with the
// corpus strings that are not ASCII): precomposed and decomposed accents,
// several scripts with combining marks, Hangul, compatibility characters,
// marks without a base, a combining grapheme joiner and mark runs longer than
// x/text's stream-safe limit of 30 non-starters.
var accentStrings = []string{
	"Résumé", "/résumé/", "café", "/Café/Crème Brûlée/", "Ðó ÿöü ßéé þïß?",
	"Ånström", "Ångström", "Ωmega/ἄλφα βῆτα", "Банковский кассир", "Йод и ёж", "Tiếng Việt/Phở bò",
	"Łódź Żółć", "Çà et là", "naïve coöperate", "São Paulo", "Mötley Crüe", "ﬁnance/ﬂow", "Ǆemal", "Ⅻ",
	"각/한국어 문서", "각", "ภาษาไทย/น้ำ ใจ", "हिन्दी/संस्कृत", "עִבְרִית", "العَرَبِيَّة",
	"́", "/́/", "a͏́", "ȩ́", "x" + strings.Repeat("́", 31) + "y",
	"o" + strings.Repeat("̣́", 20), "\U0001d15e\U0001d165", "Z͑ͫ̓ͪ̂ͫ̽͏̴̙̤̞͉͚̯̞̠͍A̴̵̜̰͔ͫ͗͢L̠ͨͧͩ͘G̴̻͈͍͔̹̑͗̎̅͛́Ǫ̵̹̻̝̳͂̌̌͘",
}

// nonASCII returns the strings that have a byte >= 0x80.
func nonASCII(in []string) []string {
	var out []string
	for _, s := range in {
		for i := 0; i < len(s); i++ {
			if s[i] >= utf8.RuneSelf {
				out = append(out, s)
				break
			}
		}
	}
	return out
}

func contentPaths(root string) ([]string, error) {
	var out []string
	for _, r := range []string{"docs/content", "hugolib/testsite/content"} {
		base := filepath.Join(root, r)
		err := filepath.WalkDir(base, func(p string, d fs.DirEntry, err error) error {
			if err != nil {
				return err
			}
			rel, err := filepath.Rel(base, p)
			if err != nil {
				return err
			}
			if rel == "." {
				return nil
			}
			rel = filepath.ToSlash(rel)
			out = append(out, rel, "/"+rel)
			if d.IsDir() {
				out = append(out, rel+"/", "/"+rel+"/")
			}
			return nil
		})
		if err != nil {
			return nil, err
		}
	}
	return out, nil
}

func dedupe(in []string) []string {
	set := map[string]bool{}
	var out []string
	for _, s := range in {
		if !utf8.ValidString(s) || set[s] {
			continue
		}
		set[s] = true
		out = append(out, s)
	}
	sort.Strings(out)
	return out
}

// The PathSpec functions, in fixture order.
var funcNames = []string{
	"MakePath", "MakePathSanitized", "URLize", "URLizeFilename", "URLEscape",
	"AbsURL", "AbsLangURL", "RelURL", "RelLangURL", "PrependBasePath", "PrependBasePathAbs",
	"IsAbsURL", "PermalinkForBaseURL",
}

func runFuncs(p *helpers.PathSpec, s string) []any {
	isAbs := func() string {
		b, err := p.IsAbsURL(s)
		if err != nil {
			return "err: " + err.Error()
		}
		return fmt.Sprint(b)
	}
	return []any{
		hsupport.Call(func() string { return p.MakePath(s) }),
		hsupport.Call(func() string { return p.MakePathSanitized(s) }),
		hsupport.Call(func() string { return p.URLize(s) }),
		hsupport.Call(func() string { return p.URLizeFilename(s) }),
		hsupport.Call(func() string { return p.URLEscape(s) }),
		hsupport.Call(func() string { return p.AbsURL(s, false) }),
		hsupport.Call(func() string { return p.AbsURL(s, true) }),
		hsupport.Call(func() string { return p.RelURL(s, false) }),
		hsupport.Call(func() string { return p.RelURL(s, true) }),
		hsupport.Call(func() string { return p.PrependBasePath(s, false) }),
		hsupport.Call(func() string { return p.PrependBasePath(s, true) }),
		hsupport.Call(isAbs),
		hsupport.Call(func() string { return p.PermalinkForBaseURL(s, p.Cfg.BaseURL().String()) }),
	}
}

func langConfig(conf config.AllProvider, p *helpers.PathSpec) map[string]any {
	b := conf.BaseURL()
	return map[string]any{
		"lang":                    conf.Language().Lang,
		"baseURL":                 b.String(),
		"withPath":                b.WithPath,
		"withoutPath":             b.WithoutPath,
		"basePath":                b.BasePath,
		"basePathNoTrailingSlash": b.BasePathNoTrailingSlash,
		"languagePrefix":          conf.LanguagePrefix(),
		"canonifyURLs":            conf.CanonifyURLs(),
		"disablePathToLower":      conf.DisablePathToLower(),
		"removePathAccents":       conf.RemovePathAccents(),
		"isMultihost":             conf.IsMultihost(),
		"languages":               len(conf.Languages()),
		"statsName":               p.ProcessingStats.Name,
		"getBasePath":             p.GetBasePath(false),
		"getBasePathRel":          p.GetBasePath(true),
		"targetLanguageBasePath":  p.GetTargetLanguageBasePath(),
	}
}

// ContentSpec inputs.
var markupNames = []string{
	"", "md", "markdown", "MARKDOWN", "goldmark", "Goldmark", "mdown", "html", "htm", "asciidoc",
	"asciidocext", "adoc", "ad", "pandoc", "pdc", "rst", "org", "emacs-org-mode", "text", "unknown",
	"json", "rest", "restructuredtext", "mmark",
}

var shortHTML = []string{
	"", "<p>Hello</p>", "<p>Hello</p>\n", "  <p>Hello</p>  \n", "<p>a</p><p>b</p>", "Hello",
	"<p>Hello", "Hello</p>", "<p> spaced </p>", "<div class=\"paragraph\">\n<p>Adoc</p>\n</div>",
	"<div class=\"paragraph\">\n<p>a</p>\n</div><div class=\"paragraph\">\n<p>b</p>\n</div>",
	"<p></p>", "<p>ภาษาไทย</p>", "\t<p>x</p>\u00a0", "<P>upper</P>", "<p>x</p>\n<p>", "<p><p>x</p>",
}

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "rust/testdata/oracle/helpers/pathspec", "output directory")
	flag.Parse()

	strs, err := corpus.Strings(*root)
	if err != nil {
		log.Fatal(err)
	}
	cpaths, err := contentPaths(*root)
	if err != nil {
		log.Fatal(err)
	}
	small := dedupe(append(append([]string{}, adversarial...), cpaths...))
	full := dedupe(append(append(append([]string{}, adversarial...), cpaths...), strs...))

	tmp, err := os.MkdirTemp("", "nh-helpers-pathspec")
	if err != nil {
		log.Fatal(err)
	}
	defer func() { _ = os.RemoveAll(tmp) }()

	var cases []map[string]any
	var setupsOut []any
	for _, st := range setups {
		site, err := hsupport.LoadSite(tmp, st.name, st.toml, nil, nil)
		if err != nil {
			log.Fatalf("%s: %v", st.name, err)
		}
		inputs := small
		if st.full {
			inputs = full
		}
		if conf := site.Confs.GetFirstLanguageConfig(); conf.RemovePathAccents() {
			inputs = dedupe(append(append(append([]string{}, small...), accentStrings...), nonASCII(strs)...))
		}
		var langs []any
		for _, conf := range site.Confs.ConfigLangs() {
			fsys := hugofs.NewFrom(afero.NewOsFs(), conf.BaseConfig())
			p, err := helpers.NewPathSpec(fsys, conf, loggers.NewDefault())
			if err != nil {
				log.Fatalf("%s: %v", st.name, err)
			}
			lang := conf.Language().Lang
			langs = append(langs, langConfig(conf, p))
			for _, s := range inputs {
				cases = append(cases, map[string]any{
					"setup": st.name,
					"lang":  lang,
					"in":    s,
					"r":     runFuncs(p, s),
				})
			}
		}
		setupsOut = append(setupsOut, map[string]any{"name": st.name, "langs": langs})
	}

	// ContentSpec over the seeksnack configuration (default markup config).
	site, err := hsupport.LoadSite(tmp, "contentspec", setups[0].toml, nil, nil)
	if err != nil {
		log.Fatal(err)
	}
	conf := site.Confs.GetFirstLanguageConfig()
	cs, err := helpers.NewContentSpec(conf, loggers.NewDefault(), afero.NewMemMapFs(), nil)
	if err != nil {
		log.Fatal(err)
	}
	for _, s := range markupNames {
		cases = append(cases, map[string]any{
			"cs": "ResolveMarkup", "in": s,
			"r": hsupport.Call(func() string { return cs.ResolveMarkup(s) }),
		})
	}
	for _, s := range full {
		cases = append(cases, map[string]any{
			"cs": "SanitizeAnchorName", "in": s,
			"r": hsupport.Call(func() string { return cs.SanitizeAnchorName(s) }),
		})
	}
	skipped := 0
	for _, s := range strs {
		if !utf8.ValidString(s) {
			skipped++
		}
	}
	for _, h := range shortHTML {
		for _, m := range []string{"markdown", "asciidoc", "html", ""} {
			cases = append(cases, map[string]any{
				"cs": "TrimShortHTML", "in": h, "markup": m,
				"r": hsupport.Str(string(cs.TrimShortHTML([]byte(h), m))),
			})
		}
	}

	header := map[string]any{
		"funcs":              funcNames,
		"setups":             setupsOut,
		"invalidUTF8Skipped": skipped,
	}
	p := filepath.Join(*out, "pathspec.json.gz")
	if err := hsupport.WriteGz(p, header, cases); err != nil {
		log.Fatal(err)
	}
	log.Printf("%s: %d cases (%d full inputs, %d small)", p, len(cases), len(full), len(small))
}
