package main

// Red-team generators (fuzz -mode long|patho|unicode|labels|tabs|hugomix)
// and the exhaustive short-string enumerator (enum). They aim at what the
// token/byte/deep/ext grammars under-sample: very long documents and lines,
// deep nesting, cmark-style pathological inputs, Unicode flanking and case
// folding, entities, tab expansion at every column, CR/CRLF line endings and
// the interplay of Hugo's extension set. See crates/goldmark/PORTING.md.

import (
	"bytes"
	"flag"
	"fmt"
	"math/rand"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"
	"unicode"

	gast "github.com/yuin/goldmark/ast"
	"github.com/yuin/goldmark/parser"
	"github.com/yuin/goldmark/text"
)

func redteamDoc(mode string, r *rand.Rand) (string, bool) {
	switch mode {
	case "long":
		return fuzzLongDoc(r), true
	case "patho":
		return fuzzPathoDoc(r, 16), true
	case "pathosmall":
		return fuzzPathoDoc(r, 11), true
	case "unicode":
		return fuzzUnicodeDoc(r), true
	case "labels":
		return fuzzLabelDoc(r), true
	case "tabs":
		return fuzzTabsDoc(r), true
	case "hugomix":
		return fuzzHugoMixDoc(r), true
	}
	return "", false
}

// lineEndings rewrites "\n" as "\r\n" or "\r" (all or some of them).
func lineEndings(r *rand.Rand, s string) string {
	switch r.Intn(8) {
	case 0:
		return strings.ReplaceAll(s, "\n", "\r\n")
	case 1:
		return strings.ReplaceAll(s, "\n", "\r")
	case 2:
		var b strings.Builder
		for _, part := range strings.SplitAfter(s, "\n") {
			if strings.HasSuffix(part, "\n") && r.Intn(2) == 0 {
				part = part[:len(part)-1] + []string{"\r\n", "\r", "\r\r\n", "\n\r"}[r.Intn(4)]
			}
			b.WriteString(part)
		}
		return b.String()
	}
	return s
}

// ---------------------------------------------------------------------------
// long: 100 KB – 5 MB documents and lines.

func fuzzLongDoc(r *rand.Rand) string {
	target := 100000 << r.Intn(6)
	if r.Intn(4) == 0 {
		target = 100000 + r.Intn(4900000)
	}
	if target > 5000000 {
		target = 5000000
	}
	var b strings.Builder
	switch r.Intn(7) {
	case 0: // many small documents of every grammar
		for b.Len() < target {
			switch r.Intn(4) {
			case 0:
				b.WriteString(fuzzDoc(r))
			case 1:
				b.WriteString(fuzzExtDoc(r))
			case 2:
				b.WriteString(fuzzHugoMixDoc(r))
			default:
				b.WriteString(fuzzBytesDoc(r))
			}
			if r.Intn(2) == 0 {
				b.WriteString("\n")
			}
		}
	case 1: // one very long line of inline tokens
		for b.Len() < target {
			var t string
			if r.Intn(2) == 0 {
				t = fuzzInline[r.Intn(len(fuzzInline))]
			} else {
				t = extFuzzInline[r.Intn(len(extFuzzInline))]
			}
			b.WriteString(strings.ReplaceAll(strings.ReplaceAll(t, "\n", " "), "\r", " "))
		}
	case 2: // a long paragraph (or container) of short inline lines
		prefix := []string{"", "> ", "- ", "1. ", "    ", "> - ", ": ", "[^1]: "}[r.Intn(8)]
		b.WriteString("Term\n")
		for b.Len() < target {
			b.WriteString(prefix)
			k := 1 + r.Intn(10)
			for j := 0; j < k; j++ {
				b.WriteString(strings.ReplaceAll(fuzzInline[r.Intn(len(fuzzInline))], "\n", " "))
			}
			b.WriteString("\n")
			if r.Intn(50) == 0 {
				prefix = []string{"", "> ", "- ", "1. ", "    ", "> - ", ": ", "[^1]: "}[r.Intn(8)]
			}
		}
	case 3: // one block construct repeated with variations
		unit := []func(i int) string{
			func(i int) string { return fmt.Sprintf("- item %d *e* `c`\n", i) },
			func(i int) string { return fmt.Sprintf("%d. item\n", i%1000000000) },
			func(i int) string { return fmt.Sprintf("| c%d | d | [^%d] |\n", i, i%7) },
			func(i int) string { return fmt.Sprintf("> line %d 'q' \"d\"\n", i) },
			func(i int) string { return fmt.Sprintf("[l%d]: /u%d 't'\n", i, i) },
			func(i int) string { return fmt.Sprintf("[l%d] [x][l%d] ", i, i*7%(i+1)) },
			func(i int) string { return fmt.Sprintf("# h%d {#id%d .c}\n", i, i%13) },
			func(i int) string { return fmt.Sprintf("[^%d]: note %d\n", i, i) },
			func(i int) string { return fmt.Sprintf("x[^%d] ", i%97) },
			func(i int) string { return fmt.Sprintf("T%d\n: d%d\n", i, i) },
			func(i int) string { return fmt.Sprintf("http://e%d.com/p?q=%d www.a%d.org a%d@b.cd ", i, i, i, i) },
			func(i int) string { return fmt.Sprintf("<div id=\"%d\">\n", i) },
			func(i int) string { return fmt.Sprintf("*a%d **b _c_ d** ", i) },
			func(i int) string { return fmt.Sprintf("~~s%d~~ ~t~ ", i) },
			func(i int) string { return fmt.Sprintf("- [%c] t%d\n", " xX"[i%3], i) },
		}
		k := r.Intn(len(unit))
		k2 := r.Intn(len(unit))
		if k == 2 {
			b.WriteString("| a | b | c |\n|---|:-:|--:|\n")
		}
		for i := 0; b.Len() < target; i++ {
			if r.Intn(20) == 0 {
				b.WriteString(unit[k2](i))
			} else {
				b.WriteString(unit[k](i))
			}
		}
	case 4: // one random document repeated
		var d string
		for len(d) == 0 {
			d = fuzzHugoMixDoc(r)
		}
		for b.Len() < target {
			b.WriteString(d)
			if r.Intn(2) == 0 {
				b.WriteString("\n")
			}
		}
	case 5: // a long line inside containers
		b.WriteString([]string{"> - ", "1. > ", "- [ ] ", "| ", "# ", "Term\n: "}[r.Intn(6)])
		for b.Len() < target {
			b.WriteString(strings.ReplaceAll(extFuzzInline[r.Intn(len(extFuzzInline))], "\n", " "))
			b.WriteString([]string{" ", "a", "b ", "", "\u00a0"}[r.Intn(5)])
		}
		b.WriteString("\n")
	case 6: // many pathological constructs (some are quadratic in Go too: bounded sizes)
		for b.Len() < target {
			b.WriteString(pathoTemplates[r.Intn(len(pathoTemplates))](1 + r.Intn(5000)))
			b.WriteString("\n\n")
		}
	}
	s := b.String()
	if r.Intn(8) == 0 {
		s = lineEndings(r, s)
	}
	return s
}

// ---------------------------------------------------------------------------
// patho: CommonMark/cmark regression-suite style pathological inputs.

func rep(s string, n int) string { return strings.Repeat(s, n) }

