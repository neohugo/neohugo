# Templates inventory spec for the seeksnack build under neohugo

> **Byte-parity sections obsolete.** This spec is research for the old byte-for-byte port
> (the `crates/` tree deleted in T00 of [`REWRITE_PLAN.md`](../REWRITE_PLAN.md); recoverable with
> `git show go-parity-final:crates/<path>`, commit `be02933a`, local tag). The Rust rewrite in
> `rust/` compares structurally (REWRITE_PLAN.md §7), so every byte-parity target, golden-byte
> count and "reproduce Go's bytes" rule below is obsolete. The Hugo semantics it documents (Go
> file and line references, parity traps) remain a reference; scratch paths (`/Users/…`,
> `/private/tmp/…`, `$W`, `$SP`, `golden/run1`) no longer exist. Current state:
> [`HANDOFF.md`](../HANDOFF.md).

Agent: `templates-inventory`. Scope: every template that runs when neohugo builds seeksnack
(`neohugo-go --minify -d <out>`, environment `production`), every template function and every
object method or field the templates use, the control structures in the templates, and how
neohugo picks a layout for each page kind and output format.

All `file:line` citations for Go refer to `/Users/blackb1rd/git/github/org/neohugo` at tag v0.148.2
(commit d5930ba1f). Citations of the form `layouts/...:N` refer to the seeksnack site copy.

---------------------------------------------------------------------------------------------------

## 0. Method and evidence

The inventory draws on three sources:

1. **Static parse** of all 58 files under `layouts/`, using Go's `text/template/parse` with
   `SkipFuncCheck|ParseComments`. The tool is `work/templates-inventory/staticparse/main.go` and its output
   is `work/templates-inventory/static.tsv`. It lists every function identifier, field chain,
   control node, `define` and trim marker.
2. **Template metrics** from the golden binary: `--templateMetrics --printUnusedTemplates`, output in
   `work/templates-inventory/metrics.txt`. This gives execution counts per template.
3. **Dynamic trace** from an instrumented copy of neohugo at `work/templates-inventory/nh/`. The binary is
   `work/templates-inventory/neohugo-trace` and runs with `TPL_TRACE=<file>`. Apart from the known
   nondeterminism (image `_hu_` hashes and the term-collision RSS files), its output matched the golden
   output. It records:
   - every function and method call, with the dynamic argument types, the result type and the source
     `file:line`. It hooks `evalCall` and `evalField` in
     `tpl/internal/go_templates/texttemplate/hugo_template.go`.
   - every struct-field and map-key lookup, with the receiver type.
   - every `if`/`with` condition (type and truth value) and every `range` (type).
   - every `template` action and every printed value type.
   - the html/template escaper chain for each action, hooked in `evalPipeline` in `exec.go`.
   - for each rendered page: kind, output format, type, layout, lookup path and the chosen template
     plus base. Hooked in `hugolib/site_render.go`.
   - render-hook template selection (`hugolib/page__per_output.go`) and alias template selection
     (`hugolib/alias.go`).
   - a dump of the whole template store (keys, categories and descriptors) after `NewStore`.
   - for every `sort` call, whether collator order differs from byte order.

   Raw data is in `trace.tsv`, `trace-mapped.tsv`, `lines.tsv` and `store.txt`. Generated tables are in
   `appendix.md`, which is appended as Appendix A below.

> **Side finding: golden reproducibility.** A fresh build with the golden binary today differs from
> `golden/run1` in about 4690 files. The only differences are image-processing `_hu_<hash>` filenames
> and the HTML/JSON that references them, plus the known term-collision RSS files. The text content is
> identical once `_hu_[0-9a-f]+` is normalised. The image hash therefore depends on something in the
> environment, probably source file mtime or cache state; this is for the images agent to track down.
> The parity harness should normalise `_hu_` hashes, or the Rust port must reproduce the hash inputs
> exactly.

---------------------------------------------------------------------------------------------------

## 1. Every template involved in the build

### 1.1 Site layouts (58 files) and what each maps to in the neohugo template store

neohugo v0.146+ uses a new template layout (`_partials`, `_shortcodes`, `_markup`, no `_default`).
Old-style paths are converted by `fromLegacyPath` in `tpl/tplimpl/templatestore.go:1270-1298`:

- the `/_default` prefix is stripped;
- `/partials` becomes `/_partials` and `/shortcodes` becomes `/_shortcodes`;
- `x-baseof` becomes `baseof.x`.

Taxonomy, term and section legacy paths are also copied to new keys by the legacy mapping tables in
`tpl/tplimpl/legacy.go:86-130`, which are compiled in `templatestore.go:1826-1898` and applied at
`templatestore.go:1310-1420` and `1471-1497`.

The final store contents come from the dump in `work/templates-inventory/store.txt`. In the table,
"Executions" is the golden `--templateMetrics` count for the whole build, across both languages
(en, th) and all pager pages.

| Site file | Store key / category / descriptor (after mapping) | Needs baseof | Executions | Used for |
|---|---|---|---|---|
| `_default/baseof.html` | key `""`, CategoryBaseof, D{OF:html, MT:text/html} | – | (applied to 7 overlays) | base for every HTML page layout |
| `_default/single.html` | key `""`, Layout, D{Layout:single, OF:html} | yes (starts with `{{ define "title" }}`) | 235 | all regular pages except `layout: simple` and `layout: latesturl`; includes `/search/` (`layout: search` has no matching template) |
| `_default/list.html` | key `""`, D{Layout:list, OF:html} | yes | 71 | section pages (40 en + 26 th, plus pagers) |
| `index.html` | key `""`, D{Kind:home, OF:html}. Layout `index` is normalised to Kind home at `templatestore.go:1745-1751` | yes | 21 | home HTML: 1 per language plus 13 en pagers and 6 th pagers |
| `_default/index.json` | key `""`, D{Kind:home, OF:json, MT:application/json, IsPlainText:true}. Also a spurious legacy copy at key `/index` with Kind section (from `/_default/THESECTION`), never matched | no | 2 | home JSON (`/index.json`, `/th/index.json`) |
| `_default/rss.xml` | key `""`, D{OF:rss, MT:application/rss+xml, IsPlainText:false}, so it runs as an **html/template** | no | 1491 | RSS for home, sections, taxonomies and terms. It shadows the embedded `rss.xml` |
| `404.html` | key `""`, D{OF:"404", MT:text/html}. The name `404` is parsed as the *output format* identifier (`common/paths/pathparser.go:182`) | yes | 21 | 404 page, including its paginated copies (§6.1) |
| `_default/simple.html` | key `""`, D{Layout:simple}; plus spurious legacy key `/simple` Kind section | yes | 3 | `disclaimer.md`, `privacy.md`, `terms.md` (`layout: "simple"`). `--printUnusedTemplates` wrongly reports it as unused because it checks the overlay rather than the baseof variant |
| `_default/latesturl.html` | key `""`, D{Layout:latesturl}; plus spurious legacy key `/latesturl` | **no** (starts with `{{ range`) | 1 | `content/latesturl.md` (`type: seo`, `layout: latesturl`, outputs `/latesturl/index.html`) |
| `taxonomy/list.html` | **legacy-mapped** to key `""`, D{Kind:taxonomy, OF:html}, via mapping `/taxonomy/list` → `ltaxBase` (`legacy.go:113`). The original copy at key `/taxonomy` D{Layout:list} is never reached | yes | 123 | the 6 taxonomy list pages per language (brands, categories, companies, countries, ingredients, tags) plus pagers |
| `term/term.html` | **legacy-mapped** to key `""`, D{Kind:term, OF:html}, via `/term/term` → `ltermBase` (`legacy.go:89`). Originals remain at key `/term` (Kind term, and a spurious Kind section copy) | yes | 1479 | every term page plus pagers |
| `robots.txt` | key `""`, D{OF:robots, MT:text/plain, IsPlainText:true}; `robots` is the output-format identifier | no | 1 | `/robots.txt` (en only). The file content is `User-agent: *` with no trailing newline and no actions |
| `sitemap.xml` | key `""`, D{OF:sitemap, MT:application/xml}; html/template | no | 2 | `/en/sitemap.xml` and `/th/sitemap.xml`. Shadows the embedded `sitemap.xml` |
| `_default/_markup/render-heading.html` | key `""`, CategoryMarkup, D{OF:html, Variant1:heading} | n/a | 578 | every Markdown heading, in both html and json content renders |
| `_default/_markup/render-image.html` | key `""`, CategoryMarkup, D{OF:html, Variant1:image}. It **shadows** the embedded `_markup/render-image.html` at insert time (`templatestore.go:1210-1214`) | n/a | 1050 | every Markdown image |
| `partials/head.html` | `/_partials/head` | – | 1953 | every HTML page |
| `partials/header.html` | `/_partials/header` | – | 1953 | |
| `partials/footer.html` | `/_partials/footer` | – | 1953 | |
| `partials/marketing/jsonLd.html` | `/_partials/marketing/jsonld` (lower-cased) | – | 1953 | also defines inline partial `partials/marketing/jsonld/breadcrumb` (1953 runs) |
| `partials/comments.html` | `/_partials/comments` | – | 1714 | single pages and term pages; defines inline partials `partials/comment/rating-review` (22282 runs) and `partials/comment/rating-comment` (4) |
| `partials/breadcrumb.html` | `/_partials/breadcrumb` | – | 1929 | defines `breadcrumbnav`, called recursively with `{{ template }}` |
| `partials/pagination.html` | `/_partials/pagination` | – | 1694 | |
| `partials/ads/adsensehead.html` | | – | 1953 | |
| `partials/ads/adsensemanual.html` | | – | 3837 | |
| `partials/marketing/google/googleGtag.html` | `/_partials/marketing/google/googlegtag` | – | 1953 | |
| `partials/marketing/google/googleTagManagerHead.html` | `/_partials/marketing/google/googletagmanagerhead` | – | 1953 | |
| `partials/carousel.html` | | – | 21 | home only |
| `partials/related.html` | | – | 235 | |
| `partials/rating/rating.html` | | – | 200 | defines inline `partials/rating/rating-review` (1152) |
| `partials/single/socialshare.html` | | – | 1714 | |
| `partials/single/nutritionfacts.html` | | – | 126 | |
| `partials/single/ingredientslist.html` | | – | 128 | |
| `partials/single/author.html` | | – | 213 | |
| `partials/single/date.html` | | – | 231 | |
| `partials/single/whenseen.html` | | – | 39 | |
| `partials/single/taste.html`, `smell.html` | | – | 3 / 3 | |
| `partials/taxonomy/{tags,categories,countries,companies,brands,ingredients}.html` | | – | 225/224/155/226/202/195 | |
| `partials/taxonomy/company/{website,facebook,twitter,instagram,youtube}.html` | | – | 7/5/5/5/5 | term pages with `website`, `facebook` and similar front matter |
| `partials/marketing/google/googleTagManagerBody.html` | parsed, never called | – | 0 | |
| `partials/marketing/facebook/likeshare.html` | parsed, never called | – | 0 | |
| `partials/marketing/twitter/share.html` | parsed, never called | – | 0 | |
| `posts/list.html`, `posts/single.html` | key `/posts`; no `posts` section exists | yes | 0 | They must still **parse**. They reference `partial "posts/gh-comments"`, `"posts/disqus"` and `"posts/math"`, which do not exist, `.URL` and `.ReadingTime`. Parse succeeds because partial names are strings |
| `shortcodes/modalImage.html` | `/_shortcodes/modalimage` | – | 0 | must parse; uses `with … else` |
| `shortcodes/url.html` | `/_shortcodes/url` | – | 0 | must parse |

### 1.2 Embedded (internal) templates that run during this build

Embedded templates are loaded by `insertEmbedded` (`templatestore.go:1024-1098`) from
`tpl/tplimpl/embedded/templates/`. They are all `noBaseOf` and `SubCategoryEmbedded`.

| Embedded template | Executions | Why it runs |
|---|---|---|
| `alias.html` | 1494 (1481 output files; some are term-collision duplicates) | (a) The page-1 alias `<section>/page/1/index.html` (or `404/page/1.html`) for **every HTML page that has a paginator**. `renderPaginator` writes it via `writeDestAlias` (`hugolib/site_render.go:228-246`, alias write at line 244; `pagination.disableAliases` is false). (b) The default-language root redirect `/en/index.html` → `https://seeksnack.com/`. Looked up by `aliasHandler.renderAlias` (`hugolib/alias.go:48-68`) with Desc{Kind:"", OF:alias, MT:text/html} |
| `sitemapindex.xml` | 1 | kind `sitemapindex` (multilingual site), output `/sitemap.xml`. Rendered with **data = `s.h.Sites`** (`[]*hugolib.Site`) rather than a page (`site_render.go:176-179`). Uses `.SitemapAbsURL`, `.Lastmod.IsZero`, `.Lastmod.Format` |
| `_markup/render-link.html` | 558 | Every Markdown link. The user supplies no link hook, and `markup.goldmark.renderHooks.link.useEmbedded` defaults to `auto`, which becomes `fallback` for multilingual single-host sites (`config/allconfig/allconfig.go:1121-1128`) |
| `_markup/render-table.html.html` | 119 | Every Markdown table in **html** content renders. The embedded table hook is copied per output format as `render-table.<fmt>.<suffix>` (`templatestore.go:1071-1082`) |
| `_markup/render-table.json.json` | 119 | The same tables rendered for the **json** output format. `index.json` calls `.Plain` on every page, which renders content in the JSON output context. This copy is `IsPlainText:true`, so it runs as a **text/template** with no escaping |
| `_shortcodes/ref.html` | 6 | `{{< ref "privacy.md" >}}` and `{{< ref "terms.md" >}}` in `content/privacy.md:79` and `content/terms.md:11,15`. The body is `{{ ref . .Params }}` |

The embedded `rss.xml`, `sitemap.xml` and `robots.txt` are **not** in the store because the user
templates have identical node keys and win at insert time (`templatestore.go:1205-1215`: an existing
entry with at least as many identifiers is kept). The embedded `render-image.html` is shadowed the same
way. Everything else embedded is parsed but never executed:

- `_partials/{disqus, google_analytics, opengraph, pagination, schema, twitter_cards, _funcs/get-page-images}`
- all other `_shortcodes/*`
- `_markup/render-codeblock-goat.html`
- `_server/error.html`
- `_hugo/build/js/batch-esm-runner.gotmpl`

The Rust port only needs these unused ones if it wants the full function surface. Parse-time function
checks do require that every function they reference exists.

Verbatim bodies of the five embedded templates that run (from `tpl/tplimpl/embedded/templates/`):

```
alias.html
<!DOCTYPE html>
<html lang="{{ site.Language.LanguageCode }}">
  <head>
    <title>{{ .Permalink }}</title>
    <link rel="canonical" href="{{ .Permalink }}">
    <meta name="robots" content="noindex">
    <meta charset="utf-8">
    <meta http-equiv="refresh" content="0; url={{ .Permalink }}">
  </head>
</html>
```
The data is `hugolib.aliasPage{Permalink string; page.Page}` (`hugolib/alias.go:43-46`). `.Permalink`
is the **struct field**, the redirect target. Four escaping contexts appear: attr (lang), RCDATA (title),
URL attr (canonical href) and attr (meta content).

```
sitemapindex.xml
{{ printf "<?xml version=\"1.0\" encoding=\"utf-8\" standalone=\"yes\"?>" | safeHTML }}
<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  {{ range . }}
  <sitemap>
    <loc>{{ .SitemapAbsURL }}</loc>
    {{ if not .Lastmod.IsZero }}
      <lastmod>{{ .Lastmod.Format "2006-01-02T15:04:05-07:00" | safeHTML }}</lastmod>
    {{ end }}
  </sitemap>
  {{ end }}
</sitemapindex>
```

```
_markup/render-link.html
{{- $u := urls.Parse .Destination -}}
{{- $href := $u.String -}}
{{- if strings.HasPrefix $u.String "#" -}}
  {{- $href = printf "%s#%s" .PageInner.RelPermalink $u.Fragment -}}
{{- else if and $href (not $u.IsAbs) -}}
  {{- $path := strings.TrimPrefix "./" $u.Path -}}
  {{- with or
    ($.PageInner.GetPage $path)
    ($.PageInner.Resources.Get $path)
    (resources.Get $path)
  -}}
    {{- $href = .RelPermalink -}}
    {{- with $u.RawQuery -}}
      {{- $href = printf "%s?%s" $href . -}}
    {{- end -}}
    {{- with $u.Fragment -}}
      {{- $href = printf "%s#%s" $href . -}}
    {{- end -}}
  {{- end -}}
{{- end -}}
<a href="{{ $href }}" {{- with .Title }} title="{{ . }}" {{- end }}>{{ .Text }}</a>
{{- /**/ -}}
```
In this build no link starts with `#`. Of the 558 links, 555 are absolute. The other 3 are relative
(`$.PageInner.GetPage` returns a `*page.nopPage`, which is falsy through IsZero, and the resource
lookups return nil), so `$href` keeps `$u.String`. 60 links carry a title.

```
_markup/render-table.html   (copied as render-table.html.html and render-table.json.json)
<table
  {{- range $k, $v := .Attributes }}
    {{- if $v }}
      {{- printf " %s=%q" $k ($v | transform.HTMLEscape) | safeHTMLAttr }}
    {{- end }}
  {{- end }}>
  <thead>
    {{- range .THead }}
      <tr>
        {{- range . }}
          <th
            {{- with .Alignment }}
              {{- printf " style=%q" (printf "text-align: %s" .) | safeHTMLAttr }}
            {{- end -}}
          >
            {{- .Text -}}
          </th>
        {{- end }}
      </tr>
    {{- end }}
  </thead>
  <tbody>
    {{- range .TBody }}
      <tr>
        {{- range . }}
          <td
            {{- with .Alignment }}
              {{- printf " style=%q" (printf "text-align: %s" .) | safeHTMLAttr }}
            {{- end -}}
          >
            {{- .Text -}}
          </td>
        {{- end }}
      </tr>
    {{- end }}
  </tbody>
</table>
```
In this build `.Attributes` is always empty (238 ranges over an empty map) and `.Alignment` is always
`""`. The `transform.HTMLEscape` and `safeHTMLAttr` paths therefore never run.

```
_shortcodes/ref.html
{{ ref . .Params }}
```
Here `.` is `*hugolib.ShortcodeWithPage` and `.Params` is `[]interface{}{"privacy.md"}`. The call goes
to `tpl/urls/urls.go:94` `Ref`, then `refArgsToMap`, then `Page.Ref`.

### 1.3 Named templates and inline partials inside site files

The static parse finds these `define` blocks:

- **baseof blocks.** `_default/baseof.html` has `block "head"` (line 7), `block "title"` (line 10, with a
  default body at 11-17), `block "afterbody"` (22), `block "content"` (25) and `block "script"` (28).
  `block` means define plus template.
- **Overlay overrides.**
  - `head`, `afterbody`, `content` and `script` are defined in `404.html`, `index.html`, `list.html`,
    `simple.html`, `single.html`, `taxonomy/list.html` and `term/term.html`.
  - `title` is overridden only in `single.html:1`, `term/term.html:4-7` and `posts/*`.
  - Everywhere else the baseof default title runs. For home it is `printf "%s · %s" HomeTitle Site.Title`;
    for other pages it is `printf "%s · %s" .Title .Site.Title`.
- **Title escaping.** The title block sits inside `<title>`, which is RCDATA. html/template therefore
  executes it through a derived template named `title$htmltemplate_stateRCDATA_elementTitle` (seen in
  the trace at `baseof.html:10`), and its prints go through `rcdataescaper`.
- **`breadcrumbnav`** (`partials/breadcrumb.html:6-92`). It is recursive. It is called at line 3 with
  `(dict "p1" . "p2" .)` and recurses at line 8 with `(dict "p1" .p1.Parent "p2" .p2)`. The branch at
  line 10 (`.p1.Site.Home`) is **never reached** because every page's Parent chain ends at home.
- **Inline partials** extracted by `extractInlinePartials` (`templatestore.go:953-991`): any define
  whose name starts with `partials/` or `_partials/` becomes `/_partials/<name>.html`, with
  SubCategoryInline and noBaseOf. There are four:
  - `partials/comment/rating-review` (`comments.html:349-389`)
  - `partials/comment/rating-comment` (`comments.html:391-407`)
  - `partials/marketing/jsonld/breadcrumb` (`jsonLd.html:374-413`)
  - `partials/rating/rating-review` (`rating.html:66-80`)

  They are called as `partial "comment/rating-review" (dict …)` and similar.

### 1.4 Other template executions

- **Asset templates.** `resources.ExecuteAsTemplate "ts/search.ts" (dict "api" …)` and
  `"ts/comment.ts"` parse `assets/ts/search.ts` and `assets/ts/comment.ts` as **text/template**
  (`TextParse`, `templatestore.go:756`). Each contains one action: `"{{ .api }}"` at `search.ts:11` and
  `comment.ts:47`, a map-key lookup on `map[string]interface{}`. Each runs once per build because
  resources are cached. The `resources.ExecuteAsTemplate "style.seeksnack.css"` call at `head.html:271`
  sits in the development branch and never runs.
- **i18n strings.** Translation values such as `"{{ .Context }}g"` and
  `"จำนวนหน่วยบริโภคต่อ{{ .Context }}"` are Go text templates executed by go-i18n.
  `i18n "<id>" (dict "Context" x)` runs in `nutritionfacts.html`. See the i18n spec.
- **Content shortcodes.** Only the three `ref` calls exist; no user shortcode is used in content.

### 1.5 Output-format and engine choice per template

Whether a template runs under html/template or text/template follows the output format's `IsPlainText`
flag (`output/outputFormat.go:88-227`):

- **html/template, with contextual escaping:** HTML (`html`), `404`, `alias`, `rss`, `sitemap` and
  `sitemapindex`. RSS and sitemap are html/template, which is why the templates wrap the `<?xml … ?>`
  header in `safeHTML`, and why RSS attributes and text are HTML-escaped. For example
  `alt="{{ .Title }}"` inside the CDATA string becomes `alt="Alice&#39;s Coconut Mochi"`, and
  `.Description | html` becomes `&#34;…&#34;`.
- **text/template, no escaping:** `json` (`index.json`, `render-table.json.json`), `robots`
  (`robots.txt`), asset templates, and i18n (go-i18n uses text/template).

---------------------------------------------------------------------------------------------------

## 2. Template functions: inventory, call sites, argument types, Go implementation and semantics

Function lookup at execution time goes through `templateExecHelper.GetFunc`
(`tpl/tplimpl/template_funcs.go:43-58`). Hugo's function map (`TemplateFuncs`) takes precedence; the
html/template `GoFuncs` (escapers) and text/template builtins fill in behind it
(`templatestore.go:2050-2088`).

Built-ins that Hugo **overrides**:

- `eq`, `ne`, `lt`, `le`, `gt` and `ge` (tpl/compare)
- `index` and `slice` (tpl/collections)
- `print`, `printf` and `println` (tpl/fmt)

Built-ins that come from **Go** unchanged: `and`, `or`, `not`, `len` and `html`, from
`tpl/internal/go_templates/texttemplate/funcs.go:39-63`. `and` and `or` short-circuit at
`hugo_template.go:338-360`.

Namespace functions such as `resources.Get` evaluate as a zero-argument namespace function (`resources`
returns `*resources.Namespace`) followed by a method call. Some funcs take a leading
`context.Context`, injected at `template_funcs.go:44-55` and `88-114`.

Full per-site tables with dynamic argument and result types are in **Appendix A.1** (functions) and
**A.2** (methods). The summary below adds the Go implementation and the edge cases this site actually hits.

### 2.1 Functions executed in the production build

| Function | Go implementation | Call sites (seeksnack) | Semantics and edge cases this site hits |
|---|---|---|---|
| `partial` | `tpl/partials/partials.go:112 Include` → `lookup:133`, which uses `TemplateStore.LookupPartial` (`templatestore.go:586-607`) | 104 static sites. Examples: `baseof.html:8,24,26`, `single.html:19-164`, `head.html:11,12,236`, `comments.html:28-54,234-271` | Returns `template.HTML`. Name resolution is **case-insensitive** because the path parser lower-cases (`marketing/jsonLd.html` → `/_partials/marketing/jsonld`). A missing extension means html. Data types seen: `*hugolib.pageState`, `map[string]interface{}` (from dict), `[]string`, `string` and `[]interface{}`. For example `partial "taxonomy/companies" .Params.Companies` receives a string or `[]string`. No partial uses `return`, and there is no `partialCached` |
| `i18n` | `tpl/lang/lang.go:48 Translate` → `deps.Translate` (go-i18n) | `single.html:116`; `index.html:94,110-120`; `comments.html:4,110,124,132,156,158,167,170,286`; `rating.html:2`; `nutritionfacts.html` (many); `smell.html:2`; `taste.html:2` | Args are `(string)` or `(string, map[string]interface{}{"Context": int\|float64\|string})`. Returns string. `readingTime` appears only in the unused `posts/single.html` |
| `printf` | `tpl/fmt/fmt.go:52` → `fmt.Sprintf` | 48 static sites. Examples: `head.html:19,122,157`; `jsonLd.html:121,201`; `baseof.html:13,15`; `rss.xml:15,41,50`; `render-image.html:2,3`; `index.html:99,130`; taxonomy partials | Needs **exact Go fmt semantics**. (1) `printf "%s" .Params.image` with no image passes nil and yields **`%!s(<nil>)`** (`head.html:157`). That value goes to `absLangURL`, `url.Parse` fails, and the input is returned unchanged, so 3356 golden `og:image`/`twitter:image` values are `content="%!s(<nil>)"`. (2) `%q` with a `media.Type` (Stringer) and a Permalink at `rss.xml:41` uses `strconv.Quote` semantics. (3) `%d` with ints at `render-image.html:2`. (4) `printf "%s" $val` with string. (5) `printf "%s%s%s%s"` at `jsonLd.html:201` |
| `print` | `tpl/fmt/fmt.go:47` → `fmt.Sprint` | `head.html:142` `print $permalink "page/" .PageNumber "/"` | Go `Sprint` inserts spaces only between operands **when neither is a string**. Output example: `https://seeksnack.com/404.htmlpage/2/` (404 pager canonical) |
| `dict` | `tpl/collections/collections.go:154 Dictionary` | 164 static sites | Keys are strings only (no `[]string` keys used). Returns `map[string]any`; map iteration is not used on dicts except `sort` by key "position" |
| `slice` | `tpl/collections/collections.go:515` → `common/collections/slice.go Slice` | `index.json:1`, `rss.xml:3`, `head.html:275`, `jsonLd.html:379,380,385`, `list.html:112`, `single.html:223`, `index.html:228`, `404.html:74`, `taxonomy/list.html:124`, `term.html:222` | **Typed result:** `[]string` for strings, `[]int` for `slice 1 2 3 4 5 6`, and `resource.Resources` for resources. With no args it returns `[]interface{}{}` |
| `first` | `tpl/collections/collections.go:191` | `rss.xml:11` (`$pages \| first $limit`), `index.html:37`, `carousel.html:11`, `head.html:133` (`first 1 .AllTranslations`), `head.html:194` and `related.html:1` (`… \| first 5`) | limit arrives as `int` or `int64` (from `mul`/params) and goes through `cast.ToIntE`. A limit larger than the length is clamped. Returns the same slice type (`page.Pages`) |
| `where` | `tpl/collections/where.go:29` (`checkWhereArray:388`, `evaluateSubElem:296`, `checkCondition:59`) | `index.html:26`, `carousel.html:1`, `latesturl.html:1`, `rss.xml:5,7` | Always `where <page.Pages> "Params.type" "snacks"`. The key path is split on `.` to `["Params","type"]`. `evaluateSubElem` calls the method `Params()`, which returns `maps.Params`, then `GetNested(["type"])` (**case-insensitive**, lower-cased). The comparison is `string == string`. Pages without `type` yield an invalid value, and invalid versus a valid string is false. Result type is `page.Pages` |
| `sort` | `tpl/collections/sort.go:30` (pairList, `Less:172` → `compare.LtCollate` → `compareGetWithCollator` at `tpl/compare/compare.go:255`) | `index.html:26`, `carousel.html:1`, `latesturl.html:1`, `index.json:2` (`"Date"`); `list.html:25`, `taxonomy/list.html:30`, `term.html:118` (`"Title"`); `jsonLd.html:403` (`"position" "asc"`); `head.html:212,266`, `categories.html:7`, `companies.html:10`, `countries.html:10`, `ingredients.html:8`, `tags.html:9`, `term.html:96` (`[]string`, no key) | **Language-aware collation.** `collate.New(language.Tag)` for the current site language (`langs/language.go:64-80`) from golang.org/x/text v0.26.0, with collation tables from **CLDR 23 / Unicode 6.2**. Strings first go through `strconv.ParseFloat`; if both sides parse, the comparison is numeric, otherwise collator order (see §8 risk R1). The sort is **stable** (`sort.Stable`). Key "Date" compares `time.Time` via Unix seconds (float); ties keep input order. Key "Title" calls the method `Title()`. `[]map` with "position" uses map index; values are mixed `int` and `int64` and compare numerically. The dynamic check found **104 distinct sort calls (all `th`) whose collator order differs from byte order**, such as Thai prevowel handling in ingredient/tag lists and `sort .Paginator.Pages "Title"` on th term pages. There were 0 differences for `en` |
| `index` | `tpl/collections/index.go:33` | `footer.html:2-4`, `header.html:1,70`, `rss.xml:14,48`, `jsonLd.html:3,20,39,…,141,206`, `comments.html:180`, `categories.html:11`, `tags.html:3` | Receivers seen: `maps.Params` (exact key `"logo"`, `"footer"`, `"socialmedia"`, `"quicklinks"`), `map[string]interface{}` (Site.Data.comments with a missing key returns nil), `[]string`, `[]interface{}` and **`string` with int, which returns a `uint8` byte**. The last case is how `rss.xml:48` `{{ index . 0 \| humanize }}` produces `<category>115th</category>` when `categories: seafood` is a plain string (see `humanize`), and how `jsonLd.html:141` produces `"category":"115"` |
| `isset` | `tpl/collections/collections.go:326 IsSet` | `tags.html:1` `isset . 0` | On a slice it is true when `len > index` |
| `seq` | `tpl/collections/collections.go:417` | `carousel.html:6` (`seq $totalShow`, int64 5 → [1..5]); `rating.html:71`, `comments.html:398` (`seq 5`) | Returns `[]int`. Size is limited to 2000 |
| `newScratch` | `tpl/collections/collections.go:685` → `*maps.Scratch` | `jsonLd.html:375` | See the Scratch methods in §3.7 |
| `eq` | `tpl/compare/compare.go:99 Eq` | 124 static sites | Types seen: `string`/`string`; `*pageState`/`*pageState` (`breadcrumb.html:12`). The `compare.Eqer` path is `pageState.Eq`, `hugolib/page.go:206-213`: unwrap, then pointer identity; `*page.Pager`/`*page.Pager` (`pagination.html:73`, pointer equality); `int64`/`int` (normalised to int64); `nil`/`string` (`eq .p1.Data.Singular "company"` when Singular is missing, which is false). `eq hugo.Environment "development"` is always false |
| `ne` | `compare.go:159` | `rss.xml:23`, `head.html:141`, `jsonLd.html:404`, `related.html:53` | Implemented as the negation of Eq |
| `ge` | `compare.go:170` → `compareGet` | `rss.xml:10` (`int ≥ int`); **`rss.xml:27`, `footer.html:76` `ge .Site.Params.yearCreate (now.Format "2006")`** (`string`/`string`: "2019" ≥ "2026"); `rating.html:72`, `comments.html:399` (`int`/`float64` ≥ `int`) | String/string compares **numerically when both parse as floats** (`compare.go:300-310`), so 2019 ≥ 2026 is false and the output is `2019 -2026`. The output **depends on the build year** through `now` |
| `gt` / `lt` / `le` | `compare.go:182 / 220 / 194` | `jsonLd.html:23,42,…` `gt $i 0`; `pagination.html:20,63,64` | int versus int64 mixes are handled as floats |
| `and` / `or` / `not` | Go builtins (`texttemplate/funcs.go`, short-circuit in `hugo_template.go:338-360`) | many | `and`/`or` return the **deciding operand**, not a bool. For example `with and .Params.youtube_video` returns the string and `or 5-args` returns the first truthy value. Truthiness is `hreflect.IsTruthfulValue` (`common/hreflect/helpers.go:96-130`), which honours `IsZero()` (`types.Zeroer`), so `time.Time`, `*source.File` and `*page.nopPage` can be falsy although non-nil |
| `len` | Go builtin `length` | `list.html:23`, `taxonomy/list.html:25`, `term.html:116`, `jsonLd.html:395` | Always `page.Pages`. Results are assigned but unused except in jsonLd |
| `default` | `tpl/compare/compare.go:48 Default` | `head.html:178,183` (`$translation.Language.Params.LanguageCodeOpenGraph \| default "en_US"`); `date.html:6,7`, `whenseen.html:12`, `comments.html:224` (`default "…" .Site.Params.dateFormat`, where the param is missing and the arg is **nil**) | "Set" means non-empty string, non-zero number, any bool, non-zero time, or non-nil |
| `add` / `sub` / `mul` | `tpl/math/math.go:66 / 251 / 187` → `common/math/math.go DoArithmetic` | `add`: `carousel.html:7`, `pagination.html:64`, `related.html:52`. `sub`: `pagination.html:62,64`, `jsonLd.html:398`. `mul`: `list.html:18`, `index.html:28` | Results are **`int64`** even for int inputs (per the trace), and they print as decimal |
| `dateFormat` | `tpl/time/time.go:74 Format` → `htime.ToTimeInDefaultLocationE` (`common/htime/time.go:143`) → `TimeFormatter.Format` (`common/htime/time.go:95-139`) | `date.html:6,7`, `whenseen.html:12`, `comments.html:224,317` | A `time.Time` input is **formatted to RFC3339 and re-parsed**, which drops fractional seconds: `2020-11-12T13:24:30.025Z` → `2020-11-12T13:24:30Z`. String inputs (`when_seen: "2020-09-23"`, comment dates) go through `cast.ToTimeInDefaultLocationE`. Then Go `Time.Format`, followed by **locale substitution of month and day names** with go-playground/locales for the page language. Example on th pages: `Nov` → `พ.ย.`, giving `พ.ย. 12, 2020` |
| `now` | `tpl/time/time.go:84` → `htime.Now()` (`Clock`, overridable by `--clock`) | `rss.xml:27,28,30`, `footer.html:76,77,80` (`now.Format "2006"` / `"2006 "`) | Non-deterministic across years. The golden output was built in **2026** |
| `humanize` | `tpl/inflect/inflect.go:37` → `github.com/gobuffalo/flect` v1.0.3 (`Humanize`, `Ordinalize`) | `rss.xml:48`, `index.html:137`, `categories.html:17,25`, `companies.html:16`, `countries.html:16`, `term.html:113` | If the arg is an `int` or its string form passes `strconv.Atoi`, the result is `Ordinalize`. A **`uint8` from `index "seafood" 0` is 115, giving "115th"**. Otherwise the result is `Humanize(ToLower(Humanize(s)))`, which capitalises the first letter, treats `-` and `_` as separators, and upper-cases known acronyms from flect's table (`acronyms.go`). Examples: `biscuit-roll` → `Biscuit roll`, `.Data.Singular` `brand` → `Brand` |
| `urlize` | `tpl/urls/urls.go:75` → `helpers/url.go:30 URLize` = `URLEscape(MakePathSanitized(s))`, where `MakePathSanitized` is at `helpers/path.go:59` (`MakePath` → `UnicodeSanitize`, then lowercase because `disablePathToLower = false`) | `author.html:3`, `brands.html:31`, `categories.html:15,23`, `companies.html:14,22`, `countries.html:14`, `ingredients.html:12`, `tags.html:13` | Examples: `Thai Lotte Co., Ltd.` → `thai-lotte-co.-ltd.` and `Koala's March` → `koalas-march`. Thai text is kept and then percent-encoded by `url.Parse(...).String()`. One nil arg becomes `""` |
| `lower` | `tpl/strings/strings.go:418 ToLower` | `categories.html:15,23`, `ingredients.html:12`, `tags.html:13` | Applied to the already-urlized string |
| `trim` | `tpl/strings/strings.go:440 Trim` | `comments.html:91,179` `trim .RelPermalink "/"` | Go `strings.Trim` with a cutset |
| `relLangURL` | `tpl/urls/urls.go:168` → `helpers/url.go:116 RelURL(addLanguage=!multihost)` | 32 static sites (taxonomy partials, `author.html:3`) | Adds the `th/` prefix on th. **canonifyURLs = true** means `RelURL` does not add the context root (`url.go:162`). Absolute inputs are returned unchanged, for example `https://www.facebook.com/lepanbakery/`. Output relative URLs are later made absolute by the canonify post-processor, which belongs to the publishing agent |
| `absLangURL` | `tpl/urls/urls.go:180` → `helpers/url.go:53 AbsURL(addLanguage)` | `header.html:10,33` (`"/" \| absLangURL`), `head.html:157`, `jsonLd.html:121` | If `url.Parse` errors the input is returned as-is, which is what happens to `%!s(<nil>)`. Already-absolute inputs are unchanged |
| `urls.Parse` | `tpl/urls/urls.go:54` → `net/url.Parse` | embedded `render-link.html:1` | `*url.URL` with `.String()`, `.IsAbs()`, `.Path`, `.Fragment` and `.RawQuery`. It must match Go `net/url` exactly, including re-encoding in `String()` |
| `strings.HasPrefix` / `strings.TrimPrefix` | `tpl/strings/strings.go:194 / 483` | `render-link.html:3,6` | Note the TrimPrefix argument order `(prefix, s)` |
| `path.Ext` | `tpl/path/path.go:45` → `cast.ToStringE(arg)` → `path.Ext` | 40 static sites; the image `if` guards | The arg is a **resource**. `cast.ToStringE` uses `fmt.Stringer`, which is `resourceAdapter.String()` → `Name()` (`resources/transform.go:349`, the original file name, for example `koala-s-march-chocolate.jpg`). A **nil resource gives ""**; 6 such calls occur |
| `reflect.IsSlice` | `tpl/reflect/reflect.go:34` → `hreflect.IsSlice` | taxonomy partials, line 5, 8 or 9 | Args are `string`, `[]string` or `[]interface{}` |
| `safeHTML` / `safeCSS` / `safeJS` / `safeURL` | `tpl/safe/safe.go:39 / 33 / 51 / 63` (cast to `template.HTML/CSS/JS/URL`) | `safeHTML`: `rss.xml:15,40,41,47,50`, `sitemap.xml:1,8`, `head.html:122,242,253,260`, `header.html:166`, `render-heading.html:1`, embedded sitemapindex. `safeCSS`: `head.html:278`. `safeJS`: `jsonLd.html:15,273`. `safeURL`: `render-heading.html:1` | Removes html/template escaping in that context. `safeHTML` takes a `string` or an `hstring.HTML` (heading `.Text`) |
| `html` | Go builtin `HTMLEscaper` (`texttemplate/funcs.go`) | `rss.xml:50` (`.Description \| html`), `socialshare.html:56` (`"Check out this site " \| html`) | Escapes `"`→`&#34;`, `'`→`&#39;`, `&`, `<`, `>` and NUL. html/template treats it as the final escaper (chain `html` or `urlescaper → html`) |
| `jsonify` | `tpl/encoding/encoding.go:64` → `encoding/json.Encoder` with `SetEscapeHTML(true)`, trailing newline trimmed | `index.json:21` | Input `[]map[string]interface{}` from Scratch. **Map keys are sorted**, and `&`, `<`, `>`, U+2028 and U+2029 are escaped as `&` and similar (269 × `&` in the golden `index.json`). Non-ASCII is kept raw. Returns `template.HTML`, printed raw in text/template |
| `markdownify` | `tpl/transform/transform.go:180` | `comments.html:278,323` | Only on 2 pages that have comment data. Renders with the page's Goldmark converter and strips the wrapping `<p>` for a single paragraph (markdown agent) |
| `transform.Unmarshal` | `tpl/transform/unmarshal.go:38` | `jsonLd.html:204` | Input is the JSON body of a **`resources.GetRemote`** result. Output is `map[string]interface{}` with float64 numbers; `.items` is `[]interface{}` |
| `resources.Get` | `tpl/resources/resources.go:93` | 110 static sites | `(string)`. Paths used: `images/watermark.png`, `images/favicon/*`, `scss/website.scss`, `ts/*.ts`, `/vendor/…` (leading slash; `node_modules` is mounted at `assets/vendor`), `$logo` params and `site.Params.marketing.twitter.logo`. Returns `*resources.resourceAdapter`, or nil for 3 calls from `render-link.html:10` |
| `resources.GetRemote` | `tpl/resources/resources.go:118` → `resources/resource_factories/create/remote.go` (key `remoteResourceKeys:322`) | `jsonLd.html:202` | **Network**: 87 calls to `https://www.googleapis.com/youtube/v3/videos?key=…&id=<youtube_video>` on snack pages that have `youtube_video`. The golden output contains live YouTube data (title, description, viewCount, commentCount, tags) served from the **file cache** `~/Library/Caches/hugo_cache/seeksnack/filecache/getresource/<uint64 key>`, which holds 51 raw HTTP response dumps. See §8 R2 |
| `resources.Concat` | `resources.go:209` | `list.html:112`, `single.html:223`, `index.html:228`, `404.html:74`, `taxonomy/list.html:124`, `term.html:222` | `(string, resource.Resources)` |
| `resources.ExecuteAsTemplate` | `resources.go:249` | `list.html:107`, `single.html:210,214`, `index.html:223`, `404.html:69`, `taxonomy/list.html:119`, `term.html:212,216` (prod branch) | `(string, map, resource)`. Executes `assets/ts/*.ts` as a text/template (§1.4) |
| `resources.PostProcess` | `resources.go:321` | `head.html:276` | Returns `*postpub.PostPublishResource`. Its `.Content` is inlined into `<style>` on every HTML page, and the **final CSS depends on `hugo_stats.json`** of this build (PurgeCSS in `postcss.config.js` reads `./hugo_stats.json`; `build.writeStats = true`) |
| `toCSS` | `tpl/css/css.go:166` alias → `Sass` (`css.go:83`) | `head.html:276` (prod), with opts `{enableSourceMap:false, includePaths:[node_modules, assets/scss], outputStyle:compressed}` | libsass or dartsass (resources agent) |
| `postCSS` | `tpl/css/css.go:171` alias → `PostCSS` (`css.go:55`) | `head.html:276` | Runs node postcss with `HUGO_ENVIRONMENT=production` |
| `minify` | `tpl/resources/resources.go:300` | `head.html:276` | |
| `fingerprint` | `tpl/resources/resources.go:269` | 15 sites; the `js.Build … \| fingerprint` pipelines | Uses the default sha256; `.Permalink` becomes `…/js/set-theme.<hash>.js` |
| `js.Build` | `tpl/js/js.go:55` (esbuild) | 52 static sites; prod runs `(map{targetPath,minify:true}, res)` or `(map{target:"es2015"}, res)` | |
| `images.Overlay` | `tpl/images/images.go` (embeds `*images.Filters`) → `resources/images/filters.go:49 Overlay(src, x, y)` | `render-image.html:6`, `index.json:6`, `list.html:37`, `single.html:54`, `index.html:51`, `carousel.html:27`, `related.html:15`, `taxonomy/list.html:42`, `term.html:35,130` | `(*resourceAdapter, int 0, int 0)` returns `images.filter`. At `term.html:35` the filter is built but **never applied**; the next line is commented out |
| `images.Filter` | `tpl/images/images.go:118` | `render-image.html:7`, `index.json:9`, `list.html:38`, `single.html:55`, `index.html:52`, `carousel.html:28`, `related.html:16`, `taxonomy/list.html:43`, `term.html:131` | `(filter, resource)` from the pipe; the last arg is the resource |
| `hugo` (`hugo.Environment`) | `tpl/hugo/init.go:26-44` returns `neohugo.HugoInfo`; `Environment` is a struct field (`common/neohugo/neohugo.go:68`) | 17 sites | Always `"production"` |
| `site` | `tpl/site/init.go:28-42` returns the current `*page.siteWrapper` | `alias.html:2`, `head.html:152,153,160,169,221`, `header.html:107` | |
| `urls`, `strings`, `path`, `reflect`, `resources`, `images`, `js`, `transform` | Namespace accessors (no args) | as above | |
| `ref` | `tpl/urls/urls.go:94` | embedded `_shortcodes/ref.html` | `(ShortcodeWithPage, []interface{}{"privacy.md"})` returns the absolute permalink string |

### 2.2 Functions referenced in templates but never executed in production

These must still exist at parse time. Go templates fail with `function "x" not defined` during parse.

- `absURL` (`taxonomy/list.html:80`, dev only), `urldecode` (`taxonomy/list.html:80`, dev), `hugo.Generator` (`head.html:273`, dev).
- `apply` and `after` (`brands.html:10,20`; `company/*.html:11,21`). These sit in the `reflect.IsSlice` branch, and brand, website and social values are always strings in content.
- `toCSS` (dev variant at `head.html:271`) and `resources.ExecuteAsTemplate "style.seeksnack.css"` (dev).
- The `posts/*` templates reference `.URL`, `.ReadingTime`, `partial "posts/…"`, and `default` with a pipe.

### 2.3 Escaper functions html/template inserts (Appendix A.6)

The trace shows these chains, with total call counts:

| Chain | Calls |
|---|---|
| `attrescaper` | 447809 |
| `htmlescaper` | 265518 |
| `urlfilter → urlnormalizer → attrescaper` | 133112 |
| `jsstrescaper` | 111411 |
| `urlescaper → attrescaper` | 23382 |
| `srcsetescaper → attrescaper` | 20640 |
| `rcdataescaper` | 13194 |
| `jsvalescaper` | 11561 |
| `html` | 3563 |
| `cssvaluefilter` | 1953 |
| `urlescaper → html` | 1714 |
| `htmlnamefilter` | 1156 |
| `urlnormalizer → attrescaper` | 91 |

Sites that exercise unusual contexts:

- **JSON-LD inside `<script type="application/ld+json">`** (`jsonLd.html`).
  - Quoted `"{{ x }}"` gives `jsstrescaper`: `/` becomes `\/`, `'` becomes `'`, `+` becomes
    `+`, and so on.
  - Unquoted `{{ x }}` gives `jsvalescaper`, which is JSON encoding: `"contentUrl": {{ $imageLogo.Permalink }}`
    at `jsonLd.html:49,102,183,230,308,360`, `"description": {{ .Site.Params.description }}`
    (`70,276,328`), `"owns": {{ . }}` (`195`), the tag values (`196`), and `author`, `company` and
    `BaseURL` at `165,169,170`.
  - Golden examples: `"datePublished":"2020-11-12 13:24:30.025 +0000 UTC"`. Here `.PublishDate` is a
    `time.Time` printed with **Go `Time.String()`**, including fractional seconds, and then JS-escaped.
    Also `"keywords":["Koala's March","Chocolate","Thailand"]` (jsvalescaper keeps `'`) and
    `"keywords":"[Koala's March …]"` (`$ytItem.snippet.tags` is a `[]interface{}` printed with
    `fmt.Sprint` and then jsstrescaper).
- **`<style>{{ .Content | safeCSS }}</style>`** (`head.html:278`) runs `cssvaluefilter` on `template.CSS`,
  which passes it through.
- **`<h{{ .Level }}`** (`render-heading.html:1`) runs `htmlnamefilter` on an int.
- **`srcset="{{ … }}"`** (`render-image.html:11,13`) runs `srcsetescaper`.
- **URL query context:** `googleGtag.html:2` `?id={{ . }}`, `adsensehead.html:2` `?client={{…}}`,
  `index.html:11`/`404.html:11` `appId={{ . }}`, `socialshare.html:5-56` (`?u={{ .Permalink }}` and
  similar) all use `urlescaper`, which percent-encodes. `comments.html:220` `href="#comment-{{…}}"` also
  uses `urlescaper`.
- **Inline JS strings:** `googleGtag.html:8` `gtag('config', '{{ . }}')` and
  `googleTagManagerHead.html:7` use `jsstrescaper`.
- **Missing values in html/template** (for example `whenseen.html:11` `datetime="{{ $dateFormat }}"`,
  where `$dateFormat` came from `.Date.Format` on a map and is invalid) print as **empty**. html/template
  `stringify` skips untyped nil. In text/template the same value would print `<no value>`.

---------------------------------------------------------------------------------------------------

## 3. Methods and fields accessed on objects

Method and field resolution is in `evalField` (`texttemplate/hugo_template.go:156-259`), calling
`templateExecHelper.GetMethod` (`template_funcs.go:88-114`) and then `hreflect.GetMethodByName`
(`common/hreflect/helpers.go:141`). Go method names are **case-sensitive**, so `.Title` works and
`.title` does not.

Map lookups go through `GetMapValue` (`template_funcs.go:70-84`):

- for **`maps.Params`** (page `.Params`, `site.Params`, `Language.Params` and nested param maps) the key
  is **lower-cased**, so lookups are case-insensitive;
- for any other map (`map[string]interface{}` from `dict`, `.Site.Data`, JSON from Unmarshal, arrays of
  tables in config, and front-matter maps nested inside arrays) the lookup is an exact, case-sensitive
  `MapIndex`.

A missing key returns an invalid value (`missingkey=default`). A field lookup on an invalid receiver
returns zero without error (`hugo_template.go:157-161`), so chains such as `.Date.Format` on a dict
silently yield invalid.

Full per-type tables are in Appendix A.2 (methods) and A.3 (fields and map keys). The summary with Go
implementations follows.

### 3.1 Page (`*hugolib.pageState`; `hugolib.pageWithWeight0` in term/RSS ranges over weighted pages)

| Accessor | Go implementation | Sites (examples) | Notes |
|---|---|---|---|
| `.Title` | `hugolib/page__meta.go:222` | 46 static | Defaults for auto sections and taxonomies are set in `page__meta.go:740-790`: `Biscuits`, `Potato-Chips`, `Brands`, and `404 Page not found` for 404 (content-model agent) |
| `.Permalink` / `.RelPermalink` | targetPaths holder (`hugolib/page__per_output.go` ~470-500; `page__output.go:31-40`). For output formats that are not permalinkable, the main format is used | 90 / 67 | Percent-encoded for non-ASCII |
| `.Params` | `page__meta.go:198` (`maps.Params`) | many | Keys in front matter are lower-cased at load. Keys used include: `author`, `date` (**`time.Time`**), `description`, `image`, `image_preview`, `image_carousel`, `categories`/`tags`/`ingredients`/`countries` (`[]string`, sometimes `[]interface{}` or a plain `string`), `brands`/`companies` (`string` or `[]string`), `when_seen` (`string`), `nutrition_facts` (nested `maps.Params`), `ingredients_percentage` (**`[]interface{}` of `map[string]interface{}`**, not Params, so `.name`/`.percentage` are exact keys), `rating` (`maps.Params` with int/float/string values), `youtube_video`, `type`, `references`, `website`/`facebook`/`twitter`/`instagram`/`youtube`, `title`. Also `LowPrice`, `HighPrice` and `Rating_tastytaste`, which are always absent except `rating_tastytaste` on 2 pages |
| `.IsHome` `.IsNode` `.IsPage` `.IsSection` `.Kind` `.Type` `.Section` `.Lang` | `page__meta.go:147/181/185/210/155/228/214/135` | `baseof.html:12`, `head.html:2,15,141,151,201,238`, `breadcrumb.html`, `rss.xml:2,4`, `sitemap.xml:11-17`, `jsonLd.html:5` | **`IsNode` is true for kind `404`** because it is `!IsPage`, so `head.html:2-3` calls `.Paginator` on the 404 page (§6.1). `.Type` is `snacks` for pages with `type: snacks`, otherwise the section name, and `page` for home/404. `.Kind` values: `home`, `section`, `page`, `taxonomy`, `term`, `404` |
| `.Date` `.Lastmod` `.PublishDate` | `page__meta.go:115/123/119` | `rss.xml:39,40,47`; `head.html:239-260`; `sitemap.xml:7,8`; `jsonLd.html:132,133` | Results are `time.Time`. Methods on them: `.Format` (Go layout, **not localised**), `.IsZero`, and printing via `String()` |
| `.Description` | `page__meta.go:131` | 11 | |
| `.Content` | `page__per_output.go:152` | `single.html:96`, `simple.html:21`, `term.html:82` | Returns `template.HTML` |
| `.Plain` | `page__per_output.go:188` | `index.json:16` | Renders each page's content **in the JSON output format context**, which triggers `render-table.json.json` and the HTML hooks with weight 1 (§5.4) |
| `.Resources` then `.Get` | `hugolib/page.go:361`; `resource.Resources.Get` at `resources/resource/resources.go:106` | many | **Case-insensitive** name match (`EqualFold`). A nil or missing name returns nil |
| `.AllTranslations` `.Translations` `.IsTranslated` | `hugolib/page.go:411/446/398` | `head.html:125-135,168-189`, `header.html:95-133`, `sitemap.xml:22` | Sorted by language weight (en, th) |
| `.Language` | page `Language()` → `*langs.Language` | `head.html:129,178,183`, `header.html:116,124` (prints `String()` = Lang) | |
| `.AlternativeOutputFormats` | `hugolib/page.go:556` | `head.html:121` | Home gives JSON+RSS; section/taxonomy/term give RSS |
| `.OutputFormats.Get "RSS"` | `page__paths.go:107`, `resources/page/page_outputformat.go:88` (case-insensitive name) | `rss.xml:40` | |
| `.Scratch` | `hugolib/page__common.go:111` returns `*maps.Scratch` | `index.json:1,11,21`, `head.html:159`, `comments.html:192-341` | |
| `.Pages` / `.RegularPages` | `hugolib/page.go:306/281` | `rss.xml:5,7` | |
| `.Data` (`.Data.Pages`, `.Data.Singular`) | `hugolib/page__data.go:31` returns `page.Data` (a `map[string]any` type with a `Pages()` method) | `list.html:16`, `sitemap.xml:4`, `breadcrumb.html:18-77`, `term.html:113` | `Singular` is an exact map key; it is missing on non-taxonomy pages, which prints empty and compares as `eq nil "company"` false |
| `.Site` | `hugolib/page.go:373` returns `*page.siteWrapper` | 181135 calls | |
| `.Paginator` / `.Paginate` | `hugolib/page__paginator.go:72 / 45`, **sync.Once** | `head.html:3`, `list.html:16,23,25`, `index.html:26`, `pagination.html:1`, `taxonomy/list.html:25,30`, `term.html:116,118` | The **first call wins** (§6.1) |
| `.Parent` | `hugolib/page__tree.go:116` | `breadcrumb.html:7,8`, `jsonLd.html:389` | Home returns nil (`page.Page` interface), which is falsy |
| `.Sitemap.ChangeFreq` | `page__meta.go:218` returns `config.SitemapConfig` (struct field) | `sitemap.xml:8` | `weekly` |
| `.File.UniqueID` | `page__meta.go:143` → `source/fileInfo.go:94,119` = **md5 hex of the content-relative path** | `comments.html:97` | Example: `md5("biscuit/koalas-march-chocolate/index.en.md") = 88a69a13…`. `with .File` is **false for term pages without a content file**: 1468 times the `*source.File` is non-nil but `IsZero` |
| `.Page` | `hugolib/page__tree.go:185` (self) | `index.json:7`, `head.html:170,171`, `header.html:108,109` | |
| `.GetPage` (render-hook `PageInner`) | `hugolib.pageForRenderHooks` | `render-link.html:8` | Returns `*page.nopPage`, which is falsy, for relative link targets |

### 3.2 Site (`*page.siteWrapper`, `resources/page/site.go`)

| Accessor | Line | Sites | Notes |
|---|---|---|---|
| `.Title` | 219 | 29 static | `SeekSnack` |
| `.Params` | 272 (`maps.Params`, case-insensitive) | 115771 calls | Keys used (as written in templates): `HomeTitle`, `description` (per-language override), `footer`/`header`, `Monetization.Ads.{Showads,manualads,DataAdClient,DataAdSlot}`, `Marketing.Google.{GoogleAnalyticsGtag,GoogleTagManager}`, `marketing.twitter.logo`, `Social.Facebook.{Appid,Link,Enable}`, `Social.twitter.name`, `twitter` (**missing**), `Comment.{Apipro,enabled,recaptcha.siteKey,recaptcha.encryptedKey}`, `Typescript.Compiler.Target`, `Carousel.TotalShow` (int64), `Api.Youtube`, `BaseURL`, `baseURLSearch`, `company`/`Company`, `author`, `authorEmail`, `yearCreate` (string), `hideCopyright` (bool), `DynamicContent` (bool false), `dateFormat`/`dateformat` (**missing**), `Sitemap.Priority.{Section (float64 0.3), term (float64 0.5), page (int64 1), Tanoxomy (typo, never reached)}` |
| `.BaseURL` | 247 | `404.html:34`, `jsonLd.html` | `https://seeksnack.com/` |
| `.LanguageCode` | 223 | `baseof.html:4` | The language's `LanguageCode` (`en`/`th`), **not** the top-level `en-us` |
| `.Language` | 187 | `alias.html:2` (via `site`), `nutritionfacts.html:1` | `.Language.Params.LanguageCode` and `.Language.LanguageCode` (`langs/language.go:101,108`) |
| `.Languages` | 191 | `head.html:169`, `header.html:107` | `langs.Languages`; elements use `.Lang` (field) and `.LanguageName` (field) |
| `.RegularPages` | 199 | `index.html:26`, `carousel.html:1`, `latesturl.html:1`, `head.html:194`, `related.html:1`, `rss.xml:5` (home via `$pctx = .Site`) | Default page order |
| `.RegularPages.Related .` | `resources/page/pages_related.go:56` | `head.html:194`, `related.html:1` | Config `related` block: indices categories(100), brands(80), companies(60); threshold 80; `includeNewer=false`; `toLower=false` (content-model agent) |
| `.AllPages` | 195 | `index.json:2` | **All languages**, so `en/index.json` includes Thai pages |
| `.Taxonomies` | 251 returns `page.TaxonomyList` (`map[string]Taxonomy`, where `Taxonomy` is `map[string]WeightedPages`) | `index.html:98,128` | Ranged as a **map with sorted keys** (§4) |
| `.GetPage` | 183 | `index.html:99,130` | `"/brands"`, `"/brands/<key>"` |
| `.Data` | 280 | `comments.html:178-182` | `map[string]any` built from `data/comments/**.json`; exact keys |
| `.Config.Services.RSS.Limit` | 239 returns `page.SiteConfig`, then struct fields | `rss.xml:9` | `10` (int) from top-level `rssLimit` |
| `.Home` | 211 | `breadcrumb.html:10` | Never executed |
| `.SitemapAbsURL`, `.Lastmod` (`*hugolib.Site`) | `hugolib/site.go` | embedded sitemapindex | |

### 3.3 Pager (`*page.Pager`, `resources/page/pagination.go`)

| Accessor | Line |
|---|---|
| `.PageNumber` | 81 |
| `.URL` | 86 |
| `.Pages` | 92 |
| `.HasPrev` / `.Prev` | 157 / 162 |
| `.HasNext` / `.Next` | 170 / 175 |
| `.First` / `.Last` | 183 / 188 |
| `.Pagers` | 193, returns the `page.pagers` slice |
| `.PagerSize` | 205, value 12 (`pagination.pagerSize`) |
| `.TotalPages` | 210 |

Sites: `pagination.html` (all), `list.html:17,18`, `index.html:27,28`, `head.html:141-142`.

### 3.4 Resources (`*resources.resourceAdapter`, `resources/transform.go`)

| Accessor | Line | Notes |
|---|---|---|
| `.Permalink` / `.RelPermalink` | 318 / 339 | May trigger publishing or transformation |
| `.Resize "WxH [webp]"` | 260 | Image processing (images agent) |
| `.Width` / `.Height` | 378 / 264 | `render-image.html:2,20,21` |
| `.Title` | 353 | `carousel.html:43` (resource title) |
| `.Content` (postpub) | `resources/postpub/postpub.go` | `head.html:278` |
| String conversion | 349, `Name()` | Via `path.Ext` |

### 3.5 OutputFormat and MediaType

- `page.OutputFormat.Rel` (struct field `Format.Rel`, `alternate`), `.Permalink`, `.MediaType` and
  `.MediaType.Type` (`media.Type` struct field `Type`): `head.html:122`, `rss.xml:41`.
- In `rss.xml:41`, `%q` of `.MediaType` goes through its `String()`, giving `"application/rss+xml"`.

### 3.6 Language (`*langs.Language`)

| Accessor | Kind | Value |
|---|---|---|
| `.Lang` | field | `en` / `th` |
| `.LanguageName` | field | `🇺🇸 English` / `🇹🇭 ไทย` |
| `.LanguageCode` | method | |
| `.Params` | method | `LanguageCodeOpenGraph`, `LanguageCode`; case-insensitive |
| printed via `String()` | method | Returns Lang (`header.html:116`, `id="{{ $translation.Language }}"`) |

### 3.7 Scratch (`*maps.Scratch`, `common/maps/scratch.go`)

| Method | Line | Semantics |
|---|---|---|
| `Set` | 73 | Overwrites |
| `Get` | 89 | Returns the value, or nil if missing |
| `Add` | 41-66 | For an existing slice: `collections.Append` (`common/collections/append.go:24`). The type tightens, for example `[]interface{}` + `map` gives `[]map[string]interface{}`, and `[]interface{}` + page gives `page.Pages`. For an existing number: `math.DoArithmetic '+'`, giving `int64`. For a missing key: stores the addend |
| `SetInMap` / `Delete` | 108 / 81 | |

All of these return `""`, which is printed. That is harmless, but in text/template (`index.json`) the
empty string is printed and trimmed by `{{-`/`-}}`.

### 3.8 Other receivers

- `time.Time`: `.Format` (Go layout, not localised), `.IsZero`.
- Goldmark render-hook contexts:
  - heading (`goldmark.headingContext`): `.Level` (int), `.Anchor` (string), `.Text` (`hstring.HTML`);
  - image (`imageLinkContext`): `.Destination`, `.Page` (`*pageForRenderHooks`), `.Text`;
  - link (`linkContext`): `.Destination`, `.Title`, `.Text`, `.PageInner`;
  - table (`tables.tableContext`): `.Attributes`, `.THead`, `.TBody`, with `hooks.TableCell.Alignment`
    and `.Text`.
- `hugolib.aliasPage.Permalink` (struct field).
- `hugolib.ShortcodeWithPage.Params` (field, `[]interface{}`).
- `url.URL`: `String()`, `IsAbs()`, `.Path`, `.Fragment`, `.RawQuery`.
- `neohugo.HugoInfo.Environment` (field).
- `services.RSS.Limit` (field).

---------------------------------------------------------------------------------------------------

## 4. Control structures and syntax the templates use

From the static parse over all 58 files. Every construct listed here must be supported by the Rust
template engine and behave exactly as `text/template` plus `html/template` do.

- **Actions and whitespace trimming.**
  - `{{- ` and ` -}}` appear in 28 files; the heaviest users are `jsonLd.html` (97/80), `head.html` (56/33)
    and `rss.xml` (22/20).
  - Trim markers remove *all* adjacent whitespace (space, `\t`, `\r`, `\n`).
  - A trim marker needs a space: `{{- x` and `{{-x` are different.
- **Comments.**
  - 47 comment nodes, including multi-line ones. `nutritionfacts.html:278-280` is a comment that contains
    `}}` inside it:
    ```
    {{/* TODO Because of Netlify BUG, then this cannot loop by default value }}
    range $name := .vitamins ... % */}}
    ```
    The lexer must scan for `*/` followed by an optional trim marker and `}}`.
  - `{{- /**/ -}}` appears in the embedded render-link.
  - `{{/* *- ... -* */}}` appears at `term.html:36`.
- **Variables.**
  - `$x := …` appears 305 times; assignment `$x = …` appears 47 times. Examples: `pagination.html:59-70`
    `$ellipsed`/`$shouldEllipse`; `head.html:134,139,142` `$permalink` reassigned inside `range` and
    `with`; `related.html:52`.
  - Variables are scoped to the enclosing block, but assignment updates the outer variable.
  - The same name is re-declared in the same scope in `index.html:48-53` (`{{ $imageFile := $imageFile.Resize … }}`)
    and in the `$watermark` re-declarations in `index.json:4-5`. This is legal in Go templates.
  - `$` is the root data.
- **Pipelines and sub-expressions.**
  - `|` pipelines appear 153 times. Parenthesised sub-expressions appear 308 times. There are multi-line
    pipelines inside an action, for example `list.html:85-87` where `resources.Get … |` continues on the
    next line, and a multi-line `with or (…) (…) (…)` in render-link.
  - Chained calls on a sub-expression result: `(sort … "Date").Reverse` (4 sites) and
    `(resources.Get "…").Permalink` (20 sites in `head.html`). `($scratch.Get "current").Parent`
    (`jsonLd.html:389`) calls a method on a Scratch result.
- **Method calls with arguments:** `.Paginate …`, `.Resize "300x240"`, `.Scratch.Set "k" v`,
  `.OutputFormats.Get "RSS"`, `.Site.GetPage …`, `.Resources.Get x`, `.Site.RegularPages.Related .`,
  `now.Format "2006"`, `.Date.Format "…"`.
- **Field chains on maps** (case-insensitive when a `maps.Params` is involved):
  `.Site.Params.Monetization.Ads.Showads`, `$ytItem.snippet.thumbnails.standard.url`, `.p1.Data.Singular`,
  `.context.Site.Language.Params.LanguageCode`.
- **`if` / `else if` / `else`.** 241 `if`, 97 `else`, 32 `else if`. Examples: the `breadcrumb.html`
  chains and the `index.html:109-124` taxonomy name chain.
- **`with` / `else`.** 223 `with`; `with … else` appears only in the unused `modalImage.html`. `with`
  rebinds dot only when the value is truthy.
- **`range`.**
  - 39 × `range pipeline`, 22 × `range $i, $e := …`, 8 × `range $x := …`.
  - Ranged types: `page.Pages`, `[]string`, `[]interface{}`, `[]int`, `[]map[string]interface{}`,
    `maps.Params` (footer quicklinks), `map[string]interface{}` (Site.Data.comments and the table
    attributes), `page.TaxonomyList`, `page.Taxonomy`, `page.OutputFormats`, `page.pagers`,
    `langs.Languages`, `[]*hugolib.Site`, `[]hooks.TableRow`.
  - **Maps range in sorted key order** (`fmtsort.Sort`, `exec.go` walkRange Map case). With two
    variables, the first gets the key.
  - `range $i := (seq 5)` with one variable binds the element.
  - `{{ range … }}{{ else }}` is never used. `break` and `continue` are not used.
- **`define` / `block` / `template`.** Covered in §1.3. `template "breadcrumbnav" (dict …)` is
  recursive. `block` appears in baseof.
- **Nil and missing handling.**
  - Missing map keys and fields on nil yield invalid values. In an html/template print they become `""`;
    in `if`/`with` they are false; passed as a function argument they become an untyped nil
    (`printf "%s" nil` gives `%!s(<nil>)`, and `default` falls back).
  - `eq nil "x"` is false.
  - `.Date.Format` on a `map` is invalid and **does not error**, because `.Date` is a map-key miss that
    yields invalid, and `.Format` on an invalid receiver returns zero.
- **Literals:** strings, raw strings (backticks at `head.html:122` and `header.html:166-182`),
  integers, `true`/`false`, and `nil` (not used).
- **Truthiness.** `hreflect.IsTruthfulValue` (`common/hreflect/helpers.go:96-130`):
  - A value implementing `IsZero()` uses it. This covers `time.Time`, `*source.File`, `*page.nopPage`,
    and possibly others.
  - Otherwise: non-empty string, slice or map; non-zero number; true; non-nil pointer or interface;
    structs are always true.
  - Observed in the trace: `with .File` on term pages is false even though the pointer is non-nil.

---------------------------------------------------------------------------------------------------

## 5. Layout lookup: what is chosen for each page kind, and why

### 5.1 Algorithm (neohugo v0.146+ template store)

1. **Query construction.** `pageState.resolveTemplate` (`hugolib/page.go:494-517`) calls
   `GetInternalTemplateBasePathAndDescriptor` (`page.go:479-491`). It builds:
   - `Path = p.PathInfo().BaseReTyped(p.m.pageConfig.Type)` (`common/paths/pathparser.go:640-651`).
     If front matter sets `type`, the first path segment is **replaced by the type**:
     `/biscuit/koalas-march-chocolate` becomes `/snacks/koalas-march-chocolate`, and `/latesturl`
     (`type: seo`) becomes `/seo`.
   - `Desc = {Kind, Lang, LayoutFromUser: p.Layout() (front matter "layout"), OutputFormat, MediaType, IsPlainText}`.
2. **Normalisation.** `TemplateQuery.init` (`templatestore.go:400-425`):
   - a kind that is not one of the 5 main kinds (`404`, `sitemap`, `sitemapindex`, `robotstxt`, alias)
     becomes `Kind=""`;
   - if `Kind != ""`, `LayoutFromTemplate` becomes `single` for pages and `list` for all other kinds.
3. **Template walk.** `LookupPagesLayout` (`templatestore.go:563-584`) calls `findBestMatchWalkPath`
   (`823-852`). It walks tree keys from the root to the query path (`""`, then `/snacks`, then
   `/snacks/<slug>`) and scores every CategoryLayout candidate with `descriptorHandler.compareDescriptors`
   and `doCompare` (`templatedescriptor.go:63-223`).
4. **Rejection rules** in `doCompare`. A candidate does not match when any of these holds:
   - it is plain-text and the query is not;
   - its `Kind` is set and differs;
   - its `LayoutFromTemplate` is set, is not `all`, and differs from both the query layout and
     `LayoutFromUser`;
   - its `Lang` is set and differs;
   - its `OutputFormat` differs **and** (its media type differs, or the special skip rule at
     `templatedescriptor.go:125-131` applies — the skip rule does not apply to baseof);
   - its `MediaType` differs.
5. **Weights** (`templatedescriptor.go:152-221`):
   - base: `w1 = 1`;
   - `+5` kind match;
   - `+4` standard layout match, or `+2` for layout `all`;
   - `+6` custom layout match (`LayoutFromUser`), which also sets `w2 = 2`;
   - `+1` lang match, **or** when the template has no lang and the query lang equals
     `defaultContentLanguage` (en). So en scores are 1 higher than th, but the ranking is the same;
   - `+4` output-format match;
   - `+1` media-type match;
   - `+6` Variant1, `+4` Variant2.
6. **Tie-breaks.** `bestMatch.isBetter` (`templatestore.go:1940-1993`):
   - user templates are preferred over embedded ones, except for markup hooks;
   - a closer distance wins only if w2/w3 are not worse;
   - then higher w1;
   - then lexically smaller path.
7. **Base template.** If the chosen template is not `noBaseOf`, `findBestMatchBaseof`
   (`templatestore.go:326-347`) picks the best base variant along the path. Variants are pre-built at
   parse time:
   - `FindAllBaseTemplateCandidates` (`468-485`) finds compatible baseof templates;
   - `applyBaseTemplate` (`templates.go:127-182`) clones the namespace, parses **baseof first and the
     overlay second** into the same named template, and stores the result under `(base key, base D)`.
   - Whether a layout needs a base is decided by `needsBaseTemplate` (`templates.go:265-300`): the first
     action that is not a comment is `define`.
   - The overlay's top-level text is whitespace and defines, so it does not replace the base body. This is
     the Go text/template "empty template body does not overwrite" rule.

### 5.2 Result table for this site (dynamic trace, `lines.tsv` RENDER records)

| Kind / output format | Count (en+th) | Query (Path, Desc) | Chosen template, weight reasoning | Base |
|---|---|---|---|---|
| page / html, `type: snacks` | 229 | `/snacks/<slug>`, {Kind:page, Layout:single} | `/single.html` (key "": 1 + 4 + 1(en) + 4 + 1 = 11). No `layouts/snacks/` exists, so the type rewrite has **no layout effect**. It does change `.Type` (used by `jsonLd.html:5`) and the lookup path | `_default/baseof.html` |
| page / html, no type (5 th pages: type = section, i.e. `biscuit-stick`×2, `crepe`, `seafood`, `sponge-cake`) | 5 | `/<section>/<slug>` | `/single.html` | baseof |
| page / html, `layout: simple` (disclaimer, privacy, terms) | 3 | {LayoutFromUser: simple} | `/simple.html`: 1 + 6(custom) + 1 + 4 + 1 = 13, beating single 11 | baseof |
| page / html, `type: search`, `layout: search` | 1 | `/search`, LayoutFromUser search (no match; `LayoutFromUserMustMatch` is false) | `/single.html` | baseof |
| page / html, `type: seo`, `layout: latesturl` | 1 | `/seo` | `/latesturl.html` = 13 | **none** (no define) |
| home / html | 2 | `/`, {Kind:home, Layout:list} | `/index.html` D{Kind:home} = 1 + 5 + 1 + 4 + 1 = 12, beating list.html 11 | baseof |
| home / json | 2 | {Kind:home, OF:json, IsPlainText} | `/index.json` = 12. HTML candidates are rejected on media type | none (text/template) |
| home / rss | 2 | {Kind:home, Layout:list, OF:rss} | `/rss.xml` D{OF:rss} = 1 + 1 + 4 + 1 = 7. list.html is rejected on output format and media type | none |
| section / html | 66 | `/<section>`, {Kind:section, Layout:list} | `/list.html` = 11 | baseof |
| section / rss | 66 | | `/rss.xml` | none |
| taxonomy / html | 12 | `/brands` etc., {Kind:taxonomy, Layout:list} | **legacy-mapped** `taxonomy/list.html`, D{Kind:taxonomy} at key "" = 12, beating list.html 11 | baseof |
| taxonomy / rss | 12 | | `/rss.xml` | none |
| term / html | 1411 | `/brands/lays`, {Kind:term, Layout:list} | **legacy-mapped** `term/term.html`, D{Kind:term} = 12 | baseof |
| term / rss | 1411 | | `/rss.xml` | none |
| 404 / 404 | 2 (+19 pagers) | `/404`, Kind→"", OF "404" | `/404.html` D{OF:404} = 1 + 1 + 4 + 1 = 7. `list.html` is rejected (layout list vs ""), `index.html` is rejected (kind), and the embedded `alias.html` is rejected (OF differs, same MT, skip rule because query Kind is "") | baseof (baseof compare ignores OF via the skip rule for CategoryBaseof) |
| sitemap / sitemap | 2 | `/_sitemap.xml`, OF sitemap | `/sitemap.xml` (user) | none |
| sitemapindex | 1 | OF sitemapindex, **data = `s.h.Sites`** | embedded `/sitemapindex.xml`. The user `sitemap.xml` is rejected: OF differs, MT is the same, and the skip rule applies | none |
| robotstxt / robots | 1 (en only; `shouldRenderStandalonePage`, `site_render.go:52-66`) | OF robots | `/robots.txt` | none |
| alias | 1481 files | {Kind:"", OF:alias, MT:text/html}, path = page base (`alias.go:48-66`) | embedded `/alias.html` (the only OF:alias candidate) | none |

### 5.3 Partial and shortcode lookup

- **Partials.** `LookupPartial` (`templatestore.go:586-607`) parses the name as
  `ComponentFolderLayouts` + `TypePartial`. That lower-cases it and resolves the output format and media
  type from the extension; with no extension it defaults to **html**. It then calls
  `findBestMatchGet("/_partials/<path-without-ext>")`.
- **Partial caching.** The result is cached per name in `cacheLookupPartials`.
- **Legacy partial names.** Old-style `partials/…` template names are aliased into the parse namespace
  (`templates.go:113-119`). This matters for `{{ template "partials/…" }}`, which this site does not use.
- **Shortcodes.** `LookupShortcode` (`templatestore.go:618-671`) walks `treeShortcodes` by page path and
  name. Only the embedded `ref` is used.

### 5.4 Markdown render-hook lookup (CategoryMarkup)

`hugolib/page__per_output.go:270-395` builds a descriptor from the page and the current output format,
with `Variant1` set to `link`, `image`, `heading`, `table`, `blockquote` and so on. It calls
`LookupPagesLayout` with CategoryMarkup and a `Consider` filter.

Results from the trace (`HOOK` lines):

- **html output:**
  - `render-heading.html` (user)
  - `render-image.html` (user)
  - `render-link.html` (embedded, because `ignoreEmbedded=false`: auto became fallback)
  - `render-table.html.html` (embedded)
  - blockquote: none, so Goldmark's default rendering is used
- **json output** (when `index.json` calls `.Plain`):
  - the **same user html hooks** for heading and image, and the embedded html link hook. For a markup
    template with no full match, `compareDescriptors` sets `w1 = 1` when the variants match and the
    query output format is not the default (`templatedescriptor.go:70-79`);
  - the table hook uses the dedicated embedded `render-table.json.json`, a text/template.
- **Cache reuse.** Hook renderers are cached per (type, id, output format). If more than one output
  format candidate exists, `incrPageOutputTemplateVariation` forces per-format content rendering.

---------------------------------------------------------------------------------------------------

## 6. Order-dependent and stateful behaviours that affect bytes

### 6.1 The paginator is created on the first call (sync.Once)

`head.html:2-4` runs `{{ if or .IsHome .IsNode }}{{ $pag = .Paginator }}` **before** the `content`
block. It runs inside `baseof.html:8`'s `partial "head.html"`, which comes before `block "content"` at
line 25. As a result:

- **Home.** `.Paginator` (`page__paginator.go:72-117`) paginates **`s.RegularPages()`**. That is all
  regular pages of the language in default order: snacks, `disclaimer`, `privacy`, `terms`, `search`
  and `latesturl`. The later `.Paginate (sort (where … "snacks") "Date").Reverse` in `index.html:26`
  **returns the already-built pager and ignores its argument**. That explains the
  `{{ if eq .Params.Type "snacks" }}` filter at `index.html:38`, and why some home pager pages show fewer
  than 12 cards: golden `page/14/index.html` shows **0** product cards, because its 12 pages are all
  non-snack pages. The result is 14 en pages (home + `page/2..14`) and 7 th pages.
- **Section.** `head.html` builds from `p.RegularPages()`, and `list.html:16`'s `.Paginate .Data.Pages`
  gets the same pager.
- **Term and taxonomy.** Built from `p.Pages()`.
- **404.** `IsNode()` is true, so the 404 page gets a paginator over `RegularPages()`, which resolves to
  the site's regular pages. It renders `404/page/N.html` for N = 2..14 (en) and `th/404/page/N.html`,
  plus the page-1 alias `404/page/1.html`. The output format `404` has `Ugly: true`, which gives
  `…/page/N.html`. The canonical is `https://seeksnack.com/404.htmlpage/2/`, from
  `print $permalink "page/" .PageNumber "/"`.
- **Pager pages** are rendered by `renderPaginator` (`site_render.go:228-270`) with the **same
  template** and `p.paginator.current` advanced, so every template re-runs per pager page. That includes
  the carousel, sidebar and `head.html` on home.
- A Rust implementation must reproduce "first caller builds the pager, later callers receive it
  unchanged" semantics per page and output format.

### 6.2 Scratch state

`.Scratch` is per page and is shared across that page's output formats and pager renders.

- `comments.html:192-341` sets `hasComments`, `hasReplies`, `threadID` and `replyIndices` per page
  render.
- `index.json:1` starts with `$.Scratch.Add "index" slice` on the home page. For the JSON output that is
  the first use, so the key is new and the call stores `[]interface{}{}`. If the key already held a slice
  (for example from an earlier output render of the same page), `Add` would **append**. In this build
  `index.json` renders once per language; the Rust port should still keep Scratch semantics identical.
- `head.html:159` `.Scratch.Set "og_image"` is never read.
- `newScratch` in `jsonLd.html:375` is fresh per call.

### 6.3 Time and network dependence

- `now` (copyright year, §2.1).
- `resources.GetRemote` (YouTube API, §2.1). The cache is keyed by
  `hashing.HashString(uri, optionsMap)` (`remote.go:322-333`, `common/hashing/hashing.go:81`,
  hashstructure-based).
- `hugo_stats.json` feeding PurgeCSS through `resources.PostProcess`.

### 6.4 Data-dependent collisions

Terms `Lays` and `Lay's` urlize to the same path `/brands/lays/` (also `/tags/lays/`, several
`ingredients/*`, and `th/ingredients/*`). Both render to the same file, and the last writer wins. This is
nondeterministic in Go (8 RSS files differ between `golden/run1` and `run2`). The parity harness must
whitelist these paths.

---------------------------------------------------------------------------------------------------

## 7. Other byte-relevant details found

- **`title` block whitespace.** `term.html:4-7` produces `<title>Koala's March ·\nSeekSnack</title>`.
  The newline survives minification.
- **Language selection and hreflang.** `header.html:107-132` and `head.html:125-139`. Canonical links on
  th pages point to the **en** translation (`range first 1 .AllTranslations` sets `$permalink`), for
  example `th/page/3/` has canonical `https://seeksnack.com/page/3/`.
- **Carousel image sources.** `carousel.html` uses `$imageFile.RelPermalink` (jpg) for the webp
  `<source>` too; `$imagewebp` is computed but unused. The image is still generated, which affects the
  output file list.
- **`term.html:28-61` watermark.** It builds `$watermarkFilter` but never applies it (commented out),
  while still calling `$watermark.Resize "600x480"`, which is a processed image that gets published.
- **Sitemap priority.** `sitemap.xml:11` compares `.Kind` with `"tanoxomy"` (a typo), so taxonomy pages
  get priority `0`.
- **Sitemap types.** `.Site.Params.Sitemap.Priority.Section` is float64 `0.3` and prints as `0.3`;
  `term` is float64 `0.5`; `page` is int64 `1`.
- **RSS channel title.** `rss.xml:19` `eq .Title .Site.Title` is true once (the en home, whose `.Title` is
  the site title) and false 1490 times. On 2 pages the `with .Title` is false because `.Title` is `""`
  (the th home and an empty-name term), so only `.Site.Title` prints. Golden output: `index.xml` and
  `th/index.xml` have `<title>SeekSnack</title>`; `brands/index.xml` has `<title>Brands on SeekSnack</title>`.
- **RSS limits.** `$limit` (10) is applied **after** `where`, with `first`. Taxonomy RSS pages
  (`.Pages` = term pages, which have no `type`) have 0 items.
- **Comments data.** Data is only found for keys at depth 2 (`<parent>/<child>` equal to the slug).
  Thai slugs (`th/...`) never match the 3-level data path. So `data/comments/th/…` never renders, and
  only 2 pages render comments.
- **`with .Params.when_seen`.** Values are `""` for 132 pages, which is falsy. Where present, `whenseen`
  prints `datetime` empty, because `$dateFormat` is invalid, and a localised date.

---------------------------------------------------------------------------------------------------

## 8. Recommendations for the Rust port (templates-inventory subsystem)

### 8.1 Port line by line from Go

These are the code paths that decide *which* template runs and *what* the template functions return.
Deviating from them changes bytes.

| Go source | Lines | What to port |
|---|---|---|
| `tpl/tplimpl/templatestore.go` | 2096 | Store insertion, legacy path conversion, embedded insertion and shadowing, baseof variants, `LookupPagesLayout`, `LookupPartial`, `LookupShortcode`, `bestMatch.isBetter`, `toKeyCategoryAndDescriptor`, inline-partial extraction |
| `tpl/tplimpl/templatedescriptor.go` | 238 | Weights and matching (exact constants in §5.1) |
| `tpl/tplimpl/legacy.go` | 130 | Legacy term, taxonomy and section mappings |
| `tpl/tplimpl/templates.go` | 366 | `needsBaseTemplate`, `applyBaseTemplate` (parse order base then overlay), BOM removal, embedded aliases |
| `tpl/tplimpl/templatetransform.go` | 352 | Partial `return` wrapping (unused here) and the `templates.Defer` rewrite (unused). Port it minimally |
| `tpl/tplimpl/template_funcs.go` | 175 | `GetMapValue` case-insensitivity for `maps.Params`; context-arg injection |
| `common/paths/pathparser.go` | 787 | Identifier parsing for layout file names: output format vs kind vs layout vs lang, and `BaseReTyped` |
| `tpl/collections/{where.go 543, sort.go 197, collections.go 687, index.go 144, reflect_helpers.go 216}` | 1787 | `where`, `sort`, `first`, `dict`, `slice`, `seq`, `isset`, `index`, `newScratch` |
| `tpl/compare/compare.go` | 388 | `Eq` normalisation, `compareGetWithCollator` (string-as-float rule, collator, time as Unix), `Default` |
| `tpl/fmt/fmt.go` + Go `fmt` verbs | 117 | `Sprintf`/`Sprint` semantics: `%s`, `%q`, `%d`, `%v`, `%!s(<nil>)`, Sprint spacing |
| `tpl/inflect/inflect.go` + `github.com/gobuffalo/flect` (humanize.go 36, ordinalize.go 43, ident.go 122, acronyms.go 152, capitalize.go 24) | 75 + ~377 | `humanize` and `Ordinalize` |
| `tpl/urls/urls.go`, `helpers/url.go` (189), `helpers/path.go` (428) | 254 + 617 | `urlize`, `relLangURL`, `absLangURL`, `ref` arg mapping, `URLEscape`, `UnicodeSanitize` |
| `tpl/time/time.go` (150), `common/htime/time.go` (177) | 327 | `dateFormat` (RFC3339 round-trip, locale month and day substitution), `now` with clock |
| `tpl/strings/strings.go` (subset: `ToLower`, `Trim`, `HasPrefix`, `TrimPrefix`) | ~60 | |
| `tpl/encoding/encoding.go` (Jsonify) | 116 | Go `encoding/json` output: sorted keys, HTML-escaping, float formatting |
| `tpl/safe/safe.go`, `tpl/reflect/reflect.go`, `tpl/path/path.go` (Ext) | 66 + 36 + 162 | |
| `tpl/math/math.go` + `common/math/math.go` | 365 + 133 | `add`, `sub`, `mul` promotion to int64/float64 |
| `common/maps/params.go` (384), `common/maps/scratch.go` (160), `common/collections/append.go` (152), `slice.go` (95) | 791 | Params lower-casing and GetNested; Scratch Add/Append typing |
| `common/hreflect/helpers.go` | 295 | Truthiness including `IsZero` |
| `tpl/internal/go_templates/texttemplate/{exec.go 1135, hugo_template.go 451, funcs.go 785, parse/*}` and `htmltemplate/*` (~4.5k) | — | The template-engine agent owns these; this inventory is the conformance checklist |
| `hugolib/page__paginator.go` | ~117 | First-call-wins paginator |
| `hugolib/page.go:479-517`, `hugolib/alias.go:48-80`, `hugolib/site_render.go` (standalone rules, sitemapindex data = Sites) | — | Template query construction |

### 8.2 Rust crates

| Status | Crate | Reason |
|---|---|---|
| **Safe** (does not affect output bytes) | `once_cell`/`std::sync::OnceLock` | Paginator once |
| **Safe** | `indexmap` | |
| **Safe** | `md-5` | `File.UniqueID` = md5 hex |
| **Safe** | `percent-encoding` | Only as a helper inside a port of Go's `url.PathEscape`/`URL.String`; do not substitute its encode sets for Go's |
| **Safe** | `regex` | Nothing here needs it |
| **Safe** | `chrono` or `time` | Only as storage; formatting must be a port of Go's `time.Format` layout engine and `Time.String()` |
| **Safe** | `phf` | Static tables (acronyms, month names) |
| **Not safe** | `tera`, `handlebars`, `minijinja`, `gtmpl` | Not Go template semantics, and no html/template contextual escaping. Port Go's `text/template` and `html/template` instead |
| **Not safe** | `serde_json` default serializer (for `jsonify` or jsvalescaper) | It does not escape `<`, `>`, `&`, U+2028 and U+2029 the way Go does, and float formatting differs. A custom formatter or a port of the Go encoder is needed |
| **Not safe without verification** | `icu_collator` / ICU4X | Go uses `golang.org/x/text/collate` built from **CLDR 23 / Unicode 6.2**, while ICU4X uses current CLDR. Thai ordering (prevowels and tone marks) affects 104 distinct sorts. Either port x/text collate (`collate/*.go` ~755 lines, `internal/colltab/*.go` ~1500 lines, generated `tables.go` 4.9 MB, which could be code-generated into Rust arrays) or prove equality on all sort inputs of this site with a Go-generated corpus |
| **Not safe** | `url` crate | WHATWG URL parsing differs from Go `net/url` in error cases (for example `%!s(<nil>)` must *fail* to parse so `absLangURL` returns the input unchanged) and in `String()` escaping. Port Go's `net/url` (Parse, String, `shouldEscape`) |
| **Not safe** | `Inflector`, `heck`, `titlecase` | Not identical to gobuffalo/flect's `Humanize` and `Ordinalize` |
| **Not safe** | `rust-i18n`, `fluent` | go-i18n plural rules plus a text/template body (i18n agent) |
| **Not safe** | `chrono` strftime | Go layout-based formatting and go-playground/locales month and day names are needed |

### 8.3 Parity risks, most important first

- **R1: collation in `sort`.** Every `sort` call uses the site-language collator (x/text, CLDR 23).
  104 distinct Thai sort results differ from byte order; en had none. The evidence, with both orderings
  for each call, is in `work/templates-inventory/sortdiff.txt`, and it can serve as a conformance corpus
  for a Rust collator. Wrong collation reorders tags, ingredients,
  categories, companies and term-page product grids on th pages.
- **R2: `resources.GetRemote` plus file cache.** 87 snack pages embed live YouTube API data. The golden
  output was produced from `~/Library/Caches/hugo_cache/seeksnack/filecache/getresource/*`, which holds
  raw HTTP response dumps under hashstructure-derived uint64 keys. The Rust port must either compute the
  same cache key (`hashing.HashString(uri, map[string]any{})`) and read the same dump format, or the
  harness must supply the responses. Otherwise output changes (view counts) or the build fails when
  offline.
- **R3: paginator first-call semantics.** `head.html` calls `.Paginator` before `.Paginate`. Home and
  404 pagination use `RegularPages()` in default order. That drives 14+7 home pager pages, 13+6 404
  pager pages and their aliases.
- **R4: html/template contextual escaping.**
  - JSON-LD relies on exact `jsstrescaper` and `jsvalescaper` output (`\/`, `'`, `+`, JSON
    encoding of `[]string`).
  - Also `urlescaper`, `srcset`, `rcdata` in `<title>`, and `cssvaluefilter`.
  - Nil prints as `""` in html/template and as `<no value>` in text/template.
  - RSS and sitemap are html/template, not XML-aware. Appendix A.6 lists every action with its chain.
- **R5: Go fmt semantics.**
  - `%!s(<nil>)` appears in 3356 meta tags.
  - `time.Time` prints via `String()` with fractional seconds and `+0000 UTC`.
  - `%q` of a Stringer, Sprint spacing, float `0.3`, int64 printing.
- **R6: quirky functions.**
  - `index` on a string returns a byte, then `humanize` turns 115 into `115th`.
  - `ge` on numeric strings compares as numbers.
  - `humanize` depends on flect's acronym table.
  - `path.Ext` on a resource uses its `Name()`.
  - `urlize` Unicode sanitisation and lower-casing.
  - `dateFormat` RFC3339 truncation and Thai month abbreviations.
- **R7: case-insensitivity boundaries.** `maps.Params` is case-insensitive; data, dict, JSON and
  array-element maps are exact; `Resources.Get` uses EqualFold; partial names are lower-cased; method
  names are exact.
- **R8: truthiness with `IsZero`.** `*source.File` on term pages, `time.Time` and `nopPage`.
- **R9: time dependence.** The copyright year comes from `now`. Run the parity build in the same calendar
  year, or support `--clock`.
- **R10: `hugo_stats.json` → PurgeCSS → inlined `<style>`.** Template output feeds back into the CSS
  inlined in every page. `build.writeStats` must emit a byte-identical `hugo_stats.json`, and
  PostProcess must run after all pages are rendered.
- **R11: term URL collisions.** Nondeterministic in Go too; whitelist these paths.
- **R12: legacy-mapping quirks.** Spurious section-kind copies at keys `/index`, `/latesturl`, `/simple`
  and `/term` are harmless here because no such sections exist. Port them anyway so any site behaves the
  same.
- **R13: unused templates must parse.** This includes `posts/*`, `modalImage.html` (`with/else`),
  `url.html`, and the dev branches that call `absURL`, `urldecode`, `hugo.Generator` and dev
  `toCSS`/`ExecuteAsTemplate`.

---------------------------------------------------------------------------------------------------

## Appendix A: generated from the dynamic trace

Generated by `work/templates-inventory/gen_appendix.py` from `trace-mapped.tsv`.

- Line numbers are 1-based lines in the source file that owns the parse tree. For baseof-applied
  templates, lines in `define` blocks refer to the overlay file and lines in the root refer to
  `baseof.html`.
- Counts are across the whole build: both languages, all pager pages and all output formats.
- In the escaper table, the line number for some escaper-inserted nodes reflects the position of the
  owning action.
- `EMB:` means an embedded template; `ASSET:` means an `assets/ts` file run through `ExecuteAsTemplate`.

### A.1 Top-level template functions actually executed (dynamic trace)

| func | calls | arg types (×count) | result types | call sites (file:line) |
|---|---|---|---|---|
| `absLangURL` | 6067 | `string`×6067 | `string`×6067 | partials/head.html:157, partials/header.html:10, partials/header.html:33, partials/marketing/jsonLd.html:121 |
| `add` | 2144 | `int,int`×1692, `int64,int`×452 | `int64`×2144 | partials/carousel.html:7, partials/pagination.html:64, partials/related.html:52 |
| `and` | 29033 | `n=2`×25127, `n=1`×3906 | `()`×29033 | EMB:_markup/render-link.html:5, partials/ads/adsensehead.html:1, partials/ads/adsensemanual.html:1, partials/breadcrumb.html:18, partials/breadcrumb.html:23, partials/breadcrumb.html:28, partials/breadcrumb.html:33, partials/breadcrumb.html:56, partials/breadcrumb.html:63, partials/breadcrumb.html:70, partials/breadcrumb.html:77, partials/comments.html:139, partials/head.html:141, partials/head.html:190, partials/marketing/jsonLd.html:5, partials/marketing/jsonLd.html:143, partials/marketing/jsonLd.html:198, partials/pagination.html:64 |
| `dateFormat` | 503 | `string,time.Time`×462, `string,string`×41 | `string`×503 | partials/comments.html:224, partials/single/date.html:6, partials/single/date.html:7, partials/single/whenseen.html:12 |
| `default` | 2297 | `string,string`×1794, `string,nil(interface {})`×503 | `string`×2297 | partials/comments.html:224, partials/head.html:178, partials/head.html:183, partials/single/date.html:6, partials/single/date.html:7, partials/single/whenseen.html:12 |
| `dict` | 52018 | `string,string,string,string`×22282, `string,string`×9453, `string,*hugolib.pageState,string,*hugolib.pageState`×5550, `string,string,string,bool`×3906, `string,int64,string,*hugolib.pageState`×3645, `string,bool,string,[]string,string,string`×1953, `string,int,string,*hugolib.pageState`×1953, `string,int,string,string`×1148, `string,int`×909, `string,string,string,string,string,string,string,string,string,[]string,string,string,string,string,string,[]string`×430, `string,*hugolib.pageState`×254, `string,time.Time`×231, `string,maps.Params`×200, `string,float64`×50, `string,string,string,nil(interface {}),string,string,string,string,string,nil(interface {}),string,string,string,string,string,nil(interface {})`×10, `string,string,string,string,string,string,string,string,string,[]string,string,string,string,string,string,nil(interface {})`×10, `string,float64,string,string`×8, `string,string,string,string,string,string,string,string,string,nil(interface {}),string,string,string,string,string,[]string`×6, `string,string,string,string,string,string,string,string,string,nil(interface {}),string,string,string,string,string,nil(interface {})`×6, `string,string,string,string,string,string,string,string,string,string,string,string,string,string,string,[]string`×6, `string,string,string,nil(interface {}),string,string,string,string,string,nil(interface {}),string,string,string,string,string,[]string`×4, `string,string,string,string,string,string,string,string,string,[]interface {},string,string,string,string,string,[]string`×2, `string,string,string,string,string,string,string,string,string,[]string,string,string,string,string,string,[]interface {}`×2 | `map[string]interface {}`×52018 | _default/index.json:11, _default/list.html:10, _default/list.html:104, _default/list.html:107, _default/list.html:108, _default/list.html:111, _default/simple.html:10, _default/simple.html:51, _default/simple.html:54, _default/simple.html:55, _default/simple.html:58, _default/single.html:11, _default/single.html:42, _default/single.html:92, _default/single.html:105, _default/single.html:110, _default/single.html:136, _default/single.html:207, _default/single.html:210, _default/single.html:211, _default/single.html:214, _default/single.html:215, _default/single.html:219, 404.html:20, 404.html:66, 404.html:69, 404.html:70, 404.html:73, index.html:20, index.html:220, index.html:223, index.html:224, index.html:227, partials/breadcrumb.html:3, partials/breadcrumb.html:8, partials/comments.html:28, partials/comments.html:29, partials/comments.html:30, partials/comments.html:31, partials/comments.html:46, partials/comments.html:47, partials/comments.html:48, partials/comments.html:49, partials/comments.html:50, partials/comments.html:51, partials/comments.html:52, partials/comments.html:53, partials/comments.html:54, partials/comments.html:234, partials/comments.html:237, partials/comments.html:240, partials/comments.html:243, partials/head.html:275, partials/marketing/jsonLd.html:397, partials/rating/rating.html:23, partials/rating/rating.html:26, partials/rating/rating.html:29, partials/rating/rating.html:32, partials/rating/rating.html:35, partials/rating/rating.html:38, partials/rating/rating.html:41, partials/rating/rating.html:44, partials/rating/rating.html:47, partials/rating/rating.html:50, partials/rating/rating.html:53, partials/rating/rating.html:56, partials/rating/rating.html:59, partials/single/nutritionfacts.html:21, partials/single/nutritionfacts.html:26, partials/single/nutritionfacts.html:38, partials/single/nutritionfacts.html:63, partials/single/nutritionfacts.html:77, partials/single/nutritionfacts.html:153, partials/single/nutritionfacts.html:170, partials/single/nutritionfacts.html:187, partials/single/nutritionfacts.html:204, partials/single/nutritionfacts.html:219, partials/single/nutritionfacts.html:236, partials/single/nutritionfacts.html:245, partials/single/nutritionfacts.html:266, partials/single/nutritionfacts.html:291, partials/single/nutritionfacts.html:307, partials/single/nutritionfacts.html:323, partials/single/nutritionfacts.html:515, partials/single/nutritionfacts.html:611, taxonomy/list.html:10, taxonomy/list.html:116, taxonomy/list.html:119, taxonomy/list.html:120, taxonomy/list.html:123, term/term.html:14, term/term.html:209, term/term.html:212, term/term.html:213, term/term.html:216, term/term.html:217, term/term.html:220 |
| `eq` | 220008 | `string,string`×209411, `*hugolib.pageState,*hugolib.pageState`×5550, `int64,int`×2450, `nil(interface {}),string`×1314, `*page.Pager,*page.Pager`×1178, `int,int`×105 | `bool`×220008 | _default/_markup/render-image.html:8, _default/_markup/render-image.html:12, _default/list.html:5, _default/list.html:33, _default/list.html:45, _default/list.html:80, _default/rss.xml:19, _default/simple.html:5, _default/simple.html:27, _default/single.html:6, _default/single.html:49, _default/single.html:61, _default/single.html:174, 404.html:15, 404.html:42, index.html:15, index.html:38, index.html:46, index.html:58, index.html:109, index.html:111, index.html:113, index.html:115, index.html:117, index.html:119, index.html:136, index.html:196, partials/breadcrumb.html:12, partials/breadcrumb.html:18, partials/breadcrumb.html:23, partials/breadcrumb.html:28, partials/breadcrumb.html:33, partials/breadcrumb.html:56, partials/breadcrumb.html:63, partials/breadcrumb.html:70, partials/breadcrumb.html:77, partials/carousel.html:12, partials/carousel.html:19, partials/carousel.html:35, partials/comments.html:185, partials/comments.html:290, partials/footer.html:10, partials/footer.html:15, partials/footer.html:20, partials/head.html:174, partials/head.html:175, partials/head.html:269, partials/header.html:12, partials/header.html:16, partials/header.html:21, partials/header.html:72, partials/header.html:112, partials/header.html:114, partials/marketing/jsonLd.html:5, partials/marketing/jsonLd.html:22, partials/marketing/jsonLd.html:41, partials/marketing/jsonLd.html:75, partials/marketing/jsonLd.html:94, partials/marketing/jsonLd.html:175, partials/marketing/jsonLd.html:222, partials/marketing/jsonLd.html:281, partials/marketing/jsonLd.html:300, partials/marketing/jsonLd.html:333, partials/marketing/jsonLd.html:352, partials/pagination.html:63, partials/pagination.html:73, partials/related.html:11, partials/related.html:23, partials/single/nutritionfacts.html:14, partials/single/nutritionfacts.html:36, sitemap.xml:11, sitemap.xml:13, sitemap.xml:15, sitemap.xml:17, taxonomy/list.html:5, taxonomy/list.html:26, taxonomy/list.html:50, taxonomy/list.html:55, taxonomy/list.html:77, taxonomy/list.html:92, term/term.html:9, term/term.html:31, term/term.html:40, term/term.html:45, term/term.html:138, term/term.html:165, term/term.html:180 |
| `fingerprint` | 5859 | `*resources.resourceAdapter`×5859 | `*resources.resourceAdapter`×5859 | _default/list.html:11, _default/list.html:112, _default/simple.html:11, _default/simple.html:59, _default/single.html:12, _default/single.html:223, 404.html:21, 404.html:74, index.html:21, index.html:228, partials/head.html:276, taxonomy/list.html:11, taxonomy/list.html:124, term/term.html:15, term/term.html:222 |
| `first` | 4618 | `int,page.Pages`×4576, `int64,page.Pages`×42 | `page.Pages`×4618 | _default/rss.xml:11, index.html:37, partials/carousel.html:11, partials/head.html:133, partials/head.html:194, partials/related.html:1 |
| `ge` | 10715 | `int,int`×7231, `string,string`×3444, `float64,int`×40 | `bool`×10715 | _default/rss.xml:10, _default/rss.xml:27, partials/comments.html:399, partials/footer.html:76, partials/rating/rating.html:72 |
| `gt` | 60886 | `int,int`×58590, `int,int64`×2296 | `bool`×60886 | partials/marketing/jsonLd.html:23, partials/marketing/jsonLd.html:42, partials/marketing/jsonLd.html:76, partials/marketing/jsonLd.html:95, partials/marketing/jsonLd.html:176, partials/marketing/jsonLd.html:223, partials/marketing/jsonLd.html:282, partials/marketing/jsonLd.html:301, partials/marketing/jsonLd.html:334, partials/marketing/jsonLd.html:353, partials/pagination.html:20, partials/pagination.html:64 |
| `html` | 5277 | `string`×5277 | `string`×5277 | _default/rss.xml:1, partials/single/socialshare.html:1 |
| `hugo` | 7263 | `()`×7263 | `neohugo.HugoInfo`×7263 | _default/list.html:5, _default/list.html:80, _default/simple.html:5, _default/simple.html:27, _default/single.html:6, _default/single.html:174, 404.html:15, 404.html:42, index.html:15, index.html:196, partials/head.html:269, taxonomy/list.html:5, taxonomy/list.html:77, taxonomy/list.html:92, term/term.html:9, term/term.html:165, term/term.html:180 |
| `humanize` | 6558 | `string`×6545, `uint8`×13 | `string`×6558 | _default/rss.xml:48, index.html:137, partials/taxonomy/categories.html:17, partials/taxonomy/categories.html:25, partials/taxonomy/companies.html:16, partials/taxonomy/countries.html:16, term/term.html:113 |
| `i18n` | 16612 | `string`×15526, `string,map[string]interface {}`×1086 | `string`×16612 | _default/single.html:116, index.html:94, index.html:110, index.html:112, index.html:114, index.html:116, index.html:118, index.html:120, partials/comments.html:4, partials/comments.html:110, partials/comments.html:124, partials/comments.html:132, partials/comments.html:156, partials/comments.html:158, partials/comments.html:167, partials/comments.html:170, partials/comments.html:286, partials/rating/rating.html:2, partials/single/nutritionfacts.html:4, partials/single/nutritionfacts.html:15, partials/single/nutritionfacts.html:17, partials/single/nutritionfacts.html:21, partials/single/nutritionfacts.html:26, partials/single/nutritionfacts.html:33, partials/single/nutritionfacts.html:38, partials/single/nutritionfacts.html:42, partials/single/nutritionfacts.html:54, partials/single/nutritionfacts.html:62, partials/single/nutritionfacts.html:63, partials/single/nutritionfacts.html:76, partials/single/nutritionfacts.html:77, partials/single/nutritionfacts.html:152, partials/single/nutritionfacts.html:153, partials/single/nutritionfacts.html:169, partials/single/nutritionfacts.html:170, partials/single/nutritionfacts.html:186, partials/single/nutritionfacts.html:187, partials/single/nutritionfacts.html:203, partials/single/nutritionfacts.html:204, partials/single/nutritionfacts.html:218, partials/single/nutritionfacts.html:219, partials/single/nutritionfacts.html:235, partials/single/nutritionfacts.html:236, partials/single/nutritionfacts.html:245, partials/single/nutritionfacts.html:265, partials/single/nutritionfacts.html:266, partials/single/nutritionfacts.html:290, partials/single/nutritionfacts.html:291, partials/single/nutritionfacts.html:306, partials/single/nutritionfacts.html:307, partials/single/nutritionfacts.html:322, partials/single/nutritionfacts.html:323, partials/single/nutritionfacts.html:514, partials/single/nutritionfacts.html:515, partials/single/nutritionfacts.html:610, partials/single/nutritionfacts.html:611, partials/single/smell.html:2, partials/single/taste.html:2 |
| `images` | 16755 | `()`×16755 | `*images.Namespace`×16755 | _default/_markup/render-image.html:6, _default/_markup/render-image.html:7, _default/index.json:6, _default/index.json:9, _default/list.html:37, _default/list.html:38, _default/single.html:54, _default/single.html:55, index.html:51, index.html:52, partials/carousel.html:27, partials/carousel.html:28, partials/related.html:15, partials/related.html:16, taxonomy/list.html:42, taxonomy/list.html:43, term/term.html:35, term/term.html:130, term/term.html:131 |
| `index` | 27098 | `maps.Params,string`×21337, `[]string,int`×3935, `map[string]interface {},string`×1714, `[]interface {},int`×96, `string,int`×16 | `[]interface {}`×12034, `string`×11294, `maps.Params`×1953, `nil(interface {})`×1714, `map[string]interface {}`×87, `uint8`×16 | _default/rss.xml:14, _default/rss.xml:48, partials/comments.html:180, partials/footer.html:2, partials/footer.html:3, partials/footer.html:4, partials/header.html:1, partials/header.html:70, partials/marketing/jsonLd.html:3, partials/marketing/jsonLd.html:20, partials/marketing/jsonLd.html:39, partials/marketing/jsonLd.html:73, partials/marketing/jsonLd.html:92, partials/marketing/jsonLd.html:141, partials/marketing/jsonLd.html:173, partials/marketing/jsonLd.html:206, partials/marketing/jsonLd.html:220, partials/marketing/jsonLd.html:279, partials/marketing/jsonLd.html:298, partials/marketing/jsonLd.html:331, partials/marketing/jsonLd.html:350, partials/taxonomy/categories.html:11, partials/taxonomy/tags.html:3 |
| `isset` | 225 | `[]string,int`×224, `[]interface {},int`×1 | `bool`×225 | partials/taxonomy/tags.html:1 |
| `js` | 9526 | `()`×9526 | `*js.Namespace`×9526 | _default/list.html:11, _default/list.html:104, _default/list.html:108, _default/list.html:112, _default/simple.html:11, _default/simple.html:51, _default/simple.html:55, _default/simple.html:59, _default/single.html:12, _default/single.html:207, _default/single.html:211, _default/single.html:215, _default/single.html:223, 404.html:21, 404.html:66, 404.html:70, 404.html:74, index.html:21, index.html:220, index.html:224, index.html:228, taxonomy/list.html:11, taxonomy/list.html:116, taxonomy/list.html:120, taxonomy/list.html:124, term/term.html:15, term/term.html:209, term/term.html:213, term/term.html:217, term/term.html:222 |
| `jsonify` | 2 | `[]map[string]interface {}`×2 | `template.HTML`×2 | _default/index.json:21 |
| `le` | 3168 | `int,int`×3168 | `bool`×3168 | partials/pagination.html:63 |
| `len` | 3626 | `page.Pages`×3626 | `int`×3626 | _default/list.html:23, partials/marketing/jsonLd.html:395, taxonomy/list.html:25, term/term.html:116 |
| `lower` | 3362 | `string`×3362 | `string`×3362 | partials/taxonomy/categories.html:15, partials/taxonomy/categories.html:23, partials/taxonomy/ingredients.html:12, partials/taxonomy/tags.html:13 |
| `lt` | 1403 | `int,int64`×1403 | `bool`×1403 | partials/pagination.html:64 |
| `markdownify` | 2 | `string`×2 | `template.HTML`×2 | partials/comments.html:278 |
| `minify` | 1953 | `*resources.resourceAdapter`×1953 | `*resources.resourceAdapter`×1953 | partials/head.html:276 |
| `mul` | 92 | `int,int`×92 | `int64`×92 | _default/list.html:18, index.html:28 |
| `ne` | 9440 | `int64,int`×4281, `int,int`×3668, `string,string`×1491 | `bool`×9440 | _default/rss.xml:23, partials/head.html:141, partials/marketing/jsonLd.html:404, partials/related.html:53 |
| `newScratch` | 1953 | `()`×1953 | `*maps.Scratch`×1953 | partials/marketing/jsonLd.html:375 |
| `not` | 18942 | `bool`×18940, `<invalid>`×2 | `bool`×18942 | EMB:_markup/render-link.html:5, EMB:sitemapindex.xml:6, _default/rss.xml:26, _default/rss.xml:39, partials/breadcrumb.html:9, partials/comments.html:195, partials/footer.html:74, partials/head.html:239, partials/head.html:244, partials/head.html:250, partials/head.html:257, partials/marketing/jsonLd.html:406, partials/pagination.html:69, sitemap.xml:7 |
| `now` | 6888 | `()`×6888 | `time.Time`×6888 | _default/rss.xml:27, _default/rss.xml:30, partials/footer.html:76, partials/footer.html:80 |
| `or` | 40180 | `n=2`×24551, `n=5`×15624, `n=3`×3, `n=4`×2 | `()`×40180 | EMB:_markup/render-link.html:7, _default/_markup/render-image.html:8, _default/list.html:33, _default/rss.xml:4, _default/single.html:49, _default/single.html:138, index.html:46, partials/breadcrumb.html:18, partials/breadcrumb.html:23, partials/breadcrumb.html:28, partials/breadcrumb.html:33, partials/breadcrumb.html:56, partials/breadcrumb.html:63, partials/breadcrumb.html:70, partials/breadcrumb.html:77, partials/carousel.html:19, partials/comments.html:229, partials/footer.html:10, partials/head.html:2, partials/head.html:141, partials/header.html:12, partials/header.html:72, partials/pagination.html:63, partials/pagination.html:64, partials/related.html:11, partials/single/nutritionfacts.html:58, partials/single/nutritionfacts.html:72, partials/single/nutritionfacts.html:89, partials/single/nutritionfacts.html:113, partials/single/nutritionfacts.html:130, partials/single/nutritionfacts.html:148, partials/single/nutritionfacts.html:165, partials/single/nutritionfacts.html:182, partials/single/nutritionfacts.html:199, partials/single/nutritionfacts.html:214, partials/single/nutritionfacts.html:241, partials/single/nutritionfacts.html:261, partials/single/nutritionfacts.html:282, partials/single/nutritionfacts.html:286, partials/single/nutritionfacts.html:302, partials/single/nutritionfacts.html:318, partials/single/nutritionfacts.html:334, partials/single/nutritionfacts.html:350, partials/single/nutritionfacts.html:366, partials/single/nutritionfacts.html:382, partials/single/nutritionfacts.html:398, partials/single/nutritionfacts.html:414, partials/single/nutritionfacts.html:430, partials/single/nutritionfacts.html:444, partials/single/nutritionfacts.html:460, partials/single/nutritionfacts.html:476, partials/single/nutritionfacts.html:494, partials/single/nutritionfacts.html:510, partials/single/nutritionfacts.html:526, partials/single/nutritionfacts.html:542, partials/single/nutritionfacts.html:558, partials/single/nutritionfacts.html:574, partials/single/nutritionfacts.html:590, partials/single/nutritionfacts.html:606, partials/single/nutritionfacts.html:622, partials/single/nutritionfacts.html:638, partials/single/nutritionfacts.html:654, partials/single/nutritionfacts.html:670, partials/single/nutritionfacts.html:686, partials/single/nutritionfacts.html:702, term/term.html:31, term/term.html:107 |
| `partial` | 52403 | `string,*hugolib.pageState`×26768, `string,map[string]interface {}`×24162, `string,[]string`×797, `string,string`×674, `string,[]interface {}`×2 | `template.HTML`×52403 | _default/baseof.html:8, _default/baseof.html:24, _default/baseof.html:26, _default/list.html:2, _default/list.html:21, _default/list.html:22, _default/list.html:74, _default/list.html:75, _default/simple.html:2, _default/single.html:3, _default/single.html:19, _default/single.html:20, _default/single.html:35, _default/single.html:42, _default/single.html:83, _default/single.html:86, _default/single.html:89, _default/single.html:92, _default/single.html:99, _default/single.html:102, _default/single.html:105, _default/single.html:110, _default/single.html:136, _default/single.html:139, _default/single.html:142, _default/single.html:147, _default/single.html:150, _default/single.html:155, _default/single.html:160, _default/single.html:164, 404.html:2, index.html:2, index.html:31, index.html:32, index.html:35, index.html:88, partials/comments.html:28, partials/comments.html:29, partials/comments.html:30, partials/comments.html:31, partials/comments.html:46, partials/comments.html:47, partials/comments.html:48, partials/comments.html:49, partials/comments.html:50, partials/comments.html:51, partials/comments.html:52, partials/comments.html:53, partials/comments.html:54, partials/comments.html:234, partials/comments.html:237, partials/comments.html:240, partials/comments.html:243, partials/head.html:11, partials/head.html:12, partials/head.html:236, partials/marketing/jsonLd.html:15, partials/marketing/jsonLd.html:273, partials/rating/rating.html:23, partials/rating/rating.html:26, partials/rating/rating.html:29, partials/rating/rating.html:32, partials/rating/rating.html:35, partials/rating/rating.html:38, partials/rating/rating.html:41, partials/rating/rating.html:44, partials/rating/rating.html:47, partials/rating/rating.html:50, partials/rating/rating.html:53, partials/rating/rating.html:56, partials/rating/rating.html:59, taxonomy/list.html:2, taxonomy/list.html:20, taxonomy/list.html:21, taxonomy/list.html:86, taxonomy/list.html:87, term/term.html:2, term/term.html:22, term/term.html:23, term/term.html:65, term/term.html:68, term/term.html:71, term/term.html:74, term/term.html:77, term/term.html:79, term/term.html:108, term/term.html:110, term/term.html:174, term/term.html:175 |
| `path` | 24551 | `()`×24551 | `*path.Namespace`×24551 | _default/_markup/render-image.html:8, _default/_markup/render-image.html:12, _default/list.html:33, _default/list.html:45, _default/single.html:49, _default/single.html:61, index.html:46, index.html:58, partials/carousel.html:19, partials/carousel.html:35, partials/footer.html:10, partials/footer.html:15, partials/footer.html:20, partials/header.html:12, partials/header.html:16, partials/header.html:21, partials/related.html:11, partials/related.html:23, taxonomy/list.html:50, taxonomy/list.html:55, term/term.html:31, term/term.html:40, term/term.html:45, term/term.html:138 |
| `postCSS` | 1953 | `*resources.resourceAdapter`×1953 | `*resources.resourceAdapter`×1953 | partials/head.html:276 |
| `print` | 222 | `string,string,int,string`×222 | `string`×222 | partials/head.html:142 |
| `printf` | 40284 | `string,string,string`×26672, `string,string`×6083, `string,string,string,string,string`×1802, `string,nil(interface {})`×1691, `string`×1494, `string,string,media.Type`×1491, `string,int,int`×1050, `string,string,nil(interface {})`×1 | `string`×40284 | EMB:sitemapindex.xml:1, _default/_markup/render-image.html:2, _default/_markup/render-image.html:3, _default/baseof.html:13, _default/baseof.html:15, _default/rss.xml:15, _default/rss.xml:41, _default/rss.xml:50, index.html:99, index.html:130, partials/comments.html:184, partials/head.html:19, partials/head.html:122, partials/head.html:157, partials/marketing/jsonLd.html:121, partials/marketing/jsonLd.html:201, partials/single/author.html:3, partials/taxonomy/brands.html:31, partials/taxonomy/categories.html:15, partials/taxonomy/categories.html:17, partials/taxonomy/categories.html:23, partials/taxonomy/companies.html:14, partials/taxonomy/companies.html:16, partials/taxonomy/companies.html:22, partials/taxonomy/company/facebook.html:32, partials/taxonomy/company/instagram.html:32, partials/taxonomy/company/twitter.html:31, partials/taxonomy/company/website.html:31, partials/taxonomy/company/youtube.html:31, partials/taxonomy/countries.html:14, partials/taxonomy/countries.html:16, partials/taxonomy/ingredients.html:12, partials/taxonomy/tags.html:13, sitemap.xml:1 |
| `ref` | 6 | `*hugolib.ShortcodeWithPage,[]interface {}`×6 | `string`×6 | EMB:_shortcodes/ref.html:1 |
| `reflect` | 1550 | `()`×1550 | `*reflect.Namespace`×1550 | partials/taxonomy/brands.html:8, partials/taxonomy/categories.html:5, partials/taxonomy/categories.html:10, partials/taxonomy/companies.html:8, partials/taxonomy/company/facebook.html:9, partials/taxonomy/company/instagram.html:9, partials/taxonomy/company/twitter.html:8, partials/taxonomy/company/website.html:8, partials/taxonomy/company/youtube.html:8, partials/taxonomy/countries.html:8, partials/taxonomy/ingredients.html:6, partials/taxonomy/tags.html:8 |
| `relLangURL` | 4768 | `string`×4768 | `string`×4768 | partials/single/author.html:3, partials/taxonomy/brands.html:3, partials/taxonomy/brands.html:31, partials/taxonomy/categories.html:15, partials/taxonomy/categories.html:23, partials/taxonomy/companies.html:3, partials/taxonomy/companies.html:14, partials/taxonomy/companies.html:22, partials/taxonomy/company/facebook.html:32, partials/taxonomy/company/instagram.html:32, partials/taxonomy/company/twitter.html:31, partials/taxonomy/company/website.html:31, partials/taxonomy/company/youtube.html:31, partials/taxonomy/countries.html:3, partials/taxonomy/countries.html:14, partials/taxonomy/ingredients.html:12, partials/taxonomy/tags.html:13 |
| `resources` | 75447 | `()`×75447 | `*resources.Namespace`×75447 | EMB:_markup/render-link.html:10, _default/_markup/render-image.html:4, _default/index.json:4, _default/list.html:11, _default/list.html:35, _default/list.html:100, _default/list.html:101, _default/list.html:103, _default/list.html:106, _default/list.html:107, _default/list.html:112, _default/rss.xml:35, _default/simple.html:11, _default/simple.html:47, _default/simple.html:48, _default/simple.html:50, _default/simple.html:53, _default/simple.html:54, _default/simple.html:59, _default/single.html:12, _default/single.html:52, _default/single.html:204, _default/single.html:206, _default/single.html:209, _default/single.html:210, _default/single.html:213, _default/single.html:214, _default/single.html:221, _default/single.html:223, 404.html:21, 404.html:62, 404.html:63, 404.html:65, 404.html:68, 404.html:69, 404.html:74, index.html:21, index.html:49, index.html:216, index.html:217, index.html:219, index.html:222, index.html:223, index.html:228, partials/carousel.html:25, partials/footer.html:9, partials/head.html:27, partials/head.html:32, partials/head.html:37, partials/head.html:42, partials/head.html:47, partials/head.html:52, partials/head.html:57, partials/head.html:62, partials/head.html:67, partials/head.html:73, partials/head.html:79, partials/head.html:85, partials/head.html:91, partials/head.html:98, partials/head.html:102, partials/head.html:106, partials/head.html:110, partials/head.html:114, partials/head.html:118, partials/head.html:153, partials/head.html:276, partials/header.html:11, partials/marketing/jsonLd.html:4, partials/marketing/jsonLd.html:202, partials/related.html:13, taxonomy/list.html:11, taxonomy/list.html:40, taxonomy/list.html:112, taxonomy/list.html:113, taxonomy/list.html:115, taxonomy/list.html:118, taxonomy/list.html:119, taxonomy/list.html:124, term/term.html:15, term/term.html:33, term/term.html:128, term/term.html:206, term/term.html:208, term/term.html:211, term/term.html:212, term/term.html:215, term/term.html:216, term/term.html:221, term/term.html:222 |
| `safeCSS` | 1953 | `string`×1953 | `template.CSS`×1953 | partials/head.html:278 |
| `safeHTML` | 23285 | `string`×22707, `hstring.HTML`×578 | `template.HTML`×23285 | EMB:sitemapindex.xml:1, EMB:sitemapindex.xml:7, _default/_markup/render-heading.html:1, _default/rss.xml:15, _default/rss.xml:40, _default/rss.xml:41, _default/rss.xml:47, _default/rss.xml:50, partials/head.html:122, partials/head.html:242, partials/head.html:253, partials/head.html:260, partials/header.html:166, sitemap.xml:1, sitemap.xml:8 |
| `safeJS` | 1953 | `template.HTML`×1953 | `template.JS`×1953 | partials/marketing/jsonLd.html:15, partials/marketing/jsonLd.html:273 |
| `safeURL` | 1156 | `string`×1156 | `template.URL`×1156 | _default/_markup/render-heading.html:1 |
| `seq` | 1177 | `int`×1156, `int64`×21 | `[]int`×1177 | partials/carousel.html:6, partials/comments.html:398, partials/rating/rating.html:71 |
| `site` | 7236 | `()`×7236 | `*page.siteWrapper`×7236 | EMB:alias.html:2, partials/head.html:152, partials/head.html:153, partials/head.html:160, partials/head.html:169, partials/head.html:221, partials/header.html:107 |
| `slice` | 11258 | `()`×5399, `string,string`×1953, `int,int,int,int,int,int`×1953, `*resources.resourceAdapter,*resources.resourceAdapter,*resources.resourceAdapter,*resources.resourceAdapter,*resources.resourceAdapter`×1714, `*resources.resourceAdapter,*resources.resourceAdapter,*resources.resourceAdapter,*resources.resourceAdapter`×239 | `[]interface {}`×5399, `resource.Resources`×1953, `[]string`×1953, `[]int`×1953 | _default/index.json:1, _default/list.html:112, _default/rss.xml:3, _default/simple.html:59, _default/single.html:223, 404.html:74, index.html:228, partials/head.html:275, partials/marketing/jsonLd.html:379, partials/marketing/jsonLd.html:380, partials/marketing/jsonLd.html:385, taxonomy/list.html:124, term/term.html:222 |
| `sort` | 4895 | `[]map[string]interface {},string,string`×1953, `page.Pages,string`×1718, `[]string`×1220, `[]interface {}`×4 | `[]map[string]interface {}`×1953, `page.Pages`×1718, `[]string`×1220, `[]interface {}`×4 | _default/index.json:2, _default/latesturl.html:1, _default/list.html:25, index.html:26, partials/carousel.html:1, partials/head.html:212, partials/head.html:266, partials/marketing/jsonLd.html:403, partials/taxonomy/categories.html:7, partials/taxonomy/companies.html:10, partials/taxonomy/countries.html:10, partials/taxonomy/ingredients.html:8, partials/taxonomy/tags.html:9, taxonomy/list.html:30, term/term.html:96, term/term.html:118 |
| `strings` | 561 | `()`×561 | `*strings.Namespace`×561 | EMB:_markup/render-link.html:3, EMB:_markup/render-link.html:6 |
| `sub` | 11062 | `int,int`×7417, `int64,int`×3645 | `int64`×11062 | partials/marketing/jsonLd.html:398, partials/pagination.html:62, partials/pagination.html:64 |
| `toCSS` | 1953 | `map[string]interface {},*resources.resourceAdapter`×1953 | `*resources.resourceAdapter`×1953 | partials/head.html:276 |
| `transform` | 87 | `()`×87 | `*transform.Namespace`×87 | partials/marketing/jsonLd.html:204 |
| `trim` | 3428 | `string,string`×3428 | `string`×3428 | partials/comments.html:91, partials/comments.html:179 |
| `urlize` | 4158 | `string`×4157, `nil(interface {})`×1 | `string`×4158 | partials/single/author.html:3, partials/taxonomy/brands.html:31, partials/taxonomy/categories.html:15, partials/taxonomy/categories.html:23, partials/taxonomy/companies.html:14, partials/taxonomy/companies.html:22, partials/taxonomy/countries.html:14, partials/taxonomy/ingredients.html:12, partials/taxonomy/tags.html:13 |
| `urls` | 558 | `()`×558 | `*urls.Namespace`×558 | EMB:_markup/render-link.html:1 |
| `where` | 1534 | `page.Pages,string,string`×1534 | `page.Pages`×1534 | _default/latesturl.html:1, _default/rss.xml:5, _default/rss.xml:7, index.html:26, partials/carousel.html:1 |

### A.2 Methods actually invoked (receiver type → method)

| receiver type | method | calls | arg types | result types | call sites |
|---|---|---|---|---|---|
| `*hugolib.Site` | `Lastmod` | 4 | `()`×4 | `time.Time`×4 | EMB:sitemapindex.xml:6, EMB:sitemapindex.xml:7 |
| `*hugolib.Site` | `SitemapAbsURL` | 2 | `()`×2 | `string`×2 | EMB:sitemapindex.xml:5 |
| `*hugolib.pageForRenderHooks` | `GetPage` | 3 | `string`×3 | `*page.nopPage`×3 | EMB:_markup/render-link.html:8 |
| `*hugolib.pageForRenderHooks` | `Resources` | 1053 | `()`×1053 | `resource.Resources`×1053 | EMB:_markup/render-link.html:9, _default/_markup/render-image.html:1 |
| `*hugolib.pageState` | `AllTranslations` | 3588 | `()`×3588 | `page.Pages`×3588 | partials/head.html:126, partials/head.html:133, partials/head.html:171, partials/header.html:109 |
| `*hugolib.pageState` | `AlternativeOutputFormats` | 1953 | `()`×1953 | `page.OutputFormats`×1953 | partials/head.html:121 |
| `*hugolib.pageState` | `Content` | 1717 | `()`×1717 | `template.HTML`×1717 | _default/simple.html:21, _default/single.html:96, term/term.html:82 |
| `*hugolib.pageState` | `Data` | 14148 | `()`×14148 | `page.Data`×14148 | _default/list.html:16, partials/breadcrumb.html:18, partials/breadcrumb.html:23, partials/breadcrumb.html:28, partials/breadcrumb.html:33, partials/breadcrumb.html:56, partials/breadcrumb.html:63, partials/breadcrumb.html:70, partials/breadcrumb.html:77, sitemap.xml:4, term/term.html:113 |
| `*hugolib.pageState` | `Date` | 6607 | `()`×6607 | `time.Time`×6607 | _default/rss.xml:39, _default/rss.xml:40, _default/rss.xml:47, partials/head.html:244, partials/head.html:257, partials/head.html:260 |
| `*hugolib.pageState` | `Description` | 6910 | `()`×6910 | `string`×6910 | _default/list.html:68, _default/rss.xml:50, _default/single.html:30, index.html:81, partials/head.html:20, partials/marketing/jsonLd.html:131, partials/related.html:48, partials/single/socialshare.html:47, taxonomy/list.html:72, term/term.html:26 |
| `*hugolib.pageState` | `File` | 1714 | `()`×1714 | `*source.File`×1714 | partials/comments.html:97 |
| `*hugolib.pageState` | `IsHome` | 27758 | `()`×27758 | `bool`×27758 | _default/baseof.html:12, _default/rss.xml:2, _default/rss.xml:4, partials/breadcrumb.html:9, partials/breadcrumb.html:38, partials/breadcrumb.html:49, partials/head.html:2, partials/head.html:15, partials/head.html:141, partials/head.html:151, partials/marketing/jsonLd.html:263, partials/marketing/jsonLd.html:271, partials/marketing/jsonLd.html:323, partials/marketing/jsonLd.html:406 |
| `*hugolib.pageState` | `IsNode` | 3626 | `()`×3626 | `bool`×3626 | partials/head.html:2, partials/head.html:141 |
| `*hugolib.pageState` | `IsPage` | 7788 | `()`×7788 | `bool`×7788 | partials/breadcrumb.html:13, partials/head.html:201, partials/head.html:238, partials/marketing/jsonLd.html:5 |
| `*hugolib.pageState` | `IsSection` | 1489 | `()`×1489 | `bool`×1489 | _default/rss.xml:4 |
| `*hugolib.pageState` | `IsTranslated` | 7589 | `()`×7589 | `bool`×7589 | partials/head.html:125, partials/head.html:168, partials/header.html:95, sitemap.xml:22 |
| `*hugolib.pageState` | `Kind` | 10053 | `()`×10053 | `string`×10053 | partials/breadcrumb.html:18, partials/breadcrumb.html:23, partials/breadcrumb.html:28, partials/breadcrumb.html:33, partials/breadcrumb.html:56, partials/breadcrumb.html:63, partials/breadcrumb.html:70, partials/breadcrumb.html:77, sitemap.xml:11, sitemap.xml:13, sitemap.xml:15, sitemap.xml:17, taxonomy/list.html:17, term/term.html:115 |
| `*hugolib.pageState` | `Lang` | 10402 | `()`×10402 | `string`×10402 | partials/head.html:170, partials/head.html:174, partials/header.html:108, partials/header.html:112, sitemap.xml:25, sitemap.xml:30 |
| `*hugolib.pageState` | `Language` | 5382 | `()`×5382 | `*langs.Language`×5382 | partials/head.html:129, partials/head.html:178, partials/head.html:183, partials/header.html:116, partials/header.html:124 |
| `*hugolib.pageState` | `Lastmod` | 3919 | `()`×3919 | `time.Time`×3919 | partials/head.html:250, partials/head.html:253, sitemap.xml:7, sitemap.xml:8 |
| `*hugolib.pageState` | `OutputFormats` | 1491 | `()`×1491 | `page.OutputFormats`×1491 | _default/rss.xml:40 |
| `*hugolib.pageState` | `Page` | 7048 | `()`×7048 | `*hugolib.pageState`×7048 | _default/index.json:7, partials/head.html:170, partials/head.html:171, partials/header.html:108, partials/header.html:109 |
| `*hugolib.pageState` | `Pages` | 1423 | `()`×1423 | `page.Pages`×1423 | _default/rss.xml:7 |
| `*hugolib.pageState` | `Paginate` | 92 | `page.Pages`×92 | `*page.Pager`×92 | _default/list.html:16, index.html:26 |
| `*hugolib.pageState` | `Paginator` | 6755 | `()`×6755 | `*page.Pager`×6755 | _default/list.html:23, _default/list.html:25, partials/head.html:3, partials/pagination.html:1, taxonomy/list.html:25, taxonomy/list.html:30, term/term.html:116, term/term.html:118 |
| `*hugolib.pageState` | `Params` | 36632 | `()`×36632 | `maps.Params`×36632 | _default/index.json:7, _default/index.json:12, _default/index.json:13, _default/index.json:15, _default/index.json:18, _default/list.html:32, _default/rss.xml:47, _default/rss.xml:50, _default/single.html:33, _default/single.html:38, _default/single.html:39, _default/single.html:42, _default/single.html:46, _default/single.html:47, _default/single.html:82, _default/single.html:85, _default/single.html:88, _default/single.html:91, _default/single.html:98, _default/single.html:101, _default/single.html:104, _default/single.html:107, _default/single.html:114, _default/single.html:135, _default/single.html:148, _default/single.html:153, _default/single.html:158, index.html:38, index.html:45, partials/carousel.html:18, partials/carousel.html:20, partials/carousel.html:21, partials/head.html:157, partials/head.html:190, partials/head.html:202, partials/head.html:211, partials/head.html:232, partials/head.html:265, partials/marketing/jsonLd.html:121, partials/marketing/jsonLd.html:134, partials/marketing/jsonLd.html:140, partials/marketing/jsonLd.html:143, partials/marketing/jsonLd.html:156, partials/marketing/jsonLd.html:157, partials/marketing/jsonLd.html:195, partials/marketing/jsonLd.html:196, partials/marketing/jsonLd.html:198, partials/marketing/jsonLd.html:201, partials/marketing/jsonLd.html:242, partials/marketing/jsonLd.html:243, partials/related.html:10, partials/related.html:43, partials/single/ingredientslist.html:19, partials/single/nutritionfacts.html:2, taxonomy/list.html:31, taxonomy/list.html:32, taxonomy/list.html:64, term/term.html:28, term/term.html:30, term/term.html:64, term/term.html:67, term/term.html:70, term/term.html:73, term/term.html:76, term/term.html:79, term/term.html:84 |
| `*hugolib.pageState` | `Parent` | 14769 | `()`×14769 | `*hugolib.pageState`×10887, `nil(page.Page)`×3882 | partials/breadcrumb.html:7, partials/breadcrumb.html:8, partials/marketing/jsonLd.html:389 |
| `*hugolib.pageState` | `Permalink` | 56764 | `()`×56764 | `string`×56764 | _default/latesturl.html:2, _default/rss.xml:20, _default/rss.xml:38, _default/rss.xml:46, _default/rss.xml:49, _default/rss.xml:50, index.html:133, partials/breadcrumb.html:51, partials/breadcrumb.html:58, partials/breadcrumb.html:65, partials/breadcrumb.html:72, partials/breadcrumb.html:86, partials/head.html:124, partials/head.html:130, partials/head.html:134, partials/head.html:135, partials/head.html:138, partials/head.html:161, partials/head.html:197, partials/header.html:117, partials/header.html:125, partials/marketing/jsonLd.html:9, partials/marketing/jsonLd.html:121, partials/marketing/jsonLd.html:411, partials/related.html:7, partials/single/socialshare.html:5, partials/single/socialshare.html:13, partials/single/socialshare.html:22, partials/single/socialshare.html:30, partials/single/socialshare.html:38, partials/single/socialshare.html:47, partials/single/socialshare.html:56, sitemap.xml:5, sitemap.xml:7, sitemap.xml:26, sitemap.xml:31 |
| `*hugolib.pageState` | `Plain` | 476 | `()`×476 | `string`×476 | _default/index.json:16 |
| `*hugolib.pageState` | `PublishDate` | 927 | `()`×927 | `time.Time`×927 | partials/head.html:239, partials/head.html:242, partials/marketing/jsonLd.html:132, partials/marketing/jsonLd.html:133 |
| `*hugolib.pageState` | `RegularPages` | 66 | `()`×66 | `page.Pages`×66 | _default/rss.xml:5 |
| `*hugolib.pageState` | `RelPermalink` | 5425 | `()`×5425 | `string`×5425 | _default/index.json:17, _default/list.html:31, _default/list.html:65, _default/list.html:69, index.html:44, index.html:78, index.html:82, partials/carousel.html:17, partials/comments.html:91, partials/comments.html:179, taxonomy/list.html:38, taxonomy/list.html:69, taxonomy/list.html:73 |
| `*hugolib.pageState` | `Resources` | 5019 | `()`×5019 | `resource.Resources`×5019 | _default/index.json:7, _default/list.html:32, _default/single.html:47, index.html:45, partials/carousel.html:18, partials/carousel.html:21, partials/related.html:10, taxonomy/list.html:32, term/term.html:30 |
| `*hugolib.pageState` | `Scratch` | 4163 | `()`×4163 | `*maps.Scratch`×4163 | _default/index.json:1, _default/index.json:11, _default/index.json:21, partials/comments.html:192, partials/comments.html:196, partials/comments.html:197, partials/comments.html:198, partials/comments.html:199, partials/comments.html:201, partials/comments.html:220, partials/comments.html:290, partials/comments.html:341, partials/head.html:159 |
| `*hugolib.pageState` | `Section` | 238 | `()`×238 | `string`×238 | partials/head.html:210 |
| `*hugolib.pageState` | `Site` | 181135 | `()`×181135 | `*page.siteWrapper`×181135 | _default/baseof.html:4, _default/baseof.html:13, _default/baseof.html:15, _default/index.json:2, _default/latesturl.html:1, _default/list.html:104, _default/list.html:107, _default/list.html:108, _default/rss.xml:2, _default/rss.xml:9, _default/rss.xml:13, _default/rss.xml:19, _default/rss.xml:23, _default/rss.xml:24, _default/rss.xml:25, _default/rss.xml:26, _default/rss.xml:27, _default/rss.xml:30, _default/rss.xml:32, _default/rss.xml:37, _default/simple.html:51, _default/simple.html:54, _default/simple.html:55, _default/single.html:1, _default/single.html:138, _default/single.html:207, _default/single.html:210, _default/single.html:211, _default/single.html:214, _default/single.html:215, 404.html:5, 404.html:34, 404.html:66, 404.html:69, 404.html:70, index.html:5, index.html:26, index.html:29, index.html:98, index.html:99, index.html:130, index.html:183, index.html:220, index.html:223, index.html:224, partials/ads/adsensehead.html:1, partials/ads/adsensehead.html:2, partials/ads/adsensemanual.html:1, partials/ads/adsensemanual.html:6, partials/ads/adsensemanual.html:7, partials/breadcrumb.html:41, partials/breadcrumb.html:53, partials/carousel.html:1, partials/carousel.html:2, partials/comments.html:1, partials/comments.html:139, partials/comments.html:178, partials/comments.html:180, partials/comments.html:182, partials/footer.html:1, partials/footer.html:74, partials/footer.html:76, partials/footer.html:79, partials/footer.html:82, partials/head.html:16, partials/head.html:17, partials/head.html:19, partials/head.html:122, partials/head.html:194, partials/head.html:207, partials/head.html:217, partials/head.html:229, partials/head.html:264, partials/head.html:282, partials/header.html:1, partials/header.html:2, partials/header.html:34, partials/header.html:69, partials/marketing/google/googleGtag.html:1, partials/marketing/google/googleTagManagerHead.html:1, partials/marketing/jsonLd.html:2, partials/marketing/jsonLd.html:12, partials/marketing/jsonLd.html:19, partials/marketing/jsonLd.html:29, partials/marketing/jsonLd.html:30, partials/marketing/jsonLd.html:35, partials/marketing/jsonLd.html:36, partials/marketing/jsonLd.html:38, partials/marketing/jsonLd.html:48, partials/marketing/jsonLd.html:52, partials/marketing/jsonLd.html:55, partials/marketing/jsonLd.html:63, partials/marketing/jsonLd.html:64, partials/marketing/jsonLd.html:68, partials/marketing/jsonLd.html:70, partials/marketing/jsonLd.html:72, partials/marketing/jsonLd.html:82, partials/marketing/jsonLd.html:83, partials/marketing/jsonLd.html:88, partials/marketing/jsonLd.html:89, partials/marketing/jsonLd.html:91, partials/marketing/jsonLd.html:101, partials/marketing/jsonLd.html:105, partials/marketing/jsonLd.html:108, partials/marketing/jsonLd.html:124, partials/marketing/jsonLd.html:127, partials/marketing/jsonLd.html:165, partials/marketing/jsonLd.html:169, partials/marketing/jsonLd.html:170, partials/marketing/jsonLd.html:172, partials/marketing/jsonLd.html:182, partials/marketing/jsonLd.html:186, partials/marketing/jsonLd.html:189, partials/marketing/jsonLd.html:198, partials/marketing/jsonLd.html:201, partials/marketing/jsonLd.html:216, partials/marketing/jsonLd.html:217, partials/marketing/jsonLd.html:219, partials/marketing/jsonLd.html:229, partials/marketing/jsonLd.html:233, partials/marketing/jsonLd.html:236, partials/marketing/jsonLd.html:248, partials/marketing/jsonLd.html:249, partials/marketing/jsonLd.html:261, partials/marketing/jsonLd.html:262, partials/marketing/jsonLd.html:263, partials/marketing/jsonLd.html:266, partials/marketing/jsonLd.html:270, partials/marketing/jsonLd.html:271, partials/marketing/jsonLd.html:276, partials/marketing/jsonLd.html:278, partials/marketing/jsonLd.html:288, partials/marketing/jsonLd.html:289, partials/marketing/jsonLd.html:294, partials/marketing/jsonLd.html:295, partials/marketing/jsonLd.html:297, partials/marketing/jsonLd.html:307, partials/marketing/jsonLd.html:311, partials/marketing/jsonLd.html:314, partials/marketing/jsonLd.html:321, partials/marketing/jsonLd.html:322, partials/marketing/jsonLd.html:323, partials/marketing/jsonLd.html:326, partials/marketing/jsonLd.html:328, partials/marketing/jsonLd.html:330, partials/marketing/jsonLd.html:340, partials/marketing/jsonLd.html:341, partials/marketing/jsonLd.html:346, partials/marketing/jsonLd.html:347, partials/marketing/jsonLd.html:349, partials/marketing/jsonLd.html:359, partials/marketing/jsonLd.html:363, partials/marketing/jsonLd.html:366, partials/marketing/jsonLd.html:409, partials/pagination.html:2, partials/pagination.html:21, partials/pagination.html:122, partials/related.html:1, partials/single/nutritionfacts.html:1, sitemap.xml:14, sitemap.xml:16, sitemap.xml:18, taxonomy/list.html:116, taxonomy/list.html:119, taxonomy/list.html:120, term/term.html:6, term/term.html:107, term/term.html:209, term/term.html:212, term/term.html:213, term/term.html:216, term/term.html:217 |
| `*hugolib.pageState` | `Sitemap` | 1730 | `()`×1730 | `config.SitemapConfig`×1730 | sitemap.xml:8 |
| `*hugolib.pageState` | `Title` | 51998 | `()`×51998 | `string`×51998 | _default/baseof.html:15, _default/index.json:11, _default/list.html:19, _default/list.html:59, _default/list.html:66, _default/rss.xml:19, _default/rss.xml:23, _default/rss.xml:45, _default/rss.xml:50, _default/simple.html:18, _default/single.html:1, _default/single.html:17, _default/single.html:27, index.html:72, index.html:79, index.html:137, index.html:139, partials/breadcrumb.html:16, partials/breadcrumb.html:21, partials/breadcrumb.html:26, partials/breadcrumb.html:31, partials/breadcrumb.html:36, partials/breadcrumb.html:46, partials/breadcrumb.html:60, partials/breadcrumb.html:67, partials/breadcrumb.html:74, partials/breadcrumb.html:81, partials/breadcrumb.html:88, partials/footer.html:26, partials/head.html:19, partials/marketing/jsonLd.html:13, partials/marketing/jsonLd.html:65, partials/marketing/jsonLd.html:117, partials/marketing/jsonLd.html:120, partials/marketing/jsonLd.html:263, partials/marketing/jsonLd.html:271, partials/marketing/jsonLd.html:323, partials/marketing/jsonLd.html:407, partials/related.html:37, partials/single/socialshare.html:13, partials/single/socialshare.html:47, partials/single/socialshare.html:56, taxonomy/list.html:16, taxonomy/list.html:18, taxonomy/list.html:70, term/term.html:5, term/term.html:20, term/term.html:24 |
| `*hugolib.pageState` | `Translations` | 716 | `()`×716 | `page.Pages`×716 | sitemap.xml:22 |
| `*hugolib.pageState` | `Type` | 238 | `()`×238 | `string`×238 | partials/marketing/jsonLd.html:5 |
| `*images.Namespace` | `Filter` | 6881 | `images.filter,*resources.resourceAdapter`×6881 | `*resources.resourceAdapter`×6881 | _default/_markup/render-image.html:7, _default/index.json:9, _default/list.html:38, _default/single.html:55, index.html:52, partials/carousel.html:28, partials/related.html:16, taxonomy/list.html:43, term/term.html:131 |
| `*images.Namespace` | `Overlay` | 9874 | `*resources.resourceAdapter,int,int`×9874 | `images.filter`×9874 | _default/_markup/render-image.html:6, _default/index.json:6, _default/list.html:37, _default/single.html:54, index.html:51, partials/carousel.html:27, partials/related.html:15, taxonomy/list.html:42, term/term.html:35, term/term.html:130 |
| `*js.Namespace` | `Build` | 9526 | `map[string]interface {},*resources.resourceAdapter`×9526 | `*resources.resourceAdapter`×9526 | _default/list.html:11, _default/list.html:104, _default/list.html:108, _default/list.html:112, _default/simple.html:11, _default/simple.html:51, _default/simple.html:55, _default/simple.html:59, _default/single.html:12, _default/single.html:207, _default/single.html:211, _default/single.html:215, _default/single.html:223, 404.html:21, 404.html:66, 404.html:70, 404.html:74, index.html:21, index.html:220, index.html:224, index.html:228, taxonomy/list.html:11, taxonomy/list.html:116, taxonomy/list.html:120, taxonomy/list.html:124, term/term.html:15, term/term.html:209, term/term.html:213, term/term.html:217, term/term.html:222 |
| `*langs.Language` | `LanguageCode` | 1494 | `()`×1494 | `string`×1494 | EMB:alias.html:2 |
| `*langs.Language` | `Params` | 1920 | `()`×1920 | `maps.Params`×1920 | partials/head.html:178, partials/head.html:183, partials/single/nutritionfacts.html:1 |
| `*maps.Scratch` | `Add` | 11676 | `string,map[string]interface {}`×6074, `string,*hugolib.pageState`×5598, `string,[]interface {}`×2, `string,int`×2 | `string`×11676 | _default/index.json:1, _default/index.json:11, partials/comments.html:196, partials/marketing/jsonLd.html:388, partials/marketing/jsonLd.html:397 |
| `*maps.Scratch` | `Delete` | 2 | `string`×2 | `string`×2 | partials/comments.html:341 |
| `*maps.Scratch` | `Get` | 39977 | `string`×39977 | `*hugolib.pageState`×16794, `int64`×7294, `nil(interface {})`×6120, `page.Pages`×3906, `int`×3906, `[]map[string]interface {}`×1955, `string`×2 | _default/index.json:21, partials/comments.html:201, partials/comments.html:220, partials/comments.html:290, partials/marketing/jsonLd.html:387, partials/marketing/jsonLd.html:388, partials/marketing/jsonLd.html:389, partials/marketing/jsonLd.html:395, partials/marketing/jsonLd.html:396, partials/marketing/jsonLd.html:397, partials/marketing/jsonLd.html:398, partials/marketing/jsonLd.html:403 |
| `*maps.Scratch` | `Set` | 22679 | `string,*hugolib.pageState`×5598, `string,int64`×5598, `string,[]interface {}`×3906, `string,int`×3669, `string,string`×1955, `string,nil(page.Page)`×1953 | `string`×22679 | partials/comments.html:192, partials/comments.html:197, partials/comments.html:198, partials/head.html:159, partials/marketing/jsonLd.html:379, partials/marketing/jsonLd.html:380, partials/marketing/jsonLd.html:381, partials/marketing/jsonLd.html:389, partials/marketing/jsonLd.html:395, partials/marketing/jsonLd.html:398 |
| `*maps.Scratch` | `SetInMap` | 2 | `string,string,int`×2 | `string`×2 | partials/comments.html:199 |
| `*page.OutputFormat` | `MediaType` | 3206 | `()`×3206 | `media.Type`×3206 | _default/rss.xml:41, partials/head.html:122 |
| `*page.OutputFormat` | `Permalink` | 3206 | `()`×3206 | `string`×3206 | _default/rss.xml:41, partials/head.html:122 |
| `*page.Pager` | `First` | 270 | `()`×270 | `*page.Pager`×270 | partials/pagination.html:26 |
| `*page.Pager` | `HasNext` | 540 | `()`×540 | `bool`×540 | partials/pagination.html:92, partials/pagination.html:106 |
| `*page.Pager` | `HasPrev` | 270 | `()`×270 | `bool`×270 | partials/pagination.html:27 |
| `*page.Pager` | `Last` | 270 | `()`×270 | `*page.Pager`×270 | partials/pagination.html:105 |
| `*page.Pager` | `Next` | 203 | `()`×203 | `*page.Pager`×203 | partials/pagination.html:94 |
| `*page.Pager` | `PageNumber` | 16941 | `()`×16941 | `int`×16941 | _default/list.html:18, index.html:28, partials/head.html:141, partials/head.html:142, partials/pagination.html:62, partials/pagination.html:63, partials/pagination.html:64, partials/pagination.html:76, partials/pagination.html:83 |
| `*page.Pager` | `PagerSize` | 92 | `()`×92 | `int`×92 | _default/list.html:17, index.html:27 |
| `*page.Pager` | `Pagers` | 270 | `()`×270 | `page.pagers`×270 | partials/pagination.html:61 |
| `*page.Pager` | `Pages` | 3367 | `()`×3367 | `page.Pages`×3367 | _default/list.html:23, _default/list.html:25, index.html:37, taxonomy/list.html:25, taxonomy/list.html:30, term/term.html:116, term/term.html:118 |
| `*page.Pager` | `Prev` | 203 | `()`×203 | `*page.Pager`×203 | partials/pagination.html:35 |
| `*page.Pager` | `TotalPages` | 4862 | `()`×4862 | `int`×4862 | partials/pagination.html:20, partials/pagination.html:62 |
| `*page.Pager` | `URL` | 1720 | `()`×1720 | `string`×1720 | partials/pagination.html:29, partials/pagination.html:35, partials/pagination.html:83, partials/pagination.html:94, partials/pagination.html:108 |
| `*page.siteWrapper` | `AllPages` | 2 | `()`×2 | `page.Pages`×2 | _default/index.json:2 |
| `*page.siteWrapper` | `BaseURL` | 11597 | `()`×11597 | `string`×11597 | 404.html:34, partials/marketing/jsonLd.html:36, partials/marketing/jsonLd.html:63, partials/marketing/jsonLd.html:64, partials/marketing/jsonLd.html:89, partials/marketing/jsonLd.html:170, partials/marketing/jsonLd.html:217, partials/marketing/jsonLd.html:261, partials/marketing/jsonLd.html:262, partials/marketing/jsonLd.html:295, partials/marketing/jsonLd.html:321, partials/marketing/jsonLd.html:322, partials/marketing/jsonLd.html:347 |
| `*page.siteWrapper` | `Config` | 1491 | `()`×1491 | `page.SiteConfig`×1491 | _default/rss.xml:9 |
| `*page.siteWrapper` | `Data` | 5142 | `()`×5142 | `map[string]interface {}`×5142 | partials/comments.html:178, partials/comments.html:180, partials/comments.html:182 |
| `*page.siteWrapper` | `GetPage` | 15694 | `string`×15694 | `*hugolib.pageState`×15694 | index.html:99, index.html:130 |
| `*page.siteWrapper` | `Language` | 1620 | `()`×1620 | `*langs.Language`×1620 | EMB:alias.html:2, partials/single/nutritionfacts.html:1 |
| `*page.siteWrapper` | `LanguageCode` | 1953 | `()`×1953 | `string`×1953 | _default/baseof.html:4 |
| `*page.siteWrapper` | `Languages` | 1794 | `()`×1794 | `langs.Languages`×1794 | partials/head.html:169, partials/header.html:107 |
| `*page.siteWrapper` | `Params` | 115771 | `()`×115771 | `maps.Params`×115771 | _default/baseof.html:13, _default/list.html:104, _default/list.html:107, _default/list.html:108, _default/rss.xml:13, _default/rss.xml:23, _default/rss.xml:24, _default/rss.xml:25, _default/rss.xml:26, _default/rss.xml:27, _default/rss.xml:30, _default/rss.xml:32, _default/simple.html:51, _default/simple.html:54, _default/simple.html:55, _default/single.html:138, _default/single.html:207, _default/single.html:210, _default/single.html:211, _default/single.html:214, _default/single.html:215, 404.html:5, 404.html:66, 404.html:69, 404.html:70, index.html:5, index.html:29, index.html:220, index.html:223, index.html:224, partials/ads/adsensehead.html:1, partials/ads/adsensehead.html:2, partials/ads/adsensemanual.html:1, partials/ads/adsensemanual.html:6, partials/ads/adsensemanual.html:7, partials/carousel.html:2, partials/comments.html:1, partials/comments.html:139, partials/footer.html:1, partials/footer.html:74, partials/footer.html:76, partials/footer.html:79, partials/footer.html:82, partials/head.html:17, partials/head.html:152, partials/head.html:153, partials/head.html:207, partials/head.html:217, partials/head.html:221, partials/head.html:229, partials/head.html:264, partials/head.html:282, partials/header.html:1, partials/header.html:69, partials/marketing/google/googleGtag.html:1, partials/marketing/google/googleTagManagerHead.html:1, partials/marketing/jsonLd.html:2, partials/marketing/jsonLd.html:12, partials/marketing/jsonLd.html:19, partials/marketing/jsonLd.html:30, partials/marketing/jsonLd.html:35, partials/marketing/jsonLd.html:38, partials/marketing/jsonLd.html:52, partials/marketing/jsonLd.html:55, partials/marketing/jsonLd.html:68, partials/marketing/jsonLd.html:70, partials/marketing/jsonLd.html:72, partials/marketing/jsonLd.html:83, partials/marketing/jsonLd.html:88, partials/marketing/jsonLd.html:91, partials/marketing/jsonLd.html:105, partials/marketing/jsonLd.html:108, partials/marketing/jsonLd.html:124, partials/marketing/jsonLd.html:127, partials/marketing/jsonLd.html:165, partials/marketing/jsonLd.html:169, partials/marketing/jsonLd.html:172, partials/marketing/jsonLd.html:186, partials/marketing/jsonLd.html:189, partials/marketing/jsonLd.html:198, partials/marketing/jsonLd.html:201, partials/marketing/jsonLd.html:216, partials/marketing/jsonLd.html:219, partials/marketing/jsonLd.html:233, partials/marketing/jsonLd.html:236, partials/marketing/jsonLd.html:249, partials/marketing/jsonLd.html:263, partials/marketing/jsonLd.html:266, partials/marketing/jsonLd.html:270, partials/marketing/jsonLd.html:271, partials/marketing/jsonLd.html:276, partials/marketing/jsonLd.html:278, partials/marketing/jsonLd.html:289, partials/marketing/jsonLd.html:294, partials/marketing/jsonLd.html:297, partials/marketing/jsonLd.html:311, partials/marketing/jsonLd.html:314, partials/marketing/jsonLd.html:323, partials/marketing/jsonLd.html:326, partials/marketing/jsonLd.html:328, partials/marketing/jsonLd.html:330, partials/marketing/jsonLd.html:341, partials/marketing/jsonLd.html:346, partials/marketing/jsonLd.html:349, partials/marketing/jsonLd.html:363, partials/marketing/jsonLd.html:366, partials/pagination.html:2, partials/pagination.html:21, partials/pagination.html:122, sitemap.xml:14, sitemap.xml:16, sitemap.xml:18, taxonomy/list.html:116, taxonomy/list.html:119, taxonomy/list.html:120, term/term.html:107, term/term.html:209, term/term.html:212, term/term.html:213, term/term.html:216, term/term.html:217 |
| `*page.siteWrapper` | `RegularPages` | 2233 | `()`×2233 | `page.Pages`×2233 | _default/latesturl.html:1, _default/rss.xml:5, index.html:26, partials/carousel.html:1, partials/head.html:194, partials/related.html:1 |
| `*page.siteWrapper` | `Taxonomies` | 21 | `()`×21 | `page.TaxonomyList`×21 | index.html:98 |
| `*page.siteWrapper` | `Title` | 31053 | `()`×31053 | `string`×31053 | _default/baseof.html:13, _default/baseof.html:15, _default/rss.xml:19, _default/rss.xml:23, _default/rss.xml:37, _default/single.html:1, index.html:183, partials/breadcrumb.html:41, partials/breadcrumb.html:53, partials/head.html:16, partials/head.html:19, partials/head.html:122, partials/head.html:160, partials/header.html:2, partials/header.html:34, partials/marketing/jsonLd.html:29, partials/marketing/jsonLd.html:48, partials/marketing/jsonLd.html:82, partials/marketing/jsonLd.html:101, partials/marketing/jsonLd.html:182, partials/marketing/jsonLd.html:229, partials/marketing/jsonLd.html:248, partials/marketing/jsonLd.html:288, partials/marketing/jsonLd.html:307, partials/marketing/jsonLd.html:340, partials/marketing/jsonLd.html:359, partials/marketing/jsonLd.html:409, term/term.html:6 |
| `*path.Namespace` | `Ext` | 24551 | `*resources.resourceAdapter`×24545, `nil(resource.Resource)`×6 | `string`×24551 | _default/_markup/render-image.html:8, _default/_markup/render-image.html:12, _default/list.html:33, _default/list.html:45, _default/single.html:49, _default/single.html:61, index.html:46, index.html:58, partials/carousel.html:19, partials/carousel.html:35, partials/footer.html:10, partials/footer.html:15, partials/footer.html:20, partials/header.html:12, partials/header.html:16, partials/header.html:21, partials/related.html:11, partials/related.html:23, taxonomy/list.html:50, taxonomy/list.html:55, term/term.html:31, term/term.html:40, term/term.html:45, term/term.html:138 |
| `*postpub.PostPublishResource` | `Content` | 1953 | `()`×1953 | `string`×1953 | partials/head.html:278 |
| `*reflect.Namespace` | `IsSlice` | 1550 | `string`×777, `[]string`×770, `[]interface {}`×3 | `bool`×1550 | partials/taxonomy/brands.html:8, partials/taxonomy/categories.html:5, partials/taxonomy/categories.html:10, partials/taxonomy/companies.html:8, partials/taxonomy/company/facebook.html:9, partials/taxonomy/company/instagram.html:9, partials/taxonomy/company/twitter.html:8, partials/taxonomy/company/website.html:8, partials/taxonomy/company/youtube.html:8, partials/taxonomy/countries.html:8, partials/taxonomy/ingredients.html:6, partials/taxonomy/tags.html:8 |
| `*resources.Namespace` | `Concat` | 1953 | `string,resource.Resources`×1953 | `*resources.resourceAdapter`×1953 | _default/list.html:112, _default/simple.html:59, _default/single.html:223, 404.html:74, index.html:228, taxonomy/list.html:124, term/term.html:222 |
| `*resources.Namespace` | `ExecuteAsTemplate` | 3667 | `string,map[string]interface {},*resources.resourceAdapter`×3667 | `*resources.resourceAdapter`×3667 | _default/list.html:107, _default/simple.html:54, _default/single.html:210, _default/single.html:214, 404.html:69, index.html:223, taxonomy/list.html:119, term/term.html:212, term/term.html:216 |
| `*resources.Namespace` | `Get` | 67787 | `string`×67787 | `*resources.resourceAdapter`×67784, `nil(resource.Resource)`×3 | EMB:_markup/render-link.html:10, _default/_markup/render-image.html:4, _default/index.json:4, _default/list.html:11, _default/list.html:35, _default/list.html:100, _default/list.html:101, _default/list.html:103, _default/list.html:106, _default/rss.xml:35, _default/simple.html:11, _default/simple.html:47, _default/simple.html:48, _default/simple.html:50, _default/simple.html:53, _default/single.html:12, _default/single.html:52, _default/single.html:204, _default/single.html:206, _default/single.html:209, _default/single.html:213, _default/single.html:221, 404.html:21, 404.html:62, 404.html:63, 404.html:65, 404.html:68, index.html:21, index.html:49, index.html:216, index.html:217, index.html:219, index.html:222, partials/carousel.html:25, partials/footer.html:9, partials/head.html:27, partials/head.html:32, partials/head.html:37, partials/head.html:42, partials/head.html:47, partials/head.html:52, partials/head.html:57, partials/head.html:62, partials/head.html:67, partials/head.html:73, partials/head.html:79, partials/head.html:85, partials/head.html:91, partials/head.html:98, partials/head.html:102, partials/head.html:106, partials/head.html:110, partials/head.html:114, partials/head.html:118, partials/head.html:153, partials/head.html:276, partials/header.html:11, partials/marketing/jsonLd.html:4, partials/related.html:13, taxonomy/list.html:11, taxonomy/list.html:40, taxonomy/list.html:112, taxonomy/list.html:113, taxonomy/list.html:115, taxonomy/list.html:118, term/term.html:15, term/term.html:33, term/term.html:128, term/term.html:206, term/term.html:208, term/term.html:211, term/term.html:215, term/term.html:221 |
| `*resources.Namespace` | `GetRemote` | 87 | `string`×87 | `*resources.resourceAdapter`×87 | partials/marketing/jsonLd.html:202 |
| `*resources.Namespace` | `PostProcess` | 1953 | `*resources.resourceAdapter`×1953 | `*postpub.PostPublishResource`×1953 | partials/head.html:276 |
| `*resources.resourceAdapter` | `Height` | 2100 | `()`×2100 | `int`×2100 | _default/_markup/render-image.html:2, _default/_markup/render-image.html:21 |
| `*resources.resourceAdapter` | `Permalink` | 46747 | `()`×46747 | `string`×46747 | _default/list.html:12, _default/list.html:113, _default/rss.xml:36, _default/simple.html:12, _default/simple.html:60, _default/single.html:13, _default/single.html:224, 404.html:22, 404.html:75, index.html:22, index.html:229, partials/head.html:27, partials/head.html:32, partials/head.html:37, partials/head.html:42, partials/head.html:47, partials/head.html:52, partials/head.html:57, partials/head.html:62, partials/head.html:67, partials/head.html:73, partials/head.html:79, partials/head.html:85, partials/head.html:91, partials/head.html:98, partials/head.html:102, partials/head.html:106, partials/head.html:110, partials/head.html:114, partials/head.html:118, partials/head.html:153, partials/marketing/jsonLd.html:49, partials/marketing/jsonLd.html:102, partials/marketing/jsonLd.html:183, partials/marketing/jsonLd.html:230, partials/marketing/jsonLd.html:308, partials/marketing/jsonLd.html:360, taxonomy/list.html:12, taxonomy/list.html:125, term/term.html:16, term/term.html:223 |
| `*resources.resourceAdapter` | `RelPermalink` | 31436 | `()`×31436 | `string`×31436 | _default/_markup/render-image.html:11, _default/_markup/render-image.html:13, _default/_markup/render-image.html:19, _default/index.json:14, _default/list.html:43, _default/list.html:48, _default/list.html:58, _default/single.html:59, _default/single.html:64, _default/single.html:74, index.html:56, index.html:60, index.html:71, partials/carousel.html:32, partials/carousel.html:36, partials/carousel.html:42, partials/footer.html:14, partials/footer.html:21, partials/footer.html:25, partials/header.html:15, partials/header.html:22, partials/header.html:26, partials/related.html:21, partials/related.html:26, partials/related.html:36, taxonomy/list.html:48, taxonomy/list.html:53, taxonomy/list.html:58, taxonomy/list.html:63, term/term.html:39, term/term.html:43, term/term.html:46, term/term.html:50, term/term.html:136, term/term.html:141, term/term.html:151 |
| `*resources.resourceAdapter` | `Resize` | 28463 | `string`×28463 | `*resources.resourceAdapter`×28463 | _default/_markup/render-image.html:5, _default/_markup/render-image.html:9, _default/index.json:5, _default/index.json:8, _default/index.json:10, _default/list.html:34, _default/list.html:36, _default/list.html:39, _default/single.html:51, _default/single.html:53, _default/single.html:56, index.html:48, index.html:50, index.html:53, partials/carousel.html:24, partials/carousel.html:26, partials/carousel.html:29, partials/footer.html:11, partials/footer.html:12, partials/header.html:13, partials/related.html:12, partials/related.html:14, partials/related.html:17, taxonomy/list.html:39, taxonomy/list.html:41, taxonomy/list.html:44, term/term.html:32, term/term.html:34, term/term.html:37, term/term.html:127, term/term.html:129, term/term.html:132 |
| `*resources.resourceAdapter` | `Title` | 105 | `()`×105 | `string`×105 | partials/carousel.html:43 |
| `*resources.resourceAdapter` | `Width` | 2100 | `()`×2100 | `int`×2100 | _default/_markup/render-image.html:2, _default/_markup/render-image.html:20 |
| `*source.File` | `UniqueID` | 246 | `()`×246 | `string`×246 | partials/comments.html:97 |
| `*strings.Namespace` | `HasPrefix` | 558 | `string,string`×558 | `bool`×558 | EMB:_markup/render-link.html:3 |
| `*strings.Namespace` | `TrimPrefix` | 3 | `string,string`×3 | `string`×3 | EMB:_markup/render-link.html:6 |
| `*tables.tableContext` | `Attributes` | 238 | `()`×238 | `map[string]interface {}`×238 | EMB:_markup/render-table.html:2 |
| `*tables.tableContext` | `TBody` | 238 | `()`×238 | `[]hooks.TableRow`×238 | EMB:_markup/render-table.html:23 |
| `*tables.tableContext` | `THead` | 238 | `()`×238 | `[]hooks.TableRow`×238 | EMB:_markup/render-table.html:8 |
| `*transform.Namespace` | `Unmarshal` | 87 | `*resources.resourceAdapter`×87 | `map[string]interface {}`×87 | partials/marketing/jsonLd.html:204 |
| `*url.URL` | `IsAbs` | 558 | `()`×558 | `bool`×558 | EMB:_markup/render-link.html:5 |
| `*url.URL` | `String` | 1116 | `()`×1116 | `string`×1116 | EMB:_markup/render-link.html:2, EMB:_markup/render-link.html:3 |
| `*urls.Namespace` | `Parse` | 558 | `string`×558 | `*url.URL`×558 | EMB:_markup/render-link.html:1 |
| `goldmark.headingContext` | `Anchor` | 1156 | `()`×1156 | `string`×1156 | _default/_markup/render-heading.html:1 |
| `goldmark.headingContext` | `Level` | 1156 | `()`×1156 | `int`×1156 | _default/_markup/render-heading.html:1 |
| `goldmark.headingContext` | `Text` | 578 | `()`×578 | `hstring.HTML`×578 | _default/_markup/render-heading.html:1 |
| `goldmark.imageLinkContext` | `Destination` | 1050 | `()`×1050 | `string`×1050 | _default/_markup/render-image.html:1 |
| `goldmark.imageLinkContext` | `Page` | 1050 | `()`×1050 | `*hugolib.pageForRenderHooks`×1050 | _default/_markup/render-image.html:1 |
| `goldmark.imageLinkContext` | `Text` | 1050 | `()`×1050 | `hstring.HTML`×1050 | _default/_markup/render-image.html:22 |
| `goldmark.linkContext` | `Destination` | 558 | `()`×558 | `string`×558 | EMB:_markup/render-link.html:1 |
| `goldmark.linkContext` | `PageInner` | 6 | `()`×6 | `*hugolib.pageForRenderHooks`×6 | EMB:_markup/render-link.html:8, EMB:_markup/render-link.html:9 |
| `goldmark.linkContext` | `Text` | 558 | `()`×558 | `hstring.HTML`×558 | EMB:_markup/render-link.html:21 |
| `goldmark.linkContext` | `Title` | 558 | `()`×558 | `string`×558 | EMB:_markup/render-link.html:21 |
| `hugolib.pageWithWeight0` | `Date` | 3350 | `()`×3350 | `time.Time`×3350 | _default/rss.xml:47 |
| `hugolib.pageWithWeight0` | `Description` | 3350 | `()`×3350 | `string`×3350 | _default/rss.xml:50 |
| `hugolib.pageWithWeight0` | `Params` | 21791 | `()`×21791 | `maps.Params`×21791 | _default/rss.xml:47, _default/rss.xml:50, term/term.html:119, term/term.html:126, term/term.html:160 |
| `hugolib.pageWithWeight0` | `Permalink` | 10050 | `()`×10050 | `string`×10050 | _default/rss.xml:46, _default/rss.xml:49, _default/rss.xml:50 |
| `hugolib.pageWithWeight0` | `RelPermalink` | 11739 | `()`×11739 | `string`×11739 | term/term.html:125, term/term.html:157, term/term.html:161 |
| `hugolib.pageWithWeight0` | `Resources` | 3913 | `()`×3913 | `resource.Resources`×3913 | term/term.html:126 |
| `hugolib.pageWithWeight0` | `Title` | 14526 | `()`×14526 | `string`×14526 | _default/rss.xml:45, _default/rss.xml:50, term/term.html:152, term/term.html:158 |
| `page.Data` | `Pages` | 73 | `()`×73 | `page.Pages`×73 | _default/list.html:16, sitemap.xml:4 |
| `page.OutputFormats` | `Get` | 1491 | `string`×1491 | `*page.OutputFormat`×1491 | _default/rss.xml:40 |
| `page.Pages` | `Related` | 2188 | `*hugolib.pageState`×2188 | `page.Pages`×2188 | partials/head.html:194, partials/related.html:1 |
| `page.Pages` | `Reverse` | 45 | `()`×45 | `page.Pages`×45 | _default/index.json:2, _default/latesturl.html:1, index.html:26, partials/carousel.html:1 |
| `resource.Resources` | `Get` | 9985 | `string`×6998, `nil(interface {})`×2987 | `*resources.resourceAdapter`×6995, `nil(resource.Resource)`×2990 | EMB:_markup/render-link.html:9, _default/_markup/render-image.html:1, _default/index.json:7, _default/list.html:32, _default/single.html:47, index.html:45, partials/carousel.html:18, partials/carousel.html:21, partials/related.html:10, taxonomy/list.html:32, term/term.html:30, term/term.html:126 |
| `time.Time` | `Format` | 15816 | `string`×15816 | `string`×15816 | EMB:sitemapindex.xml:7, _default/rss.xml:27, _default/rss.xml:30, _default/rss.xml:40, _default/rss.xml:47, partials/footer.html:76, partials/footer.html:80, partials/head.html:242, partials/head.html:253, partials/head.html:260, sitemap.xml:8 |
| `time.Time` | `IsZero` | 5421 | `()`×5421 | `bool`×5421 | EMB:sitemapindex.xml:6, _default/rss.xml:39, partials/head.html:239, partials/head.html:244, partials/head.html:250, partials/head.html:257, sitemap.xml:7 |

### A.3 Struct-field and map-key accesses (non-method `.X` lookups)

Receiver `maps.Params` = case-insensitive lookup (key lower-cased, tplimpl/template_funcs.go:70 GetMapValue). Receiver `map[string]interface {}` / `page.Data` = exact, case-sensitive Go map index. `<invalid>` result = missing key (prints as empty in html/template, falsy in if/with).

| kind | receiver type | key/field | calls | result types | sites |
|---|---|---|---|---|---|
| field | `config.SitemapConfig` | `ChangeFreq` | 1730 | `string`×1730 | sitemap.xml:8 |
| field | `hooks.TableCell` | `Alignment` | 3174 | `string`×3174 | EMB:_markup/render-table.html:12, EMB:_markup/render-table.html:27 |
| field | `hooks.TableCell` | `Text` | 3174 | `hstring.HTML`×3174 | EMB:_markup/render-table.html:16, EMB:_markup/render-table.html:31 |
| field | `hugolib.ShortcodeWithPage` | `Params` | 6 | `[]interface {}`×6 | EMB:_shortcodes/ref.html:1 |
| field | `hugolib.aliasPage` | `Permalink` | 4482 | `string`×4482 | EMB:alias.html:4, EMB:alias.html:5, EMB:alias.html:8 |
| field | `langs.Language` | `Lang` | 12558 | `string`×12558 | partials/head.html:129, partials/head.html:174, partials/head.html:175, partials/header.html:112, partials/header.html:114 |
| field | `langs.Language` | `LanguageName` | 1794 | `string`×1794 | partials/header.html:120, partials/header.html:127 |
| field | `media.Type` | `Type` | 1715 | `string`×1715 | partials/head.html:122 |
| field | `neohugo.HugoInfo` | `Environment` | 7263 | `string`×7263 | _default/list.html:5, _default/list.html:80, _default/simple.html:5, _default/simple.html:27, _default/single.html:6, _default/single.html:174, 404.html:15, 404.html:42, index.html:15, index.html:196, partials/head.html:269, taxonomy/list.html:5, taxonomy/list.html:77, taxonomy/list.html:92, term/term.html:9, term/term.html:165, term/term.html:180 |
| field | `page.OutputFormat` | `Rel` | 1715 | `string`×1715 | partials/head.html:122 |
| field | `page.SiteConfig` | `Services` | 1491 | `services.Config`×1491 | _default/rss.xml:9 |
| field | `services.Config` | `RSS` | 1491 | `services.RSS`×1491 | _default/rss.xml:9 |
| field | `services.RSS` | `Limit` | 1491 | `int`×1491 | _default/rss.xml:9 |
| field | `url.URL` | `Path` | 3 | `string`×3 | EMB:_markup/render-link.html:6 |
| mapkey | `map[string]interface {}` | `_id` | 12 | `string`×12 | partials/comments.html:198, partials/comments.html:199, partials/comments.html:206, partials/comments.html:214, partials/comments.html:282, partials/comments.html:285 |
| mapkey | `map[string]interface {}` | `api` | 2 | `string`×2 | ASSET:comment.ts:47, ASSET:search.ts:11 |
| mapkey | `map[string]interface {}` | `comment` | 2 | `string`×2 | partials/comments.html:278 |
| mapkey | `map[string]interface {}` | `commentCount` | 87 | `string`×87 | partials/marketing/jsonLd.html:244 |
| mapkey | `map[string]interface {}` | `comments` | 5142 | `map[string]interface {}`×5142 | partials/comments.html:178, partials/comments.html:180, partials/comments.html:182 |
| mapkey | `map[string]interface {}` | `contentDetails` | 87 | `map[string]interface {}`×87 | partials/marketing/jsonLd.html:213 |
| mapkey | `map[string]interface {}` | `context` | 2037 | `int`×1148, `time.Time`×462, `*hugolib.pageState`×380, `string`×39, `float64`×8 | partials/comments.html:397, partials/rating/rating.html:70, partials/single/date.html:6, partials/single/date.html:7, partials/single/ingredientslist.html:19, partials/single/nutritionfacts.html:1, partials/single/nutritionfacts.html:2, partials/single/whenseen.html:12 |
| mapkey | `map[string]interface {}` | `date` | 4 | `string`×4 | partials/comments.html:223, partials/comments.html:224 |
| mapkey | `map[string]interface {}` | `Date` | 270 | `<invalid>`×270 | partials/single/date.html:1, partials/single/whenseen.html:1 |
| mapkey | `map[string]interface {}` | `description` | 87 | `string`×87 | partials/marketing/jsonLd.html:210 |
| mapkey | `map[string]interface {}` | `duration` | 87 | `string`×87 | partials/marketing/jsonLd.html:213 |
| mapkey | `map[string]interface {}` | `email` | 2 | `<invalid>`×1, `string`×1 | partials/comments.html:208 |
| mapkey | `map[string]interface {}` | `icon` | 25389 | `string`×25389 | partials/footer.html:50, partials/header.html:74 |
| mapkey | `map[string]interface {}` | `items` | 87 | `[]interface {}`×87 | partials/marketing/jsonLd.html:205 |
| mapkey | `map[string]interface {}` | `link` | 94003 | `string`×94003 | partials/footer.html:38, partials/footer.html:49, partials/header.html:73, partials/marketing/jsonLd.html:23, partials/marketing/jsonLd.html:42, partials/marketing/jsonLd.html:76, partials/marketing/jsonLd.html:95, partials/marketing/jsonLd.html:176, partials/marketing/jsonLd.html:223, partials/marketing/jsonLd.html:282, partials/marketing/jsonLd.html:301, partials/marketing/jsonLd.html:334, partials/marketing/jsonLd.html:353 |
| mapkey | `map[string]interface {}` | `name` | 61311 | `string`×61308, `<invalid>`×3 | partials/comments.html:209, partials/comments.html:215, partials/header.html:72, partials/single/ingredientslist.html:20, partials/single/ingredientslist.html:22 |
| mapkey | `map[string]interface {}` | `p1` | 45792 | `*hugolib.pageState`×45792 | partials/breadcrumb.html:7, partials/breadcrumb.html:8, partials/breadcrumb.html:9, partials/breadcrumb.html:12, partials/breadcrumb.html:13, partials/breadcrumb.html:16, partials/breadcrumb.html:18, partials/breadcrumb.html:21, partials/breadcrumb.html:23, partials/breadcrumb.html:26, partials/breadcrumb.html:28, partials/breadcrumb.html:31, partials/breadcrumb.html:33, partials/breadcrumb.html:36, partials/breadcrumb.html:38, partials/breadcrumb.html:41, partials/breadcrumb.html:46, partials/breadcrumb.html:49, partials/breadcrumb.html:51, partials/breadcrumb.html:53, partials/breadcrumb.html:56, partials/breadcrumb.html:58, partials/breadcrumb.html:60, partials/breadcrumb.html:63, partials/breadcrumb.html:65, partials/breadcrumb.html:67, partials/breadcrumb.html:70, partials/breadcrumb.html:72, partials/breadcrumb.html:74, partials/breadcrumb.html:77, partials/breadcrumb.html:81, partials/breadcrumb.html:86, partials/breadcrumb.html:88 |
| mapkey | `map[string]interface {}` | `p2` | 10301 | `*hugolib.pageState`×10301 | partials/breadcrumb.html:8, partials/breadcrumb.html:12, partials/breadcrumb.html:18, partials/breadcrumb.html:23, partials/breadcrumb.html:28, partials/breadcrumb.html:33 |
| mapkey | `map[string]interface {}` | `page` | 16794 | `*hugolib.pageState`×16794 | partials/marketing/jsonLd.html:406, partials/marketing/jsonLd.html:407, partials/marketing/jsonLd.html:409, partials/marketing/jsonLd.html:411 |
| mapkey | `map[string]interface {}` | `percentage` | 1357 | `string`×735, `<invalid>`×622 | partials/single/ingredientslist.html:23 |
| mapkey | `map[string]interface {}` | `position` | 11196 | `int64`×7290, `int`×3906 | partials/marketing/jsonLd.html:404, partials/marketing/jsonLd.html:405 |
| mapkey | `map[string]interface {}` | `publishedAt` | 87 | `string`×87 | partials/marketing/jsonLd.html:212 |
| mapkey | `map[string]interface {}` | `rating` | 334430 | `string`×334230, `maps.Params`×200 | partials/comments.html:358, partials/comments.html:360, partials/comments.html:361, partials/comments.html:364, partials/comments.html:366, partials/comments.html:367, partials/comments.html:370, partials/comments.html:372, partials/comments.html:373, partials/comments.html:376, partials/comments.html:378, partials/comments.html:379, partials/comments.html:382, partials/comments.html:384, partials/comments.html:385, partials/rating/rating.html:21 |
| mapkey | `map[string]interface {}` | `ratingName` | 23438 | `string`×23438 | partials/comments.html:352, partials/comments.html:394, partials/rating/rating.html:68 |
| mapkey | `map[string]interface {}` | `replyThread` | 4 | `<invalid>`×4 | partials/comments.html:195, partials/comments.html:290 |
| mapkey | `map[string]interface {}` | `reviewBitter` | 1 | `<invalid>`×1 | partials/comments.html:256 |
| mapkey | `map[string]interface {}` | `reviewChewy` | 1 | `<invalid>`×1 | partials/comments.html:265 |
| mapkey | `map[string]interface {}` | `reviewCrispy` | 1 | `<invalid>`×1 | partials/comments.html:262 |
| mapkey | `map[string]interface {}` | `reviewJuicy` | 1 | `<invalid>`×1 | partials/comments.html:268 |
| mapkey | `map[string]interface {}` | `reviewPackage` | 2 | `<invalid>`×1, `float64`×1 | partials/comments.html:229, partials/comments.html:242 |
| mapkey | `map[string]interface {}` | `reviewQuantity` | 2 | `<invalid>`×1, `float64`×1 | partials/comments.html:229, partials/comments.html:239 |
| mapkey | `map[string]interface {}` | `reviewSalty` | 1 | `<invalid>`×1 | partials/comments.html:253 |
| mapkey | `map[string]interface {}` | `reviewSavory` | 1 | `<invalid>`×1 | partials/comments.html:259 |
| mapkey | `map[string]interface {}` | `reviewSmell` | 2 | `<invalid>`×1, `float64`×1 | partials/comments.html:229, partials/comments.html:236 |
| mapkey | `map[string]interface {}` | `reviewSour` | 1 | `<invalid>`×1 | partials/comments.html:250 |
| mapkey | `map[string]interface {}` | `reviewSpicy` | 1 | `<invalid>`×1 | partials/comments.html:271 |
| mapkey | `map[string]interface {}` | `reviewSweet` | 1 | `<invalid>`×1 | partials/comments.html:247 |
| mapkey | `map[string]interface {}` | `reviewTastyTaste` | 3 | `float64`×2, `<invalid>`×1 | partials/comments.html:229, partials/comments.html:233 |
| mapkey | `map[string]interface {}` | `Site` | 272 | `<invalid>`×272 | partials/comments.html:224, partials/single/date.html:2, partials/single/whenseen.html:2 |
| mapkey | `map[string]interface {}` | `snippet` | 522 | `map[string]interface {}`×522 | partials/marketing/jsonLd.html:209, partials/marketing/jsonLd.html:210, partials/marketing/jsonLd.html:211, partials/marketing/jsonLd.html:212, partials/marketing/jsonLd.html:241 |
| mapkey | `map[string]interface {}` | `standard` | 174 | `map[string]interface {}`×174 | partials/marketing/jsonLd.html:211 |
| mapkey | `map[string]interface {}` | `statistics` | 174 | `map[string]interface {}`×174 | partials/marketing/jsonLd.html:244, partials/marketing/jsonLd.html:245 |
| mapkey | `map[string]interface {}` | `tags` | 87 | `[]interface {}`×69, `<invalid>`×18 | partials/marketing/jsonLd.html:241 |
| mapkey | `map[string]interface {}` | `text` | 11718 | `string`×11718 | partials/footer.html:39 |
| mapkey | `map[string]interface {}` | `thumbnails` | 174 | `map[string]interface {}`×174 | partials/marketing/jsonLd.html:211 |
| mapkey | `map[string]interface {}` | `title` | 87 | `string`×87 | partials/marketing/jsonLd.html:209 |
| mapkey | `map[string]interface {}` | `type` | 65024 | `string`×65024 | partials/marketing/jsonLd.html:22, partials/marketing/jsonLd.html:41, partials/marketing/jsonLd.html:75, partials/marketing/jsonLd.html:94, partials/marketing/jsonLd.html:175, partials/marketing/jsonLd.html:222, partials/marketing/jsonLd.html:281, partials/marketing/jsonLd.html:300, partials/marketing/jsonLd.html:333, partials/marketing/jsonLd.html:352 |
| mapkey | `map[string]interface {}` | `url` | 87 | `string`×87 | partials/marketing/jsonLd.html:211 |
| mapkey | `map[string]interface {}` | `viewCount` | 87 | `string`×87 | partials/marketing/jsonLd.html:245 |
| mapkey | `maps.Params` | `a` | 79 | `maps.Params`×68, `<invalid>`×11 | partials/single/nutritionfacts.html:285 |
| mapkey | `maps.Params` | `added` | 116 | `maps.Params`×59, `<invalid>`×57 | partials/single/nutritionfacts.html:240 |
| mapkey | `maps.Params` | `Ads` | 19254 | `maps.Params`×19254 | partials/ads/adsensehead.html:1, partials/ads/adsensehead.html:2, partials/ads/adsensemanual.html:1, partials/ads/adsensemanual.html:6, partials/ads/adsensemanual.html:7 |
| mapkey | `maps.Params` | `Api` | 174 | `maps.Params`×174 | partials/marketing/jsonLd.html:198, partials/marketing/jsonLd.html:201 |
| mapkey | `maps.Params` | `Apipro` | 3667 | `string`×3667 | _default/list.html:107, _default/simple.html:54, _default/single.html:210, _default/single.html:214, 404.html:69, index.html:223, taxonomy/list.html:119, term/term.html:212, term/term.html:216 |
| mapkey | `maps.Params` | `Appid` | 1995 | `string`×1995 | 404.html:5, index.html:5, partials/head.html:217 |
| mapkey | `maps.Params` | `author` | 13292 | `string`×13292 | _default/rss.xml:24, _default/rss.xml:25, partials/head.html:264, partials/marketing/jsonLd.html:55, partials/marketing/jsonLd.html:68, partials/marketing/jsonLd.html:108, partials/marketing/jsonLd.html:127, partials/marketing/jsonLd.html:165, partials/marketing/jsonLd.html:189, partials/marketing/jsonLd.html:236, partials/marketing/jsonLd.html:266, partials/marketing/jsonLd.html:314, partials/marketing/jsonLd.html:326, partials/marketing/jsonLd.html:366 |
| mapkey | `maps.Params` | `Author` | 2657 | `<invalid>`×1801, `string`×856 | _default/single.html:33, _default/single.html:39, partials/head.html:202, partials/head.html:232 |
| mapkey | `maps.Params` | `authorEmail` | 2982 | `string`×2982 | _default/rss.xml:23, _default/rss.xml:24 |
| mapkey | `maps.Params` | `b1` | 79 | `maps.Params`×73, `<invalid>`×6 | partials/single/nutritionfacts.html:301 |
| mapkey | `maps.Params` | `b12` | 79 | `maps.Params`×62, `<invalid>`×17 | partials/single/nutritionfacts.html:413 |
| mapkey | `maps.Params` | `b2` | 79 | `maps.Params`×71, `<invalid>`×8 | partials/single/nutritionfacts.html:317 |
| mapkey | `maps.Params` | `b3` | 79 | `maps.Params`×62, `<invalid>`×17 | partials/single/nutritionfacts.html:333 |
| mapkey | `maps.Params` | `b5` | 79 | `maps.Params`×62, `<invalid>`×17 | partials/single/nutritionfacts.html:349 |
| mapkey | `maps.Params` | `b6` | 79 | `maps.Params`×62, `<invalid>`×17 | partials/single/nutritionfacts.html:365 |
| mapkey | `maps.Params` | `b7` | 79 | `maps.Params`×62, `<invalid>`×17 | partials/single/nutritionfacts.html:381 |
| mapkey | `maps.Params` | `b9` | 79 | `maps.Params`×62, `<invalid>`×17 | partials/single/nutritionfacts.html:397 |
| mapkey | `maps.Params` | `BaseURL` | 1953 | `string`×1953 | partials/marketing/jsonLd.html:12, partials/marketing/jsonLd.html:270 |
| mapkey | `maps.Params` | `baseURLSearch` | 3993 | `string`×3993 | partials/marketing/jsonLd.html:30, partials/marketing/jsonLd.html:83, partials/marketing/jsonLd.html:249, partials/marketing/jsonLd.html:289, partials/marketing/jsonLd.html:341 |
| mapkey | `maps.Params` | `bitter` | 200 | `<invalid>`×147, `int`×53 | partials/rating/rating.html:34 |
| mapkey | `maps.Params` | `Brands` | 464 | `string`×404, `<invalid>`×60 | _default/single.html:85, partials/marketing/jsonLd.html:134 |
| mapkey | `maps.Params` | `c` | 79 | `maps.Params`×62, `<invalid>`×17 | partials/single/nutritionfacts.html:429 |
| mapkey | `maps.Params` | `calcium` | 74 | `maps.Params`×67, `<invalid>`×7 | partials/single/nutritionfacts.html:509 |
| mapkey | `maps.Params` | `calories` | 126 | `int`×124, `<invalid>`×2 | partials/single/nutritionfacts.html:29 |
| mapkey | `maps.Params` | `carbohydrate` | 126 | `maps.Params`×124, `<invalid>`×2 | partials/single/nutritionfacts.html:198 |
| mapkey | `maps.Params` | `Carousel` | 21 | `maps.Params`×21 | partials/carousel.html:2 |
| mapkey | `maps.Params` | `categories` | 4039 | `[]string`×3947, `<invalid>`×65, `string`×19, `[]interface {}`×8 | _default/index.json:15, _default/rss.xml:47 |
| mapkey | `maps.Params` | `Categories` | 464 | `[]string`×442, `<invalid>`×14, `string`×6, `[]interface {}`×2 | _default/single.html:148, partials/marketing/jsonLd.html:140 |
| mapkey | `maps.Params` | `chewy` | 200 | `<invalid>`×147, `int`×53 | partials/rating/rating.html:43 |
| mapkey | `maps.Params` | `chloride` | 74 | `maps.Params`×62, `<invalid>`×12 | partials/single/nutritionfacts.html:525 |
| mapkey | `maps.Params` | `cholesterol` | 126 | `maps.Params`×95, `<invalid>`×31 | partials/single/nutritionfacts.html:147 |
| mapkey | `maps.Params` | `choline` | 74 | `maps.Params`×62, `<invalid>`×12 | partials/single/nutritionfacts.html:493 |
| mapkey | `maps.Params` | `chromium` | 74 | `maps.Params`×62, `<invalid>`×12 | partials/single/nutritionfacts.html:541 |
| mapkey | `maps.Params` | `Comment` | 7095 | `maps.Params`×7095 | _default/list.html:107, _default/simple.html:54, _default/single.html:210, _default/single.html:214, 404.html:69, index.html:223, partials/comments.html:1, partials/comments.html:139, taxonomy/list.html:119, term/term.html:212, term/term.html:216 |
| mapkey | `maps.Params` | `Companies` | 464 | `string`×444, `<invalid>`×14, `[]string`×6 | _default/single.html:82, partials/marketing/jsonLd.html:195 |
| mapkey | `maps.Params` | `Company` | 3444 | `string`×3444 | _default/rss.xml:32, partials/footer.html:82 |
| mapkey | `maps.Params` | `company` | 8673 | `string`×8673 | partials/marketing/jsonLd.html:35, partials/marketing/jsonLd.html:52, partials/marketing/jsonLd.html:88, partials/marketing/jsonLd.html:105, partials/marketing/jsonLd.html:124, partials/marketing/jsonLd.html:169, partials/marketing/jsonLd.html:186, partials/marketing/jsonLd.html:216, partials/marketing/jsonLd.html:233, partials/marketing/jsonLd.html:294, partials/marketing/jsonLd.html:311, partials/marketing/jsonLd.html:346, partials/marketing/jsonLd.html:363 |
| mapkey | `maps.Params` | `Compiler` | 5620 | `maps.Params`×5620 | _default/list.html:104, _default/list.html:108, _default/simple.html:51, _default/simple.html:55, _default/single.html:207, _default/single.html:211, _default/single.html:215, 404.html:66, 404.html:70, index.html:220, index.html:224, taxonomy/list.html:116, taxonomy/list.html:120, term/term.html:209, term/term.html:213, term/term.html:217 |
| mapkey | `maps.Params` | `copper` | 74 | `maps.Params`×62, `<invalid>`×12 | partials/single/nutritionfacts.html:557 |
| mapkey | `maps.Params` | `Countries` | 235 | `[]string`×156, `<invalid>`×79 | _default/single.html:88 |
| mapkey | `maps.Params` | `crispy` | 200 | `int`×134, `<invalid>`×66 | partials/rating/rating.html:40 |
| mapkey | `maps.Params` | `d` | 79 | `maps.Params`×62, `<invalid>`×17 | partials/single/nutritionfacts.html:443 |
| mapkey | `maps.Params` | `DataAdClient` | 5790 | `string`×5790 | partials/ads/adsensehead.html:2, partials/ads/adsensemanual.html:6 |
| mapkey | `maps.Params` | `DataAdSlot` | 3837 | `string`×3837 | partials/ads/adsensemanual.html:7 |
| mapkey | `maps.Params` | `date` | 466 | `time.Time`×462, `<invalid>`×4 | _default/single.html:38, _default/single.html:42 |
| mapkey | `maps.Params` | `description` | 4174 | `string`×4160, `<invalid>`×14 | _default/index.json:12, partials/head.html:17, partials/marketing/jsonLd.html:70, partials/marketing/jsonLd.html:276, partials/marketing/jsonLd.html:328 |
| mapkey | `maps.Params` | `Description` | 3913 | `string`×3899, `<invalid>`×14 | term/term.html:160 |
| mapkey | `maps.Params` | `dietary_fiber` | 122 | `maps.Params`×84, `<invalid>`×38 | partials/single/nutritionfacts.html:213 |
| mapkey | `maps.Params` | `DynamicContent` | 4187 | `bool`×4187 | partials/head.html:282, partials/pagination.html:2, partials/pagination.html:21, partials/pagination.html:122 |
| mapkey | `maps.Params` | `e` | 79 | `maps.Params`×62, `<invalid>`×17 | partials/single/nutritionfacts.html:459 |
| mapkey | `maps.Params` | `Enable` | 1714 | `bool`×1714 | _default/single.html:138, term/term.html:107 |
| mapkey | `maps.Params` | `enabled` | 1714 | `bool`×1714 | partials/comments.html:1 |
| mapkey | `maps.Params` | `Facebook` | 5426 | `maps.Params`×3947, `<invalid>`×1474, `string`×5 | _default/single.html:138, 404.html:5, index.html:5, partials/head.html:207, partials/head.html:217, term/term.html:67, term/term.html:107 |
| mapkey | `maps.Params` | `fat` | 126 | `maps.Params`×120, `<invalid>`×6 | partials/single/nutritionfacts.html:57 |
| mapkey | `maps.Params` | `fluoride` | 74 | `maps.Params`×62, `<invalid>`×12 | partials/single/nutritionfacts.html:573 |
| mapkey | `maps.Params` | `footer` | 15478 | `maps.Params`×15478 | _default/rss.xml:13, partials/footer.html:1, partials/header.html:69, partials/marketing/jsonLd.html:2, partials/marketing/jsonLd.html:19, partials/marketing/jsonLd.html:38, partials/marketing/jsonLd.html:72, partials/marketing/jsonLd.html:91, partials/marketing/jsonLd.html:172, partials/marketing/jsonLd.html:219, partials/marketing/jsonLd.html:278, partials/marketing/jsonLd.html:297, partials/marketing/jsonLd.html:330, partials/marketing/jsonLd.html:349 |
| mapkey | `maps.Params` | `Google` | 3906 | `maps.Params`×3906 | partials/marketing/google/googleGtag.html:1, partials/marketing/google/googleTagManagerHead.html:1 |
| mapkey | `maps.Params` | `GoogleAnalyticsGtag` | 1953 | `string`×1953 | partials/marketing/google/googleGtag.html:1 |
| mapkey | `maps.Params` | `GoogleTagManager` | 1953 | `string`×1953 | partials/marketing/google/googleTagManagerHead.html:1 |
| mapkey | `maps.Params` | `header` | 1953 | `maps.Params`×1953 | partials/header.html:1 |
| mapkey | `maps.Params` | `hideCopyright` | 3444 | `bool`×3444 | _default/rss.xml:26, partials/footer.html:74 |
| mapkey | `maps.Params` | `HomeTitle` | 105 | `string`×105 | _default/baseof.html:13, index.html:29, partials/marketing/jsonLd.html:263, partials/marketing/jsonLd.html:271, partials/marketing/jsonLd.html:323 |
| mapkey | `maps.Params` | `Image` | 9310 | `string`×7836, `<invalid>`×1474 | _default/rss.xml:50, _default/single.html:46, _default/single.html:47, partials/marketing/jsonLd.html:121, term/term.html:28, term/term.html:30 |
| mapkey | `maps.Params` | `image` | 7267 | `string`×4172, `<invalid>`×3095 | partials/head.html:157, taxonomy/list.html:31, taxonomy/list.html:32, term/term.html:119 |
| mapkey | `maps.Params` | `image_carousel` | 210 | `string`×210 | partials/carousel.html:20, partials/carousel.html:21 |
| mapkey | `maps.Params` | `image_preview` | 9053 | `string`×6066, `<invalid>`×2987 | _default/index.json:7, _default/index.json:13, _default/list.html:32, index.html:45, partials/carousel.html:18, partials/related.html:10, term/term.html:126 |
| mapkey | `maps.Params` | `Ingredients` | 235 | `[]string`×196, `<invalid>`×39 | _default/single.html:153 |
| mapkey | `maps.Params` | `ingredients_percentage` | 363 | `[]interface {}`×256, `<invalid>`×107 | _default/single.html:107, partials/single/ingredientslist.html:19 |
| mapkey | `maps.Params` | `Instagram` | 1479 | `<invalid>`×1474, `string`×5 | term/term.html:73 |
| mapkey | `maps.Params` | `iodine` | 74 | `maps.Params`×62, `<invalid>`×12 | partials/single/nutritionfacts.html:589 |
| mapkey | `maps.Params` | `iron` | 74 | `maps.Params`×73, `<invalid>`×1 | partials/single/nutritionfacts.html:605 |
| mapkey | `maps.Params` | `juicy` | 200 | `<invalid>`×139, `int`×59, `float64`×2 | partials/rating/rating.html:46 |
| mapkey | `maps.Params` | `k` | 79 | `maps.Params`×62, `<invalid>`×17 | partials/single/nutritionfacts.html:475 |
| mapkey | `maps.Params` | `LanguageCode` | 126 | `string`×126 | partials/single/nutritionfacts.html:1 |
| mapkey | `maps.Params` | `LanguageCodeOpenGraph` | 1794 | `string`×1794 | partials/head.html:178, partials/head.html:183 |
| mapkey | `maps.Params` | `Link` | 238 | `string`×238 | partials/head.html:207 |
| mapkey | `maps.Params` | `links` | 3906 | `[]interface {}`×3906 | partials/footer.html:36 |
| mapkey | `maps.Params` | `logo` | 42 | `string`×42 | partials/head.html:152, partials/head.html:153 |
| mapkey | `maps.Params` | `LowPrice` | 229 | `<invalid>`×229 | partials/marketing/jsonLd.html:143 |
| mapkey | `maps.Params` | `magnesium` | 74 | `maps.Params`×62, `<invalid>`×12 | partials/single/nutritionfacts.html:621 |
| mapkey | `maps.Params` | `manganese` | 74 | `maps.Params`×62, `<invalid>`×12 | partials/single/nutritionfacts.html:637 |
| mapkey | `maps.Params` | `manualads` | 3837 | `bool`×3837 | partials/ads/adsensemanual.html:1 |
| mapkey | `maps.Params` | `marketing` | 42 | `maps.Params`×42 | partials/head.html:152, partials/head.html:153 |
| mapkey | `maps.Params` | `Marketing` | 3906 | `maps.Params`×3906 | partials/marketing/google/googleGtag.html:1, partials/marketing/google/googleTagManagerHead.html:1 |
| mapkey | `maps.Params` | `minerals` | 128 | `maps.Params`×76, `<invalid>`×52 | partials/single/nutritionfacts.html:282, partials/single/nutritionfacts.html:492 |
| mapkey | `maps.Params` | `molybdenum` | 74 | `maps.Params`×62, `<invalid>`×12 | partials/single/nutritionfacts.html:653 |
| mapkey | `maps.Params` | `Monetization` | 19254 | `maps.Params`×19254 | partials/ads/adsensehead.html:1, partials/ads/adsensehead.html:2, partials/ads/adsensemanual.html:1, partials/ads/adsensemanual.html:6, partials/ads/adsensemanual.html:7 |
| mapkey | `maps.Params` | `monounsaturated` | 120 | `maps.Params`×62, `<invalid>`×58 | partials/single/nutritionfacts.html:129 |
| mapkey | `maps.Params` | `name` | 1953 | `string`×1953 | partials/head.html:229 |
| mapkey | `maps.Params` | `note_nutrition_facts` | 126 | `<invalid>`×125, `string`×1 | partials/single/nutritionfacts.html:722 |
| mapkey | `maps.Params` | `nutrition_facts` | 361 | `maps.Params`×252, `<invalid>`×109 | _default/single.html:104, partials/single/nutritionfacts.html:2 |
| mapkey | `maps.Params` | `package` | 200 | `int`×198, `<invalid>`×2 | partials/rating/rating.html:58 |
| mapkey | `maps.Params` | `page` | 239 | `int64`×239 | sitemap.xml:18 |
| mapkey | `maps.Params` | `percentage` | 2838 | `<invalid>`×2228, `int`×602, `string`×8 | partials/single/nutritionfacts.html:58, partials/single/nutritionfacts.html:66, partials/single/nutritionfacts.html:72, partials/single/nutritionfacts.html:80, partials/single/nutritionfacts.html:89, partials/single/nutritionfacts.html:113, partials/single/nutritionfacts.html:130, partials/single/nutritionfacts.html:148, partials/single/nutritionfacts.html:156, partials/single/nutritionfacts.html:165, partials/single/nutritionfacts.html:173, partials/single/nutritionfacts.html:182, partials/single/nutritionfacts.html:190, partials/single/nutritionfacts.html:199, partials/single/nutritionfacts.html:207, partials/single/nutritionfacts.html:214, partials/single/nutritionfacts.html:222, partials/single/nutritionfacts.html:241, partials/single/nutritionfacts.html:248, partials/single/nutritionfacts.html:261, partials/single/nutritionfacts.html:269, partials/single/nutritionfacts.html:286, partials/single/nutritionfacts.html:294, partials/single/nutritionfacts.html:302, partials/single/nutritionfacts.html:310, partials/single/nutritionfacts.html:318, partials/single/nutritionfacts.html:326, partials/single/nutritionfacts.html:334, partials/single/nutritionfacts.html:350, partials/single/nutritionfacts.html:366, partials/single/nutritionfacts.html:382, partials/single/nutritionfacts.html:398, partials/single/nutritionfacts.html:414, partials/single/nutritionfacts.html:430, partials/single/nutritionfacts.html:444, partials/single/nutritionfacts.html:460, partials/single/nutritionfacts.html:476, partials/single/nutritionfacts.html:494, partials/single/nutritionfacts.html:510, partials/single/nutritionfacts.html:518, partials/single/nutritionfacts.html:526, partials/single/nutritionfacts.html:542, partials/single/nutritionfacts.html:558, partials/single/nutritionfacts.html:574, partials/single/nutritionfacts.html:590, partials/single/nutritionfacts.html:606, partials/single/nutritionfacts.html:614, partials/single/nutritionfacts.html:622, partials/single/nutritionfacts.html:638, partials/single/nutritionfacts.html:654, partials/single/nutritionfacts.html:670, partials/single/nutritionfacts.html:686, partials/single/nutritionfacts.html:702 |
| mapkey | `maps.Params` | `phosphorus` | 74 | `maps.Params`×62, `<invalid>`×12 | partials/single/nutritionfacts.html:669 |
| mapkey | `maps.Params` | `polyunsaturated` | 120 | `maps.Params`×62, `<invalid>`×58 | partials/single/nutritionfacts.html:112 |
| mapkey | `maps.Params` | `potassium` | 126 | `maps.Params`×68, `<invalid>`×58 | partials/single/nutritionfacts.html:181 |
| mapkey | `maps.Params` | `Priority` | 1716 | `maps.Params`×1716 | sitemap.xml:14, sitemap.xml:16, sitemap.xml:18 |
| mapkey | `maps.Params` | `protein` | 126 | `maps.Params`×117, `<invalid>`×9 | partials/single/nutritionfacts.html:260 |
| mapkey | `maps.Params` | `quantity` | 200 | `int`×198, `<invalid>`×2 | partials/rating/rating.html:55 |
| mapkey | `maps.Params` | `rating` | 235 | `maps.Params`×200, `<invalid>`×35 | _default/single.html:135 |
| mapkey | `maps.Params` | `Rating_tastytaste` | 231 | `<invalid>`×227, `int`×4 | partials/marketing/jsonLd.html:156, partials/marketing/jsonLd.html:157 |
| mapkey | `maps.Params` | `recaptcha` | 1714 | `maps.Params`×1714 | partials/comments.html:139 |
| mapkey | `maps.Params` | `References` | 1479 | `<invalid>`×1474, `[]string`×5 | term/term.html:84 |
| mapkey | `maps.Params` | `salty` | 200 | `<invalid>`×121, `int`×77, `string`×2 | partials/rating/rating.html:31 |
| mapkey | `maps.Params` | `saturated` | 120 | `maps.Params`×96, `<invalid>`×22, `string`×2 | partials/single/nutritionfacts.html:71 |
| mapkey | `maps.Params` | `savory` | 200 | `<invalid>`×147, `int`×53 | partials/rating/rating.html:37 |
| mapkey | `maps.Params` | `Section` | 66 | `float64`×66 | sitemap.xml:14 |
| mapkey | `maps.Params` | `selenium` | 74 | `maps.Params`×62, `<invalid>`×12 | partials/single/nutritionfacts.html:685 |
| mapkey | `maps.Params` | `serving_size` | 126 | `string`×126 | partials/single/nutritionfacts.html:24 |
| mapkey | `maps.Params` | `servings_per_container` | 126 | `int`×105, `float64`×20, `string`×1 | partials/single/nutritionfacts.html:19 |
| mapkey | `maps.Params` | `Showads` | 5790 | `bool`×5790 | partials/ads/adsensehead.html:1, partials/ads/adsensemanual.html:1 |
| mapkey | `maps.Params` | `siteKey` | 1714 | `string`×1714 | partials/comments.html:139 |
| mapkey | `maps.Params` | `Sitemap` | 1716 | `maps.Params`×1716 | sitemap.xml:14, sitemap.xml:16, sitemap.xml:18 |
| mapkey | `maps.Params` | `smell` | 200 | `int`×197, `<invalid>`×3 | partials/rating/rating.html:52 |
| mapkey | `maps.Params` | `smell_review` | 235 | `<invalid>`×231, `string`×4 | _default/single.html:101 |
| mapkey | `maps.Params` | `Social` | 5900 | `maps.Params`×5900 | _default/single.html:138, 404.html:5, index.html:5, partials/head.html:207, partials/head.html:217, partials/head.html:229, term/term.html:107 |
| mapkey | `maps.Params` | `sodium` | 126 | `maps.Params`×120, `<invalid>`×6 | partials/single/nutritionfacts.html:164 |
| mapkey | `maps.Params` | `sour` | 200 | `<invalid>`×127, `int`×71, `string`×2 | partials/rating/rating.html:28 |
| mapkey | `maps.Params` | `spicy` | 200 | `<invalid>`×131, `int`×69 | partials/rating/rating.html:49 |
| mapkey | `maps.Params` | `sugar` | 122 | `maps.Params`×120, `<invalid>`×2 | partials/single/nutritionfacts.html:230 |
| mapkey | `maps.Params` | `sweet` | 200 | `int`×161, `<invalid>`×39 | partials/rating/rating.html:25 |
| mapkey | `maps.Params` | `tags` | 2896 | `<invalid>`×1774, `[]string`×1117, `[]interface {}`×5 | _default/index.json:18, partials/head.html:211, partials/head.html:265, partials/marketing/jsonLd.html:196 |
| mapkey | `maps.Params` | `Tags` | 1714 | `<invalid>`×1488, `[]string`×225, `[]interface {}`×1 | _default/single.html:158, term/term.html:79 |
| mapkey | `maps.Params` | `Target` | 5620 | `string`×5620 | _default/list.html:104, _default/list.html:108, _default/simple.html:51, _default/simple.html:55, _default/single.html:207, _default/single.html:211, _default/single.html:215, 404.html:66, 404.html:70, index.html:220, index.html:224, taxonomy/list.html:116, taxonomy/list.html:120, term/term.html:209, term/term.html:213, term/term.html:217 |
| mapkey | `maps.Params` | `taste_review` | 235 | `<invalid>`×231, `string`×4 | _default/single.html:98 |
| mapkey | `maps.Params` | `tastytaste` | 200 | `int`×197, `float64`×2, `<invalid>`×1 | partials/rating/rating.html:22 |
| mapkey | `maps.Params` | `term` | 1411 | `float64`×1411 | sitemap.xml:16 |
| mapkey | `maps.Params` | `title` | 3915 | `string`×3915 | partials/footer.html:34, taxonomy/list.html:64 |
| mapkey | `maps.Params` | `Title` | 636 | `string`×636 | partials/related.html:43 |
| mapkey | `maps.Params` | `total` | 3734 | `<invalid>`×2106, `int`×1532, `float64`×60, `string`×36 | partials/single/nutritionfacts.html:58, partials/single/nutritionfacts.html:60, partials/single/nutritionfacts.html:72, partials/single/nutritionfacts.html:74, partials/single/nutritionfacts.html:89, partials/single/nutritionfacts.html:113, partials/single/nutritionfacts.html:130, partials/single/nutritionfacts.html:148, partials/single/nutritionfacts.html:150, partials/single/nutritionfacts.html:165, partials/single/nutritionfacts.html:167, partials/single/nutritionfacts.html:182, partials/single/nutritionfacts.html:184, partials/single/nutritionfacts.html:199, partials/single/nutritionfacts.html:201, partials/single/nutritionfacts.html:214, partials/single/nutritionfacts.html:216, partials/single/nutritionfacts.html:231, partials/single/nutritionfacts.html:233, partials/single/nutritionfacts.html:241, partials/single/nutritionfacts.html:243, partials/single/nutritionfacts.html:261, partials/single/nutritionfacts.html:263, partials/single/nutritionfacts.html:286, partials/single/nutritionfacts.html:290, partials/single/nutritionfacts.html:302, partials/single/nutritionfacts.html:306, partials/single/nutritionfacts.html:318, partials/single/nutritionfacts.html:322, partials/single/nutritionfacts.html:334, partials/single/nutritionfacts.html:350, partials/single/nutritionfacts.html:366, partials/single/nutritionfacts.html:382, partials/single/nutritionfacts.html:398, partials/single/nutritionfacts.html:414, partials/single/nutritionfacts.html:430, partials/single/nutritionfacts.html:444, partials/single/nutritionfacts.html:460, partials/single/nutritionfacts.html:476, partials/single/nutritionfacts.html:494, partials/single/nutritionfacts.html:510, partials/single/nutritionfacts.html:514, partials/single/nutritionfacts.html:526, partials/single/nutritionfacts.html:542, partials/single/nutritionfacts.html:558, partials/single/nutritionfacts.html:574, partials/single/nutritionfacts.html:590, partials/single/nutritionfacts.html:606, partials/single/nutritionfacts.html:610, partials/single/nutritionfacts.html:622, partials/single/nutritionfacts.html:638, partials/single/nutritionfacts.html:654, partials/single/nutritionfacts.html:670, partials/single/nutritionfacts.html:686, partials/single/nutritionfacts.html:702 |
| mapkey | `maps.Params` | `TotalShow` | 21 | `int64`×21 | partials/carousel.html:2 |
| mapkey | `maps.Params` | `trans` | 120 | `maps.Params`×69, `<invalid>`×51 | partials/single/nutritionfacts.html:88 |
| mapkey | `maps.Params` | `twitter` | 3948 | `maps.Params`×1995, `<invalid>`×1953 | partials/head.html:152, partials/head.html:153, partials/head.html:221, partials/head.html:229 |
| mapkey | `maps.Params` | `Twitter` | 1479 | `<invalid>`×1474, `string`×5 | term/term.html:70 |
| mapkey | `maps.Params` | `Type` | 239 | `string`×231, `<invalid>`×8 | index.html:38 |
| mapkey | `maps.Params` | `Typescript` | 5620 | `maps.Params`×5620 | _default/list.html:104, _default/list.html:108, _default/simple.html:51, _default/simple.html:55, _default/single.html:207, _default/single.html:211, _default/single.html:215, 404.html:66, 404.html:70, index.html:220, index.html:224, taxonomy/list.html:116, taxonomy/list.html:120, term/term.html:209, term/term.html:213, term/term.html:217 |
| mapkey | `maps.Params` | `vitamins` | 207 | `maps.Params`×158, `<invalid>`×49 | partials/single/nutritionfacts.html:282, partials/single/nutritionfacts.html:284 |
| mapkey | `maps.Params` | `Website` | 1479 | `<invalid>`×1472, `string`×7 | term/term.html:64 |
| mapkey | `maps.Params` | `when_seen` | 235 | `string`×171, `<invalid>`×64 | _default/single.html:91 |
| mapkey | `maps.Params` | `yearCreate` | 6888 | `string`×6888 | _default/rss.xml:27, _default/rss.xml:30, partials/footer.html:76, partials/footer.html:79 |
| mapkey | `maps.Params` | `Youtube` | 1653 | `<invalid>`×1474, `string`×179 | partials/marketing/jsonLd.html:198, partials/marketing/jsonLd.html:201, term/term.html:76 |
| mapkey | `maps.Params` | `youtube_video` | 2678 | `<invalid>`×2152, `string`×526 | _default/single.html:114, partials/head.html:190, partials/marketing/jsonLd.html:198, partials/marketing/jsonLd.html:201, partials/marketing/jsonLd.html:242, partials/marketing/jsonLd.html:243 |
| mapkey | `maps.Params` | `zinc` | 74 | `maps.Params`×62, `<invalid>`×12 | partials/single/nutritionfacts.html:701 |
| mapkey | `page.Data` | `Singular` | 14075 | `string`×12771, `<invalid>`×1304 | partials/breadcrumb.html:18, partials/breadcrumb.html:23, partials/breadcrumb.html:28, partials/breadcrumb.html:33, partials/breadcrumb.html:56, partials/breadcrumb.html:63, partials/breadcrumb.html:70, partials/breadcrumb.html:77, term/term.html:113 |

### A.4 `range` sites and the dynamic type ranged over

| site | pipeline | type ×count |
|---|---|---|
| EMB:_markup/render-table.html:2 | `$k, $v := .Attributes` | `map[string]interface {} (decl=2)`×238 |
| EMB:_markup/render-table.html:8 | `.THead` | `[]hooks.TableRow (decl=0)`×238 |
| EMB:_markup/render-table.html:10 | `.` | `hooks.TableRow (decl=0)`×238 |
| EMB:_markup/render-table.html:23 | `.TBody` | `[]hooks.TableRow (decl=0)`×238 |
| EMB:_markup/render-table.html:25 | `.` | `hooks.TableRow (decl=0)`×1000 |
| EMB:sitemapindex.xml:3 | `.` | `[]*hugolib.Site (decl=0)`×1 |
| _default/index.json:2 | `(sort .Site.AllPages "Date").Reverse` | `page.Pages (decl=0)`×2 |
| _default/latesturl.html:1 | `(sort (where .Site.RegularPages "Params.type" "snacks") "Date").Reverse` | `page.Pages (decl=0)`×1 |
| _default/list.html:25 | `$index, $page := sort (.Paginator.Pages) "Title"` | `page.Pages (decl=2)`×71 |
| _default/rss.xml:43 | `$pages` | `page.Pages (decl=0)`×1491 |
| index.html:37 | `$index, $el := (first $totalPostsToShow $paginator.Pages)` | `page.Pages (decl=2)`×21 |
| index.html:98 | `$taxonomy_term, $taxonomy := .Site.Taxonomies` | `page.TaxonomyList (decl=2)`×21 |
| index.html:128 | `$key, $value := $taxonomy` | `page.Taxonomy (decl=2)`×126 |
| partials/carousel.html:6 | `seq $totalShow` | `[]int (decl=0)`×21 |
| partials/carousel.html:11 | `$index, $el := (first $totalShow $paginator)` | `page.Pages (decl=2)`×21 |
| partials/comments.html:182 | `$parentKey, $parent := .Site.Data.comments` | `map[string]interface {} (decl=2)`×1714 |
| partials/comments.html:183 | `$childKey, $child := .` | `map[string]interface {} (decl=2)`×5142 |
| partials/comments.html:194 | `$comments` | `map[string]interface {} (decl=0)`×2 |
| partials/comments.html:289 | `$comments` | `map[string]interface {} (decl=0)`×2 |
| partials/comments.html:398 | `$i := (seq 5)` | `[]int (decl=1)`×4 |
| partials/footer.html:31 | `$quickLinks` | `maps.Params (decl=0)`×1953 |
| partials/footer.html:36 | `.links` | `[]interface {} (decl=0)`×3906 |
| partials/footer.html:48 | `$socialMedia` | `[]interface {} (decl=0)`×1953 |
| partials/head.html:121 | `.AlternativeOutputFormats` | `page.OutputFormats (decl=0)`×1953 |
| partials/head.html:126 | `.AllTranslations` | `page.Pages (decl=0)`×897 |
| partials/head.html:133 | `first 1 .AllTranslations` | `page.Pages (decl=0)`×897 |
| partials/head.html:171 | `.Page.AllTranslations` | `page.Pages (decl=0)`×897 |
| partials/head.html:173 | `$siteLanguages` | `langs.Languages (decl=0)`×1794 |
| partials/head.html:196 | `.` | `page.Pages (decl=0)`×184 |
| partials/head.html:213 | `$sorted` | `[]string (decl=0)`×221, `[]interface {} (decl=0)`×1 |
| partials/head.html:267 | `$sorted` | `[]string (decl=0)`×224, `[]interface {} (decl=0)`×1 |
| partials/header.html:71 | `$socialMedia` | `[]interface {} (decl=0)`×1953 |
| partials/header.html:109 | `.Page.AllTranslations` | `page.Pages (decl=0)`×897 |
| partials/header.html:111 | `$siteLanguages` | `langs.Languages (decl=0)`×1794 |
| partials/marketing/jsonLd.html:21 | `$i, $e := $socialMedia` | `[]interface {} (decl=2)`×229 |
| partials/marketing/jsonLd.html:40 | `$i, $e := $socialMedia` | `[]interface {} (decl=2)`×229 |
| partials/marketing/jsonLd.html:74 | `$i, $e := $socialMedia` | `[]interface {} (decl=2)`×229 |
| partials/marketing/jsonLd.html:93 | `$i, $e := $socialMedia` | `[]interface {} (decl=2)`×229 |
| partials/marketing/jsonLd.html:174 | `$i, $e := $socialMedia` | `[]interface {} (decl=2)`×229 |
| partials/marketing/jsonLd.html:196 | `$i, $e := .Params.tags` | `[]string (decl=2)`×222, `<invalid> (decl=2)`×6, `[]interface {} (decl=2)`×1 |
| partials/marketing/jsonLd.html:221 | `$i, $e := $socialMedia` | `[]interface {} (decl=2)`×87 |
| partials/marketing/jsonLd.html:280 | `$i, $e := $socialMedia` | `[]interface {} (decl=2)`×1724 |
| partials/marketing/jsonLd.html:299 | `$i, $e := $socialMedia` | `[]interface {} (decl=2)`×1724 |
| partials/marketing/jsonLd.html:332 | `$i, $e := $socialMedia` | `[]interface {} (decl=2)`×1724 |
| partials/marketing/jsonLd.html:351 | `$i, $e := $socialMedia` | `[]interface {} (decl=2)`×1724 |
| partials/marketing/jsonLd.html:385 | `slice 1 2 3 4 5 6` | `[]int (decl=0)`×1953 |
| partials/marketing/jsonLd.html:396 | `$scratch.Get "reversed"` | `page.Pages (decl=0)`×1953 |
| partials/marketing/jsonLd.html:403 | `sort ($scratch.Get "pages") "position" "asc"` | `[]map[string]interface {} (decl=0)`×1953 |
| partials/pagination.html:61 | `$pag.Pagers` | `page.pagers (decl=0)`×270 |
| partials/rating/rating.html:71 | `$i := (seq 5)` | `[]int (decl=1)`×1152 |
| partials/related.html:5 | `.` | `page.Pages (decl=0)`×184 |
| partials/single/ingredientslist.html:19 | `$ingredients := .context.Params.ingredients_percentage` | `[]interface {} (decl=1)`×128 |
| partials/taxonomy/categories.html:8 | `$val := $sorted` | `[]string (decl=1)`×220, `[]interface {} (decl=1)`×1 |
| partials/taxonomy/companies.html:11 | `$val := $sorted` | `[]string (decl=1)`×3 |
| partials/taxonomy/countries.html:11 | `$val := $sorted` | `[]string (decl=1)`×155 |
| partials/taxonomy/ingredients.html:9 | `$val := $sorted` | `[]string (decl=1)`×195 |
| partials/taxonomy/tags.html:10 | `$val := $sorted` | `[]string (decl=1)`×197, `[]interface {} (decl=1)`×1 |
| sitemap.xml:4 | `.Data.Pages` | `page.Pages (decl=0)`×2 |
| sitemap.xml:22 | `.Translations` | `page.Pages (decl=0)`×716 |
| taxonomy/list.html:30 | `$index, $page := sort (.Paginator.Pages) "Title"` | `page.Pages (decl=2)`×123 |
| term/term.html:98 | `$i, $e := $sorted` | `[]string (decl=2)`×5 |
| term/term.html:118 | `$index, $page := sort (.Paginator.Pages) "Title"` | `page.Pages (decl=2)`×1479 |

### A.5 `if` / `with` sites: condition value type = truth ×count

| kind | site | condition | type=truth ×count |
|---|---|---|---|
| if | EMB:_markup/render-link.html:3 | `strings.HasPrefix $u.String "#"` | `bool=false`×558 |
| if | EMB:_markup/render-link.html:5 | `and $href (not $u.IsAbs)` | `bool=false`×555, `bool=true`×3 |
| with | EMB:_markup/render-link.html:7 | `or ($.PageInner.GetPage $path) ($.PageInner.Resources.Get $path) (resources.Get $path)` | `nil(resource.Resource)=false`×3 |
| with | EMB:_markup/render-link.html:21 | `.Title` | `string=false`×498, `string=true`×60 |
| with | EMB:_markup/render-table.html:12 | `.Alignment` | `string=false`×622 |
| with | EMB:_markup/render-table.html:27 | `.Alignment` | `string=false`×2552 |
| if | EMB:sitemapindex.xml:6 | `not .Lastmod.IsZero` | `bool=true`×2 |
| if | _default/_markup/render-image.html:8 | `or (eq (path.Ext $imageFile) ".jpg") (eq (path.Ext $imageFile) ".png")` | `bool=true`×1050 |
| if | _default/_markup/render-image.html:12 | `eq (path.Ext $imageFile) ".jpg"` | `bool=true`×1050 |
| with | _default/baseof.html:4 | `.Site.LanguageCode` | `string=true`×1953 |
| if | _default/baseof.html:12 | `.IsHome` | `bool=false`×218, `bool=true`×21 |
| with | _default/index.json:7 | `.Page.Resources.Get .Params.image_preview` | `nil(resource.Resource)=false`×2984, `*resources.resourceAdapter=true`×476 |
| if | _default/list.html:5 | `eq hugo.Environment "development"` | `bool=false`×71 |
| if | _default/list.html:33 | `or (eq (path.Ext $imageFile) ".jpg") (eq (path.Ext $imageFile) ".png")` | `bool=true`×232, `bool=false`×2 |
| if | _default/list.html:45 | `eq (path.Ext $imageFile) ".jpg"` | `bool=true`×232 |
| if | _default/list.html:80 | `eq hugo.Environment "development"` | `bool=false`×71 |
| if | _default/rss.xml:2 | `.IsHome` | `bool=false`×1489, `bool=true`×2 |
| if | _default/rss.xml:4 | `or $.IsHome $.IsSection` | `bool=false`×1423, `bool=true`×68 |
| if | _default/rss.xml:10 | `ge $limit 1` | `bool=true`×1491 |
| if | _default/rss.xml:19 | `eq .Title .Site.Title` | `bool=false`×1490, `bool=true`×1 |
| with | _default/rss.xml:19 | `.Title` | `string=true`×1488, `string=false`×2 |
| if | _default/rss.xml:23 | `ne .Title .Site.Title` | `bool=true`×1490, `bool=false`×1 |
| with | _default/rss.xml:23 | `.Title` | `string=true`×1488, `string=false`×2 |
| with | _default/rss.xml:23 | `.Site.Params.authorEmail` | `string=true`×1491 |
| with | _default/rss.xml:24 | `.Site.Params.authorEmail` | `string=true`×1491 |
| with | _default/rss.xml:24 | `$.Site.Params.author` | `string=true`×1491 |
| with | _default/rss.xml:25 | `$.Site.Params.author` | `string=true`×1491 |
| if | _default/rss.xml:26 | `not .Site.Params.hideCopyright` | `bool=true`×1491 |
| if | _default/rss.xml:27 | `ge .Site.Params.yearCreate (now.Format "2006")` | `bool=false`×1491 |
| if | _default/rss.xml:39 | `not .Date.IsZero` | `bool=true`×1489, `bool=false`×2 |
| with | _default/rss.xml:40 | `.OutputFormats.Get "RSS"` | `*page.OutputFormat=true`×1491 |
| with | _default/rss.xml:47 | `.Params.categories` | `[]string=true`×3491, `<invalid>=false`×39, `[]string=false`×14, `string=true`×13, `[]interface {}=true`×6 |
| if | _default/rss.xml:50 | `.Params.Image` | `string=true`×3563 |
| if | _default/simple.html:5 | `eq hugo.Environment "development"` | `bool=false`×3 |
| if | _default/simple.html:27 | `eq hugo.Environment "development"` | `bool=false`×3 |
| if | _default/single.html:6 | `eq hugo.Environment "development"` | `bool=false`×235 |
| with | _default/single.html:33 | `.Params.Author` | `string=true`×213, `<invalid>=false`×22 |
| if | _default/single.html:38 | `.Params.date` | `time.Time=true`×231, `<invalid>=false`×4 |
| if | _default/single.html:39 | `.Params.Author` | `string=true`×213, `<invalid>=false`×18 |
| if | _default/single.html:46 | `.Params.Image` | `string=true`×232, `<invalid>=false`×3 |
| if | _default/single.html:49 | `or (eq (path.Ext $imageFile) ".jpg") (eq (path.Ext $imageFile) ".png")` | `bool=true`×232 |
| if | _default/single.html:61 | `eq (path.Ext $imageFile) ".jpg"` | `bool=true`×232 |
| with | _default/single.html:82 | `.Params.Companies` | `string=true`×223, `<invalid>=false`×9, `[]string=true`×3 |
| with | _default/single.html:85 | `.Params.Brands` | `string=true`×202, `<invalid>=false`×32, `string=false`×1 |
| with | _default/single.html:88 | `.Params.Countries` | `[]string=true`×155, `<invalid>=false`×79, `[]string=false`×1 |
| with | _default/single.html:91 | `.Params.when_seen` | `string=false`×132, `<invalid>=false`×64, `string=true`×39 |
| with | _default/single.html:98 | `.Params.taste_review` | `<invalid>=false`×231, `string=true`×3, `string=false`×1 |
| with | _default/single.html:101 | `.Params.smell_review` | `<invalid>=false`×231, `string=true`×3, `string=false`×1 |
| if | _default/single.html:104 | `.Params.nutrition_facts` | `maps.Params=true`×126, `<invalid>=false`×109 |
| if | _default/single.html:107 | `.Params.ingredients_percentage` | `[]interface {}=true`×128, `<invalid>=false`×107 |
| with | _default/single.html:114 | `.Params.youtube_video` | `<invalid>=false`×146, `string=true`×89 |
| with | _default/single.html:135 | `.Params.rating` | `maps.Params=true`×200, `<invalid>=false`×35 |
| if | _default/single.html:138 | `or .Site.Params.Social.Facebook.Enable .Site.Params.Marketing.Twitter.Enable` | `bool=true`×235 |
| with | _default/single.html:148 | `.Params.Categories` | `[]string=true`×220, `<invalid>=false`×10, `string=true`×3, `[]interface {}=true`×1, `[]string=false`×1 |
| with | _default/single.html:153 | `.Params.Ingredients` | `[]string=true`×195, `<invalid>=false`×39, `[]string=false`×1 |
| with | _default/single.html:158 | `.Params.Tags` | `[]string=true`×221, `<invalid>=false`×12, `[]interface {}=true`×1, `[]string=false`×1 |
| if | _default/single.html:174 | `eq hugo.Environment "development"` | `bool=false`×235 |
| with | 404.html:5 | `.Site.Params.Social.Facebook.Appid` | `string=true`×21 |
| if | 404.html:15 | `eq hugo.Environment "development"` | `bool=false`×21 |
| if | 404.html:42 | `eq hugo.Environment "development"` | `bool=false`×21 |
| with | index.html:5 | `.Site.Params.Social.Facebook.Appid` | `string=true`×21 |
| if | index.html:15 | `eq hugo.Environment "development"` | `bool=false`×21 |
| if | index.html:38 | `eq .Params.Type "snacks"` | `bool=true`×229, `bool=false`×10 |
| if | index.html:46 | `or (eq (path.Ext $imageFile) ".jpg") (eq (path.Ext $imageFile) ".png")` | `bool=true`×228, `bool=false`×1 |
| if | index.html:58 | `eq (path.Ext $imageFile) ".jpg"` | `bool=true`×228 |
| with | index.html:99 | `$.Site.GetPage (printf "/%s" $taxonomy_term)` | `*hugolib.pageState=true`×126 |
| if | index.html:109 | `eq $taxonomy_term "brands"` | `bool=false`×105, `bool=true`×21 |
| if | index.html:111 | `eq $taxonomy_term "categories"` | `bool=false`×84, `bool=true`×21 |
| if | index.html:113 | `eq $taxonomy_term "companies"` | `bool=false`×63, `bool=true`×21 |
| if | index.html:115 | `eq $taxonomy_term "countries"` | `bool=false`×42, `bool=true`×21 |
| if | index.html:117 | `eq $taxonomy_term "ingredients"` | `bool=false`×21, `bool=true`×21 |
| if | index.html:119 | `eq $taxonomy_term "tags"` | `bool=true`×21 |
| if | index.html:129 | `$key` | `string=true`×15568 |
| with | index.html:130 | `$.Site.GetPage (printf "/%s/%s" $taxonomy_term $key)` | `*hugolib.pageState=true`×15568 |
| if | index.html:136 | `eq $taxonomy_term "categories"` | `bool=false`×14483, `bool=true`×1085 |
| if | index.html:196 | `eq hugo.Environment "development"` | `bool=false`×21 |
| if | partials/ads/adsensehead.html:1 | `and .Site.Params.Monetization.Ads.Showads` | `bool=true`×1953 |
| if | partials/ads/adsensemanual.html:1 | `and .Site.Params.Monetization.Ads.Showads .Site.Params.Monetization.Ads.manualads` | `bool=true`×3837 |
| if | partials/breadcrumb.html:7 | `.p1.Parent` | `*hugolib.pageState=true`×3621, `nil(page.Page)=false`×1929 |
| if | partials/breadcrumb.html:9 | `not .p1.IsHome` | `bool=false`×1929 |
| if | partials/breadcrumb.html:12 | `eq .p1 .p2` | `bool=false`×3621, `bool=true`×1929 |
| if | partials/breadcrumb.html:13 | `.p1.IsPage` | `bool=false`×1694, `bool=true`×235 |
| if | partials/breadcrumb.html:18 | `and (eq .p1.Data.Singular "company") (or (eq .p2.Kind "term") (eq .p2.Kind "taxonomy"))` | `bool=false`×1591, `bool=true`×103 |
| if | partials/breadcrumb.html:23 | `and (eq .p1.Data.Singular "brand") (or (eq .p2.Kind "term") (eq .p2.Kind "taxonomy"))` | `bool=false`×1488, `bool=true`×103 |
| if | partials/breadcrumb.html:28 | `and (eq .p1.Data.Singular "country") (or (eq .p2.Kind "term") (eq .p2.Kind "taxonomy"))` | `bool=false`×1465, `bool=true`×23 |
| if | partials/breadcrumb.html:33 | `and (eq .p1.Data.Singular "ingredient") (or (eq .p2.Kind "term") (eq .p2.Kind "taxonomy"))` | `bool=true`×822, `bool=false`×643 |
| if | partials/breadcrumb.html:38 | `.p1.IsHome` | `bool=false`×622, `bool=true`×21 |
| if | partials/breadcrumb.html:49 | `.p1.IsHome` | `bool=true`×1908, `bool=false`×1713 |
| if | partials/breadcrumb.html:56 | `and (eq .p1.Data.Singular "company") (or (eq .p1.Kind "term") (eq .p1.Kind "taxonomy"))` | `bool=false`×1618, `bool=true`×95 |
| if | partials/breadcrumb.html:63 | `and (eq .p1.Data.Singular "brand") (or (eq .p1.Kind "term") (eq .p1.Kind "taxonomy"))` | `bool=false`×1524, `bool=true`×94 |
| if | partials/breadcrumb.html:70 | `and (eq .p1.Data.Singular "country") (or (eq .p1.Kind "term") (eq .p1.Kind "taxonomy"))` | `bool=false`×1503, `bool=true`×21 |
| if | partials/breadcrumb.html:77 | `and (eq .p1.Data.Singular "ingredient") (or (eq .p1.Kind "term") (eq .p1.Kind "taxonomy"))` | `bool=true`×762, `bool=false`×741 |
| if | partials/carousel.html:12 | `eq $index 0` | `bool=false`×84, `bool=true`×21 |
| if | partials/carousel.html:19 | `or (eq (path.Ext $imageFile) ".jpg") (eq (path.Ext $imageFile) ".png")` | `bool=true`×105 |
| if | partials/carousel.html:20 | `.Params.image_carousel` | `string=true`×105 |
| with | partials/carousel.html:23 | `$imageFile` | `*resources.resourceAdapter=true`×105 |
| if | partials/carousel.html:35 | `eq (path.Ext $imageFile) ".jpg"` | `bool=true`×105 |
| if | partials/comments.html:1 | `.Site.Params.Comment.enabled` | `bool=true`×1714 |
| with | partials/comments.html:97 | `.File` | `*source.File=false`×1468, `*source.File=true`×246 |
| if | partials/comments.html:139 | `and .Site.Params.Comment.recaptcha.siteKey .Site.Params.Comment.recaptcha.encryptedKey` | `string=false`×1714 |
| if | partials/comments.html:178 | `.Site.Data.comments` | `map[string]interface {}=true`×1714 |
| if | partials/comments.html:185 | `eq $page $slug` | `bool=false`×5140, `bool=true`×2 |
| if | partials/comments.html:193 | `$comments` | `<invalid>=false`×1712, `map[string]interface {}=true`×2 |
| if | partials/comments.html:195 | `not .replyThread` | `bool=true`×2 |
| if | partials/comments.html:229 | `or .reviewTastyTaste .reviewSmell .reviewQuantity .reviewPackage` | `<invalid>=false`×1, `float64=true`×1 |
| with | partials/comments.html:233 | `.reviewTastyTaste` | `float64=true`×1 |
| with | partials/comments.html:236 | `.reviewSmell` | `float64=true`×1 |
| with | partials/comments.html:239 | `.reviewQuantity` | `float64=true`×1 |
| with | partials/comments.html:242 | `.reviewPackage` | `float64=true`×1 |
| with | partials/comments.html:247 | `.reviewSweet` | `<invalid>=false`×1 |
| with | partials/comments.html:250 | `.reviewSour` | `<invalid>=false`×1 |
| with | partials/comments.html:253 | `.reviewSalty` | `<invalid>=false`×1 |
| with | partials/comments.html:256 | `.reviewBitter` | `<invalid>=false`×1 |
| with | partials/comments.html:259 | `.reviewSavory` | `<invalid>=false`×1 |
| with | partials/comments.html:262 | `.reviewCrispy` | `<invalid>=false`×1 |
| with | partials/comments.html:265 | `.reviewChewy` | `<invalid>=false`×1 |
| with | partials/comments.html:268 | `.reviewJuicy` | `<invalid>=false`×1 |
| with | partials/comments.html:271 | `.reviewSpicy` | `<invalid>=false`×1 |
| if | partials/comments.html:290 | `eq .replyThread ($.Scratch.Get "threadID")` | `bool=false`×2 |
| if | partials/comments.html:399 | `ge $ratingnumber $i` | `bool=true`×20 |
| if | partials/footer.html:10 | `or (eq (path.Ext $imageFile) ".jpg") (eq (path.Ext $imageFile) ".png")` | `bool=true`×1953 |
| if | partials/footer.html:15 | `eq (path.Ext $imageFile) ".jpg"` | `bool=false`×1953 |
| if | partials/footer.html:20 | `eq (path.Ext $imageFile) ".png"` | `bool=true`×1953 |
| if | partials/footer.html:74 | `not .Site.Params.hideCopyright` | `bool=true`×1953 |
| if | partials/footer.html:76 | `ge .Site.Params.yearCreate (now.Format "2006")` | `bool=false`×1953 |
| if | partials/head.html:2 | `or .IsHome .IsNode` | `bool=true`×1715, `bool=false`×238 |
| if | partials/head.html:15 | `.IsHome` | `bool=false`×1932, `bool=true`×21 |
| if | partials/head.html:125 | `.IsTranslated` | `bool=false`×1056, `bool=true`×897 |
| with | partials/head.html:140 | `$pag` | `*page.Pager=true`×1715, `string=false`×238 |
| if | partials/head.html:141 | `and (or $.IsHome $.IsNode) (ne .PageNumber 1)` | `bool=false`×1493, `bool=true`×222 |
| if | partials/head.html:151 | `.IsHome` | `bool=false`×1932, `bool=true`×21 |
| if | partials/head.html:152 | `site.Params.marketing.twitter.logo` | `string=true`×21 |
| with | partials/head.html:165 | `$og_image` | `string=true`×1953 |
| if | partials/head.html:168 | `.IsTranslated` | `bool=false`×1056, `bool=true`×897 |
| if | partials/head.html:174 | `eq $translation.Lang .Lang` | `bool=false`×1794, `bool=true`×1794 |
| if | partials/head.html:175 | `eq $pageLang .Lang` | `bool=false`×897, `bool=true`×897 |
| with | partials/head.html:190 | `and .Params.youtube_video` | `<invalid>=false`×1864, `string=true`×89 |
| with | partials/head.html:195 | `$related` | `page.Pages=false`×1769, `page.Pages=true`×184 |
| if | partials/head.html:201 | `.IsPage` | `bool=false`×1715, `bool=true`×238 |
| with | partials/head.html:202 | `.Params.Author` | `string=true`×213, `<invalid>=false`×25 |
| with | partials/head.html:207 | `.Site.Params.Social.Facebook.Link` | `string=true`×238 |
| with | partials/head.html:211 | `.Params.tags` | `[]string=true`×221, `<invalid>=false`×15, `[]interface {}=true`×1, `[]string=false`×1 |
| with | partials/head.html:217 | `.Site.Params.Social.Facebook.Appid` | `string=true`×1953 |
| with | partials/head.html:221 | `site.Params.twitter` | `<invalid>=false`×1953 |
| with | partials/head.html:226 | `$og_image` | `string=true`×1953 |
| with | partials/head.html:229 | `.Site.Params.Social.twitter.name` | `string=true`×1953 |
| with | partials/head.html:232 | `.Params.Author` | `<invalid>=false`×1736, `string=true`×217 |
| if | partials/head.html:238 | `.IsPage` | `bool=false`×1715, `bool=true`×238 |
| if | partials/head.html:239 | `not .PublishDate.IsZero` | `bool=true`×231, `bool=false`×7 |
| if | partials/head.html:244 | `not .Date.IsZero` | `bool=false`×7 |
| if | partials/head.html:250 | `not .Lastmod.IsZero` | `bool=true`×231, `bool=false`×7 |
| if | partials/head.html:257 | `not .Date.IsZero` | `bool=true`×1692, `bool=false`×23 |
| with | partials/head.html:264 | `.Site.Params.author` | `string=true`×1953 |
| with | partials/head.html:265 | `.Params.tags` | `<invalid>=false`×1727, `[]string=true`×224, `[]interface {}=true`×1, `[]string=false`×1 |
| if | partials/head.html:269 | `eq hugo.Environment "development"` | `bool=false`×1953 |
| with | partials/head.html:276 | `$styles := resources.Get "scss/website.scss" \| toCSS $cssOpts \| postCSS \| minify \| fin` | `*postpub.PostPublishResource=true`×1953 |
| with | partials/head.html:282 | `.Site.Params.DynamicContent` | `bool=false`×1953 |
| if | partials/header.html:12 | `or (eq (path.Ext $imageFile) ".jpg") (eq (path.Ext $imageFile) ".png")` | `bool=true`×1953 |
| if | partials/header.html:16 | `eq (path.Ext $imageFile) ".jpg"` | `bool=false`×1953 |
| if | partials/header.html:21 | `eq (path.Ext $imageFile) ".png"` | `bool=true`×1953 |
| if | partials/header.html:72 | `or (eq .name "facebook") (eq .name "instagram") (eq .name "twitter") (eq .name "youtube") ` | `bool=true`×9765, `bool=false`×5859 |
| if | partials/header.html:95 | `.IsTranslated` | `bool=false`×1056, `bool=true`×897 |
| if | partials/header.html:112 | `eq $translation.Lang .Lang` | `bool=false`×1794, `bool=true`×1794 |
| if | partials/header.html:114 | `eq $pageLang .Lang` | `bool=false`×897, `bool=true`×897 |
| with | partials/marketing/google/googleGtag.html:1 | `.Site.Params.Marketing.Google.GoogleAnalyticsGtag` | `string=true`×1953 |
| with | partials/marketing/google/googleTagManagerHead.html:1 | `.Site.Params.Marketing.Google.GoogleTagManager` | `string=true`×1953 |
| if | partials/marketing/jsonLd.html:5 | `and .IsPage (eq .Type "snacks")` | `bool=false`×1724, `bool=true`×229 |
| if | partials/marketing/jsonLd.html:22 | `eq $e.type "social"` | `bool=true`×1603, `bool=false`×229 |
| if | partials/marketing/jsonLd.html:23 | `gt $i 0` | `bool=true`×1374, `bool=false`×229 |
| if | partials/marketing/jsonLd.html:41 | `eq $e.type "social"` | `bool=true`×1603, `bool=false`×229 |
| if | partials/marketing/jsonLd.html:42 | `gt $i 0` | `bool=true`×1374, `bool=false`×229 |
| if | partials/marketing/jsonLd.html:75 | `eq $e.type "social"` | `bool=true`×1603, `bool=false`×229 |
| if | partials/marketing/jsonLd.html:76 | `gt $i 0` | `bool=true`×1374, `bool=false`×229 |
| if | partials/marketing/jsonLd.html:94 | `eq $e.type "social"` | `bool=true`×1603, `bool=false`×229 |
| if | partials/marketing/jsonLd.html:95 | `gt $i 0` | `bool=true`×1374, `bool=false`×229 |
| with | partials/marketing/jsonLd.html:134 | `.Params.Brands` | `string=true`×200, `<invalid>=false`×28, `string=false`×1 |
| with | partials/marketing/jsonLd.html:140 | `.Params.Categories` | `[]string=true`×220, `<invalid>=false`×4, `string=true`×3, `[]interface {}=true`×1, `[]string=false`×1 |
| if | partials/marketing/jsonLd.html:143 | `and .Params.LowPrice .Params.HighPrice` | `<invalid>=false`×229 |
| if | partials/marketing/jsonLd.html:156 | `.Params.Rating_tastytaste` | `<invalid>=false`×227, `int=true`×2 |
| if | partials/marketing/jsonLd.html:175 | `eq $e.type "social"` | `bool=true`×1603, `bool=false`×229 |
| if | partials/marketing/jsonLd.html:176 | `gt $i 0` | `bool=true`×1374, `bool=false`×229 |
| with | partials/marketing/jsonLd.html:195 | `.Params.Companies` | `string=true`×221, `<invalid>=false`×5, `[]string=true`×3 |
| if | partials/marketing/jsonLd.html:196 | `$i` | `int=true`×576, `int=false`×222 |
| if | partials/marketing/jsonLd.html:198 | `and .Params.youtube_video .Site.Params.Api.Youtube` | `<invalid>=false`×142, `string=true`×87 |
| if | partials/marketing/jsonLd.html:203 | `$ytResource` | `*resources.resourceAdapter=true`×87 |
| with | partials/marketing/jsonLd.html:205 | `$ytData.items` | `[]interface {}=true`×87 |
| if | partials/marketing/jsonLd.html:211 | `$ytItem.snippet.thumbnails.standard` | `map[string]interface {}=true`×87 |
| if | partials/marketing/jsonLd.html:222 | `eq $e.type "social"` | `bool=true`×609, `bool=false`×87 |
| if | partials/marketing/jsonLd.html:223 | `gt $i 0` | `bool=true`×522, `bool=false`×87 |
| if | partials/marketing/jsonLd.html:263 | `.IsHome` | `bool=false`×1703, `bool=true`×21 |
| if | partials/marketing/jsonLd.html:271 | `.IsHome` | `bool=false`×1703, `bool=true`×21 |
| if | partials/marketing/jsonLd.html:281 | `eq $e.type "social"` | `bool=true`×12068, `bool=false`×1724 |
| if | partials/marketing/jsonLd.html:282 | `gt $i 0` | `bool=true`×10344, `bool=false`×1724 |
| if | partials/marketing/jsonLd.html:300 | `eq $e.type "social"` | `bool=true`×12068, `bool=false`×1724 |
| if | partials/marketing/jsonLd.html:301 | `gt $i 0` | `bool=true`×10344, `bool=false`×1724 |
| if | partials/marketing/jsonLd.html:323 | `.IsHome` | `bool=false`×1703, `bool=true`×21 |
| if | partials/marketing/jsonLd.html:333 | `eq $e.type "social"` | `bool=true`×12068, `bool=false`×1724 |
| if | partials/marketing/jsonLd.html:334 | `gt $i 0` | `bool=true`×10344, `bool=false`×1724 |
| if | partials/marketing/jsonLd.html:352 | `eq $e.type "social"` | `bool=true`×12068, `bool=false`×1724 |
| if | partials/marketing/jsonLd.html:353 | `gt $i 0` | `bool=true`×10344, `bool=false`×1724 |
| if | partials/marketing/jsonLd.html:387 | `($scratch.Get "current")` | `<invalid>=false`×6120, `*hugolib.pageState=true`×5598 |
| if | partials/marketing/jsonLd.html:404 | `ne .position 1` | `bool=true`×3645, `bool=false`×1953 |
| if | partials/marketing/jsonLd.html:406 | `not .page.IsHome` | `bool=true`×3645, `bool=false`×1953 |
| with | partials/pagination.html:2 | `.Site.Params.DynamicContent` | `bool=false`×1694 |
| if | partials/pagination.html:20 | `gt $pag.TotalPages 1` | `bool=false`×1424, `bool=true`×270 |
| with | partials/pagination.html:21 | `.Site.Params.DynamicContent` | `bool=false`×270 |
| with | partials/pagination.html:26 | `$pag.First` | `*page.Pager=true`×270 |
| if | partials/pagination.html:27 | `$pag.HasPrev` | `bool=true`×203, `bool=false`×67 |
| if | partials/pagination.html:65 | `$showNumber` | `bool=false`×1990, `bool=true`×1178 |
| if | partials/pagination.html:72 | `$showNumber` | `bool=false`×1990, `bool=true`×1178 |
| if | partials/pagination.html:73 | `eq . $pag` | `bool=false`×908, `bool=true`×270 |
| if | partials/pagination.html:86 | `$shouldEllipse` | `bool=false`×1786, `bool=true`×204 |
| if | partials/pagination.html:92 | `$pag.HasNext` | `bool=true`×203, `bool=false`×67 |
| with | partials/pagination.html:105 | `$pag.Last` | `*page.Pager=true`×270 |
| if | partials/pagination.html:106 | `$pag.HasNext` | `bool=true`×203, `bool=false`×67 |
| with | partials/pagination.html:122 | `.Site.Params.DynamicContent` | `bool=false`×270 |
| with | partials/rating/rating.html:21 | `.rating` | `maps.Params=true`×200 |
| with | partials/rating/rating.html:22 | `.tastytaste` | `int=true`×197, `float64=true`×2, `<invalid>=false`×1 |
| with | partials/rating/rating.html:25 | `.sweet` | `int=true`×139, `<invalid>=false`×39, `int=false`×22 |
| with | partials/rating/rating.html:28 | `.sour` | `<invalid>=false`×127, `int=false`×56, `int=true`×15, `string=false`×2 |
| with | partials/rating/rating.html:31 | `.salty` | `<invalid>=false`×121, `int=false`×41, `int=true`×36, `string=false`×2 |
| with | partials/rating/rating.html:34 | `.bitter` | `<invalid>=false`×147, `int=false`×52, `int=true`×1 |
| with | partials/rating/rating.html:37 | `.savory` | `<invalid>=false`×147, `int=false`×47, `int=true`×6 |
| with | partials/rating/rating.html:40 | `.crispy` | `int=true`×121, `<invalid>=false`×66, `int=false`×13 |
| with | partials/rating/rating.html:43 | `.chewy` | `<invalid>=false`×147, `int=false`×52, `int=true`×1 |
| with | partials/rating/rating.html:46 | `.juicy` | `<invalid>=false`×139, `int=false`×51, `int=true`×8, `float64=true`×2 |
| with | partials/rating/rating.html:49 | `.spicy` | `<invalid>=false`×131, `int=false`×37, `int=true`×32 |
| with | partials/rating/rating.html:52 | `.smell` | `int=true`×196, `<invalid>=false`×3, `int=false`×1 |
| with | partials/rating/rating.html:55 | `.quantity` | `int=true`×198, `<invalid>=false`×2 |
| with | partials/rating/rating.html:58 | `.package` | `int=true`×198, `<invalid>=false`×2 |
| if | partials/rating/rating.html:72 | `ge $ratingnumber $i` | `bool=true`×4102, `bool=false`×1658 |
| with | partials/related.html:3 | `$related` | `page.Pages=true`×184, `page.Pages=false`×51 |
| if | partials/related.html:11 | `or (eq (path.Ext $imageFile) ".jpg") (eq (path.Ext $imageFile) ".png")` | `bool=true`×636 |
| if | partials/related.html:23 | `eq (path.Ext $imageFile) ".jpg"` | `bool=true`×636 |
| if | partials/related.html:53 | `ne $index 5` | `bool=true`×551, `bool=false`×85 |
| if | partials/single/ingredientslist.html:20 | `$ingredients.name` | `string=true`×1357, `<invalid>=false`×3 |
| with | partials/single/nutritionfacts.html:2 | `.context.Params.nutrition_facts` | `maps.Params=true`×126 |
| if | partials/single/nutritionfacts.html:14 | `eq $lang "th"` | `bool=false`×63, `bool=true`×63 |
| with | partials/single/nutritionfacts.html:19 | `.servings_per_container` | `int=true`×105, `float64=true`×20, `string=true`×1 |
| with | partials/single/nutritionfacts.html:24 | `.serving_size` | `string=true`×126 |
| with | partials/single/nutritionfacts.html:29 | `.calories` | `int=true`×116, `int=false`×8, `<invalid>=false`×2 |
| if | partials/single/nutritionfacts.html:36 | `eq $lang "th"` | `bool=false`×58, `bool=true`×58 |
| with | partials/single/nutritionfacts.html:57 | `.fat` | `maps.Params=true`×120, `<invalid>=false`×6 |
| if | partials/single/nutritionfacts.html:58 | `or .total .percentage` | `int=true`×94, `float64=true`×18, `<invalid>=false`×6, `int=false`×2 |
| with | partials/single/nutritionfacts.html:60 | `.total` | `int=true`×94, `float64=true`×18 |
| with | partials/single/nutritionfacts.html:66 | `.percentage` | `int=true`×112 |
| with | partials/single/nutritionfacts.html:71 | `.saturated` | `maps.Params=true`×96, `<invalid>=false`×22, `string=false`×2 |
| if | partials/single/nutritionfacts.html:72 | `or .total .percentage` | `int=true`×56, `<invalid>=false`×28, `float64=true`×10, `int=false`×2 |
| with | partials/single/nutritionfacts.html:74 | `.total` | `int=true`×56, `float64=true`×10 |
| with | partials/single/nutritionfacts.html:80 | `.percentage` | `int=true`×66 |
| with | partials/single/nutritionfacts.html:88 | `.trans` | `maps.Params=true`×69, `<invalid>=false`×51 |
| if | partials/single/nutritionfacts.html:89 | `or .total .percentage` | `<invalid>=false`×67, `int=false`×2 |
| with | partials/single/nutritionfacts.html:112 | `.polyunsaturated` | `maps.Params=true`×62, `<invalid>=false`×58 |
| if | partials/single/nutritionfacts.html:113 | `or .total .percentage` | `<invalid>=false`×62 |
| with | partials/single/nutritionfacts.html:129 | `.monounsaturated` | `maps.Params=true`×62, `<invalid>=false`×58 |
| if | partials/single/nutritionfacts.html:130 | `or .total .percentage` | `<invalid>=false`×62 |
| with | partials/single/nutritionfacts.html:147 | `.cholesterol` | `maps.Params=true`×95, `<invalid>=false`×31 |
| if | partials/single/nutritionfacts.html:148 | `or .total .percentage` | `int=true`×44, `<invalid>=false`×41, `int=false`×6, `float64=true`×2, `string=false`×2 |
| with | partials/single/nutritionfacts.html:150 | `.total` | `int=true`×44, `float64=true`×2 |
| with | partials/single/nutritionfacts.html:156 | `.percentage` | `int=true`×44, `string=false`×2 |
| with | partials/single/nutritionfacts.html:164 | `.sodium` | `maps.Params=true`×120, `<invalid>=false`×6 |
| if | partials/single/nutritionfacts.html:165 | `or .total .percentage` | `int=true`×114, `<invalid>=false`×6 |
| with | partials/single/nutritionfacts.html:167 | `.total` | `int=true`×112, `<invalid>=false`×2 |
| with | partials/single/nutritionfacts.html:173 | `.percentage` | `int=true`×110, `<invalid>=false`×4 |
| with | partials/single/nutritionfacts.html:181 | `.potassium` | `maps.Params=true`×68, `<invalid>=false`×58 |
| if | partials/single/nutritionfacts.html:182 | `or .total .percentage` | `<invalid>=false`×58, `int=true`×10 |
| with | partials/single/nutritionfacts.html:184 | `.total` | `int=true`×10 |
| with | partials/single/nutritionfacts.html:190 | `.percentage` | `int=true`×10 |
| with | partials/single/nutritionfacts.html:198 | `.carbohydrate` | `maps.Params=true`×124, `<invalid>=false`×2 |
| if | partials/single/nutritionfacts.html:199 | `or .total .percentage` | `int=true`×122, `<invalid>=false`×2 |
| with | partials/single/nutritionfacts.html:201 | `.total` | `int=true`×122 |
| with | partials/single/nutritionfacts.html:207 | `.percentage` | `int=true`×116, `<invalid>=false`×6 |
| with | partials/single/nutritionfacts.html:213 | `.dietary_fiber` | `maps.Params=true`×84, `<invalid>=false`×38 |
| if | partials/single/nutritionfacts.html:214 | `or .total .percentage` | `int=true`×46, `<invalid>=false`×38 |
| with | partials/single/nutritionfacts.html:216 | `.total` | `int=true`×46 |
| with | partials/single/nutritionfacts.html:222 | `.percentage` | `int=true`×42, `<invalid>=false`×4 |
| with | partials/single/nutritionfacts.html:230 | `.sugar` | `maps.Params=true`×120, `<invalid>=false`×2 |
| if | partials/single/nutritionfacts.html:231 | `.total` | `int=true`×116, `<invalid>=false`×2, `int=false`×2 |
| with | partials/single/nutritionfacts.html:233 | `.total` | `int=true`×116 |
| with | partials/single/nutritionfacts.html:240 | `.added` | `maps.Params=true`×59, `<invalid>=false`×57 |
| if | partials/single/nutritionfacts.html:241 | `or .total .percentage` | `<invalid>=false`×57, `int=true`×2 |
| with | partials/single/nutritionfacts.html:243 | `.total` | `int=true`×2 |
| with | partials/single/nutritionfacts.html:248 | `.percentage` | `<invalid>=false`×2 |
| with | partials/single/nutritionfacts.html:260 | `.protein` | `maps.Params=true`×117, `<invalid>=false`×9 |
| if | partials/single/nutritionfacts.html:261 | `or .total .percentage` | `int=true`×108, `<invalid>=false`×9 |
| with | partials/single/nutritionfacts.html:263 | `.total` | `int=true`×108 |
| with | partials/single/nutritionfacts.html:269 | `.percentage` | `<invalid>=false`×108 |
| if | partials/single/nutritionfacts.html:282 | `or .vitamins .minerals` | `maps.Params=true`×81, `<invalid>=false`×45 |
| with | partials/single/nutritionfacts.html:284 | `.vitamins` | `maps.Params=true`×79, `<invalid>=false`×2 |
| with | partials/single/nutritionfacts.html:285 | `.a` | `maps.Params=true`×68, `<invalid>=false`×11 |
| if | partials/single/nutritionfacts.html:286 | `or .total .percentage` | `<invalid>=false`×56, `int=true`×10, `string=false`×2 |
| with | partials/single/nutritionfacts.html:290 | `.total` | `string=false`×6, `<invalid>=false`×2, `int=true`×2 |
| with | partials/single/nutritionfacts.html:294 | `.percentage` | `int=true`×8, `<invalid>=false`×2 |
| with | partials/single/nutritionfacts.html:301 | `.b1` | `maps.Params=true`×73, `<invalid>=false`×6 |
| if | partials/single/nutritionfacts.html:302 | `or .total .percentage` | `<invalid>=false`×51, `int=true`×20, `string=false`×2 |
| with | partials/single/nutritionfacts.html:306 | `.total` | `<invalid>=false`×8, `int=true`×8, `string=false`×4 |
| with | partials/single/nutritionfacts.html:310 | `.percentage` | `int=true`×12, `<invalid>=false`×8 |
| with | partials/single/nutritionfacts.html:317 | `.b2` | `maps.Params=true`×71, `<invalid>=false`×8 |
| if | partials/single/nutritionfacts.html:318 | `or .total .percentage` | `<invalid>=false`×53, `int=true`×18 |
| with | partials/single/nutritionfacts.html:322 | `.total` | `int=true`×14, `<invalid>=false`×4 |
| with | partials/single/nutritionfacts.html:326 | `.percentage` | `<invalid>=false`×14, `int=true`×4 |
| with | partials/single/nutritionfacts.html:333 | `.b3` | `maps.Params=true`×62, `<invalid>=false`×17 |
| if | partials/single/nutritionfacts.html:334 | `or .total .percentage` | `<invalid>=false`×62 |
| with | partials/single/nutritionfacts.html:349 | `.b5` | `maps.Params=true`×62, `<invalid>=false`×17 |
| if | partials/single/nutritionfacts.html:350 | `or .total .percentage` | `<invalid>=false`×62 |
| with | partials/single/nutritionfacts.html:365 | `.b6` | `maps.Params=true`×62, `<invalid>=false`×17 |
| if | partials/single/nutritionfacts.html:366 | `or .total .percentage` | `<invalid>=false`×62 |
| with | partials/single/nutritionfacts.html:381 | `.b7` | `maps.Params=true`×62, `<invalid>=false`×17 |
| if | partials/single/nutritionfacts.html:382 | `or .total .percentage` | `<invalid>=false`×62 |
| with | partials/single/nutritionfacts.html:397 | `.b9` | `maps.Params=true`×62, `<invalid>=false`×17 |
| if | partials/single/nutritionfacts.html:398 | `or .total .percentage` | `<invalid>=false`×62 |
| with | partials/single/nutritionfacts.html:413 | `.b12` | `maps.Params=true`×62, `<invalid>=false`×17 |
| if | partials/single/nutritionfacts.html:414 | `or .total .percentage` | `<invalid>=false`×62 |
| with | partials/single/nutritionfacts.html:429 | `.c` | `maps.Params=true`×62, `<invalid>=false`×17 |
| if | partials/single/nutritionfacts.html:430 | `or .total .percentage` | `<invalid>=false`×62 |
| with | partials/single/nutritionfacts.html:443 | `.d` | `maps.Params=true`×62, `<invalid>=false`×17 |
| if | partials/single/nutritionfacts.html:444 | `or .total .percentage` | `<invalid>=false`×62 |
| with | partials/single/nutritionfacts.html:459 | `.e` | `maps.Params=true`×62, `<invalid>=false`×17 |
| if | partials/single/nutritionfacts.html:460 | `or .total .percentage` | `<invalid>=false`×62 |
| with | partials/single/nutritionfacts.html:475 | `.k` | `maps.Params=true`×62, `<invalid>=false`×17 |
| if | partials/single/nutritionfacts.html:476 | `or .total .percentage` | `<invalid>=false`×62 |
| with | partials/single/nutritionfacts.html:492 | `.minerals` | `maps.Params=true`×74, `<invalid>=false`×7 |
| with | partials/single/nutritionfacts.html:493 | `.choline` | `maps.Params=true`×62, `<invalid>=false`×12 |
| if | partials/single/nutritionfacts.html:494 | `or .total .percentage` | `<invalid>=false`×62 |
| with | partials/single/nutritionfacts.html:509 | `.calcium` | `maps.Params=true`×67, `<invalid>=false`×7 |
| if | partials/single/nutritionfacts.html:510 | `or .total .percentage` | `<invalid>=false`×57, `int=true`×10 |
| with | partials/single/nutritionfacts.html:514 | `.total` | `<invalid>=false`×4, `int=true`×4, `string=false`×2 |
| with | partials/single/nutritionfacts.html:518 | `.percentage` | `int=true`×6, `<invalid>=false`×4 |
| with | partials/single/nutritionfacts.html:525 | `.chloride` | `maps.Params=true`×62, `<invalid>=false`×12 |
| if | partials/single/nutritionfacts.html:526 | `or .total .percentage` | `<invalid>=false`×62 |
| with | partials/single/nutritionfacts.html:541 | `.chromium` | `maps.Params=true`×62, `<invalid>=false`×12 |
| if | partials/single/nutritionfacts.html:542 | `or .total .percentage` | `<invalid>=false`×62 |
| with | partials/single/nutritionfacts.html:557 | `.copper` | `maps.Params=true`×62, `<invalid>=false`×12 |
| if | partials/single/nutritionfacts.html:558 | `or .total .percentage` | `<invalid>=false`×62 |
| with | partials/single/nutritionfacts.html:573 | `.fluoride` | `maps.Params=true`×62, `<invalid>=false`×12 |
| if | partials/single/nutritionfacts.html:574 | `or .total .percentage` | `<invalid>=false`×62 |
| with | partials/single/nutritionfacts.html:589 | `.iodine` | `maps.Params=true`×62, `<invalid>=false`×12 |
| if | partials/single/nutritionfacts.html:590 | `or .total .percentage` | `<invalid>=false`×62 |
| with | partials/single/nutritionfacts.html:605 | `.iron` | `maps.Params=true`×73, `<invalid>=false`×1 |
| if | partials/single/nutritionfacts.html:606 | `or .total .percentage` | `<invalid>=false`×51, `int=true`×22 |
| with | partials/single/nutritionfacts.html:610 | `.total` | `<invalid>=false`×12, `int=true`×8, `string=false`×2 |
| with | partials/single/nutritionfacts.html:614 | `.percentage` | `int=true`×14, `<invalid>=false`×8 |
| with | partials/single/nutritionfacts.html:621 | `.magnesium` | `maps.Params=true`×62, `<invalid>=false`×12 |
| if | partials/single/nutritionfacts.html:622 | `or .total .percentage` | `<invalid>=false`×62 |
| with | partials/single/nutritionfacts.html:637 | `.manganese` | `maps.Params=true`×62, `<invalid>=false`×12 |
| if | partials/single/nutritionfacts.html:638 | `or .total .percentage` | `<invalid>=false`×62 |
| with | partials/single/nutritionfacts.html:653 | `.molybdenum` | `maps.Params=true`×62, `<invalid>=false`×12 |
| if | partials/single/nutritionfacts.html:654 | `or .total .percentage` | `<invalid>=false`×62 |
| with | partials/single/nutritionfacts.html:669 | `.phosphorus` | `maps.Params=true`×62, `<invalid>=false`×12 |
| if | partials/single/nutritionfacts.html:670 | `or .total .percentage` | `<invalid>=false`×62 |
| with | partials/single/nutritionfacts.html:685 | `.selenium` | `maps.Params=true`×62, `<invalid>=false`×12 |
| if | partials/single/nutritionfacts.html:686 | `or .total .percentage` | `<invalid>=false`×62 |
| with | partials/single/nutritionfacts.html:701 | `.zinc` | `maps.Params=true`×62, `<invalid>=false`×12 |
| if | partials/single/nutritionfacts.html:702 | `or .total .percentage` | `<invalid>=false`×62 |
| with | partials/single/nutritionfacts.html:722 | `.note_nutrition_facts` | `<invalid>=false`×125, `string=true`×1 |
| if | partials/taxonomy/brands.html:8 | `(reflect.IsSlice .)` | `bool=false`×202 |
| if | partials/taxonomy/categories.html:5 | `(reflect.IsSlice .)` | `bool=true`×221, `bool=false`×3 |
| if | partials/taxonomy/categories.html:10 | `reflect.IsSlice $val` | `bool=false`×322, `bool=true`×1 |
| if | partials/taxonomy/companies.html:8 | `(reflect.IsSlice .)` | `bool=false`×223, `bool=true`×3 |
| if | partials/taxonomy/company/facebook.html:9 | `(reflect.IsSlice .)` | `bool=false`×5 |
| if | partials/taxonomy/company/instagram.html:9 | `(reflect.IsSlice .)` | `bool=false`×5 |
| if | partials/taxonomy/company/twitter.html:8 | `(reflect.IsSlice .)` | `bool=false`×5 |
| if | partials/taxonomy/company/website.html:8 | `(reflect.IsSlice .)` | `bool=false`×7 |
| if | partials/taxonomy/company/youtube.html:8 | `(reflect.IsSlice .)` | `bool=false`×5 |
| if | partials/taxonomy/countries.html:8 | `(reflect.IsSlice .)` | `bool=true`×155 |
| if | partials/taxonomy/ingredients.html:6 | `(reflect.IsSlice .)` | `bool=true`×195 |
| if | partials/taxonomy/tags.html:1 | `(isset . 0)` | `bool=true`×225 |
| if | partials/taxonomy/tags.html:3 | `(index . 0)` | `string=true`×198, `string=false`×27 |
| if | partials/taxonomy/tags.html:8 | `(reflect.IsSlice .)` | `bool=true`×198 |
| if | sitemap.xml:5 | `.Permalink` | `string=true`×1730 |
| if | sitemap.xml:7 | `not .Lastmod.IsZero` | `bool=true`×1720, `bool=false`×10 |
| with | sitemap.xml:8 | `.Sitemap.ChangeFreq` | `string=true`×1730 |
| if | sitemap.xml:11 | `eq .Kind "tanoxomy"` | `bool=false`×1730 |
| if | sitemap.xml:13 | `eq .Kind "section"` | `bool=false`×1664, `bool=true`×66 |
| if | sitemap.xml:15 | `eq .Kind "term"` | `bool=true`×1411, `bool=false`×253 |
| if | sitemap.xml:17 | `eq .Kind "page"` | `bool=true`×239, `bool=false`×14 |
| if | sitemap.xml:22 | `.IsTranslated` | `bool=false`×1014, `bool=true`×716 |
| if | taxonomy/list.html:5 | `eq hugo.Environment "development"` | `bool=false`×123 |
| if | taxonomy/list.html:26 | `eq $kindTerm "term"` | `bool=false`×123 |
| if | taxonomy/list.html:31 | `$page.Params.image` | `<invalid>=false`×1402, `string=true`×9 |
| if | taxonomy/list.html:50 | `eq (path.Ext $imageFile) ".jpg"` | `bool=false`×5, `bool=true`×4 |
| if | taxonomy/list.html:55 | `eq (path.Ext $imageFile) ".png"` | `bool=true`×5 |
| if | taxonomy/list.html:77 | `eq hugo.Environment "development"` | `bool=false`×1402 |
| if | taxonomy/list.html:92 | `eq hugo.Environment "development"` | `bool=false`×123 |
| if | term/term.html:9 | `eq hugo.Environment "development"` | `bool=false`×1479 |
| if | term/term.html:28 | `.Params.Image` | `<invalid>=false`×1470, `string=true`×9 |
| if | term/term.html:31 | `or (eq (path.Ext $imageFile) ".jpg") (eq (path.Ext $imageFile) ".png")` | `bool=true`×9 |
| if | term/term.html:40 | `eq (path.Ext $imageFile) ".jpg"` | `bool=false`×5, `bool=true`×4 |
| if | term/term.html:45 | `eq (path.Ext $imageFile) ".png"` | `bool=true`×5 |
| with | term/term.html:64 | `.Params.Website` | `<invalid>=false`×1472, `string=true`×7 |
| with | term/term.html:67 | `.Params.Facebook` | `<invalid>=false`×1474, `string=true`×5 |
| with | term/term.html:70 | `.Params.Twitter` | `<invalid>=false`×1474, `string=true`×5 |
| with | term/term.html:73 | `.Params.Instagram` | `<invalid>=false`×1474, `string=true`×5 |
| with | term/term.html:76 | `.Params.Youtube` | `<invalid>=false`×1474, `string=true`×5 |
| with | term/term.html:79 | `.Params.Tags` | `<invalid>=false`×1476, `[]string=true`×3 |
| with | term/term.html:84 | `.Params.References` | `<invalid>=false`×1474, `[]string=true`×5 |
| if | term/term.html:107 | `or .Site.Params.Social.Facebook.Enable .Site.Params.Marketing.Twitter.Enable` | `bool=true`×1479 |
| if | term/term.html:119 | `$page.Params.image` | `string=true`×3913, `<invalid>=false`×2 |
| if | term/term.html:138 | `eq (path.Ext $imageFile) ".jpg"` | `bool=true`×3913 |
| if | term/term.html:165 | `eq hugo.Environment "development"` | `bool=false`×2 |
| if | term/term.html:180 | `eq hugo.Environment "development"` | `bool=false`×1479 |

### A.6 html/template escaper chains inserted per action (contextual autoescape)

Chain names are the `_html_template_*` functions html/template appended to the pipeline (prefix stripped); `html` = the text/template builtin HTMLEscaper used explicitly. Input type = value type entering the first escaper.

| site | action (pre-escape pipeline) | escaper chain | input type ×count |
|---|---|---|---|
| EMB:_markup/render-link.html:21 | `.` | attrescaper | `string`×60 |
| EMB:_markup/render-link.html:21 | `.Text` | htmlescaper | `hstring.HTML`×558 |
| EMB:_markup/render-link.html:21 | `$href` | urlfilter → urlnormalizer → attrescaper | `string`×558 |
| EMB:_markup/render-table.html:16 | `.Text` | htmlescaper | `hstring.HTML`×311 |
| EMB:_markup/render-table.html:31 | `.Text` | htmlescaper | `hstring.HTML`×1276 |
| EMB:_shortcodes/ref.html:1 | `ref . .Params` | htmlescaper | `string`×6 |
| EMB:alias.html:2 | `site.Language.LanguageCode` | attrescaper | `string`×1494 |
| EMB:alias.html:4 | `.Permalink` | rcdataescaper | `string`×1494 |
| EMB:alias.html:5 | `.Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×1494 |
| EMB:alias.html:8 | `.Permalink` | attrescaper | `string`×1494 |
| EMB:sitemapindex.xml:1 | `printf "<?xml version=\"1.0\" encoding=\"utf-8\" standalone=\"yes\"?>" \| safeHT` | htmlescaper | `template.HTML`×1 |
| EMB:sitemapindex.xml:5 | `.SitemapAbsURL` | htmlescaper | `string`×2 |
| EMB:sitemapindex.xml:7 | `.Lastmod.Format "2006-01-02T15:04:05-07:00" \| safeHTML` | htmlescaper | `template.HTML`×2 |
| _default/_markup/render-heading.html:1 | `.Anchor \| safeURL` | attrescaper | `template.URL`×578 |
| _default/_markup/render-heading.html:1 | `.Text \| safeHTML` | htmlescaper | `template.HTML`×578 |
| _default/_markup/render-heading.html:1 | `.Level` | htmlnamefilter | `int`×1156 |
| _default/_markup/render-heading.html:1 | `.Anchor \| safeURL` | urlescaper → attrescaper | `template.URL`×578 |
| _default/_markup/render-image.html:11 | `$imagewebp.RelPermalink` | srcsetescaper → attrescaper | `string`×1050 |
| _default/_markup/render-image.html:13 | `$imageFile.RelPermalink` | srcsetescaper → attrescaper | `string`×1050 |
| _default/_markup/render-image.html:19 | `$imagewebp.RelPermalink` | urlfilter → urlnormalizer → attrescaper | `string`×1050 |
| _default/_markup/render-image.html:20 | `$imageFile.Width` | attrescaper | `int`×1050 |
| _default/_markup/render-image.html:21 | `$imageFile.Height` | attrescaper | `int`×1050 |
| _default/_markup/render-image.html:22 | `.Text` | attrescaper | `hstring.HTML`×1050 |
| _default/baseof.html:4 | `.` | attrescaper | `string`×3906 |
| _default/baseof.html:8 | `partial "head.html" .` | htmlescaper | `template.HTML`×1953 |
| _default/baseof.html:17 | `$title` | rcdataescaper | `string`×239 |
| _default/baseof.html:24 | `partial "header.html" .` | htmlescaper | `template.HTML`×1953 |
| _default/baseof.html:26 | `partial "footer.html" .` | htmlescaper | `template.HTML`×1953 |
| _default/latesturl.html:2 | `.Permalink` | htmlescaper | `string`×154 |
| _default/list.html:2 | `partial "ads/adsensehead.html" .` | htmlescaper | `template.HTML`×71 |
| _default/list.html:12 | `$jsWebsiteTheme.Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×71 |
| _default/list.html:19 | `.Title` | htmlescaper | `string`×71 |
| _default/list.html:21 | `partial "ads/adsensemanual.html" .` | htmlescaper | `template.HTML`×71 |
| _default/list.html:22 | `partial "breadcrumb" .` | htmlescaper | `template.HTML`×71 |
| _default/list.html:31 | `.RelPermalink` | urlfilter → urlnormalizer → attrescaper | `string`×234 |
| _default/list.html:43 | `$imagewebp.RelPermalink` | srcsetescaper → attrescaper | `string`×232 |
| _default/list.html:48 | `$imageFile.RelPermalink` | srcsetescaper → attrescaper | `string`×232 |
| _default/list.html:58 | `$imageFile.RelPermalink` | urlfilter → urlnormalizer → attrescaper | `string`×232 |
| _default/list.html:59 | `.Title` | attrescaper | `string`×232 |
| _default/list.html:65 | `.RelPermalink` | urlfilter → urlnormalizer → attrescaper | `string`×234 |
| _default/list.html:66 | `.Title` | htmlescaper | `string`×234 |
| _default/list.html:68 | `.Description` | htmlescaper | `string`×234 |
| _default/list.html:69 | `.RelPermalink` | urlfilter → urlnormalizer → attrescaper | `string`×234 |
| _default/list.html:74 | `partial "pagination.html" .` | htmlescaper | `template.HTML`×71 |
| _default/list.html:75 | `partial "ads/adsensemanual.html" .` | htmlescaper | `template.HTML`×71 |
| _default/list.html:113 | `$script.Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×71 |
| _default/rss.xml:15 | `printf "<?xml version=\"1.0\" encoding=\"utf-8\" standalone=\"yes\"?>" \| safeHT` | htmlescaper | `template.HTML`×1491 |
| _default/rss.xml:19 | `.Site.Title` | rcdataescaper | `string`×1491 |
| _default/rss.xml:19 | `.` | rcdataescaper | `string`×1488 |
| _default/rss.xml:20 | `.Permalink` | htmlescaper | `string`×1491 |
| _default/rss.xml:23 | `.` | htmlescaper | `string`×1488 |
| _default/rss.xml:23 | `.Site.Title` | htmlescaper | `string`×1491 |
| _default/rss.xml:24 | `.` | htmlescaper | `string`×2982 |
| _default/rss.xml:25 | `.` | htmlescaper | `string`×2982 |
| _default/rss.xml:30 | `.Site.Params.yearCreate` | htmlescaper | `string`×1491 |
| _default/rss.xml:30 | `now.Format "2006 "` | htmlescaper | `string`×1491 |
| _default/rss.xml:32 | `.Site.Params.Company` | htmlescaper | `string`×1491 |
| _default/rss.xml:36 | `$imageFile.Permalink` | htmlescaper | `string`×1491 |
| _default/rss.xml:37 | `.Site.Title` | rcdataescaper | `string`×1491 |
| _default/rss.xml:38 | `.Permalink` | htmlescaper | `string`×1491 |
| _default/rss.xml:40 | `.Date.Format "Mon, 02 Jan 2006 15:04:05 -0700" \| safeHTML` | htmlescaper | `template.HTML`×1489 |
| _default/rss.xml:41 | `printf "<atom:link href=%q rel=\"self\" type=%q />" .Permalink .MediaType \| saf` | htmlescaper | `template.HTML`×1491 |
| _default/rss.xml:45 | `.Title` | rcdataescaper | `string`×3563 |
| _default/rss.xml:46 | `.Permalink` | htmlescaper | `string`×3563 |
| _default/rss.xml:47 | `.Date.Format "Mon, 02 Jan 2006 15:04:05 -0700" \| safeHTML` | htmlescaper | `template.HTML`×3563 |
| _default/rss.xml:48 | `index . 0 \| humanize` | htmlescaper | `string`×3510 |
| _default/rss.xml:49 | `.Permalink` | htmlescaper | `string`×3563 |
| _default/rss.xml:50 | `.Title` | attrescaper | `string`×3563 |
| _default/rss.xml:50 | `.Description \| html` | html | `string`×3563 |
| _default/rss.xml:50 | `"]]>" \| safeHTML` | htmlescaper | `template.HTML`×3563 |
| _default/rss.xml:50 | `"<![CDATA[" \| safeHTML` | htmlescaper | `template.HTML`×3563 |
| _default/rss.xml:50 | `(printf "%s%s" .Permalink .Params.Image)` | urlfilter → urlnormalizer → attrescaper | `string`×3563 |
| _default/simple.html:2 | `partial "ads/adsensehead.html" .` | htmlescaper | `template.HTML`×3 |
| _default/simple.html:12 | `$jsWebsiteTheme.Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×3 |
| _default/simple.html:18 | `.Title` | htmlescaper | `string`×3 |
| _default/simple.html:21 | `.Content` | htmlescaper | `template.HTML`×3 |
| _default/simple.html:60 | `$script.Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×3 |
| _default/single.html:1 | `.Site.Title` | rcdataescaper | `string`×235 |
| _default/single.html:1 | `.Title` | rcdataescaper | `string`×235 |
| _default/single.html:3 | `partial "ads/adsensehead.html" .` | htmlescaper | `template.HTML`×235 |
| _default/single.html:13 | `$jsWebsiteTheme.Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×235 |
| _default/single.html:19 | `partial "ads/adsensemanual.html" .` | htmlescaper | `template.HTML`×235 |
| _default/single.html:20 | `partial "breadcrumb" .` | htmlescaper | `template.HTML`×235 |
| _default/single.html:27 | `.Title` | htmlescaper | `string`×235 |
| _default/single.html:30 | `.Description` | htmlescaper | `string`×235 |
| _default/single.html:35 | `partial "single/author" .` | htmlescaper | `template.HTML`×213 |
| _default/single.html:42 | `partial "single/date" (dict "context" .Params.date)` | htmlescaper | `template.HTML`×231 |
| _default/single.html:59 | `$imagewebp.RelPermalink` | srcsetescaper → attrescaper | `string`×232 |
| _default/single.html:64 | `$imageFile.RelPermalink` | srcsetescaper → attrescaper | `string`×232 |
| _default/single.html:74 | `$imageFile.RelPermalink` | urlfilter → urlnormalizer → attrescaper | `string`×232 |
| _default/single.html:75 | `$title` | attrescaper | `string`×232 |
| _default/single.html:83 | `partial "taxonomy/companies" .` | htmlescaper | `template.HTML`×226 |
| _default/single.html:86 | `partial "taxonomy/brands" .` | htmlescaper | `template.HTML`×202 |
| _default/single.html:89 | `partial "taxonomy/countries" .` | htmlescaper | `template.HTML`×155 |
| _default/single.html:92 | `partial "single/whenseen" (dict "context" .)` | htmlescaper | `template.HTML`×39 |
| _default/single.html:96 | `.Content` | htmlescaper | `template.HTML`×235 |
| _default/single.html:99 | `partial "single/taste" .` | htmlescaper | `template.HTML`×3 |
| _default/single.html:102 | `partial "single/smell" .` | htmlescaper | `template.HTML`×3 |
| _default/single.html:105 | `partial "single/nutritionfacts" (dict "context" .)` | htmlescaper | `template.HTML`×126 |
| _default/single.html:110 | `partial "single/ingredientslist" (dict "context" .)` | htmlescaper | `template.HTML`×128 |
| _default/single.html:116 | `i18n "video"` | htmlescaper | `string`×89 |
| _default/single.html:128 | `.` | urlnormalizer → attrescaper | `string`×89 |
| _default/single.html:136 | `partial "rating/rating" (dict "rating" .)` | htmlescaper | `template.HTML`×200 |
| _default/single.html:139 | `partial "single/socialshare" .` | htmlescaper | `template.HTML`×235 |
| _default/single.html:142 | `partial "comments" .` | htmlescaper | `template.HTML`×235 |
| _default/single.html:147 | `partial "ads/adsensemanual.html" .` | htmlescaper | `template.HTML`×235 |
| _default/single.html:150 | `partial "taxonomy/categories" .` | htmlescaper | `template.HTML`×224 |
| _default/single.html:155 | `partial "taxonomy/ingredients" .` | htmlescaper | `template.HTML`×195 |
| _default/single.html:160 | `partial "taxonomy/tags" .` | htmlescaper | `template.HTML`×222 |
| _default/single.html:164 | `partial "related" .` | htmlescaper | `template.HTML`×235 |
| _default/single.html:224 | `$script.Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×235 |
| 404.html:2 | `partial "ads/adsensehead.html" .` | htmlescaper | `template.HTML`×21 |
| 404.html:11 | `.` | urlescaper → attrescaper | `string`×21 |
| 404.html:22 | `$jsWebsiteTheme.Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×21 |
| 404.html:34 | `.Site.BaseURL` | urlfilter → urlnormalizer → attrescaper | `string`×21 |
| 404.html:75 | `$script.Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×21 |
| index.html:2 | `partial "ads/adsensehead.html" .` | htmlescaper | `template.HTML`×21 |
| index.html:11 | `.` | urlescaper → attrescaper | `string`×21 |
| index.html:22 | `$jsWebsiteTheme.Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×21 |
| index.html:29 | `.Site.Params.HomeTitle` | htmlescaper | `string`×21 |
| index.html:31 | `partial "ads/adsensemanual.html" .` | htmlescaper | `template.HTML`×21 |
| index.html:32 | `partial "breadcrumb" .` | htmlescaper | `template.HTML`×21 |
| index.html:35 | `partial "carousel" .` | htmlescaper | `template.HTML`×21 |
| index.html:44 | `.RelPermalink` | urlfilter → urlnormalizer → attrescaper | `string`×229 |
| index.html:56 | `$imagewebp.RelPermalink` | srcsetescaper → attrescaper | `string`×228 |
| index.html:60 | `$imageFile.RelPermalink` | srcsetescaper → attrescaper | `string`×228 |
| index.html:71 | `$imageFile.RelPermalink` | urlfilter → urlnormalizer → attrescaper | `string`×228 |
| index.html:72 | `.Title` | attrescaper | `string`×228 |
| index.html:78 | `.RelPermalink` | urlfilter → urlnormalizer → attrescaper | `string`×229 |
| index.html:79 | `.Title` | htmlescaper | `string`×229 |
| index.html:81 | `.Description` | htmlescaper | `string`×229 |
| index.html:82 | `.RelPermalink` | urlfilter → urlnormalizer → attrescaper | `string`×229 |
| index.html:88 | `partial "pagination.html" .` | htmlescaper | `template.HTML`×21 |
| index.html:94 | `i18n "categories"` | htmlescaper | `string`×21 |
| index.html:104 | `$taxonomy_term` | attrescaper | `string`×126 |
| index.html:110 | `i18n "snackbrands"` | htmlescaper | `string`×21 |
| index.html:112 | `i18n "snackcategories"` | htmlescaper | `string`×21 |
| index.html:114 | `i18n "snackcompanies"` | htmlescaper | `string`×21 |
| index.html:116 | `i18n "snackcountries"` | htmlescaper | `string`×21 |
| index.html:118 | `i18n "snackingredients"` | htmlescaper | `string`×21 |
| index.html:120 | `i18n "snacktags"` | htmlescaper | `string`×21 |
| index.html:126 | `$taxonomy_term` | attrescaper | `string`×126 |
| index.html:133 | `.Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×15568 |
| index.html:137 | `.Title \| humanize` | htmlescaper | `string`×1085 |
| index.html:139 | `.Title` | htmlescaper | `string`×14483 |
| index.html:183 | `.Site.Title` | htmlescaper | `string`×21 |
| index.html:229 | `$script.Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×21 |
| partials/ads/adsensehead.html:2 | `.Site.Params.Monetization.Ads.DataAdClient` | urlescaper → attrescaper | `string`×1953 |
| partials/ads/adsensemanual.html:6 | `.Site.Params.Monetization.Ads.DataAdClient` | attrescaper | `string`×3837 |
| partials/ads/adsensemanual.html:7 | `.Site.Params.Monetization.Ads.DataAdSlot` | attrescaper | `string`×3837 |
| partials/breadcrumb.html:16 | `.p1.Title` | htmlescaper | `string`×235 |
| partials/breadcrumb.html:21 | `.p1.Title` | htmlescaper | `string`×103 |
| partials/breadcrumb.html:26 | `.p1.Title` | htmlescaper | `string`×103 |
| partials/breadcrumb.html:31 | `.p1.Title` | htmlescaper | `string`×23 |
| partials/breadcrumb.html:36 | `.p1.Title` | htmlescaper | `string`×822 |
| partials/breadcrumb.html:41 | `.p1.Site.Title` | htmlescaper | `string`×21 |
| partials/breadcrumb.html:46 | `.p1.Title` | htmlescaper | `string`×622 |
| partials/breadcrumb.html:51 | `.p1.Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×1908 |
| partials/breadcrumb.html:53 | `.p1.Site.Title` | htmlescaper | `string`×1908 |
| partials/breadcrumb.html:58 | `.p1.Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×95 |
| partials/breadcrumb.html:60 | `.p1.Title` | htmlescaper | `string`×95 |
| partials/breadcrumb.html:65 | `.p1.Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×94 |
| partials/breadcrumb.html:67 | `.p1.Title` | htmlescaper | `string`×94 |
| partials/breadcrumb.html:72 | `.p1.Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×21 |
| partials/breadcrumb.html:74 | `.p1.Title` | htmlescaper | `string`×21 |
| partials/breadcrumb.html:81 | `.p1.Title` | htmlescaper | `string`×762 |
| partials/breadcrumb.html:86 | `.p1.Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×741 |
| partials/breadcrumb.html:88 | `.p1.Title` | htmlescaper | `string`×741 |
| partials/carousel.html:7 | `.` | attrescaper | `int`×105 |
| partials/carousel.html:7 | `add . 1` | attrescaper | `int64`×105 |
| partials/carousel.html:17 | `.RelPermalink` | urlfilter → urlnormalizer → attrescaper | `string`×105 |
| partials/carousel.html:32 | `$imageFile.RelPermalink` | srcsetescaper → attrescaper | `string`×105 |
| partials/carousel.html:36 | `$imageFile.RelPermalink` | srcsetescaper → attrescaper | `string`×105 |
| partials/carousel.html:42 | `$imageFile.RelPermalink` | urlfilter → urlnormalizer → attrescaper | `string`×105 |
| partials/carousel.html:43 | `.Title` | attrescaper | `string`×105 |
| partials/comments.html:4 | `i18n "comments"` | htmlescaper | `string`×1714 |
| partials/comments.html:28 | `partial "comment/rating-review" (dict "rating" "tasty-taste" "ratingName" "Tasty` | htmlescaper | `template.HTML`×1714 |
| partials/comments.html:29 | `partial "comment/rating-review" (dict "rating" "smell" "ratingName" "Smell")` | htmlescaper | `template.HTML`×1714 |
| partials/comments.html:30 | `partial "comment/rating-review" (dict "rating" "quantity" "ratingName" "Quantity` | htmlescaper | `template.HTML`×1714 |
| partials/comments.html:31 | `partial "comment/rating-review" (dict "rating" "package" "ratingName" "Package")` | htmlescaper | `template.HTML`×1714 |
| partials/comments.html:46 | `partial "comment/rating-review" (dict "rating" "sweet" "ratingName" "Sweet")` | htmlescaper | `template.HTML`×1714 |
| partials/comments.html:47 | `partial "comment/rating-review" (dict "rating" "sour" "ratingName" "Sour")` | htmlescaper | `template.HTML`×1714 |
| partials/comments.html:48 | `partial "comment/rating-review" (dict "rating" "salty" "ratingName" "Salty")` | htmlescaper | `template.HTML`×1714 |
| partials/comments.html:49 | `partial "comment/rating-review" (dict "rating" "bitter" "ratingName" "Bitter")` | htmlescaper | `template.HTML`×1714 |
| partials/comments.html:50 | `partial "comment/rating-review" (dict "rating" "savory" "ratingName" "Savory")` | htmlescaper | `template.HTML`×1714 |
| partials/comments.html:51 | `partial "comment/rating-review" (dict "rating" "crispy" "ratingName" "Crispy")` | htmlescaper | `template.HTML`×1714 |
| partials/comments.html:52 | `partial "comment/rating-review" (dict "rating" "chewy" "ratingName" "Chewy")` | htmlescaper | `template.HTML`×1714 |
| partials/comments.html:53 | `partial "comment/rating-review" (dict "rating" "juicy" "ratingName" "Juicy")` | htmlescaper | `template.HTML`×1714 |
| partials/comments.html:54 | `partial "comment/rating-review" (dict "rating" "spicy" "ratingName" "Spicy")` | htmlescaper | `template.HTML`×1714 |
| partials/comments.html:91 | `trim .RelPermalink "/"` | attrescaper | `string`×1714 |
| partials/comments.html:97 | `.UniqueID` | attrescaper | `string`×246 |
| partials/comments.html:110 | `i18n "form_name"` | attrescaper | `string`×1714 |
| partials/comments.html:124 | `i18n "form_email"` | attrescaper | `string`×1714 |
| partials/comments.html:132 | `i18n "form_comment"` | attrescaper | `string`×1714 |
| partials/comments.html:156 | `i18n "success_msg"` | htmlescaper | `string`×1714 |
| partials/comments.html:158 | `i18n "error_msg"` | htmlescaper | `string`×1714 |
| partials/comments.html:167 | `i18n "submit"` | htmlescaper | `string`×1714 |
| partials/comments.html:170 | `i18n "reset"` | htmlescaper | `string`×1714 |
| partials/comments.html:192 | `$.Scratch.Set "hasComments" 0` | htmlescaper | `string`×1714 |
| partials/comments.html:196 | `$.Scratch.Add "hasComments" 1` | htmlescaper | `string`×2 |
| partials/comments.html:197 | `$.Scratch.Set "hasReplies" 0` | htmlescaper | `string`×2 |
| partials/comments.html:198 | `$.Scratch.Set "threadID" ._id` | htmlescaper | `string`×2 |
| partials/comments.html:199 | `$.Scratch.SetInMap "replyIndices" ._id 0` | htmlescaper | `string`×2 |
| partials/comments.html:201 | `$.Scratch.Get "hasComments"` | attrescaper | `int64`×2 |
| partials/comments.html:206 | `._id` | attrescaper | `string`×2 |
| partials/comments.html:208 | `.email` | urlnormalizer → attrescaper | `<invalid>`×1, `string`×1 |
| partials/comments.html:209 | `.name` | attrescaper | `string`×2 |
| partials/comments.html:214 | `._id` | attrescaper | `string`×2 |
| partials/comments.html:215 | `.name` | htmlescaper | `string`×2 |
| partials/comments.html:220 | `$.Scratch.Get "hasComments"` | urlescaper → attrescaper | `int64`×2 |
| partials/comments.html:223 | `.date` | attrescaper | `string`×2 |
| partials/comments.html:224 | `dateFormat (default "Jan 2, 2006" .Site.Params.dateformat) .date` | htmlescaper | `string`×2 |
| partials/comments.html:234 | `partial "comment/rating-comment" (dict "context" . "ratingName" "Tasty Taste")` | htmlescaper | `template.HTML`×1 |
| partials/comments.html:237 | `partial "comment/rating-comment" (dict "context" . "ratingName" "Smell")` | htmlescaper | `template.HTML`×1 |
| partials/comments.html:240 | `partial "comment/rating-comment" (dict "context" . "ratingName" "Quantity")` | htmlescaper | `template.HTML`×1 |
| partials/comments.html:243 | `partial "comment/rating-comment" (dict "context" . "ratingName" "Package")` | htmlescaper | `template.HTML`×1 |
| partials/comments.html:278 | `.comment \| markdownify` | htmlescaper | `template.HTML`×2 |
| partials/comments.html:282 | `._id` | attrescaper | `string`×2 |
| partials/comments.html:285 | `._id` | attrescaper | `string`×2 |
| partials/comments.html:286 | `i18n "reply"` | htmlescaper | `string`×2 |
| partials/comments.html:341 | `$.Scratch.Delete "replyIndices"` | htmlescaper | `string`×2 |
| partials/comments.html:352 | `.ratingName` | htmlescaper | `string`×22282 |
| partials/comments.html:358 | `.rating` | attrescaper | `string`×22282 |
| partials/comments.html:360 | `.rating` | attrescaper | `string`×22282 |
| partials/comments.html:361 | `.rating` | attrescaper | `string`×22282 |
| partials/comments.html:364 | `.rating` | attrescaper | `string`×22282 |
| partials/comments.html:366 | `.rating` | attrescaper | `string`×22282 |
| partials/comments.html:367 | `.rating` | attrescaper | `string`×22282 |
| partials/comments.html:370 | `.rating` | attrescaper | `string`×22282 |
| partials/comments.html:372 | `.rating` | attrescaper | `string`×22282 |
| partials/comments.html:373 | `.rating` | attrescaper | `string`×22282 |
| partials/comments.html:376 | `.rating` | attrescaper | `string`×22282 |
| partials/comments.html:378 | `.rating` | attrescaper | `string`×22282 |
| partials/comments.html:379 | `.rating` | attrescaper | `string`×22282 |
| partials/comments.html:382 | `.rating` | attrescaper | `string`×22282 |
| partials/comments.html:384 | `.rating` | attrescaper | `string`×22282 |
| partials/comments.html:385 | `.rating` | attrescaper | `string`×22282 |
| partials/comments.html:394 | `.ratingName` | htmlescaper | `string`×4 |
| partials/footer.html:14 | `$imagewebp.RelPermalink` | srcsetescaper → attrescaper | `string`×1953 |
| partials/footer.html:21 | `$imageFile.RelPermalink` | srcsetescaper → attrescaper | `string`×1953 |
| partials/footer.html:25 | `$imageFile.RelPermalink` | urlfilter → urlnormalizer → attrescaper | `string`×1953 |
| partials/footer.html:26 | `.Title` | attrescaper | `string`×1953 |
| partials/footer.html:34 | `.title` | htmlescaper | `string`×3906 |
| partials/footer.html:38 | `.link` | urlfilter → urlnormalizer → attrescaper | `string`×11718 |
| partials/footer.html:39 | `.text` | htmlescaper | `string`×11718 |
| partials/footer.html:49 | `.link` | urlfilter → urlnormalizer → attrescaper | `string`×15624 |
| partials/footer.html:50 | `.icon` | attrescaper | `string`×15624 |
| partials/footer.html:79 | `.Site.Params.yearCreate` | htmlescaper | `string`×1953 |
| partials/footer.html:80 | `now.Format "2006 "` | htmlescaper | `string`×1953 |
| partials/footer.html:82 | `.Site.Params.Company` | htmlescaper | `string`×1953 |
| partials/head.html:11 | `partial "marketing/google/googleGtag.html" .` | htmlescaper | `template.HTML`×1953 |
| partials/head.html:12 | `partial "marketing/google/googleTagManagerHead.html" .` | htmlescaper | `template.HTML`×1953 |
| partials/head.html:23 | `$description` | attrescaper | `string`×1953 |
| partials/head.html:27 | `(resources.Get "images/favicon/apple-touch-icon-57x57.png").Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×1953 |
| partials/head.html:32 | `(resources.Get "images/favicon/apple-touch-icon-114x114.png").Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×1953 |
| partials/head.html:37 | `(resources.Get "images/favicon/apple-touch-icon-72x72.png").Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×1953 |
| partials/head.html:42 | `(resources.Get "images/favicon/apple-touch-icon-144x144.png").Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×1953 |
| partials/head.html:47 | `(resources.Get "images/favicon/apple-touch-icon-60x60.png").Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×1953 |
| partials/head.html:52 | `(resources.Get "images/favicon/apple-touch-icon-120x120.png").Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×1953 |
| partials/head.html:57 | `(resources.Get "images/favicon/apple-touch-icon-76x76.png").Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×1953 |
| partials/head.html:62 | `(resources.Get "images/favicon/apple-touch-icon-152x152.png").Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×1953 |
| partials/head.html:67 | `(resources.Get "images/favicon/favicon-196x196.png").Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×1953 |
| partials/head.html:73 | `(resources.Get "images/favicon/favicon-96x96.png").Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×1953 |
| partials/head.html:79 | `(resources.Get "images/favicon/favicon-32x32.png").Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×1953 |
| partials/head.html:85 | `(resources.Get "images/favicon/favicon-16x16.png").Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×1953 |
| partials/head.html:91 | `(resources.Get "images/favicon/favicon-128.png").Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×1953 |
| partials/head.html:98 | `(resources.Get "images/favicon/mstile-144x144.png").Permalink` | attrescaper | `string`×1953 |
| partials/head.html:102 | `(resources.Get "images/favicon/mstile-70x70.png").Permalink` | attrescaper | `string`×1953 |
| partials/head.html:106 | `(resources.Get "images/favicon/mstile-150x150.png").Permalink` | attrescaper | `string`×1953 |
| partials/head.html:110 | `(resources.Get "images/favicon/mstile-310x150.png").Permalink` | attrescaper | `string`×1953 |
| partials/head.html:114 | `(resources.Get "images/favicon/mstile-310x310.png").Permalink` | attrescaper | `string`×1953 |
| partials/head.html:118 | `(resources.Get "images/favicon/favicon.ico").Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×1953 |
| partials/head.html:122 | `printf `<link rel="%s"  href="%s" type="%s" title="%s" />` .Rel .Permalink .Medi` | htmlescaper | `template.HTML`×1715 |
| partials/head.html:129 | `.Language.Lang` | attrescaper | `string`×1794 |
| partials/head.html:130 | `.Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×1794 |
| partials/head.html:135 | `.Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×897 |
| partials/head.html:145 | `$permalink` | urlfilter → urlnormalizer → attrescaper | `string`×1953 |
| partials/head.html:159 | `.Scratch.Set "og_image" $og_image` | htmlescaper | `string`×1953 |
| partials/head.html:160 | `site.Title` | attrescaper | `string`×1953 |
| partials/head.html:161 | `.Permalink` | attrescaper | `string`×1953 |
| partials/head.html:162 | `$title` | attrescaper | `string`×1953 |
| partials/head.html:163 | `$description` | attrescaper | `string`×1953 |
| partials/head.html:166 | `.` | attrescaper | `string`×1953 |
| partials/head.html:178 | `$translation.Language.Params.LanguageCodeOpenGraph \| default "en_US"` | attrescaper | `string`×897 |
| partials/head.html:183 | `$translation.Language.Params.LanguageCodeOpenGraph \| default "en_US"` | attrescaper | `string`×897 |
| partials/head.html:191 | `.` | attrescaper | `string`×89 |
| partials/head.html:197 | `.Permalink` | attrescaper | `string`×636 |
| partials/head.html:205 | `.` | attrescaper | `string`×213 |
| partials/head.html:208 | `.` | attrescaper | `string`×238 |
| partials/head.html:210 | `.Section` | attrescaper | `string`×238 |
| partials/head.html:214 | `.` | attrescaper | `string`×797, `<invalid>`×1 |
| partials/head.html:218 | `.` | attrescaper | `string`×1953 |
| partials/head.html:220 | `$twitter_card` | attrescaper | `string`×1953 |
| partials/head.html:224 | `$title` | attrescaper | `string`×1953 |
| partials/head.html:225 | `$description` | attrescaper | `string`×1953 |
| partials/head.html:227 | `.` | attrescaper | `string`×1953 |
| partials/head.html:230 | `.` | attrescaper | `string`×1953 |
| partials/head.html:233 | `.` | attrescaper | `string`×217 |
| partials/head.html:236 | `partial "marketing/jsonLd.html" .` | htmlescaper | `template.HTML`×1953 |
| partials/head.html:242 | `.PublishDate.Format "2006-01-02T15:04:05-07:00" \| safeHTML` | attrescaper | `template.HTML`×231 |
| partials/head.html:253 | `.Lastmod.Format "2006-01-02T15:04:05-07:00" \| safeHTML` | attrescaper | `template.HTML`×231 |
| partials/head.html:260 | `.Date.Format "2006-01-02T15:04:05-07:00" \| safeHTML` | attrescaper | `template.HTML`×1692 |
| partials/head.html:264 | `.` | attrescaper | `string`×1953 |
| partials/head.html:267 | `.` | attrescaper | `string`×807, `<invalid>`×1 |
| partials/head.html:278 | `.Content \| safeCSS` | cssvaluefilter | `template.CSS`×1953 |
| partials/header.html:10 | `"/" \| absLangURL` | urlfilter → urlnormalizer → attrescaper | `string`×1953 |
| partials/header.html:15 | `$imagewebp.RelPermalink` | srcsetescaper → attrescaper | `string`×1953 |
| partials/header.html:22 | `$imageFile.RelPermalink` | srcsetescaper → attrescaper | `string`×1953 |
| partials/header.html:26 | `$imageFile.RelPermalink` | urlfilter → urlnormalizer → attrescaper | `string`×1953 |
| partials/header.html:27 | `$title` | attrescaper | `string`×1953 |
| partials/header.html:33 | `"/" \| absLangURL` | urlfilter → urlnormalizer → attrescaper | `string`×1953 |
| partials/header.html:34 | `.Site.Title` | htmlescaper | `string`×1953 |
| partials/header.html:73 | `.link` | urlfilter → urlnormalizer → attrescaper | `string`×9765 |
| partials/header.html:74 | `.icon` | attrescaper | `string`×9765 |
| partials/header.html:116 | `$translation.Language` | attrescaper | `*langs.Language`×897 |
| partials/header.html:117 | `$translation.Permalink` | attrescaper | `string`×897 |
| partials/header.html:120 | `.LanguageName` | htmlescaper | `string`×897 |
| partials/header.html:124 | `$translation.Language` | attrescaper | `*langs.Language`×897 |
| partials/header.html:125 | `$translation.Permalink` | attrescaper | `string`×897 |
| partials/header.html:127 | `.LanguageName` | htmlescaper | `string`×897 |
| partials/header.html:166 | `safeHTML `\n            <div class="d-flex flex-column container my-3 py-3 shado` | htmlescaper | `template.HTML`×1953 |
| partials/marketing/google/googleGtag.html:2 | `.` | urlescaper → attrescaper | `string`×1953 |
| partials/marketing/google/googleGtag.html:8 | `.` | jsstrescaper | `string`×1953 |
| partials/marketing/google/googleTagManagerHead.html:7 | `.` | jsstrescaper | `string`×1953 |
| partials/marketing/jsonLd.html:9 | `.Permalink` | jsstrescaper | `string`×229 |
| partials/marketing/jsonLd.html:12 | `.Site.Params.BaseURL` | jsstrescaper | `string`×229 |
| partials/marketing/jsonLd.html:13 | `.Title` | jsstrescaper | `string`×229 |
| partials/marketing/jsonLd.html:15 | `partial "marketing/jsonld/breadcrumb" . \| safeJS` | jsvalescaper | `template.JS`×229 |
| partials/marketing/jsonLd.html:23 | `$e.link` | jsstrescaper | `string`×1603 |
| partials/marketing/jsonLd.html:29 | `.Site.Title` | jsstrescaper | `string`×229 |
| partials/marketing/jsonLd.html:30 | `.Site.Params.baseURLSearch` | jsstrescaper | `string`×229 |
| partials/marketing/jsonLd.html:35 | `.Site.Params.company` | jsstrescaper | `string`×229 |
| partials/marketing/jsonLd.html:36 | `.Site.BaseURL` | jsstrescaper | `string`×229 |
| partials/marketing/jsonLd.html:42 | `$e.link` | jsstrescaper | `string`×1603 |
| partials/marketing/jsonLd.html:48 | `.Site.Title` | jsstrescaper | `string`×229 |
| partials/marketing/jsonLd.html:49 | `$imageLogo.Permalink` | jsvalescaper | `string`×229 |
| partials/marketing/jsonLd.html:52 | `.Site.Params.company` | jsstrescaper | `string`×229 |
| partials/marketing/jsonLd.html:55 | `.Site.Params.author` | jsstrescaper | `string`×229 |
| partials/marketing/jsonLd.html:63 | `.Site.BaseURL` | jsstrescaper | `string`×229 |
| partials/marketing/jsonLd.html:64 | `.Site.BaseURL` | jsstrescaper | `string`×229 |
| partials/marketing/jsonLd.html:65 | `.Title` | jsstrescaper | `string`×229 |
| partials/marketing/jsonLd.html:68 | `.Site.Params.author` | jsstrescaper | `string`×229 |
| partials/marketing/jsonLd.html:70 | `.Site.Params.description` | jsvalescaper | `string`×229 |
| partials/marketing/jsonLd.html:76 | `$e.link` | jsstrescaper | `string`×1603 |
| partials/marketing/jsonLd.html:82 | `.Site.Title` | jsstrescaper | `string`×229 |
| partials/marketing/jsonLd.html:83 | `.Site.Params.baseURLSearch` | jsstrescaper | `string`×229 |
| partials/marketing/jsonLd.html:88 | `.Site.Params.company` | jsstrescaper | `string`×229 |
| partials/marketing/jsonLd.html:89 | `.Site.BaseURL` | jsstrescaper | `string`×229 |
| partials/marketing/jsonLd.html:95 | `$e.link` | jsstrescaper | `string`×1603 |
| partials/marketing/jsonLd.html:101 | `.Site.Title` | jsstrescaper | `string`×229 |
| partials/marketing/jsonLd.html:102 | `$imageLogo.Permalink` | jsvalescaper | `string`×229 |
| partials/marketing/jsonLd.html:105 | `.Site.Params.company` | jsstrescaper | `string`×229 |
| partials/marketing/jsonLd.html:108 | `.Site.Params.author` | jsstrescaper | `string`×229 |
| partials/marketing/jsonLd.html:117 | `.Title` | jsstrescaper | `string`×229 |
| partials/marketing/jsonLd.html:120 | `.Title` | jsstrescaper | `string`×229 |
| partials/marketing/jsonLd.html:121 | `printf "%s%s" .Permalink .Params.Image \| absLangURL` | jsstrescaper | `string`×229 |
| partials/marketing/jsonLd.html:124 | `.Site.Params.company` | jsstrescaper | `string`×229 |
| partials/marketing/jsonLd.html:127 | `.Site.Params.author` | jsstrescaper | `string`×229 |
| partials/marketing/jsonLd.html:131 | `.Description` | jsstrescaper | `string`×229 |
| partials/marketing/jsonLd.html:132 | `.PublishDate` | jsstrescaper | `time.Time`×229 |
| partials/marketing/jsonLd.html:133 | `.PublishDate` | jsstrescaper | `time.Time`×229 |
| partials/marketing/jsonLd.html:137 | `.` | jsstrescaper | `string`×200 |
| partials/marketing/jsonLd.html:141 | `index . 0` | jsstrescaper | `string`×221, `uint8`×3 |
| partials/marketing/jsonLd.html:157 | `.Params.Rating_tastytaste` | jsstrescaper | `int`×2 |
| partials/marketing/jsonLd.html:165 | `.Site.Params.author` | jsvalescaper | `string`×229 |
| partials/marketing/jsonLd.html:169 | `.Site.Params.company` | jsvalescaper | `string`×229 |
| partials/marketing/jsonLd.html:170 | `.Site.BaseURL` | jsvalescaper | `string`×229 |
| partials/marketing/jsonLd.html:176 | `$e.link` | jsstrescaper | `string`×1603 |
| partials/marketing/jsonLd.html:182 | `.Site.Title` | jsstrescaper | `string`×229 |
| partials/marketing/jsonLd.html:183 | `$imageLogo.Permalink` | jsvalescaper | `string`×229 |
| partials/marketing/jsonLd.html:186 | `.Site.Params.company` | jsstrescaper | `string`×229 |
| partials/marketing/jsonLd.html:189 | `.Site.Params.author` | jsstrescaper | `string`×229 |
| partials/marketing/jsonLd.html:195 | `.` | jsvalescaper | `string`×221, `[]string`×3 |
| partials/marketing/jsonLd.html:196 | `$e` | jsvalescaper | `string`×797, `<invalid>`×1 |
| partials/marketing/jsonLd.html:209 | `$ytItem.snippet.title` | jsstrescaper | `string`×87 |
| partials/marketing/jsonLd.html:210 | `$ytItem.snippet.description` | jsstrescaper | `string`×87 |
| partials/marketing/jsonLd.html:211 | `$ytItem.snippet.thumbnails.standard.url` | jsstrescaper | `string`×87 |
| partials/marketing/jsonLd.html:212 | `$ytItem.snippet.publishedAt` | jsstrescaper | `string`×87 |
| partials/marketing/jsonLd.html:213 | `$ytItem.contentDetails.duration` | jsstrescaper | `string`×87 |
| partials/marketing/jsonLd.html:216 | `$page.Site.Params.company` | jsstrescaper | `string`×87 |
| partials/marketing/jsonLd.html:217 | `$page.Site.BaseURL` | jsstrescaper | `string`×87 |
| partials/marketing/jsonLd.html:223 | `$e.link` | jsstrescaper | `string`×609 |
| partials/marketing/jsonLd.html:229 | `$page.Site.Title` | jsstrescaper | `string`×87 |
| partials/marketing/jsonLd.html:230 | `$imageLogo.Permalink` | jsvalescaper | `string`×87 |
| partials/marketing/jsonLd.html:233 | `$page.Site.Params.company` | jsstrescaper | `string`×87 |
| partials/marketing/jsonLd.html:236 | `$page.Site.Params.author` | jsstrescaper | `string`×87 |
| partials/marketing/jsonLd.html:241 | `$ytItem.snippet.tags` | jsstrescaper | `[]interface {}`×69, `<invalid>`×18 |
| partials/marketing/jsonLd.html:242 | `$page.Params.youtube_video` | jsstrescaper | `string`×87 |
| partials/marketing/jsonLd.html:243 | `$page.Params.youtube_video` | jsstrescaper | `string`×87 |
| partials/marketing/jsonLd.html:244 | `$ytItem.statistics.commentCount` | jsstrescaper | `string`×87 |
| partials/marketing/jsonLd.html:245 | `$ytItem.statistics.viewCount` | jsstrescaper | `string`×87 |
| partials/marketing/jsonLd.html:248 | `$page.Site.Title` | jsstrescaper | `string`×87 |
| partials/marketing/jsonLd.html:249 | `$page.Site.Params.baseURLSearch` | jsstrescaper | `string`×87 |
| partials/marketing/jsonLd.html:261 | `.Site.BaseURL` | jsstrescaper | `string`×1724 |
| partials/marketing/jsonLd.html:262 | `.Site.BaseURL` | jsstrescaper | `string`×1724 |
| partials/marketing/jsonLd.html:263 | `.Title` | jsstrescaper | `string`×1703 |
| partials/marketing/jsonLd.html:263 | `.Site.Params.HomeTitle` | jsstrescaper | `string`×21 |
| partials/marketing/jsonLd.html:266 | `.Site.Params.author` | jsstrescaper | `string`×1724 |
| partials/marketing/jsonLd.html:270 | `.Site.Params.BaseURL` | jsstrescaper | `string`×1724 |
| partials/marketing/jsonLd.html:271 | `.Title` | jsstrescaper | `string`×1703 |
| partials/marketing/jsonLd.html:271 | `.Site.Params.HomeTitle` | jsstrescaper | `string`×21 |
| partials/marketing/jsonLd.html:273 | `partial "marketing/jsonld/breadcrumb" . \| safeJS` | jsvalescaper | `template.JS`×1724 |
| partials/marketing/jsonLd.html:276 | `.Site.Params.description` | jsvalescaper | `string`×1724 |
| partials/marketing/jsonLd.html:282 | `$e.link` | jsstrescaper | `string`×12068 |
| partials/marketing/jsonLd.html:288 | `.Site.Title` | jsstrescaper | `string`×1724 |
| partials/marketing/jsonLd.html:289 | `.Site.Params.baseURLSearch` | jsstrescaper | `string`×1724 |
| partials/marketing/jsonLd.html:294 | `.Site.Params.company` | jsstrescaper | `string`×1724 |
| partials/marketing/jsonLd.html:295 | `.Site.BaseURL` | jsstrescaper | `string`×1724 |
| partials/marketing/jsonLd.html:301 | `$e.link` | jsstrescaper | `string`×12068 |
| partials/marketing/jsonLd.html:307 | `.Site.Title` | jsstrescaper | `string`×1724 |
| partials/marketing/jsonLd.html:308 | `$imageLogo.Permalink` | jsvalescaper | `string`×1724 |
| partials/marketing/jsonLd.html:311 | `.Site.Params.company` | jsstrescaper | `string`×1724 |
| partials/marketing/jsonLd.html:314 | `.Site.Params.author` | jsstrescaper | `string`×1724 |
| partials/marketing/jsonLd.html:321 | `.Site.BaseURL` | jsstrescaper | `string`×1724 |
| partials/marketing/jsonLd.html:322 | `.Site.BaseURL` | jsstrescaper | `string`×1724 |
| partials/marketing/jsonLd.html:323 | `.Title` | jsstrescaper | `string`×1703 |
| partials/marketing/jsonLd.html:323 | `.Site.Params.HomeTitle` | jsstrescaper | `string`×21 |
| partials/marketing/jsonLd.html:326 | `.Site.Params.author` | jsstrescaper | `string`×1724 |
| partials/marketing/jsonLd.html:328 | `.Site.Params.description` | jsvalescaper | `string`×1724 |
| partials/marketing/jsonLd.html:334 | `$e.link` | jsstrescaper | `string`×12068 |
| partials/marketing/jsonLd.html:340 | `.Site.Title` | jsstrescaper | `string`×1724 |
| partials/marketing/jsonLd.html:341 | `.Site.Params.baseURLSearch` | jsstrescaper | `string`×1724 |
| partials/marketing/jsonLd.html:346 | `.Site.Params.company` | jsstrescaper | `string`×1724 |
| partials/marketing/jsonLd.html:347 | `.Site.BaseURL` | jsstrescaper | `string`×1724 |
| partials/marketing/jsonLd.html:353 | `$e.link` | jsstrescaper | `string`×12068 |
| partials/marketing/jsonLd.html:359 | `.Site.Title` | jsstrescaper | `string`×1724 |
| partials/marketing/jsonLd.html:360 | `$imageLogo.Permalink` | jsvalescaper | `string`×1724 |
| partials/marketing/jsonLd.html:363 | `.Site.Params.company` | jsstrescaper | `string`×1724 |
| partials/marketing/jsonLd.html:366 | `.Site.Params.author` | jsstrescaper | `string`×1724 |
| partials/marketing/jsonLd.html:379 | `$scratch.Set "pages" slice` | htmlescaper | `string`×1953 |
| partials/marketing/jsonLd.html:380 | `$scratch.Set "reversed" slice` | htmlescaper | `string`×1953 |
| partials/marketing/jsonLd.html:381 | `$scratch.Set "current" .` | htmlescaper | `string`×1953 |
| partials/marketing/jsonLd.html:388 | `$scratch.Add "reversed" ($scratch.Get "current")` | htmlescaper | `string`×5598 |
| partials/marketing/jsonLd.html:389 | `$scratch.Set "current" ($scratch.Get "current").Parent` | htmlescaper | `string`×5598 |
| partials/marketing/jsonLd.html:395 | `$scratch.Set "position" (len ($scratch.Get "reversed"))` | htmlescaper | `string`×1953 |
| partials/marketing/jsonLd.html:397 | `$scratch.Add "pages" (dict "position" ($scratch.Get "position") "page" .)` | htmlescaper | `string`×5598 |
| partials/marketing/jsonLd.html:398 | `$scratch.Set "position" (sub ($scratch.Get "position") 1)` | htmlescaper | `string`×5598 |
| partials/marketing/jsonLd.html:405 | `.position` | htmlescaper | `int64`×3645, `int`×1953 |
| partials/marketing/jsonLd.html:407 | `.page.Title` | htmlescaper | `string`×3645 |
| partials/marketing/jsonLd.html:409 | `.page.Site.Title` | htmlescaper | `string`×1953 |
| partials/marketing/jsonLd.html:411 | `.page.Permalink` | htmlescaper | `string`×5598 |
| partials/pagination.html:29 | `.URL` | urlfilter → urlnormalizer → attrescaper | `string`×203 |
| partials/pagination.html:35 | `$pag.Prev.URL` | urlfilter → urlnormalizer → attrescaper | `string`×203 |
| partials/pagination.html:76 | `.PageNumber` | htmlescaper | `int`×270 |
| partials/pagination.html:83 | `.PageNumber` | htmlescaper | `int`×908 |
| partials/pagination.html:83 | `.URL` | urlfilter → urlnormalizer → attrescaper | `string`×908 |
| partials/pagination.html:94 | `$pag.Next.URL` | urlfilter → urlnormalizer → attrescaper | `string`×203 |
| partials/pagination.html:108 | `.URL` | urlfilter → urlnormalizer → attrescaper | `string`×203 |
| partials/rating/rating.html:2 | `i18n "rating"` | htmlescaper | `string`×200 |
| partials/rating/rating.html:23 | `partial "rating/rating-review" (dict "context" . "ratingName" "Tasty Taste")` | htmlescaper | `template.HTML`×199 |
| partials/rating/rating.html:26 | `partial "rating/rating-review" (dict "context" . "ratingName" "Sweet")` | htmlescaper | `template.HTML`×139 |
| partials/rating/rating.html:29 | `partial "rating/rating-review" (dict "context" . "ratingName" "Sour")` | htmlescaper | `template.HTML`×15 |
| partials/rating/rating.html:32 | `partial "rating/rating-review" (dict "context" . "ratingName" "Salty")` | htmlescaper | `template.HTML`×36 |
| partials/rating/rating.html:35 | `partial "rating/rating-review" (dict "context" . "ratingName" "Bitter")` | htmlescaper | `template.HTML`×1 |
| partials/rating/rating.html:38 | `partial "rating/rating-review" (dict "context" . "ratingName" "Savory")` | htmlescaper | `template.HTML`×6 |
| partials/rating/rating.html:41 | `partial "rating/rating-review" (dict "context" . "ratingName" "Crispy")` | htmlescaper | `template.HTML`×121 |
| partials/rating/rating.html:44 | `partial "rating/rating-review" (dict "context" . "ratingName" "Chewy")` | htmlescaper | `template.HTML`×1 |
| partials/rating/rating.html:47 | `partial "rating/rating-review" (dict "context" . "ratingName" "Juicy")` | htmlescaper | `template.HTML`×10 |
| partials/rating/rating.html:50 | `partial "rating/rating-review" (dict "context" . "ratingName" "Spicy")` | htmlescaper | `template.HTML`×32 |
| partials/rating/rating.html:53 | `partial "rating/rating-review" (dict "context" . "ratingName" "Smell")` | htmlescaper | `template.HTML`×196 |
| partials/rating/rating.html:56 | `partial "rating/rating-review" (dict "context" . "ratingName" "Quantity")` | htmlescaper | `template.HTML`×198 |
| partials/rating/rating.html:59 | `partial "rating/rating-review" (dict "context" . "ratingName" "Package")` | htmlescaper | `template.HTML`×198 |
| partials/rating/rating.html:68 | `.ratingName` | htmlescaper | `string`×1152 |
| partials/related.html:7 | `.Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×636 |
| partials/related.html:21 | `$imagewebp.RelPermalink` | srcsetescaper → attrescaper | `string`×636 |
| partials/related.html:26 | `$imageFile.RelPermalink` | srcsetescaper → attrescaper | `string`×636 |
| partials/related.html:36 | `$imageFile.RelPermalink` | urlfilter → urlnormalizer → attrescaper | `string`×636 |
| partials/related.html:37 | `.Title` | attrescaper | `string`×636 |
| partials/related.html:43 | `.Params.Title` | htmlescaper | `string`×636 |
| partials/related.html:48 | `.Description` | htmlescaper | `string`×636 |
| partials/single/author.html:3 | `(printf "authors/%s/" (. \| urlize)) \| relLangURL` | urlfilter → urlnormalizer → attrescaper | `string`×213 |
| partials/single/author.html:4 | `.` | htmlescaper | `string`×213 |
| partials/single/date.html:6 | `dateFormat (default "2006-01-02T15:04:05Z07:00" $siteDateFormat) .context` | attrescaper | `string`×231 |
| partials/single/date.html:7 | `dateFormat (default "Jan 2, 2006" $siteDateFormat) .context` | htmlescaper | `string`×231 |
| partials/single/ingredientslist.html:22 | `$ingredients.name` | htmlescaper | `string`×1357 |
| partials/single/ingredientslist.html:23 | `$ingredients.percentage` | htmlescaper | `string`×735, `<invalid>`×622 |
| partials/single/nutritionfacts.html:4 | `i18n "nutrition_fact"` | htmlescaper | `string`×126 |
| partials/single/nutritionfacts.html:15 | `i18n "nutrition_fact"` | htmlescaper | `string`×63 |
| partials/single/nutritionfacts.html:17 | `i18n "nutrition_fact"` | htmlescaper | `string`×63 |
| partials/single/nutritionfacts.html:21 | `i18n "servings_per_container" (dict "Context" .)` | htmlescaper | `string`×126 |
| partials/single/nutritionfacts.html:26 | `i18n "serving_size" (dict "Context" .)` | htmlescaper | `string`×126 |
| partials/single/nutritionfacts.html:33 | `i18n "amount_per_serving"` | htmlescaper | `string`×116 |
| partials/single/nutritionfacts.html:38 | `i18n "calories" (dict "Context" .)` | htmlescaper | `string`×58 |
| partials/single/nutritionfacts.html:42 | `i18n "calories"` | htmlescaper | `string`×58 |
| partials/single/nutritionfacts.html:45 | `$calorie` | htmlescaper | `int`×58 |
| partials/single/nutritionfacts.html:54 | `i18n "daily_value"` | htmlescaper | `string`×126 |
| partials/single/nutritionfacts.html:62 | `i18n "total_fat"` | htmlescaper | `string`×112 |
| partials/single/nutritionfacts.html:63 | `i18n "gram" (dict "Context" .)` | htmlescaper | `string`×112 |
| partials/single/nutritionfacts.html:67 | `.` | htmlescaper | `int`×112 |
| partials/single/nutritionfacts.html:76 | `i18n "saturated_fat"` | htmlescaper | `string`×66 |
| partials/single/nutritionfacts.html:77 | `i18n "gram" (dict "Context" .)` | htmlescaper | `string`×66 |
| partials/single/nutritionfacts.html:82 | `.` | htmlescaper | `int`×66 |
| partials/single/nutritionfacts.html:152 | `i18n "cholesterol"` | htmlescaper | `string`×46 |
| partials/single/nutritionfacts.html:153 | `i18n "milligram" (dict "Context" .)` | htmlescaper | `string`×46 |
| partials/single/nutritionfacts.html:158 | `.` | htmlescaper | `int`×44 |
| partials/single/nutritionfacts.html:169 | `i18n "sodium"` | htmlescaper | `string`×112 |
| partials/single/nutritionfacts.html:170 | `i18n "milligram" (dict "Context" .)` | htmlescaper | `string`×112 |
| partials/single/nutritionfacts.html:175 | `.` | htmlescaper | `int`×110 |
| partials/single/nutritionfacts.html:186 | `i18n "potassium"` | htmlescaper | `string`×10 |
| partials/single/nutritionfacts.html:187 | `i18n "milligram" (dict "Context" .)` | htmlescaper | `string`×10 |
| partials/single/nutritionfacts.html:192 | `.` | htmlescaper | `int`×10 |
| partials/single/nutritionfacts.html:203 | `i18n "total_carbohydrate"` | htmlescaper | `string`×122 |
| partials/single/nutritionfacts.html:204 | `i18n "gram" (dict "Context" .)` | htmlescaper | `string`×122 |
| partials/single/nutritionfacts.html:209 | `.` | htmlescaper | `int`×116 |
| partials/single/nutritionfacts.html:218 | `i18n "dietary_fiber"` | htmlescaper | `string`×46 |
| partials/single/nutritionfacts.html:219 | `i18n "gram" (dict "Context" .)` | htmlescaper | `string`×46 |
| partials/single/nutritionfacts.html:224 | `.` | htmlescaper | `int`×42 |
| partials/single/nutritionfacts.html:235 | `i18n "toatl_sugars"` | htmlescaper | `string`×116 |
| partials/single/nutritionfacts.html:236 | `i18n "gram" (dict "Context" .)` | htmlescaper | `string`×116 |
| partials/single/nutritionfacts.html:245 | `i18n "added_sugars" (dict "Context" .)` | htmlescaper | `string`×2 |
| partials/single/nutritionfacts.html:265 | `i18n "protein"` | htmlescaper | `string`×108 |
| partials/single/nutritionfacts.html:266 | `i18n "gram" (dict "Context" .)` | htmlescaper | `string`×108 |
| partials/single/nutritionfacts.html:290 | `i18n "vitamins_a"` | htmlescaper | `string`×10 |
| partials/single/nutritionfacts.html:291 | `i18n "microgram" (dict "Context" .)` | htmlescaper | `string`×2 |
| partials/single/nutritionfacts.html:295 | `.` | htmlescaper | `int`×8 |
| partials/single/nutritionfacts.html:306 | `i18n "thiamin"` | htmlescaper | `string`×20 |
| partials/single/nutritionfacts.html:307 | `i18n "milligram" (dict "Context" .)` | htmlescaper | `string`×8 |
| partials/single/nutritionfacts.html:311 | `.` | htmlescaper | `int`×12 |
| partials/single/nutritionfacts.html:322 | `i18n "riboflavin"` | htmlescaper | `string`×18 |
| partials/single/nutritionfacts.html:323 | `i18n "milligram" (dict "Context" .)` | htmlescaper | `string`×14 |
| partials/single/nutritionfacts.html:327 | `.` | htmlescaper | `int`×4 |
| partials/single/nutritionfacts.html:514 | `i18n "calcium"` | htmlescaper | `string`×10 |
| partials/single/nutritionfacts.html:515 | `i18n "milligram" (dict "Context" .)` | htmlescaper | `string`×4 |
| partials/single/nutritionfacts.html:519 | `.` | htmlescaper | `int`×6 |
| partials/single/nutritionfacts.html:610 | `i18n "iron"` | htmlescaper | `string`×22 |
| partials/single/nutritionfacts.html:611 | `i18n "milligram" (dict "Context" .)` | htmlescaper | `string`×8 |
| partials/single/nutritionfacts.html:615 | `.` | htmlescaper | `int`×14 |
| partials/single/nutritionfacts.html:724 | `.` | htmlescaper | `string`×1 |
| partials/single/smell.html:2 | `i18n "smell"` | htmlescaper | `string`×3 |
| partials/single/smell.html:11 | `.` | htmlescaper | `string`×3 |
| partials/single/socialshare.html:5 | `.Permalink` | urlescaper → attrescaper | `string`×1714 |
| partials/single/socialshare.html:13 | `.Permalink` | urlescaper → attrescaper | `string`×1714 |
| partials/single/socialshare.html:13 | `.Title` | urlescaper → attrescaper | `string`×1714 |
| partials/single/socialshare.html:22 | `.Permalink` | urlescaper → attrescaper | `string`×1714 |
| partials/single/socialshare.html:30 | `.Permalink` | urlescaper → attrescaper | `string`×1714 |
| partials/single/socialshare.html:38 | `.Permalink` | urlescaper → attrescaper | `string`×1714 |
| partials/single/socialshare.html:47 | `.Description` | urlescaper → attrescaper | `string`×1714 |
| partials/single/socialshare.html:47 | `.Permalink` | urlescaper → attrescaper | `string`×1714 |
| partials/single/socialshare.html:47 | `.Title` | urlescaper → attrescaper | `string`×1714 |
| partials/single/socialshare.html:56 | `.Permalink` | urlescaper → attrescaper | `string`×1714 |
| partials/single/socialshare.html:56 | `.Title` | urlescaper → attrescaper | `string`×1714 |
| partials/single/socialshare.html:56 | `"Check out this site "` | urlescaper → html | `string`×1714 |
| partials/single/taste.html:2 | `i18n "taste"` | htmlescaper | `string`×3 |
| partials/single/taste.html:11 | `.` | htmlescaper | `string`×3 |
| partials/single/whenseen.html:11 | `$dateFormat` | attrescaper | `<invalid>`×39 |
| partials/single/whenseen.html:12 | `dateFormat (default "Jan 2, 2006" $siteDateFormat) .context` | htmlescaper | `string`×39 |
| partials/taxonomy/brands.html:3 | `"brands/" \| relLangURL` | urlfilter → urlnormalizer → attrescaper | `string`×202 |
| partials/taxonomy/brands.html:31 | `(printf "brands/%s/" (. \| urlize)) \| relLangURL` | urlfilter → urlnormalizer → attrescaper | `string`×202 |
| partials/taxonomy/brands.html:33 | `.` | htmlescaper | `string`×202 |
| partials/taxonomy/categories.html:15 | `(printf "categories/%s/" ($category \| urlize \| lower)) \| relLangURL` | urlfilter → urlnormalizer → attrescaper | `string`×323 |
| partials/taxonomy/categories.html:17 | `(printf "%s" $category) \| humanize` | htmlescaper | `string`×323 |
| partials/taxonomy/categories.html:23 | `(printf "categories/%s/" (. \| urlize \| lower)) \| relLangURL` | urlfilter → urlnormalizer → attrescaper | `string`×3 |
| partials/taxonomy/categories.html:25 | `. \| humanize` | htmlescaper | `string`×3 |
| partials/taxonomy/companies.html:3 | `"companies/" \| relLangURL` | urlfilter → urlnormalizer → attrescaper | `string`×226 |
| partials/taxonomy/companies.html:14 | `(printf "companies/%s/" ($val \| urlize)) \| relLangURL` | urlfilter → urlnormalizer → attrescaper | `string`×3 |
| partials/taxonomy/companies.html:16 | `(printf "%s" $val) \| humanize` | htmlescaper | `string`×3 |
| partials/taxonomy/companies.html:22 | `(printf "companies/%s/" (. \| urlize)) \| relLangURL` | urlfilter → urlnormalizer → attrescaper | `string`×223 |
| partials/taxonomy/companies.html:24 | `.` | htmlescaper | `string`×223 |
| partials/taxonomy/company/facebook.html:32 | `(printf "https://www.facebook.com/%s/" (.)) \| relLangURL` | urlfilter → urlnormalizer → attrescaper | `string`×5 |
| partials/taxonomy/company/facebook.html:34 | `.` | htmlescaper | `string`×5 |
| partials/taxonomy/company/instagram.html:32 | `(printf "https://www.instagram.com/%s/" (.)) \| relLangURL` | urlfilter → urlnormalizer → attrescaper | `string`×5 |
| partials/taxonomy/company/instagram.html:34 | `.` | htmlescaper | `string`×5 |
| partials/taxonomy/company/twitter.html:31 | `(printf "https://twitter.com/%s/" (.)) \| relLangURL` | urlfilter → urlnormalizer → attrescaper | `string`×5 |
| partials/taxonomy/company/twitter.html:33 | `.` | htmlescaper | `string`×5 |
| partials/taxonomy/company/website.html:31 | `(printf "%s" (.)) \| relLangURL` | urlfilter → urlnormalizer → attrescaper | `string`×7 |
| partials/taxonomy/company/website.html:33 | `.` | htmlescaper | `string`×7 |
| partials/taxonomy/company/youtube.html:31 | `(printf "https://www.youtube.com/%s/" (.)) \| relLangURL` | urlfilter → urlnormalizer → attrescaper | `string`×5 |
| partials/taxonomy/company/youtube.html:33 | `.` | htmlescaper | `string`×5 |
| partials/taxonomy/countries.html:3 | `"countries/" \| relLangURL` | urlfilter → urlnormalizer → attrescaper | `string`×155 |
| partials/taxonomy/countries.html:14 | `(printf "countries/%s/" ($val \| urlize)) \| relLangURL` | urlfilter → urlnormalizer → attrescaper | `string`×155 |
| partials/taxonomy/countries.html:16 | `(printf "%s" $val) \| humanize` | htmlescaper | `string`×155 |
| partials/taxonomy/ingredients.html:12 | `(printf "ingredients/%s/" ($val \| urlize \| lower)) \| relLangURL` | urlfilter → urlnormalizer → attrescaper | `string`×2255 |
| partials/taxonomy/ingredients.html:14 | `$val` | htmlescaper | `string`×2255 |
| partials/taxonomy/tags.html:13 | `(printf "tags/%s/" ($val \| urlize \| lower)) \| relLangURL` | urlfilter → urlnormalizer → attrescaper | `string`×781 |
| partials/taxonomy/tags.html:15 | `$val` | htmlescaper | `string`×780, `<invalid>`×1 |
| sitemap.xml:1 | `printf "<?xml version=\"1.0\" encoding=\"utf-8\" standalone=\"yes\" ?>" \| safeH` | htmlescaper | `template.HTML`×2 |
| sitemap.xml:7 | `.Permalink` | htmlescaper | `string`×1730 |
| sitemap.xml:8 | `safeHTML (.Lastmod.Format "2006-01-02T15:04:05-07:00")` | htmlescaper | `template.HTML`×1720 |
| sitemap.xml:9 | `.` | htmlescaper | `string`×1730 |
| sitemap.xml:14 | `.Site.Params.Sitemap.Priority.Section` | htmlescaper | `float64`×66 |
| sitemap.xml:16 | `.Site.Params.Sitemap.Priority.term` | htmlescaper | `float64`×1411 |
| sitemap.xml:18 | `.Site.Params.Sitemap.Priority.page` | htmlescaper | `int64`×239 |
| sitemap.xml:25 | `.Lang` | attrescaper | `string`×716 |
| sitemap.xml:26 | `.Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×716 |
| sitemap.xml:30 | `.Lang` | attrescaper | `string`×716 |
| sitemap.xml:31 | `.Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×716 |
| taxonomy/list.html:2 | `partial "ads/adsensehead.html" .` | htmlescaper | `template.HTML`×123 |
| taxonomy/list.html:12 | `$jsWebsiteTheme.Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×123 |
| taxonomy/list.html:18 | `.Title` | htmlescaper | `string`×123 |
| taxonomy/list.html:20 | `partial "ads/adsensemanual.html" .` | htmlescaper | `template.HTML`×123 |
| taxonomy/list.html:21 | `partial "breadcrumb" .` | htmlescaper | `template.HTML`×123 |
| taxonomy/list.html:23 | `$tanaxomies` | htmlescaper | `string`×123 |
| taxonomy/list.html:38 | `.RelPermalink` | urlfilter → urlnormalizer → attrescaper | `string`×9 |
| taxonomy/list.html:48 | `$imagewebp.RelPermalink` | srcsetescaper → attrescaper | `string`×9 |
| taxonomy/list.html:53 | `$imageFile.RelPermalink` | srcsetescaper → attrescaper | `string`×4 |
| taxonomy/list.html:58 | `$imageFile.RelPermalink` | srcsetescaper → attrescaper | `string`×5 |
| taxonomy/list.html:63 | `$imageFile.RelPermalink` | urlfilter → urlnormalizer → attrescaper | `string`×9 |
| taxonomy/list.html:64 | `$page.Params.title` | attrescaper | `string`×9 |
| taxonomy/list.html:69 | `.RelPermalink` | urlfilter → urlnormalizer → attrescaper | `string`×9 |
| taxonomy/list.html:70 | `.Title` | htmlescaper | `string`×9 |
| taxonomy/list.html:72 | `.Description` | htmlescaper | `string`×9 |
| taxonomy/list.html:73 | `.RelPermalink` | urlfilter → urlnormalizer → attrescaper | `string`×9 |
| taxonomy/list.html:86 | `partial "pagination.html" .` | htmlescaper | `template.HTML`×123 |
| taxonomy/list.html:87 | `partial "ads/adsensemanual.html" .` | htmlescaper | `template.HTML`×123 |
| taxonomy/list.html:125 | `$script.Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×123 |
| term/term.html:2 | `partial "ads/adsensehead.html" .` | htmlescaper | `template.HTML`×1479 |
| term/term.html:5 | `.Title` | rcdataescaper | `string`×1479 |
| term/term.html:6 | `.Site.Title` | rcdataescaper | `string`×1479 |
| term/term.html:16 | `$jsWebsiteTheme.Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×1479 |
| term/term.html:22 | `partial "ads/adsensemanual.html" .` | htmlescaper | `template.HTML`×1479 |
| term/term.html:23 | `partial "breadcrumb" .` | htmlescaper | `template.HTML`×1479 |
| term/term.html:24 | `.Title` | htmlescaper | `string`×1479 |
| term/term.html:26 | `.Description` | htmlescaper | `string`×1479 |
| term/term.html:39 | `$imagewebp.RelPermalink` | srcsetescaper → attrescaper | `string`×9 |
| term/term.html:43 | `$imageFile.RelPermalink` | srcsetescaper → attrescaper | `string`×4 |
| term/term.html:46 | `$imageFile.RelPermalink` | srcsetescaper → attrescaper | `string`×5 |
| term/term.html:50 | `$imageFile.RelPermalink` | urlfilter → urlnormalizer → attrescaper | `string`×9 |
| term/term.html:51 | `$title` | attrescaper | `string`×9 |
| term/term.html:65 | `partial "taxonomy/company/website" .` | htmlescaper | `template.HTML`×7 |
| term/term.html:68 | `partial "taxonomy/company/facebook" .` | htmlescaper | `template.HTML`×5 |
| term/term.html:71 | `partial "taxonomy/company/twitter" .` | htmlescaper | `template.HTML`×5 |
| term/term.html:74 | `partial "taxonomy/company/instagram" .` | htmlescaper | `template.HTML`×5 |
| term/term.html:77 | `partial "taxonomy/company/youtube" .` | htmlescaper | `template.HTML`×5 |
| term/term.html:79 | `partial "taxonomy/tags" .` | htmlescaper | `template.HTML`×3 |
| term/term.html:82 | `.Content` | htmlescaper | `template.HTML`×1479 |
| term/term.html:100 | `$e` | urlfilter → urlnormalizer → attrescaper | `string`×8 |
| term/term.html:101 | `$e` | htmlescaper | `string`×8 |
| term/term.html:108 | `partial "single/socialshare" .` | htmlescaper | `template.HTML`×1479 |
| term/term.html:110 | `partial "ads/adsensemanual.html" .` | htmlescaper | `template.HTML`×1479 |
| term/term.html:113 | `humanize .Data.Singular` | htmlescaper | `string`×1479 |
| term/term.html:125 | `$page.RelPermalink` | urlfilter → urlnormalizer → attrescaper | `string`×3913 |
| term/term.html:136 | `$imagewebp.RelPermalink` | srcsetescaper → attrescaper | `string`×3913 |
| term/term.html:141 | `$imageFile.RelPermalink` | srcsetescaper → attrescaper | `string`×3913 |
| term/term.html:151 | `$imageFile.RelPermalink` | urlfilter → urlnormalizer → attrescaper | `string`×3913 |
| term/term.html:152 | `$page.Title` | attrescaper | `string`×3913 |
| term/term.html:157 | `$page.RelPermalink` | urlfilter → urlnormalizer → attrescaper | `string`×3913 |
| term/term.html:158 | `$page.Title` | htmlescaper | `string`×3913 |
| term/term.html:160 | `$page.Params.Description` | htmlescaper | `string`×3899, `<invalid>`×14 |
| term/term.html:161 | `$page.RelPermalink` | urlfilter → urlnormalizer → attrescaper | `string`×3913 |
| term/term.html:174 | `partial "pagination.html" .` | htmlescaper | `template.HTML`×1479 |
| term/term.html:175 | `partial "comments" .` | htmlescaper | `template.HTML`×1479 |
| term/term.html:223 | `$script.Permalink` | urlfilter → urlnormalizer → attrescaper | `string`×1479 |

### A.7 `template` actions (named template calls)

| site | action | executing (overlay) template | dot type ×count |
|---|---|---|---|
| _default/baseof.html:7 | `{{template "head" .}}` | `taxonomy/list.html` | `*hugolib.pageState`×123 |
| _default/baseof.html:7 | `{{template "head" .}}` | `term/term.html` | `*hugolib.pageState`×1479 |
| _default/baseof.html:7 | `{{template "head" .}}` | `404.html` | `*hugolib.pageState`×21 |
| _default/baseof.html:7 | `{{template "head" .}}` | `index.html` | `*hugolib.pageState`×21 |
| _default/baseof.html:7 | `{{template "head" .}}` | `single.html` | `*hugolib.pageState`×235 |
| _default/baseof.html:7 | `{{template "head" .}}` | `simple.html` | `*hugolib.pageState`×3 |
| _default/baseof.html:7 | `{{template "head" .}}` | `list.html` | `*hugolib.pageState`×71 |
| _default/baseof.html:10 | `{{template "title$htmltemplate_stateRCDATA_elementTitle" .}}` | `taxonomy/list.html` | `*hugolib.pageState`×123 |
| _default/baseof.html:10 | `{{template "title$htmltemplate_stateRCDATA_elementTitle" .}}` | `term/term.html` | `*hugolib.pageState`×1479 |
| _default/baseof.html:10 | `{{template "title$htmltemplate_stateRCDATA_elementTitle" .}}` | `404.html` | `*hugolib.pageState`×21 |
| _default/baseof.html:10 | `{{template "title$htmltemplate_stateRCDATA_elementTitle" .}}` | `index.html` | `*hugolib.pageState`×21 |
| _default/baseof.html:10 | `{{template "title$htmltemplate_stateRCDATA_elementTitle" .}}` | `single.html` | `*hugolib.pageState`×235 |
| _default/baseof.html:10 | `{{template "title$htmltemplate_stateRCDATA_elementTitle" .}}` | `simple.html` | `*hugolib.pageState`×3 |
| _default/baseof.html:10 | `{{template "title$htmltemplate_stateRCDATA_elementTitle" .}}` | `list.html` | `*hugolib.pageState`×71 |
| _default/baseof.html:22 | `{{template "afterbody" .}}` | `taxonomy/list.html` | `*hugolib.pageState`×123 |
| _default/baseof.html:22 | `{{template "afterbody" .}}` | `term/term.html` | `*hugolib.pageState`×1479 |
| _default/baseof.html:22 | `{{template "afterbody" .}}` | `404.html` | `*hugolib.pageState`×21 |
| _default/baseof.html:22 | `{{template "afterbody" .}}` | `index.html` | `*hugolib.pageState`×21 |
| _default/baseof.html:22 | `{{template "afterbody" .}}` | `single.html` | `*hugolib.pageState`×235 |
| _default/baseof.html:22 | `{{template "afterbody" .}}` | `simple.html` | `*hugolib.pageState`×3 |
| _default/baseof.html:22 | `{{template "afterbody" .}}` | `list.html` | `*hugolib.pageState`×71 |
| _default/baseof.html:25 | `{{template "content" .}}` | `taxonomy/list.html` | `*hugolib.pageState`×123 |
| _default/baseof.html:25 | `{{template "content" .}}` | `term/term.html` | `*hugolib.pageState`×1479 |
| _default/baseof.html:25 | `{{template "content" .}}` | `404.html` | `*hugolib.pageState`×21 |
| _default/baseof.html:25 | `{{template "content" .}}` | `index.html` | `*hugolib.pageState`×21 |
| _default/baseof.html:25 | `{{template "content" .}}` | `single.html` | `*hugolib.pageState`×235 |
| _default/baseof.html:25 | `{{template "content" .}}` | `simple.html` | `*hugolib.pageState`×3 |
| _default/baseof.html:25 | `{{template "content" .}}` | `list.html` | `*hugolib.pageState`×71 |
| _default/baseof.html:28 | `{{template "script" .}}` | `taxonomy/list.html` | `*hugolib.pageState`×123 |
| _default/baseof.html:28 | `{{template "script" .}}` | `term/term.html` | `*hugolib.pageState`×1479 |
| _default/baseof.html:28 | `{{template "script" .}}` | `404.html` | `*hugolib.pageState`×21 |
| _default/baseof.html:28 | `{{template "script" .}}` | `index.html` | `*hugolib.pageState`×21 |
| _default/baseof.html:28 | `{{template "script" .}}` | `single.html` | `*hugolib.pageState`×235 |
| _default/baseof.html:28 | `{{template "script" .}}` | `simple.html` | `*hugolib.pageState`×3 |
| _default/baseof.html:28 | `{{template "script" .}}` | `list.html` | `*hugolib.pageState`×71 |
| partials/breadcrumb.html:3 | `{{template "breadcrumbnav" (dict "p1" . "p2" .)}}` | `_partials/breadcrumb.html` | `map[string]interface {}`×1929 |
| partials/breadcrumb.html:8 | `{{template "breadcrumbnav" (dict "p1" .p1.Parent "p2" .p2)}}` | `_partials/breadcrumb.html` | `map[string]interface {}`×3621 |

### A.8 Values printed by `{{ ... }}` actions after escaping (print sites), by type

| printed value type | templates (×count) |
|---|---|
| `hstring.HTML` | EMB×1587 |
| `string` | partials/comments.html×396224, partials/marketing/jsonLd.html×166064, partials/head.html×87599, partials/footer.html×72261, _default/rss.xml×62407, term/term.html×57484, partials/header.html×40536, index.html×33802, partials/single/socialshare.html×20568, _default/baseof.html×10004, sitemap.xml×9762, EMB×8750, partials/breadcrumb.html×8409, partials/ads/adsensemanual.html×7674, _default/single.html×6563, _default/_markup/render-image.html×6300, partials/taxonomy/ingredients.html×4510, partials/related.html×4452, partials/marketing/google/googleGtag.html×3906, partials/single/nutritionfacts.html×3059, partials/pagination.html×2898, _default/_markup/render-heading.html×2890, partials/single/ingredientslist.html×2714, _default/list.html×2666, partials/rating/rating.html×2504, partials/ads/adsensehead.html×1953, partials/marketing/google/googleTagManagerHead.html×1953, partials/taxonomy/tags.html×1562, taxonomy/list.html×1188, partials/carousel.html×735, partials/taxonomy/companies.html×678, partials/taxonomy/categories.html×652, partials/taxonomy/brands.html×606, _default/index.json×478, partials/taxonomy/countries.html×465, partials/single/date.html×462, partials/single/author.html×426, _default/latesturl.html×154, 404.html×105, partials/single/whenseen.html×78, _default/simple.html×15, partials/taxonomy/company/website.html×14, partials/taxonomy/company/facebook.html×10, partials/taxonomy/company/instagram.html×10, partials/taxonomy/company/twitter.html×10, partials/taxonomy/company/youtube.html×10, partials/single/smell.html×6, partials/single/taste.html×6, ASSET×2 |
| `template.HTML` | _default/index.json×2 |
