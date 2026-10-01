# Developing neohugo

neohugo is the Cargo workspace at the repository root: an idiomatic Rust rewrite of Hugo's site
and page model with Tera 2 templates. The plan, binding for every task and review, is
[`docs/rust-port/REWRITE_PLAN.md`](docs/rust-port/REWRITE_PLAN.md); §1.2 "What Rust style means
here" is the review checklist. The current state (crate map, commands, gates, deviations, open
items) is [`docs/rust-port/HANDOFF.md`](docs/rust-port/HANDOFF.md).

**The old port** (the byte-identical `crates/` tree and its byte-exact Go oracles) was deleted in
T00. It is recoverable at commit `be02933a`, tagged `go-parity-final` in the local repository
(the tag is not on GitHub): `git show go-parity-final:crates/<crate>/<path>`. Salvage rules, not
code, from it (§6.2).

**The Go implementation** (Hugo's Go tree, `tools/go-oracle`, `tools/neohugo/oracle.sh` and the
Go workflows) was removed after commit `44529028`. What it generated is frozen:
`testdata/oracle/`, `testdata/golden/`, `crates/build/tests/it/testsite-go.txtar`,
`crates/highlight/tests/data/` with `crates/highlight/src/data/chroma-lexers.tsv`,
`crates/funcs/tests/fixtures/remarshal/go.txt` and `docs/data/docs.yaml`, as well as the Go
outputs the old port recorded at `be02933a`, such as `testdata/corpus/minify/*.tsv`
(PROVENANCE.md). To regenerate any of it, run the old recipe in a worktree of that commit (`git
worktree add <dir> 44529028`) and copy the result back: `testdata/golden/README.md`,
`crates/highlight/README.md`, the docstring of `tools/neohugo/fixtures2json.py`; `docs.yaml` is
written by the Go binary's `gen docshelper`.
Hugo's test data that the tests read is in `testdata/upstream/`, at its Go-tree path. Go-tree
paths in comments and READMEs (`resources/images/text.go`, `tpl/tplimpl/embedded/templates/`, …)
name files of that commit: `git show 44529028:<path>`.

**The workspace moved to the repository root** after the Go implementation was removed; until
then it was `rust/` (at `44529028` as well: `git show 44529028:rust/<path>`). Ids recorded
before the move keep the prefix (the sources of `testdata/golden/images/manifest.json`, e.g.
`rust/testdata/site-assets/…`); `neohugo_testkit::fixture::repo_file` and `sites.py`'s
`repo_file` resolve them at the root.

## Layout

```
Cargo.toml  Cargo.lock      all third-party deps and features live in [workspace.dependencies]
.cargo/config.toml          target-dir, jobs = 4
clippy.toml deny.toml       thread_local ban; licence policy
PROVENANCE.md THIRD_PARTY/  every non-original file; licences cargo cannot see
crates/<name>/              the product crates of §2.1 (T00 wrote stubs with the real
                            dependency edges of §2.3), testkit (dev), workspace-hack (internal)
testdata/oracle/<area>/     Go-oracle fixtures as plain JSON (neohugo schema, below; frozen)
testdata/golden/<label>/    the Go build's manifests, structure dumps and images (frozen)
testdata/baselines/         the ratchet's baselines (tools/neohugo/changes/README.md)
testdata/corpus/            corpora: seeksnack bodies and front matter, dates, minifier, Thai strings
testdata/site-assets/       the images tools/rust-port/i01/sites.py puts into its sites
testdata/upstream/          Hugo's test data the tests read, at its Go-tree path (fixture ids)
testdata/COUNTS.json        per fixture: old path, record and value counts at conversion
```

Member crates: `[lib] doctest = false`; one integration binary `tests/it/main.rs`
(`autotests = false`); `view`, `sitefuncs`, `render`, `build` and `cli` also set
`[lib] test = false`. Members never add third-party `features =`; every member depends on
`neohugo-workspace-hack`. Each stub lists its *planned* third-party dependencies so that
`Cargo.lock` pins them; the owning task adjusts the list.

## Rules for agents (§8.1)