var pathoTemplates = []func(n int) string{
	func(n int) string { return rep("[", n) },
	func(n int) string { return rep("[a", n) },
	func(n int) string { return rep("[", n) + "a" + rep("]", n) },
	func(n int) string { return rep("[", n) + "a" + rep("](b)", n) },
	func(n int) string { return rep("![", n) + "a" + rep("](b)", n) },
	func(n int) string { return rep("[![", n) + "a" + rep("](b)](c)", n) },
	func(n int) string { return rep("*", n) + "a" + rep("*", n) },
	func(n int) string { return rep("_", n) + "a" + rep("_", n) },
	func(n int) string { return rep("*a ", n) + rep("b*", n) },
	func(n int) string { return rep("*a **b ", n) },
	func(n int) string { return rep("**a *b ", n) + rep("*", n) },
	func(n int) string { return rep("*_", n) + "a" + rep("_*", n) },
	func(n int) string { return rep("a**b", n) + rep("c* ", n) },
	func(n int) string { return rep("<", n) },
	func(n int) string { return rep("<a", n) },
	func(n int) string { return rep("<a ", n) },
	func(n int) string { return rep("<!--", n) },
	func(n int) string { return rep("<?", n) },
	func(n int) string { return rep("<![CDATA[", n) },
	func(n int) string { return rep("<a"+rep(" b=c", 3), n) },
	func(n int) string { return "<a" + rep(" b=c", n) + ">" },
	func(n int) string { return "<a" + rep(" b", n) + ">" },
	func(n int) string { return "<!--" + rep("-", n) + "-->" },
	func(n int) string { return rep("`", n) },
	func(n int) string { return rep("`a", n) },
	func(n int) string {
		var b strings.Builder
		for k := 1; k < n && k < 400; k++ {
			b.WriteString(rep("`", k) + "a" + rep("`", k+1) + " ")
		}
		return b.String()
	},
	func(n int) string {
		var b strings.Builder
		for k := 1; k < n && k < 400; k++ {
			b.WriteString(rep("`", k) + "a" + rep("`", k) + " ")
		}
		return b.String()
	},
	func(n int) string { return rep("> ", n) + "a" },
	func(n int) string { return rep("- ", n) + "a" },
	func(n int) string { return rep("1. ", n) + "a" },
	func(n int) string { return rep("-\t", n) + "a" },
	func(n int) string { return rep(">\t", n) + "a" },
	func(n int) string { return rep("> - ", n) + "a" },
	func(n int) string { return rep("- > ", n) + "a\n" + rep("  ", n) + "b" },
	func(n int) string { return "[a](" + rep("(", n) + rep(")", n) + ")" },
	func(n int) string { return "[a](" + rep("(", n%64) + rep(")", n%64) + ")" },
	func(n int) string {
		var b strings.Builder
		for i := 0; i < n; i++ {
			fmt.Fprintf(&b, "[a%d]: /u%d\n", i, i)
		}
		b.WriteString("\n")
		for i := 0; i < n; i++ {
			fmt.Fprintf(&b, "[a%d] ", i)
		}
		return b.String()
	},
	func(n int) string {
		var b strings.Builder
		for i := 0; i < n; i++ {
			fmt.Fprintf(&b, "[^%d]: x%d\n", i, i)
		}
		b.WriteString("\n")
		for i := 0; i < n; i++ {
			fmt.Fprintf(&b, "[^%d] ", n-i)
		}
		return b.String()
	},
	func(n int) string { return "Term\n" + rep(": ", n) + "a" },
	func(n int) string { return rep(": ", n) + "a" },
	func(n int) string { return rep("Term\n: ", n) + "a" },
	func(n int) string { return rep("|a", n) + "\n" + rep("|-", n) + "\n" + rep("|b", n) + "\n" },
	func(n int) string { return "|a|\n|-|\n" + rep("|b", n) + "\n" },
	func(n int) string { return "# h {a=" + rep("[", n) + "}\n" },
	func(n int) string { n = min(n, 3000); return "# h {a=" + rep("{a=", n) + "}\n" },
	func(n int) string { return "# h {a=" + rep("[", n) + rep("]", n) + "}\n" },
	func(n int) string { return "# h {a=" + rep("[", n) + rep("]", n) + "\n" },
	func(n int) string { n = min(n, 3000); return "# h {a=" + rep("{b=", n) + "1" + rep("}", n) + "}\n" },
	func(n int) string { return "# h {a=" + rep("[1,", n) + "2" + rep("]", n) + " .c #d}\n" },
	func(n int) string { return "h {a=" + rep("[", n) + rep("]", n) + "}\n===\n" },
	func(n int) string { n = min(n, 3000); return "# " + rep("{", n) },
	func(n int) string { n = min(n, 3000); return "# " + rep("{a=", n) },
	func(n int) string { n = min(n, 3000); return "# h " + rep("{.a}", n) },
	func(n int) string { return "# h {" + rep(".a ", n) + "}" },
	func(n int) string { return "# h {" + rep("#a ", n) + "}" },
	func(n int) string { return rep("~~", n) },
	func(n int) string { return rep("~", n) + "a" + rep("~", n) },
	func(n int) string { return rep("~~a ", n) + rep("b~~", n) },
	func(n int) string { return "www." + rep("a.", n) + "com" },
	func(n int) string { return "http://" + rep("a", n) + ".com" },
	func(n int) string { return "http://a.com/" + rep("(", n) + rep(")", n) },
	func(n int) string { return "http://a.com/" + rep("&amp;", n) },
	func(n int) string { return rep("a", n) + "@b.com" },
	func(n int) string { return "a@" + rep("b.", n) + "com" },
	func(n int) string { return rep("'", n) },
	func(n int) string { return rep("\"", n) },
	func(n int) string { return rep("'a\" ", n) },
	func(n int) string { return rep("...", n) + rep("--", n) + rep("<<", n) + rep(">>", n) },
	func(n int) string { return rep("\\", n) },
	func(n int) string { return "&" + rep("a", n) + ";" },
	func(n int) string { return "&#" + rep("1", n) + ";" },
	func(n int) string { return "&#x" + rep("f", n) + ";" },
	func(n int) string { return rep("<div>\n", n) },
	func(n int) string { return rep("#", n) + " a" },
	func(n int) string { return "a\n" + rep("=", n) },
	func(n int) string { return rep("`", n) + "\na\n" + rep("`", n) },
	func(n int) string { return "```" + rep("a", n) + "\nb\n```" },
	func(n int) string { return "```a {" + rep("b=[", n) + "\nb\n```" },
	func(n int) string { return "[" + rep("a", 998+n%4) + "]: /u\n\n[" + rep("a", 998+n%4) + "]" },
	func(n int) string { return "[" + rep("a", 998+n%4) + "]" + "\n\n[" + rep("a", 998+n%4) + "]: /u" },
	func(n int) string { return "[^" + rep("a", 996+n%4) + "]: x\n\n[^" + rep("a", 996+n%4) + "]" },
	func(n int) string { return "> a\n" + rep("b\n", n) },
	func(n int) string { return rep("> ", n%2000) + "a\n" + rep("b\n", n) },
	func(n int) string { return rep("a\n", n) },
	func(n int) string { return rep("a ", n) },
	func(n int) string { return rep("- a\n", n) },
	func(n int) string { return rep("- a\n\n", n) },
	func(n int) string { return rep("1. a\n", n) },
	func(n int) string { return "| a | b |\n|---|---|\n" + rep("| c | d |\n", n) },
	func(n int) string { return rep("*", n) },
	func(n int) string { return rep("_", n) },
	func(n int) string { return "**" + rep("a", n) + "**" },
	func(n int) string { return "[" + rep("a", n) + "](b)" },
	func(n int) string { return "[a](" + rep("b", n) + ")" },
	func(n int) string { return "[a](<" + rep("b", n) + ">)" },
	func(n int) string { return "[a](b \"" + rep("c", n) + "\")" },
	func(n int) string { return "<http://" + rep("a", n) + ">" },
	func(n int) string { return "<" + rep("a", n) + "@b.c>" },
	func(n int) string { return rep("\t", n) + "a" },
	func(n int) string { return rep(" ", n) + "a" },
	func(n int) string { return rep("- [ ] ", n) + "a" },
	func(n int) string { return rep("[^", n) },
	func(n int) string { return rep("[^a]", n) + "\n\n[^a]: b" },
	func(n int) string { return rep("!", n) + rep("[a]", n) },
	func(n int) string { return rep("- ", n/20+1) + rep("- - - - - - - - - - ", n/20) + "a" },
	func(n int) string { return rep("* ", n) + "*" },
	func(n int) string { return rep("+ ", n) },
	func(n int) string { return rep("1) ", n) },
	func(n int) string { return "[a]: <" + rep("b", n) + ">" },
	func(n int) string { return "[a]: b \"" + rep("c\n", n) + "\"" },
	func(n int) string { return rep("[a]:\n", n) },
	func(n int) string { return rep("a  \n", n) },
	func(n int) string { return rep("a\\\n", n) },
	func(n int) string { return rep("[a](b) ", n) + rep("[", n) },
	func(n int) string { return rep("[a]", n) + "\n\n[a]: /u" },
	func(n int) string { return rep("[[a]](b) ", n) },
	func(n int) string { return rep("![[a](b)](c) ", n) },
	func(n int) string { return rep("[a](<b)", n) },
	func(n int) string { return rep("*a_", n) + rep("_b*", n) },
	func(n int) string { return rep("\u00a0*a*\u00a0", n) },
	func(n int) string { return rep("-", n) + "\n" + rep("=", n) },
	func(n int) string { return rep("[^1]: ", n) + "a\n\nb[^1]" },
	func(n int) string { return rep("[^1]:\n    ", n) + "a\n\nb[^1]" },
	func(n int) string { return rep("[^1]: [^1]", n) },
	func(n int) string { return rep("- Term\n  : ", n%500) + "a" },
	func(n int) string { return rep("| [^1] ", n) + "|\n" + rep("|-", n) + "|\n\n[^1]: x" },
}

