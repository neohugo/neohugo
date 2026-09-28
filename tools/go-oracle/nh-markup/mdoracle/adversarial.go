package mdoracle

import (
	"fmt"
	"strings"
)

// Adversarial returns the hand-written documents: auto IDs (Thai,
// mixed-script, duplicates, entities, the TextPlain first-child quirk),
// attributes on headings and blocks, tables with alignment, blockquote
// alerts, images, code fences with attributes, raw HTML, typographer,
// linkify, strikethrough, footnotes, definition lists, task lists and deep
// nesting.
func Adversarial() []Doc {
	var docs []Doc
	add := func(name, src string) {
		docs = append(docs, Doc{Name: "adversarial/" + name, Src: []byte(src)})
	}

	add("autoid-thai", `### รสชาติ

### สแน็คแจ๊ค รสดั้งเดิม (ขนมถั่วลั่นเตาอบกรอบ)

## ขนมปังกรอบ ๑๒๓ Thai digits

## ภาษาไทย English 中文 123 mixed

## Ünïcödé İstanbul ǅ

## Café Crème Brûlée à la carte

## Ωmega Straße ﬁ ligature Å Kelvin K Angstrom Å ½ ² Ⅻ

## 🍫

## 日本語の見出し

## مرحبا بالعالم

## Привет, мир!

## Ḍ̇ combining marks é ä

## Dup

## Dup

## dup-1

## heading

#

## !!!

## _under_score_ and __strong__

### Edit layouts/_default/index.JSON

### Squidy - Seasoned Roller Squid Hot&Spicy

### White Koala's March (Chocolate Filling )
`)
	add("autoid-inline", "## **Strong *em* more** tail\n\n## ![alt *x*](img.png \"T\") img\n\n## &amp; &copy; entity &#65; &#x42;\n\n## `code` span and <span class=\"x\">raw</span> html\n\n## [link *text*](http://example.com) after\n\n## <https://auto.link/x> autolink\n\n## ~~strike~~ through\n\n## \"quoted\" -- dashes... 'single'\n\nSetext line one\nline two\n===\n\nAnother *setext*\n---\n\n# Trailing hashes ##\n\n## Tab\there\n\n## NBSP here \n")
	add("autoid-dups", strings.Repeat("## Same\n\n", 12)+"## Same-1\n\n## same-2\n\n## Same {#same}\n\n## X {#same}\n\n## Y {#same-1}\n")
	add("attributes", `## Heading {#custom-id .cls1 .cls2 data-x="1" num=3.5 flag=true}

## Another {.only-class}

## Onclick {onclick="alert(1)" title="t"}

## Arr {hl=[1,"2-3"] x=false}

A paragraph
{.para-class #para-id}

> quote
{.q}

- item 1
- item 2
{.list-class}

| a | b |
|---|---|
| 1 | 2 |
{.table-class data-t="x"}

{.solitary}

Text

{.after-blank}
`)
	add("tables", "| Left | Center | Right | None |\n|:-----|:------:|------:|------|\n| l | c | r | n |\n| *em* | `code` | [link](http://x) | ![img](i.png) |\n| a \\| b | x | | |\n| only one |\n| 1 | 2 | 3 | 4 | 5 |\n\nhead only\n\n| h1 | h2 |\n| -- | -- |\n\n> | t | in quote |\n> |---|:--|\n> | 1 | 2 |\n\n- | t | in list |\n  |---|---|\n  | 1 | 2 |\n")
	add("blockquotes", `> [!NOTE]
> Useful information.

> [!warning]- Foldable title
> Body *text*.

> [!TIP]+ Plus
> Line 1
> Line 2

> [!CAUTION] inline title only

> [!NOTE]

> [!Important]
>
> paragraph two

> plain quote

> outer
> > inner
> > > innermost

> - list in quote
> - second

>`+" "+`nbsp start

> [!note]`+"\t"+`tab title
> body
`)
	add("images", `![alt](a.png)

![alt *em* text](b.jpg "The Title")

![](c.png)

Inline ![one](1.png) and ![two](2.png "t2") images.

[![inside link](x.png)](http://example.com)

![ref image][ref]

[ref]: /ref.png "Ref Title"

![a](<with space.png>)

![esc\*aped](d\_e.png 'single "title"')

![with attrs](attr.png "T")
{.img-class #img-id data-x="1"}

Text ![inline with attrs](i.png)
{.para-only}
`)
	add("codefences", "```go {.class linenos=table hl_lines=[2,\"4-5\"] style=monokai}\nfunc main() {\n}\n```\n\n```bash\necho hi\n```\n\n```\nno lang\n```\n\n~~~ {#id .c}\ntilde\n~~~\n\n```dmylang {hl_Lines=[1] anchorLineNos=true lineNoStart=3 title=\"T\"}\nx\n```\n\n```python extra words\nprint(1)\n```\n\n```yaml\na: 1\n\n\n```\n\n    indented code\n    block\n\n```html\n<div>\n```\n\n```JSON\n{}\n```\n\n``` go\nspace before lang\n```\n")
	add("rawhtml", "<div class=\"x\">\n*md inside*\n</div>\n\n<!-- a comment -->\n\n<!--StartFragment-->\n\nInline <span class=\"s\">span</span> and <i class=\"fas fa-star\"></i> icon and <!-- inline comment -->.\n\n<script>alert(1)</script>\n\n<p>para html</p>\n\n<div>\nunclosed\n\ntext <br> after <br /> break\n")
	add("typographer", "\"Double quotes\" and 'single quotes' and it's and the '90s and 'twas.\n\nDashes -- and --- and ellipsis... and <<angle>> quotes.\n\n5'10\" tall and \"Monitor 21\"\" and \"unbalanced.\n\nLay's and Mala (seasoning) and don't, won't, y'all'd've.\n\n\"Nested 'inner' quotes\" end.\n")
	add("linkify", "Visit https://www.example.com/path?q=1. And www.example.org, or http://foo.bar/(baz) and https://x.y/a_b_.\n\nMail me at someone@example.com. Or <someone@example.org>.\n\n**http://bold.com** and (https://paren.example.com) and ftp://files.example.com/x.\n\nhttps://en.wikipedia.org/wiki/Mala_(seasoning) and http://www.copyright.gov.\n")
	add("extensions", "~~strike~~ and ~single~ and ~~*nested*~~.\n\nFootnote ref[^1] and another[^note].\n\n[^1]: The first footnote.\n[^note]: A *named* footnote.\n\n    With a second paragraph.\n\nTerm 1\n: Definition 1\n\nTerm *2*\n: Definition 2a\n: Definition 2b\n\n- [x] done task\n- [ ] open task\n  - [X] nested done\n\n1. [ ] ordered task\n")
	var nested strings.Builder
	for i := 0; i < 14; i++ {
		indent := strings.Repeat("  ", i)
		if i%2 == 0 {
			fmt.Fprintf(&nested, "%s- level %d\n", indent, i)
		} else {
			fmt.Fprintf(&nested, "%s1. level %d\n", indent, i)
		}
	}
	nested.WriteString("\n\n1. a\n\n   b\n2. c\n\n- x\n\n  > quote in item\n  > ```\n  > code\n  > ```\n")
	add("nested-lists", nested.String())
	add("links", "[inline](http://a.com \"Title\") [ref][r] [collapsed][] [shortcut] <http://auto.com> <mailto:x@y.z> [empty]() [dangerous](javascript:alert(1)) [rel](/rel/path#frag) [q](?a=b&c=d) [thai](http://www.lotte.co.th/product/%E0%B9%84) [space](<a b>)\n\n[r]: http://ref.com\n[collapsed]: http://c.com\n[shortcut]: /s \"S\"\n\n## Heading with [link](http://h.com) inside\n")
	add("headings-toc", "# H1 first\n\n## H2 a\n\n### H3 a\n\n#### H4 a\n\n##### H5 a\n\n###### H6 a\n\n## H2 b\n\n#### H4 skip\n\n# H1 second\n\n### H3 after h1\n\n## H2 *emph* `code` [l](x) ![i](y) ~~s~~ <b>raw</b> &amp;\n")
	add("toc-deep-first", "#### starts deep\n\n## then two\n\n# then one\n\n###### six\n")
	add("empty", "")
	add("only-ws", "   \n\n\t\n")
	add("crlf", "## CRLF heading\r\n\r\nPara line one\r\nline two\r\n\r\n| a | b |\r\n|---|---|\r\n| 1 | 2 |\r\n")
	add("invalid-utf8", "## Bad \xff\xfe bytes\n\nText \xc3 and \xe0\xb8 end.\n\n![alt \xff](x\xfe.png)\n")
	add("hugo-shortcode-text", "{{< ref \"privacy.md\" >}} and {{% notice %}}text{{% /notice %}}\n\n{{ .Content | plainify | jsonify }} and {{end}}\n\n[HAHAHUGOSHORTCODE123s0HBHB](HAHAHUGOSHORTCODE123s1HBHB)\n")
	add("hugo-ctx-inline", "Before\n{{__hugo_ctx pid=7}}\nInside **bold** [link](http://x) ![img](i.png)\n{{__hugo_ctx/}}\nAfter\n\n{{__hugo_ctx pid=9}}\n## Heading in ctx\n{{__hugo_ctx/}}\n\n<div>\n{{__hugo_ctx pid=3}}\n</div>\n")
	return docs
}
