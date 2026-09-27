# Output and publishing: where every golden file comes from, and how neohugo writes it

Scope: the neohugo Go build of **seeksnack** with `--minify -d <out>`. This spec covers which producer writes each of the
6943 files in `golden/run1`, alias pages, the canonify (absURL) transformer, the publish transformer chain, on-disk path
rules, sitemaps, output formats per page kind, nondeterminism, static copying and the final write step.

Paths used below:

- `NH` = `/Users/blackb1rd/git/github/org/neohugo` (Go source, read only)
- `S` = `/private/tmp/claude-501/-Users-blackb1rd-git-github-org-neohugo/0835ef6d-534f-4ac6-a7eb-50e9240d6eb1/scratchpad`
- `W` = `$S/work/output-publishing` (my experiments)
- `G` = `$S/golden/run1`

Line numbers refer to `NH` at `v0.148.2` (`d5930ba1f`).

---

## 0. Key findings

1. **Every golden file is accounted for.** The classifier `$W/classify_golden.py` (full text in Appendix A) puts all
   6943 files into 27 producer categories with 0 unknown. It byte-checks every static, bundle and asset copy against
   its source. Its kind classification matches the Go `--devMode` trace on all 3203 unique render targets, with 0
   mismatches.
2. **Alias files: 1481, none from front matter.** There are 1478 `…/page/1/index.html` paginator aliases (856 EN, 622
   TH), 2 aliases for the 404 page's paginator (`404/page/1.html`, `th/404/page/1.html`) and 1 main-language redirect
   (`en/index.html` → `/`). No content file sets `aliases`. Every list node (home, section, taxonomy, term) gets a
   `page/1` alias because the `partials/head.html` layout calls `.Paginator` for every node, and so does the 404 page.
   Hugo's "Aliases 866/628" statistic is higher than the file count because it counts the duplicate writes from
   colliding terms.
3. **The canonify transformer runs *before* minify and works on raw template output.** Evidence: the golden output has
   `srcset=https://seeksnack.com/…` unquoted. The srcset branch only rewrites *quoted* values, and minify removes the
   quotes, so the rewrite must have happened first. It rewrites 1940 rendered HTML files. RSS is always canonified but
   has nothing to rewrite in this site. JSON, sitemap and robots output is never canonified. My Python port
   (`$W/absurl_ref.py`) was checked byte for byte against the Go output on all 4900 rendered HTML and RSS files.
4. **The Hugo generator `<meta>` tag is NOT injected.** neohugo inverted the flag (`hugolib/site.go:1483`,
   `pd.AddHugoGeneratorTag = s.conf.DisableHugoGeneratorInject`). The default is false, so the tag is never added. Do
   not inject it.
5. **Nondeterminism: 26 files, not 8.** There are 13 term URLs where two distinct term pages share one output path:
   8 EN, 5 TH, each with `index.html` and `index.xml`. `renderPages` runs `runtime.NumCPU()` workers, so the last
   writer wins at random. With `HUGO_NUMWORKERMULTIPLIER=1` the build is deterministic: the page that is **last in
   byte-lexicographic tree order** wins. `$W/out-seq1` is such a deterministic, cold-cache golden. It is identical to
   `golden/run1` except for 8 collision files.
6. **Cold-cache naming.** The golden output is a *cold* build (no `resources/_gen`). A warm rebuild renames 1444
   processed images and changes 1791 HTML and 2 JSON files that reference them. I confirmed this independently; `specs/images.md` §0.3 has the
   details.
7. **The output depends on the clock.** The footer and RSS copyright use `now.Format "2006"`, which gives "2026" in
   1940 HTML files and 1478 RSS files. The Rust port must use the same current year, or both builds must be pinned with
   `--clock`.
8. **Path rules on disk.** Output paths are raw UTF-8 (never percent-encoded), lower-cased and `paths.Sanitize`d
   per element. Thai stays as is. On darwin, file-system-derived names are NFC-normalized; front-matter term strings
   are not normalized. Links inside documents are percent-encoded with Go `url.URL.EscapedPath` semantics (uppercase
   hex).

---

## 1. Inventory of the golden output (6943 files)

Output of `python3 $W/classify_golden.py $G $S/sites/seeksnack --trace $W/trace.log` (full output in
`$W/classify_run1.txt`, per-file list in `$W/golden_list.tsv`):

| category (producer) | total | en | th | producer / Go code |
|---|---:|---:|---:|---|
| alias:paginator-page1 | 1478 | 856 | 622 | `site_render.go:239-248` renderPaginator → `alias.go:87` |
| resource:processed-image (`_hu_`) | 1461 | 1461 | 0 | image processing (`resources/image.go:459-480` names); published on `.RelPermalink`/`.Permalink` |
| html:term | 1398 | 809 | 589 | `layouts/term/term.html` (plus 13 collision double writes) |
| rss:term | 1398 | 809 | 589 | `layouts/_default/rss.xml` |
| resource:page-bundle-copy | 531 | 531 | 0 | `hugolib/page.go:524-554` renderResources, all content images copied once, at the EN path |
| html:page | 239 | 159 | 80 | `single.html` 235, `simple.html` 3, `latesturl.html` 1 |
| html:taxonomy-paginator-pageN | 111 | 65 | 46 | `site_render.go:250-264` |
| html:term-paginator-pageN | 68 | 39 | 29 | same |
| html:section | 66 | 40 | 26 | `layouts/_default/list.html` |
| rss:section | 66 | 40 | 26 | `rss.xml` |
| static | 23 | 23 | 0 | fsync copy of `static/` (`commands/hugobuilder.go:437-482`) |
| html:404-paginator-pageN | 19 | 13 | 6 | `404/page/N.html` (N=2..14 EN, 2..7 TH) |
| html:home-paginator-pageN | 19 | 13 | 6 | `page/N/index.html` |
| resource:assets (resources.Get + `.Permalink`) | 19 | 19 | 0 | `images/favicon/*` (18 png + favicon.ico) from `assets/images/favicon` |
| html:taxonomy | 12 | 6 | 6 | `layouts/taxonomy/list.html` |
| rss:taxonomy | 12 | 6 | 6 | `rss.xml` |
| html:section-paginator-pageN | 5 | 4 | 1 | |
| resource:js.Build + fingerprint | 3 | 3 | 0 | `js/set-theme.<sha256>.js`, `js/website.<sha256>.js` ×2 |
| alias:404-paginator-page1 | 2 | 1 | 1 | `404/page/1.html`, `th/404/page/1.html` |
| html:404 | 2 | 1 | 1 | `404.html`, `th/404.html` (`layouts/404.html`) |
| html:home | 2 | 1 | 1 | `index.html`, `th/index.html` (`layouts/index.html`) |
| json:home | 2 | 1 | 1 | `index.json`, `th/index.json` (`layouts/_default/index.json`) |
| rss:home | 2 | 1 | 1 | `index.xml`, `th/index.xml` |
| sitemap (site `layouts/sitemap.xml`) | 2 | 1 | 1 | `en/sitemap.xml`, `th/sitemap.xml` |
| alias:main-language-redirect | 1 | 1 | 0 | `en/index.html` (`site_render.go:339-367`) |
| robotstxt (site `layouts/robots.txt`) | 1 | 1 | 0 | `robots.txt` |
| sitemapindex (embedded `sitemapindex.xml`) | 1 | 1 | 0 | `sitemap.xml` |
| **total** | **6943** | | | |

Extension view: 3427 html, 1481 xml, 1169 jpg, 807 webp, 35 png, 5 js, 4 woff2, 4 ttf, 2 txt, 2 json, 2 ico, 2 bak,
1 yml, 1 DS_Store, 1 css.

Cross-check against the Go build summary (`$W/plain.log`):

- **Pages EN 1892 / TH 1337.** This counts one `renderAndWritePage` per non-paginator render. EN: 1023 html (1 home +
  159 page + 40 section + 6 taxonomy + 809 term + 8 collision duplicates) + 1 json + 864 rss + 1 404 + 1 sitemap +
  1 robots + 1 sitemapindex = 1892.
- **Paginator pages EN 134 / TH 88.** These equal the paginator-pageN rows: 65+39+13+13+4 and 46+29+6+6+1.
- **Aliases EN 866 / TH 628.** EN: 856 + 1 (404) + 1 (main redirect) + 8 collision duplicates. TH: 622 + 1 + 5.
- **Non-page files EN 531.** These are the bundle copies.
- **Static files 23.**
- **Processed images 1836.** This counts variations, including the unpublished watermark resizes, not files.

Static files (23, byte-identical to `static/`): `.DS_Store`, `ads.txt`, `favicon.ico`, `pinterest-e4aaf.html`,
`serviceworker.html`, `serviceworker.js`, `yandex_10ba5f74f6b19b4e.html`, `ytss.html`, `css/seeksnack.css`,
`admin/{index.html,index.html.bak,config.yml,config.yml.bak,decap-cms.js}`, `assets/images/logo.png`,
`assets/webfonts/*` (8 ttf/woff2). Static files are **not** transformed: no canonify, no minify.

What is *not* in the output:

- Content dotfiles such as `.gitkeep` in 14 populated sections are not published.
  `source/sourceSpec.go:55-73 IgnoreFile` ignores names that start with `.` or `#`, or end with `~`. Static files do
  not go through this filter, which is why `static/.DS_Store` is published.
- No generated CSS file exists. The SCSS output is inlined in `<style>`.
- Nothing from `node_modules` is published directly. The `node_modules → assets/vendor` mount is consumed by
  `resources.Concat` and `js.Build`.
- `hugo_stats.json` (`writeStats=true`) goes to the **site root** (`hugo_sites_build.go:749`), not to the publish
  dir. It is only rewritten if its content changed (`:751-756`).
- `.hugo_build.lock` and `resources/_gen/**` also go to the site root.

---

## 2. Aliases

### 2.1 Template: `tpl/tplimpl/embedded/templates/alias.html` (10 lines)

