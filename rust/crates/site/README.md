# neohugo-site

Capture and assembly into the `Model` (REWRITE_PLAN.md §2.4, phases A4–B2; T23a). The next
phases (auto nodes, URLs, relations, taxonomies, translations, `get_page`/`ref`) are T23b's and
build on the seams listed below.

| Piece | API |
|---|---|
| Entry point | `load_model(Arc<Config>, &Vfs, &LoadModelOptions { clock, content: ContentFilter }) -> Result<Model, ModelError>`; `LoadModelOptions::from_config(&cfg, clock)` |
| Model | `Model { config, pages: IdVec<PageId, Page>, sites: IdVec<LangIdx, SiteModel>, bundle_resources: IdVec<ResourceId, BundleResource>, data: Arc<Map>, diagnostics }`; `page(id)`, `bundle_owner(id)` |
| Page | `Page { id, lang, kind, role: PageRole, key: ContentKey, source: Option<SourceFile>, meta: PageMeta }`; `PageRole::{Standalone, Bundled { bundle: ContentKey }}` |
| Source | `SourceFile { file: FileRef, file_info, info (after front matter path), front_matter, text: Arc<str>, body_offset, mod_time }`, `body()`, `base_filename()` |
| Trees | `SiteModel { lang, tree: SiteTree, resources: BTreeMap<ContentKey, ResourceId>, cascade: CascadeIndex }`; `SiteTree::{get, insert, remove, longest_prefix, descendants, iter}` (segment-wise) |
| Cascade | `CascadeIndex::{new, add_branch, received(key), in_force(key, own), branches}`; `meta::cascaded_params(cfg, index, lang, kind, key)` for pages without a file |
| Data | `data::load(&Vfs) -> Result<Data { map, diagnostics }, DataError>` |

## Phases

1. **Capture** (A4; rayon over files): `Vfs::discover_content` (its duplicates become
   `duplicate-content-path` warnings), read, `split_front_matter`, `decode_front_matter`,
   `capture_overrides`, the page's own `cascade`. Front matter `lang` moves a page only to an
   enabled language (`lang: TH` → `th`; a disabled `fr` is ignored and stays in the params);
   `kind` is normalised; `path` re-parses the page's path (`/custom/moved` →
   `/custom/moved/index.md`, `_index` for branch kinds). `data::load` runs alongside.
2. **Tree** (B1): kind from the override, else home (empty key, also for a root `index.md`,
   with the `warning-home-page-is-leaf-bundle` warning), taxonomy (`/<plural>`), term (below
   it, segment-wise), section (other branch bundles) or page; pages of a disabled `page` kind
   are dropped. A key claimed twice after the overrides keeps the first file and warns.
   Content files inside leaf bundles are `PageRole::Bundled` pages registered as bundle
   resources of their language; other bundle files are `BundleResource`s of their language.
3. **Cascade → meta → dates** (B2; the cascade index is built per language in tree order, the
   pages run in parallel): the site `[[cascade]]` at the root, each branch page's own entries
   merged over what it receives; `Cascade::apply`, then `meta_from_params` with the
   language's date sources, time zone and sitemap defaults; `hasCJKLanguage` detected on the
   body; `_build` and `published: <bool>` read as Hugo does. Unparsable front matter dates are
   `front-matter-date` errors (Hugo logs them as errors).
4. **Filter** (B2): drafts, `publishDate` after the clock and `expiryDate` before it, and
   disabled kinds. Home, section and taxonomy pages stay for the structure with their build
   policy switched off; other pages are removed with the bundle files below them in their
   language (as Hugo's `DeletePageAndResourcesBelow`; pages below a removed page stay).

Page ids follow the capture order (key, then language), so the model is the same for any
number of threads.

## Seams for T23b

- Auto nodes (missing home, root sections, taxonomies, terms, standalone pages): create a
  `Page` with `source: None`, params from `meta::cascaded_params`, `meta_from_params`, insert
  into `SiteModel::tree`. Page ids are appended after the content pages.
- `Page` gains the URL and relation fields of §2.4 (title, section, type, formats, urls,
  links, parent, ancestors, lists, translations, terms, resources).
- `BundleResource` gains `target_base`, `meta` and `publish`; owners come from
  `SiteTree::longest_prefix` over the language's tree (plus the fallback language), bundled
  pages' from `Model::bundle_owner`.
- Node dates (a branch without dates takes its descendants') and `.Site.Lastmod`.

## Acceptance (tests/it)

| Test | Fixtures | Result |
|---|---|---|
| `capture` | `oracle/hugolib/capture/*` (9 sites; the 5 `sc-err-*` are T34's shortcode errors) | per language tree pages (key, file, kind) and resource trees equal: 1,121 pages, 96 resources; 1,143 page configs (override params `kind`/`lang`/`path`, own cascade) equal; duplicate warnings equal |
| `assemble` | `oracle/hugolib/assemble/*` (17 sites) | every page with a file equal: page set per language, kind, bundle role, params after cascade, build options, draft, own dates, typed fields (16 per page), cascade in force: 1,248 pages, 19,776 fields; duplicate, date and home-leaf diagnostics counted |
| `data` | `oracle/hugolib/data/*` (15 cases) | trees equal (numbers by value), warnings and rejected files counted, the 4 load errors fail |
| `model` | R (the seeksnack reconstruction content), `docs/data`, `seeksnack.txtar` | FM overrides, drafts/future/expired, bundle roles, cascade, keys with `/` (R), nested JSON, case preserved (D), determinism |
| `real_sites` (ignored) | `sites.py make docs\|testsite\|seeksnack` | `NEOHUGO_SITES=<dir>:… cargo test -p neohugo-site real_sites -- --ignored --nocapture` |

## Deviations from Hugo (`expected_diffs.toml`)

- **Taxonomy prefix is segment-wise**: `content/tagsfoo/_index.md` is a section, not a term of
  `tags` (Hugo's go-radix matches characters).
- **YAML 1.2** data (`017` is decimal) and **TOML local times** as strings (neohugo-base).
- **CSV data files** are lists of rows; Hugo rejects them (a Go type detail).
- Data **messages** are ours; a scalar data file is an error diagnostic, as in Hugo, but the
  load goes on (Hugo logs it as an error too).
