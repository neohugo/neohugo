package main

import (
	"bytes"
	"fmt"
	"io/fs"
	"math/rand"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strings"

	"github.com/neohugo/neohugo/parser/pageparser"
	"github.com/yuin/goldmark"
	gast "github.com/yuin/goldmark/ast"
	"github.com/yuin/goldmark/extension"
	east "github.com/yuin/goldmark/extension/ast"
	"github.com/yuin/goldmark/parser"
	"github.com/yuin/goldmark/renderer/html"
	"github.com/yuin/goldmark/text"
	"github.com/yuin/goldmark/util"
)

// Extension configurations (crates/goldmark/tests/common/mod.rs builds the
// same instances by name).
var extConfigNames = []string{
	"hugo", "hugo-autoid", "hugo-xhtml",
	"x-table", "x-strike", "x-linkify", "x-tasklist", "x-deflist", "x-footnote", "x-typo",
	"x-gfm", "x-cjk", "x-cjk-esc", "x-cjk-linkify", "x-cjk-css3", "x-all",
	"x-table-attr", "x-table-style", "x-table-none", "x-table-styletr",
	"x-footnote-opts", "x-footnote-fn",
	"x-linkify-proto", "x-linkify-www", "x-linkify-email", "x-typo-custom",
}

// hugoTypographer is neohugo's toTypographicPunctuationMap with the default
// markup.goldmark.extensions.typographer config (goldmark_config.Default).
func hugoTypographer() goldmark.Extender {
	return extension.NewTypographer(extension.WithTypographicSubstitutions(map[extension.TypographicPunctuation][]byte{
		extension.LeftSingleQuote:  []byte("&lsquo;"),
		extension.RightSingleQuote: []byte("&rsquo;"),
		extension.LeftDoubleQuote:  []byte("&ldquo;"),
		extension.RightDoubleQuote: []byte("&rdquo;"),
		extension.EnDash:           []byte("&ndash;"),
		extension.EmDash:           []byte("&mdash;"),
		extension.Ellipsis:         []byte("&hellip;"),
		extension.LeftAngleQuote:   []byte("&laquo;"),
		extension.RightAngleQuote:  []byte("&raquo;"),
		extension.Apostrophe:       []byte("&rsquo;"),
	}))
}

// hugoExtensions is the goldmark extension list neohugo's newMarkdown
// (markup/goldmark/convert.go) installs for seeksnack's config, minus
// Hugo's own extensions (hooks, attributes/auto IDs, ...), in the same order.
func hugoExtensions() []goldmark.Extender {
	return []goldmark.Extender{
		extension.Table,
		extension.Strikethrough,
		extension.Linkify,
		extension.TaskList,
		hugoTypographer(),
		extension.DefinitionList,
		extension.Footnote,
	}
}

type tableStyleTransformer struct{}

// As extension/table_test.go's tableStyleTransformer, but tolerant of
// documents that do not start with a table.
func (a *tableStyleTransformer) Transform(node *gast.Document, reader text.Reader, pc parser.Context) {
	fc := node.FirstChild()
	if fc == nil || fc.Kind() != east.KindTable {
		return
	}
	cell := fc.FirstChild().FirstChild()
	if cell == nil {
		return
	}
	cell.SetAttributeString("style", []byte("font-size:1em"))
}

type footnoteID struct{}

func (a *footnoteID) Transform(node *gast.Document, reader text.Reader, pc parser.Context) {
	node.Meta()["footnote-prefix"] = "article12-"
}

