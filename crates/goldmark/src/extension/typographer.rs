// Go: github.com/yuin/goldmark@v1.7.12/extension/typographer.go

use std::sync::{Arc, LazyLock};

use go_unicode as unicode;

use crate::ast::{Ast, NodeId};
use crate::parser::{
    self, Context, ContextKey, Delimiter, DelimiterProcessor, InlineParser, OptionValue,
    ParserOption,
};
use crate::text::Reader;
use crate::util;
use crate::{Extender, Markdown};

static UNCLOSE_COUNTER_KEY: LazyLock<ContextKey> = LazyLock::new(parser::new_context_key);

#[derive(Debug, Default)]
struct UnclosedCounter {
    single: i64,
    double: i64,
}

impl UnclosedCounter {
    // Go: extension/typographer.go:unclosedCounter.Reset
    fn reset(&mut self) {
        self.single = 0;
        self.double = 0;
    }
}

// Go: extension/typographer.go:getUnclosedCounter
fn get_unclosed_counter(pc: &mut Context) -> &mut UnclosedCounter {
    pc.compute_if_absent(
        *UNCLOSE_COUNTER_KEY,
        || Box::new(UnclosedCounter::default()),
    )
    .downcast_mut::<UnclosedCounter>()
    .expect("unclosed counter")
}

/// TypographicPunctuation is a key of the punctuations that can be replaced with
/// typographic entities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TypographicPunctuation {
    /// LeftSingleQuote is ' .
    LeftSingleQuote = 1,
    /// RightSingleQuote is ' .
    RightSingleQuote = 2,
    /// LeftDoubleQuote is " .
    LeftDoubleQuote = 3,
    /// RightDoubleQuote is " .
    RightDoubleQuote = 4,
    /// EnDash is -- .
    EnDash = 5,
    /// EmDash is --- .
    EmDash = 6,
    /// Ellipsis is ... .
    Ellipsis = 7,
    /// LeftAngleQuote is << .
    LeftAngleQuote = 8,
    /// RightAngleQuote is >> .
    RightAngleQuote = 9,
    /// Apostrophe is ' .
    Apostrophe = 10,
}

use TypographicPunctuation::*;

const TYPOGRAPHIC_PUNCTUATION_MAX: usize = 11;

/// The substitutions indexed by [`TypographicPunctuation`] (Go `[][]byte`;
/// `None` is a nil entry, which disables the substitution).
pub type Substitutions = Vec<Option<Vec<u8>>>;

/// An TypographerConfig struct is a data structure that holds configuration of the
/// Typographer extension.
#[derive(Debug, Clone)]
pub struct TypographerConfig {
    pub substitutions: Substitutions,
}

// Go: extension/typographer.go:newDefaultSubstitutions
fn new_default_substitutions() -> Substitutions {
    let mut replacements: Substitutions = vec![None; TYPOGRAPHIC_PUNCTUATION_MAX];
    replacements[LeftSingleQuote as usize] = Some(b"&lsquo;".to_vec());
    replacements[RightSingleQuote as usize] = Some(b"&rsquo;".to_vec());
    replacements[LeftDoubleQuote as usize] = Some(b"&ldquo;".to_vec());
    replacements[RightDoubleQuote as usize] = Some(b"&rdquo;".to_vec());
    replacements[EnDash as usize] = Some(b"&ndash;".to_vec());
    replacements[EmDash as usize] = Some(b"&mdash;".to_vec());
    replacements[Ellipsis as usize] = Some(b"&hellip;".to_vec());
    replacements[LeftAngleQuote as usize] = Some(b"&laquo;".to_vec());
    replacements[RightAngleQuote as usize] = Some(b"&raquo;".to_vec());
    replacements[Apostrophe as usize] = Some(b"&rsquo;".to_vec());

    replacements
}

const OPT_TYPOGRAPHIC_SUBSTITUTIONS: &str = "TypographicSubstitutions";

impl TypographerConfig {
    // Go: extension/typographer.go:TypographerConfig.SetOption
    /// SetOption implements SetOptioner.
    pub fn set_option(&mut self, name: &str, value: &OptionValue) {
        if name == OPT_TYPOGRAPHIC_SUBSTITUTIONS {
            self.substitutions = value
                .downcast_ref::<Substitutions>()
                .expect("interface conversion: not [][]uint8")
                .clone();
        }
    }
}

