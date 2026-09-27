package main

import (
	"fmt"
	"math/rand"
	"strings"
)

// Structured generators: grammar-shaped inputs that reach the deep
// branches of css.minifyProperty, svg path data and html attribute
// handling far more often than byte-level mutation does.

var cssProps = []string{"font", "font-family", "font-weight", "margin", "padding", "border-width", "border", "border-top", "border-left", "outline", "background", "background-size", "background-repeat", "background-position", "box-shadow", "-ms-filter", "filter", "color", "background-color", "border-color", "border-left-color", "text-decoration-color", "caret-color", "fill", "stroke", "column-rule", "text-shadow", "text-decoration", "text-emphasis", "flex", "flex-basis", "order", "flex-grow", "flex-shrink", "unicode-range", "z-index", "counter-reset", "width", "transform", "src", "url", "--var", "grid-template-columns", "content", "line-height"}

var cssVals = []string{"0", "0px", "0.0em", "1", "1px", "2px", "-0.5em", "+.50%", "100%", "50%", "0%", "10%", "20%", "40%", "60%", "80%", "1.0", "010", "1e3", "3E2px", "12PX", "0deg", "0Q", "calc(1px + 2%)", "var(--x)", "var(--x, 1px)", "min(1px,2%)", "auto", "none", "normal", "bold", "initial", "inherit", "unset", "medium", "small", "x-large", "currentColor", "invert", "solid", "transparent", "left", "right", "top", "bottom", "center", "repeat", "no-repeat", "space", "round", "repeat-x", "scroll", "padding-box", "border-box", "cover", "contain", "red", "black", "WHITE", "Navy", "darkslategray", "#FFF", "#ffffff", "#FF0000", "#aabbcc", "#AABBCCDD", "#11223344", "#000000ff", "#00000000", "#fff0", "rgb(255,0,0)", "rgb(10%,20%,40%)", "rgb(0%,0%,0%)", "rgba(0,0,0,0)", "rgba(255,255,255,1)", "rgba(1,2,3,.5)", "rgba(1,2,3,50%)", "rgb(1 2 3/50%)", "rgb(300,-5,128)", "hsl(120,100%,50%)", "hsl(-120,50%,50%)", "hsl(480,50%,50%)", "hsla(0,0%,0%,.5)", "hsla(0,0%,100%,1)", "hsl(0,50,50)", "url(a.png)", "url( 'b c.png' )", "url(\"data:image/svg+xml;charset=utf-8,%3Csvg xmlns='http://www.w3.org/2000/svg'/%3E\")", "url(data:text/plain;charset=us-ascii,%41%42)", "local(\"Arial\")", "local('A B')", "format(\"woff2\")", "\"Helvetica Neue\"", "'Arial'", "\"-apple-system\"", "\"a  b\"", "Arial", "sans-serif", "U+0000-00FF", "U+0100", "U+4??", "u+0-10ffff", "U+20-10", "U+FFFFFFFFFFFFFFFFF", "\"progid:DXImageTransform.Microsoft.Alpha(Opacity=80)\"", "progid:DXImageTransform.Microsoft.Alpha(opacity=80)", "rotate(0deg)", "translate(0px, 0%)", "scale(1.50)", "!important"}

var cssSeps = []string{" ", " ", " ", ",", ", ", " / ", "/", "  "}

func genCSSDecl(r *rand.Rand) string {
	p := cssProps[r.Intn(len(cssProps))]
	n := 1 + r.Intn(6)
	var b strings.Builder
	b.WriteString(p)
	b.WriteString(":")
	if r.Intn(4) == 0 {
		b.WriteString(" ")
	}
	for i := 0; i < n; i++ {
		if i > 0 {
			b.WriteString(cssSeps[r.Intn(len(cssSeps))])
		}
		b.WriteString(cssVals[r.Intn(len(cssVals))])
	}
	if r.Intn(8) == 0 {
		b.WriteString(" !important")
	}
	return b.String()
}

func genCSS(r *rand.Rand) (string, bool) {
	n := 1 + r.Intn(4)
	var decls []string
	for i := 0; i < n; i++ {
		decls = append(decls, genCSSDecl(r))
	}
	body := strings.Join(decls, ";")
	if r.Intn(3) == 0 {
		return body, true // inline
	}
	sel := []string{"a", "DIV.Foo", "a[href=\"x\"]", "a[x='y' i]", "#id>P+Q~r", "@media (min-width:1px){a"}[r.Intn(6)]
	s := sel + "{" + body + "}"
	if strings.HasPrefix(sel, "@media") {
		s += "}"
	}
	return s, false
}

var pathCmdsStr = "MmLlHhVvCcSsQqTtAaZz"

func genPath(r *rand.Rand) string {
	var b strings.Builder
	n := 1 + r.Intn(8)
	nums := []string{"0", "1", "10", "100", "-1", ".5", "0.5", "-.5", "1.5", "4.646", "1e2", "1.25e-1", "200.00", "0.0001", "3", "99.999"}
	for i := 0; i < n; i++ {
		c := pathCmdsStr[r.Intn(len(pathCmdsStr))]
		if i == 0 && r.Intn(3) != 0 {
			c = "Mm"[r.Intn(2)]
		}
		b.WriteByte(c)
		var k int
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
		reps := 1 + r.Intn(3)
		if r.Intn(5) == 0 {
			reps = 0
		}
		for j := 0; j < k*reps; j++ {
			if j > 0 || r.Intn(2) == 0 {
				b.WriteString([]string{" ", ",", "", " , "}[r.Intn(4)])
			}
			if (c == 'A' || c == 'a') && (j%7 == 3 || j%7 == 4) {
				b.WriteString([]string{"0", "1", "0", "1", "2"}[r.Intn(5)])
				continue
			}
			// repeat previous coordinate sometimes to trigger C->S, L->H/V etc.
			b.WriteString(nums[r.Intn(len(nums))])
		}
	}
	return b.String()
}

