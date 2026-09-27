package main

import (
	"fmt"
	"math/rand"
	"os"
	"regexp"
	"sort"
	"strconv"
	"strings"

	"github.com/tdewolff/minify/v2"
	"github.com/tdewolff/minify/v2/css"
	"github.com/tdewolff/minify/v2/html"
	"github.com/tdewolff/minify/v2/json"
	"github.com/tdewolff/minify/v2/svg"
	"github.com/tdewolff/minify/v2/xml"
)

// Red-team generators (mode `rt`): grammar-generated documents over the full
// element/attribute/color tables, random option combinations ("o:"
// configurations, mirrored by tests/common/configs.rs), byte-level damage
// (invalid UTF-8, NUL, CR/CRLF, truncation at every offset) and large
// documents.
//
//	rt KIND N SEED OUT   # OUT "-" = uncompressed records on stdout
//
// KIND:
//
//	html    documents over every html hash name as tag/attribute, three chaos levels
//	hugo    well-formed pages shaped like neohugo's seeksnack output
//	css     stylesheets / inline declarations: numbers, units, all named colors,
//	        rgb()/hsl(), url(), calc(), at-rules, escapes, the 100-value/100-level limits
//	svg     documents with path data, viewBox, colors, CDATA/PI/doctype, namespaces
//	xml     feeds and documents with CDATA, entities, attributes, doctypes
//	json    numbers, strings with escapes, invalid input, nesting up to 3000
//	units   Number/Decimal at 17 precisions, Mediatype (incl. > 1024-byte runs),
//	        DataURI, PathData at 7 precisions (num/dec/mt/duri/path records)
//	bytes   generated documents and all upstream test literals with invalid UTF-8,
//	        NUL, CR/CRLF conversion or truncation
//	refuzz  the same damage applied to the inputs of the checked-in fixtures
//	trunc   every prefix of generated documents and upstream literals
//	big     64 KiB - 2 MiB documents
//	regress the checked-in regression fixtures (redteam*.txt.gz; N = depth)
//
// Every html/css/svg/xml/json/bytes/refuzz/trunc/big record uses a random
// "o:" configuration (dynM). Every record is a `min` record (fixtures.go)
// except for `units`, so the output streams straight into the Rust checker
// (or `replay`/`rerun`) or into a fixture file.

// ---------------------------------------------------------------- configs

// rtDelims are the template delimiters an "o:" configuration can select.
var rtDelims = [][2]string{{"", ""}, html.GoTemplateDelims, html.EJSTemplateDelims, html.PHPTemplateDelims, {"[[", "]]"}, {"{", "}"}}

// dynM builds the configuration encoded in an "o:" name:
//
//	o:HHHHHHH:T:CP:CF:SP:SF:JP:JF:X:U:N
//
// H = html KeepComments, KeepSpecialComments, KeepDefaultAttrVals,
// KeepDocumentTags, KeepEndTags, KeepQuotes, KeepWhitespace (0/1 each);
// T = rtDelims index; CP/CF = css Precision / KeepCSS2,Inline; SP/SF = svg
// Precision / KeepComments,Inline; JP/JF = json Precision / KeepNumbers;
// X = xml KeepWhitespace; U = M.URL (0 nil, 1 http, 2 https, 3 ftp);
// N = nested stand-ins (0 none, 1 trimming js, 2 positioned js error,
// 3 io.Copy js + mathml, 4 io.Copy css + js, 5 plain error svg + mathml).
func dynM(name string) *minify.M {
	f := strings.Split(name, ":")
	if len(f) != 12 || f[0] != "o" {
		panic("bad dyn config " + name)
	}
	bit := func(s string, i int) bool { return s[i] == '1' }
	num := func(s string) int {
		n, err := strconv.Atoi(s)
		if err != nil {
			panic(err)
		}
		return n
	}
	h := &html.Minifier{
		KeepComments:        bit(f[1], 0),
		KeepSpecialComments: bit(f[1], 1),
		KeepDefaultAttrVals: bit(f[1], 2),
		KeepDocumentTags:    bit(f[1], 3),
		KeepEndTags:         bit(f[1], 4),
		KeepQuotes:          bit(f[1], 5),
		KeepWhitespace:      bit(f[1], 6),
		TemplateDelims:      rtDelims[num(f[2])],
	}
	c := &css.Minifier{Precision: num(f[3]), KeepCSS2: bit(f[4], 0), Inline: bit(f[4], 1)}
	s := &svg.Minifier{Precision: num(f[5]), KeepComments: bit(f[6], 0), Inline: bit(f[6], 1)}
	j := &json.Minifier{Precision: num(f[7]), KeepNumbers: bit(f[8], 0)}
	x := &xml.Minifier{KeepWhitespace: bit(f[9], 0)}
	m := allM(h, c, j, s, x)
	switch f[10] {
	case "1":
		m = withURL(m, "http://example.com/")
	case "2":
		m = withURL(m, "https://example.com/")
	case "3":
		m = withURL(m, "ftp://example.com/")
	}
	switch f[11] {
	case "1":
		m.AddFuncRegexp(regexp.MustCompile(jsPattern), trimCopyFunc)
	case "2":
		m.AddFuncRegexp(regexp.MustCompile(jsPattern), errJSFunc)
	case "3":
		m.AddFunc("application/javascript", copyFunc)
		m.AddFunc("application/mathml+xml", copyFunc)
	case "4":
		m.AddFunc("text/css", copyFunc)
		m.AddFunc("application/javascript", copyFunc)
	case "5":
		m.AddFunc("image/svg+xml", errPlainFunc)
		m.AddFunc("application/mathml+xml", errPlainFunc)
	}
	return m
}

type rtGen struct {
	*advGen
	htmlNames, cssNames, svgNames []string
	colorNames, colorHexes        []string
	ms                            map[string]*minify.M
	chaos                         int // 0 clean, 1 moderate, 2 chaotic (per document)
}

// hashNames lists every name of a hasher-generated Hash type.
func hashNames(str func(uint32) string, to func([]byte) uint32, maxLen int) []string {
	var names []string
	for start := uint32(0); start < 1<<14; start++ {
		for n := uint32(1); n <= uint32(maxLen); n++ {
			h := start<<8 | n
			s := str(h)
			if s != "" && to([]byte(s)) == h {
				names = append(names, s)
			}
		}
	}
	sort.Strings(names)
	return names
}

func newRTGen(seed int64) *rtGen {
	g := &rtGen{advGen: &advGen{rand.New(rand.NewSource(seed))}, ms: map[string]*minify.M{}}
	g.htmlNames = hashNames(func(h uint32) string { return html.Hash(h).String() }, func(b []byte) uint32 { return uint32(html.ToHash(b)) }, 24)
	g.cssNames = hashNames(func(h uint32) string { return css.Hash(h).String() }, func(b []byte) uint32 { return uint32(css.ToHash(b)) }, 27)
	g.svgNames = hashNames(func(h uint32) string { return svg.Hash(h).String() }, func(b []byte) uint32 { return uint32(svg.ToHash(b)) }, 28)
	for h := range css.ShortenColorName {
		g.colorNames = append(g.colorNames, h.String())
	}
	sort.Strings(g.colorNames)
	for k := range css.ShortenColorHex {
		g.colorHexes = append(g.colorHexes, k)
	}
	sort.Strings(g.colorHexes)
	return g
}

func (g *rtGen) m(cfg string) *minify.M {
	if m, ok := g.ms[cfg]; ok {
		return m
	}
	if len(g.ms) > 256 {
		g.ms = map[string]*minify.M{} // each M holds compiled regexps
	}
	var m *minify.M
	if strings.HasPrefix(cfg, "o:") {
		m = dynM(cfg)
	} else {
		m = configByName(cfg)
	}
	g.ms[cfg] = m
	return m
}

func (g *rtGen) bits(n int, p int) string {
	b := make([]byte, n)
	for i := range b {
		b[i] = '0'
		if g.r.Intn(100) < p {
			b[i] = '1'
		}
	}
	return string(b)
}

func (g *rtGen) prec() string {
	if g.chance(2) {
		return "0"
	}
	return g.pick([]string{"-1", "1", "2", "3", "4", "5", "6", "8", "15", "16", "1", "2", "3"})
}

// cfgName returns a random "o:" configuration; half of them use neohugo's
// HTML options (KeepSpecialComments, KeepDefaultAttrVals, KeepDocumentTags,
// KeepEndTags).
func (g *rtGen) cfgName() string {
	if g.chance(8) {
		return "seeksnack"
	}
	h := "0111100"
	if g.chance(2) {
		h = g.bits(7, 50)
	}
	t := "0"
	if g.chance(4) {
		t = strconv.Itoa(g.r.Intn(len(rtDelims)))
	}
	u := "0"
	if g.chance(4) {
		u = strconv.Itoa(g.r.Intn(4))
	}
	n := strconv.Itoa(g.r.Intn(6))
	if g.chance(3) {
		n = "0"
	}
	return strings.Join([]string{"o", h, t, g.prec(), g.bits(1, 60) + g.bits(1, 10), g.prec(), g.bits(2, 25), g.prec(), g.bits(1, 20), g.bits(1, 30), u, n}, ":")
}

// minRecSafe is minRec, except that a panic of the Go minifier is recorded
// as `min CONFIG MEDIATYPE INPUT ! HEX(panic value) -` (the port must panic
// too) instead of ending the run.
func minRecSafe(w *fixWriter, cfg string, m *minify.M, mediatype string, in []byte) {
	defer func() {
		if p := recover(); p != nil {
			w.rec(panicRec(cfg, mediatype, in, p)...)
		}
	}()
	minRec(w, cfg, m, mediatype, in)
}

func panicRec(cfg, mediatype string, in []byte, p any) []string {
	return []string{"min", cfg, hx([]byte(mediatype)), hx(in), "!", hx([]byte(fmt.Sprint(p))), "-"}
}

func (g *rtGen) rec(w *fixWriter, mt string, in []byte) {
	cfg := g.cfgName()
	minRecSafe(w, cfg, g.m(cfg), mt, in)
}

func (g *rtGen) randCase(s string) string {
	switch g.r.Intn(4) {
	case 0:
		return strings.ToUpper(s)
	case 1:
		b := []byte(s)
		for i := range b {
			if g.chance(2) && 'a' <= b[i] && b[i] <= 'z' {
				b[i] -= 'a' - 'A'
			}
		}
		return string(b)
	}
	return s
}

// ---------------------------------------------------------------- HTML

var rtWS = []string{" ", " ", "  ", "\n", "\t", "\r\n", "\r", "\f", " \n ", "\n\n", "\u00a0", "\v", "\u2003", ""}

var rtEntities = []string{"&amp;", "&amp", "&AMP;", "&lt;", "&lt", "&LT", "&gt;", "&gt", "&quot;", "&quot", "&QUOT;", "&apos;", "&apos", "&nbsp;", "&nbsp", "&NBSP;", "&copy;", "&copy", "&COPY", "&reg", "&shy;", "&not", "&notin;", "&notit;", "&hellip;", "&mdash;", "&ndash;", "&lsquo;", "&rsquo;", "&ldquo;", "&rdquo;", "&middot;", "&times;", "&divide;", "&euro;", "&thinsp;", "&zwj;", "&zwnj;", "&lrm;", "&Tab;", "&NewLine;", "&excl;", "&num;", "&dollar;", "&percnt;", "&lpar;", "&rpar;", "&ast;", "&plus;", "&comma;", "&period;", "&sol;", "&colon;", "&semi;", "&equals;", "&quest;", "&commat;", "&lsqb;", "&bsol;", "&rsqb;", "&Hat;", "&lowbar;", "&grave;", "&lcub;", "&verbar;", "&rcub;", "&aacute;", "&Aacute", "&AElig;", "&acE;", "&NotNestedGreaterGreater;", "&CounterClockwiseContourIntegral;", "&fjlig;", "&bne;", "&unknown;", "&;", "&", "& ", "&#", "&#;", "&#x;", "&#X41;", "&#65;", "&#065;", "&#0065", "&#x41", "&#0;", "&#9;", "&#10;", "&#13;", "&#32;", "&#34;", "&#38;", "&#39;", "&#60;", "&#62;", "&#96;", "&#127;", "&#128;", "&#130;", "&#159;", "&#160;", "&#173;", "&#xD800;", "&#xDFFF;", "&#xFFFD;", "&#xFEFF;", "&#x10FFFF;", "&#x110000;", "&#99999999999999999999;", "&#x0000000000041;", "&#-1;", "&#x-1;", "&#1a;", "&#xg;", "&#x3C;", "&#x3c;", "&#X3E;", "&#x22;", "&#x27;", "&#x26;", "&#x60;", "&#x3D;", "&#61;"}

