# neohugo-config

The configuration of a project (docs/rust-port/REWRITE_PLAN.md §2.4, §3.1 A1): one
`Value`-tree pipeline, then typed serde structs whose `Default` holds Hugo's defaults.

```
LoadOptions { source, config_files, cli: CliOverrides, env }
  └─ load() ─► Config { sites: IdVec<LangIdx, SiteConfig>, output_formats, media_types, … }
```

1. **Bootstrap**: environment = `--environment` → `HUGO_ENVIRONMENT` → `HUGO_ENV` →
   `production`; config directory = `--configDir` or `config`.
2. **Sources** (`source`): the project file (first of `hugo.toml`, `hugo.yaml`, `hugo.yml`,
   `hugo.json`, `config.*`; or the `--config` list, first file wins, `custom` finds
   `custom.toml`…), then `config/_default/**` and `config/<env>/**` in path order. A file's
   name places it: `hugo.*`/`config.*` at the root, `params.en.toml` under
   `languages.en.params`, `menus.en.toml` (or `menu.en.toml`) under `languages.en.menus`,
   `name.toml` under `name`.
3. **Normalise** (`tree::normalize_keys`: lower-case keys except inside arrays, `menu` →
   `menus`, drop `internal`) and **migrate legacy keys** (`tree::LEGACY_KEYS`, applied at the
   root and in each language table; a current key wins; each migration is a deprecation
   `Diagnostic`).
4. **Merge once**: file < `_default` < `<env>` < CLI (`CliOverrides::to_tree`) < `HUGO_*`
   (`env`). `disableKinds`/`disableLanguages` strings are split on commas and white space.
   `_merge = "none"` in a table makes it replace instead of merge.
5. **Languages**: the enabled languages (not `disabled`, not in `disableLanguages`) sorted by
   (weight, key), the default language first. Default = `defaultContentLanguage`, else `en`
   if configured, else the first language. Each language's tree is `languages.X` merged over
   the root: tables merge deeply, `menus`, `taxonomies` and `permalinks` replace.
6. **Typed decode** (`de`): a serde `Deserializer` over `Value` that matches struct fields and
   enum variants ignoring ASCII case and converts scalars weakly (`"true"`, `"12"`, a single
   value for a list). Errors carry the key path; `load` maps it back to the file, line and
   column the value was written on (TOML via `toml::de::DeTable` spans; YAML/JSON by key
   search) and reports the key as written (`languages.th.pagination.pagerSize`).

## Public API

