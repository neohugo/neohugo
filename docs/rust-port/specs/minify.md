# Minification byte-parity spec (tdewolff/minify v2.23.8 + tdewolff/parse v2.8.1)

Agent: `minify`. Scope: everything that turns neohugo's pre-minify bytes into the `--minify` golden bytes
for the seeksnack site, plus the `minify` template function on the postcss CSS resource.

Paths used below:

- `NH` = `/Users/blackb1rd/git/github/org/neohugo` (neohugo Go source, v0.148.2, built with go1.27.1)
- `MIN` = `$GOMODCACHE/github.com/tdewolff/minify/v2@v2.23.8`
- `PAR` = `$GOMODCACHE/github.com/tdewolff/parse/v2@v2.8.1`
- `SP` = `/private/tmp/claude-501/-Users-blackb1rd-git-github-org-neohugo/0835ef6d-534f-4ac6-a7eb-50e9240d6eb1/scratchpad`
- `W` = `$SP/work/minify` (everything I built for this spec lives here)

---------------------------------------------------------------------------------------------------

## 0. TL;DR

1. **The Go minifier is fully deterministic, and I confirmed the golden output with a reproducer.** I built the site
   without `--minify` (`$W/nomin`, a fresh `--ignoreCache` build with the same file list as golden), then ran every
   `.html/.xml/.json` file through neohugo's own `minifiers.New()` with the seeksnack config (`$W/harness`). Result:
   **4896 of the 4905 minified files are byte-identical to `golden/run1`.** The other 9 are all RSS files that
   either equal `golden/run2` or differ only in the known term-title collision nondeterminism
   ("Disodium 5'-Guanylate" vs "Disodium 5-Guanylate", "INS 322(i)" vs "INS 322i"). None of that comes from the
   minifier. The 11 static .html/.js/.css files are not minified by neohugo, so they were excluded. So the
   **golden output equals `minify(pre-minify output)`**, and the minifier can be ported and tested on its own,
   apart from the rest of the pipeline.
2. What seeksnack actually exercises. I recorded every nested call with `$W/recorder`. The table shows unique
   inputs and total calls:

   | minifier | where | unique inputs | calls |
   |---|---|---|---|
   | HTML | 3422 HTML pages (pages, lists, terms, 404, **1494 alias redirect pages**) | 3421 | 3422 |
   | XML | 1481 files (RSS `application/rss+xml` + `sitemap.xml` `application/xml`) | 1481 | 1481 |
   | JSON | `index.json` ×2 (en, th) + `<script type=application/ld+json>` ×1940 | 1719 | 1942 |
   | CSS | `<style>` (re-minify of already-minified CSS) ×1940, `style=""` attrs ×5751 (inline mode) | 3 | 7691 |
   | CSS | `resources.Minify` on postcss output (`toCSS\|postCSS\|minify`), **runs even without `--minify`** | 1 | 1 |
   | SVG | `url("data:image/svg+xml;…")` inside CSS → `minify.DataURI` → SVG minifier (4 per page, 7 at resource time) | 4 (+3) | 7760 |
   | JS | 3 inline `<script>` (gtag, GTM IIFE, adsbygoogle) + 3 inline `on*=""` handlers (inline mode) | 6 | 18790 |
   | none | `<script type="x-tmpl-mustache">`: no minifier is registered, so ErrNotExist and the raw bytes pass through | 1 | 1940 |
   | none | `robots.txt` (text/plain, no transformer), static files, js.Build/esbuild output, images | – | – |

3. **No Rust crate produces the same bytes.** minify-html, lightningcss, oxc, swc, esbuild, serde_json,
   html5ever and quick-xml all differ. **Recommendation: port tdewolff minify+parse line by line**, about 24.7k Go
   lines in scope. Roughly 7.5k of those are generated tables or AST printers you can generate or skip, which leaves
   about 15k lines of real logic. Most of the risk sits in: in-place buffer mutation and Go slice aliasing, the exact
   number formatting (`minify.Number`/`Decimal`), the perfect-hash name sets (whether a name is in the set changes
   behaviour), the JS renamer (Go `sort.Sort` = pdqsort, which is unstable), tdewolff's own non-correctly-rounded
   `ParseFloat`, and the `DataURI` keep/re-encode decision. That decision depends on the length of the SVG
   minifier's output; seeksnack's margins are only 6–36 bytes.
4. **Cross-subsystem warning (not minify-related, but it breaks golden comparisons).** Processed-image names
   (`*_hu_<hash>.jpg/webp`) change when the `resources/_gen` cache in the site directory is in a different state.
   My first build ran concurrently with another agent's build in the shared `sites/seeksnack` directory. It produced
   1444/1461 different image names. A `--ignoreCache` build reproduces golden exactly. **Build golden and
   comparison trees with `--ignoreCache`, or from a private copy of the site. Never run two builds concurrently in
   the same site directory.**

---------------------------------------------------------------------------------------------------

## 1. How neohugo wires minification

### 1.1 Config

- `--minify` flag: `NH/commands/commandeer.go:615` defines it and `NH/commands/helpers.go:80` maps it to
  `minifyOutput`. `NH/config/allconfig/load.go:173-176` then does `cfg.Set("minify.minifyOutput", true)`. The
  decoder is registered at `NH/config/allconfig/alldecoders.go:158-162`, which calls `minifiers.DecodeConfig`.
- Defaults (`NH/minifiers/config.go:29-53`, `defaultTdewolffConfig`):
  ```go
  HTML: html.Minifier{KeepDocumentTags: true, KeepSpecialComments: true, KeepEndTags: true, KeepDefaultAttrVals: true, KeepWhitespace: false},
  CSS:  css.Minifier{Precision: 0, KeepCSS2: true},
  JS:   js.Minifier{Version: 2022},
  JSON: json.Minifier{},
  SVG:  svg.Minifier{KeepComments: false, Precision: 0},
  XML:  xml.Minifier{KeepWhitespace: false},
  ```
- `DecodeConfig` (`NH/minifiers/config.go:81-135`) handles two upstream renames before `mapstructure.WeakDecode`:
  - css/svg `decimal` → `precision`, applied only if > 0.
  - html `keepconditionalcomments` → `keepspecialcomments`, only if the latter is not set. The old key is then
    deleted. This matters because `html.Minifier.Minify` (`MIN/html/html.go:75-79`) prints
    `DEPRECATED: KeepConditionalComments is replaced by KeepSpecialComments` **to stdout** and flips the flag
    whenever `KeepConditionalComments` is true. Keys arrive lowercased from the config loader, so neohugo removes
    the key and nothing is printed. If you pass camelCase keys straight to DecodeConfig, the message is printed.
- **Effective seeksnack config**, as decoded by `$W/recorder` with lowercase keys, which is how the real loader
  delivers them:
  ```
  HTML: KeepComments:false KeepConditionalComments:false KeepSpecialComments:true KeepDefaultAttrVals:true
        KeepDocumentTags:true KeepEndTags:true KeepQuotes:false KeepWhitespace:false TemplateDelims:["",""]
  CSS:  KeepCSS2:true Precision:0 (Inline set per call from params)
  JS:   Precision:0 KeepVarNames:false useAlphabetVarNames:false Version:2022
  JSON: Precision:0 KeepNumbers:false
  SVG:  KeepComments:false Precision:0
  XML:  KeepWhitespace:false
  ```
  `TemplateDelims` is empty, so the HTML lexer never detects templates and `HasTemplate` is always false. That is
  why `{{ key }}` inside the mustache `<script>` is harmless.

### 1.2 Registration: `NH/minifiers/minifiers.go:77-103` (`New`)

The `minify.M` gets:

| key (literal, from `mediaTypes.BySuffix`) | minifier |
|---|---|
| `text/css` | CSS |
| `text/javascript` + regexp `^(application\|text)/(x-)?(java\|ecma)script$` | JS |
| `application/json` + regexp `^(application\|text)/(x-\|(ld\|manifest)\+)?json$` | JSON |
| `image/svg+xml` | SVG |
| `application/rss+xml`, `application/xml` (both have suffix `xml`, `NH/media/builtin.go:136-137`) | XML |
| `text/html`, plus every output format with `IsHTML` (all `text/html`) | HTML |