```
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

This is Go **html/template**, so the value is context-escaped: HTML-text escaping in `<title>`, URL
normalizer + attribute escaper in `href`, attribute escaper in `content`. In this site every alias URL contains only
`[A-Za-z0-9/:.%-]`, so escaping is a no-op. `site.Language.LanguageCode` is `en` or `th` (the per-language
`LanguageCode`; the root `languageCode="en-us"` is not used). The site has no `layouts/alias.html` override, so the
embedded template is used via `aliasHandler.renderAlias` (`hugolib/alias.go:50-85`), with OutputFormat `alias` and an
empty kind.

After the tdewolff HTML minifier, all 1481 files have exactly this shape. The classifier's `ALIAS_RE` matches every
one, and a separate check confirmed that the three URLs in each file are identical:

```
<!doctype html><html lang={LANG}><head><title>{P}</title><link rel=canonical href={P}><meta name=robots content="noindex"><meta charset=utf-8><meta http-equiv=refresh content="0; url={P}"></head></html>
```

There is no trailing newline. Note that `content="noindex"` **keeps its quotes**. tdewolff treats `content` as an
RDFa attribute and always quotes it (`minify/v2@v2.23.8/html/html.go:501`). `{P}` is the target page's Permalink in the
alias's output format. I verified `{P} = "https://seeksnack.com/" + GoEscapedPath(<dir of alias>)` for all 1481 files
(0 mismatches).

### 2.2 Publishing an alias: `alias.go:87-116`

```
publishDestAlias(allowRoot, path, permalink, outputFormat, p):
    targetPath = targetPathAlias(path)                 # :118-183
    content    = renderAlias(permalink, p)             # template above, data = {Permalink, Page}
    pd = Descriptor{Src: content, TargetPath: targetPath, OutputFormat: outputFormat,
                    StatCounter: &ProcessingStats.Aliases}
    if RelativeURLs || CanonifyURLs: pd.AbsURLPath = s.absURLPath(targetPath)   # canonify runs, harmless here
    publisher.Publish(pd)                               # canonify -> minify -> write
```

`targetPathAlias(src)` (`:118-183`):

1. Error on empty input.
2. `alias = path.Clean(filepath.ToSlash(src))`. This removes any trailing `/`.
3. Error if the result is `/` and `!allowRoot`, or if the first component is `..`.
4. Check Windows naming rules (characters `:*?"<>|`, control chars, components ending in space or dot, reserved
   names). On non-Windows these checks only log at INFO.
5. `alias = TrimPrefix(alias, "/")`.
6. If it ends with `/` (only possible for the root), append `index.html`. Otherwise, unless it already ends with
   `.html`, append `/index.html`.

### 2.3 The three sources of aliases

**(a) Paginator page 1**, 1480 files. `site_render.go:228-248 renderPaginator` runs after the owning page is
rendered, once per page whose template initialized a paginator (`p.paginator.current != nil`):

```
if f.IsHTML && !Pagination().DisableAliases:
    d = p.targetPathDescriptor; d.Type = f; d.Addends = "/page/1"     # Pagination().Path == "page"
    writeDestAlias(CreateTargetPaths(d).TargetFilename, p.Permalink(), f, p)
```

`f.IsHTML` is true for both `html` and the `404` output format. That gives:

- `X/page/1/index.html` → `X/`.
- For the 404 page, `CreateTargetPaths` puts the page in the non-branch Ugly branch, which yields `404/page/1.html`
  (and `th/404/page/1.html`) pointing to `https://seeksnack.com/404.html` (and `…/th/404.html`).
- No alias is written for the RSS or JSON formats, because their templates do not paginate and they are not HTML.

Which pages paginate: `layouts/partials/head.html:1-4` calls `.Paginator` when `.IsHome` or `.IsNode`. That covers
home, section, taxonomy, term and 404 (404 is a node). `layouts/index.html:26` and `_default/list.html:16` call
`.Paginate`. The result is 856 + 622 aliases (EN: 1 home + 40 sections + 6 taxonomies + 809 terms; TH: 1 + 26 + 6 +
589) plus 2 for 404.

**(b) Main language redirect**, 1 file. `site_render.go:339-367 renderMainLanguageRedirect` is called at the end of
`Site.render` (`site.go:1604-1610`), and only when `shouldRenderStandalonePage("")` holds, which means
`languageIdx == 0 && outIdx == 0`. The steps:

- Skip if `DisableDefaultLanguageRedirect`, if the site is multihost, or if it is neither in a subdir nor
  multilingual. seeksnack is multilingual (en, th), so it proceeds.
- `defaultContentLanguageInSubdir=false` selects the else branch:
  `publishDestAlias(true, "en", s.AbsURL("", false) = "https://seeksnack.com/", html, nil)`, which writes
  `en/index.html`.
- The page is nil, so `site` in the template resolves to the first site, giving `lang=en`.

**(c) Front-matter `aliases`**, 0 files for this site. They are still worth porting. `site_render.go:270-335
renderAliases` runs only for `outIdx==0` on the first build (`site.go:1585-1598`), *before* pages, so a real page
overwrites a bad alias. For each page with `Aliases()`, it loops over the page's HTML output formats, deduplicated by
`f.Path`:

- A relative alias gets `path.Join(path.Join(SubResourceBaseLink, ".."), a)`.
- An absolute alias gets `path.Join(f.Path, a)`.
- `.html` is appended for ugly-URL sections. The language is prepended for multihost.
- The result goes to `writeDestAlias(a, of.Permalink(), f, p)`.
- Front-matter parsing is at `page__meta.go:566-574`. Values are passed through `cast.ToStringSlice`,
  `http(s)://` values are an error, and `filepath.ToSlash` is applied. Values are **not** lower-cased or sanitized.

---

## 3. Publish transformer chain and canonifyURLs

### 3.1 Who sets which Descriptor fields

`site.go:1440-1489 renderAndWritePage`:

```
render template -> buffer; if buffer empty: return (no file written!)
pd = Descriptor{Src, TargetPath, StatCounter, OutputFormat: p.outputFormat()}
if of.Name == "rss":           pd.AbsURLPath = absURLPath(targetPath)      # ALWAYS canonify RSS
elif of.IsHTML:                                                          # html, 404 (and alias)
    if RelativeURLs || CanonifyURLs: pd.AbsURLPath = absURLPath(targetPath)
    if watching && Running && !DisableLiveReload: pd.LiveReloadBaseURL = ...   # server only
    if p.IsHome(): pd.AddHugoGeneratorTag = s.conf.DisableHugoGeneratorInject  # NB inverted in neohugo -> false
publisher.Publish(pd)
```

`absURLPath(targetPath)` (`site.go:1420-1433`) returns `GetDottedRelativePath(targetPath)` when `relativeURLs` is set.
Otherwise it returns `BaseURL().String()` with a trailing `/` added, which is `"https://seeksnack.com/"` for every file.

### 3.2 Chain: `publisher/publisher.go:156-190 createTransformerChain`, applied at `:94-133`

Order:

1. `urlreplacers.NewAbsURLTransformer(path)` for HTML, or `NewAbsURLInXMLTransformer(path)` for any non-HTML format
   with `AbsURLPath` set (that is, RSS). Only when `AbsURLPath != ""`.
2. `livereloadinject` (HTML, server only; never in this build).
3. `metainject.HugoGenerator` (HTML, only if `AddHugoGeneratorTag`; never, see §0.4).
4. The minifier for the media type, if `min.MinifyOutput` (`--minify`) and a minifier is registered.
   `minifiers/minifiers.go:74-104` registers:
   - css, js (plus a JS regex), json (plus a JSON regex), svg;
   - **xml by suffix**, which covers `application/xml` (sitemap) and `application/rss+xml` (RSS, suffixes xml,rss);
   - html, plus every `IsHTML` output-format media type.
   - `text/plain` (robots.txt) gets no minifier.

`transform/chain.go:77-124 Apply` ping-pongs between two pooled buffers. The output of step *i* is the input of step
*i+1*.

Then `helpers.OpenFileForWriting(fs, TargetPath)` (`helpers/path.go:308-324`) opens the file with `fs.Create`
(`O_RDWR|O_CREATE|O_TRUNC`, mode 0666 before umask). If the parent directory is missing it runs `MkdirAll(dir, 0777)`
and retries. It then does `io.Copy`. When writeStats is on and the format is HTML, the writer is a MultiWriter that also
feeds `htmlElementsCollector` (hugo_stats.json only; it does not change output bytes).

Per output kind in this build:

| output | canonify | minifier |
|---|---|---|
| HTML page, 404, alias, paginator | HTML absurl (quotes `"` `'`) | tdewolff HTML |
| RSS `index.xml` | XML absurl (quotes `&#34;` `&#39;`) | tdewolff XML |
| JSON `index.json` | none | tdewolff JSON |
| sitemap / sitemapindex | none | tdewolff XML |
| robots.txt | none | none |
| static files | none (fsync copy) | none |
| resources (images, js) | none (resource publish, `resource.go:503+`) | none at publish time (JS minified by esbuild) |

### 3.3 The absURL replacer: `transform/urlreplacers/absurlreplacer.go` (274 lines) and `absurl.go` (36 lines)

Port this **literally**, including its quirks. The Python port is in Appendix B and at `$W/absurl_ref.py`. Its
behavior matches the Go transformer: I checked it on all 4900 rendered HTML and RSS files. The only 25 mismatches came
from template-level `relURL` differences between the builds being compared (§3.5), not from the replacer.