| item | what |
|---|---|
| `load(&LoadOptions) -> Result<Config, ConfigError>` | the pipeline |
| `LoadOptions` | `source`, `config_files`, `cli`, `env` (the process environment: `HUGO_*` plus `HOME`, `XDG_CACHE_HOME`, `TMPDIR`, `USER`) |
| `CliOverrides` | typed CLI layer (`base_url`, `environment`, `destination`, `minify`, `build_drafts/future/expired`, `cache_dir`, `themes_dir`, `theme`, `ignore_cache`, `config_dir`); `to_tree()` |
| `Config` | `project_dir`, `environment`, `config_files`, `sites`, `disabled_languages`, `multihost`, `default_language_in_subdir`, `output_formats: Arc<OutputFormats>`, `media_types: Arc<MediaTypes>`, `content_types`, `default_output_format`, `dirs`, `cache_dir`, `mounts`, `themes`, `build`, `caches`, `security`, `privacy`, `imaging`, `minify`, `content: ContentFilter`, `timeout`, `ignore_files`, `ignore_logs`, `enable_git_info`, `raw: Params`, `diagnostics`; `default_site()`, `site(key)` |
| `SiteConfig` (`site`) | per language: `language: Language` (key, name, code, `Direction`, weight, `time_zone`, `url_prefix`, own title), `base_url: BaseUrl`, `title`, `copyright`, `params: Params`, `taxonomies: IdVec<TaxonomyIdx, TaxonomyDef>`, `outputs: KindOutputs`, `permalinks: Permalinks`, `pagination`, `markup: MarkupConfig`, `front_matter: Vec<(DateField, Vec<DateSource>)>`, `related`, `sitemap`, `services`, `menus: Vec<MenuEntryConfig>`, `cascade: Vec<CascadeConfig>`, `urls: UrlPolicy` (`LinkStyle`, `LinkOutput`, `UglyUrls`, `PathCase`, `Accents`), `disable_kinds: KindSet`, `aliases: AliasPolicy`, `robots_txt: RobotsPolicy`, `emoji: EmojiPolicy`, `titles: TitleConfig`, `summary_length`, `content_dir`, `static_dirs`, `ref_links`, `main_sections`, `has_cjk_language`; `site_urls()` → `base::url::SiteUrls` |
| `MediaTypes` / `MediaType` (`media`) | `IdVec<MediaTypeId, _>` sorted by type string; `decode`, `get`, `by_type`, `by_suffix`; `BUILTIN`, `ContentTypes` |
| `OutputFormats` / `OutputFormat` (`output`) | `IdVec<FormatId, _>` in render order (non-zero weights ascending, then name); `Escaping`, `UglyPolicy`, `LinkPolicy`, `Listing`, `Placement`; `builtin`, `decode`, `by_name` |
| `global` | `Dirs`, `BuildConfig`, `CachesConfig`/`FileCache`/`MaxAge` (`:cacheDir`, `:project`, `:resourceDir` resolved), `SecurityPolicy`/`Whitelist`, `PrivacyConfig`, `ImagingConfig`, `MinifyConfig`, `MountConfig`, `ContentFilter` |
| `markup` | `MarkupConfig`: goldmark extensions/parser/renderer/render hooks (`UseEmbedded`), `HighlightConfig`, `TocConfig`, `AsciidocConfig` |
| `de`, `tree`, `env`, `duration` | the pipeline's building blocks (`de::from_value` is reusable for any case-insensitive typed decode) |

## Acceptance (T13)

`cargo test -p neohugo-config -- --nocapture` prints the tallies.

| bullet | evidence |
|---|---|
| `nh-allconfig/load` values for docs, testsite, reconstruction | `load::acceptance_sites` (strict, no accepted differences): `repo/docs`, `repo/docs-development`, `repo/testsite` 294/294 checks, `seeksnack/{config,build,build-env-development}` 426/426. Each case compares the language order, multihost, timeout, compiled cache directories, per-language facts (baseURL, prefix, name, weight, title, time zone, direction), kind outputs, disabled kinds, and 57 keys of Go's per-language `hugo config` dump (zero values ignored; `[minify.tdewolff]` defaults belong to the minify crate) |
| … and t24 sites | `sites::t24_sites`: the 17 `hugolib/assemble` sites load; languages equal Go's for all; the formats of the enabled kinds equal Go's `.Site` render formats for 25 of 26 site languages (`asm-taxo` adds `json` in page front matter, which the site config cannot know; the check is "a subsequence of Go's") |
| other `load` groups (coverage) | `load::other_groups`: basic 2738/2747, configdir 1015/1015, env 2051/2054, languages 2287/2290, merge 1049/1111, mounts 1045/1045, sections 3126/3139, themes 1122/1136; every difference is listed in `expected_diffs.toml` with its reason |
| media tables equal | `media::builtin_media_tables`: the 41 built-in types (field by field, sorted: main, sub, mime suffix, suffixes, delimiter, first/full suffix) and the 6 default content types equal Go's; `media::output_formats`: the 15 built-in output formats in render order, every field |
| `[mediaTypes]`/`[outputFormats]` decoding | DecodeTypes 1165/1410 equal, DecodeConfig 1203/1530 equal; named cases that differ are listed in `tests/it/media.rs` with reasons; generated cases are tallied by category (see deviation 8) |
| legacy-key table on a synthetic S-style config | `api::legacy_keys`: a seeksnack-style config written with every legacy key (root and language level) decodes to the current settings; the deprecation notices are an insta snapshot |
| env typing; `CliOverrides` | `api::env_typing` (int, float, bool, JSON list, nested new keys, custom delimiter, JSON table, disableKinds splitting, unparsable value stays a string); `api::cli_overrides_and_precedence` (file < dir < env dir < CLI < env; `HUGO_ENVIRONMENT`) |
| `[caches]` with `:cacheDir`/`:project`; privacy | `api::caches_resolve_placeholders`, `api::privacy`, plus the oracle's `cachesCompiled` and `privacy` in every load case |
| error spans | `api::error_positions`: TOML, YAML and JSON syntax errors; typed errors in a config-dir file, in a language table, in an array element; language errors — each with file, line and column |
| insta snapshots | `tests/it/snapshots/`: typed summaries of docs, testsite, seeksnack and the 17 t24 sites; deprecation notices; an error display |

