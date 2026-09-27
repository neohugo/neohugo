package main

import (
	"fmt"
	"math/rand"
	"os"
	"path/filepath"
	"strconv"
	"strings"

	"github.com/tdewolff/minify/v2"
	"github.com/tdewolff/minify/v2/svg"
)

// Adversarial generators (mode `adv`): grammar-shaped whole documents
// (HTML pages with head/body/inline CSS/SVG/JSON-LD, CSS stylesheets, SVG
// documents, RSS/XML feeds, JSON), large windows of the golden pages with
// mutations, and byte-level helper inputs (Number/Decimal near rounding
// and exponent boundaries, data URIs around the base64/percent-encoding
// length decision, path data). They complement fixtures.go/structured.go,
// which use small (<= 700 byte) inputs.
//
//	adv OUTDIR N SEED   # writes OUTDIR/fuzz.txt.gz (min records) and OUTDIR/units.txt.gz

type advGen struct {
	r *rand.Rand
}

func (g *advGen) pick(list []string) string { return list[g.r.Intn(len(list))] }
func (g *advGen) chance(n int) bool         { return g.r.Intn(n) == 0 }

var advSpaces = []string{" ", " ", " ", "  ", "\n", "\n  ", "\t", "\r\n", "\f", " \n\t ", "\u00a0", ""}

func (g *advGen) ws() string {
	if g.chance(3) {
		return ""
	}
	return g.pick(advSpaces)
}

// ---------------------------------------------------------------- numbers

var advDigits = "0123456789"

func (g *advGen) digits(n int) string {
	var b strings.Builder
	for i := 0; i < n; i++ {
		if g.chance(3) {
			b.WriteByte("09"[g.r.Intn(2)]) // runs of 0 and 9 hit rounding and carry paths
		} else {
			b.WriteByte(advDigits[g.r.Intn(10)])
		}
	}
	return b.String()
}

// number returns a numeric literal: sign, integer part, fraction, exponent.
func (g *advGen) number() string {
	var b strings.Builder
	switch g.r.Intn(6) {
	case 0:
		b.WriteByte('-')
	case 1:
		b.WriteByte('+')
	}
	intLen := g.r.Intn(6)
	if g.chance(12) {
		intLen = 10 + g.r.Intn(25)
	}
	b.WriteString(g.digits(intLen))
	if intLen == 0 || g.chance(2) {
		b.WriteByte('.')
		fl := 1 + g.r.Intn(6)
		if g.chance(10) {
			fl = 10 + g.r.Intn(20)
		}
		b.WriteString(g.digits(fl))
	}
	if g.chance(5) {
		b.WriteByte("eE"[g.r.Intn(2)])
		switch g.r.Intn(3) {
		case 0:
			b.WriteByte('-')
		case 1:
			b.WriteByte('+')
		}
		el := 1 + g.r.Intn(3)
		if g.chance(15) {
			el = 17 + g.r.Intn(4)
		}
		b.WriteString(g.digits(el))
	}
	return b.String()
}

// ---------------------------------------------------------------- CSS

var advUnits = []string{"px", "PX", "Px", "em", "EM", "rem", "%", "vh", "vw", "deg", "DEG", "rad", "grad", "turn", "s", "ms", "Q", "q", "cm", "mm", "in", "pt", "pc", "ex", "ch", "fr", "dpi", "dppx", "x", "hz", "khz", "vmin", "e", "e3", "px2", "\\70x"}

var advIdents = []string{"auto", "none", "normal", "bold", "bolder", "initial", "inherit", "unset", "revert", "medium", "small", "x-small", "xx-small", "large", "x-large", "xx-large", "smaller", "larger", "currentColor", "currentcolor", "CURRENTCOLOR", "invert", "solid", "dashed", "transparent", "TRANSPARENT", "left", "right", "top", "bottom", "center", "LEFT", "Center", "repeat", "no-repeat", "repeat-x", "repeat-y", "space", "round", "scroll", "fixed", "local", "padding-box", "border-box", "content-box", "cover", "contain", "red", "Red", "black", "white", "WHITE", "navy", "darkslategray", "lightgoldenrodyellow", "aqua", "fuchsia", "yellow", "blue", "gray", "grey", "orange", "tan", "linen", "sans-serif", "serif", "monospace", "Arial", "Helvetica", "italic", "oblique", "small-caps", "block", "inline-block", "flex", "grid", "absolute", "relative", "hidden", "visible", "pointer", "ease-in-out", "linear", "infinite", "alternate", "both", "forwards", "i", "I", "a", "-webkit-box", "-moz-x", "--custom", "progid"}

var advFuncs = []string{"rgb(", "rgba(", "hsl(", "hsla(", "RGB(", "Rgba(", "HSL(", "calc(", "var(", "min(", "max(", "clamp(", "attr(", "env(", "url(", "local(", "format(", "linear-gradient(", "radial-gradient(", "translate(", "translateX(", "rotate(", "scale(", "matrix(", "cubic-bezier(", "steps(", "counter(", "rect(", "alpha(", "drop-shadow(", "blur(", "repeat(", "minmax(", "fit-content(", "image-set(", "cross-fade(", "element(", "Alpha("}

var advProps = []string{"font", "font-family", "font-weight", "font-size", "line-height", "margin", "padding", "border-width", "border", "border-top", "border-bottom", "border-left", "border-right", "outline", "background", "background-size", "background-repeat", "background-position", "background-image", "background-color", "box-shadow", "-ms-filter", "filter", "color", "border-color", "border-left-color", "border-right-color", "border-top-color", "border-bottom-color", "text-decoration-color", "text-emphasis-color", "caret-color", "outline-color", "fill", "stroke", "column-rule", "text-shadow", "text-decoration", "text-emphasis", "flex", "flex-basis", "order", "flex-grow", "flex-shrink", "unicode-range", "z-index", "counter-reset", "counter-increment", "orphans", "widows", "width", "height", "transform", "transition", "animation", "src", "url", "content", "grid-template-columns", "grid-area", "opacity", "display", "position", "top", "left", "cursor", "quotes", "Color", "BACKGROUND", "Margin", "-webkit-transition", "*zoom", "_height", "--x", "--Y", "--empty"}

func (g *advGen) hexColor() string {
	n := []int{3, 4, 6, 8, 6, 6, 8, 5, 9, 2}[g.r.Intn(10)]
	var b strings.Builder
	b.WriteByte('#')
	pal := "0123456789abcdefABCDEF"
	if g.chance(2) {
		pal = "0fF"
	}
	prev := byte('0')
	for i := 0; i < n; i++ {
		c := pal[g.r.Intn(len(pal))]
		if i%2 == 1 && g.chance(2) {
			c = prev // doubled digits shorten
		}
		b.WriteByte(c)
		prev = c
	}
	return b.String()
}

