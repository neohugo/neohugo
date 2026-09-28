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
- Error texts: Go wraps a file error as `readAndProcessContent: "file:l:c": msg`; nh-common's
  `Error` prints the position first (`"file:l:c": readAndProcessContent: msg`). The test maps
  one to the other; the texts are otherwise identical.

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

## Deliberate deviations

_Wave B (T21–T24): list every deviation from the Go code here (README rule 1)._

## Known gaps

_Wave B: list unported / stubbed functionality here._
