# nh-page — porting notes

neohugo resources/page/** (Page/Site traits, Pages ops, paths, permalinks, pagination, taxonomies), pagemeta, navigation, related.

Crate lead: T11 (page-api-paths). T12 (page-collections) owns the modules marked below.

## Go file → Rust module

| Rust module | Go source(s) | Owner | Status / note |
|---|---|---|---|
| `page` | `resources/page/page.go` (+ `ToPages` from `pages.go`) | T11 page-api-paths | ported; the `Page` trait, `PageRef`, `init()` |
| `site` | `resources/page/site.go` | T11 page-api-paths | ported; `SiteRef` is `siteWrapper`, `DummySite` is `testSite` |
| `page_nop` | `resources/page/page_nop.go` | T11 page-api-paths | ported (`NopPage`, `NopMarkup`, `NopContent`, `NopContentRenderer`) |
| `page_outputformat` | `resources/page/page_outputformat.go` | T11 page-api-paths | ported |
| `page_paths` | `resources/page/page_paths.go` | T11 page-api-paths | ported |
| `permalinks` | `resources/page/permalinks.go` | T11 page-api-paths | ported |
| `page_matcher` | `resources/page/page_matcher.go` | T11 page-api-paths | ported |
| `page_markup` | `resources/page/page_markup.go` (+ `tpl.StripHTML`, deviation 8) | T11 page-api-paths | ported |
| `page_data` | `resources/page/page_data.go` | T11 page-api-paths | ported |
| `page_kinds` | `resources/page/page_kinds.go` | T11 page-api-paths | ported (constants) |
| `page_author` | `resources/page/page_author.go` | T11 page-api-paths | ported (types) |
| `pagemeta::page_frontmatter` | `resources/page/pagemeta/page_frontmatter.go` | T11 page-api-paths | ported (content-adapter decoding: deviation 7) |
| `pagemeta::pagemeta` | `resources/page/pagemeta/pagemeta.go` | T11 page-api-paths | ported |
| `pages` | `resources/page/pages.go` | T12 page-collections | ported (see "T12: page collections" below) |
| `pages_sort` | `resources/page/pages_sort.go` | T12 page-collections | ported (see "T12: page collections" below) |
| `pages_sort_search` | `resources/page/pages_sort_search.go` | T12 page-collections | ported (see "T12: page collections" below) |
| `pages_cache` | `resources/page/pages_cache.go` | T12 page-collections | ported (see "T12: page collections" below) |
| `pages_language_merge` | `resources/page/pages_language_merge.go` | T12 page-collections | ported (see "T12: page collections" below) |
| `pages_prev_next` | `resources/page/pages_prev_next.go` | T12 page-collections | ported (see "T12: page collections" below) |
| `pages_related` | `resources/page/pages_related.go` | T12 page-collections | ported (see "T12: page collections" below) |
| `pagegroup` | `resources/page/pagegroup.go` | T12 page-collections | ported (see "T12: page collections" below) |
| `pagination` | `resources/page/pagination.go` | T12 page-collections | ported (see "T12: page collections" below) |
| `taxonomy` | `resources/page/taxonomy.go` | T12 page-collections | ported (see "T12: page collections" below) |
| `weighted` | `resources/page/weighted.go` | T12 page-collections | ported (see "T12: page collections" below) |
| `page_lazy_contentprovider` | `resources/page/page_lazy_contentprovider.go` | T12 page-collections | ported (see "T12: page collections" below) |
| `navigation::menu` | `navigation/menu.go` | T12 page-collections | ported (see "T12: page collections" below) |
| `navigation::menu_cache` | `navigation/menu_cache.go` | T12 page-collections | ported (see "T12: page collections" below) |
| `navigation::pagemenus` | `navigation/pagemenus.go` | T12 page-collections | ported (see "T12: page collections" below) |
| `related` | `related/inverted_index.go` | T12 page-collections | ported (see "T12: page collections" below) |

Every GO PORTING CHECKLIST entry of the T11 modules is `OK`; every ported function carries a
`// Go:` line.

## Dependencies

- nh-*: nh-common, nh-config, nh-media, nh-langs, nh-hugofs, nh-helpers, nh-markup, nh-resource,
  nh-tpl.
- Wave A: go-value, go-sort, go-time, go-url, go-fmt (`%q` in the permalinks errors), go-path,
  go-strconv, go-unicode, gotemplate (html/template's `stripTags`, deviation 8).
- crates.io: none. dev: `serde_json`, `flate2` (`rust_backend`, gunzip of the fixtures), `sha2`
  (SHA-256 of the long summary results in the fixture; README rule 2), nh-parser (go-toml local
  dates in the front matter fixtures).

## API notes for the other tasks

- **`init()`** (`nh_page::page::init`) registers `page.Page` and `resource.Resource` with
  `hreflect::register_interface` (and `page.Pages`/`resource.Resources` as named slices of them),
  and installs `resource_from_value` as nh-resource's value converter. nh-hugolib calls it once at
  startup (tests call it too); it is idempotent.
- **`Page` trait additions** (with defaults that panic, so the skeleton implementors compile):
  `current_section()`, `sections_entries()`, `sections_path()` (Go `CurrentSection()`,
  `SectionsEntries()`, `SectionsPath()`, read by the permalink expander). nh-hugolib's
  `PageHandle` must implement them.
- **Named types**: `page_outputformat::OUTPUT_FORMATS_METHODS` (`page.OutputFormats.Get`),
  `page_data::DATA_METHODS` (`page.Data.Pages`); `page.OutputFormats` elements are
  `page.OutputFormat` values, `Get` returns a `*page.OutputFormat` (`OutputFormatPtr`). `Format`
  is a named field of `page.OutputFormat`, so `output.Format` fields are `.Format.X`.
- **`page.Data`**: a lazy `.Data.Pages` is stored as a `LazyPages` object (Go's
  `func() page.Pages`).
- **`page_markup`** works on bytes: `extract_summary_from_html(mt, &[u8], i64, bool)` and
  `extract_summary_from_html_with_divider(mt, &[u8], &[u8])`; `HtmlSummary::{summary,
  content_without_summary, content}` return `Vec<u8>`.
- **`pagemeta`**: `FrontMatterDescriptor` has Go's fields (`base_filename`, `path_or_title`,
  `mod_time`, `git_author_date`, `page_config`, `location`; the params are
  `page_config.params`). `FrontMatterHandler::new_with_logger(Some(site logger), cfg)` is Go's
  `NewFrontmatterHandler(logger, cfg)`. `PageConfig.cascade` is an `Option` (Go's nil check in
  `Init`); `PageConfig` gained `dates_strings`, `content_adapter_data`, `cascade_compiled`,
  `is_from_content_adapter`. `BuildConfig::default()` is Go's zero `BuildConfig{}`;
  `BuildConfig::default_build_config()` is `DefaultBuildConfig` and
  `PageConfig::default_page_config()` is `DefaultPageConfig`.
- **`page_matcher`**: `decode_cascade_config_with_logger(logger, legacy, in)` (Go's logger
  argument; `decode_cascade_config` passes none), `decode_cascade`, `check_cascade_pattern`.
- **`permalinks`**: `PermalinkExpander::{new, expand, expand_pattern}`; `decode_permalinks_config`.
- **`page_nop`**: `nil_page_value()` is `page.NilPage` as a template value (`.GetPage` misses);
  `nop_page()`/`nil_page()` the objects.
- `pages_from_value` (Go `ToPages`) accepts `page.Pages`, `[]page.Page`, `[]any` of pages,
  `page.WeightedPages` (elements `WeightedPage`) and a `page.PageGroup` object (its `Pages` field).

## Go behaviour reproduced on purpose

- `CreateTargetPaths`: sanitize per element (`MakePathSanitized`), `#` → `%23`, `PathEscape`
  for plain links and `URLEscape` for front matter urls (a `url.Parse` error panics, like Go),
  `..` urls joined onto `/`, the language prefix only when the url does not already start with
  it, `baseNameSameAsType` forcing ugly, root formats dropping the prefix, 404/sitemap/robots
  compared by struct equality with the built-in formats.
- `PermalinkExpander`: `strings.Replace(.., 1)` on the partially expanded pattern (a replacement
  that itself contains a later token is replaced again), the `referenceTime` date-layout test for
  unknown attributes, `\:` escapes, the sections slice rules (out of range low → empty, `last`,
  negative → 0, `Atoi` failures → 0).
- `ExtractSummaryFromHTML`: Go's `for i, r := range s` with `utf8.RuneLen(r)` (3 for an invalid
  byte's U+FFFD) in the end-of-paragraph test, so the last word of a paragraph ending in an
  invalid byte is not counted; the last rune of a paragraph is never part of a word.
- `expandSummaryDivider`: `pOrDiv`'s first alternative is not anchored (it can match anywhere to
  the left and the scan jumps back by its length).
- `HandleDates`: a `time.Time` param is used as is only when its location is the page's
  location POINTER (`Arc::ptr_eq`), otherwise it goes through `ToTimeInDefaultLocationE` (RFC3339
  round trip: sub-seconds dropped, named zones become fixed offsets) and is written back to the
  params; every chain logs handler errors and moves on (the same unparsable key is logged once per
  chain).
- `DecodeBuildConfig` returns Go's normalised values for bool/int forms, unknown values and
  case-folded keys.

## Deliberate deviations

1. **Pooled path builder.** Go's `pagePathBuilder.Last()` indexes `els[-1]` when a pooled
   builder has an empty non-nil `els` (a panic that depends on the pool); the port answers `""`
   as for a fresh builder. No descriptor of the oracle reaches it.
2. **Map order.** Where Go iterates a map in random order the port uses byte order:
   `DecodePermalinksConfig` (which error is reported; a flat `key` entry and a
   `[permalinks.<kind>]` entry with the same key overwrite each other in random order),
   `NewPermalinkExpander` (first error; two section keys that trim to the same key, e.g. `""` and
   `"/"`, keep a random one), `mapToPageMatcherParamsConfig` (`_target` and `target` both set;
   `params` vs `Params`), the disallowed cascade key reported. The oracle avoids these inputs.
3. **Go panics as errors.** A reversed sections slice (`:sections[2:1]`, Go: `slice bounds out
   of range`), a cascade kind glob that does not compile (Go: nil pointer dereference) and a
   `.Data.pages` value that is not Pages (`try_data_pages`; `data_pages` panics) return Go's panic
   message as an error; `Source.ValueAsString` returns an error where Go panics.
4. **nil vs empty.** `Page::aliases()`/`keywords()` are `Vec`s, so `NamedPageMetaValue` returns an
   empty `[]string` where Go returns a nil one; `FrontmatterConfig` lists are `Vec`s (Go's nil list
   from an empty `[frontmatter]` entry is an empty one). `setParamIfNotSet` on a page config
   without params creates the map (Go would panic writing to a nil map; Hugo never passes one).
5. **HtmlSummary** stores a manual divider that was not found as the zero range (Go: `Low: -1`);
   both are `IsZero`, and nothing reads the field. Its source is bytes.
6. **NopPage** (`*page.nopPage`): `Hugo()` (a zero `neohugo.HugoInfo`, whose methods would
   dereference nil) is an explicit unsupported error; the trait's `site()` and `path_info()`
   panic like Go's nil results would when used.
7. **Content adapters.** `PageConfig.Compile` with `IsFromContentAdapter` (Go: `WeakDecode` of the
   adapter's map into the whole `PageConfig`) returns `neohugo-rs: content adapters
   (_content.gotmpl) are not supported`; seeksnack has none. The adapter date handler is ported.
8. **`tpl.StripHTML`** (CJK word counting) is a private copy (as in nh-markup) because nh-tpl's
   port (T13) has not landed; switch to `nh_tpl::template::strip_html` then.
9. **Output formats.** `page.OutputFormat` values are `page.OutputFormat` objects and `Get`
   returns `*page.OutputFormat` (`OutputFormatPtr`) (the skeleton used the pointer type name for
   both).
10. `PageMatcherParamsConfig`'s source structure (config printing only) is a map with the
    struct's field names.
11. `Page::current_section()` returns an `Option`; the permalink expander uses the page's own
    `sections_entries`/`sections_path` when it is `None` (Go's `CurrentSection()` is never nil).

## Known gaps

- The `Page` trait's new methods have panicking defaults until nh-hugolib implements them (T23).
- `NopContentRenderer` returns empty byte results instead of converter result objects (unused).

## Verification

The seeksnack site is private. The oracles build this repository's `docs/` site (the English
content, `docs/hugo.toml`; the remote-data content adapter and the build-stats writer are left
out and link/image render hooks and shortcodes are stubbed, since only the content model matters)
and `hugolib/testsite` (en + nn), plus 7 synthetic sites sharing one content tree
(`tools/go-oracle/nh-page/psupport/sites.go`): a seeksnack-like site (CM §9.4 examples,
`[permalinks] posts = ...`), `defaultContentLanguageInSubdir` with `timeZone`, global and
per-section `uglyURLs`, multihost (en/th with their own baseURLs and time zones),
`disablePathToLower`, a baseURL with a path; custom output formats with `path`, `baseName`,
`isPlainText`, `noUgly`, `ugly`, `root`, `permalinkable`, `protocol`; every built-in format; url
and slug front matter with and without trailing slashes, `..`, Unicode, `#`, spaces and `:`
patterns; permalinks with every token; front matter dates in YAML, TOML and JSON and every
string layout cast accepts, filename dates and bad dates. Every build is the real neohugo build,
in process, in a MemMapFs with deterministic mod times.

The `paths`, `frontmatter` and `summary` oracles run themselves again with `go run -overlay`
(`psupport.RunOverlaid`): the overlay adds a recording hook to `CreateTargetPaths`,
`HandleDates` and the two summary extractors, generated from the current Go sources on every
run (a missing anchor is an error), so every call of the real builds is captured.

| topic | inputs | Rust test | checks |
|---|---|---|---|
| `paths/<site>.json.gz` | every `TargetPathDescriptor` the 9 builds created (page outputs, paginator pages and page-1 aliases, pagination URLs, 404/sitemap/robots), each `*paths.Path` re-parsed from its input with the site's recorded PathParser, the site's PathSpec config; adversarial variants of one recorded descriptor per (kind, bundle, url, prefix) class: every output format of the site, uglyURLs on/off, 34 urls, 12 base names/slugs, 6 addends, 7 expanded permalinks, 6 prefix/force-prefix combinations | `tests/paths.rs` | 80,299 descriptors (6,439 recorded from the builds), `TargetFilename`, `SubResourceBaseTarget`, `SubResourceBaseLink`, `Link`, `RelPermalink`, `PermalinkForOutputFormat` (272 Go panics matched), plus the CM §9.4 table asserted literally |
| `permalinks/<site>.json.gz`, `decode.json.gz` | the 1,564 pages of the builds (kind, date, title, slug, section, file, current section entries/path); `Expand` with the site's config and an every-token config; `ExpandPattern` with 58 patterns (every token, sections slices valid/out of range/malformed, Go layouts, escapes, unknown attributes); `DecodePermalinksConfig` of the sites' sections, `config-en.json` and adversarial maps | `tests/permalinks.rs` | 89,148 expansions, 21 decodes |
| `frontmatter/<site>.json.gz`, `decode.json.gz` | every `HandleDates` call of the builds (params before, descriptor, result); replays over 8 `[frontmatter]` variants (default, docs', `:default`, `:filename`, `:fileModTime`, `:git` with a synthetic author date, custom keys, a string for a list) × 3 time zones × with/without git date, with the logged errors; `DecodeFrontMatterConfig` and `DecodeBuildConfig` of fixed inputs | `tests/frontmatter.rs` | 1,830 calls + 16,568 replays; 29 decodes |
| `summary/build.json.gz`, `adversarial.json.gz` | the rendered HTML of every page (749 distinct `ExtractSummaryFromHTML`/`...WithDivider` calls of the builds: auto, manual with Hugo's divider, CJK) and variants (word counts, AsciiDoc/RST/HTML media types, other dividers); 6,000 adversarial calls (paragraph/div/RST wrappers, dividers everywhere, invalid UTF-8, whitespace, CJK) | `tests/summary.rs` | 18,733 (results as values or SHA-256 of long strings, all ranges); Go runtime panics (a divider before the RST wrapper) match as panics |
| `misc/misc.json.gz` | `DecodeCascadeConfig` (18 inputs incl. merging, errors, the logged pattern warning, the source hash), `PageMatcher.Matches` (16 matchers × 9 pages), `NewOutputFormat`, `OutputFormats.Get`, `MarkupToMediaType`, `PageConfig.Init`/`Compile`, `NamedPageMetaValue` on the nop page | `tests/misc.rs` | 275 |

Results: 0 differences outside the deviations above. `tests/misc.rs` also checks `page.Data`
and the nop page's template table. Mutations of the `%23` escape, the location pointer test in
`newDateFieldHandler` and the `RuneLen` end-of-paragraph test are each detected.

Regenerate (platform independent; time zones from the system; each reproduces byte for byte):

```sh
export GOTOOLCHAIN=go1.27.1
go run ./tools/go-oracle/nh-page/paths
go run ./tools/go-oracle/nh-page/permalinks
go run ./tools/go-oracle/nh-page/frontmatter
go run ./tools/go-oracle/nh-page/summary
go run ./tools/go-oracle/nh-page/misc
```

## T12: page collections

Every GO PORTING CHECKLIST entry of the T12 modules is `OK` except the Go test helper
`Pages.shuffle` (math/rand; not ported). Every ported function carries a `// Go:` line.

### API notes for the other tasks

- **Additions to the `Page` trait** (T11's `page.rs`; defaults keep every implementor compiling):
  `related_docs_handler()` (Go `GetInternalRelatedDocsHandler`; `None` makes `.Related` fail with
  `invalid type`), `fragments_identifiers(ctx)` (Go `Fragments(ctx).Identifiers`; `None` = not a
  `FragmentProvider`), `apply_filter_to_headings(ctx, filter)` (returns the
  `*hugolib.pageHeadingsFiltered` wrapper), `is_ancestor(&Value)` (default: through
  `tpl_call_method("IsAncestor")`). nh-hugolib's `PageHandle` must implement the first three.
- **Named-type method tables** to register (T23 `tplapi::named_types`): `pages::PAGES_METHODS`
  (`page.Pages`), `pagegroup::PAGES_GROUP_METHODS` (`page.PagesGroup`),
  `taxonomy::TAXONOMY_METHODS` (`page.Taxonomy`), `taxonomy::TAXONOMY_LIST_METHODS`
  (`page.TaxonomyList`), `taxonomy::ORDERED_TAXONOMY_METHODS` (`page.OrderedTaxonomy`),
  `weighted::WEIGHTED_PAGES_METHODS` (`page.WeightedPages`), `navigation::menu::MENU_METHODS`
  (`navigation.Menu`). Objects: `pagegroup::PageGroup` (`page.PageGroup`: fields `Key`, `Pages`,
  the promoted `page.Pages` methods, `ProbablyEq`, `Slice`), `pagination::PagerRef`
  (`*page.Pager`), `weighted::WeightedPage` (`page.WeightedPage`),
  `taxonomy::OrderedTaxonomyEntry`, `navigation::menu::MenuEntryRef` (`*navigation.MenuEntry`),
  `MenuConfigValue`. Values: `taxonomy_list_to_value`, `taxonomy_to_value`, `menus_to_value`,
  `page_menus_to_value`, `menu_to_value`, `pages_group_to_value`, `pagers_to_value`.
- **Pages methods** that Go declares with a `context.Context` (`ByLength`, `GroupBy`, `Related`,
  `RelatedIndices`, `RelatedTo`) take the `HostCtx`; `group_by_ctx`, `related_opt`,
  `related_indices`, `related_to` return `Option` for Go's nil results (the template value of a
  nil `PagesGroup`/`Pages` is a typed nil).
- **`pages_cache::clear()`** (= `page::clear()`) must be called at the start of every
  `Site.render` (T24).
- **Pagination**: `paginate(td, seq, size)` (Go `Paginate`), `paginate_with(url_factory, seq,
  size)`, `resolve_pager_size(conf, options)`, `new_pagination_url_factory(td)`; the "first call
  wins" `.Paginator`/`.Paginate` state is T23's. `PaginatorNotSupportedFunc` for page kinds
  without pagination.
- **Related**: `related::Document` gained `fragments`, `apply_filter_to_headings`, `as_any`;
  `pages_related::PageDocument` wraps a page; `InvertedIndex::{add_ctx, search_ctx}` take the
  context. `related::decode_config` takes a `maps.Params` value (`Value::Invalid` = nil).
- **Menus**: `navigation::menu::decode_config_namespace` returns the whole `ConfigNamespace`
  (nh-allconfig's `menus` field); `decode_config` only the menus. Menus are immutable
  `Arc<MenuEntry>` trees: hugolib (T23 `assembleMenus`) builds the entries with their children and
  page values (`set_page_values(&mut MenuEntry, page)`) before sharing them.
  `pagemenus::page_menus_from_page_for(ms, page)` is Go's `PageMenusFromPage` (the skeleton's
  page-less `page_menus_from_page` only handles a nil value); `pagemenus::PageMenusQuery`
  (`new_menu_query_provider(page menus, site menus, page)`) answers `IsMenuCurrent` /
  `HasMenuCurrent`.
- **`LazyContentProvider`**: `init_do()` (Go `lcp.init.Do`), `current()` (the provider, or `None`
  for the nop provider), `plain`, `content`, `reset`, `provider()`.

### Go behaviour reproduced on purpose

- **Stable sorts** (`sort.Stable`, `sort.SliceStable`) through `go_sort::stable_by`, unstable ones
  (`sort.Sort` in `sortKeys`) through `go_sort::sort_by`, so comparators that are not strict weak
  orders give Go's order: `ByParam` compares strings with `CompareStrings < 1` (i.e. `<=`) and
  sorts nil params last with `v1 == nil → false`; `sortKeys` sorts string keys with the same `<=`.
- **Collators**: the current site's language (`p.Site().Current()`); `collator1` locked for the
  whole `ByTitle`/`ByLinkTitle`/`Alphabetical` sort and per comparison in `DefaultPageSort`,
  `collator2` for `ByParam` and `sortKeys`. `lessPageLanguage` uses `compare.Strings`.
- **The global sorted-pages cache** (`spc`): keyed by name and the element identity of the input
  lists; a hit returns the list as sorted at first computation even when the current site (and so
  the collator) changed since; `clear()` resets it. `GroupByDate` & co. reuse `ByDate`/`Reverse`
  through it, `MergeByLanguage` too. The menu cache (`smc`) likewise (never cleared, as in Go).
- **GroupBy**: the `page.Page` method set as reflect sees it (`GROUP_BY_METHODS`, checked against
  the Go interface by the oracle). A result type that cannot be a map key panics in Go
  (`reflect.MapOf: invalid key type …`), methods needing arguments panic (`reflect: Call with too
  few input arguments`, `not supported`), an unknown key panics (`reflect: Elem of invalid type
  page.Page`): the port returns those messages as errors. The key's static kind decides the sort
  (an interface result such as `Content` is not sorted even when it holds a string).
- **GroupByParam**: the key type is the dynamic type of the first non-nil, non-`[]string`
  (lower-cased) param; pages with another type are skipped; map-typed keys panic in Go (error).
- **groupByDateField**: sorted descending unless the order is `asc`, `rev` or `reverse` (Go's
  inverted `rev`), consecutive equal formatted dates, the current site's time formatter.
- **Pagination**: a nil sequence paginates as empty GROUPS (`ToPagesGroup(nil)` is ok), one pager
  when there are no elements (`TotalPages` is 0 then), `splitPageGroups` starts a new group for
  every page of a nil key, `ResolvePagerSize` through `cast.ToIntE`.
- **Related**: `Add` returns the error of the last `RelatedKeywords` call (so a page whose last
  index fails aborts `getOrCreateIndex`), duplicate keywords count twice, `norm`'s
  `x/y*100 + 0.5` is an FMA on arm64 (one `FMADDD` in the arm64 build of `searchDate`;
  `f64::mul_add` here; unit test `related::tests::norm_fma`), the cardinality percentage is
  `ceil(len/numDocs*100)` (no fusion there: FMUL + FRINTP), the integer divisions of `avgWeight`
  and `threshold` truncate toward zero, the fragments filter is the sorted filter list.
- **Menus**: `Menu.Add` appends and stable-sorts (weight with 0 last, `compare.Strings(Name)`,
  `Identifier`), a front matter list of menu names shares ONE entry whose `Menu` is the last name
  (Go's `&me` in the loop), `cast.ToStringE` accepts a number as a menu name.

### Deliberate deviations

1. **Go map order.** Where Go iterates a map in random order the port uses first-appearance or
   key order: GroupBy/GroupByParam keys of kinds other than int and string (bool, `time.Time`,
   pages, …) keep first-appearance order; `Taxonomy.TaxonomyArray` (and so the order of
   collator-equal names in `Alphabetical`) and `Taxonomy.Page` use key order; related candidates
   are collected in first-match order, so results with the same weight, publish date and name
   keep that order (Go: random); `DecodeConfig` of related prints `[basic fragments]`; menus
   decode in key order (only which error is reported can differ). The oracles mark such results
   (`unordered`, `ties`) and the tests compare them as sets.
2. **Go panics as errors.** GroupBy's reflect panics (above), `WeightedPages.Page()` of an empty
   list, `Limit` with a negative count and `Taxonomy.Page` with a nil owner return the panic
   message as an error; `getOneOPage` of a taxonomy whose first entry has no pages returns no page
   (Go panics; hugolib never builds one); an `Add` to a finalized index panics like Go.
3. **nil vs empty.** Rust lists have no nil: `pagesEqual` treats a nil and an empty list alike
   (Go: not equal), `HasChildren` is "has at least one child" (hugolib only sets children by
   adding one). Go's nil results are `Option`s in the typed API and typed nils in template values.
4. **Identity.** Go compares page interface values (`pagesEqual`, map keys): the port compares
   `Arc` pointers, else page id + wrapper type + weight0 + ordinal; menu entries and pagers by
   `Arc` pointer. A `Pager` holds its `Paginator` and the paginator its pagers: the `Arc` cycle
   keeps a paginator alive until the process ends.
5. **Menus source structure.** Go's `PrepareParams` of an entry's params also lower-cases the keys
   of nested maps it shares with the decoded source structure (aliasing); the port's source
   structure keeps them. It is only printed by `hugo config`; the source hash is computed before
   and matches.
6. **Locks** (HUGO_LAYER.md §4.8): the sorted-pages cache, the menu cache, the related indices and
   the lazy content provider compute outside their locks and keep the first stored value (Go
   computes under the lock); with sequential rendering the results are the same.
7. `PageSize()` (deprecated) does not log Go's deprecation notice. `Pager.Paginator` (the
   embedded `*Paginator` field) is a `*page.Paginator` object (`PaginatorRef`) with its methods.

### Verification

The oracles build the psupport sites (docs/, hugolib/testsite, the seeksnack-like and multihost
synthetic sites) and two sites made to stress ties (`csupport.TiesSite`, `TiesThSite`: 40 items
with equal/zero/negative weights, equal and missing dates, equal titles and link titles, Thai
titles including ฯลฯ and ํา that order differently under the en and th collators, rating/mixed
params that are ints, floats, strings, bools, dates or missing, weighted tags, translations, a
Thai-first language order, pagination paths `seite` and `หน้า Ü`, a related config with a
cardinality threshold, a date pattern and an `applyFilter` fragments index). A `go run -overlay`
hook adds `hugolib.OracleSetCurrentSite`, so every case runs with each site as the current one.
The Rust tests rebuild the recorded page values as fake pages (`tests/csupport`): kind, titles,
weights, dates, params, paths, files, languages, wrappers (weight0/ordinal), fragments and
headings, parents, ancestors.

| topic | inputs | Rust test | checks |
|---|---|---|---|
| `collections/<site>.json.gz` | every site's collections, every node's Pages/RegularPages, GetTerms lists, shuffles, cross-language lists (436 lists) × every site as the current site | `tests/collections.rs` | 99,881: every ByX, Reverse, ByParam (10 keys), Limit, SortByDefault, SortByLanguage; GroupBy (22 keys incl. errors) in 3 orders, GroupByParam (10 keys), GroupByDate (5 formats), GroupByPublishDate/ExpiryDate/Lastmod/ParamDate in 5 orders; Next/Prev; MergeByLanguage; 98 cache sequences (20 where the collators give different orders); taxonomies (ByCount, Alphabetical per current site, Get/Count, Page, WeightedPages Sort/Next/Prev) |
| `collections/methods.json.gz` | the `page.Page` interface through reflect | `tests/collections.rs` | the 101-method GroupBy table |
| `pagination/<site>.json.gz` | every `page.Paginate` call of the builds (the paginator of every list page, 251 calls), replayed with pager sizes 1, 3, 10 and 100 and with grouped pages (GroupByDate, GroupBy Section, GroupByParam, a nil key), bad sizes and sequences; descriptors rebuilt with the T11 path support | `tests/pagination.rs` | 1,540 paginators, every pager (URL, pages, groups, navigation, sizes) |
| `pagination/resolve.json.gz` | `ResolvePagerSize` options | `tests/pagination.rs` | 15 |
| `related/<site>.json.gz` | `Pages.Related` for every regular page of every site with the site's config, option maps, bad arguments; direct indices with the default and 5 custom configs (includeNewer, toLower, thresholds 0–100, cardinality thresholds, date patterns, negative/zero weights, fragments with and without applyFilter, param indices of mixed types): document searches for every page, index subsets, fragments, named slices | `tests/related.rs` | 13,828 (with filtered headings; tie runs as sets) |
| `related/decode.json.gz` | `DecodeConfig` of the custom configs and 10 invalid ones | `tests/related.rs` | 15 |
| `menus/<site>.json.gz` | the hugolib/menu_test.go sites, docs/ and a nested multilingual site: DecodeConfig of each language's menus, the assembled menu entries (URL, KeyName, HasChildren), PageMenusFromPage of every front matter `menu`/`menus` value, IsMenuCurrent/HasMenuCurrent of every page × entry, Sort/ByWeight/ByName/Reverse/Limit of every menu and of shuffles | `tests/menus.rs` | 413 cases, 1,209 IsMenuCurrent/HasMenuCurrent pairs |
| `menus/decode.json.gz` | adversarial `DecodeConfig` inputs | `tests/menus.rs` | 9 |

`tests/go_tables.rs` ports pages_sort_test.go, pagination_test.go, pagegroup_test.go,
pages_prev_next_test.go, pages_sort_search_test.go (plus 1,500-page lists for the binary search
path), pages_cache_test.go, pages_test.go, pages_related_test.go and navigation/menu_cache_test.go
over a port of Go's `testPage`; the `src/related.rs` unit tests port inverted_index_test.go;
`src/page_lazy_contentprovider.rs` tests the reset semantics; `tests/tpl_methods.rs` covers the
template method tables. Mutations that the tests detect: disabling the sorted-pages cache, `< 0`
instead of `< 1` in `ByParam`, Rust's sort for `pageBy.Sort`, no integer `threshold/matches`
division, `<` instead of `<=` in the cardinality test.

Results: 0 differences outside the deviations above. The fixtures come from linux/amd64 Go; no
recorded case reaches `norm`'s FMA rounding difference (it needs an average weight below the
index minimum), which the unit test pins to arm64. `ResolvePagerSize` is not recorded with
out-of-range floats (`int(f)` differs between amd64 and arm64).

Regenerate (each reproduces byte for byte):

```sh
export GOTOOLCHAIN=go1.27.1
go run ./tools/go-oracle/nh-page/collections
go run ./tools/go-oracle/nh-page/pagination
go run ./tools/go-oracle/nh-page/related
go run ./tools/go-oracle/nh-page/menus
```
