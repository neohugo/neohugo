//go:build go1.25

// Command regexp is the Go oracle for nh-common's `goregexp` module, the port of Go's
// `regexp` and `regexp/syntax` packages. It runs go1.27.1's regexp over:
//
//   - a pattern × input matrix (every syntax feature, Unicode and Thai text, invalid UTF-8,
//     empty matches, (?i) with the special folds ſ, K (Kelvin), Σ) through MatchString,
//     FindStringSubmatchIndex, FindAllStringSubmatchIndex, FindAllStringIndex, FindAllString
//     with n, FindAllStringSubmatch, ReplaceAllString (with $1/${name} templates),
//     ReplaceAllLiteralString, ReplaceAllStringFunc and Split, for Perl and longest-match
//     regexps;
//   - the parser, simplifier and compiler (syntax.Regexp.String, Simplify, syntax.Prog.String)
//     and the compile errors of those patterns, of adversarial error patterns and of every
//     Unicode class name and (?i) fold range;
//   - Go's own regexp/testdata/re2-search.txt;
//   - seeded random patterns and inputs;
//   - the regexps of Hugo's Go sources and of the docs site (layouts and content).
//
// It writes gzipped JSON fixtures to crates/nh-common/tests/fixtures/regexp.
//
//	GOTOOLCHAIN=go1.27.1 go run ./tools/go-oracle/nh-common/regexp -out crates/nh-common/tests/fixtures/regexp
//
// It needs go1.25 or later (unicode.CategoryAliases); the fixtures come from go1.27.1 (Unicode 17).
package main

import (
	"bufio"
	"bytes"
	"compress/gzip"
	"encoding/json"
	"flag"
	"fmt"
	"go/build"
	"log"
	"math/rand"
	"os"
	"path/filepath"
	"regexp"
	"regexp/syntax"
	"runtime"
	"sort"
	"strconv"
	"strings"
	"unicode"
	"unicode/utf8"
)

func main() {
	out := flag.String("out", "crates/nh-common/tests/fixtures/regexp", "output directory")
	big := flag.String("big", "", "write only a large random fuzz fixture (out of repo) to this file")
	bigN := flag.Int("bign", 200000, "patterns of the -big fixture")
	bigSeed := flag.Int64("bigseed", 1, "seed of the -big fixture")
	flag.Parse()
	if *big != "" {
		// Red-team mode: many more patterns, longer inputs (the NFA takes over from the
		// backtracker above 256K bits / len(prog)).
		cases := fuzzCases(*bigSeed, *bigN, *bigN, true)
		if err := writeGz(*big, nil, cases); err != nil {
			log.Fatal(err)
		}
		fmt.Printf("%s: %d cases\n", *big, len(cases))
		return
	}
	if err := os.MkdirAll(*out, 0o755); err != nil {
		log.Fatal(err)
	}
	withInputs := func(in []string) map[string]any {
		return map[string]any{"inputs": bss(in), "templates": templates}
	}
	files := []struct {
		name   string
		header map[string]any
		cases  []any
	}{
		{"syntax.json.gz", nil, syntaxCases()},
		{"tables.json.gz", nil, tableCases()},
		{"matrix.json.gz", withInputs(inputs), matrixCases()},
		{"long.json.gz", withInputs(longInputs()), longCases()},
		{"re2search.json.gz", nil, re2SearchCases()},
		{"fuzz.json.gz", nil, fuzzCases(20260928, 20000, 7000, false)},
		{"hugo.json.gz", withInputs(hugoInputs), hugoCases()},
	}
	for _, f := range files {
		if err := writeGz(filepath.Join(*out, f.name), f.header, f.cases); err != nil {
			log.Fatal(err)
		}
		fmt.Printf("%s: %d cases\n", f.name, len(f.cases))
	}
}

func writeGz(path string, header map[string]any, cases []any) error {
	var buf bytes.Buffer
	zw, err := gzip.NewWriterLevel(&buf, gzip.BestCompression)
	if err != nil {
		return err
	}
	enc := json.NewEncoder(zw)
	enc.SetEscapeHTML(false)
	doc := map[string]any{"go": runtime.Version(), "cases": cases}
	for k, v := range header {
		doc[k] = v
	}
	if err := enc.Encode(doc); err != nil {
		return err
	}
	if err := zw.Close(); err != nil {
		return err
	}
	return os.WriteFile(path, buf.Bytes(), 0o644)
}

// bs encodes a Go string: a JSON string when it is valid UTF-8, else {"x": hex}.
func bs(s string) any {
	if utf8.ValidString(s) {
		return s
	}
	return map[string]any{"x": fmt.Sprintf("%x", s)}
}

// big encodes a long string (over 4 KiB) by its length and checksum.
func big(s string) any {
	if len(s) > 4096 {
		return map[string]any{"len": len(s), "ck": fnv([]byte(s))}
	}
	return bs(s)
}

func bss(ss []string) []any {
	if ss == nil {
		return nil
	}
	out := make([]any, len(ss))
	for i, s := range ss {
		out[i] = bs(s)
	}
	return out
}

// fnv is FNV-1a 64 over bytes.
func fnv(b []byte) string {
	h := uint64(14695981039346656037)
	for _, c := range b {
		h ^= uint64(c)
		h *= 1099511628211
	}
	return strconv.FormatUint(h, 16)
}

// intsText is the checksum text of index matches: "a,b,;c,d,;".
func intsText(ms [][]int) []byte {
	var b []byte
	for _, m := range ms {
		for _, v := range m {
			b = strconv.AppendInt(b, int64(v), 10)
			b = append(b, ',')
		}
		b = append(b, ';')
	}
	return b
}