func newExtMarkdown(name string) goldmark.Markdown {
	unsafe := goldmark.WithRendererOptions(html.WithUnsafe())
	switch name {
	case "hugo":
		return goldmark.New(goldmark.WithExtensions(hugoExtensions()...),
			goldmark.WithParserOptions(parser.WithAttribute()), unsafe)
	case "hugo-autoid":
		return goldmark.New(goldmark.WithExtensions(hugoExtensions()...),
			goldmark.WithParserOptions(parser.WithAttribute(), parser.WithAutoHeadingID()), unsafe)
	case "hugo-xhtml":
		return goldmark.New(goldmark.WithExtensions(hugoExtensions()...),
			goldmark.WithParserOptions(parser.WithAttribute()),
			goldmark.WithRendererOptions(html.WithUnsafe(), html.WithXHTML(), html.WithHardWraps()))
	case "x-table":
		// extension/table_test.go:TestTable
		return goldmark.New(goldmark.WithRendererOptions(html.WithUnsafe(), html.WithXHTML()),
			goldmark.WithExtensions(extension.Table))
	case "x-strike":
		return goldmark.New(unsafe, goldmark.WithExtensions(extension.Strikethrough))
	case "x-linkify":
		return goldmark.New(unsafe, goldmark.WithExtensions(extension.Linkify))
	case "x-tasklist":
		return goldmark.New(unsafe, goldmark.WithExtensions(extension.TaskList))
	case "x-deflist":
		return goldmark.New(unsafe, goldmark.WithExtensions(extension.DefinitionList))
	case "x-footnote":
		return goldmark.New(unsafe, goldmark.WithExtensions(extension.Footnote))
	case "x-typo":
		return goldmark.New(unsafe, goldmark.WithExtensions(extension.Typographer))
	case "x-gfm":
		return goldmark.New(unsafe, goldmark.WithExtensions(extension.GFM))
	case "x-cjk":
		return goldmark.New(goldmark.WithRendererOptions(html.WithXHTML(), html.WithUnsafe()),
			goldmark.WithExtensions(extension.CJK))
	case "x-cjk-esc":
		// extension/cjk_test.go:TestEscapedSpace (3)
		return goldmark.New(goldmark.WithRendererOptions(html.WithXHTML(), html.WithUnsafe()),
			goldmark.WithExtensions(extension.NewCJK(extension.WithEscapedSpace())))
	case "x-cjk-linkify":
		return goldmark.New(goldmark.WithRendererOptions(html.WithXHTML(), html.WithUnsafe()),
			goldmark.WithExtensions(extension.NewCJK(extension.WithEscapedSpace()), extension.Linkify))
	case "x-cjk-css3":
		return goldmark.New(goldmark.WithRendererOptions(html.WithXHTML(), html.WithUnsafe()),
			goldmark.WithExtensions(extension.NewCJK(extension.WithEastAsianLineBreaks(extension.EastAsianLineBreaksCSS3Draft))))
	case "x-all":
		return goldmark.New(
			goldmark.WithExtensions(extension.GFM, extension.DefinitionList, extension.Footnote,
				extension.Typographer, extension.CJK),
			goldmark.WithParserOptions(parser.WithAttribute(), parser.WithAutoHeadingID()),
			goldmark.WithRendererOptions(html.WithUnsafe(), html.WithXHTML()))
	case "x-table-attr":
		return goldmark.New(unsafe, goldmark.WithExtensions(
			extension.NewTable(extension.WithTableCellAlignMethod(extension.TableCellAlignAttribute))))
	case "x-table-style":
		return goldmark.New(goldmark.WithRendererOptions(html.WithXHTML(), html.WithUnsafe()),
			goldmark.WithExtensions(
				extension.NewTable(extension.WithTableCellAlignMethod(extension.TableCellAlignStyle))))
	case "x-table-none":
		return goldmark.New(goldmark.WithRendererOptions(html.WithXHTML(), html.WithUnsafe()),
			goldmark.WithExtensions(
				extension.NewTable(extension.WithTableCellAlignMethod(extension.TableCellAlignNone))))
	case "x-table-styletr":
		return goldmark.New(
			goldmark.WithParserOptions(parser.WithASTTransformers(util.Prioritized(&tableStyleTransformer{}, 0))),
			unsafe,
			goldmark.WithExtensions(
				extension.NewTable(extension.WithTableCellAlignMethod(extension.TableCellAlignStyle))))
	case "x-footnote-opts":
		return goldmark.New(unsafe, goldmark.WithExtensions(extension.NewFootnote(
			extension.WithFootnoteIDPrefix("article12-"),
			extension.WithFootnoteLinkClass("link-class"),
			extension.WithFootnoteBacklinkClass("backlink-class"),
			extension.WithFootnoteLinkTitle("link-title-%%-^^"),
			extension.WithFootnoteBacklinkTitle("backlink-title"),
			extension.WithFootnoteBacklinkHTML("^"),
			extension.WithFootnoteHTMLOptions(html.WithXHTML()),
		)))
	case "x-footnote-fn":
		return goldmark.New(
			goldmark.WithParserOptions(parser.WithASTTransformers(util.Prioritized(&footnoteID{}, 100))),
			unsafe,
			goldmark.WithExtensions(extension.NewFootnote(
				extension.WithFootnoteIDPrefixFunction(func(n gast.Node) []byte {
					v, ok := n.OwnerDocument().Meta()["footnote-prefix"]
					if ok {
						return util.StringToReadOnlyBytes(v.(string))
					}
					return nil
				}),
				extension.WithFootnoteLinkClass([]byte("link-class")),
				extension.WithFootnoteBacklinkClass([]byte("backlink-class")),
				extension.WithFootnoteLinkTitle([]byte("link-title-%%-^^")),
				extension.WithFootnoteBacklinkTitle([]byte("backlink-title")),
				extension.WithFootnoteBacklinkHTML([]byte("^")),
			)))
	case "x-linkify-proto":
		return goldmark.New(goldmark.WithRendererOptions(html.WithXHTML(), html.WithUnsafe()),
			goldmark.WithExtensions(extension.NewLinkify(
				extension.WithLinkifyAllowedProtocols([]string{"ssh:"}),
				extension.WithLinkifyURLRegexp(regexp.MustCompile(`\w+://[^\s]+`)),
			)))
	case "x-linkify-www":
		return goldmark.New(goldmark.WithRendererOptions(html.WithXHTML(), html.WithUnsafe()),
			goldmark.WithExtensions(extension.NewLinkify(
				extension.WithLinkifyWWWRegexp(regexp.MustCompile(`www\.example\.com`)),
			)))
	case "x-linkify-email":
		return goldmark.New(goldmark.WithRendererOptions(html.WithXHTML(), html.WithUnsafe()),
			goldmark.WithExtensions(extension.NewLinkify(
				extension.WithLinkifyEmailRegexp(regexp.MustCompile(`user@example\.com`)),
			)))
	case "x-typo-custom":
		// Empty substitutions are non-nil []byte("") (still substituted,
		// with nothing); the others differ from the defaults.
		return goldmark.New(unsafe, goldmark.WithExtensions(extension.NewTypographer(
			extension.WithTypographicSubstitutions(map[extension.TypographicPunctuation]string{
				extension.LeftSingleQuote: "",
				extension.EnDash:          "&#8211;",
				extension.Ellipsis:        "",
				extension.Apostrophe:      "'",
				extension.LeftAngleQuote:  "<<",
			}))))
	}
	panic("unknown config " + name)
}

