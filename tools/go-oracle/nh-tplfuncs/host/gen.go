package main

import (
	"html/template"
	"math"
	"strings"
	"time"

	"github.com/neohugo/neohugo/common/maps"
)

// genStrings: the strings namespace over the value corpus, the texts and
// argument matrices (regexps, truncate, repeat limits, Thai and CJK text).
func genStrings(o *oracle) {
	const e = "en/0"
	unary := []string{
		"Chomp", "CountRunes", "RuneCount", "CountWords", "ContainsNonSpace", "FirstUpper",
		"ToLower", "ToUpper", "TrimSpace", "Title",
	}
	for _, m := range unary {
		for _, v := range scalars() {
			o.add("strings_unary", e, "strings."+m, v)
		}
		for _, s := range texts() {
			o.add("strings_unary", e, "strings."+m, s)
		}
	}
	// Title in the title case styles and Thai.
	for _, env := range []string{"ap/0", "chicago/0", "gostyle/0", "firstupper/0", "th/0"} {
		for _, s := range texts() {
			o.add("strings_title", env, "strings.Title", s)
		}
		for _, v := range []any{nil, 42, template.HTML("<b>a tale</b>"), []any{1}} {
			o.add("strings_title", env, "strings.Title", v)
		}
	}
	binary := []string{"Contains", "ContainsAny", "Count", "HasPrefix", "HasSuffix", "Trim", "TrimLeft", "TrimPrefix", "TrimRight", "TrimSuffix"}
	for _, m := range binary {
		for _, a := range small() {
			for _, b := range small() {
				o.add("strings_binary", e, "strings."+m, a, b)
			}
		}
		for _, s := range texts() {
			for _, b := range []any{"a", " ", "ข้", "o", "<", "", "Hello"} {
				o.add("strings_binary", e, "strings."+m, s, b)
				o.add("strings_binary", e, "strings."+m, b, s)
			}
		}
	}
	// Split: the delimiter is a typed string parameter.
	for _, s := range append(texts(), "a,b,,c", ",", "") {
		for _, d := range []any{",", " ", "", "ข้", "\xff", 1, nil, template.HTML(",")} {
			o.add("strings_split", e, "strings.Split", s, d)
		}
	}
	for _, v := range small() {
		o.add("strings_split", e, "strings.Split", v, ",")
	}
	o.add("strings_split", e, "strings.Split", "a")
	o.add("strings_split", e, "strings.Split", "a", ",", ",")
	// Replace with and without limits.
	for _, s := range []any{"aab", "11a11", 12345, "ข้าวข้าว", "", "\xffa\xff", template.HTML("<a>")} {
		for _, old := range []any{"a", 1, "", "ข้", "\xff", nil} {
			for _, nw := range []any{"b", 2, "", "<x>"} {
				o.add("strings_replace", e, "strings.Replace", s, old, nw)
				for _, lim := range []any{-1, 0, 1, 2, "1", 1.5, "x", nil, int64(math.MaxInt64)} {
					o.add("strings_replace", e, "strings.Replace", s, old, nw, lim)
				}
			}
		}
	}
	o.add("strings_replace", e, "strings.Replace", "a", "b")
	o.add("strings_replace", e, "strings.Replace", "a", "b", "c", 1, 2)
	// SliceString and Substr.
	idx := []any{nil, 0, 1, 2, 3, 6, 7, -1, -3, -100, 100, int8(2), int64(3), 2.0, 2.5, "2", "x", uint(1), int64(math.MaxInt64), int64(math.MinInt64)}
	for _, s := range []any{"abcdef", "ĀĀĀ", "ข้าวผัด", "", 123, 1.2e3, "\xff\xfeab", template.HTML("<b>x</b>")} {
		o.add("strings_slice", e, "strings.SliceString", s)
		o.add("strings_slice", e, "strings.Substr", s)
		for _, a := range idx {
			o.add("strings_slice", e, "strings.SliceString", s, a)
			o.add("strings_slice", e, "strings.Substr", s, a)
			for _, b := range idx {
				o.add("strings_slice", e, "strings.SliceString", s, a, b)
				o.add("strings_slice", e, "strings.Substr", s, a, b)
			}
		}
		o.add("strings_slice", e, "strings.SliceString", s, 0, 1, 2)
		o.add("strings_slice", e, "strings.Substr", s, 0, 1, 2)
	}
	// Repeat, including its limits.
	for _, s := range []any{"yo", "~", "", "ข้าว", 1221, template.HTML("<tag>"), nil, "a"} {
		for _, n := range []any{0, 1, 2, 16, -1, "2", "x", nil, 2.5, int64(math.MaxInt64), int64(1) << 62, int64(1) << 49, int64(1)<<48 + 1} {
			o.add("strings_repeat", e, "strings.Repeat", n, s)
		}
	}
	// Diff.
	docs := []any{"", "foo\n", "bar\n", "foo", "a\nb\nc\nd\ne\nf\ng\nh\ni\n", "a\nb\nX\nd\ne\nf\ng\nh\nY\n", "a\nb\nc\n", "c\nb\na\n", "x\ny\nx\ny\nx\n", nil, 42, "ข้าว\nผัด\n", "\xff\n"}
	for _, a := range docs {
		for _, b := range docs {
			o.add("strings_diff", e, "strings.Diff", "old", a, "new", b)
		}
	}
	o.add("strings_diff", e, "strings.Diff", 1, "a", "new", "b")
	o.add("strings_diff", e, "strings.Diff", "old", "a", nil, "b")
	o.add("strings_diff", e, "strings.Diff", "old", "a", "new")
	// The regexp family.
	patterns := []any{"[G|g]o", "([G|g]o)", "(?i)hugo", "^", "$", "", "a*", "(a)|(b)", "\\p{Thai}+", "(ข้)(า)", "[", "(?P<name>\\w+)", "\\bfoo\\b", "x*?", 1, nil, template.HTML("a")}
	inputs := []any{"Hugo is a static site generator written in Go.", "", "aaa", "abab", "ข้าวผัด ข้าว", "foo bar foo", "\xffgo\xfe", 42, nil, []string{"a"}}
	for _, p := range patterns {
		for _, in := range inputs {
			o.add("strings_regexp", e, "strings.FindRE", p, in)
			o.add("strings_regexp", e, "strings.FindRESubmatch", p, in)
			for _, lim := range []any{-1, 0, 1, 2, "1", nil, "x"} {
				o.add("strings_regexp", e, "strings.FindRE", p, in, lim)
				o.add("strings_regexp", e, "strings.FindRESubmatch", p, in, lim)
			}
			for _, repl := range []any{"X", "$1", "${1}x", "$name", "$2", "", 7} {
				o.add("strings_regexp", e, "strings.ReplaceRE", p, repl, in)
				for _, n := range []any{-1, 0, 1, "1", nil} {
					o.add("strings_regexp", e, "strings.ReplaceRE", p, repl, in, n)
				}
			}
		}
	}
	o.add("strings_regexp", e, "strings.FindRE", "a")
	o.add("strings_regexp", e, "strings.ReplaceRE", "a", "b")
	// Truncate: plain text and HTML, ellipsis, CJK.
	ts := []any{
		"I am a test sentence", "", "a b c d e f g h i j k", "<b>Should be escaped</b>", "Hello中国 Good 好的",
		"ข้าวผัดกุ้ง อร่อยมาก ข้าวมันไก่", "IamanextremelylongwordthatjustgoesonandonandonjusttoannoyyoualmostasifIwaswritteninGerman",
		template.HTML("I have a <a href='/markdown'>Markdown link</a> inside."),
		template.HTML("<p>test <b>hello</b> test something</p>"), template.HTML("<p>a<b><i>b</b>c d e</p>"),
		template.HTML(strings.Repeat("<p>P</p>", 20)), template.HTML("A <br> tag that's not closed"),
		template.HTML("<p>Hello中国 Good 好的</p>"), template.HTML("With <a href=\"/markdown\">Markdown</a> inside."),
		template.HTML("<img src=\"x\"/> image and <hr/> rule then words"), template.HTML("<\t>odd tag</\t> words here"),
		"\xff\xfe\xfd\xfc words", template.HTML("\xff<b>x</b> y z"), 12345678901234, nil,
	}
	for _, n := range []any{0, 1, 2, 3, 4, 5, 10, 13, 14, 18, 20, 100, -1, "5", nil, 2.5} {
		for _, t := range ts {
			o.add("strings_truncate", e, "strings.Truncate", n, t)
			for _, el := range []any{"", "…", " <more>", template.HTML(" <a href='#'>Read more</a>"), 1, nil} {
				o.add("strings_truncate", e, "strings.Truncate", n, el, t)
			}
		}
	}
	o.add("strings_truncate", e, "strings.Truncate", 10)
	o.add("strings_truncate", e, "strings.Truncate", 10, "a", "b", "c")
	// The func map aliases.
	for _, f := range []string{"chomp", "countrunes", "countwords", "lower", "upper", "title", "trim", "hasPrefix", "hasSuffix", "slicestr", "substr", "split", "replace", "replaceRE", "findRE", "findRESubmatch", "truncate"} {
		o.add("strings_aliases", e, "f:"+f, "Hello World", "o")
		o.add("strings_aliases", e, "f:"+f, 3, "Hello World")
		o.add("strings_aliases", e, "f:"+f, "Hello World")
	}
}