```
doReplace(path, from, quotes):                       # :240-254
    root = TrimLeading(url.Parse(path).Path)         # "https://seeksnack.com/" -> ""
    lexer{content, path, root, quotes, pos=0, start=0}.replace()

prefixes (in this order, :73-81): "src=", "href=", "url=", "action=" -> checkCandidateBase
                                  "srcset="                        -> checkCandidateSrcset
each prefix: {disabled=false, nextPos=0 (Go zero value!)}

prefix.find(bs, start):                              # :53-71
    if disabled: return false
    if nextPos == -1:
        idx = bytes.Index(bs[start:], p.b); if idx == -1 { disabled = true; return false }
        nextPos = start + idx + len(p.b)
    return true                                      # NB stale nextPos (< start) is NOT rechecked

replace():                                           # :205-238
    while pos < len:
        match = prefix with smallest nextPos among those whose find() is true (ties: first in list)
        if none: pos = len; break
        pos = match.nextPos; match.nextPos = -1; match.f(lexer)
    if pos > start: emit()                           # emit = write content[start:pos]; start = pos

consumeQuote():  for q in quotes: if content[pos:] startswith q: pos += len(q); emit(); return q
                 return nil

checkCandidateBase (href/src/url/action):            # :105-131
    consumeQuote()                                   # quote optional -> unquoted `href=/x` IS rewritten
    if content[pos] != '/': return
    if pos+1 >= len: return
    if DecodeRune(content[pos+1:]) == '/': return    # protocol-relative //host
    if pos > start: emit()
    pos += 1                                         # drop the leading '/'
    write(path)                                      # "https://seeksnack.com/"
    if root != "" && content[pos:] startswith root: pos += len(root)
    start = pos

checkCandidateSrcset:                                # :145-202
    q = consumeQuote(); if q == nil: return          # srcset MUST be quoted
    if content[pos] != '/' or pos+1 >= len or next rune == '/': return
    posEnd = bytes.Index(content[pos:], q); if posEnd < 0 || posEnd > 2000: return
    if pos > start: emit()
    section = content[pos : pos+posEnd+1]            # includes 1st byte of the closing quote
    fields = bytes.Fields(section)                   # split on unicode.IsSpace runs
    for i, f: if f[0]=='/': write(path); n=1; (skip root); write(f[n:]) else write(f)
              if i < len-1: write(" ")               # whitespace normalized to single spaces
    pos += len(section); start = pos
```

Test vectors, produced by running the Go code (`$W/absurltest/main.go`). Base is `https://seeksnack.com/`:

| input | output |
|---|---|
| `<a href="/about">` | `<a href="https://seeksnack.com/about">` |
| `<a href='/about'>` / `<a href=/about>` | rewritten likewise (quote optional) |
| `<a href="//cdn.x/a.js">`, `href="https://x/a"` | unchanged |
| `<img data-src="/a.png">` | rewritten (substring match, not attribute-aware) |
| `<form action="/search">`, `content="0; url=/foo"` | rewritten |
| `content="0; url=https://seeksnack.com/"` | unchanged (alias pages) |
| `<img srcset="/a.png 1x, /b.png 2x">` | both URLs rewritten |
| `srcset="/a.png 1x,  https://x/b.png   2x"` | `srcset="https://seeksnack.com/a.png 1x, https://x/b.png 2x"` (spaces collapsed) |
| `<img srcset=/a.png>` | unchanged (unquoted srcset) |
| `style="background:url(/a.png)"` | unchanged (`url(` is not `url=`) |
| `<p>use href="/x"</p>`, `<script>…'src="/x"'…</script>` | rewritten (also in text and scripts) |
| `<a HREF="/x">`, `<a href= "/x">` | unchanged (case-sensitive, no space allowed) |
| `<a href="/">` | `https://seeksnack.com/` |
| `<a href="/` (EOF) / `href=""` | unchanged |
| content starting with `/foo` | `https://seeksnack.com/` written **4 times** (nextPos=0 quirk) |
| content starting with `"/x"` | **Go panics** (slice bounds [2:1]) |
| `<img srcset="/a.png?href=/b 1x"><a href="/c">` | `…a.png?href=/b 1x"https://seeksnack.com/b 1x">…` (stale-nextPos rewind duplicates bytes) |
| XML: `&lt;img src=&#34;/a.png&#34;&gt;` | rewritten |
| XML: `<atom:link href="/index.xml"/>`, `href='/a'` | **unchanged** (`"` and `'` are not XML quotes) |
| XML: `<x href=/a>` | rewritten (quote optional) |
| base `https://example.org/docs/`, `href="/docs/a"` | `https://example.org/docs/a` (root `docs/` stripped) |

What this means for seeksnack:

- In the pre-transform HTML, which I built with `HUGO_CANONIFYURLS=false` and no minify
  (`$W/out-nomin-nocanon-seq`), the candidates are all double-quoted: 28011+2578+781+583+3 `href=` (mostly `<a>`),
  ~20k `srcset=` on `<source>`, and ~9.7k `src=`.
- No `url=`/`action=` candidates exist, and no srcset value has more than one candidate.
- 1940 HTML files change. Among rendered, non-alias HTML only `latesturl/index.html` is unaffected. Alias files are
  absolute already, and the 5 static HTML files are never touched.
- RSS contains no relative candidates. Its `<img src="https://…">` values are absolute and sit inside CDATA or escaped
  text.
- None of the edge quirks (leading `/`, stale nextPos) fire on this site, but port them literally anyway.

### 3.4 Ordering evidence: canonify before minify

`$G/privacy/index.html` has `<source type=image/webp srcset=https://seeksnack.com/images/favicon/favicon-32x32_hu_62d8c7963ef8923b.webp>`.
With canonify off (`$W/out-nomin-nocanon-seq`) the template emits `srcset="/images/…"` in quotes. If minify ran first,
the quotes would be gone, and `checkCandidateSrcset` returns early on an unquoted value. The chain order in
`publisher.go:161-187` confirms canonify runs first.

### 3.5 canonifyURLs also changes template output (not a publish-step concern, but it affects parity)

`helpers/url.go:161-163`: when `canonifyURLs` is on, `RelURL` does **not** call `paths.AddContextRoot`, so there is no
`path.Join`/clean. For example `relLangURL "ingredients//"` gives `/ingredients//` with canonify on and
`/ingredients/` with it off. The golden output contains `href=https://seeksnack.com/ingredients//` for pages with an
empty ingredient (25 files). The Rust `relURL`/`relLangURL` must implement the canonify branch. `specs/i18n-lang-misc.md`
§3.2 covers this too.

---

## 4. File path rules on disk

### 4.1 Page target paths: `resources/page/page_paths.go:109-293 CreateTargetPaths` (port exactly)

Inputs come from `hugolib/page__paths.go:111-181 createTargetPathDescriptor`:

- `Kind`, `Path` and `Section` path infos (lower-cased and normalized unless `disablePathToLower`), `UglyURLs=false`.
- `ForcePrefix = IsMultihost || Kind == sitemap`.
- `URL` from front matter (unused here).
- `BaseName` = slug, else the standalone format's `BaseName`, else `BaseNameNoIdentifier`.
- `PrefixFilePath` and `PrefixLink` come from `site.go:1360-1379`. Both are `""` for en. For th they are `"th"`. For
  the sitemap kind they are always the language (`alwaysInSubDir`), which is why `en/sitemap.xml` exists.
- `ExpandedPermalink` comes from `[permalinks]`. Only `posts` is configured, and it is unused.

Resulting rules for this site (`defaultContentLanguageInSubdir=false`, uglyURLs off, `disablePathToLower=false`):

| kind / format | file on disk | Link |
|---|---|---|
| home html/json/rss | `[th/]index.html` / `index.json` / `index.xml` | `/`, `/th/` |
| section | `[th/]<sec>/index.html`, `index.xml` | `/<sec>/` |
| taxonomy | `[th/]<plural>/index.html`, `index.xml` | |
| term | `[th/]<plural>/<Sanitize(term key)>/index.html`, `index.xml` | |
| page (leaf bundle or single file) | `[th/]<containerDir>/<bundle-or-basename>/index.html` | |
| paginator N≥2 (branch) | `…/page/N/index.html` (`Addends="/page/N"`) | `…/page/N/` |
| 404 | `[th/]404.html` (format `404`: Ugly, no BaseName, `noSubResources`) | `/404.html` |
| 404 paginator | `[th/]404/page/N.html` (non-branch + Ugly → `ConcatLast(".html")`) | |
| sitemap | `en/sitemap.xml`, `th/sitemap.xml` (Ugly, BaseName from `[sitemap] filename` via `paths.Filename`) | |
| sitemapindex | `sitemap.xml` (Root → prefixes cleared) | |
| robots | `robots.txt` (Root, BaseName robots, text/plain → `.txt`) | |

- **Sanitize.** When `URL==""`, `page_paths.go:259-263` runs `pb.Sanitize()` on every element, which is
  `PathSpec.MakePathSanitized` = `strings.ToLower(paths.Sanitize(s))` (`helpers/path.go:43-64`).
- **`paths.Sanitize`** (`common/paths/path.go:282-336`):
  - kept: `unicode.IsLetter`, `unicode.IsDigit` (Nd only), `unicode.IsMark`, `. / \ _ # + ~ - @`, and `%` followed by
    two hex bytes;
  - `' '` is never kept;
  - a run of `unicode.IsSpace` becomes one `-`, but only after a kept char, not after a `-`, and only if another kept
    char follows;
  - everything else (`' ( ) & , ! – ’ U+2063 ² ½ Ⅻ …`) is **dropped**.
  - Go test vectors (`$W/sanitizetest`): `Lay's→Lays`, `INS 322(i)→INS-322i`, `Gilim Co., Ltd→Gilim-Co.-Ltd`,
    `a b→a-b`, `x⁣y→xy`, `Ⅻ²½→""`, `a–b→ab`, `x%41y→x%41y`, `x%4→x4`, `  lead→lead`, `trail  →trail`.
- **Term keys are *not* sanitized.** A term's tree key is `pathparser.Parse("/"+plural+"/"+value+"/_index.md").Base()`
  (`content_map_page.go:1676-1678`). `NormalizePathStringBasic` (`common/paths/pathparser.go:50-58`) only lower-cases
  and turns `' '` into `-`. So the key keeps `'`, `(`, `)` and `&`, while the output directory drops them. This is the
  collision mechanism in §7. A leading or trailing space becomes a leading or trailing `-` in the key, and that `-` is
  kept by Sanitize, so the output has dirs like `th/tags/-เค้ก-/` and `th/ingredients/แป้งสาลี--/`.
