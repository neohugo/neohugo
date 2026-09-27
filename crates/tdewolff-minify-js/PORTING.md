# tdewolff-minify-js — porting notes

Byte-exact port of `github.com/tdewolff/minify/v2@v2.23.8/js` (the version
pinned in neohugo's `go.mod`): the JavaScript minifier — variable hoisting
(`hoistVars`), renaming (`renamer`), the statement-list and expression
optimizers and the printer (`jsMinifier`). It walks and mutates the AST of
`crates/tdewolff-parse-js` (parse/v2@v2.8.1/js) and uses
`crates/tdewolff-minify` (`M`, `Minifier`, `Writer`, `number`, `param_is`),
`crates/tdewolff-parse` (`GoBytes`, `Input`, `buffer::Reader`,
`strconv::len_int`), `go-sort` (Go's `sort.SliceStable`), `go-strconv`
(`ParseInt(s, 16, 32)`) and `go-unicode` (`utf8.RuneLen`/`EncodeRune`).
Golden toolchain: go1.27.1 darwin/arm64 (no platform-dependent code on this
path, see *FMA*).

## Go file → Rust module map

| Go (`minify/v2@v2.23.8/js/…`) | Rust |
|---|---|
| `js.go` `Minifier`, `Minify`, `Minifier.Minify`, `minVersion` | `src/lib.rs` (`Minifier`, `minify`, `Minifier::minify`, the `tdewolff_minify::Minifier` impl) |
| `js.go` `jsMinifier` and all its methods, `blockType`, `expectExpr` | `src/js.rs` (`JsMinifier`, `BlockType`, `ExpectExpr`, `minify`; the `CallExpr` and `BinaryExpr` cases of `minifyExpr` are split out as `minify_call_expr`/`minify_binary_expr`) |
| `stmtlist.go` | `src/stmtlist.rs` (`optimize_stmt`, `optimize_stmt_list`) |
| `util.go` (byte constants, predicates, precedence maps, `optimizeCondExpr`, `optimizeUnaryExpr`, `mergeBinaryExpr`, `minifyString`, `replaceEscapes`, `minifyRegExp`, number literals) | `src/util.rs` (`optimizeCondExpr` is a free function taking `minVersion(2020)`) |
| `vars.go` (`renamer`, `hasDefines`, `bindingVars`, `addDefinition`, `mergeVarDecls`, `mergeVarDeclExprStmt`, `countHoistLength`, `hoistVars`) | `src/vars.rs` |
| `util_test.go`, `js_test.go:TestRenamerIndices` | `src/unit_tests.rs` |
| `js_test.go` (`TestJS`, `TestJSVarRenaming`, `TestJSVersion`, `TestReaderError`, `TestWriterError`, `ExampleMinify`), `html/html_test.go:TestHTMLCSSJS` | `tests/upstream.rs` + generated `tests/upstream_tables/mod.rs` |

Every ported function carries a `// Go: <file>:<Func>` comment.

## Public API

```rust
use tdewolff_minify_js::{minify, Minifier};

// Go: js.Minifier{Precision, KeepVarNames, Version} (+ the unexported
// useAlphabetVarNames as a #[doc(hidden)] pub field for the upstream tests)
let o = Minifier { version: 2022, ..Default::default() };   // neohugo's default
o.minify(&m, &mut w, &mut r, params)?;   // Go (*Minifier).Minify(m, w, r, params)
minify(&m, &mut w, &mut r, params)?;     // Go js.Minify (zero Minifier)
let out = o.minify_bytes(b"var a = 1", None)?; // convenience (private copy)
```

* `params` is `Option<&Params>` (`None` = nil map); `params["inline"] == "1"`
  parses in inline mode (the HTML minifier's `on*` attributes).
* `Minifier` implements `tdewolff_minify::Minifier` (`Send + Sync`), so it
  registers in an `M` exactly like Go's `m.Add`/`m.AddRegexp`:
  neohugo's `minifiers.New` is `m.add("text/javascript", js.clone());
  m.add_regexp(Regexp::must_compile("^(application|text)/(x-)?(java|ecma)script$"), js)`.
* Readers are `tdewolff_parse::GoReader`; with a `buffer::Reader` (Go
  `buffer.NewReader`) the minifier works **in place** on the caller's bytes,
  exactly like Go (see below).

### Integration with `crates/tdewolff-minify`

No change to `tdewolff-minify` is needed: its HTML minifier calls
`m.minify_mimetype(b"application/javascript", …)` for `<script>` contents
and `on*` attributes (with `inline=1`), which reaches this minifier through
the regexp registration; errors come back as `GoError::Parse` and go through
`update_error_position` like any other nested minifier's. The `html`
fixture (below) runs 1,589 HTML documents through the full neohugo `M`
(tdewolff-minify's HTML/CSS/JSON/SVG/XML + this JS minifier) and matches Go,
and `tests/upstream.rs` runs html_test.go's `TestHTMLCSSJS` (the 2 rows
tdewolff-minify could only check without JS) with the real JS minifier.

## Deliberate deviations (API shape only; bytes are identical)

* **Arena instead of pointers.** Go's `*js.Var`, `js.IExpr`, … are
  `NodeId`s into the parse-js arena; the minifier's in-place mutations
  (`expr.Op = …`, `stmt.Body.List = …`, `lit.Data = lit.Data[:n-1]`,
  `binary.X = nil`, `callExpr.Optional = true`, `decl.TokenType =
  ErrorToken`, `v.Uses--`, `Scope.Declared = Declared[1:]`, …) are applied to
  the arena at the same points. New Go nodes (`&js.GroupExpr{…}`,
  `&js.BinaryExpr{…}`, the `null` literal of `a==null`, the block of
  `minifyStmtOrBlock`) are new arena nodes.
* **Slices are `Vec`s.** Go statement/declaration/comma lists are slice
  headers; the port stores `Vec`s in the nodes and moves them in and out
  (`block.List = optimizeStmtList(block.List, …)` is take → optimize → put
  back). This is equivalent because no two *live* lists share a backing
  array: every Go site that could write through shared capacity was checked
  (`append(list[:i+1], append(block.List, list[i+1:]...)...)` splices a
  detached else-block; `right.List = append(left.List, right.List...)` and
  `src.List = src.List[:0]` leave only dead lists behind;
  `decl2.List = append(decl2.List[:i], decl2.List[i+1:]...)` inside
  `mergeVarDecls`'s loop is re-read through the field in both; `range`
  loops evaluate the header once, the port copies the list at the same
  moment, and no list is modified while it is being printed).
* **`renameScope(scope js.Scope)`** takes the scope by value in Go; only the
  shared `Declared` array (sorted in place) and the `*Var`s are written, so
  the port passes the `ScopeId`.
* **Module scope.** parse-js has a single module scope (see its PORTING.md);
  `hoistVars(&ast.BlockStmt)` reads its `VarDecls` before anything mutates
  them, afterwards only `decl.Scope.Func` is used — the same values.
* **`useAlphabetVarNames`** is Go-unexported; it is a `#[doc(hidden)] pub`
  field so struct-update syntax keeps working and the upstream tests can set
  it.
* **`Minifier::minify_bytes`** is a convenience that Go does not have.
* **Write errors** are ignored until the final `w.Write(nil)`, as in Go
  (`tdewolff_minify::Writer::write(&[])`).
* **Panics.** Where Go would panic (index out of range on `Declared[1:]` /
  `Declared[NumForDecls:]`, a nil `BindingObjectItem.Key`, writing an empty
  token), the port panics too (explicit `assert!`/`expect`). None was
  reachable in ~490k fuzzed and generated inputs (Go recorded no panic
  either). Writing into a package-level `[]byte` global would panic in the
  port (`GoBytes::from_static`); Go never does it on this path.
* **Stack depth.** The parser and the printer are recursive like Go's. The
  parser's limits (1000 nested statements, 1000 nested expressions) are per
  function body, so nesting multiplies across function expressions; Go grows
  its stacks up to 1 GB, Rust does not. Measured (release, parse + minify):
  990-deep binary/conditional/call/arrow chains need < 1 MiB, 5 nested
  functions × 900-deep parentheses 4 MiB, 20 × 900 16 MiB; real code
  (jquery, typescript.js) < 64 KiB. Callers minifying untrusted input should
  run on a thread with a large stack (the tests use 1 GB virtual).

## Go semantics that needed care

* **In-place writes to the input** (`GoBytes` aliasing, so every alias sees
  them, as in Go): the renamer's `name[0] = c` / `name[:n]` within capacity
  (`getName`), `minifyString` rewriting the quote bytes, `replaceEscapes`
  shifting bytes and `append(append(b[:i], '\\'), b[i:]...)` growing into
  the input's spare capacity, `removeUnderscoresAndSuffix`,
  `binary/octal/hexadecimalNumber` writing decimal digits over the literal
  and `append(b, 'n')`, `minify.Number` (tdewolff-minify), `minifyRegExp`'s
  `append(b[:i], b[i+1:]...)`, `DirectivePrologueStmt` quotes. The fixtures
  compare the whole input buffer (`buf[:cap]`) after every call.
* **The NUL sentinel is not restored**: Go's `js.Minify` calls
  `parse.NewInput(r)` without `defer z.Restore()`, so when the input has spare
  capacity the byte after it stays 0 — inside the HTML minifier's buffer this
  is the `<` of `</script>`. The port does the same (the `embedded` and
  `html` fixtures check the surrounding buffer).
* **`sort.SliceStable` with a position-dependent less** (`minifyVarDecl`:
  `j != 0 || ok`): ported with `go_sort::sort::slice_stable`, whose less
  receives the current slice and indices, so the insertion-sort blocks of 20
  and SymMerge calls are Go's. `renameScope`'s `sort.Sort(VarsByUses)` is
  Go's pdqsort (`Ast::sort_vars_by_uses`), so tie order matches.
* **Go map zero values**: the precedence maps return `OpExpr` for missing
  keys; `identOrder[c]` is 0 for bytes not in the alphabet.
* **`js.IsIdentifierContinue/End`** only decode the first/last rune; the
  port passes the first/last ≤4 bytes of a `GoBytes` (identical results,
  including `RuneError` cases).
* **Integer semantics**: `n*2 + int64(c-'0')` etc. with `wrapping_*`; octal
  escapes `num*8 + b - '0'` in `byte` arithmetic; `strconv.ParseInt(s, 16,
  32)` via go-strconv.
* **`hasSideEffects(nil)` is `true`** (a nil interface matches no case) — so
  an untagged template always has side effects; the port's `Node::Nil` falls
  into the same default arm.
* **`mergeBinaryExpr`'s recursion limit** (`50 < len(strings)` checked at the
  top of each iteration) can return after `left.X = nil` was set, leaving a
  nil `X` deeper in the tree that the printer later skips — ported as is.
* **Hugo configuration**: neohugo uses `js.Minifier{Version: 2022}`
  (`minifiers/config.go`), `KeepVarNames`/`Precision`/`Version` are
  user-configurable; the fixtures cover `v2022`, `v2022-inline`, the zero
  Minifier, `keep`, `keep-alpha`, `alpha`, `keep-alpha-v2019/v2014`,
  `alpha-v2019`, `p3-v2022`.

## FMA

None. `go tool objdump` of a darwin/arm64 build of the oracle shows no
`FMADD`/`FMSUB`/`FNMADD`/`FNMSUB` — in fact no float instruction at all — in
`minify/v2/js`, `minify.Number` and the `parse/v2/js`/`parse/v2/strconv`
functions this path links. The output is platform-independent, so fixtures
generated on linux/amd64 are valid. Re-checked by the red-team on a
linux/arm64 build (`CGO_ENABLED=0 GOARCH=arm64`, go1.27.1): the only fused
instructions in the binary are in `math`, `compress/flate`, the runtime,
`parse/v2/css.HSL2RGB` and `minify/v2/css.rgbToToken`; the float-register
instructions in `minify/v2/js` and `parse/v2/js` are `FMOVD`/`FMOVQ` struct
copies. `parse/v2/strconv.ParseFloat` (FMA-sensitive on arm64) is linked
for the CSS/SVG minifiers only; the JS path never calls it. The linux/arm64
oracle under `qemu-aarch64-static` reproduces every checked-in fixture byte
for byte, and its red-team record files are identical to the amd64
oracle's. (The `html` fixture runs neohugo's whole `M`, so a document with
CSS colours would be FMA-sensitive through the CSS minifier: regenerate it
with `ARCH=arm64`.)

## Tests and parity evidence

Oracle: `tools/go-oracle/tdewolff-minify-js` (package main in the neohugo
module; gofmt, `go vet` (go1.25 and go1.27) and golangci-lint clean;
`gen.sh` regenerates everything: `SCRATCH=… gen.sh`, or on a linux/amd64
host `ARCH=arm64 SCRATCH=… gen.sh`, which builds a linux/arm64 oracle with
`CGO_ENABLED=0 GOARCH=arm64` and runs it under `qemu-aarch64-static`; the
checked-in fixtures come from that arm64 build). Modes: `stdin CFG`,
`fixtures DIR SCALE SEED`, `tables OUT.rs`, `corpus OUT.tsv CFGS ROOT…`,
`sample N SEED`, and the red-team modes `gen KIND OUT.rec.gz N SEED`
(`redteam.go`, `redteam_logic.go`: `lits`, `logic`, `num`, `soup`, `prog`,
`corpus`, `corpus:LISTFILE`, `html`; digests), `files OUT.rec.gz CFGS
LISTFILE` (whole files × configurations; `html` wraps the file in a
`<script>` through neohugo's `M`) and `enumerate OUT CFGS ALPHA MAXLEN`
(`enumerate.go`: every sequence of ≤ MAXLEN symbols of a byte or token
alphabet; one combined digest per input, the Rust side enumerates the same
inputs). A configuration name is `-`-separated tokens (`vN`, `pN`, `keep`,
`alpha` — set through reflect/unsafe —, `inline`, or `0`);
`tests/common/mod.rs` parses the same names. `examples/minify.rs` is the
Rust twin of `stdin` (`cargo run --release --example minify -- v2022 <
in.js`); `examples/check.rs` checks any record file (`cargo run --release
--example check -- /abs/FILE.rec.gz DIFFDIR`, differing inputs go to
DIFFDIR) and `examples/enumerate.rs` an `enumerate` file.

Checked-in fixtures (2.9 MB, gzip, length-prefixed binary-safe records in
`tests/fixtures/`, read with `flate2`, decompression only). Each record
compares the output, the error string (`err.Error()`, positions included)
and the input buffer after the call:

| fixture | content | records |
|---|---|---|
| `literals` | every string literal of the upstream minify/js + parse/js tests, hand-written cases (seeksnack's 6 recorded inline scripts/handlers, Hugo's embedded google_analytics/disqus scripts rendered, aliasing/hoisting/merging/escape/number edge cases, branches found uncovered by coverage) and 392 `!(X op Y)` shapes, × 9 configurations, full outputs | 31,932 |
| `embedded` | literals minified from a window of `<script>…</script>` / `onclick="…"` buffers (NUL sentinel and in-place writes land in the surrounding bytes) | 1,182 |
| `html` | HTML documents through neohugo's full `M` (literals in scripts and `on*` attributes, raw and entity-escaped) and html_test.go's literals (also through TestHTMLCSSJS's `M`) | 1,589 |
| `adversarial` | long `var` lists (SliceStable path), up to 3,600 locals with tied uses (pdqsort, multi-character names), deep nesting at and over the parser limits, 20,000-term chains, long escapes | 216 |
| `grammar` | generated programs (grammar generator adapted from the parse-js oracle, biased to function scopes, declarations, if/return/throw merging and rewritten expressions; 85% parse) — digests | 2,000 |
| `fuzz` | mutated literals and mutated programs — digests | 18,000 |
| `repo` | the neohugo repository's 28 JS files (578 KB) × 4 configurations — digests | 112 |
| `corpuswin` | mutated windows (≤16 KiB) of the module-cache JS corpus — digests | 3,000 |
| `redteam` | the red-team generators: literal-heavy programs (strings with every escape form, `</script>`, quotes, `${`; templates; numbers in every base with separators, BigInt and huge/tiny exponents; regexps) in the contexts that rewrite them, logic trees (`?:`, `&&`/`\|\|`/`??`, `!`, `==null`/`=== void 0` pairs, `typeof` comparisons) in if/return/throw-merging statement contexts, numbers at precisions 0–21, token soup, generated programs, and HTML documents with these scripts (also byte-mutated, for error positions) — 28 configurations (all versions, precisions, keep/alpha/inline), digests | 6,000 |
| `enum-min` | every sequence of ≤ 3 of 29 statement/expression tokens (`var a`, `if(a)`, `else `, `return a`, `a?b:c`, `void 0`, `a==null`, `try{}catch(e)`, `x:`, …) through `v2022` and `keep-inline` — combined digests | 25,259 |

* `tests/upstream.rs`: `TestJS` (722 rows), `TestJSVarRenaming` (52),
  `TestJSVersion` (3 × 5 versions), `TestHTMLCSSJS` (6), `TestReaderError`,
  `TestWriterError`, `ExampleMinify` (registered in an `M` like neohugo; the
  parse-error string with its position context). The tables are extracted
  literally from the Go test files by the oracle, which first checks that
  every row passes in Go 1.27.1.
* `src/unit_tests.rs`: `TestBinaryNumber`, `TestOctalNumber`,
  `TestHexadecimalNumber`, `TestString` (52 rows), `TestHasSideEffects` (23),
  `TestRenamerIndices` (100,000 names), plus a `getName` aliasing check.
* `tests/corpus.rs` (`#[ignore]`, reads the Go module cache): all 83 JS files
  (27.5 MB: tdewolff's `_benchmarks` — typescript 10 MB, antd 6.7 MB,
  echarts, victory, ace, jquery-ui, jquery, moment — and upstream test corpus,
  esbuild@v0.25.6 scripts, x/tools@v0.34.0, the repo) × 6 configurations
  against `tests/fixtures/corpus.tsv`.

Results (2026-09-27, linux/amd64; debug-with-overflow-checks and release):

* checked-in fixtures 58,031/58,031 checks (89,290 with the red-team's
  `redteam` and `enum-min`); upstream 7 tests (783 table rows) and 7 unit
  tests pass; `cargo clippy --all-targets` and `cargo fmt --check` clean.
* 20× sets (`TDEWOLFF_MINIFY_JS_FIXTURES=… cargo test --release --test
  fixtures`), seeds 7 and 11, each: literals 31,932, embedded 1,182, html
  1,589, adversarial 216, grammar 40,000, fuzz 360,000, corpuswin 60,000,
  repo 112 — 495,031 checks per seed, all identical to Go (also in the
  debug profile with overflow checks).
* corpus: 498/498 (83 files × 6 configurations, 27.5 MB) identical.
* line coverage of `src/` by the checked-in tests: 97% (llvm-cov); the
  uncovered lines are unreachable after the parser's `WhileToFor` (every
  `WhileStmt` path), Go type-assertion panics, the `undefined` case with
  `prec > OpMember` (no caller passes `OpPrimary`), the arrow-body comma and
  empty-return paths (already merged by `optimizeStmtList`),
  `optimizeUnaryExpr`'s `needsGroupX` rewrite (its score can never be
  positive), `\u0000` at the end of the literal (the closing quote is always
  there), and two `for`-init merges.
* Performance (release, linux/amd64, whole process): typescript.js 348 ms
  (Go 351 ms), antd 186 ms (Go 175 ms), jquery 22 ms (Go 16 ms).

Red-team (2026-09-27, linux/amd64 host; go1.27.1 oracle built for amd64
for the large runs — the arm64 build under qemu reproduces every
checked-in fixture byte for byte and writes byte-identical record files for
20000-input samples of every generator, seed 99). Each check compares the
output, the error string and the input buffer afterwards; **0 differences
in every run**:

| run | checks |
|---|---|
| `fixtures DIR 5 101` (all eight fixture kinds at 5× scale, new seed) | 150,031 |
| `gen lits` seeds 1001, 1002 | 500,000 + 1,500,000 |
| `gen logic` seeds 6001, 6002 | 600,000 + 1,000,000 |
| `gen num` seeds 7001, 7002 (8 numbers per input at precisions 0–21: ~16M literals) | 1,000,000 + 1,000,000 |
| `gen soup` seeds 3001, 3002 | 500,000 + 1,000,000 |
| `gen prog` seeds 2001, 2002 | 400,000 + 500,000 |
| `gen corpus` seed 4001 (module-cache windows ≤ 32 KiB, byte mutations) | 150,000 |
| `gen corpus:LIST` seeds 4101, 4102 (windows of the node_modules files below) | 300,000 + 300,000 |
| `gen html` seeds 5001, 5002, 5003 (through neohugo's `M`, incl. `on*` attributes, entity-escaped, byte-mutated scripts) | 100,000 + 200,000 + 300,000 |
| `files`: every distinct `.js/.mjs/.cjs` under `/opt/node{20,21,22}/lib` (npm, pnpm, yarn, corepack, typescript, eslint, prettier, playwright, …), `/usr/share/javascript` and the Go/Ruby/LLVM distributions — 3830 files, 86.6 MB — × 13 configurations (`v2022`, `v2022-inline`, `0`, `keep`, `p3-v2022`, `alpha-v2019`, `v2015`, `keep-alpha`, `p1-v2022`, `v2019-inline`, `alpha-v2014`, `p10`, `keep-v2016`) + each file as a `<script>` through neohugo's `M` | 53,620 |
| `enumerate`: ≤ 4 of 29 statement/expression tokens × `v2022`, `keep-inline` | 732,540 inputs |
| `enumerate`: ≤ 4 of 40 lexer/parser tokens × `v2022` | 2,625,640 inputs |
| `enumerate`: all strings of ≤ 4 bytes over a 39-byte alphabet × `v2022` | 2,374,320 inputs |
| `enumerate`: ≤ 4 of 30 whitespace/line-terminator/comment/BOM/hashbang tokens × `v2022`, `v2022-inline` | 837,930 inputs |
| by hand: a variable used 65,536–65,538 times (uint16 wrap of `Var.Uses` → renaming order), 10,005 and 12,000 declarations (`hoistVars`' 10000 limit), 120- and 400-term string concatenations (`mergeBinaryExpr`'s limit of 50), 63 modern-syntax snippets × 3 configurations | 196 |

No divergence was found, so no port code changed; the new generators and
a sample of them (`redteam`, `enum-min`) are checked in as regression
fixtures.

## Known gaps

* None in the ported code. The seeksnack golden check of the whole minify
  stack (golden/canonical vs a Rust build) still needs the site corpus, which
  is not in this session; the six recorded seeksnack JS inputs are in the
  `literals` fixture with Go's outputs.
