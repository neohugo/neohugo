# gotemplate — porting notes

Port of Go's `text/template` and `html/template` **as forked in neohugo**:
`tpl/internal/go_templates/{texttemplate,texttemplate/parse,htmltemplate}`.
The fork is the **go1.24.0** stdlib (see `scripts/fork_go_templates/main.go`)
plus Hugo's two files `texttemplate/hugo_template.go` and
`htmltemplate/hugo_template.go`. This crate ports that code, not the
go1.27.1 stdlib: e.g. no `<meta content="…url=…">` URL states,
`isJSType("")` is false, context equality ignores `jsBraceDepth`.

Values are [`go_value::Value`]s. Reflection is replaced by the value model
and by the host's `ExecHelper` (see "Host contract").

## Go file → Rust module map

| Go (`tpl/internal/go_templates/…`) | Rust | Notes |
|---|---|---|
| `texttemplate/parse/lex.go` | `src/parse/lex.rs` | state functions → `State` enum dispatched in `next_item`; bytes in, Go UTF-8 decoding |
| `texttemplate/parse/node.go` | `src/parse/node.rs` | `Node` enum over structs; `NodeLike` = Go's `Node` interface; `NodeId` on edit targets (Action/Template/Text) |
| `texttemplate/parse/parse.go` | `src/parse/parse.rs` | parse state in `TreeParser` (one per tree, sharing the lexer and tree set like Go's nested `startParse`); errors are `Result`s |
| `*parse.Tree` pointer semantics | `src/parse/mod.rs` `SharedTree` | see "Shared, mutable parse trees" |
| `texttemplate/template.go`, `option.go` | `src/text/template.rs` | `Template` = Go `*Template` (a handle; clones are the same pointer) |
| `texttemplate/exec.go` + `hugo_template.go` (`state`, `evalFunction`, `evalField`, `evalCall`, `isTrue`) | `src/text/exec.rs` | panics → `Result<_, Ctrl>` (`Break`/`Continue`/`Err`) |
| `texttemplate/funcs.go` | `src/text/funcs.rs` | builtins as the `Builtin` enum with explicit Go signatures (arity, `printf`'s `string` format) |
| `texttemplate/hugo_template.go` (`ExecHelper`, `Executer`, `Preparer`, `TryValue`, `TryError`, `GoFuncs`) + `common/hreflect.IsTruthfulValue` | `src/text/hugo.rs` | |
| `htmltemplate/escape.go` | `src/html/escape.rs` | edits keyed by `NodeId`, applied by `commit` through `SharedTree::update` |
| `htmltemplate/template.go` + `hugo_template.go` (`Prepare`, `All`, `CloneShallow`, `StripTags`) | `src/html/template.rs` | |
| `htmltemplate/{attr,content,context,css,error,html,js,transition,url}.go`, `*_string.go` | `src/html/*.rs` | see the per-module headers |
| `fmtsort` | — | `Map` entries are byte-ordered; `Object::map_keys` is specified sorted |
| `helper.go` (ParseFiles/ParseGlob), `doc.go`, `testenv`, `cfg` | — | not needed |

## Host contract

This section documents the API against `crates/GOTEMPLATE_CONTRACT.md`
(which extends template-engine spec §16).

### C1 `ExecHelper` — `gotemplate::text::ExecHelper`

```rust
pub type Func = Arc<dyn Fn(HostCtx<'_>, &[Value]) -> go_value::Result<Value> + Send + Sync>;

pub trait ExecHelper: Send + Sync {
    fn init(&self, ctx: HostCtx<'_>, template_name: &str) {}
    fn get_func(&self, ctx: HostCtx<'_>, name: &str) -> Option<Func> { None }
    fn has_method(&self, ctx: HostCtx<'_>, receiver: &Value, name: &str) -> bool { /* Object::has_method */ }
    fn call_method(&self, ctx: HostCtx<'_>, receiver: &Value, name: &str, args: &[Value]) -> go_value::Result<Value> { /* Object::call_method */ }
    fn get_map_value(&self, ctx: HostCtx<'_>, receiver: &Value, key: &Value) -> Option<Value> { /* exact key */ }
    fn on_called(&self, ctx: HostCtx<'_>, name: &str, args: &[Value], result: &Value) {}
    fn is_true(&self, v: &Value) -> bool { is_truthful_value(v) }
}
```

The signatures are the contract's; every method has a default equal to
Go's behaviour without a helper (`DefaultHelper`), so plain
`Template::execute` works like the fork's `Execute`.

- `get_func` is consulted first; `None` falls back to the template's own
  `Funcs` and then the builtins (Go `findFunction`), else
  `"%q is not a defined function"`.
- Parse-time names: `Template::funcs(&FuncMap)` (names + exec funcs) or
  `Template::func_names(names)` (names only — Hugo resolves through the
  helper). The builtins and, after escaping, the `_html_template_*` /
  `_eval_args_` escapers are always known.
- `and`/`or` always take the engine's short-circuit path (by name, as in
  Go where `isBuiltin = name == "and" || name == "or"` when a helper is
  set).
- `get_map_value`: `None` is Go's invalid result (missing key: the
  `missingkey` option applies); `Some(Value::Invalid)` is a present key
  holding a nil interface. Hugo's `maps.Params` lookup should return
  `None` for a present key whose value is nil (Go: `reflect.ValueOf(nil)`).
