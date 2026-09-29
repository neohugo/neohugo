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

Named basic types (`type HTML string`, `time.Month`, …) are objects whose
`Object::underlying` returns the basic value. Where Go's reflection switches
on the Kind, the engine uses that value: truthiness (after `is_zero`),
`len`, and the builtin comparisons `eq`/`ne`/`lt`/`le`/`gt`/`ge`
(`funcs::basic_underlying`). Error texts keep the named type name
(`len of type main.Month`). Tested in `tests/named_basic.rs` against Go.

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

Results: a host function or method (or a `call`ed func value) returns a
nil `any` as `Invalid`, and an object field holding a nil `any` may be
`Invalid` too. A Go call result or struct field is never the invalid
`reflect.Value`, so the engine turns such an `Invalid` into a nil
`interface {}` (`TypedNil("interface {}")`): `f.X`, `(try f).Value.X` and
`.Field.X` fail with `nil pointer evaluating interface {}.X` as in Go, and
the end of the pipeline command turns it back into `Invalid` (Go
`evalPipeline`), which is what later commands, variables and hosts see.

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
`error calling %s: <err>` with `cause` = the host error, except the errors Go's `evalCall`
reports itself before the call (argument count, `validateType`, `goodFunc`): hosts check those
(they receive `any` arguments) and mark them with `go_value::Error::eval_call(msg, at)`, and the
engine reports them without the prefix and without a cause, at the node Go uses (the function
identifier or field for the count and `goodFunc`, the argument for its type). Verified by
`tests/callerr.rs` against `tools/go-oracle/gotemplate/callerr` (33 cases, Go 1.27.1
`text/template`; the fork's `goodFunc` text is the Hugo layer's). `try` returns a
`TryValue` object (`.Value`, `.Err` = `*template.TryError` with `.Err`,
`.Cause`, `Error()`). Without an error `.Err` is a nil `*template.TryError`
(`TypedNil`), declared to go-fmt as a type whose `Error` method panics on a
nil receiver, so fmt prints it `<nil>` for every verb (`printf "%s" .Err`),
as Go's `catchPanic` does.

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
   The same holds for syntactic nesting, which the parser, the escaper,
   `String()` and execution walk recursively (Go has no limit but its 1 GB
   maximum stack): on a 128 MiB thread 30000 nested `{{if}}`s and 50000
   nested parentheses work, 100000 overflow the stack (the process aborts).
4. **`DefinedTemplates`** lists names sorted (Go: map order).
5. **Template names and error texts** are Rust `String`s: invalid UTF-8
   in a `define` name or inside an error message (e.g. the `%v` of a string
   in `range can't iterate over %v`) becomes U+FFFD where Go keeps the raw
   bytes. Output bytes are unaffected.
6. **Host functions have no Go signature**: argument conversion and arity
   are the host's (contract C7). Messages for such errors may differ from
   Go's `evalArg` messages. Go checks a host function's arity (and each
   argument's type) before evaluating the arguments, so when an argument
   also fails Go reports the arity error and the host reports the
   argument's error, at another node. Output bytes differ only when `try`
   captures such an error and the template prints its text.
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
    `nil pointer evaluating interface {}.X` as in Go). Messages only
    (also `non-function <error Value>` of `call` on a value piped from a
    function declared to return `error`), with one exception: Go's `slice`
    builtin does not unwrap interface-kinded index arguments, so an index
    read straight from an interface-typed slot (`{{slice .S 0 .N}}` with
    `.N` from a `map[string]any` or a range element of `[]any`) is Go's
    error `cannot index slice/array with type interface {}` where the value
    model slices. Hugo overrides `slice` (collections.Slice), so Hugo
    templates never reach the builtin.
17. **Template names with `%` verbs in parse errors.** Go's parser splices
    the template name into `errorf`'s format string, so `%d`, `%s`… in a
    name consume the message's arguments (and print Go pointers in some
    messages); here the name is text. Messages only.
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
| red-team regressions: `parse_oracle.rs` `parse_redteam_regressions`, `exec_oracle.rs` `exec_redteam_regressions`, `html_exec.rs` `html_redteam_regressions` | `text/redteam_parse.txt.gz` (2,494 cases), `text/redteam_exec.txt.gz` (3,028 cases: the minimized findings — nil `*TryError` printing, `TryValue.Value` after an error, `ExecError` fields and identity, `slice` with interface-kinded indexes — plus a fixed-seed `rtexec` sample), `html/redteam.txt.gz` (1,180 scripts, 10,353 operations: host functions returning a nil `any` in chains/`try`/escaping, text and html, plus fixed-seed `rthtml`/`rtns`/`rttns`/`rtlayout` samples); written by the `rtfixtures` mode (Go outputs holding pointer-like numbers are left out so the files regenerate byte for byte) |

