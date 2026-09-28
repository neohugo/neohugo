# nh-resource — porting notes

neohugo resources/resource (Resource interfaces, Resources, params, dates) and resources/internal
(keys, target paths). Owner and crate lead: Wave B task T11 (page-api-paths).

## Go file → Rust module

| Rust module | Go source(s) | Owner | Status / note |
|---|---|---|---|
| `resourcetypes` | `resources/resource/resourcetypes.go` | T11 page-api-paths | ported; the Go interface family is one object-safe trait `Resource` |
| `resources` | `resources/resource/resources.go` | T11 page-api-paths | ported (all functions, getters included) |
| `dates` | `resources/resource/dates.go` | T11 page-api-paths | ported |
| `params` | `resources/resource/params.go` | T11 page-api-paths | ported |
| `resource_helpers` | `resources/resource/resource_helpers.go` | T11 page-api-paths | ported |
| `internal::key` | `resources/internal/key.go` | T11 page-api-paths | ported |
| `internal::resourcepaths` | `resources/internal/resourcepaths.go` | T11 page-api-paths | ported (+ `new_resource_paths`, a constructor with no Go counterpart) |

Every GO PORTING CHECKLIST entry is `OK`; every ported function carries a `// Go:` line.

## Dependencies

- nh-*: nh-common, nh-config, nh-media, nh-langs, nh-helpers.
- Wave A: go-value, go-path, go-time, go-unicode.
- crates.io: none. dev: `serde_json`, `flate2` (`rust_backend`, gunzip of the fixtures).

## API notes for the other tasks

- `Resource` gained two optional methods with defaults: `stale_version()` (Go `StaleInfo`,
  `None` when not implemented) and `mark_stale()` (Go `StaleMarker`).
- Template values: a non-page resource is `ResourceRef`; `resource.Resources` is a named list
  (`resources_to_value`). The named-type methods are `resources::RESOURCES_METHODS` (Get,
  GetMatch, Match, ByType, Mount, MergeByLanguage, MergeByLanguageInterface); nh-hugolib registers
  them. Their elements are converted with `resource_from_value_any`, whose converter nh-page
  installs (`nh_page::page::init()`, which also calls `resourcetypes::register()`: the
  `resource.Resource` interface for `hreflect`). Until `init` runs only `ResourceRef` elements are
  recognised.
- Results follow HUGO_LAYER.md §4.7: a missing `Get`/`GetMatch` is `TypedNil("resource.Resource")`,
  an empty `Match`/`ByType` result `TypedNil("resource.Resources")` (Go's nil slice).
- `Mount` returns a `resource.resourceGetterFunc` object with `Get`.

## Go behaviour reproduced on purpose

- `Resources.Get` on a nil receiver returns nil before converting the name (so a bad name does
  not panic); the `try_get(None, ..)` form is that receiver.
- `Get`/`GetMatch`/`Match` fall back to `NameNormalized()` only for resources that implement it,
  and `Match` only when no `Name()` matched.
- `MergeByLanguage` never adds a resource of `r2` that has no translation key.
- `GetParam`: only the listed Go types are returned (`[]any`, `template.HTML`, uint kinds,
  `maps.Params` values give nil); float32 becomes float64 and every int kind `int`; go-toml local
  dates are converted with `AsTime(UTC)`; a typed nil `[]string` is returned as is.

## Deliberate deviations

1. **Go panics.** `Get`, `GetMatch`, `Match`, `ByType`, `Mount`'s getter and the cached getter
   panic in Go on an argument cast cannot convert or a glob that does not compile. The `try_*`
   forms (and the template dispatch) return the panic value as an error; the plain forms panic
   with the same message.
2. **nil vs empty.** `Resources` is a `Vec`; functions whose Go result distinguishes a nil slice
   return `Option<Resources>` (`None` = nil) in their `try_*` form.
3. **cachedResourceGetter** computes a missing entry without holding the cache lock (HUGO_LAYER.md
   §4.8); the first stored value wins, as in Go's `maps.Cache.GetOrCreate`.
4. **unwrapResourceGetter** works on template values: an object with a `Get` method, an object
   with a `Resources` method (Go's `ResourcesProvider`), a `resource.Resources` list, or a slice
   of those.
5. `NewResourceError` keeps the error's message (the error value itself is not wrapped).

## Known gaps

None.

## Verification

Oracle `tools/go-oracle/nh-resource/resources` (package `main` in the neohugo module) writes
`tests/fixtures/resources/resources.json.gz`; the test is `tests/resources.rs`.

| topic | inputs | checks |
|---|---|---|
| `resources` | 5 resource sets (Go's `TestResourcesMount` sets; names that differ from their normalized names, Thai/accented/space names, a resource without `NameNormalized`, translation keys) × 38 `Get` arguments (exact, `./`, case-folded, normalized-only, missing, `int`/`int64`/`float64`/`bool`/`nil`/`template.HTML`/`[]string`), 26 glob patterns (every gobwas feature, bad patterns, non-strings) for `GetMatch`/`Match`, 8 `ByType` arguments, 13 `Mount` (base, target) pairs × 17 names, every `MergeByLanguage` pair; `NewCachedResourceGetter` over two sets; `Param` (with and without fallback), `GetParam`, `GetParamToLower` over 30 keys of a params map with every value kind | 1,812 cases, 0 failures (26 are Go panics, matched by message) |

`resources/internal` cannot be imported by an oracle (Go's internal-package rule), so its tests
are ported by hand (`resource_paths`, `resource_transformation_key`), with the template dispatch
(`resources_template_methods`), dates and the stale helpers (`dates_and_stale`).

Regenerate (platform independent; reproduces byte for byte):

```sh
export GOTOOLCHAIN=go1.27.1
go run ./tools/go-oracle/nh-resource/resources
```