// fuzzPathoDoc: log-uniform sizes 1 .. 2^maxShift.
func fuzzPathoDoc(r *rand.Rand, maxShift int) string {
	n := 1 << uint(r.Intn(maxShift))
	n += r.Intn(n)
	s := pathoTemplates[r.Intn(len(pathoTemplates))](n)
	if len(s) > 3000000 {
		s = s[:3000000]
	}
	var b strings.Builder
	if r.Intn(3) == 0 {
		b.WriteString([]string{"> ", "- ", "1. ", "    ", "Term\n: ", "[^1]: ", "| ", "# ", "a\n"}[r.Intn(9)])
	}
	b.WriteString(s)
	if r.Intn(3) == 0 {
		b.WriteString([]string{"\n", "\n\n", "a", "]", ")", "*", "`", "}", "\n[a]: /u\n", "\n[^1]: n\n"}[r.Intn(10)])
	}
	out := b.String()
	if r.Intn(10) == 0 {
		out = lineEndings(r, out)
	}
	return out
}

// ---------------------------------------------------------------------------
// unicode: flanking with every Unicode P/S/Z rune, invalid UTF-8, entities.

var (
	rtPunct   []rune // unicode.IsPunct || unicode.IsSymbol (every category P and S)
	rtSpace   []rune // unicode.IsSpace, Zs/Zl/Zp and friends
	rtLetters []rune // a sample of letters, marks and digits
	rtFold    []rune // runes with a non-trivial simple fold orbit
	rtEntity  []string
)

func initRuneTables() {
	if rtPunct != nil {
		return
	}
	for c := rune(0); c <= unicode.MaxRune; c++ {
		switch {
		case unicode.IsPunct(c) || unicode.IsSymbol(c):
			rtPunct = append(rtPunct, c)
		case unicode.IsSpace(c) || unicode.In(c, unicode.Zs, unicode.Zl, unicode.Zp) || c == 0x200b || c == 0xfeff || c == 0x180e:
			rtSpace = append(rtSpace, c)
		case (unicode.IsLetter(c) || unicode.IsMark(c) || unicode.IsNumber(c)) && c%37 == 0:
			rtLetters = append(rtLetters, c)
		}
		if unicode.SimpleFold(c) != c {
			rtFold = append(rtFold, c)
		}
	}
	rtEntity = html5EntityNames()
}

// html5EntityNames returns goldmark's HTML5 entity names (from the generated
// table when GOLDMARK_DIR is set; a fixed sample otherwise).
func html5EntityNames() []string {
	if os.Getenv("GOLDMARK_DIR") == "" {
		return []string{"amp", "AMP", "copy", "nbsp", "ouml", "Ouml", "lt", "gt", "quot", "ngE", "nvlt", "fjlig",
			"ThickSpace", "NotNestedGreaterGreater", "CounterClockwiseContourIntegral", "bne", "acE", "dollar"}
	}
	d := genDecls(filepath.Join(goldmarkDir(), "util", "html5entities.gen.go"))
	length := int(intLit(d["_html5entitiesLength"]))
	names := stringLit(d["_html5entitiesName"])
	nameIndex := stringLit(d["_html5entitiesNameIndex"])
	var ret []string
	c := 0
	for i := 0; i < length; i++ {
		t := c + int(nameIndex[i])
		ret = append(ret, names[c:t])
		c = t
	}
	sort.Strings(ret)
	return ret
}

var rtInvalid = []string{
	"\xff", "\xfe", "\xc3", "\xe0\xb8", "\xed\xa0\x80", "\xed\xbf\xbf", "\xf4\x90\x80\x80", "\xc0\xaf", "\xe0\x80\xaf",
	"\x80", "\xbf", "\xf0\x9f", "\xf0\x9f\x8d", "\x00", "\ufffd", "\x7f", "\x01", "\x0b", "\x0c", "\u2028", "\u2029",
	"\u00a0", "\u3000", "\u200b", "\ufeff",
}

func randRune(r *rand.Rand) string {
	switch r.Intn(10) {
	case 0, 1, 2, 3:
		return string(rtPunct[r.Intn(len(rtPunct))])
	case 4, 5:
		return string(rtSpace[r.Intn(len(rtSpace))])
	case 6:
		return string(rtLetters[r.Intn(len(rtLetters))])
	case 7:
		return rtInvalid[r.Intn(len(rtInvalid))]
	case 8:
		return string(rune(0x21 + r.Intn(0x5e)))
	}
	return []string{"a", "1", " ", "\t", "\n"}[r.Intn(5)]
}

func randEntity(r *rand.Rand) string {
	switch r.Intn(6) {
	case 0, 1:
		e := "&" + rtEntity[r.Intn(len(rtEntity))]
		if r.Intn(5) != 0 {
			e += ";"
		}
		return e
	case 2:
		v := []int64{0, 1, 9, 0x7f, 0x80, 0x9f, 0xd7ff, 0xd800, 0xdbff, 0xdc00, 0xdfff, 0xe000, 0xfffd, 0xfffe, 0xffff,
			0x10000, 0x10ffff, 0x110000, 0x7fffffff, 9999999, 99999999, 1234567, 12345678}[r.Intn(23)]
		if r.Intn(2) == 0 {
			return fmt.Sprintf("&#%d;", v)
		}
		x := "x"
		if r.Intn(2) == 0 {
			x = "X"
		}
		h := fmt.Sprintf("%x", v)
		if r.Intn(2) == 0 {
			h = strings.ToUpper(h)
		}
		return "&#" + x + h + ";"
	case 3:
		d := 1 + r.Intn(9)
		var b strings.Builder
		b.WriteString("&#")
		if r.Intn(2) == 0 {
			b.WriteString("x")
			for i := 0; i < d; i++ {
				b.WriteByte("0123456789abcdefABCDEF"[r.Intn(22)])
			}
		} else {
			for i := 0; i < d; i++ {
				b.WriteByte("0123456789"[r.Intn(10)])
			}
		}
		if r.Intn(6) != 0 {
			b.WriteString(";")
		}
		return b.String()
	case 4:
		return "&" + rep("a", r.Intn(40)) + []string{";", "", "0;", "Z;"}[r.Intn(4)]
	}
	return []string{"&", "&;", "&#;", "&#x;", "&#X;", "& amp;", "&amp", "&AMP;", "&Amp;"}[r.Intn(9)]
}

var unicodeDelims = []string{"*", "**", "***", "_", "__", "___", "~", "~~", "'", "\"", "`", "[", "]", "](u)", "<", ">", "\\", "|", "--", "...", "{", "}"}

func fuzzUnicodeDoc(r *rand.Rand) string {
	initRuneTables()
	var b strings.Builder
	lines := 1 + r.Intn(4)
	for l := 0; l < lines; l++ {
		if r.Intn(4) == 0 {
			b.WriteString(fuzzBlockPrefixes[r.Intn(len(fuzzBlockPrefixes))])
		}
		k := 1 + r.Intn(12)
		for i := 0; i < k; i++ {
			switch r.Intn(6) {
			case 0, 1:
				b.WriteString(unicodeDelims[r.Intn(len(unicodeDelims))])
			case 2, 3:
				b.WriteString(randRune(r))
			case 4:
				b.WriteString(randEntity(r))
			default:
				// flanking probe: X D a D Y
				d := unicodeDelims[r.Intn(6)]
				b.WriteString(randRune(r) + d + []string{"a", randRune(r), " "}[r.Intn(3)] + d + randRune(r))
			}
		}
		b.WriteString("\n")
	}
	return lineEndings(r, b.String())
}

