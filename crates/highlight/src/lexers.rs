//! Language lookup: which Chroma lexer a fence's language names (Chroma decides whether a
//! language is known, as in Hugo), and which syntect syntax stands in for that lexer.

use std::collections::BTreeMap;

use syntect::parsing::{SyntaxReference, SyntaxSet};

/// Chroma's lexer lookup table (`data/chroma-lexers.tsv`).
const CHROMA_LEXERS: &str = include_str!("data/chroma-lexers.tsv");

/// two-face's syntaxes plus the crate's own (`src/syntaxes/`), linked by `build.rs`.
const SYNTAXES_DUMP: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/syntaxes.dump"));

/// Chroma lexers whose syntect syntax the automatic match (name, aliases, file extensions)
/// would not find, or would find wrongly.
const SYNTAX_OVERRIDES: &[(&str, Option<&str>)] = &[
    ("Go HTML Template", Some("Go HTML Template")),
    ("Go Template", Some("Go Template")),
    ("Go Text Template", Some("Go Template")),
    ("plaintext", None),
    ("Bash Session", Some("Bourne Again Shell (bash)")),
    ("PowerShell", None),
    ("react", Some("JavaScript")),
    ("TypeScript", Some("TypeScript")),
    ("TSX", Some("TypeScriptReact")),
    ("JSON", Some("JSON")),
    ("YAML", Some("YAML")),
    ("TOML", Some("TOML")),
    ("XML", Some("XML")),
    ("markdown", Some("Markdown")),
    ("Diff", Some("Diff")),
    ("CSV", Some("Separated Values")),
    ("SCSS", Some("SCSS")),
    ("Sass", Some("Sass")),
    ("Docker", Some("Dockerfile")),
    ("Makefile", Some("Makefile")),
    ("INI", Some("INI")),
    // File extensions shared with an unrelated syntax (`.cpy`, `.fs`/`.frt`, `.m`, `.p`, `.pc`,
    // `.re`, `.v`, `.vsh`): plain text is better than a wrong grammar.
    ("COBOL", None),
    ("Forth", None),
    ("Mathematica", None),
    ("OpenEdge ABL", None),
    ("PkgConfig", None),
    ("ReasonML", None),
    ("V", None),
    ("V shell", None),
];

/// Chroma lexers that write whitespace outside other tokens as whitespace tokens (`w`)
/// rather than plain text.
const WHITESPACE_LEXERS: &[&str] = &["YAML"];

/// How whitespace outside other tokens is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Whitespace {
    /// As plain text.
    Text,
    /// As [`crate::TokenType::TextWhitespace`] tokens.
    Token,
}

/// A Chroma lexer, by its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Lexer<'a> {
    /// Chroma's lexer name (`Go HTML Template`, `plaintext`).
    pub name: &'a str,
}

impl Lexer<'_> {
    /// How the lexer writes whitespace.
    #[must_use]
    pub fn whitespace(self) -> Whitespace {
        if WHITESPACE_LEXERS.contains(&self.name) {
            Whitespace::Token
        } else {
            Whitespace::Text
        }
    }
}

/// The language tables.
#[derive(Debug)]
pub(crate) struct Languages {
    pub syntaxes: SyntaxSet,
    /// Lower-cased lexer names and aliases → lexer name.
    by_name: BTreeMap<&'static str, &'static str>,
    /// File extensions and names (exact) → lexer name.
    by_file: BTreeMap<&'static str, &'static str>,
    /// Lexer name → index of its syntax in `syntaxes` (absent: tokenised as plain text).
    syntax_of: BTreeMap<&'static str, usize>,
}

