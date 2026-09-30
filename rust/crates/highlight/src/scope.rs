//! The bridge from syntect scopes to Chroma token types.
//!
//! A token's scope stack is read innermost first; the first scope that matches a rule decides
//! the type (the longest matching prefix wins, atom by atom). [`Rule::Defer`] scopes (the
//! quotes of a string, the `$` of a variable) take the type of an outer scope, as Chroma
//! includes them in that token. [`Rule::Whole`] scopes (comments, doctypes) give their type to
//! everything inside them, as Chroma lexes them as one token. A stack no rule matches is plain
//! text.

use std::collections::HashMap;

use syntect::parsing::Scope;

use crate::token::TokenType;
use crate::token::TokenType as T;

/// What a scope means.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    /// The token has this type.
    Type(TokenType),
    /// This scope and everything inside it has this type.
    Whole(TokenType),
    /// The type comes from an outer scope.
    Defer,
}

use Rule::{Defer, Type, Whole};

/// Scope prefixes and their meaning. `prefix @lang` applies only to scopes whose last atom is
/// `lang` (a syntax's own conventions, where Chroma's lexer for that language differs from the
/// generic choice); such rules outrank generic ones, then the longest prefix wins.
pub const RULES: &[(&str, Rule)] = &[
    // ── comments and preprocessor-like blocks: one token ──
    ("comment", Whole(T::Comment)),
    ("comment.line", Whole(T::CommentSingle)),
    ("comment.block", Whole(T::CommentMultiline)),
    ("meta.tag.sgml.doctype", Whole(T::CommentPreproc)),
    ("meta.tag.preprocessor.xml", Whole(T::CommentPreproc)),
    ("punctuation.definition.comment", Defer),
    // ── strings ──
    ("string", Type(T::LiteralString)),
    ("string.quoted.double", Type(T::LiteralStringDouble)),
    ("string.quoted.single", Type(T::LiteralStringSingle)),
    ("string.quoted.other", Type(T::LiteralString)),
    ("string.quoted.raw", Type(T::LiteralString)),
    ("string.quoted.backtick", Type(T::LiteralStringBacktick)),
    ("string.regexp", Type(T::LiteralStringRegex)),
    ("string.interpolated", Type(T::LiteralStringInterpol)),
    ("string.unquoted.heredoc", Type(T::LiteralStringHeredoc)),
    ("punctuation.definition.string", Defer),
    ("constant.character.escape", Type(T::LiteralStringEscape)),
    ("constant.character.entity", Type(T::NameEntity)),
    ("punctuation.definition.entity", Defer),
    ("punctuation.terminator.entity", Defer),
    ("constant.character", Type(T::LiteralStringChar)),
    // ── numbers and constants ──
    ("constant.numeric", Type(T::LiteralNumber)),
    ("constant.numeric.integer", Type(T::LiteralNumberInteger)),
    (
        "constant.numeric.integer.hexadecimal",
        Type(T::LiteralNumberHex),
    ),
    ("constant.numeric.integer.octal", Type(T::LiteralNumberOct)),
    ("constant.numeric.integer.binary", Type(T::LiteralNumberBin)),
    ("constant.numeric.float", Type(T::LiteralNumberFloat)),
    ("constant.numeric.hex", Type(T::LiteralNumberHex)),
    ("constant.numeric.octal", Type(T::LiteralNumberOct)),
    ("constant.numeric.binary", Type(T::LiteralNumberBin)),
    ("constant.numeric.imaginary", Type(T::LiteralNumber)),
    ("punctuation.separator.decimal", Defer),
    ("punctuation.separator.date", Defer),
    ("punctuation.separator.time", Defer),
    ("punctuation.separator.exponent", Defer),
    ("punctuation.definition.numeric", Defer),
    ("storage.type.numeric", Defer),
    ("constant.language", Type(T::KeywordConstant)),
    ("constant.other", Type(T::NameConstant)),
    ("constant", Type(T::NameConstant)),
    // ── keywords and operators ──
    ("keyword", Type(T::Keyword)),
    ("keyword.control", Type(T::Keyword)),
    ("keyword.declaration", Type(T::KeywordDeclaration)),
    ("keyword.other", Type(T::Keyword)),
    ("keyword.operator", Type(T::Operator)),
    ("keyword.operator.word", Type(T::OperatorWord)),
    ("keyword.operator.logical.python", Type(T::OperatorWord)),
    ("storage", Type(T::Keyword)),
    ("storage.type", Type(T::KeywordType)),
    ("storage.type.function", Type(T::KeywordDeclaration)),
    ("storage.type.class", Type(T::KeywordDeclaration)),
    ("storage.modifier", Type(T::Keyword)),
    // ── names ──
    ("entity.name", Type(T::Name)),
    ("entity.name.function", Type(T::NameFunction)),
    ("entity.name.class", Type(T::NameClass)),
    ("entity.name.type", Type(T::NameClass)),
    ("entity.name.struct", Type(T::NameClass)),
    ("entity.name.enum", Type(T::NameClass)),
    ("entity.name.interface", Type(T::NameClass)),
    ("entity.name.namespace", Type(T::NameNamespace)),
    ("entity.name.module", Type(T::NameNamespace)),
    ("entity.name.label", Type(T::NameLabel)),
    ("entity.name.constant", Type(T::NameConstant)),
    ("entity.name.tag", Type(T::NameTag)),
    ("entity.name.section", Type(T::GenericHeading)),
    ("entity.other.attribute-name", Type(T::NameAttribute)),
    ("entity.other.inherited-class", Type(T::NameClass)),
    ("support.function", Type(T::NameBuiltin)),
    ("support.type", Type(T::KeywordType)),
    ("support.class", Type(T::NameClass)),
    ("support.constant", Type(T::NameConstant)),
    ("support.variable", Type(T::NameVariable)),
    ("variable", Type(T::NameVariable)),
    ("variable.function", Type(T::NameFunction)),
    ("variable.language", Type(T::NameBuiltinPseudo)),
    ("variable.parameter", Type(T::Name)),
    ("variable.other", Type(T::NameVariable)),
    ("punctuation.definition.variable", Defer),
    // ── punctuation ──
    ("punctuation", Type(T::Punctuation)),
    ("punctuation.definition.keyword", Defer),
    // ── markup ──
    ("markup.heading", Type(T::GenericHeading)),
    ("markup.bold", Type(T::GenericStrong)),
    ("markup.italic", Type(T::GenericEmph)),
    ("markup.underline", Type(T::GenericUnderline)),
    ("markup.inserted", Type(T::GenericInserted)),
    ("markup.deleted", Type(T::GenericDeleted)),
    ("markup.raw", Type(T::LiteralStringBacktick)),
    ("meta.diff.header", Type(T::GenericHeading)),
    ("meta.diff.range", Type(T::GenericSubheading)),
    ("invalid", Type(T::Error)),
    // ── Go templates (the crate's own syntaxes, after Chroma's go_template lexer) ──
    ("text.whitespace @go-template", Type(T::TextWhitespace)),
    ("text.other @go-template", Type(T::Other)),
    ("string @go-template", Type(T::LiteralString)),
    ("variable.other @go-template", Type(T::NameOther)),
    ("variable.other.member @go-template", Type(T::NameAttribute)),
    (
        "punctuation.section.embedded @go-template",
        Type(T::CommentPreproc),
    ),
    // ── HTML and XML (Chroma: whole attribute values are strings, `=` an operator) ──
    ("string @html", Whole(T::LiteralString)),
    ("comment @html", Whole(T::Comment)),
    ("punctuation.separator.key-value @html", Type(T::Operator)),
    ("string @xml", Type(T::LiteralString)),
    ("comment @xml", Whole(T::Comment)),
    (
        "punctuation.separator.key-value @xml",
        Type(T::NameAttribute),
    ),
    // ── TOML (Chroma: keys and table names are NameOther) ──
    ("entity.name @toml", Type(T::NameOther)),
    ("constant.other.datetime @toml", Type(T::LiteralDate)),
    ("comment @toml", Whole(T::Comment)),
    ("string.quoted.triple @toml", Type(T::LiteralStringDouble)),
    // ── YAML (Chroma: plain scalars are Literal, block scalars doc strings) ──
    ("string.unquoted.plain @yaml", Type(T::Literal)),
    ("string.unquoted.block @yaml", Type(T::LiteralStringDoc)),
    (
        "keyword.control.flow.block-scalar @yaml",
        Type(T::Punctuation),
    ),
    ("constant.numeric @yaml", Type(T::LiteralNumber)),
    ("constant.other.timestamp @yaml", Type(T::LiteralDate)),
    ("comment @yaml", Whole(T::Comment)),
    ("entity.name.other.anchor @yaml", Type(T::CommentPreproc)),
    ("variable.other.alias @yaml", Type(T::CommentPreproc)),
    ("punctuation.definition.anchor @yaml", Defer),
    ("punctuation.definition.alias @yaml", Defer),
    ("entity.other.document @yaml", Type(T::NameNamespace)),
    (
        "punctuation.definition.block.sequence.item @yaml",
        Type(T::Text),
    ),
    ("punctuation.definition.mapping @yaml", Type(T::Text)),
    ("punctuation.definition.sequence @yaml", Type(T::Text)),
    ("constant.character.escape @yaml", Defer),
    // ── JSON (Chroma: keys are tags) ──
    ("meta.mapping.key @json", Whole(T::NameTag)),
    // ── XML (Chroma: `<`, `>` belong to the tag, `=` to the attribute name) ──
    ("punctuation.definition.tag @xml", Type(T::NameTag)),
    ("punctuation.separator.namespace @xml", Defer),
    // ── Go ──
    ("string @go", Type(T::LiteralString)),
    ("keyword.other.package @go", Type(T::KeywordNamespace)),
    ("keyword.other.import @go", Type(T::KeywordNamespace)),
    ("variable @go", Type(T::NameOther)),
    ("variable.function @go", Type(T::NameFunction)),
    ("storage.type.keyword @go", Type(T::KeywordDeclaration)),
    ("storage.type.keyword.type @go", Type(T::KeywordDeclaration)),
    // ── shell ──
    ("storage.modifier @shell", Type(T::NameBuiltin)),
    ("variable.function @shell", Type(T::Text)),
    ("variable.parameter.option @shell", Type(T::Text)),
    ("punctuation.definition.parameter @shell", Defer),
    ("string.unquoted @shell", Type(T::Text)),
    ("keyword.operator.logical.pipe @shell", Type(T::Punctuation)),
    ("keyword.control.regexp @shell", Type(T::Operator)),
    // ── CSS ──
    ("comment @css", Whole(T::Comment)),
    ("support.type.property-name @css", Type(T::Keyword)),
    // ── JavaScript (Chroma: names are NameOther, declarations reserved words) ──
    ("variable @js", Type(T::NameOther)),
    ("variable.language @js", Type(T::KeywordPseudo)),
    ("support @js", Type(T::NameOther)),
    ("entity.name.function @js", Type(T::NameOther)),
    ("meta.property.object @js", Type(T::NameOther)),
    ("meta.mapping.key @js", Type(T::NameOther)),
    ("storage.type @js", Type(T::KeywordReserved)),
    (
        "keyword.control.import-export @js",
        Type(T::KeywordReserved),
    ),
    ("punctuation.separator.key-value @js", Type(T::Operator)),
    ("constant.character.escape @js", Defer),
    ("support.type.object.dom @js", Type(T::NameBuiltin)),
    // ── TypeScript (Chroma: names are NameOther, declarations reserved words) ──
    ("variable @ts", Type(T::NameOther)),
    ("variable.language @ts", Type(T::KeywordPseudo)),
    ("support @ts", Type(T::NameOther)),
    ("entity.name.function @ts", Type(T::NameOther)),
    ("meta.property.object @ts", Type(T::NameOther)),
    ("meta.mapping.key @ts", Type(T::NameOther)),
    ("storage.type @ts", Type(T::KeywordReserved)),
    (
        "keyword.control.import-export @ts",
        Type(T::KeywordReserved),
    ),
    ("punctuation.separator.key-value @ts", Type(T::Operator)),
    // ── TypeScript (Chroma: names are NameOther, declarations reserved words) ──
    ("variable @tsx", Type(T::NameOther)),
    ("variable.language @tsx", Type(T::KeywordPseudo)),
    ("support @tsx", Type(T::NameOther)),
    ("entity.name.function @tsx", Type(T::NameOther)),
    ("meta.property.object @tsx", Type(T::NameOther)),
    ("meta.mapping.key @tsx", Type(T::NameOther)),
    ("storage.type @tsx", Type(T::KeywordReserved)),
    (
        "keyword.control.import-export @tsx",
        Type(T::KeywordReserved),
    ),
    ("punctuation.separator.key-value @tsx", Type(T::Operator)),
];