// ---------------------------------------------------------------------------
// labels: link reference label matching (full Unicode case folding,
// whitespace collapsing, 999-char limit) and footnote labels.

var foldSpecials = []string{
	"ß", "ẞ", "ss", "SS", "Ss", "ſs", "ſſ", "İ", "i̇", "ı", "I", "i", "K", "k", "K", "Å", "å", "Å", "ſ", "s", "S",
	"ς", "σ", "Σ", "ﬀ", "ff", "FF", "ﬁ", "fi", "ﬃ", "ŉ", "ʼn", "ǰ", "ΐ", "ᾈ", "ᾀ", "ἀι", "ᾳ", "ῼ", "ﬆ", "st",
	"µ", "μ", "Μ", "ǅ", "ǆ", "Ǆ", "ϐ", "β", "ẛ", "ṡ", "Ω", "ω", "Ω", "ⅰ", "Ⅰ", "ⓐ", "Ⓐ", "𐐀", "𐐨", "Ꭰ", "ꭰ",
	"ǈ", "ȿ", "Ȿ", "ɐ", "Ɐ", "ᲀ", "в", "ꙋ", "ᲈ", "Ᲊ", "ᲊ", "ʸ",
}

func foldVariant(r *rand.Rand, s string) string {
	var b strings.Builder
	for _, c := range s {
		switch r.Intn(5) {
		case 0:
			b.WriteRune(unicode.SimpleFold(c))
		case 1:
			b.WriteRune(unicode.ToUpper(c))
		case 2:
			b.WriteRune(unicode.ToLower(c))
		default:
			b.WriteRune(c)
		}
	}
	return b.String()
}

var labelSpaces = []string{" ", "  ", "\t", "\n", " \n ", "\u00a0", "\u3000", "\r\n"}

func randLabel(r *rand.Rand) string {
	initRuneTables()
	var b strings.Builder
	k := 1 + r.Intn(6)
	if r.Intn(20) == 0 {
		k = 900 + r.Intn(120)
	}
	for i := 0; i < k; i++ {
		switch r.Intn(6) {
		case 0, 1:
			b.WriteString(foldSpecials[r.Intn(len(foldSpecials))])
		case 2:
			b.WriteRune(rtFold[r.Intn(len(rtFold))])
		case 3:
			b.WriteString(labelSpaces[r.Intn(len(labelSpaces))])
		case 4:
			b.WriteString([]string{"a", "B", "\\]", "\\[", "*", "`", "!", "^", "\xff", "&amp;", "&#223;"}[r.Intn(11)])
		default:
			b.WriteString(randRune(r))
		}
	}
	return b.String()
}

func fuzzLabelDoc(r *rand.Rand) string {
	var b strings.Builder
	n := 1 + r.Intn(3)
	labels := make([]string, n)
	for i := range labels {
		labels[i] = randLabel(r)
		if r.Intn(4) == 0 {
			b.WriteString(fmt.Sprintf("[^%s]: note %d\n", labels[i], i))
		} else {
			b.WriteString(fmt.Sprintf("[%s]: /u%d%s\n", labels[i], i, []string{"", " 't'", " \"t\"", " (t)"}[r.Intn(4)]))
		}
	}
	b.WriteString("\n")
	k := 1 + r.Intn(5)
	for i := 0; i < k; i++ {
		l := labels[r.Intn(n)]
		if r.Intn(3) != 0 {
			l = foldVariant(r, l)
		}
		if r.Intn(4) == 0 {
			l = strings.ReplaceAll(l, " ", labelSpaces[r.Intn(len(labelSpaces))])
		}
		switch r.Intn(6) {
		case 0:
			fmt.Fprintf(&b, "[%s] ", l)
		case 1:
			fmt.Fprintf(&b, "[%s][] ", l)
		case 2:
			fmt.Fprintf(&b, "[x][%s] ", l)
		case 3:
			fmt.Fprintf(&b, "![%s] ", l)
		case 4:
			fmt.Fprintf(&b, "[^%s] ", l)
		default:
			fmt.Fprintf(&b, "[*%s*][%s] ", l, foldVariant(r, l))
		}
	}
	b.WriteString("\n")
	return b.String()
}

// ---------------------------------------------------------------------------
// tabs: tabs and spaces at every column inside container prefixes.

var tabsPrefix = []string{" ", " ", "  ", "   ", "\t", "\t", ">", "> ", "-", "- ", "*", "+", "1.", "2)", "10.", ":", ": ", "[^1]:", "```", "~~~", "    ", "|", "- [ ]", "#"}
var tabsContent = []string{"a", "b", "\t", " ", "  ", "`", "*", "|", "\\", "-", "#", "x"}

func fuzzTabsDoc(r *rand.Rand) string {
	var b strings.Builder
	lines := 1 + r.Intn(7)
	if r.Intn(3) == 0 {
		b.WriteString("Term\n")
	}
	for l := 0; l < lines; l++ {
		k := r.Intn(6)
		for i := 0; i < k; i++ {
			b.WriteString(tabsPrefix[r.Intn(len(tabsPrefix))])
			if r.Intn(3) == 0 {
				b.WriteString(rep(" ", r.Intn(4)) + "\t")
			}
		}
		k = r.Intn(5)
		for i := 0; i < k; i++ {
			b.WriteString(tabsContent[r.Intn(len(tabsContent))])
		}
		b.WriteString("\n")
		if r.Intn(6) == 0 {
			b.WriteString([]string{"\n", "\t\n", "  \n", " \t \n"}[r.Intn(4)])
		}
	}
	s := b.String()
	if r.Intn(6) == 0 {
		s = lineEndings(r, s)
	}
	return s
}

// ---------------------------------------------------------------------------
// hugomix: structured documents with Hugo's extension set nested in each
// other (footnotes in tables and lists, linkify in link text and autolinks,
// typographer in code and attributes, attribute lists, definition lists with
// lists, task lists, ...).

type hmix struct {
	r *rand.Rand
}

var hmixWords = []string{"a", "foo", "Bar", "ไทย", "日本語", "x_y", "don't", "Smiths'", "'90s", "5'6\"", "it's", "--", "---", "...",
	"<<", ">>", "\"q\"", "'s'", "&amp;", "&copy;", "&#39;", "\u00a0", "\\*", "\\|", "\\[", "a|b", "~", "_", "*"}

var hmixURLs = []string{"http://example.com", "https://a.b/c?d=e&f=g", "www.example.org", "https://x.y/(a)", "ftp://f.o/p",
	"a@b.co", "mailto:a@b.cd", "https://seeksnack.com/privacy/", "http://a.b:8080/p.", "www.a.b/c)", "https://a.b/c&amp;"}

func (g *hmix) pick(s []string) string { return s[g.r.Intn(len(s))] }

func (g *hmix) attrs() string {
	r := g.r
	if r.Intn(4) == 0 {
		return g.pick([]string{"{}", "{#}", "{.}", "{=}", "{#id", "{ #a #b }", "{.a .b .c}", "{class=x .y}", "{class=1}",
			"{key=\"unterminated}", "{a=\"e\\\"q\"}", "{a=[1,2,}", "{a=[1, \"x\", true, null, {b=2}]}", "{a=-1.5e3}",
			"{a=+}", "{a=1e999}", "{#a.b:c-d_e}", "{#ไทย}", "{.'q'}", "{data-x=\"--\"}", "{a='single'}", "{ }", "{\t#t\t}",
			"{#id .c key=val}", "{key=val,k2=v2}", "{.c,#i}", "{a=b c=d}", "{a b}", "{#x}{.y}", "{#x} {.y}", "\\{#x}"})
	}
	var b strings.Builder
	b.WriteString("{")
	k := 1 + r.Intn(4)
	for i := 0; i < k; i++ {
		if i > 0 {
			b.WriteString(g.pick([]string{" ", ",", ", ", "  "}))
		}
		switch r.Intn(6) {
		case 0:
			b.WriteString("#" + g.pick([]string{"id", "x-y", "a:b", "a.b", "h1", "ไทย", "é"}))
		case 1:
			b.WriteString("." + g.pick([]string{"c", "cls", "a-b", "x_y", "c1"}))
		case 2:
			b.WriteString(g.pick([]string{"key", "data-x", "title", "style", "class", "id", "lang", "_a", ":b"}) + "=" +
				g.pick([]string{"val", "\"v 2\"", "1", "-2.5", "true", "false", "null", "[1,2]", "\"'q' --\"", "x.y", "{a=1}"}))
		default:
			b.WriteString(g.pick([]string{"linenos=table", "hl_lines=[1,3]", "anchor=\"a\"", "k=v"}))
		}
	}
	b.WriteString("}")
	return b.String()
}

