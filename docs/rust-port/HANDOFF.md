# neohugo → Rust port: handoff (start here)

This document is the entry point for continuing the Rust port of neohugo in a
new session, for example a Claude Code cloud session on Linux x86_64. It was
written when the first local session (macOS arm64) stopped on 2026-09-27.

## 1. Goal and acceptance test

Port neohugo (this Go repository, a Hugo fork) to Rust. The acceptance test is
byte parity. The Rust binary must build the seeksnack site
(`github.com/blackb1rd/seeksnack-seeksnack` at commit `ae6c922`) into output that
is byte-identical to the golden Go build: all 6943 files listed with their
SHA-256 in `tools/rust-port/golden/canonical.sha256`.

The golden was built with go1.27.1 on darwin/arm64 by
`tools/rust-port/build-site.sh` using these settings:

- `--minify --clock 2026-09-27T12:00:00Z`;
- `HUGO_NUMWORKERMULTIPLIER=1` (deterministic; 13 taxonomy terms collide on one URL, and the last in tree order wins);
- a cold `resources/` cache;
- the checked-in YouTube GetRemote cache (`HUGO_CACHEDIR=tools/rust-port/testdata/hugo_cache`).

The procedure was verified on a fresh clone and reproduces the golden 100%:

```sh
tools/rust-port/prepare-site.sh https://github.com/blackb1rd/seeksnack-seeksnack.git "$W"
tools/rust-port/build-site.sh <neohugo-binary> "$W/seeksnack" "$W/out"
tools/rust-port/compare.py "$W/out"            # exit 0 == byte parity
```

The site repo is **private**. In a cloud session, attach
`blackb1rd/seeksnack-seeksnack` as a second repository (the Claude GitHub App
needs access to it), then pass its local checkout path to `prepare-site.sh`
instead of the URL. The script clones it again and checks out `ae6c922`.

Rules:

- The site dir must be named `seeksnack`; it keys the GetRemote cache.
- `npm ci` must use the site's lock file; the CSS bytes depend on postcss, cssnano, purgecss and caniuse-lite.
- js.Build needs esbuild **0.25.6** (`npm i esbuild@0.25.6`). The Rust port finds it through `NEOHUGO_ESBUILD_BINARY`.

## 2. Read these first

1. `crates/README.md` holds the porting rules. Every crate is a faithful line-by-line port with
   Go-oracle differential tests. It also covers the FMA rule, the allowed crates, and the standalone crate layout
   (each crate has an empty `[workspace]` for now).
2. `docs/rust-port/specs/*.md` is the research. It covers every subsystem the seeksnack build uses,
   with Go file and line references and parity traps. `architecture-core.md` is the overview;
   `template-engine.md` §16 is the engine contract.
3. `crates/HUGO_LAYER.md`, `crates/WAVE_B_PLAN.json` and `crates/GOTEMPLATE_CONTRACT.md` hold the
   Hugo-layer design, its 31-task plan, and the template engine interface.
   `docs/rust-port/HUGO_LAYER_CRITIQUE.md` is the review the design was revised against; all 21
   points were addressed.
4. Each crate's `PORTING.md` has its Go→Rust map, deviations, verification and gaps.

The specs and design docs mention paths such as `$SCRATCH/work/...`, `$W/...` and
`/private/tmp/.../scratchpad`. Those were the first session's scratch
directories. They do not exist anymore, and their large corpora (GBs) were
not committed. Regenerate anything you need with the Go oracles in
`tools/go-oracle/<crate>/`. The specs themselves are in `docs/rust-port/specs/`.

## 3. Environment

- **Go 1.27.1 exactly** for every oracle and for building the Go reference: `export GOTOOLCHAIN=go1.27.1`.
  - Unicode 17 tables, `encoding/json` on jsonv2 (default in 1.27), and the new jpeg DCT and flate encoder all depend on it.
  - go.mod's `go 1.23.0` line sets the GODEBUG defaults the go-url fixtures rely on, so do not change it.
  - The Go project's CI uses go1.25. `tools/go-oracle/go-unicode` is gated with `//go:build go1.27` so `go build ./...` still passes there.
- **Rust** with edition 2024 (≥1.85; the first session used 1.98). Build each crate with its own target dir, e.g. `CARGO_TARGET_DIR=/tmp/targets/<crate>`.
- **C/C++ toolchain** for `libwebp-sys` and `libsass-sys`, and for the cgo oracles (gift, go-hashstructure, libwebp-sys, libsass-sys).
- **Node + npm** for the site's postcss pipeline, plus esbuild 0.25.6.

### Platform caveat (important on Linux x86_64)

