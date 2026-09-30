//! Code highlighting with syntect and two-face, emitting Chroma class names (or inline styles),
//! fence options and style CSS (REWRITE_PLAN.md §2.1, decision D1).
//!
//! Hugo highlights with Chroma; sites style its output with Chroma class names (`.chroma .k`)
//! or rely on its inline styles. This crate tokenises with syntect (two-face's syntaxes plus
//! its own Go template syntaxes), maps each token's scopes to a Chroma token type
//! ([`scope::RULES`]), and writes the HTML Hugo writes: Chroma's line structure, line numbers,
//! highlighted lines, classes or inline styles from Chroma's own style definitions, inside
//! Hugo's wrappers. Whether a language is known at all is Chroma's decision (its lexer table),
//! as in Hugo.
//!
//! [`Highlight`] implements [`neohugo_markup::Highlighter`] for code fences and serves the
//! `highlight` template function ([`Highlight::highlight_with`]) and style sheets
//! ([`Highlight::css`]).

#![forbid(unsafe_code)]

mod html;
mod lexers;
mod options;
pub mod scope;
mod style;
mod token;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, OnceLock, PoisonError};

use neohugo_base::Map;
use neohugo_base::diag::Diagnostic;
use neohugo_config::markup::HighlightConfig;
use neohugo_markup::{HighlightOptions, Highlighter, HookError};
use syntect::parsing::{ParseState, Scope, ScopeStack, SyntaxReference};
use syntect::util::LinesWithEndings;

pub use lexers::{Lexer, Whitespace};
pub use options::{CodeLayout, LineNumberLayout, Options, OptionsError, Styling};
pub use style::{CssMode, FALLBACK_STYLE};
pub use token::TokenType;

use crate::html::Block;
use crate::lexers::Languages;
use crate::scope::{Classifier, ScopeMap};
use crate::style::{CssSettings, Style, Styles};

/// Why highlighting failed.
#[derive(Debug, thiserror::Error)]
pub enum HighlightError {
    /// Invalid options.
    #[error(transparent)]
    Options(#[from] OptionsError),
    /// A style sheet was asked for a style that does not exist.
    #[error("unknown highlighting style {0:?}")]
    UnknownStyle(String),
}

/// The options argument of the `highlight` template function.
#[derive(Clone, Copy, Debug)]
pub enum OptionsArg<'a> {
    /// No options: the site's defaults.
    None,
    /// An option string (`"linenos=table,hl_lines=2"`).
    Str(&'a str),
    /// An option map (`{"hl_inline": true, "noClasses": true}`).
    Map(&'a Map),
}

/// A token: its Chroma type and its text.
pub type Token<'a> = (TokenType, &'a str);

/// The syntaxes, Chroma's lexer table, the scope rules and the bundled styles. They do not
/// depend on the site and loading the syntaxes is most of the cost of a [`Highlight`], so they
/// are loaded once per process and shared (the server makes a `Highlight` for every rebuild).
struct Tables {
    languages: Languages,
    scopes: ScopeMap,
    styles: Styles,
}

fn tables() -> &'static Tables {
    static TABLES: OnceLock<Tables> = OnceLock::new();
    TABLES.get_or_init(|| Tables {
        languages: Languages::load(),
        scopes: ScopeMap::new(),
        styles: Styles::bundled(),
    })
}

/// The highlighter: syntaxes, Chroma's lexer table and styles, and the site's defaults.
pub struct Highlight {
    languages: &'static Languages,
    scopes: &'static ScopeMap,
    styles: &'static Styles,
    defaults: Options,
    /// Style names that fell back to [`FALLBACK_STYLE`], for [`Highlight::diagnostics`].
    fallbacks: Mutex<BTreeSet<String>>,
}

impl std::fmt::Debug for Highlight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Highlight")
            .field("defaults", &self.defaults)
            .finish_non_exhaustive()
    }
}

impl Highlight {
    /// A highlighter with the site's `[markup.highlight]` settings. The first one in a process
    /// loads the syntaxes; later ones share them.
    #[must_use]
    pub fn new(config: &HighlightConfig) -> Self {
        let tables = tables();
        Self {
            languages: &tables.languages,
            scopes: &tables.scopes,
            styles: &tables.styles,
            defaults: Options::from_config(config),
            fallbacks: Mutex::new(BTreeSet::new()),
        }
    }

    /// The site's default options.
    #[must_use]
    pub fn defaults(&self) -> &Options {
        &self.defaults
    }

    /// Whether `lang` names a language Chroma knows (Hugo's `transform.CanHighlight`).
    #[must_use]
    pub fn can_highlight(&self, lang: &str) -> bool {
        self.languages.lexer(lang).is_some()
    }