func (g *advGen) cssString() string {
	q := `"'`[g.r.Intn(2)]
	parts := []string{"a", "Helvetica Neue", "b c", "  ", "x\\\ny", "\\\r\n", "\\'", "\\\"", "'", "\"", "é", "-apple-system", "Arial", "a  b", " a", "1a", "sans serif", "\\", "url(x)", ")", ";", "}", "{", "\\\n", "Segoe UI"}
	var b strings.Builder
	b.WriteByte(q)
	n := g.r.Intn(4)
	for i := 0; i < n; i++ {
		p := g.pick(parts)
		if len(p) == 1 && p[0] == q {
			p = "\\" + p
		}
		b.WriteString(p)
	}
	if !g.chance(20) {
		b.WriteByte(q)
	}
	return b.String()
}

var advSVGSnippets = []string{
	"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'><path d='M 0 0 L 10 10'/></svg>",
	"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"10px\" height=\"10.0px\"><rect width=\"100%\" fill=\"#FF0000\"/></svg>",
	"<svg xmlns='http://www.w3.org/2000/svg'><circle r='5' fill='white' stroke='#ffffff'/></svg>",
	"<svg xmlns='http://www.w3.org/2000/svg' version='1.1'><g><text x='0' y='0'>a &amp; b</text></g></svg>",
	"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0,0,100.00,100'><path fill='none' stroke='currentColor' stroke-width='2' d='M4.646 1.646a.5.5 0 0 1 .708 0l6 6a.5.5 0 0 1 0 .708l-6 6'/></svg>",
	"<svg xmlns='http://www.w3.org/2000/svg'><style>.a{fill:red}</style><path class='a' d='m0 0h10v10h-10z'/></svg>",
	"<svg xmlns='http://www.w3.org/2000/svg'><style><![CDATA[ .a { fill : #FF0000 !important } ]]></style></svg>",
	"<svg xmlns='http://www.w3.org/2000/svg'/>",
	"<svg/>",
	"x",
	"",
}

// encodeDataPayload percent-encodes (or not) a payload in several styles.
func (g *advGen) encodeDataPayload(s string) string {
	switch g.r.Intn(4) {
	case 0:
		return s
	case 1:
		var b strings.Builder
		for i := 0; i < len(s); i++ {
			c := s[i]
			if c == '<' || c == '>' || c == '#' || c == '"' || c == '%' || c == ' ' || c >= 0x80 || (g.chance(8) && c != '\'') {
				fmt.Fprintf(&b, "%%%02X", c)
			} else {
				b.WriteByte(c)
			}
		}
		return b.String()
	case 2:
		var b strings.Builder
		for i := 0; i < len(s); i++ {
			fmt.Fprintf(&b, "%%%02x", s[i])
		}
		return b.String()
	default:
		return strings.ReplaceAll(strings.ReplaceAll(s, "<", "%3C"), ">", "%3E")
	}
}

func (g *advGen) b64(s string) string {
	const enc = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
	var b strings.Builder
	src := []byte(s)
	for i := 0; i < len(src); i += 3 {
		var v uint32
		n := 0
		for k := 0; k < 3; k++ {
			v <<= 8
			if i+k < len(src) {
				v |= uint32(src[i+k])
				n++
			}
		}
		for k := 0; k < 4; k++ {
			if k <= n {
				b.WriteByte(enc[(v>>(18-6*uint(k)))&0x3F])
			} else {
				b.WriteByte('=')
			}
		}
	}
	return b.String()
}

// dataURI returns a data URI with a random mediatype and payload whose
// size puts base64Len, asciiLen and len(orig) close to each other.
func (g *advGen) dataURI() string {
	mts := []string{"", "text/plain", "text/plain;charset=us-ascii", "TEXT/PLAIN;CHARSET=US-ASCII", "text/plain;charset=utf-8", "image/svg+xml", "image/svg+xml;charset=utf-8", "image/svg+xml;charset=US-ASCII", "image/svg+xml;utf8", "text/css", "text/html", "application/json", "application/octet-stream", "image/png", "text/x", ";charset=us-ascii", "text/plain;a=b;charset=us-ascii;c=d", "text/plain;charset=us-asciix", "image/svg+xml;charset=utf-8;base64"}
	mt := g.pick(mts)
	var payload string
	switch {
	case strings.HasPrefix(mt, "image/svg"):
		payload = g.pick(advSVGSnippets)
	case strings.HasPrefix(mt, "text/css"):
		payload = g.cssDecls(3)
	case strings.HasPrefix(mt, "text/html"):
		payload = "<p class=\"a\">x  y</p>"
	case strings.HasPrefix(mt, "application/json"):
		payload = "{\"a\": 1.50, \"b\": [1, 2]}"
	default:
		pl := []string{"Hello, World!", "a", "", "  ", "#", "%", "é", "\x00\x01\x02\xff", "<>", "abc def", strings.Repeat("A", g.r.Intn(64)), strings.Repeat("%", g.r.Intn(10)), strings.Repeat("\xfe", g.r.Intn(30))}
		payload = g.pick(pl)
		if g.chance(3) {
			payload += g.pick(pl)
		}
	}
	if g.chance(4) {
		return "data:" + mt + ";base64," + g.b64(payload)
	}
	return "data:" + mt + "," + g.encodeDataPayload(payload)
}

func (g *advGen) cssFunc(depth int) string {
	f := g.pick(advFuncs)
	low := strings.ToLower(f)
	var b strings.Builder
	b.WriteString(f)
	switch low {
	case "rgb(", "rgba(", "hsl(", "hsla(":
		n := 3 + g.r.Intn(2)
		if g.chance(8) {
			n = 2 + g.r.Intn(4)
		}
		useSpace := g.chance(3)
		for i := 0; i < n; i++ {
			if i > 0 {
				if useSpace {
					if i == 3 {
						b.WriteString(g.pick([]string{" / ", "/", " "}))
					} else {
						b.WriteString(" ")
					}
				} else {
					b.WriteString(g.pick([]string{",", ", ", " ,", ","}))
				}
			}
			switch g.r.Intn(8) {
			case 0:
				b.WriteString(g.pick([]string{"0", "255", "256", "300", "-5", "128", "51", "102", "153", "204", "0.5", ".5", "1", "1.0", "360", "-120", "480", "1e2", "254.5", "0.1"}))
			case 1:
				b.WriteString(g.pick([]string{"0%", "100%", "50%", "20%", "40%", "60%", "80%", "10%", "33.3%", "120%", "-10%", "99.9995%", "0.0005%", "1e2%"}))
			case 2:
				b.WriteString(g.number())
			case 3:
				b.WriteString(g.number() + "%")
			case 4:
				b.WriteString(g.number() + g.pick(advUnits))
			case 5:
				b.WriteString(g.pick([]string{"var(--x)", "none", "calc(1 + 2)", "a"}))
			default:
				b.WriteString(fmt.Sprintf("%d", g.r.Intn(300)))
			}
		}
		if g.chance(10) {
			b.WriteString(",")
		}
	case "url(":
		b.Reset()
		u := g.pick([]string{"a.png", " b.png ", "'c d.png'", "\"e.png\"", "\"" + g.dataURI() + "\"", "'" + g.dataURI() + "'", g.dataURI(), "  ", "", "'a\\\nb'", "x)y"})
		b.WriteString("url(" + u)
	case "local(", "format(":
		b.WriteString(g.pick([]string{"\"Arial\"", "'A B'", "Arial", "\"woff2\"", "'x y'", "\"a\\\nb\""}))
	case "var(", "env(":
		b.WriteString(g.pick([]string{"--x", "--y, 1px", "--a,#FFF", "--z,  ", "safe-area-inset-top"}))
	default:
		n := 1 + g.r.Intn(3)
		for i := 0; i < n; i++ {
			if i > 0 {
				b.WriteString(g.pick([]string{",", ", ", " ", " + ", " - ", " * ", "/"}))
			}
			b.WriteString(g.cssAtom(depth + 1))
		}
	}
	if !g.chance(30) {
		b.WriteString(")")
	}
	return b.String()
}