- **One git worktree per agent**, based on `rust-port`. Only green commits are merged; rebase
  before merging. At most one agent edits a given crate at a time (crate lock); a bug found in
  another crate becomes a short fix task that takes that crate's lock.
- **Environment:**
  ```sh
  export CARGO_TARGET_DIR=/home/user/neohugo/target   # shared by all worktrees
  export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
  ```
- **Build commands:** only `cargo test -p <crate>` (plus `-p` of direct dependants after an API
  change). Never `cargo check`, `clippy` or `doc` in the edit–test loop; never `--workspace`;
  `--release` only for measurements and packaging (since T70); never `cargo clean` (`cargo clean -p X` only when coordinated).
  `cargo fetch`/`cargo metadata` use `--target x86_64-unknown-linux-gnu` /
  `--filter-platform x86_64-unknown-linux-gnu`. Builds run offline after T00.
- **Shared target dir and stale artifacts.** Every worktree has the same layout, so a path crate
  gets the same cargo metadata hash in every worktree and cargo may reuse another worktree's
  artifact when its source mtimes look older. Before a final test run (and when a result looks
  impossible), `touch` the sources of the crates you test, e.g.
  `find crates/<name> -name '*.rs' -exec touch {} +`. Read data paths at run time with
  `neohugo_testkit::fixture::{repo_dir, testdata}`, never with `env!("CARGO_MANIFEST_DIR")`.
