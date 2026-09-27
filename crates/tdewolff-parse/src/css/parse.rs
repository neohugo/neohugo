//! Go: parse/css/parse.go — the CSS grammar state machine.

use crate::buffer::Reader;
use crate::css::hash::{Document, Font_Face, Keyframes, Media, Page, Supports, to_hash};
use crate::css::lex::*;
use crate::error::{GoError, new_error};
use crate::gobytes::GoBytes;
use crate::input::Input;
use crate::util::{copy, to_lower};

static WS_BYTES: &[u8] = b" ";
static END_BYTES: &[u8] = b"}";
static EMPTY_BYTES: &[u8] = b"";

/// Go: css.GrammarType — the type of grammar.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GrammarType {
    ErrorGrammar = 0, // extra token when errors occur
    CommentGrammar,
    AtRuleGrammar,
    BeginAtRuleGrammar,
    EndAtRuleGrammar,
    QualifiedRuleGrammar,
    BeginRulesetGrammar,
    EndRulesetGrammar,
    DeclarationGrammar,
    TokenGrammar,
    CustomPropertyGrammar,
}

pub use GrammarType::*;

impl GrammarType {
    // Go: parse/css/parse.go:GrammarType.String
    /// Returns the string representation of a GrammarType.
    pub fn string(self) -> String {
        grammar_type_string(self as u32)
    }
}

/// `GrammarType(n).String()` for any integer value.
pub fn grammar_type_string(n: u32) -> String {
    match n {
        0 => "Error",
        1 => "Comment",
        2 => "AtRule",
        3 => "BeginAtRule",
        4 => "EndAtRule",
        5 => "QualifiedRule",
        6 => "BeginRuleset",
        7 => "EndRuleset",
        8 => "Declaration",
        9 => "Token",
        10 => "CustomProperty",
        _ => return format!("Invalid({})", n),
    }
    .to_string()
}

/// Go: css.State — the state function the parser currently is in.
pub type State = fn(&mut Parser) -> GrammarType;

/// Go: css.Token — a single TokenType and its associated data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub token_type: TokenType,
    pub data: GoBytes,
}

impl Token {
    pub fn new(token_type: TokenType, data: GoBytes) -> Token {
        Token { token_type, data }
    }

    // Go: parse/css/parse.go:Token.String
    pub fn string(&self) -> Vec<u8> {
        let mut s = self.token_type.string().into_bytes();
        s.extend_from_slice(b"('");
        self.data.write_to(&mut s);
        s.extend_from_slice(b"')");
        s
    }
}

/// Go: css.Parser — the state for the parser.
pub struct Parser {
    l: Lexer,
    state: Vec<State>,
    err: Vec<u8>, // Go string, "" = no error
    err_pos: usize,

    buf: Vec<Token>,
    level: isize,

    data: GoBytes,
    tt: TokenType,
    keep_ws: bool,
    prev_ws: bool,
    prev_end: bool,
    prev_comment: bool,
}

impl Parser {
    // Go: parse/css/parse.go:NewParser
    /// Returns a new CSS parser. `is_inline` specifies whether this is an
    /// inline style attribute.
    pub fn new(r: Input, is_inline: bool) -> Parser {
        let l = Lexer::new(r);
        let mut p = Parser {
            l,
            state: Vec::with_capacity(4),
            err: Vec::new(),
            err_pos: 0,
            buf: Vec::new(),
            level: 0,
            data: GoBytes::nil(),
            tt: ErrorToken,
            keep_ws: false,
            prev_ws: false,
            prev_end: false,
            prev_comment: false,
        };

        if is_inline {
            p.state.push(Parser::parse_declaration_list);
        } else {
            p.state.push(Parser::parse_stylesheet);
        }
        p
    }

    /// The underlying `*parse.Input` (shared handle).
    pub fn input(&self) -> &Input {
        &self.l.r
    }

    // Go: parse/css/parse.go:Parser.HasParseError
    /// Returns true if there is a parse error (and not a read error).
    pub fn has_parse_error(&self) -> bool {
        !self.err.is_empty()
    }

