# goldmark — porting notes

Byte-exact port of `github.com/yuin/goldmark` **v1.7.12** (the version pinned
by neohugo's `go.mod`): the core (`ast`, `text`, `util`, `parser` with all
block and inline parsers, attributes and delimiter processing, `renderer` +
`renderer/html`, `markdown.go`) **and all of `extension/`** (table,
strikethrough, linkify, tasklist, definition list, footnote, typographer,
GFM, CJK and `extension/ast`). Hugo's own extensions (render hooks,
hugocontext, attributes/auto IDs, blockquotes, tables, codeblocks) live in
`nh-markup` and use the plugin API described below.

## Go file → Rust module

| Go (goldmark v1.7.12) | Rust | Notes |
|---|---|---|
| `markdown.go` | `src/lib.rs` | `new`, `Markdown`, `Extender`, `with_*` options, `default_parser/renderer`, `convert` |
| `ast/ast.go` | `src/ast/mod.rs` | arena `Ast` + `NodeId`; `NodeKind` registry; `Attribute`/`AttrValue`; `walk`/`walk_ref`; `Dump` |
| `ast/block.go` | `src/ast/block.rs` | block node payloads + `Ast::new_*` constructors/accessors |
| `ast/inline.go` | `src/ast/inline.rs` | inline payloads, `Text` flags, `merge_or_append_text_segment`, `merge_or_replace_text_segment` |
| `text/segment.go` | `src/text/segment.rs` | `Segment`, `Segments` |
| `text/reader.go` | `src/text/reader.rs` | `Reader` trait; `TextReader` (`text.NewReader`), `BlockReader` (`text.NewBlockReader`), `RuneStream` |
| `util/util.go`, `util_safe.go`/`util_unsafe_*.go` | `src/util/mod.rs` (+ `tables.rs`: the byte tables copied verbatim) | `BufWriter`, `BytesFilter`, `PrioritizedValue`, escapes, `URLEscape`, reference resolution, case folding, `FindClosure`, indentation helpers |
| `util/html5entities.go` + `.gen.go` | `src/util/html5entities.rs` + `html5entities_gen.rs` | table generated from the `.gen.go` file by the oracle (`gentables`), cross-checked entry by entry |
| `util/unicode_case_folding.go` + `.gen.go` | `src/util/unicode_case_folding.rs` + `unicode_case_folding_gen.rs` | same |
| `util/util_cjk.go` | `src/util/util_cjk.rs` | range tables + `IsEastAsianWideRune`, `IsSpaceDiscardingUnicodeRune`, `EastAsianWidth` |
| `parser/parser.go` | `src/parser/mod.rs` | `Context`, `IDs`, `Reference`, `State`, traits, `Parser`, `parseBlocks/openBlocks/closeBlocks/parseBlock` |
| `parser/delimiter.go` | `src/parser/delimiter.rs` | `Delimiter`, `DelimiterProcessor`, `scan_delimiter`, `process_delimiters`, `DelimiterBottom` |
| `parser/{atx_heading,setext_headings,thematic_break,list,list_item,blockquote,code_block,fcode_block,html_block,paragraph}.go` | `src/parser/<same>.rs` | one file each |
| `parser/{code_span,emphasis,link,link_ref,auto_link,raw_html,attribute}.go` | `src/parser/<same>.rs` | one file each |
| regexps in `parser/html_block.go`, `parser/raw_html.go` | `src/parser/regexps.rs` | hand-written matchers (see deviations) |
| `renderer/renderer.go` | `src/renderer/mod.rs` | |
| `renderer/html/html.go` | `src/renderer/html.rs` | (+ `render_attribute_list`, `HtmlRendererOption`) |
| `extension/package.go` | `src/extension/mod.rs` | re-exports; Go's package-level extension vars are constructor fns (see below) |
| `extension/ast/{table,strikethrough,tasklist,definition_list,footnote}.go` | `src/extension/ast/<same>.rs` | payload structs (`CustomNode`), `KIND_*` statics, `new_*(ast, ..) -> NodeId` constructors |
| `extension/table.go` | `src/extension/table.rs` | paragraph transformer (200), escaped-pipe AST transformer (0), renderer (500), options |
| `extension/strikethrough.go` | `src/extension/strikethrough.rs` | inline parser (500) + delimiter processor, renderer (500) |
| `extension/linkify.go` | `src/extension/linkify.rs` | inline parser (999); `urlRegexp`/`wwwURLRegxp` as hand-written matchers |
| `extension/tasklist.go` | `src/extension/tasklist.rs` | inline parser (0), renderer (500) |
| `extension/definition_list.go` | `src/extension/definition_list.rs` | block parsers (101, 102), renderer (500) |
| `extension/footnote.go` | `src/extension/footnote.rs` | block parser (999), inline parser (101), AST transformer (999), renderer (500), options |
| `extension/typographer.go` | `src/extension/typographer.rs` | inline parser (9999) |
| `extension/gfm.go`, `extension/cjk.go` | `src/extension/gfm.rs`, `cjk.rs` | |
| Go runtime slice growth (`growslice`, `roundupsize`, size classes) | `src/goslice.rs` (+ `goslice_tables.rs`) | Go slice semantics for the parser's opened-block list |

