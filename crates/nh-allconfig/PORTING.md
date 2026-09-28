# nh-allconfig — porting notes

neohugo config/allconfig (load, decode all sections, compile, per-language configs),
hugolib/segments, deploy/deployconfig (decode only). Crate lead: Wave B task T09
(allconfig-modules), which also owns `nh_hugofs::modules` (the project module, local themes and
`_vendor`; see the modules section below).

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `allconfig` | `config/allconfig/allconfig.go` | T09 allconfig-modules | ported |
| `alldecoders` | `config/allconfig/alldecoders.go` | T09 allconfig-modules | ported: all 33 decoders, in Go's weight order |
| `configlanguage` | `config/allconfig/configlanguage.go` | T09 allconfig-modules | ported: every `AllProvider` method |
| `load` | `config/allconfig/load.go` | T09 allconfig-modules | ported |
| `segments` | `hugolib/segments/segments.go` | T09 allconfig-modules | ported |
| `deployconfig` | `deploy/deployconfig/deployConfig.go` | T09 allconfig-modules | STUB (decode and validate only; `hugo deploy` is not ported) |
| `sections` | (NEW) | T09 allconfig-modules | `GetConfigSection` names → typed sections (see below) |
| `json` | `commands/config.go` (`configCmd`, `configMountsCmd`), Go's `json.Marshal(*Config)` | T09 allconfig-modules | NEW: `hugo config` / `hugo config mounts` output |

Every GO PORTING CHECKLIST entry is `OK`, apart from the `STUB` entries of
`nh_hugofs::modules::client` (Go module management, below).

### `nh_hugofs::modules` (owned by T09)

| Rust module | Go source | Note |
|---|---|---|
| `modules::config` | `modules/config.go` | ported (`DecodeConfig`, `ApplyProjectConfigDefaults`, `HugoVersion`) |
| `modules::collect` | `modules/collect.go` | ported: project module, `themes/` and `_vendor` (`modules.txt`) imports, theme configs, mounts, `_jsconfig` auto-mounts |
| `modules::module` | `modules/module.go` | ported (`Time()` is always zero: no Go modules) |
| `modules::client` | `modules/client.go` | `NewClient`, `Graph`, `createThemeDirname`, `toEnv`, `isProbablyModule` (x/mod `CheckPath`/`SplitPathVersion` v0.25.0) ported; `Tidy`, `Vendor`, `Get`, `Init`, `Verify`, `Clean`, `go get` return `ErrorKind::FeatureNotAvailable` "neohugo-rs: … is not supported" |

A project with a `go.mod` **and** a module import fails to load with
`neohugo-rs: Go modules (go.mod with module imports; …) is not supported` (Go would run
`go list`/`go mod download`). Without a `go.mod`, a module path is looked up in `themes/<path>`
like Go does, with Go's `module does not exist` error when it is missing.

### Config sections (`AllProvider::get_config_section`)

Go names (same values as Go): `security`, `build`, `frontmatter`, `caches`, `markup`,
`mediaTypes`, `outputFormats`, `permalinks`, `minify`, `allModules`, `deployment`,
`httpCacheCompiled`. Rust-only names (Go callers read the fields directly): `contentTypes`,
`imaging` (the `ConfigNamespace`), `httpCache`, `services`, `privacy`, `server`, `sitemap`,
`pagination`, `page`, `related`, `taxonomies`, `outputs`, `kindOutputFormats`, `cascade`,
`menus`, `segmentFilter`, `params`.

## Dependencies

- nh-*: nh-common, nh-parser, nh-langs, nh-config, nh-media, nh-hugofs, nh-markup,
  nh-transform, nh-helpers, nh-images, nh-page.
- Wave A: go-value, go-time, go-json, go-path, go-sort, go-strconv, go-unicode.
- crates.io: none.
- dev: `serde_json`, `flate2` (`rust_backend`, gunzip of fixtures).

## Deliberate deviations

1. **Shared maps.** Go's provider hands out its own maps, and several decoders write into them
   (images `DecodeConfig` merges defaults into the `imaging` map, `fromLoadConfigResult`
   injects `params` and merged `SetParams` entries into the language maps, the languages
   decoder sets `languagecode`, CLI flag maps are aliased into the tree so a file's `internal`
   leaks into the flags, `deleteMergeStrategies` strips `_merge` from the live imaging and
   segments source structures). nh-config's `DefaultConfigProvider` is copy-on-write: each of
   these writes is replayed explicitly (`write_back_imaging`, `cfg.set(...)`, `FlagAlias`,
   `imaging_live`/`segments_live`). Not replayed (only visible in the raw tree, not in any
   decoded section): the output-format decoder turning a `mediaType` string of a theme's or a
   language's own map into a `media.Type`, and the theme config maps aliased into the site's
   `languages` map.
2. **`deleteMergeStrategies`** removes `_merge` from the decoded values Go keeps
   (`strip_merge`), not from the provider (nh-config has no delete operation).