/// `(outer, inner, type)`: a scope with prefix `inner` inside one with prefix `outer` is
/// entirely `type`. HTML's `style` and event attribute values embed CSS and JavaScript, which
/// Chroma leaves as one string.
pub const NESTED: &[(&str, &str, TokenType)] = &[
    (
        "meta.attribute-with-value.style",
        "string.quoted",
        T::LiteralString,
    ),
    (
        "meta.attribute-with-value.style",
        "source.css",
        T::LiteralString,
    ),
    (
        "meta.attribute-with-value.event",
        "string.quoted",
        T::LiteralString,
    ),
    (
        "meta.attribute-with-value.event",
        "source.js",
        T::LiteralString,
    ),
];

/// A compiled rule: a scope prefix, optionally only for scopes ending in a language atom.
#[derive(Clone, Copy, Debug)]
struct Compiled {
    prefix: Scope,
    /// The atom a matching scope must end with (`@yaml` in [`RULES`]).
    language: Option<u16>,
    rule: Rule,
}

/// The compiled [`RULES`] and [`NESTED`].
#[derive(Debug)]
pub(crate) struct ScopeMap {
    rules: Vec<Compiled>,
    nested: Vec<(Scope, Scope, TokenType)>,
}

impl ScopeMap {
    /// Compiles [`RULES`].
    ///
    /// # Panics
    /// On a malformed rule (checked by the crate's tests).
    pub fn new() -> Self {
        let scope = |s: &str| Scope::new(s).unwrap_or_else(|e| panic!("rule {s}: {e:?}"));
        let rules = RULES
            .iter()
            .map(|(s, rule)| {
                let (prefix, language) = match s.split_once(" @") {
                    Some((p, l)) => (p, Some(l)),
                    None => (*s, None),
                };
                Compiled {
                    prefix: scope(prefix),
                    language: language.map(|l| scope(l).atom_at(0)),
                    rule: *rule,
                }
            })
            .collect();
        let nested = NESTED
            .iter()
            .map(|(outer, inner, t)| (scope(outer), scope(inner), *t))
            .collect();
        Self { rules, nested }
    }

