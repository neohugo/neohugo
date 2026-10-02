# neohugo Rust rewrite: handoff (start here)

State as of T70 (2026-09-30) and the removal of the Go implementation (2026-10-01, §9), branch
`rust-port`. The plan and its task table are [`REWRITE_PLAN.md`](REWRITE_PLAN.md) (§8.2 holds
every task's state); the workspace's own rules and CI details are
[`DEVELOPMENT.md`](../../DEVELOPMENT.md). This document says what exists, how
to build, test and run it, how the gates work, what deviates from Hugo, and what is open.

## 0. Summary for the pull request

neohugo is now an idiomatic Rust rewrite of the Go neohugo (a Hugo fork) and replaces it in
place: the same binary (`neohugo`), version line and release archives (§9). The Cargo workspace
is at the repository root: Hugo's site and page model (content tree, bundles, kinds, front
matter, cascade, permalinks, output formats, taxonomies, menus, pagination, i18n, Hugo Pipes,
image processing, Markdown with render hooks and shortcodes) with **Tera 2** templates instead
of Go templates (decision D4). It is not a byte-for-byte port: the Go build was the oracle (its
outputs are frozen as golden data, §9), and outputs are compared structurally.

- **Gates passed** (REWRITE_PLAN.md §7.3; each against the Go neohugo's build of the same site):
  - A-T, the testsite: L1 56/56, L2 and L3 equal on every page, structure oracle equal.
  - A-R, the seeksnack reconstruction: L1 713/713, L2 427/427, structure 673/673, A7 1.0.
  - A-D1, the Hugo docs with the `i01` patches: L1 888/888, L2 756/756, L3 749/750, A7 1.0.
  - A-D2, the Hugo docs with the `reduced` patches (Chroma-class highlighting, GoAT diagrams,
    emoji, math, Tailwind, the real Alpine/Turbo `js_build`): L1 889/889, L2 757/757, L3
    750/751, A7 1.0.
  - A-D3, the docs **without patches against the published site** https://neohugo.github.io/
    (the Go build of 2025-10-13; T74): L1 2373/2373, L2 776/776, L3 769/770, L4 873/873, A7
    1.0 — every page's visible text equals the published one (the one L3 difference is Go's
    stats tokenizer reading `<?xml`/`<=` as tags). `tools/docs/build.sh` builds the site.
  - A-DET: identical output with 1 and 8 threads and across runs.
- **A-P, performance** (T70; release build, 4 CPUs, medians of 5–6 runs): the docs site builds
  cold in 3.29 s against Go's 3.99 s (0.82×; goal ≤ 1.5×), warm in 3.26 s against 3.51 s
  (0.93×; goal ≤ 1.0×), peak RSS 384 MB (goal ≤ 1 GB). The testsite and the seeksnack
  reconstruction are faster than Go as well (§5).
- **Commands:** `neohugo build` (also with no command), `server` (live reload, memory or
  disk), `templates check`, `config`, `version`.
- **CI/CD:** `.github/workflows/ci.yml`, the only build workflow (fmt, clippy `-D warnings`,
  licence check, the whole test suite with the gate tests, release builds for five targets);
  tags `v<version>` publish a GitHub release with archives, licence notices and checksums.
- **Go removed** (2026-10-01): the Go implementation, its tools and its workflows are gone; what
  it generated is frozen at commit `44529028` (§9).
- **Known deviations** from Hugo are listed and classified (§7); none is silent.
- **Open:** real seeksnack (A-S, needs the private repository) and the migrate tool (T73), the
  remaining COULD features (T72: `:git` lastmod, Org front matter), server extras (§8).

## 1. What exists

```
Cargo.toml, Cargo.lock      the Cargo workspace at the repository root (26 crates), with
                            DEVELOPMENT.md, PROVENANCE.md, THIRD_PARTY/, deny.toml
crates/<name>/              one crate each; README.md per crate (API, state, accepted deviations)
sites/<site>/               the Tera layouts of the test sites (testsite, seeksnack, docs + patches)
testdata/                   Go-oracle fixtures (oracle/), corpora, golden Go-build data (golden/),
                            Hugo's test data (upstream/), the ratchet baselines (baselines/)
tools/neohugo/              harness: compare.sh, structdiff.py, manifest.py, selftest.py,
                            node.sh, licence-check.sh, notices.py, package.py,
                            changes/ (the ratchet's changes files)
tools/rust-port/i01/        sites.py (generates every test site), patches.json, site txtars
.github/workflows/ci.yml    CI and releases of neohugo
docs/rust-port/             this file, template-api.md (the template API, generated from
                            crates/funcs/src/spec.rs), REWRITE_PLAN.md, specs/ (research of the
                            old port, "byte-parity sections obsolete"), archive/ (the old port's
                            docs)
```

The old byte-for-byte port (the root `crates/` of `be02933a`, not today's crates; line-by-line
ports of Go packages) was deleted in T00; it is at that commit (local tag `go-parity-final`).
Its documents are in [`archive/`](archive/README.md). The Go implementation (Hugo's Go tree,
`tools/go-oracle`, `tools/neohugo/oracle.sh`) is at commit `44529028` (§9).

### Crate map

Dependencies point down the table (lower crates never depend on higher ones). Lines are
`src` + `tests` at T70.