// extEdgeCases are hand-written inputs for the extensions.
var extEdgeCases = []string{
	// tables
	"| a | b |\n|---|---|\n| c | d |\n",
	"| a | b |\n| --- | --- |\n| c |\n| d | e | f |\n",
	"a | b\n-- | --\nc | d\n",
	"| a |\n|---|\n",
	"| a | b |\n|:--|--:|\n|:-:|---|\n",
	"| abc | defghi |\n:-: | -----------:\nbar | baz\n",
	"| a | b |\n|---|\n| c | d |\n",
	"|a|\n|-|\n|b|\n\npara\n",
	"para\ntext\n| a | b |\n|---|---|\n| c | d |\n",
	"para\n| a | b |\n|---|---|\n",
	"| a `b|c` d |\n|---|\n| `e \\| f` |\n",
	"| `\\|` | `a\\|b\\|c` |\n|---|---|\n| `x \\| y` | z\\|w |\n",
	"| `code \\| pipe` | b |\n|---|---|\n| `x` | `y \\| z \\| w` |\n",
	"| a \\| b | c |\n|---|---|\n",
	"| *a* | **b** |\n|---|---|\n| [l](u) | <b>h</b> |\n",
	"- | a | b |\n  |---|---|\n  | c | d |\n",
	"> | a | b |\n> |---|---|\n> | c | d |\n",
	"* 0\n-|\n\t0",
	"   | a |\n   |---|\n",
	"    | a |\n    |---|\n",
	"| a |\n    |---|\n",
	"| a |\n|---|\n| b |\n\n| c |\n|---|\n",
	"|\n|\n",
	"||\n||\n",
	"| a | b |\n| - | - |\n| c | d |\nlazy\n",
	"| a | b |\n|---|---:\n| c | d |\n",
	"| a | b |\n|---|--- |\n| c | d |\n",
	"| a |\n|:---:|\n|\n",
	"Header\n---|---\n",
	"a|b\n-|-\n\\|c|d\n",
	"| h1 | h2 |\n| -- | -- |\n| c1 | c2 |\n\na\n\n\n| h3 | h4 |\n| -- | -- |\n| c3 | c4 |",
	"| a |\n| -: |\n| b |\r\n| c |\r\n",
	"| a | b |\r\n|---|---|\r\n| c | d |\r\n",
	"|ไทย|日本|\n|---|---|\n|รส|語|\n",
	"| a |\n|---|\n| b | c | d |\n",
	"x\n| a |\n|---|\ny\n| b |\n|---|\n",
	"# h\n| a |\n|---|\n",
	"| a |\n|---|\n# h\n",
	"| a |\n|---|\n- item\n",
	"[ref]: /url\n| a |\n|---|\n| [ref] |\n",
	"| a |\n|---|\n[ref]: /url\n",
	"Term\n: | a |\n  |---|\n",
	// strikethrough
	"~~del~~ ~single~ ~~~three~~~\n",
	"~~a ~~b~~ c~~\n",
	"~a~~b~\n",
	"~~ a ~~\n",
	"a~~b~~c ~~d\n",
	"**~~a~~** ~~*b*~~\n",
	"~~a\nb~~\n",
	"\\~~a~~\n",
	"~~a~\n",
	// linkify
	"http://example.com www.example.com a@b.com\n",
	"Visit https://example.com/path?a=b&c=d. Now!\n",
	"(https://en.wikipedia.org/wiki/Mala_(seasoning))\n",
	"https://en.wikipedia.org/wiki/Mala_(seasoning)\n",
	"http://a.b/c)) (http://a.b/(c)) http://a.b/c(\n",
	"http://a.b/c&amp; http://a.b/c&amp;d; http://a.b/&x1;\n",
	"www.example.com, www.example.com: www.example.com?! www.example.com*\n",
	"*www.example.com* _http://a.bc_ ~ftp://f.oo~\n",
	"http://localhost:8080/x http://a.b:80 http://a.b:x\n",
	"https://a.b.c.d/e.f.g. https://a/b http://.com http://a.Com\n",
	"http://a.b/c#frag?q=1'\"x\n",
	"http://a.b/c$d http://a.b/c`d` www.a.b/$x\n",
	"HTTP://EXAMPLE.COM Http://a.b\n",
	"xhttp://a.b ahttp://a.b 1www.a.b\n",
	"foo@bar.baz foo.bar@baz.qux foo@bar foo@bar.baz. foo@bar.baz-x foo@bar.baz_ -a@b.cd\n",
	"a.b-c_d+e@f-g.h.ij mailto:a@b.cd <a@b.cd>\n",
	"@a.b a@ a@.b a@b..c a@-b.c\n",
	"[http://a.b](http://c.d) [www.a.b] [a@b.cd]\n",
	"`http://a.b` <http://a.b> <b>http://a.b</b>\n",
	"http://a.b\u00a0c www.ไทย.com http://a.bไทย\n",
	"https://seeksnack.com/privacy/ and\nhttps://seeksnack.com/terms/\n",
	"www." + strings.Repeat("a", 300) + ".com\n",
	"http://" + strings.Repeat("a.", 140) + "com\n",
	"a\\ http://a.b\n",
	"\thttp://tab.com\n",
	"http://a.b/c;d;\n",
	"http://a.b/?a=1&b=2;\n",
	// task lists
	"- [ ] task\n- [x] done\n- [X] Done\n",
	"- [ ]\n- [x]\n",
	"1. [x] a\n2. [ ] b\n",
	"- a [x] b\n- [x]b\n- [ ]  spaced\n",
	"- > [x] quoted\n- text\n  [x] second line\n",
	"- [x] a\n  - [ ] nested\n",
	"[x] not in list\n",
	"- [\t] tab\n- [y] no\n",
	"- [x] [link](u)\n- [ ] ~~d~~\n",
	"-\n  [x] after blank\n",
	// definition lists
	"Term\n: Definition\n",
	"Term 1\nTerm 2\n: Def a\n: Def b\n",
	"Term\n\n: Loose def\n\n: Second\n",
	"Term\n:   Def\n    continued\n\n    para 2\n",
	"Term\n: Def\nlazy\n",
	"Term\n:\n",
	": no term\n",
	"Term\n:    code?\n",
	"Term\n:         indented code\n",
	"Term\n: a\n\nTerm2\n: b\n",
	"Term\n: > quote\n: - list\n",
	"- Term\n  : def\n",
	"> Term\n> : def\n",
	"Term\n : one space\n",
	"Term\n\t: tab\n",
	"Term\n:\tTabbed\n",
	"| a |\n|---|\n: def\n",
	"# Heading\n: def\n",
	"Term *em*\n: Def **strong**\n",
	"c1\n:   c2\n    c3\n\na\n\nc4\n:   c5\n    c6",
	"Apple\n:   Pomaceous fruit of plants of the genus Malus in \n    the family Rosaceae.\n\nOrange\n:   The fruit of an evergreen tree of the genus Citrus.\n",
	"Term\n: Def\n\n    code after\n",
	"a\n: b\n: c\n\n: d\n",
	// footnotes
	"Text[^1].\n\n[^1]: Note.\n",
	"A[^1] B[^1] C[^2] D[^x]\n\n[^1]: One\n[^2]: Two\n[^3]: Unused\n",
	"[^1]: Def before ref.\n\nRef[^1]\n",
	"Ref[^a b]\n\n[^a b]: spaced label\n",
	"Img![^1] and ![^1] and !x^1]\n\n[^1]: n\n",
	"[^1]: Para one.\n\n    Para two.\n\n        code\n\nRef[^1]\n",
	"Ref[^1]\n\n[^1]:\n",
	"Ref[^1]\n\n[^1]: a\n[^1]: b\n",
	"Ref[^1]\n\n> [^1]: in quote\n",
	"- Ref[^1]\n\n  [^1]: in list\n",
	"[^1]: outer\n    [^2]: inner\n\nA[^1] B[^2]\n",
	"[^]: empty\n\nA[^]\n",
	"[^ ]: blank\n\nA[^ ]\n",
	"A[^1][^2][^1]\n\n[^2]: two\n[^1]: one\n",
	"[link[^1]](u)\n\n[^1]: n\n",
	"*a[^1]*\n\n[^1]: *em* note\n",
	"A[^1]\n\n[^1]: note\nlazy continuation\n",
	"A[^1]\n\n[^1]: note\n\nnot continuation\n",
	"A [^1]\n[^1]: interrupts paragraph\n",
	"A[^long-label_1]\n\n[^long-label_1]: x\n",
	"A[^1]\n\n  [^1]: indented def\n",
	"A[^1]\n\n\t[^1]: tab def\n",
	"A[^1]\n\n[^1]:no space\n",
	"A[^\\]]\n\n[^\\]]: esc\n",
	"[^1]: a\n\n# Heading[^1]\n",
	// paddings (tabs) around footnotes, definitions and tables
	"> \t[^1]: a\n\nA[^1]\n",
	"-\t[^1]: x\n\nA[^1]\n",
	"1.\t[^1]:\ty\n\t  more\n\nA[^1]\n",
	"  -\t[^1]: z\n\nA[^1]\n",
	">\t[^1]:\t\tz\n\nA[^1]\n",
	"[^1]:\t\tcode?\n\nA[^1]\n",
	"[^1]: a\n\tb\n\t\tc\n\nA[^1]\n",
	"Term\n:\t\tdouble tab\n",
	"Term\n: \tmix\n\t  cont\n",
	"-\tTerm\n\t: def\n",
	"> Term\n>\t: def\n",
	"-\t| a |\n\t|---|\n\t| b |\n",
	">\t| a | b |\n>\t|:-|-:|\n",
	"1.\t- [x]\ttask\n",
	"-\t[ ] a\n",
	// typographer
	"This should 'be' replaced \"too\"\n",
	"**--** *---* a...<< b>> c.. d-- e----\n",
	"Some say '90s, '90s. others 90's, I can't '90\n",
	"'twas 'em 'net 'less 'x\n",
	"Alice's I'm Don't You'd I've I'll You're 'sa 'mo\n",
	"\"Monitor 21\"\" and \"Monitor\"\"\n",
	"*\"quoted\"* **'single'** ***\"both\"***\n",
	"\"a 'b' c\" 'd \"e\" f'\n",
	"'unclosed\n\n'next' para\n",
	"\"unclosed\n\n\"next\" para\n",
	"The Smiths' dog. doin' it. '5' \"5\"\n",
	"\"I'm not doin' that,\" Bill said.\n",
	"[\"link\"](u) (\"paren\") ['single']\n",
	"รส'ไทย' \"日本\" 'ü' 'İ'\n",
	"'a''b' \"a\"\"b\" ''' \"\"\"\n",
	"x'y x' 'y 1'2 '1 2'\n",
	"'s 'm 't 'd 've 'll 're\n",
	"a'\nb\"\n'c\n\"d\n",
	"5'6\" 5' 6\"\n",
	"\\'escaped\\' \\\"too\\\"\n",
	"`'code'` <b title='x'>'y'</b>\n",
	"- 'a'\n- \"b\n- c\"\n",
	"> \"quote\n> continues\"\n",
	"...\n---\n--\n",
	"a <<b>> c << d >>\n",
	"'?' '!' \"?\" \"!\" '.' \".\"\n",
	"he said \"'hi'\".\n",
	// combinations
	"| 'a' | \"b\" |\n|---|---|\n| c--d | e...f |\n",
	"Term 'x'\n: \"def\" http://a.bc ~~s~~\n",
	"- [x] \"task\" http://a.bc[^1]\n\n[^1]: 'n'\n",
	"# Head 'q' {#id}\n## http://a.bc\n",
	"~~http://a.bc~~ *www.a.bc* _a@b.cd_\n",
	"| [^1] | http://a.bc |\n|---|---|\n\n[^1]: n\n",
}