Every ported function carries a `// Go: <path>:<Func>` comment.

## Public API summary (for downstream crates)

```rust
// Build (Go: goldmark.New(goldmark.WithExtensions(...), goldmark.WithParserOptions(...), goldmark.WithRendererOptions(...)))
let md = goldmark::new(vec![
    goldmark::with_extensions(vec![Box::new(MyExt)]),
    goldmark::with_parser_options(vec![goldmark::parser::with_attribute(), Box::new(goldmark::parser::with_auto_heading_id())]),
    goldmark::with_renderer_options(vec![Box::new(goldmark::renderer::html::with_unsafe())]),
]);
// Convert
let mut out: Vec<u8> = Vec::new();            // any util::BufWriter
md.convert(src, &mut out)?;
// Parse and render separately, with a caller context (Hugo)
let mut pc = goldmark::parser::new_context(vec![goldmark::parser::with_ids(Box::new(my_ids))]);
pc.set_value(MY_KEY, true);
let doc = md.parse(src, &mut pc);             // ParseResult { ast, root }
md.renderer().render(&mut my_writer, src, &doc.ast, doc.root)?;
```

- `goldmark::Markdown` is `Send + Sync` (one instance can serve many pages,
  as in Hugo). Parser and renderer tables are built lazily on first use (Go's
  `sync.Once`); options added after that are ignored (Go would crash).
- `ast::Ast` is the node arena; nodes are `ast::NodeId`. All `ast.Node`
  methods are `Ast` methods taking the id: `parent`, `first_child`,
  `next_sibling`, `append_child(parent, child)`, `insert_before/after`,
  `replace_child`, `remove_child`, `child_count` (Go's counter, including its
  InsertBefore over-counting), `lines`/`lines_mut`/`set_lines`,
  `has_blank_previous_lines`, `attributes`/`set_attribute`/`attribute`,
  `is_raw`, `text` (deprecated `Node.Text`), `dump`. Typed access:
  `heading(n)`, `list(n)`, `link(n)`, `image(n)`, `text_node(n)`,
  `string_node(n)`, `auto_link(n)`, `raw_html(n)`, `html_block(n)`,
  `fenced_code_block(n)` (+ `_mut`), or `ast.value(n)` (`NodeValue`).
- `ast::walk(&mut Ast, root, &mut |ast, n, entering| -> Result<WalkStatus>)`
  mutates while walking with Go's semantics (first child read after the
  entering callback, next sibling after the child's walk); `walk_ref` is the
  read-only variant.
- `text::Reader<'a>` (object safe; lines borrow the source, so a line can be
  held while advancing). `text::new_reader`, `text::new_block_reader`.
