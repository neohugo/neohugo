# nh-deps — porting notes

neohugo deps/deps.go: per-site dependency container (the Rust context struct).

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `deps` | `deps/deps.go` | T20 hugolib-capture |  |

## Dependencies

- nh-*: nh-common, nh-config, nh-media, nh-hugofs, nh-helpers, nh-resource, nh-page, nh-allconfig, nh-tpl, nh-tplimpl, nh-resources
- Wave A: go-value, go-path (`filepath.Ext` in the HasBytes check).
- crates.io (justify each): none. dev: `serde_json` (unused for now).

## Deliberate deviations

Status (T20): ported. Every EX checklist entry is `OK` except `StopStageRender` (deferred
templates, `renderDeferred`: T24).

1. **`TranslateFunc` returns `Result<String>`** (Go: `string`). Go's translate func panics for a
   few inputs and text/template turns the panic into the `i18n`/`T` error; the `Err` carries Go's
   panic text. `nh_i18n::i18n::Translator::func` is now `func_e` (T20 updated nh-i18n's call site
   and test).
2. **`Compile(prototype)`** needs `TranslationProvider` (nh-i18n), which depends on this crate: it
   lives in nh-hugolib (`hugo_sites::compile_deps`), and `HugoSites.translation_provider` holds the
   provider (Go `Deps.TranslationProvider`).
3. **`Init`/`Clone`** are `Deps::init(DepsCfg)` (first site: the fields of Go's `firstSiteDeps`
   literal are in `DepsCfg`) and `Deps::clone_for(conf)` (Go `Clone` + `Init`). As in Go, a clone
   shares fs (with the wrapped publish dir), SourceSpec (the first site's; Go's `Init` keeps the
   copied non-nil one), memory cache, BuildState, Counters, listeners, error handler and the
   resources `SpecCommon`; it gets its own exec helper, PathSpec (over a BaseFs built with
   `NewPathSpecWithBaseBaseFsProvided`), ContentSpec, file caches and resources Spec.
4. The HasBytes receiver records `__h_pp_l1` files in `BuildState.filenames_with_post_prefix`
   and `__hdeferred/` files in `BuildState.filenames_with_deferred_prefix` (Go:
   `DeferredExecutions.FilenamesWithPostPrefix`); patterns in Go's order (deferred, post).
5. `globalErrHandler`: `start_error_collector` / `stop_error_collector` replace the channel and
   Go's reader goroutine in `HugoSites.Build` (which keeps the first 50 errors); `send_error`
   without a collector logs the error. The resources Spec's `error_sender` is set to it (Go passes
   `d` as the error handler).
6. `Listeners[T]`: funcs take `&[T]`; `notify` runs them under the lock like Go.
7. Not ported (not used by a one-shot seeksnack build): `Metrics` (templateMetrics), `WasmDispatchers`
   (KaTeX etc.), `JSBatcherClient` (`js.Batch`), `OnChangeListeners`, `BuildClosers` (none
   registered on the ported paths), `MkdirTemp`, `SignalRebuild`, `OverloadedTemplateFuncs`.

## Known gaps

- `StopStageRender` / deferred executions (T24 owns `renderDeferred`; unused by seeksnack).

## Test seam: `Deps::for_tests(conf)` (T15, T17, T18, T19)

Unchanged contract: a `Deps` with only `conf` (warn logger, fresh BuildState/Counters, a fresh
memory cache, no services); add services with `with_*`. New public fields (`mem_cache`, listeners,
`global_err_handler`) are filled by `for_tests`, so struct users need no change.

## Verification

`tests/deps.rs`: BuildState (Incr from 1, sorted post-prefix names), Listeners (removal on true),
the error collector (50-error cap, logging without a collector). The construction path is tested
end to end by nh-hugolib `tests/construction.rs` (seeksnack reconstruction: 2 sites, shared vs
per-site services, one PostProcess counter through `SpecCommon`, template stores, translators).