    /// The Chroma lexer `lang` names.
    #[must_use]
    pub fn lexer(&self, lang: &str) -> Option<Lexer<'static>> {
        self.languages.lexer(lang)
    }

    /// Every Chroma lexer with the name of the syntect syntax that tokenises it (`None`:
    /// plain text).
    pub fn lexer_syntaxes(&self) -> impl Iterator<Item = (&'static str, Option<&str>)> {
        self.languages.mapping()
    }

    /// The names of the bundled Chroma styles.
    pub fn style_names(&self) -> impl Iterator<Item = &str> {
        self.styles.names()
    }

    /// The `highlight` template function: `code` in `lang` with the site's defaults
    /// overridden by `opts`.
    ///
    /// # Errors
    /// Invalid options.
    pub fn highlight_with(
        &self,
        code: &str,
        lang: &str,
        opts: OptionsArg<'_>,
    ) -> Result<String, HighlightError> {
        let mut o = self.defaults.clone();
        match opts {
            OptionsArg::None => {}
            OptionsArg::Str(s) => o.apply_str(s)?,
            OptionsArg::Map(m) => o.apply_map(m)?,
        }
        Ok(self.highlight(code, lang, &o, None))
    }

    /// The HTML of `code` in `lang` with options `o`; `attributes` are a fence's attributes
    /// for the wrapping `<div>`.
    #[must_use]
    pub fn highlight(
        &self,
        code: &str,
        lang: &str,
        o: &Options,
        attributes: Option<&Map>,
    ) -> String {
        let code = code.replace("\r\n", "\n");
        let (lang, syntax, whitespace) = match self.languages.lexer(lang) {
            Some(lexer) => (
                lang.to_owned(),
                self.languages.syntax(lexer),
                lexer.whitespace(),
            ),
            None if o.guess_syntax => {
                let (lang, syntax) = self.languages.guess(&code);
                (lang, syntax, Whitespace::Text)
            }
            None => return html::plain(&code, lang, o.layout),
        };
        let tokens = self.tokenise(&code, syntax, whitespace);
        let style = self.style(&o.style);
        let inline = match o.styling {
            Styling::Inline => style::inline_map(
                style,
                CssSettings {
                    all_classes: false,
                    tab_width: o.tab_width,
                    highlight_lines: !o.highlight_ranges().is_empty(),
                },
            ),
            Styling::Classes => BTreeMap::new(),
        };
        Block {
            lang: &lang,
            options: o,
            attributes,
            inline: &inline,
        }
        .format(&tokens)
    }

    /// The tokens of `code` in `lang` (`None` when Chroma has no lexer for `lang`); plain
    /// text is one [`TokenType::Text`] token.
    #[must_use]
    pub fn tokens<'c>(&self, code: &'c str, lang: &str) -> Option<Vec<Token<'c>>> {
        let lexer = self.languages.lexer(lang)?;
        Some(self.tokenise(code, self.languages.syntax(lexer), lexer.whitespace()))
    }

    /// A style sheet for the Chroma style `style` (`hugo gen chromastyles`).
    ///
    /// # Errors
    /// An unknown style.
    pub fn css(&self, style: &str, mode: CssMode) -> Result<String, HighlightError> {
        let s = self
            .styles
            .get(&style.to_lowercase())
            .ok_or_else(|| HighlightError::UnknownStyle(style.to_owned()))?;
        Ok(style::stylesheet(s, mode))
    }

    /// Warnings so far: the style names that fell back to [`FALLBACK_STYLE`].
    #[must_use]
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        self.fallbacks
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .map(|name| {
                Diagnostic::warning(format!(
                    "unknown highlighting style {name:?}; using {FALLBACK_STYLE:?}"
                ))
                .with_id("highlight-style")
            })
            .collect()
    }

    /// The style named `name` (exactly, as Chroma), else the fallback with a warning.
    fn style(&self, name: &str) -> &Style {
        self.styles.get(name).unwrap_or_else(|| {
            self.fallbacks
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .insert(name.to_owned());
            self.styles.fallback()
        })
    }

    /// Tokenises `code` with `syntax` (plain text without one) and coalesces equal types.
    fn tokenise<'c>(
        &self,
        code: &'c str,
        syntax: Option<&SyntaxReference>,
        whitespace: Whitespace,
    ) -> Vec<Token<'c>> {
        let Some(syntax) = syntax else {
            return vec![(TokenType::Text, code)];
        };
        // (type, start, end) in `code`, coalesced like Chroma's `Coalesce`.
        let mut spans: Vec<(TokenType, usize, usize)> = Vec::new();
        let mut push = |t: TokenType, start: usize, end: usize| match spans.last_mut() {
            Some((last, s, e)) if *last == t && *e == start && *e - *s < 8192 => *e = end,
            _ => spans.push((t, start, end)),
        };
        let mut classifier = Classifier::new(self.scopes);
        self.scan(code, syntax, |stack, start, end| {
            let t = stack.map_or(TokenType::Text, |s| classifier.classify(s));
            if t != TokenType::Text || whitespace == Whitespace::Text {
                push(t, start, end);
                return;
            }
            // Runs of whitespace in plain text are whitespace tokens.
            let text = &code[start..end];
            let mut run_start = start;
            let mut run_ws = None;
            for (i, c) in text.char_indices() {
                let ws = c.is_whitespace();
                if run_ws.is_some_and(|w| w != ws) {
                    let t = if ws {
                        TokenType::Text
                    } else {
                        TokenType::TextWhitespace
                    };
                    push(t, run_start, start + i);
                    run_start = start + i;
                }
                run_ws = Some(ws);
            }
            let t = if run_ws == Some(true) {
                TokenType::TextWhitespace
            } else {
                TokenType::Text
            };
            push(t, run_start, end);
        });
        spans
            .into_iter()
            .map(|(t, start, end)| (t, &code[start..end]))
            .collect()
    }

    /// The scope stacks of `code` in `lang`, for studying the scope map: `(scopes, text)`.
    #[doc(hidden)]
    #[must_use]
    pub fn scopes<'c>(&self, code: &'c str, lang: &str) -> Vec<(String, &'c str)> {
        let Some(syntax) = self
            .languages
            .lexer(lang)
            .and_then(|l| self.languages.syntax(l))
        else {
            return Vec::new();
        };
        let mut out = Vec::new();
        self.scan(code, syntax, |stack, start, end| {
            let names = stack.map_or_else(String::new, |s| {
                s.iter()
                    .map(|x| x.build_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            });
            out.push((names, &code[start..end]));
        });
        out
    }

    /// Calls `f` with the scope stack of every non-empty run of `code`, in order (`None`: the
    /// rest of the code after a parse error).
    fn scan(
        &self,
        code: &str,
        syntax: &SyntaxReference,
        mut f: impl FnMut(Option<&[Scope]>, usize, usize),
    ) {
        let syntaxes = &self.languages.syntaxes;
        let mut state = ParseState::new(syntax);
        let mut stack = ScopeStack::new();
        let mut offset = 0;
        for line in LinesWithEndings::from(code) {
            let Ok(ops) = state.parse_line(line, syntaxes) else {
                f(None, offset, code.len());
                return;
            };
            let mut pos = 0;
            for (at, op) in ops {
                let at = at.min(line.len());
                if at > pos {
                    f(Some(stack.as_slice()), offset + pos, offset + at);
                    pos = at;
                }
                if stack.apply(&op).is_err() {
                    f(None, offset + pos, code.len());
                    return;
                }
            }
            if pos < line.len() {
                f(Some(stack.as_slice()), offset + pos, offset + line.len());
            }
            offset += line.len();
        }
    }
}