    // Go: parse/css/parse.go:Parser.Err
    /// Returns the error encountered during parsing, this is often io.EOF but
    /// also other errors can be returned.
    pub fn err(&self) -> Option<GoError> {
        if !self.err.is_empty() {
            let mut r = Reader::new(self.l.r.bytes());
            return Some(GoError::Parse(Box::new(new_error(
                Some(&mut r),
                self.err_pos as isize,
                self.err.clone(),
            ))));
        }
        self.l.err()
    }

    // Go: parse/css/parse.go:Parser.Next
    /// Returns the next Grammar. It returns ErrorGrammar when an error was
    /// encountered. Using Err() one can retrieve the error message.
    pub fn next(&mut self) -> (GrammarType, TokenType, GoBytes) {
        self.err.clear();

        if self.prev_end {
            self.tt = RightBraceToken;
            self.data = GoBytes::from_static(END_BYTES);
            self.prev_end = false;
        } else {
            (self.tt, self.data) = self.pop_token(true);
        }
        let state = *self.state.last().expect("state");
        let gt = state(self);
        (gt, self.tt, self.data.clone())
    }

    // Go: parse/css/parse.go:Parser.Offset
    /// Returns the offset for the current Grammar.
    pub fn offset(&self) -> usize {
        self.l.r.offset()
    }

    // Go: parse/css/parse.go:Parser.Values
    /// Returns the Tokens for the last Grammar. Only AtRuleGrammar,
    /// BeginAtRuleGrammar, BeginRulesetGrammar and Declaration will return the
    /// at-rule components, ruleset selector and declaration values
    /// respectively. (Go returns the internal buffer, which the next grammar
    /// overwrites.)
    pub fn values(&self) -> &[Token] {
        &self.buf
    }

    /// Mutable access to the internal buffer returned by `Values()` (Go
    /// callers may write through the returned slice).
    pub fn values_mut(&mut self) -> &mut Vec<Token> {
        &mut self.buf
    }

    // Go: parse/css/parse.go:Parser.popToken
    fn pop_token(&mut self, allow_comment: bool) -> (TokenType, GoBytes) {
        self.prev_ws = false;
        self.prev_comment = false;
        let (mut tt, mut data) = self.l.next();
        while !self.keep_ws && tt == WhitespaceToken || tt == CommentToken {
            if tt == WhitespaceToken {
                self.prev_ws = true;
            } else {
                self.prev_comment = true;
                if allow_comment && self.state.len() == 1 {
                    break;
                }
            }
            (tt, data) = self.l.next();
        }
        (tt, data)
    }

    // Go: parse/css/parse.go:Parser.initBuf
    fn init_buf(&mut self) {
        self.buf.clear();
    }

    // Go: parse/css/parse.go:Parser.pushBuf
    fn push_buf(&mut self, tt: TokenType, data: GoBytes) {
        self.buf.push(Token {
            token_type: tt,
            data,
        });
    }

    ////////////////////////////////////////////////////////////////

    // Go: parse/css/parse.go:Parser.parseStylesheet
    fn parse_stylesheet(&mut self) -> GrammarType {
        if self.tt == CDOToken || self.tt == CDCToken {
            return TokenGrammar;
        } else if self.tt == AtKeywordToken {
            return self.parse_at_rule();
        } else if self.tt == CommentToken {
            return CommentGrammar;
        } else if self.tt == ErrorToken {
            return ErrorGrammar;
        }
        self.parse_qualified_rule()
    }

    // Go: parse/css/parse.go:Parser.parseDeclarationList
    fn parse_declaration_list(&mut self) -> GrammarType {
        if self.tt == CommentToken {
            (self.tt, self.data) = self.pop_token(false);
        }
        while self.tt == SemicolonToken {
            (self.tt, self.data) = self.pop_token(false);
        }

        // IE hack: *color:red;
        if self.tt == DelimToken && self.data.at(0) == b'*' {
            let (tt, data) = self.pop_token(false);
            self.tt = tt;
            self.data = self.data.append_bytes(&data);
        }

        if self.tt == ErrorToken {
            return ErrorGrammar;
        } else if self.tt == AtKeywordToken {
            return self.parse_at_rule();
        } else if self.tt == IdentToken || self.tt == DelimToken {
            return self.parse_declaration();
        } else if self.tt == CustomPropertyNameToken {
            return self.parse_custom_property();
        }

        // parse error
        self.init_buf();
        self.l.r.move_(-(self.data.len() as isize));
        let mut msg = b"unexpected token '".to_vec();
        self.data.write_to(&mut msg);
        msg.extend_from_slice(b"' in declaration");
        self.err = msg;
        self.err_pos = self.l.r.offset();
        self.l.r.move_(self.data.len() as isize);

        if self.tt == RightBraceToken {
            // right brace token will occur when we've had a decl error that ended in a right brace token
            // as these are not handled by decl error, we handle it here explicitly. Normally its used to end eg. the qual rule.
            self.push_buf(self.tt, self.data.clone());
            return ErrorGrammar;
        }
        self.parse_declaration_error(self.tt, self.data.clone())
    }