### Red-team corpora (not checked in)

Seeded generators in `tools/go-oracle/gotemplate/redteam_*.go`, replayed by
ignored tests that take the corpus paths from environment variables
(`:`-separated lists). The red-team replays classify the documented
deviations anywhere in a line (inside `try` values in the output, URL- or
JS-escaped): complex numbers (1), host signatures (6: arity and argument
type errors, their order and location), lossy error texts (5), nil `*T`
fields (7), static interface types (15, incl. `slice` with an
interface-kinded index), `%` in template names (17), numbers derived from
Go pointer addresses, Go panics on nil trees (8), named string types
(go-fmt gaps). Everything else must be equal. Results at hand-off (after
the fixes below): 0 unexplained differences.

| mode (seeds × size) | what | cases |
|---|---|---|
| `rtparse` (1 × 200k, 2 × 300k, 3 × 300k) | token soup, literal syntaxes (numbers incl. hex/octal/binary floats, underscores, imaginary; char constants; interpreted/raw strings with bad escapes), nested `define`/`block`/`if`/`with`/`range`/`else if`/`else with`, `break`/`continue` anywhere, variable scope, trim markers next to comments, 25 delimiter pairs (overlapping `-`, `/*`, spaces, multi-byte), unicode/invalid UTF-8, byte mutations; all parse modes and func sets; tree dump + errors | 800,000 |
| `rtexec` (1 × 20k, 2 × 150k, 3 × 150k templates, × plain/hugo) | grammar-generated templates over the `exec` data model, funcs and Hugo helper: random pipelines, method/field/key chains, all builtins with random args, printf formats, `range` over every kind with 1/2 variables and break/continue, `if`/`with`/`else with`, variables, `define`/`block`/`template` DAGs, every `missingkey` option, 7 data kinds | 640,000 |
| `rtpairs pairs` (systematic) | every pair of 123 model values through `eq ne lt le gt ge`, `eq` with 3 args, `index`, `slice` (2 shapes), `and`, `or`, `print`, `html`, `js`, `urlquery`, `call`, `printf` (plain/hugo) | 544,644 |
| `rtpairs fmt` | every model value through 57 `printf` formats (1 and 2 args) | 28,044 |
| `rthtml` (1 × 100k, 2 × 150k, 3 × 150k scripts) | random HTML skeletons with actions in every context (attribute names/values quoted and unquoted, `on*`, `style`, `srcset`, URL parts, `<script>` of 17 types with regexp/division/template-literal/JSON tokens, `<style>` strings/urls/comments, comments, RCDATA, branches ending in different contexts, range re-entry, `{{template}}` in other contexts, recursion, missing templates): parse, `Prepare` (error text + `ErrorCode`), escaped trees of the namespace incl. derived templates, execution with 1–3 values (+ Hugo path) | 400,000 scripts, 2.9M operations |
| `rtns` (1 × 100k, 2 × 150k) | random html namespace operation sequences: `New`, `Parse` (redefinitions, empty/comment-only bodies), `Lookup`, `AddParseTree` (parsed and shared trees), `Clone`, `CloneShallow`, `Funcs`, `Delims`, `Option`, `Execute`, `ExecuteTemplate`, Hugo path, `Templates`, identity/tree checks, escaped trees, after-execute errors | 250,000 scripts, 2.67M operations |
| `rttns` (1 × 100k, 2 × 150k) | the same for text/template (`DefinedTemplates` names sorted, deviation 4) | 250,000 scripts, 2.60M operations |
| `rtlayout` (1 × 20k, 2 × 60k) | the 122 layouts of this repository with 1–3 HTML-significant or in-action mutations, stub functions returning a nil `any` (contract C7): parse, `Prepare`, escaped trees, execution | 80,000 scripts, 520k operations |

