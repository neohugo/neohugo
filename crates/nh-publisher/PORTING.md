# nh-publisher — porting notes

neohugo `publisher/*` (DestinationPublisher, the transformer chain order, the
htmlElementsCollector behind `hugo_stats.json`) and the `golang.org/x/net/html` subset the
collector parses with. Wave B task T07 (transform-publisher).

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `publisher` | `publisher/publisher.go` | T07 transform-publisher | |
| `html_elements_collector` | `publisher/htmlElementsCollector.go` | T07 transform-publisher | state machine verbatim |
| `xnethtml` (+ `atom`, `atom_table`, `token`, `parse`, `node`, `foreign`, `doctype`, `escape`) | `golang.org/x/net@v0.41.0/html`: `token.go`, `parse.go`, `node.go`, `foreign.go`, `const.go`, `doctype.go`, `escape.go` (unescaping), `atom/atom.go` + `atom/table.go` | T07 transform-publisher | NEW; `atom_table.rs` is generated |

Every checklist entry is `OK`; nothing is stubbed.

## Public API

- `publisher::{Descriptor, DestinationPublisher, Publisher, PublishStats,
  create_transformer_chain}`. `DestinationPublisher::new(fs, min, &BuildStats)` (the skeleton
  signature: the caller builds the `minifiers.Client`, Go's `NewDestinationPublisher` does it
  with `minifiers.New(mediaTypes, outputFormats, cfg)`), `publish(Descriptor)`,
  `publish_stats()`, and new: `transform(&Descriptor) -> Result<Vec<u8>>` (the bytes `publish`
  writes). `Descriptor.live_reload_base_url` is a URL string (parsed with go-url when set).
- `html_elements_collector::{HtmlElements (merge, sort), HtmlElement, HtmlElementsCollector
  (new, write, get_html_elements), HtmlElementsCollectorWriter (new, write, err),
  parse_html_element, parse_start_tag, is_closed_by_tag}`. `HtmlElementsCollector::write(doc)`
  is one `Publish`: a new writer and one `Write` of the whole document; it returns the writer's
  last parse error (Go keeps it in the writer and never reads it). `HtmlElementsCollectorWriter`
  keeps its state across several `write`s like Go's (the upstream test streams the minifier's
  many writes into it).
- `xnethtml::{parse, Document, Node, NodeId, NodeType, Attribute, Token, TokenType, Tokenizer,
  atom, parse_element_attributes, walk_elements}`. `parse(&[u8]) -> Result<(Document, NodeId),
  String>` is `html.Parse(strings.NewReader(s))`: nodes live in an arena (`Document.nodes`,
  `NodeId` = Go's `*Node`, identity = index); `Err` carries the text of the Go panic the input
  triggers. The skeleton's `Attribute {key, val: String}` became Go's `{namespace, key, val}` as
  bytes and `parse_element_attributes` takes bytes (no other crate used them).

## Dependencies

- nh-*: nh-common, nh-config, nh-media, nh-hugofs, nh-helpers (`open_file_for_writing`,
  `unique_strings_reuse`, `unique_strings_sorted`), nh-transform.
- Wave A: go-unicode, go-json (unused so far: T24 encodes `hugo_stats.json`), go-html (the
  entity tables, identical to x/net/html's: same 2,138 + 91 entries), go-url, go-strconv (`%q`).
- crates.io: none. **No HTML parser crate:** html5ever and friends implement the WHATWG
  algorithm, not x/net/html's divergences (the `<template>`+foreign-content early exit,
  `resetInsertionMode` treating td/th/head specially, `inSelectInTable` matching `<math select>`,
  no NUL replacement in tag names, the attribute-value entity rules of `unescape(…, true)`,
  `Token.Data` from the atom table), and the collected classes/ids depend on them. Tests only:
  `serde_json`, `flate2` (decompression), `base64`, `sha2` — all on the README list.

## The x/net/html subset

The collector's element strings are whatever lies between a `<` and the next unquoted `>` (by
the collector's own quote tracking), e.g. `<div a='x'><span class=y>`, `<th …>`, `<svg …>`,
`<template …>`, `<select …><textarea>`, so `html.Parse` of them reaches any insertion mode, the
foreign-content rules and the tokenizer's raw-text/RCDATA/script/comment/doctype/CDATA states.
The whole of `parse.go` (all 23 insertion modes, the adoption agency algorithm, foster
parenting, `parseForeignContent`, `resetInsertionMode`, implied tokens) and `token.go` (every
state, including the script-data escape states) is ported function by function. Not ported:
`render.go`, `ParseFragment`/`NewTokenizerFragment`'s parser half (never called; the tokenizer's
context tag is ported), the tokenizer's streaming buffer/`maxBuf` (the input is always a whole
string, so its spans are absolute offsets: the bytes they denote are the same) and the escaping
half of `escape.go`. Where Go panics (a bad parser state, an index out of range, a nil
dereference, `AppendChild` of an attached node) the port records Go's panic text and stops: the
collector treats it like a parse error (Go would crash). No input of the oracle panics Go.