    ////////////////////////////////////////////////////////////////

    // Go: parse/css/parse.go:Parser.parseAtRule
    fn parse_at_rule(&mut self) -> GrammarType {
        self.init_buf();
        self.data = to_lower(copy(&self.data));
        let mut at_rule_name = self.data.clone();
        if at_rule_name.len() > 0 && at_rule_name.at(1) == b'-' {
            let i = at_rule_name.slice_from(2).index_byte(b'-');
            if i != -1 {
                at_rule_name = at_rule_name.slice_from(i as usize + 2); // skip vendor specific prefix
            }
        }
        let at_rule = to_hash(&at_rule_name.slice_from(1));

        let mut first = true;
        let mut skip_ws = false;
        loop {
            let (tt, data) = self.pop_token(false);
            if tt == LeftBraceToken && self.level == 0 {
                if at_rule == Font_Face || at_rule == Page {
                    self.state.push(Parser::parse_at_rule_declaration_list);
                } else if at_rule == Document
                    || at_rule == Keyframes
                    || at_rule == Media
                    || at_rule == Supports
                {
                    self.state.push(Parser::parse_at_rule_rule_list);
                } else {
                    self.state.push(Parser::parse_at_rule_unknown);
                }
                return BeginAtRuleGrammar;
            } else if (tt == SemicolonToken || tt == RightBraceToken) && self.level == 0
                || tt == ErrorToken
            {
                self.prev_end = tt == RightBraceToken;
                return AtRuleGrammar;
            } else if tt == LeftParenthesisToken
                || tt == LeftBraceToken
                || tt == LeftBracketToken
                || tt == FunctionToken
            {
                self.level += 1;
            } else if tt == RightParenthesisToken
                || tt == RightBraceToken
                || tt == RightBracketToken
            {
                if self.level == 0 {
                    // TODO: buggy
                    self.push_buf(tt, data);
                    if 1 < self.state.len() {
                        self.state.pop();
                    }
                    self.err = b"unexpected ending in at rule".to_vec();
                    self.err_pos = self.l.r.offset();
                    return ErrorGrammar;
                }
                self.level -= 1;
            }
            if first {
                if tt == LeftParenthesisToken || tt == LeftBracketToken {
                    self.prev_ws = false;
                }
                first = false;
            }
            if data.len() == 1 && (data.at(0) == b',' || data.at(0) == b':') {
                skip_ws = true;
            } else if self.prev_ws && !skip_ws && tt != RightParenthesisToken {
                self.push_buf(WhitespaceToken, GoBytes::from_static(WS_BYTES));
            } else {
                skip_ws = false;
            }
            if tt == LeftParenthesisToken {
                skip_ws = true;
            }
            self.push_buf(tt, data);
        }
    }

    // Go: parse/css/parse.go:Parser.parseAtRuleRuleList
    fn parse_at_rule_rule_list(&mut self) -> GrammarType {
        if self.tt == RightBraceToken || self.tt == ErrorToken {
            self.state.pop();
            EndAtRuleGrammar
        } else if self.tt == AtKeywordToken {
            self.parse_at_rule()
        } else {
            self.parse_qualified_rule()
        }
    }

    // Go: parse/css/parse.go:Parser.parseAtRuleDeclarationList
    fn parse_at_rule_declaration_list(&mut self) -> GrammarType {
        while self.tt == SemicolonToken {
            (self.tt, self.data) = self.pop_token(false);
        }
        if self.tt == RightBraceToken || self.tt == ErrorToken {
            self.state.pop();
            return EndAtRuleGrammar;
        }
        self.parse_declaration_list()
    }

