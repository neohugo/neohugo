# tdewolff-parse-js — porting notes

Byte-exact port of `github.com/tdewolff/parse/v2@v2.8.1/js` (the version
pinned in neohugo's `go.mod`): the ECMAScript lexer, the parser (ASI,
regular-expression re-lexing, arrow-function speculation with
`UndeclareScope`, scope/Var/Uses analysis, `HoistUndeclared`), the AST node
types with `Scope`/`Var`/`VarArray`/`VarsByUses`, and the `String()`,
`JS()` and `JSON()` writers. Built on `crates/tdewolff-parse` (`Input`,
`GoBytes`, `GoError`, `new_error`, `printable`) and `crates/go-unicode`
(Unicode 17.0.0 tables of go1.27.1). Golden toolchain: go1.27.1
darwin/arm64.

## Go file → Rust module map

| Go (`parse/v2@v2.8.1/js/…`) | Rust |
|---|---|
| `tokentype.go` | `src/tokentype.rs` (`TokenType(u16)` + Go-named consts, `is_numeric`/`is_punctuator`/`is_operator`/`is_identifier_name`/`is_reserved_word`/`is_identifier`, `TokenType::bytes`/`string`) |
| `table.go` | `src/table.rs` (`OpPrec(i64)` + consts + `string`, `KEYWORDS` list, `keyword(&[u8])` lookup) |
| `lex.go` | `src/lex.rs` (`Lexer`, `is_identifier_start/continue/end`, identifier byte tables, operator maps) |
| `util.go` | `src/util.rs` (`as_identifier_name`, `as_decimal_literal`, crate-private `is_lhs_expr`) |
| `parse.go` | `src/parse.rs` (`parse`, `Options`, `NESTED_STMT_LIMIT`, `NESTED_EXPR_LIMIT`; private `Parser`) |
| `ast.go` types, `Var`, `Scope`, `VarArray`, `VarsByUses`, `DeclType`, `PropertyName.Is*` | `src/ast.rs` |
| `ast.go` `String()` methods | `src/ast_string.rs` (`Ast::string`, `node_string`, `scope_string`, `var_array_string`, `property_name_string`) |
| `ast.go` `JS()`/`JSON()` methods, `ErrInvalidJSON` | `src/ast_js.rs` (`JsWriter`, `Ast::js`, `js_string`, `node_js`, `json`, `json_string`, `ERR_INVALID_JSON`) |
| `walk.go` | not ported (see gaps) |
| — | `src/dump.rs`: canonical serializations used by the differential tests (mirrors the oracle's `dump.go`) |

No float arithmetic in the package, so no FMA sites.

## Public API for downstream crates (the JS minifier)

### Arena model (`src/ast.rs`)

Go's AST is a pointer graph that the minifier mutates in place. Every node
Go handles through a pointer or interface (`IStmt`, `IExpr`, `IBinding`,
`*Var`, `*VarDecl`, `*BlockStmt`, `*MethodDecl`, …) lives in one arena,
`Ast::nodes: Vec<Node>`, and is referenced by `NodeId` (a `u32` index,
`Copy`). Scopes live in `Ast::scopes: Vec<Scope>`, referenced by `ScopeId`.

| Go | Rust |
|---|---|
| `IStmt`/`IExpr`/`IBinding`/`*Var`/`*VarDecl`/`*BlockStmt`/`*MethodDecl` | `NodeId` |
| `nil` interface/pointer | `NodeId::NIL` (`is_nil()`/`is_some()`), `ScopeId::NIL` |
| pointer identity `a == b` | `NodeId` equality |
| `switch n := i.(type)` / `v, ok := i.(*Var)` | `match ast.node(i) { Node::Var(v) => …, … }` (NIL yields `Node::Nil`, so a nil interface matches no type, as in Go); `ast.as_var(i)`, `ast.is_var(i)` |
| field access through a pointer | typed accessors `ast.var(id)`, `var_mut`, `block`, `block_mut`, `var_decl(_mut)`, `func_decl(_mut)`, `method_decl(_mut)`, `class_decl(_mut)`, `arrow_func(_mut)`, `literal(_mut)`, `switch_stmt(_mut)` (panic like a failed type assertion), or `ast.node_mut(id)` |
| `&T{…}` | `ast.alloc(Node::T(…))`, `ast.alloc_var(data, link, uses, decl)`, `ast.alloc_scope(Scope{…})` |
| `&js.BlockStmt{List: l}` (zero `Scope`) | `ast.alloc_block(l)` (allocates a fresh zero scope) |
| `ast.BlockStmt` / `ast.List` / `ast.Scope` | `ast.block_stmt` (a `Node::BlockStmt` id) / `ast.list()` / `ast.module_scope()` |
| `v.Name()` | `ast.var_name(v)` |
| `scope.Declare/Use/AddUndeclared/MarkForStmt/MarkFuncArgs/HoistUndeclared/UndeclareScope/Unscope` | `ast.declare(s, decl, name)`, `use_var`, `add_undeclared`, `mark_for_stmt`, `mark_func_args`, `hoist_undeclared`, `undeclare_scope`, `unscope` (+ `find_declared`, `find_undeclared`) |
| `sort.Sort(js.VarsByUses(scope.Declared[i:]))` | `ast.sort_vars_by_uses(s, i)` or `sort_vars_by_uses(&ast.nodes, &mut slice)` — Go's pdqsort via `go-sort`, so ties land where Go puts them |
| `Scope.String()`, `VarArray.String()` | `ast.scope_string(s)`, `ast.var_array_string(&[NodeId])` |

Go value types are plain Rust values inside their parent node:
`BindingElement`, `Params`, `PropertyName`, `Property`, `Element`, `Arg`,
`Args`, `CaseClause`, `Alias`, `Field`, `ClassElement`, `TemplatePart`, and
embedded `LiteralExpr`s (`DotExpr.y`, `PropertyName.literal`). Go
`*PropertyName` fields (`Property.Name`, `BindingObjectItem.Key`) are
`Option<Box<PropertyName>>`, `NewExpr.Args *Args` is `Option<Args>`,
`ImportStmt.List` is `Option<Vec<Alias>>` (Go's `List != nil` is observable
in `String()`/`JS()`). Rust keywords get a trailing underscore (`else_`,
`static_`, `async_`, `await_`, `type_`).

`Var { data: GoBytes, link: NodeId, uses: u16, decl: DeclType }`: `data`
is a Go `[]byte` (`GoBytes`) that aliases the input buffer exactly like Go
(the lexer hands out `buf[start:pos:pos]`, so `cap == len`; property names
re-read from strings are `data[1:len-1]` with one byte of spare capacity;
`parse.Copy` gives fresh `cap == len` slices). The minifier's renamer
writes into `Var.data` in place (`name[0] = c`) — with `GoBytes` those writes
are visible through every alias, as in Go. `uses` is Go's `uint16` and wraps
(`wrapping_add`/`wrapping_sub` everywhere); `num_for_decls`/`num_func_args`/
`num_arg_uses` truncate like Go's `uint16(len(...))`.

### Lexer / parser / printers

- `Lexer::new(Input)` (the `Input` handle is shared, like `*parse.Input`),
  `next() -> (TokenType, GoBytes)`, `reg_exp()` (Go `RegExp`), `err() ->
  Option<GoError>` (`io.EOF` = `GoError::Eof`, lexer errors are
  `GoError::Parse` with Go's message/line/column/context).
- `parse(&Input, Options { while_to_for, inline }) -> Result<Ast, GoError>`;
  the error is Go's `*parse.Error` (`GoError::Parse`, `error_bytes()` is
  `err.Error()`), or the lexer's/reader's error.
- `is_identifier_start/continue/end(&[u8])`, `as_identifier_name`,
  `as_decimal_literal` (generic over `ByteView`, so `&[u8]` and `GoBytes`
  both work), `KEYWORDS`, `keyword`.
- `ast.string()` / `node_string(id)` (Go `String()`), `ast.js_string()` /
  `node_js(id, &mut JsWriter)` (Go `JS()`), `ast.json_string()` (Go
  `JSONString`, the error is `err.Error()` bytes). All return `Vec<u8>`
  (Go strings are bytes).
- `dump::{lex_dump, parse_dump, ast_dump, string_dump, js_dump, json_dump}`:
  deterministic serializations of the token stream and of the whole graph
  (tree, scopes, VarDecls, Vars with identities numbered by first
  encounter), identical to the Go oracle's; handy for debugging downstream.

### Stack depth

The parser is recursive like Go's, bounded by Go's limits (1000 nested
statements, 1000 nested expressions per function body). Measured: 998-level
nesting needs ~8 MB of stack in a debug build and < 1 MB in release. Go's
limits do not bound the product (expression depth resets inside function
bodies), so adversarial input can recurse deeper than any fixed stack;
callers that parse untrusted input should run on a thread with a large stack
(the tests use 1 GB virtual). Stack overflow aborts the process (Go would
grow its stack up to 1 GB).

## Deliberate deviations (API shape only; outputs are identical)

- **Module scope.** Go's `parseModule` returns its `BlockStmt` by value, so
  `ast.BlockStmt.Scope` is a *copy* of the local scope that every child's
  `Parent`/`Func` and every top-level `VarDecl.Scope` point to (in non-inline
  mode `ast.Scope.Func != &ast.Scope`). The port has a single module scope
  (its `func` is itself). The two Go scopes hold identical values when
  `Parse` returns; they only diverge if a caller mutates one and then reads
  the other's slice header. minify/js reads `ast.BlockStmt.Scope.VarDecls`
  only in `hoistVars`, before anything mutates the scope, and afterwards
  only goes through `decl.Scope.Func` (the local one), so the minifier sees
  the same values. The oracle's dump aliases Go's local scope to the copy.
- **Blocks embedded by value** (`FuncDecl.Body`, `MethodDecl.Body`,
  `ArrowFunc.Body`, `AST.BlockStmt`) are separate `Node::BlockStmt` nodes
  referenced by id, like `*BlockStmt` fields. Copying a Go `BlockStmt` value
  copies its embedded `Scope`; in the port, copying the id shares the scope.
  The parser never copies blocks.
- **Slices.** AST lists and scope lists are `Vec`s. Go slices share backing
  arrays (`append(s[:i], s[i+1:]...)` in place, `range` over a header whose
  length is fixed while elements shift). The parser's own uses are all
  equivalent to the `Vec` operations used (checked case by case:
  `Undeclared` removal in `Declare`, the pop in `parseIdentifierArrowFunc`,
  the truncations in `UndeclareScope`/`Unscope`, comment splicing in
  `parseModule`/`parseStmtList`). Downstream code that relies on Go slice
  aliasing (minify's `optimizeStmtList`/`hoistVars` do in-place `append`
  while ranging) must reproduce that explicitly.
- **Parser allocation order.** Nodes are allocated where convenient (often
  after their children); ids carry no meaning beyond identity. Value-returning
  Go functions (`parseImportStmt`, `parseArrayLiteral`, …) return Rust
  structs; pointer-returning ones allocate first and fill in place, like Go.
- **Errors.** `Parser.err` stores the `Error()` bytes (`fmt.Errorf` only ever
  formats `%s` of a name here). `parse` returns `GoError`.
- **`TokenType`, `OpPrec`, `DeclType`** are newtypes (not enums) so any value
  has a `string()` (`Invalid(N)`) and bit tests work as in Go.
- **`Input.Peek(-n)`** (used by `RegExp`) is `move_(-n); peek(0); move_(n)`
  since the shared `Input` port takes an unsigned offset.
- **`JsWriter`** models Go's `io.Writer`/`parse.Indenter` pair: an output
  buffer plus `Option<usize>` indentation (Go flattens nested Indenters, so
  this is exact, including the `w.(parse.Indenter)` type assertions).
- **Panics.** Where Go panics (nil dereference in `UnaryExpr.JSON`, the
  `*Var` assertion in `exprToBinding`, index out of range when the lexer is
  driven past EOF after an error, the `[:len-1]` pop in
  `parseIdentifierArrowFunc`), the port panics too (the oracle and the Rust
  dumps both record `PANIC`). None of these is reachable from a successful
  `Parse` + minify path on the corpus.
- `Var.Info()` (prints Go pointer addresses) is not ported.

## Tests and parity evidence

- Oracle: `tools/go-oracle/tdewolff-parse-js` (package main in the neohugo
  module, gofmt/vet clean). `gen.sh` regenerates everything
  (`SCRATCH=… tools/go-oracle/tdewolff-parse-js/gen.sh`). Commands:
  `fixtures DIR NFUZZ SEED`, `fuzzcorpus OUT N SEED MAXLEN ROOT…`,
  `corpus OUT.tsv ROOT…`, `dump MODE FILE`, `time FILE`.
  `gen_upstream_tables.py` converts the upstream Go test tables to
  `tests/upstream_tables.rs`.
- Eight serializations are compared for every input: `lex` (plain token
  stream incl. data bytes and capacities, continuing after lexer errors),
  `lexre` (same with `RegExp()` re-lexing by a previous-token heuristic),
  `parse` (`Options{}`), `parsew2f` (`WhileToFor`, as minify),
  `parseinline` (`WhileToFor+Inline`, as minify's `inline=1`) — each a full
  graph dump: every node and field, every scope (`Parent`, `Func`,
  `Declared`, `Undeclared`, `VarDecls`, `NumForDecls`, `NumFuncArgs`,
  `NumArgUses`, flags), every Var (`Data`+cap, `Link`, `Uses`, `Decl`) with
  identities numbered by first encounter — or Go's exact error string;
  `string` (`ast.String()`), `js` (`ast.JSString()`), `json`
  (`ast.JSONString()` output and error).
- `tests/fixtures.rs`: `literals.rec.gz` — every string literal of the
  upstream `parse/js` and `minify/js` `_test.go` files plus ~140 hand-written
  edge cases (3164 inputs, full dumps); `fuzz.rec.gz` — 30000 seeded
  mutations (digests); `fuzzcorpus.rec.gz` — 4000 mutated windows of the JS
  corpus (digests). A 300000 + 100000 set lives in
  `$SCRATCH/work/tdewolff-parse-js/full`
  (`TDEWOLFF_PARSE_JS_FIXTURES=… cargo test --release --test fixtures`).
- `tests/upstream.rs`: the upstream tests ported literally — `TestTokens`
  (88), `TestRegExp` (12), `TestOffset`, `TestLexerErrors` (9 + extras),
  `ExampleNewLexer`, `TestParse` (380, `String()`), `TestParseError` (264),
  `TestParseScope` (97, with the `ScopeVars` helper), `TestScope` (incl. the
  `VarsByUses` sort), `TestParseInputError`, `TestAsIdentifierName`,
  `TestAsDecimalLiteral`, `TestJS` (JS() printer table), `TestJSON`, plus a
  nesting-limit test.
- `tests/corpus.rs` (`#[ignore]`, needs the corpora outside the repo):
  FNV digests of all eight dumps for 3975 files / 103 MB — every
  `.js/.mjs/.cjs/.ts/.mts/.cts` of the pristine seeksnack site (incl.
  node_modules: jquery, bootstrap, md5, typescript.js 9 MB, @taplo 35 MB, …),
  the golden `js/` outputs (canonical and nominify) and the recorded minifier
  inputs/outputs in `work/minify/corpus2/js`.

Results (2026-09-27): all tests pass in debug and release; literals 3164 × 8
modes identical; fuzz 300000 × 8 and corpus-window fuzz 100000 × 8
identical; corpus 3975 files × 8 modes identical (the literal set mixes
2324 successful and 840 failing parses; the corpus exercises every
real-world construct, TypeScript files the error paths).

Performance (release, darwin/arm64): jquery.js 4.5 ms per parse (Go 3.5
ms), typescript.js 360 ms (Go 233 ms); the difference is the O(n²) name
lookups over `GoBytes` in huge scopes, which the port keeps as in Go.

## Known gaps

- `walk.go` (`Walk`/`IVisitor`) is not ported: minify does not use it, and
  its visitor receives pointers into value-type fields, which would need a
  separate reference type. `walk_test.go` is therefore not ported.
- `Var.Info()` (debug output with pointer addresses) is not ported.