impl Languages {
    /// Loads two-face's syntaxes plus the crate's own (linked at compile time by `build.rs`)
    /// and maps every Chroma lexer to one.
    pub fn load() -> Self {
        let syntaxes: SyntaxSet = syntect::dumps::from_binary(SYNTAXES_DUMP);

        let mut by_name = BTreeMap::new();
        let mut by_file = BTreeMap::new();
        for line in CHROMA_LEXERS.lines().filter(|l| !l.starts_with('#')) {
            let mut cols = line.split('\t');
            let (Some(kind), Some(key), Some(lexer)) = (cols.next(), cols.next(), cols.next())
            else {
                continue;
            };
            match kind {
                "n" => by_name.insert(key, lexer),
                _ => by_file.insert(key, lexer),
            };
        }

        let mut syntax_of = BTreeMap::new();
        let mut keys: BTreeMap<&str, Vec<(&str, bool)>> = BTreeMap::new();
        for (k, l) in &by_name {
            keys.entry(*l).or_default().push((k, true));
        }
        for (k, l) in &by_file {
            keys.entry(*l).or_default().push((k, false));
        }
        for (lexer, keys) in &keys {
            let found = match SYNTAX_OVERRIDES.iter().find(|(l, _)| l == lexer) {
                Some((_, name)) => name.and_then(|n| syntaxes.find_syntax_by_name(n)),
                None => find_syntax(&syntaxes, lexer, keys),
            };
            if let Some(s) = found {
                let index = syntaxes
                    .syntaxes()
                    .iter()
                    .position(|x| std::ptr::eq(x, s))
                    .unwrap_or_default();
                syntax_of.insert(*lexer, index);
            }
        }
        Self {
            syntaxes,
            by_name,
            by_file,
            syntax_of,
        }
    }

    /// The Chroma lexer `lang` names (Chroma's `lexers.Get`: name or alias ignoring case,
    /// then a file extension or name).
    pub fn lexer(&self, lang: &str) -> Option<Lexer<'static>> {
        let name = self
            .by_name
            .get(lang.to_lowercase().as_str())
            .or_else(|| self.by_file.get(lang))?;
        Some(Lexer { name })
    }

    /// The syntax that tokenises `lexer`; `None` means plain text.
    pub fn syntax(&self, lexer: Lexer<'_>) -> Option<&SyntaxReference> {
        self.syntax_of
            .get(lexer.name)
            .map(|&i| &self.syntaxes.syntaxes()[i])
    }

    /// Every Chroma lexer name with the syntect syntax standing in for it (`None`: plain
    /// text).
    pub fn mapping(&self) -> impl Iterator<Item = (&'static str, Option<&str>)> {
        let mut lexers: Vec<&'static str> = self
            .by_name
            .values()
            .chain(self.by_file.values())
            .copied()
            .collect();
        lexers.sort_unstable();
        lexers.dedup();
        lexers.into_iter().map(|l| {
            let syntax = self.syntax(Lexer { name: l }).map(|s| s.name.as_str());
            (l, syntax)
        })
    }

    /// A lexer for `code` when its language is unknown (`guessSyntax`): the syntax whose
    /// first-line pattern matches, else plain text.
    pub fn guess(&self, code: &str) -> (String, Option<&SyntaxReference>) {
        let first = code.lines().next().unwrap_or("");
        match self.syntaxes.find_syntax_by_first_line(first) {
            Some(s) => (s.name.to_lowercase(), Some(s)),
            None => ("plaintext".to_owned(), None),
        }
    }
}

/// The syntax matching a Chroma lexer by name, then its aliases, then its file extensions.
fn find_syntax<'s>(
    syntaxes: &'s SyntaxSet,
    lexer: &str,
    keys: &[(&str, bool)],
) -> Option<&'s SyntaxReference> {
    let lower = lexer.to_lowercase();
    syntaxes
        .syntaxes()
        .iter()
        .find(|s| s.name.to_lowercase() == lower)
        .or_else(|| {
            keys.iter()
                .filter(|(_, is_name)| *is_name)
                .find_map(|(k, _)| syntaxes.find_syntax_by_token(k))
        })
        .or_else(|| {
            keys.iter()
                .filter(|(_, is_name)| !is_name)
                .find_map(|(k, _)| syntaxes.find_syntax_by_extension(k))
        })
}