impl Highlighter for Highlight {
    /// A code fence no hook handled: the site's defaults, overridden by the fence's options;
    /// the fence's attributes go on the wrapping `<div>`; line ids default to `hl-<ordinal>`.
    fn highlight(&self, code: &str, lang: &str, o: &HighlightOptions) -> Result<String, HookError> {
        let mut opts = self.defaults.clone();
        opts.apply_map(&o.options).map_err(HookError::new)?;
        // Without `lineanchors`, line ids are numbered per code block, as in Hugo.
        if opts.line_anchors.is_empty() {
            opts.line_anchors = format!("hl-{}", o.ordinal);
        }
        // Hugo's code block renderer ends the code with a newline.
        let mut code = code.to_owned();
        if !code.is_empty() && !code.ends_with('\n') {
            code.push('\n');
        }
        Ok(Highlight::highlight(
            self,
            &code,
            lang,
            &opts,
            Some(&o.attributes),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlighters_share_the_tables() {
        let a = Highlight::new(&HighlightConfig::default());
        let b = Highlight::new(&HighlightConfig::default());
        assert!(std::ptr::eq(a.languages, b.languages));
        assert!(std::ptr::eq(a.scopes, b.scopes));
        assert!(std::ptr::eq(a.styles, b.styles));
        // Only the tables are shared: the fallback notices stay per highlighter.
        a.fallbacks.lock().unwrap().insert("x".into());
        assert!(b.fallbacks.lock().unwrap().is_empty());
    }
}