Verification: `tests/xnethtml.rs` compares a full dump (type, namespace, data, atom and
attributes in order of every node) of 28,377 parses with Go's: all 1,712 `#data` inputs of
x/net/html's html5lib tree-construction test files (`testdata/webkit/*.dat` and `scripted/`,
parsed as documents), the collector's adversarial element strings and 15,000 random tag-soup
documents; 0 differences. The atom table is generated from x/net/html's `table.go` by the
oracle (`-atoms`) and checked against `atom.Lookup`/`Atom.String`.

## Go behaviour reproduced on purpose

- Collector: a `Write` stops at the first `utf8.RuneError` (invalid UTF-8 or a literal U+FFFD);
  the `(?i)^(pre|textarea|script|style)` prefix match (so `<prefix>` skips its content too), the
  `(?i)^!DOCTYPE` skip, `<!--` only when the buffer is exactly `<!--`, the quote state that
  carries over between elements, `<<div` giving the tag `<div`, the element set keyed by the raw
  element string, the `thead/tbody/tfoot/td/tr` → `div` substitution of the FIRST occurrence of
  the tag name in the string, `th`/`caption`/`col`/`colgroup`/`frame`/`head` ignored in body
  (their tag is still collected, their classes/ids not), `<image>` → `img`, the `(?i)^class$|
  transition` class attributes, and the Vue/Alpine `:class` handling (`htmlJsonFixer`,
  `jsonAttrRe` = `'?(.*?)'?:\s.*` with `$1`, then `extractSingleQuotedStrings`, which adds false
  positives on purpose). The Go regexps are matched by hand with RE2 semantics: `(?i)` is
  Unicode simple folding (`ſ` matches `s`, `K` (U+212A) matches `k`), `\s` is `[\t\n\f\r ]`, `.`
  any rune but `\n`, leftmost-first priorities for `jsonAttrRe`'s greedy/lazy parts.
- `HTMLElements.Merge` keeps Go's nil-vs-empty distinction (`null` in `hugo_stats.json`);
  `getHTMLElements` sorts and dedupes (`UniqueStringsSorted`: empty → nil).
- Publisher: the chain order absURL (HTML quotes for `IsHTML` formats, XML quotes otherwise) →
  livereload (HTML, server only) → generator tag (HTML, never) → minifier (when the client's
  `MinifyOutput` is set; `Descriptor.Minify` is ignored, as in Go); `Create` (truncate) with
  `MkdirAll` of the parent on ENOENT; one write of the whole output (none when it is empty); the
  collector sees exactly the written bytes and only for `IsHTML` formats; the stat counter counts
  successful writes; `failed to process "<target>": <err>` with Go's `%q`.

## Deliberate deviations

- Go panics in `parseStartTag` (only on strings the collector never produces) and in x/net/html
  become errors recorded like a parse error (Go would crash the build).
- `HtmlElementsCollector` is not internally locked (Go's `sync.RWMutex`): the publisher holds it
  in a `Mutex` for the duration of one document's scan. The collected set does not depend on
  the order documents arrive in.
- The collector's element strings are `String`s (they are valid UTF-8: the scan stops at the
  first invalid byte); attribute values and tag names are converted with a lossy conversion that
  cannot trigger for them.

## Tests and oracles