`getMinifier` (`:107-124`) returns `noopMinifier` for disabled types. `M.Match`/`MinifyMimetype`
(`MIN/minify.go:176-217`) first parse the media type (`parse.Mediatype`, which splits off `;params`). Then they try
the literal map, then the patterns in insertion order, and otherwise return `ErrNotExist`. `m.URL` is nil, which
matters for URL shortening (see 3.2).

For Rust, a `match` on the mimetype string plus the two hand-coded regex predicates is enough. The `regex` crate is
also safe to use here.

### 1.3 Where it runs

- **Publishing** (`NH/publisher/publisher.go:156-190`, `createTransformerChain`): the chain order is
  1. `urlreplacers.NewAbsURLTransformer` for HTML or `NewAbsURLInXMLTransformer` for XML. seeksnack has
     `canonifyURLs=true`, so this runs, and it runs **before** minify.
  2. livereload (server only).
  3. `metainject.HugoGenerator` (home page only, and only when `AddHugoGeneratorTag` is set).
  4. `minifiers.Client.Transformer(outputFormat.MediaType)` when `MinifyOutput`. `Transformer` (`:41-53`) returns
     nil when `m.Match` finds no minifier. So `text/plain` (robots.txt) is written untouched.
  - The minifier input is the pooled buffer after steps 1-3. Because the buffer implements `Bytes()`,
    `parse.NewInput` uses its slice directly (no copy). It overwrites `b[len]` with NUL and restores it afterwards.
  - Alias pages (`NH/hugolib/alias.go:95-116`) also go through `Publish` with the page's HTML output format, so all
    1494 redirect pages are minified. Example: `<!doctype html><html lang=en><head><title>https://seeksnack.com/marshmallow/</title><link rel=canonical href=https://seeksnack.com/marshmallow/><meta name=robots content="noindex"><meta charset=utf-8><meta http-equiv=refresh content="0; url=https://seeksnack.com/marshmallow/"></head></html>`.
  - `hugo_stats.json` (`writeStats`) is collected from the post-minify bytes (MultiWriter, `publisher.go:118-120`).
    I checked that it is byte-identical with and without `--minify`.
- **Resources** (`NH/resources/resource_transformers/minifier/minify.go:48-52`): `resources.Minify` / the
  `minify` template function (`NH/tpl/resources/resources.go:300`) builds its own Client from the same config. It
  minifies by `ctx.InMediaType` and adds `.min` to the target path. **It always runs, with or without `--minify`.**
  seeksnack uses it once: `toCSS | postCSS | minify | fingerprint` in `layouts/partials/head.html:276`, with the
  result inlined as `<style>{{ .Content | safeCSS }}</style>`. That CSS is then minified *again* by the HTML
  minifier's `<style>` handling, which must be idempotent (it is).