    // Go: parse/css/parse.go:Parser.parseAtRuleUnknown
    fn parse_at_rule_unknown(&mut self) -> GrammarType {
        self.keep_ws = true;
        if self.tt == RightBraceToken && self.level == 0 || self.tt == ErrorToken {
            self.state.pop();
            self.keep_ws = false;
            return EndAtRuleGrammar;
        }
        if self.tt == LeftParenthesisToken
            || self.tt == LeftBraceToken
            || self.tt == LeftBracketToken
            || self.tt == FunctionToken
        {
            self.level += 1;
        } else if self.tt == RightParenthesisToken
            || self.tt == RightBraceToken
            || self.tt == RightBracketToken
        {
            self.level -= 1;
        }
        TokenGrammar
    }

    // Go: parse/css/parse.go:Parser.parseQualifiedRule
    fn parse_qualified_rule(&mut self) -> GrammarType {
        self.init_buf();
        let mut first = true;
        let mut in_attr_sel = false;
        let mut skip_ws = true;
        let mut tt: TokenType;
        let mut data: GoBytes;
        loop {
            if first {
                tt = self.tt;
                data = self.data.clone();
                self.tt = WhitespaceToken;
                self.data = GoBytes::from_static(EMPTY_BYTES);
                first = false;
            } else {
                (tt, data) = self.pop_token(false);
            }
            if tt == LeftBraceToken && self.level == 0 {
                self.state
                    .push(Parser::parse_qualified_rule_declaration_list);
                return BeginRulesetGrammar;
            } else if tt == ErrorToken {
                self.err = b"unexpected ending in qualified rule".to_vec();
                self.err_pos = self.l.r.offset();
                return ErrorGrammar;
            } else if tt == LeftParenthesisToken
                || tt == LeftBraceToken
                || tt == LeftBracketToken
                || tt == FunctionToken
            {
                self.level += 1;
            } else if tt == RightParenthesisToken
                || tt == RightBraceToken
                || tt == RightBracketToken
            {
                if self.level == 0 {
                    // TODO: buggy
                    self.push_buf(tt, data);
                    if 1 < self.state.len() {
                        self.state.pop();
                    }
                    self.err = b"unexpected ending in qualified rule".to_vec();
                    self.err_pos = self.l.r.offset();
                    return ErrorGrammar;
                }
                self.level -= 1;
            }
            if data.len() == 1
                && (data.at(0) == b','
                    || data.at(0) == b'>'
                    || data.at(0) == b'+'
                    || data.at(0) == b'~')
            {
                if data.at(0) == b',' {
                    return QualifiedRuleGrammar;
                }
                skip_ws = true;
            } else if self.prev_ws && !skip_ws && !in_attr_sel {
                self.push_buf(WhitespaceToken, GoBytes::from_static(WS_BYTES));
            } else {
                skip_ws = false;
            }
            if tt == LeftBracketToken {
                in_attr_sel = true;
            } else if tt == RightBracketToken {
                in_attr_sel = false;
            }
            self.push_buf(tt, data);
        }
    }

    // Go: parse/css/parse.go:Parser.parseQualifiedRuleDeclarationList
    fn parse_qualified_rule_declaration_list(&mut self) -> GrammarType {
        while self.tt == SemicolonToken {
            (self.tt, self.data) = self.pop_token(false);
        }
        if self.tt == RightBraceToken || self.tt == ErrorToken {
            self.state.pop();
            return EndRulesetGrammar;
        }
        self.parse_declaration_list()
    }