var extFuzzBlockPrefixes = []string{
	"| a | b |", "|---|---|", "| :-- | --: |", ":-:|", "---|---", "a | b", "-|-", "|", "| ",
	"- [ ] ", "- [x] ", "* [X]", "1. [ ] ",
	": ", ":   ", ":\t", " : ", "Term",
	"[^1]: ", "[^a]: ", "[^1]:", "    ", "\t",
}

var extFuzzContainers = []string{"> ", ">", ">\t", "- ", "-\t", "1. ", "1.\t", "  ", "\t", " \t", ": ", ":\t", "[^1]: ", "[^1]:\t"}

var extFuzzInline = []string{
	"~~", "~", "~~a~~", "~a~", "|", " | ", "\\|", "`a|b`", "`\\|`", "`x \\| y`",
	"[^1]", "[^a]", "![^1]", "[^", "^", ":", ": ",
	"'", "\"", "--", "---", "...", "<<", ">>", "'90s", "'twas", "don't", "I've", "Smiths'",
	"http://example.com", "https://a.b/c?d=e&f;", "http://x.y/(a)", "www.example.com.", "ftp://f.o/",
	"http://a.b:8080/p", "https://x.y/a&amp;", "a@b.co", "a.b@c.de.", "x_y@z.io", "a@b.c-",
	"[ ]", "[x]", "[X] ", "*http://a.b*", "(www.a.b)", "&amp;", "&copy;",
}

