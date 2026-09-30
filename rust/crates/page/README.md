# neohugo-page

Per-page rules (REWRITE_PLAN.md §2.4, phases B2 and B4): typed front matter, dates, build
policy, cascade, target paths and permalinks, the default sort order and default titles. The
crate holds no page store; `neohugo-site` calls it while it assembles the model.

| Piece | API |
|---|---|
| Capture overrides | `capture_overrides(&Params) -> CaptureOverrides { kind, lang, path: Option<ContentKey> }` |
| Cascade | `Cascade::decode(&Value)`, `Cascade::from_config(&[CascadeConfig])`, `Cascade::inherit(parent, own)`, `Cascade::apply(&MatchCtx, &mut Params)`; `CascadeTarget::new(kind, path, lang, env)` (Hugo globs via `base::glob`), `matches`, `path_looks_like_file` |
| Front matter | `meta_from_params(Params, &MetaCtx) -> PageMeta` (reserved keys typed; normalised values written back to params; `params:` merged last; dates resolved) |
| Dates | `DateResolver::{new, from_site, resolve(&mut Params, Option<&FileCtx>, &TimeZone)} -> DateOutcome { dates, slug, unparsable }` |
| Build | `BuildPolicy { list: ListMode, render: RenderMode, publish_resources }`, `BuildPolicy::decode`, `.headless()` |
| Markup | `Markup::{Markdown, Html}`, `Markup::detect(MarkupSource { media_type, markup, ext }, &MediaTypes)` |
| Permalinks | `PermalinkPattern::{parse, expand(&PermalinkCtx)}`; `PermalinkPatterns::{compile(&config::Permalinks), get(kind, section)}`; Go layouts (`:2006`, `:Jan`, `:02`, `:MST`) become `strftime` at parse |
| Target paths | `target_paths(&UrlInputs) -> TargetPaths { target: OutputPath, link: UrlPath, resources: Option<ResourceBase> }`; `links(&TargetPaths, &SiteUrls, &OutputFormat) -> Links { rel_permalink: UrlPath, permalink: Permalink }`; `SourcePath::{from_path_info, from_key}` |
| Order | `default_order(&SortKey, &SortKey, &dyn Collate)` (ordinal, taxonomy weight, weight with 0 last, date desc to the second, collated link title, source path) |
| Titles | `default_title(kind, raw, &TitleConfig)` |

## Acceptance (tests/it)

The oracle tests replay `rust/testdata/oracle/page/{paths,permalinks,frontmatter,misc,collections}`
(the per-page families; `menus`, `pagination` and `related` are `neohugo-nav`'s, `summary` is
`neohugo-markup`'s) and print pass rates. Every difference is either exact or a reviewed class
of `expected_diffs.toml` with its exact count:

| Family | Checks | Exact | Accepted |
|---|---|---|---|
| paths (target file, link, resource dirs, rel/permalink) | 80,299 | 99.66 % | 272 `url-query-or-escape` |
| permalinks (site config, every-token config, 55 patterns) | 89,148 | 100 % | – |
| frontmatter dates (recorded calls and 16,568 replays) | 18,398 | 99.62 % | 70 `unix-seconds-in-utc` |
| build options | 15 | 100 % | – |
| cascade decode / match | 18 / 144 | 100 % / 87.5 % | 18 `bad-glob-rejected` |
| markup detection / page config | 26 / 21 | 57.7 % / 71.4 % | 12 `markup-not-supported`, 5 `content-adapter` |
| default sort (`SortByDefault`, every current site) | 617 | 94.65 % | 33 `thai-paiyannoi-collation` |
| source paths (`SourcePath::from_path_info` over the vfs parser) | 1,798 | 100 % | – |

Overall 190,484 checks, 99.78 % exact, no unexplained difference.

## Deviations from Hugo (by design)

- **Attribute expansion.** Hugo replaces permalink attributes one by one with
  `strings.Replace(…, 1)` in the partially expanded pattern; here each attribute is expanded in
  place. They differ only when a title or slug contains text such as `:slug` (no fixture case).
- **Unix-second dates** are UTC, not the process's local zone.
- **Cascade globs that do not compile** are errors (Go ignores or panics).
- **Only Markdown and HTML** content; content adapters are not supported.
- **Page links are URL paths.** `TargetPaths::link` is unescaped and escaped once when written;
  a front matter `url` with a query or `%` escapes is escaped differently from Hugo.
- **Resource output directories are clean paths** (`/th/section`, not `/th/section/`).
- **Standalone formats** (404, sitemap, robots.txt; no resource directory) are recognised by
  name, not by struct equality with the built-in formats.
- **Menus in front matter**: a list of menu names gives one entry per menu (Go shares one entry
  whose menu is the last name).
- **Date errors are data** (`DateOutcome::unparsable`), reported once per key by the caller.