func (g *advGen) cssAtom(depth int) string {
	k := g.r.Intn(14)
	if depth > 2 && k >= 11 {
		k = 0
	}
	switch k {
	case 0, 1:
		return g.number()
	case 2, 3:
		return g.number() + g.pick(advUnits)
	case 4:
		return g.number() + "%"
	case 5, 6:
		return g.pick(advIdents)
	case 7:
		return g.hexColor()
	case 8:
		return g.cssString()
	case 9:
		return g.pick([]string{"U+0000-00FF", "U+0100", "U+4??", "u+0-10ffff", "U+20-10", "U+FFFFFFFFFFFFFFFFF", "U+0-7F", "U+00??", "U+1F600-1F64F", "U+A5", "u+???", "U+0025-00FF", "U+0-0", "U+10-1F"})
	case 10:
		return g.pick([]string{"!important", "/", ",", "+", "*", "!", "=", "@", "\\30", "#", ";", "."})
	default:
		return g.cssFunc(depth)
	}
}

func (g *advGen) cssValue() string {
	n := 1 + g.r.Intn(6)
	var b strings.Builder
	for i := 0; i < n; i++ {
		if i > 0 {
			b.WriteString(g.pick([]string{" ", " ", ",", ", ", " / ", "/", "  ", "\n", ""}))
		}
		b.WriteString(g.cssAtom(0))
	}
	if g.chance(8) {
		b.WriteString(g.pick([]string{" !important", "!important", "! important", " !IMPORTANT", "!ie"}))
	}
	return b.String()
}

func (g *advGen) cssDecl() string {
	p := g.pick(advProps)
	if strings.HasPrefix(p, "--") {
		return p + ":" + g.pick([]string{" ", "", "  a  b  ", "{a:b}", " 1px ", "[x]", "calc( 1px )"})
	}
	var v string
	if g.chance(3) {
		// reuse the structured generator's realistic values
		v = genCSSDecl(g.r)
		if i := strings.IndexByte(v, ':'); i >= 0 {
			v = v[i+1:]
		}
	} else {
		v = g.cssValue()
	}
	return p + g.ws() + ":" + g.ws() + v
}

func (g *advGen) cssDecls(n int) string {
	var decls []string
	k := 1 + g.r.Intn(n)
	for i := 0; i < k; i++ {
		decls = append(decls, g.cssDecl())
	}
	s := strings.Join(decls, ";"+g.ws())
	if g.chance(2) {
		s += ";"
	}
	return s
}

var advSelectors = []string{"a", "DIV", "div.Foo", ".Bar", "#Id", "a:hover", "a::before", "A:NOT(.x)", "ul>li", "p+p", "p~P", "a[href]", "a[href=\"x\"]", "a[href='x y']", "a[data-x=\"a-b\"]", "a[x=\"1a\"]", "a[x=y i]", "a[x=\"y\" I]", "*", "html", "BODY", ":root", "input[type=text]", "h1,h2", "svg|a", "a\\:b", ".a.b", "td:nth-child(2n+1)", "[lang|=en]", "a[href$=\".pdf\"]"}

func (g *advGen) cssRule(depth int) string {
	switch g.r.Intn(12) {
	case 0:
		if depth < 2 {
			var b strings.Builder
			b.WriteString(g.pick([]string{"@media (min-width:1px)", "@media screen and (max-width: 100px)", "@supports (display:grid)", "@MEDIA print", "@media all", "@document url(x)", "@layer a"}))
			b.WriteString(g.ws() + "{")
			n := 1 + g.r.Intn(3)
			for i := 0; i < n; i++ {
				b.WriteString(g.cssRule(depth + 1))
			}
			if !g.chance(20) {
				b.WriteString("}")
			}
			return b.String()
		}
	case 1:
		return "@font-face{font-family:" + g.cssString() + ";src:" + g.pick([]string{"url(a.woff2) format(\"woff2\")", "local(\"Arial\"),url( 'b c.woff' )", "url(\"" + g.dataURI() + "\")"}) + ";unicode-range:" + g.pick([]string{"U+0000-00FF,U+0131,U+0152-0153", "U+4??", "u+0-10ffff", "U+20-7E, U+A0-FF", "U+0-7F,U+0-FF", "U+30-39,U+31"}) + "}"
	case 2:
		return g.pick([]string{"@charset \"utf-8\";", "@import url(x.css);", "@import url( x.css ) screen;", "@import url(  );", "@import 'y.css';", "@import url(\"z.css\");", "@import url( 'q.css' );", "@import url(\n a b \n);", "@namespace svg url(http://www.w3.org/2000/svg);"})
	case 3:
		return "@keyframes a{0%{" + g.cssDecls(2) + "}50.0%{" + g.cssDecls(2) + "}to{" + g.cssDecls(1) + "}}"
	case 4:
		return g.pick([]string{"/*! keep  this   comment */", "/* drop */", "/*!*/", "/*! a\n  b */", "<!--", "-->"})
	case 5:
		return g.pick([]string{"a{", "}", "a{b}", "a{:b}", "a{b:}", "a{;;}", "@x;", "{a:b}", "a{b:c d{e}}", "a{b:c;d:(e}", "a{b:[c]}", "\\", "a{b:c\"d}"})
	}
	n := 1 + g.r.Intn(3)
	var sels []string
	for i := 0; i < n; i++ {
		sels = append(sels, g.pick(advSelectors))
	}
	return strings.Join(sels, ","+g.ws()) + g.ws() + "{" + g.ws() + g.cssDecls(4) + g.ws() + "}"
}

func (g *advGen) cssSheet() string {
	var b strings.Builder
	n := 1 + g.r.Intn(6)
	for i := 0; i < n; i++ {
		b.WriteString(g.cssRule(0))
		b.WriteString(g.ws())
	}
	return b.String()
}

// ---------------------------------------------------------------- SVG

var advSVGElems = []string{"g", "path", "rect", "circle", "ellipse", "line", "polyline", "polygon", "text", "tspan", "use", "defs", "symbol", "linearGradient", "stop", "clipPath", "mask", "title", "desc", "metadata", "foreignObject", "style", "svg", "svg:g", "svg:rect", "inkscape:grid", "sodipodi:namedview", "a", "image", "switch"}

