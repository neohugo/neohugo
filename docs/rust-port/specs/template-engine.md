# Spec: The Go template engine in neohugo (text/template + html/template + tplimpl glue) — Rust port

> **Byte-parity sections obsolete.** This spec is research for the old byte-for-byte port
> (the `crates/` tree deleted in T00 of [`REWRITE_PLAN.md`](../REWRITE_PLAN.md); recoverable with
> `git show go-parity-final:crates/<path>`, commit `be02933a`, local tag). The Rust rewrite in
> `rust/` compares structurally (REWRITE_PLAN.md §7), so every byte-parity target, golden-byte
> count and "reproduce Go's bytes" rule below is obsolete. The Hugo semantics it documents (Go
> file and line references, parity traps) remain a reference; scratch paths (`/Users/…`,
> `/private/tmp/…`, `$W`, `$SP`, `golden/run1`) no longer exist. Current state:
> [`HANDOFF.md`](../HANDOFF.md).

Agent: `template-engine`. Scope: `tpl/internal/go_templates/**` (forked Go packages), `tpl/tplimpl/**`
(Hugo glue), `tpl/internal/templatefuncsRegistry.go`, `tpl/tplimplinit`, `tpl/template.go`,
`tpl/partials`, `common/hreflect`, plus the Go stdlib behaviour (fmt, strconv, encoding/json) that the
engine uses to turn values into output bytes.

All Go paths are relative to `/Users/blackb1rd/git/github/org/neohugo` unless stated otherwise. Line
numbers are from HEAD `d5930ba1f` (tag v0.148.2).

Artifacts produced for this spec (all under
`/private/tmp/claude-501/-Users-blackb1rd-git-github-org-neohugo/0835ef6d-534f-4ac6-a7eb-50e9240d6eb1/scratchpad/work/template-engine/`):

| Artifact | What it is |
|---|---|
| `tsite/` | Minimal Hugo site with probe templates (`layouts/index.html` = html/template probes, `layouts/index.json` = text/template probes, partials `p1.html`, `ret.html`). |
| `expected/probe-index.html`, `expected/probe-index.json` | Output of the Go binary for the probe site (`neohugo-go -d ../tout`). Use as a conformance fixture for the Rust engine. |
| `escdump/` | Go tool that uses a *copy of the forked* html/template to parse all seeksnack layouts the way tplimpl does (baseof+overlay; shared namespace for partials) and dumps every parse tree **after contextual escaping**. |
| `escdump/dump.txt` | Escaped trees for all seeksnack layouts (4343 lines). Oracle for the Rust escaper: the Rust port can print its trees in the same `Node.String()` format and diff. |
| `escdump/dump-embedded.txt` | Same for the embedded templates seeksnack executes (alias.html, sitemapindex.xml, _shortcodes/ref.html, _markup/render-link.html, _markup/render-table.html). |

---------------------------------------------------------------------------------------------------

## 0. Executive summary (most important findings)

1. **The forked Go template packages are go1.24.0**, not the toolchain version (the binary is built with
   go1.27.1). `scripts/fork_go_templates/main.go:18` — "The current is built with 3901409b5d
   [release-branch.go1.24] go1.24.0". The Rust port must reproduce **go1.24 html/template** behaviour.
   Concretely vs go1.27 stdlib: no `<meta content="…url=…">` URL filtering (`stateMetaContent`/
   `stateMetaContentURL` do not exist — seeksnack's 1,482 alias pages contain
   `<meta http-equiv="refresh" content="0; url={{ .Permalink }}">`, escaped with plain
   `_html_template_attrescaper` only), `isJSType("")` is **false** (stdlib 1.27 says true), context
   equality ignores `jsBraceDepth`, `escapeBranch` does not `clone()` the context.
2. **Hugo modifications are confined to two files** (`texttemplate/hugo_template.go`, 451 lines;
   `htmltemplate/hugo_template.go`, 94 lines) plus mechanical renames. They add: an `ExecHelper`
   hook interface (function lookup, method lookup, map lookup, call hook), a `context.Context`
   threaded through execution and **auto-injected as first argument** into any func/method whose first
   parameter is `context.Context`, **case-insensitive key lookup for `maps.Params`** (the only
   case-insensitivity; methods/fields/other maps are exact-case), **`IsZero()`-aware truthiness**
   (`hreflect.IsTruthfulValue`) used by if/with/and/or/not, the `try` keyword, and unwrapping of
   `types.PrintableValueProvider` (hstring.HTML, page.Summary) to `template.HTML` inside html escapers.
3. **Hugo's func map shadows Go builtins**: `eq ne lt le gt ge index slice print printf println` are
   Hugo's implementations; the `js` *namespace* shadows the `js` builtin (so `{{ x | js }}` returns the
   JS namespace object!). Remaining Go builtins actually reachable: `and or not len html urlquery call`,
   plus the html/template escaper funcs (`_html_template_*`, `_eval_args_`).
4. **Output format decides the engine**: HTML, RSS, sitemap, sitemapindex, alias, AMP → html/template
   (contextual escaping); JSON, robots.txt, CSV, calendar, markdown, webmanifest, gotmpl and
   `resources.ExecuteAsTemplate` → text/template. seeksnack: `index.xml` (RSS) and `sitemap.xml` are
   **html/template** (e.g. `<title>{{ .Title }}</title>` in RSS is RCDATA-escaped → `Alice&#39;s`).
5. html/template **rewrites template text**, not just values: HTML comments `<!-- … -->` in text state
   are deleted (evidence: seeksnack's `<!-- RSS etc -->`, `<!-- Facebook -->`, `<!-- Google Tag Manager -->`
   are absent in golden output), JS `//…` and `/*…*/` comments inside `<script>` are removed (block
   comment → `" "` or `"\n"`), CSS comments → `" "`, a stray `<` in text → `&lt;` (except `<!DOCTYPE`,
   case-insensitive), and `<script`/`</script`/`<!--` inside JS string/regexp literals → `\x3C…`.
6. **Value printing must be a Go-`fmt` clone.** 1,678 golden files contain `content="%!s(&lt;nil&gt;)"`
   produced by `{{ $og_image = (printf "%s" .Params.image) | absLangURL }}` with a missing param
   (`layouts/partials/head.html:157`). Missing values print `""` in html/template but `<no value>` in
   text/template; floats print with Go `%v` = `strconv.FormatFloat(f,'g',-1,64)` (`1e+06`, `1.5e+10`,
   `-0`); maps print `map[k:v …]` with sorted keys; `time.Time` prints `2020-09-06 15:46:26.955 +0000 UTC`.
7. **JS contexts use Go `encoding/json` v1** (`jsValEscaper`): `"Lay's"` → `"Lay's"` but `<>&` →
   `\u003c\u003e\u0026`, numbers/true/null are padded with spaces (`var f =  5 ;`), time.Time →
   `"2020-09-06T15:46:26.955Z"`. JS *string* contexts use `jsStrEscaper`: `'` → `\u0027`, `/` → `\/`,
   `+` → `\u002b` (golden JSON-LD: `"@id": "https:\/\/seeksnack.com\/…"`, `"datePublished":
   "2020-09-06 15:46:26.955 \u002b0000 UTC"`). serde_json is not byte-compatible → write a custom
   encoder. (Notation: in this spec `\uXXXX` written in code spans always means the literal six output
   bytes backslash-u-X-X-X-X; "U+XXXX" means the Unicode character itself.)
8. seeksnack escaper census (occurrences in `escdump/dump.txt`): htmlescaper 440, attrescaper 350,
   urlfilter|urlnormalizer|attrescaper 159, jsstrescaper 95, srcsetescaper 33, urlescaper 21,
   rcdataescaper 18, jsvalescaper 17 (+4 in the unused modalImage shortcode), htmlnamefilter 2
   (`<h{{ .Level }}>` in render-heading), cssvaluefilter 1 (`<style>{{ .Content | safeCSS }}`), and one
   predefined-escaper merge (`{{ "Check out this site " | html }}` inside a URL query →
   `| _html_template_urlescaper | html`).
9. Recommended Rust approach: **line-by-line port** of parse (2,529 lines), exec/funcs/template (~2,700),
   html/template (~4,540), fmtsort (154), the Hugo glue (tplimpl ~3,460 + registry), and targeted ports of
   Go `fmt` printing, `strconv` float formatting/quoting and `encoding/json` encoding. Replace Go
   reflection with a `Value` enum + a `TplObject` trait with explicit per-type method tables. Port Go's
   own test tables (exec_test, escape_test, js/css/url/html tests) as Rust tests.

---------------------------------------------------------------------------------------------------

## 1. Inventory of the forked packages

Fork script: `scripts/fork_go_templates/main.go` (copies `text/template`, `text/template/parse`,
`html/template`, `internal/fmtsort`, `internal/testenv`, `internal/cfg` from a go1.24.0 source tree,
rewrites imports, and applies the renames below with `strings.NewReplacer`, main.go:52-65):

```
"type state struct"                                   -> "type stateOld struct"
"func (s *state) evalFunction"                        -> "func (s *state) evalFunctionOld"
"func (s *state) evalField("                          -> "func (s *state) evalFieldOld("
"func (s *state) evalCall("                           -> "func (s *state) evalCallOld("
"func isTrue(val reflect.Value) (truth, ok bool) {"   -> "func isTrueOld(val reflect.Value) (truth, ok bool) {"
content.go: remove the `type ( CSS string; HTML string; ... )` block and use stdlib html/template types
            (gofmt -r "CSS -> htmltemplate.CSS" etc. for CSS, HTML, HTMLAttr, JS, JSStr, URL, Srcset)
```
Files whose name contains `hugo` are Hugo-owned and are not overwritten.

### 1.1 File list and non-test line counts

`tpl/internal/go_templates/texttemplate/parse` (upstream, no Hugo changes):

| file | lines | notes |
|---|---:|---|
| lex.go | 687 | lexer (delims `{{`/`}}`, trim markers `{{- `/` -}}`, comments `/* */`) |
| node.go | 1011 | AST nodes + `String()`/`writeTo` (used for error context and by the escaper) |
| parse.go | 831 | recursive-descent parser, `IsEmptyTree` (parse.go:270) |
| **total** | **2529** | |

`tpl/internal/go_templates/texttemplate`:

