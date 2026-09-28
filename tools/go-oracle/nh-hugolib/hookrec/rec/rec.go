// Package rec is the render-hook and shortcode recorder shared by the
// nh-hugolib hookrec and content oracles (T22): the overlay that adds the
// recorder to package hugolib (overlay_rec.go.txt, applied with `go run
// -overlay` through hsupport.RunOverlaid; the repository is never modified),
// the patches that wrap hookRendererTemplate.Render* and
// renderShortcodeWithPage, and the sites the oracles build.
//
// A record maps (page path + language, output format of the content output,
// kind, ordinal) to the output bytes of one template execution. The kind is
// the hook type (link, image, heading, codeblock, blockquote, table,
// passthrough) or "shortcode:<name>". Executions nested in another recorded
// execution are marked nested (their output is part of the outer one).
package rec

import (
	_ "embed"
	"strings"

	"github.com/neohugo/neohugo/tools/go-oracle/nh-hugolib/hsupport"
)

//go:embed overlay_rec.go.txt
var overlayRec string

// Files are the files the recorder adds to package hugolib.
func Files() map[string]string {
	return map[string]string{
		"hugolib/zz_nh_oracle_t22.go": overlayRec,
	}
}

func hookWrap(sig string) hsupport.Patch {
	return hsupport.Patch{
		File:   "hugolib/site.go",
		After:  sig + " {\n",
		Insert: "\treturn nhHook(hr, cctx, w, func(w io.Writer) error { return hr.templateHandler.ExecuteWithContext(cctx, hr.templ, w, ctx) })\n",
	}
}

// Patches wrap the template executions of hookRendererTemplate and
// renderShortcodeWithPage with the recorder, and carry the content output
// (hooks) and the output format (shortcodes) to it.
func Patches() []hsupport.Patch {
	return []hsupport.Patch{
		{
			File:   "hugolib/site.go",
			After:  "\ttempl           *tplimpl.TemplInfo\n",
			Insert: "\tnhPco           *pageContentOutput\n\tnhKind          string\n",
		},
		hookWrap("func (hr hookRendererTemplate) RenderLink(cctx context.Context, w io.Writer, ctx hooks.LinkContext) error"),
		hookWrap("func (hr hookRendererTemplate) RenderHeading(cctx context.Context, w io.Writer, ctx hooks.HeadingContext) error"),
		hookWrap("func (hr hookRendererTemplate) RenderCodeblock(cctx context.Context, w hugio.FlexiWriter, ctx hooks.CodeblockContext) error"),
		hookWrap("func (hr hookRendererTemplate) RenderPassthrough(cctx context.Context, w io.Writer, ctx hooks.PassthroughContext) error"),
		hookWrap("func (hr hookRendererTemplate) RenderBlockquote(cctx context.Context, w hugio.FlexiWriter, ctx hooks.BlockquoteContext) error"),
		hookWrap("func (hr hookRendererTemplate) RenderTable(cctx context.Context, w hugio.FlexiWriter, ctx hooks.TableContext) error"),
		{
			File:   "hugolib/page__per_output.go",
			After:  "\t\t\t\tresolvePosition: resolvePosition,\n",
			Insert: "\t\t\t\tnhPco:           pco,\n\t\t\t\tnhKind:          layoutDescriptor.Variant1,\n",
		},
		{
			File:   "hugolib/shortcode.go",
			After:  "\tOrdinal int\n",
			Insert: "\n\tnhFormat string\n",
		},
		{
			File:   "hugolib/shortcode.go",
			After:  "\t\tName:        sc.name,\n",
			Insert: "\t\tnhFormat:    po.f.Name,\n",
		},
		{
			File:   "hugolib/shortcode.go",
			After:  "\tbuffer := bp.GetBuffer()\n\tdefer bp.PutBuffer(buffer)\n",
			Insert: "\tif nhRec.on {\n\t\tif err := nhShortcode(ctx, data, buffer, func(b *bytes.Buffer) error { return h.ExecuteWithContext(ctx, tmpl, b, data) }); err != nil {\n\t\t\treturn \"\", fmt.Errorf(\"failed to process shortcode: %w\", err)\n\t\t}\n\t\treturn buffer.String(), nil\n\t}\n",
		},
	}
}