/// A TypographerOption interface sets options for the TypographerParser
/// (Go: `withTypographicSubstitutions`, the only implementation).
#[derive(Debug, Clone)]
pub struct TypographerOption {
    value: Substitutions,
}

impl TypographerOption {
    /// Go: `SetTypographerOption`.
    pub fn set_typographer_option(&self, p: &mut TypographerConfig) {
        p.substitutions = self.value.clone();
    }
}

impl ParserOption for TypographerOption {
    fn set_parser_option(self: Box<Self>, c: &mut parser::Config) {
        c.options.insert(
            OPT_TYPOGRAPHIC_SUBSTITUTIONS.to_string(),
            Arc::new(self.value),
        );
    }
}

// Go: extension/typographer.go:WithTypographicSubstitutions
/// WithTypographicSubstitutions is a functional otpion that specify replacement text
/// for punctuations. (Go takes a `map[TypographicPunctuation]T`; the keys are
/// distinct, so the order of the pairs does not matter.)
pub fn with_typographic_substitutions<T: AsRef<[u8]>>(
    values: &[(TypographicPunctuation, T)],
) -> TypographerOption {
    let mut replacements = new_default_substitutions();
    for (k, v) in values {
        // Go: []byte(v) of a string is never nil, even when empty.
        replacements[*k as usize] = Some(v.as_ref().to_vec());
    }

    TypographerOption {
        value: replacements,
    }
}

struct TypographerDelimiterProcessor;

impl DelimiterProcessor for TypographerDelimiterProcessor {
    // Go: extension/typographer.go:typographerDelimiterProcessor.IsDelimiter
    fn is_delimiter(&self, b: u8) -> bool {
        b == b'\'' || b == b'"'
    }

    // Go: extension/typographer.go:typographerDelimiterProcessor.CanOpenCloser
    fn can_open_closer(&self, opener: &Delimiter, closer: &Delimiter) -> bool {
        opener.char == closer.char
    }

    // Go: extension/typographer.go:typographerDelimiterProcessor.OnMatch
    fn on_match(&self, _ast: &mut Ast, _consumes: i64) -> NodeId {
        // Go returns nil. The typographer never pushes its delimiters, so
        // this is never called.
        unreachable!("typographer delimiters are never pushed")
    }
}

static DEFAULT_TYPOGRAPHER_DELIMITER_PROCESSOR: LazyLock<Arc<dyn DelimiterProcessor>> =
    LazyLock::new(|| Arc::new(TypographerDelimiterProcessor));

struct TypographerParser {
    config: TypographerConfig,
}

// Go: extension/typographer.go:NewTypographerParser
/// NewTypographerParser return a new InlineParser that parses
/// typographer expressions.
pub fn new_typographer_parser(opts: &[TypographerOption]) -> Box<dyn InlineParser> {
    let mut p = TypographerParser {
        config: TypographerConfig {
            substitutions: new_default_substitutions(),
        },
    };
    for o in opts {
        o.set_typographer_option(&mut p.config);
    }
    Box::new(p)
}

impl TypographerParser {
    fn sub(&self, p: TypographicPunctuation) -> Option<&Vec<u8>> {
        self.config.substitutions[p as usize].as_ref()
    }

    /// `node := gast.NewString(s.Substitutions[p]); node.SetCode(true)`
    fn new_code_string(&self, ast: &mut Ast, p: TypographicPunctuation) -> NodeId {
        let v = self.config.substitutions[p as usize]
            .clone()
            .unwrap_or_default();
        let node = ast.new_string(v);
        ast.string_node_mut(node).unwrap().set_code(true);
        node
    }
}

impl TypographerParser {
    // Go: extension/typographer.go:typographerParser.CloseBlock
    /// Never called (see the note in the InlineParser impl).
    #[allow(dead_code)]
    fn close_block(&self, _parent: NodeId, pc: &mut Context) {
        get_unclosed_counter(pc).reset();
    }
}

impl InlineParser for TypographerParser {
    // Go: extension/typographer.go:typographerParser.Trigger
    fn trigger(&self) -> &[u8] {
        b"'\"-.,<>*["
    }

