# nh-hugolib — porting notes

neohugo hugolib/** : HugoSites/Site/pageState, content capture, content map + assembly, content rendering + shortcodes, render loop, aliases, postProcess, build stats.

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `hugo_sites` | `hugolib/hugo_sites.go` (minus data loading) + `hugolib/site.go` 143-454, 858-867 (construction) | T20 hugolib-capture | crate lead |
| `build_process` | `hugolib/hugo_sites_build.go` process/processFull/processFiles | T20 hugolib-capture | split of hugo_sites_build.go |
| `template_exec` | — | T20 hugolib-capture | NEW: single dispatch point for hugolib template executions (test seam) |
| `pages_capture` | `hugolib/pages_capture.go` | T20 hugolib-capture |  |
| `content_map` | `hugolib/content_map.go` | T20 hugolib-capture |  |
| `content_map_trees` | `hugolib/content_map_page.go` 88-259, 636-994 | T20 hugolib-capture | split: trees, shifter, pageMap struct, newPageMap |
| `page__meta` | `hugolib/page__meta.go` (minus setMetaPost*, applyDefaultValues, initLazyProviders) | T20 hugolib-capture |  |
| `page__new` | `hugolib/page__new.go` | T20 hugolib-capture |  |
| `page__content_parse` | `hugolib/page__content.go` 60-224, 266-490 | T20 hugolib-capture | split: parse half |
| `shortcode_parse` | `hugolib/shortcode.go` 193-310, 526-545, 562-736 | T20 hugolib-capture | split: extraction half |
| `page_kinds` | `hugolib/page_kinds.go` | T20 hugolib-capture |  |
| `file_info` | `hugolib/fileInfo.go` | T20 hugolib-capture |  |
| `content_map_page` | `hugolib/content_map_page.go` (queries + assembly) | T21 hugolib-assemble |  |
| `page__meta_post` | `hugolib/page__meta.go` setMetaPost, setMetaPostParams, applyDefaultValues | T21 hugolib-assemble | split |
| `build_assemble` | `hugolib/hugo_sites_build.go` assemble + `hugolib/site.go` initRenderFormats, shouldBuild | T21 hugolib-assemble | split |
| `page__init` | `hugolib/page__meta.go` initLazyProviders + `hugolib/page.go` initPage, initCommonProviders, shiftToOutputFormat, template variations | T21 hugolib-assemble | split |
| `page__paths` | `hugolib/page__paths.go` | T21 hugolib-assemble |  |
| `pagecollections` | `hugolib/pagecollections.go` | T21 hugolib-assemble |  |
| `collections` | `hugolib/collections.go` | T21 hugolib-assemble |  |
| `page__data` | `hugolib/page__data.go` | T21 hugolib-assemble |  |
| `page__tree` | `hugolib/page__tree.go` | T21 hugolib-assemble |  |
| `site_sections` | `hugolib/site_sections.go` | T21 hugolib-assemble |  |
| `page__content` | `hugolib/page__content.go` (render half) | T22 hugolib-content |  |
| `page__output` | `hugolib/page__output.go` | T22 hugolib-content |  |
| `page__per_output` | `hugolib/page__per_output.go` + `hugolib/site.go` hookRendererTemplate | T22 hugolib-content |  |
| `shortcode` | `hugolib/shortcode.go` (render half) | T22 hugolib-content |  |
| `shortcode_page` | `hugolib/shortcode_page.go` | T22 hugolib-content |  |
| `site` | `hugolib/site.go` (the rest) | T23 hugolib-site |  |
| `hugo_sites_data` | `hugolib/hugo_sites.go` Data, loadData, handleDataFile, readData | T23 hugolib-site | split |
| `site_output` | `hugolib/site_output.go` | T23 hugolib-site |  |
| `page` | `hugolib/page.go` (the rest) | T23 hugolib-site |  |
| `page__common` | `hugolib/page__common.go` | T23 hugolib-site |  |
| `page__menus` | `hugolib/page__menus.go` | T23 hugolib-site |  |
| `page__paginator` | `hugolib/page__paginator.go` | T23 hugolib-site |  |
| `page__position` | `hugolib/page__position.go` | T23 hugolib-site |  |
| `page__ref` | `hugolib/page__ref.go` | T23 hugolib-site |  |
| `page_unwrap` | `hugolib/page_unwrap.go` | T23 hugolib-site |  |
| `gitinfo` | `hugolib/gitinfo.go` | T23 hugolib-site | STUB (enableGitInfo=false) |
| `codeowners` | `hugolib/codeowners.go` | T23 hugolib-site | STUB |
| `permalinker` | `hugolib/permalinker.go` | T23 hugolib-site |  |
| `tplapi::page_methods` | — | T23 hugolib-site | NEW: template-visible method set of page.Page (replaces reflection on *pageState) |
| `tplapi::site_methods` | — | T23 hugolib-site | NEW: template-visible method set of page.Site (*page.siteWrapper) and *hugolib.Site |
| `tplapi::named_types` | — | T23 hugolib-site | NEW: NamedTypeRegistry assembly (page.Pages, resource.Resources, maps.Params, page.Taxonomy ...) |
| `hugo_sites_build` | `hugolib/hugo_sites_build.go` (Build, render, writeBuildStats, postProcess, ...) | T24 hugolib-build |  |
| `site_render` | `hugolib/site_render.go` + `hugolib/site.go` render | T24 hugolib-build |  |
| `alias` | `hugolib/alias.go` | T24 hugolib-build |  |

Go files split across modules (hugo_sites_build.go, site.go, content_map_page.go, page__content.go,
shortcode.go, page.go, page__meta.go) are split so that each build phase has one owner whose
acceptance test needs only earlier phases (HUGO_LAYER.md §12.1). Each part's GO PORTING CHECKLIST
lists exactly its line ranges.

## Dependencies

- nh-*: nh-common, nh-parser, nh-langs, nh-config, nh-media, nh-hugofs, nh-markup, nh-transform, nh-tpl, nh-helpers, nh-resource, nh-images, nh-page, nh-doctree, nh-allconfig, nh-tplimpl, nh-resources, nh-resource-transformers, nh-esbuild, nh-publisher, nh-deps, nh-i18n, nh-tplfuncs
- Wave A: go-json, go-time, go-fmt, go-path, go-sort, go-strconv, go-unicode
- crates.io (justify each): none. dev: `serde_json` (fixture reader), `flate2` with
  `rust_backend` (gunzip of the fixtures; decompression only, README rule 2).

## T20 hugolib-capture: status

Construction (`HugoSites::new` = Go `NewHugoSites` + `newHugoSites`) and `process`
(`build_process::process` -> `pages_capture` -> `content_map::add_fi` -> `page__new::new_page`
-> `page__content_parse` / `shortcode_parse`) are ported. Every EX entry of the T20 checklists is
`OK` except: `buildCounters.loggFields` (logging only), `loadGitInfo` (explicit error when
`enableGitInfo` is set; off in scope), `BuildCfg.shouldRender` (T24: needs render state),
`pageMeta.newContentConverter` (T22: needs the render-hook page). Stubs: content adapters
(`_content.gotmpl`: `addPagesFromGoTmplFi` and adapter front matter return
`neohugo-rs: content adapters (_content.gotmpl) are not supported`), partial rebuilds.

### Construction order (for T21–T25)

1. memory cache; first site `Deps::init(DepsCfg{fs, conf: first language config, log,
   build_state, mem_cache, counters})` (HasBytes publish fs, PathSpec+BaseFs, ContentSpec with
   the site's content types and logger, SourceSpec, file caches, resources Spec with
   `SpecCommon.incr` = the BuildState);
2. `Configs::validate`; `PageTrees::new(num_languages)`;
3. per `Configs::config_langs()` (default language first): `FrontMatterHandler::new_with_logger`,
   `Language::set_params(conf.params)`, deps (`clone_for` for i > 0), `new_page_map(i,
   new_content_map_config(conf, lang))`, `new_site_ref_linker`, `DestinationPublisher::new(publish
   fs, minifiers::Client::new(site media types, site output formats, cfg), build stats)`,
   `RelatedDocsHandler::new(conf.related)`;
4. sites sorted (default language, weight, lang);
5. `HugoInfo::new_with_deps` (modules as dependencies); per site: `TemplateStore::new` (i = 0,
   with `StoreOptions.log`, the named-type registry of `tplapi::named_types`) or
   `with_site_opts`, both with `SiteOptions{site: deps.site (the same `Arc<OnceLock>`), func
   map from the factory}`, stored in `Deps.template_store` and `Site.template_store`; then
   `compile_deps` (i18n `new_resource` for the first, `clone_resource` for the others).
6. `HugoSites::freeze()` sets every `Deps.site` (a `SiteHandle`); T24's `build` calls it after
   assemble.

### Test seams

- `NewHugoSitesCfg.func_map_factory`: `None` = `nh_tplfuncs::tplimplinit::create_func_map`;
  tests pass the names-only func map (`tests/support/mod.rs::names_only_func_map_factory`, names
  from the `funcnames` fixture, every function a failing stub).
- `tests/support/mod.rs::new_sites(site_json, tmp)`: recreates a fixture site in a temp dir,
  loads the config like the Go oracle (production, no environment, caches in `tmp/_cache`) and
  returns `HugoSites` + the captured log. T21/T22 can start from it and call
  `build_process::process`.
- `template_exec::execute` / `HugoSites.template_executor` (T22 replay, T24 stub): implemented;
  `None` executes through the site's template store with the caller's context.
- `Deps::for_tests` unchanged.

### Data model notes

- `ContentNode::Page(PageId, lang_index)` (the shifter needs the page's language without the
  arena); `ContentNode::Resources`/`Pages` are the per-language slots. `ResourceSource.page` is a
  bundled content file's page (Go `r.(*pageState)`), `ResourceSource.path` is `None` then.
- `PageMap.dims` is the site's shape of the shared trees (Go `pageTrees.Shape(0, i)`); pass it
  to every tree method/walk. `PageMap.cfg` is Go's `contentMapConfig`; `page_reverse_index` is
  `contentTreeReverseIndex` (`get(key, &trees, &pages, dims)`).
- `ContentNodeShifter` implements `delete_in_place` (Go's in-place `Delete`); walks that modify
  the trees must use `NodeShiftTree::walk_mut` (T21).
- `PageMeta`: `kind()` is `page_config.kind` (the skeleton's `kind` field is gone), `lang` is the
  site's language (Go `p.s.Lang()`), `params()` is Go's `Params()` (front matter after
  `PrepareParams`; the `params: Arc<Map>` field is still T21's normalised params).
- `new_page(h, NewPageMeta{..})` is Go's `h.newPage(&pageMeta{...})` for T21's synthetic pages
  too: set `path_info`, `site_idx`, `page_config` (kind), `standalone_output_format`, `term`,
  `singular` as Go sets them.
- Page ids: `PageId` = arena index (creation order); `pid` = `HugoSites.page_id_counter` + 1 (Go's
  `pageIDCounter`; only unique).
- `CachedContent.pi.source` holds the source bytes (read once; Go re-reads through the
  `/cont/src` dynacache). `ContentItem::Source{low, high}` are Go's `v.Pos()..v.Pos()+len(Val)`.
- `Shortcode.templ` (Go `templ`) replaces the skeleton's `info`/`templs`; `indentation` is bytes;
  `params` is `Invalid`, a `[]interface {}` list, a `map[string]interface {}` map, or
  `TypedNil("[]string")` (Go's nil `[]string` for a shortcode without params).

### Go behaviour reproduced on purpose

- `AddFi` in Go's single-worker order (files are added where Go enqueues them), so page ids,
  duplicate handling and duplicate warnings (`printPathWarnings`) are Go's.
- Duplicates: page vs bundle/section at the same base, same base in two languages by filename,
  `index.md` at the content root (leaf-bundle home warning), `lang`/`kind`/`path` front matter
  (`lang` of a disabled language drops the page), taxonomy detection by character prefix
  (`/tagsfoo` is under `/tags`), term values from the unnormalized base.
- Error texts: a file error wrapped by `process` reads `readAndProcessContent: "file:l:c": msg`
  as in Go (nh-common's `Error::wrap`); the capture test compares the texts unchanged.

### Deviations

1. Taxonomy views: Go builds them from a map in random order and sorts by plural
   (`sort.Slice`); two singulars with the same plural would come out in random order. The port
   starts from the singulars in byte order (then the same pdqsort).
2. The content source is read once at page creation (see above), not through `/cont/src`.
3. No stale/identity tracking (`MarkStale`, rebuilds).
4. `HugoSites.freeze` creates a reference cycle (`SiteHandle` in `Deps.site`) that lives until
   the process ends, like Go's pointer graph.
5. `cargo fmt` of the crate (required by the checks) reformatted the other tasks' skeleton
   modules (whitespace only); `page.rs` got a one-line doc fix for clippy and
   `tplapi/page_methods.rs` uses `meta.kind()`.

### Verification (T20)

| topic | inputs | Rust test | checks |
|---|---|---|---|
| `capture/*.json.gz` | 14 sites: docs/ (958 pages; passthrough and emoji turned off: not ported, not capture-relevant), hugolib/testsite, nh-page's synthetic en/th site, the seeksnack reconstruction (`crates/nh-allconfig/tests/fixtures/load/seeksnack/hugo.toml`) with a synthetic en/th tree (sections, nested sections, leaf/branch bundles with resources, bundled content pages incl. a Thai one, terms and taxonomy `_index`, drafts/future/expired, build options, headless, cascade (map and list forms), translations by filename, Thai paths, YAML/TOML/JSON/no/empty front matter, `path`/`lang`/`kind` overrides, ignored files, summary dividers, shortcodes: positional/named/typed params, `{{% %}}` and `{{< >}}`, nested, indented, inline-less, version 1, embedded ref/relref, escaped), Go's `TestExtractShortcodes` inputs, 5 shortcode error sites, edge trees (duplicates, character-level taxonomy prefix, spaces/punctuation/dots/BOM/empty files), translations by contentDir, a leaf-bundle home, disabled kinds | `tests/capture.rs` | trees in raw walk order (keys, node kinds, language slots), each site's shaped walk with the match flag, 1,143 pages (kind, path info, file, PageConfig kind/path/lang/params as typed values, cascade, content items, shortcodes), errors, log: 0 differences |
| `funcnames/funcnames.json.gz` | `tplimplinit.CreateFuncMap` names (159) | `tests/support` (names-only func map) | used by every test |
| construction | the seeksnack reconstruction | `tests/construction.rs` | 2 sites en/th, deps sharing, one PostProcess counter, template stores (+ ref/relref), translators, publishers, freeze |

Regenerate (both byte for byte; builds sites in a temp dir, never in the repository):

```sh
export GOTOOLCHAIN=go1.27.1
go run ./tools/go-oracle/nh-hugolib/capture -root .
go run ./tools/go-oracle/nh-hugolib/funcnames -root .
```

The docs and testsite fixtures reference repository files by FNV hash; the test fails with a
"regenerate" message when they change.

## T22 hugolib-content: status

`page__content` (render half of page__content.go), `page__output`, `page__per_output` (+ site.go
`hookRendererTemplate`, page__meta.go `newContentConverter`, the page.go helpers content rendering
needs), `shortcode` (render half) and `shortcode_page` (+ `ShortcodeWithPage`) are ported. Every
EX entry of their checklists is `OK`; not ported: identity/dependency tracking
(`trackDependency`, `IdentifierBase`, `GetDependencyManager`), and the `pageForShortcode` /
`pageForRenderHooks` methods, which are T23's method tables (`tplapi/page_methods.rs`).

### API for T21, T23, T24

- `PageOutput::new(h, ps, pp, f, render)` (Go `newPageOutput`; takes `&HugoSites` for the
  self-reference cell, see below) or `PageOutput::new_with_target_paths(h, ps, holder, f,
  render)`. `set_content_provider(Some(pco))`, `pco()`, `provider_slot()`, `content_renderer()`
  (the content output behind the slot: `Pco`, or the `Lazy` provider's, created on first use;
  `None` = nop). `get_internal_template_base_path_and_descriptor(ps)` is Go's
  `(po *pageOutput) GetInternalTemplateBasePathAndDescriptor` (page.go:480-492, in T23's
  checklist: implemented here, where the type lives).
- `PageContentOutput::new(&po, idx)` (Go `newPageContentOutput`). For `shiftToOutputFormat`'s
  lazy provider (T21): `LazyContentProvider::new(Box::new(move || Ok(PageContentOutput::new(&po,
  idx)? as Arc<dyn OutputFormatContentProvider>)))`; `ContentProviderSlot::resolve()` downcasts it
  back.
- Template methods (T23): `pco.content(ctx, &handle)`, `plain(ctx, &handle)`, `plain_words`,
  `summary`, `truncated`, `word_count`, `fuzzy_word_count`, `reading_time`, `len`,
  `table_of_contents`, `fragments`, `content_without_summary`, `render_string(ctx, &handle, args)`,
  `render_shortcodes`, `render(ctx, layouts)`, `markup(opts)` / `c()` (the scope,
  `CachedContentScope`: the `page.Markup` value). All take the template context as `HostCtx` and
  pass it through unchanged. A page whose current provider is the nop provider returns Go's nop
  values (T23).
- `impl PageState` in page__per_output.rs: `path_or_title`, `pos_from_input`, `pos_offset`,
  `parse_error`, `wrap_error`, `get_page_info_for_error`, `get_content_converter(h, po)` (page.go
  checklist entries of T23: do not duplicate them, mark them `OK`); `new_content_converter(h, ps,
  markup)` is page__meta.go's `newContentConverter` (T20 left it to T22).
- `HugoSites.self_ref` is now `page__output::HugoSitesRef` (`Arc<OnceLock<Weak<HugoSites>>>`,
  set by `freeze` as before): page outputs are created during assembly, before the `Arc` exists,
  and content rendering reaches the frozen `HugoSites` through the shared cell.
- `template_exec::ExecCall.ordinal`: T22's callers pass 0; an executor that needs Go's
  per-(page, format, kind) ordinal counts the calls itself (the T22 replay does). Hooks are
  `ExecKind::Hook("link" | "image" | "heading" | "codeblock" | "blockquote" | "table" |
  "passthrough")`, shortcodes `ExecKind::Shortcode(name)`; `output_format` is the content output's
  format (Go `pco.po.f.Name`), which differs from the page's current format when content is
  reused.
- Go's hugolib-only context keys travel in `TplContext.host` as `page__content::HostState`
  (the content callback of `setGetContentCallbackInContext`, used by `.RenderShortcodes`). T23/T24
  code that sets `host` must keep it (extend `HostState` rather than replacing it).
- Rendering never holds a lock (§4.8): the render-hook cache lock is held for the template lookup
  only (as Go's `renderCacheMu`); content caches are `PageMap` partitions (compute outside the
  lock, first stored wins).
- `.HasShortcode` (T23's method table): `PageState::has_shortcode(name)` (page__per_output.rs;
  Go `(p *pageState) HasShortcode`, page.go L365-371, in T23's checklist: mark it `OK` there).

### `transferNames` (follow-up to T22)

Go's `shortcodeHandler.nameSet` is a map behind a `sync.RWMutex` because it grows during
rendering: `transferNames(in)` adds the names of another handler, and `.HasShortcode` reads it
(`hasName`). Go calls it in two places, both ported:

- `cachedContentScope.RenderString` (page__content.go L1039): after rendering a string that
  contains shortcodes, the names of the string's own handler go to the page's handler (`We need
  a consolidated view in $page.HasShortcode`), whatever template called it (shortcode, render
  hook, another page's template).
- the content callback of `contentToC` (page__content.go L714), called by `.RenderShortcodes`
  when the context carries it: the included page's names go to the including page's handler.
  The callback is in the context only while the including page's `{{% %}}` shortcodes run, so
  `{{% include %}}` transfers and `{{< include >}}` does not; chains transfer transitively (the
  included page's set already holds what it included or rendered with `RenderString`).

`ShortcodeHandler.name_set` is private and an `RwLock<BTreeSet<String>>`, held only to read or
insert names (HUGO_LAYER.md §4.8): `add_name(&self)`, `has_name`, `names()` (byte order; Go's map
order never matters) and `transfer_names(&self, input)`, which copies `input`'s names under its
read lock first and then inserts them under this set's write lock, so the two locks are never
held together (Go holds only the target's lock while it ranges over the source; a page that
transfers into itself cannot deadlock the port). The order of the calls is Go's (a probe of
`.HasShortcode` sees exactly the names transferred so far).

### T22 deviations

1. The markup converter (Go `pageState.contentConverter`, once per page) is kept per page OUTPUT
   (`PageOutput::content_converter`); converters are stateless between conversions (a fresh ID
   factory per conversion), so every output renders the same bytes.
2. `expandShortcodeTokens`: Go checks `(k+4) < len(source)` and then slices `source[end:end+4]`,
   which can read past `len` into the slice's spare capacity (or panic) for a token right at the
   end of the content after a `<p>`; the port treats such a token as not wrapped (Go's own test
   table, ported in `shortcode.rs`, expects exactly that result). Markdown output never ends inside
   a `<p>`.
3. No identity/dependency tracking and no stale versions (`StaleValue` is always fresh, `version`
   is the render version); `Reset` does not replace the render hooks (no server mode).
4. Error texts: `wrapError` adds the filename as the error position (Go's
   `hugofs.AddFileInfoToError` also reads line numbers from the file); `parseError`/shortcode
   errors use `herrors` file errors. Only texts differ, never bytes of a successful build.

### Go behaviour reproduced on purpose

- Content reuse across output formats: `initRenderHooks` moves the page's template variations
  state 0 -> 1; a hook lookup that finds more than one output-format candidate (or a shortcode with
  more than one `ofCount` format, or a `.RenderShortcodes`d page with variations) increments it,
  and only state 1 lets `shiftToOutputFormat` reuse another output's content output. The hook
  descriptor comes from the page's CURRENT output while the cache key and the rendered-content
  key use the content output's format; `getRenderer` does not cache "not found".
- Front matter `summary` is rendered through `po.contentRenderer.ParseAndRenderContent` +
  `TrimShortHTML` whenever the computed summary is empty (also an empty manual summary); a
  summary divider wins over front matter `summary` only when the summary before it is non-empty.
- CJK word counts (`isCJKLanguage`: runes per non-ASCII word), `(wc+500)/501` vs `(wc+212)/213`
  reading time, fuzzy `(wc+100)/100*100`; Thai is not CJK (`hasCJKLanguage` detection), so a
  Thai sentence without spaces is one word.
- `{{% %}}` inner content of nested/version-1 shortcodes is rendered with the page's CURRENT
  output's content renderer and loses a single wrapping `<p>…</p>\n` when the inner text has no
  newline (Go's `\A<p>(.*)</p>\n\z`); shortcodes without inner content re-indent every line after
  the first with the source indentation; `.Parent.Inner` is empty inside the child (Go sets
  `Inner` after the children rendered).
- `is_in_goldmark` is set only by the render function of `{{% %}}` shortcodes in markdown
  content; hooks and `{{< >}}` shortcodes get the caller's context (the hooks run inside a nested
  `{{% %}}` render see it set, as in Go).
- `mustContentToC` panics on error (Go panics in `.TableOfContents`/`.Fragments`); the other
  `must*` report a fatal error and return zero values.

### Verification (T22)

The acceptance replays every render-hook and shortcode template execution of the Go build
through `template_exec::TemplateExecutor` (`tests/content_support`: keyed (page, content output
format, kind, ordinal); a call without a record, a recorded call never made, or a different
`is_in_goldmark` fails; Go pids in recorded placeholders/hugocontext markers are mapped to the
port's). The harness sets what T21's assembly computes and content reads (kind, type, layout,
markup, content media type, front matter summary, `isCJKLanguage`, render formats) and stands in
for `initLazyProviders`/`shiftToOutputFormat(true, idx)` (outputs per format name, content reuse).

| topic | inputs | Rust test | checks |
|---|---|---|---|
| `content/*.json.gz` (but `hasshortcode`) | process + assemble in Go, then the render loop's order: per site and render format `preparePagesForRender` on all sites, then every page (with a file) of the rendering site: `.TableOfContents`, `.Content`, `.ContentWithoutSummary`, summary type, `.Summary`, `.Truncated`, `.Plain`, `.PlainWords`, `.WordCount`, `.FuzzyWordCount`, `.ReadingTime`, `.Len`, `.Fragments` (+ `ToHTML(1,3,true)`), the content output's format, the variations state, fatal errors. Sites: a content-focused en/th site (`{{< >}}`/`{{% %}}`, nested, `.Inner`/`.InnerDeindent`, positional/named params, `.Parent`, `.Ordinal`, inline, version 1, output-format shortcode variants, `.Page.TableOfContents`, shortcodes in summaries; manual/front matter/auto summaries, `summaryLength`; link, image, heading (+ rss variant), codeblock, blockquote/alert, table (+ `render-table.json.json`), passthrough off; TOC levels; CJK/`isCJKLanguage`/Thai; HTML content and `markup: html`; bundled page; JSON and RSS outputs), the seeksnack reconstruction (+ its hook templates), hugolib/testsite, docs/ (+ a code block hook: Chroma is not ported; values as FNV hashes), Go's TestExtractShortcodes site | `tests/content.rs` | 4,017 page/format value sets (docs 3,772), 19,128 replayed executions, 0 differences |
| `hookrec/*.json.gz` | a REAL Go build (page layouts reading `.Content`, `.Summary`, `.Plain`, `.TableOfContents`, `.WordCount` in html/json/rss) of the same sites but docs: every hook/shortcode execution recorded, then the rendered-content caches (`/cont/ren`, `/cont/toc`, `/cont/pla`) per page and format | `tests/hookrec.rs` | 97 cache entries, 284 replayed executions, 0 differences |
| unit | Go's `TestReplaceShortcodeTokens` table; the `<p>` cleanup regexp | `src/shortcode.rs` | all equal |
| `content/hasshortcode.json.gz` | the content oracle in ACTION MODE on `rec.HasShortcodeSite` (en/th): the `RenderString` and `.RenderShortcodes` calls of the templates are recorded with the execution that made them (their own executions are replayable records, not nested ones), and `.HasShortcode` of every page for every name of the site (+ one never used) is probed before and after each call (`HasShortcode "__nhprobe"` in the templates) and after each page's values, in render order. Cases: `RenderString` on the page itself from `{{< >}}` and `{{% %}}` shortcodes, with `dict "display" "block"`, without shortcodes, from a link render hook, on another page, in a Thai page; `.RenderShortcodes` from `{{% include %}}` (with the included page's own `RenderString`), a chain of three includes, and from `{{< include >}}` (no transfer) | `tests/content.rs` (`content_hasshortcode`; `content_support::Replay` makes each recorded call on the port with the execution's context and compares its result and every probe) | 48 page/format value sets, 70 replayed executions, 11 `RenderString`, 13 `.RenderShortcodes`, 57 in-template probes + 48 per-page probes, 0 differences |

Mutations checked (each fails the tests): reading time divisor, the `{{% %}}` inner `<p>` cleanup
and the `<p>TOKEN</p>` unwrap, the hook-candidate variation increment (unused records), setting
`is_in_goldmark` for `{{< >}}` shortcodes, dropping either `transferNames` call.

The action mode is off for the other sites, so their fixtures are unchanged. Page ids come from
one counter per oracle process: regenerate with the full command below (a single `-site` run
numbers the pages differently).

Regenerate (byte for byte; sites are built in a temp dir, never in the repository):

```sh
export GOTOOLCHAIN=go1.27.1
go run ./tools/go-oracle/nh-hugolib/content -root .
go run ./tools/go-oracle/nh-hugolib/hookrec -root .
```

## T21 hugolib-assemble: status

`build_assemble` (`assemble`, `initRenderFormats`, `shouldBuild`), `content_map_page` (queries +
assembly steps), `page__meta_post` (`setMetaPost`, `setMetaPostParams`, `applyDefaultValues`),
`page__init` (`initLazyProviders`, `initPage`, `initCommonProviders`, `shiftToOutputFormat`,
template variations), `page__paths`, `pagecollections`, `collections`, `page__data`,
`page__tree` and `site_sections` are ported. Every checklist entry of these modules is `OK`
(also the non-EX ones); not ported: the rebuild-only functions of content_map_page.go
(`debugPrint`, `dynacacheGC*`, `resolveAndClearStateForIdentities`,
`resolveAndResetDependententPageOutputs`) and the rebuild branches inside the assembly steps
(`assembleChanges`, post hooks comparing dates). No stubs on the build path; resource configs
of content adapters (`rs.rc`) return the explicit unsupported error (capture already rejects
content adapters).

### Page-output lifecycle (for T23, T24)

- `build_assemble::assemble(&mut h, &cfg)` after `build_process::process`, then
  `h.freeze()`. It fills `Site.home`, `Site.lastmod`, `Site.render_formats`,
  `HugoSites.render_formats` (global index), the main sections (`conf.compiled()`), and
  `HugoSites.translation_key_pages`; `assembleResources` already ran `init_page` and
  `shift_to_output_format(h, true, 0)` on every page (the page outputs and bundle resources
  exist before freezing).
- `PageState::init_page(h) -> Result<&PageLazy>` (Go `initPage`; the result or error is
  cached): `PageLazy.paths` (`PagePaths`: `output_formats()` is Go's `.OutputFormats`,
  `first_output_format`, `target_paths` by format name) and `outputs` (one per global render
  format, shared `Arc` per format NAME; a standalone page has one slot). The target path
  descriptor is `PageCommon.target_path_descriptor` (unset for a page without output formats,
  Go's zero descriptor).
- `PageState::shift_to_output_format(h, is_rendering_site, idx)` (T24's
  `HugoSites::prepare_pages_for_render` already calls it for every page of every site before
  each format): sets `current_output_idx` (0 for one-slot pages); rendering site: resets a
  BUILT paginator, installs the output's `pco` (created, or reused from another output when
  `can_reuse_page_output_content`); other sites: installs (or resets) a
  `ContentProviderSlot::Lazy` whose factory creates the content output of the page's CURRENT
  output when first used (Go's closure reads `p.pageOutput`), `pco` untouched. The lazy
  factory needs the frozen `HugoSites` (installed only by the render loop).
- `PageState::current_output()` (T20's `page.rs`) is the current output: its
  `target_paths` give `.RelPermalink`/`.Permalink` (`target_paths.output_format`),
  `rel_url`, the target file name; `po.render` whether it renders; `po.paginator`.

### Queries and navigation API (for T23)

- `content_map_page::PageMap::get_pages_in_section(h, site, &PageMapQueryPagesInSection{path,
  key_part, recursive, include_self, include})`, `get_pages_with_term(h, site,
  &PageMapQueryPagesBelowPath{path, key_part, include})`, `get_terms_for_page_in_taxonomy`
  (`.GetTerms`), `get_or_create_resources_for_page(h, page)` (`.Resources`),
  `for_each_page`/`for_each_page_including_bundled_pages`; `site_taxonomies(h, site)` is
  `Site.Taxonomies()`'s lazy init (`create_site_taxonomies`). `page_predicates::*` are Go's
  `pagePredicates` (`and`/`or` = `P.And`/`P.Or`). The exact query shapes of page.go
  `Pages()`/`RegularPages()` and site.go `Pages()`/`RegularPages()` are in
  `tests/assemble.rs` (`Dumper::pages_of`, ...): copy them into T23's methods.
- `pagecollections::new_page_finder(h, site)`: `get_page_for_refs(&refs)` (`Site.GetPage`;
  `None` becomes `page.NilPage`), `get_page(Some(ctx), ref)` (`pageSiteAdapter.GetPage`),
  `get_page_ref(ctx, ref)` (ref/relref).
- `page__tree`: `parent`, `current_section`, `first_section`, `in_section`, `is_ancestor`,
  `is_descendant`, `ancestors`, `sections`, `page`, `sections_entries`, `sections_path` on
  handles, and `parent_id`/`current_section_id`/`first_section_id`/`sections_*_id` on the
  arena. `page__data::data(&handle)` (`.Data`), `collections::{slice, group}`,
  `site_sections::{home, sections}`.
- `page__paths::get_language_target_path_lang`/`get_language_permalink_lang` (crate-visible)
  are Go's `Site` methods the descriptor needs during assembly; T23's `Site` methods can
  delegate to them.

### Changes outside the T21 modules

- `hugo_sites.rs` (T20): new field `HugoSites.translation_key_pages` (Go
  `translationKeyPages`).
- `tplapi/page_methods.rs` (T23): `Page::weight()`, `Page::site()` and `Resource::name()` of
  `PageHandle` are filled (the default page sort reads the weight and
  `Site().Current().Language()`; the resource sort and `NameNormalizedOrName` read the name).
- `Cargo.toml`/`Cargo.lock`: `go-url` (landed Wave A crate: `url.QueryUnescape` of expanded
  permalinks).
- T20 checklist entries implemented here, where their only caller is: `DeletePageAndResourcesBelow`
  (content_map_trees.rs L223-232, in `content_map_page.rs`) and `setMetaPostPrepareRebuild`
  (page__meta.rs L72-75, in `page__meta_post.rs`). `viewName.IsZero` is T20's
  (`content_map.rs`).

### Go behaviour reproduced on purpose

- Cache keys: `"gagesInSection/" + path + "/" + keyPart + "/" + recursive + "/" + includeSelf`
  and `path + "/" + keyPart`, both in `cachePages1`: a term's `.Pages` and `.RegularPages`
  share one key, the first query wins (the oracle queries terms in both orders).
- Terms: tree key = `PathParser.Base("/" + plural + "/" + value + "/_index.md")` (lower-case,
  spaces to `-`, not sanitized: `Lay's` and `Lays` are two terms; `a/b` is a nested term); the
  LAST value seen in walk order is `m.term` (title-cased into the default title in
  `applyAggregatesToTaxonomiesAndTerms`), `.Name` is the first; `types.ToStringSlicePreserveString`
  nil vs empty (an empty `[]string` still reads `<plural>_weight`, whose conversion error is
  logged); `""` values skipped with their ordinal kept; the walked key `""` becomes `"/"` and
  stays so for the rest of the page's taxonomies.
- A term that should not build is deleted from the page tree during the walk and its entries
  from EVERY language's entries tree (`TreeShiftTree.DeletePrefix`).
- `removeShouldNotBuild` disables (not deletes) home, sections and taxonomies; deletes the rest
  with their resources (this site's dimension only).
- Resources: ownership only checked for branch pages, so a page whose key prefixes a leaf
  bundle's (`/leafy` and `/leafy/b`) creates and lists that bundle's resources and visits its
  bundled page (whose `setMetaPost` then runs twice: the cascade compares equal, the dates are
  restored, `resourcePath` is the last walker's); with `duplicateResourceFiles` or multihost the
  walks are non-exact and another language's resources are cloned into the current dimension
  (else a page only creates the resources of its own language); the sort
  is Go's non-strict `less` through `go_sort::stable_by`.
- `setMetaPostParams`: the reserved-key switch (params written back normalised; `outputs`,
  `draft`, `sitemap`, `resources` left as they were), `[]any` of strings to `[]string`, empty
  `[]any` to `[]string{}`, `published`, `_build` (deprecation), `headless`, `params` merged
  last; `applyDefaultValues` titles (home = site title, section = CreateTitle(Pluralize(dir)),
  taxonomy with `-` to space, term, 404) only without a file; kinds disabled by config are
  disabled builds.
- Cascade: merge into the page's own cascade (own keys win), matchers on kind/lang/path
  (lower-cased, leading `/`)/environment, params and fields only where the page has no value;
  the site cascade enters at the home page; terms get theirs in step 2 from the taxonomy walk.
- Dates: branches with all-zero dates (and the home page, for `Site.lastmod`) take the max of
  the dates events of their descendants (publish date only if before `htime.Now()`); terms and
  taxonomies in step 2 from their entries; events are handled after each walk.
- `createTargetPathDescriptor` reads `h.Conf` (the FIRST site's config) for uglyURLs and
  multihost; sitemaps are always in a language subdir; expanded permalinks are
  `url.QueryUnescape`d (`""` on error) and a trailing `//` loses one slash.

### Deviations

1. The "dates" listeners of `applyAggregates`/`applyAggregatesToTaxonomiesAndTerms` cannot
   borrow the page arena (nh-doctree's handlers are `'static`): each records (listener,
   source) and the updates are applied in delivery order after the events are handled. Go's
   listeners only read the source's dates and write their own page's, so the result is equal.
2. Go picks the main section in map order (a random one of the sections with the most pages);
   the port takes the first in byte order. The oracle records ties as a set.
3. A page without output formats: Go's zero `pagePaths{}`; the port keeps the computed
   descriptor in `PagePaths` but leaves `PageCommon.target_path_descriptor` unset.
4. The cascade `PageMatcher.Matches` is applied to the values it reads (kind, lang, path,
   environment) and the permalink expander reads a `PathsPageView` (kind, date, title, slug,
   section, file, path info, the current section's `SectionsEntries`/`SectionsPath`):
   assembly has no `page.Page` handle (the `Arc<HugoSites>` does not exist yet).
5. `assembleResources`: the pages tree is not modified by that walk, so its visits are taken
   first and each page is handled in walk order (the resources tree walk is a `walk_mut`).
6. `setMetaPost` on a second run compares the cascades structurally instead of by
   `hashing.HashUint64` (equal hashes = equal values).
7. Assembly step 1 runs per site in site order (Go runs the sites in parallel; the sites only
   touch their own language dimension).
8. `.Data` is built on each call (Go: once per page); its values are the same.
9. `Site.Taxonomies()` after a `CreateSiteTaxonomies` error: an empty list (Go keeps the
   taxonomies built so far; the error is a missing taxonomy view, which cannot happen).
10. `h.newPage` errors in `addMissingTaxonomies`/`addStandalonePages` are returned (Go ignores
    them and inserts a nil page, which panics later).

### Verification (T21)

| topic | inputs | Rust test | checks |
|---|---|---|---|
| `assemble/*.json.gz` | process + assemble in Go (`go run -overlay`: `NHOracleAssemble` in package hugolib), 17 sites: docs/, hugolib/testsite, nh-page's synthetic site, the seeksnack reconstruction, T20's edge sites (shortcodes, edge-tree, contentdir, homeleaf, nokinds), T22's content site, and 7 assembly sites: `asm-taxo` (term case/slug/punctuation/nested/space collisions, numeric/bool/nil/nested-list values, weights and a bad weight, content terms, unused terms, permalinks for page/section/taxonomy/term with `:year :month :slug :title :sections :section :filename :contentbasename`, url patterns, uglyURLs per section, sitemap config, robots, resources metadata, the leafy double visit, a main-section tie), `asm-build`/`asm-flags` (drafts, `published`, future/expired pages, terms and sections, build list/render/publishResources, headless, `_build`, disableKinds, outputs per kind; with and without buildDrafts/buildFuture/buildExpired), `asm-cascade` (site and front matter cascades, `_target` kind/path/lang/environment, merges, fields, taxonomy cascades to terms, en/th), `asm-i18n` (en/th/fr, defaultContentLanguageInSubdir, translationKey with resources, language-specific resources, headless bundle, aliases, custom formats with path/baseName/permalinkable/isPlainText/noUgly/weight, `outputs` front matter), `asm-multihost`, `asm-ugly` (uglyURLs, disablePathToLower, no list-title capitalisation/pluralisation, permalinks `:slugorcontentbasename`) | `tests/assemble.rs` | 1,461 pages (meta, params with Go types, dates, cascade, build, sitemap, target path descriptor, output formats, 6,014 page outputs with slot sharing, target paths, permalinks, tree relations), 2,924 collections, 1,436 `.Data`, 41 `GetTerms`, 31,421 `GetPage` results (plain, old two-argument forms, with a context page, ref), 128 resources, 54 taxonomy keys, 2,046 cache keys, the log: 0 differences |
| lifecycle | `asm-i18n`, `asm-cascade` | `tests/lifecycle.rs` | outputs shared by name, one-slot standalone pages, rendering-site shift (keep/create/reuse `pco`, reset only a built paginator), non-rendering shift (lazy provider installed then reset, created for the current output, `pco` untouched, `pco` back on the next rendering shift) |

Mutations checked (each fails the tests): the `gagesInSection` key, the last-value `m.term`
update, the date aggregation of zero-date branches, the `[]any` → `[]string` conversion.

Regenerate (byte for byte; each site is built in a temp dir, never in the repository):

```sh
export GOTOOLCHAIN=go1.27.1
go run ./tools/go-oracle/nh-hugolib/assemble -root .
```

## Deliberate deviations

_Wave B (T21–T24): list every deviation from the Go code here (README rule 1). T20, T21 and
T22 list theirs in their sections above._

## Known gaps

_Wave B: list unported / stubbed functionality here._
