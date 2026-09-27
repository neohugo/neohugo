# go-url — porting notes

Port of go1.27.1 `net/url` (plus the `net/netip.ParseAddr` subset that
`parseHost` uses), byte-for-byte: URL fields and results are `Vec<u8>`
(Go strings are bytes: `%ff` unescapes to a raw 0xFF, `RawQuery`/`Opaque`
are kept verbatim), inputs are `impl AsRef<[u8]>`, and error values print
Go's exact `Error()` strings.

## Go file → Rust module map

| Go (go1.27.1 `$GOROOT/src`) | Rust |
|---|---|
| `net/url/url.go` (Error, EscapeError, InvalidHostError, ishex, unhex, shouldEscape, QueryUnescape, PathUnescape, unescape, QueryEscape, PathEscape, escape, URL, User, UserPassword, Userinfo.*, getScheme, Parse, ParseRequestURI, parse, parseAuthority, parseHost, URL.setPath, URL.EscapedPath, validEncoded, URL.setFragment, URL.EscapedFragment, validOptionalPort, URL.String, URL.Redacted, Values.*, ParseQuery, urlParamsWithinMax, parseQuery, Values.Encode, resolvePath, URL.IsAbs, URL.Parse, URL.ResolveReference, URL.Query, URL.RequestURI, URL.Hostname, URL.Port, splitHostPort, URL.MarshalBinary/AppendBinary/UnmarshalBinary, URL.JoinPath, URL.joinPath, validUserinfo, stringContainsCTLByte, JoinPath, URL.Clone) | `src/lib.rs` |
| `net/url/encoding_table.go` + `net/url/gen_encoding_table.go` (encoding consts, table, reference shouldEscape/ishex) | `src/encoding.rs` (table built by a `const fn` port of the generator) |
| `net/netip/netip.go` (ParseAddr, parseAddrError, parseIPv4Fields, parseIPv4, parseIPv6, Addr.Is4) | `src/netip.rs` |
| `strings.Cut`/`Index`/`LastIndex` | private helpers in `src/lib.rs` |
| `path.Join` (JoinPath) | `go-path` crate |
| `strconv.Quote` (error strings) | `go-strconv` crate |

## Public API (for downstream crates)

```rust
go_url::parse(raw) -> Result<Url, Error>                 // url.Parse
go_url::parse_request_uri(raw) -> Result<Url, Error>     // url.ParseRequestURI
go_url::query_escape(s) / path_escape(s) -> Vec<u8>
go_url::query_unescape(s) / path_unescape(s) -> Result<Vec<u8>, Error>
go_url::parse_query(q) -> (Values, Option<Error>)       // map + first error, like Go
go_url::join_path(base, &[elems]) -> Result<Vec<u8>, Error>
go_url::user(name) / user_password(name, pw) -> Userinfo
go_url::encoding_table() -> &[u8; 256]                   // for tests

pub struct Url { scheme, opaque, user: Option<Userinfo>, host, path, fragment,
                 raw_query, raw_path, raw_fragment: Vec<u8>, force_query, omit_host: bool }
Url::string() -> Vec<u8>              // URL.String()
Url::escaped_path() / escaped_fragment() / redacted() / request_uri() -> Vec<u8>
Url::hostname() / port() -> &[u8]
Url::is_abs() / query() -> Values / parse(ref) / resolve_reference(&Url) / join_path(&[elems])
Url::set_path(p) / set_fragment(f) -> Result<(), Error>   // Go's unexported setPath/setFragment
Url::marshal_binary() / append_binary(b) / unmarshal_binary(text)
Url::clone()                          // URL.Clone (derive)
Userinfo::username() / password() -> (&[u8], bool) / string()
pub struct Values(pub BTreeMap<Vec<u8>, Vec<Vec<u8>>>)  // get/set/add/del/has/encode/clone

pub enum Error { Url { op, url, err: Box<Error> }, Escape(Vec<u8>), InvalidHost(Vec<u8>), Other(String) }
// Display == Go Error(): `parse "http://x/%zz": invalid URL escape "%zz"`
```

## Deliberate deviations

* GODEBUG settings are compile-time constants matching the golden binary:
  it is built from a module declaring `go 1.23.0`, so its `DefaultGODEBUG`
  (checked with `go version -m bin/neohugo-go`) contains
  `urlstrictcolons=0` and `urlmaxqueryparams=0`: multiple colons in an
  http(s) host use the last colon for the port, and `ParseQuery` has no
  parameter limit. The oracles live in the same module, so they see the same
  defaults. `IncNonDefault` counters are not ported.
* `encoding_table.go` is rebuilt at compile time from the generator's
  reference `shouldEscape` instead of being copied; the test compares all
  256 entries with the table the oracle parses out of `encoding_table.go`.
* `validUserinfo` ranges over runes in Go; the port checks bytes. Every
  non-ASCII rune (and `RuneError` for invalid bytes) falls into the `default:
  return false` case, so the result is identical.
* `strings.ToLower(scheme)` uses ASCII lowercasing: `getScheme` only returns
  `[a-zA-Z][a-zA-Z0-9+.-]*`.
