package main

import (
	"log"
	"sort"

	"github.com/neohugo/neohugo/common/paths"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/corpus"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
)

// urlStrings are adversarial URL-ish inputs for the escaping helpers.
var urlStrings = []string{
	"http://example.com/", "https://seeksnack.com/", "https://seeksnack.com/th/", "mailto:hugo@rules.com",
	"webcal://example.com/cal.ics", "//cdn.example.com/x.js", "http://a b.com/", "http://[::1]:80/x",
	"http://user:pass@host:8080/p/a/t/h?query=1&b=%20#frag ment", "/tags/คริสปี้พาย-/", "/tags/คริสปี้พาย /",
	"/potato-chips/wise-chili-olé-chili--spice-flavor-potato-chips/", "/a%2Fb/", "/a%zzb/", "/%", "/%f",
	"/%ff/", "a:b", ":a", "1a:b", "/a?b?c", "/a#b#c", "/a;b", "/a,b", "/!$&'()*+,;=:@", "/ /",
	"/\x7f/", "/a\tb", "/a\nb", "http://%41.com/", "http://h/%e0%b8%81/", "http:///x", "http:",
	"file:///path/to/file", "file://c/Users/x", "/images/watermark_hu_3bf49ff914f6e68c.png",
	"/scss/website.min.186be38d09dc13506b8cedff2d4a7af52dc9c20dce9752e3fc6cd86a865b81e4.css",
	"/biscuit/page/2/", "/th/tags/นม/", "/section/name.html", "/section/name/", "/section/name/index.html",
	"/index.html", "/name.xml", "/.xml", "/", "", "index.html", "/a/b.c/", "../a", "./a/./b",
}

var contextRoots = []string{
	"https://seeksnack.com/", "http://example.com/тря", "", "http://h/%ff/",
}

var permalinkHosts = []string{
	"https://seeksnack.com/", "http://abc.com/foo", "mailto:x@y.z",
}

func stringCases(root string, pathIn []input) []map[string]any {
	// set[s] is true for the path and URL inputs (every helper) and false for
	// the text corpus (the text helpers only: Sanitize, MakeTitle, the escapes).
	set := map[string]bool{}
	strs, err := corpus.Strings(root)
	if err != nil {
		log.Fatal(err)
	}
	for _, s := range strs {
		set[s] = false
	}
	for _, in := range pathIn {
		switch in.set {
		case "repo", "spec", "spec-layout", "slashes", "sweep-other":
			set[in.s] = true
		}
	}
	for _, s := range urlStrings {
		set[s] = true
	}
	var inputs []string
	for s := range set {
		inputs = append(inputs, s)
	}
	sort.Strings(inputs)

	var cases []map[string]any
	for _, s := range inputs {
		c := map[string]any{"in": goval.Str(s)}
		str := func(k string, f func() string) {
			c[k] = try(func() any { return goval.Str(f()) })
		}
		pair := func(k string, f func() (string, string)) {
			c[k] = try(func() any {
				a, b := f()
				return []any{goval.Str(a), goval.Str(b)}
			})
		}
		str("Sanitize", func() string { return paths.Sanitize(s) })
		str("MakeTitle", func() string { return paths.MakeTitle(s) })
		str("PathEscape", func() string { return paths.PathEscape(s) })
		str("URLEscape", func() string { return paths.URLEscape(s) })
		str("NormalizePathStringBasic", func() string { return paths.NormalizePathStringBasic(s) })
		c["HasExt"] = paths.HasExt(s)
		if !set[s] {
			c["textOnly"] = true
			cases = append(cases, c)
			continue
		}
		str("AddLeadingSlash", func() string { return paths.AddLeadingSlash(s) })
		str("AddTrailingSlash", func() string { return paths.AddTrailingSlash(s) })
		str("AddLeadingAndTrailingSlash", func() string { return paths.AddLeadingAndTrailingSlash(s) })
		str("Ext", func() string { return paths.Ext(s) })
		str("ExtNoDelimiter", func() string { return paths.ExtNoDelimiter(s) })
		pair("PathAndExt", func() (string, string) { return paths.PathAndExt(s) })
		pair("FileAndExt", func() (string, string) { return paths.FileAndExt(s) })
		pair("FileAndExtNoDelimiter", func() (string, string) { return paths.FileAndExtNoDelimiter(s) })
		str("Filename", func() string { return paths.Filename(s) })
		str("ReplaceExtension", func() string { return paths.ReplaceExtension(s, "xml") })
		str("Dir", func() string { return paths.Dir(s) })
		c["FieldsSlash"] = try(func() any { return strsOf(paths.FieldsSlash(s)) })
		str("ToSlashTrimLeading", func() string { return paths.ToSlashTrimLeading(s) })
		str("TrimLeading", func() string { return paths.TrimLeading(s) })
		str("ToSlashTrimTrailing", func() string { return paths.ToSlashTrimTrailing(s) })
		str("TrimTrailing", func() string { return paths.TrimTrailing(s) })
		str("ToSlashTrim", func() string { return paths.ToSlashTrim(s) })
		str("ToSlashPreserveLeading", func() string { return paths.ToSlashPreserveLeading(s) })
		c["IsSameFilePath"] = paths.IsSameFilePath(s, "/a/b/c")
		str("CommonDirPath", func() string { return paths.CommonDirPath(s, "/a/b/c") })
		str("AbsPathify", func() string { return paths.AbsPathify("/work/dir", s) })
		c["GetRelativePath"] = try(func() any {
			r, err := paths.GetRelativePath(s, "/a")
			return res(r, err)
		})
		c["GetRelativePathNoBase"] = try(func() any {
			r, err := paths.GetRelativePath(s, "")
			return res(r, err)
		})
		str("TrimExt", func() string { return paths.TrimExt(s) })
		str("PrettifyURLPath", func() string { return paths.PrettifyURLPath(s) })
		str("PrettifyURL", func() string { return paths.PrettifyURL(s) })
		str("Uglify", func() string { return paths.Uglify(s) })
		c["UrlStringToFilename"] = try(func() any {
			f, ok := paths.UrlStringToFilename(s)
			return []any{goval.Str(f), ok}
		})
		c["UrlFromFilename"] = try(func() any {
			u, err := paths.UrlFromFilename(s)
			if err != nil {
				return map[string]any{"err": err.Error()}
			}
			return map[string]any{"ok": goval.Str(u.String())}
		})
		var acr []any
		for _, base := range contextRoots {
			acr = append(acr, try(func() any { return goval.Str(paths.AddContextRoot(base, s)) }))
		}
		c["AddContextRoot"] = acr
		var mp []any
		for _, host := range permalinkHosts {
			mp = append(mp, try(func() any { return goval.Str(paths.MakePermalink(host, s).String()) }))
		}
		c["MakePermalink"] = mp
		cases = append(cases, c)
	}
	return cases
}

// res encodes (s, err) as {"ok": s} or {"err": message}.
func res(s string, err error) map[string]any {
	if err != nil {
		return map[string]any{"err": err.Error()}
	}
	return map[string]any{"ok": goval.Str(s)}
}

func strsOf(ss []string) []any {
	out := make([]any, len(ss))
	for i, s := range ss {
		out[i] = goval.Str(s)
	}
	return out
}
