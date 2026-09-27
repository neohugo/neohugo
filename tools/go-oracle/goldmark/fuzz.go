package main

import (
	"flag"
	"fmt"
	"math/rand"
	"os"
	"strings"
)

// Block-level line prefixes.
var fuzzBlockPrefixes = []string{
	"", "", "", "", "", "", "", "",
	"# ", "## ", "###### ", "####### ", "#", "#\t", " # ", "   ## ",
	"> ", ">", "> > ", ">\t", " > ", ">>",
	"- ", "* ", "+ ", "-", "1. ", "2) ", "10. ", "0. ", "123456789. ", "1234567890. ",
	"  - ", "    - ", "\t- ", "-\t", "- - ", "1. - ", "- 1. ", "-   ", "-     ",
	"    ", "\t", "        ", "  ", "   ",
	"```", "~~~", "````", "``` go", "~~~ js ", "```a`b", "   ```",
	"***", "---", "___", "* * *", "- - -", "===", "==", "--", " ---",
	"<div>", "</div>", "<div class=\"x\">", "<table>", "<pre>", "</pre>", "<script>", "</script>",
	"<style>", "<textarea>", "<!-- ", "-->", "<?php ", "?>", "<!DOCTYPE html>", "<![CDATA[", "]]>",
	"<a href=\"x\">", "<x-y z='1'>", "</a>", "<custom>", "<DIV>", "<p>", "<h1>", "<ſcript>",
	"[foo]: /url", "[foo]: /url \"title\"", "[Foo]:\n/url\n'title'", "[bar]: <a b>", "[baz]:", "[ba\\]z]: /x",
	"|", "| a | b |", "{#id .cls}",
}

// Inline tokens.
var fuzzInline = []string{
	"a", "b", "foo", "bar", "Baz", "text", "word", " ", " ", " ", "  ", "\t",
	"*", "**", "***", "_", "__", "___", "*a*", "**b**", "_c_", "__d__", "a*b", "a_b",
	"`", "``", "```", "`code`", "`` a ` b ``", "` `",
	"[", "]", "[a]", "[foo]", "[Foo]", "[bar]", "[]", "[a](b)", "[a](<b c>)", "[a](b \"t\")", "[a](b 't')",
	"[a](b (t))", "[a]( b )", "[a][foo]", "[a][]", "[foo][]", "![img](src)", "![a *b*](c \"d\")", "!", "![",
	"(", ")", "<", ">", "<http://a.b/c>", "<mailto:x@y.z>", "<a@b.co>", "<a b>", "<a href=\"x\">",
	"</a>", "<b>", "<br/>", "<!-- c -->", "<!-->", "<!--->", "<?x?>", "<!X y>", "<![CDATA[x]]>",
	"<a\nhref='x'>", "<span\n>",
	"&amp;", "&copy;", "&nosuch;", "&#123;", "&#x1F600;", "&#0;", "&#X41;", "&#99999999;", "&#065;", "&",
	"&ouml;", "&#xD800;", "&AMP;",
	"\\", "\\*", "\\_", "\\[", "\\\\", "\\`", "\\<", "\\&", "\\ ", "\\a",
	"http://x.com", "www.x.com", "a@b.com",
	" ", "ไทย", "รสชาติ", "日本語", "ü", "İ", "ǅ", "ſ", "K", "�", "🍫",
	"\"", "'", "--", "...", "{", "}", "{#x}", "{.c}", "{a=b}", "#", "##", "=", "-", "+", "|", ":",
	"\x00", "\xff", "\xc3", "\xe0\xb8", "\r",
}

func fuzzLine(r *rand.Rand) string {
	var b strings.Builder
	if r.Intn(3) != 0 {
		b.WriteString(fuzzBlockPrefixes[r.Intn(len(fuzzBlockPrefixes))])
		if r.Intn(6) == 0 {
			b.WriteString(fuzzBlockPrefixes[r.Intn(len(fuzzBlockPrefixes))])
		}
	}
	n := r.Intn(8)
	for i := 0; i < n; i++ {
		b.WriteString(fuzzInline[r.Intn(len(fuzzInline))])
	}
	switch r.Intn(12) {
	case 0:
		b.WriteString("  ")
	case 1:
		b.WriteString("\\")
	case 2:
		b.WriteString(" ")
	case 3:
		b.WriteString("\r")
	}
	return b.String()
}

func fuzzDoc(r *rand.Rand) string {
	var b strings.Builder
	n := 1 + r.Intn(10)
	for i := 0; i < n; i++ {
		b.WriteString(fuzzLine(r))
		if i != n-1 || r.Intn(2) == 0 {
			b.WriteString("\n")
		}
		if r.Intn(5) == 0 {
			b.WriteString("\n")
		}
	}
	return b.String()
}