    /// The [`NESTED`] type of a stack.
    fn nested(&self, stack: &[Scope]) -> Option<TokenType> {
        self.nested.iter().find_map(|(outer, inner, t)| {
            let at = stack.iter().position(|s| outer.is_prefix_of(*s))?;
            stack[at + 1..]
                .iter()
                .any(|s| inner.is_prefix_of(*s))
                .then_some(*t)
        })
    }

    /// The rule of `scope`: language-specific rules first, then the longest prefix.
    fn rule(&self, scope: Scope) -> Option<Rule> {
        let len = scope.len() as usize;
        let last = len.checked_sub(1).map(|i| scope.atom_at(i));
        self.rules
            .iter()
            .filter(|c| c.prefix.is_prefix_of(scope))
            .filter(|c| c.language.is_none() || c.language == last)
            .max_by_key(|c| (c.language.is_some(), c.prefix.len()))
            .map(|c| c.rule)
    }
}

/// Classifies scope stacks for one piece of code, caching per scope and per stack.
pub(crate) struct Classifier<'m> {
    map: &'m ScopeMap,
    by_scope: HashMap<Scope, Option<Rule>>,
    by_stack: HashMap<Vec<Scope>, TokenType>,
}

impl<'m> Classifier<'m> {
    pub fn new(map: &'m ScopeMap) -> Self {
        Self {
            map,
            by_scope: HashMap::new(),
            by_stack: HashMap::new(),
        }
    }

    fn rule(&mut self, scope: Scope) -> Option<Rule> {
        let map = self.map;
        *self
            .by_scope
            .entry(scope)
            .or_insert_with(|| map.rule(scope))
    }

    /// The token type of a scope stack (outermost first).
    pub fn classify(&mut self, stack: &[Scope]) -> TokenType {
        if let Some(t) = self.by_stack.get(stack) {
            return *t;
        }
        let whole = self.map.nested(stack).or_else(|| {
            stack.iter().find_map(|&s| match self.rule(s) {
                Some(Whole(t)) => Some(t),
                _ => None,
            })
        });
        let t = whole.unwrap_or_else(|| {
            stack
                .iter()
                .rev()
                .find_map(|&s| match self.rule(s) {
                    Some(Type(t) | Whole(t)) => Some(t),
                    Some(Defer) | None => None,
                })
                .unwrap_or(TokenType::Text)
        });
        self.by_stack.insert(stack.to_vec(), t);
        t
    }
}