// genPath: the path namespace.
func genPath(o *oracle) {
	const e = "en/0"
	ps := []any{
		"foo/bar.txt", "foo/bar/txt ", "foo/bar.t", "foo.bar.txt", ".x", "", "/", "//", "/a/b/", "a/../b", "./a", "../../x",
		"C:\\foo\\bar.txt", "ข้าว/ผัด.md", "\xff/\xfe.x", "a//b.tar.gz", "/.hidden", "noext", 42, 1.5, nil, template.HTML("a/b.html"),
		[]string{"a"}, true,
	}
	for _, m := range []string{"Base", "BaseName", "Clean", "Dir", "Ext", "Split"} {
		for _, p := range ps {
			o.add("path", e, "path."+m, p)
		}
		o.add("path", e, "path."+m)
		o.add("path", e, "path."+m, "a", "b")
	}
	joins := [][]any{
		{}, {"a"}, {"my", "path", "filename.txt"}, {[]string{"my", "path"}, "filename.txt"}, {[]any{"a", "b"}, "c"},
		{"", ""}, {"/a", "../b"}, {nil}, {[]any{nil}}, {1, 2.5}, {[]string(nil), "x"}, {[]any(nil)}, {"ข้าว", "/ผัด/"},
		{[]any{"a", []string{"b"}}}, {template.HTML("a"), "b"}, {map[string]any{"a": 1}},
	}
	for _, j := range joins {
		o.add("path", e, "path.Join", j...)
	}
}

