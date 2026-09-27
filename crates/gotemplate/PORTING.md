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
  A builtin reached through the host lookup is called like any host
  function (Go: `isBuiltin` is false then), so `call` errors
  (`unreachable`, as in Hugo) and arity errors are reported as
  `error calling …`.

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

1. **Complex numbers** are not representable: complex literals are an
   execution error.
2. **`slice` builtin** copies (the value model has no shared backing
   arrays; `cap == len`), so re-slicing beyond `len` is out of range.
3. **Stack depth.** `MAX_EXEC_DEPTH` is Go's 100000, but Rust stacks do not
   grow: a 10000-deep `{{template}}` recursion needs ~64 MB of stack. Hosts
   that allow deep recursion must run executions on threads with large
   stacks. (Hugo limits partial nesting to 999 levels.)
4. **`DefinedTemplates`** lists names sorted (Go: map order).
5. **Template names** are Rust `String`s (lossy for non-UTF-8 `define`
   names).
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

## Verification

All fixtures come from `tools/go-oracle/gotemplate` (go1.27.1 toolchain,
the fork copied by `sync-fork.sh`, build tag `gotemplate_oracle`).

| test | what |
|---|---|
| `tests/html_escdump.rs` | the escaper reproduces Go's escaped trees of all 120 layouts in this repository (`docs/layouts`, `create/skeletons/theme/layouts`, `tpl/tplimpl/embedded/templates`), namespaces built as tplimpl does (shared namespace + `CloneShallow` per overlay/baseof), every tree incl. the derived `name$htmltemplate_*` templates, byte for byte (665 KB dump) |
| `src/html/tests/oracle.rs` | 39,048 escaper calls (17 escapers × 2,297 argument lists: every byte, special runes, invalid UTF-8, fuzz, Safe types, numbers, nils, time, collections, Stringers, errors, json/text marshalers, PrintableValue types, multi-arg), 53,730 transition runs (82,260 steps from 170 start contexts: contexts, error texts, brace slices incl. capacity/aliasing), 59,569 leaf-function checks — all equal to Go |
| `src/html/tests/go_tests.rs` | Go's html_test, js_test, css_test, url_test, transition_test tables |
| `tests/html_strip_tags.rs` | `stripTags` on 3,949 inputs |
| `tests/smoke.rs` | text/template end-to-end smoke cases |

Regenerate (repo root):

```sh
tools/go-oracle/gotemplate/sync-fork.sh
GOTOOLCHAIN=go1.27.1 go run -tags gotemplate_oracle ./tools/go-oracle/gotemplate escfuncs crates/gotemplate/tests/fixtures/html
GOTOOLCHAIN=go1.27.1 go run -tags gotemplate_oracle ./tools/go-oracle/gotemplate escdump \
  crates/gotemplate/tests/fixtures/html/escdump.txt docs/layouts create/skeletons/theme/layouts tpl/tplimpl/embedded/templates
```

## Gaps

- The probe site of template-engine spec Appendix A/B and the seeksnack
  escdump (`$SCRATCH/work/template-engine/…`) were not committed by the
  first session; the escdump oracle here uses the layouts in this
  repository instead. The seeksnack layouts (private repo) should be added
  to the escdump corpus when available.