var rtTextWords = []string{"a", "text", "Hello", "world", "x", "1", "=", "<", ">", "a<b", "a>b", "'", "\"", "`", "/", "?", "{{", "}}", "{{ .X }}", "{{ if .Y }}a{{ end }}", "{{/* c */}}", "{{-", "-}}", "<%", "%>", "<% x %>", "<?", "?>", "<?php echo 1; ?>", "[[", "]]", "[[ x ]]", "{", "}", "{x}", "é", "中文", "\u00a0", "\u200b", "\x00", "\xff", "\xc3", "\xe2\x82", "\xed\xa0\x80", "\xef\xbf\xbd", "\x7f"}

var rtCleanWords = []string{"a", "text", "Hello", "world", "x", "1", "the quick", "fox", "é", "中文", "a.b", "a-b", "(c)", "!", "?", ","}
var rtCleanWS = []string{" ", " ", " ", "  ", "\n", "\n  ", "\t", " \n", ""}

func (g *rtGen) htmlText() string {
	var b strings.Builder
	n := 1 + g.r.Intn(6)
	for i := 0; i < n; i++ {
		if g.chaos == 0 {
			if g.chance(2) {
				b.WriteString(g.pick(rtCleanWS))
			} else if g.chance(8) {
				b.WriteString(g.pick([]string{"&amp;", "&lt;", "&gt;", "&quot;", "&nbsp;", "&copy;", "&#39;", "&#x27;", "&hellip;"}))
			} else {
				b.WriteString(g.pick(rtCleanWords))
			}
			continue
		}
		switch g.r.Intn(4) {
		case 0:
			b.WriteString(g.pick(rtWS))
		case 1:
			b.WriteString(g.pick(rtEntities))
		default:
			b.WriteString(g.pick(rtTextWords))
		}
	}
	return b.String()
}

var rtAttrVals = []string{"", " ", "  ", "x", "a b", " a  b ", "a\tb\nc\r\nd", "a=b", "a==b", "=", "==", "'", "\"", "''", "\"\"", "a'b", "a\"b", "a'b\"c", "a''b\"c", "a'\"'b", "`", "a`b", "<", ">", "a>b", "a<b", "&", "&amp;", "&quot;", "&#39;", "&#34;", "&apos;", "&quot;&#39;", "&lt;&gt;", "&nbsp;", "&nbsp", "1", "01", "1.0", "one", "on", "ON", "true", "none", "auto", "{{ .X }}", "a{{.Y}}b", "{{", "}}", "<%= x %>", "<?x?>", "[[y]]", "{z}", "\x00", "é", "\u00a0", "\xff", "http://example.com/", "HTTP://Example.COM/a b", "https://example.com/?a=1&amp;b=2", "HTTPS:x", "http:", "https:x", "https:", "httpx:", "data:,", "data:text/plain,a%20b", "DATA:text/plain;charset=US-ASCII,%41", "data:image/svg+xml,%3Csvg%3E%3C/svg%3E", "data:;base64,YQ==", "javascript:alert(1)", "JavaScript:void(0)", "#top", "/a/b?c#d", "mailto:a@b.c", "tel:+1", "//cdn.example.com/x.js", "text/javascript", "TEXT/JavaScript", "application/javascript", "module", "text/css", "TEXT/CSS", "text/css; charset=utf-8", " text/css ", "text", "submit", "radio", "RADIO", "get", "GET", "post", "rect", "RECT", "all", "ALL", "application/x-www-form-urlencoded", "multipart/form-data", "image/png, image/*", "text/html; charset=utf-8", "TEXT/HTML;CHARSET=UTF-8", "text/html ; charset = \"utf-8\"", "width=device-width, initial-scale=1.0", "width=device-width,initial-scale=1.00,maximum-scale=01.50,user-scalable=no", "initial-scale=.50", "a, b, c", "a,b , c", "ltr", "rtl", "keywords", "viewport", " Viewport ", "content-type", " Content-Type ", "utf-8", "UTF-8"}

func (g *rtGen) attrValue(name string) string {
	low := strings.ToLower(name)
	switch {
	case low == "style" && g.chance(2):
		return g.cssDecls(4)
	case strings.HasPrefix(low, "on") && g.chance(2):
		return g.pick([]string{"alert('x')", "javascript:alert(1)", "JAVASCRIPT:x()", "  x()  ", "", " ", "a&amp;&amp;b", "return false;", "javascript:", "javascript: ", "if(a<b){c()}", "{{ .JS }}"})
	case low == "srcset" && g.chance(2):
		return g.pick([]string{"a.png 1x, b.png 2x", " a.png  1x ,b.png 2x ", "HTTP://x/a.png 100w"})
	}
	if g.chance(5) {
		return g.htmlText()
	}
	v := g.pick(rtAttrVals)
	if g.chance(6) {
		v += g.pick(rtAttrVals)
	}
	return v
}

func (g *rtGen) attrName() string {
	if g.chance(10) {
		return g.pick([]string{"data-x", "aria-label", "x", "on", "onx", "ON", "o", "_", "a:b", "xmlns:x", "xml:lang", "@click", ":class", "v-if", "#x", "[y]", "(z)", "{{ .A }}", "a{{.B}}", "\"", "'", "<", "=", "é", "\x00"})
	}
	n := g.pick(g.htmlNames)
	if g.chance(5) {
		n = g.randCase(n)
	}
	return n
}

func (g *rtGen) attr() string {
	if g.chaos == 0 || (g.chaos == 1 && !g.chance(4)) {
		// well-formed attribute
		name := g.pick(g.htmlNames)
		if g.chance(10) {
			return name
		}
		val := g.attrValue(name)
		switch {
		case val != "" && !strings.ContainsAny(val, " \t\n\r\f\"'`=<>") && g.chance(3):
			return name + "=" + val
		case g.chance(4):
			return name + "='" + strings.ReplaceAll(val, "'", "&#39;") + "'"
		default:
			return name + "=\"" + strings.ReplaceAll(val, "\"", "&quot;") + "\""
		}
	}
	name := g.attrName()
	switch g.r.Intn(9) {
	case 0:
		return name // boolean
	case 1:
		return name + "="
	case 2:
		return name + "=" + g.pick(rtWS)
	}
	val := g.attrValue(name)
	eq := "="
	if g.chance(8) {
		eq = g.pick([]string{" = ", "\n=\n", "\t=", "= "})
	}
	switch g.r.Intn(6) {
	case 0: // unquoted (may run into the tag end or other attributes)
		return name + eq + val
	case 1:
		return name + eq + "'" + strings.ReplaceAll(val, "'", "&#39;") + "'"
	case 2:
		return name + eq + "\"" + val + "\"" // raw: may break the tag
	case 3:
		return name + eq + "'" + val + "'"
	default:
		return name + eq + "\"" + strings.ReplaceAll(val, "\"", "&quot;") + "\""
	}
}

// optional end tag families and what follows them
var rtOptEnd = []string{"p", "li", "dt", "dd", "option", "optgroup", "tr", "td", "th", "thead", "tbody", "tfoot", "rb", "rt", "rtc", "rp", "colgroup", "caption", "html", "head", "body"}
var rtBlockish = []string{"address", "article", "aside", "blockquote", "details", "div", "dl", "fieldset", "figcaption", "figure", "footer", "form", "h1", "h2", "h3", "h4", "h5", "h6", "header", "hgroup", "hr", "main", "menu", "nav", "ol", "p", "pre", "section", "table", "ul", "a", "audio", "del", "ins", "map", "noscript", "video", "span", "i", "b", "img", "br", "button", "select", "textarea", "input", "label", "iframe", "object", "svg", "math", "template", "slot", "x-y"}

func (g *rtGen) tagName() string {
	var n string
	switch g.r.Intn(10) {
	case 0, 1:
		n = g.pick(rtOptEnd)
	case 2, 3:
		n = g.pick(rtBlockish)
	case 4:
		n = g.pick([]string{"x-custom", "my-el", "a-", "svg:rect", "x:y", "é", "h7", "a1", "_", "ruby", "wbr", "listing", "xmp", "plaintext", "noembed", "noframes", "frameset", "frame", "image", "isindex", "keygen", "menuitem", "bgsound", "basefont"})
	default:
		n = g.pick(g.htmlNames)
	}
	if g.chance(6) {
		n = g.randCase(n)
	}
	return n
}

func (g *rtGen) startTag(b *strings.Builder, name string) {
	b.WriteString("<" + name)
	na := g.r.Intn(4)
	if g.chance(12) {
		na = 8 + g.r.Intn(40) // exercise the token buffer growth
	}
	for i := 0; i < na; i++ {
		if g.chaos == 0 {
			b.WriteString(g.pick([]string{" ", " ", " ", "\n", "  "}))
		} else {
			b.WriteString(g.pick([]string{" ", " ", "\n", "\t", "  ", "\r\n", "/", " /", ""}))
		}
		b.WriteString(g.attr())
		if g.chance(15) && i > 0 {
			// duplicate attribute
			b.WriteString(" " + g.attr())
		}
	}
	if g.chaos == 0 {
		b.WriteString(g.pick([]string{">", ">", ">", " >", "/>"}))
		return
	}
	b.WriteString(g.pick([]string{">", ">", ">", " >", "/>", " />", "\n>", "/ >", ""}))
}

func (g *rtGen) endTag(b *strings.Builder, name string) {
	switch g.r.Intn(12) {
	case 0:
		name = strings.ToUpper(name)
	case 1:
		b.WriteString("</" + name + " x=y>")
		return
	case 2:
		b.WriteString("</" + name + " >")
		return
	case 3:
		b.WriteString("</" + name + "\n>")
		return
	case 4:
		b.WriteString("</" + name + "/>")
		return
	}
	b.WriteString("</" + name + ">")
}

var rtComments = []string{"<!-- c -->", "<!---->", "<!--->", "<!-->", "<!-- a -- b -->", "<!--!-->", "<!--[if IE]><p>x  y</p><![endif]-->", "<!--[if lt IE 9]><script src=a.js></script><![endif]-->", "<!--[if IE 8]><link rel=stylesheet href=ie.css><![endif]-->", "<!--[if IE]>x<![endif]-->", "<!--[if IE]><![endif]-->", "<!--[if IE]>  <![endif]-->", "<!--[if IE]><!--[if IE 6]>y<![endif]--><![endif]-->", "<!--[if !IE]><!--><p>a</p><!--<![endif]-->", "<!--[if !IE]>--><p>b</p><!--<![endif]-->", "<![if !IE]><p>c</p><![endif]>", "<![endif]-->", "<!--<![endif]-->", "<!--[if gte mso 9]><xml><o:x/></xml><![endif]-->", "<!--[if IE]><p class = \"a\" >x</p ><![endif]-->", "<!--[if IE]><style>a { color : red }</style><![endif]-->", "<!--[if IE]><svg><path d=\"M 0 0 L 10 10\"/></svg><![endif]-->", "<!--#include virtual=\"/x\" -->", "<!--#-->", "<!--#echo var=\"x\"-->", "<!--# -->", "<!", "<!x>", "<!DOCTYPE x>", "<!-", "<?xml version=\"1.0\"?>", "<?x ?>", "</>", "</ >", "</ x>", "<>", "< p>", "<3", "<!--[if IE]>", "<!--[if", "<![CDATA[x]]>", "<!--{{ .C }}-->", "<!-- {{ c }} -->"}

func (g *rtGen) rawContent(name string) string {
	switch strings.ToLower(name) {
	case "style":
		if g.chance(3) {
			return g.cssDecls(3)
		}
		return g.cssSheet()
	case "script":
		switch g.r.Intn(5) {
		case 0:
			return g.jsonValue(0)
		case 1:
			return g.pick([]string{"", " ", "\n", "var x = 1 ;", "  a < b && c > d  ", "document.write('</scr'+'ipt>')", "<!-- x -->", "<!-- <script> -->", "{{ .JS }}", "x = \"</style>\"", "/* </p> */"})
		default:
			return g.htmlText()
		}
	}
	return g.htmlText()
}