The golden was produced on **darwin/arm64**. Some Go outputs depend on the platform:

- The arm64 Go compiler fuses `x*y+z` into FMA and amd64 does not. This affects gift resampling (11 of the golden images), the flate EstimatedBits choice and tdewolff ParseFloat.
- arm64 and amd64 give different results for `int64(+Inf)` and for the `slices.Min`/`Max` NaN payload.
- libwebp's DSP paths and libm, and libsass's number formatting, depend on the C toolchain.

The Rust ports replicate the **arm64** behaviour explicitly (`mul_add` etc.), and every checked-in fixture came from arm64 Go. So `cargo test` is meaningful on any machine. **But do not regenerate float-sensitive fixtures (gift, go-flate, tdewolff strconv) with Go on amd64.** The fixtures would change even though the port is right.

To produce arm64 Go output on Linux x86_64, build the oracle for linux/arm64 and run it under `qemu-aarch64-static` (`apt install qemu-user-static`). linux/arm64 Go reproduces the checked-in darwin/arm64 fixtures of gift (all of them) and go-png. Pure-Go oracles need only `GOARCH=arm64`. cgo oracles additionally need a C cross-compiler; zig works (`zig cc -target aarch64-linux-musl`, with `-ldflags '-linkmode external -extldflags -static'`). The recipe is in `crates/gift/PORTING.md` (§FMA sites, §Regenerating fixtures).

The first Linux x86_64 cloud checks (go-value, go-html, go-path, go-sort, go-unicode, go-time, tdewolff-parse) passed with 0 real mismatches over about 20M fresh cases. The only differences were the Go platform differences listed above. libwebp-sys and libsass-sys pass on linux/x86_64 when compiled by clang with `-ffp-contract=on -mfma`, which their `build.rs` now selects off Apple (see their PORTING.md). gcc gives different bytes: FMA contraction in libwebp, argument evaluation order in LibSass.

## 4. Crate status (2026-09-27)

Legend:

- **verified**: ported, red-teamed and committed.
- **ported**: complete; the red-team was interrupted or never run.
- **partial**: work in progress.

| crate | ports | status | evidence / next step |
|---|---|---|---|
| go-value | value model (`interface{}`+reflection), time data | verified | used by everything; has a NilKind registry for named Go types |
| go-unicode | unicode (U17), utf8, utf16, strings/bytes | verified | exhaustive per code point, 9.2M strings vectors |
| go-strconv | strconv | verified | all 2^32 float32 patterns, Go test tables |
| go-time | time | verified | ~538k checks + 1,202 zones |
| go-url, go-html, go-path, go-sort | net/url, html, path(+filepath), sort/slices | verified | ~15M adversarial cases; Linux-checked |
| go-yaml, go-hashstructure | yaml.v2 + metadecoders, hashstructure | verified | 368k YAML inputs × 4 paths |
| go-image | image, color, draw, jpeg | verified | all 538 site JPEGs, ~4M cases |
| xtext-collate | x/text collate + CLDR 23 tables | verified | 30k tags, Thai enumerations |
| tdewolff-parse | tdewolff/parse (minus js) | verified | ~21M fuzzed streams; Linux-checked |
| go-flate | compress/flate + zlib | ported | 6,676 cases + 11 golden PNG IDAT streams; red-team unfinished (fuzz seeds 4–11 unchecked); PORTING.md predates the red-team additions |
| go-fmt | fmt over Value | ported | 4.49M Sprintf outputs; red-team finished its fixes, but no final report |
| go-json | encoding/json (jsonv2-backed v1) | ported | red-team interrupted. **Oracle bug: `adv.go -mode advtext` can make Go's Indent loop forever; fix before large runs** |
| goldmark | yuin/goldmark v1.7.12 + extensions | ported | CommonMark spec, full seeksnack corpus, 8,658 ext edge cases; **red-team never run** |
| tdewolff-minify | minify (html/css/json/svg/xml) | ported | ~114k records; red-team interrupted |
| tdewolff-parse-js | parse/js | ported | 33k inputs × 8 dump modes; red-team interrupted |
| libwebp-sys | vendored libwebp 1.3.2 + gowebp wrapper | ported | object code identical to cgo per unit; 807/807 golden webps; linux/x86_64 passes every test when built with clang `-ffp-contract=on -mfma` (now the `build.rs` default off Apple); red-team not run |
| libsass-sys | vendored libsass 3.6.6 + golibsass wrapper | ported | 2,052 cases; linux/x86_64 passes every test with clang (gcc evaluates C++ arguments right to left: 194 error positions differ), now the `build.rs` default off Apple; red-team not run |
| go-png | image/png | verified | Go reader/writer/paeth tests ported; 217,650 checked-in differential checks (synth, filter ties, pooled encoders, 75 real files, 42,590 truncations + 42,590 reader failures, 9k generated/mutated PNGs) + 1.56M out of repo (`GO_PNG_BIG`), 0 differences; 11 golden PNGs re-encode byte-identical; fixtures from an arm64 (qemu) oracle build, identical to amd64; see `crates/go-png/PORTING.md` |
| gift | disintegration/gift + Hugo filters | ported | all 61 Go test functions ported (TestGolden with arm64 ±1 expectations); 603 site images identical (first session); every FMA site re-read from the arm64 disassembly (gift 119, resampling.go 15, Go math 68 instructions) and mutation-tested (125 `mul_add`: 77 killed by checked-in tests, 9 only by out-of-repo arm64 runs, 21 equivalent, 18 unobservable); linux/arm64 oracle under qemu reproduces every darwin/arm64 fixture; see `crates/gift/PORTING.md`; **red-team not run** |
| tdewolff-minify-js | minify/js | ported | 783 upstream table rows, 58k checked-in fixture checks (+2×495k out of repo, 27.5 MB corpus × 6 configs) identical; 1,589 HTML docs through Hugo's full minifier; no FMA sites; **red-team not run** |
| gotemplate | forked text/template + html/template | ported | host contract (`GOTEMPLATE_CONTRACT.md` C1–C13) implemented and documented in `crates/gotemplate/PORTING.md`; Go's lex/parse/exec/escape/content/clone/multi/template tests ported; oracles: 4,873 parse + 13,470 Hugo-like exec + 23,542 html exec operations, 39k escaper calls, 54k transitions; the escaper reproduces Go's escaped trees of all 120 repo layouts byte for byte; **red-team not run; the seeksnack layouts (private repo) are not yet in the escdump corpus**; nh-tplimpl (T13) can now replace its `engine.rs` placeholder |
| nh-* (25 crates) | the Hugo layer | skeleton | all `cargo check`; 1,121 `todo!()`; ownership per `WAVE_B_PLAN.json` |