func (g *hmix) inline(k int) string {
	r := g.r
	var b strings.Builder
	for i := 0; i < k; i++ {
		if i > 0 && r.Intn(3) != 0 {
			b.WriteString(" ")
		}
		switch r.Intn(16) {
		case 0, 1, 2:
			b.WriteString(g.pick(hmixWords))
		case 3:
			d := g.pick([]string{"*", "**", "_", "__", "~~", "~"})
			b.WriteString(d + g.inline(1+r.Intn(2)) + d)
		case 4:
			b.WriteString("`" + g.pick([]string{"a|b", "'q'", "--", "\\|", "{#x}", "[^1]", "http://a.bc", "\"x\""}) + "`")
		case 5:
			b.WriteString("[" + g.inline(1+r.Intn(2)) + "](" + g.pick([]string{"/u", "http://a.b", "<a b>", "u \"t 'x'\"", "u 'it''s'"}) + ")")
		case 6:
			b.WriteString("[" + g.pick(hmixURLs) + "](" + g.pick(hmixURLs) + ")")
		case 7:
			b.WriteString(g.pick(hmixURLs))
		case 8:
			b.WriteString("<" + g.pick(hmixURLs) + ">")
		case 9:
			b.WriteString(fmt.Sprintf("[^%d]", 1+r.Intn(3)))
		case 10:
			b.WriteString("![" + g.inline(1) + "](i.png" + g.pick([]string{"", " \"t\""}) + ")")
		case 11:
			b.WriteString(g.pick([]string{"<span class=\"x\">", "</span>", "<i>", "</i>", "<br>", "<!-- c -->", "<b title='q'>"}))
		case 12:
			b.WriteString(g.attrs())
		case 13:
			b.WriteString(g.pick([]string{"[x]", "[ ]", "[ref]", "[Ref][]", "[^x]", "[a]"}))
		case 14:
			b.WriteString(g.pick([]string{"\"", "'", "\\", "&", "|", ":", "]", "["}))
		default:
			b.WriteString(g.pick([]string{"  \n", "\\\n", "\n"}))
		}
	}
	return b.String()
}

// indent prefixes every line but the first with pad (sometimes lazily not).
func (g *hmix) indent(s, first, pad string) string {
	lines := strings.SplitAfter(s, "\n")
	var b strings.Builder
	for i, l := range lines {
		if l == "" {
			continue
		}
		switch {
		case i == 0:
			b.WriteString(first)
		case l == "\n":
		case g.r.Intn(12) == 0:
		default:
			b.WriteString(pad)
		}
		b.WriteString(l)
	}
	return b.String()
}

func (g *hmix) block(depth int) string {
	r := g.r
	n := r.Intn(16)
	if depth <= 0 && n >= 5 && n <= 9 {
		n = 0
	}
	switch n {
	case 0, 1:
		s := g.inline(1+r.Intn(6)) + "\n"
		if r.Intn(4) == 0 {
			s += g.attrs() + "\n"
		}
		return s
	case 2:
		s := rep("#", 1+r.Intn(6)) + " " + g.inline(1+r.Intn(3))
		if r.Intn(3) == 0 {
			s += " " + rep("#", r.Intn(3))
		}
		if r.Intn(2) == 0 {
			s += " " + g.attrs()
		}
		return s + "\n"
	case 3:
		s := g.inline(1 + r.Intn(3))
		if r.Intn(2) == 0 {
			s += " " + g.attrs()
		}
		return s + "\n" + g.pick([]string{"===", "---", "=", "-"}) + "\n"
	case 4:
		cols := 1 + r.Intn(4)
		var b strings.Builder
		row := func() {
			if r.Intn(4) != 0 {
				b.WriteString("|")
			}
			for c := 0; c < cols+r.Intn(3)-1; c++ {
				b.WriteString(" " + strings.ReplaceAll(g.inline(1+r.Intn(3)), "\n", " ") + " |")
			}
			b.WriteString("\n")
		}
		row()
		b.WriteString("|")
		for c := 0; c < cols; c++ {
			b.WriteString(g.pick([]string{"---", ":--", "--:", ":-:", " - "}) + "|")
		}
		b.WriteString("\n")
		for k := r.Intn(4); k > 0; k-- {
			row()
		}
		if r.Intn(3) == 0 {
			b.WriteString(g.attrs() + "\n")
		}
		return b.String()
	case 5:
		marker := g.pick([]string{"- ", "* ", "+ ", "1. ", "2) ", "- [ ] ", "- [x] ", "-\t", "1.\t"})
		pad := rep(" ", len(strings.TrimRight(marker, "\t")))
		if strings.HasSuffix(marker, "\t") {
			pad = "    "
		}
		var b strings.Builder
		for k := 1 + r.Intn(3); k > 0; k-- {
			b.WriteString(g.indent(g.block(depth-1), marker, pad))
			if r.Intn(4) == 0 {
				b.WriteString(g.indent(g.block(depth-1), pad, pad))
			}
			if r.Intn(4) == 0 {
				b.WriteString("\n")
			}
		}
		return b.String()
	case 6:
		p := g.pick([]string{"> ", ">", ">\t"})
		return g.indent(g.block(depth-1), p, p)
	case 7:
		var b strings.Builder
		for k := 1 + r.Intn(2); k > 0; k-- {
			b.WriteString(strings.ReplaceAll(g.inline(1+r.Intn(3)), "\n", " ") + "\n")
		}
		if r.Intn(3) == 0 {
			b.WriteString("\n")
		}
		for k := 1 + r.Intn(2); k > 0; k-- {
			m := g.pick([]string{": ", ":   ", ":\t"})
			b.WriteString(g.indent(g.block(depth-1), m, "    "))
			if r.Intn(3) == 0 {
				b.WriteString("\n")
			}
		}
		return b.String()
	case 8:
		return g.indent(g.block(depth-1), fmt.Sprintf("[^%d]: ", 1+r.Intn(3)), "    ")
	case 9:
		return g.block(depth-1) + g.block(depth-1)
	case 10:
		fence := g.pick([]string{"```", "~~~", "````"})
		var b strings.Builder
		b.WriteString(fence + g.pick([]string{"", "go", "go " + g.attrs(), g.attrs(), "'q' --", "a`b"}) + "\n")
		for k := r.Intn(3); k > 0; k-- {
			b.WriteString(g.pick([]string{"'q' \"d\" -- ...", "\t| a | b |", "[^1]", "http://a.bc", "{#x}", "<b>"}) + "\n")
		}
		if r.Intn(6) != 0 {
			b.WriteString(fence + "\n")
		}
		return b.String()
	case 11:
		return "    " + g.pick([]string{"code 'q' --", "| a |", "[^1]: x", "http://a.b"}) + "\n"
	case 12:
		return g.pick([]string{"<div class=\"x\">\n'q' http://a.b\n</div>\n", "<!-- c -->\n", "<table><tr><td>[^1]</td></tr></table>\n", "<details>\n\n*x*\n\n</details>\n"})
	case 13:
		return g.pick([]string{"***\n", "---\n", "___\n", "- - -\n"})
	case 14:
		return fmt.Sprintf("[%s]: %s %s\n", g.pick([]string{"ref", "Ref", "a", "x"}), g.pick(hmixURLs), g.pick([]string{"", "'t'", "\"t\"", "(t)"}))
	default:
		return fmt.Sprintf("[^%d]: %s\n", 1+r.Intn(3), g.inline(1+r.Intn(3)))
	}
}

