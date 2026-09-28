package main

import (
	"html/template"
	"strings"
	"time"
)

// genTables: the inputs of the Go test tables of the host namespaces
// (tpl/{strings,path,urls,time,transform,lang,inflect,os}/*_test.go), without
// the rows that need a test-only type (tstNoStringer, testing.T, test
// resources). Go computes the expected values.
func genTables(o *oracle) {
	const e = "en/0"
	add := func(m string, args ...any) { o.add("go_tables", e, m, args...) }

	// strings_test.go
	for _, s := range []string{"\n a\n", "\n a\n\n", "\n a\r\n", "\n a\n\r\n", "\n a\r\r", "\n a\r"} {
		add("strings.Chomp", s)
		add("strings.Chomp", template.HTML(s))
	}
	for _, r := range [][2]any{{"", ""}, {"123", "23"}, {"123", "234"}, {"123", ""}, {"", "a"}, {123, "23"}, {123, "234"}, {123, ""}, {template.HTML("123"), []byte("23")}, {template.HTML("123"), []byte("234")}, {template.HTML("123"), []byte("")}} {
		add("strings.Contains", r[0], r[1])
	}
	for _, r := range [][2]any{{"", ""}, {"", "1"}, {"", "123"}, {"1", ""}, {"1", "1"}, {"111", "1"}, {"123", "789"}, {"123", "729"}, {"a☺b☻c☹d", "uvw☻xyz"}, {1, ""}, {1, "1"}, {111, "1"}, {123, "789"}, {123, "729"}, {[]byte("123"), template.HTML("789")}, {[]byte("123"), template.HTML("729")}, {[]byte("a☺b☻c☹d"), template.HTML("uvw☻xyz")}} {
		add("strings.ContainsAny", r[0], r[1])
	}
	for _, s := range []string{"", " ", "        ", "\t", "\r", "a", "    a", "a\n"} {
		add("strings.ContainsNonSpace", s)
	}
	for _, s := range []string{"foo bar", "旁边", `<div class="test">旁边</div>`} {
		add("strings.CountRunes", s)
		add("strings.RuneCount", s)
	}
	for _, s := range []string{"Do Be Do Be Do", "旁边", `<div class="test">旁边</div>`, "Here's to you...", "Here’s to you...", "Here’s to you…"} {
		add("strings.CountWords", s)
	}
	for _, r := range [][2]any{{"abcd", "ab"}, {"abcd", "cd"}, {template.HTML("abcd"), "ab"}, {template.HTML("abcd"), "cd"}, {template.HTML("1234"), 12}, {template.HTML("1234"), 34}, {[]byte("abcd"), "ab"}} {
		add("strings.HasPrefix", r[0], r[1])
		add("strings.HasSuffix", r[0], r[1])
	}
	for _, r := range [][4]any{{"aab", "a", "b", nil}, {"11a11", 1, 2, nil}, {12345, 1, 2, nil}, {"aab", "a", "b", 1}, {"11a11", 1, 2, 2}} {
		if r[3] == nil {
			add("strings.Replace", r[0], r[1], r[2])
		} else {
			add("strings.Replace", r[0], r[1], r[2], r[3])
		}
	}
	for _, r := range [][3]any{
		{"abc", 1, 2}, {"abc", 1, 3}, {"abcdef", 1, int8(3)}, {"abcdef", 1, int16(3)}, {"abcdef", 1, int32(3)}, {"abcdef", 1, int64(3)},
		{"abc", 0, 1}, {"abcdef", nil, nil}, {"abcdef", 0, 6}, {"abcdef", 0, 2}, {"abcdef", 2, nil}, {"abcdef", int8(2), nil},
		{"abcdef", int16(2), nil}, {"abcdef", int32(2), nil}, {"abcdef", int64(2), nil}, {123, 1, 3}, {"abcdef", 6, nil},
		{"abcdef", 4, 7}, {"abcdef", -1, nil}, {"abcdef", -1, 7}, {"abcdef", 1, -1}, {"ĀĀĀ", 0, 1},
	} {
		switch {
		case r[1] == nil && r[2] == nil:
			add("strings.SliceString", r[0])
		case r[2] == nil:
			add("strings.SliceString", r[0], r[1])
		default:
			add("strings.SliceString", r[0], r[1], r[2])
		}
	}
	for _, r := range [][2]any{{"a, b", ", "}, {"a & b & c", " & "}, {"http://example.com", "http://"}, {123, "2"}} {
		add("strings.Split", r[0], r[1])
	}
	for _, r := range [][3]any{
		{"abc", 1, 2}, {"abc", 0, 1}, {"abcdef", 0, 0}, {"abcdef", 1, 0}, {"abcdef", -1, 0}, {"abcdef", -1, 2}, {"abcdef", -3, 3},
		{"abcdef", -1, nil}, {"abcdef", -2, nil}, {"abcdef", -3, 1}, {"abcdef", 0, -1}, {"abcdef", 2, -1}, {"abcdef", 4, -4},
		{"abcdef", 7, 1}, {"abcdef", 6, nil}, {"abcdef", 1, 100}, {"abcdef", -100, 3}, {"abcdef", -3, -1}, {"abcdef", 2, nil},
		{"abcdef", int8(2), nil}, {"abcdef", int16(2), nil}, {"abcdef", int32(2), nil}, {"abcdef", int64(2), nil}, {"abcdef", 2, int8(3)},
		{"abcdef", 2, int16(3)}, {"abcdef", 2, int32(3)}, {"abcdef", 2, int64(3)}, {123, 1, 3}, {1.2e3, 0, 4}, {"abcdef", 2.0, nil},
		{"abcdef", 2.0, 2}, {"abcdef", 2, 2.0}, {"ĀĀĀ", 1, 2}, {"abcdef", "doo", nil}, {"abcdef", "doo", "doo"}, {"abcdef", 1, "doo"}, {"", 0, nil},
	} {
		if r[2] == nil {
			add("strings.Substr", r[0], r[1])
		} else {
			add("strings.Substr", r[0], r[1], r[2])
		}
	}
	for _, v := range []any{"test", template.HTML("hypertext"), []byte("bytes")} {
		add("strings.Title", v)
	}
	for _, v := range []any{"TEST", template.HTML("LoWeR"), []byte("BYTES"), "test", template.HTML("UpPeR"), []byte("bytes")} {
		add("strings.ToLower", v)
		add("strings.ToUpper", v)
	}
	for _, r := range [][2]any{{"abba", "a"}, {"abba", "ab"}, {"<tag>", "<>"}, {`"quote"`, `"`}, {1221, "1"}, {1221, "12"}, {"007", "0"}, {template.HTML("<tag>"), "<>"}, {[]byte("<tag>"), "<>"}} {
		add("strings.Trim", r[0], r[1])
		add("strings.TrimLeft", r[1], r[0])
		add("strings.TrimRight", r[1], r[0])
	}
	for _, r := range [][2]any{{"aabbaa", "a"}, {"aabb", "b"}, {1234, "12"}, {1234, "34"}} {
		add("strings.TrimPrefix", r[1], r[0])
		add("strings.TrimSuffix", r[1], r[0])
	}
	for _, r := range [][2]any{{"yo", "2"}, {"~", "16"}, {"<tag>", "0"}, {"yay", "1"}, {1221, "1"}, {1221, 2}, {template.HTML("<tag>"), "2"}, {[]byte("<tag>"), 2}, {"ab", -1}} {
		add("strings.Repeat", r[1], r[0])
	}
	for _, r := range [][2]any{{"foo\n", "bar\n"}, {"foo\n", "foo\n"}, {"foo\n", ""}, {"foo\n", nil}, {"", ""}} {
		add("strings.Diff", "old", r[0], "new", r[1])
	}
	for _, v := range []any{"\n\r test \n\r", template.HTML("\n\r test \n\r"), []byte("\n\r test \n\r")} {
		add("strings.TrimSpace", v)
	}

	// regexp_test.go
	const hugo = "Hugo is a static site generator written in Go."
	for _, lim := range []any{2, -1, 1, "1", nil} {
		if lim == nil {
			add("strings.FindRE", "[G|g]o", hugo)
		} else {
			add("strings.FindRE", "[G|g]o", hugo, lim)
		}
	}
	add("strings.FindRE", "[G|go", hugo)
	add("strings.FindRESubmatch", `<a\s*href="(.+?)">(.+?)</a>`, `<li><a href="#foo">Foo</a></li><li><a href="#bar">Bar</a></li>`, -1)
	add("strings.FindRESubmatch", "([G|g]o)", hugo, -1)
	add("strings.FindRESubmatch", "([G|g]o)", hugo, 1)
	add("strings.FindRESubmatch", "([G|go", hugo)
	add("strings.ReplaceRE", "^https?://([^/]+).*", "$1", "http://gohugo.io/docs")
	add("strings.ReplaceRE", "^https?://([^/]+).*", "$2", "http://gohugo.io/docs")
	add("strings.ReplaceRE", "(ab)", "AB", "aabbaab")
	add("strings.ReplaceRE", "(ab)", "AB", "aabbaab", 1)
	add("strings.ReplaceRE", "(ab", "AB", "aabb")

	// truncate_test.go
	for _, r := range [][3]any{
		{10, "I am a test sentence", nil},
		{10, "", "I am a test sentence"},
		{10, "", "a b c d e f g h i j k"},
		{12, "", "<b>Should be escaped</b>"},
		{10, template.HTML(" <a href='#'>Read more</a>"), "I am a test sentence"},
		{20, template.HTML("I have a <a href='/markdown'>Markdown link</a> inside."), nil},
		{10, "IamanextremelylongwordthatjustgoesonandonandonjusttoannoyyoualmostasifIwaswritteninGermanActuallyIbettheresagermanwordforthis", nil},
		{10, template.HTML("<p>IamanextremelylongwordthatjustgoesonandonandonjusttoannoyyoualmostasifIwaswritteninGermanActuallyIbettheresagermanwordforthis</p>"), nil},
		{13, template.HTML("With <a href=\"/markdown\">Markdown</a> inside."), nil},
		{14, "Hello中国 Good 好的", nil},
		{15, "", template.HTML("A <br> tag that's not closed")},
		{14, template.HTML("<p>Hello中国 Good 好的</p>"), nil},
		{2, template.HTML("<p>P1</p><p>P2</p>"), nil},
		{3, template.HTML(strings.Repeat("<p>P</p>", 20)), nil},
		{18, template.HTML("<p>test <b>hello</b> test something</p>"), nil},
		{4, template.HTML("<p>a<b><i>b</b>c d e</p>"), nil},
		{10, nil, nil},
		{nil, nil, nil},
	} {
		if r[2] == nil {
			add("strings.Truncate", r[0], r[1])
		} else {
			add("strings.Truncate", r[0], r[1], r[2])
		}
	}

	// path_test.go
	for _, p := range []string{`foo/bar.txt`, `foo/bar/txt `, `foo/bar.t`, `foo.bar.txt`, `.x`, ``, `foo/bar.json`, `foo.bar.txt `, `foo/bar/txt`, `foo/bar`} {
		for _, m := range []string{"Base", "BaseName", "Dir", "Ext", "Split", "Clean"} {
			add("path."+m, p)
		}
	}
	add("path.Join", []string{"my", "path", "filename.txt"})
	add("path.Join", []any{"my", "path", "filename.txt"})
	add("path.Join", "my", "path", "filename.txt")
	add("path.Join", nil)

	// urls_test.go
	add("urls.Parse", "http://www.google.com")
	add("urls.Parse", "http://j@ne:password@google.com")
	for _, v := range []any{"", "a", "/a/b", "./../a/b", []any{""}, []any{"a"}, []any{"/a", "b"}, []any{".", "..", "/a", "b"}, []any{"https://example.org", "a"}, []any{nil}} {
		add("urls.JoinPath", v)
	}
	add("urls.URLEncode", "https://example.org/?a=1&b=2 é")
	add("urls.URLDecode", "https%3A%2F%2Fexample.org%2F%3Fa%3D1%26b%3D2+%C3%A9")

	// time_test.go
	for _, r := range [][2]any{{"2020-10-20", ""}, {"2020-10-20", "America/New_York"}, {"2020-01-20", "America/New_York"}, {"2020-10-20 20:33:59", ""}, {"2020-10-20 20:33:59", "America/New_York"}, {"2020-09-23T20:33:44-0700", ""}, {"2020-09-23T20:33:44+0200", ""}, {"2020-09-23T20:33:44-0700", "America/New_York"}, {"2020-09-23T20:33:44+0200", "Europe/Oslo"}, {"2020-01-20", "invalid-timezone"}, {"invalid-value", ""}} {
		add("time.AsTime", r[0], r[1])
	}
	for _, r := range [][2]any{{"nanosecond", 10}, {"ns", 10}, {"microsecond", 20}, {"us", 20}, {"µs", 20}, {"millisecond", 20}, {"ms", 20}, {"second", 30}, {"s", 30}, {"minute", 20}, {"m", 20}, {"hour", 20}, {"h", 20}, {"hours", 20}, {"hour", "30"}} {
		add("time.Duration", r[0], r[1])
	}
	for _, z := range []string{"America/Denver", "Australia/Adelaide", "Europe/Oslo", "UTC", "", "InvalidTimeZoneName"} {
		add("time.In", z, t1)
	}
	for _, r := range [][2]any{
		{"Monday, Jan 2, 2006", "2015-01-21"}, {"Monday, Jan 2, 2006", t1}, {"This isn't a date layout string", "2015-01-21"},
		{"Monday, Jan 2, 2006", 1421733600}, {"Monday, Jan 2, 2006", 1421733600.123}, {time.RFC3339, "2016-03-29T16:35:17+02:00"},
		{":date_long", t1}, {":time_short", t1}, {":date_full", "2020-01-02T15:04:05+07:00"},
	} {
		add("time.Format", r[0], r[1])
	}

	// transform_test.go
	for _, v := range []any{`"Foo & Bar's Diner" <y@z>`, "Hugo & Caddy > Wordpress & Apache"} {
		add("transform.HTMLEscape", v)
	}
	for _, v := range []any{`&quot;Foo &amp; Bar&#39;s Diner&quot; &lt;y@z&gt;`, "Hugo &amp; Caddy &gt; Wordpress &amp; Apache"} {
		add("transform.HTMLUnescape", v)
	}
	for _, v := range []any{"<em>Note:</em> blah <b>blah</b>", "<div data-action='click->my-controller#doThing'>qwe</div>"} {
		add("transform.Plainify", v)
	}
	add("transform.Markdownify", "Hello **World!**")
	add("transform.Markdownify", []byte("Hello Bytes **World!**"))
	add("transform.Markdownify", "#First\n\nThis is some *bold* text.\n\n## Second\n\nThis is some more text.")
	for _, v := range []any{`{ "slogan": "Hugo Rocks!" }`, `slogan: "Hugo Rocks!"`, `slogan = "Hugo Rocks!"`, "", "   ", "thisisnotavaliddataformat", `{ notjson }`} {
		add("transform.Unmarshal", v)
	}
	add("transform.Unmarshal", "a,b,c")
	add("transform.Unmarshal", map[string]any{"delimiter": ";"}, "a;b;c")
	for _, f := range []string{"json", "yaml", "toml"} {
		add("transform.Remarshal", f, "title = \"Test Metadata\"\n\n[[resources]]\n  src = \"**image-4.png\"\n  title = \"The Fourth Image!\"\n  [resources.params]\n    byline = \"picasso\"\n")
		add("transform.Remarshal", f, `{"a": 1.0, "b": [1, 2.0]}`)
	}
	add("transform.Remarshal", "json", "")
	add("transform.Remarshal", "blah", "a: 1")
	add("transform.Remarshal", "json", "  bad: data\nfoo")

	// lang_test.go
	for _, r := range [][4]any{
		{2, -12345.6789, "", ""}, {2, -12345.6789, "- . ,", ""}, {2, -12345.1234, "- . ,", ""}, {2, 12345.6789, "- . ,", ""},
		{0, 12345.6789, "- . ,", ""}, {11, -12345.6789, "- . ,", ""}, {2, 927.675, "- .", ""}, {2, 1927.675, "- .", ""},
		{2, 2927.675, "- .", ""}, {3, -12345.6789, "- ,", ""}, {6, -12345.6789, "- , .", ""}, {3, -12345.6789, "-|,| ", "|"},
		{6, -12345.6789, "-|,| ", "|"}, {6, -12345.6789, "\u200f- ٫ ٬", ""}, {6, -12345.6789, "\u200f-|٫| ", "|"},
	} {
		switch {
		case r[2] == "":
			add("lang.FormatNumberCustom", r[0], r[1])
		case r[3] == "":
			add("lang.FormatNumberCustom", r[0], r[1], r[2])
		default:
			add("lang.FormatNumberCustom", r[0], r[1], r[2], r[3])
		}
	}
	for _, env := range []string{"en/0", "th/0"} {
		for _, r := range [][2]any{{2, 512.5032}, {0, 12345.6789}, {3, -1234.5}, {1, 0.25}} {
			o.add("go_tables", env, "lang.FormatNumber", r[0], r[1])
			o.add("go_tables", env, "lang.FormatPercent", r[0], r[1])
			o.add("go_tables", env, "lang.FormatCurrency", r[0], "USD", r[1])
			o.add("go_tables", env, "lang.FormatAccounting", r[0], "NOK", r[1])
		}
	}

	// inflect_test.go
	for _, v := range []any{"MyCamel", "óbito", "", "103", "41", 103, int64(92), "5.5", "this is a TEST", "my-first-Post"} {
		add("inflect.Humanize", v)
	}
	add("inflect.Pluralize", "cat")
	add("inflect.Pluralize", "")
	add("inflect.Singularize", "cats")
	add("inflect.Singularize", "")

	// os_test.go (the site's files)
	for _, f := range []string{"/files/hello.txt", "files/hello.txt", "../f2.txt", "", "b"} {
		add("os.ReadFile", f)
		add("os.FileExists", f)
		add("os.Stat", f)
	}
}
