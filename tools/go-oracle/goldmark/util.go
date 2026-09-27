package main

import (
	"bytes"
	"fmt"
	"math/rand"
	"strconv"
	"strings"
	"unicode/utf8"

	"github.com/yuin/goldmark/ast"
	"github.com/yuin/goldmark/parser"
	"github.com/yuin/goldmark/renderer/html"
	"github.com/yuin/goldmark/util"
)

var utilSeeds = []string{
	"", " ", "\t", "\n", "a", "abc", " a b ", "a  b\t\tc\n", "  \t\n\x0b\x0c\r x \r\n",
	"&amp;", "&copy;", "&nosuch;", "&#123;", "&#x1F600;", "&#0;", "&#X41;", "&#99999999;", "&#065;", "&#089;",
	"&#x;", "&#;", "&#12", "&#xFFFFFFFFF;", "&#x110000;", "&#xD800;", "&#0123;", "&#00;", "&#0x10;", "&",
	"&amp", "&AMP;", "&ouml;&Ouml;", "&NotNestedGreaterGreater;", "&aelig;&AElig;", "a&b;c", "&&amp;",
	"\\", "\\*", "\\a", "\\\\*", "a\\!b\\\"c\\#", "\\ ", "x\\", "\\&amp;",
	"http://a.b/c d", "http://x.y/%41%zz%4", "%", "%4", "%41", "%%41", "a b<c>\"d\"'e'",
	"ü", "日本語", "ไทย", "\u00a0", "\xff", "\xc3", "\xc3a", "\xe0\xb8", "\xf0\x9f\x8d\xab", "\xf0\x9f", "\x00",
	"javascript:alert(1)", "JaVaScRiPt:x", "vbscript:x", "file:///etc", "data:x", "data:image/png;base64,x",
	"data:image/gif;x", "data:image/jpeg;", "data:image/webp;", "data:image/svg+xml;", "data:image/bmp;", "data:image/",
	"a@b.c", "a.b-c+d@e-f.g.h", "@a", "a@", "a@-b", "a@b-", "a@b..c", "a@" + strings.Repeat("x", 70) + ".com",
	"a@" + strings.Repeat("y", 62) + "-z.c", "a!#$%&'*+/=?^_`{|}~-@x.y", "a\"b@c.d",
	"http:", "http://", "a:b", "ab:", "irc://x", "made-up-scheme+foo://x", strings.Repeat("a", 32) + ":x", strings.Repeat("a", 33) + ":x",
	"a<b:c", "localhost:5001/foo", "a:b c", "HTTP://X", "1a:b", "a:\x00",
	"[a]", "[a", "a]", "[[a]]", "[a[b]c]", "[`]`]", "[\\]]", "`a]` ]", "``a`]``]", "[a](b)",
	"Foo  BAR", "ẞ", "ß", "ﬀ", "İ", "Σσς", "ǅ", "  Foo\tbar  ", "ᾈ", "K",
	"# heading", "Heading Two!", "--x__y", "日本 語", "   ", "a-b_c d",
}

var utilAlphabet = []string{
	"a", "B", "0", "9", " ", "\t", "\n", "\r", "\x0b", "\x0c", "&", "#", "x", "X", ";", "\\", "*", "[", "]",
	"`", "%", "4", "1", "F", "<", ">", "\"", "'", ":", "@", ".", "-", "_", "+", "/", "ü", "ไ", "\u00a0",
	"\xff", "\xc3", "\x00", "amp", "copy", "ouml", "(", ")", "~", "!",
}

func utilInputs() []string {
	ret := append([]string{}, utilSeeds...)
	r := rand.New(rand.NewSource(7))
	for i := 0; i < 1500; i++ {
		var b strings.Builder
		n := r.Intn(12)
		for j := 0; j < n; j++ {
			b.WriteString(utilAlphabet[r.Intn(len(utilAlphabet))])
		}
		ret = append(ret, b.String())
	}
	return ret
}

func itoa(i int) []byte { return []byte(strconv.Itoa(i)) }

func boolb(b bool) []byte {
	if b {
		return []byte("true")
	}
	return []byte("false")
}

type bufW struct{ bytes.Buffer }