// fm is a YAML front matter block.
func fm(lines ...string) string {
	return "---\n" + strings.Join(lines, "\n") + "\n---\n"
}

func inline(m map[string]string) map[string]hsupport.File {
	out := map[string]hsupport.File{}
	for k, v := range m {
		out[k] = hsupport.File{Content: v}
	}
	return out
}

// Sites are the sites of the T22 oracles: a content-focused synthetic en/th
// site, the reconstructed seeksnack site of hsupport (plus the seeksnack hook
// templates), hugolib/testsite and docs/ (plus a code block render hook:
// Chroma highlighting is not ported), and the shortcode extraction site.
func Sites(root string) ([]hsupport.Site, error) {
	sites := []hsupport.Site{ContentSite()}

	seeksnack, err := hsupport.SeeksnackSite(root)
	if err != nil {
		return nil, err
	}
	seeksnack.Files["layouts/_markup/render-table.json.json"] = hsupport.File{Content: tableJSON}
	seeksnack.Files["layouts/_markup/render-image.html"] = hsupport.File{Content: `<img src="{{ .Destination | safeURL }}" alt="{{ .Text }}"{{ with .Title }} title="{{ . }}"{{ end }}>`}
	seeksnack.Files["content/blog/table.md"] = hsupport.File{Content: fm(`title: "Table"`) + "\n| a | b |\n|---|:-:|\n| 1 | **2** |\n\n![img](koala.jpg \"t\")\n"}
	sites = append(sites, seeksnack)

	repo, err := hsupport.RepoSites(root)
	if err != nil {
		return nil, err
	}
	for _, s := range repo {
		if s.Name == "docs" {
			s.Files["layouts/_markup/render-codeblock.html"] = hsupport.File{Content: `<pre data-lang="{{ .Type }}"><code>{{ .Inner }}</code></pre>`}
		}
		sites = append(sites, s)
	}

	for _, s := range hsupport.EdgeSites() {
		if s.Name == "shortcodes" {
			sites = append(sites, s)
		}
	}
	return sites, nil
}

const tableJSON = `{"head":[{{ range $i, $r := .THead }}{{ if $i }},{{ end }}[{{ range $j, $c := $r }}{{ if $j }},{{ end }}{{ printf "%s:%s" $c.Alignment $c.Text | jsonify }}{{ end }}]{{ end }}],` +
	`"body":[{{ range $i, $r := .TBody }}{{ if $i }},{{ end }}[{{ range $j, $c := $r }}{{ if $j }},{{ end }}{{ printf "%s:%s" $c.Alignment $c.Text | jsonify }}{{ end }}]{{ end }}]}`

const contentBody = `
Intro with [a link](/posts/auto/ "Link title") and ![an image](/img.png "Image title")
and <em>raw html</em> and {{< pos "one" 2 >}}.

<!--more-->

# First heading {#custom-id .cls}

Text $$ x^2 $$ with passthrough off and \(y\).

## Second heading

> A quote with **bold**.

> [!NOTE]
> An alert.

` + "```go {linenos=true}\nfunc main() {}\n```\n\n```\nplain fence\n```\n" + `
| Col A | Col B |
|:------|------:|
| a1    | b1    |
| *a2*  | b2    |

### Third heading

{{< toc >}}

#### Fourth heading

##### Fifth

###### Sixth
`

const shortcodesBody = `
Summary with {{< named a="x" b=3 >}} and {{% md %}}**bold md**{{% /md %}}.

<!--more-->

{{< outer >}}outer text {{< inner >}}inner one{{< /inner >}} {{< inner >}}inner two {{< inner >}}deep{{< /inner >}}{{< /inner >}}{{< /outer >}}

{{% outer %}}
* item with {{% inner %}}*md inner* [nested link](/n "t"){{% /inner %}}
{{% /outer %}}

  {{< deindent >}}
  indented line one
    indented two
  {{< /deindent >}}

  {{< pos "indented" >}}

{{< pos 1 "two" 3.5 true >}} {{< pos >}} {{< named >}}

{{% v1 %}}v1 *markup*{{% /v1 %}} {{< v1 >}}v1 plain{{< /v1 >}}

{{< fmt >}} {{% fmt %}}

{{< greet.inline >}}Hi {{ .Page.Title }}{{< /greet.inline >}} {{< greet.inline />}}

{{</* escaped "x" */>}}

{{% md %}}
## Heading from md shortcode

[md link](https://example.org)
{{% /md %}}

{{< hl >}}**not markdown**{{< /hl >}}
`

