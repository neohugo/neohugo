# neohugo Rust rewrite: handoff (start here)

State as of T70 (2026-09-30) and the removal of the Go implementation (2026-10-01, §9), branch
`rust-port`. The plan and its task table are [`REWRITE_PLAN.md`](REWRITE_PLAN.md) (§8.2 holds
every task's state); the workspace's own rules and CI details are
[`rust/README.md`](../../rust/README.md). This document says what exists, how
to build, test and run it, how the gates work, what deviates from Hugo, and what is open.

## 0. Summary for the pull request

neohugo-rs is an idiomatic Rust rewrite of neohugo (a Hugo fork) in `rust/`: Hugo's site and
page model (content tree, bundles, kinds, front matter, cascade, permalinks, output formats,
taxonomies, menus, pagination, i18n, Hugo Pipes, image processing, Markdown with render hooks
and shortcodes) with **Tera 2** templates instead of Go templates (decision D4). It is not a
byte-for-byte port: the Go build was the oracle (its outputs are frozen as golden data, §9), and
outputs are compared structurally.

- **Gates passed** (REWRITE_PLAN.md §7.3; each against the Go neohugo's build of the same site):
  - A-T, the testsite: L1 56/56, L2 and L3 equal on every page, structure oracle equal.
  - A-R, the seeksnack reconstruction: L1 713/713, L2 427/427, structure 673/673, A7 1.0.
  - A-D1, the Hugo docs with the `i01` patches: L1 888/888, L2 756/756, L3 748/750, A7 0.9987.
  - A-D2, the Hugo docs with the `reduced` patches (Chroma-class highlighting, GoAT diagrams,
    emoji, math, Tailwind, the real Alpine/Turbo `js_build`): L1 889/889, L2 757/757, L3
    743/751, A7 0.9907.
  - A-DET: identical output with 1 and 8 threads and across runs.
- **A-P, performance** (T70; release build, 4 CPUs, medians of 5–6 runs): the docs site builds
  cold in 3.29 s against Go's 3.99 s (0.82×; goal ≤ 1.5×), warm in 3.26 s against 3.51 s
  (0.93×; goal ≤ 1.0×), peak RSS 384 MB (goal ≤ 1 GB). The testsite and the seeksnack
  reconstruction are faster than Go as well (§5).
- **Commands:** `neohugo-rs build` (also with no command), `server` (live reload, memory or
  disk), `templates check`, `config`, `version`.
- **CI/CD:** `.github/workflows/rust.yml`, the only build workflow (fmt, clippy `-D warnings`,
  licence check, the whole test suite with the gate tests, release builds for five targets);
  tags `v<version>` publish a GitHub release with archives, licence notices and checksums.
- **Go removed** (2026-10-01): the Go implementation, its tools and its workflows are gone; what
  it generated is frozen at commit `44529028` (§9).
- **Known deviations** from Hugo are listed and classified (§7); none is silent.
- **Open:** real seeksnack (A-S, needs the private repository) and the migrate tool (T73), the
  remaining COULD features (T72), server extras (§8).

## 1. What exists

```
rust/                       the Cargo workspace (26 crates), PROVENANCE.md, THIRD_PARTY/, deny.toml
rust/crates/<name>/         one crate each; README.md per crate (API, state, accepted deviations)
rust/sites/<site>/          the Tera layouts of the test sites (testsite, seeksnack, docs + patches)
rust/testdata/              Go-oracle fixtures (oracle/), corpora, golden Go-build data (golden/),
                            Hugo's test data (upstream/), the ratchet baselines (baselines/)
rust/docs/template-api.md   the template API, generated from crates/funcs/src/spec.rs
tools/neohugo/              harness: compare.sh, structdiff.py, manifest.py, selftest.py,
                            node.sh, licence-check.sh, notices.py, package.py,
                            changes/ (the ratchet's changes files)
tools/esbuild/install.sh    esbuild for js_build (the binary of the npm package node.sh installs)
tools/rust-port/i01/        sites.py (generates every test site), patches.json, site txtars
.github/workflows/rust.yml  CI and releases of neohugo-rs
docs/rust-port/             this file, REWRITE_PLAN.md, specs/ (research of the old port,
                            "byte-parity sections obsolete"), archive/ (the old port's docs)
```

The old byte-for-byte port (`crates/`, line-by-line ports of Go packages) was deleted in T00;
it is at commit `be02933a` (local tag `go-parity-final`). Its documents are in
[`archive/`](archive/README.md). The Go implementation (Hugo's Go tree, `tools/go-oracle`,
`tools/neohugo/oracle.sh`) is at commit `44529028` (§9).

### Crate map

Dependencies point down the table (lower crates never depend on higher ones). Lines are
`src` + `tests` at T70.

| Crate | Package | Role | Lines |
|---|---|---|---|
| `base` | `neohugo-base` | shared vocabulary: `Value`/`Map`/`Params`, dates, paths and URLs, `IdVec`, diagnostics, inflection and title case, globs | 4.9k + 1.5k |
| `config` | `neohugo-config` | configuration pipeline (normalise, legacy keys, merge, per language, themes, `HUGO_*`) and the typed `Config` | 6.5k + 3.1k |
| `vfs` | `neohugo-vfs` | mounts → one union view per component, walkers, the path parser (file → language, format, bundle kind, key) | 1.7k + 1.5k |
| `pageparser` | `neohugo-pageparser` | content files: front matter, summary divider, shortcode lexing and assembly | 1.7k + 0.8k |
| `locale` | `neohugo-locale` | ICU4X collation, plurals, numbers and dates; i18n bundles with the `{{ .Field }}` evaluator | 1.7k + 1.3k |
| `page` | `neohugo-page` | per-page rules: dates, permalinks, cascade matching, build options, menus in front matter | 2.5k + 1.8k |
| `site` | `neohugo-site` | capture and assembly into the `Model`: page tree, kinds, sections, taxonomies, translations, resources, data | 3.7k + 2.8k |
| `nav` | `neohugo-nav` | menus, pagination and pager URLs, related content, the alias plan | 1.6k + 2.2k |
| `markup` | `neohugo-markup` | Markdown through comrak behind an engine-neutral API, plus Hugo's passes (heading IDs, attributes, deflists, alerts, passthrough, emoji, linkify, typographer, TOC) | 4.5k + 3.7k |
| `highlight` | `neohugo-highlight` | code highlighting with syntect + two-face, emitting Chroma class names or inline styles; Chroma's style files | 2.6k + 0.8k |
| `minify` | `neohugo-minify` | output minification (minify-html, lightningcss, oxc) | 1.5k + 1.3k |
| `images` | `neohugo-images` | image processing (resize, fit, fill, crop, filters, text, QR, dither, EXIF), the image cache | 6.3k + 3.3k |
| `esbuild` | `neohugo-esbuild` | an esbuild `--service` client (`js_build`) | 2.5k + 1.0k |
| `resources` | `neohugo-resources` | the `ResourceStore`: assets, page resources, pipes (Sass via grass, PostCSS, Tailwind, Babel, `js_build`, minify, fingerprint, `execute_as_template`, `post_process`), `get_remote` with its cache | 5.0k + 3.2k |
| `publish` | `neohugo-publish` | sinks, canonify/absolute URLs, URL-token extraction, held outputs, `hugo_stats.json`, static sync | 1.9k + 1.3k |
| `layouts` | `neohugo-layouts` | layout scan and lookup (Hugo's v0.146 names and scoring), embedded templates in Tera | 2.4k + 1.7k |
| `funcs` | `neohugo-funcs` | the template API (`spec.rs`, the single source of truth) and the pure functions | 4.9k + 1.2k |
| `view` | `neohugo-view` | the serialisable views templates read (`page`, `site`, `hugo`, …) and their caches | 2.8k + 1.3k |
| `sitefuncs` | `neohugo-sitefuncs` | site-bound Tera functions (`get_page`, `ref`, `i18n`, resources, images, `paginate`, `partial`, `defer`, …) | 3.0k + 1.9k |
| `render` | `neohugo-render` | the render `Session`: content (shortcodes, hooks), layout jobs, waves | 2.7k + 1.3k |
| `build` | `neohugo-build` | build orchestration (phases B–E7 of REWRITE_PLAN.md §3), `BuildRequest`/`BuildReport` | 1.1k + 2.0k |
| `serve` | `neohugo-serve` | `neohugo-rs server`: listeners, file serving, LiveReload, watching, rebuilds | 2.4k + 1.0k |
| `cli` | `neohugo` | the `neohugo-rs` binary (clap); the gate tests live in its `tests/it` | 1.9k + 2.0k |
| `migrate` | `neohugo-migrate` | stub (T73: Go-template → Tera converter) | – |
| `testkit` | `neohugo-testkit` | dev-only: fixture readers, txtar sites, the template contract test | 0.7k + 0.7k |
| `workspace-hack` | `neohugo-workspace-hack` | feature unification of shared dependencies | – |

## 2. Build, test, run

Environment (every checkout and worktree shares one target directory):

```sh
cd rust
export CARGO_TARGET_DIR=/home/user/neohugo/rust/target CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
cargo build --release --offline --locked -p neohugo      # → $CARGO_TARGET_DIR/release/neohugo-rs
cargo test -p <crate> --offline --locked                  # the edit–test loop
```

The full check CI runs (workspace-wide; not for the edit–test loop):

```sh
N=/home/user/neohugo/tools/neohugo/node_modules            # tools/neohugo/node.sh && tools/esbuild/install.sh
NEOHUGO_ESBUILD_BINARY=/home/user/neohugo/tools/esbuild/bin/esbuild NEOHUGO_NODE_MODULES=$N \
NEOHUGO_POSTCSS_BIN=$N/.bin/postcss NEOHUGO_TAILWINDCSS_BIN=$N/.bin/tailwindcss \
NEOHUGO_BABEL_BIN=$N/.bin/babel \
  cargo test --workspace --offline --locked               # includes the gate tests A-T, A-R, A-D2
cargo fmt --all --check
cargo clippy --workspace --all-targets --offline --locked -- -D warnings
../tools/neohugo/licence-check.sh
python3 ../tools/neohugo/selftest.py                      # the harness's self-test
python3 ../tools/rust-port/i01/sites.py patches --check   # patches.json ↔ rust/sites/docs/patches
```

A test whose external tool is missing prints `SKIPPED …` and passes (rust/README.md lists the
tools); run with `-- --show-output` and grep for `SKIPPED` before trusting a green run. After
switching worktrees, `touch` the sources of the crates you test (shared target directory; see
rust/README.md).

Running it (Hugo's flags in kebab-case, the camelCase spellings as aliases; the full table is
`rust/crates/cli/README.md`):

```sh
neohugo-rs [build] -s <site> [-d <out>] [--minify] [-b <url>] [-e <env>] [-D -E -F] [--clock <rfc3339>]
neohugo-rs server -s <site> [-p 1313] [--bind 127.0.0.1] [--render-to-disk] [--disable-live-reload] [--poll 1s]
neohugo-rs templates check -s <site> [--coverage summary|full|none] [--deny-warnings]
neohugo-rs config -s <site> [--format json|toml]
neohugo-rs version
```

`NEOHUGO_TIMINGS=1` prints the build's phase timings on stderr (T70).

**Configuration.** The project file is the first of `neohugo.{toml,yaml,yml,json}`,
`hugo.*`, `config.*` (a warning names the others when several exist), plus `config/` dirs and
`HUGO_*` variables, merged with themes as Hugo does (`rust/crates/config/README.md`). A
`neohugo.toml` next to a `hugo.toml` wins, so a site can carry both a Go-Hugo and a neohugo
configuration. Layouts must be Tera with Hugo's v0.146 names (`home.html`, `single.html`,
`_partials/`, `_shortcodes/`, `_markup/`); legacy names are errors with a hint. The template
API (every function, filter, test and context, with Hugo's name for each) is
`rust/docs/template-api.md`; `neohugo-rs templates check` checks a site's templates against it.

## 3. Gates and the harness

The oracle is the golden data the Go build wrote to `rust/testdata/golden/<label>/` (manifests
of a minified and an unminified build, the structure dump; `tools/neohugo/oracle.sh` with the Go
binaries), frozen at `44529028` (§9; `rust/testdata/golden/README.md` has the recipe to
regenerate it in a worktree of that commit). Sites are generated outside the repository by
`tools/rust-port/i01/sites.py make <site> <dir> [--overlay rust/sites/<site>] [--docs-patches
i01|reduced]`; the Rust side replaces the layouts with the overlay's Tera files.

`tools/neohugo/compare.sh <label> [--ref golden] [--task Txx] [--update] [--report-only]`
builds the Rust side (both passes, plus its structure dump), compares with `structdiff.py` at
the levels of REWRITE_PLAN.md §7.2 (L1 file set, L2 links and URLs, L3 visible text and
heading IDs, L4 assets, S the structure oracle, A7 the share of pages with equal text) and
checks the result against the **ratchet**: the baseline `rust/testdata/baselines/<label>.json`
may change only through entries in `tools/neohugo/changes/<task>.md`, each with one class
(`engine-difference`, `bug-fixed`, `accepted-deviation`) and a reason; an unlisted difference
fails (`tools/neohugo/changes/README.md`).

| Gate | Site (label) | How it runs | State |
|---|---|---|---|
| A-T | testsite | `cargo test -p neohugo --test it parity` (`testsite_gate_a_t`; in-process comparison with the Go build's output `crates/build/tests/it/testsite-go.txtar` and the structure oracle); also `compare.sh testsite` | passed (T60, T03) |
| A-R | seeksnack (reconstruction) | `cargo test -p neohugo --test it reconstruction` (`gate_a_r`: `compare.sh seeksnack --ref golden`) | passed (T62) |
| A-D1 | docs-i01 | `tools/neohugo/compare.sh docs-i01` (not a committed test) | passed (T65) |
| A-D2 | docs-reduced | `cargo test -p neohugo --test it docs` (`gate_a_d2`: `compare.sh docs-reduced --ref golden`) | passed (T66) |
| A-DET | mini, edge trees | `cargo test -p neohugo-build --test it determinism` | passed (T36) |
| A-S | real seeksnack | needs the private repository (`tools/rust-port/prepare-site.sh`); the path set of `tools/rust-port/golden/canonical.sha256` after L1 normalisation | open (T73) |
| A-P | docs (release) | §5 | goals met (T70) |

The gate tests need python3, bash, node, the node tools and esbuild (else `SKIPPED`).

## 4. CI/CD

`.github/workflows/rust.yml` is the repository's only build workflow (`stale.yml` manages
issues). It runs on pushes to `main` and `rust-port`, on every pull request (no path filters),
on `v[0-9]*` tags and by hand. Jobs: **Lint** (fmt, clippy `-D warnings`, licence check,
structdiff self-test, tag = `v<workspace version>`), **Test** (the whole workspace with every
tool installed: `tools/neohugo/node.sh`, then `tools/esbuild/install.sh`; the summary lists
`SKIPPED` tests), **Build** (release for `x86_64`/`aarch64` Linux, `x86_64`/`aarch64` macOS,
`x86_64` Windows; `notices.py` writes `THIRD_PARTY_NOTICES.txt`, `package.py` the archive),
**Release** (tags only: the GitHub release `v<version>`, marked latest unless the version has a
`-`, which makes a pre-release). Cutting a release: set `[workspace.package] version`, merge,
tag `v<version>`, push the tag (rust/README.md "CI and releases").

## 5. Performance (A-P, T70)

Release build of `neohugo-rs` (default features), the Go `neohugo` that `oracle.sh install`
built for T01 (`go build -trimpath -ldflags="-s -w"`; the Go tree and `oracle.sh` are at
`44529028`, §9).
Each run: a freshly generated site (`sites.py make`; Rust with its overlay), the environment of
`compare.sh` (clean env, `TZ=UTC`, HTTP disabled, the golden GetRemote cache, node modules on
`PATH`) but with each implementation's default parallelism (no `HUGO_NUMWORKERMULTIPLIER`),
`--clock 2026-09-27T12:00:00Z --minify -d <out>`. **Cold:** no `resources/`, empty cache
directory. **Warm:** the same site again, `resources/_gen` (image cache) and cache directory
kept, output removed. Wall time and the peak RSS of the neohugo process (`wait4`; external
tools such as Tailwind and esbuild not included). Machine: 4 CPUs, 15 GB RAM, nothing else
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
2. Start Tailwind (the `defer` templates of `styles.css`) as soon as `hugo_stats.json` is known
   instead of after the whole wave; it is an external process and could overlap page patching.
3. Warm builds gain little because image processing is already cheap (Go saves ≈ 0.5 s warm,
   Rust ≈ 0.15 s); the remaining cost is rendering, so wave-1 profiling (Tera value cloning of
   page views) is the next step if the warm ratio needs to drop further.

## 6. Provenance and licences

Every file not written for the rewrite has a row in `rust/PROVENANCE.md` (source, version,
licence, verbatim/modified/rewritten/generated), and material cargo cannot see has its licence
in `rust/THIRD_PARTY/` (Hugo, CLDR via ICU4X, emoji data, two-face's syntaxes, Chroma styles,
flect, prose, Go's JPEG writer, Go fonts, x/image, rsc.io/qr, hashstructure, livereload-js).
`tools/neohugo/licence-check.sh` checks every crate of the dependency graph against
`rust/deny.toml`; `tools/neohugo/notices.py <target> <file>` writes the notices of the linked
crates for a release (a crate without a licence file gets the MIT or Apache-2.0 text when that
is one of its licences, anything else fails). No Zola code: Zola ≥ 0.22 (EUPL-1.2) was never
opened; no pre-0.22 MIT Zola file was copied either (D2 allowed it; none was needed).

T70 audit: 535 third-party packages pass the licence check, 438 linked packages have notices;
no Go-source, "Copyright" or Zola text in the sources besides the attributed ports listed in
PROVENANCE.md. Fixed: `THIRD_PARTY/emoji/` was listed but missing (added, with the Unicode
licence and gemoji's MIT licence; the latter written offline, to be compared with gemoji's
`LICENSE`); rows added for `rust/sites/**`, `testsite-go.txtar` and `rust/testdata/golden/**`.

## 7. Known deviations from Hugo

Decisions (REWRITE_PLAN.md §11 D4, D5): Tera-only templates with v0.146 names; content is
rendered before layouts (another page's content inside a shortcode goes through
`page_content`); explicit pagination, a conflicting second `paginate` is an error; lazily
published resources are published on reference; no `#ZgotmplZ`; YAML 1.2 (`yes` stays a
string); deterministic winners for URL collisions; newer CLDR collation; no `$_hugo_config`
v1 shortcodes; segment-aware prefix lookup; target-path assets: earlier language wins; site
functions inside components need `page=` or `@__nh`; `hugo.version` is `0.149.0-DEV`.

Allowed output differences (REWRITE_PLAN.md §7.3): minifier bytes, highlight span structure,
typographic characters vs entities, CSS/JS bundle bytes, term-collision winners, the order of
equal-weight Thai titles, the `generator` meta, KaTeX HTML → MathML, GoAT SVG bytes.

Per site (the ratchet's changes files, `tools/neohugo/changes/`):

| Site | Entry | Class |
|---|---|---|
| seeksnack | `hugo_stats.json`: the classes of a lone `<th>` are recorded (Go's tokenizer drops them) | engine-difference (T62) |
| docs-i01, docs-reduced | `host-on-21yunbox`: a pipe table on lazy list-item continuation lines stays text (comrak, like cmark-gfm) | engine-difference (T65, T66) |
| docs-i01, docs-reduced | `hugo_stats.json`: `<?xml` and `<=` are not tags | engine-difference / accepted-deviation (T65, T66) |
| docs-reduced | 3 GoAT pages: svgbob draws words, Go one `<text>` per character | accepted-deviation (T66) |
| docs-reduced | 3 math pages: MathML instead of KaTeX HTML; mhchem `\ce`/`\pu` are `<merror>` | accepted-deviation (T66) |

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
- **vfs**: normalised `original` names; no NFC normalisation (Linux); byte-ordered walks.
- **page**: attribute expansion in place; Unix-second dates in UTC; bad cascade globs are
  errors; Markdown and HTML content only (no content adapters); one menu entry per menu name.
- **site**: segment-wise taxonomy prefixes; bundle files belong to their owner; `ref` from a
  bundled page resolves; lists use the page's language's collator; strict `.Resources` order;
  main-section ties by name.
- **markup** (comrak): no textual context markers (spans instead); a fence without hook or
  highlighter renders plain; 3 goldmark structural quirks and 2 typographer cases on docs pages
  not reproduced; no CJK line-break handling; integer attribute values.
- **highlight** (syntect): TextMate grammars instead of Chroma lexers (span structure differs,
  some languages plain text); Chroma's own style files; attributes in key order;
  `guessSyntax` by first line.
- **images**: pixels by PSNR, not bytes; JPEG with an exactly rounded DCT; own `_hu_` names;
  smart crop sizes exact but centred; stricter spec grammar; paletted PNG written true-colour;
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
- **T72** (COULD): the docs patches for `images.Text`, `qr_code` and Dither are not lifted yet
  although T72a implemented them; smartcrop; the Chroma style gallery page; `:git` lastmod;
  content adapters (`_content.html` calling `add_page`); Org front matter.
- **Server** (T71 leftovers): the browser error page, `[server]` headers and redirects, fast
  render (partial rebuilds), TLS, `--openBrowser`.
- **Performance** proposals of §5 (HTML minification, overlapping Tailwind); none is needed for
  the A-P goals.
- **A-D1** runs only through `compare.sh docs-i01`; a committed test like `gate_a_d2` would
  keep it green in CI.
- `neohugo-config`: `TocConfig::end_level` is `u8`, so `endLevel = -1` cannot be decoded
  (markup README "Plan issues").
- `rust/THIRD_PARTY/emoji/LICENSE-GEMOJI` to be compared with gemoji's `LICENSE` (written
  offline).
- The real-site tests that read `NEOHUGO_SITES` are ignored by default.
- The follow-ups of the Go removal outside the repository (§9): branch protection, unused
  secrets, the channels frozen at the last Go build, the binary's name.

## 9. Without Go (2026-10-01)

Branch `go-removal` (from `44529028`) removed the Go implementation and its CI/CD; the
repository is the Rust implementation only. `44529028` is the last commit with the Go tree (no
tag): `git worktree add <dir> 44529028`, or `git show 44529028:<path>` for the Go-tree paths
that comments and READMEs cite.

- **Removed:** Hugo's Go packages, `main.go`, `go.mod`/`go.sum`, `magefile.go`, the release
  (GoReleaser, hugoreleaser), Docker, snap and golangci-lint configuration, `testscripts/`,
  `scripts/`, `tools/go-oracle/`, `tools/neohugo/oracle.sh`, `tools/esbuild/build.sh`, the
  highlight oracle (`crates/highlight/tests/data/oracle/`), and the workflows `ci.yml`,
  `release.yml`, `benchmark.yml`, `golangci-lint.yml`, `image.yml`. `docs/go.mod`, `docs/go.sum`
  and `docs/hugo.work` stay: they belong to the docs site, which stays byte-identical (only
  `docs/rust-port/` changes).
- **Test data moved:** Hugo's test data the tests read is in `rust/testdata/upstream/` at its
  Go-tree path (`hugolib/testsite`, `resources/testdata`, `resources/images/testdata`,
  `tpl/images/testdata`, `media/testdata/fake.png`; 90 files). Fixture ids keep the old paths;
  `neohugo_testkit::fixture::repo_file` resolves them (and `tools/rust-port/i01/sites.py` does
  the same for the testsite). The image oracles read five more Go-tree images and
  `snap/local/logo.png` from byte-identical copies (`crates/images/tests/it/common.rs`). The
  process oracle compares 11,046 cases; the `NEOHUGO_GOROOT` hook for Go's own image test data
  is gone.
- **esbuild:** `tools/neohugo/node/package.json` pins `esbuild` 0.25.6 (the version the Go build
  linked); `tools/neohugo/node.sh` installs it with the other node tools, and
  `tools/esbuild/install.sh` copies its platform binary to `tools/esbuild/bin/esbuild` and checks
  `--version` against the pin. CI runs both, and a failure fails the Test job. A checkout made
  before needs `tools/neohugo/node.sh && tools/esbuild/install.sh` once (the lock file changed).
- **Frozen references:** what the Go implementation generated stays as committed: the oracle
  fixtures (`rust/testdata/oracle/`), the golden data (`rust/testdata/golden/`),
  `rust/crates/build/tests/it/testsite-go.txtar`, the highlight fixtures
  (`rust/crates/highlight/tests/data/`) and `docs/data/docs.yaml`. To regenerate, run the old
  recipe in a worktree of `44529028` and copy the result back (`rust/testdata/golden/README.md`,
  `rust/crates/highlight/README.md`, `tools/neohugo/fixtures2json.py`). `compare.sh` takes only
  `--ref golden`; `selftest.py` perturbs the Go testsite output of `testsite-go.txtar`.
- **Releases:** tags `v<version>` instead of `rust-v<version>`; the Rust workflow publishes the
  GitHub release and marks it latest unless the version has a `-`. The tags `v0.148.2` and
  older are the Go releases.

Follow-ups outside the repository:

- **Branch protection:** required status checks that name jobs of the removed workflows (CI's
  `Build (ubuntu-latest, Go 1.25)`, Golangci-lint, Release, Benchmark, Docker image) would block
  every pull request; require the Rust workflow's `Lint`, `Test` and `Build (<target>)` instead.
- **Secrets** no workflow reads any more: `DOCKERHUB_USERNAME`, `DOCKERHUB_TOKEN`, `CR_PAT`
  (`image.yml`), `NEOHUGO_GITHUB_TOKEN`, `NEOHUGO_EMAIL` (the docs deploy of `release.yml`).
- **Public channels frozen at the last Go build:** the Docker images `neohugo/neohugo` and
  `ghcr.io/neohugo/neohugo`; the documentation site neohugo.github.io (deployed on `v*` tags by
  `release.yml`); and `/releases/latest`, which stays at the Go `v0.148.2` until the first Rust
  release that is not a pre-release (`v0.149.0`).
- **The binary is still named `neohugo-rs`** (`[[bin]]` of `rust/crates/cli/Cargo.toml`); the
  release archives, `package.py`, `notices.py`, `compare.sh` and the docs use that name.

## 10. History

- 2026-09-27: the first session's byte-for-byte port (`archive/HANDOFF-old-port.md`).
- 2026-09-29: REWRITE_PLAN.md revision 2; T00 deleted the old port.
- T00–T66: the rewrite, the harness and the gates (REWRITE_PLAN.md §8.2).
- T70 (2026-09-30): this handoff, A-P, the licence and provenance audit, archive of the old
  port's documents, `hugo.is_server`/`site.server_port`.
- 2026-10-01: the Go implementation and its CI/CD removed (§9).