- **Lower-casing** is Go `strings.ToLower`: per-rune *simple* case mapping. Go gives `İstanbul→istanbul` and
  `ΣΑΣ→σασ`. Rust's `str::to_lowercase` gives `i̇stanbul` and `σας`, so **do not use it**. Map per char, and special-case
  U+0130 to `i` (see §10).
- **Thai and other non-ASCII names** are written to disk as raw UTF-8 bytes, never percent-decoded or encoded. Thai
  letters (Lo) and vowel or tone marks (Mn) survive Sanitize. Compare paths as bytes: with `en_US.UTF-8` collation,
  `sort -u` merges distinct Thai paths (I hit this: 2695 vs 3203 "unique" targets).
- **NFC.** On darwin only, file-system-derived names are NFC-normalized (`hugofs/component_fs.go:173-176`). For
  example the content dir `wise-chili-olé-…` is stored NFC in the source and written NFC. Front-matter strings are not
  normalized.
- **Directory names from content** are sanitized too. `potato-chips/herrs-salt-&-vinegar-potato-chips/` becomes
  `potato-chips/herrs-salt--vinegar-potato-chips/`, and `companies/Berli-Jucker-Foods-Ltd.Berli-Jucker-PLC/` becomes
  `…/berli-jucker-foods-ltd.berli-jucker-plc/`.
- **Resource file names** inside a bundle are only path-normalized (lower-case, space to `-`), **not** sanitized. For
  example `lay_butter-garlic-scallops-–-thailand.jpg` keeps the EN DASH. The bundle directory part comes from the
  owning page's `SubResourceBaseTarget`, which is sanitized.
- **Links** (`tp.Link`, `page_paths.go:275-290`):
  - `#` is first replaced by `%23`;
  - then `paths.PathEscape` (`path.go:388-394`) runs `url.Parse(p).EscapedPath()`;
  - that returns RawPath when it is valid-encoded and round-trips, else `escape(Path, encodePath)`;
  - unreserved `A-Za-z0-9-_.~` and `$&+,/:;=@` stay, everything else becomes `%XX` in **uppercase** hex;
  - because a Thai byte is invalid in RawPath, the whole path is re-escaped;
  - `!'()*` survive when RawPath is otherwise valid;
  - an invalid `%zz` panics.
  - Examples: `/th/tags/นม/` → `/th/tags/%E0%B8%99%E0%B8%A1/`, `wise-chili-olé…` → `wise-chili-ol%C3%A9…`.
  - The permalink is baseURL + Link.

### 4.2 The `en/` directory

It holds only `en/index.html` (main-language redirect, §2.3b) and `en/sitemap.xml` (per-language sitemap, which is
always in a language subdir when multilingual). All EN pages live at the root.

---

## 5. Sitemaps

- **Creation.** `content_map_page.go:1930-2005 addStandalonePages` adds, per site:
  - `/404` (format `404`);
  - `/_robots` (only for language index 0, and only when `enableRobotsTXT`);
  - `/_sitemap` (format `SitemapFormat` with `BaseName = paths.Filename("sitemap.xml") = "sitemap"`);
  - `/_sitemapindex`, only for language index 0, and only when not multihost and (in a subdir or multilingual).
    seeksnack is multilingual, so it gets a sitemapindex.
- **When they render.** `site_render.go:54-67 shouldRenderStandalonePage`:
  - the sitemap renders once per site, in `outIdx==0`;
  - 404 renders once per site;
  - robots and sitemapindex render once overall (`languageIdx==0 && outIdx==0`).
  - For sitemapindex the template data is `s.h.Sites` (`site_render.go:176-179`).
- **Templates:**
  - Per-language sitemap: the **site's `layouts/sitemap.xml`**. The golden output proves it: `<priority>0</priority>`
    for home, priorities from `site.Params.sitemap.priority` (page 1, section 0.3, term 0.5), and taxonomy 0 because
    of the template typo `Tanoxomy`. `hreflang` comes from `.Lang`. The template ranges over `.Data.Pages`, which for
    the sitemap kind is `p.s.Pages()` (`page.go:338-339`): all listable pages of that site. That is 1023 EN and 707 TH
    `<url>` entries, **including both pages of every colliding term**, so there are duplicate `<loc>`s (8 EN, 5 TH).
    Pages with a zero date get no `<lastmod>`.
  - Root sitemapindex: the **embedded** `tpl/tplimpl/embedded/templates/sitemapindex.xml`. The site has none. It
    ranges over sites and emits `<loc>{{ .SitemapAbsURL }}</loc>` plus `<lastmod>` = `site.Lastmod`, formatted
    `2006-01-02T15:04:05-07:00`. `SitemapAbsURL` (`site.go:1222-1233`) is
    `AbsURL(lang) + "/" + conf.Sitemap.Filename`, giving `https://seeksnack.com/en/sitemap.xml` and `…/th/sitemap.xml`.
    Only enabled languages (en, th) appear.
- **changefreq and priority.** `[sitemap] changefreq='weekly'` becomes each page's `.Sitemap` unless the front matter
  has `sitemap:` (`page__meta.go:575-580, 652-654`). In that case `config.DecodeSitemap(prototype=site config, fm)`
  merges the two. For example `search.md` has `priority: 0.1`, but the site template ignores `.Sitemap.Priority`, so it
  still shows `1`. The config `priority` default is -1, and the embedded sitemap would omit `<priority>`, but the
  embedded sitemap is not used here.
- **Minification.** Both sitemaps go through the XML minifier. It also removes the space in the site template's
  `standalone="yes" ?>`, which becomes `standalone="yes"?>`. See `specs/minify.md`.

---

## 6. Output formats per kind

Config: `[outputs] home=[HTML,JSON,RSS] page=[HTML] section=[HTML,RSS] taxonomy=[HTML,RSS] term=[HTML,RSS]`. It is
resolved in `hugolib/site_output.go:57-109`. Defaults for kinds not listed: sitemap→sitemap, robotstxt→robots,
404→`404`.

`Site.initRenderFormats` (`site.go:800-837`) takes the union and sorts with `Formats.Less` (`outputFormat.go:252-263`):
weight first, with 0 sorting last, then name. The EN and TH render passes (`hugo_sites_build.go:376-435`) are
`[html, json, rss]`, in the order *en.html, en.json, en.rss, th.html, th.json, th.rss*. Standalone pages (sitemap,
robots, 404, sitemapindex) render inside each site's `outIdx==0` (html) pass.

| format | Name | media type | BaseName | suffix | flags | file |
|---|---|---|---|---|---|---|
| HTML | html | text/html | index | .html | IsHTML, Permalinkable, Weight 10 | `index.html` |
| JSON | json | application/json | index | .json | IsPlainText | `index.json` |
| RSS | rss | application/rss+xml | index | .xml (FirstSuffix) | NoUgly | `index.xml` |
| 404 | 404 | text/html | "" | .html | Ugly, IsHTML, NotAlternative | `404.html` |
| sitemap | sitemap | application/xml | sitemap | .xml | Ugly | `<lang>/sitemap.xml` |
| sitemapindex | sitemapindex | application/xml | sitemap | .xml | Ugly, Root | `sitemap.xml` |
| robots | robots | text/plain | robots | .txt | Root | `robots.txt` |
| alias | alias | text/html | "" | | Ugly, IsHTML | (alias paths, §2) |

Definitions are in `output/outputFormat.go:87-220`.

Layouts, from the trace: home `index.html`; page `single.html` (235), `simple.html` (3), `latesturl.html` (1);
section `list.html`; taxonomy `taxonomy/list.html`; term `term/term.html`; all RSS `rss.xml` (site
`_default/rss.xml`); home JSON `_default/index.json`; 404 `404.html`; sitemap `sitemap.xml` (site); sitemapindex
embedded; robots `robots.txt` (site, 13 bytes `User-agent: *` with **no trailing newline**; the embedded one has a
`\n`).

The front-matter key `output:` (singular, used in 5 files) is **not** `outputs:`. It is ignored and becomes a param.

---

## 7. Nondeterminism: term collisions

**Mechanism.**

- *Keys:* `content_map_page.go:1672-1705` creates one term page per distinct pathparser key (§4.1). The keys keep
  `'`, `(` and `)`.
- *Paths:* `page_paths.go:259-263` sanitizes the target path, so two term pages get the same `TargetFilename`.
- *Rendering:* `site_render.go:70-120 renderPages` walks the page tree (radix tree, byte-lexicographic key order) and
  sends each page into a channel read by `config.GetNumWorkerMultiplier()` goroutines. That is `runtime.NumCPU()`
  unless `HUGO_NUMWORKERMULTIPLIER` is set (`config/env.go:33-40`). Each worker renders a page and publishes it.
- *Writing:* `Publish` → `OpenFileForWriting` → `fs.Create` (O_TRUNC) + `io.Copy`. Nothing checks for duplicate
  targets, so the last `Publish` to finish wins.
- *Scope:* this happens per render pass, so the HTML and RSS winners are independent. A torn write mixing both variants
  is theoretically possible, since the two file descriptors truncate independently. I never observed one: every
  collision file matched one of the two clean variants across 5 builds.

**The 13 colliding targets (×`index.html` and `index.xml` = 26 files).** The first column is the loser, the second the
winner under sequential (last-wins) rendering:

