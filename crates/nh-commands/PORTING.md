# nh-commands — porting notes

neohugo commands/* + main.go (bin `neohugo-rs`): flags -> config, build, version/env/config commands, static copy (spf13/fsync).

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `commandeer` | `commands/commandeer.go`, `main.go` | T25 commands-cli | `execute`/`execute_with`, `RootCommand`, flag declarations |
| `commands` | `commands/commands.go` | T25 commands-cli | the command tree (`new_tree`), `new_exec` |
| `helpers` | `commands/helpers.go` | T25 commands-cli | `flagsToCfg` (flag -> config key mapping) |
| `hugobuilder` | `commands/hugobuilder.go` (+ `countingStatFs`, `chmodFilter` of `server.go`) | T25 commands-cli | load config, build, static copy |
| `config` | `commands/config.go` | T25 commands-cli | `config`, `config mounts` |
| `env` | `commands/env.go` | T25 commands-cli | `env`, `version` |
| `fsync` | — | T25 commands-cli | NEW: spf13/fsync@v0.10.1 Syncer (copy-if-different, chmod, mtimes) |
| `pflag` | — | T25 commands-cli | NEW: spf13/pflag@v1.0.6 FlagSet parse (+ `encoding/csv` record read/write of string slices) |
| `cobra` | — | T25 commands-cli | NEW: spf13/cobra@v1.9.1 `Find`/`stripFlags`/`legacyArgs`/suggestions/`ParseFlags` + bep/simplecobra@v0.6.0 `checkArgs`/`CommandError` |
| `funcmap` | `tpl/tplimplinit/tplimplinit.go` (loop) | T25 commands-cli | the binary's template func map factory (interim, see below) |

The binary is `neohugo-rs` (`src/main.rs`): `commandeer::execute_with(args, ExecOptions {
func_map_factory: funcmap::production_func_map_factory(), .. })`, exit code 0/1 like Go's
`main` (`log.Fatalf("Error: %s", err)`).

## Dependencies

- nh-*: nh-common, nh-config, nh-allconfig, nh-hugofs, nh-helpers, nh-hugolib, nh-deps,
  nh-parser (config TOML/YAML encoders), nh-tplimpl + nh-tplfuncs (the interim func map)
- Wave A: go-value, go-json, go-path, go-strconv, go-unicode
- crates.io: none. clap is not in the offline crate cache; the command line parser is a port of
  pflag/cobra/simplecobra, which also reproduces Go's parse rules and error texts exactly.
- dev: `serde_json` (fixture reader), `flate2` with `rust_backend` (gunzip of the fixtures;
  decompression only, README rule 2).

## The template func map (for I01)

`HugoSites::new` takes `NewHugoSitesCfg.func_map_factory` (`None` = hugolib's default,
`nh_tplfuncs::tplimplinit::create_func_map`). At this task's base `create_func_map` and the
`hugo`, `page` and `site` namespace constructors are `todo!()` (T19 is in progress), so
`funcmap::production_func_map_factory()` returns `Some(interim_func_map_factory())`: Go's
`CreateFuncMap` loop over every other namespace (their real implementations), Go's extra `return`
and `try` mappings (partials, safe), `site` and `hugo` as Go's namespaces define them
(`WrapSite(d.Site)`, `d.Site.Hugo()`), and `page` bound to an explicit "not ported yet" error.
Functions T19 has not ported yet (`now`, `upper`, `transform.XMLEscape`, …) still panic with
`todo!()` when a template calls them.

**I01: once T19 lands, make `production_func_map_factory()` return `None` and delete
`interim_func_map_factory`.** Tests and tools pass their own factory through
`ExecOptions.func_map_factory`.

## Deliberate deviations

1. **Parser.** A port of pflag/cobra (modules `pflag`, `cobra`), not clap: same command
   resolution (`Find`: flags that take a value skip the next argument, `--` ends the flags,
   the `serve` alias), same flag syntax (`--f=v`, `--f v`, `-f v`, `-fv`, `-f=v`, combined bool
   shorthands `-DFE`, bool `NoOptDefVal`, string slices read as CSV and appended on repeat),
   same errors (`unknown flag: --x`, `unknown shorthand flag: 'x' in -x`, `flag needs an
   argument: …`, `bad flag syntax: …`, `invalid argument "v" for "-D, --buildDrafts" flag:
   strconv.ParseBool: …`, csv `parse error on line 1, column 2: …`, `unknown command "x" for
   "neohugo"` with cobra's "Did you mean this?" suggestions in cobra's command order, and
   simplecobra's `checkArgs` error after the command ran).
2. **Help and usage texts are not ported** (stdout only): `-h`/`--help`/`help` and command
   errors print a short usage note instead of cobra's template. Exit codes and stderr are Go's.
3. **Unsupported commands** (`server`/`serve`, `deploy`, `new`, `convert`, `import`, `list`, `mod`,
   `gen`, `release`, `completion` and their sub commands) are resolved like Go, but their flags
   are not declared: the command line fails with the user error `neohugo-rs: the "neohugo
   server" command is not supported` before flag parsing (Go parses their flags, then runs).
4. **Static copy, then build, sequentially** (Go: an errgroup unless `cleanDestinationDir`).
   Go's two goroutines both create the (lazy) `HugoSites`; a construction error gets the
   prefix of whichever fails first in Go (`error copying static files:` or `error building
   site:`); the port always reports `error copying static files:`.
5. **HugoSites ownership.** `hugolib::hugo_sites_build::build` consumes the unbuilt sites and
   returns the frozen `Arc`; `fullBuild` creates them (`new_hugo_sites`), copies static files
   from their base fs, builds, and stores the result in `RootCommand.hugo_sites`
   (`HugoBuilder::hugo` returns it). The logger that Go's `NewHugoSites` creates from
   `DepsCfg{LogLevel, StdOut, StdErr}` (distinct level warn, the `ignoreLogs` statements,
   errors stored when watching) is created in `new_hugo_sites`.
6. **`config` in TOML or YAML** (TOML is the default format!) fails with nh-parser's explicit
   `neohugo-rs: TOML encoding (go-toml Encoder) is not supported` / YAML error:
   `parser.InterfaceToConfig` has no TOML/YAML encoder in nh-parser. `--format json` and every
   other path (the JSON dump, `unsupported format: "xml"`, the language lookup) are Go's.
7. **`config mounts` with a verbose logger** (`--logLevel info|debug`): Go adds the module's
   meta params and `hugoVersion`; the port returns an explicit error.
8. **`env` with a verbose logger** lists only the non-Go dependencies (Go also lists every Go
   module of the binary's build info). The version line has no VCS revision unless the build
   sets `NEOHUGO_VCS_REVISION`/`NEOHUGO_VCS_TIME` (nh-config), and `BuildDate=unknown`.
9. **Not supported (explicit errors):** `-w/--watch`, `--gc` (after the build, as in Go),
   `--profile-cpu`, `--profile-mem`, `--profile-mutex`, `--trace`, `--printMemoryUsage`,
   `renderStaticToDisk` (server only).
10. **No effect:** `--printPathWarnings` (nh-hugofs has no `CreateCountingFs`, so T24's
    `printPathWarningsOnce` has nothing to report), `--panicOnWarning` (nh-common's `Logger`
    has no post-handler hook), the terminal cursor escape codes of `fullBuild`.
11. **fsync errors** are returned instead of Go's panic/recover; Go also runs the deferred
    `syncstats` of the failing entry (whose own panic would replace the error); the port does
    not.
12. **`--clock`** installs `htime::StartClock` with `nh_common::htime::set_clock`, which keeps
    the first clock of the process (Go reassigns `htime.Clock`; a CLI run sets it once).

## Go behaviour reproduced on purpose

- `flagsToCfg` maps every *changed* flag of the executed command, including the flags that
  are not config keys (`format`, `lang`, `printZero` of `config`, `source`, `logLevel`, …):
  `minify` → `minifyOutput`, `destination` → `publishDir`; `quiet`, `renderToMemory`, `clock`
  (and `verbose`, `watch`, `liveReloadPort`) below `internal.`.
- Only persistent flags reach sub commands: `config --minify` is `unknown flag: --minify`,
  `config -D` is `unknown shorthand flag: 'D' in -D`.
- `-dD` sets the destination to `D`; `-Dd out` sets `-d` from the next argument… and then
  `out` is an unknown command (cobra's `stripFlags` treats `-Dd` as a flag without a value);
  `--minify abc` is `unknown command "abc"`; `-Dx` reports `'x' in -x`.
- `PreRun` errors (`--logLevel foo`) are command errors (`Error: command error: invalid log
  level: "foo", …`).
- After `PreRun`, Go's `log` package writes to `r.StdOut`: main's `Error: …` line goes to
  stdout (nowhere with `--quiet`); before it (parse errors) to stderr. `log.Fatalf` adds a
  newline only when the message has none (the suggestion texts end with one).
- simplecobra's `checkArgs` runs after the command: `config foo --format json` prints the
  config, then fails with `unknown command "foo" for "neohugo config"` (the name is always
  `args[1]`); `config --format json foo` does not fail.
- The environment of a build: `--environment`, else `HUGO_ENVIRONMENT`, else `HUGO_ENV`, else
  `production`; `config` leaves it to `allconfig` (production).
- `loadConfig` fails with `Unable to locate config file or config directory. …` when no config
  file was found (`config` does not).
- Static copy: verbatim (dot files, `.bak`), symlinked files and directories followed, broken
  symlinks skipped, the first mount of an overlapping path wins, file modes and times synced
  (never directory modes: `chmodFilter`), identical files not rewritten, `cleanDestinationDir`
  deletes extraneous entries except hidden directories; `copyStatic` ignores a not-exist error
  (no static directory); the static file count is `stat calls / 2`.

## Verification

| topic | inputs | Rust test | checks |
|---|---|---|---|
| `cli/cli.json.gz` | `go run ./tools/go-oracle/nh-commands/cli`: neohugo's real command tree in a child process per case (131 command lines, each run twice), in temp copies of the seeksnack reconstruction (+ alternative config files, a config dir, two themes dirs), the repo's `docs/`, `hugolib/testsite` with and without a config, an empty dir; explicit environment (HOME, TMPDIR, empty PATH, case variables such as `HUGO_ENVIRONMENT`, `HUGO_PARAMS_*`, `HUGO_CACHEDIR`) | `tests/cli.rs` | parse (131): the resolved command, the flag args, the parse error, the changed flags (type, value), the positional args and the `flagsToCfg` provider (goval encoding); run (90): exit code, stdout, stderr of `config` (json, printZero, lang, env, baseURL, cacheDir, destination, source, clock, config files, configDir, themes, themesDir, contentDir, renderSegments, noBuildLock, ignoreVendorPaths, quiet, renderToMemory, logLevel, env overrides), `config mounts`, `version`, `env`, help, and every command line that fails before the build (flag and command errors, invalid clock, no config, missing source); command errors compare stderr and the `Error:` lines (help text not ported). 3 known gaps (TOML/YAML config output) are checked to fail explicitly. The version line is masked (`$OS/$ARCH`, `$DATE`, no revision), as are `GOOS`/`GOARCH` and `Total in N ms`. |
| `staticcopy/staticcopy.json.gz` | `go run ./tools/go-oracle/nh-commands/staticcopy`: 13 synthetic sites (text, binary around fsync's 1000-byte buffer, empty files and dirs, hidden files and dirs, `.bak`, NFC/NFD/Thai/emoji/space names, modes 0600/0755/0444, fixed mtimes; several mounts into static with overlaps, a theme, a file mount, `staticDir` lists; symlinks to files, dirs, outside, broken; a symlinked static dir; existing publish dirs with identical/same-size/grown files, dir↔file swaps and extra entries, with and without `cleanDestinationDir`, `noTimes`/`noChmod`; a custom `publishDir`; no/empty static dir; 23 files) — allconfig + `filesystems.NewBase` + copyStaticTo's Syncer | `tests/staticcopy.rs` | the static file counts, the error, and every entry of the publish dir afterwards (kind, bytes, mode unless noChmod, mtime unless noTimes; directories the root mapping creates report "now"): 118 files, identical |
| `smoke/smoke.json.gz` | `go run ./tools/go-oracle/nh-commands/smoke`: 8 builds of a synthetic site (baseof/single/list/index layouts using only functions ported at this base, markdown, taxonomies, drafts/future, static files incl. `.DS_Store`, build stats): `--minify --clock 2026-09-27T12:00:00Z -d $ROOT/out`, `build --destination=`, `-DF`, `--quiet` with the default publishDir, `--cleanDestinationDir --noBuildLock` over an existing `public/`, `-e staging -b …`, `HUGO_ENVIRONMENT`, `-s` | `tests/smoke.rs` | exit code, stdout (the processing stats table), stderr, every published file, `hugo_stats.json`, `.hugo_build.lock`: 124 files, identical |
| Go tests | spf13/fsync `fsync_test.go` (TestSync, TestDeleteFileFilter, TestDeleteFileFilterNotSet) | `tests/fsync_go_tests.rs` | ported assertions |

Results: 0 mismatches. The release binary built the smoke site into output identical to the
Go binary's (`diff -r`).

Regenerate (each byte for byte; every site is built in a temp dir):

```sh
export GOTOOLCHAIN=go1.27.1
go run ./tools/go-oracle/nh-commands/cli -root .
go run ./tools/go-oracle/nh-commands/staticcopy
go run ./tools/go-oracle/nh-commands/smoke
```

The cli oracle reaches `commands.newExec`, `mapLegacyArgs` and `flagsToCfg` with
`//go:linkname` and the root `Commandeer` of `simplecobra.Exec` with `reflect`/`unsafe`.

## Known gaps / requests

- nh-parser: TOML (go-toml v2 Encoder) and YAML (yaml.v2 Marshal) encoders for
  `parser.InterfaceToConfig` (the default `neohugo config` output).
- nh-hugofs: `hugofs.NewCreateCountingFs` + a `DuplicatesReporter` that T24's
  `printPathWarningsOnce` can read (`--printPathWarnings`).
- nh-common: a post-handler hook on `Logger` for `loggers.PanicOnWarningHook`
  (`--panicOnWarning`).
- T19: `tplimplinit::create_func_map` (then I01 switches the binary to it, see above).
- cobra's help/usage templates (stdout only).