// genURLs: the urls namespace on every site (baseURL with and without a
// path, canonifyURLs, multilingual, multihost).
func genURLs(o *oracle) {
	envs := []string{"en/0", "sub/0", "canon/0", "multi/0", "multi/1", "multisub/0", "multisub/1", "multihost/0", "multihost/1", "th/0"}
	in := []any{
		"", "/", "foo", "/foo", "foo/", "/foo/bar/", "foo/bar.css", "https://other.com/x", "http://example.org/a", "//cdn.example.com/a.js",
		"#frag", "?q=1", "a b", "ข้าว/ผัด", "/sub/foo", "/docs/x", "mailto:a@b.c", "../x", "./x", "Foo Bar!", "/en/foo", "/th/foo", "th/x",
		"\xffbad", "%41", "a?b#c", 42, nil, template.HTML("/h"), template.URL("/u"), []any{"x"}, "data:image/png;base64,AAA",
	}
	for _, env := range envs {
		for _, m := range []string{"AbsURL", "RelURL", "AbsLangURL", "RelLangURL", "URLize", "Anchorize"} {
			for _, s := range in {
				o.add("urls_"+strings.ReplaceAll(env, "/", ""), env, "urls."+m, s)
			}
		}
	}
	const e = "en/0"
	for _, s := range in {
		o.add("urls", e, "urls.Parse", s)
		o.add("urls", e, "urls.URLEncode", s)
		o.add("urls", e, "urls.URLDecode", s)
		o.add("urls", e, "urls.JoinPath", s)
		o.add("urls", e, "urls.JoinPath", s, "b")
		o.add("urls", e, "urls.JoinPath", "https://example.org/a/", s)
	}
	for _, s := range []any{"https://user:pw@example.org:8080/a%20b/c?x=1&y=2#frag", "http://[::1]:80/x", "rel/path?q", "::bad", "http://a b.com/", "https://example.org/%zz", "mailto:x@y", "/p?a=1&a=2&b=%41"} {
		o.add("urls", e, "urls.Parse", s)
	}
	for _, s := range []any{"a%20b+c", "%zz", "%", "a+b", "ข้าว%E0%B8%82", "%E0%B8"} {
		o.add("urls", e, "urls.URLDecode", s)
	}
	for _, j := range [][]any{{}, {[]any{"", "a"}}, {[]string{"/a", "b"}}, {[]any{".", "..", "/a", "b"}}, {[]any{nil}}, {"https://example.org", "../a"}, {"%zz", "a"}, {[]string(nil)}} {
		o.add("urls", e, "urls.JoinPath", j...)
	}
	// Ref and RelRef from a page (the page's Ref resolves against its site).
	for _, env := range []string{"en/0", "multi/0", "multi/1", "multihost/1", "sub/0"} {
		refs := []any{
			"/a/p1", "a/p2.md", "/b/q1", "p1", "/a/p1#frag", "#frag", "/missing", "", nil, 42,
			[]any{"/a/p1"}, []any{"/a/p1", "json"}, []any{"/a/p1", "html"}, []any{}, []any{"a", "b", "c"}, []string{"/a/p2"},
			map[string]any{"path": "/a/p1"}, map[string]any{"path": "/a/p1", "lang": "th"}, map[string]any{"path": "/a/p1", "outputFormat": "rss"},
			map[string]string{"path": "/b/q1"}, maps.Params{"path": "/a/p1"},
		}
		for _, r := range refs {
			o.add("urls_ref", env, "urls.Ref", pageArg{"/a/p1"}, r)
			o.add("urls_ref", env, "urls.RelRef", pageArg{"/a/p1"}, r)
		}
		o.add("urls_ref", env, "urls.Ref", "not a page", "/a/p1")
		o.add("urls_ref", env, "urls.RelRef", nil, "/a/p1")
		o.add("urls_ref", env, "f:ref", pageArg{"/b/q1"}, "/a/p2")
		o.add("urls_ref", env, "f:relref", pageArg{"/"}, "/a/p2")
	}
}