3. **nil vs empty.** `GoSlice<T>` and `Option<Map>` keep Go's nil/empty distinction where the
   JSON dump or a hash shows it; other fields use empty values.
4. **Map order.** Go iterates maps (languages, decoders' inputs) in random order; the port uses
   sorted order. Four oracle cases depend on the order in Go (`basic/cache-dir-relative`,
   `languages/lang-overrides-root-scalars`, `sections/related-invalid-type`,
   `sections/outputs`): the fixtures record every Go variant and the port must match one.
5. **Mount include/exclude files** are `Vec<String>` in `Mount` (normalised at decode); the raw
   `any` values are kept in `Config.mounts_raw_files` for the JSON dump.
6. **Directory order.** `_jsconfig` auto-mounts come from a directory listing in OS order
   (APFS sorts, ext4 does not); tests compare those mount runs as a set.
7. **Config files** are read from the OS file system (nh-config's loader); the descriptor's
   `fs` is used for module and theme lookups.
8. **Owner Arcs** (`Configs.base` and the per-language `Arc<Config>`) are snapshots taken after
   `Init`; Go shares one pointer that later code may mutate.
9. **Segments.** A matcher entry with no field set after the first makes Go's combined predicate
   a nil function; calling it panics in Go and in the port (`nil_predicate`).
10. **Timeouts.** `ConfigCompiled.timeout` is a `go_time::Duration`; `AllProvider::timeout`
    saturates a negative value to zero for `std::time::Duration`.
11. **DeployConfig.ordering** holds compiled `goregexp::Regexp`s (Go `regexp`); `hugo deploy`
    itself is not ported.
12. **`ConfigSourceDescriptor`** gains `getenv` (the process environment of
    `helpers.GetCacheDir`), `fs` and `logger` so the tests do not touch the process environment.

## Known gaps / requests

- Go module management (`hugo mod get/tidy/vendor/verify/clean/init/npm pack`) and resolving
  remote modules: explicit `FeatureNotAvailable` errors (see above).
- T25 (commands): use `json::config_dump(configs, lang, print_zero)` and `json::mounts_dump`
  for `neohugo config [--format json] [--printZero] [--lang]` and `neohugo config mounts`.
- T20/T23: `ConfigCompiled::main_sections()` / `set_main_sections`, `Configs::config_langs()`.

## Verification

| topic | inputs | Rust test | checks |
|---|---|---|---|
| `load/{basic,configdir,env,languages,merge,mounts,repo,sections,seeksnack,themes}.json.gz` | 227 site trees recreated in a temp dir and loaded with CLI-like flags: every config file format and legacy `config.toml`, config dirs with environments and per-language files, `HUGO_*` env overrides (nested keys, `HUGO_PARAMS_*`, owners), languages (per-language overrides, multihost, disabled languages, `defaultContentLanguageInSubdir`), `_merge` strategies, every section (caches, cascade, segments, deployment, security, minify, build stats, markup, permalinks, outputs, taxonomies, related, server, imaging, …) including invalid values, mounts (include/exclude, lang, legacy dirs, `_vendor`, themes with configs and `min_version`), the repo's `docs/` site and `hugolib/testsite`, the seeksnack reconstruction | `tests/load.rs` | per case: every decoded section of the base and every language config, languages order, `IsMultihost`, compiled values, cache dirs, SourceHash values, the modules and mounts, the `hugo config` and `hugo config mounts` text, the raw provider tree, the log and error texts (basic 48, configdir 10, env 29, languages 24, merge 13, mounts 13, repo 3, sections 64, seeksnack 3, themes 20) |
| seeksnack | `tests/fixtures/load/seeksnack/hugo.toml` (reconstructed from `docs/rust-port/specs/architecture-core-data/`), `--minify --clock 2026-09-27T12:00:00Z`, production | `tests/seeksnack.rs` | imaging SourceHash `4bf645f71319dd1d` for base and both languages; `config-en.json`, `config-en-printzero.json`, `config-th.json` and `config-mounts.json` byte-identical (apart from `workingdir`/`cachedir` and the `_jsconfig` directory order) |
| Go tests | `deployConfig_test.go`, `segments_test.go` (`src/segments.rs`), `modules/config_test.go`, `modules/collect_test.go` (nh-hugofs) | `tests/go_tables.rs`, unit tests | ported assertions |
| unsupported | go.mod + module import; module path under `themes/` | `tests/unsupported.rs` | explicit error; themes lookup |

Results: 0 mismatches.

Regenerate (about 75 s; each case runs in its own process because some Go defaults are
package-level values a decode mutates; a case with several Go results is rerun 1,000 times):

```sh
export GOTOOLCHAIN=go1.27.1
go run ./tools/go-oracle/nh-allconfig/load -root . -out crates/nh-allconfig/tests/fixtures/load
```

The fixtures regenerate byte for byte.
