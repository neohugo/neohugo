# Go oracles

Go programs that run neohugo's Go code and dump what it computes, as test fixtures for the Rust
rewrite (`docs/rust-port/REWRITE_PLAN.md` §6.4). Only the oracles whose fixtures the rewrite
uses are kept; the byte-exact oracles of the old port were deleted in T00 and are recoverable at
commit `be02933a` (local tag `go-parity-final`).

- Each oracle writes to `rust/testdata/oracle/<area>/…` (`<area>` = the old crate name without
  `nh-`), run from the repository root: `go run ./tools/go-oracle/<oracle> [-root .] [-out …]`.
- The oracles still write the typed `goval` / `tval` / `rsupport.Enc` JSON. After regenerating,
  convert in place to the plain neohugo schema:
  `tools/neohugo/fixtures2json.py convert rust/testdata/oracle/<area> rust/testdata/oracle/<area>`
  (the `regen.sh` scripts do this).
- They run natively; the old linux/arm64 + qemu builds are gone, because output bytes are no
  longer compared with Go.
- Comments that name `crates/nh-…` (sources, tests, `PORTING.md`) describe the old port, which
  is only available through `git show go-parity-final:crates/…`.