// genTime: the time namespace (en and th formatters and locations).
func genTime(o *oracle) {
	layouts := []any{
		"2006-01-02", "Monday, January 2, 2006", "Mon Jan 2 15:04:05 MST 2006", "Jan 2006", "January", "Monday", "Mon",
		"2 January 2006", time.RFC3339, time.RFC3339Nano, time.RFC1123Z, time.Kitchen, "15:04:05.000", "Z07:00", "-0700", "MST",
		":date_full", ":date_long", ":date_medium", ":date_short", ":time_full", ":time_long", ":time_medium", ":time_short",
		":DATE_LONG", ":unknown", "", "no layout", "Jan Monday January Mon", "\xff 2006",
	}
	vals := []any{
		t1, t2, t3, t4, zero, "2021-03-04", "2021-03-04T05:06:07+07:00", "2021-03-04T05:06:07.123456789Z", "2021-03-04 05:06:07",
		"04 Mar 21 05:06 +0700", "invalid", "", 1614834367, int64(1614834367), 1.5, nil, true, "2006-01-02T15:04:05", "1st of May",
	}
	for _, env := range []string{"en/0", "th/0", "multi/1"} {
		for _, l := range layouts {
			for _, v := range vals {
				o.add("time_format", env, "time.Format", l, v)
			}
		}
		o.add("time_format", env, "time.Format", 1, t1)
		o.add("time_format", env, "f:dateFormat", "Monday, Jan 2, 2006", "2015-01-21")
		for _, v := range vals {
			o.add("time_astime", env, "time.AsTime", v)
			for _, loc := range []any{"UTC", "Asia/Bangkok", "America/New_York", "Europe/Oslo", "", "Local", "Invalid/Zone", 1, nil} {
				o.add("time_astime", env, "time.AsTime", v, loc)
			}
			o.add("time_astime", env, "f:time", v)
			o.add("time_astime", env, "f:time", v, "Asia/Bangkok")
		}
		o.add("time_astime", env, "f:time", t1, "UTC", 1)
	}
	const e = "en/0"
	for _, z := range []any{"", "UTC", "Asia/Bangkok", "America/New_York", "Asia/Kolkata", "Invalid/Zone", "Local", 1, nil} {
		for _, t := range []any{t1, t2, t4, zero, "2021-01-01", nil} {
			o.add("time_in", e, "time.In", z, t)
		}
	}
	units := []any{"nanosecond", "ns", "microsecond", "us", "µs", "millisecond", "ms", "second", "s", "minute", "m", "hour", "h", "day", "", 1, nil, "S"}
	nums := []any{0, 1, -1, 60 * 60, 1.5, "30", "x", nil, int64(math.MaxInt64), int64(math.MinInt64), uint64(math.MaxUint64), 1e10}
	for _, u := range units {
		for _, n := range nums {
			o.add("time_duration", e, "time.Duration", u, n)
		}
	}
	for _, s := range []any{"1h12m10s", "300ms", "-1.5h", "2h45m", "1.5µs", "1us", "0", "", "1", "1d", "9223372036854775807ns", "9223372036854775808ns", "-9223372036854775808ns", ".5s", "1h1h", 5, nil, "+3m", "3m-2s"} {
		o.add("time_duration", e, "time.ParseDuration", s)
	}
	o.add("time_duration", e, "f:duration", "second", 3600)
	// time.Now under the --clock of main (checked for its range).
	o.add("time_now", e, "time.Now")
	o.add("time_now", e, "f:now")
}