// fuzzBytesAlphabet drives the byte-level fuzzer: short random strings over
// markdown-significant characters hit block/inline boundary cases the
// token grammar above does not.
var fuzzBytesAlphabet = []string{
	" ", " ", " ", "  ", "\t", "\n", "\n", "\n", ">", "-", "*", "_", "`", "[", "]", "(", ")", "<", "!", "#",
	"1", "2.", "9)", ".", "a", "b", "\\", "&", ";", "=", "~", "|", ":", "\"", "'", "\r", "\x00", "\u00a0",
	"x", "y", "/", "?", "{", "}", "+", "0", "ไ", "\xff", "@", "%", "http:", "<a ", "<!--", "-->",
}

func fuzzBytesDoc(r *rand.Rand) string {
	var b strings.Builder
	n := r.Intn(40)
	for i := 0; i < n; i++ {
		b.WriteString(fuzzBytesAlphabet[r.Intn(len(fuzzBytesAlphabet))])
	}
	return b.String()
}

// fuzzDeepDoc builds deeply nested containers (blockquotes, lists, list
// items with tabs), exercising the opened-block slice growth and lazy
// continuation lines.
func fuzzDeepDoc(r *rand.Rand) string {
	var b strings.Builder
	containers := []string{"> ", ">", "- ", "* ", "1. ", "  ", "    ", "\t", " > ", "-\t", "10) ", "  - "}
	lines := 1 + r.Intn(8)
	for l := 0; l < lines; l++ {
		depth := r.Intn(40)
		if r.Intn(4) == 0 {
			depth = r.Intn(3)
		}
		for d := 0; d < depth; d++ {
			b.WriteString(containers[r.Intn(len(containers))])
		}
		b.WriteString(fuzzLine(r))
		b.WriteString("\n")
	}
	return b.String()
}

var fuzzPluginTokens = []string{"{{x}}", "{{", "}}", "{{a b}}", "%%", "%% ", "  %%", "~", "~~", "~~~", "~~a~~", "\n{{y}}\n", "<b>", "<!-- c -->"}

// fuzzPluginDoc sprinkles the test extension's syntax (tools/go-oracle/goldmark
// plugin.go) into token documents.
func fuzzPluginDoc(r *rand.Rand) string {
	doc := fuzzDoc(r)
	var b strings.Builder
	for _, line := range strings.SplitAfter(doc, "\n") {
		if r.Intn(3) == 0 {
			b.WriteString(fuzzPluginTokens[r.Intn(len(fuzzPluginTokens))])
		}
		b.WriteString(line)
		if r.Intn(3) == 0 {
			b.WriteString(fuzzPluginTokens[r.Intn(len(fuzzPluginTokens))])
		}
	}
	return b.String()
}

func writeFuzz(g *gmfWriter, n int, seed int64, cfgs []string) {
	writeFuzzMode(g, n, seed, cfgs, "tokens")
}

func writeFuzzMode(g *gmfWriter, n int, seed int64, cfgs []string, mode string) {
	r := rand.New(rand.NewSource(seed))
	for i := 0; i < n; i++ {
		var md string
		switch mode {
		case "bytes":
			md = fuzzBytesDoc(r)
		case "deep":
			md = fuzzDeepDoc(r)
		case "plugin":
			md = fuzzPluginDoc(r)
		case "ext":
			md = fuzzExtDoc(r)
		case "extbytes":
			md = fuzzExtBytesDoc(r)
		default:
			md = fuzzDoc(r)
		}
		cfg := cfgs[i%len(cfgs)]
		g.record(fmt.Sprintf("fuzz-%s/%d/%d/%s", mode, seed, i, cfg))
		g.field("cfg", []byte(cfg))
		g.field("md", []byte(md))
		g.field("html", convert(cfg, []byte(md)))
	}
}

func fuzzMain(args []string) {
	fs := flag.NewFlagSet("fuzz", flag.ExitOnError)
	n := fs.Int("n", 10000, "number of documents")
	seed := fs.Int64("seed", 1, "random seed")
	cfgs := fs.String("cfg", strings.Join(configNames, ","), "configs")
	mode := fs.String("mode", "tokens", "tokens|bytes|deep|plugin|ext|extbytes")
	_ = fs.Parse(args)
	g := newGMF(os.Stdout)
	writeFuzzMode(g, *n, *seed, strings.Split(*cfgs, ","), *mode)
	g.flush()
}