- **Disk:** `tools/neohugo/disk.sh` fails above 8 GB of `target` (`NEOHUGO_TARGET_LIMIT_MB`) or below 2 GB free (raised from the plan's 2.5 GB once ~20 GB became free).
  Every task reports `du -sh /home/user/neohugo/target` and `df -h /` when it ends.
- **Clean room:** never open Zola ≥ 0.22 source (EUPL-1.2). Copied files go through
  `PROVENANCE.md` first.
- **Review checklist:** §1.2 (typed model, no thread-locals, no Go-order emulation, no Go error
  texts, no Go method tables or transliterated state machines, no Zola-isms) plus the task's
  acceptance criteria.

## Lanes

Four lanes (§8.3): A the model and critical path (T00 → T10 → T13 → T20 → T21 → T23a → T23b →
T33 → T34 → T36 → T37 → T60 → T65); B templates, markup and render; C harness, output and
resources; D layout conversion, images and nav. Heavy crates (`highlight`, `minify`, `images`,
ICU data in `locale`, `serve`) stay out of lanes A/B until round 8.

## Commands

| What | Command |
|---|---|
| per crate | `cargo test -p <crate>` (from your worktree's root) |
| phase end | `cargo clippy -p <crate> -- -D warnings` per crate of the phase; `cargo fmt --check`; `tools/neohugo/licence-check.sh` |
| graph | `cargo metadata --format-version 1 --filter-platform x86_64-unknown-linux-gnu` |
| disk | `tools/neohugo/disk.sh` |
| fixtures | `tools/neohugo/fixtures2json.py convert <dir> <dir>` after regenerating a Go oracle (in a worktree of `44529028`) |
| acceptance | `tools/neohugo/compare.sh <site> [--docs-patches i01\|reduced] [KEEP=1]` (T03) |
| templates | `neohugo templates check -s <site-dir>` (T37) |
| CI, locally | see "CI and releases" below (workspace-wide: not for the edit–test loop) |

## CI and releases

`.github/workflows/ci.yml` builds, tests and releases this workspace; it is the repository's
only build workflow (`stale.yml` manages issues).

**When it runs.** On pushes to `main` and `rust-port`, on every pull request, on every
`v[0-9]*` tag, and by hand (`workflow_dispatch`); there are no path filters. A newer run on the
same ref cancels the older one, except on tags.

| Job | Runner | Steps |
|---|---|---|
| Lint | ubuntu-24.04 | `cargo fmt --all --check`; `cargo clippy --workspace --all-targets --locked -- -D warnings`; `tools/neohugo/licence-check.sh`; `tools/neohugo/selftest.py`; on a tag, the tag must be `v<version of [workspace.package]>` |
| Test | ubuntu-24.04 | `cargo test --workspace --locked --no-fail-fast` with the tools below; the job summary lists every test that printed `SKIPPED`, and any such test fails the job |
| Build | one native runner per target | `cargo build --release --locked -p neohugo --target <triple>` with the version variables (below); `neohugo version`; `tools/neohugo/notices.py` (licences of the linked crates); `tools/neohugo/package.py` → artifact `neohugo-<triple>` |
| Release | ubuntu-24.04 | tags only, after the other three: the GitHub release (below) |

Build targets and runners: `x86_64-unknown-linux-gnu` (ubuntu-22.04), `aarch64-unknown-linux-gnu`
(ubuntu-22.04-arm), `x86_64-apple-darwin` (macos-15-intel), `aarch64-apple-darwin` (macos-15),
`x86_64-pc-windows-msvc` (windows-2025). The binary's native C code, libwebp (`webp` →
`libwebp-sys`) and `ring` (`rustls`, for `get_remote`), compiles with each runner's own C
compiler through the `cc` crate; there are no cross toolchains. The Linux binaries are linked on
Ubuntu 22.04, so they need no glibc newer than its 2.35 (the smoke test prints the exact
version; linked on 24.04 they would need 2.39: std's weak `pidfd_spawnp` reference still
records `GLIBC_2.39`). The Windows binary links the MSVC runtime statically (`+crt-static`), so
it needs no Visual C++ redistributable.

The release is a drop-in replacement for the Go releases (`.goreleaser.yml` at `44529028`):
the binary is `neohugo` (`neohugo.exe`), and an archive is named as theirs,
`neohugo_<version>_<os>-<arch>.tar.gz` (`.zip` for Windows; `<os>` `linux`, `darwin` or
`windows`, `<arch>` `amd64` or `arm64`). It holds, at its root, the binary, `README.md` and
`LICENSE` (as the Go archives did) and `THIRD_PARTY_NOTICES.txt`, `PROVENANCE.md` and
`THIRD_PARTY/`; the build job writes a `.sha256` next to it. `neohugo version` prints the Go
format, `neohugo v<version>[-<commit>] <os>/<arch> BuildDate=<date|unknown>[ VendorInfo=<vendor>]`
(`crates/cli/src/version.rs`): the build job sets `NEOHUGO_BUILD_COMMIT` (the commit),
`NEOHUGO_BUILD_DATE` (its time in UTC, RFC 3339) and `NEOHUGO_VENDOR_INFO=neohugo` for the
build, and the smoke test checks the line; without them (a local build) the line has no commit,
`BuildDate=unknown` and no vendor. `package.py` takes the version from `Cargo.toml` and checks
that `neohugo version` prints it.

`THIRD_PARTY_NOTICES.txt` is written by `tools/neohugo/notices.py <triple> <file>`: the licence
and notice files of every crate linked into `neohugo` for that target (the normal-dependency
closure of `neohugo` without proc macros; vendored C libraries such as libwebp included), each
text printed once. A linked crate that ships no licence file gets the MIT text when MIT is one of
its licences; any other such crate fails the step, so it is noticed before a release.

**Toolchain.** CI installs `RUST_TOOLCHAIN` (1.94.1, the toolchain the workspace is developed
with) through rustup; `rust-version = "1.94"` in `Cargo.toml` stays the MSRV. A newer clippy
brings new lints, which `-D warnings` turns into failures, so the pin moves in a commit of its
own that also fixes the new findings. There is deliberately no `rust-toolchain.toml`: the
agents' offline rustup has only `stable` (which is 1.94.1), and a pinned channel makes rustup
look for a toolchain named `1.94.1` and try to download it. Add one (and drop `RUST_TOOLCHAIN`)
once every environment can install toolchains.

**Tools the tests use.** Without its tool a test prints `SKIPPED …` and passes, so CI provides
all of them, and its Test job fails when a test prints `SKIPPED`:

| Tests | Tool | In CI |
|---|---|---|
| `neohugo-esbuild`: `jsbuild_synth`, `jsbuild_docs`, `build_errors_are_messages`, `inline_source_map`, `concurrent_builds_share_one_service`, `plugin_callbacks`, `version_ping_and_build_round_trip`; `neohugo-resources`: `js_build_docs`, `js_build_t16site` and the js_build half of `execute_as_template_with_tera` (silent) | esbuild: `NEOHUGO_ESBUILD_BINARY`, else `tools/esbuild/bin/esbuild` | `tools/esbuild/install.sh` after `tools/neohugo/node.sh`, like a local install: the binary of the npm package `tools/neohugo/node/package.json` pins, checked with `--version`; a failure fails the job |
| `neohugo-resources`: `babel_fake_tool`, `postcss_oracle_fake_tool`, `post_process_reconstruction_chain_fake_postcss`, `tailwind_docs_styles_fake_tool`, `tools_get_hugo_environment` | `node` on `PATH` (the fake tools are node scripts) | `actions/setup-node`, Node 22 |
| `neohugo-resources`: `postcss_oracle_real_tool`, `post_process_reconstruction_chain_real_postcss`, `tailwind_docs_styles_real_tool`, `babel_real_tool` | `NEOHUGO_POSTCSS_BIN`, `NEOHUGO_TAILWINDCSS_BIN`, `NEOHUGO_BABEL_BIN` (plugins: `NEOHUGO_NODE_MODULES`) | `tools/neohugo/node.sh`; the variables point into the `node_modules/.bin` it leaves under `tools/neohugo/` |
| `neohugo`: `gate_a_r`, `gate_a_d2` (`tools/neohugo/compare.sh … --ref golden`) | `python3`, `bash` and `node` on `PATH`; the node modules (`NEOHUGO_NODE_MODULES`, else `tools/neohugo/node.sh path`) and esbuild (`NEOHUGO_ESBUILD_BINARY`, else the main checkout's `tools/esbuild/bin/esbuild`) | the runner's `python3` and `bash`; the three rows above |

`neohugo-images`' `sizes_match_the_process_oracle` compares 11,046 cases (at least 10,000 must
compare); the cases of 85 more sources need Go's own image test data, which is not in the
repository (`crates/images/README.md`). Python is needed only by the gate tests and the tools
(`licence-check.sh`, `selftest.py`, `notices.py`, `package.py` (Python 3.11 or later), and
`tools/rust-port/i01/sites.py` for the ignored real-site tests that read `NEOHUGO_SITES`). Not
needed by `cargo test`: dart-sass (grass compiles Sass in process) and the network (`get_remote`
tests read caches with the network off).

**Reproduce CI locally** (from the repository root, offline; `touch` the sources first, see the
shared-target note above):

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
tools/neohugo/licence-check.sh
python3 tools/neohugo/selftest.py
N=$(tools/neohugo/node.sh path)   # once: tools/neohugo/node.sh && tools/esbuild/install.sh
NEOHUGO_ESBUILD_BINARY=$PWD/tools/esbuild/bin/esbuild NEOHUGO_NODE_MODULES=$N \
NEOHUGO_POSTCSS_BIN=$N/.bin/postcss NEOHUGO_TAILWINDCSS_BIN=$N/.bin/tailwindcss \
NEOHUGO_BABEL_BIN=$N/.bin/babel \
  cargo test --workspace --locked --offline --no-fail-fast -- --show-output
cargo build --release --locked --offline -p neohugo --target x86_64-unknown-linux-gnu \
  --target-dir <scratch>/target                  # never the shared target dir
python3 tools/neohugo/notices.py x86_64-unknown-linux-gnu <scratch>/THIRD_PARTY_NOTICES.txt
python3 tools/neohugo/package.py <scratch>/target/x86_64-unknown-linux-gnu/release/neohugo \
  x86_64-unknown-linux-gnu <scratch>/dist <scratch>/THIRD_PARTY_NOTICES.txt
```

**Cutting a release.**
1. Set `version` in `[workspace.package]` of `Cargo.toml` (e.g. `0.149.0`); commit and merge
   it.
2. Tag that commit and push the tag: `git tag v0.149.0 <commit>`, `git push origin v0.149.0`.
3. The workflow checks the tag against the version, runs lint, test and the five builds, then
   creates the GitHub release `v<version>`, titled `neohugo <version>`, with the five archives
   and `neohugo_<version>_checksums.txt` (`<sha256>  <archive>` per archive, the checksums file
   of the Go releases; the `.sha256` files of the build jobs are checked and joined into it).
   A version with a `-` (e.g. `0.150.0-rc.1`) makes a pre-release, never latest; any other
   release is marked latest only if no published release has a higher version, so a patch
   release of an older line does not take latest from a newer one.

Re-running the workflow (or its failed jobs) for a tag replaces the assets of the release an
earlier run created. The tags `v0.148.2` and older are the Go implementation's releases.

**Caches.** `Swatinem/rust-cache` per job and target; only pushes to `main` and `rust-port` save
them, and pull requests restore their base branch's. A cold test job builds about 3.2 GB of
`target/` (1.8 GB of it test executables, 0.8 GB of which debug info: `debug =
"line-tables-only"` of the dev profile already applies), well within the runner's disk.

## Test data: the neohugo schema

Fixtures are JSON documents (`.json`) or JSON lines (`.jsonl`), gzipped when named `.gz`.
The Go oracles' type tags are unwrapped into plain JSON; Go values plain JSON cannot hold are
single-key objects: `{"$nh:time": "<RFC 9557>"}`, `{"$nh:local": "<TOML local date/time>"}`,
`{"$nh:bytes": "<hex>"}` (not UTF-8), `{"$nh:float": "NaN"|"+Inf"|"-Inf"}`,
`{"$nh:keys": [..]}`, `{"$nh:len": n}`. Other objects with a `"t"` key are oracle records.
The full description is the docstring of `tools/neohugo/fixtures2json.py`; read fixtures with
`neohugo_testkit::fixture` (`oracle`, `oracle_lines`, `Tag`, `GoString`) and compare semantic
fields only, with a reviewed `expected_diffs.toml` per crate.

## Optional features

`neohugo-funcs` compiles two SHOULD template functions only with a feature: `to_math` (`math`,
pulldown-latex) and `diagrams_goat` (`goat`, svgbob and its geometry crates). Without it the
name is registered as a stub that fails when called. The `neohugo` crate (the `neohugo`
binary) turns both on by default (`default = ["goat", "math"]`), so the release build and CI's
`-p neohugo` builds render the documentation site's formulas and diagrams (gate A-D2, T66);
`--no-default-features` leaves them out. Their cost (T66): 53 more packages in `neohugo`'s
normal dependency tree (nalgebra, parry2d, sauron, futures, …; all licences pass
`licence-check.sh`, which always checks `--all-features`) and about 1.4 MB (1.5 %) of a
stripped debug binary. A test of `neohugo-funcs`
alone builds without them unless `--features goat,math` is given (`tests/it/{math,goat}.rs`).

## Feature unification

Cargo unifies features per invocation over the selected packages, so `-p a` and `-p b` could
build a shared dependency twice with different features. cargo 1.94.1 has no stable
`resolver.feature-unification = "workspace"` (it warns "ignoring `resolver.feature-unification`
without `-Zfeature-unification`"), so `crates/workspace-hack` depends on the light shared
dependencies with the union of their features (declared in the "workspace-hack only" block of
`Cargo.toml`). Check it after changing dependencies: for every member `m`, run

```sh
cargo tree -p m --target x86_64-unknown-linux-gnu -e normal,dev,no-proc-macro \
  --prefix none --no-dedupe -f '{p}|{f}'
```

and compare each package's feature list across members; a light crate that differs goes into
the hack with the union. Deliberately left out (heavy, or used by one lane only): ICU crates with
compiled data (`icu_normalizer`, `icu_properties`, `icu_calendar`, `icu_time`), `phf`, `rand`,
`getrandom` 0.2/0.4, `hashbrown` 0.14, `http`, `httparse`, `tracing-core`, and host-only
proc-macro dependencies (`syn`).