- `util::*`: every exported goldmark util function (snake_case).
- `renderer::html`: `Config` (with `set_option`, reusable by other renderers
  like Go's embedded `html.Config`), `Writer` trait + `DEFAULT_WRITER`,
  `new_writer`, `render_attributes`, the `*_ATTRIBUTE_FILTER` statics,
  `is_dangerous_url`, `bind` (method → `NodeRendererFunc`).

## Plugin (extension) API

Everything an extension does in Go maps 1:1:

| Go | Rust |
|---|---|
| `type ext struct{}; func (e *ext) Extend(m goldmark.Markdown)` | `impl goldmark::Extender for Ext { fn extend(&self, m: &mut Markdown) }` |
| `m.Parser().AddOptions(parser.WithBlockParsers(util.Prioritized(p, 100)))` | `m.parser().add_options(vec![parser::with_block_parsers(vec![util::prioritized(Box::new(p) as Box<dyn BlockParser>, 100)])])` |
| `parser.WithInlineParsers`, `WithParagraphTransformers`, `WithASTTransformers`, `WithOption`, `WithAttribute`, `WithEscapedSpace` | same names, snake_case |
| `m.Renderer().AddOptions(renderer.WithNodeRenderers(util.Prioritized(r, 500)))` | `m.renderer().add_options(vec![renderer::with_node_renderers(vec![util::prioritized(Box::new(r) as Box<dyn NodeRenderer>, 500)])])` |
| `var KindX = ast.NewNodeKind("X")` | `static KIND_X: LazyLock<NodeKind> = LazyLock::new(|| ast::new_node_kind("X"));` |
| `type X struct { ast.BaseInline; F int }` | `#[derive(Debug)] struct X { f: i64 }` + `impl ast::CustomNode for X` (`as_any`, `as_any_mut`; optional `is_raw`, `text`, `soft_line_break`, `dump_fields`) and `ast.new_custom_node(*KIND_X, NodeType::Inline, Box::new(X{..}))`; read back with `ast.custom::<X>(n)` |
| `BlockParser` (`Trigger`, `Open`, `Continue`, `Close`, `CanInterruptParagraph`, `CanAcceptIndentedLine`) | `parser::BlockParser` (`trigger() -> Option<&[u8]>` — `None` = Go's nil trigger, i.e. a free parser appended to every trigger list — `open`, `continue_`, `close`, ...) |
| `InlineParser` (`Trigger`, `Parse`) + optional `CloseBlocker` | `parser::InlineParser` (`trigger`, `parse`, `is_close_blocker() -> true` + `close_block`) |
| `ParagraphTransformer`, `ASTTransformer` | `parser::ParagraphTransformer`, `parser::AstTransformer` |
| `SetOptioner.SetOption(name, value)` | `set_option(&mut self, name, &OptionValue)` default no-op method on each trait (Go calls it only on types implementing SetOptioner; a no-op is equivalent) |
| `DelimiterProcessor` + `parser.ScanDelimiter` + `pc.PushDelimiter` + `parser.ProcessDelimiters` | `parser::DelimiterProcessor` (`on_match(&self, ast, consumes) -> NodeId`), `scan_delimiter` returns a `Delimiter` value, `ast.new_delimiter(d)` puts it in the arena, `pc.push_delimiter(ast, id)`, `process_delimiters(ast, DelimiterBottom::Nil, pc)` |
| `NodeRenderer.RegisterFuncs(reg)` + `reg.Register(kind, r.renderX)` | `fn register_funcs(self: Arc<Self>, reg)` + `reg.register(kind, html::bind(&self, Self::render_x))` or any `Arc<dyn Fn(&mut dyn BufWriter, &[u8], &Ast, NodeId, bool) -> Result<WalkStatus, Error>>` |
| `w.(*render.Context)` in a renderer | `w.as_any_mut().downcast_mut::<Ctx>()` (implement `util::BufWriter` for the context type) |
| `parser.NewContextKey()`, `pc.Get/Set/ComputeIfAbsent` | `parser::new_context_key()` (in a `LazyLock`), `pc.get`/`get_as::<T>`/`get_as_mut`/`set(key, Option<Box<..>>)`/`set_value`/`compute_if_absent` |
| `pc.IDs()`, `parser.WithIDs(ids)` | `pc.ids()`, `parser::with_ids(Box<dyn IDs>)` |

Registration semantics are Go's exactly: block/inline parsers,
transformers and renderers are sorted by priority with Go's pdqsort
(`go-sort`, so ties keep Go's order); parsers are dispatched by trigger byte
in priority order (free block parsers appended after the triggered ones);
node renderers are registered from the highest priority number to the lowest
so the lowest number wins for a kind; renderer and parser options reach every
component's `set_option` before it is frozen. `tests/plugin_api.rs` is a
worked example (a port of `extension.Strikethrough` plus a custom
inline/block/transformer/renderer extension mirroring Hugo's hugocontext),
verified byte-for-byte against the same extension in Go.

## Extensions API (Go `extension` package)

| Go | Rust (`goldmark::extension::*`) |
|---|---|
| `extension.Table`, `Strikethrough`, `Linkify`, `TaskList`, `DefinitionList`, `Footnote`, `Typographer`, `GFM`, `CJK` | `table()`, `strikethrough()`, `linkify()`, `task_list()`, `definition_list()`, `footnote()`, `typographer()`, `gfm()`, `cjk()` → `Box<dyn Extender>` |
| `NewTable(opts...)`, `NewLinkify`, `NewFootnote`, `NewTypographer`, `NewCJK` | `new_table(Vec<TableOption>)`, `new_linkify(Vec<LinkifyOption>)`, `new_footnote(Vec<FootnoteOption>)`, `new_typographer(Vec<TypographerOption>)`, `new_cjk(Vec<CJKOption>)` |
| `WithTableCellAlignMethod(TableCellAlignStyle)`, `WithTableHTMLOptions(html.WithXHTML())` | `with_table_cell_align_method(TableCellAlignMethod::Style)`, `with_table_html_options(vec![Arc::new(html::with_xhtml())])` |
| `WithLinkifyAllowedProtocols([]string{"ssh:"})`, `WithLinkifyURLRegexp(re)`, `...WWWRegexp`, `...EmailRegexp` | `with_linkify_allowed_protocols(&["ssh:"])`, `with_linkify_url_regexp(Arc<dyn LinkifyRegexp>)` (any `Fn(&[u8]) -> Option<(start, end)>` is a `LinkifyRegexp`: Go's `FindSubmatchIndex(b)[0:2]`) |
| `WithFootnoteIDPrefix("p-")`, `...IDPrefixFunction(func(ast.Node) []byte)`, `...LinkTitle`, `...BacklinkTitle`, `...LinkClass`, `...BacklinkClass`, `...BacklinkHTML`, `...HTMLOptions` | `with_footnote_id_prefix("p-")`, `with_footnote_id_prefix_function(Arc<dyn Fn(&Ast, NodeId) -> Option<Vec<u8>>>)`, ... (`impl AsRef<[u8]>` for Go's `[]byte \| string` generics) |
| `WithTypographicSubstitutions(map[TypographicPunctuation]T{...})` | `with_typographic_substitutions(&[(TypographicPunctuation::EnDash, "&ndash;"), ...])` (pairs; keys are distinct so Go's map order is irrelevant; an empty string stays a non-nil substitution) |
| `WithEastAsianLineBreaks(style...)`, `WithEscapedSpace()` (CJK) | `with_east_asian_line_breaks(&[EastAsianLineBreaks::Css3Draft])` (`&[]` = default Simple), `with_escaped_space()` |
| `NewStrikethroughHTMLRenderer()` etc. (Hugo's TOC uses it) | `new_strikethrough_html_renderer(vec![])`, `new_table_html_renderer(&[])`, `new_footnote_html_renderer(&[])`, `new_task_check_box_html_renderer(vec![])`, `new_definition_list_html_renderer(vec![])`; parsers/transformers: `new_*_parser()`, `new_table_paragraph_transformer()`, `new_table_ast_transformer()`, `new_footnote_ast_transformer()` |
| `extension/ast`: `KindTable`, `*ast.TableCell`, `Alignment.String()`, `KindDefinitionTerm`, `KindStrikethrough`, ... | `extension::ast::KIND_TABLE` (deref the `LazyLock`), `ast.custom::<extension::ast::TableCell>(n)` (or `extension::ast::table_cell(&ast, n)`), `Alignment::as_str()`, `KIND_DEFINITION_TERM`, `KIND_STRIKETHROUGH`, ... |
| Attribute filters (`TableAttributeFilter`, `StrikethroughAttributeFilter`, ...) | `TABLE_ATTRIBUTE_FILTER`, `STRIKETHROUGH_ATTRIBUTE_FILTER`, ... |

Neohugo's extension set for seeksnack (`markup/goldmark/convert.go`) is, in
order: `table()`, `strikethrough()`, `linkify()`, `task_list()`,
`new_typographer(vec![with_typographic_substitutions(<hugo config>)])`,
`definition_list()`, `footnote()`, with `parser::with_attribute()` and
`html::with_unsafe()` (see `tests/common/mod.rs:hugo_extensions`).

### Go behaviour reproduced on purpose (bug-for-bug)

- **No `CloseBlock` for extension parsers.** strikethrough/linkify/tasklist/
  typographer declare `CloseBlock(parent, pc)`, which does not satisfy
  `parser.CloseBlocker` (`CloseBlock(parent, block text.Reader, pc)`), so Go
  never calls them. In particular the typographer's unclosed single/double
  quote counters are **never reset per block**: they live for the whole
  document (one `parser.Context`, blocks parsed in `walkBlock` post-order).
  Seeksnack's own front-matter-as-markdown renders depend on it (a `""` after
  an earlier unclosed `"` becomes `&quot;&rdquo;`). The Rust parsers are not
  close blockers either (`is_close_blocker()` = false); the reset method is
  kept, uncalled. Test: `typographer_counters_are_not_reset_per_block`.
- Typographer operator precedence: the "plural possessive" check is
  `(len>1 && IsSpace(line[0])) || (IsPunct(line[0]) && len>2 && !IsDigit(line[1]))`
  and `maybeClose` ends with `len==2 || ((len>2 && IsPunct(line[2])) || IsSpace(line[2]))`;
  written with the same (identical in Rust) precedence. The typographer's
  delimiter is only scanned for flanking, never pushed.
- Definition descriptions: `Close` converts only the **first** paragraph of a
  tight description to a TextBlock (the loop reads `NextSibling` of the
  replaced, now detached, paragraph).
- Definition list "not first item": the existing list node is returned from
  `Open` again and `AppendChild` moves it after the term paragraph.
- Footnote inline parser: for `!` it checks `line[2] == '^'` without checking
  `line[1] == '['` (`!x^1]` is a footnote ref when `1` is defined) and appends
  a `!` Text before the link; `Advance` happens before the "no footnote list"
  return (undone by the caller's position reset).
- Nested footnote definitions (`[^1]: a` / `    [^2]: b`) produce a detached
  list ↔ footnote cycle during parsing, so their contents are never inline-
  parsed; the AST transformer then re-attaches the list to the document.
- Table escaped pipes: the positions are matched against the segment of the
  *original* code-span text (`ts := &c.Segment`) while splitting the latest
  piece; escaped cells of rejected header rows stay in the list.
- Table rows beyond the header's column count are cut; missing cells are
  filled with empty cells; `Transform` re-reads `lines.Len()` each iteration
  (the paragraph's lines are shortened by `SetSliced`).
- Linkify: `m[1] -= m[1] - i` entity trimming, `line[i]` with `i == -1`
  panics like Go (only reachable with custom regexps), the `)` balance and
  trailing `?!.,:*_~` trimming, email `.`/`-`/`_` rules.

## Deliberate deviations (none changes output)

1. **Arena AST.** Nodes are arena slots addressed by `NodeId` (never reused,
   so id equality is Go pointer identity). Node methods live on `Ast`.
   `AutoLink.value` (a detached `*Text`) is stored inline as a `Text` value;
   `FencedCodeBlock.Info` stays a detached Text node id. `FencedCodeBlock.Language`
   is recomputed instead of cached in the node (same value).
   `OwnerDocument` returns `None` where Go would nil-deref.
2. **Regular expressions** (`regexp` is not an allowed crate and Go's RE2
   semantics would need care anyway): the html-block patterns (types 1–7),
   `openTagRegexp`/`closeTagRegexp` and `emailDomainRegexp` are hand-written
   matchers in `parser/regexps.rs` / `util::email_domain_regexp_match_end`
   implementing the exact patterns, including Go's `(?i)` simple-fold
   matching (`ſ` matches `s`), Perl `\s` = `[\t\n\f\r ]`, `.` excluding `\n`,
   leftmost-first capture/length selection and, for the tag patterns, reading
   through the `Reader` as an `io.RuneReader` (stopping at U+FFFD / invalid
   UTF-8 like `readRuneReader`). `Reader.Match(reg)` becomes
   `Reader::match_with(matcher)`; `FindSubMatch` is not ported (unused by
   goldmark and Hugo). Verified against Go's `regexp` on 6065 inputs ×
   12 results (`regex.gmf.gz`), including reader positions after a match.
3. **`parser.Context`** is a concrete struct (goldmark's `parseContext`);
   custom Context implementations are not supported (Hugo only wraps it),
   custom `IDs` are. Values are `Box<dyn Any + Send + Sync>`.
   `References()` returns references sorted by key instead of map order.
4. **`interface{}` values**: attribute values are `ast::AttrValue`
   (`Bytes`, `String`, `Float`, `Bool`, `Nil`, `Array`, `Attributes`, `Other`);
   option values are `Arc<dyn Any + Send + Sync>`; `ProcessDelimiters`'s
   `bottom ast.Node` is `DelimiterBottom` because goldmark relies on the
   difference between a nil interface and an interface holding a nil
   `*Delimiter` (`pushLinkBottom` when no delimiter exists).
5. **Go slice aliasing.** The parser's opened-block list is a `GoSlice`
   emulating Go slice headers over a shared backing array with Go 1.27's
   growth (`nextslicecap` + `roundupsize` + size classes; checked against Go:
   caps 1,2,4,8,16,35,71,151,303 for 32-byte `Block`s). goldmark depends on
   it (`parseBlocks` detects a transformed paragraph through the stale header,
   parser.go:1105 — removing it breaks spec examples). Elsewhere the aliasing
   is unobservable and plain `Vec`s are used: `SetLines` copies (the source
   node is always detached afterwards), `Segments.Sliced` copies (the splice
   result is identical), and `BytesFilter.Extend` copies the slots (Go shares
   slot slices; every slot of `GlobalAttributeFilter` has len == cap, so no
   extension can write into another's slot — checked by the
   `attribute_filters` test over all filter names).
6. **Text readers**: `TextReader` keeps Go's `peekedLine` cache semantics
   (the cache survives `SetPosition`, and `Advance`/`AdvanceToEOL` use its
   length). With `match_with` the matcher reads only as far as it needs,
   whereas Go's regexp engine may read one extra rune, which can leave a
   different stale cache in a `TextReader` (never in a `BlockReader`, which
   has no cache — goldmark only matches on block readers).
7. **`Dump`** returns a `String` (Go prints to stdout) and prints map entries
   sorted (Go's map order is random).
8. **Errors**: renderer functions and walkers return `Result<_, goldmark::Error>`
   (`Box<dyn Error + Send + Sync>`). `BufWriter` writes are infallible
   (goldmark discards write errors; only `Flush` is reported).
9. **Panics are mirrored** where Go panics on input the port reproduces
   (e.g. `# h {id=5}` with `WithAttribute()+WithAutoHeadingID()` panics in Go
   with "interface conversion: float64, not []uint8"; the fixtures record Go
   panics and the tests require a Rust panic on the same inputs).
10. **No recursion on document depth**: `ast.Walk`, `walkBlock`,
    `containsLink` and html `renderTexts` use explicit stacks with the same
    visiting order and the same FirstChild/NextSibling read points (Go
    recurses on growable goroutine stacks; a Rust thread stack would overflow
    on e.g. 20k nested blockquotes — `deep_nesting_on_a_small_stack` renders
    such input on a 2 MiB stack). `Ast::text` and `dump` still recurse.
11. `util.StringToReadOnlyBytes`/`BytesToReadOnlyString` and
    `PrioritizedSlice.Remove` are not ported (no Rust use).
12. **Extension regular expressions** are hand-written matchers with Go's
    leftmost-first semantics: `urlRegexp` and `wwwURLRegxp` (linkify:
    greedy `{1,256}` host run backtracking to the last `.` followed by
    `[a-z]`, then maximal `[a-z]+`, optional `:\d+` (URL only), optional
    `[/#?]` path; everything after the TLD is optional so the first success
    is final), `taskListRegexp` (`^\[([\sxX])\]\s*`, Perl `\s` =
    `[\t\n\f\r ]`) and the four table delimiter patterns (`$` = end of
    text). All classes are ASCII, so byte matching equals Go's rune matching
    (non-ASCII and invalid UTF-8 match nothing). Verified against Go's
    `regexp` on 10,027 inputs (`ext-regex.gmf.gz`, unit test
    `extension::regex_vectors`). Custom linkify patterns are any
    `LinkifyRegexp` implementation (Go takes a `*regexp.Regexp`).
13. **Table cell alignment style.** Go's `renderTableCell` (TableCellAlignStyle,
    the HTML5 default) *mutates the node* (`SetAttributeString("style", ...)`)
    and then renders its attributes. The Rust renderer has a shared `&Ast`,
    so it computes the same attribute list locally (existing `style` value +
    `;` + `text-align:X` in place, else appended) and renders it with
    `html::render_attribute_list`. Output is identical for a render; the AST
    is not modified, so rendering the same AST twice does not double the
    style as Go would (goldmark never re-renders a document; Hugo replaces
    the table renderer). A non-`[]byte` `style` value panics like Go's type
    assertion.
14. Options are enums/structs instead of Go's option interfaces
    (`TableOption`, `LinkifyOption`, `FootnoteOption`, `TypographerOption`);
    each is still both a parser/renderer option (`set_parser_option` /
    `set_config` store the same option names — `LinkifyURLRegexp`,
    `FootnoteIDPrefix`, `TableTableCellAlignMethod`, ... — so passing them via
    `WithParserOptions`/`WithRendererOptions` reaches `set_option` as in Go)
    and an extension option. HTML options inside `WithTableHTMLOptions` /
    `WithFootnoteHTMLOptions` are `Arc<dyn html::HtmlRendererOption>` (all
    goldmark html options implement it; they now derive `Clone`).
15. Extension renderers embed `html::Config` as a `config` field and forward
    `set_option` to it (Go's promoted `SetOption`), so `WithXHTML()` etc.
    reach the tasklist/footnote/table renderers exactly as in Go.
16. Extension context values: the footnote list is stored as its `NodeId`,
    the footnote-link list as `Vec<NodeId>`, the escaped-pipe list as a `Vec`
    updated by index (Go keeps pointers), the typographer counter as a
    struct.
17. `Document::get_meta` / `Ast::document` were added for read access to
    document metadata from renderers (Go's `n.OwnerDocument().Meta()`,
    used by footnote ID prefix functions).

## FMA sites

None. The only floating-point arithmetic is `float64(sign) * f` in
`parseAttributeNumber` (a plain multiply, no fused form).

## Tests and parity evidence

All fixtures come from the Go oracle `tools/go-oracle/goldmark` (package main
inside the neohugo module, so it links goldmark v1.7.12 exactly):

```
cd <repo> && GOLDMARK_DIR=$(go env GOMODCACHE)/github.com/yuin/goldmark@v1.7.12 \
  go run ./tools/go-oracle/goldmark fixtures crates/goldmark/tests/fixtures
go run ./tools/go-oracle/goldmark corpus <seeksnack>/content crates/goldmark/tests/fixtures/corpus.gmf.gz default,unsafe,all
go run ./tools/go-oracle/goldmark gentables crates/goldmark/src/util      # entity + case folding tables
go run ./tools/go-oracle/goldmark fuzz -mode tokens|bytes|deep|plugin -n N -seed S > big.gmf
GOLDMARK_DIR=... go run ./tools/go-oracle/goldmark extfixtures crates/goldmark/tests/fixtures <seeksnack>/content   # ext.gmf, ext-edge, ext-fuzz, corpus-ext
go run ./tools/go-oracle/goldmark extregex crates/goldmark/tests/fixtures/ext-regex.gmf.gz 10000
go run ./tools/go-oracle/goldmark fuzz -mode ext|extbytes -cfg hugo,x-all,... -n N -seed S > big.gmf
```

Fixture format (GMF): `=== name\n` then `key <len>\n<raw bytes>\n` fields —
raw bytes so inputs can hold invalid UTF-8, NUL and CR.

| test | content | result |
|---|---|---|
| `oracle::spec_json` | 652 CommonMark examples (`_test/spec.json`) × configs xu (goldmark's TestSpec), default, attr; also asserts Go matches spec.json (TrimSpace, as TestSpec) | 1956/1956 byte-identical |
| `oracle::extra_txt`, `options_txt` | `_test/extra.txt` 68 cases (xu/default/all), `_test/options.txt` 7 cases (attr/default/all) | 204 + 21 identical |
| `oracle::edge_cases` | 149 hand-written quirk inputs × 9 configs (default, xu, attr, unsafe, hard, all, escspace, ea-simple, ea-css3) | 1341/1341 identical (incl. 2 Go panics) |
| `oracle::fuzz_fixed_seed` | 3000 grammar-fuzz docs | identical |
| `oracle::seeksnack_corpus` | 251 seeksnack `.md` bodies (front matter stripped with neohugo's pageparser) × default/unsafe/all + whole files × default | 1004/1004 identical |
| `plugin_api` | test extension + Strikethrough port vs Go: 149 edge inputs × 2 configs + 4000 plugin-fuzz docs; writer downcast | 4298/4298 identical |
| `util::util_vectors` | 1625 inputs × ~90 function results (URLEscape both modes, reference resolution incl. base-0 octal and range quirks, case folding, FindClosure, indent helpers, HTML writers, IsPunctRune/IsSpaceRune, ToRune...) | ~147k values identical |
| `util::ids_generate`, `attribute_filters`, `cjk_predicates` | default IDs sequence; all 11 filters × 90 names; 3 CJK predicates over every code point | identical |
| `regex::html_block_and_tag_regexps` | 6065 inputs × 12 pattern results + 3000 email domains | identical |
| `extension::extension_test_files` | goldmark's `extension/_test/*.txt` (table 13, strikethrough 5, linkify 19, tasklist 4, definition_list 6, footnote 5, typographer 19 cases) × (the _test.go config, `hugo`, `x-all`) + the 13 `DoTestCase` cases of the extension `_test.go` files (align methods, style transformer, footnote options/ID prefix function, linkify custom regexps, CJK escaped space); also asserts Go equals goldmark's expected output | 227/227 identical |
| `extension::extension_edge_cases` | 184 hand-written extension inputs (tables with missing/extra cells, escaped pipes in code spans, tables in containers/tabs, linkify trailing punctuation/parens/entities/ports/256-char hosts/emails, task lists, definition lists incl. tabs, footnotes incl. nested/undefined/repeated/`![^1]`/padding, typographer quotes/apostrophes/decades/counters) + the 149 core edge cases, × 26 extension configs | 8658/8658 identical |
| `extension::extension_fuzz_fixed_seed` | 5000 extension-grammar + 3000 extension byte-level + 2000 core-token docs over the 26 configs | 10000/10000 identical |
| `extension::seeksnack_corpus_hugo_extensions` | 251 seeksnack bodies × `hugo` (neohugo's goldmark extension set + WithAttribute + WithUnsafe) and `hugo-autoid` (+ WithAutoHeadingID), + the 251 whole files (front matter included) × `hugo` | 753/753 identical |
| cross-check with neohugo's own converter (scratch `work/goldmark-ext/cmp_nohooks.py`) | the `hugo` outputs above vs `work/markdown/corpus-nohooks` (neohugo `markup/goldmark` via the `mdref` harness), normalizing only Hugo-side rendering (heading ids, table-hook whitespace/style format, Hugo's `</p></blockquote>`) | 251/251 identical |
| `extension::regex_vectors` (unit) | 10,027 inputs × urlRegexp, wwwURLRegxp, taskListRegexp, 4 table delimiter regexps vs Go `regexp` | all identical |
| `extension::ast_block_node_text`, `ast_inline_node_text` | ports of `extension/ast_test.go` | pass |
| external (scratch, `goldmark fuzz -mode ext|extbytes|tokens|deep -cfg <26 configs>`) | 300k + 200k extension-grammar, 200k extension byte-level, 100k core-token, 30k deep-nesting docs; seeksnack corpus concatenated ×8 (6.3 MB) × hugo/hugo-autoid/x-all | 830,003/830,003 identical |
| `goldmark_tests` | ports of ast/ast_test.go, ast_test.go, extra_test.go (incl. the 5 performance tests) | pass |
| `oracle::external_fuzz`, `plugin_api::external_plugin_fuzz` (ignored; corpora in the scratch dir, regenerate with `goldmark fuzz`) | 300k token-grammar + 700k byte-level + 100k deep-nesting docs over 9 configs (1,100,000 records); the whole seeksnack corpus concatenated ×8 (6.3 MB) × 3 configs; 150k plugin-extension docs | all identical |

Mutation checks: disabling the opened-block aliasing emulation or the base-0
`ParseUint` quirk makes the suite fail; the InsertBefore child-count quirk,
the reader cache quirk, typed-nil delimiter bottoms and the linkLabelState
`First` bug are ported faithfully but are output-neutral for goldmark core
(no test can observe them without extensions).

## Known gaps

- `extension/ast` `Dump` output is approximate (Table prints its
  alignments as one key/value, not Go's nested block).
- `text.Reader.FindSubMatch` and regexp-based `Match` (see deviation 2).
- `Dump` formatting is approximate (debug only).
- A custom `parser.Context` implementation cannot be substituted.
