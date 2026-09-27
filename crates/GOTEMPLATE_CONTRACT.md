# gotemplate host contract

This is the pinned contract between the Wave A `gotemplate` crate (Go `text/template` +
`html/template` as forked in `tpl/internal/go_templates`, go1.24 semantics) and the Hugo layer
(`nh-tplimpl`, which is the only consumer apart from `nh-tpl::strip_html` and nh-i18n's message
templates).

- It extends §16 of the template-engine spec
  (`$SCRATCH/specs/template-engine.md`, "MANDATORY: engine contract required by the Hugo layer").
  Where the two differ, this file wins.
- The Wave A gotemplate task implements every clause as an acceptance criterion and documents the
  resulting public API in `crates/gotemplate/PORTING.md` under "Host contract".
- Wave B task T13 reviews the gotemplate API against this file before building on it.
  `crates/nh-tplimpl/src/engine.rs` mirrors clause C1 as a Rust trait and is replaced by a
  re-export when gotemplate lands.
- Changing a signature in C1 needs T13's agreement.

Go line references are to `tpl/internal/go_templates/texttemplate/{exec.go,funcs.go,hugo_template.go}`
and `htmltemplate/{content.go,hugo_template.go,js.go}` in this repository.

## C1. The `ExecHelper` trait

The Rust equivalent of `texttemplate.ExecHelper` (hugo_template.go:45-51), extended with Hugo's
truthiness. Signatures, using `go_value` types:

```rust
pub type Func = Arc<dyn Fn(HostCtx<'_>, &[Value]) -> go_value::Result<Value> + Send + Sync>;

pub trait ExecHelper: Send + Sync {
    fn init(&self, ctx: HostCtx<'_>, template_name: &str) {}
    fn get_func(&self, ctx: HostCtx<'_>, name: &str) -> Option<Func>;
    fn has_method(&self, ctx: HostCtx<'_>, receiver: &Value, name: &str) -> bool;
    fn call_method(&self, ctx: HostCtx<'_>, receiver: &Value, name: &str, args: &[Value]) -> go_value::Result<Value>;
    fn get_map_value(&self, ctx: HostCtx<'_>, receiver: &Value, key: &Value) -> Option<Value>;
    fn on_called(&self, ctx: HostCtx<'_>, name: &str, args: &[Value], result: &Value) {}
    fn is_true(&self, v: &Value) -> bool;
}
```

- `get_func` is consulted first for every function name. If it returns `None`, the engine falls
  back to its own builtins (Go `findFunction`), and after that raises `%q is not a defined
  function`.
- Parse-time name checking uses a separate set of known names: the host's func-map names, the
  builtins, and the `_html_template_*`/`_eval_args_` escapers.
- `has_method` is called before the arguments are evaluated. `call_method` is called after.
- The engine owns the builtins (`and`, `or`, `not`, `len`, `index`, `slice`, `print*`, `eq`…,
  `call`, `html`, `js`, `urlquery`). Hugo overrides most of them through `get_func` (TE §14.11).
  `and`/`or` always go through the engine's short-circuit path (C6), even when `get_func`
  resolves the name.

## C2. Field, method and key resolution (`evalField` order)

For `.Name` (optionally with arguments) on receiver `R`:

1. `R` is `Invalid`: the result is `Invalid`, with no error (default `missingkey`).
2. `R` is a `TypedNil(T)` with `go_value::typed_nil_kind(T) == Interface`: error
   `nil pointer evaluating T.Name`.
3. `helper.has_method(ctx, R, "Name")`: call it. This applies to every receiver kind (`Object`,
   `List`/`Map` with a named Go type, `Time`, `Safe`, `String`, typed-nil pointers). Methods win
   over fields and map keys, including on named `List`/`Map` values (`page.Data.Pages` is a
   method, `.Data.Singular` is a key).
4. `R` is an `Object`: `Object::field("Name")`.
   - If a field is found and arguments were given: error `%s has arguments but cannot be invoked as function`.
5. `R` is a `Value::Map` or a `Kind::Map` object: `helper.get_map_value(ctx, R, "Name")`.
   - `None` gives `Invalid`, which prints `<no value>` in text templates.
   - A map lookup with arguments: error `%s is not a method but has arguments`.
6. `R` is a typed-nil pointer: error `nil pointer evaluating T.Name`.
7. Otherwise: error `can't evaluate field Name in type T`.

Chains (`.A.B.C`, `(pipe).A`, `$x.A`) apply these rules at each step.

## C3. Values the engine creates

- **Number literals (Go `idealConstant`) for untyped (`any`) parameters and commands:**
  - Complex literal: error (not supported).
  - Float syntax (contains `.eEpP`, not hex or rune): `float64`.
  - Integer, including rune literals such as `'a'`: `int`. An integer that overflows `int` is the
    error `%s overflows int`.
- **Other literals:** strings give `string`. `true`/`false` give `bool`. `nil` passed as an
  argument gives `Invalid`, and `nil` used as a command is the error `nil is not a command`.
- **Variables:** a variable holds whatever value it was assigned.

## C4. Host context, re-entrancy, sharing

- `execute_with_context(ctx, template, writer, data)` takes an opaque `HostCtx`, i.e.
  `&dyn Any`, which is the Hugo `TplContext`.
  - The engine passes that same `ctx` to every helper call and every func/method call.
  - It never rebuilds or replaces the context. Hugo derives child contexts itself: in `partial`,
    in shortcodes, and in `ExecuteWithContext`'s `CurrentTemplateInfo`.