| file | lines | Hugo change |
|---|---:|---|
| exec.go | 1135 | renames only (`stateOld`, `evalFunctionOld`, `evalFieldOld`, `evalCallOld`, `isTrueOld`) |
| funcs.go | 785 | none semantic (go1.24 `builtinFuncsOnce`; comparison error message text) |
| hugo_template.go | 451 | **Hugo**: ExecHelper, Executer, state, evalFunction, evalField, evalCall, isTrue, TryValue, All |
| template.go | 237 | none (go1.24: Clone doesn't copy `option`) |
| option.go | 72 | none (`missingkey` option; Hugo never sets it → default `mapInvalid`) |
| helper.go | 178 | none (ParseFiles/ParseGlob — unused by Hugo) |
| doc.go | 497 | docs only |

`tpl/internal/go_templates/htmltemplate`:

| file | lines | Hugo change |
|---|---:|---|
| escape.go | 997 | `godebug` for `jstmpllitinterp` commented out (escape.go:167); go1.24 (no meta-content URL) |
| transition.go | 688 | go1.24 (no `tMetaContent*`, no `elementMeta`) |
| template.go | 531 | none |
| js.go | 490 | uses stdlib `htmltemplate.JS/JSStr`; go1.24 `isJSType` (no `""`) |
| context.go | 292 | go1.24 (jsBraceDepth not compared in `eq`, no `clone`) |
| html.go | 270 | none |
| css.go | 262 | none |
| error.go | 249 | none |
| url.go | 216 | none |
| attr.go | 175 | none |
| content.go | 102 | types from stdlib `html/template`; `indirect` renamed `doIndirect` |
| hugo_template.go | 94 | **Hugo**: `Prepare`, `All`, `StripTags` export, `indirect` override, `CloneShallow` |
| *_string.go (attr, delim, element, jsctx, state, urlpart) | 183 | generated stringers (used by `mangle`) |
| doc.go | 240 | docs only |
| **total non-doc** | **4539** | |

`tpl/internal/go_templates/fmtsort/sort.go` — 154 lines (upstream `internal/fmtsort`: map key sorting for
`range` over maps). `testenv` (817 lines) and `cfg` (74) are test-only — do not port.

Hugo glue:

| file | lines | role |
|---|---:|---|
| tpl/tplimpl/templatestore.go | 2096 | store, parse pipeline, lookups, execution entry point |
| tpl/tplimpl/templates.go | 366 | parse namespaces (`templateNamespace`), baseof application, `needsBaseTemplate` |
| tpl/tplimpl/templatetransform.go | 352 | AST rewrites: partial `return`, `templates.Defer`, `$_hugo_config`, `.Inner` detection |
| tpl/tplimpl/templatedescriptor.go | 238 | template descriptor matching weights (lookup — see templates-inventory spec) |
| tpl/tplimpl/template_funcs.go | 175 | `templateExecHelper` (GetFunc / GetMethod / GetMapValue / OnCalled) |
| tpl/tplimpl/legacy.go | 130 | pre-0.146 layout path mappings |
| tpl/tplimpl/template_info.go | 46 | `ParseInfo` (IsInner, HasReturn, Config) |
| tpl/tplimpl/{category,subcategory}_string.go | 55 | stringers |
| tpl/template.go | 189 | `tpl.Template` interface, `tpl.Context` dispatchers, `StripHTML` |
| tpl/internal/templatefuncsRegistry.go | 325 | namespace registry (+ doc generation, not needed) |
| tpl/tplimplinit/tplimplinit.go | 96 | `CreateFuncMap` (merges namespaces and aliases) |
| tpl/partials/partials.go + init.go | 308 | `partial`, `partialCached`, `return` alias |
| common/hreflect/helpers.go | 295 | truthiness, method cache, context-type detection |

### 1.2 Upstream (go1.24) vs go1.27.1 stdlib differences that matter

(`diff $(go env GOROOT)/src/html/template/X.go tpl/internal/go_templates/htmltemplate/X.go`)

| area | go1.24 fork (port THIS) | go1.27.1 stdlib | seeksnack impact |
|---|---|---|---|
| `<meta … content=…>` | `content` attr is `contentTypeUnsafe` → plain attr escaping | `elementMeta`, `attrMetaContent`, `stateMetaContent`, `stateMetaContentURL` (url= part gets `_html_template_urlfilter`) | alias.html (1,482 files) `content="0; url={{ .Permalink }}"` → attrescaper only |
| `isJSType("")` (js.go:455) | `""` not in list → `<script type="">` body is **not** JS | `""` is JS | none (no empty type) |
| context `eq`/`clone` | jsBraceDepth ignored/not cloned | compared/cloned | none |
| `tJSDelimited` `</script` check | `bytes.Equal(bytes.ToLower(...))` | `bytes.EqualFold` | none (equivalent for ASCII) |
| error texts | older | newer | none (errors only) |
| parse `maxStackDepth` | absent | 10000 | none |
| text `Clone` copies `option` | no | yes | none |

---------------------------------------------------------------------------------------------------

## 2. Hugo-specific modifications (exact behaviour)

### 2.1 `texttemplate/hugo_template.go`

* `GoFuncs = builtinFuncs()` (l.37) — exported so Hugo can merge builtins into its func map.
* `Preparer` interface (`Prepare() (*Template, error)`); `*Template.Prepare()` returns itself (l.97).
* `ExecHelper` interface (l.45-51):
  `Init(ctx, tmpl)`, `GetFunc(ctx, tmpl, name) (fn, firstArg, found)`,
  `GetMethod(ctx, tmpl, receiver, name) (method, firstArg)`,
  `GetMapValue(ctx, tmpl, receiver, key) (value, found)`,
  `OnCalled(ctx, tmpl, name, args, result)`.
* `Executer.ExecuteWithContext(ctx, p Preparer, wr, data)` (l.67-94): `tmpl, err := p.Prepare()`
  (for html templates this runs the escaper, see 2.2), wraps `data` in `reflect.ValueOf` unless it is
  already a `reflect.Value`, builds `state{ctx, helper, prep, tmpl, wr, vars: [{"$", value}]}`,
  calls `helper.Init`, then `executeWithState` → `errRecover` + error if `Tree == nil || Root == nil`
  (`"%q is an incomplete or empty template"`) + `state.walk(value, t.Root)`.
* `state` (l.115-124) = upstream state + `ctx`, `prep`, `helper`.
* `evalFunction` (l.126-151): if helper != nil: `isBuiltin = name == "and" || name == "or"` and
  `function, first, ok = helper.GetFunc(ctx, prep, name)`; if not found, upstream `findFunction`
  (template's exec funcs, then builtins); error `%q is not a defined function`. If `first` is valid it
  is passed to evalCall as an extra leading argument.
* `evalField` (l.156-259) = upstream evalField with two hooks:
  1. method lookup: `method, first = helper.GetMethod(ctx, prep, ptr, fieldName)` (ptr = addressable
     receiver's address unless interface/pointer); if valid → `evalCall(... first)`. Then (redundant)
     `ptr.MethodByName(fieldName)`.
  2. map lookup: for `reflect.Map` receivers whose key type accepts a string:
     `result, _ = helper.GetMapValue(ctx, prep, receiver, nameVal)`; if invalid, `missingkey` option
     (default `mapInvalid` → return the invalid value, i.e. "no value").
  Everything else is upstream: invalid receiver → return zero Value (no error; with `missingkey=error`
  it would error); nil interface → error `nil pointer evaluating %s.%s`; struct field by exact
  name (`FieldByName`, promoted fields included), unexported → error; field with args → error; map
  field with args → error `%s is not a method but has arguments`; nil pointer → error; otherwise
  `can't evaluate field %s in type %s`.
* `evalCall` (l.294-432) = upstream evalCall plus:
  * `name == "try"`: recover panics → return `TryValue{Value:nil, Err:&TryError{Err, Cause}}`; on
    success return `TryValue{Value: result}` (l.296-307, 427-429). (`try` is an alias registered in
    `tpl/safe/init.go:76`.)
  * `first ...reflect.Value` (injected context) occupy argv[0..numFirst) and count towards `numIn`
    (l.313-318, 365-374, 407-410).
  * `helper.OnCalled(...)` after a successful call (l.421-424; only acts in watch mode for `Unmarshal`).
  * `and`/`or` short-circuit (l.339-360): each arg is evaluated with `evalArg(dot, reflect.Value type,
    arg)`; returns the **first arg whose truth equals `name=="or"`**, else the last arg (or the piped
    `final` value). Truth = Hugo `isTrue` (below). Returned value is the operand, not a bool.
  * `call` builtin special case (l.395-405) — upstream.
  * errors: `error calling %s: %w` (l.417).
* `isTrue(val) = hreflect.IsTruthfulValue(val), true` (l.434-436) — replaces upstream truthiness
  **everywhere** (if/with via `walkIfOrWith`, `and`/`or` via `truth()`, `not`).
* `All()` iterator over the namespace's templates (l.438-451).

### 2.2 `htmltemplate/hugo_template.go`

* `GoFuncs = funcMap` (l.32) — the escaper funcs (`_html_template_attrescaper`, …, `_eval_args_`,
  escape.go:65-83) are merged into Hugo's func lookup.
* `Prepare()` (l.35-40): `t.escape()` then return `t.text` (the underlying text template whose tree
  has been rewritten). Called once per template at store construction (`prepareTemplates`,
  templatestore.go:1599) and again (no-op) at each execution.
* `StripTags(html)` export (l.56) — used by `tpl.StripHTML` (→ `.Plain`, `plainify`, `countwords`, …).
* `indirect(a)` override (l.60-69): `doIndirect(a)` (deref pointers) and then, if the result implements
  `types.PrintableValueProvider`, returns `PrintableValue()`. Implementations:
  `common/types/hstring/stringtypes.go:34` `hstring.HTML.PrintableValue() = template.HTML(s)` (render
  hook `.Text`, `.PlainText`…), `resources/page/page_markup.go:67` `Summary.PrintableValue() = s.Text`
  (template.HTML). Used only by `stringify` (content.go:68) → all html escapers see these as
  `contentTypeHTML`. (`jsValEscaper` does *not* go through `indirect`; it uses
  `indirectToJSONMarshaler` and then `fmt.Stringer` → `hstring.HTML.String()`.)
* `CloneShallow()` (l.72-94): clones `t.text` (text Clone copies all templates of the text namespace
  but **shares the `*parse.Tree` pointers**) into a *new* html namespace containing only `t` itself.
  Used for baseof application (§3.3).

### 2.3 `common/hreflect/helpers.go`

* `IsTruthfulValue(val)` (l.96-130): `val = indirectInterface(val)`; invalid → false; **if the type
  implements `types.Zeroer` (`IsZero() bool`) → `!IsZero()`** (called even on nil pointer receivers);
  else by kind: Array/Map/Slice/String → `Len() > 0`; Bool; Complex ≠ 0; Chan/Func/Ptr/Interface →
  `!IsNil()`; Int/Uint/Float ≠ 0; Struct → true; other → false.
  Zeroer types that reach seeksnack templates: `time.Time` (Date/Lastmod/PublishDate/ExpiryDate and
  front-matter dates), `maps.Params` (`IsZero`: empty, or only key `_merge`; common/maps/params.go:63),
  `*source.File` (`fi == nil`; `{{ with .File }}` in comments.html), `page.Summary`, `media.Type`,
  `output.Format`, `tableofcontents.Heading`.
* `GetMethodByName` (l.141-171): cached `Type().MethodByName` — exact, case-sensitive.
* `IsContextType(tp)` (l.283-295): true for `context.Context` interface, the concrete ctx type, or any
  type implementing it.

### 2.4 `tpl/tplimpl/template_funcs.go` — `templateExecHelper`

* `GetFunc` (l.43-58): look up `t.funcs[name]` (see §4 for how this map is built). If the func's first
  parameter is a context type, return `reflect.ValueOf(ctx)` as `firstArg`.
* `GetMapValue` (l.70-84): **if receiver is `maps.Params`: `params[strings.ToLower(key)]`** (keys in
  Params are stored lower-case); missing → invalid. A present key whose value is `nil` →
  `reflect.ValueOf(nil)` = invalid (so YAML `null` behaves exactly like a missing key). Other maps:
  `receiver.MapIndex(key)` (exact).
* `GetMethod` (l.88-114): special case — if `strings.EqualFold(name, "mainsections")` and the receiver
  *is* the site's params map (pointer equality) → call `site.MainSections` instead. Then
  `hreflect.GetMethodByName(receiver, name)`; inject ctx if the first param is a context type.
* `OnCalled`/`Init`/`trackDependencies` — dependency tracking for server/watch mode only. Not needed
  for a one-shot build.

---------------------------------------------------------------------------------------------------

## 3. How neohugo builds and executes templates (tplimpl)

### 3.1 Namespaces and prototypes (templates.go:317-355)

`newTemplateNamespace(funcs)` creates `parseHTML = htmltemplate.New("").Funcs(funcs)`,
`parseText = texttemplate.New("").Funcs(funcs)`, `standaloneText = texttemplate.New("").Funcs(funcs)`.
`funcs` is the Hugo func map (used at *parse* time only for identifier validation; execution uses the
helper's `funcs` map). There is **one shared html namespace** and **one shared text namespace** for all
partials, shortcodes, render hooks, embedded templates and layouts that don't need a baseof.
`standaloneText` is used by `TextParse` (resources.ExecuteAsTemplate, templatestore.go:756).

### 3.2 Store construction order (`NewStore`, templatestore.go:112-164)

1. `init()` — legacy mapping tables.
2. `insertTemplates(nil,false)` (l.1240) — walk `layouts` FS; convert legacy paths
   (`/_default/`, `/partials/` → `/_partials/`, `/shortcodes/` → `/_shortcodes/`,
   `x-baseof.html` → `baseof.x.html`); compute `(key, category, descriptor)` via
   `toKeyCategoryAndDescriptor` (l.1684); insert into `treeMain` (radix tree keyed by dir) or
   `treeShortcodes`; read content with `removeLeadingBOM` and set `noBaseOf = !needsBaseTemplate(content)`
   (templates.go:19-40, 261-300: the first non-comment, non-whitespace action must match
   `^{{-?\s*define`; comments recognised are `{{/*`, `{{- /*`, `*/}}`, `*/ -}}`).
   `CategoryPartial`/shortcode/markup and anything with category > layout are `noBaseOf` automatically.
3. `insertEmbedded()` (l.1024-1098) — the embedded FS (`tpl/tplimpl/embedded/templates/**`), CRLF→LF,
   `ti.subCategory = SubCategoryEmbedded`, `noBaseOf=true`. `_markup/render-table.html` is inserted once
   per output format as `_markup/render-table.<of>.<suffix>` (l.1073-1082). Alias
   `_shortcodes/twitter.html` → `tweet.html`.
4. `parseTemplates(false)` (l.1531-1596):
   * pass 1: for every `treeMain` template with `noBaseOf` → `doParseTemplate` (templates.go:56): parse
     into `parseText` (if `D.IsPlainText`) or `parseHTML` under name `PathNoLeadingSlash()`
     (e.g. `_partials/head.html`; a clash appends `-<counter>`). Embedded partials also get an alias
     `_internal/<name>`; embedded shortcodes `_internal/shortcodes/<name>`; partials coming from a
     `partials/` dir also get the alias without leading underscore (issue #13599).
   * pass 2: for templates needing a baseof → `FindAllBaseTemplateCandidates` and `applyBaseTemplate`
     for each candidate (see 3.3). If no base candidate exists → treat as `noBaseOf` and parse directly.
   * pass 3: parse all shortcodes into the shared namespaces.
5. `extractInlinePartials(false)` (l.953-991): every template in any namespace (including baseof clones)
   whose name starts with `partials/` or `_partials/` becomes a partial `TemplInfo`
   (`_partials/<name>` + `.html` if no extension, `SubCategoryInline`). seeksnack defines four:
   `partials/marketing/jsonld/breadcrumb`, `partials/rating/rating-review`,
   `partials/comment/rating-review`, `partials/comment/rating-comment`.
6. `transformTemplates()` (l.1783-1824) → `applyTemplateTransformers` (3.5).
7. `createPrototypes(true)` — clones (for rebuilds; irrelevant for a one-shot build).
8. `prepareTemplates()` (l.1599-1609) — `Prepare()` (= html escaping) of every non-baseof template.

### 3.3 baseof + block assembly (`applyBaseTemplate`, templates.go:127-182)

For overlay O (e.g. `_default/single.html`) and base B (`_default/baseof.html`):

```
tt := htmltemplate.Must(parseHTML.CloneShallow()).New(O.PathNoLeadingSlash())   // text: parseText.Clone()
tt.Parse(B.content)     // B's top-level body becomes the body of template "<O name>"; its {{block "x"}}
                        // create named templates "x" with their default bodies
tt.Parse(O.content)     // O's {{define "x"}} replace B's blocks; O's top-level text is normally
                        // whitespace only -> "associate" keeps B's body
```
`text/template.associate` (template.go, see §5.1): a new tree replaces an existing one **unless the new
tree is empty** (`parse.IsEmptyTree`: only whitespace text, comments, or nothing — `bytes.TrimSpace`)
and the old one is non-nil. So: overlay top-level whitespace never replaces the baseof body; an empty
`{{ define "head" }}{{ end }}` in the overlay does *not* override a non-empty base block. Each
(overlay, base) pair gets its own namespace, so block names (`head`, `title`, `content`, `script`,
`afterbody`) never collide between layouts. Result stored in `O.baseVariants` keyed by base key/descriptor;
the executable `TemplInfo` has `base = B` and `noBaseOf = true`.

seeksnack: every page layout (`index.html`, `_default/{single,list,simple}.html`, `taxonomy/list.html`,
`term/term.html`, `posts/*.html`, `404.html`) starts with `{{ define … }}` and uses
`_default/baseof.html` which contains `{{ block "head" . }}{{ end }}`, `{{ block "title" . }}…{{ end }}`
(inside `<title>` → RCDATA, producing the derived template `title$htmltemplate_stateRCDATA_elementTitle`),
`{{ block "afterbody" . }}`, `{{ block "content" . }}`, `{{ block "script" . }}`.

### 3.4 Execution entry point (`TemplateStore.ExecuteWithContext`, templatestore.go:487-524)

```
parent := tpl.Context.CurrentTemplate.Get(ctx); level := parent.Level+1 (or 0)
ctx = tpl.Context.CurrentTemplate.Set(ctx, &CurrentTemplateInfo{Parent, Level, ti})
if level > 999 -> error "maximum template call stack size exceeded in %q"
t.storeSite.executer.ExecuteWithContext(ctx, ti, wr, data)   // ti.Prepare() -> escape (html) -> walk
errors wrapped by addFileContext (file/line context; irrelevant for output)
```
Callers: page rendering (`hugolib/site.go:1535 renderForTemplate`), render hooks
(`hookRendererTemplate`, hugolib/site.go:1503-1525 — heading, link, image, table, codeblock …),
shortcodes (`hugolib/shortcode.go:789`), aliases (`hugolib/alias.go:80`), `partial`
(`tpl/partials/partials.go:174`), `resources.ExecuteAsTemplate`
(`resources/resource_transformers/templates/execute_as_template.go:65`, text/template via `TextParse`),
deferred templates (`hugolib/hugo_sites_build.go:513`).
`{{ template "name" . }}` does **not** go through this function; it is a plain `walkTemplate`
(new variable scope `{$: dot}`, depth+1, max depth 100000 — exec.go:499-517).

### 3.5 AST transformations (`templatetransform.go`)

`applyTemplateTransformers` (l.67-91) walks the tree (following `{{template}}` calls once):
* `collectReturnNode` (l.333-352) — only for `CategoryPartial`: the first command whose first arg is the
  identifier `return` with ≥ 2 args is removed from the pipeline and remembered. If found,
  `ParseInfo.HasReturn = true` and the root is rewritten to (l.105, 131-143):
  `{{ $_hugo_dot := $ }}{{ $ := .Arg }}{{ range (slice .Arg) }}<original nodes>{{ $_hugo_dot.Set (<return pipeline>) }}{{ end }}`.
  The partial is then executed with data `&contextWrapper{Arg: data}` and output discarded; the
  result is `contextWrapper.Result` (partials.go:152-192). Note `range (slice .Arg)` makes a falsy
  `.Arg` still iterate once (slice of one element).
* `collectConfig` (l.268-305) — shortcodes: leading `{{ $_hugo_config := `{…}` }}`.
* `collectInner` (l.309-331) — shortcodes: sets `IsInner` if any command arg is a Field/Variable node
  containing identifier `Inner` or `InnerDeindent`.
* `handleDefer` (l.196-251) — `{{ with (templates.Defer …) }}` → replaced by `doDefer`.
seeksnack uses none of these in its own templates (grep: no `return`, no `.Inner`, no
`templates.Defer`), but the embedded `_partials/_funcs/get-page-images.html` uses `return`.

### 3.6 Partials (`tpl/partials/partials.go`)

`partial NAME [DATA]` → `Namespace.Include(ctx, name, contextList...)` (l.116): `LookupPartial(name)`
(templatestore.go:586: parses `name` as a layouts path of type partial; no output format/media type ⇒
**assume HTML**), `data = contextList[0]` or nil, then `doInclude` (l.152-192):
* return partial → value returned by `return`.
* else output buffered; result is `string` if the partial is a text/template, otherwise
  **`template.HTML(output)`** (so it is inserted unescaped in HTML text, stripped of tags in attributes,
  passed through `safeJS` in seeksnack's JSON-LD).
Evidence (probe): `{{ printf "%T" (partial "p1.html" .) }}` → `template.HTML`,
`{{ printf "%T" (partial "ret.html" .) }}` → `map[string]interface {}`.
`partialCached` caches by name+variants (not used by seeksnack). A partial name prefixed with
`partials/` only warns.

### 3.7 Output format → engine

`output/outputFormat.go`: `IsPlainText: true` for calendar, css, csv, markdown, json,
webappmanifest, robots, gotmpl. **Not** plain text: html, amp, alias, rss, sitemap, sitemapindex, 404.
Template descriptor `IsPlainText` selects `parseText` vs `parseHTML` (templates.go:69-120).
seeksnack outputs: `home = [HTML, JSON, RSS]`, `section/taxonomy/term = [HTML, RSS]`, `page = [HTML]`,
plus robots.txt (text), sitemap (html) and sitemapindex (html, embedded), aliases (html, embedded).

### 3.8 Templates executed by seeksnack

User: all of `layouts/**` (58 files, 4,438 lines). Embedded (verified in golden):
`alias.html` (1,482 alias pages, e.g. `biscuit/page/1/index.html`), `sitemapindex.xml` (root
`sitemap.xml`), `_shortcodes/ref.html` (`{{< ref "terms.md" >}}` ×6), `_markup/render-link.html`
(markdown links such as `[{{< ref "terms.md" >}}]({{< ref "terms.md" >}})`; for multilingual
single-host sites `useEmbedded: auto` becomes `fallback` (config/allconfig/allconfig.go:1121-1127) and
seeksnack has no own link hook, so the embedded hook is selected — for absolute URLs its output equals
plain goldmark's), `_markup/render-table.html` (62 content files contain
tables; golden `<table>\n  <thead>\n      <tr>\n          <th>` layout comes from this template's
trim markers). Their escaped forms are in `escdump/dump-embedded.txt`.

---------------------------------------------------------------------------------------------------

## 4. Function binding

### 4.1 Building the func map

* `tpl/tplimplinit/tplimplinit.go:60 CreateFuncMap`: for each registered namespace
  (`internal.TemplateFuncsNamespaceRegistry`, populated by each `tpl/*/init.go`):
  `funcMap[ns.Name] = ns.Context` (signature `func(ctx context.Context, v ...any) (any, error)`, returns
  the namespace struct), and for each method mapping `funcMap[alias] = method` for every alias. Duplicate
  names panic.
* `configureSiteStorage` (templatestore.go:2050-2088) builds the execution map `funcsv`:
  1. all Hugo funcs (namespaces + aliases),
  2. html/template escaper funcs (`htmltemplate.GoFuncs`) **only if not already present**,
  3. text/template builtins (`texttemplate.GoFuncs`) **only if not already present**.

Therefore Hugo overrides these Go builtins: `eq, ne, lt, le, gt, ge` (compare), `index, slice`
(collections — `slice` *builds* a list, it does not slice: `common/collections/slice.go:29` returns `[]T` when all args share type T (`slice 1 2 3` → `[]int`, `slice "a" "b"` → `[]string`), the first arg's `Slicer` result for Pages/Resources, else `[]interface{}`; no args → empty `[]interface{}`), `print, printf, println` (fmt —
same semantics as Go's `fmt.Sprint*`, tpl/fmt/fmt.go), and the **namespace `js` shadows builtin `js`**
(probe: `{{ .Title | js }}` printed the namespace struct `{0x… 0x… …}`). Go builtins still used:
`and, or, not, len, html, urlquery, call`.

Aliases defined (from `tpl/*/init.go`): absLangURL absURL add after anchorize append apply babel
base64Decode base64Encode chomp complement cond countrunes countwords dateFormat default delimit dict div
doDefer duration emojify eq errorf erroridf fileExists findRE findRESubmatch fingerprint first float ge
getCSV getenv getJSON group gt hasPrefix hasSuffix highlight hmac htmlEscape htmlUnescape humanize
i18n/T imageConfig in index int intersect isSet/isset jsonify keyVals last le lower lt markdownify md5
merge minify mod modBool mul ne newScratch now partial partialCached plainify pluralize pow print printf
println querify readDir readFile ref relLangURL relref relURL replace replaceRE return safeCSS safeHTML
safeHTMLAttr safeJS safeJSStr safeURL seq sha1 sha256 shuffle singularize slice slicestr sort split
string sub substr symdiff title trim truncate try union uniq unmarshal upper urldecode urlencode urlize
warnf warnidf where. Namespaces: cast collections compare crypto css data debug diagrams encoding fmt
hash hugo images inflect js lang math openapi3 os page partials path reflect resources safe site strings
templates time transform urls.

### 4.2 Calling conventions

* `name args…` → `evalFunction` → helper `GetFunc`; ctx injected when the first param is a context.
* `ns.Method args…` (e.g. `strings.ToLower .Title`, `resources.Get "x"`) parses as a **ChainNode**
  (IdentifierNode `ns` + field `Method`): the identifier is called with no args (returns the namespace
  struct), then `evalField` finds the method (ctx injected if needed).
* pipelines: the previous command's value is appended as the **last** argument (`final`).
* argument evaluation (`evalArg`, exec.go:935): literals are converted to the parameter type
  (`evalBool/evalString/evalInteger/evalFloat…`); to `any` params, number literals become
  **ideal constants** (`idealConstant`, exec.go:594-619): char literal → int (e.g. `'a'`→97);
  text containing `.eEpP` (and not hex/rune) → float64; else int (overflow → error). So `1.0` → float64 1,
  `1e3` → float64, `0x10` → int 16, `-0.0` → float64 −0.
* A command that is not a function but receives args → error `can't give argument to non-function`.
* Return values: 1 value, or (value, error); a non-nil error aborts execution
  (`error calling NAME: …`). Panics in funcs are recovered into errors (`safeCall`, funcs.go).

### 4.3 Builtins semantics (funcs.go) that remain reachable

| func | behaviour |
|---|---|
| `and a b…` / `or a b…` | short-circuit, return operand value (see 2.1). Probe: `and 1 0 2`→`0`, `or 0 "" "x"`→`x`, `or 0 ""`→`""` |
| `not x` | `!isTrue(x)` (Hugo truthiness) |
| `len x` | `indirect`; Array/Chan/Map/Slice/String → `Len()` (**bytes** for strings: `len "héllo"`=6); nil ptr → error; else error `len of type %s` |
| `html args…` | `HTMLEscapeString(evalArgs(args))`: NUL → U+FFFD (the character), `"`→`&#34;`, `'`→`&#39;`, `&`→`&amp;`, `<`→`&lt;`, `>`→`&gt;` (note: **no** `+` escaping, unlike html/template's escaper) |
| `urlquery args…` | `url.QueryEscape` (uppercase hex, space→`+`): probe `Home+%22Q%22+%26+%27A%27+%3Cb%3E` |
| `call f args…` | calls a func value |
`evalArgs` (funcs.go end): single string arg fast path; else each arg → `printableValue` then
`fmt.Sprint(args...)`.

---------------------------------------------------------------------------------------------------

## 5. text/template execution semantics to reproduce (exec.go, go1.24)

### 5.1 Templates, names, association (template.go)

* A `Template` = name + `*parse.Tree` + shared `common{tmpl map[string]*Template, parseFuncs, execFuncs,
  option}`. `New(name)` shares `common`. `Lookup(name)`.
* `Parse(text)` → `parse.Parse(name, text, "", "", parseFuncs, builtins())` returns a map of trees (the
  top-level tree plus one per `{{define}}`/`{{block}}`); each goes through `AddParseTree` →
  `associate`: replace unless new tree `IsEmptyTree` and an old tree exists.
* Parse mode is 0 (comments dropped; function names must exist at parse time).
* `Clone()` copies the map (templates share trees). html `Clone` deep-copies trees (`x.Tree.Copy()`),
  html `CloneShallow` does not.

### 5.2 Lexer/parser rules that affect output (parse package, go1.24)

* Delimiters `{{ }}` only. Left trim `{{- ` (hyphen followed by space/tab/CR/LF) trims all trailing
  `" \t\r\n"` of the preceding text; right trim ` -}}` trims leading `" \t\r\n"` of the following text
  (lex.go `rightTrimLength`/`leftTrimLength`, `spaceChars = " \t\r\n"`). `{{-3}}` is a number.
* Comments: `{{/* … */}}` or `{{- /* … */ -}}`; the comment must start immediately after the delimiter
  (+ optional trim marker) and end with `*/}}` or `*/ -}}`. Comments produce no node (mode 0).
* Newlines are allowed inside actions (multi-line pipelines in seeksnack `single.html:178-188`).
* Raw strings in backquotes may span lines (`header.html:166-182`).
* Keywords: `if else end range with define block template break continue nil true false`;
  `{{ else if … }}`, `{{ else with … }}` (go1.23) chain into nested nodes.
* Variables: `$x := p` declares (scoped to the enclosing if/with/range/define body — popped at `{{end}}`),
  `$x = p` assigns (must exist), `range $i, $e := …` / `range $e := …` (single var gets the
  **element**), `$` = data passed to the template. Undefined variable → parse error.
* `break`/`continue` only inside range.
* Numbers: Go syntax incl. `_`, `0x`, `0o`, `0b`, legacy octal `017`, floats, `i` imaginary, char
  constants (`newNumber`, node.go:633-715). Probe `T-NUM`: `{{ 017 }}`→15, `{{ 0b101 }}`→5, `{{ 1_000 }}`→1000.

### 5.3 walk / eval (exec.go)

* `walk` (l.262): Action → `evalPipeline`; print only if the pipeline declares no variables
  (`printValue`). Text → write bytes verbatim. If/With → `walkIfOrWith` (l.300): `truth(indirectInterface(val))`;
  `with` rebinds dot to the pipeline value; variables declared in the pipeline are popped after the node.
* `walkRange` (l.353-497): evaluate & `indirect`; kinds:
  * Int/Uint (go1.22): iterates `0..n-1` (only one variable allowed; element = value);
  * Array/Slice: index `int`, element;
  * **Map: keys sorted by `fmtsort`** (strings: byte-wise; ints numeric; floats numeric; bool false<true;
    interface keys by type then value);
  * Chan; `iter.Seq`/`iter.Seq2` funcs (go1.23);
  * Invalid (nil) → treated as empty; empty → `{{else}}` branch; other kinds → error
    `range can't iterate over %v` (probe: ranging over a string errors).
  * `break`/`continue` via panics `walkBreak`/`walkContinue`.
* `evalPipeline` (l.526): evaluates commands left→right, passing the previous value as `final`; after
  each command, **if the value's static type is `interface{}` → `Elem()`** (a nil interface becomes an
  invalid Value). Then declares/assigns variables.
* `evalCommand` (l.555): Field/Chain/Identifier/Pipe/Variable/Bool/Dot/Nil/Number/String; `nil` as a
  command → error `nil is not a command`.
* `evalFieldChain` (l.661): `.A.B.C` → evalField per identifier; only the last gets args/final.
* `evalChainNode` (l.634): `(pipe).Field` / `ident.Field`.
* `validateType` (l.893): invalid value to a nilable param → zero of that type (typed nil); to
  `interface{}` → nil interface; non-assignable → one level of deref/addr or error
  `wrong type for value; expected %s; got %s`.
* `printValue` (l.1102) / `printableValue` (l.1116): pointer → `indirect` (nil pointer stays and prints
  `<nil>`); **invalid → `"<no value>"`**; if the (addressable) type or its pointer implements `error` or
  `fmt.Stringer` use that; Chan/Func → error `can't print %s of type %s`; then `fmt.Fprint(w, v)`.
* Errors are panics `ExecError` formatted `template: %s: executing %q at <%s>: %s` with node context
  (`ErrorContext`).

### 5.4 Evidence of nil / missing semantics (probe site)

| expression | html/template | text/template |
|---|---|---|
| `{{ .Params.missing }}` | `` (empty) | `<no value>` |
| `{{ .Params.ynull }}` (YAML `null`) | `` | `<no value>` |
| `{{ .Params.ymap.missing }}`, `{{ .Params.missing.deeper }}`, `{{ (dict "a" 1).b.c }}` | `` | `<no value>` |
| `{{ print nil }}` | `&lt;nil&gt;` | `<nil>` |
| `{{ printf "%s" .Params.missing }}` | `%!s(&lt;nil&gt;)` | `%!s(<nil>)` |
| `{{ printf "%v" nil }}` | — | `<nil>` |
| `{{ with .Params.missing }}{{ .Foo.Bar }}{{ end }}` | `` (branch not taken) | |

Chained field access on an invalid value is silently invalid (exec: `if !receiver.IsValid() { return zero }`),
e.g. seeksnack `partials/single/whenseen.html:1` `{{ $dateFormat := .Date.Format "2006-01-02" }}` on a
`dict` (no "Date" key) yields invalid without error.

---------------------------------------------------------------------------------------------------

## 6. Method / field / map resolution (Go reflection → what the Rust object model must emulate)

Order inside `evalField` (Hugo version, hugo_template.go:156-259) for `.Name` on receiver R:

1. R invalid → return invalid (no error).
2. `indirect` through pointers and interfaces (nil interface → error; nil pointer continues).
3. **Methods first** (exported, exact case; methods of `*T` and `T` both visible when addressable).
   Context injected if the method's first param is `context.Context` (page `.Content`, `.Plain`,
   `.Summary`, `.RenderString`, `.ReadingTime`, … — full list in §9).
4. Struct → exported field by exact name (promoted fields from embedded structs included).
5. Map with string-compatible key:
   * `maps.Params` → `strings.ToLower(name)` lookup (Params keys are lower-cased at construction).
     Probe: `.Params.YMAP.B`=2, `site.Params.MiXeD_cAsE`="mc".
   * any other map (e.g. `map[string]any` from `dict`, `transform.Unmarshal`, `page.Data`) → exact key.
     Probe: `(dict "Key" 1).key` → `<no value>`, `.Key` → 1.
6. Otherwise error.

Consequences seen in seeksnack:
* `.Site.Params.Social.Facebook.Enable`, `.Site.Params.Comment.Apipro`, `.Params.Author`,
  `.Params.Brands`, `.Params.Low_price`: case-insensitive Params lookups.
* `.Data.Pages` on `page.Data` (a `map[string]any` type with method `Pages()`): **method wins**
  (resources/page/page_data.go:27). `.Data.Singular`: exact key `"Singular"` (hugolib/page__data.go:45).
* `maps.Params` methods shadow keys with the same exact name: `GetNested`, `IsZero`,
  `GetMergeStrategy`, `DeleteMergeStrategy`, `SetMergeStrategy` (common/maps/params.go).
* Slice types with methods (`page.Pages` `.Reverse`/`.Related`/…, `resource.Resources` `.Get`/`.GetMatch`)
  must remain both rangeable/indexable and method-bearing, and collection funcs (`sort`, `where`,
  `first`, `after`) preserve the concrete slice type (so `(sort … "Date").Reverse` works).
* Stringer types print via `String()`: `*langs.Language` → `Lang` (header.html `id="{{ $translation.Language }}"`
  → `id="en"`), `media.Type` → `"application/rss+xml"`, `time.Time` → Go time string,
  `hstring.HTML` → the string.

---------------------------------------------------------------------------------------------------

## 7. html/template contextual autoescaping (go1.24 fork)

### 7.1 When and how escaping runs

* `Template.escape()` (template.go:97 in the fork) → `escapeTemplate(tmpl, root, name)` (escape.go:24):
  `c, _ := esc.escapeTree(context{}, node, name, 0)` — start context is the zero context
  (`stateText`, no delim, no element). If the result has an error, or ends in a state other than
  `stateText`, the template is marked unusable (`ends in a non-text context`). Otherwise `esc.commit()`
  applies all edits **in place** to the parse trees and marks `escapeErr = escapeOK`.
* Each template is escaped once (first `Prepare`). Every Hugo partial/shortcode/hook/layout is
  escaped starting in `stateText`, independently of where it is later called from via `partial`
  (the `partial` func returns `template.HTML`, so callers treat its output as trusted HTML).
* `{{ template "x" }}` calls are escaped *in the caller's context*: `escapeTree` mangles the name with
  the context (`context.mangle`, context.go:57: `x$htmltemplate_<state>[_<delim>][_<urlPart>][_<jsCtx if !=Regexp>][_<attr>][_<element>]`),
  and if the context is not `stateText` a **derived copy** of the tree is escaped under that name and the
  `TemplateNode` is renamed (`editTemplateNode`). seeksnack: baseof's `{{ block "title" . }}` inside
  `<title>` → `title$htmltemplate_stateRCDATA_elementTitle` (seen in `escdump/dump.txt` for every layout).
  Recursive templates (`breadcrumbnav` calls itself) use the fixed-point logic in
  `computeOutCtx`/`escapeTemplateBody` (escape.go:664-707).
* Output context of a template body is computed; branches (`if/with/range` + `else`) must end in the
  same context (`join`, escape.go:457-505: equal → ok; differ only in urlPart → `urlPartUnknown`;
  only in jsCtx → `jsCtxUnknown`; nudged contexts may join; else `ErrBranchEnd`). `range` bodies are
  escaped twice (loop re-entry check). `{{break}}`/`{{continue}}` contexts are joined (`joinRange`).

### 7.2 Context structure (context.go)

`context{state, delim, urlPart, jsCtx, jsBraceDepth, attr, element, n, err}`.
States (context.go:88-162): Text, Tag, AttrName, AfterName, BeforeValue, HTMLCmt, RCDATA, Attr, URL,
Srcset, JS, JSDqStr, JSSqStr, JSTmplLit, JSRegexp, JSBlockCmt, JSLineCmt, JSHTMLOpenCmt,
JSHTMLCloseCmt, CSS, CSSDqStr, CSSSqStr, CSSDqURL, CSSSqURL, CSSURL, CSSBlockCmt, CSSLineCmt, Error,
Dead. `delim`: None, DoubleQuote, SingleQuote, SpaceOrTagEnd. `urlPart`: None, PreQuery, QueryOrFrag,
Unknown. `jsCtx`: Regexp, DivOp, Unknown. `element`: None, Script, Style, Textarea, Title.
`attr`: None, Script, ScriptType, Style, URL, Srcset.
`isComment` = HTMLCmt, JSBlockCmt, JSLineCmt, JSHTMLOpenCmt, JSHTMLCloseCmt, CSSBlockCmt, CSSLineCmt.
`isInScriptLiteral` = JSDqStr, JSSqStr, JSTmplLit, JSRegexp. `isInTag` = Tag, AttrName, AfterName,
BeforeValue, Attr.

### 7.3 Transitions over template text (transition.go) — must be ported exactly

* `tText` (l.53): find `<`; `<!--` → HTMLCmt; `<` + optional `/` + tag name (`eatTagName`: ASCII
  letter then alnum, allowing single `-`/`:` between alnums) → `stateTag` with element from
  `elementNameMap` {script, style, textarea, title} (end tags → elementNone). Anything else stays text.
* `tTag` (l.91): skip whitespace; `>` → `elementContentType[element]` (None→Text, Script→JS,
  Style→CSS, Textarea/Title→RCDATA); attribute name (`eatAttrName`: error on `'"<`); lower-cased;
  `type` on `<script>` → `attrScriptType`; else by `attrType(name)` (§7.4) → URL/CSS/JS/Srcset attr.
* `tAttrName`, `tAfterName` (`=` → BeforeValue, else back to Tag), `tBeforeValue` (quote detection;
  sets `state = attrStartStates[attr]`: None→Attr, Script→JS, ScriptType→Attr, Style→CSS, URL→URL,
  Srcset→Srcset).
* `contextAfterText` (escape.go:822): inside an attribute value, the value up to the delimiter is
  **HTML-unescaped** (`html.UnescapeString`) before running the value transitions; unquoted values
  containing `"'<=\`` → error. On leaving the value: if in `<script type=X>` and `!isJSType(X)` →
  element becomes None (so `<script type="x-tmpl-mustache">` / `type="text/template"` bodies are
  treated as **HTML text**; seeksnack `header.html:165` and probe `NONJS`).
  `isJSType` (js.go:455): strip params after `;`, lowercase, trim; true for application/ecmascript,
  application/javascript, application/json, application/ld+json, application/x-ecmascript,
  application/x-javascript, module, text/ecmascript, text/javascript, text/javascript1.0…1.5,
  text/jscript, text/livescript, text/x-ecmascript, text/x-javascript. (**Not** `""` in go1.24.)
* `tSpecialTagEnd` (l.217): in Script/Style/Textarea/Title bodies, find `</tag` (case-insensitive,
  followed by one of `> \t\n\f/`); inside JS string/regexp/comment states `</script` is ignored.
* `tURL` (l.262): `#` or `?` → QueryOrFrag; non-space → PreQuery (if None).
* `tJS` (l.274): finds `"`, `'`, `` ` ``, `/`, `{`, `}`, `<`, `-`, `#`; updates `jsCtx` with `nextJSCtx`
  (js.go:35: regexp vs div-op heuristic incl. keyword list `break case continue delete do else finally in
  instanceof return throw try typeof void`); `//` → JSLineCmt, `/*` → JSBlockCmt, `/` → regexp or
  div-op, `<!--` → JSHTMLOpenCmt, `-->` → JSHTMLCloseCmt, `#!` → line comment; template-literal
  brace depth tracking.
* `tJSDelimited` (l.387), `tJSTmpl` (l.352), `tBlockCmt`, `tLineCmt` (JS line terminators `\n`, `\r`, U+2028, U+2029;
  CSS `\n\f\r`), `tCSS` (l.500: `url(` → CSSURL/DqURL/SqURL, `//`, `/*`, quotes), `tCSSStr` (with
  `decodeCSS` escapes), `tHTMLCmt` (`-->`).

### 7.4 Attribute classification (`attrType`, attr.go:140, table attr.go:19-138)

`name` lower-cased; `data-` prefix stripped; `xmlns:*` → URL; `ns:name` → `name`; table lookup;
else prefix `on` → JS; else name contains `src`, `uri` or `url` → URL; else plain.
URL attrs in table: action archive background cite classid codebase data formaction href icon
longdesc manifest poster profile src usemap xmlns. CSS: style. Srcset: srcset. HTML: srcdoc.
Unsafe (escaped like plain): accept-charset async challenge charset **content** crossorigin defer
enctype form formenctype formmethod formnovalidate http-equiv keytype language method novalidate
pattern **rel** sandbox **type** **value**. (Unsafe only matters for `htmlNameFilter` of dynamic
attribute names.) seeksnack examples: `data-bs-target` → plain; `data-href` → URL (href);
`data-ad-client` → plain; `data-sitekey` → plain.

### 7.5 Which escapers get inserted (`escapeAction`, escape.go:170-266)

Actions that declare/assign variables are left alone. Otherwise `c = nudge(c)` (Tag→AttrName;
BeforeValue→unquoted value; AfterName→AttrName) and:

| context state | escaper(s) appended |
|---|---|
| URL / CSSDqStr / CSSSqStr / CSSDqURL / CSSSqURL / CSSURL, urlPart None | `_html_template_urlfilter` then (CSS strings: `_html_template_cssescaper`; else `_html_template_urlnormalizer`) |
| same, urlPart PreQuery | cssescaper (CSS strings) or urlnormalizer |
| same, urlPart QueryOrFrag | `_html_template_urlescaper` |
| same, urlPart Unknown | error (ambiguous URL context) |
| JS | `_html_template_jsvalescaper`; afterwards `jsCtx = DivOp` |
| JSDqStr / JSSqStr | `_html_template_jsstrescaper` |
| JSTmplLit | `_html_template_jstmpllitescaper` (fork: godebug gate removed) |
| JSRegexp | `_html_template_jsregexpescaper` |
| CSS | `_html_template_cssvaluefilter` |
| Text | `_html_template_htmlescaper` |
| RCDATA | `_html_template_rcdataescaper` |
| Attr | (only the delim escaper below) |
| AttrName / Tag | state := AttrName; `_html_template_htmlnamefilter` |
| Srcset | `_html_template_srcsetescaper` |
| any comment state | `_html_template_commentescaper` (→ `""`) |

then by delimiter: None → nothing; SpaceOrTagEnd → `_html_template_nospaceescaper`; quoted →
`_html_template_attrescaper`.

`ensurePipelineContains` (escape.go:271-333) merges with a trailing predefined escaper `html` or
`urlquery` (must be the last command, else error `predefined escaper %q disallowed in template`; `html`
in an unquoted attr is also disallowed): an inserted escaper equivalent to it (`equivEscapers`:
attrescaper/htmlescaper/rcdataescaper ≡ `html`; urlescaper/urlnormalizer ≡ `urlquery`) is replaced
by the predefined name; `{{ html a b }}` form becomes `{{ _eval_args_ a b | html }}`. Escapers already
present are not re-added; `appendCmd` (l.402) drops an escaper that is redundant after the previous one
(`redundantFuncs` l.378: after commentescaper → attr/html; after cssescaper, jsregexpescaper,
jsstrescaper, jstmpllitescaper → attrescaper; after urlescaper → urlnormalizer).
seeksnack evidence (`partials/single/socialshare.html:56`):
`href="mailto:?subject={{ .Title }}&amp;body={{ "Check out this site " | html }}{{- .Permalink }}"` →
`{{"Check out this site " | _html_template_urlescaper | html}}` → golden
`body=Check%20out%20this%20site%20https%3a%2f%2fseeksnack.com%2f…`.

### 7.6 Text-node rewriting (`escapeText`, escape.go:745-820)

While walking a text node through the transitions:
1. In Text/RCDATA, every `<` that does not start a transition (tag/comment) is replaced by `&lt;`,
   except when followed (case-insensitive) by `<!DOCTYPE`. Probe `TEXTLT`:
   `a < b and c > d & e <?xml x?> <!DOCTYPE y>` → `a &lt; b and c > d & e &lt;?xml x?> <!DOCTYPE y>`
   (this is why seeksnack emits `<?xml …?>` through `printf … | safeHTML`).
2. Comment content (delim None) is **removed**: HTML comments entirely; JS block comments replaced by
   `"\n"` if they contain `\n`, `\r`, U+2028 or U+2029, else `" "`; CSS block comments by `" "`; JS/CSS line
   comments and `<!--`/`-->` JS pseudo-comments are dropped up to (not including) the line terminator.
   The bytes before a comment start are kept (for `<!--` the 4-byte opener is excluded).
3. In JS string/template/regexp literal states, `(?i)<(script|/script|!--)` is rewritten to `\x3C$1`.
4. If nothing changed the node is untouched; changes are applied at commit.
Probe `JS2` shows the result: `<script>// line comment {{ .Title }}\n/* block\n comment */ var x = 1; <!-- htmlish\nvar y = …` →
`<script>\n\n var x = 1; \nvar y = …` (actions inside comments get `commentescaper` → nothing).

### 7.7 Escaper functions and exact tables

All escapers take `args ...any`. `stringify(args)` (content.go:68): if exactly one arg, after Hugo's
`indirect` (deref pointers, unwrap `PrintableValueProvider`), a `string` → plain; `template.CSS/HTML/
HTMLAttr/JS/JSStr/URL/Srcset` → that content type. Otherwise **untyped nil args are skipped**, others
go through `indirectToStringerOrError` and `fmt.Sprint(args...)` (plain).

`htmlReplacer(s, table, badRunes)` (html.go:144): replaces runes < len(table) with non-empty entries;
with `badRunes=false` also encodes U+FDD0–U+FDEF and U+FFF0–U+FFFF as `&#x%x;`.

| table | entries |
|---|---|
| htmlReplacementTable (html.go:55) | NUL→U+FFFD (the character), `"`→`&#34;`, `&`→`&amp;`, `'`→`&#39;`, `+`→`&#43;`, `<`→`&lt;`, `>`→`&gt;` |
| htmlNormReplacementTable (l.73) | same minus `&` |
| htmlNospaceReplacementTable (l.99) | `\0`→`&#xfffd;`, `\t`→`&#9;`, `\n`→`&#10;`, `\v`→`&#11;`, `\f`→`&#12;`, `\r`→`&#13;`, ` `→`&#32;`, `"`→`&#34;`, `&`→`&amp;`, `'`→`&#39;`, `+`→`&#43;`, `<`→`&lt;`, `=`→`&#61;`, `>`→`&gt;`, `` ` ``→`&#96;` |
| htmlNospaceNormReplacementTable (l.122) | same minus `&` |

| escaper | behaviour |
|---|---|
| `htmlEscaper` (html.go:45) | HTML content → unchanged; else htmlReplacementTable, badRunes=true |
| `attrEscaper` (l.27) | HTML content → `stripTags` then htmlNormReplacementTable; else htmlReplacementTable (badRunes=true) |
| `rcdataEscaper` (l.36) | HTML → htmlNorm table (no stripping); else htmlReplacementTable |
| `htmlNospaceEscaper` (l.15) | empty → `ZgotmplZ`; HTML → stripTags + nospaceNorm; else nospace table; badRunes=false |
| `htmlNameFilter` (l.233) | HTMLAttr content → as is; empty → `ZgotmplZ`; lower-case; if `attrType(s) != plain` → `ZgotmplZ`; only `[0-9a-z]` allowed else `ZgotmplZ` |
| `commentEscaper` (l.268) | `""` |
| `urlFilter` (url.go:34) | URL content → as is; if `s` has a `scheme:` (no `/` before `:`) other than http/https/mailto (case-insensitive) → `#ZgotmplZ` |
| `urlNormalizer` / `urlEscaper` (url.go:58-141) | `processURLOnto`: bytes `-._~` and ASCII alnum kept; `!#$&*+,/:;=?@[]` kept when normalizing, else `%xx`; `%` kept if normalizing and followed by 2 hex digits; everything else (incl. space, `"`, `'`, `(`, `)`, `<`, `>`, non-ASCII bytes) → `%` + **lowercase** 2-digit hex (`fmt.Fprintf("%%%02x")`). URL content forces normalize mode. |
| `srcsetFilterAndEscaper` (url.go:143) | Srcset → as is; URL → normalize + `,`→`%2c`; else split on `,`, each element: trim HTML spaces, URL = up to first space; if safe and metadata is only spaces/ASCII alnum → normalized URL + metadata, else `#ZgotmplZ` (probe `SRCSET`: `/a b.png 1x` → `#ZgotmplZ`) |
| `jsValEscaper` (js.go:153) | see 7.8 |
| `jsStrEscaper` (js.go:255) | JSStr content → `jsStrNormReplacementTable`; else `jsStrReplacementTable` |
| `jsTmplLitEscaper` (l.263) | `jsBqStrReplacementTable` |
| `jsRegexpEscaper` (l.272) | `jsRegexpReplacementTable`; empty → `(?:)` |
| `cssEscaper` (css.go:158) | `cssReplacementTable`; after a replacement other than `\\`, a space is written if at end of string or the next byte is hex or CSS space |
| `cssValueFilter` (css.go:223) | CSS content → as is; decodeCSS; any of `\0 " ' ( ) / ; @ [ \ ] \` { } < >` or `--` → `ZgotmplZ`; ident chars containing `expression`/`mozbinding` (case-insensitive) → `ZgotmplZ` |
| `_eval_args_` (escape.go:51) | `fmt.Sprint` with `indirectToStringerOrError` |

JS `replace(s, table)` (js.go:287): first `lowUnicodeReplacementTable` for runes < 0x20
(`\u0000`…`\u001f`, except TAB→`\t`, LF→`\n`, FF→`\f`, CR→`\r`, BS→`\u0008`, VT→`\u000b`),
then the table, then U+2028/U+2029 → `\u2028`/`\u2029` (6-byte escapes).

| JS table | entries (besides control chars; all replacements are literal escape text) |
|---|---|
| jsStrReplacementTable (js.go:334) | `"`→`\u0022` `` ` ``→`\u0060` `&`→`\u0026` `'`→`\u0027` `+`→`\u002b` `/`→`\/` `<`→`\u003c` `>`→`\u003e` `\`→`\\` |
| jsBqStrReplacementTable (l.356) | jsStr table + `$`→`\u0024` `{`→`\u007b` `}`→`\u007d` |
| jsStrNormReplacementTable (l.381) | jsStr table without the `\` entry |
| jsRegexpReplacementTable (l.400) | `"`→`\u0022` `$`→`\$` `&`→`\u0026` `'`→`\u0027` `(`→`\(` `)`→`\)` `*`→`\*` `+`→`\u002b` `-`→`\-` `.`→`\.` `/`→`\/` `<`→`\u003c` `>`→`\u003e` `?`→`\?` `[`→`\[` `\`→`\\` `]`→`\]` `^`→`\^` `{`→`\{` `\|`→`\|` `}`→`\}` |
| cssReplacementTable (css.go:189) | NUL→`\0` TAB→`\9` LF→`\a` FF→`\c` CR→`\d` `"`→`\22` `&`→`\26` `'`→`\27` `(`→`\28` `)`→`\29` `+`→`\2b` `/`→`\2f` `:`→`\3a` `;`→`\3b` `<`→`\3c` `>`→`\3e` `\`→`\\` `{`→`\7b` `}`→`\7d` |

`ZgotmplZ` (`filterFailsafe`, escape.go:135) appears when: urlFilter rejects a scheme (`#ZgotmplZ`),
srcset element rejected (`#ZgotmplZ`), cssValueFilter rejects, nospace escaper gets `""`, name filter
rejects. Golden seeksnack output contains **no** `ZgotmplZ`.

### 7.8 `jsValEscaper` (js.go:153-250) — JS expression context

1. One arg: `a = indirectToJSONMarshaler(arg)` (deref pointers until a `json.Marshaler`); `template.JS`
   → returned verbatim; `template.JSStr` → `"` + s + `"`; `json.Marshaler` → keep; else `fmt.Stringer`
   → `a = t.String()`. Several args: `fmt.Sprint` of them.
2. `b, err := json.Marshal(a)` (Go **encoding/json v1**, HTML escaping ON, see §8.6). On error the
   result is `" /* " + errText + " */null "` where in `errText` `<script`/`</script` (case-insensitive)
   become `\x3Cscript`/`\x3C/script`, `*/` becomes `* /` and `<!--` becomes `\x3C!--`.
3. Empty output → `" null "`.
4. If the first or last rune is a JS identifier char (`$`, `_`, ASCII alnum) → surround with one space
   on each side (numbers, `true`, `false`, `null`, negative numbers).
5. Raw U+2028/U+2029 characters in the JSON output → `\u2028`/`\u2029` (json already escapes them).
Probe `JS1`: `{{ .Title }}` → `"Home \"Q\" \u0026 'A' \u003cb\u003e"`; `[]string` → `["a","b","c"]`;
`maps.Params` → `{"a":1,"b":2,"c":[1,2]}`; int 5 → ` 5 `; −3 → ` -3 `; 4.5 → ` 4.5 `;
time → `"2020-09-06T15:46:26.955Z"`; nil/missing → ` null `; `true` → ` true `.
In an `onclick="…"` attribute the jsvalescaper output is then attr-escaped (`"` → `&#34;`), jsstrescaper
output is not (redundant) — probe `JS3`.

### 7.9 `stripTags` (html.go:181-230) and `tpl.StripHTML` (tpl/template.go:100-134)

`stripTags` runs the same transition functions over an HTML string (non-None elements treated as RCDATA
so script/style bodies are skipped), emitting only text-state bytes; attribute values skipped via
`delimEnds`; if the input was all text it is returned as-is. **It does not decode entities.**
`tpl.StripHTML` (used for `.Plain`, `plainify`, `countwords`, summaries, goldmark plain text):
if no `<`/`>` → unchanged; `strings.NewReplacer("\n"," ", "</p>",PH, "<br>",PH, "<br />",PH)` with
`PH="___hugonl_"`; `stripTags`; `PH`→`"\n"`; collapse runs of `unicode.IsSpace` runes to the first
one. seeksnack's `index.json` `"contents"` (363 KB) is produced this way (`$page.Plain`).

### 7.10 seeksnack escaping contexts (evidence)

From `escdump/dump.txt` (unique pipeline suffixes, count):
```
420 | _html_template_htmlescaper                         (text)
126 | _html_template_attrescaper                         (quoted plain attrs: alt, class, id, content, …)
125 | _html_template_urlfilter | _html_template_urlnormalizer | _html_template_attrescaper   (href/src start)
 94 | _html_template_jsstrescaper                        (JSON-LD "…{{ }}…", gtag('config','{{ . }}'))
 33 | _html_template_srcsetescaper | _html_template_attrescaper   (<source srcset=…>)
 32 | relLangURL | urlfilter | urlnormalizer | attrescaper
 19 | _html_template_urlescaper | _html_template_attrescaper      (after ? or #, share links)
 18 | _html_template_rcdataescaper                       (<title> in baseof/RSS)
 15 | _html_template_jsvalescaper                        (JSON-LD unquoted values)
  9 | safeHTML | htmlescaper      7 | humanize | htmlescaper      4 | safeHTML | attrescaper
  4 | jsvalescaper | attrescaper (modalImage onclick — shortcode unused by content)
  3 | urlnormalizer | attrescaper (URL after static prefix, e.g. youtube.com/embed/{{ . }})
  2 | safeJS | jsvalescaper       2 | markdownify | htmlescaper   2 | htmlnamefilter (<h{{ .Level }}>)
  2 | absLangURL | urlfilter | urlnormalizer | attrescaper
  1 | safeURL | urlescaper | attrescaper (href="#{{ .Anchor | safeURL }}" → normalize mode)
  1 | safeURL | attrescaper       1 | safeCSS | cssvaluefilter  1 | absURL | urldecode | htmlescaper
  1 | absLangURL | jsstrescaper    1 | _html_template_urlescaper | html
```
Golden evidence:
* `<meta name="description" content="Lay&#39;s was created …">` (attr), `<title>…Lay&#39;s…</title>`
  (RCDATA), `content="2020-12-30T07:40:22&#43;00:00"` (`safeHTML` value in attr → stripTags + norm
  table → `+`→`&#43;`), but `<lastmod>2023-09-25T14:45:00+00:00</lastmod>` in sitemap (HTML value in
  text → unchanged).
* JSON-LD (`partials/marketing/jsonLd.html`): `"@id": "https:\/\/seeksnack.com\/…"`,
  `"name": "Lay\u0027s Rock …"`, `"datePublished": "2020-09-06 15:46:26.955 \u002b0000 UTC"`,
  `"contentUrl": "https://seeksnack.com/images/favicon/mstile-70x70.png"` (jsval → JSON, no `\/`),
  `"owns": "Pepsi-Cola (Thai) Trading Co.,Ltd."`, `"keywords": ["Lay's Rock", "Seafood", …]`,
  `"owns": "TAOKAENOI FOOD \u0026 MARKETING PUBLIC COMPANY LIMITED"`,
  `"keywords" : "[Lays Thailand Snacks …]"` (a `[]any` printed with fmt then jsstr-escaped).
  The inline partial `partials/marketing/jsonld/breadcrumb` is escaped as HTML text (its `<!-- … -->`
  comments vanish, `Lay&#39;s` inside), then piped through `safeJS` → inserted verbatim.
* RSS (`_default/rss.xml`, html/template): `<title>Alice&#39;s Coconut Mochi</title>`;
  `{{ .Description | html }}` → text/template `html` builtin (`&#34;` for `"`, no `+` escaping).
* `<script id="search-result-template" type="x-tmpl-mustache">{{- safeHTML `…` }}</script>` → body is
  HTML text (non-JS type); the mustache markup passes through `htmlescaper` unchanged (HTML content).

---------------------------------------------------------------------------------------------------

## 8. Value model and printing

### 8.1 Go types that flow through seeksnack templates

(`%T` evidence from the probe site; seeksnack front matter is YAML, config is TOML, data files JSON.)

| source | Go type |
|---|---|
| YAML int / float / bool / string | `int` / `float64` / `bool` / `string` (`ydate: 2021-01-02` stays **string**) |
| YAML string list | **`[]string`** (Hugo normalises), mixed list → `[]interface {}` |
| YAML map (front matter) | `maps.Params` (lower-cased keys; nested maps also Params) |
| YAML `null` | nil (behaves as missing) |
| TOML int / float / array | `int64` / `float64` / `[]interface {}`; TOML tables → `maps.Params` |
| JSON data files / `transform.Unmarshal` | `map[string]interface {}`, `[]interface {}`, `float64`, `string`, `bool` |
| page dates (`.Date` etc.) | `time.Time` (UTC in seeksnack: front matter `…Z`) |
| `dict` | `map[string]interface {}` (case-sensitive keys) |
| `slice` | `[]T` if all args have the same type T, Slicer type (`page.Pages`, `resource.Resources`) for pages/resources, else `[]interface {}` |
| `seq` | `[]int` |
| `add/sub/mul` on ints | `int64` (probe `sub 3 1 | printf "%T"` → int64); with a float → `float64` |
| safe* funcs, partial output, `.Content`, `markdownify`, `jsonify` | `template.HTML/CSS/JS/JSStr/URL/HTMLAttr` |
| render hook `.Text` | `hstring.HTML` (Stringer + PrintableValue → template.HTML) |
| pages | `page.Page` (`*hugolib.pageState`), `page.Pages` (slice type with methods), `*page.Pager` |
| other objects | `page.Site`, `*langs.Language`, `langs.Languages`, `resource.Resource` (images, JS/CSS results), `resource.Resources`, `*maps.Scratch`, `page.OutputFormat(s)`, `media.Type`, `*source.File`, `page.Data`, `page.Taxonomy`/`TaxonomyList`/`WeightedPages`, config structs (`.Site.Config.Services.RSS.Limit`, `.Sitemap.ChangeFreq`), `*url.URL` (render-link), hook contexts, `ShortcodeWithPage`, namespace structs, `images.Filter` (gift) |

### 8.2 Printing rules (Go `fmt.Fprint` / `Sprint` / `%v`), with probe evidence

* `Sprint(args...)`: a space is added between two operands **only when neither is a string**
  (`print "a" 1 2 "b"` → `a1 2b`; `print 1 2` → `1 2`). `Sprintln`: always spaces + `\n`.
* bool → `true`/`false`; ints/uints → decimal; nil (untyped) → `<nil>`.
* **float64 `%v` = `strconv.FormatFloat(f, 'g', -1, 64)`** (fmt/print.go:445-448 → `fmtFloat(v, 64,
  'g', -1)`; algorithm in `$(go env GOROOT)/src/internal/strconv/ftoa.go:347-373 fmtEFG`): take the
  shortest round-trip decimal digits `d1d2…dn` and decimal-point position `dp`; `exp = dp-1`; because
  the precision is "shortest", `eprec = 6`; if `exp < -4 || exp >= 6` use `%e` form
  `d1[.d2…dn]e±XX` (exponent sign always present, at least 2 exponent digits), else `%f` form with
  exactly the significant digits (no trailing zeros, no trailing `.`). Probe values:
  `1.0`→`1`, `4.5`→`4.5`, `123456.0`→`123456`, `1234567.0`→`1.234567e+06`, `1e6`→`1e+06`,
  `1.5e10`→`1.5e+10`, `1e20`→`1e+20`, `1e21`→`1e+21`, `0.0001`→`0.0001`, `0.00001`→`1e-05`,
  `1e-7`→`1e-07`, `-0.0`→`-0`, `2.675`→`2.675`. float32 values use 32-bit shortest digits.
  (encoding/json uses different cut-offs — see §8.6.)
* string → as is. `[]byte` → `[1 2 3]` with %v.
* slices/arrays → `[` elems joined by single space `]` (`[a b c]`, `[1 two 3.5 true]`, `[]`,
  `[1 <nil> x]`); nested elements use their Stringer/error methods (depth>0 `handleMethods`).
* maps → `map[` k`:`v pairs joined by space `]`, keys sorted with fmtsort (`map[a:1 b:2 c:[1 2]]`,
  `map[a:<nil>]`, `map[]`).
* structs → `{f1 f2 …}`; pointer to struct/slice/map at top level → `&{…}`; nested pointers → `0x…`
  address (non-deterministic — never emitted by seeksnack; `{{ .Site }}` would print `{0xf2eb…}`).
* Stringer → `String()`; error → `Error()`; `time.Time.String()` →
  `2006-01-02 15:04:05.999999999 -0700 MST` (fraction trimmed; `+0000 UTC` for UTC; a monotonic reading
  would append ` m=+…` — only `now` has one).
* `Sprintf` verbs used by seeksnack: `%s`, `%d`, `%q`, `%v`. Error forms: wrong type
  `%!d(string=x)`; nil arg with non-v verb `%!s(<nil>)`; missing arg `%!s(MISSING)`; extra args
  `%!(EXTRA string=b)`; `%q` on string → `strconv.Quote` (Go escapes; printable Unicode kept, uses Go's
  `IsPrint` tables); `%q` on `[]string` → `["a" "b"]`; `%q` on a Stringer (`media.Type`) → quoted
  String(); `%x` on string → lowercase hex; width/precision/flags per fmt.
* text/template `printValue`: invalid → `<no value>`; nil pointer → `<nil>`; chan/func → error.
* html/template: every printed action ends in an escaper, which uses `stringify` (untyped nil skipped
  → `""`; typed values `fmt.Sprint`). So missing values print `""` in HTML templates.

### 8.3 Truthiness

`hreflect.IsTruthfulValue` (§2.3). Probe `TRUTH-01` (`FTFFFFFTFT`): nil param F; non-empty `[]string` T;
empty `slice` F; `0` F; `0.0` F; `""` F; zero `.ExpiryDate` (time.Time IsZero) F; `.Date` T; empty
`dict` F; `.Site` T.

### 8.4 Comparison / arithmetic

Implemented by Hugo's compare/math namespaces (other specs), but they require the Value model to keep
int vs uint vs float vs string distinctions: probe `eq 1 1.0` → **false**, `eq .Params.yint 5` → true,
`lt 1 2.5` → true, `ge "2019" "2026"` → false (string compare), `eq nil nil` → true, `ne 1 "1"` → true,
`add 1 2.0` → 3 (float64 printed `3`), `div 7 2` → 3, `div 7.0 2` → 3.5.

### 8.5 Range order

Maps (incl. `maps.Params`, `.Site.Taxonomies`, taxonomy term maps, data maps) iterate in fmtsort order
(byte-wise string order: `dict "z" 1 "a" 2 "M" 3` → `M a z`). Params keys are lower case, so
`range $k, $v := .Params` → `date,description,draft,iscjklanguage,lastmod,publishdate,title,…`.

### 8.6 JSON encoding (encoding/json v1, go1.27.1 toolchain; used by `jsValEscaper` and `jsonify`)

`jsonify` (tpl/encoding/encoding.go:64-110) = `json.NewEncoder` with `SetEscapeHTML(true)` (unless
`noHTMLEscape` opt), optional indent, trailing `\n` trimmed, returns `template.HTML`.
Rules to port (encode.go): nil/nil-pointer/nil-map/nil-slice → `null`; bool; ints/uints decimal;
**floats**: `'f'` format with shortest digits unless `abs < 1e-6 || abs >= 1e21` (then `'e'` with
`e-07`→`e-7` cleanup; encode.go:570-602); NaN/Inf → error; strings (`appendString`, encode.go:998):
`"`→`\"`, `\`→`\\`, BS/FF/LF/CR/TAB → `\b \f \n \r \t`, other bytes < 0x20 → `\u00XX` (lowercase
hex digits from `hex = "0123456789abcdef"`), **`<`,`>`,`&` → `\u003c`,`\u003e`,`\u0026`** (HTML escape
on), U+2028/U+2029 → `\u2028`/`\u2029`, invalid UTF-8 byte → `\ufffd`; maps: keys (string, int,
TextMarshaler) **sorted**; slices → arrays (`[]byte` → base64); structs → exported fields/json
tags/omitempty/embedding; `json.Marshaler` output is compacted (+HTML-escaped);
`encoding.TextMarshaler` → string; `time.Time` → RFC3339Nano (`"2020-09-06T15:46:26.955Z"`).
Probe `T-JSON`: `"yexp":15000000000`, `"title":"Home \"Q\" \u0026 'A' \u003cb\u003e"`,
`"ynull":null`, sorted keys.

---------------------------------------------------------------------------------------------------

## 9. Context threading (context.Context)

* Every execution carries a `context.Context` (`state.ctx`). `TemplateStore.ExecuteWithContext`
  pushes a `tpl.CurrentTemplateInfo{Parent, Level, Ops}` (tpl/template.go:160-189) used by
  `templates.Current`, the 999-level recursion guard and `partialCached` cycle detection.
* Other keys (`tpl.Context`, tpl/template.go:57-81): dependency-manager scope (watch mode),
  current Page (for `page` namespace funcs), `IsInGoldmark` (render hooks), set by hugolib/markup.
* The ctx is injected as the **first argument** of any template func or method whose first Go param
  is a context type (§2.4). Examples: `partial`/`partialCached` (`Include(ctx, name, …)`), every
  namespace constructor (`func(ctx, v ...any)`), the page methods declared with a leading
  `context.Context` in `resources/page/page.go` — `Content` (l.75, returns `any`),
  `ContentWithoutSummary`, `Plain`, `PlainWords`, `Summary`, `Truncated`, `FuzzyWordCount`,
  `WordCount`, `ReadingTime`, `Len`, `Render`, `RenderString`, `HeadingsFiltered`,
  `RenderShortcodes`, `TableOfContents`, `Fragments` — and `templates.Current(ctx)`,
  `templates.DoDefer(ctx, …)`. (`Paginate`, `Permalink`, `Title` … take no ctx.)
* Rust: pass `&ExecCtx` (cheap, `Arc` fields) through every eval call; methods declare whether they
  want it — no dynamic detection needed.

---------------------------------------------------------------------------------------------------

## 10. Proposed Rust design

### 10.1 Crate / module layout

```
crates/gotmpl/                         # generic Go-template engine (no Hugo types)
  src/parse/lex.rs        <- texttemplate/parse/lex.go        (687)
  src/parse/node.rs       <- texttemplate/parse/node.go       (1011)  incl. String() identical to Go
  src/parse/parse.rs      <- texttemplate/parse/parse.go      (831)
  src/value.rs            <- NEW: Value enum, SafeKind, IntTy/FloatTy, List/Map
  src/object.rs           <- NEW: TplObject trait (replaces reflect method/field dispatch)
  src/truth.rs            <- common/hreflect/helpers.go IsTruthfulValue (295, subset)
  src/fmtsort.rs          <- fmtsort/sort.go                  (154)
  src/gofmt/{print,format}.rs <- fmt/print.go (1208) + format.go (595): Sprint/Sprintln/Sprintf subset
  src/gostrconv/{ftoa,quote,isprint}.rs <- internal/strconv ftoa (shortest %e/%f/%g), strconv Quote, isprint tables (782)
  src/gojson.rs           <- encoding/json/encode.go (1344; only the encoder paths used)
  src/text/template.rs    <- texttemplate/template.go          (237) + option.go (72)
  src/text/exec.rs        <- texttemplate/exec.go (1135) merged with hugo_template.go (451)
  src/text/funcs.rs       <- texttemplate/funcs.go (785): and/or/not/len/html/urlquery/call + comparison (unused by Hugo but cheap)
  src/html/context.rs     <- htmltemplate/context.go + *_string.go (292+183)
  src/html/transition.rs  <- htmltemplate/transition.go        (688)
  src/html/escape.rs      <- htmltemplate/escape.go            (997)
  src/html/{html,js,css,url,attr,content,error}.rs <- html.go 270, js.go 490, css.go 262, url.go 216, attr.go 175, content.go 102, error.go 249
  src/html/template.rs    <- htmltemplate/template.go (531) + hugo_template.go (94)
  src/html/entity.rs      <- $(GOROOT)/src/html/escape.go (214) + entity.go (2261, generated table) — UnescapeString
crates/hugo-tpl/                       # Hugo glue
  src/store.rs            <- tplimpl/templatestore.go (2096)
  src/namespace.rs        <- tplimpl/templates.go (366)
  src/transform.rs        <- tplimpl/templatetransform.go (352)
  src/descriptor.rs       <- tplimpl/templatedescriptor.go (238) ; legacy.rs <- legacy.go (130); info.rs <- template_info.go (46)
  src/helper.rs           <- tplimpl/template_funcs.go (175)   (func table, method dispatch, Params lookup, mainsections)
  src/registry.rs         <- tpl/internal/templatefuncsRegistry.go (registry part) + tpl/tplimplinit (96)
  src/partials.rs         <- tpl/partials (308)
  src/strip.rs            <- tpl/template.go StripHTML (189)
  src/embedded/           <- include_str!("…/embedded/templates/**") (CRLF→LF)
```

### 10.2 Value model

```rust
pub enum Value {
    Invalid,                              // reflect.Value{}: missing key, nil interface, YAML null
    NilPtr(&'static str /*go type*/),     // typed nil pointer: falsy, prints "<nil>" (text) / "<nil>" via fmt
    Bool(bool),
    Int(i64, IntTy),                      // IntTy: Int (Go int), Int8..Int64 — keep for %T and math typing
    Uint(u64, UintTy),
    Float(f64, FloatTy),                  // F64 | F32 (F32 formats with 32-bit shortest digits)
    String(Arc<str>),
    Safe(SafeKind, Arc<str>),             // template.{HTML,CSS,HTMLAttr,JS,JSStr,URL,Srcset}
    HString(Arc<str>),                    // hstring.HTML: String()=s, PrintableValue()=Safe(HTML,s)
    Time(Arc<GoTime>),                    // time.Time (+Location, monotonic flag); methods Format/IsZero/Year/…
    List(Arc<List>),                      // List { ty: SliceTy, items: Vec<Value> }  SliceTy::{Any, Strings, Ints, Pages, Resources, Languages, …}
    Map(Arc<GoMap>),                      // GoMap { ty: MapTy::{Params, StringAny, PageData, Taxonomy…}, entries: BTreeMap<String, Value> }
    Object(Arc<dyn TplObject>),           // Page, Site, Pager, Resource, Scratch, Language, OutputFormat, namespaces, hook contexts…
}
```
* `BTreeMap<String, Value>` gives Go's sorted iteration for `range`, fmt and JSON (Rust `String`
  ordering == Go string ordering == byte order).
* `MapTy::Params` means: keys stored lower-cased, lookups lower-case the key, methods `IsZero`,
  `GetNested`, … win over keys; `IsZero` = empty or only `_merge`.
* Keep the slice element type (`SliceTy`) so `sort`/`where`/`first`/`after` can return the same type and
  `.Reverse`/`.Related`/`.Get` method tables stay reachable (Go keeps the concrete slice type).
* Pages/Resources could alternatively be `Object`s that expose `list()`; either way they must support
  range, len, index and methods.

```rust
pub trait TplObject: Send + Sync + 'static {
    fn go_type(&self) -> &'static str;                   // "*hugolib.pageState" (for %T / errors)
    fn kind(&self) -> Kind { Kind::Struct }              // emulated reflect.Kind: Struct|Map|Slice
    /// Exported method or field with this exact name (methods first). None => "can't evaluate field".
    fn member(&self, ctx: &ExecCtx, name: &str, args: &[Value]) -> Option<Result<Value, Error>>;
    fn has_member(&self, name: &str) -> bool;            // for "method vs map key" precedence
    fn map_get(&self, key: &str) -> Option<Value> { None }     // Kind::Map objects (page.Data …)
    fn list(&self) -> Option<Arc<[Value]>> { None }            // Kind::Slice objects
    fn is_zero(&self) -> Option<bool> { None }                 // types.Zeroer
    fn go_string(&self) -> Option<String> { None }             // fmt.Stringer / error
    fn printable(&self) -> Option<Value> { None }              // types.PrintableValueProvider
    fn marshal_json(&self, e: &mut JsonEnc) -> Option<Result<(), Error>> { None }
    fn identity(&self) -> usize;                                // pointer identity (eq, fmtsort)
}
```
Method dispatch is exact-case string matching over a per-type table (a `match name { … }` or a
`phf` map) — this replaces `reflect.MethodByName` and is where every Hugo type's template-visible API
lives (Page, Site, Pager, Resource, …). The members seeksnack uses are listed in §13.

### 10.3 Parser

Port lex/parse/node verbatim, keeping: item types, trim-marker logic, comment rules, number parsing
(`newNumber` incl. char constants, `_` separators, hex floats), `IsEmptyTree`, `Tree.Copy`,
`CopyList`, `Node::String()` (needed by the escaper: `handleDefer` hashes `inner.String()`, error
messages, and the escdump oracle). Assign each node a stable `NodeId` (u32) at creation; derived
copies get fresh ids. Parse-time function-name checking needs the func-name set (Hugo funcs ∪ builtins ∪
escaper names).

### 10.4 Executor

* `State { tmpl: &Template (namespace), ctx: &ExecCtx, helper: &dyn ExecHelper, out: &mut Vec<u8>,
  vars: Vec<(Arc<str>, Value)>, depth: u32 }`.
* `Result<Value, Ctrl>` with `enum Ctrl { Break, Continue, Err(ExecError) }` replaces Go panics.
* `eval_call(fun, args_nodes, final: Option<Value>, first_ctx: bool)`: evaluate arg nodes with ideal
  constant rules (§4.2), append `final` last, then call. `and`/`or` implemented as special forms
  (short-circuit, return operand). `try` → wrap result/err in a TryValue object.
* `eval_field(dot, name, args, final, receiver)` = §6 algorithm: Invalid → Invalid; NilPtr → member of
  nil receiver (only nil-safe methods like `IsZero`) else error; Object → `member()`; Map → methods of
  the map type first, then Params lower-case / exact key; List objects → methods; Time → time methods.
* `print_value`: text/template → `Invalid` ⇒ `<no value>`, else `gofmt::fprint(v)`.
* `walk_range`: Int/Uint → 0..n; List → (index Int(Go int), elem); Map → sorted; Invalid/empty → else.
* After each pipeline command, Go unwraps `interface{}`; in Rust values are already concrete — but
  keep the rule "a nil interface becomes Invalid".
* `{{ template }}` → new vars `[("$", dot)]`, depth+1 (max 100000).

### 10.5 html/template escaper

* Port escape.go/transition.go/context.go line-by-line. Replace pointer-keyed edit maps with
  `HashMap<NodeId, …>`; `commit()` walks the (mutable) trees and applies edits: insert escaper commands
  (`ensurePipelineContains` + `appendCmd` rules), replace text bytes, rename template calls, register
  derived trees (`name$htmltemplate_…`).
* Escape all templates at store build (as Go's `prepareTemplates` does); after that freeze trees in
  `Arc` for parallel execution.
* Escaper functions are ordinary builtin funcs named `_html_template_*` taking `&[Value]`, using a
  Rust `stringify` that mirrors content.go + Hugo's `indirect` (unwrap `HString`/Summary → HTML).
* Namespaces: parseHTML/parseText shared namespaces + one namespace per (overlay, base) pair. Go shares
  tree pointers between parseHTML and the baseof clones (text Clone); in Rust clone-on-write is fine
  because layouts never `{{template}}` into partial trees in practice.

### 10.6 Go stdlib clones needed for byte parity

| component | used by | notes |
|---|---|---|
| `fmt` Sprint/Sprintln/Sprintf + `%v` value printing | print/printf/println, text printValue, escaper stringify, `%!s(<nil>)` | port `printArg`, `printValue` (maps sorted with fmtsort, `&{}` rules), `badVerb`, MISSING/EXTRA, width/prec/flags for `s d q v x T f` |
| `strconv.FormatFloat` 'g'/'e'/'f' shortest + fixed | fmt, json, `%f` | get shortest digits (ryu or Rust `{:e}`), then Go layout (`fmtE/fmtF/fmtEFG`) |
| `strconv.Quote` + `IsPrint` tables | `%q` | port `strconv/isprint.go` tables (Unicode version of go1.27.1 = 17.0.0) |
| `encoding/json` encoder | jsValEscaper, jsonify | §8.6 |
| `html.UnescapeString` | contextAfterText (attr value decoding) | port `html/escape.go` + entity table |
| `net/url.QueryEscape` | `urlquery` builtin | uppercase `%XX`, space → `+` |
| `time.Time.String()`/`Format` | printing dates | owned by the time spec, but the engine calls it |
| `unicode.IsSpace`, `strings.ToLower` (Go per-rune simple mapping) | StripHTML, Params keys, htmlNameFilter | Rust `char::is_whitespace` == Go IsSpace set; Go ToLower ≠ Rust `to_lowercase` for İ/Σ (use per-char simple lowercase) |

---------------------------------------------------------------------------------------------------

## 11. What to port line-by-line (Go sources and line counts)

| Go source | lines (non-test) | port mode |
|---|---:|---|
| tpl/internal/go_templates/texttemplate/parse/{lex,node,parse}.go | 2,529 | line-by-line |
| tpl/internal/go_templates/texttemplate/{exec,funcs,template,option,hugo_template}.go | 2,680 | line-by-line (reflection → Value/TplObject) |
| tpl/internal/go_templates/htmltemplate/*.go (excl. doc.go) | 4,539 | line-by-line |
| tpl/internal/go_templates/fmtsort/sort.go | 154 | line-by-line (over Value) |
| common/hreflect/helpers.go | 295 | IsTruthfulValue / IsContextType semantics only |
| tpl/tplimpl/{templatestore,templates,templatetransform,template_funcs,templatedescriptor,legacy,template_info}.go | 3,403 | line-by-line (drop watch/rebuild paths: RefreshFiles, createTemplatesSnapshot, trackDependencies) |
| tpl/template.go | 189 | StripHTML + CurrentTemplateInfo |
| tpl/partials/{partials,init}.go | 308 | line-by-line |
| tpl/tplimplinit/tplimplinit.go + registry part of templatefuncsRegistry.go | ~210 | re-implement as a static table |
| Go stdlib subsets (fmt, strconv ftoa/quote/isprint, encoding/json encode, html unescape, url.QueryEscape) | ~5,000 source → ~2,000 Rust | behaviour port, verified by tests |
| **Total engine + glue** | **≈ 14,300 Go lines** | |

Not needed: texttemplate/helper.go (ParseFiles/Glob), doc.go files, testenv/, cfg/, watch-mode
dependency tracking, metrics, `scripts/fork_go_templates`.

---------------------------------------------------------------------------------------------------

## 12. Rust crates: safe vs unsafe for byte parity

Safe (no effect on output bytes, or only used behind a Go-compatible layer):
* `regex` — only for `(?i)<(script|/script|!--)` (escape.go:730), `(?i)<(/?)script` (js.go:151) and the
  error-context regex; Go and Rust both use Unicode simple case folding for `(?i)`. (Or hand-code.)
* `memchr`, `smallvec`, `rustc-hash`/`ahash` (for unordered internal maps only), `parking_lot`,
  `rayon` (page-level parallelism; each page's output is independent), `once_cell`/`OnceLock`,
  `typed-arena`/`bumpalo` for AST storage, `phf` for method tables.
* `ryu` (or std `format!("{:e}")`) **only to obtain shortest digits + exponent**; the textual layout
  must be Go's (`1e+06`, `1.5e+10`, `-0`). Fuzz-compare against Go for random f64/f32.
* `std::collections::BTreeMap<String, _>` for Go-sorted maps.

NOT safe (would break parity):
* `serde_json` for jsonify / jsValEscaper — prints `15000000000.0`/`1.5e10` vs Go `15000000000`,
  does not escape `< > &` as `\u003c\u003e\u0026`, no `\u2028`/`\u2029`, no Go struct/time/[]byte rules.
  (A custom `serde_json::ser::Formatter` could fix floats/escapes but the Go-specific type rules still
  need a custom encoder — write one over `Value`.)
* `html-escape`, `askama_escape`, `v_htmlescape`, `htmlescape` — use `&quot;`/`&#x27;`/do not escape
  `+` or NUL; Go uses `&#34;` `&#39;` `&#43;` and U+FFFD for NUL.
* `percent-encoding`, `urlencoding`, `url` crate serialization — uppercase hex / different reserved
  sets; html/template uses **lowercase** `%xx` with its own sets; Go `url.QueryEscape` uses uppercase
  and `+`.
* `tera`, `handlebars`, `minijinja`, `askama`, `gtmpl`/`gtmpl-value` — different grammar/semantics;
  none implements Go's html/template contextual escaper nor Hugo's hooks. Not a base for the port.
* `ammonia`, `html5ever`, `scraper` for `stripTags` — different tokenisation; port `stripTags`.
* Rust `f64` `Display`/`Debug`, `chrono` formatting, `str::to_lowercase` (special casing),
  `char::is_alphanumeric` for Go identifier rules (use Go `unicode.IsLetter/IsDigit` semantics).

---------------------------------------------------------------------------------------------------

## 13. Template-visible API used by seeksnack (method/field names on objects)

Field/method identifiers (exact case, counts across `layouts/**`): Params(238) Site(221)
Permalink(116) Title(91) RelPermalink(67) Get(37) Resize(32) Scratch(23) Kind(22) BaseURL(16)
IsHome(14) Data(14) PageNumber(12) Format(12) Description(12) Date(12) Resources(11) Pages(11)
Lang(11) Set(10) Singular(9) Paginator(9) Page(8) URL(7) RegularPages(6) Language(6) IsZero(6)
Content(6) Add(6) Sitemap(5) Reverse(4) PublishDate(4) Lastmod(4) IsTranslated(4) IsPage(4)
AllTranslations(4) Width(3) Type(3) TotalPages(3) Text(3) Parent(3) Height(3) HasNext(3) SetInMap(2)
Section(2) Related(2) Paginate(2) PagerSize(2) Next(2) MediaType(2) Level(2) LanguageName(2) IsNode(2)
GetPage(2) Anchor(2) UniqueID(1) Translations(1) Taxonomies(1) Rel(1) ReadingTime(1) Prev(1) Plain(1)
Pagers(1) OutputFormats(1) Last(1) IsSection(1) Home(1) HasPrev(1) First(1) File(1) Destination(1)
Delete(1) Config(1) ChangeFreq(1) AlternativeOutputFormats(1) AllPages(1) — plus Params keys
(case-insensitive) such as Social.Facebook.Enable, Comment.Apipro/Apidev, Typescript.Compiler.Target,
Monetization.Ads.*, Marketing.*, HomeTitle, LanguageCodeOpenGraph, Carousel.TotalShow, Author,
Brands, Companies, Countries, Categories, Tags, Image, Low_price…, and `.Site.Config.Services.RSS.Limit`.
Embedded templates add: `.Destination .Title .Text .PageInner.{RelPermalink,GetPage,Resources}`
(render-link), `.Attributes .THead .TBody .Alignment .Text` (render-table), `.SitemapAbsURL .Lastmod`
(sitemapindex), `.Permalink` + `site.Language.LanguageCode` (alias), `ref . .Params` (ref shortcode);
`urls.Parse` → `*url.URL` methods `String IsAbs Path RawQuery Fragment`.
Template funcs used by seeksnack: eq dict i18n partial resources.Get or printf path.Ext relLangURL sort
index js.Build and hugo.Environment fingerprint urlize slice safeHTML not gt reflect.IsSlice first
images.Overlay images.Filter default humanize resources.Concat now (.Format) lower apply where ge
dateFormat ne len absLangURL sub site.Params seq add site.Languages safeURL safeJS mul markdownify html
trim toCSS title urldecode transform.Unmarshal site.Title site safeCSS resources.PostProcess
resources.GetRemote resources.ExecuteAsTemplate print postCSS newScratch minify lt le jsonify isset
hugo.Generator absURL (+ keywords template/define/block/break).

---------------------------------------------------------------------------------------------------

## 14. Parity risks (ranked)

1. **Porting the wrong html/template version.** Use the go1.24.0 fork semantics (no meta-content URL
   filtering, `isJSType("")` false, no jsBraceDepth in `eq`). Porting from current Go would change
   1,482 alias files (`url={{ .Permalink }}` would gain `urlfilter`; output identical for http URLs
   but not in general) and potentially script-type handling.
2. **fmt printing of nil/missing** — `%!s(<nil>)` in 1,678 golden files; `""` vs `<no value>` vs `<nil>`
   depending on html/text and typed/untyped nil. The Value model must distinguish `Invalid` (skipped by
   html stringify, `<no value>` in text) from `NilPtr` (`<nil>`), and Hugo's Params lookup must turn
   `null` into Invalid.
3. **Case rules**: only `maps.Params` keys are case-insensitive (lower-cased); methods, struct fields,
   `dict`/data/unmarshal maps are exact. Methods shadow map keys. `mainsections` special case.
4. **Truthiness** must call `IsZero` (zero time, empty Params, nil `*File`, Summary …) before kind rules.
5. **Escaping tables** exact: `&#34;` `&#39;` `&#43;` `&lt;` `&gt;` `&amp;`, NUL → U+FFFD; lowercase `%xx`
   in URLs; JS `\u0027 \/ \u002b \u003c …`; CSS `\22 ` spacing rule; `html` builtin (no `+`) vs
   `htmlescaper` (`+`→`&#43;`); `urlquery` uppercase `%XX` and `+`.
6. **Template text rewriting** by the escaper (HTML/JS/CSS comment removal, stray `<` → `&lt;`,
   `<!DOCTYPE` exemption, `\x3C` in script literals). Many seeksnack pages contain `<!-- … -->` in
   layouts that must disappear.
7. **Engine choice per output format** (RSS/sitemap/alias/sitemapindex are html/template!) and
   RCDATA handling for `<title>`/`<textarea>` (incl. derived templates for `block` inside `<title>`).
8. **JS value encoding** (Go encoding/json v1 with HTML escaping, number/keyword padding, time
   RFC3339Nano, sorted keys) for JSON-LD in every page.
9. **Content types**: `partial` returns `template.HTML` (html partial) — trusted in HTML text,
   tag-stripped in attributes; `hstring.HTML`/Summary unwrap; `safe*` types; `jsonify` returns HTML.
10. **Float formatting** `%v` (`e+06` at exp ≥ 6) and JSON (`'f'` until 1e21) — two different rules.
11. **Function precedence**: Hugo funcs override Go builtins (`eq … println`), `js` namespace shadows
    the builtin; `and`/`or` return operands; `slice` builds a typed list; `len` counts bytes.
12. **Template assembly**: `IsEmptyTree` association rule for baseof/define; inline partial extraction
    (`partials/...` defines); embedded templates (render-link fallback for multilingual single-host,
    render-table per output format, alias, sitemapindex, ref); `needsBaseTemplate` detection incl.
    leading comments; BOM stripping; CRLF only normalised for embedded templates.
13. **Whitespace control**: trim markers trim `" \t\r\n"` only; `{{-3}}` is a number; comment syntax.
14. **Range order**: fmtsort byte order for string keys (uppercase before lowercase).
15. **`.Plain`/`plainify`/`countwords`** depend on `stripTags` + `StripHTML` whitespace collapsing
    (`unicode.IsSpace`), feeding `index.json` (363 KB) and search data.
16. **Non-determinism outside the engine**: `now` (footer/RSS copyright year "2019 -2026"), remote
    YouTube data (`resources.GetRemote`) printed into JSON-LD; pointer-address printing (`{{ .Site }}`)
    — not used by seeksnack but would never match.
17. **Unicode tables**: `strconv.IsPrint` (for `%q`), Go `ToLower`, `IsLetter` for identifiers — ASCII
    in seeksnack, low risk.

---------------------------------------------------------------------------------------------------

## 15. Test plan / assets

1. **Port Go's own tests** (11,226 lines of `_test.go` in the fork): `parse/lex_test.go` (595),
   `parse/parse_test.go` (863), `texttemplate/exec_test.go` (1,981, table `execTests`),
   `htmltemplate/escape_test.go` (2,205: `TestEscape`, `TestEscapeText` context table, `TestEscapeSet`,
   `TestErrors`, `TestEnsurePipelineContains`, `TestRedundantFuncs`), `content_test.go` (462: every
   content type × every context — ideal oracle), `js_test.go`, `css_test.go`, `url_test.go`,
   `html_test.go` (stripTags), `transition_test.go`, `hugo_template_test.go`.
2. **Probe fixture** `work/template-engine/tsite` + `expected/probe-index.{html,json}` (Appendix A).
3. **Escaper oracle** `work/template-engine/escdump/dump.txt`: build the same namespaces in Rust
   (baseof+overlay per layout; everything else in one shared namespace), escape, print each tree with
   `Node::String()` sorted by name, diff.
4. **Golden site**: `golden/run1` (minified) and `golden/nominify` (unminified, 6,943 files) — diff the
   unminified Rust output against `golden/nominify` first to separate template issues from minifier
   issues.

---------------------------------------------------------------------------------------------------

## Appendix A — probe outputs from the Go binary (html/template, `layouts/index.html`)

```
<html><head><title>Home &#34;Q&#34; &amp; &#39;A&#39; &lt;b&gt;</title></head><body>

PRINT-01:5|4.5|5|12345678901|007|true|||
PRINT-02:[a b c]|[1 x 2.5]|map[a:1 b:2 c:[1 2]]|2021-01-02|-3|1.5e&#43;10|
PRINT-03:2020-09-06 15:46:26.955 &#43;0000 UTC|2020-09-06 15:46:26.955 &#43;0000 UTC|2020-09-06 15:46:26.955 &#43;0000 UTC|0001-01-01 00:00:00 &#43;0000 UTC|false|
PRINT-04:3|1|2.5|1e&#43;21|1e-06|1e-07|[x y]|[1 two 3.5 true]|mc|mc|map[key:v]|v|
PRINT-05:1|1|1.5|1000|16|97|true|s|&lt;nil&gt;|1 2|ab|a1b|x
|
PRINT-06:[a b c]|&#34;x&#34;|3|[a b c]|3.14|map[a:1 b:2 c:[1 2]]|int|float64|[]string|maps.Params|int64|float64|time.Time|string|[]interface {}|int|
PRINT-07:[1 2 3]|map[a:2 b:1]|[1 2 3]|[]|map[]|["a","b"]|{"a":1,"b":2,"c":[1,2]}|
PRINT-09:Home "Q" & 'A' <b>|Home &amp;#34;Q&amp;#34; &amp;amp; &amp;#39;A&amp;#39; &amp;lt;b&amp;gt;|Home &#34;Q&#34; &amp; &#39;A&#39; &lt;b&gt;|Home&#43;%22Q%22&#43;%26&#43;%27A%27&#43;%3Cb%3E|
TRUTH-01:FTFFFFFTFT|
AND-OR:0|2|x||true|007|
LEN:6|3|3|
RANGE-MAP:a=1;b=2;c=[1 2];|M=3;a=2;z=1;|
RANGE-INT:012|0a1b|EMPTY|
WITH-ELSE:B007|
BREAK:12|1245|
EQ:false|true|true|true|false|true|true|
ATTR:<a title="Home &#34;Q&#34; &amp; &#39;A&#39; &lt;b&gt;" href="desc%20&#43;%20plus%20/%20slash" data-x="[a b c]">x</a>
UNQ:<a title=Home&#32;&#34;Q&#34;&#32;&amp;&#32;&#39;A&#39;&#32;&lt;b&gt;>x</a>
URL1:<a href="/p?q=Home%20%22Q%22%20%26%20%27A%27%20%3cb%3e#Home%20%22Q%22%20%26%20%27A%27%20%3cb%3e">x</a>
URL2:<a href="#ZgotmplZ">x</a><a href="http://x.org/a%20b?c=d&amp;e">y</a><a href="mailto:a@b">z</a><img src="/a/%c3%bc%20b.png">
SRCSET:<img srcset="#ZgotmplZ, /c.png 2x">
JS1:<script>var a = "Home \"Q\" \u0026 'A' \u003cb\u003e"; var b = "Home \u0022Q\u0022 \u0026 \u0027A\u0027 \u003cb\u003e"; var c = 'Home \u0022Q\u0022 \u0026 \u0027A\u0027 \u003cb\u003e'; var d = ["a","b","c"]; var e = {"a":1,"b":2,"c":[1,2]}; var f =  5 ; var g =  4.5 ; var h = "2020-09-06T15:46:26.955Z"; var i =  null ; var j =  null ; var k =  true ; var l =  -3 ;</script>
JS2:<script>

 var x = 1; 
var y = `tmpl Home \u0022Q\u0022 \u0026 \u0027A\u0027 \u003cb\u003e`;</script>
JS3:<button onclick="f(&#34;Home \&#34;Q\&#34; \u0026 &#39;A&#39; \u003cb\u003e&#34;, 'Home \u0022Q\u0022 \u0026 \u0027A\u0027 \u003cb\u003e')">b</button>
JSON-LD:<script type="application/ld+json">{"a": "Home \"Q\" \u0026 'A' \u003cb\u003e", "b": "Home \u0022Q\u0022 \u0026 \u0027A\u0027 \u003cb\u003e", "d": "2020-09-06 15:46:26.955 \u002b0000 UTC", "s": "a'b\"c\u003cd\u003e\u0026e+f" }</script>
NONJS:<script type="text/template">Home &#34;Q&#34; &amp; &#39;A&#39; &lt;b&gt; <b>Home &#34;Q&#34; &amp; &#39;A&#39; &lt;b&gt;</b></script>
CSS1:<style>p { color: red; background: url(/a%20b.png); font-family: "Home \22Q\22  \26  \27 A\27  \3c b\3e "; }   </style>
CSS2:<p style="color: ZgotmplZ">p</p>
TEXTAREA:<textarea>Home &#34;Q&#34; &amp; &#39;A&#39; &lt;b&gt;</textarea>
TEXTLT:a &lt; b and c > d & e &lt;?xml x?> <!DOCTYPE y>
SAFE:<b>x</b>|<a href="javascript:x">x</a>|<p title="t&#43;">p</p>
TYPES:template.HTML|template.HTML|map[string]interface {}|map[a:1]|
PARTIAL:[<b>Home &#34;Q&#34; &amp; &#39;A&#39; &lt;b&gt;</b>
]
TPL:<i title="Home &#34;Q&#34; &amp; &#39;A&#39; &lt;b&gt;">Home &#34;Q&#34; &amp; &#39;A&#39; &lt;b&gt;</i>
H-TRIM:[x]
H-ERR:||
PRINTF-NIL:%!s(&lt;nil&gt;)|%!s(&lt;nil&gt;)|<meta content="%!s(&lt;nil&gt;)">
```
(Input: `tsite/content/_index.md` front matter — title `Home "Q" & 'A' <b>`, yint 5, yfloat 4.5,
yfloat0 5.0, ybig 12345678901, ystr "007", ybool, ynull, ylist [a,b,c], ylistmixed [1,x,2.5], ymap
{B:2,a:1,c:[1,2]}, ydate 2021-01-02, yneg −3, yexp 1.5e10; `hugo.toml` params intv 3, floatv 1.0,
floatv2 2.5, bigf 1e21, smallf 1e-6, tinyf 1e-7, arr [x,y], mixed [1,"two",3.5,true], Mixed_Case,
nested.Key.)

## Appendix B — probe outputs (text/template, `layouts/index.json`)

```
T-PRINT-01:5|4.5|<no value>|<no value>|[a b c]|map[a:1 b:2 c:[1 2]]|2020-09-06 15:46:26.955 +0000 UTC|Home "Q" & 'A' <b>|
T-PRINT-02:1|<no value>|<no value>|Home &#34;Q&#34; &amp; &#39;A&#39; &lt;b&gt;|Home+%22Q%22+%26+%27A%27+%3Cb%3E|
T-PRINT-03:a1 2b|1 2|ab|1.5 true|[1 <nil> x]|map[a:<nil>]|%!s(<nil>)|<nil>|%!d(string=x)|a %!s(MISSING)|a%!(EXTRA string=b)|["a" "b"]|[map[a:1]]|6869|  3.1|7   |0007|
T-PRINT-04:{alternate {json application/json  index alternate  true false false false false false false 0} /index.json https://example.org/index.json}|application/json|"application/json"|Home "Q" & 'A' <b>|int|[]interface {}|int|3|3|3|3|3.5|3|int64|
T-CASE:2|2|<no value>|1|mc|
T-TRIM:[x]|[]|[ab]|
T-SCOPE:21|2|dotvaldotval|wHome "Q" & 'A' <b>|
T-RANGE:date,description,draft,iscjklanguage,lastmod,publishdate,title,ybig,ybool,ydate,yexp,yfloat,yfloat0,yint,ylist,ylistmixed,ymap,yneg,ynull,ystr,|0:P1;|12|
T-NUM:3|1e+06|123456|1.234567e+06|0.0001|1e-05|-0|1e+20|0.1|2.675|1000|5|15|15|
T-FLOAT32:0.1|
```
Notes: `T-PRINT-04` shows that printing a struct (`.OutputFormats.Get "json"`) dumps Go struct fields —
never do this in parity-critical output. `T-SCOPE`: inner `$x :=` shadows only inside the `if`; `$y =`
assigns the outer variable; `{{ template "sc" "dotval" }}` sets both `.` and `$`.

## Appendix C — reproduction commands

```
# probe site
cd …/scratchpad/work/template-engine/tsite && …/scratchpad/bin/neohugo-go -d ../tout
# escaper dump (Go 1.27 toolchain, copies of the forked packages under escdump/gt)
cd …/scratchpad/work/template-engine/escdump && GOFLAGS=-mod=mod GOPROXY=off go build -o dump ./cmd/dump
./dump …/scratchpad/sites/seeksnack/layouts > dump.txt
./dump ./emb > dump-embedded.txt
```

---------------------------------------------------------------------------------------------------

## 16. MANDATORY: engine contract required by the Hugo layer (added by the orchestrator)

The Hugo layer (crates/nh-tplimpl, nh-tplfuncs, nh-hugolib — being designed in parallel, see
crates/HUGO_LAYER.md) consumes `crates/gotemplate`. A design review found that the following Hugo
semantics must live INSIDE the engine; they cannot be patched on later from outside. Treat this
section as acceptance criteria for the gotemplate crate, and document the resulting public API in
crates/gotemplate/PORTING.md under a heading "Host contract".

1. **ExecHelper trait** — the Rust equivalent of `texttemplate.ExecHelper`
   (tpl/internal/go_templates/texttemplate/hugo_template.go:45-51), extended with truthiness:
   - `init(ctx, tmpl)`
   - `get_func(ctx, tmpl, name) -> Option<Func>` — resolution at execution time, plus a separate
     parse-time set of known function names (Hugo's func map ∪ builtins ∪ `_html_template_*`).
   - `get_method(ctx, tmpl, receiver: &Value, name) -> Option<Method>` — consulted for EVERY
     receiver kind, including `Value::List`/`Value::Map` with named Go types (page.Pages.Reverse,
     page.Taxonomy.Alphabetical…), `Value::Time` (Format, IsZero, Year, …), `Value::Safe`, and
     `Value::Object` (default: delegate to `Object::has_method/call_method`). Methods take
     precedence over struct fields and map keys (Go `evalField` order).
   - `get_map_value(ctx, tmpl, receiver: &Value, key) -> Option<Value>` — map key lookup hook
     (Hugo lower-cases keys for `maps.Params`; exact-case for other maps).
   - `on_called(ctx, tmpl, name, args, result)` — may be a no-op.
   - `is_true(&Value) -> (truth: bool, ok: bool)` — replaces Go's `isTrue`; Hugo implements it with
     `hreflect.IsTruthfulValue` (calls `IsZero()` first: zero `time.Time`, empty/`_merge`-only
     `maps.Params`, `Object::is_zero`, typed nils). Used by `if`, `with`, `else with`, `and`, `or`,
     `not`, and `range`'s emptiness check where Go uses isTrue.
2. **Host context threading.** `execute(tmpl, out, data, ctx)` takes an opaque host context
   (`go_value::HostCtx` / `&dyn Any`). The engine passes the SAME ctx unchanged to every helper call
   and every func/method call (Go injects `context.Context` as first argument). It never rebuilds
   or replaces it.
3. **Re-entrancy.** A func or method invoked during execution may itself execute other templates
   (partials, markdown render hooks through `.Content`, `resources.ExecuteAsTemplate`), possibly
   the same template, recursively. So: templates are immutable after preparation/escaping and
   shareable across threads (`Arc`); all execution state is per call; the engine holds no lock
   while calling out to funcs, methods or the helper.
4. **`and` / `or`** short-circuit and return the deciding operand (not a bool), as Go ≥1.18.
5. **Argument evaluation** follows Go `evalArg`/`validateType`/`idealConstant`: ideal numeric/char
   constants, untyped `nil` passed to an interface parameter, `Invalid` args. Since Rust funcs
   take `&[Value]`, define and document exactly which Value each literal/constant becomes
   (e.g. `1` → int, `1.0` → float64, `'a'` → per Go rules) and how a func signals arity/type errors.
6. **Printing & nils.** text/template `printValue`: `Invalid` → `<no value>`; a typed nil
   (`Value::TypedNil(T)`, including a nil *non-empty interface* where
   `go_value::typed_nil_kind(T) == Interface`) → `<nil>`; otherwise go-fmt `Sprint` semantics.
   A nil value of an EMPTY interface type becomes `Invalid` after a pipeline command (Go's
   `value.Kind()==Interface && NumMethod()==0` unwrap) — the host represents such values as
   `Invalid` directly. Field/method access on `Invalid` yields `Invalid` (or the missingkey
   error), and on a typed nil pointer yields Go's "nil pointer evaluating X.Y" error unless the
   helper's `get_method` resolves a nil-safe method.
7. **html/template escapers** unwrap `Object::printable_value()` (Hugo's `indirect` /
   PrintableValueProvider, htmltemplate/hugo_template.go) before `stringify`, and treat
   `Value::Safe(kind, s)` as the corresponding template content type.
8. **Errors** from funcs/methods propagate as template execution errors with Go's message format;
   `try` (Hugo keyword) wraps them.
9. Provide a small test FuncMap/helper in the crate's tests so all of the above is covered by
   differential tests against the Go fork (probe site: work/template-engine/tsite + expected).