var htmlTags = []string{"a", "p", "div", "span", "i", "b", "pre", "textarea", "script", "style", "meta", "link", "input", "img", "br", "select", "option", "optgroup", "table", "tr", "td", "li", "ul", "template", "iframe", "svg", "button", "label", "form", "html", "head", "body", "title", "colgroup", "x-custom"}
var htmlAttrs = []string{"class=\" a  b \"", "class=\"\"", "id=x", "name=x", "id=\"\"", "href=\" HTTP://Example.com/a \"", "href=\"https://x/\"", "src=\"data:text/plain;charset=US-ASCII,Hello%20World\"", "src=\"data:image/svg+xml;charset=utf-8,%3Csvg xmlns='http://www.w3.org/2000/svg'%3E%3Cpath d='M 0 0 L 10 10'/%3E%3C/svg%3E\"", "style=\" color : RED ; \"", "style=\"\"", "style=\"background:url(x) no-repeat 0 0\"", "onclick=\"javascript:alert('x');\"", "onload=\"  \"", "type=\"text/javascript\"", "type=\"application/ld+json\"", "type=\"TEXT/CSS\"", "type=text", "type=radio", "value=\"\"", "value=on", "method=GET", "enctype=\"application/x-www-form-urlencoded\"", "colspan=1", "rowspan=\"1\"", "shape=rect", "span=1", "media=all", "async", "defer=\"defer\"", "hidden=\" hidden \"", "content=\"a, b\"", "content=\"width=device-width, initial-scale=1.00, maximum-scale=1.0\"", "content=\"text/html; charset=UTF-8\"", "http-equiv=\"Content-Type\"", "name=keywords", "name=viewport", "charset=utf-8", "property=\"og:title\"", "title='a \"b\" c'", "title=\"a 'b' c\"", "alt=\"&amp;lt; &lt; &quot; &#39; &#x27; &hellip; &nbsp;\"", "data-x=`y`", "x=a=b", "amp-boilerplate", "action=\"\"", "lang=\"EN\""}
var htmlTexts = []string{" ", "  text  ", "\n  x \n", "a&amp;b", "&lt;&gt;&quot;", "&#60;&#x3C;", "&hellip;&middot;&nbsp;", "x = 1;", "{\"a\": 1.50, \"b\" : [ 1 , 2 , ] }", "a { color : #FF0000 ; }", "<!--[if IE]><p>x</p><![endif]-->", "<!-- c -->", "<!--#ssi-->", "{{ .X }}", "\t", "é ", ""}

func genHTML(r *rand.Rand) string {
	var b strings.Builder
	n := 1 + r.Intn(8)
	var stack []string
	for i := 0; i < n; i++ {
		switch r.Intn(4) {
		case 0, 1:
			t := htmlTags[r.Intn(len(htmlTags))]
			b.WriteString("<" + t)
			na := r.Intn(4)
			for j := 0; j < na; j++ {
				b.WriteString([]string{" ", "\n  ", "  "}[r.Intn(3)])
				b.WriteString(htmlAttrs[r.Intn(len(htmlAttrs))])
			}
			if r.Intn(6) == 0 {
				b.WriteString("/")
			}
			b.WriteString(">")
			stack = append(stack, t)
		case 2:
			b.WriteString(htmlTexts[r.Intn(len(htmlTexts))])
		case 3:
			if len(stack) > 0 {
				t := stack[len(stack)-1]
				stack = stack[:len(stack)-1]
				b.WriteString("</" + t + []string{">", " >", ">\n"}[r.Intn(3)])
			}
		}
	}
	for len(stack) > 0 && r.Intn(3) != 0 {
		t := stack[len(stack)-1]
		stack = stack[:len(stack)-1]
		fmt.Fprintf(&b, "</%s>", t)
	}
	return b.String()
}

// genStructured writes n structured cases per generator.
func genStructured(w *fixWriter, n int, seed int64) {
	r := rand.New(rand.NewSource(seed))
	cssCfgs := []string{"seeksnack", "default", "css2-prec3", "prec1-inline"}
	for i := 0; i < n; i++ {
		s, inl := genCSS(r)
		cfg := cssCfgs[i%len(cssCfgs)]
		mt := "text/css"
		if inl {
			mt = "text/css;inline=1"
		}
		minRec(w, cfg, configByName(cfg), mt, []byte(s))
	}
	svgCfgs := []string{"seeksnack", "css2-prec3", "prec1-inline"}
	for i := 0; i < n; i++ {
		p := genPath(r)
		svg := fmt.Sprintf(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 %d %d"><path d="%s"/><path d="%s" fill="#FFFFFF"/></svg>`, r.Intn(100), r.Intn(100), p, genPath(r))
		cfg := svgCfgs[i%len(svgCfgs)]
		minRec(w, cfg, configByName(cfg), "image/svg+xml", []byte(svg))
	}
	htmlCfgs := []string{"seeksnack", "default", "html-keepall", "html-ws", "html-quotes", "dummyjs", "url-https"}
	for i := 0; i < n; i++ {
		h := genHTML(r)
		cfg := htmlCfgs[i%len(htmlCfgs)]
		minRec(w, cfg, configByName(cfg), "text/html", []byte(h))
	}
}