| output dir | tree keys (first → last) |
|---|---|
| `brands/lays/` | `lay's` → `lays` (`lays` comes from `brands = "lays"` in the TOML front matter of `potato-crisps/lays-baked-kc-masterpiece…`) |
| `tags/lays/` | `lay's` → `lays` |
| `ingredients/disodium-5-guanylate/` | `disodium-5'-guanylate` → `disodium-5-guanylate` |
| `ingredients/disodium-5-inosinate/` | `…5'-inosinate` → `…5-inosinate` |
| `ingredients/disodium-5-ribonucleotide/` | `…5'-ribonucleotide` → `…5-ribonucleotide` |
| `ingredients/disodium-5-ribonucleotides/` | `…5'-ribonucleotides` → `…5-ribonucleotides` |
| `ingredients/ins-322i/`, `th/ingredients/ins-322i/` | `ins-322(i)` → `ins-322i` |
| `ingredients/ins-500ii/`, `th/ingredients/ins-500ii/` | `ins-500(ii)` → `ins-500ii` |
| `th/ingredients/ไดโซเดียม-5-กัวไนเลต/` | `…5'-กัวไนเลต` → `…5-กัวไนเลต` |
| `th/ingredients/ไดโซเดียม-5-ไอโนซิเนต/` | `…5'-ไอโนซิเนต` → `…5-ไอโนซิเนต` |
| `th/ingredients/ไดโซเดียม-5-ไรโบนิวคลีโอไทด์/` | `…5'-ไรโบ…` → `…5-ไรโบ…` |

The byte order holds because `'` (0x27) < `-` (0x2D) and `(` (0x28) < letters.

**Knock-on effects.**

- Paginator aliases `X/page/1/index.html` are written twice with identical bytes, since the permalink is the same, so
  they are harmless.
- `X/page/N/` for N≥2 only exists for the variant with enough items (for example `brands/lays/page/2/` comes from
  `lay's`). It is deterministic.
- The sitemap lists both pages, so it is deterministic.
- The `Aliases` and `Pages` stats count both writes.

**Observed.** Golden run1 and run2 differ in 8 RSS files. My parallel cold build (`$W/out-fresh`) differs from run1 in
12 files, 4 of them HTML. Across run1, run2, fresh and seq there are at most 2 variants per file. Two sequential builds
(`HUGO_NUMWORKERMULTIPLIER=1`, cold) were byte-identical to each other. They differ from run1 in 8 files and from run2
in 2 files.

**Recommendation for the Rust port.** Render deterministically. Walk the tree in byte-lexicographic key order and let
the **last** page in that order win. That is exactly Go's behavior with `HUGO_NUMWORKERMULTIPLIER=1`.

- Parallel rendering is fine. Either group writes by target path and commit them in tree order, or run a pre-pass that
  detects duplicate `TargetFilename`s and keeps only the last.
- For acceptance, compare against `$W/out-seq1`, which gives exact parity on all 6943 files. Alternatively, compare
  against `golden/run1` with `$W/compare_outputs.py --variants $W/collision_variants.json`, which accepts any observed
  variant of the 26 files. `out-seq1` passes that check with 0 differences and 8 tolerated.
- The variants JSON records only the variants observed so far: 14 of the 26 files have two known variants, 12 have one.
  Regenerate it if more builds become available.

---

## 8. The final publish step, static copy and permissions

- **Order within a build** (`commands/hugobuilder.go:518-567 fullBuild`):
  - If `cleanDestinationDir` is false (the default and this build), `copyStatic` and `buildSites` run **concurrently**
    (errgroup).
  - If it is true, static is copied first, with `Delete` enabled, keeping only dirs whose name starts with `.`.
  - Nothing in this site's static tree collides with a rendered path, so the concurrency is harmless. The Rust port can
    copy static files first.
- **Static copy** (`hugobuilder.go:437-482` + `spf13/fsync@v0.10.1/fsync.go`, 370 lines):
  - Source: the union static FS (`h.Static[""]`). Here that is only the `static` → `static` mount. `node_modules` is
    mounted to `assets/vendor`, not static. Destination: `PublishDirStatic`, at the publish root.
  - Every file is copied, including dotfiles and `*.bak`. There is no ignore filter.
  - A file is (re)written only if the destination is missing, has a different size, or has different bytes
    (`fsync.go:257-299 equal`).
  - `syncstats` (`:226-254`) then always syncs file **permissions**. `NoChmod=false`; the `ChmodFilter` in
    `commands/server.go:1159-1167` skips directories only.
  - It also always syncs **mtimes** of files *and* directories (`NoTimes=false`). The golden static files carry the
    source mtimes (for example `ads.txt` is Oct 13 2025). Directories are created with `MkdirAll(0755)`.
  - The build-summary count is `statCounter/2`.
- **Rendered files, aliases, sitemaps and robots** always go through `OpenFileForWriting`: `Create` truncates and
  always rewrites, with no "unchanged" check. New directories are created with `0777 &^ umask`. Mode is `0666 &^
  umask`. With umask 022, the golden has all 6943 files at 0644 and all 4891 dirs at 0755.
- **Resources:**
  - `genericResource.Publish` (`resources/resource.go:503+`) writes through `helpers.OpenFilesForWriting` →
    `OpenFileForWriting`.
  - A processed image served from the file cache (`sourceFilenameIsHash`) is skipped if the target already exists.
    Other resources are always rewritten.
  - Page bundle resources are published by `pageState.renderResources` (`hugolib/page.go:524-554`) the first time any
    render pass touches the page, if `PublishResources` (default true). `resources.IsPublished` prevents a second copy.
  - Resources shared across languages are published once, at the path without a language prefix. TH pages link to
    `/almonds/…/x.jpg`, not `/th/…`.
- **No other writes to the publish dir.** `postProcess` (`hugo_sites_build.go:600-717`) only rewrites files containing
  `__h_pp_l1` placeholders, and there are none (`resources.PostProcess` is unused). It would write with
  `afero.WriteFile` 0666, only if changed.
- **`writeBuildStats`** writes `<site>/hugo_stats.json` (`:719-775`). It uses a JSON encoder with
  `SetEscapeHTML(false)` and `SetIndent("", "  ")`, and writes only when the content changed.

---

## 9. Other parity risks seen from the output side

1. **Cold versus warm image cache.** The golden is cold. A warm build (`$W/out-plain`, since deleted to save space) differed from the golden in 4688 `diff -rq` lines (1444 renamed images, 1791 HTML, 2 JSON, 7 collision RSS); a fresh
   copy (`$W/out-fresh`) matched the golden except for the collision files. Cause: `genericResource.Key()`
   (`resources/resource.go:447-466`) appends `_<hash>` only while `!sourceFilenameIsHash`, and only the cache-read path
   sets that flag (`image_cache.go:67`). This changes the key of the `images.Overlay $watermark` filter, and so the
   hashes of every watermarked derivative. The Rust port must always use cold semantics. See `specs/images.md`.
2. **Clock.** `now.Format "2006"` appears in the footer and the RSS copyright. Pin it.
3. **`relURL` under canonify** (§3.5).
4. **Empty render.** A template that renders 0 bytes produces **no file** (`site.go:1455-1457`).
5. **Minified alias bytes** depend on the tdewolff rules for `content=` quoting and `<!doctype html>` lower-casing (§2.1).
6. **Byte-wise path comparison and sorting everywhere.** Never use locale collation.

---

## 10. Rust port guidance

### 10.1 Port line by line from Go

| Go source | lines | what |
|---|---:|---|
| `transform/urlreplacers/absurlreplacer.go` | 274 | canonify lexer, including the quirks in §3.3 |
| `transform/urlreplacers/absurl.go` | 36 | HTML and XML entry points |
| `transform/urlreplacers/absurlreplacer_test.go` | 236 | port as unit tests, plus the §3.3 vectors |
| `transform/chain.go` | 124 | transformer chain (a trivial `Vec<Box<dyn Fn>>`) |
| `publisher/publisher.go` | 190 | Descriptor, transformer order, write |
| `hugolib/alias.go` | 183 | alias template execution and `targetPathAlias` |
| `hugolib/site_render.go` | 367 | renderPages, renderPaginator (page/1 alias), renderAliases, renderMainLanguageRedirect, shouldRenderStandalonePage |
| `hugolib/site.go` (parts) | ~150 | `absURLPath` 1420-1433, `renderAndWritePage` 1440-1489, `render` 1580-1613, `getLanguage*Lang` 1360-1379, `SitemapAbsURL` 1222-1233, `initRenderFormats` 800-837 |
| `hugolib/hugo_sites_build.go` `render` | 88 | pass order (351-438) |
| `hugolib/content_map_page.go` `addStandalonePages` | 76 | 1930-2005 |
| `hugolib/page__paths.go` | 181 | descriptor, per-format target paths, permalinks |
| `resources/page/page_paths.go` | 462 | `CreateTargetPaths` and `pagePathBuilder`, **exactly** |
| `resources/page/pagination.go` `newPaginationURLFactory` | 12 | 406-417 |
| `common/paths/path.go` | ~80 of 430 | `Sanitize`, `isAllowedPathCharacter`, `PathEscape`, `AddLeadingSlash`, `Dir`, `FieldsSlash` |
| `common/paths/pathparser.go` `NormalizePathStringBasic` | 9 | term keys |
| Go stdlib `net/url/url.go` (parts) | ~150 of 1353 | `shouldEscape`, `escape`, `unescape`, `validEncoded`, `setPath`, `EscapedPath` in encodePath mode |
| `helpers/path.go` | ~50 | `MakePath`, `MakePathSanitized`, `OpenFileForWriting`, `OpenFilesForWriting`, `GetDottedRelativePath` |
| `output/outputFormat.go` | ~180 | built-in formats and `Formats.Less` |
| `hugolib/site_output.go` | 109 | kind → formats |
| `minifiers/minifiers.go` | 131 | which media types get which minifier |
| `commands/hugobuilder.go` | ~90 | `copyStaticTo`, `fullBuild` ordering |
| `spf13/fsync/fsync.go` | ~150 of 370 | `sync`, `syncstats`, `equal`, only the non-Delete path is needed |
| `source/sourceSpec.go` `IgnoreFile` | 20 | dotfile, `#` and `~` content filter |
| embedded `alias.html`, `sitemapindex.xml` | 21 | template text, verbatim |