// genTransform: the transform namespace.
func genTransform(o *oracle) {
	const e = "en/0"
	vals := scalars()
	for _, t := range texts() {
		vals = append(vals, t)
	}
	html := []any{
		"<p>Hello</p>", "&amp; &lt; &gt; &quot; &#39; &nbsp; &copy; &#x1F600; &unknown; &", "a < b > c & d \" e ' f",
		"<script>alert(1)</script>", "Tab\tNewline\nCR\rNUL\x00VT\x0bFF\x0c", "\ufffd \xff \uFFFE \U0010FFFF \uD7FF \uE000",
		template.HTML("<b>b</b>"), "<p>para <br/> break</p><p>two</p>", ":smile: :heart: :not_an_emoji: :+1:",
	}
	vals = append(vals, html...)
	for _, m := range []string{"HTMLEscape", "HTMLUnescape", "Plainify", "XMLEscape", "Emojify"} {
		for _, v := range vals {
			o.add("transform_text", e, "transform."+m, v)
		}
	}
	for _, f := range []string{"htmlEscape", "htmlUnescape", "plainify", "emojify"} {
		o.add("transform_text", e, "f:"+f, "<a> &amp;")
	}
	// Unmarshal of strings in every format, with options, and of resources.
	docs := []any{
		`{"a": 1, "b": [1, 2.5, "x", null, true], "c": {"d": "e"}}`, `[1, 2]`, `"str"`, `{"a": }`, `{"a": 1.0e2, "big": 12345678901234567890}`,
		"a = 1\nb = \"x\"\n[c]\nd = 2021-01-01T00:00:00Z\ne = 1979-05-27\n", "a = 1\na = 2\n", "title = 'T'\n[[arr]]\nx = 1\n[[arr]]\nx = 2\n",
		"a: 1\nb: [x, 2]\nc:\n  d: null\n", "- a\n- b\n", "a: [\n", "date: 2021-01-01\nx: yes\ny: 1.5\nz: ~\n",
		"a,b\n1,2\n", "a;b\n1;2\n", "<root><a>1</a></root>", "  ", "", "x", 42, nil, template.HTML(`{"h": 1}`), []byte(`{"b": 1}`),
		"# comment\na,b\n", "a\tb\n1\t2\n",
	}
	for _, d := range docs {
		o.add("transform_unmarshal", e, "transform.Unmarshal", d)
	}
	opts := []any{
		map[string]any{}, map[string]any{"delimiter": ";"}, map[string]any{"Delimiter": "\t"}, map[string]any{"delimiter": ";;"},
		map[string]any{"comment": "#"}, map[string]any{"targetType": "map"}, map[string]any{"targetType": "slice"}, map[string]any{"lazyQuotes": true},
		map[string]any{"lazyQuotes": "x"}, map[string]any{"delimiter": 1}, map[string]any{"unknown": 1}, maps.Params{"delimiter": ";"},
		map[string]any(nil), "notamap", nil,
	}
	for _, op := range opts {
		for _, d := range []any{"a;b\n1;2\n", `{"a": 1}`, "a: 1\n"} {
			o.add("transform_unmarshal", e, "transform.Unmarshal", op, d)
		}
	}
	for _, p := range []string{"data.json", "data.toml", "data.yaml", "data.csv", "data.xml", "data.txt", "img.png"} {
		o.add("transform_unmarshal", e, "transform.Unmarshal", callArg{"resources.Get", []any{p}})
		o.add("transform_unmarshal", e, "transform.Unmarshal", map[string]any{"delimiter": ";"}, callArg{"resources.Get", []any{p}})
	}
	o.add("transform_unmarshal", e, "transform.Unmarshal")
	o.add("transform_unmarshal", e, "transform.Unmarshal", 1, 2, 3)
	o.add("transform_unmarshal", e, "transform.Unmarshal", pageArg{"/a/p1"})
	o.add("transform_unmarshal", e, "f:unmarshal", `{"x": 1}`)
	// Remarshal.
	for _, f := range []any{"json", "JSON ", "yaml", "toml", "xml", "csv", "x", ""} {
		for _, d := range []any{`{"a": 1, "b": 2.5, "c": {"d": 3.0}, "e": [1.0]}`, "a = 1\nb = 1.5\n", "a: 1\nb: [1, 2]\n", map[string]any{"a": 1.0, "b": map[string]any{"c": 2.0}}, "", "  ", "x", 42, nil, `{"a": }`} {
			o.add("transform_remarshal", e, "transform.Remarshal", f, d)
		}
	}
	o.add("transform_remarshal", e, "transform.Remarshal", 1, "a: 1")
	// Markdownify through the home page of the site.
	md := []any{
		"", "Hello **bold**", "# Heading", "Line 1\n\nLine 2", "A [link](/a/p1/)", "`code` & <tag>", "ข้าว *ผัด*", "- a\n- b",
		"<b>raw html</b>", "Hello\nWorld", 42, nil, template.HTML("*html*"), "{{< sc >}}", "a  \nb",
	}
	for _, env := range []string{"en/0", "multi/1"} {
		for _, m := range md {
			o.add("transform_markdownify", env, "transform.Markdownify", m)
		}
		o.add("transform_markdownify", env, "f:markdownify", "*x*")
	}
}

// genLang: the lang namespace (en and th).
func genLang(o *oracle) {
	nums := []any{0, 1, -1, 1.5, -1.5, 512.5032, 1234567.891, -1234567.891, 0.005, 1e21, 1e-7, math.Inf(1), math.NaN(), "12.5", "x", nil, int64(math.MaxInt64), uint64(math.MaxUint64), 999.995, 1234.5}
	precs := []any{0, 1, 2, 3, 20, 21, -1, "2", "x", nil, 2.5}
	for _, env := range []string{"en/0", "th/0", "multi/1"} {
		for _, p := range precs {
			for _, n := range nums {
				o.add("lang_format", env, "lang.FormatNumber", p, n)
				o.add("lang_format", env, "lang.FormatPercent", p, n)
				for _, c := range []any{"USD", "eur", "THB", "JPY", "xyz", "", nil} {
					o.add("lang_format", env, "lang.FormatCurrency", p, c, n)
					o.add("lang_format", env, "lang.FormatAccounting", p, c, n)
				}
			}
		}
		for _, p := range []any{0, 1, 2, 3, -1, -2, "2", "x", 30} {
			for _, n := range nums {
				o.add("lang_custom", env, "lang.FormatNumberCustom", p, n)
				for _, opt := range [][]any{{"- , ."}, {"- , .", " "}, {"-|,|.", "|"}, {"-"}, {"- ."}, {""}, {"a b c d"}, {"- , .", ""}, {1}, {nil}} {
					o.add("lang_custom", env, "lang.FormatNumberCustom", append([]any{p, n}, opt...)...)
				}
			}
		}
		// Translate (i18n files of the site).
		for _, id := range []any{"hello", "apples", "html", "missing", "", 42, nil, template.HTML("hello")} {
			o.add("lang_translate", env, "lang.Translate", id)
			for _, data := range []any{nil, 1, 3, 1.5, "3", map[string]any{"Name": "Bob", "Count": 2}, map[string]any{"Count": 1}, maps.Params{"name": "x"}} {
				o.add("lang_translate", env, "lang.Translate", id, data)
			}
			o.add("lang_translate", env, "lang.Translate", id, 1, 2)
		}
		o.add("lang_translate", env, "f:i18n", "hello", map[string]any{"Name": "i18n"})
		o.add("lang_translate", env, "f:T", "apples", 5)
	}
	// Merge of the page lists of two languages.
	for _, env := range []string{"multi/0", "multi/1"} {
		o.add("lang_merge", env, "lang.Merge", pagesArg{"regular"}, pagesArg{"regular"})
		o.add("lang_merge", env, "lang.Merge", nil, pagesArg{"regular"})
		o.add("lang_merge", env, "lang.Merge", pagesArg{"regular"}, nil)
		o.add("lang_merge", env, "lang.Merge", []any{}, []any{1})
		o.add("lang_merge", env, "lang.Merge", "a", "b")
		o.add("lang_merge", env, "lang.Merge", pagesArg{"all"}, []string{"x"})
		o.add("lang_merge", env, "lang.Merge", callArg{"resources.Match", []any{"*.json"}}, callArg{"resources.Match", []any{"*.toml"}})
	}
}