func (g *advGen) svgAttr() string {
	names := []string{"x", "y", "width", "height", "rx", "r", "cx", "cy", "viewBox", "d", "fill", "stroke", "stop-color", "flood-color", "lighting-color", "color", "style", "transform", "version", "preserveAspectRatio", "baseProfile", "contentStyleType", "contentScriptType", "xml:space", "xmlns", "xmlns:xlink", "xmlns:svg", "xmlns:inkscape", "xlink:href", "href", "inkscape:label", "sodipodi:docname", "class", "id", "stroke-width", "opacity", "points", "type", "offset", "font-size"}
	name := g.pick(names)
	var val string
	switch name {
	case "d":
		val = genPath(g.r)
	case "viewBox":
		val = g.pick([]string{"0 0 100 100", "0,0,100,100", "0 0 100.00 1e2", " 0 0 10 10", "0 0 10", "0  0 10 10", "-0.50 -0.5 24.000 24", "0 0 10PX 10", "0 0 10 10 10", "a b c d", "0,0 10,10", "0 0 010.0 0010"})
	case "fill", "stroke", "stop-color", "flood-color", "lighting-color", "color":
		val = g.pick([]string{"#FFFFFF", "#ffffff", "#FF0000", "#f00", "#aabbcc", "#AABBCC", "white", "WHITE", "red", "none", "currentColor", "url(#a)", "URL(#b)", "url(", "#12345", "", "black", "#000000", "rgb(255,0,0)", "lightgoldenrodyellow"})
	case "style":
		val = g.cssDecls(3)
	case "version":
		val = g.pick([]string{"1.1", "1.10", "1.0", "1.1px"})
	case "preserveAspectRatio":
		val = g.pick([]string{"xMidYMid meet", "none", "xMidYMid  meet"})
	case "baseProfile":
		val = g.pick([]string{"none", "full", "tiny"})
	case "contentStyleType":
		val = g.pick([]string{"text/css", "TEXT/CSS", " text/css ", "text/x"})
	case "contentScriptType":
		val = g.pick([]string{"application/ecmascript", "text/javascript"})
	case "xml:space":
		val = g.pick([]string{"preserve", "default"})
	case "xmlns":
		val = "http://www.w3.org/2000/svg"
	case "type":
		val = g.pick([]string{"text/css", "TEXT/CSS", "text/x"})
	default:
		switch g.r.Intn(5) {
		case 0:
			val = g.number()
		case 1:
			val = g.number() + g.pick(advUnits)
		case 2:
			val = g.pick([]string{"0", "0px", "10px", "10.50PX", "1e2", "50%", "5em", "0.0", "-0", "+1", "100%", "1 2", "a\"b", "a'b", "a&amp;b", "&quot;", "&apos;x&apos;", "#a"})
		default:
			val = g.pick([]string{"a", "b c", "translate(1 2)", "1,2 3,4", "M0 0", ""})
		}
	}
	q := g.pick([]string{"\"", "'", "\""})
	if strings.Contains(val, q) {
		q = "\""
		if strings.Contains(val, "\"") {
			q = "'"
		}
	}
	return name + "=" + q + val + q
}

func (g *advGen) svgNode(b *strings.Builder, depth int) {
	switch g.r.Intn(10) {
	case 0:
		b.WriteString(g.pick([]string{"<!-- c -->", "<!---->", "<?pi x?>", "<![CDATA[ a < b & c ]]>", "<![CDATA[]]>", " text &amp; more ", "\n  ", "&#x41;", "<!DOCTYPE svg [ <!ENTITY a \"b\"> ]>"}))
		return
	case 1:
		if depth > 0 {
			b.WriteString("<style" + g.pick([]string{"", " type=\"text/css\"", " type='TEXT/CSS'"}) + ">")
			if g.chance(2) {
				b.WriteString("<![CDATA[" + g.cssSheet() + "]]>")
			} else {
				b.WriteString(strings.ReplaceAll(g.cssSheet(), "<", ""))
			}
			b.WriteString("</style>")
			return
		}
	}
	el := g.pick(advSVGElems)
	b.WriteString("<" + el)
	na := g.r.Intn(5)
	for i := 0; i < na; i++ {
		b.WriteString(g.pick([]string{" ", "\n  ", "  "}))
		b.WriteString(g.svgAttr())
	}
	if depth > 3 || g.chance(3) {
		b.WriteString(g.pick([]string{"/>", " />", "></" + el + ">", ">  </" + el + " >"}))
		return
	}
	b.WriteString(">")
	n := g.r.Intn(4)
	for i := 0; i < n; i++ {
		g.svgNode(b, depth+1)
	}
	if !g.chance(25) {
		b.WriteString("</" + el + g.pick([]string{">", " >", ">\n"}))
	}
}

func (g *advGen) svgDoc() string {
	var b strings.Builder
	if g.chance(3) {
		b.WriteString("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>\n")
	}
	if g.chance(5) {
		b.WriteString("<!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\" \"http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd\">\n")
	}
	b.WriteString("<svg")
	na := g.r.Intn(6)
	for i := 0; i < na; i++ {
		b.WriteString(" ")
		b.WriteString(g.svgAttr())
	}
	b.WriteString(">")
	n := 1 + g.r.Intn(6)
	for i := 0; i < n; i++ {
		g.svgNode(&b, 1)
	}
	b.WriteString("</svg>")
	return b.String()
}

// ---------------------------------------------------------------- XML

