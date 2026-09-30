# neohugo (`neohugo-rs`)

The command line (REWRITE_PLAN.md §2.1, §4.8, §7.5). **State: T37.** clap derive; `anyhow` only
here.

```
neohugo-rs [build flags]                 # no command: build (as `hugo`)
neohugo-rs build [build flags]
neohugo-rs templates check [project flags] [--coverage summary|full|none] [--deny-warnings]
neohugo-rs config [project flags] [--format json|toml]
neohugo-rs version                       # also --version
```

## Flags

Kebab-case, with Hugo's camelCase spelling as an alias.

| Flag | Alias | Commands | Effect |
|---|---|---|---|
| `-s`, `--source DIR` | | all | project directory (default: the working directory) |
| `--config A,B` | | all | configuration files, relative to the source, first wins |
| `--config-dir DIR` | `--configDir` | all | `CliOverrides::config_dir` |
| `-e`, `--environment ENV` | | all | wins over `HUGO_ENVIRONMENT` / `HUGO_ENV` |
| `-b`, `--base-url URL` | `--baseURL`, `--baseUrl` | all | `baseURL` |
| `-t`, `--theme A,B` | | all | `theme` |
| `--themes-dir DIR` | `--themesDir` | all | `themesDir` |
| `--cache-dir DIR` | `--cacheDir` | all | `cacheDir` |
| `--ignore-cache` | `--ignoreCache` | all | `ignoreCache` |
| `-D`, `--build-drafts` | `--buildDrafts` | all | `buildDrafts` |
| `-E`, `--build-expired` | `--buildExpired` | all | `buildExpired` |
| `-F`, `--build-future` | `--buildFuture` | all | `buildFuture` |
| `--clock TIME` | | all | the build's "now" (RFC 3339 with offset) |
| `-d`, `--destination DIR` | | build | publish directory, relative to the source |
| `--clean-destination-dir` | `--cleanDestinationDir` | build | static sync removes files the static dirs lack |
| `--minify` | | build | `minify.minifyOutput` |
| `-M`, `--render-to-memory` | `--renderToMemory` | build | `SinkKind::Memory`: nothing is written |
| `--threads N` | | build | render pool size (output does not depend on it) |
| `-q`, `--quiet` | | build | no summary on success |

**Environment.** The process's `HUGO_*` variables (`HUGO_TITLE`, `HUGO_PARAMS_X`,
`HUGO_BASEURL`, `HUGO_CACHEDIR`, `HUGO_ENVIRONMENT`/`HUGO_ENV`, …) plus `HOME`,
`XDG_CACHE_HOME`, `TMPDIR`, `USER` go to `neohugo_config::load`
(`neohugo_build::process_env`); precedence is neohugo-config's (file < config dir < flags <
environment; `-e` wins over `HUGO_ENV*`).

**Exit codes** (`Exit`): 0 success; 1 build errors, check errors (or warnings with
`--deny-warnings`), or a project that does not load; 2 usage errors (clap).