- `text::go_funcs()` returns the builtins as `Func`s (Go
  `texttemplate.GoFuncs`), `html::go_funcs()` the escapers (Go
  `htmltemplate.GoFuncs`), for hosts that merge them into their lookup.
  The builtin `Func`s are created once; when the helper hands one back,
  the engine recognises it (pointer identity) and applies the builtin's Go
  signature (arity, `printf`'s `string` format) exactly as Go's `evalCall`
  inspects the reflect signature. As in Go, `isBuiltin` stays false on that
  path, so `call`, `and`, `or` reached through the host lookup behave like
  Hugo (`call` → `error calling call: unreachable`; `and`/`or` still take
  the short-circuit path by name).
- Hugo's helper must pass `has_method`/`call_method` through to `Object`
  for engine objects (`TryValue`, `TryError`, `FuncValue`).

### C2 field/method/key resolution — `exec.rs` `eval_field`

Exactly the contract order: `Invalid` receiver → `Invalid` (or the
`missingkey=error` error); nil interface (`TypedNil(T)` of interface kind)
→ `nil pointer evaluating T.Name`; `helper.has_method` for every receiver
kind → `call_method` (arguments evaluated in between); `Object` of kind
Struct/Ptr/Interface → `Object::field` (with arguments →
`%s has arguments but cannot be invoked as function`); `Value::Map`,
`Kind::Map` objects and nil maps → `helper.get_map_value` (with arguments →
`%s is not a method but has arguments`); typed nil pointer →
`nil pointer evaluating T.Name`; otherwise
`can't evaluate field Name in type T`.

### C3 values the engine creates — `ideal_constant`, `eval_command`

Number literals for `any` parameters: float syntax (`.eEpP`, not hex, not
rune) → `float64`; integers and rune literals → Go `int`
(`Value::Int(_, IntKind::Int)`); an integer too large for `int` but valid
as `uint64` → `%s overflows int`. Complex literals are an error
(`complex constant … is not supported` — the value model has no complex
numbers). Strings → `Value::String`, `true`/`false` → `Value::Bool`,
`nil` argument → `Value::Invalid`, `nil` command → `nil is not a command`.

### C4 context, re-entrancy, sharing

- `text::Executer::new(helper).execute_with_context(ctx, &dyn Preparer, &mut dyn Write, &Value)`;
  html templates implement `Preparer` (escape once, then the text
  template). `html::Template::execute_with_helper` and
  `text::Template::execute_with_helper` are shortcuts.
- The same `ctx: HostCtx` (`&dyn Any`) is passed to every helper call and
  every function/method call; the engine never replaces it.
- Re-entrant: execution holds no lock while calling out. Templates are
  `Send + Sync` handles; an execution walks an immutable snapshot
  (`Arc<Tree>`) of each tree it enters, so later edits (copy-on-write) never
  disturb a running execution.

Data passed to `execute*` is Go's `data any`: a `TypedNil` of an
interface type (e.g. a nil `error`) is treated as an untyped nil
(`Invalid`), as `reflect.ValueOf` does.

### C5 truthiness

`helper.is_true(&v)` is used for `if`, `with`, `else with`, `and`, `or`,
`not` (Go `isTrue(indirectInterface(v))`). `range` decides emptiness by
length. `text::is_truthful_value` is `hreflect.IsTruthfulValue` over the
value model: `Object::is_zero` first, zero `time.Time`, `maps.Params` that is
empty or holds only `_merge`, then kind rules (typed nils are false).

### C6 `and`/`or`

Short-circuit left to right; the result is the first operand whose truth
equals `name == "or"`, else the last operand (or the piped value),
returned unchanged.

### C7 arguments

Arguments are evaluated as for `any` parameters (Go `evalArg` with an
interface type). Before calling a host `Func` or method, a
`TypedNil(T)` of interface kind becomes `Invalid` (a nil non-empty
interface converted to `any`). Other values, typed nils included, are
passed unchanged. Hosts check their own parameter types and counts. The
engine checks the builtins' Go signatures (arity messages
`wrong number of args for %s: want %d got %d` / `want at least`, and
`printf`'s `string` format: `expected string; found %s`,
`wrong type for value; expected string; got %s`,
`invalid value; expected string`).