* Go's error interface is the enum [`Error`]; `fmt.Errorf("invalid host:
  %w", err)` and `fmt.Errorf("invalid port %q after host", p)` are formatted
  eagerly into `Error::Other`.
* `net/netip.Addr` is reduced to what `parseHost` needs (V4 bytes or V6
  bytes + zone, `is4()`); zone interning (`unique.Make`) is not ported.
* No `Display` for `Url` (its bytes may be invalid UTF-8); use `string()`.

## FMA sites

None (no floating-point arithmetic).

## Verification

Oracle: `tools/go-oracle/go-url` (`go run ./tools/go-oracle/go-url -out
crates/go-url/tests/fixtures/url.txt`). Every record runs the real Go code
and stores all 13 URL fields plus String, EscapedPath, EscapedFragment,
Redacted, RequestURI, Hostname, Port, IsAbs, Query().Encode(),
User.String() and MarshalBinary (or the exact error string).

* Inputs: every string literal and literal tuple of
  `$GOROOT/src/net/url/url_test.go` (Parse, ParseRequestURI, Query/Path
  Escape/Unescape, ParseQuery with the full decoded map, Resolve both ways,
  JoinPath), a scheme × userinfo × 59-host grid (IPv6 literals, zones,
  strict colons, `%`-escapes in hosts), random structured URLs with
  mutations, random byte soup (controls, invalid UTF-8, `%` fragments) and
  random `URL` structs (String/ResolveReference/JoinPath on field
  combinations Parse never produces).
* `tests/fixtures/url.txt`: 18,328 records; `tests/fixtures/golden-sample.txt`:
  2,610 records built from 289 URLs extracted from the golden seeksnack
  output (href/src/content/loc values, also resolved against
  `https://seeksnack.com/th/ingredients/` and JoinPath'ed). All identical.
* Scratch corpora (not checked in, `GO_URL_CORPUS=<file> cargo test --release
  -- --ignored`): 469,429 random records (`-n 25000 -seed 5`) and 207,711
  records from all 23,078 golden URLs (`-only-inputs`) — all identical, in release and debug
  (overflow-checked) builds.
* Mutation check: flipping strict-colon handling, the host `%XX < 8` rule,
  the zone space exception, `splitHostPort` port validation, the `*` path
  special case, `OmitHost`, the `./` colon-segment rule, the semicolon error
  overwrite, the resolvePath trailing-slash rule or the IPv4-literal check
  each makes the fixture fail.

### Independent adversarial verification (second agent)

* Oracle mode `-adv N` (`tools/go-oracle/go-url/adversarial.go`): URLs
  assembled from atoms (`%`, `%2`, `%zz`, `%25`, `%2F`, `#`, `?`, Thai,
  emoji, NBSP, spaces, `;`, `@`, `:`, brackets, `\`, invalid UTF-8,
  controls), 23 schemes (incl. malformed `1a:`, `-a:`, `a%41:`), 30
  userinfos, 100 fixed hosts plus randomly built IPv6 literals (0-9 groups,
  ellipsis anywhere, 0-5 hex digits incl. `g`, embedded IPv4 with bad
  octets, `%25` zones with escaped/Thai/space/`%zz` zones); every URL is
  also re-parsed from its `String()` (and that again), resolved against 20
  odd bases (both directions), JoinPath'ed, and paired with random URL
  structs whose RawPath/RawFragment disagree with Path/Fragment. Fixed
  tails: every `%XX` in every component (incl. zones), every byte in 14
  positions, and `ParseQuery` with 9,999-20,000 parameters (confirms
  `urlmaxqueryparams=0` in the oracle build).
* Scratch runs: seeds 11 (20k), 21-28 (30k each) and 99 (2k, with the
  query-limit tail) = 5,552,537 records, 0 mismatches (release; seed 11
  also in a debug/overflow-checked build). All 19 reachable
  `ParseAddr` error messages were hit (`IPv6 field has value >=2^16` is
  unreachable: a fifth hex digit fails first).
* `tests/fixtures/adversarial.txt` (9,034 records, `-adv 1000 -seed 7
  -every 6`) is checked in and run by `oracle_adversarial`.
* Go tables that test unexported functions are ported in `src/lib.rs`
  (`go_internal_tests`: resolvePathTests, shouldEscapeTests,
  TestParseStrictIpv6, TestParseQueryLimits rows valid under
  `urlmaxqueryparams=0`).
* Mutation check on the adversarial corpus: sub-delim/bracket list of
  `validEncoded`, zone space exception, `%2F` OmitHost escape,
  `RequestURI` `//` rule, resolvePath `first`/trailing-slash rules, `*`
  path, Redacted, empty-key skip all fail (2 to 11,623 mismatches);
  survivors (`hasPlus` for non-query modes, the `escape` early return, the
  disabled max-params check) are equivalent mutants.

## Known gaps

* `URL.Clone` is `#[derive(Clone)]`; Go's `(*URL)(nil).Clone()` has no Rust
  equivalent (use `Option<Url>`).
* `Error::Url` has no `Timeout()`/`Temporary()` (no network errors here).