func (g *advGen) xmlDoc() string {
	var b strings.Builder
	if g.chance(2) {
		b.WriteString(g.pick([]string{"<?xml version=\"1.0\" encoding=\"utf-8\" standalone=\"yes\"?>", "<?xml version='1.0'?>", "<?xml-stylesheet href=\"a.xsl\" type=\"text/xsl\"?>"}) + g.ws())
	}
	if g.chance(6) {
		b.WriteString("<!DOCTYPE rss [ <!ENTITY x \"y\"> ]>" + g.ws())
	}
	var stack []string
	n := 3 + g.r.Intn(25)
	for i := 0; i < n; i++ {
		switch g.r.Intn(9) {
		case 0, 1:
			t := g.pick([]string{"rss", "channel", "item", "title", "link", "description", "atom:link", "pubDate", "guid", "urlset", "url", "loc", "lastmod", "xhtml:link", "a", "B", "content:encoded"})
			b.WriteString("<" + t)
			na := g.r.Intn(3)
			for j := 0; j < na; j++ {
				b.WriteString(g.pick([]string{" ", "\n\t", "  "}))
				b.WriteString(g.pick([]string{"href=\"http://x/a?b=1&amp;c=2\"", "rel='self'", "type=\"application/rss+xml\"", "x=\"a &quot;b&quot; c\"", "y='a \"b\" c'", "z=\"&apos;\"", "w=unq", "v=\"a\tb\nc\"", "isPermaLink=\"false\"", "version=\"2.0\"", "xmlns:atom=\"http://www.w3.org/2005/Atom\"", "e=\"&lt;&gt;&amp;&#34;&#39;\"", "q=\"\""}))
			}
			if g.chance(4) {
				b.WriteString(g.pick([]string{"/>", " />"}))
			} else {
				b.WriteString(">")
				stack = append(stack, t)
			}
		case 2, 3:
			b.WriteString(g.pick([]string{"text", " text ", "\n\t\t", "a &amp; b", "a &lt; b &gt; c", "&apos;&quot;", "&#34;&#x27;", "Mon, 02 Jan 2006 15:04:05 -0700", "  lots   of   space  ", "é", "&nbsp;", "\u00a0", "&amp;amp;", ">", "]]>"}))
		case 4:
			b.WriteString(g.pick([]string{"<![CDATA[<p>a &amp; b</p>]]>", "<![CDATA[ plain ]]>", "<![CDATA[]]>", "<![CDATA[a]]]]><![CDATA[>b]]>", "<![CDATA[<img src=\"x\"/> text & more]]>", "<![CDATA[\n  indented\n]]>", "<![CDATA[x<y]]>", "<![CDATA[ ]]>", "<![CDATA[&amp;]]>"}))
		case 5:
			b.WriteString(g.pick([]string{"<!-- c -->", "<?pi a?>", "<!x>", "<!---->"}))
		default:
			if len(stack) > 0 {
				t := stack[len(stack)-1]
				stack = stack[:len(stack)-1]
				b.WriteString("</" + t + g.pick([]string{">", " >", "\n>"}))
			} else {
				b.WriteString(g.ws())
			}
		}
	}
	for len(stack) > 0 && !g.chance(10) {
		t := stack[len(stack)-1]
		stack = stack[:len(stack)-1]
		b.WriteString("</" + t + ">")
	}
	return b.String()
}

// ---------------------------------------------------------------- JSON

func (g *advGen) jsonValue(depth int) string {
	k := g.r.Intn(9)
	if depth > 3 && k >= 7 {
		k = 0
	}
	switch k {
	case 0, 1, 2:
		return g.number()
	case 3:
		return g.pick([]string{"true", "false", "null", "0", "-0", "-0.0", "0.0", "1e400", "-1e-400", "123456789012345678901234567890"})
	case 4, 5:
		return g.pick([]string{"\"a\"", "\"\"", "\"\\\"\"", "\"\\u00e9\"", "\"a b\"", "\"1.50\"", "\"\\/\"", "\"é\"", "\"<\\/script>\""})
	case 6:
		return g.pick([]string{"", ",", "]", "}", ":", "x", "'a'", "01", ".5", "-.5", "+1", "1.", "1e", "--1"})
	case 7:
		var b strings.Builder
		b.WriteString("[" + g.ws())
		n := g.r.Intn(5)
		for i := 0; i < n; i++ {
			if i > 0 {
				b.WriteString(g.ws() + "," + g.ws())
			}
			b.WriteString(g.jsonValue(depth + 1))
		}
		b.WriteString(g.ws() + "]")
		return b.String()
	default:
		var b strings.Builder
		b.WriteString("{" + g.ws())
		n := g.r.Intn(5)
		for i := 0; i < n; i++ {
			if i > 0 {
				b.WriteString(g.ws() + "," + g.ws())
			}
			b.WriteString(g.pick([]string{"\"a\"", "\"@type\"", "\"b c\"", "\"\""}) + g.ws() + ":" + g.ws() + g.jsonValue(depth+1))
		}
		b.WriteString(g.ws() + "}")
		return b.String()
	}
}

// ---------------------------------------------------------------- HTML

var advTextBits = []string{"text", "Hello", "  ", " ", "\n", "\n\n  ", "\t", "\r\n", "\f", "a&amp;b", "&amp;", "&lt;", "&gt;", "&quot;", "&apos;", "&#39;", "&#x27;", "&#60;", "&#x3C;", "&#x3c;", "&#62;", "&nbsp;", "&NBSP;", "&hellip;", "&middot;", "&copy;", "&COPY;", "&amp", "&ampx", "&lt", "&#128512;", "&#x1F600;", "&#0;", "&#x110000;", "&#;", "&;", "& ", "&unknownentity;", "&NotNestedGreaterGreater;", "&Aacute;", "&aacute", "&zwj;", "&thinsp;", "<", ">", "a<b", "a > b", "é", "\u00a0", "\u2003", "\u200b", "{{ .Title }}", "{{", "}}", "<?php echo 1 ?>", "<%= x %>", "x\x00y", "\xff"}

func (g *advGen) htmlText() string {
	var b strings.Builder
	n := 1 + g.r.Intn(5)
	for i := 0; i < n; i++ {
		b.WriteString(g.pick(advTextBits))
	}
	return b.String()
}

var advHTMLAttrNames = []string{"class", "CLASS", "id", "name", "href", "HREF", "src", "action", "cite", "data", "formaction", "poster", "profile", "itemid", "style", "STYLE", "onclick", "onload", "ONMOUSEOVER", "on", "one", "type", "value", "method", "enctype", "formenctype", "accept", "colspan", "rowspan", "shape", "span", "media", "dir", "title", "alt", "content", "property", "vocab", "typeof", "resource", "prefix", "about", "rev", "datatype", "inlist", "lang", "hidden", "checked", "disabled", "async", "defer", "selected", "readonly", "download", "rel", "charset", "http-equiv", "amp-boilerplate", "data-x", "aria-label", "srcset", "sizes", "width", "height", "tabindex", "for", "x", "srcdoc"}

func (g *advGen) htmlAttrValue(name string) string {
	low := strings.ToLower(name)
	switch {
	case low == "style":
		return g.cssDecls(3)
	case strings.HasPrefix(low, "on") && len(low) > 2:
		return g.pick([]string{"alert('x')", "javascript:alert(1)", "JavaScript:void(0)", "  x()  ", "", "a&amp;&amp;b", "return false;"})
	case low == "href" || low == "src" || low == "action" || low == "cite" || low == "data" || low == "formaction" || low == "poster" || low == "profile" || low == "itemid":
		return g.pick([]string{"http://example.com/a", "HTTP://Example.com/", "https://example.com/b?c=1&amp;d=2", "HTTPS://x", "Https:x", "http:", "https:", "  http://x.com  ", "/a/b", "#top", "mailto:a@b", "javascript:void(0)", g.dataURI(), " " + g.dataURI() + " ", "http", "httpx://a", "", "  ", "data:", "DATA:text/plain,a"})
	case low == "type":
		return g.pick([]string{"text/javascript", "TEXT/JAVASCRIPT", "application/javascript", "module", "text/css", "TEXT/CSS", "text", "TEXT", "submit", "radio", "RADIO", "checkbox", "application/ld+json", "application/json", "text/x-template", " text/css ", "text/css; charset=utf-8", "image/svg+xml", "text/html", "application/x-javascript", "text/ecmascript", "text/jscript"})
	case low == "value":
		return g.pick([]string{"", "on", "ON", "x", " a ", "1"})
	case low == "method":
		return g.pick([]string{"get", "GET", "post"})
	case low == "enctype" || low == "formenctype" || low == "accept":
		return g.pick([]string{"application/x-www-form-urlencoded", "APPLICATION/X-WWW-FORM-URLENCODED", "multipart/form-data", " text/plain ; charset = \"UTF-8\" ", "image/*, video/*"})
	case low == "colspan" || low == "rowspan" || low == "span":
		return g.pick([]string{"1", "one", "2", " 1 ", "01"})
	case low == "shape":
		return g.pick([]string{"rect", "RECT", "circle"})
	case low == "media":
		return g.pick([]string{"all", "ALL", "screen", " all "})
	case low == "class":
		return g.pick([]string{"a", " a  b ", "", "  ", "a\tb\nc", "x&amp;y", "\"q\"", "'q'"})
	case low == "content":
		return g.pick([]string{"width=device-width, initial-scale=1.0", "width=device-width,initial-scale=1.00,maximum-scale=1.0", "initial-scale=0.50, minimum-scale = 1.0e0", "a, b, c", "text/html; charset=utf-8", "text/html;charset=UTF-8", "TEXT/HTML; CHARSET=\"UTF-8\"", "text/html; charset=iso-8859-1", "x", "", "=1.0", "a=1.0 ,b=", "initial-scale=00.500"})
	}
	return g.htmlText()
}

