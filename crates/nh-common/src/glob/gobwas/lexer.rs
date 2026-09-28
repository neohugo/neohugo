//! Port of `github.com/gobwas/glob@v0.2.3` `syntax/lexer/{lexer,token}.go` and
//! `util/runes/runes.go` (the parts the lexer uses).
//!
//! Owner: Wave B task T02 (common-paths-text).
//!
//! Go quirks kept: `U+0000` is the end-of-input marker (a NUL ends the pattern), and any
//! `U+FFFD` (an invalid byte or an encoded replacement character) is the error
//! "could not read rune".

use std::collections::VecDeque;

use go_unicode::utf8;

pub type Rune = i32;

/// Go: `lexer.TokenType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenType {
    Eof,
    Error,
    Text,
    Char,
    Any,
    Super,
    Single,
    Not,
    Separator,
    RangeOpen,
    RangeClose,
    RangeLo,
    RangeHi,
    RangeBetween,
    TermsOpen,
    TermsClose,
}

impl TokenType {
    // Go: github.com/gobwas/glob syntax/lexer/token.go:(TokenType).String
    pub fn string(self) -> &'static str {
        match self {
            TokenType::Eof => "eof",
            TokenType::Error => "error",
            TokenType::Text => "text",
            TokenType::Char => "char",
            TokenType::Any => "any",
            TokenType::Super => "super",
            TokenType::Single => "single",
            TokenType::Not => "not",
            TokenType::Separator => "separator",
            TokenType::RangeOpen => "range_open",
            TokenType::RangeClose => "range_close",
            TokenType::RangeLo => "range_lo",
            TokenType::RangeHi => "range_hi",
            TokenType::RangeBetween => "range_between",
            TokenType::TermsOpen => "terms_open",
            TokenType::TermsClose => "terms_close",
        }
    }
}

/// Go: `lexer.Token`. `raw` is a Go string built from runes (valid UTF-8) or an error text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub typ: TokenType,
    pub raw: Vec<u8>,
}

impl Token {
    fn new(typ: TokenType, raw: impl Into<Vec<u8>>) -> Token {
        Token {
            typ,
            raw: raw.into(),
        }
    }

    // Go: github.com/gobwas/glob syntax/lexer/token.go:(Token).String
    pub fn string(&self) -> String {
        format!("{}<{}>", self.typ.string(), go_strconv::quote(&self.raw))
    }
}

const CHAR_ANY: Rune = '*' as Rune;
const CHAR_COMMA: Rune = ',' as Rune;
const CHAR_SINGLE: Rune = '?' as Rune;
const CHAR_ESCAPE: Rune = '\\' as Rune;
const CHAR_RANGE_OPEN: Rune = '[' as Rune;
const CHAR_RANGE_CLOSE: Rune = ']' as Rune;
const CHAR_TERMS_OPEN: Rune = '{' as Rune;
const CHAR_TERMS_CLOSE: Rune = '}' as Rune;
const CHAR_RANGE_NOT: Rune = '!' as Rune;
const CHAR_RANGE_BETWEEN: Rune = '-' as Rune;

const SPECIALS: [u8; 7] = [b'*', b'?', b'\\', b'[', b']', b'{', b'}'];

/// Go: `lexer.Special(c)` (also `syntax.Special`).
// Go: github.com/gobwas/glob syntax/lexer/lexer.go:Special
pub fn special(c: u8) -> bool {
    SPECIALS.contains(&c)
}

const EOF: Rune = 0;

const IN_TEXT_BREAKERS: [Rune; 4] = [CHAR_SINGLE, CHAR_ANY, CHAR_RANGE_OPEN, CHAR_TERMS_OPEN];
const IN_TERMS_BREAKERS: [Rune; 6] = [
    CHAR_SINGLE,
    CHAR_ANY,
    CHAR_RANGE_OPEN,
    CHAR_TERMS_OPEN,
    CHAR_TERMS_CLOSE,
    CHAR_COMMA,
];

// Go: github.com/gobwas/glob util/runes/runes.go:IndexRune
pub(crate) fn index_rune(s: &[Rune], r: Rune) -> isize {
    for (i, &c) in s.iter().enumerate() {
        if c == r {
            return i as isize;
        }
    }
    -1
}

/// Go: `lexer.lexer`.
pub struct Lexer<'a> {
    data: &'a [u8],
    pos: isize,
    err: Option<String>,

    tokens: VecDeque<Token>,
    terms_level: isize,

    last_rune: Rune,
    last_rune_size: isize,
    has_rune: bool,
}

