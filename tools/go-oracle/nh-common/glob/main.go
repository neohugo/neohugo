// Command glob is the Go oracle for hugofs/glob and its github.com/gobwas/glob
// (v0.2.3) dependency in crates/nh-common (Wave B task T02).
//
//	go run ./tools/go-oracle/nh-common/glob [-root .] [-out crates/nh-common/tests/fixtures/glob]
//
// Output (gzip, best compression, no name or time):
//   - compile.json.gz: glob.Compile(pattern, separators...) for every pattern
//     and separator set: the compiled matcher's String() (its whole tree) or
//     the error text;
//   - match.json.gz: Match over a pattern × input matrix, raw gobwas (with
//     separators) and through hugofs/glob.GetGlob (lower-casing), one string
//     of '0'/'1'/'p' (panic) per pattern;
//   - filter.json.gz: FilenameFilter decisions for include/exclude glob
//     configurations (mount includeFiles/excludeFiles, the single-file mount
//     inclusion func, Append chains) over file and directory names, and the
//     hugofs/glob path helpers.
//
// Patterns: the gobwas and Hugo test patterns, the globs Hugo builds
// (mounts, page matchers, resources.Match, KeyRenamer) and random token
// soups from a fixed seed (math/rand with a fixed source is deterministic).
// Nothing here depends on the platform: linux/amd64 and linux/arm64 (qemu)
// produce the same bytes.
package main

import (
	"flag"
	"fmt"
	"io/fs"
	"log"
	"math/rand"

	"path/filepath"
	"sort"
	"strings"

	"github.com/gobwas/glob"
	hglob "github.com/neohugo/neohugo/hugofs/glob"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
)