func (g *rtGen) htmlNode(b *strings.Builder, depth int) {
	switch g.r.Intn(16) {
	case 0, 1, 2, 3:
		b.WriteString(g.htmlText())
		return
	case 4:
		if g.chaos > 0 || g.chance(3) {
			b.WriteString(g.pick(rtComments))
			return
		}
	case 5:
		// raw text elements
		name := g.pick([]string{"script", "style", "iframe", "noscript", "xmp", "plaintext", "title", "textarea", "noembed", "noframes", "SCRIPT", "Style"})
		g.startTag(b, name)
		b.WriteString(g.rawContent(name))
		if !g.chance(12) {
			g.endTag(b, name)
		}
		return
	case 6:
		// pre/textarea/listing whitespace
		name := g.pick([]string{"pre", "textarea", "listing", "PRE", "code"})
		g.startTag(b, name)
		b.WriteString(g.pick([]string{"\n", "\r\n", "", "  ", "\n\n"}))
		n := g.r.Intn(4)
		for i := 0; i < n; i++ {
			if g.chance(2) {
				b.WriteString(g.htmlText())
			} else if depth < 5 {
				g.htmlNode(b, depth+1)
			}
		}
		if !g.chance(10) {
			g.endTag(b, name)
		}
		b.WriteString(g.pick(rtWS))
		return
	case 7:
		if depth < 4 {
			// foreign content
			if g.chance(2) {
				svg := g.svgDoc()
				if i := strings.Index(svg, "<svg"); i > 0 {
					svg = svg[i:]
				}
				b.WriteString(svg)
			} else {
				b.WriteString(g.pick([]string{"<math><mi>x</mi><mo>=</mo><mn>1.50</mn></math>", "<math display=\"block\"><mrow> <mi> a </mi> </mrow></math>", "<MATH><mtext>a  b</mtext></MATH>", "<math><annotation-xml encoding=\"text/html\"><p>x</p></annotation-xml></math>", "<math>", "<svg>", "<svg><foreignObject><p>a  b</p></foreignObject></svg>", "<svg><title>t</title><desc>  d  </desc></svg>", "<svg viewBox=\"0 0 10 10\"><path d=\"M 0 0 L 10 10\"/></svg>", "<svg><style>a{color:red}</style></svg>", "<svg/>", "<svg xmlns=\"http://www.w3.org/2000/svg\"><![CDATA[x]]></svg>"}))
			}
			return
		}
	case 8:
		if depth < 5 {
			// optional end tags: sequences of siblings
			fams := [][]string{{"p"}, {"li"}, {"dt", "dd"}, {"option"}, {"optgroup", "option"}, {"tr", "td", "th"}, {"thead", "tbody", "tfoot"}, {"rb", "rt", "rtc", "rp"}, {"td"}, {"colgroup", "col"}}
			fam := fams[g.r.Intn(len(fams))]
			n := 1 + g.r.Intn(4)
			for i := 0; i < n; i++ {
				name := g.pick(fam)
				g.startTag(b, name)
				if g.chance(2) {
					b.WriteString(g.htmlText())
				}
				if g.chance(3) {
					g.htmlNode(b, depth+1)
				}
				if g.chance(2) {
					b.WriteString(g.pick(rtWS))
				}
				if g.chance(2) {
					g.endTag(b, name)
				}
				if g.chance(3) {
					b.WriteString(g.pick(rtWS))
				}
			}
			// what follows an omitted end tag matters
			if g.chance(2) {
				nx := g.pick(rtBlockish)
				if g.chance(2) {
					b.WriteString("</" + nx + ">")
				} else {
					g.startTag(b, nx)
				}
			}
			return
		}
	case 9:
		if depth < 5 {
			// template element
			g.startTag(b, g.pick([]string{"template", "TEMPLATE", "slot"}))
			n := g.r.Intn(4)
			for i := 0; i < n; i++ {
				g.htmlNode(b, depth+1)
			}
			if !g.chance(8) {
				b.WriteString("</template>")
			}
			b.WriteString(g.pick(rtWS))
			return
		}
	case 10:
		// select contents (text is skipped after select/optgroup/option)
		b.WriteString("<select>")
		b.WriteString(g.pick(rtWS))
		n := g.r.Intn(5)
		for i := 0; i < n; i++ {
			switch g.r.Intn(4) {
			case 0:
				b.WriteString("<optgroup label=a>" + g.pick(rtWS))
			case 1:
				b.WriteString("</optgroup>" + g.pick(rtWS))
			default:
				b.WriteString("<option" + g.pick([]string{"", " value=\"\"", " selected", " value=a"}) + ">" + g.htmlText())
				if g.chance(2) {
					b.WriteString("</option>")
				}
				b.WriteString(g.pick(rtWS))
			}
		}
		if !g.chance(8) {
			b.WriteString("</select>")
		}
		return
	case 11:
		// meta / link / input / a special handling
		b.WriteString(g.pick([]string{
			"<meta http-equiv=\"" + g.pick([]string{"content-type", "Content-Type", " content-type ", "CONTENT-TYPE", "refresh", "x"}) + "\" content=\"" + g.attrValue("content") + "\"" + g.pick([]string{"", " charset=utf-8", " name=viewport"}) + ">",
			"<meta name=\"" + g.pick([]string{"keywords", "viewport", " Keywords ", "VIEWPORT", "description"}) + "\" content=\"" + g.pick(rtAttrVals) + "\">",
			"<meta content=\"" + g.pick(rtAttrVals) + "\" name=" + g.pick([]string{"viewport", "keywords"}) + ">",
			"<meta charset=" + g.pick([]string{"utf-8", "\"UTF-8\"", "''", "x"}) + ">",
			"<input type=" + g.pick([]string{"text", "radio", "RADIO", "\"\"", "checkbox"}) + " value=" + g.pick([]string{"\"\"", "on", "ON", "x", "''"}) + ">",
			"<input value=\"\" type=radio>",
			"<a id=" + g.pick([]string{"x", "\"x\"", "''", "y"}) + " name=" + g.pick([]string{"x", "\"x\"", "''", "z"}) + ">",
			"<script src=a.js charset=utf-8></script>",
			"<script charset=utf-8 src=\"\"></script>",
			"<script></script>", "<style></style>", "<script type=module></script>", "<style media=all></style>",
			"<script>\n</script>", "<style> </style>",
			"<link rel=stylesheet type=\"text/css\" href=\"" + g.attrValue("href") + "\">",
			"<form action=\"\" method=get enctype=\"application/x-www-form-urlencoded\">",
			"<button type=submit>b</button>",
			"<td colspan=1 rowspan=\"1\">", "<area shape=rect>", "<col span=1>",
			"<style amp-boilerplate>body { a : b }</style>",
			"<img src=\"" + g.dataURI() + "\" alt=\"" + g.htmlText() + "\">",
			"<object data=\"" + g.attrValue("data") + "\" type=\"" + g.attrValue("type") + "\">",
			"<embed type=\" Image/SVG+XML ; a = b \">",
			"<source type=\"video/mp4; codecs=&quot;avc1&quot;\">",
		}))
		return
	}
	name := g.tagName()
	g.startTag(b, name)
	if depth < 6 {
		n := g.r.Intn(5)
		for i := 0; i < n; i++ {
			g.htmlNode(b, depth+1)
		}
	}
	if !g.chance(5) {
		g.endTag(b, name)
	}
	if g.chance(3) {
		b.WriteString(g.pick(rtWS))
	}
}

func (g *rtGen) htmlDoc() string {
	g.chaos = g.r.Intn(3)
	var b strings.Builder
	if g.chance(2) {
		b.WriteString(g.pick([]string{"<!DOCTYPE html>", "<!doctype html>", "<!DOCTYPE HTML PUBLIC \"-//W3C//DTD HTML 4.01//EN\">", "<!doctype>", "<!DOCTYPE html SYSTEM \"about:legacy-compat\">", "<!DOCTYPE", "<!doctypehtml>"}))
		b.WriteString(g.pick(rtWS))
	}
	doc := g.chance(2)
	if doc {
		g.startTag(&b, g.pick([]string{"html", "HTML"}))
		b.WriteString(g.pick(rtWS))
		if g.chance(2) {
			g.startTag(&b, "head")
		}
		nh := g.r.Intn(5)
		for i := 0; i < nh; i++ {
			b.WriteString(g.pick(rtWS))
			g.htmlNode(&b, 1)
		}
		if g.chance(2) {
			b.WriteString(g.pick([]string{"</head>", "</HEAD>", "</head >"}))
		}
		b.WriteString(g.pick(rtWS))
		if g.chance(2) {
			g.startTag(&b, "body")
		}
	}
	n := 1 + g.r.Intn(10)
	for i := 0; i < n; i++ {
		g.htmlNode(&b, 0)
	}
	if doc && g.chance(2) {
		b.WriteString(g.pick([]string{"</body></html>", "</body>\n</html>\n", "</html>", "</BODY></HTML>", "</body>", " </body> </html> "}))
	}
	return b.String()
}

// ---------------------------------------------------------------- CSS

var rtUnits = []string{"", "", "", "px", "PX", "Px", "em", "EM", "rem", "%", "vh", "vw", "vmin", "vmax", "deg", "DEG", "rad", "grad", "turn", "s", "ms", "MS", "Q", "q", "cm", "mm", "in", "pt", "pc", "ex", "ch", "fr", "dpi", "dpcm", "dppx", "x", "hz", "khz", "e", "e3", "E", "e-", "px2", "\\70x", "\\31 ", "\\", "-x", "_x", "é", "n", "n+1"}

func (g *rtGen) cssNumber() string {
	if g.chance(4) {
		return g.pick([]string{"0", "00", "000", "0.0", ".0", "0.", "-0", "+0", "-.0", "+.0", "-0.0", "0e0", "0e1", "0E-1", "1", "01", "1.0", "1.00", "10", "100", "1000", "10000", "100000", "1000000", "1e6", "1e-6", "1e21", "1e-7", "1e308", "1e309", "1e-324", "5e-324", ".5", "0.5", "-.5", "+.5", "0.50", "00.500", "1.5e1", "15e-1", "1.5E+1", "1.5e+01", "123456789", "123456789012345678901234567890", "0.1234567890123456789", "9.999999", "99.9995", "0.0005", "0.00049", "1e2", "1e3", "1e-2", "10e-3", "2.5e-5", "-2.5e+5", "+1e", "1e", "1e+", "1.e1", ".e1", "e1", "--1", "+-1", "1.2.3", ".5.5", "-.5-.5", "1-1", "1+1"})
	}
	return g.number()
}

func (g *rtGen) cssColor() string {
	switch g.r.Intn(10) {
	case 0, 1:
		return g.randCase(g.pick(g.colorNames))
	case 2:
		return g.randCase(g.pick(g.colorHexes))
	case 3:
		return g.hexColor()
	case 4:
		return g.pick([]string{"#000", "#fff", "#FFF", "#000000", "#ffffff", "#FFFFFF", "#f00", "#ff0000", "#FF0000", "#ff000000", "#ff0000ff", "#FF0000FF", "#0000", "#000f", "#fff0", "#ffff", "#00000000", "#000000ff", "#aabbccdd", "#aabbccff", "#abcd", "#abcf", "#AbCdEf", "#aAbBcC", "#12345", "#1234567", "#123456789", "#", "#g00", "#ff00gg", "transparent", "currentColor", "currentcolor", "inherit", "none", "black", "white", "red", "Red", "RED", "navy", "#000080", "#808080", "gray", "grey", "darkgray", "darkgrey", "lightslategray", "lightslategrey", "rebeccapurple", "#639"})
	}
	fn := g.pick([]string{"rgb(", "rgba(", "hsl(", "hsla(", "RGB(", "Rgba(", "HSL(", "HsLa(", "hwb(", "lab(", "color("})
	var b strings.Builder
	b.WriteString(fn)
	n := 3
	switch g.r.Intn(6) {
	case 0:
		n = 4
	case 1:
		n = g.r.Intn(6)
	}
	space := g.chance(3)
	for i := 0; i < n; i++ {
		if i > 0 {
			if space {
				if i == 3 {
					b.WriteString(g.pick([]string{" / ", "/", " /", "/ ", " "}))
				} else {
					b.WriteString(g.pick([]string{" ", "  ", "\n"}))
				}
			} else {
				b.WriteString(g.pick([]string{",", ", ", " ,", " , ", ",,"}))
			}
		}
		switch g.r.Intn(9) {
		case 0:
			b.WriteString(strconv.Itoa(g.r.Intn(300) - 20))
		case 1:
			b.WriteString(strconv.Itoa(g.r.Intn(101)) + "%")
		case 2:
			b.WriteString(g.pick([]string{"0", "255", "256", "300", "-5", "-0", "128", "127.5", "254.5", "255.5", "0.5", ".5", "1", "1.0", "360", "-120", "480", "720", "1e2", "2.55e2", "0.1", "100%", "0%", "50%", "-10%", "110%", "33.3%", "66.6667%", "99.9%", "0.5%", "12.5%", "1e2%", "1e-2%", "45deg", "0.5turn", "1rad", "100grad", "none", "var(--x)", "calc(1 + 2)", "calc(50%)", "a", "", "50%%"}))
		case 3:
			b.WriteString(g.cssNumber())
		case 4:
			b.WriteString(g.cssNumber() + "%")
		case 5:
			b.WriteString(g.cssNumber() + g.pick(rtUnits))
		default:
			b.WriteString(fmt.Sprintf("%d.%d", g.r.Intn(256), g.r.Intn(1000)))
		}
	}
	if !g.chance(20) {
		b.WriteString(")")
	}
	return b.String()
}

