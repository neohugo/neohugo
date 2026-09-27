# nh-page — porting notes

neohugo resources/page/** (Page/Site traits, Pages ops, paths, permalinks, pagination, taxonomies), pagemeta, navigation, related.

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `page` | `resources/page/page.go` | T11 page-api-paths |  |
| `site` | `resources/page/site.go` | T11 page-api-paths |  |
| `page_nop` | `resources/page/page_nop.go` | T11 page-api-paths |  |
| `page_outputformat` | `resources/page/page_outputformat.go` | T11 page-api-paths |  |
| `page_paths` | `resources/page/page_paths.go` | T11 page-api-paths |  |
| `permalinks` | `resources/page/permalinks.go` | T11 page-api-paths |  |
| `page_matcher` | `resources/page/page_matcher.go` | T11 page-api-paths |  |
| `page_markup` | `resources/page/page_markup.go` | T11 page-api-paths |  |
| `page_data` | `resources/page/page_data.go` | T11 page-api-paths |  |
| `page_kinds` | `resources/page/page_kinds.go` | T11 page-api-paths |  |
| `page_author` | `resources/page/page_author.go` | T11 page-api-paths |  |
| `pagemeta::page_frontmatter` | `resources/page/pagemeta/page_frontmatter.go` | T11 page-api-paths |  |
| `pagemeta::pagemeta` | `resources/page/pagemeta/pagemeta.go` | T11 page-api-paths |  |
| `pages` | `resources/page/pages.go` | T12 page-collections |  |
| `pages_sort` | `resources/page/pages_sort.go` | T12 page-collections |  |
| `pages_sort_search` | `resources/page/pages_sort_search.go` | T12 page-collections |  |
| `pages_cache` | `resources/page/pages_cache.go` | T12 page-collections |  |
| `pages_language_merge` | `resources/page/pages_language_merge.go` | T12 page-collections |  |
| `pages_prev_next` | `resources/page/pages_prev_next.go` | T12 page-collections |  |
| `pages_related` | `resources/page/pages_related.go` | T12 page-collections |  |
| `pagegroup` | `resources/page/pagegroup.go` | T12 page-collections |  |
| `pagination` | `resources/page/pagination.go` | T12 page-collections |  |
| `taxonomy` | `resources/page/taxonomy.go` | T12 page-collections |  |
| `weighted` | `resources/page/weighted.go` | T12 page-collections |  |
| `page_lazy_contentprovider` | `resources/page/page_lazy_contentprovider.go` | T12 page-collections |  |
| `navigation::menu` | `navigation/menu.go` | T12 page-collections |  |
| `navigation::menu_cache` | `navigation/menu_cache.go` | T12 page-collections |  |
| `navigation::pagemenus` | `navigation/pagemenus.go` | T12 page-collections |  |
| `related` | `related/inverted_index.go` | T12 page-collections |  |

## Dependencies

- nh-*: nh-common, nh-config, nh-media, nh-langs, nh-hugofs, nh-helpers, nh-markup, nh-resource, nh-tpl
- Wave A (to add when available): go-sort, xtext-collate (via nh-langs), go-time, go-url
- crates.io (justify each): none

## Deliberate deviations

_Wave B: list every deviation from the Go code here (README rule 1)._

## Known gaps

_Wave B: list unported / stubbed functionality here._