**Skip:** `transform/metainject` (never active; do not inject), `livereloadinject` (server only), and
`publisher/htmlElementsCollector.go` (556 lines). The collector is only needed if `hugo_stats.json` must be produced,
and that file is not part of the output dir.

### 10.2 Rust crates

**Safe to use:**

- `memchr` / `memchr::memmem` as `bytes.Index`.
- `rayon` for parallel rendering, as long as the tree-order last-wins commit for duplicate targets is kept.
- `walkdir` for the static copy. Include hidden files, and do not follow the `.gitignore` semantics of the `ignore`
  crate.
- `filetime` to copy mtimes (optional; mtimes are not part of byte parity).
- `sha2`/`md-5` only for the test harness.
- `unicode-normalization` for NFC of darwin file names (only file-system names, never front-matter strings).
- `unicode-general-category`, or `unicode-properties` with the general-category feature, to implement Go's
  `unicode.IsLetter` (L*), `IsDigit` (Nd), `IsMark` (M*) and `IsSpace`. Go 1.27 uses **Unicode 17.0.0**; pin a crate
  with the same tables.
- `percent-encoding` **only** for the `escape()` step, with an `AsciiSet` equal to Go's encodePath set (escape all
  except `A-Za-z0-9 - _ . ~ $ & + , / : ; = @`; it emits uppercase hex). The RawPath/validEncoded logic still has to be
  hand-ported.

**Not safe:**

- `url` (WHATWG). Its percent-encoding sets and path normalization (`.`, `..`, backslashes, IDNA) differ from Go
  `net/url`: `'`, `(`, `)`, `!`, `*` and non-ASCII are handled differently, and `join` semantics are not Hugo's
  string concatenation.
- `str::to_lowercase`. It uses full case mapping plus a contextual final sigma (`ΣΑΣ→σας` versus Go `σασ`;
  `İ→i̇` versus Go `i`). Use per-char simple mapping: `char::to_lowercase` gives 1 char except for U+0130, which must be
  mapped to `i`. `char::is_alphabetic`/`is_numeric`/`is_whitespace` are also wrong substitutes: `Alphabetic` includes
  Nl and Other_Alphabetic, and `is_numeric` includes Nl and No.
- Any HTML-aware URL rewriter such as `lol_html` in place of the byte-level absurl lexer. It would miss `href=` in
  text or scripts, rewrite attributes Go leaves alone (uppercase `HREF`), and drop the quirks.
- `fs::write`, which is fine by itself. Atomic rename-based writers are also fine, but must not change modes (0644/0755
  with umask 022).

### 10.3 Suggested Rust structure

```
publish(desc):
    bytes = desc.src
    if let Some(base) = desc.abs_url_path { bytes = if desc.format.is_html { absurl_html(bytes, base) } else { absurl_xml(bytes, base) } }
    if minify && let Some(m) = minifier_for(desc.format.media_type) { bytes = m(bytes) }
    write_file_create_dirs(publish_dir/desc.target_path, bytes)   // truncate; 0666&^umask; dirs 0777&^umask

render order: for site in [en, th]: for (outIdx, fmt) in [html, json, rss]:
    if outIdx==0 && first_build && !disableAliases: render_front_matter_aliases(site)
    for page in site.tree_in_byte_order():                          // parallel OK; commit duplicates in tree order
        if standalone && !should_render_standalone(kind, site.idx, outIdx): continue
        publish_resources_once(page)
        publish(render(page, fmt)); if paginator: page/1 alias (HTML formats) + page/N (N>=2)
    if site.idx==0 && outIdx==0: main_language_redirect()          // en/index.html
static: copy all files of static/ to publish root (can run first; no collisions in this site)
```

### 10.4 Verification assets

- `$W/classify_golden.py`: provenance classifier (Appendix A).
- `$W/compare_outputs.py` and `$W/collision_variants.json`: byte comparator that tolerates collision variants.
- `$W/absurl_ref.py`: Python reference of the canonify lexer (Appendix B). `python3 absurl_ref.py out-nomin-nocanon-seq out-nomin-seq`.
- `$W/out-seq1`: deterministic cold `--minify` build (HUGO_NUMWORKERMULTIPLIER=1). It equals the golden except for the
  8 collision files, and is the recommended exact-parity target.
- `$W/out-fresh`: cold parallel `--minify` build.
- `$W/out-nomin-seq`: cold, sequential, **unminified**, canonify on. This is the exact input to the minifier and the
  exact output of canonify.
- `$W/out-nomin-nocanon-seq`: the same with `HUGO_CANONIFYURLS=false`. This is the absurl input, except for the §3.5
  relURL difference in 25 files.
- `$W/trace.log`: `--devMode` TRACE log with `rendering outputFormat … kind … using layout … to …` for all 3229
  renders.
- `$W/absurltest/`, `$W/sanitizetest/`: Go harnesses that call neohugo packages through a `go.mod` replace.
- `$W/sitecopy/seeksnack`: a private copy of the site for cold builds (`rm -rf resources` first).

---

## Appendix A: `classify_golden.py`

