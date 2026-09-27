# Markdown rendering (goldmark) and shortcodes: Rust port spec (seeksnack parity)

Agent: `markdown`. Scope: everything between a content file's body bytes and the HTML string that
templates receive as `.Content` / `.Plain` / `markdownify`, including render hooks and shortcodes.

Scratch assets referenced below are in
`/private/tmp/claude-501/-Users-blackb1rd-git-github-org-neohugo/0835ef6d-534f-4ac6-a7eb-50e9240d6eb1/scratchpad/work/markdown/`
(abbreviated `$W`). Go sources: neohugo at `/Users/blackb1rd/git/github/org/neohugo` (abbrev. `NH`), goldmark at
`~/go/pkg/mod/github.com/yuin/goldmark@v1.7.12` (abbrev. `GM`).

---

## 0. Key findings (TL;DR)

1. **The seeksnack markdown surface is small but must be exact.** 251 `.md` files. They use ATX and setext headings,
   emphasis, pipe tables (119), links (280, all absolute `http(s)`), autolinks `<...>`, 3 linkified bare URLs/emails,
   images (525, all in stand-alone paragraphs), lists, 9 blockquotes, inline raw HTML (`<i class=...>`), HTML comment
   blocks (`<!--StartFragment-->`), backslash hard breaks (2), code spans (3), typographer substitutions (215 `&rsquo;`,
   32 `&ldquo;`/`&rdquo;` pairs, 3 `&hellip;`), 223 NBSP (U+00A0) characters, lots of Thai text. **No** fenced/indented code,
   footnotes, definition lists, task lists, strikethrough, `<!--more-->`, emoji shortcodes, or heading attributes `{#id}`.
2. **Only one shortcode is used in content:** the embedded `ref` shortcode, 6 times in `privacy.md` and `terms.md`, in the
   form `[{{< ref "privacy.md" >}}]({{< ref "privacy.md" >}})`. It goes through placeholder substitution
   (`HAHAHUGOSHORTCODE<pid>s<n>HBHB`) and renders to the target's absolute permalink (`https://seeksnack.com/privacy/`).
   Site shortcodes `url` and `modalImage` are defined but unused.
3. **Four render hooks shape the output**:
   * site `layouts/_default/_markup/render-heading.html` (every heading),
   * site `layouts/_default/_markup/render-image.html` (every image; does image processing),
   * **embedded** `_markup/render-link.html`, used for every link and autolink, because for multilingual single-host sites
     Hugo turns `renderHooks.link.useEmbedded = "auto"` into `"fallback"` (`NH/config/allconfig/allconfig.go:1120-1128`).
     This is why `https://en.wikipedia.org/wiki/Mala_(seasoning)` comes out as `.../Mala_%28seasoning%29` and `Lay's` as `Lay%27s`.
   * **embedded** `_markup/render-table.html`. Hugo *always* renders tables through a hook (`NH/markup/goldmark/tables/tables.go:72-75`).
4. **Heading IDs** use Hugo's own `render.TextPlain` (`NH/markup/goldmark/internal/render/context.go:296-327`). It has a quirk:
   inside any non-text inline node it only follows the **first child**. For example, `## **Strong *em* more** tail` gets the ID
   `strong--tail`. The text is then sanitized with Go `unicode.IsLetter/IsDigit/ToLower`. Thai combining vowels (Mn) are
   dropped: `รสชาติ` becomes `รสชาต` (verified in golden). IDs are deduplicated per document with `-1`, `-2`, and so on.
5. **A line-by-line port of goldmark v1.7.12 plus Hugo's goldmark glue is required.** comrak and pulldown-cmark
   differ byte-wise on 210/251 seeksnack files. Even after aggressive normalization, comrak still differs on 2 files and
   pulldown on 6: typographer heuristics and linkify differ. Rust `char::is_alphabetic` / `is_numeric` / `to_lowercase` differ
   from Go's `unicode.IsLetter` / `IsDigit` / `ToLower` on 1749 / 1154 / 1 code points, including Thai U+0E31, U+0E34-0E3A, U+0E4D.
   That would break heading IDs.
6. **Verification is already done.** Every seeksnack page went through neohugo's own goldmark converter (Go harness `$W/mdref`, no
   hooks), and the real hooks were emulated with 3 regex rules. That reproduces Hugo's actual `.Content` byte-for-byte on
   **242/246** mapped pages. The other 4 differ only by (a) the embedded link hook's URL normalization (2 pages) and
   (b) the `ref` shortcode (2 pages), which the harness does not expand. So the markdown layer is fully specified by
   goldmark + Hugo glue + the 4 hook contracts described here.
7. **Cross-subsystem finding (image hashes).** Rebuilding the seeksnack site copy now gives *different processed-image
   filenames* than `golden/run1`, for example `19030102_hu_e53d606df535cad8.webp` vs golden `19030102_hu_93106ed2c8cca33a.webp`.
   A copy of the site with an **empty `resources/` cache** reproduces the golden names on the first build. The second
   build in the same copy (warm `resources/_gen`) produces the other names again (`$W/hashtest`). The third warm build
   is stable at the non-golden name. `--ignoreCache` on a warm copy reproduces the golden names. **Golden output
   corresponds to a cold-cache build.** Reproduce it with `--ignoreCache` or with an empty `resources/`. This affects image
   `src`/`srcset` inside `.Content` (image hook) and index.json. Handed to the images/resources agent.

---

## 1. Where markdown output reaches the golden files

| Consumer | Template | What is used |
|---|---|---|
| Regular pages | `layouts/_default/single.html:96` `{{ .Content }}` | full rendered HTML |
| privacy/terms/disclaimer (`layout: simple`) | `layouts/_default/simple.html:21` | `.Content` (the `ref` shortcode is here) |
| Term pages with `_index.md` body (e.g. `/ingredients/ins-124/`, companies) | `layouts/term/term.html:82` | `.Content` |
| Taxonomy list | `layouts/taxonomy/list.html:27` | `.Content` |
| `index.json` (home JSON output) | `layouts/_default/index.json:16` | `$page.Plain` for every page in `.Site.AllPages` that has an `image_preview` resource |
| Comments | `layouts/partials/comments.html:278,323` | `{{ .comment \| markdownify }}` (2 comments rendered: `rice-crackers/dozo-japanese-rice-original-flavoured`, `brands/yubari-melon`) |
| CSS | `postcss.config.js` purgecss reads `hugo_stats.json` (`writeStats=true`), which contains tags/classes/ids from **all** HTML output including markdown | the set of tags (`table`, `thead`, `blockquote`, `picture`, `hr`, `code`, `br`, …) and heading ids influence which CSS survives in every page's inline `<style>` |

**Not used (so not output-affecting; implement lazily):** `.Summary` (auto summary is computed eagerly in
`NH/hugolib/page__content.go:622-632` but no template reads it; one page has front matter `summary:`), `.TableOfContents`,
`.Fragments`, `.WordCount`, `.ReadingTime` (only in `layouts/posts/single.html`, no posts exist), `.Truncated`,
`RenderString` (other than markdownify), `emojify`.

The content-only derived site (`$W/contentsite`, built to `$W/contentsite-out`) prints `.Content` and `.Plain` of every
page. It is the **best golden for the markdown layer**: canonifyURLs is disabled there, so you see exactly what
goldmark + hooks emit. `$W/contentmap.json` maps 246 content files to their output files. The 5 missing ones have `&`
in their path, which is the paths agent's topic.

---

## 2. Effective configuration

Config decoding: `NH/markup/markup_config/config.go` `Decode` (lines 49-74) plus `normalizeConfig` (76-107), then
`goldmark_config.Config.Init` (`NH/markup/goldmark/goldmark_config/config.go:113-121`, `Parser.Init` 296-302).