impl<'a> Lexer<'a> {
    // Go: github.com/gobwas/glob syntax/lexer/lexer.go:NewLexer
    pub fn new(source: &'a [u8]) -> Lexer<'a> {
        Lexer {
            data: source,
            pos: 0,
            err: None,
            tokens: VecDeque::with_capacity(4),
            terms_level: 0,
            last_rune: 0,
            last_rune_size: 0,
            has_rune: false,
        }
    }

    // Go: github.com/gobwas/glob syntax/lexer/lexer.go:Next
    pub fn next_token(&mut self) -> Token {
        loop {
            if let Some(e) = &self.err {
                return Token::new(TokenType::Error, e.as_bytes());
            }
            if let Some(t) = self.tokens.pop_front() {
                return t;
            }

            self.fetch_item();
        }
    }

    // Go: github.com/gobwas/glob syntax/lexer/lexer.go:peek
    fn peek(&mut self) -> (Rune, isize) {
        if self.pos == self.data.len() as isize {
            return (EOF, 0);
        }

        let (mut r, w) = utf8::decode_rune_in_string(&self.data[self.pos as usize..]);
        let mut w = w as isize;
        if r == utf8::RUNE_ERROR {
            self.errorf("could not read rune");
            r = EOF;
            w = 0;
        }

        (r, w)
    }

    // Go: github.com/gobwas/glob syntax/lexer/lexer.go:read
    fn read(&mut self) -> Rune {
        if self.has_rune {
            self.has_rune = false;
            self.seek(self.last_rune_size);
            return self.last_rune;
        }

        let (r, s) = self.peek();
        self.seek(s);

        self.last_rune = r;
        self.last_rune_size = s;

        r
    }

    // Go: github.com/gobwas/glob syntax/lexer/lexer.go:seek
    fn seek(&mut self, w: isize) {
        self.pos += w;
    }

    // Go: github.com/gobwas/glob syntax/lexer/lexer.go:unread
    fn unread(&mut self) {
        if self.has_rune {
            self.errorf("could not unread rune");
            return;
        }
        self.seek(-self.last_rune_size);
        self.has_rune = true;
    }

    // Go: github.com/gobwas/glob syntax/lexer/lexer.go:errorf
    fn errorf(&mut self, msg: &str) {
        self.err = Some(msg.to_string());
    }

    // Go: github.com/gobwas/glob syntax/lexer/lexer.go:inTerms
    fn in_terms(&self) -> bool {
        self.terms_level > 0
    }

    // Go: github.com/gobwas/glob syntax/lexer/lexer.go:termsEnter
    fn terms_enter(&mut self) {
        self.terms_level += 1;
    }

    // Go: github.com/gobwas/glob syntax/lexer/lexer.go:termsLeave
    fn terms_leave(&mut self) {
        self.terms_level -= 1;
    }

    // Go: github.com/gobwas/glob syntax/lexer/lexer.go:fetchItem
    fn fetch_item(&mut self) {
        let r = self.read();
        if r == EOF {
            self.tokens.push_back(Token::new(TokenType::Eof, ""));
        } else if r == CHAR_TERMS_OPEN {
            self.terms_enter();
            self.tokens.push_back(Token::new(TokenType::TermsOpen, "{"));
        } else if r == CHAR_COMMA && self.in_terms() {
            self.tokens.push_back(Token::new(TokenType::Separator, ","));
        } else if r == CHAR_TERMS_CLOSE && self.in_terms() {
            self.tokens
                .push_back(Token::new(TokenType::TermsClose, "}"));
            self.terms_leave();
        } else if r == CHAR_RANGE_OPEN {
            self.tokens.push_back(Token::new(TokenType::RangeOpen, "["));
            self.fetch_range();
        } else if r == CHAR_SINGLE {
            self.tokens.push_back(Token::new(TokenType::Single, "?"));
        } else if r == CHAR_ANY {
            if self.read() == CHAR_ANY {
                self.tokens.push_back(Token::new(TokenType::Super, "**"));
            } else {
                self.unread();
                self.tokens.push_back(Token::new(TokenType::Any, "*"));
            }
        } else {
            self.unread();

            if self.in_terms() {
                self.fetch_text(&IN_TERMS_BREAKERS);
            } else {
                self.fetch_text(&IN_TEXT_BREAKERS);
            }
        }
    }

    // Go: github.com/gobwas/glob syntax/lexer/lexer.go:fetchRange
    fn fetch_range(&mut self) {
        let mut want_hi = false;
        let mut want_close = false;
        let mut seen_not = false;
        loop {
            let r = self.read();
            if r == EOF {
                self.errorf("unexpected end of input");
                return;
            }

            if want_close {
                if r != CHAR_RANGE_CLOSE {
                    self.errorf("expected close range character");
                } else {
                    self.tokens
                        .push_back(Token::new(TokenType::RangeClose, utf8::rune_to_string(r)));
                }
                return;
            }

            if want_hi {
                self.tokens
                    .push_back(Token::new(TokenType::RangeHi, utf8::rune_to_string(r)));
                want_close = true;
                continue;
            }

            if !seen_not && r == CHAR_RANGE_NOT {
                self.tokens
                    .push_back(Token::new(TokenType::Not, utf8::rune_to_string(r)));
                seen_not = true;
                continue;
            }

            let (n, w) = self.peek();
            if n == CHAR_RANGE_BETWEEN {
                self.seek(w);
                self.tokens
                    .push_back(Token::new(TokenType::RangeLo, utf8::rune_to_string(r)));
                self.tokens
                    .push_back(Token::new(TokenType::RangeBetween, utf8::rune_to_string(n)));
                want_hi = true;
                continue;
            }

            self.unread(); // unread first peek and fetch as text
            self.fetch_text(&[CHAR_RANGE_CLOSE]);
            want_close = true;
        }
    }

    // Go: github.com/gobwas/glob syntax/lexer/lexer.go:fetchText
    fn fetch_text(&mut self, breakers: &[Rune]) {
        let mut data: Vec<Rune> = Vec::new();
        let mut escaped = false;

        loop {
            let r = self.read();
            if r == EOF {
                break;
            }

            if !escaped {
                if r == CHAR_ESCAPE {
                    escaped = true;
                    continue;
                }

                if index_rune(breakers, r) != -1 {
                    self.unread();
                    break;
                }
            }

            escaped = false;
            data.push(r);
        }

        if !data.is_empty() {
            self.tokens
                .push_back(Token::new(TokenType::Text, utf8::from_runes(&data)));
        }
    }
}