## Accepted deviations

1. **Theme configuration is not merged.** A1 has no theme layer (themes are mounts, §3.1 A2),
   so a theme's `hugo.toml` does not contribute params, menus, languages or formats (`merge/*`,
   `themes/*` cases). Theme lookup errors belong to the file system layer (T20).
2. **Project-wide settings come from the default language.** Go keeps imaging, media types and
   output formats per language; neohugo has one of each per project (the default language's).
   Per-language `contentDir` and `staticDir` are kept (`SiteConfig::content_dir`, `static_dirs`).
3. **Untyped sections**: `deployment` (`hugo deploy` is not ported), `segments`, `httpCache`,
   `server`, `module` imports/replacements/workspace/hugoVersion and `[minify.tdewolff]` stay in
   `Config::raw`; their validation errors are not reported here. Resample filter names are the
   images crate's (T41).
4. **Errors instead of silently ignored values**: a `[caches]` entry that is not a table,
   `HUGO_SITEMAP=weekly` (a table expected), an output format without a media type, a missing
   `--config` file, no configuration at all (Go leaves that check to the CLI), invalid media
   type keys (`/`, `text/`).
5. **Lists in `HUGO_*`** written as `['a']` are parsed as lists (Go keeps the literal string as
   one pattern); a number of seconds written as a string is a valid `maxAge`.
6. **`disableLanguages` inside a language table** is ignored (a root-only setting; Go applies
   the default language's merged value).
7. **The cache directory is never created while loading**: `$XDG_CACHE_HOME/hugo_cache` (or
   `$HOME/.cache/hugo_cache`) is used when its first existing ancestor is a directory, else
   `$TMPDIR/hugo_cache_$USER`. A relative `cacheDir` (or `:project/…`) is an error, as in Go.
8. **No mapstructure weak decoding of struct fields**: a media type entry can set only
   `suffixes` and `delimiter` (not `mainType`, `subType`, `type`, `firstSuffix`,
   `suffixesCSV`), an output format's name is its key, non-string suffixes and fractional
   weights are rejected, and keys are normalised before decoding (so `RSS` and `rss` collide).
   These account for all differences of the generated `media` cases.
9. `--clock` is not configuration (the CLI parses it into `base::Clock`).

## Notes for later tasks

- **Decoders for other crates**: `Permalinks::decode(&Value)` (rejects kinds without
  permalinks and non-string patterns, as Hugo does), `decode_front_matter(&Map)`,
  `decode_cascade(&Value)`, `DateSource::parse`; `markup::TocConfig::end_level` is
  `Option<u8>` (`endLevel = -1` is `None`).
- **base serde** (F1): `IdVec` and `Params` serialize and `Map` deserializes in base; `de_map`
  remains for sections where `null` means an empty table.
- **vfs (T20)**: default mounts for unconfigured components, `_jsconfig` auto-mounts and
  language content directories (`SiteConfig::content_dir`) are the file system layer's; the
  mount targets are validated here.
- **markup (T22)**: `UseEmbedded::Auto` is resolved by the renderer (Go: fallback for
  multilingual single-host sites); the default is already `Fallback` in that case.
- **plan**: §3.1 A1 has no theme-config layer (see deviation 1); decide whether themes may
  contribute configuration.
