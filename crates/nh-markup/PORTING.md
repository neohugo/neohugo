# nh-markup — porting notes

neohugo markup/**: converter + hooks API, goldmark glue (render hooks, autoid, attributes, tables,
blockquotes, hugocontext, images, TOC), highlight config (+ stub highlighter), markup_config;
asciidoc/pandoc/rst/org stubs. Owner and crate lead: Wave B task T06 (markup).

## Go file → Rust module

| Rust module | Go source(s) | Status / note |
|---|---|---|
| `markup` | `markup/markup.go` | ported |
| `converter::converter` | `markup/converter/converter.go` | ported |
| `converter::hooks` | `markup/converter/hooks/hooks.go` | ported: hook traits, `RendererType`, `TableCell`/`Table`, `text.Position` object |
| `markup_config` | `markup/markup_config/config.go` | ported |
| `tableofcontents` | `markup/tableofcontents/tableofcontents.go` | ported |
| `highlight::config` | `markup/highlight/config.go` | ported (`toHTMLOptions` is Chroma: STUB) |
| `highlight::highlight` | `markup/highlight/highlight.go` | STUB highlighter: every highlighting call returns `neohugo-rs: highlighting (chroma) is not supported` |
| `highlight::chromalexers` | `markup/highlight/chromalexers/chromalexers.go` + chroma v2.19.0 `registry.go` (`Get`, `Match`) | ported for the existence check Hugo makes; data in `chromalexers_data.rs` (generated) |
| `internal::attributes` | `markup/internal/attributes/attributes.go` | ported |
| `goldmark::convert` | `markup/goldmark/convert.go` | ported (+ goldmark's `renderer.Render`, see deviation 1) |
| `goldmark::render_hooks` | `markup/goldmark/render_hooks.go` | ported |
| `goldmark::autoid` | `markup/goldmark/autoid.go` | ported; `github-ascii` uses `autoid_accents.rs` (generated, deviation 4) |
| `goldmark::toc` | `markup/goldmark/toc.go` | ported |
| `goldmark::goldmark_config` | `markup/goldmark/goldmark_config/config.go` | ported |
| `goldmark::blockquotes` | `markup/goldmark/blockquotes/blockquotes.go` | ported (alert regexp as a hand-written matcher) |
| `goldmark::codeblocks` | `markup/goldmark/codeblocks/render.go` | ported |
| `goldmark::hugocontext` | `markup/goldmark/hugocontext/hugocontext.go` | ported (`hugoCtxRe` as a hand-written matcher) |
| `goldmark::images` | `markup/goldmark/images/transform.go` | ported |
| `goldmark::tables` | `markup/goldmark/tables/tables.go` | ported |
| `goldmark::passthrough` | `markup/goldmark/passthrough/passthrough.go` | STUB: `passthrough.enable = true` is `neohugo-rs: the goldmark passthrough extension is not supported` |
| `goldmark::internal::extensions::attributes` | `markup/goldmark/internal/extensions/attributes/attributes.go` | ported |
| `goldmark::internal::render` | `markup/goldmark/internal/render/context.go` (+ `tpl.StripHTML`, deviation 5) | ported |
| `asciidocext` | `markup/asciidocext/convert.go`, `asciidocext_config/config.go` | STUB converter (`neohugo-rs: asciidoc (asciidoctor) is not supported`); the config is decoded |
| `pandoc` | `markup/pandoc/convert.go` | STUB (`neohugo-rs: pandoc is not supported`) |
| `rst` | `markup/rst/convert.go` | STUB (`neohugo-rs: reStructuredText (rst2html) is not supported`) |
| `org` | `markup/org/convert.go` | STUB (`neohugo-rs: org-mode (go-org) is not supported`) |
| `blackfriday` | `markup/blackfriday/anchors.go` | ported |

Every ported function carries a `// Go: <path>:<Func>` comment; every GO PORTING CHECKLIST entry
is `OK` except the stubs (marked `STUB`) and `attributesBlock.Dump` (debug output).

Other explicit errors (README rule 5): `enableEmoji` (goldmark-emoji) and the goldmark `extras`
(delete, insert, mark, subscript, superscript) make `GoldmarkProvider::new` fail with
`neohugo-rs: … is not supported`. The stub converters are registered like Go's, so
`markup.NewConverterProvider` works; only converting fails.

## Dependencies

- nh-*: nh-common, nh-config (mapstructure `decode`, `Provider`, `AllProvider`), nh-media
  (`ContentTypes`).
- Wave A: go-value, goldmark, go-html, go-unicode, go-strconv (`Atoi`, `ParseUint`, `Quote`,
  `FormatFloat`), go-path (`filepath.Match` for the Chroma lexer globs), gotemplate
  (html/template's `stripTags`, for `tpl.StripHTML`, deviation 5).
- crates.io: none. The two Go regexps (`blockQuoteAlertRe`, `hugoCtxRe`) are hand-written
  matchers.
- dev: `serde_json`, `flate2` (`rust_backend`, gunzip of fixtures), nh-langs (the types of the
  `AllProvider` stub in `tests/config.rs`).

## API notes for the Hugo layer

- `GoldmarkProvider::new(ProviderConfig)` (Go `goldmark.Provider.New`) and
  `GoldmarkProvider::new_from_config(markup config, enableEmoji, logger)`.
  `markup::new_converter_provider(cfg)` uses the default content types;
  `new_converter_provider_with_content_types(cfg, &ContentTypes)` takes the site's (Go reads
  `cfg.Conf.ContentTypes().(media.ContentTypes)`; `ContentTypesProvider` cannot be downcast).
- `ProviderConfig` gained `logger: Option<Arc<Logger>>` (Go's `Logger`) and its `highlighter` is an
  `Option` (Go's nil: `NewConverterProvider` creates one).
- The converter is also a `ParseRenderer` (`as_parse_renderer`): `parse` returns the document
  (`ParsedDoc`, a `goldmark::convert::ParsedDocument`) and the TOC (`None` = Go's nil when
  `RenderTOC` is false); `render` renders a parsed document (any number of times).
- Hooks get `HostCtx` (the caller's template context, unchanged, HUGO_LAYER.md §6.2) and the
  render buffer. A hook context is a `Value::Object`; its Go `%T` and exported method set are
  Go's exactly (checked by the hooks oracle): `goldmark.linkContext`, `goldmark.imageLinkContext`,
  `goldmark.headingContext`, `*blockquotes.blockquoteContext`, `*tables.tableContext`,
  `*codeblocks.codeBlockContext`; `.Position` is a `text.Position` object; `THead`/`TBody` are
  `[]hooks.TableRow` of `hooks.TableRow` of `hooks.TableCell` (fields `Text`, `Alignment`).
- `GetRendererFunc(RendererType, &Value)`: the id is `Invalid` (Go nil), the blockquote type
  (`"regular"`/`"alert"`) or the code block language, as a `Value::String`.
- Go's `hooks.ElementPositionResolver` is the `resolve_position(PositionerSourceTarget bytes)`
  default method of the `TableRenderer`, `BlockquoteRenderer`, `CodeBlockRenderer` and
  `PassthroughRenderer` traits (hugolib's hook renderers implement it).
- `Fragments::to_html_values(start, stop, ordered)` is Go's `ToHTML(startLevel, stopLevel any,
  …)` with `cast.ToIntE`.

## Deliberate deviations

1. **Render dispatch.** Go passes its `*render.Context` to goldmark's `Renderer.Render` as the
   `util.BufWriter` and Hugo's node renderers type-assert it back. The Rust render context
   borrows the caller's template context (`HostCtx<'a>`), which cannot pass through goldmark's
   `'static` node renderer functions. `goldmark::convert` therefore builds the node renderer
   table itself, exactly as goldmark's `renderer.Render` does: the same renderers (goldmark's
   HTML renderer at 1000, the extension renderers at 500, Hugo's at 100 and 50), in Go's
   registration order, sorted with goldmark's pdqsort (`util::sort_prioritized`), every renderer
   option (`WithHardWraps`, `WithXHTML`, `WithUnsafe`, the CJK extension's `EastAsianLineBreaks`
   and escaped-space writer) passed to every renderer's `SetOption`, registered from the highest
   priority number to the lowest; then it walks the document (goldmark's `walk_ref`) and calls
   goldmark's functions on the context's buffer and Hugo's on the context. The goldmark
   `Markdown` still does all the parsing (the Hugo extensions' parser parts are `Extender`s).
   The renderer-only Hugo extensions (`newLinks`, `blockquotes.New`, `codeblocks.New`,
   `tables.New`, the renderer half of `hugocontext.New`) are node renderer entries instead of
   `Extender`s.
2. **`filterInternalAttributes`** compacts the image node's attribute slice in place in Go; the
   port filters a copy. Only observable if the same parsed document is rendered twice with a
   non-internal image attribute (paragraph attributes moved to a block image); Hugo parses per
   output format.
3. **Hook traits** take the template context as `HostCtx` and write into `&mut Vec<u8>` (the render
   buffer). A hook error travels through goldmark's walk as a `goldmark::Error` and comes back as
   the original `herrors::Error`; table, blockquote and code block errors are wrapped with
   `herrors::new_file_error_from_pos` (Go's `NewFileErrorFromPos`), whose text matches Go's
   (`"file:line:col": message`).
4. **`text.RemoveAccents`** (x/text NFD, remove `unicode.Mn`, NFC) is needed by the `github-ascii`
   auto IDs only; nh-common's was a stub when this was ported (it is now a full x/text port,
   `nh_common::text::remove_accents`, and swapping it in passes the autoid oracle — a possible
   cleanup that would retire `autoid_accents.rs`). `sanitizeAnchorNameWithHook` only sees
   its result through `bytes.TrimSpace` and single-byte runes, so the port applies
   `RemoveAccents(string(r))` per rune from a generated table (`autoid_accents.rs`: the 509
   non-Mn runes whose result differs in its ASCII bytes or the space-ness of its runes; Mn
   runes are removed with go-unicode's table). `tools/go-oracle/nh-markup/gentables` checks that
   the per-rune model is exact (every Mn rune maps to "", NFC never merges an ASCII byte with a
   following rune). Non-ASCII output runes may differ from Go's; they are dropped anyway.
5. **`tpl.StripHTML`** (used by `render.TextPlain` for raw HTML) is ported privately in
   `goldmark::internal::render` over gotemplate's `strip_tags`; nh-tpl's `strip_html` (T13) is
   not ported yet.
6. **Chroma lexers.** `chromalexers.Get(lang) != nil` decides whether code block attributes are
   Chroma options. The port keeps chroma v2.19.0's lexer names, aliases (as registered and
   lower-cased) and file name globs (generated, `chromalexers_data.rs`) and ports
   `LexerRegistry.Get`/`Match` for existence only (no lexer objects, no cache).
7. **Map order.** `normalizeHighlightOptions` re-inserts lower-cased keys while ranging over the
   map; with two keys that fold to one, Go keeps a random one, the port the last in byte order.
   `idFactory.StringValues` returns the keys sorted (Go: map order; its caller sorts).
8. **`markup_config.Decode`.** Go's `GetStringMap("markup")` is nil when the value is missing, not
   convertible, or a nil `maps.Params`, and Decode then returns the defaults without
   `ApplyLegacyConfig` (legacy `pygments*` keys are ignored without a `[markup]` section);
   nh-config's `get_string_map` returns an empty map, so the port converts with
   `maps::to_string_map_e` itself. `normalizeConfig` mutates the provider's own maps in Go (they
   are shared); the port mutates its copy. Go's `markup_config.Default` shares the
   `AsciidocExt.Attributes` map with every decoded config, so a decode with asciidoc attributes
   leaks them into every later decode in the process; the port does not reproduce that (config
   oracle: that case is decoded last).
9. **Config structs** follow Go's field layout (`Parser.Attribute`, `Extras.Delete.Enable`,
   `Passthrough.Delimiters`, `RenderHook.EnableDefault` as `Option<Box<bool>>` for Go's `*bool`);
   `Highlight.HL_lines_parsed` is a `Vec` (Go's nil and empty are equal; only `toHTMLOptions`,
   a stub, tells them apart). The decode types' Go names are used in mapstructure error texts
   only (`ImageRenderHook`/`LinkRenderHook` share `goldmark_config.ImageRenderHook`, the extras
   share `goldmark_config.Delete`).
10. **TOC** headings are `Arc<Heading>`, mutated with `Arc::make_mut` while the builder owns them;
    `id`/`title`/identifiers are `GoString` (Go strings may hold invalid UTF-8).
11. **Logging.** The hugocontext renderer logs the page with `%q` in Go; the port quotes its
    `String()` or type name (log text only). Without a logger nothing is logged.
12. **`render.Context` values** are typed `Box<dyn Any + Send>` per node kind; `PeekValue` returns
    the stored value mutably (Go returns the stored pointer).
13. **No recursion on the document depth**: `render.TextPlain`'s first-child recursion is a loop,
    `renderTexts` uses an explicit stack, walks are goldmark's iterative walks
    (`tests/deep.rs` renders 3,000-deep blockquotes/lists/links on a 2 MiB stack).

## Go behaviour reproduced on purpose

- `render.TextPlain` follows only the first child of non-text inline nodes (`## **a *b* c** d`
  → `a--d`); entities are resolved with goldmark's `ResolveEntityNames` (named only).
- Blockquotes without a hook: `TrimSpace`d inner HTML and no newline before `</blockquote>`;
  with attributes no newline after `<blockquote …>`.
- Alerts: `strings.Cut` at the first newline, `<p>` re-added when the first line does not end
  with `</p>`; the title is `TrimSpace`d and loses a trailing `</p>`; NBSP is not consumed by
  `[^\S\r\n]` (Go's `\s` is ASCII).
- Headings without a hook render all attributes with `cast.ToString` values (`num=3.5`,
  `flag=true`, arrays and nulls as empty strings), dropping `on*` attributes.
- Linkify `www.` links get `LinkifyProtocol`; email autolinks get `mailto:` in the hook's
  destination; autolink `.Text` is the raw label (not escaped).
- `TOC`: a heading without an `id` has level 0; rows start at level 1 headings or the first
  heading; `HeadingsMap` keeps the last heading of a duplicate ID; `Identifiers` include the
  duplicates recorded by the ID factory (explicit `{#id}` too).
- Go panics reproduced (the Go oracle records them; `tests/hooks.rs` requires the same
  message): `{id=5}` on a heading (`interface conversion: interface {} is float64, not
  []uint8`, attributes transformer and TOC), attribute values that are neither bool, float,
  array nor string reaching `attributes.New` (`not implemented: <nil>`), a missing table hook
  (`table hook renderer not found`), a nil `GetRenderer`.
- Errors reproduced: `no code renderer found for "lang"`, `failed to parse Markdown attributes;
  you may need to quote the values`, `markup: Configured defaultMarkdownHandler "x" not found.`
  (+ the Blackfriday hint), mapstructure's decode errors, `invalid Highlight option: x`.

## Verification

Go oracles in `tools/go-oracle/nh-markup/` (package `main` in the neohugo module, calling
neohugo's real `markup` packages; shared code in `mdoracle`) write
`crates/nh-markup/tests/fixtures/<topic>/<topic>.json.gz`. `cargo test` needs neither Go nor the
network.

Corpus (`mdoracle.LoadCorpus`): the 959 `.md` files of `docs/content`, the 2 of
`hugolib/testsite`, the 12 of `docs/rust-port` (front matter stripped with neohugo's
pageparser, as Hugo does), and 22 adversarial documents (995 in all) (`mdoracle/adversarial.go`: Thai and
mixed-script headings, duplicates, entities and the first-child quirk, heading and block
attributes, tables with alignment and in containers, blockquote alerts, images with and
without titles and with attributes, code fences with attributes, raw HTML, typographer,
linkify, strikethrough, footnotes, definition lists, task lists, 14-level nested lists, links,
TOC level shapes, CRLF, invalid UTF-8, shortcode-like text, `{{__hugo_ctx}}` markers). Configs
(`mdoracle.Configs`): neohugo's defaults (`unsafe = false`), the seeksnack `[markup]` section
with its legacy `pygments*` keys, `github-ascii` + block attributes + unwrapped images + TOC
levels 1–4 ordered, `blackfriday` + XHTML + hard wraps + typographer off + `http` linkify
protocol + TOC 3/-1, CJK (east-asian line breaks css3draft, escaped space) with most extensions
off and custom typographer substitutions, and no-auto-ID + block attributes + term IDs.

| topic | content | Rust test | result |
|---|---|---|---|
| `convert` | every document × 6 configs, no link/image/heading/blockquote hooks (Go replicas of the embedded `render-table.html` and of a code block hook, as Hugo always has both), `RenderTOC` on: HTML + TOC (headings tree, identifiers, headings map, `ToHTML` for the config's levels and 4 fixed level pairs); the decoded config dumps; 7 deeply nested documents | `tests/convert.rs`, `tests/deep.rs` (2 MiB stack) | 5,970/5,970 HTML identical, 5,970/5,970 TOCs identical, 6/6 config dumps, 7/7 deep |
| `hooks` | recording renderers for link, image, heading, blockquote, table and code block hooks that dump every context (`%T`, method set, Destination, Title, Text, PlainText, IsBlock, Ordinal, Anchor, Level, Page, PageInner, Attributes, Options, AttributesSlice, OptionsSlice, Position via an ElementPositionResolver with the PositionerSourceTarget, THead/TBody cells with Alignment, blockquote Type/AlertType/AlertTitle/AlertSign, code block Type/Inner) and write markers into the output; the corpus × seeksnack, `docs/rust-port` + adversarial × ascii, adversarial × all configs, each adversarial document also wrapped in `hugocontext.Wrap` (pid 7 → `inner:7`, pid 9 → nil lookup); 9 failing documents (hook errors, missing code block renderer, bad attributes, Go panics) | `tests/hooks.rs` | 1,357 conversions: 1,287 HTML identical + 70 errors/panics identical; 11,313 hook calls, every dump identical |
| `autoid` | `SanitizeAnchorName` for github, github-ascii and blackfriday over a Unicode sweep (every rune below U+3400, every 13th above, in 48-rune chunks with rotating separators and Unicode spaces) and 4,000 seeded random strings (letters of many scripts, combining marks, compatibility characters, spaces, invalid UTF-8) | `tests/autoid.rs` | 18,117/18,117 identical |
| `config` | `markup_config.Decode` of 30 TOML configs (typographer/attribute bools, legacy keys, weak decoding, bad types, `_merge`, asciidoc, passthrough, render hooks), `NewConverterProvider` with 5 handlers (`Get`/`IsGoldmark` of 19 names, the error), `ResolveMarkup`, `chromalexers.Get` of 156 names | `tests/config.rs` | all identical (dumps and error texts) |

Go's `_test.go` tables are ported as unit tests: `autoid_test.go`, `blockquotes_test.go`,
`hugocontext_test.go`, `highlight/config_test.go`, `tableofcontents_test.go` (in the modules),
`markup_test.go` (`tests/config.rs`). `tests/units.rs` checks Parse + Render (twice) against
Convert, the explicit unsupported errors and the table values.

A mutation check (a space after `<blockquote>` in the default renderer) fails 1,228 of the 5,970
`convert` records.

Regenerate (every fixture and generated table regenerates byte for byte; nothing here depends
on the platform's floating point):

```sh
export GOTOOLCHAIN=go1.27.1
go run ./tools/go-oracle/nh-markup/convert
go run ./tools/go-oracle/nh-markup/hooks
go run ./tools/go-oracle/nh-markup/autoid
go run ./tools/go-oracle/nh-markup/config
go run ./tools/go-oracle/nh-markup/gentables   # src/goldmark/autoid_accents.rs, src/highlight/chromalexers_data.rs (~5 min)
```

## Known gaps

- Chroma highlighting, asciidoc, pandoc, rst, org, passthrough, goldmark-emoji and the goldmark
  extras are explicit errors (above).
- The TOC transformer renders heading inline nodes with goldmark's default renderer, as Go does;
  a `{{__hugo_ctx}}` node inside emphasis inside a heading would make goldmark's renderer index
  its function table by a kind it never registered, which panics in Rust (index out of range)
  and in Go only when the kind number exceeds the table (kind numbers are allocated in a
  different order). `.RenderShortcodes` never puts the markers there.
- The seeksnack content itself is private; the substitute corpus above was used instead.

## Requests to other crates

- nh-common (T02): `text::remove_accents` could use the approach of deviation 4 for autoid only;
  a full port needs NFD/NFC tables of Unicode 15.0.0 (x/text v0.26.0).
- nh-tpl (T13): `strip_html` is ported here privately (deviation 5); nh-tpl can reuse or replace
  it.
- nh-hugolib (T22): implement the hook traits (with `resolve_position` for the base-context
  hooks), build `ProviderConfig` with the logger, and call
  `new_converter_provider_with_content_types` with the site's content types. The ParsedDoc of
  `parse` may be rendered several times.