// genInflect: the inflect namespace.
func genInflect(o *oracle) {
	const e = "en/0"
	words := []any{
		"MyCamel", "óbito", "", "103", "41", 103, int64(92), "5.5", "this is a TEST", "my-first-Post", "cat", "cats", "person", "people",
		"ox", "oxen", "sheep", "quiz", "matrix", "ข้าว", "hello_world", "HTTPServer", "1", "2", "3", "11", "12", "13", "21", "-1", "0",
		"bus", "buses", "child", "news", "octopus", "analysis", "  spaced  ", "ID", "user_id", 1.5, nil, true, template.HTML("tag"),
	}
	for _, m := range []string{"Humanize", "Pluralize", "Singularize"} {
		for _, w := range words {
			o.add("inflect", e, "inflect."+m, w)
		}
		for _, v := range scalars() {
			o.add("inflect", e, "inflect."+m, v)
		}
	}
	for _, f := range []string{"humanize", "pluralize", "singularize"} {
		o.add("inflect", e, "f:"+f, "item_count")
	}
}

// genOS: the os namespace against the site's working dir.
func genOS(o *oracle) {
	const e = "en/0"
	for _, k := range []any{"HUGO_T19_UNSET", "NEOHUGO_T19_UNSET", "PATH", "HOME", "", 42, nil, "HUGO"} {
		o.add("os", e, "os.Getenv", k)
	}
	files := []any{"files/hello.txt", "/files/hello.txt", "files/missing.txt", "files", "files/sub/x.md", "assets/data.json", "hugo.toml", "", ".", "/", "a/p1.md", "content/a/p1.md", "../x", "files/../files/hello.txt", 42, nil}
	for _, f := range files {
		o.add("os", e, "os.ReadFile", f)
		o.add("os", e, "os.FileExists", f)
		o.add("os", e, "os.Stat", f)
	}
	for _, d := range []any{"files", "files/sub", "assets/js", "content/a", "missing", "files/hello.txt", 42} {
		o.add("os", e, "os.ReadDir", d)
	}
	o.add("os", e, "f:readFile", "files/hello.txt")
	o.add("os", e, "f:fileExists", "files/hello.txt")
	o.add("os", e, "f:readDir", "files")
	o.add("os", e, "f:getenv", "HUGO_T19_UNSET")
}

// genDebug: the debug namespace.
func genDebug(o *oracle) {
	const e = "en/0"
	for _, v := range append(scalars(), map[string]any{"z": 1, "a": []any{1, "b", map[string]any{"c": nil}}}, []any{1.5, template.HTML("<b>")}) {
		o.add("debug", e, "debug.Dump", v)
		o.add("debug", e, "debug.VisualizeSpaces", v)
	}
	o.add("debug", e, "debug.VisualizeSpaces", "a b\tc\nd\re\vf\x00g\ufffdh")
	o.add("debug", e, "debug.Timer", "t1")
	o.add("debug", e, "debug.Timer", 1)
}

// genTemplates: the templates namespace (Exists, Defer, Current without a
// template).
func genTemplates(o *oracle) {
	const e = "en/0"
	for _, n := range []any{
		"_partials/hello.html", "partials/hello.html", "_partials/hello.txt", "_partials/sub/deep.html", "hello.html", "single.html",
		"_default/single.html", "shortcodes/sc.html", "_shortcodes/sc.html", "_markup/render-link.html", "_default/_markup/render-link.html",
		"missing.html", "", "/_partials/hello.html", "_PARTIALS/HELLO.HTML", 42, nil,
	} {
		o.add("templates", e, "templates.Exists", n)
	}
	o.add("templates", e, "templates.Defer")
	o.add("templates", e, "templates.Defer", 1)
	o.add("templates", e, "templates.Current")
}