### C8 printing — `print_value`/`printable_value`

`Invalid` → `<no value>`; typed nils print through go-fmt (`<nil>`, `[]`,
`map[]` by `typed_nil_kind`); a non-nil `Kind::Ptr` object without
`String`/`Error` prints as the struct it points to (`{…}`, Go's
`indirect`); `Kind::Func` objects and nil funcs/chans →
`can't print %s of type %s`; everything else `go_fmt::fprint`.

### C9 html escapers

`html/content.rs` `stringify` applies Hugo's `indirect`
(`Object::printable_value`), maps `Value::Safe(kind, s)` to content types,
and skips `Invalid` arguments. `jsValEscaper` uses
`Object::marshal_json` and then `Object::go_string` (no `indirect`).

### C10 identity

The engine never rebuilds `List`/`Map`/`Object` values: the `Arc`s handed
to funcs, methods and the helper are the ones it received or produced.

### C11 `range`

Ints/uints (`0..n-1`, element of the same Go type, one variable only),
`Value::List` and `Kind::Slice` objects (index is Go `int`), `Value::Map`
(byte-ordered keys) and `Kind::Map` objects (`map_keys()` order), typed nil
slices/maps and `Invalid` (empty → `else`); anything else is
`range can't iterate over %v`. `break`/`continue` are supported.

### C12 errors and `try`

Execution errors are `Error::Exec(ExecError { name, message, cause })` with
Go's text `template: %s: executing %q at <%s>: %s`; func/method errors are
`error calling %s: <err>` with `cause` = the host error. `try` returns a
`TryValue` object (`.Value`, `.Err` = `*template.TryError` with `.Err`,
`.Cause`, `Error()`).

### C13 API for nh-tplimpl

| Go | Rust |
|---|---|
| `texttemplate.New(name)`, `.New`, `.Parse`, `.Lookup`, `.AddParseTree`, `.Clone`, `.Templates`/`All`, `.Funcs`, `.Option`, `.Delims` | `text::Template::{new, new_associated, parse, lookup, add_parse_tree, clone_ns, templates/all, funcs/func_names, set_option, delims}` |
| `htmltemplate.New`, `.New`, `.Parse`, `.Lookup`, `.AddParseTree`, `.Clone`, `.CloneShallow`, `.Templates`/`All`, `.Prepare`, `.Funcs` | `html::Template::{new, new_associated, parse, lookup, add_parse_tree, clone_ns, clone_shallow, templates/all, prepare, funcs/func_names}` |
| `t.Tree` (`*parse.Tree`, mutated by `templatetransform`) | `tree() -> Option<SharedTree>`; `SharedTree::{get, update, set}` |
| `parse.IsEmptyTree` | `parse::is_empty_tree` |
| `Node.String()` | `NodeLike::to_bytes` |
| `htmltemplate.StripTags` | `html::strip_tags` |
| `texttemplate.GoFuncs`, `htmltemplate.GoFuncs` | `text::go_funcs()`, `html::go_funcs()` |
| `Executer.ExecuteWithContext` | `text::Executer::execute_with_context` |

## Shared, mutable parse trees

Go templates hold `*parse.Tree` pointers that are shared (`Clone`,
`CloneShallow`, `AddParseTree` aliases such as `_internal/…`) and mutated in
place (escaper commits, Hugo's AST transforms). `SharedTree` is that
pointer: clones are the same tree, `update` mutates it for all holders
(`Arc::make_mut` under a lock — a running execution keeps its old
snapshot). Node identity (Go pointers, used as escaper map keys) is
`NodeId`: `copy()`/`copy_list()` (Go `Copy`) assign fresh ids, Rust
`Clone` (copy-on-write) keeps them.

`escaper.commit` applies the recorded edits by walking the trees of the
templates named in `called` (plus the derived templates), and falls back to
every tree of the namespace if an edit was not found there.

## Deliberate deviations

1. **Complex numbers** are not representable: complex literals parse
   exactly as Go's `newNumber` (a port of `fmt.Sscan`'s complex scanning),
   but evaluating one is an execution error
   (`complex constant … is not supported`).