func fuzzExtDoc(r *rand.Rand) string {
	var b strings.Builder
	n := 1 + r.Intn(10)
	for i := 0; i < n; i++ {
		if r.Intn(2) == 0 {
			// container prefixes (with tabs: paddings) before the extension syntax
			for k := r.Intn(4) - 1; k > 0; k-- {
				b.WriteString(extFuzzContainers[r.Intn(len(extFuzzContainers))])
			}
			b.WriteString(extFuzzBlockPrefixes[r.Intn(len(extFuzzBlockPrefixes))])
			if r.Intn(5) == 0 {
				b.WriteString(fuzzBlockPrefixes[r.Intn(len(fuzzBlockPrefixes))])
			}
		} else if r.Intn(3) != 0 {
			b.WriteString(fuzzBlockPrefixes[r.Intn(len(fuzzBlockPrefixes))])
		}
		k := r.Intn(8)
		for j := 0; j < k; j++ {
			if r.Intn(2) == 0 {
				b.WriteString(extFuzzInline[r.Intn(len(extFuzzInline))])
			} else {
				b.WriteString(fuzzInline[r.Intn(len(fuzzInline))])
			}
		}
		switch r.Intn(12) {
		case 0:
			b.WriteString("  ")
		case 1:
			b.WriteString("\\")
		case 2:
			b.WriteString(" |")
		case 3:
			b.WriteString("\r")
		}
		if i != n-1 || r.Intn(2) == 0 {
			b.WriteString("\n")
		}
		if r.Intn(5) == 0 {
			b.WriteString("\n")
		}
	}
	return b.String()
}