// genPartials: partials, partialCached and return through the site's
// template store (and the page in the context).
func genPartials(o *oracle) {
	for _, env := range []string{"en/0", "multi/1"} {
		data := []any{nil, "x", 42, 1.5, map[string]any{"a": 1}, []any{1, "b"}, template.HTML("<b>h</b>"), pageArg{"/a/p1"}}
		for _, name := range []any{"hello.html", "hello", "hello.txt", "ret.html", "retnil.html", "retstr.html", "nested.html", "current.html", "outer.html", "dot.html", "exists.html", "sub/deep.html", "sub/deep", "partials/hello.html", "missing.html", "err.html", "loop.html", "", 42} {
			o.add("partials", env, "partials.Include", name)
			for _, d := range data {
				o.add("partials", env, "partials.Include", name, d)
			}
			o.add("partials", env, "partials.Include", name, "a", "extra")
		}
		o.add("partials", env, "partials.Include")
		// The context's page.
		o.addCtx("partials", env, ctxOpts{page: "/a/p1"}, "partials.Include", "page.html")
		o.addCtx("partials", env, ctxOpts{page: "/"}, "partials.Include", "page.html")
		o.add("partials", env, "partials.Include", "page.html")
		// partialCached: first execution wins per name and variants.
		for range 2 {
			o.add("partials_cached", env, "partials.IncludeCached", "cached.html", nil)
			o.add("partials_cached", env, "partials.IncludeCached", "cached.html", 1)
			o.add("partials_cached", env, "partials.IncludeCached", "cached.html", 1, "v1")
			o.add("partials_cached", env, "partials.IncludeCached", "cached.html", 1, "v1", 2)
			o.add("partials_cached", env, "partials.IncludeCached", "cached.html", 1, map[string]any{"a": 1})
			o.add("partials_cached", env, "partials.Include", "cached.html")
			o.add("partials_cached", env, "f:partialCached", "hello.html", "c")
			o.add("partials_cached", env, "partials.IncludeCached", "ret.html", "r")
		}
		o.add("partials_cached", env, "partials.IncludeCached", "loop.html", 1)
		o.add("partials_cached", env, "partials.IncludeCached", "missing.html", 1)
		o.add("partials_cached", env, "partials.IncludeCached", "err.html", 1)
		o.add("partials_cached", env, "partials.IncludeCached", "hello.html")
		o.add("partials_cached", env, "f:partial", "hello.html", "via func map")
		o.add("partials_cached", env, "f:return")
		o.add("partials_cached", env, "f:return", 1)
	}
}

// genHost: the context namespaces of the func map (hugo, site, page) and
// the try func.
func genHost(o *oracle) {
	for _, env := range []string{"en/0", "multi/0", "multi/1", "multihost/1"} {
		o.add("host", env, "f:hugo")
		o.add("host", env, "f:site")
		o.add("host", env, "f:page")
		o.addCtx("host", env, ctxOpts{page: "/a/p1"}, "f:page")
		o.addCtx("host", env, ctxOpts{page: "/"}, "f:page", 1)
		o.add("host", env, "f:site", 1, 2)
		o.add("host", env, "f:try", 1)
		o.add("host", env, "f:try", nil)
		o.add("host", env, "f:try")
		o.add("host", env, "f:try", 1, 2)
		o.add("host", env, "f:time")
		o.add("host", env, "f:css")
	}
}

// genImages: the images namespace (the filter constructors, Filter, Config).
func genImages(o *oracle) {
	const e = "en/0"
	args1 := []any{0, 1, -1, 10, 50.5, "20", "x", nil, float32(1.5), 100, 1e10}
	for _, m := range []string{"Brightness", "Contrast", "Gamma", "GaussianBlur", "Hue", "Pixelate", "Saturation", "Sepia", "Opacity", "Process"} {
		for _, a := range args1 {
			o.add("images_filters", e, "images."+m, a)
		}
		o.add("images_filters", e, "images."+m)
	}
	for _, s := range []any{"resize 10x", "fill 5x5 Center", "RESIZE 20x png", "", 42} {
		o.add("images_filters", e, "images.Process", s)
	}
	for _, m := range []string{"Grayscale", "Invert", "AutoOrient"} {
		o.add("images_filters", e, "images."+m)
		o.add("images_filters", e, "images."+m, 1)
	}
	for _, m := range []string{"ColorBalance", "Colorize", "UnsharpMask"} {
		o.add("images_filters", e, "images."+m, 1, 2, 3)
		o.add("images_filters", e, "images."+m, "1", 2.5, nil)
		o.add("images_filters", e, "images."+m, 1)
	}
	o.add("images_filters", e, "images.Sigmoid", 0.5, 7)
	o.add("images_filters", e, "images.Sigmoid", "x", nil)
	for _, p := range [][]any{{}, {1}, {1, 2}, {1, 2, 3}, {1, 2, 3, 4}, {1, 2, 3, 4, "#ff0000"}, {1, "#0f0"}, {1, "#12345"}, {1, 2, 3, 4, 5, 6}, {"x"}, {-2, "#ffffff80"}} {
		o.add("images_filters", e, "images.Padding", p...)
	}
	img := callArg{"resources.Get", []any{"img.png"}}
	wm := callArg{"resources.Get", []any{"wm.png"}}
	o.add("images_filters", e, "images.Overlay", wm, 1, 2)
	o.add("images_filters", e, "images.Overlay", wm, "3", nil)
	o.add("images_filters", e, "images.Overlay", "notanimage", 1, 2)
	o.add("images_filters", e, "images.Mask", wm)
	o.add("images_filters", e, "images.Mask", 1)
	o.add("images_filters", e, "images.Text", "hello")
	o.add("images_filters", e, "images.Dither")
	// Filter on an image resource.
	o.add("images_filter", e, "images.Filter", callArg{"images.Grayscale", nil}, img)
	o.add("images_filter", e, "images.Filter", callArg{"images.Brightness", []any{20}}, callArg{"images.Invert", nil}, img)
	o.add("images_filter", e, "images.Filter", callArg{"images.Overlay", []any{wm, 2, 2}}, img)
	o.add("images_filter", e, "images.Filter", callArg{"images.Process", []any{"resize 6x"}}, img)
	o.add("images_filter", e, "images.Filter", img)
	o.add("images_filter", e, "images.Filter", callArg{"images.Grayscale", nil}, "notanimage")
	o.add("images_filter", e, "images.Filter", callArg{"images.Grayscale", nil}, nil)
	o.add("images_filter", e, "images.Filter", callArg{"images.Grayscale", nil}, callArg{"resources.Get", []any{"data.json"}})
	o.add("images_filter", e, "images.Filter", callArg{"images.Grayscale", nil}, pageArg{"/a/p1"})
	o.add("images_filter", e, "images.Filter", "notafilter", img)
	// Config of files relative to the working directory.
	for _, p := range []any{"assets/img.png", "assets/img.jpg", "assets/img.gif", "files/img.png", "content/b/q1/cat.png", "b/q1/cat.png", "assets/data.json", "missing.png", "", nil, 42} {
		o.add("images_config", e, "images.Config", p)
		o.add("images_config", e, "f:imageConfig", p)
	}
	o.add("images_config", e, "images.QR", "hello")
	o.add("images_config", e, "images.QR")
	o.add("images_config", e, "images.QR", "")
}