// Templates for ReplaceAllString (Go's Expand rules: $1x is ${1x}, $10, $01, ${name}, $$).
var templates = []string{
	"<$0>",
	"[$1|${1}x|$1x|$2]",
	"${name}.$name.$$.$",
	"${",
	"$10$01${01}${1",
	"x$ß${ß}$_${_}$9",
	"$$1${2}$ł",
}

// Inputs of the matrix.
var inputs = []string{
	"", "a", "aa", "aaa", "ab", "abc", "abcabc", "baaab", "xyz", "banana", "aab", "abab",
	"hello, world", "Hello World", "foo:and:bar", "a,b,,c", " a b  c ", "\n", "a\nb\n", "\r\n",
	"line1\nline2\r\nline3\n", "123-456-7890", "2024-01-02 03:04:05", "x=1&y=2",
	`<h2 id="a">Title</h2><p>x</p><h2>B</h2>`, "<!-- c --><html><head>", "https://www.example.com/path?q=1",
	"_under_score", "k", "K", "\u212a", "s", "S", "\u017f", "ſtraße STRASSE", "ΣσςΣ", "İıIi", "ǅǄǆ",
	"สวัสดีครับ ภาษาไทย 123", "เค้ก-แป้งสาลี", "日本語日本語", "😀😃 emoji", "\xff", "a\xffb", "\xe0\x80\xaf",
	"\xed\xa0\x80", "\xe6\x97", "\xef\xbf\xbd", "a\xe6\x97b\xe6", "tab\there", "$1 ${x}", "(a)[b]{c}",
	"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaab", "\x00\x01\x7f", "word boundary, déjà vu", "x--y---z", "AbCdEf",
	"a.b*c+d?e", "\\", "-ab-axb-axxb-",
}

