# neohugo-markup

Markdown for neohugo: comrak behind an engine-neutral API (REWRITE_PLAN.md §2.4), plus
Hugo's passes. The API is T22's; this file records the **T04 comrak spike** that settles the
engine before the API is frozen.

## Decision

**comrak 0.55.0 is the engine** (workspace features `attributes`, `shortcodes`; no `bon`,
`syntect` or CLI features). pulldown-cmark stays the documented fallback behind the same API,
but it was **not evaluated**: it is neither in `Cargo.lock` nor in the offline registry cache,
and this task had no network. Nothing measured below calls for it.

Reasons:

1. **Block structure and inline parsing already match goldmark on the corpora.** Natively,
   with no neohugo pass, 862/959 docs pages and 251/251 seeksnack bodies (plain goldmark
   configurations; 249/251 with Hugo's extensions) are equal after normalisation. Once the
   differences owned by passes that T22 writes anyway are folded (Hugo's comment dropping,
   typography, footnote markup), **925/959** docs pages are equal; the remaining 34 are listed
   under "residual differences" and each maps to a pass or an accepted quirk.
2. **Every feature the sites use exists in comrak or is a small AST pass**: tight definition
   lists parse (`Term\n: def`), heading attributes parse (11/11), alerts parse (278/278 on
   the docs), emoji shortcodes resolve exactly like goldmark-emoji (2007/2007 candidates),
   `codeFences = false` output is byte-identical (2036/2036 `<pre>` blocks), fenced code
   language and content equal Hugo's hook data (2006/2006).
3. **`sourcepos` is exact for inline nodes** (links, images, code, emphasis, raw HTML, text)
   in paragraphs, lists, tables, blockquotes, headings and definition lists — the input for
   `inner_page` spans — with one characterised comrak defect (below).
4. **Hooks fit the AST.** `NodeValue::Raw` is valid under any parent (`can_contain_type`),
   so "run the hook post-order, replace the node by raw HTML" works without tripping
   comrak's debug-build AST validation in `format_html`.
5. The gaps are all **semantic goldmark/Hugo quirks** (passthrough delimiters, block
   attributes, deflist tightness, typographer and linkify heuristics), which any engine other
   than a goldmark port needs as custom passes too; switching engines would not remove one.

## Measurements

Inputs: the 959 `docs/content` bodies of `testdata/oracle/markup/convert` (Hugo's goldmark
converter, six markup configurations, with a stub highlighter) and `markup/hooks` (hook
contexts, `seeksnack` configuration), and the 251 seeksnack bodies of
`testdata/corpus/goldmark/corpus{,-ext}.gmf.gz` (plain goldmark instances `default`,
`unsafe`, `all`, `hugo`, `hugo-autoid`). comrak runs natively with the options closest to
each configuration (`tests/it/comrak_spike/engine.rs`).

Normalisation (`normalize.rs`): entities decoded, attributes sorted, whitespace collapsed
outside `<pre>` and dropped next to block tags, XHTML slashes ignored, code blocks folded to
`<pre lang>text</pre>` (Chroma, the stub highlighter and `<pre><code class="language-…">`
alike), `align="x"` → `style="text-align: x"` (Hugo's table template), ids on `h1`–`h6`/`dt`
dropped (auto ids are a pass). Feature rows compare the feature's own elements, not pages.

### Whole documents

| input | matched / total |
|---|---|
| docs, cfg `default` (unsafe off, typographer, linkify, deflist, footnote, `attribute.title`) | 862 / 959 |
| docs, cfg `seeksnack` (same, unsafe on) | 870 / 959 |
| docs, cfg `ascii` (github-ascii ids, `attribute.block`, dt ids, image not wrapped) | 864 / 959 |
| docs, cfg `blackfriday` (xhtml, hardWraps, no typographer) | 897 / 959 |
| docs, cfg `cjk` (CJK line breaks, escaped space, custom typographer quotes — not in comrak) | 452 / 959 |
| docs, cfg `noattr` | 861 / 959 |
| docs, cfg `default`, after pass-owned folds | **925 / 959** |
| docs, cfg `seeksnack`, after pass-owned folds | **925 / 959** |
| seeksnack, goldmark `default` / `unsafe` / `all` | 251 / 251 each |
| seeksnack, goldmark `hugo` / `hugo-autoid` | 249 / 251 (both: an unbalanced `"`, typographer) |

### Per feature (native comrak, no pass)

| feature | matched / total | verdict |
|---|---|---|
| definition lists `<dl>` | 145 / 175 | parse OK; **pass** for goldmark tightness and list splitting |
| definition details `<dd>` | 866 / 890 | same |
| tight `<dd>` (no `<p>`) | 866 / 889 | same (the plan's "840" is stale: 889 tight of 890) |
| heading attributes `{#id .class k=v}` | 11 / 11 | comrak parses (AST only; the renderer ignores attrs); render via heading hook/pass |
| block attributes (`attribute.block`, cfg ascii) | 0 / 7 consumed | **pass**; comrak keeps the line as paragraph text (7/7, incl. lazy continuation into a blockquote paragraph) |
| fence language | 2006 / 2006 | OK (Hugo hooks fenced blocks only; indented code is not hooked) |
| fence content (`Inner`, chomped) | 2006 / 2006 | OK |
| fence attributes `{k=v …}` via comrak's parser | 236 / 237 | **pass**: parse the raw info string with Hugo's grammar (fails on `hl_lines=[3, "6-8"]`, values untyped) |
| math `$$ … $$` verbatim | 7 / 7 | by accident only; **pass** |
| math `\[ … \]` verbatim | 1 / 6 | **pass** (`\[` is a CommonMark escape; content gets mangled) |
| math `\( … \)` verbatim | 0 / 3 | **pass** |
| GitHub alerts (type, title, sign) | 278 / 278 | docs covered; comrak knows 5 types and no `+`/`-` sign → **pass** for Hugo semantics |
| regular blockquotes with alerts on | 34 / 34 | OK |
| emoji `:name:` | 1965 / 2007 resolve; 2007 / 2007 same as goldmark-emoji | OK (only encoding differs: Unicode vs `&#x…;`) |
| links, all `<a>` | 4865 / 4924 | differences are footnote markup and linkify |
| linkify (bare URL / `www.` / email anchors) | 60 / 73 (+16 comrak-only) | **pass** for parity (see residuals) |
| typographer (quotes, dashes, ellipsis) | 3533 / 3567 (pages 362 / 425) | **pass** for parity (goldmark heuristics) |
| raw HTML omitted (`unsafe = false`) | 4 / 4 comments | OK, plus the comment-drop pass |
| raw HTML passed (`unsafe = true`) | 22 / 22 tags/comments | OK |
| `codeFences = false`, byte-exact `<pre>` | 2036 / 2036 | OK: `CodeFences::Plain` = comrak's default code block |

### `sourcepos` of inline nodes (docs + seeksnack)

| node | exact |
|---|---|
| link: paragraph / list / table / deflist / blockquote / heading / footnote | 1663/1663, 708/708, 2087/2087, 498/498, 181/181, 4/4, 5/5 |
| image | 556 / 556 |
| code span, emphasis, raw inline HTML | 7609/7609, 1054/1054, 58/58 |
| text (no escapes/entities/typography) | 23237 / 23237 |
| **inlines of a paragraph that began with link reference definitions** | links 0 / 15, text 20 / 81 |

Columns are UTF-8 byte columns (1-based, end inclusive), so `line_start[line-1] + col - 1` is
the byte offset; no tab or CRLF case occurs in the corpora. **Defect:** when comrak strips
link reference definitions from the start of a paragraph and text remains (e.g. an invalid
definition such as `[x]: {{% eturl … %}}`), the remaining inlines keep lines relative to the
paragraph start: they are shifted **up by the number of removed lines** (columns correct).
Probe: `"[a]: /x\n[b]: {{% y %}}\ntext [a] more\n"` reports `text` at 2:1 instead of 3:1.
T22 must correct this (re-scan the paragraph's source for leading definitions and add the
line count) or upstream a fix; everywhere else positions can be trusted.

### Residual differences (the 34 pages after folds, cfg `default`, by first difference)

| cause | pages | pass / action |
|---|---|---|
| deflist: only the **first** paragraph of a tight `<dd>` is unwrapped (goldmark `ReplaceChild` loop stops after one); tightness = "no blank line before the `:` line" per `<dd>` | 12 | deflist pass |
| deflist: goldmark ends the `<dl>` at a link reference definition line; comrak continues it | 6 | deflist pass (split where a non-blank, node-less line sits between items) |
| linkify: goldmark keeps a trailing `'`/`"` in the URL (`https://example.org/'`) | 9 | linkify pass |
| linkify: goldmark does not linkify after `:` (`WORK:jsmith@example.org`) | 1 | linkify pass |
| table: cells padded for a short row get alignment none | 3 | table hook data |
| table: goldmark accepts a delimiter row with more cells than the header (QR.md) | 1 | accept (GFM-correct in comrak) or pre-pass |
| table as a lazy-continuation line inside a list item (21yunbox) | 1 | accept |
| list tightness (codeberg: blank line before an indented table in the last item) | 1 | accept, or goldmark tightness recompute |

## Custom passes T22 must write

Passes run on the comrak AST between `parse_document` and `format_html`; hooked nodes are
replaced by `NodeValue::Raw`.

1. **Heading ids and fragments**: Hugo's `github` / `github-ascii` / `blackfriday` anchorize,
   per-document dedupe, the `TextPlain` first-child quirk (spec markdown.md §7), TOC. Do not
   use comrak's `header_id_prefix` (different Unicode tables, emits an `<a class="anchor">`).
2. **Definition-term ids** (`autoDefinitionTermID`, cfgs ascii/noattr and the docs site).
3. **Definition lists, goldmark semantics**: per-`<dd>` tightness from the blank line before
   the `:` marker; unwrap only the first paragraph of a tight `<dd>` (comrak's tight render also
   glues a tight `<dd>`'s paragraphs without a separator, e.g. `texta|b`); split a list that
   comrak continued across link reference definitions.
4. **Block attributes** (`attribute.block`): a trailing `{…}` line of a paragraph (also a
   lazy-continuation line inside a blockquote) is removed and applied to the paragraph or the
   preceding block; after a GFM table the line would become a table row. Hugo's attribute
   grammar, not comrak's.
5. **Fence info → options/attributes**: parse the raw `NodeCodeBlock::info` (keep
   `fenced_code_attributes` off) with Hugo's grammar: commas, arrays, typed values,
   options vs attributes split.
6. **Passthrough / math**: a pre-parse scan for the configured delimiters (`\[…\]`, `$$…$$`,
   `\(…\)`) outside code, replacing each span by a same-length placeholder so `sourcepos`
   stays valid, then a post-parse swap to a passthrough node / hook. Keep `math_dollars` off
   (it also turns `$…$` into math, which Hugo does not).
7. **Alerts, Hugo semantics**: keep comrak `alerts` off; detect `[!type]`, sign and title on
   the first paragraph of a blockquote (Hugo's regex), lowercase the type, and drop the first
   line from the hook text. Without a blockquote hook the output is a plain blockquote
   (what the oracle shows).
8. **Typographer (goldmark rules)**: `parse.smart` off and goldmark's typographer over text
   nodes (spec §8.4, per-block quote counters, configurable substitutions incl. empty ones,
   entity output). Needed for seeksnack byte parity (the 2 bodies) and the 63 docs pages
   whose substitution sequence differs.
9. **Linkify (goldmark rules)**: `extension.autolink` off and goldmark's linkify over text
   nodes (spec §8.5): trigger characters, trailing-punctuation trimming, `linkifyProtocol`
   for `www.` (comrak writes `http://`, Hugo's default is `https`).
10. **Raw HTML under `unsafe = false`**: drop HTML comments (block and inline) instead of
    `<!-- raw HTML omitted -->` (Hugo's `hugocontext` renderer). Prototyped in
    `engine::to_html_passes`.
11. **`sourcepos` fix** for paragraphs that began with link reference definitions (above).
12. **Renderers Hugo owns** (hooks or default templates, not passes on the tree): tables
    (thead/tbody data, alignment none for padded cells), blockquote default
    (`…</p></blockquote>`), footnotes (`fn:1`/`fnref:1` ids, `↩︎`, `div.footnotes`),
    emoji as `&#x…;` entities if byte parity matters, `wrapStandAloneImageWithinParagraph =
    false` (block images), HTML5 void tags (comrak writes `<br />`, `<img … />`).
13. Not needed by the sites: CJK east-Asian line breaks and escaped space (cfg `cjk`).

## Risks for T22

- The `sourcepos` defect above silently gives the wrong `inner_page` for inlines of such
  paragraphs; test it explicitly.
- comrak's `format_html` validates the AST in debug builds; every pass must leave a valid
  tree (`Raw` is always allowed).
- comrak parses heading attributes with its own grammar (bare values limited to
  `[A-Za-z0-9-_:.%]`, no commas or arrays); it matched all 11 docs headings, but Hugo's
  grammar should be the one T22 applies (re-parse the heading's trailing `{…}`).
- Typographer and linkify passes are ports of goldmark heuristics (spec §8.4–8.5); without
  them seeksnack is 249/251 and 63 docs pages differ in quotes.
- The oracles cover no configuration with passthrough or `enableEmoji`, so those two verdicts
  rest on the spec and on goldmark-emoji's table, not on Hugo HTML.
- comrak upgrades: the harness asserts floors at the measured values, so a regression fails
  `cargo test -p neohugo-markup`.

## Re-running the harness

```sh
cd rust
export CARGO_TARGET_DIR=/home/user/neohugo/rust/target CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0
cargo test --offline -p neohugo-markup --test it comrak_spike -- --nocapture   # ~30 s, prints the tables
NEOHUGO_SPIKE_SHOW='definition lists' cargo test ...   # print differing items of one row (label substring)
NEOHUGO_GOLDMARK_EMOJI_TSV=/path/goldmark-emoji.tsv cargo test ...   # enable the emoji comparison
```

The emoji table (`shortname<TAB>hex code points`) comes from goldmark-emoji v1.0.6 (the version
in the repository's `go.mod`), resolved for every `:name:` in the docs:

```go
// go run . < names.txt > goldmark-emoji.tsv   (module requiring github.com/yuin/goldmark-emoji v1.0.6)
defs := definition.Github()
for sc := bufio.NewScanner(os.Stdin); sc.Scan(); {
    if e, ok := defs.Get(strings.TrimSpace(sc.Text())); ok { /* print name, "%x" of each rune in e.Unicode */ }
}
```

with `names.txt` from `grep -rhoE ':[a-z0-9_+-]+:' docs/content --include=*.md | tr -d : | sort -u`.