    // Go: extension/typographer.go:typographerParser.Parse
    fn parse<'a>(
        &self,
        ast: &mut Ast,
        _parent: NodeId,
        block: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) -> Option<NodeId> {
        let (line, _) = block.peek_line();
        let line = line.unwrap_or_default();
        let c = line[0];
        if line.len() > 2 {
            if c == b'-' {
                if self.sub(EmDash).is_some() && line[1] == b'-' && line[2] == b'-' {
                    // ---
                    let node = self.new_code_string(ast, EmDash);
                    block.advance(3);
                    return Some(node);
                }
            } else if c == b'.' {
                if self.sub(Ellipsis).is_some() && line[1] == b'.' && line[2] == b'.' {
                    // ...
                    let node = self.new_code_string(ast, Ellipsis);
                    block.advance(3);
                    return Some(node);
                }
                return None;
            }
        }
        if line.len() > 1 {
            if c == b'<' {
                if self.sub(LeftAngleQuote).is_some() && line[1] == b'<' {
                    // <<
                    let node = self.new_code_string(ast, LeftAngleQuote);
                    block.advance(2);
                    return Some(node);
                }
                return None;
            } else if c == b'>' {
                if self.sub(RightAngleQuote).is_some() && line[1] == b'>' {
                    // >>
                    let node = self.new_code_string(ast, RightAngleQuote);
                    block.advance(2);
                    return Some(node);
                }
                return None;
            } else if self.sub(EnDash).is_some() && c == b'-' && line[1] == b'-' {
                // --
                let node = self.new_code_string(ast, EnDash);
                block.advance(2);
                return Some(node);
            }
        }
        if c == b'\'' || c == b'"' {
            let before = block.precending_character();
            let d = parser::scan_delimiter(
                &line,
                before,
                1,
                DEFAULT_TYPOGRAPHER_DELIMITER_PROCESSOR.clone(),
            )?;
            if c == b'\'' {
                if self.sub(Apostrophe).is_some() {
                    // Handle decade abbrevations such as '90s
                    if d.can_open
                        && !d.can_close
                        && line.len() > 3
                        && util::is_numeric(line[1])
                        && util::is_numeric(line[2])
                        && line[3] == b's'
                    {
                        let mut after = ' ' as i32;
                        if line.len() > 4 {
                            after = util::to_rune(&line, 4);
                        }
                        if line.len() == 3
                            || util::is_space_rune(after)
                            || util::is_punct_rune(after)
                        {
                            let node = self.new_code_string(ast, Apostrophe);
                            block.advance(1);
                            return Some(node);
                        }
                    }
                    // special cases: 'twas, 'em, 'net
                    if line.len() > 1
                        && (unicode::is_punct(before) || unicode::is_space(before))
                        && (line[1] == b't'
                            || line[1] == b'e'
                            || line[1] == b'n'
                            || line[1] == b'l')
                    {
                        let node = self.new_code_string(ast, Apostrophe);
                        block.advance(1);
                        return Some(node);
                    }
                    // Convert normal apostrophes. This is probably more flexible than necessary but
                    // converts any apostrophe in between two alphanumerics.
                    if line.len() > 1
                        && (unicode::is_digit(before) || unicode::is_letter(before))
                        && (unicode::is_letter(util::to_rune(&line, 1)))
                    {
                        let node = self.new_code_string(ast, Apostrophe);
                        block.advance(1);
                        return Some(node);
                    }
                }
                if self.sub(LeftSingleQuote).is_some() && d.can_open && !d.can_close {
                    let mut nt = LeftSingleQuote;
                    // special cases: Alice's, I'm, Don't, You'd
                    if line.len() > 1
                        && (line[1] == b's'
                            || line[1] == b'm'
                            || line[1] == b't'
                            || line[1] == b'd')
                        && (line.len() < 3 || util::is_punct(line[2]) || util::is_space(line[2]))
                    {
                        nt = RightSingleQuote;
                    }
                    // special cases: I've, I'll, You're
                    if line.len() > 2
                        && ((line[1] == b'v' && line[2] == b'e')
                            || (line[1] == b'l' && line[2] == b'l')
                            || (line[1] == b'r' && line[2] == b'e'))
                        && (line.len() < 4 || util::is_punct(line[3]) || util::is_space(line[3]))
                    {
                        nt = RightSingleQuote;
                    }
                    if nt == LeftSingleQuote {
                        get_unclosed_counter(pc).single += 1;
                    }

                    let node = self.new_code_string(ast, nt);
                    block.advance(1);
                    return Some(node);
                }
                if self.sub(RightSingleQuote).is_some() {
                    // plural possesive and abbreviations: Smiths', doin'
                    //
                    // Go's operator precedence makes this
                    // `(len>1 && IsSpace(line[0])) || (IsPunct(line[0]) && (len>2 && !IsDigit(line[1])))`
                    // (ported as written).
                    if line.len() > 1 && unicode::is_space(util::to_rune(&line, 0))
                        || unicode::is_punct(util::to_rune(&line, 0))
                            && (line.len() > 2 && !unicode::is_digit(util::to_rune(&line, 1)))
                    {
                        let node = self.new_code_string(ast, RightSingleQuote);
                        block.advance(1);
                        return Some(node);
                    }
                }
                if self.sub(RightSingleQuote).is_some() && get_unclosed_counter(pc).single > 0 {
                    let is_close = d.can_close && !d.can_open;
                    // Go precedence: `len==2 || ((len>2 && IsPunct(line[2])) || IsSpace(line[2]))`
                    let maybe_close = d.can_close
                        && d.can_open
                        && line.len() > 1
                        && unicode::is_punct(util::to_rune(&line, 1))
                        && (line.len() == 2
                            || (line.len() > 2 && util::is_punct(line[2])
                                || util::is_space(line[2])));
                    if is_close || maybe_close {
                        let node = self.new_code_string(ast, RightSingleQuote);
                        block.advance(1);
                        get_unclosed_counter(pc).single -= 1;
                        return Some(node);
                    }
                }
            }
            if c == b'"' {
                if self.sub(LeftDoubleQuote).is_some() && d.can_open && !d.can_close {
                    let node = self.new_code_string(ast, LeftDoubleQuote);
                    block.advance(1);
                    get_unclosed_counter(pc).double += 1;
                    return Some(node);
                }
                if self.sub(RightDoubleQuote).is_some() && get_unclosed_counter(pc).double > 0 {
                    let is_close = d.can_close && !d.can_open;
                    let maybe_close = d.can_close
                        && d.can_open
                        && line.len() > 1
                        && (unicode::is_punct(util::to_rune(&line, 1)))
                        && (line.len() == 2
                            || (line.len() > 2 && util::is_punct(line[2])
                                || util::is_space(line[2])));
                    if is_close || maybe_close {
                        // special case: "Monitor 21""
                        if line.len() > 1 && line[1] == b'"' && unicode::is_digit(before) {
                            return None;
                        }
                        let node = self.new_code_string(ast, RightDoubleQuote);
                        block.advance(1);
                        get_unclosed_counter(pc).double -= 1;
                        return Some(node);
                    }
                }
            }
        }
        None
    }

    // Go: extension/typographer.go:typographerParser.CloseBlock(parent, pc)
    // does not match parser.CloseBlocker (whose CloseBlock also takes a
    // text.Reader), so Go never registers the typographer as a close
    // blocker: the unclosed-quote counters are NOT reset per block and live
    // for the whole document (one parser.Context). Ported as is: this
    // parser is not a close blocker, see `TypographerParser::close_block`.

    // Go: the embedded TypographerConfig's SetOption
    fn set_option(&mut self, name: &str, value: &OptionValue) {
        self.config.set_option(name, value);
    }
}

/// Go: `type typographer struct{ options []TypographerOption }`.
pub struct TypographerExt {
    options: Vec<TypographerOption>,
}

// Go: extension/typographer.go:Typographer
/// Typographer is an extension that replaces punctuations with typographic entities.
pub fn typographer() -> Box<dyn Extender> {
    Box::new(TypographerExt {
        options: Vec::new(),
    })
}

// Go: extension/typographer.go:NewTypographer
/// NewTypographer returns a new Extender that replaces punctuations with typographic entities.
pub fn new_typographer(opts: Vec<TypographerOption>) -> Box<dyn Extender> {
    Box::new(TypographerExt { options: opts })
}

impl Extender for TypographerExt {
    // Go: extension/typographer.go:typographer.Extend
    fn extend(&self, m: &mut Markdown) {
        m.parser()
            .add_options(vec![parser::with_inline_parsers(vec![util::prioritized(
                new_typographer_parser(&self.options),
                9999,
            )])]);
    }
}