| hugo.toml key | Effective value | Notes |
|---|---|---|
| `markup.defaultMarkdownHandler` | `goldmark` | |
| `extensions.definitionList` | true | enabled, unused by content |
| `extensions.footnote` | true | enabled, unused by content |
| `extensions.linkify` | true | used (3 hits); `LinkifyProtocol` default `"https"` (config.go:48) |
| `extensions.strikethrough` | true | unused |
| `extensions.table` | true | used heavily |
| `extensions.taskList` | true | unused |
| `extensions.typographer = true` | bool `true` is **deleted** by `normalizeConfig` (config.go:95-104), so the **default** substitutions apply: `&lsquo; &rsquo; &ldquo; &rdquo; &ndash; &mdash; &hellip; &laquo; &raquo;` and apostrophe `&rsquo;` (goldmark_config/config.go:30-42) |
| `extensions.cjk` | default disabled | |
| `extensions.passthrough` | default disabled | |
| `extensions.extras.*` | default disabled (`extras.New` adds nothing, `hugo-goldmark-extensions/extras@v0.3.0/inline.go:164-188`) | |
| `parser.autoHeadingID` | true | |
| `parser.autoHeadingIDType = 'github'` | copied into `AutoIDType` by `Parser.Init` (legacy key renamed in 0.144) | `github` = Unicode-preserving |
| `parser.autoDefinitionTermID` | false (default) | |
| `parser.wrapStandAloneImageWithinParagraph` | true | images stay inside `<p>`; `IsBlock` always false |
| `parser.attribute.title = true` | `parser.WithAttribute()` (convert.go:191-193), so `{...}` after ATX headings is parsed as attributes | none in content |
| `parser.attribute.block = false` | no attribute block parser | |
| `renderer.hardWraps = false`, `xhtml = false`, `unsafe = true` | `html.WithUnsafe()` only | raw HTML passes through |
| `enableEmoji` | false (default) | `emoji.Emoji` not added (convert.go:187-189) |
| `markup.highlight.codeFences` | true (default; legacy `pygmentscodefences=true` agrees) | `codeblocks.New()` installed; no fenced code in content |
| legacy `pygmentsOptions = "linenos=table"`, `pygmentsstyle` | applied to highlight config by `highlight.ApplyLegacyConfig` (`NH/markup/highlight/config.go:160-190`) | irrelevant (no code blocks) |
| legacy **`footnoteReturnLinkContents = "↩"`** | **ignored**. No code in neohugo reads it (grep finds nothing). goldmark's default backlink `&#x21a9;&#xfe0e;` would be used | footnotes unused anyway |
| `markup.goldmark.renderHooks.link/image.useEmbedded` | default `auto`, rewritten to **`fallback`** for each language (`allconfig.go:1120-1128`), because `len(languagesConfig) > 1` (11 languages declared), not multihost, and `duplicateResourceFiles=false` | embedded link hook **is used**. The embedded image hook is shadowed by the site's `render-image.html` |
| `markup.tableOfContents` | defaults startLevel 2, endLevel 3, ordered false | TOC built but unused |

---

## 3. The goldmark instance (exact composition)

Built once per site config in `NH/markup/goldmark/convert.go:84-212` (`newMarkdown`). With seeksnack config the
extension list, in `goldmark.New(WithExtensions(...))` order, is:

1. `hugocontext.New(logger)` (`NH/markup/goldmark/hugocontext/hugocontext.go:299-317`)
2. `newLinks(cfg)` (Hugo hooked renderer for Link/AutoLink/Image/Heading, `render_hooks.go:548-552`)
3. `newTocExtension(tocRendererOptions)` (`toc.go:133-141`)
4. `blockquotes.New()` (`blockquotes/blockquotes.go:41-45`)
5. `images.New(true)` (`images/transform.go:24-34`)
6. `extras.New(all disabled)` (no-op)
7. `codeblocks.New()` (highlight.CodeFences true)
8. `extension.Table` + `tables.New()`
9. `extension.Strikethrough`
10. `extension.Linkify`
11. `extension.TaskList`
12. `extension.NewTypographer(WithTypographicSubstitutions(defaults))`
13. `extension.DefinitionList`
14. `extension.Footnote`
15. `attributes.New(cfg.Parser)` (`internal/extensions/attributes/attributes.go:27-47`), added because `AutoHeadingID` is true

Parser option: `parser.WithAttribute()`. Renderer options: `html.WithUnsafe()`.

Per document: `parser.NewContext(parser.WithIDs(newIDFactory("github")))` plus `tocEnableKey = RenderTOC` (convert.go:293-299).
Hugo always passes `RenderTOC: true` for page content (`NH/hugolib/page__per_output.go:424-431, 443-450`), and `false` for
`RenderString`/markdownify.

### 3.1 Registration order and priority (lower number wins)

`util.PrioritizedSlice.Sort` sorts ascending by priority (`GM/util/util.go:877-881`, **`sort.Slice`, not stable**). No two
registrations in this config share a priority *and* a key, so the instability is harmless. Keep it that way in Rust (use a
stable sort and assert there are no ties on the same trigger/kind).

**Block parsers** (`GM/parser/parser.go:581-593` + extensions): setext heading 100, definition list 101, definition
description 102, thematic break 200, list 300, list item 400, indented code 500, ATX heading 600, fenced code 700,
blockquote 800, HTML block 900, footnote definition 999, paragraph 1000. Block parsers are indexed by trigger byte. Parsers
with `Trigger()==nil` ("free" parsers) are appended to *every* trigger list (`parser.go:846-850`).

**Inline parsers** (`parser.go:604-615` + extensions), with trigger bytes:

| prio | parser | triggers |
|---|---|---|
| 0 | tasklist checkbox (`GM/extension/tasklist.go:115`) | `[` |
| 50 | hugocontext | `{` |
| 100 | code span | `` ` `` |
| 101 | footnote reference | `[` |
| 200 | link/image | `[` `!` `]` |
| 300 | autolink `<...>` | `<` |
| 400 | raw HTML | `<` |
| 500 | emphasis | `*` `_` |
| 500 | strikethrough | `~` |
| 999 | linkify | `' '` (= any whitespace **and** "line head"), `*` `_` `~` `(` |
| 9999 | typographer | `'` `"` `-` `.` `,` `<` `>` `*` `[` |

For a trigger byte, the parsers are tried in priority order. The first non-nil result wins, and the reader position is
restored between attempts (`parser.go:1195-1203`).

**Paragraph transformers**: link reference definitions 100, table 200.
**AST transformers** (in run order): table escaped-pipe fixer 0 (`GM/extension/table.go:549`), hugocontext 10, Hugo
attributes/auto-ID 100, TOC 110, Hugo images 300, footnote 999.

**Node renderers**, registered from highest to lowest number so that lower numbers overwrite (`GM/renderer/renderer.go:137-148`):
goldmark html 1000; table/strikethrough/tasklist/deflist/footnote 500; Hugo `hookedRenderer` 100 (Link, AutoLink, Image,
Heading), Hugo blockquote 100, Hugo codeblocks 100 (FencedCodeBlock), Hugo tables 100 (Table, TableHeader, TableRow,
TableCell), hugocontext 50 (HugoContext, **RawHTML, HTMLBlock**).
The effective renderer per kind:

| Node | Renderer |
|---|---|
| Document, Paragraph, TextBlock, List, ListItem, ThematicBreak, CodeBlock, CodeSpan, Emphasis, Text, String | goldmark `renderer/html/html.go` |
| RawHTML, HTMLBlock | hugocontext (same bytes as goldmark for unsafe=true unless `{{__hugo_ctx` appears) |
| Heading, Link, AutoLink, Image | Hugo `render_hooks.go` (hook if a hook template exists, else default) |
| Blockquote | Hugo `blockquotes.go` (different from goldmark: see 6.5) |
| Table* | Hugo `tables.go` via the table hook |
| FencedCodeBlock | Hugo `codeblocks/render.go` (chroma highlighter). Not reachable for seeksnack |
| Strikethrough, TaskCheckBox, DefinitionList*, Footnote* | goldmark extension renderers |

---

## 4. Hugo content pipeline around goldmark

### 4.1 Source to goldmark input

1. The page file is lexed by `NH/parser/pageparser` (`pagelexer.go`, `pagelexer_intro.go`, `pagelexer_shortcode.go`).
   Front matter is YAML `---`, TOML `+++` (Berli-Jucker company page) or JSON. Items after the front matter are mapped in
   `contentParseInfo.mapItemsAfterFrontMatter` (`NH/hugolib/page__content.go:351-445`):
   * text items are kept as source slices (`AddBytes`),
   * `<!--more-->` becomes `TypeLeadSummaryDivider` and is replaced by `internalSummaryDividerPre` (not present in seeksnack),
   * shortcodes: `extractShortcode` (`shortcode.go:584`), placeholder
     `createShortcodePlaceholder("s", pid, ordinal)` = `"HAHAHUGOSHORTCODE" + pid + "s" + ordinal + "HBHB"` (`shortcode.go:191-195`).
     Ordinals count per page from 0.
2. `contentToRender` (`page__content.go:227-264`) concatenates the items:
   * `{{< >}}` shortcodes (`doMarkup=false`, so `insertPlaceholder()` is true, `shortcode.go:224-226`) insert the
     **placeholder text** into the markdown,
   * `{{% %}}` shortcodes (config version 2) are **rendered first** and their output is inserted into the markdown (none in seeksnack).
3. goldmark `Parse` (convert.go:240-253), then `Render` (255-276) into a `render.Context` buffer (see 6.1).
4. `expandShortcodeTokens` (`shortcode.go:738-782`) replaces each `HAHAHUGOSHORTCODE…HBHB` with the shortcode output.
   Quirk #1148: if the token is directly wrapped as `<p>TOKEN</p>`, the `<p>`/`</p>` are removed too. The bounds check
   uses `(k+4) < len(source)` (k is relative, a bug). Port it verbatim.
5. Summary handling (`page__content.go:606-632`) produces `.Summary` / `ContentWithoutSummary`. `.Content` for pages without
   a divider is the full string. **Unused by seeksnack**.

Pages with an empty body (`itemsStep2` empty, e.g. `latesturl.md`, 38 empty bodies) produce `""` without calling goldmark
(`page__content.go:552-555`).

### 4.2 `.Plain`

`contentPlain` (`page__content.go:798-863`): `result.plain = tpl.StripHTML(string(rendered.content))`.

`tpl.StripHTML` (`NH/tpl/template.go:99-134`):

```
if !strings.ContainsAny(s, "<>") { return s }
pre := strings.NewReplacer("\n"," ", "</p>","___hugonl_", "<br>","___hugonl_", "<br />","___hugonl_").Replace(s)
s = htmltemplate.StripTags(pre)                 // html/template stripTags state machine (htmltemplate/html.go:181-229)
if pre != original { s = strings.ReplaceAll(s, "___hugonl_", "\n") }
collapse runs of unicode.IsSpace runes: keep the FIRST rune of each run (so "\n " -> "\n", NBSP may survive), drop the rest
```

Entities are **not** decoded, so `.Plain` contains `&rsquo;`, `&amp;`, `&#160;`, and so on. Verified in golden `index.json`:
`"Alice&rsquo;s Coconut Mochi # Alice&rsquo;s …"`. The ` # ` comes from the heading hook's anchor link text. `StripTags` must be the
html/template context machine (it handles `<div title="1>2">` and RCDATA elements), so reuse the template engine port's
implementation.

### 4.3 `markdownify`

`NH/tpl/transform/transform.go:180-194`: `home.RenderString(ctx, s)` (`page__content.go:906-1059`, default options
`display: inline`; converter = the home page's converter with the home page's render hooks; `RenderTOC=false`; runs shortcode
lexing only if `pageparser.HasShortcode`), then `TrimShortHTML` (`NH/helpers/content.go:167-186`):

```
if bytes.Count(input, "<p>") == 1 { input = TrimSpace(input); if HasPrefix "<p>" && HasSuffix "</p>" { strip both; TrimSpace } }
```

The display-inline path in RenderString also calls `TrimShortHTML` (lines 1051-1057). So markdownify effectively trims twice,
which is idempotent. Golden examples: `It is soooo yummyyyy!!!!` and `I want to have in india I&rsquo;m looking for this specially Yubai melon steam cake`.
Each RenderString call parses with a fresh ID factory.

### 4.4 Content caching / scope

Content is rendered once per (source, markup scope + output-format name) (`page__content.go:522-524`, used at 526-531). Every seeksnack page
has only the HTML output format, and the hooks have no output-format variants. So one rendering per page is reused by
`.Content` and by `.Plain` in `index.json`.

---

## 5. Shortcodes

### 5.1 Usage inventory (grep of `content/`)

```
content/privacy.md:79  ... posted at [{{< ref "privacy.md" >}}]({{< ref "privacy.md" >}}).
content/terms.md:11    ... at any time at [{{< ref "terms.md" >}}]({{< ref "terms.md" >}}). ...
content/terms.md:15    ... found at [{{< ref "privacy.md" >}}]({{< ref "privacy.md" >}}). ...
```

These are the only `{{<`/`{{%` occurrences, 6 shortcode calls in total. `search.md` contains `{{ .Content | plainify | jsonify }}` and
`{{end}}`. These are **not** shortcodes (no `<`/`%` after `{{`) and render as literal text inside a paragraph (with typographer
applied: `&ldquo;contents&rdquo;:{{ .Content | plainify | jsonify }}`).

### 5.2 Processing of `ref`

* Embedded template `NH/tpl/tplimpl/embedded/templates/_shortcodes/ref.html` = `{{ ref . .Params }}` (19 bytes, no trailing newline).
* `tpl/urls.Ref` (`NH/tpl/urls/urls.go:94-105`, `refArgsToMap` 122-163): positional `["privacy.md"]` becomes `{path:"privacy.md", outputFormat:""}`.
* `siteRefLinker.refLink` (`NH/hugolib/site.go:879-954`): `url.Parse(ref)`, `getPageRef(p, path)`, `target.Permalink()`
  (absolute, `relative=false`). Result `https://seeksnack.com/privacy/` or `https://seeksnack.com/terms/`.
* The output is executed as an html/template shortcode, so the string is HTML-escaped (no-op here).
* Goldmark sees `[HAHAHUGOSHORTCODE<pid>s0HBHB](HAHAHUGOSHORTCODE<pid>s1HBHB)`. The embedded link hook receives destination =
  placeholder. `urls.Parse` keeps it, it is not absolute, and `GetPage`, `Resources.Get` and `resources.Get` all return nil,
  so `href` = placeholder. Link text = placeholder. After rendering, `expandShortcodeTokens` swaps both.
  Final: `<a href="https://seeksnack.com/privacy/">https://seeksnack.com/privacy/</a>` (verified, `$W/contentsite-out/privacy/index.html`).
* Placeholders must stay free of markdown-significant characters (alphanumerics only), so goldmark/typographer/linkify
  never touch them. The `pid` value never reaches the output. Any unique number works.

### 5.3 Shortcode lexer rules to port (`pagelexer_shortcode.go`, 366 lines)

`{{<` … `>}}` (no-markup) and `{{%` … `%}}` (markdown). `{{</* … */>}}` is a comment (emitted as literal text without the
comment markers). Params are positional or named (`k="v"`, `` k=`raw` ``, bare words, floats, `\"` escapes), and named and
positional cannot be mixed. Closing is `{{< /name >}}`, self-closing is `{{< name />}}`. Inner content applies to
`IsInner` templates. For seeksnack only the positional quoted-string form is exercised, but port the whole lexer (≈1.3k
lines incl. `pagelexer.go`, `pageparser.go`, `item.go`) since it is also the front-matter splitter.

Hugo-embedded shortcodes (`NH/tpl/tplimpl/embedded/templates/_shortcodes/`: comment, details, figure, gist, highlight,
instagram, param, qr, ref, relref, twitter, vimeo, x, youtube, …) are unused except `ref`.

---

## 6. Render hooks: invocation contract

### 6.1 Text capture mechanism (`NH/markup/goldmark/internal/render/context.go`)

The renderer writes into `render.Context{BufWriter: *bytes.Buffer}`. For Link, Image, Heading, Blockquote and TableCell:

* **entering**: `ctx.PushPos(ctx.Len())`, then `WalkContinue`, so children render normally into the same buffer.
* **exiting**: `text := ctx.PopRenderedString()` (slice `[pos:]`, then `Truncate(pos)`), then call the hook, which writes into the same buffer.

So `.Text` is **the inner HTML as rendered by the normal renderers**, including nested hooks such as a link inside a
heading. A Rust port needs a single growable `Vec<u8>` output with a position stack. Table values use a separate
`PushValue/PopValue/PeekValue` stack keyed by node kind. Ordinals are counted per kind per render (`GetAndIncrementOrdinal`).

### 6.2 Heading hook (site `render-heading.html`)

Template (2 lines, trailing `\n` after `<hr>`):

```
<h{{ .Level }} class="anchor-link mt-3" id="{{ .Anchor | safeURL }}">{{ .Text | safeHTML }} <a href="#{{ .Anchor | safeURL }}" aria-label="Anchor" data-anchorjs-icon="#" class="ps-1">#</a></h{{ .Level }}>
<hr>
```

Called from `hookedRenderer.renderHeading` (`render_hooks.go:477-524`). Context `headingContext`: `Page`, `PageInner`
(from the hugocontext pid stack, same page here), `Level` (1-6), `Anchor` = the node's `id` attribute bytes, `Text` =
captured inner HTML, `PlainText` = `render.TextPlain(n, source)`, `Attributes` = node attributes (only `id` here).
Nothing else is written. The default `</hN>\n` is **not** emitted when a hook exists.

Escaping consequences (html/template, done by the template engine):
* `id="…"`: attribute value of a `safeURL` value gets the attr escaper, which leaves Thai unchanged: `id="รสชาต"`.
* `href="#…"`: URL context with a `safeURL` value gets the **URL normalizer**. Non-ASCII is percent-encoded with **lowercase** hex:
  `href="#%e0%b8%a3%e0%b8%aa%e0%b8%8a%e0%b8%b2%e0%b8%95"` (verified in golden, Thai pages).

### 6.3 Image hook (site `render-image.html`)

Called from `hookedRenderer.renderImage` (`render_hooks.go:149-212`). Context `imageLinkContext`: `Destination` =
`string(n.Destination)` (**raw source bytes**: no backslash-unescaping, no entity resolution, see `GM/parser/link.go:333-372`),
`Title` = raw title bytes, `Text` = captured rendered alt children, `PlainText`, `Attributes` = node attributes minus
internal `_h__*` ones (`filterInternalAttributes`, 214-223), `IsBlock` (true only when
`wrapStandAloneImageWithinParagraph=false` and the image is its paragraph's only child, which is never the case here), `Ordinal`
(document order, set by `images/transform.go:41-76`).

With `wrapStandAloneImageWithinParagraph=true` a stand-alone image stays inside its paragraph. Output:
`<p>` + hook output + `</p>\n`. The template output ends with `</picture>\n` + `\n`, so the golden raw HTML is `…</picture>\n\n</p>`.

The hook does `.Page.Resources.Get .Destination`, resizes a watermark, overlays it, and converts to webp. That is the imaging
subsystem, invoked synchronously during markdown rendering. `alt="{{ .Text }}"` is an HTML-typed value in an attribute, so the
template engine applies stripTags + htmlNorm escaping. No seeksnack alt contains markup or quotes.

### 6.4 Link hook (embedded `_markup/render-link.html`, used in "fallback" mode)

```
{{- $u := urls.Parse .Destination -}}
{{- $href := $u.String -}}
{{- if strings.HasPrefix $u.String "#" -}}  {{- $href = printf "%s#%s" .PageInner.RelPermalink $u.Fragment -}}
{{- else if and $href (not $u.IsAbs) -}}   … GetPage / Resources.Get / resources.Get lookup, append ?query / #fragment …
{{- end -}}
<a href="{{ $href }}" {{- with .Title }} title="{{ . }}" {{- end }}>{{ .Text }}</a>
{{- /**/ -}}
```

(no trailing newline; `{{- /**/ -}}` trims it). Called for:
* `ast.Link` (`render_hooks.go:255-297`): `destination` = raw bytes, `title` = raw bytes, `text` = captured HTML,
  `AttributesHolder: attributes.Empty`.
* `ast.AutoLink` (both `<https://…>` and linkify results) (`render_hooks.go:394-437`): `destination` = `autoLinkURL(n)`,
  which replaces the linkify-inserted protocol `http` with `LinkifyProtocol` **`https`** only when `n.Protocol` is set (bare `www.`
  URLs). Email autolinks get a `mailto:` prefix. `text` = `plainText` = **raw label bytes (NOT HTML-escaped)**.

Parity-relevant behaviour (template engine side, but it shows up in markdown output):
* `urls.Parse` + `.String` = Go `net/url` `Parse` then `URL.String()`. `RawPath` is kept when it is a valid encoding, so
  `Lay's` and `Mala_(seasoning)` survive `String()`. Existing `%E0%B9…` (uppercase) survives.
* The html/template URL normalizer in `href` then escapes `'`, `(`, `)` (and anything outside
  `[A-Za-z0-9-._~!#$&*+,/:;=?@\[\]]` and `%`) as lowercase `%xx`. Golden: `href=https://en.wikipedia.org/wiki/Lay%27s`,
  `href=https://en.wikipedia.org/wiki/Mala_%28seasoning%29`, and Thai `href=http://www.lotte.co.th/product/%E0%B9%84…` (unchanged).
* `title="{{ . }}"` uses the attribute escaper. The 16 titles in content are plain ASCII words.
* All 121 distinct link destinations in content are absolute `http(s)://` or `mailto:`, plus the 2 shortcode placeholders.
  So the `GetPage`/`Resources.Get` branch only runs for placeholders and always yields nil.

Without hooks (`renderLinkDefault`, `render_hooks.go:374-392`) goldmark would emit `util.EscapeHTML(util.URLEscape(dest,true))`,
i.e. `Lay's` and `(…)` unescaped. That is the 2-page difference noted in finding 6.

### 6.5 Blockquote (no hook; Hugo default)

`NH/markup/goldmark/blockquotes/blockquotes.go:61-143`: the inner HTML is captured and `strings.TrimSpace`'d (unicode
whitespace!). `resolveBlockQuoteAlert` checks the regex ``^<p>\[!([a-zA-Z]+)\](-|\+)?[^\S\r\n]?([^\n]*)\n?`` to choose the hook
type `regular`/`alert`. No blockquote hook exists, so the default applies:

```
"<blockquote>\n" + TrimSpace(inner) + "</blockquote>\n"
```

**This differs from goldmark's own renderer.** It emits `…</p></blockquote>` with no newline between them (verified:
`pie/chocky-banana-pie`). With node attributes it would emit `<blockquote` + attrs + `>` and **no** newline. Not reachable
here.

### 6.6 Table (embedded `_markup/render-table.html`)

`NH/markup/goldmark/tables/tables.go`: on Table enter a `hooks.Table{}` is pushed. On header/row enter an empty row is
appended to THead/TBody. On cell exit the captured cell HTML plus alignment (`left|right|center|""`) are appended. On Table
exit the table hook is called with `Attributes` (none), `THead`, `TBody`, `Ordinal`. Hugo panics if no table hook exists.

The embedded template output (whitespace exact, verified byte-identical against Hugo on 119 tables):

```
<table>\n  <thead>
  per header row:   \n      <tr>   per cell: \n          <th[ style="text-align: X"]>TEXT</th>   \n      </tr>
\n  </thead>\n  <tbody>
  per body row:     \n      <tr>   per cell: \n          <td[ style="text-align: X"]>TEXT</td>   \n      </tr>
\n  </tbody>\n</table>\n
```

`$W/mdref/main.go` `tableRenderer` is a Go replica. Attributes, if any, are rendered as ` k="v|html-escaped"`. No seeksnack table has
alignment colons.

### 6.7 Code blocks

`codeblocks.New()` is installed. A fenced block would call the codeblock hook or fall back to the chroma highlighter
(`page__per_output.go:377-383`). **No fenced or indented code in seeksnack.** Recommendation: port the fenced/indented
*parsers* (needed for correct block structure) but make the fenced *renderer* return an explicit "unsupported:
highlighting" error, so a later content change fails loudly instead of silently diverging.

---

## 7. Heading ID algorithm (exact)

1. Hugo attributes transformer (`internal/extensions/attributes/attributes.go:118-169`, priority 100) walks the AST. For each
   `Heading` (and `DefinitionTerm`) without an `id` attribute it calls `generateAutoID`. With an explicit `{#id}` it calls
   `pc.IDs().Put(id)`.
2. `textHeadingID` (lines 190-204): `text := render.TextPlain(heading, source)`. If `n.Lines().Len() > 1` (multi-line setext
   heading), keep only the part after the last `\n`.
3. `render.TextPlain` (`internal/render/context.go:296-327`): for each **direct child** c of the heading, `textPlainTo(c)`:
   * `RawHTML`: `strings.TrimSpace(tpl.StripHTML(raw))`
   * `String` (typographer output such as `&rsquo;`): its value
   * `Text`: segment bytes, plus `"\n"` if soft or hard line break
   * `Emoji`: short name
   * **default (Emphasis, Link, Image, CodeSpan, AutoLink, Strikethrough…): `textPlainTo(c.FirstChild())`, which only
     follows the first child and does NOT iterate its siblings.**

   Then `util.ResolveEntityNames` (named entities only, `GM/util/util.go:641-671`), so `&rsquo;` becomes `’` and `&amp;` becomes `&`.
   Numeric references are NOT resolved here.
4. `pc.IDs().Generate(text, KindHeading)` becomes `sanitizeAnchorNameWithHook` (`NH/markup/goldmark/autoid.go:44-85, 121-148`) with idType `github`:
   ```
   b = bytes.TrimSpace(b)                  // unicode.IsSpace, trims NBSP too
   for each rune r (utf8.DecodeRune; invalid byte -> U+FFFD, width 1):
       r == '-' || r == ' '          -> '-'
       r == '_' || unicode.IsLetter(r) || unicode.IsDigit(r) -> unicode.ToLower(r)
       otherwise                     -> dropped
   if empty -> "heading"   ("term" for definition terms, "id" otherwise)
   if already in vals -> append "-" then the smallest i>=1 such that "<base>-<i>" is not in vals
   put(result) (records duplicates list)
   ```
   `vals` is per document (per `Parse`).
5. The resulting bytes are set as attribute `id` and become the hook's `.Anchor`.

Verified examples (all from `$W/t1.out.html`, `$W/t2.out.html` or golden):

| Heading source | id |
|---|---|
| `### **Snack Jack - Sour Cream Flavor (Green Pea Snack)**` | `snack-jack---sour-cream-flavor-green-pea-snack` |
| `### รสชาติ` | `รสชาต` (U+0E34 SARA I is Mn, dropped) |
| `### สแน็คแจ๊ค รสดั้งเดิม (ขนมถั่วลั่นเตาอบกรอบ)` | `สแนคแจค-รสดงเดม-ขนมถวลนเตาอบกรอบ` |
| `### Squidy - Seasoned Roller Squid Hot&Spicy` | `squidy---seasoned-roller-squid-hotspicy` |
| `### White Koala's March (… Filling )` | `white-koalas-march-…-filling-` (`’` dropped, trailing space inside parens becomes `-`) |
| `### Pocky … ([Thai Glico 50th anniversaries](https://…))` | `…-thai-glico-50th-anniversaries` (link's first child text) |
| `## **Strong *em* more** tail` | `strong--tail` (first-child quirk) |
| `## ![alt *x*](img.png "T") img` | `alt--img` |
| `## &amp; &copy; entity` | `--entity` |
| `## Ünïcödé İstanbul ǅ` | `ünïcödé-istanbul-ǆ` (Go `ToLower(İ)='i'`, `ToLower(ǅ)='ǆ'`) |
| `## 🍫` | `heading` |
| `## Dup`, `## Dup`, `## dup-1` | `dup`, `dup-1`, `dup-1-1` |
| setext `Setext line one\nline two` | `line-two` |
| `### Edit layouts/_default/index.JSON` | `edit-layouts_defaultindexjson` |

Only markdown headings are deduplicated. Headings emitted by layouts (`id="nutrition-fact"`, …) are not part of this set.

---

## 8. goldmark behaviour catalogue (what seeksnack exercises)

Evidence: `$W/corpus-nohooks/**` (all 251 files through neohugo's converter), `$W/t1.md` → `$W/t1.out.html`, `$W/t2.md` → `$W/t2.out.html`.

### 8.1 Text escaping and entities (`GM/renderer/html/html.go:802-929`, `GM/util/util.go:536-565`)
* `Writer.Write` (used for Text/String nodes): handles backslash escapes (a `\` before an ASCII punct is dropped), NUL becomes
  U+FFFD, `&#x..;` (<7 hex digits) and `&#..;` (<8 digits) are decoded to the rune and written through `escapeRune`, a named
  entity **found in goldmark's 2124-entry HTML5 table** is decoded to its UTF-8 characters (so `&copy;` becomes `©`), and an
  unknown one stays as text and is escaped (`&nosuchentity;` becomes `&amp;nosuchentity;`). Everything else goes through
  `RawWrite`, which escapes only `"`→`&quot;`, `&`→`&amp;`, `<`→`&lt;`, `>`→`&gt;` (`htmlEscapeTable`).
* An unbalanced `"` stays as `&quot;`. Golden: `…further testing was recommended&quot;.` (`ingredients/ins-124`).
* Typographer `String` nodes are `SetCode(true)`, so they are written **raw** (`&rsquo;` stays an entity, html.go:689-704).
  The minifier keeps named entities (golden has `Lay&rsquo;s`).
* `\*22%` becomes `*22%`. `\&` becomes `&amp;`.

### 8.2 Paragraph lines, breaks, whitespace (`GM/parser/parser.go:1131-1249`)
* The last text segment of each line is `TrimRightSpace` (ASCII space, `\t`, `\n`, `\r` via `spaceTable`; **not** NBSP,
  `\v` or `\f`). Inner double spaces are kept (`</strong>  Contains`).
* Hard break: line ending in `\` becomes `<br>\n` (seen 2×), and two trailing spaces also give `<br>\n`. A soft break becomes `\n`.
* **Line-head semantics:** after any inline node is produced the loop does `goto retry` with `i==0`, so the next byte is
  treated as a "line head" for parsers registered on `' '`. That is why linkify fires right after `**`:
  `**http://bold.com**` becomes `<strong><a href="http://bold.com">…</a></strong>`.

### 8.3 Emphasis / delimiters (`GM/parser/delimiter.go`)
`ScanDelimiter` flanking uses `util.IsPunctRune = unicode.IsSymbol || unicode.IsPunct` and
`util.IsSpaceRune = (r<=256 && spaceTable) || unicode.IsSpace`. The character before a delimiter comes from
`PrecendingCharacter` (`GM/text/reader.go:182-197, 392-416`); line start counts as `'\n'`. NBSP after a closing `**`
(`**Shelf Life:**\u00a0 6 Months`, 223 NBSPs in content) is Unicode whitespace, so it is right-flanking. Thai letters are
neither punct nor space. `_` intraword rules apply (`__under__score_word_` becomes `_<em>under__score_word</em>`).
`ProcessDelimiters` / `CalcComsumption` implement the CommonMark "rule of 3". Port verbatim.

### 8.4 Typographer (`GM/extension/typographer.go:166-324`), exact rules, in order, for the current byte `c`:
1. `---` becomes EmDash, then `...` becomes Ellipsis (else `.` yields nil), then `<<` becomes `&laquo;` (else nil), `>>` becomes `&raquo;` (else nil), and `--` becomes EnDash.
2. `'` or `"` runs `ScanDelimiter(line, before, 1, typographerProcessor)`. The unclosed counters (Single, Double) are per parse context and
   are **reset in `CloseBlock`** (end of each block's inline parse).
3. `'` (Apostrophe substitution enabled):
   a. decade `'90s`: `CanOpen && !CanClose && len>3 && isdigit(l[1]) && isdigit(l[2]) && l[3]=='s'` and the rune after is space/punct or end gives Apostrophe.
   b. `'twas/'em/'net/'l…`: `len>1 && (unicode.IsPunct(before)||unicode.IsSpace(before)) && l[1] in {t,e,n,l}` gives Apostrophe.
   c. `(IsDigit(before)||IsLetter(before)) && IsLetter(rune at 1)` gives Apostrophe (`Lay's`, `It's`, Thai letters count).
   d. `CanOpen && !CanClose` gives LeftSingleQuote, except that `'s|'m|'t|'d` followed by punct/space/end, or `'ve|'ll|'re` + punct/space/end, gives RightSingleQuote.
      A left quote increments Single.
   e. **(operator-precedence bug, keep it)** `len>1 && IsSpace(l[0]) || IsPunct(l[0]) && (len>2 && !IsDigit(rune at 1))`, where
      `l[0]` is `'` (punct). So any remaining `'` with ≥3 bytes left and a non-digit next becomes RightSingleQuote.
   f. `Single>0` and (`CanClose&&!CanOpen` or "maybeClose") becomes RightSingleQuote and decrements Single.
   Result `5'10"` becomes `5'10&quot;` (the `'` between digits is untouched).
4. `"`: `CanOpen&&!CanClose` gives LeftDoubleQuote (Double++). Else if `Double>0` and (close or maybeClose) gives
   RightDoubleQuote (Double--), except the `"Monitor 21""` rule (`l[1]=='"' && IsDigit(before)`) returns nil.
   Otherwise nil, and the `"` renders as `&quot;`.
All emitted nodes are `String` with `IsCode`, so they are written raw.

### 8.5 Linkify (`GM/extension/linkify.go:163-306`)
* Skipped inside link labels (`pc.IsInLinkLabel()`). A leading trigger char (space, `*`, `_`, `~`, `(`) is consumed and
  re-appended as text.
* `http:`/`https:`/`ftp:` prefix: ``^(?:http|https|ftp)://[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-z]+(?::\d+)?(?:[/#?][-a-zA-Z0-9@:%_+.~#$!?&/=\(\);,'">\^{}\[\]`]*)?``.
  `www.` prefix: ``^www\.[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-z]+(?:[/#?][-a-zA-Z0-9@:%_\+.~#!?&/=\(\);,'">\^{}\[\]`]*)?`` and `Protocol="http"`.
* Trim: trailing `.` (one), unbalanced trailing `)` (counts parens), trailing `&entity;`, then strip any trailing run of
  `? ! . , : * _ ~`.
* Email fallback: `util.FindEmailIndex` (local part via `emailTable`, domain regex
  `^[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*`). The domain must contain a `.`.
  A trailing `.` is trimmed, and the match is rejected if followed by `-` or `_`.
* Seeksnack hits: `1. http://www.bjcfoods.co.th/`, `1. https://www.yubari-melon.or.jp/`, and in terms.md
  `blackb1rd@blackb1rd.me.` (becomes `mailto:`, trailing `.` excluded) and `http://www.copyright.gov.` (trailing `.` excluded).

### 8.6 Tables (`GM/extension/table.go`)
A paragraph transformer (priority 200) scans paragraph lines for a delimiter row (regexes `^\s*\:\-+\s*$` etc. per cell). The
header row must have the same cell count as the delimiter row. Lines before the header stay a paragraph. Body rows with extra
cells are truncated. Missing cells are padded with `ast.NewTableCell()` (`table.go:243-245`). **Quirk:** padded cells get
`AlignNone`, not the column's alignment. Cell content is trimmed (ASCII `TrimLeftSpace`/`TrimRightSpace`). `\|` escapes are handled with a separate AST transformer inside code spans.

### 8.7 Lists / blocks
Tight vs loose (`GM/parser/list.go`): tight lists render `<li>text</li>`, loose lists render `<li>\n<p>…</p>\n</li>`. A TextBlock ends with `\n`
only if it has a next sibling. `<ol start="N">` appears when the start is not 1. Thematic break becomes `<hr>\n`. HTML blocks: 7 CommonMark
kinds (`GM/parser/html_block.go:13-229`). `<!--StartFragment-->` lines are type-2 blocks copied verbatim via
`SecureWrite` (NUL becomes U+FFFD). They are later removed by the minifier (`keepComments=false`). Inline raw HTML (`<i class="fas fa-star"></i>`)
is recognized by regexes in `GM/parser/raw_html.go:50-153` and written verbatim.

### 8.8 Footnotes / deflist / tasklist / strikethrough (enabled, unused)
Output formats for reference (`$W/t1.out.html`): `<sup id="fnref:1"><a href="#fn:1" class="footnote-ref" role="doc-noteref">1</a></sup>`,
`<div class="footnotes" role="doc-endnotes">\n<hr>\n<ol>\n<li id="fn:1">\n<p>…&#160;<a href="#fnref:1" class="footnote-backref" role="doc-backlink">&#x21a9;&#xfe0e;</a></p>\n</li>\n</ol>\n</div>`;
`<dl>\n<dt>Term</dt>\n<dd>Definition</dd>\n</dl>`; `<li><input disabled="" type="checkbox"> todo</li>`; `<del>…</del>` (single `~` too).

---

## 9. Porting plan

### 9.1 goldmark v1.7.12: port line-by-line (MIT license)

Non-test line counts (`GM/…`):

| Package / file | Lines | Needed? |
|---|---|---|
| `markdown.go` | 141 | yes (wiring) |
| `ast/ast.go`, `block.go`, `inline.go` | 521 + 545 + 573 | yes |
| `parser/parser.go` | 1249 | yes (block open/continue/close, lazy continuation, inline loop) |
| `parser/attribute.go` | 329 | yes (`WithAttribute` for headings) |
| `parser/atx_heading.go`, `setext_headings.go` | 248 + 126 | yes |
| `parser/list.go`, `list_item.go` | 287 + 90 | yes |
| `parser/blockquote.go`, `paragraph.go`, `thematic_break.go` | 70 + 72 + 75 | yes |
| `parser/code_block.go`, `fcode_block.go` | 102 + 122 | yes (block structure) |
| `parser/html_block.go`, `raw_html.go` | 229 + 153 | yes |
| `parser/link.go`, `link_ref.go`, `auto_link.go` | 449 + 152 + 42 | yes |
| `parser/delimiter.go`, `emphasis.go`, `code_span.go` | 238 + 50 + 84 | yes |
| `renderer/renderer.go`, `renderer/html/html.go` | 174 + 954 | yes (skip EastAsianLineBreaks) |
| `text/reader.go`, `segment.go` | 701 + 233 | yes. Segments with `Padding` (tab expansion) and `ForceNewline` must match |
| `util/util.go` | 1044 | yes (escape tables, URLEscape, case folding, FindClosure, BytesFilter) |
| `util/html5entities.gen.go` (90 KB data), `unicode_case_folding.gen.go` (28 KB) | data | yes, **generate Rust tables from these exact files** |
| `util/util_cjk.go` | 469 | no (CJK disabled) |
| `extension/table.go` + `ast/table.go` | 555 + 158 | yes |
| `extension/linkify.go` | 322 | yes |
| `extension/typographer.go` | 348 | yes |
| `extension/strikethrough.go` + ast | 118 + 29 | yes (cheap) |
| `extension/tasklist.go` + ast | 120 + 35 | yes (cheap) |
| `extension/definition_list.go` + ast | 274 + 83 | yes (cheap, unused) |
| `extension/footnote.go` + ast | 691 + 138 | recommended (unused; lower priority) |
| `extension/cjk.go`, `gfm.go` | 72 + 18 | no |
| **Total needed** | **≈ 11.5k** | |

Test fixtures to port as Rust tests: `GM/_test/spec.json` (CommonMark, 5217 lines), `GM/_test/extra.txt` (863),
`GM/_test/options.txt`, `GM/extension/_test/{table,linkify,typographer,strikethrough,tasklist,definition_list,footnote}.txt`.

### 9.2 Hugo glue: port line-by-line

| File | Lines | What |
|---|---|---|
| `NH/markup/goldmark/convert.go` | 327 | instance composition, Parse/Render, typographer map |
| `NH/markup/goldmark/render_hooks.go` | 552 | hooked renderer (heading/link/autolink/image + defaults) |
| `NH/markup/goldmark/autoid.go` | 160 | sanitizer + idFactory |
| `NH/markup/goldmark/internal/extensions/attributes/attributes.go` | 204 | auto-ID transformer |
| `NH/markup/goldmark/internal/render/context.go` | 327 | buffer/pos stack, `TextPlain` (with its quirk) |
| `NH/markup/goldmark/hugocontext/hugocontext.go` | 317 | `{{__hugo_ctx}}` parser/renderer (also owns RawHTML/HTMLBlock rendering) |
| `NH/markup/goldmark/images/transform.go` | 76 | image ordinal / isBlock |
| `NH/markup/goldmark/blockquotes/blockquotes.go` | 196 | blockquote trim + alert regex |
| `NH/markup/goldmark/tables/tables.go` | 175 | table hook plumbing |
| `NH/markup/goldmark/toc.go` + `NH/markup/tableofcontents/tableofcontents.go` | 141 + 277 | optional (unused) |
| `NH/markup/goldmark/codeblocks/render.go` | 179 | stub (error) |
| `NH/markup/goldmark/goldmark_config/config.go`, `NH/markup/markup_config/config.go` | 309 + 117 | config + legacy normalization |
| `NH/markup/converter/converter.go`, `hooks/hooks.go`, `NH/markup/internal/attributes/attributes.go` | 158 + 235 + 225 | contexts/interfaces |
| `NH/hugolib/shortcode.go`, `shortcode_page.go` | 794 + 131 | placeholders, rendering, `expandShortcodeTokens` |
| `NH/hugolib/page__content.go` | 1191 | pipeline (contentToRender, contentRendered, contentPlain, RenderString) |
| `NH/parser/pageparser/*.go` | ≈1.5k | lexer (front matter split + shortcodes + summary divider) |
| `NH/helpers/content.go` | 186 | `TrimShortHTML`, `TotalWords` |
| `NH/tpl/template.go` `StripHTML` | 35 | plus `htmltemplate.stripTags` (template engine) |
| `NH/tpl/transform/transform.go` `Markdownify` | 15 | |
| `NH/tpl/urls/urls.go` Ref + `NH/hugolib/site.go:879-954` refLink | ≈120 | `ref` shortcode |
| embedded templates `_markup/render-link.html`, `_markup/render-table.html`, `_shortcodes/ref.html` | data | ship verbatim, executed by the template engine |

Not needed: goldmark-emoji (only registered for TOC rendering), passthrough, extras, CJK, blackfriday anchors, AsciiDoc/Pandoc/RST/Org converters, chroma.

### 9.3 Architecture notes for Rust

* AST: an arena (`Vec<Node>` + `u32` ids) with parent/first/last/prev/next links. goldmark mutates the tree heavily
  (`InsertAfter`, `ReplaceChild`, `RemoveChild`, `MergeOrAppendTextSegment`, delimiters that are nodes temporarily in the tree).
  Node kinds must be extensible (Hugo adds HugoContext, table/footnote/tasklist kinds).
* Keep goldmark's **segment model**: nodes reference byte ranges of the source, not copied strings. `Text` flags are
  soft/hard/raw/code. `String` nodes are owned bytes.
* Keep the **trigger-byte dispatch tables** and priority ordering exactly (see 3.1). The priority ordering *is* behaviour.
* Hooks: the renderer takes a callback interface
  `fn heading(&HeadingCtx, &mut Vec<u8>)`, `link`, `image`, `table`, `blockquote`, …. The Hugo layer implements them by
  executing the template engine with a context object exposing exactly the fields in section 6. Hooks must be called
  synchronously in document order. Image hooks trigger image processing.
* Unicode: generate `is_letter`, `is_digit`, `is_space`, `is_punct`, `is_symbol`, `to_lower` tables **from Go 1.27's `unicode`
  package** (`unicode.Version == "17.0.0"`) with a small Go generator, and use those in goldmark, autoid, typographer and StripHTML.
  Do not use Rust std (see 10.2).

---

## 10. Rust crates: what is safe

### 10.1 Markdown engines: NOT safe (evidence)

`$W/cmpcrates.py` ran pulldown-cmark 0.13.4 (tables, strikethrough, tasklists, footnotes, smart punctuation, deflist) and comrak
0.44 (GFM exts, autolink, smart, header_ids, unsafe) over all 251 bodies and compared with the goldmark corpus:

| crate | byte-identical files | files still different after normalization (drop ids/anchors, collapse whitespace, unescape entities) |
|---|---|---|
| pulldown-cmark | 41/251 (38 of them are empty bodies) | 6 (no linkify, …) |
| comrak | 41/251 (38 of them are empty bodies) | 2, e.g. an unbalanced `"`: goldmark `&quot;` vs comrak `”` |

Beyond raw output, the Hugo layer needs goldmark-specific AST semantics: capturing hook `.Text` from inner rendering, the
`TextPlain` first-child quirk, the idFactory, table cells as hook data, the blockquote trim, and typographer `String` nodes. So a
faithful goldmark port is the only safe path. `markdown-rs` has the same problem.

### 10.2 Unicode: std is NOT safe

`$W/unicmp` compared Go 1.27 `unicode` with Rust 1.98 std over all scalar values:
* `unicode.IsLetter` vs `char::is_alphabetic`: **1749 mismatches**, including Thai U+0E31, U+0E34–U+0E3A, U+0E4D. Rust would keep them in
  anchors, so `รสชาติ` would get the wrong ID.
* `unicode.IsDigit` vs `char::is_numeric`: 1154 mismatches (², ³, ¹, ¼, Bengali fractions…).
* `unicode.ToLower` vs `char::to_lowercase`: U+0130 (`İ` gives `i` in Go, `i̇` in Rust).
* `unicode.IsSpace` vs `char::is_whitespace`: 0 mismatches, so safe.
* Crates such as `unicode-general-category` / `unicode-properties` are acceptable **only if pinned to Unicode 17.0.0 and
  cross-checked** against a Go dump. Generating tables from Go is simplest and exact.

### 10.3 Regex: `regex` crate is safe with care

goldmark uses Go RE2 regexes in `html_block.go:79-96`, `raw_html.go:50-54`, `linkify.go:14-16`, `table.go:136-139`,
`tasklist.go:16`, `util.go:769`, and Hugo uses one in `blockquotes.go:173` and `hugocontext.go:104`. Rust `regex` (leftmost-first, like
RE2) matches identically **if** you use `regex::bytes::Regex` and replace every Perl class:
* Go `\s` = `[\t\n\f\r ]`. Rust Unicode `\s` includes NBSP, and Rust ASCII `(?-u:\s)` includes `\v`. So write `[\t\n\x0C\r ]` explicitly
  (matters for `taskListRegexp ^\[([\sxX])\]\s*`, the table delimiter regexes, and the blockquote-alert `[^\S\r\n]`, with 223 NBSPs in content).
* Go `\d` is ASCII. Rust `\d` is Unicode, so use `[0-9]` (linkify `(?::\d+)?`).
* `(?i)` folding is Unicode simple folding in both (Kelvin sign etc.). Acceptable.
* `$` without `(?m)` means end of text in both. `.` excludes `\n` in both.
* Alternatively hand-write the scanners. The patterns are small.

### 10.4 Other crates

| Need | Crate | Verdict |
|---|---|---|
| byte search | `memchr` | safe |
| byte strings | `bstr` | safe (only as a utility) |
| HTML escaping | `html-escape`, `htmlescape`, `askama_escape` | **not safe** (different sets, e.g. `'`→`&#x27;`). goldmark escapes exactly `" & < >`. Hand-write it (4 lines) |
| entity table | `entities`, `html5ever` tables | **not safe**. Must be goldmark's exact 2124-name map (generate from `GM/util/html5entities.gen.go`) |
| percent-encoding | `percent-encoding`, `url` | **not safe as drop-ins**. goldmark `URLEscape` uses Go `url.QueryEscape` per multibyte char (uppercase hex, skips valid `%XX`, with the `IsHexDecimal(v[i+1])` checked twice bug). html/template's normalizer uses **lowercase** hex. Go `net/url` Parse/String semantics (RawPath validity, `validEncoded`) are not WHATWG. Hand-port `net/url` (subset) and both escapers |
| case folding for link refs | `unicase`, `caseless` | **not safe**. Use goldmark's `unicode_case_folding.gen.go` table (1530 entries) |
| regex | `regex` | safe with the class rewrites above |
| markdown | comrak / pulldown-cmark / markdown-rs | **not safe** (10.1) |

---

## 11. Parity risks (ranked)

1. **Heading IDs**: Go Unicode tables (IsLetter/IsDigit/ToLower), the `TextPlain` first-child quirk, `ResolveEntityNames` on
   typographer entities, `bytes.TrimSpace` (Unicode), and dedup order. 122 Thai heading IDs on 71 pages, and IDs go into `hugo_stats.json`
   (purgecss input) and `href="#…"` (lowercase `%xx` normalizer).
2. **Typographer heuristics**, including the precedence bug at `typographer.go:276`, per-block counter reset, `'90s`, `'twas`, `"Monitor 21""`.
   Applied to every paragraph. Unbalanced quotes become `&quot;`.
3. **Embedded link hook semantics**: Go `net/url` Parse/String plus html/template URL normalizer (`'`→`%27`, `(`→`%28`, keep `%E0`). Autolink `.Text` is
   raw (unescaped) label. `LinkifyProtocol` rewrite only for `www.` links.
4. **Hook plumbing whitespace**: heading hook adds `\n<hr>\n` with no default `</hN>\n`, image hook leaves `\n\n` before `</p>`, the table template's exact
   indentation, and the link hook has no trailing newline. The minifier collapses most whitespace but not inside all contexts; `.Plain` (index.json)
   depends on the `</p>` and `<br>` placement.
5. **Blockquote rendering** differs from goldmark (`TrimSpace` + no newline before `</blockquote>`).
6. **Emphasis flanking with NBSP / Thai / punctuation** (223 NBSPs right after `**…:**`). Uses `unicode.IsSpace/IsPunct/IsSymbol`.
7. **Linkify boundaries** (trailing `.`, `)`, `;`) and line-head semantics after inline nodes.
8. **Entities**: goldmark's own HTML5 table, numeric reference limits (`<7` hex / `<8` decimal digits), invalid codepoints become U+FFFD.
9. **Regex class semantics** (`\s`, `\d`) if the `regex` crate is used naively.
10. **`.Plain`**: must use html/template `stripTags` and first-whitespace-rune collapse, and must not decode entities.
11. **Shortcode placeholder expansion**, including the `<p>` unwrapping and the `(k+4)` bug. Only `ref` is exercised. `ref` must return the absolute
    permalink of the page in the **same language**.
12. **markdownify**: home page's converter and hooks, `TrimShortHTML` rule (`Count("<p>")==1`).
13. **Image hook side effects / hashes**: markdown order drives image processing. See finding 7: golden image names match a *cold* resources cache.
14. **Sort stability**: goldmark uses non-stable `sort.Slice` for priorities. Safe now (no ties on the same key). Keep a test.
15. **hugo_stats.json → purgecss**: any missing or extra tag, class or id in markdown output can change the inline CSS of **every** page.

---

## 12. Test assets and how to use them

* **`$W/mdref/`**: Go harness (module with `replace github.com/neohugo/neohugo => NH`). It renders markdown with neohugo's real
  `markup/goldmark` converter and seeksnack's `[markup]` config. Hooks: table (Go replica of embedded template), codeblock
  (chroma), others nil (goldmark/Hugo defaults).
  `./mdref [-toc] [-plain] file.md…` strips front matter with neohugo's pageparser. `./mdref -stdin < body.md` renders raw markdown.
  Build: `cd $W/mdref && go build -o mdref .`.
  **Use it as the differential oracle for the Rust goldmark port**: fuzz or feed arbitrary markdown to both and compare bytes.
* **`$W/corpus-nohooks/<content path>.html`**: mdref output for all 251 content files (no link/heading/image hooks, no shortcode expansion).
* **`$W/contentsite/`** and **`$W/contentsite-out/`**: derived site (seeksnack content, i18n, data, `assets/images`, `_markup` hooks, shortcodes;
  layouts reduced to `{{ .Content }}\n=====PLAIN=====\n{{ .Plain | safeHTML }}\n=====END=====`; `canonifyURLs=false`; only HTML outputs;
  RSS/sitemap/robots/404 disabled). Built with the Go binary. This is the exact per-page `.Content` and `.Plain` with the real hooks
  (image hashes = cold cache, matching golden). `$W/contentmap.json` maps content file to output file.
* **`$W/cmphooks.py`**: shows that `corpus-nohooks` plus 3 hook-emulation rules equals `contentsite-out` on 242/246 pages. The remaining 4 are
  explained (link hook URL normalization ×2, `ref` shortcode ×2).
* **`$W/t1.md`/`t1.out.html`, `$W/t2.md`/`t2.out.html`**: edge-case inputs and oracle outputs (typographer, linkify, entities, IDs, dedup,
  first-child quirk, footnotes, deflist, tasklist, alignment, blockquote alert, lists, breaks).
* **`$W/cmpcrates.py`, `$W/pdtest`, `$W/cmtest`**: the comrak/pulldown comparison. **`$W/unicmp`**: Go vs Rust Unicode dump comparison.
* **`$W/hashtest/`**: cold vs warm resources-cache reproduction of image hash names.

Recommended test ladder for the Rust port:
1. goldmark unit fixtures (spec.json, extra.txt, extension `_test/*.txt`).
2. `mdref -stdin` differential fuzzing (random CommonMark + Thai + NBSP + quotes).
3. The 251-file `corpus-nohooks` must be byte-identical with hooks disabled (table hook replica on).
4. Full hooks: `contentsite-out` must be byte-identical (needs the template engine + imaging).
5. Full site golden.

---

## 13. Cross-subsystem notes

* **canonifyURLs = true** rewrites `src=`, `href=` and `srcset=` values starting with `/` to absolute URLs at publish time (the image hook's
  `.RelPermalink` values such as `/peas/…/x.webp` become `https://seeksnack.com/peas/…`). That happens after markdown, in the publisher agent's domain.
  The content-only site disables it to show raw markdown output.
* The **minifier** (tdewolff, `keepComments=false`, `keepEndTags=true`, `keepQuotes=false`) runs after the whole page is assembled. It drops
  `<!--StartFragment-->` comments, keeps named entities (`&rsquo;`), and unquotes attributes. Markdown output must still be byte-exact
  pre-minify, because the minifier does not normalize text or entities.
* **hugo_stats.json / purgecss**: see section 1. Markdown-generated tags and IDs feed CSS purging, which is used through
  `resources.PostProcess` in `layouts/partials/head.html:276`.
* **Image processing inside the image hook**: ordering, cold-cache hash naming (finding 7).
