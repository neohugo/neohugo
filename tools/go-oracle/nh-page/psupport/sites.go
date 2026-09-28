package psupport

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

// The sites every nh-page oracle builds. The seeksnack site is private, so
// they are this repository's two Hugo sites (docs/ and hugolib/testsite) and
// synthetic sites covering what the target paths, permalinks and dates
// depend on: languages (en/th, defaultContentLanguageInSubdir, multihost),
// uglyURLs (global and per section), disablePathToLower, a baseURL with a
// path, every built-in output format plus custom ones (path, baseName,
// isPlainText, noUgly, ugly, root, permalinkable, protocol), sections,
// bundles, taxonomies and terms with Thai, accented, upper case, '&' and
// "'" values, url and slug front matter (trailing slashes, "..", Unicode,
// ":" patterns), permalinks with every token, and front matter dates in every
// form (YAML, TOML, JSON; strings in the layouts cast accepts, filename
// dates, time zones).

// siteCommon is the shared configuration of the synthetic sites.
const siteCommon = `
title = "Oracle"
enableRobotsTXT = true
[pagination]
pagerSize = 2
[taxonomies]
tag = "tags"
category = "categories"
company = "companies"
brand = "brands"
`

const langsEnTh = `
defaultContentLanguage = "en"
[languages.en]
languageName = "English"
weight = 1
[languages.th]
languageName = "ไทย"
weight = 2
`

// customFormats are the custom output formats (and the media type one of
// them needs) of the synthetic "full" sites.
const customFormats = `
[mediaTypes."text/netlify"]
delimiter = ""
[outputFormats.api]
mediaType = "application/json"
path = "api"
baseName = "data"
isPlainText = true
[outputFormats.redir]
mediaType = "text/netlify"
baseName = "_redirects"
isPlainText = true
notAlternative = true
[outputFormats.nougly]
mediaType = "text/html"
path = "nu"
noUgly = true
isHTML = true
[outputFormats.uglyfmt]
mediaType = "text/html"
path = "u"
ugly = true
isHTML = true
[outputFormats.rootfmt]
mediaType = "text/plain"
baseName = "rootfile"
root = true
isPlainText = true
[outputFormats.perma]
mediaType = "text/html"
path = "perma"
permalinkable = true
isHTML = true
[outputFormats.feed]
mediaType = "application/rss+xml"
baseName = "feed"
[outputFormats.webcal]
mediaType = "text/calendar"
protocol = "webcal://"
baseName = "events"
isPlainText = true
[outputs]
home = ["html", "rss", "json", "api", "redir", "rootfmt", "calendar", "webcal", "amp", "markdown", "csv"]
section = ["html", "rss", "perma", "nougly", "uglyfmt", "feed"]
page = ["html", "amp", "json", "perma", "nougly", "uglyfmt", "api", "markdown"]
taxonomy = ["html", "rss", "json", "css"]
term = ["html", "rss", "feed"]
`

// permalinksAll uses every permalink token.
const permalinksAll = `
[permalinks]
posts = "/posts/:year/:month/:title"
[permalinks.page]
blog = "/:sections[1:]/:year/:monthname/:day/:slug/"
news = "/n/:year-:month-:day/:weekday-:weekdayname-:yearday/:filename/"
docs = "/d/:sections/:slugorfilename/"
misc = "/m/:section/:contentbasename/:slugorcontentbasename/"
dated = "/dated/:2006/:Jan/:02/:title/"
esc = "/e/\\:literal/:sections[last]/:sections[0]/:sections[:1]/:sections[1]/"
[permalinks.section]
blog = "/b/:section/"
news = "/news-section/:sections/"
[permalinks.taxonomy]
tags = "/topics/"
[permalinks.term]
tags = "/topics/:slug/"
categories = "/cats/:title/"
`

func fm(lines ...string) string {
	return "---\n" + strings.Join(lines, "\n") + "\n---\n"
}

func body(title string) string {
	return fmt.Sprintf("\nFirst paragraph of %s with some words to count in the summary here.\n\nSecond paragraph <!--more--> with more text.\n\n## Heading\n\nThird paragraph with a [link](https://example.com).\n", title)
}