Findings (fixed; regression cases in the fixtures above):

- a nil `*template.TryError` (`(try f).Err` without an error) printed as
  `%!s(*template.TryError=<nil>)` by `printf "%s"`/`%q`/`%x`/`%X` where Go
  prints `<nil>` (`hugo.rs`: declared to go-fmt as a type whose `Error`
  panics on a nil receiver);
- a host function or method returning a nil `any` as `Invalid` (as the
  contract tells hosts to) made `f.X`, `(try f).Value.X` and similar
  silently empty where Go fails with `nil pointer evaluating interface
  {}.X` (`exec.rs` `eval_call_inner`); the same for `TryValue.Value` after
  an error and any object field holding `Invalid` (`exec.rs` `eval_field`);
- `TryError.Err` (`template.ExecError`) had no `Name`/`Err` fields and no
  `Unwrap`, and its `Err` had no `*fmt.wrapError` type and `Unwrap`
  (`hugo.rs` `ErrorValue`); `.Err`/`.Cause` were new objects on every
  access, so `eq .Err.Cause .Err.Cause` was false (now created once).

Left as documented deviations: host signatures (6), including the error
location when an argument fails before a host's arity check; a host's
typed error as `.Err.Cause` (9: `go_value::Error` is a message, e.g. an
object error's fields are unreachable); methods of a nil
`*template.TryError` (`(try 1).Err.Error`: Go calls it and reports the
nil dereference, here `nil pointer evaluating`); static interface types
(15) incl. the `slice` index case; parse/escape/exec nesting beyond the
stack (3).

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
GOTOOLCHAIN=go1.27.1 go run -tags gotemplate_oracle ./tools/go-oracle/gotemplate rtfixtures crates/gotemplate/tests/fixtures
```

Red-team corpora (repo root; `$C` is any scratch directory; each mode
takes `<out.txt.gz> <seed> <n>`):

```sh
GOTOOLCHAIN=go1.27.1 go build -tags gotemplate_oracle -o $C/oracle ./tools/go-oracle/gotemplate
$C/oracle rtparse $C/rtparse1.txt.gz 1 200000     # also seeds 2, 3 × 300000
$C/oracle rtexec $C/rtexec1.txt.gz 1 20000        # also seeds 2, 3 × 150000
$C/oracle rthtml $C/rthtml1.txt.gz 1 100000       # also seeds 2, 3 × 150000
$C/oracle rtns $C/rtns1.txt.gz 1 100000           # also seed 2 × 150000
$C/oracle rttns $C/rttns1.txt.gz 1 100000         # also seed 2 × 150000
$C/oracle rtlayout $C/rtlayout1.txt.gz 1 20000    # also seed 2 × 60000 (run from the repo root)
$C/oracle rtpairs $C/rtpairs.txt.gz pairs          # and: rtpairs $C/rtfmt.txt.gz fmt
$C/oracle rtexeclist $C/list.txt.gz cases.txt     # one case per line: <data> <quoted option> <quoted source>
cd crates/gotemplate
GOTEMPLATE_RT_PARSE=$C/rtparse1.txt.gz cargo test --test parse_oracle parse_redteam -- --ignored
GOTEMPLATE_RT_EXEC=$C/rtexec1.txt.gz:$C/rtpairs.txt.gz cargo test --test exec_oracle exec_redteam -- --ignored
GOTEMPLATE_RT_HTML=$C/rthtml1.txt.gz:$C/rtns1.txt.gz:$C/rttns1.txt.gz:$C/rtlayout1.txt.gz \
  cargo test --test html_exec html_redteam -- --ignored
```

`GOTEMPLATE_RT_VERBOSE=1` prints the complete lines of each difference.

`tools/go-oracle/gotemplate/exectests/data.go` is a verbatim copy of parts
of the fork's `exec_test.go` (line ranges in its header, plus `//nolint`
comments); re-copy it by hand if that file changes. `sync-fork.sh` marks
the copied fork as generated code, so golangci-lint skips it.

## Gaps

- The probe site of template-engine spec Appendix A/B and the seeksnack
  escdump (`$SCRATCH/work/template-engine/…`) were not committed by the
  first session; the escdump oracle here uses the layouts in this
  repository instead. The seeksnack layouts (private repo) should be added
  to the escdump corpus when available.