func fuzzHugoMixDoc(r *rand.Rand) string {
	g := &hmix{r}
	var b strings.Builder
	for k := 1 + r.Intn(5); k > 0; k-- {
		b.WriteString(g.block(1 + r.Intn(3)))
		if r.Intn(2) == 0 {
			b.WriteString("\n")
		}
	}
	s := b.String()
	if r.Intn(10) == 0 {
		s = lineEndings(r, s)
	}
	return s
}

// ---------------------------------------------------------------------------
// enum: every string of up to -len tokens over an alphabet, rendered with
// the configs in turn (goldmark enum -alpha NAME -len N -cfg a,b [-ast]).

var enumAlphabets = map[string][]string{
	"blocks": {" ", "\t", ">", "-", "1.", "a", "\n", "`", "*", "#", "="},
	"inline": {"*", "_", "a", " ", "[", "]", "(", ")", "!", "`", "\\", "<", ">"},
	"ext":    {"|", "-", ":", "~", "[^1]", "]", "'", "\"", "\n", "a", " ", "x", "[", "@b.c", "www.a.b", "."},
	"links":  {"[a]", ":", " ", "/u", "\n", "\"t\"", "<", ">", "[", "]", "(", ")", "'", "\\"},
	"html":   {"<", "a", ">", "/", "!", "-", "?", "\"", "=", " ", "\n", "div"},
	"attrs":  {"# a", "{", "}", "#", ".", "=", "b", "\"", " ", "[", "]", ",", "1", "-", "\n"},
	"bytes":  {"*", "_", "a", "\u00a0", "\u3000", "。", "\xff", "\xe0\xb8", "\x00", "€", "!", " ", "\r"},
	"lists":  {"- ", "1. ", "  ", "\t", "a", "\n", "> ", "[ ] ", ": ", "Term\n", "    ", "```"},
}

func enumMain(args []string) {
	fs := flag.NewFlagSet("enum", flag.ExitOnError)
	alpha := fs.String("alpha", "blocks", "alphabet name")
	maxLen := fs.Int("len", 5, "maximum number of tokens")
	cfgs := fs.String("cfg", "default,hugo", "configs (one per string, in turn)")
	withAST := fs.Bool("ast", false, "add AST dumps")
	_ = fs.Parse(args)
	a, ok := enumAlphabets[*alpha]
	if !ok {
		panic("unknown alphabet " + *alpha)
	}
	cs := strings.Split(*cfgs, ",")
	g := newGMF(os.Stdout)
	idx := make([]int, 0, *maxLen)
	count := 0
	var emit func()
	emit = func() {
		var b strings.Builder
		for _, i := range idx {
			b.WriteString(a[i])
		}
		md := []byte(b.String())
		cfg := cs[count%len(cs)]
		g.record(fmt.Sprintf("enum-%s/%d/%s", *alpha, count, cfg))
		g.field("cfg", []byte(cfg))
		g.field("md", md)
		g.field("html", convert(cfg, md))
		if *withAST {
			g.field("ast", parseDump(cfg, md))
		}
		count++
		if len(idx) == *maxLen {
			return
		}
		for i := range a {
			idx = append(idx, i)
			emit()
			idx = idx[:len(idx)-1]
		}
	}
	emit()
	g.flush()
	fmt.Fprintf(os.Stderr, "enum %s: %d strings\n", *alpha, count)
}

// timedConvert renders like convert and returns the time taken.
func timedConvert(cfg string, md []byte) ([]byte, time.Duration) {
	t := time.Now()
	out := convert(cfg, md)
	return out, time.Since(t)
}

// ---------------------------------------------------------------------------
// attrvec: parser.ParseAttributes called directly (Hugo's attribute blocks
// and code block attributes do), on a text.Reader and on a block reader over
// the input's lines: the result, ok, and the reader position afterwards.

func (g *hmix) attrJunk(depth int) string {
	r := g.r
	var b strings.Builder
	switch r.Intn(9) {
	case 0:
		if depth > 0 {
			b.WriteString("[")
			for k := r.Intn(4); k > 0; k-- {
				b.WriteString(g.attrJunk(depth - 1))
				b.WriteString(g.pick([]string{",", ", ", " ,", "", ",,", " "}))
			}
			b.WriteString(g.pick([]string{"]", "]", "", " ]", ",]"}))
		} else {
			b.WriteString("[]")
		}
	case 1:
		if depth > 0 {
			b.WriteString("{")
			for k := r.Intn(4); k > 0; k-- {
				b.WriteString(g.pick([]string{"a", "b", "class", "id", "_", ":x", "a.b-c", "1"}) + g.pick([]string{"=", " = ", "", "=\n"}))
				b.WriteString(g.attrJunk(depth - 1))
				b.WriteString(g.pick([]string{",", " ", "", "\n"}))
			}
			b.WriteString(g.pick([]string{"}", "}", ""}))
		} else {
			b.WriteString("{}")
		}
	case 2:
		b.WriteString("\"" + g.pick([]string{"", "a", "a b", "\\\"", "\\\\", "\\n\\t\\r\\b\\f\\/", "\\x", "\\", "ไทย", "\xff", "\n"}) + g.pick([]string{"\"", "\"", ""}))
	case 3:
		b.WriteString(g.pick([]string{"1", "-1", "+1", "0", "-0", "1.5", "1.", ".5", "1e3", "1E-3", "1e+", "1e999", "-1e999", "1e-999",
			"123456789012345678901234567890", "0.1e2", "1.2.3", "-", "+", "--1", "1_000", "0x10", "9007199254740993", "2.2250738585072011e-308"}))
	case 4:
		b.WriteString(g.pick([]string{"true", "false", "null", "True", "nul", "truex", "a", "Z9", "_x", ":y", "a.b", "a-b", "é", "1a"}))
	case 5:
		b.WriteString(g.pick([]string{"#id", "#", "#a:b.c-d_e", "#a!b", ".c", ".", ".a.b", "#ไทย", "#\xff", ".c d", "#i\n"}))
	default:
		b.WriteString(g.pick([]string{"a=1", "class=x", "class=1", "class=\"q r\"", "id=5", "k=v", "k = v", "k=", "=v", "k", "a=b c=d", " ", "\t", "\n", ",", "}", "{", "]", "\\"}))
	}
	return b.String()
}

// attrVal is a (mostly) well-formed attribute value.
func (g *hmix) attrVal(depth int) string {
	r := g.r
	if r.Intn(15) == 0 {
		return g.attrJunk(depth)
	}
	var b strings.Builder
	switch r.Intn(6) {
	case 0:
		if depth <= 0 {
			return "[]"
		}
		b.WriteString("[")
		for k := r.Intn(4); k > 0; k-- {
			b.WriteString(g.attrVal(depth - 1))
			if k > 1 {
				b.WriteString(g.pick([]string{",", ", ", " ,", " , "}))
			}
		}
		b.WriteString(g.pick([]string{"]", " ]", "\n]"}))
	case 1:
		if depth <= 0 {
			return "{}"
		}
		b.WriteString("{")
		for k := r.Intn(4); k > 0; k-- {
			if r.Intn(4) == 0 {
				b.WriteString(g.pick([]string{"#i", ".c", ".d", "#j-k"}))
			} else {
				b.WriteString(g.pick([]string{"a", "b", "class", "id", "_", ":x", "a.b-c"}) + g.pick([]string{"=", " = "}))
				b.WriteString(g.attrVal(depth - 1))
			}
			b.WriteString(g.pick([]string{",", " ", ", ", "\n"}))
		}
		b.WriteString("}")
	case 2:
		b.WriteString("\"" + g.pick([]string{"", "a", "a b", "\\\"", "\\\\", "\\n\\t\\r\\b\\f\\/", "\\x", "\u0e44\u0e17\u0e22", "\xff", "--", "'q'"}) + "\"")
	case 3:
		b.WriteString(g.pick([]string{"1", "-1", "+1", "0", "-0", "1.5", "1.", "1e3", "1E-3", "1e999", "-1e999", "1e-999", "00012",
			"123456789012345678901234567890", "0.1e2", "9007199254740993", "2.2250738585072011e-308", "4.9e-324", "1e-400",
			"1" + rep("0", 400), "0." + rep("0", 400) + "1", "1e" + rep("9", 30), "1e-" + rep("9", 30), "179769313486231580793728971405303415079934132710037826936173778980444968292764750946649017977587207096330286416692887910946555547851940402630657488671505820681908902000708383676273854845817711531764475730270069855571366959622842914819860834936475292719074168444365510704342711559699508093042880177904174497791.9999999999999999999999999999999999999999999999999999999999999999999999"}))
	case 4:
		b.WriteString(g.pick([]string{"true", "false", "null", "True", "nul", "truex", "a", "Z9", "_x", ":y", "a.b", "a-b", "x:y.z"}))
	default:
		b.WriteString(g.pick([]string{"v", "val", "x1", "\"s p\"", "2", "[1,2]", "{a=1}"}))
	}
	return b.String()
}