func (g *advGen) htmlAttr() string {
	name := g.pick(advHTMLAttrNames)
	if g.chance(8) {
		return name
	}
	val := g.htmlAttrValue(name)
	switch g.r.Intn(6) {
	case 0:
		if !strings.ContainsAny(val, " \t\n\r\f>\"'`=<") && val != "" {
			return name + "=" + val
		}
		return name + "=\"" + strings.ReplaceAll(val, "\"", "&quot;") + "\""
	case 1:
		return name + "='" + strings.ReplaceAll(val, "'", "&#39;") + "'"
	case 2:
		return name + g.pick([]string{" = ", "=", "\n=\n"}) + "\"" + strings.ReplaceAll(val, "\"", "") + "\""
	case 3:
		return name + "=\"" + val + "\"" // may break the tag: adversarial
	default:
		return name + "=\"" + strings.ReplaceAll(val, "\"", "&quot;") + "\""
	}
}

var advHTMLTags = []string{"a", "p", "P", "div", "DIV", "span", "i", "b", "em", "strong", "pre", "textarea", "code", "ul", "ol", "li", "dl", "dt", "dd", "table", "thead", "tbody", "tfoot", "tr", "th", "td", "colgroup", "col", "caption", "select", "option", "optgroup", "datalist", "form", "input", "button", "label", "img", "br", "hr", "iframe", "video", "audio", "source", "picture", "figure", "figcaption", "header", "footer", "nav", "main", "section", "article", "aside", "h1", "h2", "h3", "blockquote", "address", "details", "summary", "template", "noscript", "object", "embed", "canvas", "map", "area", "ruby", "rb", "rt", "rtc", "rp", "del", "ins", "q", "small", "sub", "sup", "time", "wbr", "x-custom", "my-el", "font", "center", "xmp", "plaintext", "title", "math", "svg", "style", "script", "iframe"}

func (g *advGen) htmlNode(b *strings.Builder, depth int) {
	switch g.r.Intn(12) {
	case 0, 1, 2:
		b.WriteString(g.htmlText())
		return
	case 3:
		b.WriteString(g.pick([]string{"<!-- c -->", "<!--[if IE]><p>x  y</p><![endif]-->", "<!--[if lt IE 9]><script src=a.js></script><![endif]-->", "<!--[if IE]>x<![endif]-->", "<!--[if !IE]><!--><p>a</p><!--<![endif]-->", "<![endif]-->", "<!--#include virtual=\"/x\" -->", "<!--#-->", "<!---->", "<!-- a -- b -->", "<!", "<!x>", "<?xml?>", "</>", "<>", "< p>", "<p/>", "<br/>", "</br>", "</p>", "</div>", "</li>", "</option>", "</optgroup>", "</td>"}))
		return
	case 4:
		b.WriteString(g.pick([]string{"<script>", "<script type=\"application/ld+json\">", "<script type=text/javascript>", "<SCRIPT type=\"module\">", "<script type=\"text/x-template\">", "<script src=a.js charset=utf-8>", "<script type=\"application/json\">"}))
		if g.chance(3) {
			b.WriteString(g.jsonValue(0))
		} else {
			b.WriteString(g.pick([]string{"", " var x = 1 ; ", "x()", "\n  if (a < b) { c() }\n", "</scrip>", "<!-- x -->", "{{ .JS }}"}))
		}
		b.WriteString(g.pick([]string{"</script>", "</SCRIPT>", "</script >"}))
		return
	case 5:
		b.WriteString(g.pick([]string{"<style>", "<style type=\"text/css\">", "<style media=all>", "<style amp-boilerplate>", "<style type=\"TEXT/CSS\" media=\"screen\">", "<STYLE>", "<style type=text/x-scss>"}))
		b.WriteString(g.cssSheet())
		b.WriteString(g.pick([]string{"</style>", "</STYLE>", "</style >"}))
		return
	case 6:
		if depth > 0 {
			svg := g.svgDoc()
			if i := strings.Index(svg, "<svg"); i > 0 {
				svg = svg[i:]
			}
			b.WriteString(svg)
			return
		}
	case 7:
		b.WriteString(g.pick([]string{"<math><mi>x</mi><mo>=</mo><mn>1.50</mn></math>", "<math display=\"block\"><mrow> <mi> a </mi> </mrow></math>"}))
		return
	}
	tag := g.pick(advHTMLTags)
	low := strings.ToLower(tag)
	b.WriteString("<" + tag)
	na := g.r.Intn(5)
	for i := 0; i < na; i++ {
		b.WriteString(g.pick([]string{" ", " ", "\n  ", "  ", "\t"}))
		b.WriteString(g.htmlAttr())
	}
	if g.chance(10) {
		b.WriteString(g.pick([]string{"/", " /"}))
	}
	b.WriteString(g.pick([]string{">", ">", ">", " >", "\n>"}))
	switch low {
	case "script", "style", "textarea", "title", "xmp", "plaintext", "iframe", "svg", "math", "pre":
		switch low {
		case "style":
			b.WriteString(g.cssSheet())
		case "script":
			b.WriteString(g.jsonValue(0))
		default:
			b.WriteString(g.htmlText())
		}
		if !g.chance(15) {
			b.WriteString("</" + tag + ">")
		}
		return
	case "input", "img", "br", "hr", "col", "wbr", "source", "area", "embed":
		return
	}
	if depth < 5 {
		n := g.r.Intn(5)
		for i := 0; i < n; i++ {
			g.htmlNode(b, depth+1)
		}
	}
	if !g.chance(6) {
		b.WriteString(g.pick([]string{"</" + tag + ">", "</" + tag + ">", "</" + tag + " >", "</" + strings.ToUpper(tag) + ">", "</" + tag + "\n>"}))
	}
	b.WriteString(g.ws())
}

