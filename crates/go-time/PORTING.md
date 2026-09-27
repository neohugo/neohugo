# go-time — porting notes

Port of the Go `time` package from **go1.27.1** (`$GOROOT/src/time`), the
golden toolchain, over the data types in `go-value`
(`go_value::Time`, `go_value::Location`, `Zone`, `ZoneTrans`).

## Go file → Rust module map

| Go file | Rust module | Notes |
|---|---|---|
| `format.go` | `src/format.rs` | layouts, `nextStdChunk`, `appendInt`, `Format`/`AppendFormat`, `Parse`/`ParseInLocation`/`parse`, `ParseError`, `quote`, `parseTimeZone`, `ParseDuration` |
| `format_rfc3339.go` | `src/format_rfc3339.rs` | RFC 3339 fast paths, strict marshal checks, `parseStrictRFC3339` |
| `time.go` | `src/time.rs` | calendar (absolute-day math), accessors, `Month`/`Weekday`/`Duration`, `Add/Sub/AddDate/Truncate/Round/div`, `Date`, `Unix*`, `In/UTC/Local`, `Zone/ZoneBounds/IsDST`, `MarshalBinary`/`UnmarshalBinary` (v1+v2), `MarshalJSON/Text`, `UnmarshalJSON/Text`; `Time.String`/`GoString` (from format.go) |
| `zoneinfo.go` | `src/zoneinfo.rs` | `UTC`, `Local`, `FixedZone` (+ unnamed-hour cache), `lookup`, `lookupFirstZone`, `tzset*` (POSIX TZ rules), `tzruleTime`, `lookupName`, `LoadLocation`, `containsDotDot` |
| `zoneinfo_read.go` | `src/zoneinfo_read.rs` | `LoadLocationFromTZData` (TZif v1/v2/v3), zip reader, `loadTzinfo*`, `loadLocation`, `readFile` |
| `zoneinfo_unix.go` | `src/zoneinfo_unix.rs` | `platformZoneSources`, `initLocal` (`$TZ` handling) |
| `zoneinfo_goroot.go` | `src/zoneinfo_read.rs` (`goroot_zone_source`) | |
| `sys_unix.go` | `src/sys_unix.rs` | open/read/preadn with Go's darwin `syscall.Errno` strings |
| `unicode/utf8` (`DecodeRuneInString`) | `src/utf8.rs` | only what `quote` needs |

Every ported function carries a `// Go: <file>:<Func>` marker.

## Public API (for downstream crates)

```rust
use go_time::{GoTimeExt, Duration, Month, Weekday, Time, Location};

// package functions
go_time::date(y, Month(m), d, h, mi, s, ns, &loc) -> Time      // time.Date
go_time::unix(sec, nsec) / unix_milli(ms) / unix_micro(us)      // Local location, like Go
go_time::now(), since(&t), until(&t)
go_time::parse(layout, value) -> Result<Time, ParseError>       // layout/value: impl AsRef<[u8]>
go_time::parse_in_location(layout, value, &loc)
go_time::parse_duration(s) -> Result<Duration, TimeError>
go_time::load_location(name) -> Result<Arc<Location>, TimeError>
go_time::load_location_from_tz_data(name, &bytes)
go_time::fixed_zone(name, offset_secs) -> Arc<Location>
go_time::utc(), go_time::local(), go_time::set_local(loc)       // time.UTC, time.Local, `time.Local = loc`
go_time::location_string(Option<&Arc<Location>>)                // (*Location).String
go_time::unmarshal_binary / unmarshal_json / unmarshal_text(&mut Time, &[u8])
constants: RFC3339, RFC3339_NANO, RFC1123Z, ANSIC, KITCHEN, DATE_TIME, ... (all of Go's)

// time.Time methods
t.format(layout) -> String, t.format_bytes(&[u8]) -> Vec<u8>, t.append_format(&mut b, layout)
t.string()        // Go String(): "2006-01-02 15:04:05.999999999 -0700 MST"
t.go_string()     // Go GoString()
t.date(), year(), month(), day(), weekday(), iso_week(), clock(), hour(), minute(),
  second(), nanosecond(), year_day()
t.add(Duration), sub(&u), add_date(y,m,d), truncate(d), round(d)
t.utc(), t.local(), t.in_loc(&loc), t.go_location(), t.zone(), t.zone_bounds(), t.is_dst()
t.go_unix(), unix_milli(), unix_micro(), go_unix_nano(), go_is_zero()
t.go_equal(&u), go_before(&u), go_after(&u), compare(&u)
t.marshal_binary(), append_binary(&mut b), marshal_json(), marshal_text(), append_text(b)

// Duration: string(), Display, hours()/minutes()/seconds() (f64), milliseconds(),
// microseconds(), nanoseconds(), truncate(m), round(m), abs(); consts NANOSECOND..HOUR
// Month / Weekday: string(), Display, consts JANUARY.., SUNDAY..
```