// fuzzExtBytesDoc is the byte-level fuzzer with extension syntax.
func fuzzExtBytesDoc(r *rand.Rand) string {
	alphabet := append([]string{"~", "~~", "|", ":", "[^", "^", "'", "\"", "--", ".", "www.", "http://", "a.b", "@", "[x]", "- [ ] ", "\n: "}, fuzzBytesAlphabet...)
	var b strings.Builder
	n := r.Intn(40)
	for i := 0; i < n; i++ {
		b.WriteString(alphabet[r.Intn(len(alphabet))])
	}
	return b.String()
}

// extFixturesMain writes the extension fixtures:
//
//	goldmark extfixtures <out dir> <seeksnack content dir>
func extFixturesMain(args []string) {
	out := args[0]
	gm := goldmarkDir()

	// goldmark's extension/_test/*.txt, with the configurations of the
	// corresponding _test.go files (plus "hugo").
	files := []struct{ file, cfg string }{
		{"table.txt", "x-table"}, {"strikethrough.txt", "x-strike"}, {"linkify.txt", "x-linkify"},
		{"tasklist.txt", "x-tasklist"}, {"definition_list.txt", "x-deflist"},
		{"footnote.txt", "x-footnote"}, {"typographer.txt", "x-typo"},
	}
	writeFile(filepath.Join(out, "ext.gmf"), func(g *gmfWriter) {
		for _, f := range files {
			cases := parseCaseFile(filepath.Join(gm, "extension", "_test", f.file))
			name := strings.TrimSuffix(f.file, ".txt")
			for _, c := range cases {
				for _, cfg := range []string{f.cfg, "hugo", "x-all"} {
					g.record(fmt.Sprintf("ext-%s/%s/%d", name, cfg, c.No))
					g.field("cfg", []byte(cfg))
					g.field("md", []byte(c.source()))
					if cfg == f.cfg {
						g.field("spec", []byte(c.expected()))
					}
					g.field("html", convert(cfg, []byte(c.source())))
				}
			}
		}
		// The DoTestCase cases of the _test.go files.
		type goCase struct{ cfg, md, expected string }
		tableMD := "\n| abc | defghi |\n:-: | -----------:\nbar | baz\n"
		fnMD := "That's some text with a footnote.[^1]\n\nSame footnote.[^1]\n\nAnother one.[^2]\n\n[^1]: And that's the footnote.\n[^2]: Another footnote.\n"
		goCases := []goCase{
			{"x-table", tableMD, ""},
			{"x-table-attr", tableMD, ""},
			{"x-table-style", tableMD, ""},
			{"x-table-none", tableMD, ""},
			{"x-table-styletr", tableMD, ""},
			{"x-table", "* 0\n-|\n\t0", ""},
			{"x-footnote-opts", fnMD, ""},
			{"x-footnote-fn", fnMD, ""},
			{"x-linkify-proto", "hoge ssh://user@hoge.com. http://example.com/", "<p>hoge <a href=\"ssh://user@hoge.com\">ssh://user@hoge.com</a>. http://example.com/</p>"},
			{"x-linkify-www", "www.google.com www.example.com", "<p>www.google.com <a href=\"http://www.example.com\">www.example.com</a></p>"},
			{"x-linkify-email", "hoge@example.com user@example.com", "<p>hoge@example.com <a href=\"mailto:user@example.com\">user@example.com</a></p>"},
			{"x-cjk-esc", "太郎は\\ **「こんにちわ」**\\ と言った\nんです", "<p>太郎は<strong>「こんにちわ」</strong>と言った\nんです</p>"},
			{"x-cjk", "太郎は\\ **「こんにちわ」**\\ と言った\nんです", ""},
			{"x-cjk-linkify", "太郎は\\ **「こんにちわ」**\\ と言った\nんです", "<p>太郎は<strong>「こんにちわ」</strong>と言った\nんです</p>"},
		}
		for i, c := range goCases {
			g.record(fmt.Sprintf("ext-gotest/%s/%d", c.cfg, i))
			g.field("cfg", []byte(c.cfg))
			g.field("md", []byte(c.md))
			if c.expected != "" {
				g.field("spec", []byte(c.expected))
			}
			g.field("html", convert(c.cfg, []byte(c.md)))
		}
	})

	// Hand-written extension edge cases plus the core edge cases, every
	// extension configuration.
	writeFile(filepath.Join(out, "ext-edge.gmf.gz"), func(g *gmfWriter) {
		all := append(append([]string{}, extEdgeCases...), edgeCases...)
		for i, md := range all {
			for _, cfg := range extConfigNames {
				g.record(fmt.Sprintf("ext-edge/%s/%d", cfg, i))
				g.field("cfg", []byte(cfg))
				g.field("md", []byte(md))
				g.field("html", convert(cfg, []byte(md)))
			}
		}
	})

	// Fixed-seed fuzz corpora.
	writeFile(filepath.Join(out, "ext-fuzz.gmf.gz"), func(g *gmfWriter) {
		writeFuzzMode(g, 5000, 20260927, extConfigNames, "ext")
		writeFuzzMode(g, 3000, 7, extConfigNames, "extbytes")
		writeFuzzMode(g, 2000, 11, extConfigNames, "tokens")
	})

	if len(args) > 1 {
		writeFile(filepath.Join(out, "corpus-ext.gmf.gz"), func(g *gmfWriter) {
			writeExtCorpus(g, args[1], []string{"hugo", "hugo-autoid"}, true)
		})
	}
}