// SyntheticContent is the content of the synthetic sites (en and th).
func SyntheticContent() map[string]string {
	c := map[string]string{
		"content/_index.md":                                               fm(`title: "Home"`, `date: 2021-06-01T10:00:00Z`) + body("home"),
		"content/_index.th.md":                                            fm(`title: "หน้าแรก"`) + body("th home"),
		"content/about.md":                                                fm(`title: "About Us"`, `date: "2020-02-03"`) + body("about"),
		"content/about.th.md":                                             fm(`title: "เกี่ยวกับ"`, `date: "2020-02-04T08:00:00+07:00"`) + body("about th"),
		"content/blog/_index.md":                                          fm(`title: "The Blog"`) + body("blog"),
		"content/blog/_index.th.md":                                       fm(`title: "บล็อก"`) + body("blog th"),
		"content/blog/post-one.md":                                        fm(`title: "Post One: A Tale"`, `date: 2023-09-25T14:45:00.876Z`, `tags: ["Go", "Lay's", "คริสปี้พาย ", "INS 160a (I)"]`, `categories: ["no-salt/low-salt-chips", "Snacks & Chips"]`, `companies: ["Berli Jucker Foods Ltd.,Berli Jucker PLC"]`, `brands: ["Lay's", "Le Pan"]`, `aliases: ["/old/post-one/"]`) + body("post one"),
		"content/blog/post-one.th.md":                                     fm(`title: "โพสต์หนึ่ง"`, `date: 2023-09-26`, `tags: ["คริสปี้พาย ", "ขนม"]`) + body("post one th"),
		"content/blog/2023-01-02-dated-post.md":                           fm(`title: "Dated"`, `lastmod: 2023-05-06T07:08:09+02:00`) + body("dated"),
		"content/blog/2022-11-12-13-14-15-dated-time.md":                  fm(`title: "Dated Time"`, `slug: "custom-slug"`) + body("dated time"),
		"content/blog/2021-13-45-bad-date.md":                             fm(`title: "Bad Date"`) + body("bad date"),
		"content/blog/sub/_index.md":                                      fm(`title: "Sub"`) + body("sub"),
		"content/blog/sub/deep/page.md":                                   fm(`title: "Deep Page"`, `date: "2019-07-08 09:10"`, `slug: "Deep Slug Ü"`) + body("deep"),
		"content/blog/bundle-leaf/index.md":                               fm(`title: "Leaf Bundle"`, `publishDate: 2020-01-01T00:00:00Z`, `expiryDate: 2099-01-01T00:00:00Z`) + body("leaf"),
		"content/blog/bundle-leaf/image.jpg":                              "not really a jpeg",
		"content/blog/bundle-leaf/data.json":                              `{"a": 1}`,
		"content/blog/bundle-leaf/sub/nested.txt":                         "nested",
		"content/blog/branch/_index.md":                                   fm(`title: "Branch Bundle"`) + body("branch"),
		"content/blog/branch/child.md":                                    fm(`title: "Child"`, `date: "Mon, 02 Jan 2006 15:04:05 MST"`) + body("child"),
		"content/blog/branch/res.txt":                                     "resource",
		"content/posts/p1.md":                                             fm(`title: "Legacy Post Ünicode"`, `date: 2018-03-04T05:06:07Z`) + body("p1"),
		"content/news/_index.md":                                          fm(`title: "News"`) + body("news"),
		"content/news/2024/story.md":                                      fm(`title: "Story"`, `date: "2024-02-29T23:59:59-05:00"`) + body("story"),
		"content/docs/_index.md":                                          fm(`title: "Docs"`) + body("docs"),
		"content/docs/guide/_index.md":                                    fm(`title: "Guide"`) + body("guide"),
		"content/docs/guide/install.md":                                   fm(`title: "Install"`, `date: "02 Jan 06 15:04 -0700"`) + body("install"),
		"content/docs/guide/leaf/index.md":                                fm(`title: "Guide Leaf"`) + body("guide leaf"),
		"content/misc/Mixed Case File.md":                                 fm(`title: "Mixed"`, `date: "2006-01-02T15:04:05"`) + body("mixed"),
		"content/misc/with.dots.in.name.md":                               fm(`title: "Dots"`) + body("dots"),
		"content/dated/entry.md":                                          fm(`title: "Entry Title"`, `date: 2017-12-31T23:00:00Z`) + body("entry"),
		"content/esc/a/b/c.md":                                            fm(`title: "Escaped"`, `date: 2016-01-01`) + body("esc"),
		"content/Upper Case Dir/Some Page.md":                             fm(`title: "Upper"`) + body("upper"),
		"content/biscuit/_index.md":                                       fm(`title: "Biscuit"`) + body("biscuit"),
		"content/biscuit/_index.th.md":                                    fm(`title: "บิสกิต"`) + body("biscuit th"),
		"content/biscuit/koalas-march-chocolate/index.md":                 fm(`title: "Koala's March Chocolate"`, `date: 2020-12-10T12:45:19.622Z`, `brands: ["Lotte"]`) + body("koala"),
		"content/biscuit/koalas-march-chocolate/index.th.md":              fm(`title: "โคอะลา"`, `date: 2020-12-10T12:45:19.622Z`) + body("koala th"),
		"content/potato-chips/_index.md":                                  fm(`title: "Potato Chips"`) + body("chips"),
		"content/potato-chips/herrs-salt-&-vinegar-potato-chips/index.md": fm(`title: "Herr's Salt & Vinegar"`, `tags: ["Herr's"]`) + body("herrs"),
		"content/potato-chips/wise-chili-olé-chili-&-spice-flavor-potato-chips/index.md": fm(`title: "Wise Chili Olé"`) + body("wise"),
		"content/disclaimer.md":                    fm(`title: "Disclaimer"`) + body("disclaimer"),
		"content/url-pages/trailing.md":            fm(`title: "U1"`, `url: "/custom/trailing/"`) + body("u1"),
		"content/url-pages/notrailing.md":          fm(`title: "U2"`, `url: "/custom/notrailing"`) + body("u2"),
		"content/url-pages/html.md":                fm(`title: "U3"`, `url: "/custom/page.html"`) + body("u3"),
		"content/url-pages/relative.md":            fm(`title: "U4"`, `url: "relative/path/"`) + body("u4"),
		"content/url-pages/dotdot.md":              fm(`title: "U5"`, `url: "/a/../../escape/"`) + body("u5"),
		"content/url-pages/unicode.md":             fm(`title: "U6"`, `url: "/ภาษา/หน้า Ü/"`) + body("u6"),
		"content/url-pages/pattern.md":             fm(`title: "U7 Pattern"`, `date: 2015-05-05`, `url: "/:year/:month/:slug/"`, `slug: "patterned"`) + body("u7"),
		"content/url-pages/hash.md":                fm(`title: "U8"`, `url: "/with#hash/"`) + body("u8"),
		"content/url-pages/xml.md":                 fm(`title: "U9"`, `url: "/feeds/custom.xml"`) + body("u9"),
		"content/url-pages/root.md":                fm(`title: "U10"`, `url: "/"`) + body("u10"),
		"content/url-pages/upper.md":               fm(`title: "U11"`, `url: "/UPPER/Case/"`) + body("u11"),
		"content/url-pages/space.md":               fm(`title: "U12"`, `url: "/with space/x/"`) + body("u12"),
		"content/url-pages/th-url.th.md":           fm(`title: "U13"`, `url: "/th/already-prefixed/"`) + body("u13"),
		"content/url-pages/th-rel.th.md":           fm(`title: "U14"`, `url: "rel-th/"`) + body("u14"),
		"content/slugs/s1.md":                      fm(`title: "S1"`, `slug: "Slug With Spaces"`) + body("s1"),
		"content/slugs/s2.md":                      fm(`title: "S2"`, `slug: "ขนมไทย"`) + body("s2"),
		"content/slugs/s3.md":                      fm(`title: "S3"`, `slug: "a/b"`) + body("s3"),
		"content/slugs/s4.md":                      fm(`title: "S4"`, `slug: "index"`) + body("s4"),
		"content/slugs/bundle/index.md":            fm(`title: "S5"`, `slug: "bundle-slug"`) + body("s5"),
		"content/builds/render-never.md":           fm(`title: "Never"`, `build: {render: never}`) + body("never"),
		"content/builds/render-link.md":            fm(`title: "Link"`, `build: {render: link, list: local}`) + body("link"),
		"content/builds/headless/index.md":         fm(`title: "Headless"`, `headless: true`) + body("headless"),
		"content/dates/yaml-ts.md":                 fm(`title: "D1"`, `date: 2001-02-03T04:05:06+07:00`, `lastmod: 2001-03-03`, `publishdate: 2001-01-01T00:00:00Z`) + body("d1"),
		"content/dates/aliases.md":                 fm(`title: "D2"`, `pubdate: "2002-02-02"`, `modified: "2002-03-03T10:00:00Z"`, `unpublishdate: "2099-12-31"`) + body("d2"),
		"content/dates/published.md":               fm(`title: "D3"`, `published: "Jan 2, 2006"`, `expirydate: ""`) + body("d3"),
		"content/dates/bad.md":                     fm(`title: "D4"`, `date: "not a date"`, `lastmod: "2003-04-05"`) + body("d4"),
		"content/dates/epoch.md":                   fm(`title: "D5"`, `date: 1136214245`) + body("d5"),
		"content/dates/rfc822.md":                  fm(`title: "D6"`, `date: "02 Jan 06 15:04 MST"`) + body("d6"),
		"content/dates/datetime-local.md":          fm(`title: "D7"`, `date: "2006-01-02 15:04:05.999"`) + body("d7"),
		"content/dates/toml.md":                    "+++\ntitle = \"D8\"\ndate = 2008-08-08\nlastmod = 2008-09-09T10:11:12\npublishDate = 2008-07-07T01:02:03+09:00\nexpiryDate = 2098-01-01T00:00:00Z\n+++\n" + body("d8"),
		"content/dates/toml-string.md":             "+++\ntitle = \"D9\"\ndate = \"2009-09-09\"\n+++\n" + body("d9"),
		"content/dates/json.md":                    "{\n\"title\": \"D10\",\n\"date\": \"2010-10-10T10:10:10+10:00\",\n\"lastmod\": \"2010-11-11\"\n}\n" + body("d10"),
		"content/dates/2012-12-12-fromname.md":     fm(`title: "D11"`) + body("d11"),
		"content/dates/2013-01-01_underscore.md":   fm(`title: "D12"`, `slug: ""`) + body("d12"),
		"content/dates/2014-02-03 spaced.md":       fm(`title: "D13"`) + body("d13"),
		"content/dates/2015-03-04T05-06-07.md":     fm(`title: "D14"`) + body("d14"),
		"content/dates/nodate.md":                  fm(`title: "D15"`) + body("d15"),
		"content/dates/emptydate.md":               fm(`title: "D16"`, `date: ""`, `publishDate: null`) + body("d16"),
		"content/dates/2016-05-05-bundle/index.md": fm(`title: "D17"`) + body("d17"),
		"content/dates/time-only.md":               fm(`title: "D18"`, `date: "15:04:05"`) + body("d18"),
		"content/dates/iso-week.md":                fm(`title: "D19"`, `date: "2006-01-02T15:04:05.000Z"`, `lastmod: "2006-01-02T15:04:05-07:00"`) + body("d19"),
		"content/cjk/chinese.md":                   fm(`title: "CJK"`, `isCJKLanguage: true`) + "\n中文内容测试。这是一个很长的段落，用于测试摘要的字数计算。\n\n第二段内容。\n",
		"content/summaries/manual.md":              fm(`title: "Manual"`) + "\nIntro text.\n\n<!--more-->\n\nRest of it.\n",
		"content/summaries/manual-inline.md":       fm(`title: "Manual Inline"`) + "\nIntro text <!--more--> continues here.\n\nMore.\n",
		"content/summaries/frontmatter.md":         fm(`title: "FM Summary"`, `summary: "This is the **summary** from front matter."`) + body("fm summary"),
		"content/summaries/short.md":               fm(`title: "Short"`) + "\nShort.\n",
		"content/summaries/html.html":              fm(`title: "HTML Page"`) + "<p>An HTML page <!--more--> with a divider.</p>\n<p>Another.</p>\n",
		"content/summaries/long.md":                fm(`title: "Long"`) + "\n" + strings.Repeat("word ", 150) + "\n\n" + strings.Repeat("more ", 50) + "\n",
	}
	return c
}