    // Go: parse/css/parse.go:Parser.parseDeclaration
    fn parse_declaration(&mut self) -> GrammarType {
        self.init_buf();
        self.data = to_lower(copy(&self.data));

        let (tt_name, data_name) = (self.tt, self.data.clone());
        let (tt, data) = self.pop_token(false);
        if tt != ColonToken {
            self.l.r.move_(-(data.len() as isize));
            self.err = b"expected colon in declaration".to_vec();
            self.err_pos = self.l.r.offset();
            self.l.r.move_(data.len() as isize);
            self.push_buf(tt_name, data_name);
            return self.parse_declaration_error(tt, data);
        }

        let mut skip_ws = true;
        loop {
            let (tt, data) = self.pop_token(false);
            if (tt == SemicolonToken || tt == RightBraceToken) && self.level == 0
                || tt == ErrorToken
            {
                self.prev_end = tt == RightBraceToken;
                return DeclarationGrammar;
            } else if tt == LeftParenthesisToken
                || tt == LeftBraceToken
                || tt == LeftBracketToken
                || tt == FunctionToken
            {
                self.level += 1;
            } else if tt == RightParenthesisToken
                || tt == RightBraceToken
                || tt == RightBracketToken
            {
                if self.level == 0 {
                    // TODO: buggy
                    self.err = b"unexpected ending in declaration".to_vec();
                    self.err_pos = self.l.r.offset();
                    self.push_buf(tt_name, data_name);
                    self.push_buf(ColonToken, GoBytes::from_slice(b":"));
                    return self.parse_declaration_error(tt, data);
                }
                self.level -= 1;
            }
            if data.len() == 1
                && (data.at(0) == b','
                    || data.at(0) == b'/'
                    || data.at(0) == b':'
                    || data.at(0) == b'!'
                    || data.at(0) == b'=')
            {
                skip_ws = true;
            } else if (self.prev_ws || self.prev_comment) && !skip_ws {
                self.push_buf(WhitespaceToken, GoBytes::from_static(WS_BYTES));
            } else {
                skip_ws = false;
            }
            self.push_buf(tt, data);
        }
    }

    // Go: parse/css/parse.go:Parser.parseDeclarationError
    fn parse_declaration_error(&mut self, mut tt: TokenType, mut data: GoBytes) -> GrammarType {
        // we're on the offending (tt,data), keep popping tokens till we reach ;, }, or EOF
        self.tt = tt;
        self.data = data.clone();
        loop {
            if (tt == SemicolonToken || tt == RightBraceToken) && self.level == 0
                || tt == ErrorToken
            {
                self.prev_end = tt == RightBraceToken;
                if tt == SemicolonToken {
                    self.push_buf(tt, data);
                }
                return ErrorGrammar;
            } else if tt == LeftParenthesisToken
                || tt == LeftBraceToken
                || tt == LeftBracketToken
                || tt == FunctionToken
            {
                self.level += 1;
            } else if tt == RightParenthesisToken
                || tt == RightBraceToken
                || tt == RightBracketToken
            {
                self.level -= 1;
            }

            if self.prev_ws {
                self.push_buf(WhitespaceToken, GoBytes::from_static(WS_BYTES));
            }
            self.push_buf(tt, data);

            (tt, data) = self.pop_token(false);
        }
    }

    // Go: parse/css/parse.go:Parser.parseCustomProperty
    fn parse_custom_property(&mut self) -> GrammarType {
        self.init_buf();
        let (tt, data) = self.pop_token(false);
        if tt != ColonToken {
            self.l.r.move_(-(data.len() as isize));
            self.err = b"expected colon in custom property".to_vec();
            self.err_pos = self.l.r.offset();
            self.l.r.move_(data.len() as isize);
            return ErrorGrammar;
        }
        let mut val = GoBytes::empty(); // []byte{}
        loop {
            let (tt, data) = self.l.next();
            if (tt == SemicolonToken || tt == RightBraceToken) && self.level == 0
                || tt == ErrorToken
            {
                self.prev_end = tt == RightBraceToken;
                self.push_buf(CustomPropertyValueToken, val);
                return CustomPropertyGrammar;
            } else if tt == LeftParenthesisToken
                || tt == LeftBraceToken
                || tt == LeftBracketToken
                || tt == FunctionToken
            {
                self.level += 1;
            } else if tt == RightParenthesisToken
                || tt == RightBraceToken
                || tt == RightBracketToken
            {
                if self.level == 0 {
                    // TODO: buggy
                    self.push_buf(tt, data);
                    self.err = b"unexpected ending in custom property".to_vec();
                    self.err_pos = self.l.r.offset();
                    return ErrorGrammar;
                }
                self.level -= 1;
            }
            val = val.append_bytes(&data);
        }
    }
}