## 5. What to do next (in order)

1. **Finish Wave A.**
   - Port `gotemplate` (text/template, then html/template) against the contract. This is the long pole; every Hugo-layer task needs it.
   - Finish `tdewolff-minify-js`. The whole-page golden check for the minify stack needs it.
   - ~~Write tests and PORTING.md for `go-png`.~~ Done.
   - ~~Write PORTING.md for `gift` and port its unit tests.~~ Done.
2. **Red-team the "ported" crates.** Use an independent pass that extends the Go oracle with adversarial and random inputs and fixes any divergence. The first session found and fixed real bugs this way (go-time, go-yaml, xtext-collate).
3. **Wave B, the Hugo layer.**
   - Follow `crates/WAVE_B_PLAN.json`: 27 port tasks with disjoint module ownership, then 4 integration tasks.
   - Each task has concrete oracle and acceptance tests. The critical path is in `HUGO_LAYER.md` §12.
   - Wire the Wave A dependencies into the nh-* `Cargo.toml` files as each lands.
4. **Integration.**
   - Merge all crates into one root Cargo workspace (drop the per-crate `[workspace]` tables).
   - Build the `neohugo` binary (`nh-commands`).
   - Iterate with `build-site.sh` + `compare.py` until 6943/6943 match.
   - Classify each remaining diff by subsystem and fix it in the owning crate with a new oracle test.

## 6. How work was run, and cloud notes

- The first session ran ports as parallel agents, one crate each, followed by an independent **red-team agent** per crate. Every result was committed only after the crate's tests passed.
- A cloud machine has 4 vCPUs, so run fewer agents at once, and give each crate its own `CARGO_TARGET_DIR`.
- Commit to `rust-port` often; nothing outside git survives the session.
- Pushing needs GitHub write access to `neohugo/neohugo`. Without the Claude GitHub App installed on the org, cloud pushes fail with 403. As of 2026-09-27 evening, cloud pushes to `rust-port-cloud/*` worked.
- Cloud verification branches: `rust-port-cloud/linux-verify-4` has unmerged work. It holds an extended xtext-collate/go-image Linux verification and its own copy of the `tags*.tsv` fixtures plus a nested `.gitignore`; `rust-port` already force-added the original fixtures. Review it and merge or cherry-pick, and resolve the fixture overlap in favour of `rust-port`'s Mac-generated files, which are byte-identical to the fixtures the tests were written against.
- `tools/go-oracle/*` are `package main` programs inside the neohugo Go module. Keep them gofmt- and vet-clean, because CI runs `go vet`/`go test ./...` on the whole module.