// SyntheticSites are the synthetic site configurations (all with
// SyntheticContent and the oracle layouts).
func SyntheticSites() []Site {
	mk := func(name, toml string) Site {
		files := SyntheticContent()
		for k, v := range Layouts() {
			files[k] = v
		}
		return Site{Name: name, TOML: toml, Files: files}
	}
	return []Site{
		mk("seeksnack", `baseURL = "https://seeksnack.com/"
canonifyURLs = true
`+siteCommon+langsEnTh+`
[permalinks]
posts = "/posts/:year/:month/:title"
[outputs]
home = ["html", "json", "rss"]
page = ["html"]
section = ["html", "rss"]
taxonomy = ["html", "rss"]
term = ["html", "rss"]
`),
		mk("subdir", `baseURL = "https://example.com/"
defaultContentLanguageInSubdir = true
timeZone = "Asia/Bangkok"
`+siteCommon+langsEnTh+customFormats+permalinksAll),
		mk("ugly", `baseURL = "https://example.com/"
uglyURLs = true
`+siteCommon+langsEnTh+customFormats+permalinksAll),
		mk("ugly-section", `baseURL = "https://example.com/"
[uglyURLs]
blog = true
docs = false
`+siteCommon+langsEnTh),
		mk("multihost", `defaultContentLanguage = "th"
`+siteCommon+`
[languages.en]
baseURL = "https://en.example.com/"
weight = 2
timeZone = "America/New_York"
[languages.th]
baseURL = "https://th.example.org/sub/"
weight = 1
`+customFormats+permalinksAll),
		mk("nolower", `baseURL = "https://example.com/"
disablePathToLower = true
`+siteCommon+langsEnTh+permalinksAll),
		mk("subpath", `baseURL = "https://example.org/sub/path/"
`+siteCommon+langsEnTh+customFormats),
	}
}