| test | oracle | cases |
|---|---|---|
| `tests/xnethtml.rs` | `tools/go-oracle/nh-publisher/xnethtml` → `fixtures/xnethtml/parse.jsonl.gz` | 28,377 parse-tree dumps (above) |
| `tests/collector.rs` | `tools/go-oracle/nh-publisher/collector` (overlay) → `fixtures/collector/collector.jsonl.gz` | 59,510: `parseHTMLElement` of 10,400+ adversarial element strings (th/caption/col/colgroup/frame/image quirks, quotes, entities, `:class`/`x-bind:class`/`v-bind:class`/`x-transition`, foreign content, NUL, Unicode) × 3 configs; whole documents (upstream `TestClassCollector`, 43 hand-written documents with comments/CDATA/PI/script/style/pre/textarea content, open quotes, U+FFFD and invalid UTF-8, doctypes; the html5lib inputs; 4,000 random documents; the element strings) one per collector and per group through one collector × 5 BuildStats configs; 1,100+ multi-write streams (random splits, rune-aligned or not, and the writes of the tdewolff HTML minifier streaming into the collector); 3,000+ `isClosedByTag` |
| `tests/site.rs` | `tools/go-oracle/nh-publisher/site` + `sitefix` (overlay; see below) → `fixtures/site/<variant>.jsonl.gz` | every `Publish` of 12 real builds replayed through `DestinationPublisher::publish` on a MemMapFs: written bytes (SHA-256, and the bytes when ≤ 4 KiB) and the collector's elements |

The site oracle builds, in process with hugolib, this repository's `docs/` site (patched for an
offline, deterministic build: no `resources.GetRemote`, no npm packages in `js.Build`, `minify`
instead of the Tailwind CLI, a fixed clock) and `hugolib/testsite` (two languages, synthetic
layouts full of absURL/minifier/collector edge cases, aliases, 404, RSS, sitemap, JSON, robots)
in 12 variants: canonifyURLs and relativeURLs on/off/both, a baseURL with and without a path
(`https://example.org/docs/`, `/sub/`, `/sub` without the slash), minify off / default /
customised (keepWhitespace, keepComments, keepQuotes, keepEndTags=false, keepConditionalComments,
CSS/JS/JSON/SVG precision, JS version, XML keepWhitespace), and BuildStats with tags/classes/ids
disabled. It records every Publish (descriptor, source bytes, written bytes), the minifier
client's media types/output formats/config and the build's `hugo_stats.json`. A second phase
(`tools/go-oracle/nh-publisher/sitefix`, no hugolib) runs as linux/arm64 under qemu (the golden build's FMA behaviour): it recomputes every written
file with the publisher's own chain (`createTransformerChain` + `Chain.Apply`), checks the Go
collector over the recorded HTML gives the build's `hugo_stats.json` byte for byte, and writes
the checked-in subset (all testsite records; each docs build's non-HTML outputs and 16 HTML
pages, with the Go collector's elements of exactly those pages) plus the full records outside
the repository.

Results: 4,800 publishes (6 docs variants × 760, 6 testsite variants × 40; 4,578 HTML files
through the collector), 0 differences between the native and the arm64 Go chain, the Go
collector over the recorded HTML reproduces each build's `hugo_stats.json` byte for byte, and
the Rust publisher matches all of them (the full set with the ignored test, 366 publishes in the
checked-in subset): written bytes and elements.

Regenerating (from the repo root, `GOTOOLCHAIN=go1.27.1`; `$FULL` is any scratch directory):

```sh
go run ./tools/go-oracle/nh-publisher/xnethtml
go run ./tools/go-oracle/nh-publisher/xnethtml -atoms crates/nh-publisher/src/xnethtml/atom_table.rs
go run ./tools/go-oracle/nh-publisher/collector
go run ./tools/go-oracle/nh-publisher/site -full $FULL
# the publisher package imports libwebp/libsass (cgo): cross-compile with zig, see crates/gift/PORTING.md
NH_T07_ORACLE_ARCH=arm64 NH_T07_ORACLE_CC=zcc NH_T07_ORACLE_CXX=zcxx \
  go run ./tools/go-oracle/nh-publisher/sitefix -full $FULL      # zcc = zig cc -target aarch64-linux-musl -UNDEBUG
NH_T07_SITE_FULL=$FULL-arm64 cargo test --offline --test site -- --ignored   # the full records
```

## Known gaps

- `hugo_stats.json` itself is written by `hugolib` (T24: merge the sites' `publish_stats()`,
  `sort`, Go's JSON encoder with `SetEscapeHTML(false)` and two-space indent).