func (g *rtGen) cssURL() string {
	u := g.pick([]string{"a.png", " b.png ", "c d.png", "'e f.png'", "\"g.png\"", "'h\"i.png'", "\"j'k.png\"", "l(m).png", "'n(o).png'", "p\\)q.png", "'r\\'s.png'", "\"t\\\"u.png\"", "v\\ w.png", "'x\\\ny.png'", "\"\"", "''", "", "  ", "é.png", "%20.png", "a\x00b", "data:image/png;base64,iVBORw0KGgo=", "'data:image/svg+xml;charset=utf-8,%3Csvg xmlns=%27http://www.w3.org/2000/svg%27/%3E'", "\"" + g.dataURI() + "\"", g.dataURI(), "'" + g.dataURI() + "'", "#a", "\"#b\"", "http://x/a?b=1&c=2", "'http://x/a b'", "a\tb", "a\nb"})
	fn := g.pick([]string{"url(", "URL(", "Url(", "url( ", "url(\n", "src("})
	end := g.pick([]string{")", " )", "\n)", ")", ""})
	return fn + u + end
}

func (g *rtGen) cssVal(depth int) string {
	switch g.r.Intn(14) {
	case 0, 1, 2:
		return g.cssNumber() + g.pick(rtUnits)
	case 3, 4:
		return g.cssColor()
	case 5:
		return g.cssURL()
	case 6:
		if depth < 3 {
			f := g.pick([]string{"calc(", "CALC(", "-webkit-calc(", "min(", "max(", "clamp(", "var(", "env(", "attr(", "translate(", "rotate(", "scale(", "matrix(", "linear-gradient(", "radial-gradient(", "repeat(", "minmax(", "cubic-bezier(", "steps(", "counter(", "format(", "local(", "image-set(", "drop-shadow(", "blur(", "alpha(", "expression("})
			var b strings.Builder
			b.WriteString(f)
			n := 1 + g.r.Intn(4)
			for i := 0; i < n; i++ {
				if i > 0 {
					b.WriteString(g.pick([]string{",", ", ", " ", " + ", " - ", " * ", " / ", "+", "-", "*", "/"}))
				}
				b.WriteString(g.cssVal(depth + 1))
			}
			if !g.chance(15) {
				b.WriteString(")")
			}
			return b.String()
		}
		return "0"
	case 7:
		return g.randCase(g.pick(g.cssNames))
	case 8:
		return g.cssString()
	case 9:
		return g.pick([]string{"U+0000-00FF", "U+0100", "U+4??", "u+0-10ffff", "U+20-10", "U+FFFFFFFFFFFFFFFFF", "U+0-7F", "U+00??", "U+1F600-1F64F", "U+A5", "u+???", "U+0025-00FF", "U+0-0", "U+10-1F", "U+??????", "U+0-10FFFF", "U+110000", "U+-1", "U+1-", "U+", "u+1e3", "U+1E3", "U+0000FF"})
	case 10:
		return g.pick([]string{"!important", "! important", "!IMPORTANT", "/", ",", "+", "*", "!", "=", "@", "\\30", "\\31 a", "\\", "#", ".", ">", "~", "|", "&", "^", "$", "?", "{", "}", "[", "]", "(", ")", ";", ":", "--x", "-", "_", "\u00a0", "é", "\\é", "\\0", "\\FFFFFF", "\\110000", "\\\n", "<!--", "-->", "/**/", "/*! k */", "/* c */", "/*", "*/"})
	case 11:
		return g.pick([]string{"progid:DXImageTransform.Microsoft.Alpha(Opacity=80)", "\"progid:DXImageTransform.Microsoft.gradient(startColorstr='#80000000', endColorstr='#80000000')\"", "alpha(opacity=50)", "Alpha(Opacity=50)"})
	default:
		return g.randCase(g.pick([]string{"auto", "none", "normal", "bold", "bolder", "lighter", "initial", "inherit", "unset", "revert", "medium", "thin", "thick", "left", "right", "top", "bottom", "center", "repeat", "no-repeat", "repeat-x", "repeat-y", "space", "round", "scroll", "fixed", "local", "padding-box", "border-box", "content-box", "cover", "contain", "solid", "dashed", "dotted", "double", "groove", "ridge", "inset", "outset", "hidden", "italic", "oblique", "small-caps", "sans-serif", "serif", "monospace", "cursive", "fantasy", "system-ui", "Arial", "Helvetica", "x-small", "xx-large", "larger", "smaller", "caption", "icon", "menu", "message-box", "small-caption", "status-bar", "400", "700", "100", "900", "1", "0", "block", "inline", "flex", "grid", "table", "ease", "linear", "infinite", "both", "forwards", "all"}))
	}
}

var rtProps = []string{"font", "font-family", "font-weight", "font-size", "font-style", "font-stretch", "font-variant", "line-height", "margin", "margin-top", "padding", "padding-left", "border", "border-top", "border-right", "border-bottom", "border-left", "border-width", "border-style", "border-color", "border-radius", "border-top-left-radius", "outline", "outline-width", "outline-color", "background", "background-color", "background-image", "background-position", "background-position-x", "background-size", "background-repeat", "background-attachment", "background-clip", "background-origin", "box-shadow", "text-shadow", "-ms-filter", "filter", "color", "fill", "stroke", "stop-color", "flood-color", "caret-color", "column-rule", "column-rule-color", "text-decoration", "text-decoration-color", "text-emphasis", "text-emphasis-color", "border-left-color", "flex", "flex-basis", "flex-grow", "flex-shrink", "flex-flow", "order", "unicode-range", "z-index", "counter-reset", "counter-increment", "orphans", "widows", "width", "height", "min-width", "max-height", "transform", "transition", "animation", "animation-name", "src", "content", "quotes", "grid-template-columns", "grid-area", "opacity", "display", "position", "top", "left", "right", "bottom", "cursor", "vertical-align", "text-align", "letter-spacing", "word-spacing", "white-space", "list-style", "list-style-type", "overflow", "visibility", "zoom", "*zoom", "_height", "-webkit-transition", "-moz-box-shadow", "--x", "--Y", "--empty", "--", "Color", "BACKGROUND", "MARGIN", "Font-Weight", "x", "\\63olor", "col\\or", "é"}

func (g *rtGen) cssDecl() string {
	p := g.pick(rtProps)
	var b strings.Builder
	b.WriteString(p)
	b.WriteString(g.pick([]string{":", ":", ": ", " : ", "\n:\n", ":\t", "::", ""}))
	if strings.HasPrefix(p, "--") && g.chance(2) {
		b.WriteString(g.pick([]string{"", " ", "  a  b  ", "{a:b}", " 1px ", "[x]", "calc( 1px )", " ; ", "!important", "  0.50  ", "#FFFFFF", " url( a ) ", "'  '", "(", "{"}))
	} else if g.chance(40) {
		// around the limits: minifyProperty skips > 100 values, minifyTokens
		// stops at 100 nested levels
		if g.chance(2) {
			n := 95 + g.r.Intn(12)
			for i := 0; i < n; i++ {
				if i > 0 {
					b.WriteString(g.pick([]string{" ", ",", " / "}))
				}
				b.WriteString(g.cssVal(2))
			}
		} else {
			d := 95 + g.r.Intn(12)
			f := g.pick([]string{"calc(", "rgb(", "f(", "var(", "min("})
			b.WriteString(strings.Repeat(f, d) + g.cssVal(2) + g.pick([]string{"", " 1px", ",#FFFFFF", " 0.50"}) + strings.Repeat(")", d-g.r.Intn(2)))
		}
	} else {
		n := 1 + g.r.Intn(6)
		for i := 0; i < n; i++ {
			if i > 0 {
				b.WriteString(g.pick([]string{" ", " ", " ", ",", ", ", " , ", " / ", "/", "  ", "\n", "\t"}))
			}
			b.WriteString(g.cssVal(0))
		}
	}
	if g.chance(8) {
		b.WriteString(g.pick([]string{" !important", "!important", "! important", " !IMPORTANT", "!ie", " ! ie", "!important!important", "/*c*/!important"}))
	}
	return b.String()
}

func (g *rtGen) cssDeclList() string {
	var b strings.Builder
	n := 1 + g.r.Intn(6)
	for i := 0; i < n; i++ {
		if i > 0 {
			b.WriteString(g.pick([]string{";", ";", "; ", ";\n", ";;", " ; "}))
		}
		if g.chance(12) {
			b.WriteString(g.pick([]string{"/* c */", "/*! k */", "/**/"}))
		}
		b.WriteString(g.cssDecl())
	}
	if g.chance(2) {
		b.WriteString(";")
	}
	return b.String()
}

var rtSelectors = []string{"a", "DIV", "div.Foo", ".Bar", "#Id", "a:hover", "a::before", "a:BEFORE", "A:NOT(.x)", ":is(a, b)", ":where(.a)", "ul>li", "ul > li", "p+p", "p ~ P", "a[href]", "a[href=\"x\"]", "a[href='x y']", "a[data-x=\"a-b\"]", "a[x=\"1a\"]", "a[x=\"-a\"]", "a[x=\"--a\"]", "a[x=\"\"]", "a[x=\"a\\\"b\"]", "a[x=y i]", "a[x=\"y\" I]", "a[x=\"y\"s]", "*", "*|*", "html", "BODY", ":root", "input[type=text]", "h1,h2", "h1 , h2", "svg|a", "a\\:b", ".a.b", ".\\31 a", "#\\31", "td:nth-child(2n+1)", "td:nth-child( 2N + 1 )", "li:nth-of-type(odd)", "[lang|=en]", "a[href$=\".pdf\"]", "a[href^='http']", "a[href*=x]", "a[x~=y]", "::selection", ":-moz-selection", "a:hover::after", ".é", "#中文", "a b c d", "a\nb", "a\tb", "\\30 x", "a /* c */ b", "@page :first", "from", "to", "0%", "50.0%", "100%"}