func (g *advGen) htmlPage() string {
	var b strings.Builder
	b.WriteString(g.pick([]string{"<!DOCTYPE html>", "<!doctype html>", "<!DOCTYPE HTML PUBLIC \"-//W3C//DTD HTML 4.01//EN\">", "", "<!DOCTYPE html>\n"}))
	b.WriteString(g.ws())
	b.WriteString(g.pick([]string{"<html>", "<html lang=\"en\">", "<HTML>", "", "<html prefix=\"og: http://ogp.me/ns#\">"}))
	b.WriteString(g.ws())
	b.WriteString(g.pick([]string{"<head>", "<HEAD>", "", "<head profile=\"http://x\">"}))
	nh := g.r.Intn(8)
	for i := 0; i < nh; i++ {
		b.WriteString(g.ws())
		switch g.r.Intn(8) {
		case 0:
			b.WriteString(g.pick([]string{"<meta charset=\"utf-8\">", "<meta charset=UTF-8>", "<meta http-equiv=\"Content-Type\" content=\"text/html; charset=utf-8\">", "<meta http-equiv=\" content-type \" content=\"TEXT/HTML ; CHARSET = UTF-8\">", "<meta http-equiv=\"content-type\" content=\"text/html; charset=utf-8\" charset=utf-8>", "<meta http-equiv=\"X-UA-Compatible\" content=\"IE=edge\">", "<meta http-equiv=content-type content=text/html>"}))
		case 1:
			b.WriteString("<meta name=\"" + g.pick([]string{"viewport", " VIEWPORT ", "keywords", "Keywords", "description", "robots"}) + "\" content=\"" + g.htmlAttrValue("content") + "\">")
		case 2:
			b.WriteString("<meta property=\"og:" + g.pick([]string{"title", "image", "url"}) + "\" content=\"" + g.htmlText() + "\">")
		case 3:
			b.WriteString("<title>" + g.htmlText() + "</title>")
		case 4:
			b.WriteString("<link rel=\"stylesheet\" " + g.pick([]string{"type=\"text/css\" ", "type=TEXT/CSS ", "", "media=all "}) + "href=\"" + g.htmlAttrValue("href") + "\">")
		default:
			g.htmlNode(&b, 1)
		}
	}
	b.WriteString(g.ws())
	b.WriteString(g.pick([]string{"</head>", "", "</HEAD>"}))
	b.WriteString(g.ws())
	b.WriteString(g.pick([]string{"<body>", "<body class=\"a\">", "", "<BODY onload=\"x()\">"}))
	nb := 1 + g.r.Intn(12)
	for i := 0; i < nb; i++ {
		g.htmlNode(&b, 0)
	}
	b.WriteString(g.ws())
	b.WriteString(g.pick([]string{"</body></html>", "</body>\n</html>\n", "", "</html>", "</BODY></HTML>"}))
	return b.String()
}

// ---------------------------------------------------------------- driver

var advMutDict = []string{"<", ">", "</", "/>", "=", "\"", "'", " ", "\n", "\t", "&", "&amp;", "&#", ";", "<!--", "-->", "<![CDATA[", "]]>", "<?", "?>", "{", "}", "(", ")", "[", "]", ":", ",", ".", "0", "1", "9", "-", "+", "e", "E", "%", "#", "/", "\\", "*", "!", "\x00", "\xff", "\u00a0"}

// mutateBig applies a few mutations to a (possibly large) input.
func (g *advGen) mutateBig(b []byte) []byte {
	ops := 1 + g.r.Intn(6)
	for k := 0; k < ops; k++ {
		pos := 0
		if len(b) > 0 {
			pos = g.r.Intn(len(b) + 1)
		}
		switch g.r.Intn(6) {
		case 0, 1:
			t := g.pick(advMutDict)
			b = append(b[:pos], append([]byte(t), b[pos:]...)...)
		case 2:
			if len(b) > 0 {
				n := 1 + g.r.Intn(12)
				if pos+n > len(b) {
					n = len(b) - pos
				}
				b = append(b[:pos], b[pos+n:]...)
			}
		case 3:
			if pos < len(b) {
				b[pos] = advMutDict[g.r.Intn(len(advMutDict))][0]
			}
		case 4:
			if len(b) > 0 {
				i := g.r.Intn(len(b))
				n := 1 + g.r.Intn(64)
				if i+n > len(b) {
					n = len(b) - i
				}
				d := cp(b[i : i+n])
				b = append(b[:pos], append(d, b[pos:]...)...)
			}
		default:
			c := byte(g.r.Intn(256))
			b = append(b[:pos], append([]byte{c}, b[pos:]...)...)
		}
	}
	return b
}

func loadCorpus(root string, exts []string, max int) [][]byte {
	if root == "" {
		return nil
	}
	files := walkFiles(root, exts)
	step := 1
	if len(files) > max {
		step = len(files) / max
	}
	var out [][]byte
	for i := 0; i < len(files); i += step {
		if b, err := os.ReadFile(files[i]); err == nil && len(b) > 0 {
			out = append(out, b)
		}
	}
	return out
}

func (g *advGen) window(b []byte, min, max int) []byte {
	if len(b) <= min {
		return cp(b)
	}
	n := min + g.r.Intn(max-min+1)
	if n >= len(b) {
		return cp(b)
	}
	i := g.r.Intn(len(b) - n + 1)
	return cp(b[i : i+n])
}

var advHTMLCfgs = []string{"seeksnack", "default", "html-keepall", "html-endtags", "html-special", "html-ws", "html-quotes", "html-gotmpl", "html-php", "url-http", "url-https", "css2-prec3", "prec1-inline", "dummyjs", "errjs", "t-htmlcsssvg", "xml-json-rx", "errplain"}
var advCSSCfgs = []string{"seeksnack", "default", "css2-prec3", "prec1-inline", "t-css"}
var advSVGCfgs = []string{"seeksnack", "default", "css2-prec3", "prec1-inline", "svgerr", "t-htmlcsssvg"}
var advXMLCfgs = []string{"seeksnack", "default", "css2-prec3", "prec1-inline"}

