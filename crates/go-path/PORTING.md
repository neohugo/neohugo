# go-path — porting notes

Port of go1.27.1 `path` and the unix build of `path/filepath`
(`Separator = '/'`, `ListSeparator = ':'`, empty volume names).

## Go file → Rust module map

| Go (go1.27.1 `$GOROOT/src`) | Rust |
|---|---|
| `path/path.go` (lazybuf, Clean, Split, Join, Ext, Base, IsAbs, Dir) | `src/path.rs` |
| `path/match.go` (ErrBadPattern, Match, scanChunk, matchChunk, getEsc) | `src/path.rs`, `BadPattern` in `src/lib.rs` |
| `internal/filepathlite/path.go` (lazybuf, Clean, IsLocal/unixIsLocal, ToSlash, FromSlash, Split, Ext, Base, Dir, VolumeName) | `src/filepath.rs` |
| `internal/filepathlite/path_unix.go`, `path_nonwindows.go` (IsPathSeparator, IsAbs, volumeNameLen, postClean) | `src/filepath.rs` |
| `path/filepath/path.go` (Clean, IsLocal, ToSlash, FromSlash, SplitList, Split, Join, Ext, IsAbs, Rel, Base, Dir, VolumeName) | `src/filepath.rs` |
| `path/filepath/path_unix.go` (HasPrefix, splitList, join, sameWord) | `src/filepath.rs` |
| `path/filepath/match.go` (Match, scanChunk, matchChunk, getEsc with `runtime.GOOS != "windows"`) | `src/filepath.rs` |
| `unicode/utf8.DecodeRuneInString` | `src/utf8.rs` (private copy) |

## Public API

Every function has a byte form (`*_bytes`, arbitrary bytes like Go strings)
and a `&str` form. Both packages only cut and join at ASCII bytes (`/`, `.`,
`:`), so valid UTF-8 input always gives valid UTF-8 output and the `&str`
forms are exact.

```rust
go_path::path::{clean, split, join, ext, base, is_abs, dir, r#match}          // + *_bytes
go_path::filepath::{clean, split, join, ext, base, is_abs, is_local, dir, rel,
                    r#match, split_list, to_slash, from_slash, volume_name, has_prefix}  // + *_bytes
go_path::filepath::{SEPARATOR, LIST_SEPARATOR, is_path_separator}
go_path::BadPattern          // ErrBadPattern, Display "syntax error in pattern"
go_path::filepath::RelError  // msg: "Rel: can't make <targ> relative to <base>"
```

`join`/`join_bytes` take `&[S]` with `S: AsRef<str>` / `AsRef<[u8]>`.

## Deliberate deviations

* `filepath.ToSlash`/`FromSlash`/`VolumeName` are the unix identities (no
  Windows/plan9 code paths). Windows-only branches that remain in shared
  code (`volLen > 1 && UNC`, `VolumeNameLen(baseVol) > 2`) are kept but are
  dead on unix, as in Go.
* Not ported (they touch the filesystem): `Abs`, `EvalSymlinks`, `Walk`,
  `WalkDir`, `Glob`, `Localize`.
* `utf8.DecodeRuneInString` is a private copy (`src/utf8.rs`) so the crate
  has no dependencies; `go-unicode` holds the canonical port.

## FMA sites

None (no floating-point arithmetic).

## Verification

Oracle: `tools/go-oracle/go-path -out <file> [-n N] [-seed S] [-inputs F
-only-inputs]`. Records are `op, args, results` for `path.{Clean, Split,
Ext, Base, Dir, IsAbs, Join, Match}` and `filepath.{Clean, Split, Ext, Base,
Dir, IsAbs, IsLocal, ToSlash, FromSlash, VolumeName, SplitList, Join, Match,
Rel}` (match result + error string; Rel result + error string).

* `tests/fixtures/path.txt` (46,946 records): every string literal of
  `$GOROOT/src/path/*_test.go` and `$GOROOT/src/path/filepath/*_test.go`
  through all single-argument functions, every literal tuple as Join, Match
  and Rel (both orders), 1,000 random paths/joins/pairs built from `/`,
  `.`, `..`, `\`, `:`, `é`, `\xff`, ..., and 3,000 random pattern/name pairs
  (classes, ranges, negation, escapes, malformed patterns, invalid UTF-8).
* `tests/fixtures/golden-sample.txt` (7,488 records): 155 file paths from
  the golden seeksnack output and site sources, used as-is, rooted, joined,
  as Rel targets and against glob patterns.
* Scratch corpora: 1,698,946 records (`-n 60000 -seed 9`) and 743,760
  records from all 15,494 golden/site paths — all identical, release and
  debug builds.

### Independent adversarial verification (second agent)

* Oracle mode `-adv N` (`tools/go-oracle/go-path/adversarial.go`): paths
  from dot runs (`...`, `....`), slash runs, Thai/emoji/invalid UTF-8,
  `\`, `C:`, NUL/TAB segments through every single-argument function,
  Join of up to 5 elements, Rel between a path and lexical variants of it
  (both orders, rooted and not), glob patterns with multi-byte and
  malformed classes (`[ก-ฮ]`, `[\xff-a]`, `[]a]`, `[^]`, `[a-]`, `[\`,
  trailing `\`), and a fixed tail: 24 UTF-8 boundary/malformed sequences
  (overlong 2/3/4-byte, surrogates, > U+10FFFF, truncated, every
  `acceptRanges` row) × 24 × 14 patterns × 5 names, and every byte against
  10 patterns × 10 names.
* Scratch runs: seeds 17, 31, 32 (60k each): 7,069,800 records, 0
  mismatches (release; seed 17 also debug).
* `tests/fixtures/adversarial.txt` (15,475 records, `-adv 3000 -seed 5
  -every 16`) is checked in and run by `oracle_adversarial`.
* Mutation check: Match last-chunk rule (2,241 mismatches), `?` vs `/`
  (272), `inrange` (553), getEsc RuneError (29,209), class upper bound
  (525), Rel `..` rule (3,215), IsLocal `hasDots` (102), each
  `utf8.acceptRanges`/`first` row (1,578 to 3,456). Survivors are
  equivalent mutants: the trailing-`*` shortcut (the general path gives
  the same answer) and `out.w > 0`→`> 1` in Clean's `..` branch (`w` is
  either 0 or `dotdot >= 2` there).

## Known gaps

None known for the ported functions.
