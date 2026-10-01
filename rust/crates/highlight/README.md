# neohugo-highlight

Code highlighting for neohugo (T25; REWRITE_PLAN.md §1.1 decision D1, §2.1): syntect 5.3 with
two-face's syntaxes tokenises, a scope map turns syntect scopes into **Chroma token types**, and
the HTML is the HTML Hugo writes with Chroma: the same wrappers, line structure, line numbers,
highlighted lines, **Chroma class names** or **inline styles** computed from **Chroma's own style
files**. Sites keep their Chroma style sheets (the docs' `chroma.css`), and
`hugo gen chromastyles` output can be reproduced ([`Highlight::css`]).

## API

```rust
pub struct Highlight;                                     // Send + Sync; build once per build
impl Highlight {
    pub fn new(&HighlightConfig) -> Self;                 // [markup.highlight]; first call loads the syntaxes (~85 ms dev)
    pub fn highlight_with(&self, code, lang, OptionsArg) -> Result<String, HighlightError>; // `highlight` function
    pub fn highlight(&self, code, lang, &Options, attributes: Option<&Map>) -> String;
    pub fn css(&self, style, CssMode) -> Result<String, HighlightError>;  // gen chromastyles
    pub fn can_highlight(&self, lang) -> bool;            // transform.CanHighlight
    pub fn tokens(&self, code, lang) -> Option<Vec<(TokenType, &str)>>;
    pub fn diagnostics(&self) -> Vec<Diagnostic>;         // unknown style names (fallback warnings)
    pub fn defaults(&self) -> &Options; pub fn lexer(&self, lang) -> Option<Lexer>;
    pub fn style_names(&self); pub fn lexer_syntaxes(&self);
}
impl neohugo_markup::Highlighter for Highlight { .. }     // fences no code-block hook handles
pub enum OptionsArg<'a> { None, Str(&'a str), Map(&'a Map) }   // "linenos=table,hl_lines=2" or a dict
pub struct Options { style, styling: Styling, line_nos, line_number_layout: LineNumberLayout,
                     anchor_line_nos, line_anchors, line_no_start, hl_lines, hl_ranges,
                     layout: CodeLayout, tab_width, guess_syntax, wrapper_class }
impl Options { from_config, apply_str, apply_map, highlight_ranges }
pub enum Styling { Classes, Inline }  LineNumberLayout { Table, Inline }  CodeLayout { Block, Inline }
pub enum TokenType { .. }  // Chroma's types: number, name, class(), parent(), category()
pub enum CssMode { AllClasses, OmitEmpty }
pub mod scope { pub const RULES; pub const NESTED; pub enum Rule { Type, Whole, Defer } }
```

Options follow Hugo: the site's `[markup.highlight]`, then a fence's `{…}` options (keys
case-insensitive, `hl_lines` as markup's 0-based ranges shifted by that map's `linenostart`), or
the function's option string / map (weakly typed like mapstructure: `"true"`, `1`, `"0x10"`).
`linenos=table|inline` also picks the layout. A fence without `lineanchors` (site or fence)
numbers its line ids `hl-<ordinal>-<n>`, the ordinal being `HighlightOptions::ordinal` (the
page's code block count), as Hugo does; the function has no prefix. Invalid values are errors (`OptionsError`).

## How it works

- **Languages** (`src/lexers.rs`). Whether a language is known is Chroma's decision:
  `src/data/chroma-lexers.tsv` is Chroma v2.19.0's `lexers.Get` result for every lexer name,
  alias (case-insensitive) and file extension/name (exact). Unknown languages render as Hugo
  does (`<pre tabindex="0"><code class="language-x" data-lang="x">`, no wrapper). A known lexer
  is tokenised by the syntect syntax found by name, alias, then extension (overrides in
  `SYNTAX_OVERRIDES`, including 8 lexers whose extensions would pick an unrelated grammar);
  lexers without one (e.g. Terminfo, PowerShell) are plain text inside Chroma's structure,
  like Chroma's `plaintext`. 116 of Chroma's 270 lexers have a grammar.
- **Go templates** (`src/syntaxes/`, original, MIT). `Go Template` (`go-template`,
  `go-text-template`, `*.gotmpl`) follows Chroma's `go_template` lexer rule by rule (including
  its quirk that numbers are names); `Go HTML Template` (`go-html-template`) is HTML with the
  template actions and comments injected everywhere (`with_prototype`), like Chroma's
  delegating lexer. 1,313 of the 2,284 docs items are `go-html-template`.
- **Scopes → token types** (`src/scope.rs`). A stack is read innermost first; the longest
  matching prefix decides, language-specific rules (`prefix @lang`, matching the scope's last
  atom) outrank generic ones. `Defer` scopes (string quotes, `$`, entity punctuation) take the
  outer type; `Whole` scopes (comments, doctypes, HTML attribute values) give their type to
  everything inside; `NESTED` rules make CSS/JS inside `style`/`on…` attributes one string, as
  Chroma does. YAML whitespace outside tokens becomes `w` tokens (Chroma's YAML lexer).
- **Styles** (`src/style.rs`, `src/styles/*.xml`). All 67 Chroma v2.19.0 style files, verbatim,
  with Chroma's inheritance (`Background`, `Text`, category, sub-category, `noinherit`), its
  synthesised line-number and line-highlight colours, its CSS properties and compression.
- **HTML** (`src/html.rs`): Chroma's formatter (lines split after `\n`, `line`/`cl` spans,
  `ln` inline or the `lntable` layout, `hl` lines, `lnlinks` anchors, `style` attributes with
  sub-category/category fallback) inside Hugo's wrappers (`<div class="{wrapperClass} {class}"
  attrs>`, `<pre tabindex="0">`, `<code class="language-x" data-lang="x">`, `hl_inline`'s
  `<code class="code-inline language-x">`). Fences get a final newline (Hugo's code block
  renderer); the function does not.

## Acceptance (T25 row of §8.2)

`cargo test -p neohugo-highlight --test it -- --nocapture` prints the tables.

| criterion | result |
|---|---|
| all docs fences highlight | **2,284/2,284** items render: 1,996 fences (goat fences excluded: they go to the goat hook), 285 `code-toggle` bodies (in the format they are written in), 2 `highlight` and 1 `hl` shortcodes; 2,283 have a Chroma lexer and get Hugo's highlighted structure, 1 (`texts`, a typo) stays plain exactly as in Hugo |
| **byte-identical to Hugo** (docs config: `noClasses=false`, `solarized-dark`, `lineNumbersInTable=false`, wrapper `highlight not-prose`; hashes of Hugo's `transform.Highlight` output) | **2,205/2,284 (96.5%)**; floor 95% |
| classes ⊆ Chroma's | every `class` written is in Chroma's `StandardTypes` (or Hugo's wrapper classes); asserted on every item |
| token coverage (non-whitespace characters Chroma classifies, 181,725) | **99.8%** get a class; **99.5%** the same family (keyword/name/string/number/…); **99.4%** the same class. Floors 99.5 / 99.0 / 99.0 |
| `go-html-template` | own syntax; 132,849 characters, **100.0%** same class (floor 99.5) |
| solarized-dark CSS | byte-identical to `hugo gen chromastyles --style=solarized-dark`, with and without `--omitEmpty` |
| inline styles (`noClasses`), `hl_inline`, `lineNumbersInTable`, `linenos`, `hl_lines`, `linenostart`, `anchorlinenos`, `lineanchors`, `tabWidth`, `wrapperClass`, styles | option matrix (5 inputs × 19 option sets: known/plain/unknown/no language) **95/95 byte-identical** to Hugo |
| Chroma style names → styles, with a fallback warning | every Chroma style is bundled (the docs use `solarized-dark`, `emacs`, Hugo's default `monokai`); an unknown name falls back to Chroma's `swapoff` (as Hugo) and is reported once by `diagnostics()` |

Per lexer (docs corpus, characters Chroma classifies):

| lexer | chars | classified % | same family % | same class % |
|---|---|---|---|---|
| Go HTML Template | 132,849 | 100.0 | 100.0 | 100.0 |
| TOML | 28,583 | 100.0 | 100.0 | 99.9 |
| YAML | 11,247 | 100.0 | 99.4 | 99.4 |
| JavaScript | 1,938 | 100.0 | 94.1 | 91.5 |
| JSON | 1,480 | 100.0 | 99.9 | 99.4 |
| Bash | 1,469 | 96.9 | 96.5 | 95.9 |
| XML | 1,389 | 100.0 | 100.0 | 100.0 |
| TypeScript | 1,228 | 80.5 | 78.5 | 69.9 |
| CSS | 558 | 92.8 | 78.9 | 69.5 |
| Diff | 218 | 100.0 | 93.1 | 69.3 |
| Go | 196 | 100.0 | 99.0 | 99.0 |
| CSV / SCSS / Go Template / react / Terminfo | 178 / 57 / 108 / 141 / 86 | 100 / 98.2 / 100 / 100 / 0 | 98.3 / 98.2 / 100 / 39.7 / 0 | 98.3 / 98.2 / 100 / 36.2 / 0 |

Speed (dev profile, syntect at opt-level 3): the first `Highlight::new` of a process ≈ 85 ms
(`build.rs` links two-face's syntaxes and the Go template syntaxes into one set at compile time
and the binary loads its dump lazily; linking it at run time took ≈ 0.45 s in release, T70),
the 2,284 docs items ≈ 1.9 s.

## Accepted deviations

- **Tokens come from TextMate grammars, not Chroma's lexers.** Where a grammar and a Chroma
  lexer disagree the span structure differs (allowed at L3, §7): shell numbers after `=`,
  JavaScript's `from`, TypeScript and CSS details, Diff headers; languages without a grammar in
  two-face (Terminfo, PowerShell, …) are plain text. The plan's risk 7.
- **Styles are Chroma's style files, not syntect themes** (the plan says "style names →
  themes"): the token types are already Chroma's, so Chroma's own definitions give Hugo's exact
  colours and every Chroma style name works; two-face's themes are unused. The "fallback
  warning" is for names Chroma does not have either (Hugo falls back silently).
- **Fence attributes** are written in key order (markup's attribute `Map` is sorted; Hugo keeps
  source order), and the `class` value is escaped (Hugo writes it raw).
- **`guessSyntax`** uses syntect's first-line detection instead of Chroma's analysers; the
  guessed language name is the syntect syntax name, lower-cased.
- **Multi-line tokens**: a Go template string or comment spanning lines, and an HTML attribute
  value holding a template action, are tokenised per line by syntect; the output agrees with
  Chroma on the docs corpus, but a grammar whose constructs need look-ahead across lines can
  split differently.

## Plan issues

- Chroma's licence and data (styles, lexer table) live in the crate (`src/styles/COPYING`,
  PROVENANCE rows); a `THIRD_PARTY/chroma/` entry would match the other assets (this task could
  only edit `THIRD_PARTY/two-face/`).
- The "~3,900 docs fences" of the plan counts `code-toggle` three times (YAML, TOML, JSON after
  `transform.Remarshal`); this crate cannot remarshal, so each body is highlighted once in its
  own format (2,284 items).
- **Shared target dir**: cargo's metadata hash of a workspace crate does not depend on the
  worktree path, so an artifact built from another worktree's newer sources looks fresh here
  (this task hit neohugo-config built by the concurrent F1 fix-up). Running with
  `--config 'profile.dev.debug="limited"'` gives the workspace crates their own artifacts (the
  registry crates keep the `package."*"` profile and are shared); `rust/README.md` should say so.

## Fixtures (`tests/data/`)

| file | content |
|---|---|
| `chroma-tokens.json.gz` | per docs item (`key` = FNV-1a of `lang\0code`): Chroma's lexer and class runs (`[class, chars]`) |
| `hugo-docs-html.json.gz` | per docs item (`key` = FNV-1a of `lang\0code\0options-JSON`): FNV-1a of Hugo's HTML with the docs config |
| `hugo-html.json` | the option matrix with Hugo's HTML (`golden.rs`) |
| `solarized-dark{,.omit-empty}.css` | `WriteCSS` of Chroma's HTML formatter (`--omitEmpty` = `WithClasses`) |
| `oracle/main.go.txt`, `oracle/regen.py` (at 44529028) | the Go oracle (Hugo's `markup/highlight` + Chroma v2.19.0) and the script that writes all of the above and `src/data/chroma-lexers.tsv` |

Regenerate in a worktree of 44529028, which has the Go tree and the oracle (`git worktree add
<dir> 44529028`; Go ≥ 1.24 with the module cache of its `go.mod`, offline): run this from its
root, then copy the fixtures above (oracle/ aside) and `src/data/chroma-lexers.tsv` here.

```sh
W=$(mktemp -d); mkdir $W/oracle; cp rust/crates/highlight/tests/data/oracle/main.go.txt $W/oracle/main.go
cp go.sum $W/oracle/ && cat > $W/oracle/go.mod <<EOF
module t25oracle
go 1.24
require github.com/neohugo/neohugo v0.0.0
replace github.com/neohugo/neohugo => $PWD
EOF
(cd $W/oracle && GOFLAGS=-mod=mod GOPROXY=off go build -o oracle .)
(cd rust && NEOHUGO_HL_DUMP=$W/corpus.json cargo test -p neohugo-highlight --test it dump_corpus)
python3 rust/crates/highlight/tests/data/oracle/regen.py $W/oracle/oracle $W
```

Study aids: `NEOHUGO_HL_PAIRS=1` (most frequent class differences per lexer),
`NEOHUGO_HL_SHOW='<lexer>:<class>'` (our scopes where Chroma writes that class),
`NEOHUGO_HL_SCOPES='<lang>|<code>'` (`print_scopes`), `NEOHUGO_HL_OURS=<file>` (our HTML per
item), `NEOHUGO_HL_LEXERS=1` (lexer → grammar table), all with
`cargo test -p neohugo-highlight --test it <test> -- --nocapture`.