- Execution is re-entrant. A func or method called during an execution may itself execute
  templates, including the same one, recursively and on the same thread. Examples: partials,
  markdown render hooks reached through `.Content`, `resources.ExecuteAsTemplate`, and
  `markdownify`.
  - Templates are immutable after `prepare()` (escaping) and are `Send + Sync`, so they can be
    shared with `Arc`.
  - All execution state belongs to the individual call.
  - The engine holds no lock while calling out to a func, a method or the helper.

## C5. Truthiness

`helper.is_true(v)` replaces Go's `isTrue` (hugo_template.go:434-436) everywhere Go uses it:

- `if` and `with`/`else with` (through `walkIfOrWith`);
- `and`/`or` (through `truth()`);
- the `not` builtin.

`range` decides emptiness by length, not by truth. A `range` over `Invalid` runs the `else`
branch.

## C6. `and` / `or`

- Both short-circuit, evaluating operands left to right (hugo_template.go:339-360).
- The result is the first operand whose truth equals `name == "or"`. If there is none, the result
  is the last operand (or the piped final value).
- The result is that operand's Value. It is not converted to `bool`.

## C7. Arguments passed to funcs and methods (Go `evalArg`/`validateType`)

- A `TypedNil(T)` argument with `typed_nil_kind(T) == Interface` becomes `Invalid`. In Go, a nil
  interface converted to the `any` parameter is an untyped nil.
- `Invalid` arguments are passed as `Invalid`.
- All other values are passed unchanged. Typed-nil pointers, slices and maps stay typed.
- Hosts with typed Go parameters report Go's errors themselves (for example
  `invalid value; expected string`, `wrong type for value; expected %s; got %s`). The engine does
  not know the parameter types.
- After each command of a pipeline, a nil value of the empty-interface type becomes `Invalid`
  (Go `value.Kind()==Interface && NumMethod()==0`, exec.go `evalPipeline`).
  - Hosts represent a nil `any` result as `Invalid` directly.
  - A nil non-empty interface (`TypedNil("page.Page")`) is not unwrapped.

## C8. Printing (`printValue`, text/template)

| Value | Printed as |
|---|---|
| `Invalid` | `<no value>` |
| `TypedNil(T)` with pointer or interface kind | `<nil>` |
| `TypedNil(T)` with slice kind | `[]` |
| `TypedNil(T)` with map kind | `map[]` |
| any other value | go-fmt `Sprint` semantics |

For any other value, `Object::go_error()` and `Object::go_string()` are used when present
(Go `error`/`fmt.Stringer`). Chan and func values are the error `can't print %s of type %s`.

## C9. html/template escapers

- `stringify` (content.go) first applies Hugo's `indirect` override (htmltemplate/hugo_template.go:60-69):
  - an `Object` with `printable_value()` is replaced by that value (`hstring.HTML` becomes
    `template.HTML`, `page.Summary` becomes its HTML text);
  - `Value::Safe(kind, s)` is the corresponding content type;
  - `Invalid` arguments are skipped (issue 25875), so missing values print as the empty string.
- `jsValEscaper` does not use `indirect`. It uses `indirectToJSONMarshaler`
  (`Object::marshal_json`) and then `fmt.Stringer` (`Object::go_string`), as in js.go:153-250.

## C10. Identity must survive the engine

- The engine must not copy `Value::List`/`Value::Map`/`Value::Object` payloads into new `Arc`s.
  A value handed to a helper or func is the same `Arc` the engine received.
- The helper's `mainsections` special case depends on this. The case is: receiver `Value::Map(m)`
  with `Arc::ptr_eq(m, site_params)` and a name that EqualFolds to `"mainsections"`
  (tplimpl/template_funcs.go:88-114).
- `eq` on pages and resources also depends on this, through `Object::identity`.

## C11. `range`

Iteration covers the following kinds:

- ints (go1.22: `0..n-1`, one variable only);
- slices, including `Kind::Slice` objects through `Object::list()`;
- maps, whose keys come in `fmtsort` order: `Map` iterates in byte order already, and
  `Kind::Map` objects give their keys through `map_keys()`;
- `Invalid`, which counts as empty.

Strings and other kinds are the error `range can't iterate over %v`. `break` and `continue` are
supported.

## C12. Errors and `try`

- Execution errors use Go's format: `template: %s: executing %q at <%s>: %s`, with the node
  context.
- Func and method errors are wrapped as `error calling %s: %w`.
- `try` (hugo_template.go:296-307, 427-429) turns an error or panic into a
  `TryValue{Value, Err}`. The Rust shape is a host `Object` with `.Value` and `.Err`.

## C13. API surface nh-tplimpl needs

- Text and html namespaces sharing the `common` map.
- `New(name)` and `Parse(src)`, returning the define/block trees.
- `Lookup`, `AddParseTree`, `Clone` (text), Hugo's `CloneShallow` (html), `Templates()`/`All()`.
- `parse::IsEmptyTree`, and the association rule that an empty tree never replaces a non-empty
  one.
- Parse trees with mutable nodes and `String()`, which `templatetransform` rewrites.
- `Prepare()`: html escaping runs once and produces the derived `name$htmltemplate_*` templates.
- `execute_with_context`.
- `StripTags`.
- The escaper func map (`GoFuncs`), merged into Hugo's func lookup.

## C14. Tests

Ship a small test FuncMap and `ExecHelper` in the crate's tests, and cover C2 to C11 with
differential tests against the Go fork. Use the probe site (`$SCRATCH/work/template-engine/tsite`
and `expected/`) and the escaper dumps (`$SCRATCH/work/template-engine/escdump`).