// Patterns of the matrix and of the syntax dump.
var patterns = []string{
	// literals, dot, anchors
	``, `a`, `ab`, `abc`, `.`, `..`, `.*`, `.+`, `.?`, `^`, `$`, `^$`, `^a`, `a$`, `^abc$`, `\A`, `\z`,
	`\Aa`, `a\z`, `(?m)^`, `(?m)$`, `(?m)^a$`, `(?m)^.*$`, `(?s).`, `(?s).*`, `(?s)a.b`, `a.b`,
	`(?-s).`, `(?m)^\s*$`, `$a`, `a^`, `^*`, `\^\$`,
	// repetition
	`a*`, `a+`, `a?`, `a*?`, `a+?`, `a??`, `a{2}`, `a{2,}`, `a{2,3}`, `a{0}`, `a{0,}`, `a{1}`,
	`a{1,}`, `a{0,1}`, `a{,2}`, `a{2}?`, `a{2,}?`, `(a{2}){3}`, `(?:a*)*`, `(?:a+)+`, `(?:a*)+`,
	`(?:a?)*`, `(a*)*`, `(a*)+`, `(a|b)*`, `(a|b)+?`, `(?U)a+`, `(?U)a+?`, `(?U)(a+)(a*)`,
	`x*y*z*`, `(|a)*`, `(|a)+`, `(a|)*`, `(a|ab)(c|bcd)(d*)`, `a{1000}`, `(a{2,3}){2}`,
	`(?:ab){3,5}`, `[a-c]{2,4}?`, `a{2}{3}`, `{`, `a{`, `a{1`, `a{1,`, `{1}`, `a{01}`, `a{1,x}`,
	`a{,}`,
	// classes
	`[abc]`, `[^abc]`, `[a-z]+`, `[^a-z]+`, `[a\-\]z]+`, `[]a]`, `[^]a]`, `[-a]`, `[a-]`, `[a-c-e]`,
	`[\d]`, `[\D]`, `[\s\S]`, `[^\s]`, `[\w-]+`, `[[:alpha:]]+`, `[[:^alpha:]]+`, `[[:word:]]+`,
	`[[:punct:]]`, `[[:space:]]+`, `[[:upper:][:digit:]]+`, `[\p{Lu}]`, `[^\p{L}]+`, `[\pL\pN]+`,
	`[\x00-\x7f]+`, `[\x{80}-\x{10FFFF}]+`, `[^\x00-\x{10FFFF}]`, `[\n]`, `[^\n]+`, `[.]`, `[*+?]`,
	`[\\]`, `[\[\]]`, `[日本語]+`, `[ก-ฮ]+`, `[a-a]`, `[aA]`, `[kK]`, `[ſs]`, `[Σσ]`, `[^aA]`, `[\x{212A}]`,
	`[\x{FFFD}]`, `[^a]`, `[^\x{FFFD}]`,
	// perl classes and boundaries
	`\d+`, `\D+`, `\s+`, `\S+`, `\w+`, `\W+`, `\b`, `\B`, `\bfoo\b`, `\b\w+\b`, `\Bo\B`, `\bก`,
	`\d{3}-\d{3}-\d{4}`, `^\d+$`, `[-+]?\d*\.\d+`,
	// unicode classes
	`\pL+`, `\PL+`, `\p{L}+`, `\p{Lu}+`, `\p{Ll}+`, `\p{Thai}+`, `\p{Han}+`, `\p{Greek}+`, `\p{^Greek}+`,
	`\P{^Greek}+`, `\p{Latin}+`, `\pN+`, `\p{Nd}+`, `\p{Mn}`, `\p{Zs}`, `\p{Any}+`, `\p{Assigned}+`,
	`\p{ASCII}+`, `\p{Letter}+`, `\p{Lowercase_Letter}+`, `\p{lowercaseletter}`, `\p{old_italic}`,
	`\p{Braille}`, `\pC`, `\p{Cc}+`, `\p{Co}`, `\p{Cs}`, `\p{LC}+`, `\p{Lc}`, `\p{Emoji}`, `\pZ`,
	`\p{Nonexistent}`, `\p{`, `\p{L`, `\p`, `\pX`, `\p{}`, `\p{^}`, `\P{Any}`, `(?i)\p{Lu}+`,
	`(?i)\P{Lu}+`, `(?i)\p{Greek}`, `(?i)\p{ASCII}`, `(?i)[^\p{Ll}]`, `\p{Cn}`,
	// escapes
	`\a\f\n\r\t\v`, `[\a\f\n\r\t\v]+`, `\x41`, `\x{41}`, `\x{10FFFF}`, `\x{110000}`, `\x{}`, `\x{zz}`,
	`\x4`, `\xg0`, `\x`, `\0`, `\07`, `\101`, `\1`, `\8`, `\_`, `\!\\`, `\Q.*+\E`, `\Qab`, `\Q\E`,
	`a\Q|\Eb`, `\C`, `\q`, `\y`, `\z\z`, `\pL\Q\pL\E`, `\x{d800}`, `\x{FFFD}`, `\x{fffd}+`,
	// groups and captures
	`()`, `(a)`, `(a)(b)`, `(.)(.)`, `(.*)`, `(..)(..)`, `(([^xyz]*)(d))`, `((a|b|c)*(d))`,
	`(((a|b|c)*)(d))`, `a*(|(b))c*`, `(.*).*`, `(?:a)`, `(?:a|b)+`, `(?P<name>a+)`,
	`(?P<name>\w+)@(?P<dom>\w+)`, `(?<name>x)`, `(?P<x>hi)|(?P<x>bye)`, `(?P<1>a)`, `(?P<_>a)`,
	`(?P<>a)`, `(?P<a-b>a)`, `(?P<name`, `(?P=name)`, `(?P>a)`, `(?'n'a)`, `(?i)(?P<n>K)`,
	`(a)|b`, `(a)|(b)`, `(a*)+$`, `(aa)*$`, `(a){0}`, `(a)(b){0}(c)`, `((a(b){0}){3}){5}(h)`,
	`(?:(?:^).)`, `(?-s)(?:(?:^).)`, `(?s)(?:(?:^).)`, `(a|ab)(bc|c)`, `(a+)(b+)?`, `(a)?(b)?`,
	// alternation and factoring
	`a|b`, `a|b|c`, `ab|ac`, `abc|abd|aef|bcx|bcy`, `a|ab|abc`, `ab|a`, `0A|0[aA]`, `0[aA]|0A`,
	`(?:A(?:A|a))`, `(?:A|(?:A|a))`, `a|`, `|a`, `|`, `a||b`, `(|)`, `x|[a-c]|y`, `[ab]|[bc]`,
	`.|\n`, `(?s:.)|a`, `a+|b+`, `\d|\w`, `foo|foobar`, `foobar|foo`, `(foo|foobar)x`, `a*|b*`,
	`(a*|b)(c*|d)`, `ab*|ac*`, `(?i)ab|AC`, `abc|ABC`, `a(?:b|c|d)e`, `x{2}a|x{2}b`, `[a-c]{2}x|[a-c]{2}y`,
	// flags
	`(?i)a`, `(?i)abc`, `(?i)k`, `(?i)K`, `(?i)\x{212A}`, `(?i)s`, `(?i)S`, `(?i)ſ`, `(?i)σ`, `(?i)Σ`,
	`(?i)ς`, `(?i)ß`, `(?i)ǅ`, `(?i)i`, `(?i)İ`, `(?i)ı`, `(?i)[k-s]+`, `(?i)[^k]`, `(?i)[a-z]+`,
	`(?i)[^a-z]+`, `(?i)\w+`, `(?i)\W`, `(?i)[[:upper:]]+`, `(?i)[[:^lower:]]+`, `(?i)straße`,
	`(?i)STRASSE`, `(?i)สวัสดี`, `(?i)k(?-i)k`, `(?i:a)b`, `a(?i)b`, `(?i)a(?-i:b)c`, `(?im)^a$`,
	`(?is).`, `(?U)(?i)a+`, `(?-i)a`, `(?i-)a`, `(?-)a`, `(?i-i)a`, `(?x)a`, `(?`, `(?i`, `(?i:`,
	`(?:`, `(?i)`, `(?i)(?s)`, `(?m:^a)b`, `(?s-m:.$)`, `(?-m)$`, `(?i)\Q.K\E`, `(?i)[kK]`,
	`(?i)[\x{212A}]`, `(?i)[ſs]+`, `(?i)ſ+`,
	// errors
	`*`, `+`, `?`, `a**`, `a*+`, `a++`, `a??*`, `a{2}*`, `a*{2}`, `(abc`, `abc)`, `)`, `(()`, `())`,
	`x[a-z`, `[z-a]`, `[a`, `[`, `[]`, `[^]`, `abc\`, `\`, `a{1001}`, `a{1000,1001}`, `a{2,1}`,
	`(a{500}){3}`, `((a{10}){10}){11}`, `a{100000000}`, `a{99999999}`, `[[:foo:]]`, `[[:alpha:]`,
	`[[:alpha]`, `[a-\d]`, `[\d-z]`, `(*)`, `(|*)`, `(?P<n>*)`, "\xff", "a\xffb", "[\xff]", "(?P<\xff>a)",
	"\\p{\xff}", "\\Q\xff\\E", "(?\xff)", "\\x{\xff}", "\\\xff", "\\p\xff", "[a-\xff]", "x{2}\xff",
}

// Long inputs (the backtracker hands over to the NFA above 256K bits / len(prog)).
func longInputs() []string {
	var b strings.Builder
	for i := 0; b.Len() < 40000; i++ {
		fmt.Fprintf(&b, `<h2 id="s%d">Section %d</h2><p>Text ภาษาไทย %d, word-%d.</p>`+"\n", i, i, i*7, i%13)
	}
	html := b.String()
	return []string{
		html,
		strings.Repeat("a", 30000) + "b",
		strings.Repeat("ab\xffc ", 6000),
		strings.Repeat("สวัสดีครับ ", 3000),
		strings.Repeat("x", 70000),
	}
}

type pat struct {
	p       string
	longest bool
}

func compile(p pat) (*regexp.Regexp, error) {
	re, err := regexp.Compile(p.p)
	if err != nil {
		return nil, err
	}
	if p.longest {
		re.Longest()
	}
	return re, nil
}

func indexes(ms [][]int) any {
	if ms == nil {
		return nil
	}
	return ms
}

func strsAll(ss [][]string) any {
	if ss == nil {
		return nil
	}
	out := make([]any, len(ss))
	for i, s := range ss {
		out[i] = bss(s)
	}
	return out
}

// header is the per-pattern information: the compile error, or NumSubexp, SubexpNames and
// LiteralPrefix.
func header(p pat) (map[string]any, *regexp.Regexp) {
	c := map[string]any{"p": bs(p.p)}
	if p.longest {
		c["longest"] = true
	}
	re, err := compile(p)
	if err != nil {
		c["err"] = big(err.Error())
		return c, nil
	}
	prefix, complete := re.LiteralPrefix()
	c["n"] = re.NumSubexp()
	c["names"] = re.SubexpNames()
	c["prefix"] = []any{bs(prefix), complete}
	return c, re
}

// full runs every operation of one regexp over one input.
func full(re *regexp.Regexp, s string) []any {
	var repl []any
	for _, t := range templates {
		repl = append(repl, bs(re.ReplaceAllString(s, t)))
	}
	return []any{
		re.MatchString(s),
		re.FindStringSubmatchIndex(s),
		indexes(re.FindAllStringSubmatchIndex(s, -1)),
		indexes(re.FindAllStringIndex(s, -1)),
		bss(re.FindAllString(s, 2)),
		strsAll(re.FindAllStringSubmatch(s, 3)),
		bs(re.FindString(s)),
		bss(re.FindStringSubmatch(s)),
		repl,
		bs(re.ReplaceAllLiteralString(s, "<$1>")),
		bs(re.ReplaceAllStringFunc(s, func(m string) string { return "(" + strconv.Itoa(len(m)) + ")" })),
		bss(re.Split(s, -1)),
		bss(re.Split(s, 2)),
		bss(re.Split(s, 0)),
		indexes(re.FindAllStringIndex(s, 1)),
	}
}

func matrixCases() []any {
	var cases []any
	for _, longest := range []bool{false, true} {
		for _, p := range patterns {
			c, re := header(pat{p, longest})
			if re != nil {
				var r []any
				for _, s := range inputs {
					r = append(r, full(re, s))
				}
				c["r"] = r
			}
			cases = append(cases, c)
		}
	}
	// Go's own find/replace/split tables (find_test.go, all_test.go).
	for _, t := range goTablePairs {
		c, re := header(pat{t[0], false})
		if re != nil {
			c["in"] = bs(t[1])
			c["r"] = []any{full(re, t[1])}
		}
		cases = append(cases, c)
	}
	return cases
}

// longCases: results on long inputs, as counts and checksums.
func longCases() []any {
	long := longInputs()
	var cases []any
	ps := []string{
		`a`, `a+`, `a*`, `x*`, `b`, `\w+`, `\b`, `\B`, `(?s)<h2.*?>.*?</h2>`, `<h2 id="(\w+)">([^<]*)</h2>`,
		`\p{Thai}+`, `[^\x00-\x7f]+`, `.`, `(?s).`, `\n`, `(?m)^<p>`, `(?m)\.</p>$`, `word-(\d+)`,
		`(?i)SECTION \d+`, `ab\xffc`, "\xff", `\x{FFFD}`, `[^a-z]`, `(a|b)*c`, `(x+x+)+y`, `^x*$`,
		`(?:x{3}){2}`, `ครับ`, `(?i)ครับ\s`, `\d{2,}`, `(a*)*b`, `[a-c]+?`, `(?U)x+`, `$`, `^`,
		`(?P<n>\d+)\.`, `[[:punct:]]+`, `a{1000}`, `(?s)(.)(.)(.)(.)(.)(.)(.)(.)(.)(.)(.)`,
		`s(\d+)|Section (\d+)|word`,
	}
	for _, longest := range []bool{false, true} {
		for _, p := range ps {
			c, re := header(pat{p, longest})
			if re != nil {
				var r []any
				for _, s := range long {
					all := re.FindAllStringSubmatchIndex(s, -1)
					r = append(r, []any{
						re.MatchString(s),
						re.FindStringSubmatchIndex(s),
						len(all),
						fnv(intsText(all)),
						fnv(intsText(re.FindAllStringIndex(s, 100))),
						fnv([]byte(re.ReplaceAllString(s, "<$1|$0>"))),
						len(re.Split(s, -1)),
					})
				}
				c["r"] = r
			}
			cases = append(cases, c)
		}
	}
	return cases
}

// syntaxDump is the parse tree, simplified tree and program of a pattern (Perl and POSIX
// flags), or the errors.
func syntaxDump(p string) map[string]any {
	c := map[string]any{"p": bs(p)}
	for _, mode := range []struct {
		name  string
		flags syntax.Flags
	}{{"perl", syntax.Perl}, {"posix", syntax.POSIX}, {"lit", syntax.Perl | syntax.Literal}} {
		re, err := syntax.Parse(p, mode.flags)
		if err != nil {
			c[mode.name] = map[string]any{"err": big(err.Error())}
			continue
		}
		s := re.Simplify()
		prog, err := syntax.Compile(s)
		if err != nil {
			log.Fatal(err)
		}
		c[mode.name] = map[string]any{
			"re":    big(re.String()),
			"simp":  big(s.String()),
			"prog":  big(prog.String()),
			"cap":   re.MaxCap(),
			"names": re.CapNames(),
		}
	}
	_, err := regexp.CompilePOSIX(p)
	c["cposix"] = errStr(err)
	return c
}

func errStr(err error) any {
	if err == nil {
		return nil
	}
	return big(err.Error())
}

func syntaxCases() []any {
	var cases []any
	var all []string
	all = append(all, patterns...)
	for _, t := range goTablePairs {
		all = append(all, t[0])
	}
	all = append(all, hugoPatterns...)
	all = append(all, errorPatterns()...)
	for _, p := range all {
		cases = append(cases, syntaxDump(p))
	}
	return cases
}

// errorPatterns: size and nesting limits and other adversarial compile errors.
func errorPatterns() []string {
	return []string{
		strings.Repeat(`\pL`, 27000),
		strings.Repeat(`\pL`, 100),
		strings.Repeat("(", 1000) + strings.Repeat(")", 1000),
		strings.Repeat("(", 1001) + strings.Repeat(")", 1001),
		strings.Repeat("(?:", 999) + "a" + strings.Repeat(")", 999),
		strings.Repeat("(?:", 1000) + "a" + strings.Repeat(")", 1000),
		strings.Repeat("(?:a", 1500) + strings.Repeat(")", 1500),
		strings.Repeat("a|", 3000) + "b",
		strings.Repeat("(a|b)", 1200),
		strings.Repeat("[a-z]*", 1100),
		strings.Repeat("a*", 1100),
		`((((((((((a{2}){2}){2}){2}){2}){2}){2}){2}){2}){2})`,
		`(((((((((a{2}){2}){2}){2}){2}){2}){2}){2}){2})`,
		`(a{1000}){1000}`,
		`((a{100}){100}){100}`,
		`\pL{1000}`,
		`[\pL\pN]{500}[\pL\pN]{500}`,
		`.{1000}.{1000}.{1000}`,
		`(?:.{1000}){3}`,
		`(x{500}y{500}){3}`,
		strings.Repeat(`(?i)\p{L}`, 3000),
		strings.Repeat("x{1000}", 3400),
		strings.Repeat("(x{1000})", 3300),
		strings.Repeat("a{2}", 2000) + "(",
		"(" + strings.Repeat("a", 5000) + "|" + strings.Repeat("b", 5000) + ")*",
		strings.Repeat(`[\x{0}-\x{10FFFF}]`, 100),
		strings.Repeat(`\Qab\E`, 700),
		strings.Repeat("ab|ac|", 800) + "ad",
		strings.Repeat("(?:ab|ac)", 600),
	}
}

// tableCases: every Unicode class name (categories, scripts, aliases, special names, in
// several spellings) with and without (?i), and the (?i) closure of every rune range.
func tableCases() []any {
	var names []string
	for n := range unicode.Categories {
		names = append(names, n)
	}
	for n := range unicode.Scripts {
		names = append(names, n)
	}
	for n := range unicode.CategoryAliases {
		names = append(names, n)
	}
	sort.Strings(names)
	extra := []string{"Any", "Assigned", "ASCII", "Ascii", "ascii", "LC", "lc", "L&", "any", "ASSIGNED",
		"old-italic", "Old Italic", "OLD_ITALIC", "_greek_", "letter", "LETTER", "cased_letter", "Cased-Letter",
		"Nonexistent", "L_", "_", "-", "", "^", "^L", "^Any", "^Assigned"}
	names = append(names, extra...)
	var cases []any
	for _, n := range names {
		for _, f := range []string{`\p{%s}`, `\P{%s}`, `(?i)\p{%s}`, `(?i)\P{%s}`, `[^\p{%s}a]`, `(?i)[\p{%s}k]`} {
			p := fmt.Sprintf(f, n)
			re, err := syntax.Parse(p, syntax.Perl)
			if err != nil {
				cases = append(cases, map[string]any{"p": p, "err": err.Error()})
				continue
			}
			prog, err := syntax.Compile(re.Simplify())
			if err != nil {
				log.Fatal(err)
			}
			ps := prog.String()
			cases = append(cases, map[string]any{"p": p, "len": len(ps), "ck": fnv([]byte(ps))})
		}
	}
	// (?i) of every rune range (appendFoldedRange), in 4096-rune chunks and as single runes.
	for lo := rune(0); lo <= unicode.MaxRune; lo += 0x1000 {
		for _, f := range []string{`(?i)[\x{%x}-\x{%x}]`, `(?i)[^\x{%x}-\x{%x}]`} {
			p := fmt.Sprintf(f, lo, lo+0xfff)
			re, err := syntax.Parse(p, syntax.Perl)
			if err != nil {
				log.Fatal(err)
			}
			s := re.String()
			cases = append(cases, map[string]any{"p": p, "len": len(s), "ck": fnv([]byte(s))})
		}
	}
	var single strings.Builder
	for r := rune(0); r <= 0x1ffff; r++ {
		if unicode.SimpleFold(r) == r && r > 0x80 {
			continue
		}
		re, err := syntax.Parse(fmt.Sprintf(`(?i)\x{%x}`, r), syntax.Perl)
		if err != nil {
			log.Fatal(err)
		}
		prog, err := syntax.Compile(re.Simplify())
		if err != nil {
			log.Fatal(err)
		}
		single.WriteString(re.String())
		single.WriteString(prog.String())
	}
	cases = append(cases, map[string]any{"p": "single-folds", "len": single.Len(), "ck": fnv([]byte(single.String()))})
	return cases
}

// re2SearchCases: Go's regexp/testdata/re2-search.txt (patterns × strings), leftmost-first and
// leftmost-longest.
func re2SearchCases() []any {
	path := filepath.Join(build.Default.GOROOT, "src", "regexp", "testdata", "re2-search.txt")
	data, err := os.ReadFile(path)
	if err != nil {
		log.Fatal(err)
	}
	var cases []any
	var strs []string
	var inStrings bool
	sc := bufio.NewScanner(bytes.NewReader(data))
	for sc.Scan() {
		line := sc.Text()
		switch {
		case line == "" || line[0] == '#' || line == "Regexp.SearchTests":
			continue
		case line == "strings":
			strs = strs[:0]
			inStrings = true
			continue
		case line == "regexps":
			inStrings = false
			continue
		case line[0] != '"':
			continue // expected results
		}
		q, err := strconv.Unquote(line)
		if err != nil {
			log.Fatalf("%s: %v", line, err)
		}
		if inStrings {
			strs = append(strs, q)
			continue
		}
		for _, longest := range []bool{false, true} {
			c, re := header(pat{q, longest})
			if re != nil {
				var r []any
				for _, s := range strs {
					r = append(r, []any{
						bs(s),
						re.MatchString(s),
						re.FindStringSubmatchIndex(s),
						indexes(re.FindAllStringSubmatchIndex(s, -1)),
						bs(re.ReplaceAllString(s, "<$1>")),
					})
				}
				c["r"] = r
			}
			cases = append(cases, c)
		}
	}
	if err := sc.Err(); err != nil {
		log.Fatal(err)
	}
	return cases
}

var fuzzTokens = []string{
	"a", "b", "c", "k", "s", "ſ", "K", "ก", "日", ".", "*", "+", "?", "|", "(", ")", "[", "]", "^", "$",
	"-", `\d`, `\w`, `\s`, `\W`, `\b`, `\B`, `\A`, `\z`, "{2}", "{1,3}", "{0,}", "{,2}", "{", "}",
	"(?i)", "(?s)", "(?m)", "(?U)", "(?:", "(?P<n>", "[^", `\pL`, `\p{Thai}`, `\PL`, `[:alpha:]`,
	`\x{212A}`, `\x41`, `\Q`, `\E`, `\`, "\n", ",", "_", "1", "a*", "b+", "(a)", "[a-c]", "[^b]",
}

var fuzzInputChars = []string{"a", "b", "c", "k", "K", "s", "S", "ſ", "\u212a", "ก", "日", "\n", " ", "_", "1", "-", ",", "\xff", "\xe0"}

// fuzzCases: n random patterns; the first maxCompiled that compile are run on 5 random inputs
// each (with long, when some inputs are up to 5000 runes long).
func fuzzCases(seed int64, n, maxCompiled int, long bool) []any {
	rng := rand.New(rand.NewSource(seed))
	var cases []any
	var compiled int
	for i := 0; i < n; i++ {
		var b strings.Builder
		n := 1 + rng.Intn(9)
		for j := 0; j < n; j++ {
			b.WriteString(fuzzTokens[rng.Intn(len(fuzzTokens))])
		}
		p := b.String()
		longest := rng.Intn(4) == 0
		c, re := header(pat{p, longest})
		if re == nil {
			cases = append(cases, c)
			continue
		}
		compiled++
		if compiled > maxCompiled {
			// Enough matching cases: keep the compile result only.
			cases = append(cases, c)
			continue
		}
		var r []any
		for k := 0; k < 5; k++ {
			var ib strings.Builder
			m := rng.Intn(13)
			if long {
				switch rng.Intn(10) {
				case 0:
					m = 1000 + rng.Intn(4000)
				case 1, 2:
					m = rng.Intn(200)
				}
			}
			for j := 0; j < m; j++ {
				ib.WriteString(fuzzInputChars[rng.Intn(len(fuzzInputChars))])
			}
			s := ib.String()
			r = append(r, []any{
				bs(s),
				indexes(re.FindAllStringSubmatchIndex(s, -1)),
				bs(re.ReplaceAllString(s, "<$1>")),
				bss(re.Split(s, -1)),
			})
		}
		c["r"] = r
		cases = append(cases, c)
	}
	return cases
}

// hugoPatterns: the regexps of Hugo's Go sources (regexp.MustCompile) and of the docs site.
var hugoPatterns = []string{
	// Go sources
	`dir has been modified \((.*?)\)`,
	`(babel|postcss|tailwind)\.config\.js`,
	`.*\..{1,6}$`,
	`[<\[](\d{4}-\d{2}-\d{2}) .*[>\]]`,
	`^(true|false)$`,
	`^[-+]?\d+$`,
	`^[-+]?\d*\.\d+$`,
	`{{[%,<][^\/]`,
	`\"(\w+)\":`,
	`\"(enable\w+)\":null`,
	`^"HTTP`,
	"^(application|text)/(x-)?(java|ecma)script$",
	`^(application|text)/(x-|(ld|manifest)\+)?json$`,
	`^\/[a-zA-Z0-9]{4}(\/[a-zA-Z0-9]+)?(\/[a-zA-Z0-9]+)?`,
	`at <(.*)>: error calling (.*?): runtime error: invalid memory address or nil pointer dereference`,
	`executing "__hdeferred/.*?" `,
	`\x1b\[[0-9;]*m`,
	`(?s)not found:|could not determine executable`,
	`(?s)^(?:\s+|<!--.*?-->|<\?.*?\?>)*`,
	`(?is)^<!doctype\s[^>]*>`,
	`(?is)^<html(?:\s[^>]*)?>`,
	`(?is)^<head(?:\s[^>]*)?>`,
	`(?i)<meta\s+name=['|"]?generator['|"]?`,
	`'?(.*?)'?:\s.*`,
	`(?i)^class$|transition`,
	`(?i)^(pre|textarea|script|style)`,
	`(?i)^!DOCTYPE`,
	`{{__hugo_ctx( pid=\d+)?/?}}\n?`,
	`^<p>\[!([a-zA-Z]+)\](-|\+)?[^\S\r\n]?([^\n]*)\n?`,
	`go1.(\d*)`,
	`//# sourceMappingURL=.*\n?`,
	`^#[0-9a-fA-F]{3,6}$`,
	`^([a-zA-Z-]+)\(`,
	`^([0-9]+)(\.[0-9]+)?([a-zA-Z-%]+)$`,
	`.*(@import "(.*\.css)";).*`,
	`.*(\/\* HUGO_IMPORT_START (.*) HUGO_IMPORT_END \*\/).*`,
	`> (\d+) \|`,
	`(?i)^((HTTPS?|NO)_PROXY|GO\w+)$`,
	`^(dart-)?sass(-embedded)?$`,
	`(?i)hugo_stats\.json`,
	`(?i)\.(js|ts|jsx|tsx)`,
	`(?i)\.(css|scss|sass)`,
	`(postcss|tailwind)\.config\.js`,
	`^.*$`,
	// docs layouts and content
	`\n+`,
	`#-hugo-placeholder-#`,
	"^https?://(www\\.)?([^/]+).*",
	`(?s)<h2.*?>.*?</h2>`,
	`a(x*)b`,
	`(-{2,})`,
	"^https?://([^/]+).*",
	`(?i)^victor`,
	`<h2.*?>((.|\n)*?)</h2>`,
	`<h2 id="(.+?)">(.+?)</h2>`,
}

var hugoInputs = []string{
	"", "https://www.gohugo.io/docs/", "http://example.org", "HTTPS://example.org/x", "mailto:x",
	"a---b----c--d", "a-b-c", "-ab-axxb-", "-ab-axb-", "line1\n\n\nline2\n",
	"<h2 id=\"x\">Heading <em>one</em></h2>\n<p>Text</p>\n<h2>Two\nlines</h2>",
	"<!DOCTYPE html>\n<html lang=\"th\"><head><meta name=\"generator\" content=\"Hugo\">",
	"  <!-- comment -->\n<?xml version=\"1.0\"?><html>", "Victor Hugo", "victor", "VICTORIA", "Vic",
	`{"enableGitInfo":null,"HTTPCache":{},"title":"x"}`, "true", "false", "True", "-12", "+3", "12.5",
	".5", "-.5", "1.", "{{< figure >}}", "{{% x %}}", "{{/* c */}}", "{{__hugo_ctx pid=12}}\n",
	"{{__hugo_ctx/}}", "<p>[!NOTE]+ Title\nbody</p>", "<p>[!warning] x</p>", "go1.27.1", "go1.",
	"//# sourceMappingURL=data:x\n", "#fff", "#12345G", "rgba(1,2,3)", "12.5px", "10%", "@import \"a.css\";",
	"/* HUGO_IMPORT_START a.css HUGO_IMPORT_END */", "> 12 |", "\x1b[31mred\x1b[0m", "GOPATH",
	"https_proxy", "NO_PROXY", "dart-sass-embedded", "hugo_stats.json", "main.TSX", "x.scss",
	"postcss.config.js", "tailwind.config.js", "a/b/c.html", "file.markdown", "/abcd/efgh/ij",
	"/ab/cd", "at <.Foo>: error calling Bar: runtime error: invalid memory address or nil pointer dereference",
	`executing "__hdeferred/abc" at`, "exec: \"foo\": executable file not found in $PATH", "x not found: y",
	"'class': 1", "class", "CLASS", "x-transition", "PRE", "script", "เค้ก-แป้งสาลี", "สวัสดี\n\nครับ",
}

func hugoCases() []any {
	var cases []any
	for _, p := range hugoPatterns {
		c, re := header(pat{p, false})
		if re != nil {
			var r []any
			for _, s := range hugoInputs {
				r = append(r, full(re, s))
			}
			c["r"] = r
		}
		cases = append(cases, c)
	}
	return cases
}

// goTablePairs: the pattern/text pairs of Go's find_test.go (findTests), all_test.go
// (replaceTests, replaceLiteralTests, splitTests) and exec_test.go.
var goTablePairs = [][2]string{
	{``, ``}, {`^abcdefg`, "abcdefg"}, {`a+`, "baaab"}, {`a`, "bababaab"}, {"abcd..", "abcdef"},
	{`a`, "a"}, {`x`, "y"}, {`b`, "abc"}, {`.`, "a"}, {`.*`, "abcdef"}, {`^`, "abcde"}, {`$`, "abcde"},
	{`^abcd$`, "abcd"}, {`^bcd'`, "abcdef"}, {`^abcd$`, "abcde"}, {`a*`, "baaab"}, {`[a-z]+`, "abcd"},
	{`[^a-z]+`, "ab1234cd"}, {`[a\-\]z]+`, "az]-bcz"}, {`[^\n]+`, "abcd\n"}, {`[日本語]+`, "日本語日本語"},
	{`日本語+`, "日本語"}, {`日本語+`, "日本語語語語"}, {`()`, ""}, {`(a)`, "a"}, {`(.)(.)`, "日a"},
	{`(.*)`, ""}, {`(.*)`, "abcd"}, {`(..)(..)`, "abcd"}, {`(([^xyz]*)(d))`, "abcd"},
	{`((a|b|c)*(d))`, "abcd"}, {`(((a|b|c)*)(d))`, "abcd"}, {`\a\f\n\r\t\v`, "\a\f\n\r\t\v"},
	{`[\a\f\n\r\t\v]+`, "\a\f\n\r\t\v"}, {`a*(|(b))c*`, "aacc"}, {`(.*).*`, "ab"}, {`[.]`, "."},
	{`/$`, "/abc/"}, {`/$`, "/abc"}, {`.`, "abc"}, {`(.)`, "abc"}, {`.(.)`, "abcd"}, {`ab*`, "abbaab"},
	{`a(b*)`, "abbaab"}, {`ab$`, "cab"}, {`axxb$`, "axxcb"}, {`data`, "daXY data"}, {`da(.)a$`, "daXY data"},
	{`zx+`, "zzx"}, {`ab$`, "abcab"}, {`(aa)*$`, "a"}, {`(?:.|(?:.a))`, ""}, {`(?:A(?:A|a))`, "Aa"},
	{`(?:A|(?:A|a))`, "a"}, {`(a){0}`, ""}, {`(?-s)(?:(?:^).)`, "\n"}, {`(?s)(?:(?:^).)`, "\n"},
	{`(?:(?:^).)`, "\n"}, {`\b`, "x"}, {`\b`, "xx"}, {`\b`, "x y"}, {`\b`, "xx yy"}, {`\B`, "x"},
	{`\B`, "xx"}, {`\B`, "x y"}, {`\B`, "xx yy"}, {`(|a)*`, "aa"}, {`0A|0[aA]`, "0a"}, {`0[aA]|0A`, "0a"},
	{`[^\S\s]`, "abcd"}, {`[^\S[:space:]]`, "abcd"}, {`[^\D\d]`, "abcd"}, {`[^\D[:digit:]]`, "abcd"},
	{`(?i)\W`, "x"}, {`(?i)\W`, "k"}, {`(?i)\W`, "s"}, {`[^\x00-\x{10FFFF}]`, "abcd"},
	{`[^\x00-\x{10FFFF}]`, ""}, {`(?:a+)(?:a+)`, "aaaa"}, {`(a+)(b+)`, "aabb"},
	{`a*`, "aaa"}, {`\\`, `\`}, {`\+`, "+"}, {`(?:x|(?:xa))`, "xa"}, {`(?:xa|x)`, "xa"},
	{"", "x"}, {"b", "abc"}, {"y", "abc"}, {"[a-c]*", "\u65e5"}, {"[^\u65e5]", "abc\u65e5def"},
	{"^[a-c]*", "abcdabc"}, {"[a-c]*$", "abcdabc"}, {"^[a-c]*$", "abcdabc"}, {"^[a-c]*", "dabce"},
	{"[a-c]*$", "dabce"}, {"^[a-c]+", "abcdabc"}, {"[a-c]+$", "abc"}, {"abc", "abcdefg"},
	{"bc", "abcbcdcdedef"}, {"x", "xxxXxxx"}, {".+", "abc"}, {"[a-c]*", "def"}, {"[a-c]+", "abcbcdcdedef"},
	{"[a-c]*", "abcbcdcdedef"}, {"a+", "banana"}, {"hello, (.+)", "hello, world"},
	{"hello, (?P<noun>.+)", "hello, world"}, {"(?P<x>hi)|(?P<x>bye)", "hi"}, {"(?P<x>hi)|(?P<x>bye)", "bye"},
	{"(x)?", "123"}, {"(a)(b){0}(c)", "xacxacx"}, {"(a)(((b))){0}c", "xacxacx"},
	{"((a(b){0}){3}){5}(h)", "say aaaaaaaaaaaaaaaah"}, {"((a(b){0}){3}){5}h", "say aaaaaaaaaaaaaaaah"},
	{"[a-c]", "defabcdef"}, {"[a-c]+", "defabcdef"}, {"[a-c]*", "defabcdef"}, {":", "foo:and:bar"},
	{"foo", "foo:and:bar"}, {"bar", "foo:and:bar"}, {"baz", "foo:and:bar"}, {"a", "baabaab"},
	{"a*", "baabaab"}, {"ba*", "baabaab"}, {"f*b*", "foobar"}, {"f+.*b+", "foobar"}, {"o{2}", "foobooboar"},
	{",", "a,b,c,d,e,f"}, {",", ","}, {",", ",,,"}, {",", ""}, {".*", ""}, {".+", ""}, {"", ""},
	{"", "foobar"}, {"a*", "abaabaccadaaae"}, {":", ":x:y:z:"},
}
