package osupport

import (
	"math/rand"
	"sort"
	"strings"
)

// ElementTags are the tag names of the adversarial element strings: the
// tree-builder quirks (th/caption/col/colgroup/frame/image ignored or renamed
// in body), the collector's div-substituted table tags, raw-text and
// foreign-content elements, case and Unicode variants and odd names the
// collector's lexer lets through.
var ElementTags = []string{
	"th", "td", "tr", "thead", "tbody", "tfoot", "caption", "col", "colgroup", "frame", "frameset",
	"image", "img", "table", "html", "head", "body", "title", "style", "script", "template", "select",
	"option", "optgroup", "textarea", "pre", "noscript", "noframes", "plaintext", "xmp", "iframe",
	"svg", "math", "foreignObject", "desc", "a", "p", "li", "dd", "dt", "form", "input", "button",
	"br", "hr", "div", "span", "TH", "Div", "TABLE", "Tr", "D\u0130V", "\u017fvg", "custom-el", "x:y",
	"<div", "?php", "![CDATA[x]]", "%", "base", "meta", "link", "nobr", "ruby", "rt", "mi", "annotation-xml",
	"font", "b", "marquee", "keygen", "area", "menu", "main", "search", "dialog", "h1", "listing",
}

// ElementAttrs are the attribute strings combined with ElementTags.
var ElementAttrs = []string{
	`class="a b"`, `class='a b'`, `class=a`, `CLASS="X Y"`, `class`, `class=""`, "class=\"a\tb\nc\r\nd\fe\"",
	`ClAsS="q"`, `id=x`, `ID="y"`, `id="a" id="b"`, `x-transition:enter="t1 t2"`, `transition="t"`,
	`data-transition="d e"`, `:class="{ 'a': x, 'b c': y }"`, `x-bind:class="{a: b, 'c': d}"`,
	`v-bind:class="isX ? 'p q' : 'r'"`, `:class="['a','b']"`, ":class=\"{ 'x':\n a,\n 'y': b }\"",
	":class=\"{a:\tb, c:\nd}\"", `:CLASS="{ 'up': 1 }"`, `x-bind:class="  { 'sp':1 }  "`, `:class="'lone"`,
	`:class="{}"`, `:class="{ , }"`, `:class="{ 'a',: b }"`, `:classx="{ 'k': 1 }"`, `a:class="plain 'q'"`,
	`class="&amp;a &lt;b"`, `class="&notin;x &notit; &amp"`, `class=&amp`, `id="&#x41;&#65;&#0;&#xD800;&#x110000;&#128;&#x;&#;"`,
	`class="a&ampb"`, `href="x&amp=y" class="c&amp=d"`, "class=\"nul\x00x\"", `class="日本 語"`,
	"class=\"a\u00a0b\"", "class=\"a\u2028b\"", `class='it"s'`, `class="x'y"`, `class=a'b`, `/class=x`,
	`class =  "sp"`, `class= x`, `=x class=y`, `a=b=c class=d`, `class="a" /`, `class=x/`, `class="z"/`,
	`xlink:href="h" class="svgc"`, `viewbox="0 0 1 1" class="v"`, `definitionurl="u" class="m"`,
	`encoding="text/html" class="e"`, `type="hidden" class="h"`, `color="red" class="f"`,
	`class="\r\nw"`, `id="  spaced  "`, `class="` + strings.Repeat("c ", 40) + `"`, `x-transition="{ 'n': 1 }"`,
}

// ElementStrings returns the adversarial element strings (each starts with
// "<" and ends with ">", like the collector's).
func ElementStrings() []string {
	seen := map[string]bool{}
	var out []string
	add := func(s string) {
		if !seen[s] {
			seen[s] = true
			out = append(out, s)
		}
	}
	for _, t := range ElementTags {
		add("<" + t + ">")
		add("<" + t + "/>")
		for _, a := range ElementAttrs {
			add("<" + t + " " + a + ">")
			add("<" + t + "\t" + a + "/>")
		}
	}
	rnd := rand.New(rand.NewSource(11))
	for i := 0; i < 3000; i++ {
		t := ElementTags[rnd.Intn(len(ElementTags))]
		var b strings.Builder
		b.WriteString("<" + t)
		for j := rnd.Intn(4); j >= 0; j-- {
			b.WriteString([]string{" ", "\n", "\t", "  "}[rnd.Intn(4)])
			b.WriteString(ElementAttrs[rnd.Intn(len(ElementAttrs))])
		}
		b.WriteString([]string{">", "/>", " >", " / >"}[rnd.Intn(4)])
		add(b.String())
	}
	sort.Strings(out)
	return out
}