// writeExtCorpus renders every .md file below a content directory (front
// matter stripped with neohugo's pageparser) with the given configs, and
// optionally the whole files with the first config.
func writeExtCorpus(g *gmfWriter, root string, cfgs []string, full bool) {
	var files []string
	err := filepath.WalkDir(root, func(path string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if !d.IsDir() && strings.HasSuffix(path, ".md") {
			files = append(files, path)
		}
		return nil
	})
	if err != nil {
		panic(err)
	}
	sort.Strings(files)
	for _, f := range files {
		b, err := os.ReadFile(f)
		if err != nil {
			panic(err)
		}
		cfm, err := pageparser.ParseFrontMatterAndContent(bytes.NewReader(b))
		if err != nil {
			panic(err)
		}
		rel, _ := filepath.Rel(root, f)
		for _, cfg := range cfgs {
			g.record("corpus/" + cfg + "/" + rel)
			g.field("cfg", []byte(cfg))
			g.field("md", cfm.Content)
			g.field("html", convert(cfg, cfm.Content))
		}
		if full {
			g.record("corpus-full/" + cfgs[0] + "/" + rel)
			g.field("cfg", []byte(cfgs[0]))
			g.field("md", b)
			g.field("html", convert(cfgs[0], b))
		}
	}
}

// extCorpusMain: goldmark extcorpus <content dir> <out.gmf[.gz]> [cfg,cfg,...]
func extCorpusMain(args []string) {
	cfgs := []string{"hugo", "hugo-autoid"}
	if len(args) > 2 {
		cfgs = strings.Split(args[2], ",")
	}
	writeFile(args[1], func(g *gmfWriter) {
		writeExtCorpus(g, args[0], cfgs, true)
	})
}