func (g *rtGen) cssRule(depth int) string {
	switch g.r.Intn(14) {
	case 0, 1:
		if depth < 3 {
			var b strings.Builder
			b.WriteString(g.pick([]string{"@media (min-width:1px)", "@media screen and (max-width: 100px)", "@media (min-width:0.50em) and (max-width : 100.0px)", "@media only screen and (-webkit-min-device-pixel-ratio:1.5),(min-resolution:144dpi)", "@MEDIA print", "@media all", "@media", "@media not all and (monochrome)", "@supports (display:grid)", "@supports not (display : grid) and (x:y)", "@supports (color: rgb(0 0 0 / 50%))", "@document url(x)", "@layer a", "@layer a, b", "@container (min-width: 400px)", "@-moz-document url-prefix()", "@media (min-width:1e3px)", "@media (min-width: 0px)", "@media(min-width:1px)and (max-width:2px)", "@page", "@page :first"}))
			b.WriteString(g.pick(rtWS) + "{")
			n := 1 + g.r.Intn(3)
			for i := 0; i < n; i++ {
				b.WriteString(g.cssRule(depth + 1))
			}
			if !g.chance(15) {
				b.WriteString("}")
			}
			return b.String()
		}
	case 2:
		return "@font-face" + g.pick(rtWS) + "{font-family:" + g.cssString() + ";src:" + g.pick([]string{"url(a.woff2) format(\"woff2\")", "local(\"Arial\"),url( 'b c.woff' )", "url(\"" + g.dataURI() + "\")", "url(a.eot?#iefix) format('embedded-opentype')", "local(Arial Bold)"}) + ";unicode-range:" + g.cssVal(0) + ";font-weight:" + g.pick([]string{"normal", "bold", "400", "700", "100 900"}) + ";font-display:swap}"
	case 3:
		return g.pick([]string{"@charset \"utf-8\";", "@charset 'UTF-8';", "@CHARSET \"x\";", "@import url(x.css);", "@import url( x.css ) screen;", "@import url(  );", "@import 'y.css';", "@import \"z.css\" print;", "@import url(\"z.css\");", "@import url( 'q.css' );", "@import url(\n a b \n);", "@import url(a.css) supports(display:grid) screen and (min-width:1px);", "@import url(a.css) layer(x);", "@namespace svg url(http://www.w3.org/2000/svg);", "@namespace url(\"http://x\");", "@import", "@import;", "@x;", "@x y z;", "@x{}", "@x y{a:b}"})
	case 4:
		return "@" + g.pick([]string{"keyframes", "-webkit-keyframes", "KEYFRAMES"}) + " a{" + g.pick([]string{"0%", "from", "FROM", "0.0%", "00%"}) + "{" + g.cssDeclList() + "}" + g.pick([]string{"50%", "50.0%", "33.333%", "1e1%"}) + "{" + g.cssDeclList() + "}" + g.pick([]string{"to", "100%", "TO", "100.0%"}) + "{" + g.cssDeclList() + "}}"
	case 5:
		return g.pick([]string{"/*! keep  this   comment */", "/* drop */", "/*!*/", "/*! a\n  b */", "/*", "/**/", "<!--", "-->", "<!-- a{b:c} -->"})
	case 6:
		return g.pick([]string{"a{", "}", "a{b}", "a{:b}", "a{b:}", "a{;;}", "@x;", "{a:b}", "a{b:c d{e}}", "a{b:c;d:(e}", "a{b:[c]}", "\\", "a{b:c\"d}", "a{b:c'd", "a{b:url(x}", "a{b:url('x}", "a,{b:c}", ",a{b:c}", "a{{b:c}}", "a{b:c}}", ";", "a{b:c!}", "a{!important}", "a{b:!important}", "a{b:c;;d:e}", "a{--x:{}}", "a{b:\"c\nd\"}"})
	case 7:
		// nested rules (CSS nesting)
		return g.pick(rtSelectors) + "{" + g.cssDeclList() + ";" + g.pick([]string{"&:hover", "& > a", ".x &", "&.y"}) + "{" + g.cssDeclList() + "}}"
	}
	n := 1 + g.r.Intn(3)
	var sels []string
	for i := 0; i < n; i++ {
		sels = append(sels, g.pick(rtSelectors))
	}
	return strings.Join(sels, g.pick([]string{",", ", ", " ,", ",\n"})) + g.pick(rtWS) + "{" + g.pick(rtWS) + g.cssDeclList() + g.pick(rtWS) + "}"
}

func (g *rtGen) cssDoc() string {
	var b strings.Builder
	n := 1 + g.r.Intn(6)
	for i := 0; i < n; i++ {
		b.WriteString(g.cssRule(0))
		b.WriteString(g.pick(rtWS))
	}
	return b.String()
}

// ---------------------------------------------------------------- SVG

func (g *rtGen) svgNumber() string {
	if g.chance(3) {
		return g.pick([]string{"0", "1", "10", "100", "-1", ".5", "0.5", "-.5", "1.5", "4.646", "1e2", "1e-2", "1.25e-1", "200.00", "0.0001", "3", "99.999", "99.9995", "0.00049", "1000", "10000", "123456.789", "-0", "+1", "1e+2", "1E2", "5e-324", "1e308", "12345678901234567890", "0.30000000000000004", ".1.2", "-.1-.2", "1.2.3", "01", "001.100", "2.5e-5", "9.9999999"})
	}
	return g.number()
}

func (g *rtGen) pathData() string {
	var b strings.Builder
	n := 1 + g.r.Intn(10)
	for i := 0; i < n; i++ {
		c := pathCmdsStr[g.r.Intn(len(pathCmdsStr))]
		if i == 0 && g.r.Intn(4) != 0 {
			c = "Mm"[g.r.Intn(2)]
		}
		if g.chance(20) {
			c = "xXbB"[g.r.Intn(4)]
		}
		b.WriteByte(c)
		k := 0
		switch c {
		case 'M', 'm', 'L', 'l', 'T', 't':
			k = 2
		case 'H', 'h', 'V', 'v':
			k = 1
		case 'C', 'c':
			k = 6
		case 'S', 's', 'Q', 'q':
			k = 4
		case 'A', 'a':
			k = 7
		}
		reps := 1 + g.r.Intn(3)
		if g.chance(6) {
			reps = 0
		}
		prev := "0"
		for j := 0; j < k*reps; j++ {
			sep := g.pick([]string{" ", ",", "", " , ", "\n", "\t", "  ", ",,"})
			if j > 0 || g.chance(2) {
				b.WriteString(sep)
			}
			if (c == 'A' || c == 'a') && (j%7 == 3 || j%7 == 4) {
				b.WriteString(g.pick([]string{"0", "1", "0", "1", "2", "01", "10", "1.0", "-0"}))
				continue
			}
			v := g.svgNumber()
			if g.chance(4) {
				v = prev // repeated coordinates trigger command conversions
			}
			prev = v
			b.WriteString(v)
		}
		if g.chance(15) {
			// dropped last coordinate
			b.WriteString(g.pick([]string{" ", ","}))
		}
	}
	return b.String()
}

func (g *rtGen) svgAttr() string {
	names := []string{"x", "y", "x1", "y2", "width", "height", "rx", "r", "cx", "cy", "viewBox", "viewbox", "d", "points", "fill", "stroke", "stop-color", "flood-color", "lighting-color", "color", "style", "transform", "gradientTransform", "version", "preserveAspectRatio", "baseProfile", "contentStyleType", "contentScriptType", "xml:space", "xmlns", "xmlns:xlink", "xmlns:svg", "xmlns:inkscape", "xmlns:sodipodi", "xlink:href", "href", "inkscape:label", "sodipodi:docname", "class", "id", "stroke-width", "stroke-dasharray", "opacity", "fill-opacity", "type", "offset", "font-size", "font-family", "text-anchor", "dx", "dy", "letter-spacing", "clip-path", "mask", "filter", "stdDeviation", "fx", "k1", "values", "keyTimes"}
	name := g.pick(names)
	if g.chance(10) {
		name = g.pick(g.svgNames)
	}
	var val string
	switch strings.ToLower(name) {
	case "d":
		val = g.pathData()
	case "viewbox":
		switch g.r.Intn(3) {
		case 0:
			val = g.pick([]string{"0 0 100 100", "0,0,100,100", "0 0 100.00 1e2", " 0 0 10 10", "0 0 10", "0  0 10 10", "-0.50 -0.5 24.000 24", "0 0 10PX 10", "0 0 10 10 10", "a b c d", "0,0 10,10", "0 0 010.0 0010", "", " ", "0 0 0 0", "0 0 -1 -1", "0,0,,10,10"})
		default:
			val = g.svgNumber() + g.pick([]string{" ", ",", ", ", "  "}) + g.svgNumber() + g.pick([]string{" ", ","}) + g.svgNumber() + " " + g.svgNumber()
		}
	case "points":
		var b strings.Builder
		k := g.r.Intn(8)
		for i := 0; i < k; i++ {
			if i > 0 {
				b.WriteString(g.pick([]string{" ", ",", ", ", "  "}))
			}
			b.WriteString(g.svgNumber())
		}
		val = b.String()
	case "fill", "stroke", "stop-color", "flood-color", "lighting-color", "color":
		val = g.cssColor()
		if g.chance(4) {
			val = g.pick([]string{"none", "currentColor", "url(#a)", "URL(#b)", "url(", "url(#a) red", "inherit", ""})
		}
	case "style":
		val = g.cssDeclList()
	case "transform", "gradienttransform":
		val = g.pick([]string{"translate(", "rotate(", "scale(", "matrix(", "skewX("}) + g.svgNumber() + g.pick([]string{" ", ",", ""}) + g.svgNumber() + ")"
	case "version":
		val = g.pick([]string{"1.1", "1.10", "1.0", "1.1px", "2"})
	case "preserveaspectratio":
		val = g.pick([]string{"xMidYMid meet", "none", "xMidYMid  meet", "xMidYMid", "xMinYMin slice"})
	case "baseprofile":
		val = g.pick([]string{"none", "full", "tiny", "NONE"})
	case "contentstyletype", "type":
		val = g.pick([]string{"text/css", "TEXT/CSS", " text/css ", "text/x", "text/ecmascript"})
	case "contentscripttype":
		val = g.pick([]string{"application/ecmascript", "text/javascript"})
	case "xml:space":
		val = g.pick([]string{"preserve", "default", "PRESERVE"})
	case "xmlns":
		val = g.pick([]string{"http://www.w3.org/2000/svg", "http://www.w3.org/1999/xhtml", ""})
	default:
		switch g.r.Intn(5) {
		case 0:
			val = g.svgNumber()
		case 1:
			val = g.svgNumber() + g.pick(rtUnits)
		case 2:
			val = g.pick([]string{"0", "0px", "10px", "10.50PX", "1e2", "50%", "5em", "0.0", "-0", "+1", "100%", "1 2", "a\"b", "a'b", "a&amp;b", "&quot;", "&apos;x&apos;", "#a", "  1  ", "1,2", "1 , 2", "auto", "inherit"})
		default:
			val = g.pick([]string{"a", "b c", "translate(1 2)", "1,2 3,4", "M0 0", "", "{{ .X }}", "é", "\x00", "a\tb\nc"})
		}
	}
	q := g.pick([]string{"\"", "'", "\""})
	if strings.Contains(val, q) {
		q = "\""
		if strings.Contains(val, "\"") {
			q = "'"
		}
	}
	if g.chance(20) {
		return name + "=" + val
	}
	return name + g.pick([]string{"=", "=", " = "}) + q + val + q
}

var rtSVGElems = []string{"g", "path", "rect", "circle", "ellipse", "line", "polyline", "polygon", "text", "tspan", "textPath", "use", "defs", "symbol", "linearGradient", "radialGradient", "stop", "clipPath", "mask", "pattern", "filter", "feGaussianBlur", "title", "desc", "metadata", "foreignObject", "style", "script", "svg", "svg:g", "svg:rect", "inkscape:grid", "sodipodi:namedview", "rdf:RDF", "a", "image", "switch", "marker", "view", "animate", "set", "x"}

func (g *rtGen) svgNode(b *strings.Builder, depth int) {
	switch g.r.Intn(12) {
	case 0:
		b.WriteString(g.pick([]string{"<!-- c -->", "<!---->", "<!--! keep -->", "<?pi x?>", "<?xml-stylesheet href=\"a.css\"?>", "<![CDATA[ a < b & c ]]>", "<![CDATA[]]>", "<![CDATA[x]]]]><![CDATA[>]]>", " text &amp; more ", "\n  ", "\r\n", "&#x41;", "&#65;", "&lt;&gt;", "&amp;amp;", "&unknown;", "&nbsp;", "<!DOCTYPE svg [ <!ENTITY a \"b\"> ]>", "&a;", "é", "\x00", "{{ .X }}", "<", ">", "]]>"}))
		return
	case 1:
		if depth > 0 {
			b.WriteString("<style" + g.pick([]string{"", " type=\"text/css\"", " type='TEXT/CSS'", " type=\"text/x\"", " media=\"all\""}) + ">")
			switch g.r.Intn(3) {
			case 0:
				b.WriteString("<![CDATA[" + g.cssDoc() + "]]>")
			case 1:
				b.WriteString(strings.ReplaceAll(g.cssDoc(), "<", ""))
			default:
				b.WriteString("  <![CDATA[ " + strings.ReplaceAll(g.cssDoc(), "]]>", "") + " ]]>  ")
			}
			b.WriteString("</style>")
			return
		}
	case 2:
		if depth > 0 {
			b.WriteString("<script>" + g.pick([]string{"var a = 1;", "<![CDATA[ a < b ]]>", ""}) + "</script>")
			return
		}
	}
	el := g.pick(rtSVGElems)
	if g.chance(10) {
		el = g.pick(g.svgNames)
	}
	b.WriteString("<" + el)
	na := g.r.Intn(6)
	for i := 0; i < na; i++ {
		b.WriteString(g.pick([]string{" ", "\n  ", "  ", "\t", ""}))
		b.WriteString(g.svgAttr())
	}
	if depth > 4 || g.chance(3) {
		b.WriteString(g.pick([]string{"/>", " />", "></" + el + ">", ">  </" + el + " >", "/ >", "/"}))
		return
	}
	b.WriteString(g.pick([]string{">", ">", " >", "\n>"}))
	n := g.r.Intn(4)
	for i := 0; i < n; i++ {
		g.svgNode(b, depth+1)
	}
	if !g.chance(20) {
		b.WriteString("</" + el + g.pick([]string{">", " >", ">\n", "\n>"}))
	}
}