func attrVecInput(r *rand.Rand) string {
	g := &hmix{r}
	var b strings.Builder
	b.WriteString(g.pick([]string{"", "", " ", "  ", "\t", "\n", "x"}))
	if r.Intn(8) != 0 {
		b.WriteString("{")
	}
	for k := r.Intn(5); k > 0; k-- {
		switch r.Intn(10) {
		case 0:
			b.WriteString(g.attrJunk(1 + r.Intn(3)))
		case 1, 2:
			b.WriteString(g.pick([]string{"#id", "#", "#a:b.c-d_e", "#a!b", ".c", ".", ".a.b", "#\u0e44\u0e17\u0e22", "#\xff", ".c d", "#i\n"}))
		default:
			b.WriteString(g.pick([]string{"a", "b", "class", "id", "_", ":x", "a.b-c", "data-x", "Z9"}))
			b.WriteString(g.pick([]string{"=", "=", " = ", "=\n", "\t=\t"}))
			// a value: array, object, string, number or literal
			b.WriteString(g.attrVal(1 + r.Intn(3)))
		}
		b.WriteString(g.pick([]string{" ", ",", ", ", "", "\n", "  "}))
	}
	if r.Intn(12) == 0 {
		// deep values (parsed on an explicit stack by the Rust port)
		n := 1 + r.Intn(50000)
		b.WriteString(g.pick([]string{"a=", "class=", "a = ", ".c a="}))
		b.WriteString([]string{rep("[", n), rep("{a=", n), rep("[", n) + rep("]", n), rep("{a=[", n) + "1" + rep("]}", n),
			rep("[1,", n) + "2" + rep("]", n), rep("{a=1 b=", n) + "2" + rep("}", n), rep("[", n) + rep("]", n-1)}[r.Intn(7)])
		b.WriteString(g.pick([]string{"", " ", ","}))
	}
	b.WriteString(g.pick([]string{"}", "}", "}", "", "} x", "}\n", "}}"}))
	s := b.String()
	if r.Intn(5) == 0 && len(s) > 0 {
		s = s[:r.Intn(len(s))]
	}
	return s
}

func attrVecRecord(g *gmfWriter, name string, in []byte) {
	g.record(name)
	g.field("in", in)
	for _, block := range []bool{false, true} {
		var rd text.Reader
		if block {
			segs := text.NewSegments()
			for off := 0; off < len(in); {
				i := bytes.IndexByte(in[off:], '\n')
				end := len(in)
				if i >= 0 {
					end = off + i + 1
				}
				segs.Append(text.NewSegment(off, end))
				off = end
			}
			rd = text.NewBlockReader(in, segs)
		} else {
			rd = text.NewReader(in)
		}
		var out bytes.Buffer
		func() {
			defer func() {
				if rec := recover(); rec != nil {
					out.Reset()
					out.WriteString("PANIC")
				}
			}()
			attrs, ok := parser.ParseAttributes(rd)
			if ok {
				dvalue(&out, attrs)
			} else {
				fmt.Fprintf(&out, "false %v", attrs == nil)
			}
			line, pos := rd.Position()
			fmt.Fprintf(&out, " @%d:", line)
			dseg(&out, pos)
		}()
		if block {
			g.field("block", out.Bytes())
		} else {
			g.field("reader", out.Bytes())
		}
	}
}

// attrVecMain: goldmark attrvec -n N -seed S > out.gmf
func attrVecMain(args []string) {
	fs := flag.NewFlagSet("attrvec", flag.ExitOnError)
	n := fs.Int("n", 10000, "number of inputs")
	seed := fs.Int64("seed", 1, "random seed")
	_ = fs.Parse(args)
	r := rand.New(rand.NewSource(*seed))
	g := newGMF(os.Stdout)
	for i := 0; i < *n; i++ {
		attrVecRecord(g, fmt.Sprintf("attrvec/%d/%d", *seed, i), []byte(attrVecInput(r)))
	}
	g.flush()
}

// ---------------------------------------------------------------------------
// redteamfixtures: the checked-in red-team fixtures
// (crates/goldmark/tests/fixtures/redteam*.gmf.gz, tests/redteam.rs).

// redteamRegression is an input that once made the Rust port differ from Go.
type redteamRegression struct {
	name string
	md   string
	cfgs []string
	ast  bool // add the AST dump (not for inputs whose dump is huge)
}

func redteamRegressions() []redteamRegression {
	hugoCfgs := []string{"hugo", "hugo-autoid", "attr", "default"}
	// Each '{' of a heading's last line starts a ParseAttributes, so nested
	// objects are quadratic (in Go too); they are exercised deep through
	// the direct ParseAttributes vectors (redteam-attrs.gmf.gz).
	return []redteamRegression{
		// parser/attribute.go recursion (ParseAttributes → parseAttribute →
		// parseAttributeValue → parseAttributeArray): Rust overflowed its
		// stack and aborted.
		{"attr-deep-array-open", "# h {a=" + rep("[", 20000) + "}\n", hugoCfgs, false},
		{"attr-deep-array", "# h {a=" + rep("[", 20000) + rep("]", 20000) + "}\n", hugoCfgs, true},
		{"attr-deep-array-unclosed", "# h {a=" + rep("[", 20000) + rep("]", 20000) + "\n", hugoCfgs, false},
		{"attr-deep-array-values", "# h {a=" + rep("[1,\"x\",", 10000) + "true" + rep("]", 10000) + " .c #d}\n", []string{"hugo", "default"}, true},
		{"attr-deep-setext", "h {a=" + rep("[", 20000) + rep("]", 20000) + "}\n===\n", hugoCfgs, true},
		{"attr-deep-closing-sequence", "# h ## {a=" + rep("[", 20000) + rep("]", 20000) + "}\n", hugoCfgs, true},
		{"attr-deep-object", "# h {a=" + rep("{b=", 800) + "1" + rep("}", 800) + "}\n", hugoCfgs, true},
		// {id=<array>} with auto heading IDs: Go panics (interface
		// conversion); Rust formatted the value with the recursive Debug.
		{"attr-deep-id-panics", "# h {id=" + rep("[", 20000) + rep("]", 20000) + "}\n", []string{"attr", "hugo-autoid", "hugo"}, true},
		{"attr-deep-class", "# h {class=" + rep("[", 20000) + rep("]", 20000) + "}\n", hugoCfgs, false},
		{"attr-deep-in-footnote", "[^1]: # {a=" + rep("[", 20000) + rep("]", 20000) + "}\n\nx[^1]\n", []string{"hugo", "x-all"}, true},
	}
}

// redteamTexts: documents whose Node.Text recursion is deep; the "texts"
// field holds the Text of the document and of the first inline node.
var redteamTexts = []struct{ name, md string }{
	{"text-deep-blockquote", rep("> ", 20000) + "a\n"},
	{"text-deep-list", rep("- ", 10000) + "a\n"},
	{"text-deep-emphasis", rep("*", 60000) + "a" + rep("*", 60000) + "\n"},
	{"text-deep-image", rep("![", 20000) + "a" + rep("](b)", 20000) + "\n"},
	{"text-deep-mixed", rep("> - ", 5000) + "*a\nb* `c`\n" + rep("[", 1000) + "x" + rep("](u)", 1000) + "\n"},
}