// Copies of the extension package's unexported regular expressions
// (extension/linkify.go, extension/tasklist.go, extension/table.go). The
// Rust port replaces them with hand-written matchers; these vectors pin the
// equivalence.
var (
	extWWWURLRegxp      = regexp.MustCompile(`^www\.[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-z]+(?:[/#?][-a-zA-Z0-9@:%_\+.~#!?&/=\(\);,'">\^{}\[\]` + "`" + `]*)?`)                          //nolint:golint,lll
	extURLRegexp        = regexp.MustCompile(`^(?:http|https|ftp)://[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-z]+(?::\d+)?(?:[/#?][-a-zA-Z0-9@:%_+.~#$!?&/=\(\);,'">\^{}\[\]` + "`" + `]*)?`) //nolint:golint,lll
	extTaskListRegexp   = regexp.MustCompile(`^\[([\sxX])\]\s*`)
	extTableDelimLeft   = regexp.MustCompile(`^\s*\:\-+\s*$`)
	extTableDelimRight  = regexp.MustCompile(`^\s*\-+\:\s*$`)
	extTableDelimCenter = regexp.MustCompile(`^\s*\:\-+\:\s*$`)
	extTableDelimNone   = regexp.MustCompile(`^\s*\-+\s*$`)
)

var extRegexAlphabet = []string{
	"http://", "https://", "ftp://", "www.", "http", "https:/", "a", "Z", "0", "9", "com", "b.c", ".x", ".",
	"..", "-", "_", "@", ":", ":80", ":8a", "%", "+", "~", "#", "=", "/", "?", "!", "&", ";", ",", "'", "\"",
	">", "<", "^", "{", "}", "[", "]", "`", "$", "(", ")", " ", "\n", "\t", "\r", "\x0c", "\x0b", "ไ", "\xff",
	"\\", "*", "[x]", "[ ]", "[X]", "|", ":-", "-:", "---", "é", "A.B",
}

func indexPair(m []int) []byte {
	if m == nil {
		return []byte("-")
	}
	return []byte(fmt.Sprintf("%d %d", m[0], m[1]))
}

func boolField(b bool) []byte {
	if b {
		return []byte("1")
	}
	return []byte("0")
}

// writeExtRegexVectors: goldmark extregex <out.gmf[.gz]> [n]
func extRegexMain(args []string) {
	n := 20000
	if len(args) > 1 {
		fmt.Sscan(args[1], &n)
	}
	r := rand.New(rand.NewSource(42))
	var inputs []string
	inputs = append(inputs,
		"http://a.b", "http://a.bc/d", "https://x.y:80/z", "ftp://f.o.o", "www.a.bc", "www.a.bC", "www..bc",
		"www."+strings.Repeat("a", 255)+".com", "www."+strings.Repeat("a", 256)+".com",
		"www."+strings.Repeat("a", 257)+".com", "http://"+strings.Repeat("a", 256)+".b",
		"http://"+strings.Repeat("a.", 200)+"b", "http://a.b:", "http://a.b:1x", "http://a.b/(c)",
		"[x] a", "[ ]\n", "[\t]", "[\x0b]", "[x]  \t\n", " :--", ":-:", "-:", "--", ":", "", " \n",
	)
	for i := 0; i < n; i++ {
		var b strings.Builder
		k := 1 + r.Intn(24)
		if r.Intn(3) == 0 {
			b.WriteString([]string{"http://", "https://", "ftp://", "www.", "[", " "}[r.Intn(6)])
		}
		for j := 0; j < k; j++ {
			if r.Intn(8) == 0 {
				b.WriteString(strings.Repeat("a", r.Intn(300)))
			}
			b.WriteString(extRegexAlphabet[r.Intn(len(extRegexAlphabet))])
		}
		inputs = append(inputs, b.String())
	}
	writeFile(args[0], func(g *gmfWriter) {
		for i, in := range inputs {
			b := []byte(in)
			g.record(fmt.Sprintf("extregex/%d", i))
			g.field("in", b)
			g.field("url", indexPair(extURLRegexp.FindSubmatchIndex(b)))
			g.field("www", indexPair(extWWWURLRegxp.FindSubmatchIndex(b)))
			tm := extTaskListRegexp.FindSubmatchIndex(b)
			if tm == nil {
				g.field("task", []byte("-"))
			} else {
				g.field("task", []byte(fmt.Sprintf("%d %d", tm[1], tm[2])))
			}
			g.field("left", boolField(extTableDelimLeft.Match(b)))
			g.field("right", boolField(extTableDelimRight.Match(b)))
			g.field("center", boolField(extTableDelimCenter.Match(b)))
			g.field("none", boolField(extTableDelimNone.Match(b)))
		}
	})
}
