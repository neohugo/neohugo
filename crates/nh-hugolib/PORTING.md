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
- Wave A (to add when available): go-json, go-time, go-fmt
- crates.io (justify each): none

## Deliberate deviations

_Wave B: list every deviation from the Go code here (README rule 1)._

## Known gaps

_Wave B: list unported / stubbed functionality here._