func (b *bufW) Available() int { return 1 << 30 }
func (b *bufW) Buffered() int  { return b.Len() }
func (b *bufW) Flush() error   { return nil }

func writerOut(f func(w util.BufWriter)) []byte {
	var b bufW
	f(&b)
	return b.Bytes()
}

func writeUtilVectors(g *gmfWriter) {
	escWriter := html.NewWriter(html.WithEscapedSpace())
	for i, s := range utilInputs() {
		v := []byte(s)
		cp := func() []byte { return append([]byte{}, v...) }
		g.record(fmt.Sprintf("util/%d", i))
		g.field("in", v)
		g.field("URLEscape/true", util.URLEscape(cp(), true))
		g.field("URLEscape/false", util.URLEscape(cp(), false))
		g.field("ResolveNumericReferences", util.ResolveNumericReferences(cp()))
		g.field("ResolveEntityNames", util.ResolveEntityNames(cp()))
		g.field("UnescapePunctuations", util.UnescapePunctuations(cp()))
		g.field("EscapeHTML", util.EscapeHTML(cp()))
		g.field("ToLinkReference", []byte(util.ToLinkReference(cp())))
		g.field("DoFullUnicodeCaseFolding", util.DoFullUnicodeCaseFolding(cp()))
		g.field("ReplaceSpaces", util.ReplaceSpaces(cp(), '+'))
		g.field("FindEmailIndex", itoa(util.FindEmailIndex(cp())))
		g.field("FindURLIndex", itoa(util.FindURLIndex(cp())))
		g.field("TrimLeftSpaceLength", itoa(util.TrimLeftSpaceLength(v)))
		g.field("TrimRightSpaceLength", itoa(util.TrimRightSpaceLength(v)))
		g.field("TrimLeftSpace", util.TrimLeftSpace(v))
		g.field("TrimRightSpace", util.TrimRightSpace(v))
		g.field("IsBlank", boolb(util.IsBlank(v)))
		g.field("VisualizeSpaces", util.VisualizeSpaces(cp()))
		g.field("IsDangerousURL", boolb(html.IsDangerousURL(v)))
		for _, cs := range []bool{false, true} {
			for _, nest := range []bool{false, true} {
				g.field(fmt.Sprintf("FindClosure/%v/%v", cs, nest), itoa(util.FindClosure(v, '[', ']', cs, nest)))
			}
		}
		g.field("IndentWidth/0", []byte(fmt.Sprint(util.IndentWidth(v, 0))))
		g.field("IndentWidth/3", []byte(fmt.Sprint(util.IndentWidth(v, 3))))
		g.field("FirstNonSpacePosition", itoa(util.FirstNonSpacePosition(v)))
		for _, w := range []int{0, 1, 2, 4, 5} {
			for _, pos := range []int{0, 1, 3} {
				p, pad := util.IndentPosition(v, pos, w)
				g.field(fmt.Sprintf("IndentPosition/%d/%d", pos, w), []byte(fmt.Sprint(p, pad)))
				p, pad = util.IndentPositionPadding(v, pos, 2, w)
				g.field(fmt.Sprintf("IndentPositionPadding/%d/%d", pos, w), []byte(fmt.Sprint(p, pad)))
				p, pad = util.DedentPosition(v, pos, w)
				g.field(fmt.Sprintf("DedentPosition/%d/%d", pos, w), []byte(fmt.Sprint(p, pad)))
				p, pad = util.DedentPositionPadding(v, pos, 1, w)
				g.field(fmt.Sprintf("DedentPositionPadding/%d/%d", pos, w), []byte(fmt.Sprint(p, pad)))
			}
		}
		if len(v) > 0 {
			var rs []byte
			for p := 0; p < len(v); p++ {
				rs = append(rs, []byte(fmt.Sprintf("%d,", util.ToRune(v, p)))...)
			}
			g.field("ToRune", rs)
		}
		g.field("Write", writerOut(func(w util.BufWriter) { html.DefaultWriter.Write(w, v) }))
		g.field("RawWrite", writerOut(func(w util.BufWriter) { html.DefaultWriter.RawWrite(w, v) }))
		g.field("SecureWrite", writerOut(func(w util.BufWriter) { html.DefaultWriter.SecureWrite(w, v) }))
		g.field("WriteEscapedSpace", writerOut(func(w util.BufWriter) { escWriter.Write(w, v) }))
		var pr []byte
		dec := v
		for len(dec) > 0 {
			r, size := utf8.DecodeRune(dec)
			dec = dec[size:]
			pr = append(pr, fmt.Sprintf("%v%v,", b2i(util.IsPunctRune(r)), b2i(util.IsSpaceRune(r)))...)
		}
		g.field("IsPunctRune/IsSpaceRune", pr)
	}

	// IDs: a sequence of Generate calls against one context.
	pc := parser.NewContext()
	var ids []byte
	for _, s := range utilSeeds {
		ids = append(ids, pc.IDs().Generate([]byte(s), ast.KindHeading)...)
		ids = append(ids, '|')
		ids = append(ids, pc.IDs().Generate([]byte(s), ast.KindParagraph)...)
		ids = append(ids, '\n')
	}
	g.record("ids")
	g.field("out", ids)

	// Attribute filters.
	filters := []struct {
		name string
		f    util.BytesFilter
	}{
		{"Global", html.GlobalAttributeFilter}, {"Heading", html.HeadingAttributeFilter},
		{"Blockquote", html.BlockquoteAttributeFilter}, {"List", html.ListAttributeFilter},
		{"ListItem", html.ListItemAttributeFilter}, {"Paragraph", html.ParagraphAttributeFilter},
		{"Thematic", html.ThematicAttributeFilter}, {"Link", html.LinkAttributeFilter},
		{"Code", html.CodeAttributeFilter}, {"Emphasis", html.EmphasisAttributeFilter},
		{"Image", html.ImageAttributeFilter},
	}
	names := []string{"accesskey", "autocapitalize", "autofocus", "class", "contenteditable", "dir", "draggable",
		"enterkeyhint", "hidden", "id", "inert", "inputmode", "is", "itemid", "itemprop", "itemref", "itemscope",
		"itemtype", "lang", "part", "role", "slot", "spellcheck", "style", "tabindex", "title", "translate",
		"cite", "start", "reversed", "type", "value", "align", "color", "noshade", "size", "width", "download",
		"hreflang", "media", "ping", "referrerpolicy", "rel", "shape", "target", "border", "crossorigin",
		"decoding", "height", "importance", "intrinsicsize", "ismap", "loading", "sizes", "srcset", "usemap",
		"bgcolor", "cellpadding", "cellspacing", "frame", "rules", "summary", "char", "charoff", "valign", "abbr",
		"axis", "colspan", "headers", "rowspan", "scope", "", "i", "ids", "Id", "data-x", "onclick", "href", "src",
		"alt", "clas", "classes", "x", "xy"}
	for _, f := range filters {
		var out []byte
		for _, n := range names {
			out = append(out, b2i(f.f.Contains([]byte(n)))+'0')
		}
		g.record("filter/" + f.name)
		g.field("names", []byte(strings.Join(names, ",")))
		g.field("contains", out)
	}
}

func b2i(b bool) byte {
	if b {
		return 1
	}
	return 0
}

func writeCJK(g *gmfWriter) {
	// Run-length encode each predicate over all code points.
	type pred struct {
		name string
		f    func(rune) string
	}
	preds := []pred{
		{"IsEastAsianWideRune", func(r rune) string { return string(rune(`0`[0] + b2i(util.IsEastAsianWideRune(r)))) }},
		{"IsSpaceDiscardingUnicodeRune", func(r rune) string { return string(rune(`0`[0] + b2i(util.IsSpaceDiscardingUnicodeRune(r)))) }},
		{"EastAsianWidth", util.EastAsianWidth},
	}
	for _, p := range preds {
		var out bytes.Buffer
		prev := ""
		start := rune(0)
		for r := rune(0); r <= 0x110000; r++ {
			v := ""
			if r <= 0x10FFFF {
				v = p.f(r)
			}
			if r == 0 {
				prev = v
				continue
			}
			if v != prev || r == 0x110000 {
				fmt.Fprintf(&out, "%x %x %s\n", start, r-1, prev)
				start = r
				prev = v
			}
		}
		g.record("cjk/" + p.name)
		g.field("runs", out.Bytes())
	}
}