**Reports** (stderr for `build`, stdout for `templates check`):
`ERROR [id] <file>:<line>:<col>: <message>` (then `WARN `, `INFO `), notes (Tera's snippet)
indented below. Build errors that are not diagnostics print `ERROR build failed: <error>`, whose
message carries Tera's `--> <template>:<line>:<col>` snippet. A successful build prints
`pages … | files … (aliases …) | resources … | processed images … | static files …` and
`Total in N ms`.

### Mapping from the old port (`nh-commands`, `go-parity-final`)

| Old flag | Here |
|---|---|
| `source/s destination/d environment/e theme/t themesDir baseURL/b cacheDir ignoreCache buildDrafts/D buildFuture/F buildExpired/E clock config configDir cleanDestinationDir renderToMemory/M minify quiet` | the flags above (kebab-case + the camelCase alias) |
| `contentDir/c layoutDir/l noTimes noChmod disableKinds enableGitInfo printPathWarnings printI18nWarnings panicOnWarning` | configuration keys (file or `HUGO_*`), not flags |
| `watch/w poll` (server) | T71 (`serve`) |
| `logLevel devMode gc noBuildLock forceSyncStatic ignoreVendorPaths renderSegments templateMetrics templateMetricsHints printUnusedTemplates printMemoryUsage profile-* trace` | not supported (clap usage error) |
| commands `new`, `mod`, `deploy`, `gen`, `list`, `convert`, `import`, `env` | not supported; `config` prints the resolved configuration as JSON or TOML |

## `templates check`

Loads the project (configuration, mounts, layouts of the project, its themes and the embedded
set) and reports every problem at once, sorted by file and line; the embedded templates are
loaded but not linted.

1. **Scan** (`neohugo_layouts`): `legacy-name`, `unknown-name`, `go-template` (with the line).
   The files are left out and the check goes on.
2. **Tera.** Each template parsed alone (`tera-syntax`: Tera stops at the first syntax error),
   then all loaded as the build loads them (`neohugo_layouts::load`) against the
   `spec::FUNCS` placeholders: `tera-name` (unknown filters, functions, tests, components,
   include targets, blocks), `tera-load` (missing parents, cycles). Templates with errors are
   loaded empty and the load repeated, so the rest is still checked.
3. **Context names** (`context-name`, warning): Tera's `get_template_variables` (template,
   parents, includes) outside `spec::CONTEXTS` for the role (layout job, alias, standalone,
   sitemapindex, shortcode, render hook + `HOOK_FIELDS`). Partials and components are skipped
   (kwargs, arguments).
4. **Lints** (`src/check/lint.rs`, on the tokenizer `neohugo_funcs::scan` shared with the
   contract test): `kwarg` (unknown/missing kwargs), `legacy-literal` (`include`/`extends`
   literals with `_default/`, `partials/`, `shortcodes/` or upper case), `unknown-partial`,
   `call-attribute` (`.`/`?.` after a call or a parenthesised expression), `nested-close`
   (`}}` inside a `{{ … }}` expression), `content-field` (`page.content` & co. in shortcodes and
   hooks) — errors; `component-scope` (site-bound call in a component without `page=` or
   `@__nh`), `partial-component` (`partial(name="<literal>")` with arguments whose partial
   returns no value), `map-order` (map literal ranged without `sort_keys`), `none-compare`
   (`== none` / `!= none`) — warnings. A Tera syntax error at the position of a
   `call-attribute` / `nested-close` lint becomes that lint's note.
5. **Coverage** (site model loaded as a build does): the layout and base of every (page,
   format) query (`no-layout` warning when none; standalone kinds "not written"), every
   shortcode the content calls (`no-shortcode` error, with the content position), the render
   hooks of content pages (code block hooks per fence language). `--coverage summary`
   (default): templates with query counts plus every miss; `full`: every row; `none`: no model.

Output: a header line, the diagnostics, the coverage listing, `N error(s), M warning(s)`.

## Tests (`cargo test -p neohugo`)

| Test | What |
|---|---|
| `build::testsite_matches_go` | `sites.py`'s testsite with `rust/sites/testsite/layouts`, built by the binary with compare.sh's command line (`--clock … -d …`, no command) and with `build --source … --destination … --cleanDestinationDir -q`: **55/55 files byte-identical** to `crates/build/tests/it/testsite-go.txtar` |
| `build::flags_and_environment` | every configuration flag in both spellings, `HUGO_TITLE`, `HUGO_ENVIRONMENT`, `HUGO_ENV`, `HUGO_BASEURL`, `-M` writes nothing |
| `build::errors_are_reported_with_positions` | render and syntax errors with `file:line:col` and snippet, diagnostics, a missing project: exit 1 |
| `cli::version_help_and_usage_errors` | `version`, `--help`, usage errors exit 2 |
| `cli::config_prints_the_resolved_configuration` | JSON / TOML, config dir, `HUGO_PARAMS_*`, flags |
| `check::bad_layouts_report_every_rule` | `tests/it/bad-layouts.txtar`: each rule once at its position, nothing else, exit 1 |
| `check::testsite_overlay_is_clean` | the testsite overlay: 0 errors, 0 warnings; 35 (page, format) rows with `--coverage full` |
| `check::deny_warnings` | exit 1 on warnings with `--deny-warnings` |
