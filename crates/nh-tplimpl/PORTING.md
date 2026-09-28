# nh-tplimpl — porting notes

neohugo tpl/tplimpl/** (template store, layout lookup, baseof, AST transforms, exec helper) + embedded templates; engine.rs is the single adaptation point to the Wave A gotemplate crate.

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `engine` | — | T13 tplimpl | NEW: facade over the gotemplate crate (text/html template namespaces, parse trees, exec, escaper). The only module that imports `gotemplate`. |
| `templatestore` | `tpl/tplimpl/templatestore.go` | T13 tplimpl | Full port except `createTemplatesSnapshot` (watch mode) |
| `templates` | `tpl/tplimpl/templates.go` | T13 tplimpl |  |
| `templatetransform` | `tpl/tplimpl/templatetransform.go` | T13 tplimpl |  |
| `templatedescriptor` | `tpl/tplimpl/templatedescriptor.go` | T13 tplimpl |  |
| `template_funcs` | `tpl/tplimpl/template_funcs.go` | T13 tplimpl | `TemplateExecHelper` (hreflect-based `is_true`, `mainsections` via `Arc::ptr_eq`) |
| `legacy` | `tpl/tplimpl/legacy.go` | T13 tplimpl |  |
| `template_info` | `tpl/tplimpl/template_info.go` | T13 tplimpl |  |
| `category` | `tpl/tplimpl/category_string.go`, `tpl/tplimpl/subcategory_string.go` | T13 tplimpl |  |
| `embedded` | — | T13 tplimpl | NEW: include_bytes! of tpl/tplimpl/embedded/templates/** (read from the Go tree) |

## Dependencies

- nh-*: nh-common, nh-config, nh-media, nh-hugofs, nh-markup, nh-helpers, nh-resource, nh-page, nh-doctree, nh-tpl, nh-langs
- Wave A: gotemplate, go-value, go-unicode, go-strconv, go-fmt, go-time, go-path, go-sort
- crates.io: none (dev only: serde_json, flate2 for reading the gz JSON fixtures; go-json for the probe test's jsonify)

## Deliberate deviations

- **Mutable TemplInfo.** Go mutates `*TemplInfo` fields after insertion (template, parseInfo, base variants, overlays, category of variants…). Rust keeps the immutable identity fields on `TemplInfo` and the mutable ones in `m: RwLock<TemplInfoState>`, read through accessors (`d()`, `sub_category()`, `parse_info()`, `template()`, `content()`, `no_base_of()`, `base_template()`, `base_variants_seq()`, `overlays()`, …). `category` is `Option<Category>` (None = Go's zero Category, used by base-applied variants, deferred and `TextParse` templates) and `path_info` is `Option<Arc<Path>>` (nil in Go for the same templates).
- **Arc cycles.** A `TemplWithBaseApplied` variant and its overlay reference each other as in Go; the store is built once per build so the cycle is accepted (not collected).
- **Map iteration order.** Go iterates `map[TemplateDescriptor]*TemplInfo` (and a few other maps) in random order; Rust uses `BTreeMap` (sorted). Where Go's result does not depend on the order, the outputs are identical. `bestMatch.isBetter` is order dependent when two candidates tie on w1 but differ on w2/w3: Go's output is then itself nondeterministic, and Rust returns the result of the sorted order. The lookup oracle pins Go's iteration to the same sorted order (overlay patch) and additionally records the reverse-order result so these cases are counted (grid: docs 768, legacy 5811, modern 5321 of the records; from real builds: modern 2, the rest 0).
- **Parse name counter.** `doParseTemplate` names duplicate templates `<name>-<N>` with a global counter whose values follow Go's map order; values differ between Go runs. Both the Rust tests and the oracle normalize `-<N>` to `-N`.
- **Deferred template owner.** The owner recorded for a `templates.Defer` inner template is whichever template map iteration hits first in Go; the fixtures show it as `deferred`.
- **Transform recursion.** Transforms work on a cloned root that is written back with `SharedTree::update`. A template that (directly or indirectly) references a tree whose transform is in progress skips re-entering it (Go re-walks the same pointer; the result is the same because every transform is idempotent on an already-transformed tree).
- **Go quirks reproduced on purpose.** The PipeNode command deletion in the `return` transform skips the command after the deleted one (`{{ return $r | upper }}` keeps `upper`); `handleDefer` sets only `StringNode.Text`, not `Quoted`; `removeLeadingBOM` leaves a lone BOM; `Params` map keys are lower-cased and a nil value is treated as missing in `get_map_value`.
- **addFileContext** wraps the error with the template filename only (`herrors::new_file_error_from_name`); Go additionally reads the file to add line context via `extractIdentifiers` (kept as a ported, unused helper).
- **No bestMatch pool.** `getBest`/`putBest` allocate a fresh `BestMatch` per lookup.
- **`TextTemplate::all()`** returns templates sorted by name (Go returns map order).
- **Engine facade.** `engine::TextTemplate`/`HtmlTemplate`/`Template` wrap gotemplate's namespaces; `FuncMap` is a `BTreeMap<String, TplFunc>`.

## Known gaps

- `createTemplatesSnapshot` is not ported and `RefreshFiles` / `createPrototypesParse` return an unsupported error: they are only used by server/watch mode, which the port does not support.
- Watch-mode branches of `insertTemplates`/`parseTemplates` (`partialRebuild`, `replace`) are ported but untested.

## Verification

All fixtures are produced by Go oracles under `tools/go-oracle/nh-tplimpl/` (run with `GOTOOLCHAIN=go1.27.1`, via `go run -overlay` patches in `tsupport/overlay`) and regenerate byte for byte.

| Topic | Oracle | Sites / inputs | Rust test | Result |
|---|---|---|---|---|
| store | `store/main.go` | docs (real layouts), hugolib testsite, synthetic legacy/modern/themes, ~70 txtar archives of `tpl/tplimpl/*_integration_test.go` | `tests/store.rs` | full store dump (tree entries, byPath, shortcodes, namespaces, unused templates) identical |
| escdump | `escdump/main.go` | same sites | `tests/escdump.rs` | every parse tree after transforms + html/template escaping identical (208/91/271/291/119 trees + integration) |
| lookup | `lookup/main.go` | same sites: every lookup of a real build (with Consider replay) + a grid of pages/partial/shortcode/byName queries (docs 16326, testsite 11393, legacy 40507, modern 37308, themes 38267 records + integration) | `tests/lookup.rs` | all identical, incl. errors and candidate sets |
| probe | `probe/main.go` | probe templates executed with the minimal FuncMap on a stub page (html, json, plain) | `tests/probe.rs` | byte-identical output |
| — | Go unit tests | `templatedescriptor_test.go` + needsBaseTemplate, removeLeadingBOM, stringers, `%+v` | `tests/go_tables.rs` | pass |

Regenerate: `GOTOOLCHAIN=go1.27.1 go run ./tools/go-oracle/nh-tplimpl/<topic>` from the repository root (writes `crates/nh-tplimpl/tests/fixtures/<topic>/`).