func (g *rtGen) svgDocument() string {
	var b strings.Builder
	if g.chance(3) {
		b.WriteString(g.pick([]string{"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>\n", "<?xml version='1.0'?>", "<?xml?>", "\ufeff<?xml version=\"1.0\"?>"}))
	}
	if g.chance(5) {
		b.WriteString(g.pick([]string{"<!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\" \"http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd\">\n", "<!DOCTYPE svg [\n<!ENTITY ns_svg \"http://www.w3.org/2000/svg\">\n]>", "<!DOCTYPE svg>"}))
	}
	if g.chance(6) {
		b.WriteString("<!-- lead -->")
	}
	b.WriteString(g.pick([]string{"<svg", "<svg", "<SVG", "<svg:svg"}))
	na := g.r.Intn(7)
	for i := 0; i < na; i++ {
		b.WriteString(" ")
		b.WriteString(g.svgAttr())
	}
	b.WriteString(">")
	n := 1 + g.r.Intn(7)
	for i := 0; i < n; i++ {
		g.svgNode(&b, 1)
	}
	if !g.chance(15) {
		b.WriteString("</svg>")
	}
	if g.chance(6) {
		b.WriteString(g.pick([]string{"\n", " trailing", "<!-- t -->", "<svg/>"}))
	}
	return b.String()
}

// ---------------------------------------------------------------- XML

func (g *rtGen) xmlNode(b *strings.Builder, depth int) {
	switch g.r.Intn(10) {
	case 0, 1:
		b.WriteString(g.pick([]string{"text", " text ", "\n\t\t", "a &amp; b", "a &lt; b &gt; c", "&apos;&quot;", "&#34;&#x27;", "&#x3C;", "&#60;&#62;", "Mon, 02 Jan 2006 15:04:05 -0700", "  lots   of   space  ", "é", "&nbsp;", "\u00a0", "&amp;amp;", ">", "]]>", "\r\n", "\t", "\x00", "\xff", "&", "& ", "&;", "&#;", "&#x110000;", "a\rb", "{{ .X }}", "<", "=", "\"", "'"}))
		return
	case 2:
		b.WriteString(g.pick([]string{"<![CDATA[<p>a &amp; b</p>]]>", "<![CDATA[ plain ]]>", "<![CDATA[]]>", "<![CDATA[a]]]]><![CDATA[>b]]>", "<![CDATA[<img src=\"x\"/> text & more]]>", "<![CDATA[\n  indented\n]]>", "<![CDATA[x<y]]>", "<![CDATA[ ]]>", "<![CDATA[&amp;]]>", "<![CDATA[a&b]]>", "<![CDATA[<]]>", "<![CDATA[>]]>", "<![CDATA[&]]>", "<![CDATA[]]]>", "<![CDATA[x", "<![CDATA", "<![cdata[x]]>", "<![CDATA[\x00]]>", "<![CDATA[é]]>", "<![CDATA[ a  b ]]>"}))
		return
	case 3:
		b.WriteString(g.pick([]string{"<!-- c -->", "<?pi a?>", "<?xml-stylesheet href=\"a\"?>", "<!x>", "<!---->", "<!-- a -- b -->", "<?", "<!", "</>", "<>", "< a>", "<a", "</a", "<!DOCTYPE x>"}))
		return
	}
	t := g.pick([]string{"rss", "channel", "item", "title", "link", "description", "atom:link", "pubDate", "guid", "urlset", "url", "loc", "lastmod", "xhtml:link", "a", "B", "content:encoded", "dc:creator", "x-y", "_z", "é", "a.b", "svg", "html", "p"})
	b.WriteString("<" + t)
	na := g.r.Intn(4)
	for j := 0; j < na; j++ {
		b.WriteString(g.pick([]string{" ", "\n\t", "  ", "\t", ""}))
		b.WriteString(g.pick([]string{"href=\"http://x/a?b=1&amp;c=2\"", "rel='self'", "type=\"application/rss+xml\"", "x=\"a &quot;b&quot; c\"", "y='a \"b\" c'", "z=\"&apos;\"", "w=unq", "v=\"a\tb\nc\"", "isPermaLink=\"false\"", "version=\"2.0\"", "xmlns:atom=\"http://www.w3.org/2005/Atom\"", "xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\"", "e=\"&lt;&gt;&amp;&#34;&#39;\"", "q=\"\"", "q=''", "r=\"'\"", "s='\"'", "t=\"'&quot;\"", "u=\"&quot;&apos;&quot;\"", "k=\"  spaced  \"", "b", "c=", "d = \"x\"", "xml:space=\"preserve\"", "a=\"&#x9;&#xA;\"", "f=\"a\r\nb\"", "g=\"é\"", "h=\"\x00\""}))
	}
	if depth > 5 || g.chance(4) {
		b.WriteString(g.pick([]string{"/>", " />", "/ >", "\n/>"}))
		return
	}
	b.WriteString(g.pick([]string{">", ">", " >", "\n>"}))
	n := g.r.Intn(5)
	for i := 0; i < n; i++ {
		g.xmlNode(b, depth+1)
	}
	if !g.chance(15) {
		b.WriteString("</" + t + g.pick([]string{">", " >", "\n>", "\t>"}))
	}
}

func (g *rtGen) xmlDocument() string {
	var b strings.Builder
	if g.chance(2) {
		b.WriteString(g.pick([]string{"<?xml version=\"1.0\" encoding=\"utf-8\" standalone=\"yes\"?>", "<?xml version='1.0'?>", "<?xml-stylesheet href=\"a.xsl\" type=\"text/xsl\"?>", "\ufeff<?xml version=\"1.0\"?>", "<?xml ?>", "  <?xml version=\"1.0\"?>"}))
		b.WriteString(g.pick(rtWS))
	}
	if g.chance(5) {
		b.WriteString(g.pick([]string{"<!DOCTYPE rss [ <!ENTITY x \"y\"> ]>", "<!DOCTYPE html>", "<!DOCTYPE x PUBLIC \"a\" \"b\">", "<!DOCTYPE x [\n<!ELEMENT x (#PCDATA)>\n<!ATTLIST x a CDATA #IMPLIED>\n]>", "<!DOCTYPE x [ <!-- ] --> ]>"}) + g.pick(rtWS))
	}
	n := 1 + g.r.Intn(4)
	for i := 0; i < n; i++ {
		g.xmlNode(&b, 0)
		b.WriteString(g.pick(rtWS))
	}
	return b.String()
}

// ---------------------------------------------------------------- JSON

func (g *rtGen) jsonNumber() string {
	if g.chance(3) {
		return g.pick([]string{"0", "-0", "0.0", "-0.0", "0e0", "-0e-0", "1", "-1", "1.0", "1.50", "100", "1000", "1e3", "1E3", "1e+3", "1e-3", "0.001", "0.0001", "1e21", "1e20", "123456789012345678901234567890", "1.7976931348623157e308", "1e309", "5e-324", "1e-400", "-1e400", "0.1", "0.10", "10.0", "1.23456789012345678", "9007199254740993", "-9223372036854775808", "18446744073709551616", "01", "-01", "00", ".5", "-.5", "5.", "+1", "1e", "1e+", "--1", "0x10", "1_000", "Infinity", "NaN", "1.e3", "0.000001", "0.0000001", "1000000", "10000000"})
	}
	return g.number()
}

func (g *rtGen) jsonString() string {
	var b strings.Builder
	b.WriteByte('"')
	n := g.r.Intn(5)
	for i := 0; i < n; i++ {
		b.WriteString(g.pick([]string{"a", " ", "  ", "\\\"", "\\\\", "\\/", "\\b", "\\f", "\\n", "\\r", "\\t", "\\u0041", "\\u00e9", "\\u00E9", "\\uD83D\\uDE00", "\\uD800", "\\uDC00x", "\\u", "\\u12", "\\x", "\\", "é", "\u2028", "\u2029", "\x00", "\x01", "\x1f", "\xff", "\t", "\n", "</script>", "<!--", "{", "}", "[", "]", ",", ":", "'", "1.50", "-0"}))
	}
	if !g.chance(25) {
		b.WriteByte('"')
	}
	return b.String()
}

func (g *rtGen) jsonVal(depth int) string {
	k := g.r.Intn(10)
	if depth > 6 && k >= 7 {
		k = 0
	}
	switch k {
	case 0, 1, 2:
		return g.jsonNumber()
	case 3:
		return g.pick([]string{"true", "false", "null", "TRUE", "nul", "truex", "undefined"})
	case 4, 5:
		return g.jsonString()
	case 6:
		return g.pick([]string{"", ",", "]", "}", ":", "x", "'a'", "/* c */", "// c\n", "\ufeff"})
	case 7:
		var b strings.Builder
		b.WriteString("[" + g.pick(rtWS))
		n := g.r.Intn(5)
		for i := 0; i < n; i++ {
			if i > 0 {
				b.WriteString(g.pick(rtWS) + g.pick([]string{",", ",", ",,", ""}) + g.pick(rtWS))
			}
			b.WriteString(g.jsonVal(depth + 1))
		}
		if g.chance(8) {
			b.WriteString(",")
		}
		if !g.chance(20) {
			b.WriteString(g.pick(rtWS) + "]")
		}
		return b.String()
	default:
		var b strings.Builder
		b.WriteString("{" + g.pick(rtWS))
		n := g.r.Intn(5)
		for i := 0; i < n; i++ {
			if i > 0 {
				b.WriteString(g.pick(rtWS) + g.pick([]string{",", ",", ",,", ""}) + g.pick(rtWS))
			}
			key := g.jsonString()
			if g.chance(10) {
				key = g.pick([]string{"a", "1", "'k'", "null"})
			}
			b.WriteString(key + g.pick(rtWS) + g.pick([]string{":", ":", "", "::"}) + g.pick(rtWS) + g.jsonVal(depth+1))
		}
		if g.chance(8) {
			b.WriteString(",")
		}
		if !g.chance(20) {
			b.WriteString(g.pick(rtWS) + "}")
		}
		return b.String()
	}
}

func (g *rtGen) jsonDoc() string {
	if g.chance(15) {
		// deep nesting
		d := 1 + g.r.Intn(3000)
		open := g.pick([]string{"[", "{\"a\":", "[1,"})
		cl := map[string]string{"[": "]", "{\"a\":": "}", "[1,": "]"}[open]
		s := strings.Repeat(open, d) + g.jsonNumber() + strings.Repeat(cl, d-g.r.Intn(2))
		return s
	}
	s := g.pick(rtWS) + g.jsonVal(0) + g.pick(rtWS)
	if g.chance(10) {
		s += g.jsonVal(0)
	}
	return s
}

// ---------------------------------------------------------------- byte damage

var rtBadBytes = []string{"\x00", "\x00\x00", "\x80", "\xbf", "\xc0\x80", "\xc1\xbf", "\xc2", "\xe0\x80\x80", "\xe2\x82", "\xed\xa0\x80", "\xed\xbf\xbf", "\xef\xbf\xbe", "\xf0\x80\x80\x80", "\xf4\x90\x80\x80", "\xf5", "\xff", "\xfe\xff", "\xef\xbb\xbf", "\r", "\r\n", "\n\r", "\f", "\v", "\u00a0", "\u2028", "\u0085"}

// damage applies byte-level corruption: invalid UTF-8, NUL, CR/CRLF
// conversion, or a random truncation.
func (g *rtGen) damage(b []byte) []byte {
	switch g.r.Intn(6) {
	case 0: // CRLF / CR conversion
		s := string(b)
		switch g.r.Intn(3) {
		case 0:
			s = strings.ReplaceAll(s, "\n", "\r\n")
		case 1:
			s = strings.ReplaceAll(s, "\n", "\r")
		default:
			s = strings.ReplaceAll(s, " ", "\r\n")
		}
		return []byte(s)
	case 1: // truncation
		if len(b) > 0 {
			return cp(b[:g.r.Intn(len(b)+1)])
		}
		return b
	}
	n := 1 + g.r.Intn(4)
	for k := 0; k < n; k++ {
		pos := 0
		if len(b) > 0 {
			pos = g.r.Intn(len(b) + 1)
		}
		ins := g.pick(rtBadBytes)
		if g.chance(3) && pos < len(b) {
			// overwrite
			nb := append(cp(b[:pos]), ins...)
			if pos+len(ins) < len(b) {
				nb = append(nb, b[pos+len(ins):]...)
			}
			b = nb
		} else {
			b = append(cp(b[:pos]), append([]byte(ins), b[pos:]...)...)
		}
	}
	return b
}