// ContentSite is the synthetic en/th site of the T22 oracles: shortcodes
// ({{< >}} and {{% %}}, nested, .Inner/.InnerDeindent, positional and named
// params, .Parent, .Ordinal, inline, version 1, output format variants,
// shortcodes in summaries), manual/front matter/auto summaries with
// summaryLength, render hooks (link, image, heading, codeblock, blockquote,
// table with a json variant, passthrough off), TOC levels, Thai and CJK word
// counts, JSON/RSS outputs.
func ContentSite() hsupport.Site {
	files := map[string]string{
		"layouts/_markup/render-link.html":       `<a href="{{ .Destination | safeURL }}"{{ with .Title }} title="{{ . }}"{{ end }}>{{ .Text }}</a>`,
		"layouts/_markup/render-image.html":      `<img src="{{ .Destination | safeURL }}" alt="{{ .PlainText }}">`,
		"layouts/_markup/render-heading.html":    `<h{{ .Level }} id="{{ .Anchor }}">{{ .Text }} #{{ .Level }}</h{{ .Level }}>`,
		"layouts/_markup/render-heading.rss.xml": `<h{{ .Level }}>{{ .Text }}</h{{ .Level }}>`,
		"layouts/_markup/render-codeblock.html":  `<pre class="cb" data-lang="{{ .Type }}"><code>{{ .Inner }}</code></pre>`,
		"layouts/_markup/render-blockquote.html": `<blockquote class="{{ .Type }}">{{ .Text }}</blockquote>`,
		"layouts/_markup/render-table.json.json": tableJSON,
		"layouts/_shortcodes/pos.html":           `[{{ .Get 0 }}|{{ .Get 1 }}|{{ .Get 9 }}|{{ .IsNamedParams }}|{{ .Ordinal }}]`,
		"layouts/_shortcodes/named.html":         `[{{ .Get "a" }}-{{ .Get "b" }}-{{ .Get 0 }}-{{ .IsNamedParams }}]`,
		"layouts/_shortcodes/md.html":            `{{ .Inner }}`,
		"layouts/_shortcodes/hl.html":            `<mark>{{ .Inner }}</mark>`,
		"layouts/_shortcodes/outer.html":         `<div class="outer" data-ord="{{ .Ordinal }}">{{ .Inner }}</div>`,
		"layouts/_shortcodes/inner.html":         `<span data-parent="{{ with .Parent }}{{ .Name }}{{ .Ordinal }}{{ end }}" data-ord="{{ .Ordinal }}">{{ .Inner }}</span>`,
		"layouts/_shortcodes/deindent.html":      "<pre>{{ .InnerDeindent }}</pre>\n<p>second\nline</p>",
		"layouts/_shortcodes/v1.html":            "{{ $_hugo_config := `{ \"version\": 1 }` }}<em>{{ .Inner }}</em>",
		"layouts/_shortcodes/fmt.html":           `<b>html fmt</b>`,
		"layouts/_shortcodes/fmt.rss.xml":        `rss fmt`,
		"layouts/_shortcodes/toc.html":           `<nav class="toc">{{ .Page.TableOfContents }}</nav>`,
		"layouts/_shortcodes/title.html":         `{{ .Page.Title }}`,
		"layouts/home.html":                      `{{ .Content }}`,
		"layouts/page.html":                      `{{ .Content }}`,
		"layouts/section.html":                   `{{ .Content }}`,

		"content/_index.md":           fm(`title: "Home"`) + "\nHome **content** with a [link](/posts/).\n\n<!--more-->\n\nAfter the divider.\n",
		"content/_index.th.md":        fm(`title: "หน้าแรก"`) + "\nหน้าแรก {{< title >}}\n",
		"content/posts/_index.md":     fm(`title: "Posts"`, `summary: "Front **matter** summary with [a link](/x) and {{< pos 1 >}}"`) + "\nSection body.\n",
		"content/posts/hooks.md":      fm(`title: "Hooks"`) + contentBody,
		"content/posts/hooks.th.md":   fm(`title: "ฮุก"`) + contentBody,
		"content/posts/shortcodes.md": fm(`title: "Shortcodes"`) + shortcodesBody,
		"content/posts/auto.md": fm(`title: "Auto summary"`) + `
One two three four five six seven eight nine ten eleven twelve thirteen fourteen.
Fifteen sixteen.

Second paragraph with more words to count for the reading time and the fuzzy word count.
`,
		"content/posts/auto-short.md":     fm(`title: "Short"`) + "\nOnly a few words.\n",
		"content/posts/manual.md":         fm(`title: "Manual"`) + "\nBefore the divider {{< pos \"s\" >}}.\n\n<!--more-->\n\nAfter the divider.\n",
		"content/posts/manual-lead.md":    fm(`title: "Lead"`) + "<!--more-->\nOnly after.\n",
		"content/posts/fm-summary.md":     fm(`title: "FM summary"`, `summary: "A *front matter* summary with [link](/y)."`) + "\nBody text that is long enough to be truncated otherwise one two three four five six seven eight nine ten.\n",
		"content/posts/fm-and-divider.md": fm(`title: "FM and divider"`, `summary: "ignored?"`) + "\nIntro.\n\n<!--more-->\n\nRest.\n",
		"content/posts/cjk.md":            fm(`title: "CJK"`) + "\n這是一個中文段落，用於測試字數統計。還有更多的文字在這裡，以便自動摘要可以截斷它。English words too.\n\n第二段。\n",
		"content/posts/cjk-off.md":        fm(`title: "CJK off"`, `isCJKLanguage: false`) + "\n這是一個中文段落，用於測試字數統計。\n",
		"content/posts/thai.th.md":        fm(`title: "ไทย"`) + "\nภาษาไทยไม่มีช่องว่างระหว่างคำ แต่มีช่องว่างระหว่างประโยค นี่คือประโยคที่สาม และประโยคที่สี่ ห้า หก เจ็ด แปด เก้า สิบ สิบเอ็ด สิบสอง สิบสาม\n\n| หัว | คอลัมน์ |\n|---|---|\n| ก | ข |\n",
		"content/posts/thai-cjk.th.md":    fm(`title: "ไทย CJK"`, `isCJKLanguage: true`) + "\nภาษาไทย คำ สอง\n",
		"content/posts/html-page.html":    fm(`title: "HTML page"`) + "<p>HTML <b>content</b> {{< pos \"h\" >}}</p>\n<!--more-->\n<p>more</p>\n",
		"content/posts/empty.md":          fm(`title: "Empty"`),
		"content/posts/toc-only.md":       fm(`title: "TOC"`) + "\n# H1\n\n## H2 a\n\n### H3\n\n#### H4\n\n## H2 b\n\n##### H5\n\n## H2 a\n",
		"content/posts/json-table.md":     fm(`title: "JSON table"`, `outputs: [html, json]`) + "\n| x | y |\n|---|---|\n| 1 | 2 |\n\nText after the table.\n",
		"content/posts/markup-html.md":    fm(`title: "Markup html"`, `markup: html`) + "<div>raw <i>html</i> markup {{< pos 1 >}}</div>\n",
		"content/bundle/index.md":         fm(`title: "Bundle"`) + "\nBundle body with [link](sub/).\n",
		"content/bundle/sub.md":           fm(`title: "Bundled page"`) + "\nA *bundled* content page {{< pos \"b\" >}}.\n",
		"content/bundle/img.png":          "png",
	}
	toml := `baseURL = "https://example.org/"
title = "Content"
defaultContentLanguage = "en"
hasCJKLanguage = true
summaryLength = 12
[security]
enableInlineShortcodes = true
[markup.tableOfContents]
startLevel = 2
endLevel = 4
ordered = false
[markup.goldmark.parser.attribute]
block = true
title = true
[markup.goldmark.renderer]
unsafe = true
[outputs]
home = ["html", "rss", "json"]
page = ["html", "json"]
section = ["html", "rss"]
[languages.en]
weight = 1
[languages.th]
weight = 2
`
	return hsupport.Site{Name: "content", TOML: toml, Files: inline(files)}
}