| Crate | Package | Role | Lines |
|---|---|---|---|
| `base` | `neohugo-base` | shared vocabulary: `Value`/`Map`/`Params`, dates, paths and URLs, `IdVec`, diagnostics, inflection and title case, globs | 4.9k + 1.5k |
| `config` | `neohugo-config` | configuration pipeline (normalise, legacy keys, merge, per language, themes, `NEOHUGO_*`) and the typed `Config` | 6.5k + 3.1k |
| `vfs` | `neohugo-vfs` | mounts → one union view per component, walkers, the path parser (file → language, format, bundle kind, key) | 1.7k + 1.5k |
| `pageparser` | `neohugo-pageparser` | content files: front matter, summary divider, shortcode lexing and assembly | 1.7k + 0.8k |
| `locale` | `neohugo-locale` | ICU4X collation, plurals, numbers and dates; i18n bundles with the `{{ .Field }}` evaluator | 1.7k + 1.3k |
| `page` | `neohugo-page` | per-page rules: dates, permalinks, cascade matching, build options, menus in front matter | 2.5k + 1.8k |
| `site` | `neohugo-site` | capture and assembly into the `Model`: page tree, kinds, sections, taxonomies, translations, resources, data | 3.7k + 2.8k |
| `nav` | `neohugo-nav` | menus, pagination and pager URLs, related content, the alias plan | 1.6k + 2.2k |
| `markup` | `neohugo-markup` | Markdown through comrak behind an engine-neutral API, plus Hugo's passes (goldmark's pipe tables, heading IDs, attributes, deflists, alerts, passthrough, emoji, linkify, typographer, TOC, context markers) | 4.5k + 3.7k |
| `highlight` | `neohugo-highlight` | code highlighting: a port of Chroma v2.19.0 (its XML lexers converted to Rust data, its Go lexers ported, its regex-lexer engine on a port of the regexp2 dialect, its HTML formatter), Chroma class names or inline styles from Chroma's style files | 2.6k + 0.8k |
| `minify` | `neohugo-minify` | output minification (minify-html, lightningcss, oxc) | 1.5k + 1.3k |
| `images` | `neohugo-images` | image processing (resize, fit, fill, crop with Go's smart crop, filters, text, QR, dither, EXIF), the image cache | 6.3k + 3.3k |
| `jsbuild` | `neohugo-jsbuild` | `js_build` in process: rolldown with neohugo's plugin (assets-first resolution, `@params`, `inject`, CSS imports), TC39 decorators and the `es5` target | 9.6k + 5.8k |
| `resources` | `neohugo-resources` | the `ResourceStore`: assets, page resources, pipes (Sass via grass, PostCSS, Tailwind, Babel, `js_build`, minify, fingerprint, `execute_as_template`, `post_process`), `get_remote` with its cache | 5.0k + 3.2k |
| `publish` | `neohugo-publish` | sinks, canonify/absolute URLs, URL-token extraction, held outputs, `neohugo_stats.json`, static sync | 1.9k + 1.3k |
| `layouts` | `neohugo-layouts` | layout scan and lookup (Hugo's v0.146 names and scoring), embedded templates in Tera | 2.4k + 1.7k |
| `funcs` | `neohugo-funcs` | the template API (`spec.rs`, the single source of truth) and the pure functions (with `to_math`: KaTeX in QuickJS; `diagrams_goat`: the bep/goat port) | 4.9k + 1.2k |
| `view` | `neohugo-view` | the serialisable views templates read (`page`, `site`, `neohugo`, …) and their caches | 2.8k + 1.3k |
| `sitefuncs` | `neohugo-sitefuncs` | site-bound Tera functions (`get_page`, `ref`, `i18n`, resources, images, `paginate`, `partial`, `defer`, …) | 3.0k + 1.9k |
| `render` | `neohugo-render` | the render `Session`: content (shortcodes, hooks), layout jobs, waves | 2.7k + 1.3k |
| `build` | `neohugo-build` | build orchestration (phases B–E7 of REWRITE_PLAN.md §3; content adapters before the model), `BuildRequest`/`BuildReport` | 1.1k + 2.0k |
| `serve` | `neohugo-serve` | `neohugo server`: listeners, file serving, LiveReload, watching, rebuilds | 2.4k + 1.0k |
| `cli` | `neohugo` | the `neohugo` binary (clap); the gate tests live in its `tests/it` | 1.9k + 2.0k |
| `migrate` | `neohugo-migrate` | stub (T73: Go-template → Tera converter) | – |
| `testkit` | `neohugo-testkit` | dev-only: fixture readers, txtar sites, the template contract test | 0.7k + 0.7k |
| `workspace-hack` | `neohugo-workspace-hack` | feature unification of shared dependencies | – |

## 2. Build, test, run

Environment (every checkout and worktree shares one target directory):

```sh
export CARGO_TARGET_DIR=<main checkout>/target CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
cargo build --release --offline --locked -p neohugo      # → $CARGO_TARGET_DIR/release/neohugo
cargo test -p neohugo-<crate> --offline --locked          # the edit–test loop (cli: -p neohugo)
```

The full check CI runs (workspace-wide; not for the edit–test loop):

```sh
N=$PWD/tools/neohugo/node_modules                         # tools/neohugo/node.sh
NEOHUGO_NODE_MODULES=$N \
NEOHUGO_POSTCSS_BIN=$N/.bin/postcss NEOHUGO_TAILWINDCSS_BIN=$N/.bin/tailwindcss \
NEOHUGO_BABEL_BIN=$N/.bin/babel \
  cargo test --workspace --offline --locked               # includes the gate tests A-T, A-R, A-D2
cargo fmt --all --check
cargo clippy --workspace --all-targets --offline --locked -- -D warnings
tools/neohugo/licence-check.sh
python3 tools/neohugo/selftest.py                         # the harness's self-test
python3 tools/rust-port/i01/sites.py patches --check      # patches.json ↔ sites/docs/patches
```

A test whose external tool is missing prints `SKIPPED …` and passes (DEVELOPMENT.md lists the
tools); run with `-- --show-output` and grep for `SKIPPED` before trusting a green run. After
switching worktrees, `touch` the sources of the crates you test (shared target directory; see
DEVELOPMENT.md).

Running it (Hugo's flags in kebab-case, the camelCase spellings as aliases; the full table is
`crates/cli/README.md`):

```sh
neohugo [build] -s <site> [-d <out>] [--minify] [-b <url>] [-e <env>] [-D -E -F] [--clock <rfc3339>]
neohugo server -s <site> [-p 1313] [--bind 127.0.0.1] [--render-to-disk] [--disable-live-reload] [--poll 1s]
neohugo templates check -s <site> [--coverage summary|full|none] [--deny-warnings]
neohugo config -s <site> [--format json|toml]
neohugo version
```

`NEOHUGO_TIMINGS=1` prints the build's phase timings on stderr (T70).

**Configuration.** The project file is the first of `neohugo.{toml,yaml,yml,json}`,
`config.*` (a warning names the others when several exist; Hugo's `hugo.*` is not read), plus
`config/` dirs and `NEOHUGO_*` variables, merged with themes as Hugo does
(`crates/config/README.md`).

**`neohugo`, not `hugo`** (2026-10-01). Every name that was Hugo's is neohugo's, with no
fallback: the configuration file `neohugo.*` (no `hugo.*`), the stats file `neohugo_stats.json`
(`[build.buildStats]`; the legacy `[build] writeStats` is migrated), the default cache directory
`neohugo_cache`, the generator meta `neohugo <version>`, `@import "neohugo:vars"` in Sass,
`package.neohugo.json` among the JS config files, the reserved layouts directory `_neohugo/`
and the ids of `js_build`'s virtual modules (`neohugo:entry`, `\0neohugo-params`, …). The
tests replay the Go oracles with these names (`neohugo_testkit::fixture::neohugo_path`, the
harness's `sites.py as_neohugo_site`; the golden `project:hugo_stats.json` key reads
`neohugo_stats.json`). Templates read the build's version and environment as
`neohugo` (`neohugo.environment`, `neohugo.is_production`, `neohugo.is_development`,
`neohugo.is_server`, `neohugo.version`, `neohugo.generator`; `@neohugo` in components). Every
environment variable neohugo reads or sets is `NEOHUGO_*`: the environment
(`NEOHUGO_ENVIRONMENT`), the configuration overrides (`NEOHUGO_TITLE`, `NEOHUGO_PARAMS_X`,
`NEOHUGO_CACHEDIR`, …), the default `get_env` allowlist (`^NEOHUGO_`) and what external tools
get (`NEOHUGO_ENVIRONMENT`, `NEOHUGO_PUBLISHDIR`, `NEOHUGO_FILE_<NAME>`, so a site's
`postcss.config.js` reads `process.env.NEOHUGO_ENVIRONMENT`). There is no fallback: `hugo` is
not a template name and `HUGO_*` variables are neither read nor set. neohugo's own settings
(`NEOHUGO_NODE_MODULES`, `NEOHUGO_TIMINGS`, …; `neohugo_config::env::RESERVED`) are not
configuration overrides. Layouts must be Tera with Hugo's v0.146 names (`home.html`, `single.html`,
`_partials/`, `_shortcodes/`, `_markup/`); legacy names are errors with a hint. The template
API (every function, filter, test and context, with Hugo's name for each) is
`docs/rust-port/template-api.md`; `neohugo templates check` checks a site's templates against it.

## 3. Gates and the harness

The oracle is the golden data the Go build wrote to `testdata/golden/<label>/` (manifests
of a minified and an unminified build, the structure dump; `tools/neohugo/oracle.sh` with the Go
binaries), frozen at `44529028` (§9; `testdata/golden/README.md` has the recipe to
regenerate it in a worktree of that commit). Sites are generated outside the repository by
`tools/rust-port/i01/sites.py make <site> <dir> [--overlay sites/<site>] [--docs-patches
i01|reduced]`; the Rust side replaces the layouts with the overlay's Tera files.

`tools/neohugo/compare.sh <label> [--ref golden] [--task Txx] [--update] [--report-only]`
builds the Rust side (both passes, plus its structure dump), compares with `structdiff.py` at
the levels of REWRITE_PLAN.md §7.2 (L1 file set, L2 links and URLs, L3 visible text and
heading IDs, L4 assets, S the structure oracle, A7 the share of pages with equal text) and
checks the result against the **ratchet**: the baseline `testdata/baselines/<label>.json`
may change only through entries in `tools/neohugo/changes/<task>.md`, each with one class
(`engine-difference`, `bug-fixed`, `accepted-deviation`) and a reason; an unlisted difference
fails (`tools/neohugo/changes/README.md`).

| Gate | Site (label) | How it runs | State |
|---|---|---|---|
| A-T | testsite | `cargo test -p neohugo --test it parity` (`testsite_gate_a_t`; in-process comparison with the Go build's output `crates/build/tests/it/testsite-go.txtar` and the structure oracle); also `compare.sh testsite` | passed (T60, T03) |
| A-R | seeksnack (reconstruction) | `cargo test -p neohugo --test it reconstruction` (`gate_a_r`: `compare.sh seeksnack --ref golden`) | passed (T62) |
| A-D1 | docs-i01 | `tools/neohugo/compare.sh docs-i01` (not a committed test) | passed (T65) |
| A-D2 | docs-reduced | `cargo test -p neohugo --test it docs` (`gate_a_d2`: `compare.sh docs-reduced --ref golden`) | passed (T66; every page equal since T74) |
| A-D3 | docs-live | `cargo test -p neohugo --test it docs` (`gate_a_d3`: `compare.sh docs-live --ref golden`; the golden data is the published site, testdata/golden/README.md) | passed (T74) |
| A-DET | mini, edge trees | `cargo test -p neohugo-build --test it determinism` | passed (T36) |
| A-S | real seeksnack | needs the private repository (`tools/rust-port/prepare-site.sh`); the path set of `tools/rust-port/golden/canonical.sha256` after L1 normalisation | open (T73) |
| A-P | docs (release) | §5 | goals met (T70) |

The gate tests need python3, bash, node and the node tools (else `SKIPPED`).

## 4. CI/CD

`.github/workflows/ci.yml` is the repository's only build workflow (`stale.yml` manages issues).
It runs on pushes to `main` and `rust-port`, on every pull request (no path filters), on
`v[0-9]*` tags and by hand. Jobs: **Lint** (fmt, clippy `-D warnings`, licence check, structdiff
self-test, `sites.py patches --check`, tag = `v<workspace version>`), **Test** (Linux only, §8: the whole workspace with
every tool installed by `tools/neohugo/node.sh`; a test that
prints `SKIPPED` fails the job), **Build** (release for `x86_64`/`aarch64` Linux,
`x86_64`/`aarch64` macOS, `x86_64` Windows, with the commit, date and vendor of `neohugo
version`; `notices.py` writes `THIRD_PARTY_NOTICES.txt`, `package.py` the archive), **Release**
(tags only: the GitHub release `v<version>` with the five archives and
`neohugo_<version>_checksums.txt`; a version with a `-` makes a pre-release, any other is latest
only if no release has a higher version). Cutting a release: set `[workspace.package] version`,
merge, tag `v<version>`, push the tag (DEVELOPMENT.md "CI and releases").

## 5. Performance (A-P, T70)

Release build of the Rust `neohugo` (default features), the Go `neohugo` that `oracle.sh install`
built for T01 (`go build -trimpath -ldflags="-s -w"`; the Go tree and `oracle.sh` are at
`44529028`, §9).
Each run: a freshly generated site (`sites.py make`; Rust with its overlay), the environment of
`compare.sh` (clean env, `TZ=UTC`, HTTP disabled, the golden GetRemote cache, node modules on
`PATH`) but with each implementation's default parallelism (no `HUGO_NUMWORKERMULTIPLIER`),
`--clock 2026-09-27T12:00:00Z --minify -d <out>`. **Cold:** no `resources/`, empty cache
directory. **Warm:** the same site again, `resources/_gen` (image cache) and cache directory
kept, output removed. Wall time and the peak RSS of the neohugo process (`wait4`; external
tools such as Tailwind and, at the time, esbuild not included: `js_build` has run in process
since 2026-10-02 and was not measured again). Machine: 4 CPUs, 15 GB RAM, nothing else
running.

| Site | | Go median (min–max) | Rust median (min–max) | Rust / Go | Peak RSS Go / Rust |
|---|---|---|---|---|---|
| docs-reduced (888 files) | cold | 3.99 s (3.86–4.13) | 3.29 s (3.11–4.48) | **0.82** (goal ≤ 1.5) | 399 / 378 MB (max 384) |
| | warm | 3.51 s (3.47–4.81) | 3.26 s (3.09–4.51) | **0.93** (goal ≤ 1.0) | 290 / 359 MB (max 365) |
| testsite (55 files) | cold | 0.099 s (0.085–0.124) | 0.071 s (0.064–0.080) | 0.71 | 71 / 44 MB |
| | warm | 0.090 s (0.083–0.101) | 0.068 s (0.064–0.077) | 0.75 | 71 / 43 MB |
| seeksnack reconstruction (712 files) | cold | 2.40 s (2.37–2.46) | 0.62 s (0.60–0.71) | 0.26 | 115 / 74 MB |
| | warm | 0.61 s (0.60–0.68) | 0.57 s (0.50–0.61) | 0.93 | 108 / 64 MB |

Go n=5, Rust n=5 (docs n=6); the single slow outliers (≈ 4.5 s) of both sides were other
activity on the shared machine. Before T70's fix the Rust docs build took 3.83 s cold and
3.63 s warm (one run each), the testsite 0.50 s: every process linked the syntect syntax set
(≈ 0.45 s); `build.rs` now links it at compile time.

Where the Rust docs build spends its time (`NEOHUGO_TIMINGS=1`, warm, minified): model 40 ms,
templates 40 ms, content 630–800 ms (Markdown and highlighting of ~3,900 fences), wave 1
750–900 ms (888 layouts), deferred 1.4–1.6 s (Tailwind ≈ 0.55 s, then placeholder patching,
**HTML minification** and writing of every held page), resources 20–190 ms (images). Without
`--minify` the deferred phase is ≈ 0.7 s shorter; Go's minifier costs it ≈ 0.25 s. Proposals,
not done:
1. HTML minification: `neohugo-minify` runs minify-html twice on pages with comments or
   omitted end tags (for idempotence); fold the second pass into one (strip comments before,
   or check whether a second pass can change anything) and minify pages while wave 1 renders
   them when they hold no deferred placeholder.
2. Start Tailwind (the `defer` templates of `styles.css`) as soon as `neohugo_stats.json` is known
   instead of after the whole wave; it is an external process and could overlap page patching.
3. Warm builds gain little because image processing is already cheap (Go saves ≈ 0.5 s warm,
   Rust ≈ 0.15 s); the remaining cost is rendering, so wave-1 profiling (Tera value cloning of
   page views) is the next step if the warm ratio needs to drop further.

## 6. Provenance and licences

Every file not written for the rewrite has a row in `PROVENANCE.md` (source, version,
licence, verbatim/modified/rewritten/generated), and material cargo cannot see has its licence
in `THIRD_PARTY/` (Hugo, CLDR via ICU4X, emoji data, Chroma (lexers, styles) and regexp2,
KaTeX, GoAT, smartcrop, gift, goldmark, flect, prose, Go's JPEG writer and decoder IDCT, Go fonts,
x/image, rsc.io/qr, hashstructure, livereload-js, and the Lato font of A-D3's test data).
`tools/neohugo/licence-check.sh` checks every crate of the dependency graph against
`deny.toml`; `tools/neohugo/notices.py <target> <file>` writes the notices of the linked
crates for a release (a crate without a licence file gets the MIT or Apache-2.0 text when that
is one of its licences, anything else fails). No Zola code: Zola ≥ 0.22 (EUPL-1.2) was never
opened; no pre-0.22 MIT Zola file was copied either (D2 allowed it; none was needed).

T70 audit: 535 third-party packages pass the licence check, 438 linked packages have notices;
no Go-source, "Copyright" or Zola text in the sources besides the attributed ports listed in
PROVENANCE.md. Fixed: `THIRD_PARTY/emoji/` was listed but missing (added, with the Unicode
licence and gemoji's MIT licence; the latter written offline, to be compared with gemoji's
`LICENSE`); rows added for `sites/**`, `testsite-go.txtar` and `testdata/golden/**`.

## 7. Known deviations from Hugo

Decisions (REWRITE_PLAN.md §11 D4, D5): Tera-only templates with v0.146 names; content is
rendered before layouts (another page's content inside a shortcode goes through
`page_content`); explicit pagination, a conflicting second `paginate` is an error; lazily
published resources are published on reference; no `#ZgotmplZ`; YAML 1.2 (`yes` stays a
string); deterministic winners for URL collisions; newer CLDR collation; no `$_hugo_config`
v1 shortcodes; segment-aware prefix lookup; target-path assets: earlier language wins; site
functions inside components need `page=` or `@__nh`; `neohugo.version` is `0.149.0-DEV`; the
template object is `neohugo` and the environment variables are `NEOHUGO_*` (below).

Allowed output differences (REWRITE_PLAN.md §7.3): minifier bytes, highlight span structure,
typographic characters vs entities, CSS/JS bundle bytes, term-collision winners, the order of
equal-weight Thai titles, the `generator` meta.

Per site (the ratchet's changes files, `tools/neohugo/changes/`):

| Site | Entry | Class |
|---|---|---|
| seeksnack | `hugo_stats.json`: the classes of a lone `<th>` are recorded (Go's tokenizer drops them) | engine-difference (T62) |
| docs-i01, docs-reduced, docs-live | `hugo_stats.json`: `<?xml` and `<=` are not tags | engine-difference (T65, T66, T74) |

The earlier docs entries (a table on lazy list-item lines, the GoAT pages drawn by svgbob, the
math pages as MathML) are `bug-fixed` in T74: goldmark's table transformer, bep/goat and KaTeX
are ported.

Per crate (each README's "Accepted deviations" section, and `expected_diffs.toml` where the
tests read them, with counts):

- **base**: YAML `.inf`/`.nan` are strings; `BaseUrl` rejects non-UTF-8 paths, ports are
  `u16`; `humanize` of text without words returns the input; custom inflection errors are
  `Result`s; globs follow the documented semantics, not gobwas's optimiser bugs.
- **config**: no Hugo Modules download (themes from `themesDir`, `_vendor`, absolute paths);
  imaging, media types and output formats per project, not per language; `deployment`,
  `segments`, `httpCache`, `server` untyped; errors instead of silently ignored values; no
  mapstructure weak decoding.
- **pageparser**: a summary divider at the start of a page without front matter is a divider;
  JSON integers stay integers; YAML 1.2; TOML leap seconds stay strings; closing tags must name
  their shortcode; unclosed inline shortcodes are errors; Org front matter not decoded.
- **vfs**: normalised `original` names; on macOS NFC names in `rel` but the OS's name in
  `abs` (Hugo normalises both); byte-ordered walks.
- **page**: attribute expansion in place; Unix-second dates in UTC; bad cascade globs are
  errors; content adapters are Tera `_content.html` (`_content.gotmpl` is an error with a hint),
  an unknown adapter page `kind` is an error; one menu entry per menu name.
- **site**: segment-wise taxonomy prefixes; bundle files belong to their owner; `ref` from a
  bundled page resolves; lists use the page's language's collator; strict `.Resources` order;
  main-section ties by name.
- **markup** (comrak): pipe tables are goldmark's paragraph transformer (a pass, not comrak's
  extension); Hugo's context markers shape the blocks but the closing marker after an include
  that ends with a table is not a row of empty cells, a marker inside code is removed; a fence
  without hook or highlighter renders plain; 2 typographer cases on docs pages not reproduced;
  no CJK line-break handling; integer attribute values.
- **highlight** (the Chroma port): Raku renders as one text token (its Go lexer rewrites its
  rules while running; not ported); Chroma's panics become `Error` tokens; where Chroma loops
  forever (zero-width matches returning to a configuration it had at the same position, e.g.
  JSONata, Jungle) the port treats the position as unmatched; `\p{…}` knows Go's general
  categories (crates/highlight/README.md).
- **images**: pixels by PSNR, not bytes; JPEG with an exactly rounded DCT; own `_hu_` names;
  smart crops are Go's (muesli/smartcrop with Hugo's analysis resize); stricter spec grammar;
  paletted PNG written true-colour;
  animated GIF first frame; simplified EXIF values; text and dither by PSNR.
- **resources**: Sass by grass (dart-sass semantics); minified bytes of neohugo-minify; tools
  run in the project directory; `resources.Copy` conflicts are errors; own error texts.
- **publish**: canonify and stats-collector artefacts of Go's scanners not reproduced (counted
  in `expected_diffs.toml`); no generator injection; empty static dirs not copied.
- **layouts**: v0.146 names only (`index.*` refused); a user template does not beat a more
  specific theme template (Hugo's rule); base variants only for the selections loaded.
- **serve**: every change is a full rebuild (no fast render); no browser error page; `[server]`
  headers/redirects not read; per-language 404 pages; no TLS, `--openBrowser`, `--pprof`.
- **locale, nav**: `expected_diffs.toml` (CLDR 48 vs Go's x/text tables; pager and menu edge
  cases).

## 8. Open items

- **T73**: `neohugo-migrate` (Go-template → Tera converter, stub today) and gate **A-S** on the
  real seeksnack (needs the private repository attached; `tools/rust-port/prepare-site.sh`).
- **T72** (COULD): `:git` lastmod; Org front matter. (T74 did smartcrop, the Chroma style
  gallery and content adapters; the docs-live variant runs `images.Text`, `qr_code` and Dither
  unpatched. The i01/reduced patches stay as they are: they define A-D1 and A-D2.)
- **Server** (T71 leftovers): the browser error page, `[server]` headers and redirects, fast
  render (partial rebuilds), TLS, `--openBrowser`.
- **Performance** proposals of §5 (HTML minification, overlapping Tailwind); none is needed for
  the A-P goals.
- **A-D1** runs only through `compare.sh docs-i01`; a committed test like `gate_a_d2` would
  keep it green in CI.
- `neohugo-config`: `TocConfig::end_level` is `u8`, so `endLevel = -1` cannot be decoded
  (markup README "Plan issues").
- `THIRD_PARTY/emoji/LICENSE-GEMOJI` to be compared with gemoji's `LICENSE` (written
  offline).
- The real-site tests that read `NEOHUGO_SITES` are ignored by default.
- **Windows tests:** the Test job runs on Linux only, while the Go CI also ran its tests on
  `windows-latest` (`mage -v test`); Windows and macOS get only the release build and its smoke
  test. A Windows leg needs the Unix-only test code gated first: `use std::os::unix` in
  `crates/publish/tests/it/staticcopy.rs` and in the fake tools of
  `crates/resources/tests/it/pipes/` (§9).
- The follow-ups of the Go removal outside the repository (§9): branch protection, unused
  secrets, the channels frozen at the last Go build.

## 9. Without Go (2026-10-01)

Branch `go-removal` (from `44529028`) removed the Go implementation and its CI/CD; the
repository is the Rust implementation only. `44529028` is the last commit with the Go tree (no
tag): `git worktree add <dir> 44529028`, or `git show 44529028:<path>` for the Go-tree paths
that comments and READMEs cite.

- **Removed:** Hugo's Go packages, `main.go` with `main_test.go` and
  `main_withdeploy_test.go`, `go.mod`/`go.sum`, `magefile.go` (and `.vscode/`, its debug
  configuration), the release (GoReleaser, hugoreleaser with `hugoreleaser.env`,
  `merge-release.sh`), Docker, snap and golangci-lint configuration, `check_gofmt.sh`,
  `watchtestscripts.sh`, `testscripts/`, `scripts/`, `tools/go-oracle/`,
  `tools/neohugo/oracle.sh`, `tools/esbuild/build.sh`, the highlight oracle
  (`rust/crates/highlight/tests/data/oracle/` at `44529028`), and the Go workflows `ci.yml`
  (Go's; the current `ci.yml` is the renamed `rust.yml`, below; Go's ran its tests on
  `ubuntu-latest` and `windows-latest`, the current one tests on Linux only, §8),
  `release.yml`, `benchmark.yml`, `golangci-lint.yml` and `image.yml`. `.github/stale.yml`
  (the Probot stale bot's configuration, with other labels and periods than the
  `workflows/stale.yml` that manages issues) is not Go's and stays, for the maintainers to
  decide on. `pull-docs.sh` (a `git
  subtree pull` of `docs/` from neohugo/neohugoDocs) is gone too: `docs/` is a frozen test
  fixture now and is no longer pulled. `docs/go.mod`, `docs/go.sum` and `docs/hugo.work` stay:
  they belong to the docs site, which stays byte-identical (only `docs/rust-port/` changes).
- **Test data moved:** Hugo's test data the tests read is in `testdata/upstream/` at its
  Go-tree path (`hugolib/testsite`, `resources/testdata`, `resources/images/testdata`,
  `tpl/images/testdata`, `media/testdata/fake.png`; 90 files). Fixture ids keep the old paths;
  `neohugo_testkit::fixture::repo_file` resolves them (and `tools/rust-port/i01/sites.py` does
  the same for the testsite). The image oracles read five more Go-tree images and
  `snap/local/logo.png` from byte-identical copies (`crates/images/tests/it/common.rs`). The
  `NEOHUGO_GOROOT` hook for Go's own image test data is gone: the 80 files of it the image
  oracles read (Go 1.24.7's) are in `testdata/upstream/goroot/src/image/`, and the five Go
  1.24.7 does not have (four Go 1.27.1 JPEGs and `image/png`'s example gopher) are the old
  port's copies from `be02933a` in `testdata/upstream/old-port/`. So the process oracle
  compares all 13,250 cases (13,248 equal, two accepted corrupt-PNG differences; CI at
  `44529028` with Go 1.24.7 compared 13,089) and the EXIF oracle 2,440; both fail when a
  source is missing.
- **`js_build` in process (2026-10-02):** `js_build` bundles with rolldown 1.2.12 in process
  (crate `neohugo-jsbuild`, which replaced the esbuild `--service` client `neohugo-esbuild`), so
  nothing needs installing and the release archives are complete. rolldown and its oxc are
  exact pins (`rolldown*` `=1.2.12`, the `oxc` umbrella crate `=0.152.0`, beside the oxc 0.95
  minify-html locks), which raised the MSRV to 1.96 (CI 1.96.0); the `vendor/sauron-core` patch
  this first needed (svgbob's sauron 0.61 pinned futures `=0.3.30`, rolldown needs `^0.3.32`)
  went with svgbob, which the GoAT port replaced (T74). The plugin (`src/plugin.rs`) keeps js.Build's semantics on top of rolldown:
  assets-first resolution with shims and esbuild's external matching, `@params`, `inject`
  (side effects included), CSS imports (the CSS is discarded as before; `local-css` gives class
  names), TC39 decorators and the `es5` target (`src/lower/`), `process.env.NODE_ENV`,
  `require()` for externals in IIFE output, and named CommonJS exports. A build is a task on a
  tokio runtime and the caller blocks on a channel (`src/executor.rs`): never wait on one from
  a worker of rayon's global pool, which rolldown itself uses. The output is not esbuild's
  bytes, so the oracle tests compare behaviour: `crates/jsbuild/tests/it/{jsbuild,run}.rs` run
  Hugo's recorded bundle and neohugo's under node with recording stand-ins for the browser
  and compare the traces; errors keep Go's positions (and esbuild's wording for unresolved
  imports and `es5`). Fingerprints and `Data.Integrity` of bundles changed with the bytes.
  `tools/esbuild/` and the npm pin are gone; a checkout made before needs
  `tools/neohugo/node.sh` once (the lock file changed). rolldown also turns on serde_json's
  `preserve_order` and `arbitrary_precision` for the whole binary; the workspace-hack turns
  them on for every member and DEVELOPMENT.md ("Feature unification") has the two rules that
  follow (no reliance on `serde_json::Map` order; no `serde_json::Value` numbers through Tera,
  YAML or TOML serializers).
- **Frozen references:** what the Go implementation generated stays as committed: the oracle
  fixtures (`testdata/oracle/`), the golden data (`testdata/golden/`),
  `crates/build/tests/it/testsite-go.txtar`, the highlight fixtures
  (`crates/highlight/tests/data/`) and lexer table
  (`crates/highlight/src/data/chroma-lexers.tsv`), `crates/funcs/tests/fixtures/remarshal/go.txt`
  and `docs/data/docs.yaml`; so do the Go outputs the old port recorded at `be02933a`, such as
  `testdata/corpus/minify/*.tsv` (PROVENANCE.md). To regenerate, run the old recipe in a
  worktree of `44529028` and copy the result back (`testdata/golden/README.md`,
  `crates/highlight/README.md`, `tools/neohugo/fixtures2json.py`). `compare.sh` takes only
  `--ref golden`; `selftest.py` perturbs the Go testsite output of `testsite-go.txtar`.
- **Workspace at the root:** the Cargo workspace moved from `rust/` to the repository root
  (`Cargo.toml`, `crates/`, `sites/`, `testdata/`, `THIRD_PARTY/`, `PROVENANCE.md`; build output
  in `target/`); `rust/README.md` became `DEVELOPMENT.md`, `rust/docs/template-api.md` this
  directory's `template-api.md`, and the workflow `rust.yml` became `ci.yml` (name `CI`). Ids
  recorded below `rust/` (the sources of `testdata/golden/images/manifest.json`) resolve at the
  root through `repo_file` (`neohugo_testkit::fixture`, `sites.py`).
- **Releases:** tags `v<version>` instead of `rust-v<version>`; the CI workflow publishes the
  GitHub release (a pre-release if the version has a `-`, else latest only if no release has a
  higher version). The tags `v0.148.2` and older are the Go releases.
- **Drop-in names:** the binary is `neohugo` again (it was `neohugo-rs`); `neohugo version`
  prints the Go line,
  `neohugo v<version>[-<commit>] <os>/<arch> BuildDate=<date|unknown>[ VendorInfo=<vendor>]`
  (`crates/cli/src/version.rs`; CI sets the commit, its UTC date and `VendorInfo=neohugo` as
  the Go releases' build did); the archives are named as goreleaser named them,
  `neohugo_<version>_<os>-<arch>.tar.gz` (`.zip` for Windows), with the binary, `README.md` and
  `LICENSE` at the root plus `THIRD_PARTY_NOTICES.txt`, `PROVENANCE.md` and `THIRD_PARTY/`,
  next to `neohugo_<version>_checksums.txt`. `compare.sh` takes the binary from
  `NEOHUGO_BINARY` (was `NEOHUGO_RS`). Not reproduced from the Go release: goreleaser's
  changelog in the release notes and its `v<version>` release title (the title is now
  `neohugo <version>`); the Go commands the Rust command line does not have (`env`, which also
  printed the version line, `new`, `mod`, `deploy`, `list`, `gen`, `convert`, `import`,
  `release`, `server trust`, `config mounts`, cobra's `completion` and `help`), Hugo's flags it
  does not list (`crates/cli/README.md`; e.g. `--enableGitInfo`, `--contentDir`,
  `--disableKinds`, `--panicOnWarning`, the build's `-w`/`--watch`; the logging and
  housekeeping flags `--gc`, `--logLevel`, `--noBuildLock`, `--printI18nWarnings`,
  `--printPathWarnings`, `--printUnusedTemplates`, `--templateMetrics` and
  `--templateMetricsHints` are accepted, with a warning for those neohugo does not act on, and
  `--noTimes`/`--noChmod` work), the "Start building sites …" banner with the version line, and
  Go's per-language statistics table after a build (Pages, Paginator pages, Non-page files,
  Static files, Processed images, Aliases, Cleaned): neohugo prints one line, `pages N | files N
  (aliases N) | resources N | processed images N | static files N`, then `Total in N ms`;
  `config`'s output: Go printed one language's configuration with Hugo's keys lower-cased
  (`baseurl`, `publishdir`, …), as TOML by default or YAML/JSON, with `--lang` and
  `--printZero`; the Rust `config` prints neohugo's resolved configuration model (snake_case
  fields, one entry per site under `sites`, the merged user keys lower-cased under `raw`) as JSON
  or TOML, so a script that reads its output must change; `server`
  rendering to disk by default: the Go server wrote the publish directory and served it
  (`-M`/`--renderToMemory`: memory), the Rust one renders into memory unless `--render-to-disk`
  (a neohugo flag, not the Go build's) is given, and `-d` needs that flag; the commit and date
  of a local build (Go read them from git, a cargo build gets them only from CI's variables);
  the exit code 1 of usage errors (they exit with 2) and Go's `Error: …` prefix (clap prints
  `error: …`, a failed build `ERROR …`). As in Go, flags may come before the command and every
  command takes the persistent flags (`-s`, `-d`, `-e`, `--config`, `--configDir`, `--themesDir`,
  `--clock`, `--quiet`, `-M`, `--logLevel`, `--noBuildLock`; `crates/cli/README.md`). New: a `--version` flag, printing the
  `version` line. The Docker images and the docs deploy are below; `snap/snapcraft.yaml` and
  `hugoreleaser.yaml` were in the tree at `44529028`, but no workflow published them.

Follow-ups outside the repository:

- **Branch protection:** required status checks that name jobs of the removed workflows (CI's
  `Build (ubuntu-latest, Go 1.25)`, Golangci-lint, Release, Benchmark, Docker image) would block
  every pull request; require the CI workflow's `Lint`, `Test` and `Build (<target>)` instead.
- **Secrets** no workflow reads any more: `DOCKERHUB_USERNAME`, `DOCKERHUB_TOKEN`, `CR_PAT`
  (`image.yml`), `NEOHUGO_GITHUB_TOKEN`, `NEOHUGO_EMAIL` (the docs deploy of `release.yml`).
- **Public channels frozen at the last Go build:** the Docker images `neohugo/neohugo` and
  `ghcr.io/neohugo/neohugo`; the documentation site neohugo.github.io (deployed on `v*` tags by
  `release.yml`); and `/releases/latest`, which stays at the Go `v0.148.2` until the first Rust
  release that is not a pre-release (`v0.149.0`).

## 10. History

- 2026-09-27: the first session's byte-for-byte port (`archive/HANDOFF-old-port.md`).
- 2026-09-29: REWRITE_PLAN.md revision 2; T00 deleted the old port.
- T00–T66: the rewrite, the harness and the gates (REWRITE_PLAN.md §8.2).
- T70 (2026-09-30): this handoff, A-P, the licence and provenance audit, archive of the old
  port's documents, `hugo.is_server`/`site.server_port`.
- 2026-10-01: the Go implementation and its CI/CD removed, the Cargo workspace moved from `rust/`
  to the repository root, the binary, its version line and the release archives named as the Go
  releases named them (§9). Then: `to_css` resolves explicit-extension imports (`@import
  "x.scss"`) through the load paths as dart-sass does; the template object `hugo` and the
  `HUGO_*` environment variables renamed to `neohugo` and `NEOHUGO_*`, without fallback, and
  every other Hugo name (config file, stats file, cache dir, generator, …) made neohugo's (§2);
  the real seeksnack site ported to Tera and served (gate A-S's site, outside the repository).
  Engine fixes found there: `[build] writeStats` was not migrated (no stats file, so PostCSS
  purged the whole stylesheet); CSS minification no longer runs lightningcss's merging pass,
  which dropped fallback declarations (`background: radial-gradient(…); background:
  -webkit-radial-gradient(…)`); `<title>` whitespace is collapsed as Go's minifier does. CSS
  browser targets switched on: a project browserslist configuration (`.browserslistrc`,
  `browserslist`, `package.json` key; `crates/minify/src/targets.rs`, browserslist-rs) gives
  lightningcss targets, so minified CSS gets the prefixes and lowering autoprefixer would add;
  rules declaring a property twice are rescued as written. `execute_as_template` passes `data`
  to the template as the Tera value it is (the `TemplateExecutor` holds the context) instead of
  converting it to JSON and back on every call: seeksnack's development head, which executes
  its stylesheet with `data=page` on every page, built in 12.4 s with 3.1 GB, now 1.8 s with
  220 MB. Held outputs (pages waiting for `post_process` / `defer` placeholders) are written to
  the sink unpatched and patched there in E5 (`Sink::read`), as Hugo's post-processing does,
  instead of being kept in memory: seeksnack's production build, where the inlined
  post-processed stylesheet holds every page, went from 386 MB to 234 MB, output unchanged.
  `purge_css` (neohugo's own, no Hugo equivalent; `crates/minify/src/purge.rs`): a CSS
  resource cut down per page to the rules that page uses, PurgeCSS-style options (`safelist`,
  `greedy`, `blocklist`, `content`, `variables`, `important`), compiled once (each selector and
  declaration printed for the browserslist targets) and resolved by the publisher from the
  page's own HTML; no stats file, no PostCSS, no held pages. A pending `fingerprint` is
  computed at the call unless its chain waits for E5 (PostCSS, Tailwind, images). seeksnack
  now uses `to_css | minify | purge_css` (PostCSS, PurgeCSS, cssnano and `writeStats` removed):
  median inline CSS 54 KB → 38 KB, all HTML −17 % (−14 % gzipped), screenshots of six page
  types identical except the carousel indicators, whose attribute-selector rule PurgeCSS had
  dropped.
- 2026-10-02 (T74): the documentation site built by neohugo matches the published
  https://neohugo.github.io/ (the Go build of 2025-10-13) on every page: gate A-D3
  (`gate_a_d3`, `compare.sh docs-live`) compares `docs/` without patches (`sites.py make
  docs-live`) with the published files (`testdata/golden/docs-live/`, neohugo.github.io at
  a1928152), at that build's clock, with that day's GetRemote responses
  (`tools/rust-port/testdata/hugo_cache/docs-live/`): L1 2373/2373, L2 776/776, L4 873/873,
  A7 1.0 (the one L3 difference is Go's stats tokenizer reading `<?xml`/`<=` as tags), every
  highlighted code block and the style gallery byte-identical. `tools/docs/build.sh [--serve]`
  builds (or serves) the site with the docs' node modules (`tools/docs/package-lock.json`, the
  versions the published JS bundles were built with). Engine work: `get_remote` sniffs media
  types with Go's whole table; content adapters as Tera `_content.html` (`add_page`,
  `add_resource`, `enable_all_languages`; the news adapter ported); `to_math` runs KaTeX 0.16.22
  with mhchem in QuickJS (rquickjs) as Hugo does (pulldown-latex removed); `diagrams_goat` ports
  bep/goat v0.5.0 (svgbob removed); the `smart` anchor ports muesli/smartcrop with Hugo's
  analysis resize; pipe tables are goldmark's paragraph transformer and `{{% %}}` output is
  indented as Hugo does; highlighting ports Chroma v2.19.0 (syntect and two-face removed).
  A-D1 and A-D2 reach A7 1.0 as well (their GoAT, math and table entries `bug-fixed`).