// ---------------------------------------------------------------- driver

var rtMediatypes = map[string][]string{
	"html": {"text/html", "text/html", "text/html; charset=utf-8"},
	"css":  {"text/css", "text/css;inline=1", "text/css; charset=utf-8"},
	"svg":  {"image/svg+xml", "image/svg+xml;inline=1"},
	"xml":  {"application/xml", "application/rss+xml", "application/atom+xml", "text/xml"},
	"json": {"application/json", "application/ld+json", "text/json", "application/manifest+json", "application/x-json"},
}

func (g *rtGen) doc(kind string) (string, []byte) {
	mt := g.pick(rtMediatypes[kind])
	switch kind {
	case "html":
		return mt, []byte(g.htmlDoc())
	case "css":
		if strings.Contains(mt, "inline") {
			return mt, []byte(g.cssDeclList())
		}
		return mt, []byte(g.cssDoc())
	case "svg":
		return mt, []byte(g.svgDocument())
	case "xml":
		if g.chance(4) {
			return mt, []byte(g.svgDocument())
		}
		return mt, []byte(g.xmlDocument())
	default:
		return mt, []byte(g.jsonDoc())
	}
}

var rtKinds = []string{"html", "css", "svg", "xml", "json"}

func genRedTeam(kind string, n int, seed int64, out string) {
	g := newRTGen(seed)
	name := "rt-" + kind
	if kind == "regress" {
		name = "redteam" // the checked-in fixture
	}
	w := newFix(out, name)
	switch kind {
	case "html", "css", "svg", "xml", "json":
		for i := 0; i < n; i++ {
			mt, in := g.doc(kind)
			g.rec(w, mt, in)
		}
	case "bytes":
		// generated documents and upstream literals with byte damage
		var lits [][]byte
		for _, p := range []string{"", "html", "css", "svg", "xml", "json"} {
			lits = append(lits, testLiterals(p)...)
		}
		for i := 0; i < n; i++ {
			kind := g.pick(rtKinds)
			mt, in := g.doc(kind)
			if g.chance(3) {
				in = cp(lits[g.r.Intn(len(lits))])
				if g.chance(2) {
					mt = g.pick(rtMediatypes[g.pick(rtKinds)])
				}
			}
			g.rec(w, mt, g.damage(in))
		}
	case "trunc":
		// every prefix of generated documents and upstream literals
		var lits [][]byte
		for _, p := range []string{"html", "css", "svg", "xml", "json"} {
			lits = append(lits, testLiterals(p)...)
		}
		for cnt := 0; cnt < n; {
			kind := g.pick(rtKinds)
			mt, in := g.doc(kind)
			if g.chance(2) {
				in = lits[g.r.Intn(len(lits))]
			}
			if len(in) > 400 {
				in = in[:400]
			}
			cfg := g.cfgName()
			m := g.m(cfg)
			for k := 0; k <= len(in); k++ {
				minRecSafe(w, cfg, m, mt, cp(in[:k]))
				cnt++
			}
		}
	case "big":
		// large documents: many generated nodes concatenated
		for i := 0; i < n; i++ {
			kind := g.pick(rtKinds)
			mt := g.pick(rtMediatypes[kind])
			target := 64<<10 + g.r.Intn(2<<20)
			var b strings.Builder
			for b.Len() < target {
				switch kind {
				case "html":
					g.htmlNode(&b, 0)
				case "css":
					if strings.Contains(mt, "inline") {
						b.WriteString(g.cssDeclList() + ";")
					} else {
						b.WriteString(g.cssRule(0))
					}
				case "svg":
					if b.Len() == 0 {
						b.WriteString("<svg xmlns=\"http://www.w3.org/2000/svg\">")
					}
					g.svgNode(&b, 1)
				case "xml":
					g.xmlNode(&b, 0)
				default:
					if b.Len() == 0 {
						b.WriteString("[")
					} else {
						b.WriteString(",")
					}
					b.WriteString(g.jsonVal(0))
				}
			}
			in := []byte(b.String())
			if g.chance(3) {
				in = g.damage(in)
			}
			g.rec(w, mt, in)
		}
	case "hugo":
		// realistic Hugo-rendered pages through neohugo's configuration
		cfgs := []string{"seeksnack", "seeksnack", "dummyjs", "errjs", "html-special"}
		for i := 0; i < n; i++ {
			cfg := cfgs[i%len(cfgs)]
			minRecSafe(w, cfg, g.m(cfg), "text/html", []byte(g.hugoPage()))
		}
	case "refuzz":
		// byte damage and truncation of the inputs of the checked-in
		// fixtures (they include windows of the site corpora); files from
		// RT_INPUTS (comma-separated) or the crate's fuzz/literals/structured
		type input struct {
			mt string
			in []byte
		}
		var inputs []input
		files := []string{"crates/tdewolff-minify/tests/fixtures/fuzz.txt.gz", "crates/tdewolff-minify/tests/fixtures/literals.txt.gz", "crates/tdewolff-minify/tests/fixtures/structured.txt.gz"}
		if env := os.Getenv("RT_INPUTS"); env != "" {
			files = strings.Split(env, ",")
		}
		seen := map[string]bool{}
		eachRecord(files, func(_ string, r []string) {
			if r[0] == "min" && !seen[r[3]] {
				seen[r[3]] = true
				inputs = append(inputs, input{string(unhexStr(r[2])), unhexStr(r[3])})
			}
		})
		for i := 0; i < n; i++ {
			x := inputs[g.r.Intn(len(inputs))]
			mt := x.mt
			if g.chance(4) {
				mt = g.pick(rtMediatypes[g.pick(rtKinds)])
			}
			g.rec(w, mt, g.damage(cp(x.in)))
		}
	case "units":
		g.genUnits(w, n)
	case "regress":
		genRegress(w, n)
		w.close()
		w = newFix(out, "redteam-nested")
		genRegressNested(w, n/30)
	default:
		panic("unknown rt kind " + kind)
	}
	w.close()
}

// numLit returns a numeric literal biased to rounding, carry and exponent
// overflow boundaries.
func (g *rtGen) numLit() string {
	var b strings.Builder
	if g.chance(3) {
		b.WriteByte("+-"[g.r.Intn(2)])
	}
	digits := func(n int) {
		run := byte("09"[g.r.Intn(2)])
		for i := 0; i < n; i++ {
			switch {
			case g.chance(3):
				b.WriteByte(run)
			case g.chance(4):
				b.WriteByte('5')
			default:
				b.WriteByte(byte('0' + g.r.Intn(10)))
			}
		}
	}
	il := g.r.Intn(8)
	if g.chance(8) {
		il = 14 + g.r.Intn(8) // around float64 precision
	}
	if g.chance(20) {
		il = 30 + g.r.Intn(300)
	}
	if g.chance(4) {
		b.WriteString(strings.Repeat("0", 1+g.r.Intn(4)))
	}
	digits(il)
	if il == 0 || g.chance(2) {
		b.WriteByte('.')
		fl := g.r.Intn(8)
		if g.chance(6) {
			fl = 14 + g.r.Intn(8)
		}
		if g.chance(4) {
			b.WriteString(strings.Repeat("0", 1+g.r.Intn(8)))
		}
		digits(fl)
	}
	if g.chance(3) {
		b.WriteByte("eE"[g.r.Intn(2)])
		switch g.r.Intn(3) {
		case 0:
			b.WriteByte('-')
		case 1:
			b.WriteByte('+')
		}
		switch g.r.Intn(6) {
		case 0:
			b.WriteString(g.pick([]string{"9223372036854775807", "9223372036854775808", "9223372036854775806", "9223372036854775800", "9223372036854775790", "922337203685477580", "99999999999999999999", "2147483647", "2147483648", "4294967296", "0", "00", "1", "09"}))
		default:
			b.WriteString(strconv.Itoa(g.r.Intn(400)))
		}
	}
	return b.String()
}

func (g *rtGen) genUnits(w *fixWriter, n int) {
	for i := 0; i < n; i++ {
		// Number / Decimal at every precision
		in := []byte(g.numLit())
		if g.chance(10) {
			in = g.damage(in)
		}
		for _, p := range []int{-1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 15, 16, 17, 20, g.r.Intn(400)} {
			buf := cp(in)
			out := minify.Number(buf, p)
			w.rec("num", strconv.Itoa(p), hx(in), sameOr(out, in), sameOr(buf, in))
			if p < 8 {
				buf = cp(in)
				out = minify.Decimal(buf, p)
				w.rec("dec", strconv.Itoa(p), hx(in), sameOr(out, in), sameOr(buf, in))
			}
		}
		// Mediatype, including strings past the 1024-byte ToLower limit
		if i%4 == 0 {
			var b strings.Builder
			k := 1 + g.r.Intn(8)
			for j := 0; j < k; j++ {
				switch g.r.Intn(6) {
				case 0:
					b.WriteString(g.pick(rtWS))
				case 1:
					b.WriteString("\"" + g.randCase(g.pick([]string{"A B", "", " ", "Q", "a\"", "x;y=Z"})) + "\"")
				case 2:
					b.WriteString(strings.Repeat(g.pick([]string{"A", "a", " ", "B;", "\t"}), 1000+g.r.Intn(60)))
				default:
					b.WriteString(g.randCase(g.pick([]string{"text/html", ";", "charset=UTF-8", "=", "/", "x", "Image/SVG+XML", "a=\"B\""})))
				}
			}
			mt := []byte(b.String())
			buf := cp(mt)
			out := minify.Mediatype(buf)
			w.rec("mt", hx(mt), sameOr(out, mt), sameOr(buf, mt))
		}
		// DataURI
		if i%2 == 0 {
			in := []byte(g.dataURI())
			if g.chance(4) {
				in = g.damage(in)
			}
			for _, cfg := range []string{"seeksnack", "t-datauri", "default"} {
				buf := cp(in)
				out := minify.DataURI(g.m(cfg), buf)
				w.rec("duri", cfg, hx(in), sameOr(out, in), sameOr(buf, in))
			}
		}
		// path data
		pd := []byte(g.pathData())
		for _, p := range []int{0, 1, 2, 3, 4, 6, 15} {
			buf := cp(pd)
			out := svg.NewPathData(&svg.Minifier{Precision: p}).ShortenPathData(buf)
			w.rec("path", strconv.Itoa(p), hx(pd), sameOr(out, pd), sameOr(buf, pd))
		}
	}
}

// genRegress writes the checked-in red-team regression records
// (tests/fixtures/redteam.txt.gz): inputs that exposed divergences of the
// Rust port. n is the nesting depth of the deep cases (the fixture uses
// 100000; Go handles ~1M, the recursive Rust port overflowed its stack at
// ~30k on an 8 MB thread).
func genRegress(w *fixWriter, n int) {
	rep := strings.Repeat
	deep := func(f, inner string) string { return rep(f, n) + inner + rep(")", n) }
	cases := []struct{ mt, in string }{
		// css: parseFunction/writeFunction recursion, Token clone/drop/Equal
		{"text/css", "a{b:" + deep("f(", "1") + "}"},
		{"text/css", "a{b:" + rep("f(", n)},
		{"text/css", "a{width:" + deep("calc(", "1px") + "}"},
		{"text/css", "a{color:" + deep("rgb(", "") + "}"},
		{"text/css", "a{margin:" + deep("f(", "1") + " " + deep("f(", "1") + "}"},
		{"text/css", "a{padding:" + deep("f(", "1") + " " + deep("f(", "2") + " " + deep("f(", "1") + "}"},
		{"text/css", "a{font:" + deep("f(", "") + " 1px a}"},
		{"text/css", "a{background:url(x) " + deep("f(", "0") + " no-repeat}"},
		{"text/css", "a{b:" + deep("f(", "1") + "!important}"},
		{"text/css;inline=1", "b:" + deep("f(", "a")},
		{"text/html", "<p style=\"b:" + deep("f(", "") + "\">x</p>"},
		{"text/html", "<style>a{b:" + deep("f(", "") + "}</style>"},
		{"image/svg+xml", "<svg><style>a{b:" + deep("f(", "") + "}</style><g style=\"c:" + deep("f(", "") + "\"/></svg>"},
		// nesting that other minifiers must also survive
		{"application/json", rep("[", n) + rep("]", n)},
		{"application/xml", rep("<a>", n) + rep("</a>", n)},
		{"image/svg+xml", "<svg>" + rep("<g>", n) + rep("</g>", n) + "</svg>"},
		{"text/html", rep("<div>", n) + rep("</div>", n)},
		// css.go:748 slices data[1:len(data)-1] of a 1-byte unterminated
		// string in local(): Go panics (recorded as "!"), the port must too;
		// and the 2-byte case that Go turns into local()
		{"text/css;inline=1", "url:local('"},
		{"text/css", "a{url:local(\""},
		{"text/html", "<p style=\"url:local('\">x"},
		{"image/svg+xml", "<svg style=\"url:local('\"/>"},
		{"text/css", "a{url:local('x"},
		{"text/css", "a{url:local(')}"},
	}
	for _, cfg := range []string{"seeksnack", "default", "css2-prec3"} {
		m := configByName(cfg)
		for _, c := range cases {
			minRecSafe(w, cfg, m, c.mt, []byte(c.in))
		}
	}
}