// firstInline is the first inline node in document order.
func firstInline(n gast.Node) gast.Node {
	for n != nil {
		if n.Type() == gast.TypeInline {
			return n
		}
		n = n.FirstChild()
	}
	return nil
}

func redteamFixturesMain(args []string) {
	out := args[0]
	writeFile(filepath.Join(out, "redteam.gmf.gz"), func(g *gmfWriter) {
		for _, c := range redteamRegressions() {
			for _, cfg := range c.cfgs {
				md := []byte(c.md)
				g.record("regress/" + c.name + "/" + cfg)
				g.field("cfg", []byte(cfg))
				g.field("md", md)
				g.field("html", convert(cfg, md))
				// Without attribute parsing the dump is one Text per bracket.
				if c.ast && cfg != "default" {
					g.field("ast", parseDump(cfg, md))
				}
			}
		}
		for _, c := range redteamTexts {
			for _, cfg := range []string{"default", "hugo"} {
				md := []byte(c.md)
				g.record("regress/" + c.name + "/" + cfg)
				g.field("cfg", []byte(cfg))
				g.field("md", md)
				g.field("html", convert(cfg, md))
				doc := mdCache[cfg].Parser().Parse(text.NewReader(md))
				var texts bytes.Buffer
				//nolint:staticcheck // Node.Text is deprecated but part of the API.
				texts.Write(doc.Text(md))
				texts.WriteByte(0)
				if fi := firstInline(doc); fi != nil {
					//nolint:staticcheck // as above
					texts.Write(fi.Text(md))
				}
				g.field("texts", texts.Bytes())
			}
		}
	})
	writeFile(filepath.Join(out, "redteam-fuzz.gmf.gz"), func(g *gmfWriter) {
		fuzzWithAST = true
		hugo := []string{"hugo", "hugo-autoid", "x-all", "default"}
		writeFuzzMode(g, 800, 1, []string{"hugo", "hugo-autoid", "x-all", "hugo-xhtml", "default", "attr"}, "hugomix")
		writeFuzzMode(g, 800, 2, []string{"hugo", "default", "x-all", "hugo-autoid", "xu", "escspace", "ea-css3", "x-cjk"}, "unicode")
		writeFuzzMode(g, 500, 3, []string{"hugo", "default", "x-all", "hugo-autoid", "x-footnote"}, "labels")
		writeFuzzMode(g, 800, 4, []string{"hugo", "default", "x-all", "hugo-autoid", "xu", "x-deflist", "x-footnote", "x-table"}, "tabs")
		fuzzWithAST = false
		writeFuzzMode(g, 40, 5, hugo, "pathosmall")
	})
	writeFile(filepath.Join(out, "redteam-attrs.gmf.gz"), func(g *gmfWriter) {
		r := rand.New(rand.NewSource(6))
		for i := 0; i < 3000; i++ {
			attrVecRecord(g, fmt.Sprintf("attrvec/6/%d", i), []byte(attrVecInput(r)))
		}
	})
}

// ---------------------------------------------------------------------------
// systematic: every Unicode punctuation/symbol/space rune (and invalid
// UTF-8) around every delimiter kind, every case-folding table entry in
// reference labels, every HTML5 entity name in text, link destinations,
// titles and raw HTML (goldmark systematic -cfg a,b [-ast]).

// flankPatterns: %[1]s is the probe rune.
var flankPatterns = []string{
	"%[1]s*a*%[1]s", "*a*%[1]s", "%[1]s*a*", "a*%[1]s*a", "*%[1]s*", "%[1]s**a**%[1]s", "**%[1]s**a", "a**%[1]s**",
	"%[1]s_a_%[1]s", "a_%[1]s_a", "_%[1]s_", "%[1]s__a__%[1]s", "%[1]s~~a~~%[1]s", "~%[1]s~", "%[1]s~a~%[1]s",
	"%[1]s'a'%[1]s", "%[1]s\"a\"%[1]s", "a'%[1]s", "%[1]s'", "%[1]s***a***%[1]s", "*a%[1]s*", "_a%[1]s_",
	"%[1]s[a](b)%[1]s", "%[1]shttp://a.bc%[1]s", "www.a.bc%[1]s", "a%[1]s@b.cd", "%[1]s`a`%[1]s",
	"# %[1]s {#i%[1]s}", "%[1]s\\%[1]s", "[%[1]s]: /u\n\n[%[1]s]",
}

func systematicMain(args []string) {
	fs := flag.NewFlagSet("systematic", flag.ExitOnError)
	cfgs := fs.String("cfg", "default,hugo,x-all", "configs (every input is rendered with each)")
	withAST := fs.Bool("ast", false, "add AST dumps")
	_ = fs.Parse(args)
	initRuneTables()
	cs := strings.Split(*cfgs, ",")
	g := newGMF(os.Stdout)
	emit := func(name string, md string) {
		for _, cfg := range cs {
			g.record(name + "/" + cfg)
			g.field("cfg", []byte(cfg))
			g.field("md", []byte(md))
			g.field("html", convert(cfg, []byte(md)))
			if *withAST {
				g.field("ast", parseDump(cfg, []byte(md)))
			}
		}
	}
	// flanking: every P, S and space rune, a sample of letters, invalid bytes
	probes := []string{}
	for _, c := range rtPunct {
		probes = append(probes, string(c))
	}
	for _, c := range rtSpace {
		probes = append(probes, string(c))
	}
	for i, c := range rtLetters {
		if i%8 == 0 {
			probes = append(probes, string(c))
		}
	}
	probes = append(probes, rtInvalid...)
	for i, p := range probes {
		var b strings.Builder
		for _, pat := range flankPatterns {
			fmt.Fprintf(&b, pat, p)
			b.WriteString("\n\n")
		}
		emit(fmt.Sprintf("sys-flank/%d", i), b.String())
	}
	// case folding: every rune with a simple-fold orbit, every full-fold special
	for _, c := range rtFold {
		var b strings.Builder
		fmt.Fprintf(&b, "[x%sy]: /u\n\n", string(c))
		for f := unicode.SimpleFold(c); ; f = unicode.SimpleFold(f) {
			fmt.Fprintf(&b, "[x%sy] [X%sY][] ", string(f), string(f))
			if f == c {
				break
			}
		}
		fmt.Fprintf(&b, "[x%sy] [x%sy]\n", string(unicode.ToUpper(c)), string(unicode.ToLower(c)))
		emit(fmt.Sprintf("sys-fold/%x", c), b.String())
	}
	for i, s := range foldSpecials {
		var b strings.Builder
		fmt.Fprintf(&b, "[%s]: /u\n\n", s)
		for _, t := range foldSpecials {
			fmt.Fprintf(&b, "[%s] ", t)
		}
		b.WriteString("\n")
		emit(fmt.Sprintf("sys-foldspecial/%d", i), b.String())
	}
	// entities: every HTML5 name, with and without ';', in every context
	for i, name := range rtEntity {
		md := fmt.Sprintf("&%[1]s; &%[1]s &%[1]s;x\n\n[a](/&%[1]s; \"&%[1]s;\") [b](<&%[1]s>)\n\n<a href=\"&%[1]s;\">&%[1]s;</a>\n\n`&%[1]s;` # &%[1]s; {#&%[1]s;}\n\n[&%[1]s;]: /&%[1]s;\n\n[&%[1]s;]\n", name)
		emit(fmt.Sprintf("sys-entity/%d", i), md)
	}
	// numeric references around the bounds
	for _, v := range []int64{0, 1, 8, 9, 10, 13, 31, 32, 127, 128, 159, 160, 0xd7ff, 0xd800, 0xdbff, 0xdc00, 0xdfff, 0xe000,
		0xfdd0, 0xfffd, 0xfffe, 0xffff, 0x10000, 0x10ffff, 0x110000, 0x1fffff, 0x7fffffff, 0x80000000, 0xffffffff, 9999999, 10000000} {
		md := fmt.Sprintf("&#%d; &#x%x; &#X%X; &#%07d; &#x%08x; [a](/&#%d;) [b](/u \"&#x%x;\")\n", v, v, v, v, v, v, v)
		emit(fmt.Sprintf("sys-numeric/%x", v), md)
	}
	g.flush()
}