- **Not minified**: static files (`static/**`, e.g. `serviceworker.html`, `pinterest-e4aaf.html`,
  `admin/index.html`, `css/seeksnack.css`), js.Build/esbuild output (uses esbuild's own minifier), images, and
  `robots.txt`.

---------------------------------------------------------------------------------------------------

## 2. What seeksnack exercises, with before and after examples

Pre-minify tree: `$W/nomin`. Golden: `$SP/golden/run1`. Every unique nested input is stored with its Go output in
`$W/corpus2/{css,js,json,svg,css-resource}/<key>.in|.out|.meta.json`. The meta file holds params (`inline`,
`charset`), the call count and the nesting depth.

### 2.1 HTML (`MIN/html/html.go`)

Every tag present in the output has non-zero traits in `tagMap` (div, input, label, a, meta, i, link, li, span,
source, script, p, img, picture, button, h1-h6, strong, nav, br, ins, td, th, tr, html, head, title, ul, header, ol,
small, body, style, footer, option, textarea, hr, select, article, table, thead, tbody, em, time, aside, iframe,
blockquote, main, code, font). This matters because attribute value minification is gated on `t.Traits != 0`
(html.go:409).

Things that show up in the golden files:

- `<!DOCTYPE html>` is always rewritten to `<!doctype html>` (html.go:103-104).
- Quoting (`PAR/html/util.go:9-65` `EscapeAttrVal`). A value is unquoted unless it contains a byte from
  `charTable` (`\t \n \f \r space " ' < = > \``). Examples: `lang=en`, `href=https://…`, `type=image/png`,
  `class="img-fluid mx-auto d-block"`, `aria-label="Toggle navigation"`. `&` does **not** force quotes.
  - When quoting is needed: the original quote is kept if it does not occur in the value. Otherwise `"` is used
    when `singles > doubles` or (`singles == doubles` and `orig != '\''`), else `'`. The chosen quote is escaped as
    `&#34;` or `&#39;`. Golden example: `onclick='return socialMediaPopUp(this.href,"",900,500),!1'`.
  - "isXML" (RDFa) attributes always stay quoted if they were quoted in the source (`mustQuote`): `vocab typeof
    property resource prefix content about rev datatype inlist` (html.go:501). That gives `content="noindex"`,
    `content="IE=edge"`, `content="&nbsp;"`, `property="og:type"`… A value-less form is written as `content`
    (e.g. `<meta property="og:description" content>`) because an empty value writes only the name (html.go:497).
- Boolean attributes (`booleanAttr`) drop their value: `async`, `defer`, `selected`, `checked`, `disabled`,
  `required`… Note that `hidden` is `trimAttr`, **not** boolean.
- `trimAttr` attributes collapse whitespace and entities and are trimmed. Other attributes only get
  `ReplaceEntities` with no reverse map, so `&lt;` becomes `<` inside attributes.
- `style=""` goes through the CSS minifier with `inline=1`. `on*=""` goes through the JS minifier with `inline=1`,
  after trimming and stripping a leading `javascript:`. In both cases an empty result drops the attribute.
  - `style="display:flex;width: 100%;height:100%;"` → `style=display:flex;width:100%;height:100%`
  - `onclick="socialMediaPopUp(this.href, '', 900, 500); return false;"` → `onclick='return socialMediaPopUp(this.href,"",900,500),!1'`
  - `onchange="location = this.value;"` → `onchange="location=this.value"`
- URL attributes (`urlAttr`: href, src, action, cite, data, formaction, itemid, poster, profile, xmlns) are trimmed.
  With `m.URL == nil`, `http:`/`https:` prefixes are only lowercased (`HTTP://Example.com` → `http://Example.com`),
  and `data:` URIs go through `minify.DataURI`.
- `type` on `a link embed object source script` and `enctype formenctype accept` go through `minify.Mediatype`
  (lowercase except inside quoted strings, whitespace removed).
- `<meta>` special cases (html.go:327-365): `http-equiv=content-type` + `text/html;charset=utf-8` becomes
  `charset=utf-8`; `name=keywords` has `", "` replaced by `","`; `name=viewport` has spaces removed and numbers
  after `=` run through `minify.Number(…,-1)`. seeksnack's viewport is already minimal.
- `<a id=x name=x>` drops `name`. `<input type=text value="">` drops `value`, and so does radio with `value=on`.
  `<script src charset>` drops `charset`. `class/dir/id/name` (and `form action`) with empty values are omitted.
  `KeepDefaultAttrVals=true`, so `type=text/javascript` and `method=get` are **kept**.
- Text: `parse.ReplaceMultipleWhitespaceAndEntities(text, EntitiesMap, TextRevEntitiesMap)`. **A whitespace run
  that contains `\n` or `\r` collapses to a single `\n`, anything else to a single space.** The golden output keeps
  155 newlines in a typical page, e.g. `…loading=lazy>\n</picture>`, `</a>\n<button …>`. Leading and trailing space
  is removed around block tags. See the `omitSpace` algorithm in 3.2.
- Comments: `KeepComments=false` and `KeepSpecialComments=true`, so only `<!--[if …` conditional comments and
  `<!--#…` SSI comments survive. Markdown-sourced `<!--EndFragment-->` is removed. Go html/template already strips
  template comments.
- Raw tags. `<script>`: JSON, JS, or passthrough depending on `type`. An empty `<script></script>` or
  `<style></style>` with **no attributes** is removed entirely. `<style>` goes to CSS. `<textarea>` is written raw.
  `<iframe>` content goes to the HTML minifier, but all 89 iframes in seeksnack are empty. `<title>` is RCDATA in
  the *lexer* but has `blockTag` (not `rawTag`) in the minifier, so its text **is** whitespace-collapsed:
  `<title>  a   b  </title>` → `<title>a b</title>`.
- `<select>` / `<option>`: a text token right after `<select …>`, `<optgroup …>`, `</option>` or `</optgroup>` is
  dropped even if it is not whitespace (html.go:309-314, 518-523).
- `KeepEndTags=true` and `KeepDocumentTags=true`: no end-tag omission, and `html/head/body` are kept. `<colgroup>`
  without attributes is still removed (html.go:255).

### 2.2 XML (`MIN/xml/xml.go`): RSS + sitemaps

- `<?xml version="1.0" encoding="utf-8" standalone="yes" ?>` → `<?xml version="1.0" encoding="utf-8" standalone="yes"?>`.
  Each attribute is re-emitted as ` name=` plus the value. Values quoted with `"` go through
  `ReplaceEntities(xml.EntitiesMap)` and then `xml.EscapeAttrVal`, which always quotes (`"` unless doubles > singles).
  Values that are unquoted or single-quoted are written **verbatim**.
- Whitespace-only text between tags is dropped. Text is collapsed with `\n` preserved. `>` is not escaped.
- Empty elements collapse: `<atom:link … />` → `<atom:link …/>`, and `<a></a>` → `<a/>` (xml.go:134-150).
- **CDATA to text** when that is shorter: `EscapeCDATAVal` counts `<` as +3 and `&` as +4 and gives up if the total
  exceeds 12 (`len("<![CDATA[]]>")`). The RSS `<description><![CDATA[<img src="…" alt="…"/>…]]></description>`
  becomes `<description>&lt;img src="…" alt="…"/>…</description>`.
- Comments are removed. `</tag   >` → `</tag>`.

### 2.3 JSON (`MIN/json/json.go`, `PAR/json/parse.go`)

- `index.json` is already compact (template `jsonify`), so minification is the identity.
- JSON-LD: whitespace is removed, and **trailing commas are silently accepted and dropped** (the `,}` case in
  jsonLd.html's `offers` block). String contents are kept byte for byte, including raw `\n` and 6 spaces
  (`"name":"\n      SeekSnack\n    "`) and `\/` escapes. Numbers go through `minify.Number(text, 0)`, with
  `.5` → `0.5` and `-.5` → `-0.5` fixups (json.go:62-70).
- A JSON parse error inside `<script type=application/ld+json>` **aborts the page** (HTML returns the error, and
  Publish fails with "failed to process"). The Rust port must fail the same way.

### 2.4 CSS (`MIN/css/css.go`, `PAR/css/{lex,parse}.go`)

`KeepCSS2=true`, so numbers use `minify.Decimal` (never exponents), and ` !important` keeps its space.

The postcss→minify step (`$W/corpus2/css-resource/postcss.in` → `.out`, 52035 → 51857 bytes) shows all 47
distinct rewrite classes the port must reproduce:

| before | after | rule (css.go) |
|---|---|---|
| `@media (min-width:1200px)` | `@media(min-width:1200px)` | parser: at-rule prelude drops whitespace before `(` (first token, `PAR/css/parse.go:274-279`) |
| `flex:0 0 auto` / `1 1 auto` / `1 0 0` | `flex:none` / `flex:auto` / `flex:1 0` | `minifyProperty` Flex (1207-1239) |
| `border:var(--x) solid var(--y)` | `border:var(--x)solid var(--y)` | `writeDeclaration`: no space after a FunctionToken (441-467) |
| `transform:scale(1) translateY(0)` | `transform:scale(1)translateY(0)` | same |
| `transform:rotate(0deg)` | `transform:rotate(0)` | zero-dimension cut when `fun == 0`, and **`rotate` is not in the CSS hash table so Fun==0** (498-501) |
| `-webkit-tap-highlight-color:rgba(0,0,0,0)` | `…:transparent` | rgba all-zero (566-570) |
| `background:transparent var(--bs-btn-close-bg) center/1em auto no-repeat` | `background:var(--bs-btn-close-bg)50%/1em no-repeat` | Background branch (800-926) |
| `background:none` / `background:transparent` | `background:0 0` | Background branch (end-start==0) |
| `src:url(a) format("woff2")` | `src:url(a)format("woff2")` | no space after URLToken |
| `url("data:image/svg+xml;charset=utf-8,%3Csvg …")` | unchanged | `minify.DataURI` re-encoding would be longer (see 2.5) |

Custom properties (`--bs-*`) are written as `name:` plus the trimmed raw value (`CustomPropertyGrammar`).

The `<style>` re-minify of the output (with surrounding `\n      `) is idempotent. The golden `<style>` content
equals `postcss.out` exactly.

### 2.5 SVG via data URIs (`MIN/svg/*.go`, `MIN/common.go:55-95` `DataURI`)

Every CSS pass calls `minify.DataURI` on `url("data:…")`: 7 URIs at resource time and 4 in the purged `<style>`
of each page. That function:

1. decodes the URI (percent-decoding via `parse.DecodeURL`, which **also turns `+` into a space**, or base64);
2. minifies the data with `m.Bytes("image/svg+xml;charset=utf-8", data)`, which runs the SVG minifier;
3. computes `base64Len = 7 + 4*ceil(n/3)` and `asciiLen = n + 2*count(DataURIEncodingTable[c])`, with an early
   break once `asciiLen > base64Len`;
4. **returns the original if `len(orig) < base64Len && len(orig) < asciiLen`**. Otherwise it re-encodes, strips a
   `text/plain` prefix and `;charset=us-ascii`, and so on.

For seeksnack the original is always kept. The margins are tight:

| uri | len(orig) | svg-min len | base64Len | asciiLen |
|---|---|---|---|---|
| 0,1 | 230 | 183 | 251 | 259 |
| 2 | 313 | 259 | 355 | 349 |
| 3 | 253 | 199 | 275 | **259** (6 bytes of margin) |
| 4 | 254 | 200 | 275 | 266 |
| 5 | 259 | 214 | 295 | 296 |
| 6 | 265 | 220 | 303 | 302 |

So SVG output length and escaping counts must be exact. Arc flags and path data are rewritten, e.g.
`d='M4.646 1.646a.5.5 0 0 1 .708 0l6 6…'` → `d="M4.646 1.646a.5.5.0 01.708.0l6 6…"`.

### 2.6 JS (`MIN/js/*.go`, `PAR/js/*.go`)

All six inputs and their Go outputs are in `$W/corpus2/js/`.

```
gtag:  window.dataLayer = window.dataLayer || [];\n function gtag(){dataLayer.push(arguments);}\n gtag('js', new Date());\n\n gtag('config', 'UA-149754145-1');
   →   window.dataLayer=window.dataLayer||[];function gtag(){dataLayer.push(arguments)}gtag("js",new Date),gtag("config","UA-149754145-1")
GTM:   (function(w,d,s,l,i){w[l]=w[l]||[];w[l].push({'gtm.start':\nnew Date().getTime(),event:'gtm.js'});var f=d.getElementsByTagName(s)[0],\nj=d.createElement(s),dl=l!='dataLayer'?'&l='+l:'';j.async=true;j.src=\n'https://www.googletagmanager.com/gtm.js?id='+i+dl;f.parentNode.insertBefore(j,f);\n})(window,document,'script','dataLayer','G-8E03TCKZZV');
   →   (function(e,t,n,s,o){e[s]=e[s]||[],e[s].push({"gtm.start":(new Date).getTime(),event:"gtm.js"});var a=t.getElementsByTagName(n)[0],i=t.createElement(n),r=s!="dataLayer"?"&l="+s:"";i.async=!0,i.src="https://www.googletagmanager.com/gtm.js?id="+o+r,a.parentNode.insertBefore(i,a)})(window,document,"script","dataLayer","G-8E03TCKZZV")
ads:   (adsbygoogle = window.adsbygoogle || []).push({});   →  (adsbygoogle=window.adsbygoogle||[]).push({})
inline(onclick): socialMediaPopUp(this.href, '', 900, 500); return false;  → return socialMediaPopUp(this.href,"",900,500),!1   (and the 500,500 variant)
inline(onchange): location = this.value;  →  location=this.value
```

What these outputs demonstrate:

- **Renaming** happens only in function scopes; globals such as `gtag` and `window` are not renamed.
  - Function arguments keep their declaration order. They are not sorted: `renameScope` sorts only
    `Declared[NumFuncArgs:]`.
  - Names are taken in index order from the character-frequency alphabet `etnsoiarclduhmfpgvbjy_wOxCEkASMFTzDNLRPHIBV$WUKqYGXQZJ`
    (`MIN/js/vars.go:30`), so `w,d,s,l,i` → `e,t,n,s,o`.
  - The locals `f,j,dl` are sorted by `Uses` descending with `sort.Sort`. `j` has the most uses and gets index 5
    (`i`); then `f` → `a` and `dl` → `r`. The insertion sort keeps ties in their original order.
  - A name that collides with a reserved word or an undeclared variable, following `Link` chains, is skipped.
- **Statement merging** (`MIN/js/stmtlist.go`): consecutive ExprStmts are joined with `,`. `ExprStmt; return X` →
  `return a,X`. `return false` at the top level of an inline handler is allowed because `Options.Inline` sets
  `retrn=true` (`PAR/js/parse.go:52-62`). The trailing `;` is dropped.
- **Expression rewrites**:
  - `true`/`false` → `!0`/`!1`.
  - `new Date()` → `new Date`. The **parser** drops empty `new` argument lists (`PAR/js/parse.go:1756-1761`), and
    the printer then emits `(new Date).getTime()` when a member access follows.
  - String quotes: `"` is the default, `'` when it has fewer escapes, and a backtick when
    `backticks+${ < quotes+newlines` (Version ≥ 2015).
  - `==` stays `==`, and `typeof x==="s"` → `==`.
  - A function expression in statement position is grouped as `(function(){…})(…)`.

---------------------------------------------------------------------------------------------------

## 3. Algorithms that affect output bytes

### 3.1 Shared infrastructure (`PAR/input.go`, `PAR/util.go`, `PAR/common.go`, `MIN/common.go`)

- **`parse.Input`**: the whole input is in memory with a **NUL sentinel** at `buf[len]`. `Peek` returns 0 at EOF,
  and EOF is detected as `c == 0 && Err() != nil`. **A literal NUL byte inside the input is an ordinary character
  unless it is at EOF.**
  - `Lexeme()`/`Shift()` return `buf[start:pos:pos]`. The capacity is clamped, so a Go `append` on them
    reallocates, while writing to an index **mutates the shared input buffer**.
  - Many functions mutate the input in place: `parse.ToLower`, `ReplaceEntities`, `ReplaceMultipleWhitespace*`,
    `minify.Number/Decimal` (they may write past `end` inside the slice), `minifyColor` (lowercases hex in place),
    the JS renamer (`getName` overwrites `v.Data`, which is shared by every reference to the var), and
    `minifyString` (rewrites the quote bytes in place).
  - In the port, own the buffer as `Vec<u8>`. Represent tokens as `(start,end)` ranges plus optional owned
    replacements, **or** mutate a `&mut [u8]` exactly where Go does. Wherever Go reads a byte after a mutation, the
    Rust code must see the mutated byte.
  - Known read-after-mutate spots:
    - html.go:506 reads `attr.Data[len-1]` (the original quote) after `val` was rewritten in place. That byte is
      never touched, because the writes happen inside `val`.
    - xml lexer (`PAR/xml/lex.go:284-286`) rewrites `\t\n\r` inside quoted attribute values to spaces **in the
      input**.
    - html lexer lowercases tag names, attribute names and whole end tags **in the input** (`PAR/html/lex.go:373,
      467, 495`).
- **Whitespace** = `' ' \t \n \r \f` (`whitespaceTable`). A newline is only `\n \r`.
- **`ReplaceMultipleWhitespaceAndEntities`** (`PAR/common.go:373-412`) compacts in place. Each whitespace run
  becomes one byte, `\n` if the run contains `\n` or `\r`, else a space. The `j==1` special case moves the leading
  byte to the end of the first whitespace run. Entity replacement is interleaved and runs on `&` only when
  `i+3 < len`.
- **`replaceEntities`** (`PAR/common.go:280-360`):
  - `&#x…;`: at least one hex digit and value < 10000. Values < 128 become the raw byte; others become `&#<dec>;`,
    which may be *longer*, e.g. `&#x2026;`.
  - `&#…;`: decimal < 128 becomes the raw byte; otherwise nothing changes.
  - Named entities (alphanumeric, at most 31 characters, `;` required) are looked up in the map. html
    `EntitiesMap` has ~1095 entries mapping to the *shortest* equivalent (`&hellip;`→`&mldr;`,
    `&middot;`→`&#183;`, `&amp;`→`&`, `&quot;`→`"`); `nbsp` and `copy` are not in the map, so they are kept.
  - If the single-byte result is in `revEntitiesMap` (text only: `<`→`&lt;` for HTML; `<`→`&lt;` and `&`→`&amp;`
    for XML), the reverse form is used. If that equals the original, nothing is replaced.
  - A result of `&` followed by `[0-9a-zA-Z#]` is **not** replaced (`&amp;x` stays).
- **`minify.Number(num, prec)`** (`MIN/common.go:211-512`) and **`minify.Decimal`** (`:105-208`) are byte-level
  shortest-number printers.
  - `Number`: drops `+`, leading and trailing zeros; `.5`, `5e3` vs `5000` (exponent only when shorter:
    `n+3 <= normExp`); `1e-7` style for small numbers. With `0 < prec` it rounds.
  - `Decimal` never emits exponents and only rounds digits after the dot.
  - Port them verbatim, including the in-place `copy` shuffles. Test vectors: `$W/vectors/numbers.jsonl`, 3042
    inputs covering Number with prec 0/-1/3/15 and Decimal with prec 0/3.
- **`minify.Mediatype`** (`MIN/common.go:22-52`): removes whitespace outside `"…"` and lowercases outside strings.
  Note the `i-lastString < 1024` guard.
- **`parse.Mediatype`** (`PAR/common.go:92-151`): splits `type;k=v;…`. Params are used only for `inline=1` and
  `charset`.
- **`parse.Number/Dimension`** (`PAR/common.go:21-88`): regex-like number scanners.
- **Data URI** (`PAR/common.go:154-196`, `MIN/common.go:55-95`): see 2.5. `EncodeURL(data, DataURIEncodingTable)`
  emits uppercase `%XX`. The table escapes controls, space, `"`, `#`, `%`, `&`, `<`, `>`, `[`, `\`, `]`, `^`, `` ` ``,
  `{`, `|`, `}`, DEL and ≥0x80. Base64 is Go `StdEncoding` (padded).

### 3.2 HTML minifier (`MIN/html/html.go:71-533`, `MIN/html/buffer.go`, `PAR/html/lex.go`)

- **Lexer** (`PAR/html/lex.go`): `Next()`
  - Inside a tag, attributes are split on whitespace, `=`, `>` or `/>`. The attribute value includes its quotes;
    `TokenBuffer.read` strips them (`MIN/html/buffer.go:40-63`). Names are lowercased unless templated.
  - Outside a tag, `<` followed by a letter starts a tag; `</` followed by something other than `>` or EOF is an
    end tag or a bogus comment; `<!` is a comment, CDATA or doctype; `<?` is a bogus comment; any other `<` is text.
  - `<svg` and `<math` are consumed whole as one `SvgToken`/`MathToken` (`shiftXML`) and sent to
    `image/svg+xml` / mathml minifiers. seeksnack has none.
  - Raw-text tags: textarea, title, style, xmp, iframe, script, plaintext. Their content runs until
    `</tagname` (case-insensitive). Script additionally handles the `<!--` escape state (`:243-281`).
  - Empty raw content produces **no** text token.
- **TokenBuffer** (`MIN/html/buffer.go`): lookahead with `Peek(i)` and `Shift()`. `Attributes(hashes…)` scans the
  attribute tokens of the current tag and returns pointers so the minifier can null out `Text` (which means
  "remove this attribute").
- **Main loop state**: `omitSpace` (initially true), `inPre`, `rawTagHash`, `rawTagMediatype`.
  - **Text** (not raw, not in pre):
    1. collapse whitespace and entities;
    2. if `omitSpace` and the text starts with whitespace, drop that byte;
    3. `omitSpace = false`;
    4. empty text → `omitSpace = true`;
    5. if the text ends in whitespace, set `omitSpace = true` and scan forward with `Peek(i)`:
       - EOF: drop the trailing byte and set `omitSpace = false`;
       - a non-whitespace text token or a template token: stop and keep the byte;
       - a start or end tag (or svg/math) with `blockTag` trait: drop the byte and set `omitSpace = false`;
       - a start tag (non-block), svg or math: stop and keep the byte;
       - a non-block **end** tag: keep scanning;
       - anything else: keep scanning.
  - **Start and end tags**:
    - `KeepWhitespace` or an `objectTag` sets `omitSpace = false`; else a `blockTag` sets `omitSpace = true`.
    - An end tag whose Data is longer than `</name>` is truncated to `</name>`.
    - After a start tag with `Traits == normalTag` (exactly), if the next token is the same tag's end tag, set
      `omitSpace = false`. This keeps the space in `<i class=x></i> text`.
    - `</template>` sets `omitSpace = true`.
  - **Attribute pipeline**, in order (html.go:391-511):
    1. skip removed attributes;
    2. templated attributes are written raw;
    3. `trimAttr`: collapse whitespace and entities, then trim; otherwise only `ReplaceEntities`;
    4. only if the tag has traits:
       - omit empty `class/dir/id/name` and `action` on `form`;
       - capture the `type` of a raw tag;
       - apply `Mediatype` to type/enctype/accept;
       - drop default values (only when `!KeepDefaultAttrVals`);
       - `style`: CSS minify with `inline=1`, drop if empty;
       - `on*`: strip `javascript:`, JS minify with `inline=1`, drop if empty;
       - `urlAttr`: trim, handle the scheme, run `DataURI`;
    5. write ` name`, then `=`+`EscapeAttrVal(val, origQuote, KeepQuotes||isXML)` if the value is non-empty and the
       attribute is not boolean.
  - **Raw content dispatch** (html.go:152-173):
    - iframe: `text/html`;
    - script/style with a `type` attribute: `parse.Mediatype(type)` and its params;
    - script without type: `application/javascript`, which matches the JS regexp;
    - style without type: `text/css`.
    - `ErrNotExist` writes the raw bytes; any other error aborts.
    - `<style amp-boilerplate>` is not minified.
    - A **script or style start tag without attributes whose next token is its end tag** is dropped entirely.
- **Tables to reproduce exactly**:
  - `MIN/html/table.go`: `tagMap` (traits `normalTag|rawTag|blockTag|objectTag|omitPTag|keepPTag`), `attrMap`
    (`booleanAttr|urlAttr|trimAttr`), `jsMimetypes`, `EntitiesMap` (1095 entries), `TextRevEntitiesMap`.
  - `MIN/html/hash.go`: 265 names (tag *and* attribute names share one table).
  - A name that is not in the hash gets `Hash=0` and `Traits=0`, and that gates behaviour. **Generate the Rust tables
    from the Go source**, and do not hand-type them.

### 3.3 XML minifier (`MIN/xml/*.go`, `PAR/xml/{lex,util}.go`)

Covered in 2.2. Points to port carefully:

- the lexer's PI handling (`<?name … ?>` is StartTagPI, then attributes, then StartTagClosePI);
- `<!` that is not a comment, CDATA or DOCTYPE is treated as a start tag named `!…` (lex.go:156-164);
- a DOCTYPE with `[` … `]` internal subset;
- the in-place `\t\n\r`→space rewrite in quoted attributes;
- `EscapeCDATAVal`;
- `xml.EscapeAttrVal` (prefers `"`, escapes with `&#34;`/`&#39;`);
- whitespace trimming via `Peek` lookahead (xml.go:67-113), which differs from HTML: any start or end tag trims.

### 3.4 JSON minifier

Covered in 2.3: a 73 + 308 line port. Parser details:

- `moveWhitespace` skips only ` \n\r\t`;
- strings end at an unescaped `"` (the backslash parity is counted backwards) and are never validated;
- numbers follow the `-?(0|[1-9]\d*)(\.\d+)?([eE][+-]?\d+)?` shape with lenient fallbacks, e.g. `1.` leaves the
  `.` for the next token, which then errors;
- literals are true/false/null.

### 3.5 CSS minifier

- **Lexer** (`PAR/css/lex.go`):
  - roughly CSS Syntax 3, with tdewolff specifics. For `--x`, `case '-'` tries, in order: `consumeNumeric`
    (fails); `consumeIdentlike`, which fails because `consumeIdentToken` accepts one leading `-` and then needs a
    name-start character, and a second `-` is not one; `consumeCDCToken` (`-->`); and finally
    `consumeCustomVariableToken`, which yields a **`CustomPropertyNameToken`**. This applies both to declaration
    names (`--bs-x:` → `CustomPropertyGrammar`) and to arguments such as `var(--bs-x)`;
  - `url(` is special-cased (unquoted URL, BadURL recovery);
  - U+ unicode ranges;
  - escapes;
  - comments are tokens.
- **Parser** (`PAR/css/parse.go`), a state machine:
  - `parseStylesheet`, `parseQualifiedRule`, `parseDeclarationList`, `parseAtRule`, `parseAtRuleRuleList`,
    `parseAtRuleDeclarationList`, `parseAtRuleUnknown`, `parseCustomProperty`;
  - the IE hack `*prop`;
  - whitespace normalisation into single `WhitespaceToken(" ")` entries, skipped after `, / : ! =` and in selectors
    after `, > + ~`;
  - property names and at-rule names are lowercased copies;
  - error recovery (`parseDeclarationError`) writes the offending tokens verbatim (css.go:175-191).
- **Minifier** (`MIN/css/css.go`):
  - `minifyGrammar`: writes `{`, `}`, `;` with a queued semicolon so the last one in a block is omitted.
    `@import url(x)` becomes `@import"x"`.
  - `minifySelectors`: lowercases idents except class names; unquotes attribute selector strings when they are
    idents; keeps a space before the `i` flag.
  - `parseDeclaration`: returns nil for "complex" values (any bracket or brace, or two non-separators in a row),
    in which case the raw tokens are written.
  - `minifyTokens`:
    - numbers go through `Decimal` (KeepCSS2), except for z-index, counter-*, orphans and widows;
    - `%` values likewise;
    - dimensions: lowercase the unit, run `Decimal` on the number, and cut a zero value to `0` when the unit is in
      `optionalZeroDimension`, `prop != Flex` and **`fun == 0`**;
    - strings drop `\`+newline;
    - `url()`: unquote if `IsURLUnquoted`, run DataURI;
    - `rgb/rgba/hsl/hsla` → hex or `transparent`. Uses `strconv.ParseFloat(…, 32)` and
      `byte(x*255+0.5)`, then `ShortenColorHex` (e.g. `#ff0000`→`red`) or a 3-digit hex.
  - `minifyProperty`: per-property rules for
    - font, font-family, font-weight;
    - margin, padding, border-width (1-4 value collapse);
    - border*, outline;
    - background, background-size, background-repeat, background-position;
    - box-shadow, color, background-color (KeepCSS2 keeps `transparent`);
    - border-color*, column-rule, text-shadow, text-decoration, text-emphasis;
    - flex, flex-basis, order, flex-grow, flex-shrink;
    - unicode-range, -ms-filter.
  - `minifyColor`: named color → hex via `ShortenColorName`; lowercase hex; `#rrggbbaa` → `#rrggbb` or `#0000`;
    hex → name via `ShortenColorHex`; 6→3 and 8→4 digit shortening.
  - `writeDeclaration`: no space after a comma, `/`, a function or a url.
  - `!important` is written as ` !important` because KeepCSS2 is set.
- **Hash-set membership is behaviour.** `ToHash` returns 0 for names not in `MIN/css/hash.go`'s 420-name table. So
  the `fun == 0` test effectively means "unknown function": `rotate(0deg)`→`rotate(0)`, but `calc(0px…)` and
  `var(…)` are left alone. The port needs the **identical name set**.
- Nested `Token.Args` trees, and Go `append(values[:i], values[i+1:]...)` on shared backing arrays, including
  `append(values[:i+1], append(sizeValues, values[i+3:]...)...)` where `sizeValues` aliases `values`. Reproduce the
  logical sequence each Go statement leaves behind. Where aliasing makes it matter, simulate Go semantics
  (`Vec::splice`/`drain` on the same vec is equivalent *only if* no other slice to the same array is read
  afterwards; check every site).
- `sort.Slice` on unicode ranges: tie order does not affect the result, so any sort is fine.

### 3.6 SVG minifier (`MIN/svg/svg.go`, `buffer.go`, `pathdata.go`, `table.go`, `hash.go`)

- Uses the XML lexer.
- The TokenBuffer (`svg/buffer.go:40-63`) strips quotes and runs `ReplaceMultipleWhitespaceAndEntities(xml
  EntitiesMap)` plus trim on attribute values.
- Drops comments, PIs, DOCTYPE without `]`, `<metadata>`, namespaced tags other than xlink, empty `<defs/>`, and
  default attributes (`version=1.1`, `x=0`, `y=0`, `preserveAspectRatio="xMidYMid meet"`, …, and `xmlns` only when
  `Inline`).
- `shortenDimension` on any attribute that is exactly a dimension: `Number`, and `px` dropped.
- `viewBox` gets 4 numbers.
- Color attributes (`colorAttrMap`) use the CSS hex and name tables.
- `style` attributes and `<style>` go to the CSS minifier.
- `d` goes to `PathData.ShortenPathData`. For each command it picks the shorter of the absolute and relative forms:
  - relative↔absolute coordinates are computed in float64;
  - alternate coordinates are printed with **Go `strconv.AppendFloat(f,'g',-1,64)`** and then `minify.Number(…,15)`;
  - original coordinates are parsed with **tdewolff `strconv.ParseFloat`** (`PAR/strconv/float.go:15-94`), which
    is *not correctly rounded*: 63 of my 3042 vectors differ from Go's `strconv.ParseFloat`. Port it exactly,
    including `math.Pow10`;
  - `copyNumber` inserts a space or `.0` and turns `…00` into `…e2`; `copyFlag` handles arc flags.
- Attribute values are written with `xml.EscapeAttrVal`, so `'` becomes `"`.

### 3.7 JS minifier (`MIN/js/{js,stmtlist,util,vars}.go`, `PAR/js/{lex,parse,ast,table,tokentype,util}.go`)

- **Parser**:
  - full ES2022+ recursive descent with ASI and regex-vs-divide decisions in the lexer;
  - the `assumeArrowFunc` speculative parse of `(`… groups with scope undeclaration (`UndeclareScope`). This is
    used by both `(function…)()` and `(adsbygoogle=…)`;
  - `WhileToFor` (while → for);
  - `Inline` mode (top-level return allowed);
  - `/*! */` bang comments kept;
  - normalisations: empty `new X()` args dropped; directive prologues.
  - **Scope analysis** builds `Scope{Declared, Undeclared, VarDecls, NumForDecls, NumFuncArgs, NumArgUses}` and a
    `Var{Data, Link, Uses, Decl}` pointer graph. `Uses` counts decide renaming order. Port as an arena, with Var
    identity as an index; pointer equality becomes index equality.
- **Minifier pipeline** (`js.go:40-66`):
  1. `hoistVars(ast)` picks the best `var` declaration by a score and hoists the others into it;
  2. `optimizeStmtList(list, functionBlock)` merges ExprStmts, if/else → `&&`/`||`/`?:`, if-return merging, var
     merging, and removes a trailing `return`/`continue`;
  3. print each statement.
  - Function bodies do hoist → optimize → `renameScope` → params (trailing unused params removed) → block.
- **Renamer** (`vars.go:21-151`):
  - char-frequency alphabets `identStart="etnsoiarclduhmfpgvbjy_wOxCEkASMFTzDNLRPHIBV$WUKqYGXQZJ"`,
    `identContinue="etnsoiarcldu14023hm8f6pg57v9bjy_wOxCEkASMFTzDNLRPHIBV$WUKqYGXQZJ"`;
  - `getName(index)`: 1 character for index < 54, then 2+ characters (first from identStart, the rest from
    identContinue, little-endian);
  - `sort.Sort(VarsByUses(Declared[NumFuncArgs:]))` has `Less = Uses >`. **Go `sort.Sort` is pdqsort. It is
    unstable for n > 12**, since insertion sort handles n ≤ 12. To match ties you must port Go's
    `sort/zsortinterface.go` (479 lines: `pdqsort`, `insertionSort`, `heapSort`, `choosePivot`, `breakPatterns`
    with its deterministic xorshift, `partialInsertionSort`, …). `sort.SliceStable` (`js.go:546`) is stable, so
    Rust `sort_by` is equivalent;
  - reserved names = `js.Keywords` (`PAR/js/table.go:82-142`) plus the scope's `Undeclared` names;
  - `renameScope` runs per block, function, for, switch, try and catch scope at the points listed in `js.go`.
    Order matters because it mutates shared `Var.Data`.
- **Printer**: `write()` inserts a space when the previous token ends in an identifier character and the next
  starts with one (`IsIdentifierContinue`/`End` with **Unicode tables**), and avoids `++`/`--`/`//` joins
  (`spaceBefore`). The printer also handles:
  - precedence-driven parenthesisation (`binaryLeftPrecMap`, `binaryRightPrecMap`, `unaryPrecMap`, `exprPrec`);
  - an undeclared `undefined`, and `void x` when `x` has no side effects, are both written as **`0[0]`**
    (`(0[0])` when `OpMember < prec`). Verified: `var a;f(undefined,Infinity,a===undefined)` →
    `var a;f(0[0],1/0,a===0[0])`. Port the constant table from `MIN/js/util.go:14-94` verbatim;
  - `Infinity`→`1/0` (`(1/0)` when grouped);
  - `isNaN(x)`→`x!=x`, `Number(x)`, `Math.pow`→`**`, `Math.trunc`→`|0`, `Math.abs`;
  - string→property (`a["b"]`→`a.b`);
  - `minifyString` / `replaceEscapes`;
  - number literals (`decimalNumber`, binary, octal, hex → decimal when shorter);
  - regexp flag sorting (`minifyRegExp`);
  - template literals.

### 3.8 Unicode and float dependencies from the Go stdlib

| Go function | used in | Rust equivalent |
|---|---|---|
| `strconv.ParseFloat(s, 32)` | CSS rgb/hsl | `s.parse::<f32>() as f64` (both correctly rounded; CSS number tokens never contain inf/nan/hex/underscores) |
| `strconv.ParseFloat` via tdewolff `strconv.ParseFloat` | SVG path | **custom port** (not correctly rounded) |
| `strconv.AppendFloat(f,'g',-1,64)` | SVG path alt coords | shortest digits (Rust `format!("{:e}")` or `ryu` digits) + **Go %g layout**: `eprec=6` for shortest; exponent iff `exp < -4 \|\| exp >= 6`; `e+06` style with at least 2 exponent digits |
| `math.Pow10`, `Modf`, `Mod`, `Abs`, `Pow` | parse float, css hsl, unicode-range | std f64 ops; Pow10 must be the exact table (`10f64.powi` is **not** exact for large n) |
| `encoding/base64.StdEncoding` | DataURI | `base64::engine::general_purpose::STANDARD` |
| `encoding/hex` (lowercase) | rgbToToken, JS `\x` | trivial |
| `unicode` tables (Go 1.27 = **Unicode 17.0.0**) | JS identifiers (`Lu Ll Lt Lm Lo Nl Other_ID_Start` / `+Mn Mc Nd Pc Other_ID_Continue`), `Zs` whitespace | generate the tables from Go (do **not** use `unicode-ident`, which implements UAX31 ID_Start minus Pattern_Syntax and may be a different Unicode version) |
| `utf8.DecodeRune` / `EncodeRune`, and parse's non-validating `PeekRune` | JS lexer and strings | port `PeekRune` verbatim; `DecodeRune` returns U+FFFD for invalid bytes |
| `sort.Sort` (pdqsort) | JS renamer | port Go's `zsortinterface.go` |
| `fmt.Sprintf("U+%X")` | unicode-range | trivial |

---------------------------------------------------------------------------------------------------

## 4. Port size: files and line counts

Counts are non-test lines as of v2.23.8 / v2.8.1.

### 4.1 tdewolff/minify (12,249 lines in scope)

| file | lines | port? |
|---|---|---|
| minify.go | 375 | ~80 (M registry, Match, MinifyMimetype, Bytes). Skip cmd/http/pipe helpers |
| common.go | 523 | **all** (Mediatype, DataURI, Decimal, Number) |
| html/html.go | 533 | all |
| html/buffer.go | 139 | all |
| html/table.go | 1389 | tagMap/attrMap/jsMimetypes (~290 lines); **generate** EntitiesMap (1095 entries) |
| html/hash.go | 610 | **generate** the name→enum table (265 names); don't port the perfect hash |
| css/css.go | 1572 | ~1450 (the rest is a commented-out block) |
| css/util.go | 55 | all |
| css/table.go | 198 | all (color maps, optionalZeroDimension; PropertyOverrides unused) |
| css/hash.go | 1392 | **generate** (420 names) |
| js/js.go | 1411 | all |
| js/stmtlist.go | 354 | all |
| js/util.go | 1458 | all |
| js/vars.go | 453 | all |
| json/json.go | 73 | all |
| svg/svg.go | 359 | all |
| svg/buffer.go | 136 | all |
| svg/pathdata.go | 447 | all |
| svg/table.go | 82 | all |
| svg/hash.go | 424 | **generate** (112 names) |
| xml/xml.go | 166 | all |
| xml/buffer.go | 86 | all |
| xml/table.go | 14 | all |

Not needed: `cmd/`, `bindings/`, `minify/`, `tests/`, `_benchmarks`.

### 4.2 tdewolff/parse (12,425 lines in scope)

| file | lines | port? |
|---|---|---|
| input.go | 186 | all |
| util.go | 216 | ~150 (skip Indenter/Printable) |
| common.go | 570 | all except QuoteEntity and AppendEscape |
| error.go / position.go | 47 / 95 | minimal (error messages only; the build fails either way) |
| buffer/{reader,writer}.go | 44 / 65 | trivial (`&[u8]` / `Vec<u8>`); skip lexer.go and streamlexer.go |
| strconv/float.go | 257 | ParseFloat (~95). AppendFloat is unused by minify |
| strconv/int.go | 152 | ParseInt, LenInt, AppendInt |
| strconv/decimal.go, number.go | 149, 128 | not used by minify |
| html/lex.go, util.go, hash.go | 592, 109, 81 | all (hash: generate) |
| css/lex.go, parse.go, util.go, hash.go | 698, 493, 47, 75 | all (hash: generate) |
| xml/lex.go, util.go | 340, 87 | all |
| json/parse.go | 308 | all |
| js/lex.go | 809 | all |
| js/parse.go | 2379 | all |
| js/ast.go | 2397 | ~1100 (node types, Scope, Var, VarsByUses, IsIdent helpers). **Skip** the ~62 `JS()` writers and ~56 `String()` methods |
| js/table.go | 142 | all (precedence tables, Keywords) |
| js/tokentype.go | 404 | all (TokenType, `Bytes()` for operator printing) |
| js/util.go | 46 | all |
| js/walk.go | 288 | not needed (minify doesn't walk) |

### 4.3 Go stdlib pieces to port

- `sort/zsortinterface.go` (479 lines) for exact `sort.Sort`.
- The Go `%g` float layout (~40 lines on top of shortest-digit generation).
- `math.Pow10` table.
- Unicode range tables (generated).

**Total:** about 24.7k lines in scope. After dropping printers, hash implementations and generated tables, roughly
**15k lines of logic to hand-port.** JS is about 60% of it (parse/js ~4.5k + minify/js ~3.7k).

---------------------------------------------------------------------------------------------------

## 5. Rust crates

Safe (no effect on output bytes):

- `base64` (STANDARD engine with padding).
- `regex`, or hand-written predicates, for the two mimetype patterns.
- `memchr` (speed only).
- `phf` / `std::sync::LazyLock` for the generated tables.
- An arena such as `typed-arena`, `bumpalo` or a plain `Vec` with indices for the JS AST and the Var graph.
- `smallvec`.
- `ryu`, **only** as a shortest-digit generator feeding a hand-written Go-`%g` formatter. Verify with
  `$W/vectors/numbers.jsonl` field `goAppendFloat_g`.
- `std` `f32` parsing for Go `ParseFloat(s,32)`.

NOT safe (they would change bytes):

- **minify-html / minify-html-onepass**: different whitespace, attribute and entity rules.
- **lightningcss, cssparser**, parcel-css: different tokenization and error recovery, and different
  color/number/shorthand rules; `cssparser` is spec-strict and would not reproduce tdewolff's IE hacks, BadURL or
  custom-property handling.
- **oxc_minifier / swc_ecma_minifier / esbuild**: different renaming (frequency- and slot-based), statement
  merging and printing.
- **oxc_parser / swc_ecma_parser**: different AST shapes and **no `Uses`-count scope model**, so renaming order
  would differ.
- **serde_json**: rejects trailing commas and raw control characters in strings, and re-formats numbers and escapes.
- **html5ever / lol_html / tl / scraper**: different tokenization (tdewolff is lenient and byte-oriented), with
  entity decoding and case normalization done differently.
- **quick-xml / xmlparser / roxmltree**: entity and CDATA handling differs.
- **percent-encoding**: different escape set; also `parse.DecodeURL` maps `+` to space.
- **`f64::from_str`** as a replacement for tdewolff `ParseFloat`: that one is not correctly rounded.
- **Rust `{}`/`{:e}` Display** as a replacement for Go `AppendFloat 'g'`: different layout.
- **`sort_unstable_by`** as a replacement for Go `sort.Sort`: different tie order.
- **`unicode-ident`** or `char::is_alphabetic` for JS identifier classes: different property definition and Unicode
  version.

**Fallback while porting:** the upstream repo ships a C-ABI build (`MIN/bindings/py/minify.go`, `//export
minifyConfig`, `minifyString`). It can be built with `go build -buildmode=c-archive` and linked from Rust behind a
cargo feature. That gives 100% parity immediately, so the other subsystems can be tested while the pure-Rust port
is in progress. It does keep a Go toolchain as a build dependency, so it is only a temporary bridge.

---------------------------------------------------------------------------------------------------

## 6. Parity risks, ranked

1. **In-place mutation and Go slice aliasing.** Lexers lowercase names in the input; `ReplaceEntities`, `Number`
   and `Decimal` compact in place; the renamer writes `Var.Data`; `append(values[:i], …)` patterns appear
   throughout the CSS (`minifyProperty`), JS (`optimizeStmtList`, `addDefinition`, `hoistVars`) and HTML code. Any
   read that happens after a write must see what Go sees.
2. **JS renamer and scope analysis.**
   - `Uses` counts, which come from parser subtleties: `UndeclareScope` in arrow-function speculation,
     `HoistUndeclared`, catch/for scopes;
   - `Declared` order;
   - the pdqsort tie order for scopes with more than 12 sortable vars (not the case in seeksnack, but likely for
     any bigger inline script);
   - reserved-name collision skipping;
   - `hoistVars` scoring.
3. **Name-set membership of the perfect hashes** (CSS 420, HTML 265, SVG 112). Behaviour depends on whether a name
   is known, e.g. `Fun==0` in CSS and `Traits==0` in HTML. Generate the sets from the Go sources.
4. **Number formatting** (`minify.Number` / `Decimal` exponent/precision paths), **tdewolff ParseFloat
   rounding**, and **Go `%g`**.
5. **The DataURI keep/re-encode decision** depends on the exact SVG minifier output length. seeksnack's margin is
   as small as 6 bytes. `DecodeURL` also turns `+` into a space.
6. **HTML whitespace rules**: newline-preserving collapse, the `omitSpace` lookahead through non-block end tags,
   the `normalTag` same-tag-end rule, and dropped text after select/option/optgroup.
7. **Entity handling asymmetry**: text uses `TextRevEntitiesMap` (`<`→`&lt;`) and attributes don't;
   `&amp;`+alnum is kept; numeric entities ≥128 are kept or rewritten to decimal.
8. **Attribute quoting**: preference for the original quote, `mustQuote` for RDFa attributes (`content`,
   `property`, …), and `charTable` (backtick and `=` force quotes; `&` doesn't).
9. **Config decoding**: `keepConditionalComments` rename (stdout side effect if missed), empty TemplateDelims, CSS
   `KeepCSS2=true` (Decimal, and a space before `!important`), JS `Version=2022` (`??`, optional catch binding,
   template strings), `Precision=0` everywhere.
10. **Error semantics**: JS or JSON parse errors in nested content abort the page write (build error), while CSS is
    tolerant. `ErrNotExist` means raw passthrough, as for x-tmpl-mustache.
11. **Unicode**: Go 1.27 tables are **Unicode 17.0.0**. The parse package's `PeekRune` does no validation.
12. **Idempotence**: CSS is minified twice (resource, then `<style>`), so a small Rust divergence in the first pass
    can be hidden or amplified by the second. Test both passes.
13. **Upstream nondeterminism** (not minify): term-title collisions (8-9 RSS/HTML files differ between Go runs) and
    the image `_hu_` hashes that depend on the state of the `resources/_gen` cache.

---------------------------------------------------------------------------------------------------

## 7. Porting order and the differential-testing harness

### 7.1 Tools I built (all under `$W`)

- **`$W/harness/harness`** (Go, `main.go`, built against neohugo via a `replace` directive). It calls **neohugo's
  own `minifiers.New(media.DefaultTypes, output.DefaultFormats, testconfig.GetTestConfig(...))`** with the
  seeksnack `[minify]` section, so it is the reference implementation.
  - Stdin mode: `harness -type text/html < in > out`. Params are passed in the media type, for example
    `-type "text/css;inline=1"` or `-type "application/javascript;inline=1"`, which is how the HTML minifier calls
    nested minifiers.
  - Tree mode: `harness -tree $W/nomin -golden $SP/golden/run1 [-out DIR]` minifies every .html/.xml/.json file and
    prints `DIFF` lines and a summary.
- **`$W/recorder/recorder`** (Go). It rebuilds the same `minify.M` from `minifiers.DecodeConfig` and **wraps every
  registered minifier**, so each call is recorded, including nested ones (HTML→JS/JSON/CSS, CSS→DataURI→SVG). It
  writes a deduplicated corpus: `recorder -tree $W/nomin -golden $SP/golden/run1 -corpus DIR`. Its output is
  `$W/corpus2/{css,js,json,svg}/<key>.in/.out/.meta.json`, 29 MB. I deleted the html/xml kinds because they
  duplicate `$W/nomin`. `$W/corpus2/css-resource/postcss.{in,out}` holds the resource-level CSS pass.
- **`$W/vectors/`** (Go). `numbers.jsonl` has 3042 cases: `minify.Number` at 4 precisions, `Decimal` at 2,
  tdewolff `ParseFloat` bits, Go `ParseFloat` bits, Go `AppendFloat 'g'`, and `parse.Number` length. `text.jsonl`
  covers HTML text and attribute entity/whitespace handling, `EscapeAttrVal` in 4 modes, and `Mediatype`.
- **`$W/nomin`**: a clean non-minified build (fresh cache) whose file list is identical to golden. It is the input
  corpus for whole-file tests.
- **`$W/site3`**: a private full copy of the site (rsync) for further builds. Use `--ignoreCache` to reproduce the
  golden image names.
- Helpers: `cmpnorm.py` (tree compare ignoring `_hu_` hashes) and `showdiff.py` (first differing byte with context).

### 7.2 Suggested order, each step gated by its corpus

1. **Core** (`parse::input/util/common/strconv`, `minify::common`) → `vectors/*.jsonl` (100% required).
2. **JSON** → `corpus2/json` (1719 cases) plus the upstream `json_test.go` table.
3. **XML** → the 1481 golden `.xml` files from `nomin`, plus upstream `xml_test.go`.
4. **CSS** (lexer, parser, minifier, generated hash, color tables) → `css-resource/postcss` (the resource pass),
   `corpus2/css` (the `<style>` re-minify and 2 inline cases), upstream `css_test.go` (~387 vectors). Run the
   DataURI path with an SVG stub first, then for real after step 5.
5. **SVG** (xml lexer reuse, pathdata, tdewolff ParseFloat, Go %g) → `corpus2/svg` (4) plus the 7 resource-time
   data URIs (`corpus2/css-resource` covers them indirectly), upstream `svg_test.go` and `pathdata_test.go`.
6. **HTML** (lexer, token buffer, minifier, generated tables) → all 3422 golden HTML files. Until step 7 lands, stub
   JS through the FFI fallback or a lookup of the 6 recorded JS cases. Also upstream `html_test.go`, which uses
   default configs, so run those tests with the config the test specifies.
7. **JS** (lexer, parser with scope/Uses, AST arena, minifier, renamer, pdqsort port) → `corpus2/js` (6 cases), then
   upstream `js_test.go` (~850 vectors) and `util_test.go`, plus parse/js tests. **The upstream table-driven tests
   (~1,900 vectors across minify, plus the parse tests) should be ported wholesale as Rust tests.**
8. **Continuous differential fuzzing**: `cargo fuzz` targets that send the same bytes to the Rust port and to the Go
   reference (the stdin harness, or the c-archive bridge) and assert byte equality, one per minifier and mimetype
   and inline flag. Seed them with `corpus2` and the upstream test inputs.
9. **Final acceptance**: run the Rust neohugo `--minify` on the site and compare with `golden/run1`, treating the 8-9
   known nondeterministic RSS/HTML files as "run1 or run2".

---------------------------------------------------------------------------------------------------

## 8. Cross-subsystem notes (for the orchestrator)

- **Image cache pollution.** A build in `sites/seeksnack` that ran concurrently with another agent's build changed
  the `resources/_gen` state. Every later cached build then produced different `_hu_` names for the 1444 watermark
  (`images.Overlay`) derived images, while the 17 non-watermarked ones stayed the same. `--ignoreCache` restores the
  golden names exactly. The images agent should find out why cached and fresh processed images hash differently.
  Whatever the cause, golden comparisons must use a fresh cache or a private site copy.
- **A symlinked `node_modules` changes esbuild output.** My symlink-overlay site produced a different
  `website.<hash>.js`, so esbuild output depends on the resolved paths. Use real copies (rsync) for test builds.
- **The minify transformer runs after canonifyURLs/absURL** (publisher chain). The Rust publisher must keep that
  order.
- **`resources.Minify` runs regardless of `--minify`**, so the CSS minifier port is needed even for non-minified
  builds.
- **`hugo_stats.json`** is identical with and without `--minify`. The collector sees the minified HTML but is
  robust to it.

## 9. Open questions

- Should the Rust port aim for full tdewolff generality (recommended, because content will change), or can the JS
  minifier go through the FFI bridge until the port is complete?
- Unicode version: the golden binary was built with go1.27.1 (Unicode 17.0.0). If neohugo's Go toolchain changes
  before the port is finished, regenerate the golden output and the tables together.
- The exact cause of the cache-dependent image names (images subsystem).
