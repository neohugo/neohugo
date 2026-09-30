# neohugo Rust workspace

An idiomatic Rust rewrite of neohugo (Hugo's site and page model, Tera 2 templates). The plan,
binding for every task and review, is [`docs/rust-port/REWRITE_PLAN.md`](../docs/rust-port/REWRITE_PLAN.md);
§1.2 "What Rust style means here" is the review checklist.

**The old port** (the byte-identical `crates/` tree and its byte-exact Go oracles) was deleted in
T00. It is recoverable at commit `be02933a`, tagged `go-parity-final` in the local repository
(the tag is not on GitHub): `git show go-parity-final:crates/<crate>/<path>`. Salvage rules, not
code, from it (§6.2).

## Layout

```
Cargo.toml  Cargo.lock      all third-party deps and features live in [workspace.dependencies]
.cargo/config.toml          target-dir, jobs = 4
clippy.toml deny.toml       thread_local ban; licence policy
PROVENANCE.md THIRD_PARTY/  every non-original file; licences cargo cannot see
crates/<name>/              the product crates of §2.1 (T00 wrote stubs with the real
                            dependency edges of §2.3), testkit (dev), workspace-hack (internal)
testdata/oracle/<area>/     Go-oracle fixtures as plain JSON (neohugo schema, below)
testdata/corpus/            corpora: seeksnack bodies and front matter, dates, minifier, Thai strings
testdata/site-assets/       the images tools/rust-port/i01/sites.py puts into its sites
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
  export CARGO_TARGET_DIR=/home/user/neohugo/rust/target   # shared by all worktrees
  export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
  ```
- **Build commands:** only `cargo test -p <crate>` (plus `-p` of direct dependants after an API
  change). Never `cargo check`, `clippy` or `doc` in the edit–test loop; never `--workspace`;
  never `--release` before T70; never `cargo clean` (`cargo clean -p X` only when coordinated).
  `cargo fetch`/`cargo metadata` use `--target x86_64-unknown-linux-gnu` /
  `--filter-platform x86_64-unknown-linux-gnu`. Builds run offline after T00.
- **Shared target dir and stale artifacts.** Every worktree has the same layout, so a path crate
  gets the same cargo metadata hash in every worktree and cargo may reuse another worktree's
  artifact when its source mtimes look older. Before a final test run (and when a result looks
  impossible), `touch` the sources of the crates you test, e.g.
  `find crates/<name> -name '*.rs' -exec touch {} +`. Read data paths at run time with
  `neohugo_testkit::fixture::{rust_dir, testdata}`, never with `env!("CARGO_MANIFEST_DIR")`.
- **Disk:** `tools/neohugo/disk.sh` fails above 8 GB of `rust/target` (`NEOHUGO_TARGET_LIMIT_MB`) or below 2 GB free (raised from the plan's 2.5 GB once ~20 GB became free).
  Every task reports `du -sh /home/user/neohugo/rust/target` and `df -h /` when it ends.
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
| per crate | `cargo test -p <crate>` (in your worktree's `rust/`) |
| phase end | `cargo clippy -p <crate> -- -D warnings` per crate of the phase; `cargo fmt --check`; `tools/neohugo/licence-check.sh` |
| graph | `cargo metadata --format-version 1 --filter-platform x86_64-unknown-linux-gnu` |
| disk | `tools/neohugo/disk.sh` |
| fixtures | `tools/neohugo/fixtures2json.py convert <dir> <dir>` after regenerating a Go oracle |
| acceptance | `tools/neohugo/compare.sh <site> [--docs-patches i01\|reduced] [KEEP=1]` (T03) |
| templates | `neohugo-rs templates check -s <site-dir>` (T37) |

## Test data: the neohugo schema

Fixtures are JSON documents (`.json`) or JSON lines (`.jsonl`), gzipped when named `.gz`.
The Go oracles' type tags are unwrapped into plain JSON; Go values plain JSON cannot hold are
single-key objects: `{"$nh:time": "<RFC 9557>"}`, `{"$nh:local": "<TOML local date/time>"}`,
`{"$nh:bytes": "<hex>"}` (not UTF-8), `{"$nh:float": "NaN"|"+Inf"|"-Inf"}`,
`{"$nh:keys": [..]}`, `{"$nh:len": n}`. Other objects with a `"t"` key are oracle records.
The full description is the docstring of `tools/neohugo/fixtures2json.py`; read fixtures with
`neohugo_testkit::fixture` (`oracle`, `oracle_lines`, `Tag`, `GoString`) and compare semantic
fields only, with a reviewed `expected_diffs.toml` per crate.

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
