# tools/rust-port

Site inputs for the Rust rewrite's acceptance harness, and the record of the real seeksnack
golden build. The comparison itself lives in `tools/neohugo/` (`compare.sh`, `structdiff.py`,
`manifest.py`; see `docs/rust-port/HANDOFF.md`).

**In use:**

- `i01/sites.py` generates every test site outside the repository (`make`, `cache`, `list`) and
  keeps `i01/patches.json` and the Tera patch files of `sites/docs/patches/` in step
  (`patches [--check]`). `tools/neohugo/compare.sh` and the gate tests in
  `crates/cli/tests/it/` call it, as `tools/neohugo/oracle.sh` (at `44529028`) did for the golden data.
- `i01/testsite.txtar`, `i01/seeksnack.txtar`, `i01/errors.txtar`: inputs of the testsite, the
  seeksnack reconstruction and the error-text site (`sites.py make`).
- `testdata/hugo_cache/`: the golden GetRemote (YouTube API) cache entries; `sites.py cache
  seeksnack` serves them, and the config and resources tests read them.
- `golden/hugo_stats.json`: the real seeksnack build's `hugo_stats.json` (neohugo-publish's
  collector tests).
- `golden/canonical.sha256`: the file list (with SHA-256) of the real seeksnack golden Go build,
  6,943 files. Gate A-S (REWRITE_PLAN.md §7.3; needs the private site repository, T73) compares
  the **path set** after L1 normalisation; the hashes are no longer a target.
- `prepare-site.sh <site-repo> <workdir>` checks out the private seeksnack site at the golden
  commit, installs its npm dependencies and applies `site-overlay/` (for A-S).

**Golden data.** The reference of every comparison is what the Go neohugo built, frozen since
the Go implementation was removed after commit `44529028`:

- `testdata/golden/`: the manifests and structure dumps of the testsite, the seeksnack
  reconstruction and the docs variants, and the golden images, written by
  `tools/neohugo/oracle.sh` (T01; the docs labels again in T65 and T66);
  `testdata/golden/README.md` has the schema and the recipe to regenerate them in a
  worktree of `44529028`.
- `crates/build/tests/it/testsite-go.txtar`: the Go build of the testsite (gate A-T).
- `golden/canonical.sha256` and `golden/hugo_stats.json` (above): the real seeksnack golden Go
  build, made with the settings of `build-site.sh`.

**Historical (byte-for-byte port, obsolete):**

- `build-site.sh` records the settings the golden Go build was made with (darwin/arm64, go1.27.1,
  `--minify --clock 2026-09-27T12:00:00Z`, one worker, cold `resources/`, `testdata/hugo_cache`);
  it runs this tree's `neohugo` or a Go neohugo built at `44529028`.
- `compare.py` compares a build with `golden/canonical.sha256` byte for byte, the acceptance test
  of the deleted port. The rewrite does not aim at byte parity; use `tools/neohugo/compare.sh`.
- `i01/compare.sh` and `i01/diff.py` (the old port's byte comparison against an arm64 Go build)
  were removed in T70; they are in the history before that commit.