// RepoSites are this repository's sites: docs/ (docs/hugo.toml, the English
// content) and hugolib/testsite (en + nn), with the oracle layouts and a
// template for every shortcode their content uses.
func RepoSites(root string) ([]Site, error) {
	docs, err := ReadTree(root, "docs/content/en", "content/en/")
	if err != nil {
		return nil, err
	}
	docsTOML, err := os.ReadFile(filepath.Join(root, "docs", "hugo.toml"))
	if err != nil {
		return nil, err
	}
	docs["hugo.toml"] = string(docsTOML)
	// A mount source (module.mounts) that must exist.
	docs["hugo_stats.json"] = "{}\n"
	for k := range docs {
		// Content adapters that fetch remote data (the news section).
		if strings.HasSuffix(k, "_content.gotmpl") {
			delete(docs, k)
		}
	}
	for k, v := range ShortcodeStubs(ShortcodeNames(docs)) {
		docs[k] = v
	}
	for k, v := range Layouts() {
		docs[k] = v
	}
	// The docs site enables the embedded link and image render hooks, which
	// fail on some reference-style destinations without the site's own
	// templates; plain hooks instead.
	docs["layouts/_markup/render-link.html"] = `<a href="{{ .Destination | safeURL }}">{{ .Text }}</a>`
	docs["layouts/_markup/render-image.html"] = `<img src="{{ .Destination | safeURL }}" alt="{{ .PlainText }}">`
	toml := docs["hugo.toml"]
	delete(docs, "hugo.toml")
	// The build stats writer writes hugo_stats.json through the OS working
	// directory; the oracle builds in memory.
	toml = strings.Replace(toml, "disableIDs = true\n    enable     = true", "disableIDs = true\n    enable     = false", 1)

	ts, err := ReadTree(root, filepath.Join("hugolib", "testsite"), "")
	if err != nil {
		return nil, err
	}
	for k, v := range Layouts() {
		ts[k] = v
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
		{Name: "docs", TOML: toml, Files: docs},
		{Name: "testsite", TOML: tsTOML, Files: ts},
	}, nil
}