2. **`slice` builtin** copies (the value model has no shared backing
   arrays; `cap == len`), so re-slicing beyond `len` is out of range.
3. **Stack depth.** `MAX_EXEC_DEPTH` is Go's 100000 (Go's exact
   `exceeded maximum template depth (100000)` error is tested on a 512 MiB
   thread), but Rust stacks do not grow: a 10000-deep `{{template}}`
   recursion needs ~64 MB. Hosts should run executions on threads with at
   least 128 MiB of stack. (Hugo limits partial nesting to 999 levels.)
4. **`DefinedTemplates`** lists names sorted (Go: map order).
5. **Template names and error texts** are Rust `String`s: invalid UTF-8
   in a `define` name or inside an error message (e.g. the `%v` of a string
   in `range can't iterate over %v`) becomes U+FFFD where Go keeps the raw
   bytes. Output bytes are unaffected.
6. **Host functions have no Go signature**: argument conversion and arity
   are the host's (contract C7). Messages for such errors may differ from
   Go's `evalArg` messages; output bytes do not.
7. **Nil `*T` field access** reports `nil pointer evaluating T.Name` even
   when Go would say `can't evaluate field` (Go checks the struct's field
   set, which a typed nil does not carry).
8. **A `{{template}}` of a template without a tree** is an error (Go
   dereferences nil and crashes).
9. **`herrors.Cause`** stops at the host error (`go_value::Error` has no
   cause chain).
10. **`context.jsBraceDepth`** is an `IntSlice` (Go `[]int` semantics:
    `Clone` shares the backing array and `append` grows like Go's
    `growslice`), because the go1.24 fork neither clones nor compares it and
    the aliasing is observable in the escaper.
11. **Transition from `stateDead`** behaves like `tError` (Go would index
    out of range; the escaper never transitions from it).