// genCSS: the css namespace (LibSass through the resource transformers;
// PostCSS, TailwindCSS and Dart Sass only up to their argument checks, which
// need no external tool). LibSass is cgo: see main.
func genCSS(o *oracle) {
	const e = "en/0"
	for _, v := range []any{"a", 1, nil, "ข้าว", template.HTML("<b>"), []any{1}} {
		o.add("css", e, "css.Quoted", v)
		o.add("css", e, "css.Unquoted", v)
	}
	scss := callArg{"resources.Get", []any{"style.scss"}}
	o.add("css", e, "css.Sass", scss)
	o.add("css", e, "css.Sass", map[string]any{"outputStyle": "compressed"}, scss)
	o.add("css", e, "css.Sass", map[string]any{"targetPath": "out/x.css", "outputStyle": "expanded"}, scss)
	o.add("css", e, "css.Sass", "css/t.css", scss)
	o.add("css", e, "css.Sass", map[string]any{"transpiler": "libsass"}, scss)
	o.add("css", e, "css.Sass", map[string]any{"Transpiler": "nope"}, scss)
	o.add("css", e, "css.Sass", map[string]any{"transpiler": 1}, scss)
	o.add("css", e, "css.Sass", map[string]any{"vars": map[string]any{"c": "blue"}}, scss)
	o.add("css", e, "css.Sass")
	o.add("css", e, "css.Sass", "a", "b")
	o.add("css", e, "css.Sass", 1, 2, 3)
	o.add("css", e, "css.Sass", map[string]any{}, "notaresource")
	o.add("css", e, "f:toCSS", scss)
	for _, m := range []string{"PostCSS", "TailwindCSS"} {
		o.add("css", e, "css."+m)
		o.add("css", e, "css."+m, 1, 2, 3)
		o.add("css", e, "css."+m, "notaresource")
		o.add("css", e, "css."+m, map[string]any{}, "notaresource")
		o.add("css", e, "css."+m, 1, scss)
	}
	o.add("css", e, "f:postCSS")
}

// genJS: the js namespace (esbuild is linked into Go; the Rust test needs the
// pinned esbuild binary).
func genJS(o *oracle) {
	const e = "en/0"
	jsr := callArg{"resources.Get", []any{"js/main.js"}}
	o.add("js", e, "js.Build", jsr)
	o.add("js", e, "js.Build", "out/app.js", jsr)
	o.add("js", e, "js.Build", map[string]any{"minify": true, "targetPath": "out/min.js"}, jsr)
	o.add("js", e, "js.Build", map[string]any{"format": "esm", "target": "es2017"}, jsr)
	o.add("js", e, "js.Build")
	o.add("js", e, "js.Build", "notaresource")
	o.add("js", e, "js.Build", map[string]any{"minify": "nope"}, jsr)
	o.add("js", e, "js.Babel")
	o.add("js", e, "js.Babel", 1, 2, 3)
	o.add("js", e, "js.Babel", "notaresource")
	o.add("js", e, "js.Batch", "")
	o.add("js", e, "f:babel")
}

// genStubs: the functions the port stubs (the Rust test expects its explicit
// unsupported error): data, diagrams, openapi3, transform highlighting and
// math, templates.DoDefer.
func genStubs(o *oracle) {
	const e = "en/0"
	o.add("stubs", e, "transform.Highlight", "x := 1", "go")
	o.add("stubs", e, "transform.CanHighlight", "go")
	o.add("stubs", e, "transform.CanHighlight", "nosuchlang")
	o.add("stubs", e, "transform.ToMath", "x^2")
	o.add("stubs", e, "transform.ToMath")
	o.add("stubs", e, "transform.PortableText", []any{})
	o.add("stubs", e, "diagrams.Goat", "+--+\n|  |\n+--+")
	o.add("stubs", e, "openapi3.Unmarshal", callArg{"resources.Get", []any{"data.yaml"}})
	o.add("stubs", e, "f:getJSON", "assets/data.json")
	o.add("stubs", e, "f:getCSV", ";", "assets/data.csv")
}