```python
#!/usr/bin/env python3
"""
Classify every file in a neohugo (seeksnack) build output by its producer.

Usage:
  python3 classify_golden.py <public_dir> <site_dir> [--trace trace.log] [--list out.tsv]

  --trace   optional: log of `neohugo-go --minify --devMode -d <dir>`; every
            TRACE "rendering outputFormat X kind Y ... to Z" line is used to
            cross-check the path-based kind classification (ground truth).
  --list    optional: write "<category>\t<lang>\t<relpath>" for every file.

All comparisons are byte-wise (never locale collation: Thai names collate
equal under en_US.UTF-8 `sort -u`, which silently merges distinct paths).
"""
import os
import re
import sys
import hashlib
import unicodedata
from collections import Counter, defaultdict


def go_is_space(r):
    # Go unicode.IsSpace
    if ord(r) <= 0xFF:
        return r in '\t\n\v\f\r \x85\xa0'
    return unicodedata.category(r) == 'Zs' or r in '\u2028\u2029'


def go_sanitize(s):
    """common/paths/path.go:282 Sanitize. Kept: unicode.IsLetter (L*), unicode.IsDigit (Nd),
    unicode.IsMark (M*), the ASCII set . / \\ _ # + ~ - @, and '%' followed by two hex digits
    (byte-indexed check). ' ' is never kept. A run of unicode.IsSpace chars becomes ONE '-'
    (only if output is non-empty and the previous kept char was not '-', and only when a
    later kept char follows). Everything else is dropped."""
    b = s.encode('utf-8')
    HEX = b'0123456789abcdefABCDEF'

    def allowed(i, r):
        if r == ' ':
            return False
        cat = unicodedata.category(r)
        if cat.startswith('L') or cat == 'Nd' or cat.startswith('M'):
            return True
        if r in './\\_#+~-@':
            return True
        return r == '%' and i + 2 < len(b) and b[i + 1] in HEX and b[i + 2] in HEX

    chars, i = [], 0
    for r in s:
        chars.append((i, r))
        i += len(r.encode('utf-8'))
    if all(allowed(i, r) for i, r in chars):
        return s
    out = []
    prepend = False
    was_hyphen = False
    for i, r in chars:
        if allowed(i, r):
            was_hyphen = r == '-'
            if prepend:
                if not was_hyphen:
                    out.append('-')
                prepend = False
            out.append(r)
        elif out and not was_hyphen and go_is_space(r):
            prepend = True
    return ''.join(out)


def out_path_for_content(rel):
    # NFC (hugofs/component_fs.go applyMeta on darwin) -> lower (MakePathSanitized) -> Sanitize per element
    rel = unicodedata.normalize('NFC', rel)
    # bundle *directories* come from the owning page's target path (Sanitize'd),
    # the resource *file name* is only path-normalized (lower-case, ' ' -> '-'),
    # NOT Sanitize'd: e.g. "lay_butter-garlic-scallops-\u2013-thailand.jpg" keeps the EN DASH.
    parts = rel.split('/')
    dirs = [go_sanitize(p.lower()) for p in parts[:-1]]
    return '/'.join(dirs + [parts[-1].lower().replace(' ', '-')])

BASE_URL = "https://seeksnack.com/"
TAXONOMIES = ["companies", "categories", "countries", "ingredients", "brands", "tags"]
LANGS_SUBDIR = ["th"]          # non-default languages (defaultContentLanguageInSubdir=false)
DEFAULT_LANG = "en"

# tpl/tplimpl/embedded/templates/alias.html after tdewolff HTML minification
ALIAS_RE = re.compile(
    rb'^<!doctype html><html lang=(?P<lang>[a-zA-Z-]+)><head><title>(?P<u1>[^<]*)</title>'
    rb'<link rel=canonical href=(?P<u2>[^>]*)><meta name=robots content="noindex">'
    rb'<meta charset=utf-8><meta http-equiv=refresh content="0; url=(?P<u3>[^"]*)"></head></html>$'
)
FINGERPRINT_RE = re.compile(r'\.[0-9a-f]{64}\.(js|css)$')


def walk_files(root):
    out = []
    for dp, dns, fns in os.walk(root):
        for fn in fns:
            full = os.path.join(dp, fn)
            out.append(os.path.relpath(full, root).replace(os.sep, "/"))
    out.sort(key=lambda s: s.encode("utf-8"))
    return out


def sha(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 16), b""):
            h.update(chunk)
    return h.hexdigest()


def split_lang(rel):
    parts = rel.split("/")
    if parts[0] in LANGS_SUBDIR and len(parts) > 1:
        return parts[0], parts[1:]
    return DEFAULT_LANG, parts


def content_sections(site):
    """Top-level content dirs that contain at least one .md file (-> section pages)."""
    secs = set()
    croot = os.path.join(site, "content")
    for name in os.listdir(croot):
        d = os.path.join(croot, name)
        if not os.path.isdir(d):
            continue
        for dp, _, fns in os.walk(d):
            if any(fn.endswith(".md") for fn in fns):
                secs.add(name.lower())
                break
    return secs


def node_kind(parts, sections):
    """parts: path components of the *directory* of an index.* file (lang stripped)."""
    if len(parts) == 0:
        return "home"
    if parts[0] in TAXONOMIES:
        return "taxonomy" if len(parts) == 1 else "term"
    if len(parts) == 1 and parts[0] in sections:
        return "section"
    return "page"


def strip_pager(parts):
    """Return (parts_without_pager, pageN or None) for .../page/N."""
    if len(parts) >= 2 and parts[-2] == "page" and parts[-1].isdigit():
        return parts[:-2], int(parts[-1])
    return parts, None


def classify(public, site, rel, sections, static_map, content_files, assets_files):
    full = os.path.join(public, rel)
    lang, parts = split_lang(rel)
    base = parts[-1]
    ext = base.rsplit(".", 1)[-1] if "." in base else ""

    # 1. static files (static/ mount copied by fsync) -- checked first; nothing in
    #    this site overwrites a static path.
    if rel in static_map:
        return "static", lang

    # 2. alias redirect pages (embedded alias.html, minified)
    if ext == "html" and os.path.getsize(full) < 2048:
        with open(full, "rb") as f:
            b = f.read()
        m = ALIAS_RE.match(b)
        if m:
            if rel == DEFAULT_LANG + "/index.html":
                return "alias:main-language-redirect(/en/ -> /)", lang
            if rel.endswith("/page/1.html") and "404" in parts:
                return "alias:404-paginator-page1", lang
            if rel.endswith("page/1/index.html"):
                return "alias:paginator-page1", lang
            return "alias:front-matter", lang

    # 3. fixed standalone outputs
    if rel == "robots.txt":
        return "robotstxt(layouts/robots.txt)", lang
    if rel == "sitemap.xml":
        return "sitemapindex(embedded sitemapindex.xml)", lang
    if len(parts) == 1 and base == "sitemap.xml":
        return "sitemap(layouts/sitemap.xml)", lang
    if rel in (DEFAULT_LANG + "/sitemap.xml",):
        return "sitemap(layouts/sitemap.xml)", DEFAULT_LANG
    if base == "404.html" and len(parts) == 1:
        return "html:404", lang
    if len(parts) == 3 and parts[0] == "404" and parts[1] == "page" and base.endswith(".html"):
        return "html:404-paginator-pageN", lang

    # 4. rendered page output formats
    if base in ("index.html", "index.xml", "index.json"):
        dparts, pager = strip_pager(parts[:-1])
        kind = node_kind(dparts, sections)
        fmt = {"index.html": "html", "index.xml": "rss", "index.json": "json"}[base]
        if pager is not None:
            return f"{fmt}:{kind}-paginator-pageN", lang
        return f"{fmt}:{kind}", lang

    # 5. resources
    if FINGERPRINT_RE.search(base):
        return "resource:js.Build+fingerprint", lang
    if "_hu_" in base:
        return "resource:processed-image(_hu_)", lang
    if rel in content_files:
        return "resource:page-bundle-copy", lang
    if rel in assets_files:
        return "resource:assets(resources.Get+.Permalink)", lang
    return "UNKNOWN", lang


def main():
    args = sys.argv[1:]
    trace = None
    listout = None
    if "--trace" in args:
        i = args.index("--trace"); trace = args[i + 1]; del args[i:i + 2]
    if "--list" in args:
        i = args.index("--list"); listout = args[i + 1]; del args[i:i + 2]
    public, site = args[0], args[1]

    sections = content_sections(site)

    # static mount: site `static` -> publish root (module mount static->static)
    static_map = {}
    sroot = os.path.join(site, "static")
    for r in walk_files(sroot):
        static_map[r] = os.path.join(sroot, r)

    # content bundle files (non-md), keyed by lower-cased output path
    content_files = {}
    croot = os.path.join(site, "content")
    for r in walk_files(croot):
        if r.endswith(".md") or r.endswith(".gitkeep"):
            continue
        content_files[out_path_for_content(r)] = os.path.join(croot, r)

    assets_files = {}
    aroot = os.path.join(site, "assets")
    for r in walk_files(aroot):
        assets_files[r.lower()] = os.path.join(aroot, r)

    files = walk_files(public)
    cats = Counter()
    by_lang = Counter()
    listing = []
    problems = []
    for rel in files:
        c, lang = classify(public, site, rel, sections, static_map, content_files, assets_files)
        cats[c] += 1
        by_lang[(c, lang)] += 1
        listing.append((c, lang, rel))
        # byte-verify copies
        if c == "static" and sha(os.path.join(public, rel)) != sha(static_map[rel]):
            problems.append(("static-differs", rel))
        if c == "resource:page-bundle-copy" and sha(os.path.join(public, rel)) != sha(content_files[rel]):
            problems.append(("bundle-copy-differs", rel))
        if c.startswith("resource:assets") and sha(os.path.join(public, rel)) != sha(assets_files[rel]):
            problems.append(("assets-copy-differs", rel))

    print(f"TOTAL FILES: {len(files)}\n")
    print(f"{'category':55s} {'total':>6s} {'en':>6s} {'th':>6s}")
    for c, n in sorted(cats.items(), key=lambda kv: (-kv[1], kv[0])):
        print(f"{c:55s} {n:6d} {by_lang[(c,'en')]:6d} {by_lang[(c,'th')]:6d}")
    print()
    if problems:
        print("PROBLEMS:")
        for p in problems[:50]:
            print("  ", p)
    else:
        print("byte-verification of static / bundle / assets copies: OK")

    if listout:
        with open(listout, "w") as f:
            for c, lang, rel in listing:
                f.write(f"{c}\t{lang}\t{rel}\n")

    if trace:
        # cross-check kinds against the Go trace (ground truth for non-paginator renders)
        tre = re.compile(r'rendering outputFormat "([^"]+)" kind "([^"]+)" using layout "([^"]*)" to "([^"]+)"')
        truth = defaultdict(set)
        renders = 0
        for line in open(trace, encoding="utf-8"):
            m = tre.search(line)
            if m:
                renders += 1
                truth[m.group(4).lstrip("/")].add((m.group(1), m.group(2), m.group(3)))
        mine = {rel: c for c, _, rel in listing}
        mism = 0
        for rel, s in truth.items():
            c = mine.get(rel)
            if c is None:
                print("trace target missing from output:", rel); mism += 1; continue
            for fmt, kind, _ in s:
                ok = (c == f"{fmt}:{kind}") or (fmt == "404" and c == "html:404") or \
                     (fmt == "sitemap" and c.startswith("sitemap(")) or \
                     (fmt == "sitemapindex" and c.startswith("sitemapindex")) or \
                     (fmt == "robots" and c.startswith("robotstxt"))
                if not ok:
                    print("MISMATCH", rel, c, fmt, kind); mism += 1
        cnt = Counter()
        for line in open(trace, encoding="utf-8"):
            m = tre.search(line)
            if m:
                cnt[m.group(4)] += 1
        collisions = sorted([k for k, v in cnt.items() if v > 1], key=lambda s: s.encode())
        print(f"\ntrace: {renders} render calls, {len(truth)} unique targets, {mism} kind mismatches")
        print(f"trace: {len(collisions)} targets written twice (last-writer-wins, nondeterministic):")
        for k in collisions:
            print("   ", k, "x", cnt[k])
        layouts = Counter()
        for rel, s in truth.items():
            for fmt, kind, lay in s:
                layouts[(fmt, kind, lay)] += 1
        print("\ntrace: (outputFormat, kind, layout) -> unique targets")
        for k, v in sorted(layouts.items()):
            print("   ", k, v)


if __name__ == "__main__":
    main()
```

Example output: see §1 table (full text in `$W/classify_run1.txt`).

## Appendix B: `absurl_ref.py` (reference port of the canonify lexer; validated against Go)