func genAdversarial(outDir string, n int, seed int64) {
	g := &advGen{rand.New(rand.NewSource(seed))}
	ms := map[string]*minify.M{}
	mOf := func(cfg string) *minify.M {
		if m, ok := ms[cfg]; ok {
			return m
		}
		m := configByName(cfg)
		ms[cfg] = m
		return m
	}
	golden := os.Getenv("SEEKSNACK_GOLDEN")
	pages := loadCorpus(golden, []string{".html"}, 200)
	feeds := loadCorpus(golden, []string{".xml"}, 60)
	var cssCorpus, svgCorpus [][]byte
	if c2 := os.Getenv("CORPUS2"); c2 != "" {
		cssCorpus = append(loadCorpus(filepath.Join(c2, "css"), []string{".in"}, 10), loadCorpus(filepath.Join(c2, "css-resource"), []string{".in"}, 10)...)
		svgCorpus = loadCorpus(filepath.Join(c2, "svg"), []string{".in"}, 10)
	}

	w := newFix(outDir, "fuzz")
	for i := 0; i < n; i++ {
		// HTML pages
		cfg := advHTMLCfgs[i%len(advHTMLCfgs)]
		minRec(w, cfg, mOf(cfg), "text/html", []byte(g.htmlPage()))
		// CSS sheets and inline declarations
		cfg = advCSSCfgs[i%len(advCSSCfgs)]
		if g.chance(3) {
			minRec(w, cfg, mOf(cfg), "text/css;inline=1", []byte(g.cssDecls(6)))
		} else {
			minRec(w, cfg, mOf(cfg), "text/css", []byte(g.cssSheet()))
		}
		// SVG documents
		cfg = advSVGCfgs[i%len(advSVGCfgs)]
		mt := "image/svg+xml"
		if g.chance(4) {
			mt = "image/svg+xml;inline=1"
		}
		minRec(w, cfg, mOf(cfg), mt, []byte(g.svgDoc()))
		// XML feeds
		cfg = advXMLCfgs[i%len(advXMLCfgs)]
		minRec(w, cfg, mOf(cfg), g.pick([]string{"application/rss+xml", "application/xml", "application/atom+xml"}), []byte(g.xmlDoc()))
		// JSON
		if i%2 == 0 {
			cfg = advXMLCfgs[(i/2)%len(advXMLCfgs)]
			minRec(w, cfg, mOf(cfg), g.pick([]string{"application/json", "application/ld+json", "text/json"}), []byte(g.jsonValue(0)))
		}
		// large windows of the golden pages / feeds, mutated
		if len(pages) > 0 && i%2 == 0 {
			p := g.window(pages[g.r.Intn(len(pages))], 1024, 24*1024)
			if !g.chance(5) {
				p = g.mutateBig(p)
			}
			cfg = advHTMLCfgs[(i/2)%len(advHTMLCfgs)]
			minRec(w, cfg, mOf(cfg), "text/html", p)
		}
		if len(feeds) > 0 && i%4 == 1 {
			p := g.window(feeds[g.r.Intn(len(feeds))], 512, 12*1024)
			if !g.chance(4) {
				p = g.mutateBig(p)
			}
			cfg = advXMLCfgs[(i/4)%len(advXMLCfgs)]
			minRec(w, cfg, mOf(cfg), "application/rss+xml", p)
		}
		if len(cssCorpus) > 0 && i%4 == 2 {
			p := g.window(cssCorpus[g.r.Intn(len(cssCorpus))], 256, 8*1024)
			p = g.mutateBig(p)
			cfg = advCSSCfgs[(i/4)%len(advCSSCfgs)]
			minRec(w, cfg, mOf(cfg), "text/css", p)
		}
		if len(svgCorpus) > 0 && i%4 == 3 {
			p := g.mutateBig(cp(svgCorpus[g.r.Intn(len(svgCorpus))]))
			cfg = advSVGCfgs[(i/4)%len(advSVGCfgs)]
			minRec(w, cfg, mOf(cfg), "image/svg+xml", p)
		}
	}
	w.close()

	// byte-level helpers
	u := newFix(outDir, "units")
	for i := 0; i < 4*n; i++ {
		in := []byte(g.number())
		if g.chance(4) {
			in = g.mutateBig(in)
		}
		if len(in) > 120 {
			in = in[:120]
		}
		for _, p := range numberPrecs {
			buf := cp(in)
			out := minify.Number(buf, p)
			u.rec("num", fmt.Sprint(p), hx(in), sameOr(out, in), sameOr(buf, in))
		}
		for _, p := range decimalPrecs {
			buf := cp(in)
			out := minify.Decimal(buf, p)
			u.rec("dec", fmt.Sprint(p), hx(in), sameOr(out, in), sameOr(buf, in))
		}
	}
	for i := 0; i < n; i++ {
		in := []byte(g.dataURI())
		if g.chance(5) {
			in = g.mutateBig(in)
		}
		for _, cfg := range []string{"seeksnack", "t-datauri", "default", "css2-prec3"} {
			buf := cp(in)
			out := minify.DataURI(mOf(cfg), buf)
			u.rec("duri", cfg, hx(in), sameOr(out, in), sameOr(buf, in))
		}
		mtIn := []byte(g.pick([]string{"text/HTML; Charset=\"UTF-8\"", " a / b ; c = \"D E\" ", "x\t;\ty", "\"A\"B\"C\"", "TEXT/css;;", ""}) + g.pick([]string{"", " ", "X", "\"q Q\""}))
		buf := cp(mtIn)
		out := minify.Mediatype(buf)
		u.rec("mt", hx(mtIn), sameOr(out, mtIn), sameOr(buf, mtIn))
	}
	for i := 0; i < n; i++ {
		in := []byte(genPath(g.r))
		if g.chance(3) {
			// random numbers of every shape as coordinates
			var b strings.Builder
			b.WriteByte("MmLlCcQqAaHhVvSsTt"[g.r.Intn(18)])
			k := 1 + g.r.Intn(14)
			for j := 0; j < k; j++ {
				if j > 0 {
					b.WriteString(g.pick([]string{" ", ",", "", " , ", "\n"}))
				}
				b.WriteString(g.number())
			}
			in = []byte(b.String())
		}
		for _, prec := range []int{0, 1, 3, 5} {
			p := svg.NewPathData(&svg.Minifier{Precision: prec})
			buf := cp(in)
			out := p.ShortenPathData(buf)
			u.rec("path", fmt.Sprint(prec), hx(in), sameOr(out, in), sameOr(buf, in))
		}
	}
	u.close()
}

// genCases writes OUTDIR/fuzz.txt.gz from a case list: one case per line,
// `MEDIATYPE<TAB>CONFIG[,CONFIG...]<TAB>INPUT` where INPUT is a Go quoted
// string literal (strconv.Unquote) and CONFIG may be `*` for every named
// configuration. Lines starting with '#' are skipped.
func genCases(outDir, listFile string) {
	data, err := os.ReadFile(listFile)
	if err != nil {
		panic(err)
	}
	w := newFix(outDir, "fuzz")
	for _, line := range strings.Split(string(data), "\n") {
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}
		f := strings.SplitN(line, "\t", 3)
		if len(f) != 3 {
			panic("bad case line: " + line)
		}
		in, err := strconv.Unquote(f[2])
		if err != nil {
			panic(fmt.Sprintf("bad case input %s: %v", f[2], err))
		}
		cfgs := strings.Split(f[1], ",")
		if f[1] == "*" {
			cfgs = append(append([]string{}, allConfigNames...), "dummyjs", "errjs", "t-htmlcsssvg", "xml-json-rx", "errplain")
		}
		for _, cfg := range cfgs {
			minRecSafe(w, cfg, configByName(cfg), f[0], []byte(in))
		}
	}
	w.close()
}