// fixedPatterns are hand-picked patterns (upstream tests, Hugo usage, edge cases).
var fixedPatterns = []string{
	// gobwas glob_test.go and compiler_test.go
	"* ?at * eyes", "", "*ä", "abc", "a*c", "a?c", "a.b", "a.*", "a.**", "a.?.c", "a.?.?", "?at",
	"*", `\*`, "**", "*test", "this*", "*is *", "*is*a*", "**test**", "**is**a***test*", "*is",
	"*no*", "[!a]*", "*abc", "**abc", "???", "?*?", "sta", "sta*", "sta?", "sta?n",
	"{abc,def}ghi", "{abc,abcd}a", "{a,ab}{bc,f}", "{*,**}{a,b}", "/{rate,[a-z][a-z][a-z]}*",
	"/{rate,[0-9][0-9][0-9]}*", "{*.google.*,*.yandex.*}", "{*.google.*,yandex.*}",
	"[a-z][!a-x]*cat*[h][!b]*eyes*", "google.com", "https://*.google.*",
	"{https://*.google.*,*yandex.*,*yahoo.*,*mail.ru}",
	"{https://*gobwas.com,http://exclude.gobwas.com}", "abc*", "*def", "ab*ef",
	"{abc*def,abc?def,abc[zte]def}", "{abc*[a-c]def,abc?[d-g]def,abc[zte]?def}",
	"[a-c]", "[!a-c]", "[abc]", "[!abc]", "{a,b}", "{a,b,c}{d,e}", "a{b,c}d", "*{a,b}*",
	"a*b*c", "a**b", "*a*", "**a**", "?a?", "a???", "[a]", "[!a]", "{a}", "{}", "{,}", "{,a}",
	"{a,}", "a{,}", "{,}a", "a{b,}c", "{a,b}{}", "[]", "[!]", "[a-]", "[-a]", "[a-a]", "[b-a]",
	"[", "]", "{", "}", "[a", "{a", "a}", "a]", ",", "a,b", `\`, `a\`, `\\`, `\[a\]`,
	`\{a,b\}`, "[\\]]", "[\\-]", "[a-\\]", "*[!/]", "*/", "/*", "/**", "**/", "*/*", "**/*",
	"a\x00b", "\x00", "a\xffb", "\xef\xbf\xbd", "é", "*é", "é*", "[é]", "[à-ü]", "{é,è}",
	"ก*", "*ไทย*", "[ก-ฮ]*", "ß*", "*ǅ*",
	// Hugo: mounts, page matchers, resources.Match, KeyRenamer, tests
	"**.json", "/a/b/c/foo.json", "/a/**/foo.json", "/**/Foo.json", "**.jpg", "**.JSON",
	"**/foo", "/**/foo.json", "*.{png,jpg}", "images/*", "images/**", "/images/**",
	"**.{png,jpg,jpeg}", "**/*.jpg", "*.md", "/blog/**", "/posts/*", "/posts/**/*.md",
	"{home,section}", "page", "{taxonomy,term}", "home", "*", "{en,th}", "!en",
	"production", "{development,production}", "/biscuit/**", "/**", "**/index.*",
	"/{menu,menus}", "/menus/**", "menu", "/params/**", "/languages/*/menu*",
	"/_default/**", "_partials/*", "/_partials/**", "gallery/*/*", "*/*.jpg",
	"assets/**.scss", "data/**.{json,yaml,toml}", "node_modules/**", "*.min.*",
	"/a", "/a/b", "/", "/a/*", "a/b/c",
}

// tokens build the random patterns.
var tokens = []string{
	"a", "b", "ab", "c", "/", ".", "*", "**", "?", "[a-c]", "[!a-c]", "[ab]", "[!ab]",
	"[/]", "[!/]", "{a,b}", "{,a}", "{a,}", "{*,**}", "{a*,b?}", "{a,ab}", "{a/b,c}",
	`\*`, `\?`, "é", "ก", "{a{b,c},d}", "[", "]", "{", "}", ",", "-", "!", `\`, "x",
}

// inputAlphabet builds the random match inputs.
var inputAlphabet = []string{"a", "b", "c", "ab", "/", ".", "é", "ก", "\xff", "*", ",", "x", "A"}

var fixedInputs = []string{
	"", "a", "b", "c", "ab", "ba", "abc", "abcabc", "ac", "a/b", "a.b", "a/b/c", "a.b.c",
	"/a", "/a/", "/a/b", "/a/b.json", "/a/b/c/foo.json", "/a/b/c/d/e/foo.json",
	"/a/b/c/d/e/FOO.json", "/data/my.json", "/data/my.jSon", "data/my.jSon", "ab.json",
	"ab.jpg", "ab.gif", "é", "aé", "éa", "åä", "ก", "กข", "a\xffb", "\xff", "\xef\xbf\xbd",
	"*", "{", "}", ",", "[", "]", "\\", "a,b", "cat", "fat", "at", "this is a test",
	"this is a test3", "my cat has very bright eyes", "my dog has very bright eyes",
	"google.com", "gobwas.com", "https://account.google.com", "https://google.com",
	"http://yahoo.com", "http://google.com", "https://safe.gobwas.com",
	"http://safe.gobwas.com", "http://exclude.gobwas.com", "abcdef", "af", "abczdef",
	"abczqdef", "stagnation", "defghi", "abcda", "/rate", "/usd", "www.google.com",
	"www.yandex.com", "yandex.com", "home", "section", "page", "term", "taxonomy", "en", "th",
	"production", "/biscuit/koalas-march-chocolate", "/images/a.jpg", "images/a.jpg",
	"images/a/b.png", "x.png", "x.min.js", "/menus", "/menu", "/params/a", "/_partials/x.html",
	"a\x00b", "ǅ", "ǆ", "ß", "SS",
}

func try(f func() any) (out any) {
	defer func() {
		if r := recover(); r != nil {
			out = map[string]any{"panic": fmt.Sprint(r)}
		}
	}()
	return f()
}

// matchRow runs m over the inputs: '1' match, '0' no match, 'p' panic.
func matchRow(m func(string) bool, inputs []string) string {
	var b strings.Builder
	for _, in := range inputs {
		func() {
			defer func() {
				if r := recover(); r != nil {
					b.WriteByte('p')
				}
			}()
			if m(in) {
				b.WriteByte('1')
			} else {
				b.WriteByte('0')
			}
		}()
	}
	return b.String()
}

func randomPatterns(r *rand.Rand, n int) []string {
	out := make([]string, n)
	for i := range out {
		k := 1 + r.Intn(6)
		var b strings.Builder
		for j := 0; j < k; j++ {
			b.WriteString(tokens[r.Intn(len(tokens))])
		}
		out[i] = b.String()
	}
	return out
}

func randomInputs(r *rand.Rand, n int) []string {
	out := make([]string, n)
	for i := range out {
		k := r.Intn(7)
		var b strings.Builder
		for j := 0; j < k; j++ {
			b.WriteString(inputAlphabet[r.Intn(len(inputAlphabet))])
		}
		out[i] = b.String()
	}
	return out
}

func dedupe(ss []string) []string {
	seen := map[string]bool{}
	var out []string
	for _, s := range ss {
		if !seen[s] {
			seen[s] = true
			out = append(out, s)
		}
	}
	return out
}

func strs(ss []string) []any {
	out := make([]any, len(ss))
	for i, s := range ss {
		out[i] = goval.Str(s)
	}
	return out
}

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "crates/nh-common/tests/fixtures/glob", "output directory")
	flag.Parse()

	r := rand.New(rand.NewSource(20260928))
	patterns := dedupe(append(append([]string{}, fixedPatterns...), randomPatterns(r, 1500)...))
	inputs := dedupe(append(append([]string{}, fixedInputs...), randomInputs(r, 250)...))

	sepSets := [][]rune{nil, {'/'}, {'.'}, {'/', '.'}, {'é'}}
	sepStrs := make([]any, len(sepSets))
	for i, s := range sepSets {
		sepStrs[i] = string(s)
	}

	write := func(name string, header map[string]any, cases []map[string]any) {
		p := filepath.Join(*out, name)
		if err := goval.WriteCasesGz(p, header, cases); err != nil {
			log.Fatal(err)
		}
		log.Printf("%s: %d cases", p, len(cases))
	}

	// compile + raw match.
	var compileCases, matchCases []map[string]any
	for _, p := range patterns {
		cc := map[string]any{"pattern": goval.Str(p)}
		mc := map[string]any{"pattern": goval.Str(p)}
		var compiled []any
		var rows []any
		for _, seps := range sepSets {
			g, err := glob.Compile(p, seps...)
			if err != nil {
				compiled = append(compiled, map[string]any{"err": err.Error()})
				rows = append(rows, nil)
				continue
			}
			compiled = append(compiled, map[string]any{"ok": fmt.Sprint(g)})
			rows = append(rows, matchRow(g.Match, inputs))
		}
		cc["compile"] = compiled
		mc["raw"] = rows
		// hugofs/glob.GetGlob (cached, lower-cased, '/').
		if g, err := hglob.GetGlob(p); err != nil {
			cc["getGlob"] = map[string]any{"err": err.Error()}
			mc["hugo"] = nil
		} else {
			cc["getGlob"] = map[string]any{"ok": true}
			mc["hugo"] = matchRow(g.Match, inputs)
		}
		compileCases = append(compileCases, cc)
		matchCases = append(matchCases, mc)
	}
	write("compile.json.gz", map[string]any{"separators": sepStrs}, compileCases)
	write("match.json.gz", map[string]any{"separators": sepStrs, "inputs": strs(inputs)}, matchCases)

	write("filter.json.gz", map[string]any{}, filterCases(*root))
}

// filterConfig is one FilenameFilter under test.
type filterConfig struct {
	name       string
	inclusions []string // nil = Go nil
	exclusions []string
	funcEq     string // NewFilenameFilterForInclusionFunc(filename == funcEq)
	funcSuffix string // NewFilenameFilterForInclusionFunc(HasSuffix(filename, funcSuffix))
	appendTo   string // Append this filter to the named one
}

var filterConfigs = []filterConfig{
	{name: "excludeAlmostAllJSON", inclusions: []string{"/a/b/c/foo.json"}, exclusions: []string{"**.json"}},
	{name: "excludeAllButFooJSON", inclusions: []string{"/a/**/foo.json"}, exclusions: []string{"**.json"}},
	{name: "mixedCase", inclusions: []string{"/**/Foo.json"}},
	{name: "nop"},
	{name: "empty", inclusions: []string{}, exclusions: []string{}},
	{name: "includeOnly", inclusions: []string{"**.json", "**.jpg"}},
	{name: "excludeOnly", exclusions: []string{"**.json", "**.jpg"}},
	{name: "docs", inclusions: []string{"docs/**", "content/**"}, exclusions: []string{"**/_index.md", "*.txt"}},
	{name: "images", inclusions: []string{"images/**.{png,jpg}", "/icons/*.svg"}},
	{name: "hidden", exclusions: []string{"**/.*", "**/.*/**", "node_modules/**"}},
	{name: "deep", inclusions: []string{"/a/b/c/d/e/f.md", "x/*/y/**/z.md"}},
	{name: "singleFile", funcEq: "/package.json"},
	{name: "suffix", funcSuffix: ".json"},
	{name: "chain", funcEq: "/postcss.config.js", appendTo: "hidden"},
	{name: "chainOnNil", funcEq: "/package.json", appendTo: "nop"},
	{name: "chainOfChain", inclusions: []string{"**.js"}, appendTo: "chain"},
	{name: "badGlob", inclusions: []string{"[a-"}},
	{name: "badExclude", exclusions: []string{"{a,[}"}},
}

func buildFilter(c filterConfig, built map[string]*hglob.FilenameFilter) (*hglob.FilenameFilter, error) {
	var f *hglob.FilenameFilter
	switch {
	case c.funcEq != "":
		eq := c.funcEq
		f = hglob.NewFilenameFilterForInclusionFunc(func(s string) bool { return s == eq })
	case c.funcSuffix != "":
		suf := c.funcSuffix
		f = hglob.NewFilenameFilterForInclusionFunc(func(s string) bool { return strings.HasSuffix(s, suf) })
	default:
		var err error
		f, err = hglob.NewFilenameFilter(c.inclusions, c.exclusions)
		if err != nil {
			return nil, err
		}
	}
	if c.appendTo != "" {
		f = built[c.appendTo].Append(f)
	}
	return f, nil
}

func filterCases(root string) []map[string]any {
	// Filenames: this repository's docs tree (files and directories), with
	// and without a leading slash, plus edge cases.
	type fname struct {
		s     string
		isDir bool
	}
	var names []fname
	add := func(s string, isDir bool) { names = append(names, fname{s, isDir}) }
	for _, s := range []string{"", "/", "a", "/a", "/a/b", "/a/b/", "/a/b/c", "/a/b/c/foo.json",
		"/a/b/c/foo.bar", "/a/b/c/d/e/foo.json", "/a/b/c/d/e/FOO.json", "/data/my.json",
		"ab.txt", "ab.json", "ab.jpg", "ab.gif", "ab.bson", "/b", "/package.json",
		"package.json", "/postcss.config.js", "/x/1/y/2/3/z.md", "/x/1/y/z.md", "/.hidden",
		"/a/.git/config", "/node_modules/x/index.js", "/images/a.PNG", "/icons/x.svg",
		"/docs/_index.md", "/docs/a/b.md", "/content/x.txt", "x.txt", "/ก/ข.json", "/a\xffb.json"} {
		add(s, false)
		add(s, true)
	}
	docs := filepath.Join(root, "docs")
	err := filepath.WalkDir(docs, func(p string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		rel, err := filepath.Rel(docs, p)
		if err != nil {
			return err
		}
		if rel == "." || strings.HasPrefix(rel, "rust-port") {
			return nil
		}
		rel = filepath.ToSlash(rel)
		add(rel, d.IsDir())
		add("/"+rel, d.IsDir())
		return nil
	})
	if err != nil {
		log.Fatal(err)
	}

	var cases []map[string]any
	built := map[string]*hglob.FilenameFilter{}
	for _, c := range filterConfigs {
		f, err := buildFilter(c, built)
		cfg := map[string]any{
			"name": c.name, "inclusions": c.inclusions, "exclusions": c.exclusions,
			"funcEq": c.funcEq, "funcSuffix": c.funcSuffix, "appendTo": c.appendTo,
		}
		if err != nil {
			cases = append(cases, map[string]any{"config": cfg, "err": err.Error()})
			continue
		}
		built[c.name] = f
		var b strings.Builder
		for _, n := range names {
			if f.Match(n.s, n.isDir) {
				b.WriteByte('1')
			} else {
				b.WriteByte('0')
			}
		}
		cases = append(cases, map[string]any{"config": cfg, "isNil": f == nil, "match": b.String()})
	}

	var nameList []any
	for _, n := range names {
		nameList = append(nameList, []any{goval.Str(n.s), n.isDir})
	}
	cases = append(cases, map[string]any{"names": nameList})

	// The path helpers over the pattern and name strings.
	helperIn := map[string]bool{}
	for _, p := range fixedPatterns {
		helperIn[p] = true
	}
	for _, n := range names {
		helperIn[n.s] = true
	}
	for _, s := range []string{"data/FOO.json", "/data/FOO.json", "./FOO.json", "//", "a/b/**/foo.json",
		"dat?a/foo.json", "a/b[a-c]/foo.json", "assets/**.json", "./", "../a", "a/./b/../c/", "/./."} {
		helperIn[s] = true
	}
	var hs []string
	for s := range helperIn {
		hs = append(hs, s)
	}
	sort.Strings(hs)
	var helpers []any
	for _, s := range hs {
		parts := strings.Split(s, "/")
		helpers = append(helpers, map[string]any{
			"in":                    goval.Str(s),
			"NormalizePath":         goval.Str(hglob.NormalizePath(s)),
			"NormalizePathNoLower":  goval.Str(hglob.NormalizePathNoLower(s)),
			"ResolveRootDir":        goval.Str(hglob.ResolveRootDir(s)),
			"HasGlobChar":           hglob.HasGlobChar(s),
			"FilterGlobParts":       strs(hglob.FilterGlobParts(parts)),
			"QuoteMeta":             goval.Str(glob.QuoteMeta(s)),
			"QuoteMetaCompileMatch": try(func() any { g, err := glob.Compile(glob.QuoteMeta(s)); return err == nil && g.Match(s) }),
		})
	}
	cases = append(cases, map[string]any{"helpers": helpers})
	return cases
}