12. **Pointer dereference in html escapers** (`doIndirect`,
    `indirectToStringerOrError`, `indirectToJSONMarshaler`): a `Kind::Ptr`
    object without `String`/`Error`/`MarshalJSON` becomes a `Kind::Struct`
    view of its fields (go-fmt's pointer-receiver convention); it keeps
    `printable_value` (a value-receiver method for both Hugo types).
13. **Replacement tables** are functions `Rune -> Option<&[u8]>` instead of
    arrays (equivalent, including `htmlReplacer`'s length check); the
    regular expressions are hand-coded with Go's case-folding orbits
    (`(?i)s` matches U+017F).
14. **Error edits.** Go mutates the shared `*Error` (`Name`, `Line`,
    `Description`) when it annotates branch/range errors; here the
    annotated error is a new `Arc`. Only error texts depend on it.
15. **Static interface types.** Go names the static type of an
    interface-typed slot in some messages (`can't evaluate field X in type
    interface {}`); the value model knows only dynamic types. Only the nil
    case is faithful (a nil `any` from a map, slice, range or
    `missingkey=zero` is `TypedNil("interface {}")`, giving
    `nil pointer evaluating interface {}.X` as in Go). Messages only.
16. **Value-model limits** (messages or unrepresentable data only):
    unexported struct fields are not exposed; maps have string keys only;
    a pointer to a basic type (`*int`) is the value itself; channels and
    `iter.Seq` funcs do not exist; a non-addressable struct copy in an `eq`
    error prints `<0>` where Go prints `{0}`; outputs containing Go
    pointer addresses are not comparable.

## Verification

All fixtures come from `tools/go-oracle/gotemplate` (go1.27.1 toolchain,
the fork copied by `sync-fork.sh`, build tag `gotemplate_oracle`).

| test | what |
|---|---|
| `tests/html_escdump.rs` | the escaper reproduces Go's escaped trees of all 120 layouts in this repository (`docs/layouts`, `create/skeletons/theme/layouts`, `tpl/tplimpl/embedded/templates`), namespaces built as tplimpl does (shared namespace + `CloneShallow` per overlay/baseof), every tree incl. the derived `name$htmltemplate_*` templates, byte for byte (665 KB dump) |
| `src/html/tests/oracle.rs` | 39,048 escaper calls (17 escapers × 2,297 argument lists: every byte, special runes, invalid UTF-8, fuzz, Safe types, numbers, nils, time, collections, Stringers, errors, json/text marshalers, PrintableValue types, multi-arg), 53,730 transition runs (82,260 steps from 170 start contexts: contexts, error texts, brace slices incl. capacity/aliasing), 59,569 leaf-function checks — all equal to Go |
| `src/html/tests/go_tests.rs` | Go's html_test, js_test, css_test, url_test, transition_test tables |
| `tests/html_strip_tags.rs` | `stripTags` on 3,949 inputs |
| `tests/html_exec.rs` | full engine (parse → escape → exec) replaying scripts recorded from the fork: Go's escape_test.go (TestEscape with value and pointer data, EscapeMap, EscapeSet, TestErrors texts, EscapeErrorsNotIgnorable, IndirectPrint, EmptyTemplateHTML, PipeToMethodIsEscaped, ErrorOnUndefined, IdempotentExecute, OrphanedTemplate, AliasedParseTreeDoesNotOverescape), content_test.go (every content type × every context, Stringer, nil non-empty interfaces), clone_test.go, multi_test.go, template_test.go and the html copy of exec_test.go — 773 scripts, 3,587 operations; plus a differential corpus of 217 templates (every escaping context, layout snippets) × ~70 values through plain `Execute` and through a Hugo-like `Executer`/helper (all funcs incl. escapers via the helper, case-insensitive `maps.Params`, methods by name) — 449 scripts, 23,542 operations. Remaining differences are listed per test with their reason (`KNOWN_GAPS`, value-model limits); a stale entry fails the test |
| `src/html/tests/internal.rs` | TestEscapeText (151 contexts), TestEnsurePipelineContains, TestRedundantFuncs |
| `tests/lex_go.rs`, `tests/parse_go.rs` | Go's lex_test.go (lexTests, delims, positions) and parse_test.go (numberTests, parseTests plain and copied, comments, keywords/funcs, SkipFuncCheck, IsEmpty, ErrorContext with tree copy, errorTests, TestBlock, TestLineNum) |
| `tests/parse_oracle.rs` | 4,873 fork parse cases (trees, node structure, number flags, errors): all equal except 5 that differ only by lossy UTF-8 template names |
| `tests/exec_go.rs` | exec_test.go: 528 table cases (479 equal, 36 listed value-model deviations, 13 skipped: pointer addresses, `iter.Seq`, complex) plus 11 hand-ported tests; expectations from the FORK (for every entry with an expectation the fork agrees with stock Go) |
| `tests/exec_oracle.rs` | 13,470 Hugo-like cases (plain and Hugo-helper modes: Params case-insensitivity, methods before keys, mainsections, typed nils, time, IsZero, Stringers, try, missingkey, all builtins): 0 differ; 232 skipped (pointer addresses); listed deviations by class |
| `tests/exec_depth.rs` | `TestMaxExecDepth`: Go's exact error at depth 100000 |
| `tests/smoke.rs` | text/template end-to-end smoke cases |

Regenerate (repo root):

```sh
tools/go-oracle/gotemplate/sync-fork.sh
GOTOOLCHAIN=go1.27.1 go run -tags gotemplate_oracle ./tools/go-oracle/gotemplate parse crates/gotemplate/tests/fixtures/text
GOTOOLCHAIN=go1.27.1 go run -tags gotemplate_oracle ./tools/go-oracle/gotemplate exec crates/gotemplate/tests/fixtures/text
GOTOOLCHAIN=go1.27.1 go run -tags gotemplate_oracle ./tools/go-oracle/gotemplate exectests crates/gotemplate/tests/fixtures/text
GOTOOLCHAIN=go1.27.1 go run -tags gotemplate_oracle ./tools/go-oracle/gotemplate escfuncs crates/gotemplate/tests/fixtures/html
python3 tools/go-oracle/gotemplate/extract_tables.py   # after sync-fork.sh: regenerates htmlexec_tables.go
GOTOOLCHAIN=go1.27.1 go run -tags gotemplate_oracle ./tools/go-oracle/gotemplate htmlexec crates/gotemplate/tests/fixtures/html
GOTOOLCHAIN=go1.27.1 go run -tags gotemplate_oracle ./tools/go-oracle/gotemplate escdump \
  crates/gotemplate/tests/fixtures/html/escdump.txt docs/layouts create/skeletons/theme/layouts tpl/tplimpl/embedded/templates
```

`tools/go-oracle/gotemplate/exectests/data.go` is a verbatim copy of parts
of the fork's `exec_test.go` (line ranges in its header); re-copy it by
hand if that file changes.

## Gaps

- The probe site of template-engine spec Appendix A/B and the seeksnack
  escdump (`$SCRATCH/work/template-engine/…`) were not committed by the
  first session; the escdump oracle here uses the layouts in this
  repository instead. The seeksnack layouts (private repo) should be added
  to the escdump corpus when available.