Methods whose names clash with inherent `go_value::Time` methods are exposed
as `go_*` (`go_unix`, `go_is_zero`, `go_equal`, `go_before`, `go_after`,
`go_location`, `go_unix_nano`) with Go's exact semantics.

For the cast layer (`cast.ToTimeInDefaultLocationE` / `internal.ParseDateWith`):
`parse` (uses `Local`), `date`, `Time::date()/clock()/nanosecond()`, `unix`,
`parse_duration` are all that is needed; `tests/oracle.rs::seeksnack_fixture`
emulates `ParseDateWith` with exactly these primitives.

## Representation

* `go_value::Time.unix_sec` is Unix seconds; Go stores internal seconds since
  year 1 (`ext`). They differ by a wrapping add of `unixToInternal`, a bijection
  on `i64`, so all of Go's arithmetic on `sec()` (including its saturation in
  `addSec` and the wrap in `Unix()`) is reproduced exactly (`time::sec`/`set_sec`).
* `loc == None` is Go's nil location (UTC). Every constructor normalises
  through `set_loc` (Go `setLoc`), which maps `&utcLoc` to `None`.
* **UTC identity**: the shared `go_time::utc()` Arc, go_value's shared
  `go_value::Location::utc()` Arc (also when it is `Local`, like Go's
  `time.Local = time.UTC`), *or* any other zone-less location named "UTC"
  that is not `Local` (Go cannot build such a location other than `utcLoc`;
  initLocal's UTC fallback is `&localLoc`, i.e. Local, not UTC).
* **Local identity**: `Arc::ptr_eq` with the current `go_time::local()`.
  `local()` is initialised lazily from `$TZ` exactly like `initLocal`
  (unset → `/etc/localtime` named "Local"; "" / "UTC" / failures → UTC;
  ":foo"; absolute paths). `set_local` mirrors assigning `time.Local`.
  On the golden machine `Local` is `/etc/localtime` = Asia/Bangkok, which is
  what makes `+07:00` offsets parse into `Local` with abbreviation `+07`.

## Deliberate deviations

1. **Monotonic clock not modelled** (`go_value::Time` has no monotonic
   reading). `now()` returns a wall-clock time; `String()` never appends
   ` m=±…`; comparisons/`Sub` always use wall time. Go only prints `m=` for
   values from `time.Now()` (nondeterministic output anyway).
2. **Location lookup cache not modelled** (`cacheStart/cacheEnd/cacheZone`).
   Go fills it from the *wall clock at load time* and it memoises `lookup`
   for "now". It is transparent for sorted transition tables except when the
   last TZif transition falls inside the current year and a lookup precedes
   it within the cached tzset range; with *unsorted* transition times (only
   malformed/synthetic TZif data, or 32-bit v1 times that wrap) Go's cached
   zone wins over the binary search and results differ. The verification
   pass reproduced both (a mutated TZif whose last transition is 2026-09-20
   with a DST footer: Go answers 2026-05-10 from the cached footer rule, the
   table says otherwise); the oracle skips lookups on unsorted tables and
   inside such a cache window, so fixtures do not depend on the generation
   date. A scan of
   all 604 files of this machine's `/usr/share/zoneinfo` (tzdata 2026c) found
   no zone whose last transition is in 2026 with a DST rule, and every zone
   matches Go (`tests/zoneall.rs`).
3. **Zone offsets are `i32`** (`go_value::Zone::offset`; Go uses `int`).
   `fixed_zone` with |offset| ≥ 2^31 truncates. TZif offsets are int32 in Go
   as well; `Parse`/`UnmarshalBinary` offsets are bounded.
4. **Non-UTF-8 bytes** cannot be stored in `go_value::Location`/`Zone`
   (Rust `String`s). Real tzdata is ASCII; parsed zone names are ASCII by
   construction (`parseTimeZone`).
   * TZif abbreviations: converted lossily (U+FFFD); only the printed name
     differs (no lookup/parse semantics depend on it).
   * TZif footer (`extend`): each byte of an invalid UTF-8 sequence becomes
     the single byte `?` (`zoneinfo_read.rs:extend_string`). Go's `tzset`
     ranges runes, so such a byte is a width-1 `RuneError` that is inert for
     the TZ grammar; `?` keeps every byte offset and therefore `tzset`'s
     accept/reject decisions, offsets, DST flags and bounds identical. Only
     zone names built from those bytes differ (`?` instead of the raw byte).
     (A lossy U+FFFD conversion was a real bug: it shifted `tzsetName`'s
     `i < 3` check and accepted rules Go rejects.)
   * `$TZ`, `$ZONEINFO`, `$GOROOT`: `$TZ` is converted lossily (a non-UTF-8
     path is then not found, Go would open it); a non-UTF-8 `$ZONEINFO` or
     `$GOROOT` is treated as unset.
5. **`runtime.GOROOT()`** is only the `GOROOT` environment variable (Go falls
   back to the build-time GOROOT); it only matters for zones missing from the
   system zoneinfo. `load_location_env` / `init_local_from_env` take explicit
   values (used by tests).
6. **Embedded tzdata** (`time/tzdata`) is not supported: neohugo imports it
   only on Windows.
7. **Corrupt zip central directories** return `corrupt zip file …` where Go
   would panic on an out-of-range slice.
8. **`UnmarshalBinary` with an out-of-range nanosecond field**: negative
   values are emulated exactly (Go's sign-extension sets `hasMonotonic` and
   `stripMono` replaces the seconds); for values ≥ 2^30, Go keeps bit 30 in
   the unused wall-seconds field, which only affects `IsZero`/`==` (not
   modelled).
9. `unix()` (Go `unixTime`) resolves `Local` eagerly instead of on first use.
10. `GobEncode`/`GobDecode` are not separate functions (they are
    `marshal_binary`/`unmarshal_binary`). Timers, tickers, `Sleep` and the
    Windows/Android/iOS/Plan 9 zone sources are out of scope.

## FMA sites

None. `go tool objdump` of the oracle binary shows **0** `FMADD/FMSUB/FNMADD/FNMSUB`
in `time.*`; the float code is `Duration.Hours/Minutes/Seconds`
(`FDIVD`+`FADDD`, no multiply) and `ParseDuration`'s
`uint64(float64(f) * (float64(unit) / scale))` (`FDIVD`, `FMULD`, `FCVTZUD`:
the product is converted, not added).

## Tests

* `tests/oracle.rs` — differential tests against fixtures generated by
  `tools/go-oracle/go-time` (go1.27.1). Local is pinned to Asia/Bangkok
  (named "Local") from a checked-in TZif file, so fixtures are machine
  independent. Counts (checked-in fixtures):
  format 201,994 · parse 54,112 · duration 36,533 · calendar 29,044 ·
  binary 40,625 · zones 71,344 · tzset 22,222 · seeksnack 581 · zip 7 ·
  system 49 · localenv 40 — all matching.
  * format: layouts × instants (years −100001…292277026596, extremes
    ±2^63) × UTC/Local/fixed/26 IANA zones, plus `String()`/`GoString()`.
  * parse: Go's parse/error tables, round trips of every layout, and random
    mutations (error messages compared byte for byte), `Parse` and
    `ParseInLocation`.
  * calendar: `Date/Clock/YearDay/Weekday/ISOWeek/Unix*/IsDST/ZoneBounds`,
    `AddDate`, `Add`, `Sub/Compare/Before/After/Equal`, `Truncate/Round`,
    `Date()` normalisation including DST gaps and extreme values.
  * binary: `MarshalBinary` (v1/v2), `MarshalJSON/Text` (+ errors),
    `UnmarshalBinary` of valid and random/crafted bytes,
    `UnmarshalJSON/Text`.
  * zones/tzset: every transition ±1 s of 26 zones to 2450; synthetic TZif
    files with 42 extend strings (valid and malformed) incl. `Date()` through
    tzset rules; malformed/truncated TZif data.
  * zip: `$ZONEINFO` pointing at a stored/deflated zip.
  * system/localenv: `LoadLocation` against `/usr/share/zoneinfo`
    (errors such as `is a directory`, `not a directory`, `file name too long`,
    `malformed time zone information`, `invalid argument`; names with `//`,
    `./`, NUL, backslashes, 400-byte paths) and `Local` from 40 `$TZ` values
    (`:`-prefixed, absolute, relative with `..`, directories, `/dev/null`,
    POSIX TZ strings, case-folded names); skipped automatically when the
    system files' SHA-256 differ from the fixture.
  * seeksnack: every date-like string in the site's `content/` and `data/`,
    run through cast's 24 `TimeFormats` with `time.Parse` and rendered with the
    site's layouts and `String()`.
* Larger corpus: `oracle -big DIR` writes 20× format/parse/calendar
  fixtures; `GO_TIME_FIXTURES=DIR cargo test --release` ran 1,289,554 format,
  417,487 parse and 117,394 calendar checks with 0 mismatches.
* `tests/adversarial.rs` — `adv.txt` from `tools/go-oracle/go-time/adv.go`
  (verification pass; 66,066 records, 81,252 checks, all matching):
  random layouts built from every std chunk and near-miss literals
  (`Janet`, `Mont`, `_2006`, `Z07:0`, `.9x`, non-ASCII/invalid bytes…) with
  `Format`/`AppendFormat`, round-trip `Parse`/`ParseInLocation` and mutated
  values; value fuzz against all named/custom layouts; zone abbreviations and
  offsets of the 26 fixture zones through `ParseInLocation` (`lookupName`,
  1800–2120); 31 extra fixed zones (±1 s … ±2^31 offsets, names `é`,
  `a"b\c`, `UTC`, `Local`) through `Format`/`String`/`GoString`/
  `MarshalBinary`/`UnmarshalBinary`; crafted `UnmarshalBinary` v1/v2 blobs;
  synthetic TZif (v1/v2/v3, slim/fat, 0/1/27 leap-second records, isstd/isut
  tables, random TZ footers from the tzset grammar incl. out-of-range numbers,
  invalid-UTF-8 footers) and byte-mutated real TZif data; Go's slim testdata
  files; `Date`/`AddDate`/`Truncate`/`Round` with extreme arguments (incl.
  internal seconds `MinInt64`); `ParseDuration` fuzz.
* `tests/zoneall.rs` — every zone of `/usr/share/zoneinfo` (604 files: the
  source the golden build loads zones from) and of
  `$GOROOT/lib/time/zoneinfo.zip` (598 zones), compared through SHA-256
  digests of ~1.5M lines of lookups at every transition ±1 s 1800–2080,
  far instants, and `Date()` for wall-clock times around each transition
  (DST gaps/overlaps) rendered with `MST -07:00:00`. Skips files whose
  hashes changed. 1,202/1,202 match.
* `tests/regressions.rs` — Go-derived expectations for the bugs fixed by the
  verification pass.
* `tests/go_tables.rs` — ports of Go's `format_test.go`/`time_test.go`/
  `zoneinfo_test.go` tables (Local = America/Los_Angeles named "Local", as in
  Go's tests), including `TestTimeAddSecOverflow`, `TestTimeWithZoneTransition`,
  `TestZoneBounds`, `TestTimeIsDST`, `TestLoadFixed`, `TestAddToExactSecond`,
  `TestDefaultLoc`, `TestVersion3`, `TestFirstZone`, `TestLocationNames`,
  `TestEarlyLocation`, `TestMalformedTZData`, `TestLoadLocationFromTZDataSlim`,
  `TestLoadLocationValidatesNames`, `TestBadLocationErrMsg` (zone files
  exported by the oracle into `tests/fixtures/zoneinfo`).
* Unit tests in `src/` — `TestNextStdChunk`, `TestAppendInt`, `TestQuote`,
  `TestParseTimeZone`, `TestTzset*`.
* Golden cross-check (manual, `examples/seeksnack_dates.rs`): all 129
  distinct page dates render to strings found verbatim in the golden output
  (ISO form in HTML and sitemaps 129/129; `String()` form, JS-escaped, in
  JSON-LD for all 125 regular pages).

Regenerate fixtures (from the repo root; the output is deterministic except
for the machine-dependent, hash-gated system files):

```sh
go build -o /tmp/go-time-oracle ./tools/go-oracle/go-time
env -u ZONEINFO SEEKSNACK=<pristine seeksnack dir> /tmp/go-time-oracle -out crates/go-time/tests/fixtures
# larger corpora outside the repository (then GO_TIME_FIXTURES=<dir> cargo test --release):
/tmp/go-time-oracle -out <scratch> -big <dir> -advscale 20     # everything ×20
/tmp/go-time-oracle -advonly <dir> -advscale 20 [-zonetext <dir>]  # adversarial only (+ per-zone texts)
```

## Verification pass (independent adversarial review)

Side-by-side review of every ported function against go1.27.1 plus the new
fuzz/adversarial oracle. Findings:

1. **Fixed — `div` overflow** (`time.rs`, used by `Truncate`/`Round`): when
   Go's internal seconds are `MinInt64` the negation wraps, `r` becomes
   negative and Go's `r = d - r` wraps; the port used a plain `-`, which
   panicked in debug builds (release wrapped correctly). Example:
   `unix(9223371974719179008, 0).utc().truncate(Duration(9223372036000000000))`
   → Go `9223371983087775237 s + 709551616 ns`. Now `wrapping_sub`;
   regression in `tests/regressions.rs` and `adv.txt`.
2. **Fixed — non-UTF-8 TZif footer** (`zoneinfo_read.rs`): footer
   `IST-5\xcd30` (a mutated `Asia/Kolkata`) — Go rejects the rule (the DST
   name `\xcd` is one byte before the digit), the lossy U+FFFD conversion
   made it three bytes and the port applied a DST rule (`-108000`, DST) for
   all times after the last transition. See deviation 4.
3. **Fixed — `set_local(go_value::Location::utc())`** (Go:
   `time.Local = time.UTC`): the go_value UTC Arc was only recognised as
   `&utcLoc` structurally and not when it is Local, so times from `unix()`
   marshalled with zone offset `0000` instead of Go's `ffff` (-1, UTC) and
   `GoString` said `time.Local` instead of `time.UTC`. Regression in
   `tests/regressions.rs`.
4. Not fixed (by design, documented): Go's wall-clock-dependent lookup cache
   with unsorted transition tables (deviation 2); non-UTF-8 zone names
   (deviation 4).

No other discrepancy was found: 0 FMA sites re-confirmed with
`go tool objdump` (Duration float methods are `FDIVD`+`FADDD`; ParseDuration's
product is converted before the add); all integer arithmetic re-checked for
Go wrap-around vs Rust debug overflow (only `div` differed). Final
statistics: checked-in fixtures 537,803 differential checks (456,551 in
`oracle.rs` + 81,252 in `adversarial.rs`) + 1,202 zone digests + 30 ported
Go test functions; large corpora (`-big -advscale 20`) 1,161,878 adversarial
+ 1,995,836 format/parse/calendar/duration/binary/zones/tzset/system checks,
all matching in both release and debug builds; a scale-100 stress corpus
(`-advonly <dir> -advscale 100`, 4.69M records) ran 5,705,056 checks with 0
mismatches.

Dev-dependencies: `flate2` (gunzip of fixtures; decompression only) and
`sha2` (hashing system zoneinfo files) — both on the allowed list, test-only.

## Requests for `go-value` (not edited here)

* `Time::before/after` compare `(unix_sec, nsec)`; Go compares internal
  seconds, which differs when `unix_sec + unixToInternal` wraps (|unix| near
  2^63). Use `GoTimeExt::go_before/go_after/compare` for Go semantics.
* `Time::from_unix` uses `sec += n` (panics on overflow in debug; Go wraps)
  and takes an explicit location; `go_time::unix` is the Go-exact constructor.
* `Location::utc()` builds a new `Arc` on each call; prefer `go_time::utc()`
  (shared), although zone-less "UTC" locations are recognised structurally.
* `PartialEq for Time` compares locations structurally when the pointers
  differ; Go compares `*Location` pointers (two separately created
  `FixedZone("X", 3600)` are `!=` in Go).
* Optional: `Zone::offset` as `i64` to match Go's `int`.
* Optional: `Zone::name` and `Location::extend`/`name` as byte strings
  (`Vec<u8>`/`GoString`) so non-UTF-8 TZif abbreviations, footers and `$TZ`
  names round-trip exactly (deviation 4).
* Optional: room for Go's per-Location lookup cache
  (`cacheStart/cacheEnd/cacheZone`, filled at load time) if exact emulation of
  deviation 2 is ever wanted; real tzdata does not need it.