// genRegressNested writes nested minifier calls n deep (the fixture uses
// 3333): iframe content and raw-text elements with an HTML type are
// minified by a recursive M call per level, as in Go, so the Rust port
// needs about 2.2 KB of stack per level (release) and the test runs on a
// 1 GiB stack.
func genRegressNested(w *fixWriter, n int) {
	rep := strings.Repeat
	cases := []struct{ mt, in string }{
		{"text/html", rep("<iframe>", n) + "x"},
		{"text/html", rep("<iframe> ", n) + "<p> a </p>" + rep("</iframe>", n)},
		{"text/html", rep("<script type=text/html>", n) + "<b> x </b>"},
		{"text/html", rep("<style type=\"text/html\">", n) + "x"},
		{"text/html", rep("<iframe><svg><style>a{b:c}</style>", n) + "x"},
	}
	for _, cfg := range []string{"seeksnack", "default"} {
		m := configByName(cfg)
		for _, c := range cases {
			minRecSafe(w, cfg, m, c.mt, []byte(c.in))
		}
	}
}

// ---------------------------------------------------------------- Hugo-like pages

var rtWords = []string{"Seek", "snack", "chips", "Lay's", "crispy", "salt", "5'-guanylate", "INS 322(i)", "sugar", "100g", "3.5%", "E621", "&", "–", "é", "ข้าว", "โพด", "(new)", "#1", "a/b", "x_y", "\"quoted\"", "'single'", "<3", "a&b"}

func (g *rtGen) words(n int) string {
	var b strings.Builder
	for i := 0; i < n; i++ {
		if i > 0 {
			b.WriteString(g.pick([]string{" ", " ", " ", "  ", "\n    ", ", "}))
		}
		w := g.pick(rtWords)
		if g.chance(6) {
			w = g.pick([]string{"&amp;", "&lt;", "&gt;", "&quot;", "&#39;", "&nbsp;", "&copy;", "&hellip;", "&ndash;", "&#8217;", "&rsquo;"})
		}
		b.WriteString(w)
	}
	return b.String()
}

func (g *rtGen) indent(d int) string {
	if g.chance(5) {
		return ""
	}
	return "\n" + strings.Repeat(g.pick([]string{"  ", "\t", "    "}), d)
}

func (g *rtGen) hugoInline() string {
	switch g.r.Intn(9) {
	case 0:
		return "<a href=\"" + g.pick([]string{"https://seeksnack.com/", "/th/", "/tags/salt/", "#top", "https://example.com/a?b=1&amp;c=2", "HTTPS://Example.com/", " /x/ "}) + "\"" + g.pick([]string{"", " class=\"link\"", " rel=\"noopener noreferrer\" target=\"_blank\"", " title=\"" + g.words(2) + "\"", " aria-label=\"x\""}) + ">" + g.words(1+g.r.Intn(3)) + "</a>"
	case 1:
		// mismatched end tag
		return "<" + g.pick([]string{"strong", "em", "b", "i", "small", "code"}) + ">" + g.words(1+g.r.Intn(3)) + "</span>"
	case 2:
		t := g.pick([]string{"strong", "em", "b", "i", "span", "small", "code", "abbr", "sup", "sub", "mark", "q"})
		return "<" + t + g.pick([]string{"", " class=\"x\""}) + ">" + g.words(1+g.r.Intn(3)) + "</" + t + ">"
	case 3:
		return "<i class=\"fa " + g.pick([]string{"fa-home", "fa-search"}) + "\"></i>"
	case 4:
		return "<img src=\"" + g.pick([]string{"/images/a_hu_123.webp", "data:image/svg+xml;charset=utf-8,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'%3E%3Cpath d='M 0 0 L 16 16'/%3E%3C/svg%3E", "https://seeksnack.com/x.jpg"}) + "\" alt=\"" + g.words(2) + "\"" + g.pick([]string{"", " width=\"640\" height=\"480\"", " loading=\"lazy\" decoding=\"async\"", " srcset=\"/a-320.webp 320w, /a-640.webp 640w\" sizes=\"(max-width: 640px) 100vw, 640px\""}) + ">"
	case 5:
		return "<br>"
	case 6:
		return "<svg class=\"icon\" width=\"24\" height=\"24\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\"><path d=\"" + g.pathData() + "\"/></svg>"
	case 7:
		return "<input type=\"" + g.pick([]string{"text", "search", "checkbox", "radio"}) + "\" name=\"q\" value=\"" + g.pick([]string{"", "on", "x"}) + "\"" + g.pick([]string{"", " placeholder=\"" + g.words(2) + "\"", " required"}) + ">"
	default:
		return g.words(1 + g.r.Intn(8))
	}
}

func (g *rtGen) hugoBlock(b *strings.Builder, d int) {
	b.WriteString(g.indent(d))
	switch g.r.Intn(12) {
	case 0, 1, 2:
		b.WriteString("<p" + g.pick([]string{"", " class=\"lead\""}) + ">")
		n := 1 + g.r.Intn(5)
		for i := 0; i < n; i++ {
			if g.chance(3) {
				b.WriteString(g.pick([]string{" ", "\n", "", "  "}))
			}
			b.WriteString(g.hugoInline())
		}
		if !g.chance(8) {
			b.WriteString("</p>")
		}
	case 3:
		t := g.pick([]string{"ul", "ol"})
		b.WriteString("<" + t + g.pick([]string{"", " class=\"menu\""}) + ">")
		n := 1 + g.r.Intn(5)
		for i := 0; i < n; i++ {
			b.WriteString(g.indent(d+1) + "<li" + g.pick([]string{"", " class=\"active\""}) + ">" + g.hugoInline())
			if g.chance(2) {
				b.WriteString("</li>")
			}
		}
		b.WriteString(g.indent(d) + "</" + t + ">")
	case 4:
		h := strconv.Itoa(1 + g.r.Intn(6))
		b.WriteString("<h" + h + g.pick([]string{"", " id=\"title\""}) + ">" + g.words(1+g.r.Intn(4)) + "</h" + h + ">")
	case 5:
		b.WriteString("<table>" + g.indent(d+1) + "<thead><tr><th>" + g.words(1) + "</th><th>" + g.words(1) + "</th></tr></thead>" + g.indent(d+1) + "<tbody>")
		n := 1 + g.r.Intn(4)
		for i := 0; i < n; i++ {
			b.WriteString(g.indent(d+2) + "<tr>" + g.indent(d+3) + "<td>" + g.words(1+g.r.Intn(2)) + "</td>" + g.indent(d+3) + "<td>" + g.pick([]string{"1.50 g", "0", "12%", "&ndash;"}) + "</td>" + g.indent(d+2) + "</tr>")
		}
		b.WriteString(g.indent(d+1) + "</tbody>" + g.indent(d) + "</table>")
	case 6:
		b.WriteString("<pre><code class=\"language-go\">" + g.pick([]string{"func main() {\n\tfmt.Println(\"hi\")\n}\n", "  a  <  b\n", "x &amp;&amp; y"}) + "</code></pre>")
	case 7:
		b.WriteString("<figure>" + g.indent(d+1) + g.hugoInline() + g.indent(d+1) + "<figcaption>" + g.words(3) + "</figcaption>" + g.indent(d) + "</figure>")
	case 8:
		b.WriteString("<blockquote>" + g.indent(d+1) + "<p>" + g.words(5) + "</p>" + g.indent(d) + "</blockquote>")
	case 9:
		b.WriteString("<div class=\"" + g.pick([]string{"container", "row col-md-6", "card"}) + "\"" + g.pick([]string{"", " style=\"" + strings.ReplaceAll(g.cssDeclList(), "\"", "'") + "\"", " data-id=\"42\"", " hidden"}) + ">")
		if d < 5 {
			n := 1 + g.r.Intn(3)
			for i := 0; i < n; i++ {
				g.hugoBlock(b, d+1)
			}
		}
		b.WriteString(g.indent(d) + "</div>")
	case 10:
		b.WriteString("<form action=\"/search/\" method=\"get\">" + g.hugoInline() + "<button type=\"submit\">" + g.words(1) + "</button></form>")
	default:
		b.WriteString("<!-- " + g.words(2) + " -->" + g.pick([]string{"", "<hr>", "<nav aria-label=\"breadcrumb\"><ol><li><a href=\"/\">Home</a></li><li>" + g.words(1) + "</li></ol></nav>"}))
	}
}

// hugoPage returns a well-formed page shaped like neohugo's seeksnack output.
func (g *rtGen) hugoPage() string {
	var b strings.Builder
	b.WriteString("<!doctype html>\n<html lang=\"" + g.pick([]string{"en", "th"}) + "\"" + g.pick([]string{"", " dir=\"ltr\""}) + ">\n<head>")
	b.WriteString(g.indent(1) + "<meta charset=\"utf-8\">")
	b.WriteString(g.indent(1) + "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">")
	b.WriteString(g.indent(1) + "<title>" + g.words(3) + " | Seeksnack</title>")
	b.WriteString(g.indent(1) + "<meta name=\"description\" content=\"" + strings.ReplaceAll(g.words(6), "\"", "&quot;") + "\">")
	b.WriteString(g.indent(1) + "<meta property=\"og:title\" content=\"" + strings.ReplaceAll(g.words(3), "\"", "&quot;") + "\">")
	b.WriteString(g.indent(1) + "<link rel=\"canonical\" href=\"https://seeksnack.com/" + g.pick([]string{"", "th/", "x/"}) + "\">")
	b.WriteString(g.indent(1) + "<link rel=\"alternate\" hreflang=\"th\" href=\"https://seeksnack.com/th/\">")
	if g.chance(2) {
		b.WriteString(g.indent(1) + "<style>" + g.cssDoc() + "</style>")
	}
	b.WriteString(g.indent(1) + "<script type=\"application/ld+json\">" + g.pick([]string{"\n", ""}) + "{\n  \"@context\": \"https://schema.org\",\n  \"@type\": \"Product\",\n  \"name\": \"" + strings.ReplaceAll(g.words(2), "\"", "") + "\",\n  \"aggregateRating\": {\"ratingValue\": " + g.pick([]string{"4.50", "4", "0.0", "1e1", "-0"}) + ", \"reviewCount\": 10}\n}" + g.pick([]string{"\n", ""}) + "</script>")
	b.WriteString(g.indent(1) + "<script async src=\"https://www.googletagmanager.com/gtag/js?id=G-X\"></script>")
	b.WriteString(g.indent(1) + "<script>\n  window.dataLayer = window.dataLayer || [];\n  function gtag(){dataLayer.push(arguments);}\n</script>")
	b.WriteString(g.indent(1) + "<script type=\"x-tmpl-mustache\">\n  <li>{{ key }}</li>\n</script>")
	b.WriteString("\n</head>\n<body" + g.pick([]string{"", " class=\"home\""}) + ">")
	b.WriteString(g.indent(1) + "<header><nav><a href=\"/\" class=\"logo\">Seek<b>snack</b></a> <a href=\"/th/\">ไทย</a></nav></header>")
	b.WriteString(g.indent(1) + "<main>")
	n := 1 + g.r.Intn(8)
	for i := 0; i < n; i++ {
		g.hugoBlock(&b, 2)
	}
	b.WriteString(g.indent(1) + "</main>")
	b.WriteString(g.indent(1) + "<footer><p>&copy; 2026 Seeksnack &middot; <a href=\"/privacy/\">Privacy</a></p><button onclick=\"window.scrollTo(0, 0)\" style=\"display:none\">Top</button></footer>")
	b.WriteString("\n</body>\n</html>\n")
	return b.String()
}