```python
#!/usr/bin/env python3
"""
Literal Python port of transform/urlreplacers/absurlreplacer.go (neohugo), used as a
reference for the Rust port.  Validated byte-for-byte against the Go transformer on
the whole seeksnack site: apply(out-nomin-nocanon-seq/X) == out-nomin-seq/X for every
HTML/XML file (see __main__).

Quirks preserved on purpose:
  * prefix.nextPos starts at 0 (Go zero value), so on the first 5 loop iterations every
    prefix "matches" at position 0 (only matters if content starts with '/' or a quote);
  * a stale nextPos (< current pos, e.g. inside a srcset value already consumed) is NOT
    re-validated, so the lexer can jump backwards and re-emit bytes;
  * emit() with start > pos panics in Go (slice bounds) -> we raise.
"""
import sys
import unicodedata
from urllib.parse import urlparse

HTML_QUOTES = [b'"', b"'"]
XML_QUOTES = [b"&#34;", b"&#39;"]


def _go_is_space(cp):
    # unicode.IsSpace
    if cp <= 0xFF:
        return cp in (0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x20, 0x85, 0xA0)
    return unicodedata.category(chr(cp)) == 'Zs' or cp in (0x2028, 0x2029)


def _decode_rune(b, i):
    """utf8.DecodeRune: returns (rune, size); invalid -> (0xFFFD, 1)."""
    if i >= len(b):
        return 0xFFFD, 0
    c = b[i]
    if c < 0x80:
        return c, 1
    for n in (2, 3, 4):
        try:
            s = b[i:i + n].decode('utf-8')
            if len(s) == 1:
                return ord(s), n
        except UnicodeDecodeError:
            pass
    return 0xFFFD, 1


def _fields(bs):
    """bytes.Fields: split around runs of unicode.IsSpace."""
    out, cur, i = [], bytearray(), 0
    while i < len(bs):
        r, n = _decode_rune(bs, i)
        if _go_is_space(r):
            if cur:
                out.append(bytes(cur)); cur = bytearray()
        else:
            cur += bs[i:i + n]
        i += n
    if cur:
        out.append(bytes(cur))
    return out


class _Prefix:
    def __init__(self, b, f):
        self.disabled = False
        self.b = b
        self.f = f
        self.nextPos = 0          # Go zero value!

    def find(self, bs, start):
        if self.disabled:
            return False
        if self.nextPos == -1:
            idx = bs.find(self.b, start)      # absolute index (Go: bytes.Index(bs[start:]) is relative)
            if idx == -1:
                self.disabled = True
                return False
            self.nextPos = idx + len(self.b)   # == start + relIdx + len(p.b)
        return True


class _Lexer:
    def __init__(self, content, path, root, quotes):
        self.content = content
        self.out = bytearray()
        self.path = path
        self.root = root
        self.pos = 0
        self.start = 0
        self.quotes = quotes

    def emit(self):
        if self.start > self.pos:
            raise RuntimeError("slice bounds out of range (Go panics here)")
        self.out += self.content[self.start:self.pos]
        self.start = self.pos

    def consume_quote(self):
        for q in self.quotes:
            if self.content.startswith(q, self.pos):
                self.pos += len(q)
                self.emit()
                return q
        return None

    def check_base(self):
        self.consume_quote()
        if not self.content.startswith(b'/', self.pos):
            return
        after = self.pos + 1
        if after >= len(self.content):
            return
        r, _ = _decode_rune(self.content, after)
        if r == ord('/'):
            return  # schemaless //host
        if self.pos > self.start:
            self.emit()
        self.pos += 1
        self.out += self.path
        if self.root and self.content.startswith(self.root, self.pos):
            self.pos += len(self.root)
        self.start = self.pos

    def pos_after_url(self, q):
        if q:
            return self.content.find(q, self.pos)
        # bytes.IndexFunc(r == '>' || IsSpace(r))
        i = self.pos
        while i < len(self.content):
            r, n = _decode_rune(self.content, i)
            if r == ord('>') or _go_is_space(r):
                return i - self.pos
            i += n
        return -1

    def check_srcset(self):
        q = self.consume_quote()
        if q is None:
            return
        if not self.content.startswith(b'/', self.pos):
            return
        after = self.pos + 1
        if after >= len(self.content):
            return
        r, _ = _decode_rune(self.content, after)
        if r == ord('/'):
            return
        pe = self.content.find(q, self.pos)
        pe = pe - self.pos if pe != -1 else -1
        if pe < 0 or pe > 2000:
            return
        if self.pos > self.start:
            self.emit()
        section = self.content[self.pos:self.pos + pe + 1]
        fields = _fields(section)
        for i, f in enumerate(fields):
            if f[:1] == b'/':
                self.out += self.path
                n = 1
                if self.root and f[n:].startswith(self.root):
                    n += len(self.root)
                self.out += f[n:]
            else:
                self.out += f
            if i < len(fields) - 1:
                self.out += b' '
        self.pos += len(section)
        self.start = self.pos

    def replace(self):
        n = len(self.content)
        prefixes = [
            _Prefix(b"src=", _Lexer.check_base),
            _Prefix(b"href=", _Lexer.check_base),
            _Prefix(b"url=", _Lexer.check_base),
            _Prefix(b"action=", _Lexer.check_base),
            _Prefix(b"srcset=", _Lexer.check_srcset),
        ]
        while self.pos < n:
            match = None
            for p in prefixes:
                if not p.find(self.content, self.pos):
                    continue
                if match is None or p.nextPos < match.nextPos:
                    match = p
            if match is None:
                self.pos = n
                break
            self.pos = match.nextPos
            match.nextPos = -1
            match.f(self)
        if self.pos > self.start:
            self.emit()
        return bytes(self.out)


def absurl(content: bytes, base: str, xml: bool) -> bytes:
    """doReplace(path=base, quotes=html|xml). root = url.Parse(base).Path without leading '/'."""
    root = urlparse(base).path.lstrip('/') if '://' in base else base.lstrip('/')
    # Go: url.Parse("https://seeksnack.com/").Path == "/" -> root ""
    lx = _Lexer(content, base.encode(), root.encode(), XML_QUOTES if xml else HTML_QUOTES)
    return lx.replace()


if __name__ == "__main__":
    # usage: absurl_ref.py <nocanon_unminified_dir> <canon_unminified_dir>
    import os
    a, b = sys.argv[1], sys.argv[2]
    base = "https://seeksnack.com/"
    STATIC_HTML = {"admin/index.html", "pinterest-e4aaf.html", "serviceworker.html",
                   "yandex_10ba5f74f6b19b4e.html", "ytss.html"}
    n = ok = 0
    for dp, _, fns in os.walk(a):
        for fn in fns:
            rel = os.path.relpath(os.path.join(dp, fn), a)
            if fn.endswith(".html"):
                xml = False
            elif fn == "index.xml":
                xml = True     # RSS: always canonified (XML quotes)
            else:
                continue
            src = open(os.path.join(a, rel), "rb").read()
            exp = open(os.path.join(b, rel), "rb").read()
            if rel in STATIC_HTML:
                continue  # static files are copied by fsync, never transformed
            got = absurl(src, base, xml)
            n += 1
            if got == exp:
                ok += 1
            else:
                print("MISMATCH", rel)
    print(f"{ok}/{n} files match")
```

## Appendix C: `compare_outputs.py` (acceptance comparator)

```python
#!/usr/bin/env python3
"""
Byte-compare a candidate build (e.g. the Rust port's output) against the golden
neohugo output, tolerating the known nondeterministic term-collision files.

Usage:
  python3 compare_outputs.py <candidate_dir> <golden_dir> [--variants collision_variants.json]
  python3 compare_outputs.py --make-variants <out.json> <build_dir> [<build_dir> ...]

--make-variants collects, for every known collision target (26 files: 13 term
URLs x {index.html,index.xml}), the sha256 of each distinct variant seen in the
given (cold-cache, --minify) builds. With --variants, a candidate file at such a
path passes if it matches ANY recorded variant.

Exit code 0 when everything matches.
"""
import hashlib
import json
import os
import sys

COLLISION_TARGETS = [
    "brands/lays/index.html", "brands/lays/index.xml",
    "ingredients/disodium-5-guanylate/index.html", "ingredients/disodium-5-guanylate/index.xml",
    "ingredients/disodium-5-inosinate/index.html", "ingredients/disodium-5-inosinate/index.xml",
    "ingredients/disodium-5-ribonucleotide/index.html", "ingredients/disodium-5-ribonucleotide/index.xml",
    "ingredients/disodium-5-ribonucleotides/index.html", "ingredients/disodium-5-ribonucleotides/index.xml",
    "ingredients/ins-322i/index.html", "ingredients/ins-322i/index.xml",
    "ingredients/ins-500ii/index.html", "ingredients/ins-500ii/index.xml",
    "tags/lays/index.html", "tags/lays/index.xml",
    "th/ingredients/ins-322i/index.html", "th/ingredients/ins-322i/index.xml",
    "th/ingredients/ins-500ii/index.html", "th/ingredients/ins-500ii/index.xml",
    "th/ingredients/ไดโซเดียม-5-กัวไนเลต/index.html",
    "th/ingredients/ไดโซเดียม-5-กัวไนเลต/index.xml",
    "th/ingredients/ไดโซเดียม-5-ไรโบนิวคลีโอไทด์/index.html",
    "th/ingredients/ไดโซเดียม-5-ไรโบนิวคลีโอไทด์/index.xml",
    "th/ingredients/ไดโซเดียม-5-ไอโนซิเนต/index.html",
    "th/ingredients/ไดโซเดียม-5-ไอโนซิเนต/index.xml",
]


def walk(root):
    out = set()
    for dp, _, fns in os.walk(root):
        for fn in fns:
            out.add(os.path.relpath(os.path.join(dp, fn), root).replace(os.sep, "/"))
    return out


def sha(p):
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for c in iter(lambda: f.read(1 << 16), b""):
            h.update(c)
    return h.hexdigest()


def make_variants(out, builds):
    v = {}
    for t in COLLISION_TARGETS:
        s = []
        for b in builds:
            p = os.path.join(b, t)
            if os.path.exists(p):
                h = sha(p)
                if h not in s:
                    s.append(h)
        v[t] = s
    with open(out, "w") as f:
        json.dump(v, f, indent=1, ensure_ascii=False)
    print(f"wrote {out}: " + ", ".join(f"{len(x)}" for x in v.values()))


def main():
    a = sys.argv[1:]
    if a and a[0] == "--make-variants":
        make_variants(a[1], a[2:])
        return 0
    variants = {}
    if "--variants" in a:
        i = a.index("--variants")
        variants = json.load(open(a[i + 1]))
        del a[i:i + 2]
    cand, gold = a[0], a[1]
    cf, gf = walk(cand), walk(gold)
    missing = sorted(gf - cf, key=lambda s: s.encode())
    extra = sorted(cf - gf, key=lambda s: s.encode())
    differ, tolerated = [], []
    for r in sorted(gf & cf, key=lambda s: s.encode()):
        hc = sha(os.path.join(cand, r))
        if hc == sha(os.path.join(gold, r)):
            continue
        if r in variants and hc in variants[r]:
            tolerated.append(r)
        else:
            differ.append(r)
    print(f"golden files: {len(gf)}  candidate files: {len(cf)}")
    print(f"missing: {len(missing)}  extra: {len(extra)}  differ: {len(differ)}  tolerated-collision-variants: {len(tolerated)}")
    for name, lst in (("MISSING", missing), ("EXTRA", extra), ("DIFFER", differ)):
        for r in lst[:40]:
            print(f"  {name} {r}")
        if len(lst) > 40:
            print(f"  ... {len(lst) - 40} more")
    return 0 if not (missing or extra or differ) else 1


if __name__ == "__main__":
    sys.exit(main())
```
