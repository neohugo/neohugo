//! Go: parse/v2/js/lex.go — an ECMAScript lexer.

use go_unicode::{
    LL, LM, LO, LT, LU, MC, MN, ND, NL, OTHER_ID_CONTINUE, OTHER_ID_START, PC, RangeTable, Rune, ZS,
};
use tdewolff_parse::{GoBytes, GoError, Input, new_error_lexer, printable};

use crate::table::keyword;
use crate::tokentype::*;

static IDENTIFIER_START: [&RangeTable; 7] = [LU, LL, LT, LM, LO, NL, OTHER_ID_START];
static IDENTIFIER_CONTINUE: [&RangeTable; 11] =
    [LU, LL, LT, LM, LO, NL, MN, MC, ND, PC, OTHER_ID_CONTINUE];

#[inline]
fn is_one_of_start(r: Rune) -> bool {
    go_unicode::is_one_of(&IDENTIFIER_START, r)
}

#[inline]
fn is_one_of_continue(r: Rune) -> bool {
    go_unicode::is_one_of(&IDENTIFIER_CONTINUE, r)
}

// Go: lex.go:IsIdentifierStart
/// Returns true if the byte-slice start is the start of an identifier.
pub fn is_identifier_start(b: &[u8]) -> bool {
    let (r, _) = go_unicode::utf8::decode_rune(b);
    r == '$' as Rune || r == '\\' as Rune || r == '_' as Rune || is_one_of_start(r)
}

// Go: lex.go:IsIdentifierContinue
/// Returns true if the byte-slice start is a continuation of an identifier.
pub fn is_identifier_continue(b: &[u8]) -> bool {
    let (r, _) = go_unicode::utf8::decode_rune(b);
    r == '$' as Rune || r == '\\' as Rune || r == 0x200C || r == 0x200D || is_one_of_continue(r)
}

// Go: lex.go:IsIdentifierEnd
/// Returns true if the byte-slice end is a start or continuation of an
/// identifier.
pub fn is_identifier_end(b: &[u8]) -> bool {
    let (r, _) = go_unicode::utf8::decode_last_rune(b);
    r == '$' as Rune || r == '\\' as Rune || r == 0x200C || r == 0x200D || is_one_of_continue(r)
}

////////////////////////////////////////////////////////////////

/// Go: lex.go:Lexer — the state for the lexer.
pub struct Lexer {
    r: Input,
    err: Option<GoError>,
    prev_line_terminator: bool,
    prev_numeric_literal: bool,
    level: i64,
    template_levels: Vec<i64>,
}

impl Lexer {
    // Go: lex.go:NewLexer
    /// Returns a new Lexer for a given input (the handle is shared, like the
    /// Go `*parse.Input`).
    pub fn new(r: Input) -> Lexer {
        Lexer {
            r,
            err: None,
            prev_line_terminator: true,
            level: 0,
            prev_numeric_literal: false,
            template_levels: Vec::new(),
        }
    }

    /// The shared input handle.
    pub fn input(&self) -> &Input {
        &self.r
    }

    // Go: lex.go:Lexer.Err
    /// Returns the error encountered during lexing, this is often io.EOF but
    /// also other errors can be returned.
    pub fn err(&self) -> Option<GoError> {
        if self.err.is_some() {
            return self.err.clone();
        }
        self.r.err()
    }

    /// Go `r.Peek(-n)` (the port's `Input::peek` takes an unsigned offset).
    fn peek_back(&self, n: isize) -> u8 {
        self.r.move_(-n);
        let c = self.r.peek(0);
        self.r.move_(n);
        c
    }

    // Go: lex.go:Lexer.RegExp
    /// Reparses the input stream for a regular expression. It is assumed that
    /// we just received DivToken or DivEqToken with Next(). This function will
    /// go back and read that as a regular expression.
    pub fn reg_exp(&mut self) -> (TokenType, GoBytes) {
        if 0 < self.r.offset() && self.peek_back(1) == b'/' {
            self.r.move_(-1);
        } else if 1 < self.r.offset() && self.peek_back(1) == b'=' && self.peek_back(2) == b'/' {
            self.r.move_(-2);
        } else {
            self.err = Some(GoError::Parse(Box::new(new_error_lexer(
                &self.r,
                b"expected / or /=".to_vec(),
            ))));
            return (ErrorToken, GoBytes::nil());
        }
        self.r.skip(); // trick to set start = pos

        if self.consume_reg_exp_token() {
            return (RegExpToken, self.r.shift());
        }
        self.err = Some(GoError::Parse(Box::new(new_error_lexer(
            &self.r,
            b"unexpected EOF or newline".to_vec(),
        ))));
        (ErrorToken, GoBytes::nil())
    }

    // Go: lex.go:Lexer.Next
    /// Returns the next Token. It returns ErrorToken when an error was
    /// encountered. Using Err() one can retrieve the error message.
    pub fn next(&mut self) -> (TokenType, GoBytes) {
        self.err = None; // clear error from previous ErrorToken
        let prev_line_terminator = self.prev_line_terminator;
        self.prev_line_terminator = false;

        let prev_numeric_literal = self.prev_numeric_literal;
        self.prev_numeric_literal = false;

        let c = self.r.peek(0);
        match c {
            b' ' | b'\t' | 0x0B | 0x0C => {
                self.r.move_(1);
                while self.consume_whitespace() {}
                self.prev_line_terminator = prev_line_terminator;
                return (WhitespaceToken, self.r.shift());
            }
            b'\n' | b'\r' => {
                self.r.move_(1);
                while self.consume_line_terminator() {}
                self.prev_line_terminator = true;
                return (LineTerminatorToken, self.r.shift());
            }
            b'>' | b'=' | b'!' | b'+' | b'*' | b'%' | b'&' | b'|' | b'^' | b'~' | b'?' => {
                let tt = self.consume_operator_token();
                if tt != ErrorToken {
                    return (tt, self.r.shift());
                }
            }
            b'0'..=b'9' | b'.' => {
                let tt = self.consume_numeric_token();
                if tt != ErrorToken || self.r.pos() != 0 {
                    self.prev_numeric_literal = true;
                    return (tt, self.r.shift());
                } else if c == b'.' {
                    self.r.move_(1);
                    if self.r.peek(0) == b'.' && self.r.peek(1) == b'.' {
                        self.r.move_(2);
                        return (EllipsisToken, self.r.shift());
                    }
                    return (DotToken, self.r.shift());
                }
            }
            b',' => {
                self.r.move_(1);
                return (CommaToken, self.r.shift());
            }
            b';' => {
                self.r.move_(1);
                return (SemicolonToken, self.r.shift());
            }
            b'(' => {
                self.level += 1;
                self.r.move_(1);
                return (OpenParenToken, self.r.shift());
            }
            b')' => {
                self.level -= 1;
                self.r.move_(1);
                return (CloseParenToken, self.r.shift());
            }
            b'/' => {
                let tt = self.consume_comment_token();
                if tt != ErrorToken || self.err.is_some() {
                    if self.err.is_some() {
                        return (ErrorToken, GoBytes::nil());
                    }
                    return (tt, self.r.shift());
                } else {
                    let tt = self.consume_operator_token();
                    if tt != ErrorToken {
                        return (tt, self.r.shift());
                    }
                }
            }
            b'{' => {
                self.level += 1;
                self.r.move_(1);
                return (OpenBraceToken, self.r.shift());
            }
            b'}' => {
                self.level -= 1;
                if !self.template_levels.is_empty()
                    && self.level == self.template_levels[self.template_levels.len() - 1]
                {
                    let tt = self.consume_template_token();
                    return (tt, self.r.shift());
                }
                self.r.move_(1);
                return (CloseBraceToken, self.r.shift());
            }
            b':' => {
                self.r.move_(1);
                return (ColonToken, self.r.shift());
            }
            b'\'' | b'"' => {
                let tt = self.consume_string_token();
                return (tt, self.r.shift());
            }
            b']' => {
                self.r.move_(1);
                return (CloseBracketToken, self.r.shift());
            }
            b'[' => {
                self.r.move_(1);
                return (OpenBracketToken, self.r.shift());
            }
            b'<' | b'-' => {
                if self.consume_html_like_comment_token(prev_line_terminator) {
                    return (CommentToken, self.r.shift());
                } else {
                    let tt = self.consume_operator_token();
                    if tt != ErrorToken {
                        return (tt, self.r.shift());
                    }
                }
            }
            b'`' => {
                self.template_levels.push(self.level);
                let tt = self.consume_template_token();
                return (tt, self.r.shift());
            }
            b'#' => {
                self.r.move_(1);
                if self.consume_identifier_token() {
                    return (PrivateIdentifierToken, self.r.shift());
                }
            }
            _ => {
                if self.consume_identifier_token() {
                    if prev_numeric_literal {
                        self.err = Some(GoError::Parse(Box::new(new_error_lexer(
                            &self.r,
                            b"unexpected identifier after number".to_vec(),
                        ))));
                        return (ErrorToken, GoBytes::nil());
                    } else if let Some(kw) = keyword(&self.r.lexeme().to_vec()) {
                        return (kw, self.r.shift());
                    }
                    return (IdentifierToken, self.r.shift());
                }
                if 0xC0 <= c {
                    if self.consume_whitespace() {
                        while self.consume_whitespace() {}
                        self.prev_line_terminator = prev_line_terminator;
                        return (WhitespaceToken, self.r.shift());
                    } else if self.consume_line_terminator() {
                        while self.consume_line_terminator() {}
                        self.prev_line_terminator = true;
                        return (LineTerminatorToken, self.r.shift());
                    }
                } else if c == 0 && self.r.has_err() {
                    return (ErrorToken, GoBytes::nil());
                }
            }
        }

        let (r, _) = self.r.peek_rune(0);
        let mut msg = b"unexpected ".to_vec();
        msg.extend_from_slice(&printable(r));
        self.err = Some(GoError::Parse(Box::new(new_error_lexer(&self.r, msg))));
        self.r.move_rune(); // allow to continue after error
        (ErrorToken, self.r.shift())
    }

    ////////////////////////////////////////////////////////////////

    /*
    The following functions follow the specifications at http://www.ecma-international.org/ecma-262/5.1/
    */

    // Go: lex.go:Lexer.consumeWhitespace
    fn consume_whitespace(&mut self) -> bool {
        let c = self.r.peek(0);
        if c == b' ' || c == b'\t' || c == 0x0B || c == 0x0C {
            self.r.move_(1);
            return true;
        } else if 0xC0 <= c {
            let (r, n) = self.r.peek_rune(0);
            if r == 0x00A0 || r == 0xFEFF || go_unicode::is(ZS, r) {
                self.r.move_(n as isize);
                return true;
            }
        }
        false
    }

    // Go: lex.go:Lexer.isLineTerminator
    fn is_line_terminator(&self) -> bool {
        let c = self.r.peek(0);
        if c == b'\n' || c == b'\r' {
            return true;
        } else if c == 0xE2
            && self.r.peek(1) == 0x80
            && (self.r.peek(2) == 0xA8 || self.r.peek(2) == 0xA9)
        {
            return true;
        }
        false
    }

    // Go: lex.go:Lexer.consumeLineTerminator
    fn consume_line_terminator(&mut self) -> bool {
        let c = self.r.peek(0);
        if c == b'\n' {
            self.r.move_(1);
            return true;
        } else if c == b'\r' {
            if self.r.peek(1) == b'\n' {
                self.r.move_(2);
            } else {
                self.r.move_(1);
            }
            return true;
        } else if c == 0xE2
            && self.r.peek(1) == 0x80
            && (self.r.peek(2) == 0xA8 || self.r.peek(2) == 0xA9)
        {
            self.r.move_(3);
            return true;
        }
        false
    }

    // Go: lex.go:Lexer.consumeDigit
    fn consume_digit(&mut self) -> bool {
        let c = self.r.peek(0);
        if c.is_ascii_digit() {
            self.r.move_(1);
            return true;
        }
        false
    }

    // Go: lex.go:Lexer.consumeHexDigit
    fn consume_hex_digit(&mut self) -> bool {
        let c = self.r.peek(0);
        if c.is_ascii_digit() || (b'a'..=b'f').contains(&c) || (b'A'..=b'F').contains(&c) {
            self.r.move_(1);
            return true;
        }
        false
    }

    // Go: lex.go:Lexer.consumeBinaryDigit
    fn consume_binary_digit(&mut self) -> bool {
        let c = self.r.peek(0);
        if c == b'0' || c == b'1' {
            self.r.move_(1);
            return true;
        }
        false
    }

    // Go: lex.go:Lexer.consumeOctalDigit
    fn consume_octal_digit(&mut self) -> bool {
        let c = self.r.peek(0);
        if (b'0'..=b'7').contains(&c) {
            self.r.move_(1);
            return true;
        }
        false
    }

    // Go: lex.go:Lexer.consumeUnicodeEscape
    fn consume_unicode_escape(&mut self) -> bool {
        if self.r.peek(0) != b'\\' || self.r.peek(1) != b'u' {
            return false;
        }
        let mark = self.r.pos();
        self.r.move_(2);
        let c = self.r.peek(0);
        if c == b'{' {
            self.r.move_(1);
            if self.consume_hex_digit() {
                while self.consume_hex_digit() {}
                let c = self.r.peek(0);
                if c == b'}' {
                    self.r.move_(1);
                    return true;
                }
            }
            self.r.rewind(mark);
            return false;
        } else if !self.consume_hex_digit()
            || !self.consume_hex_digit()
            || !self.consume_hex_digit()
            || !self.consume_hex_digit()
        {
            self.r.rewind(mark);
            return false;
        }
        true
    }

    // Go: lex.go:Lexer.consumeSingleLineComment
    pub(crate) fn consume_single_line_comment(&mut self) {
        loop {
            let c = self.r.peek(0);
            if c == b'\r' || c == b'\n' || c == 0 && self.r.has_err() {
                break;
            } else if 0xC0 <= c {
                let (r, _) = self.r.peek_rune(0);
                if r == 0x2028 || r == 0x2029 {
                    break;
                }
            }
            self.r.move_(1);
        }
    }

    ////////////////////////////////////////////////////////////////

    // Go: lex.go:Lexer.consumeHTMLLikeCommentToken
    fn consume_html_like_comment_token(&mut self, prev_line_terminator: bool) -> bool {
        let c = self.r.peek(0);
        if c == b'<' && self.r.peek(1) == b'!' && self.r.peek(2) == b'-' && self.r.peek(3) == b'-' {
            // opening HTML-style single line comment
            self.r.move_(4);
            self.consume_single_line_comment();
            return true;
        } else if prev_line_terminator
            && c == b'-'
            && self.r.peek(1) == b'-'
            && self.r.peek(2) == b'>'
        {
            // closing HTML-style single line comment
            // (only if current line didn't contain any meaningful tokens)
            self.r.move_(3);
            self.consume_single_line_comment();
            return true;
        }
        false
    }

    // Go: lex.go:Lexer.consumeCommentToken
    fn consume_comment_token(&mut self) -> TokenType {
        let c = self.r.peek(1);
        if c == b'/' {
            // single line comment
            self.r.move_(2);
            self.consume_single_line_comment();
            return CommentToken;
        } else if c == b'*' {
            self.r.move_(2);
            let mut tt = CommentToken;
            loop {
                let c = self.r.peek(0);
                if c == b'*' && self.r.peek(1) == b'/' {
                    self.r.move_(2);
                    break;
                } else if c == 0 && self.r.has_err() {
                    self.err = Some(GoError::Parse(Box::new(new_error_lexer(
                        &self.r,
                        b"unexpected EOF in comment".to_vec(),
                    ))));
                    return ErrorToken;
                } else if self.consume_line_terminator() {
                    self.prev_line_terminator = true;
                    tt = CommentLineTerminatorToken;
                } else {
                    self.r.move_(1);
                }
            }
            return tt;
        }
        ErrorToken
    }

    // Go: lex.go:Lexer.consumeOperatorToken
    fn consume_operator_token(&mut self) -> TokenType {
        let c = self.r.peek(0);
        self.r.move_(1);
        if self.r.peek(0) == b'=' {
            self.r.move_(1);
            if self.r.peek(0) == b'=' && (c == b'!' || c == b'=') {
                self.r.move_(1);
                if c == b'!' {
                    return NotEqEqToken;
                }
                return EqEqEqToken;
            }
            return op_eq_tokens(c);
        } else if self.r.peek(0) == c
            && (c == b'+'
                || c == b'-'
                || c == b'*'
                || c == b'&'
                || c == b'|'
                || c == b'?'
                || c == b'<')
        {
            self.r.move_(1);
            if self.r.peek(0) == b'=' && c != b'+' && c != b'-' {
                self.r.move_(1);
                return op_op_eq_tokens(c);
            }
            return op_op_tokens(c);
        } else if c == b'?'
            && self.r.peek(0) == b'.'
            && (self.r.peek(1) < b'0' || self.r.peek(1) > b'9')
        {
            self.r.move_(1);
            return OptChainToken;
        } else if c == b'=' && self.r.peek(0) == b'>' {
            self.r.move_(1);
            return ArrowToken;
        } else if c == b'>' && self.r.peek(0) == b'>' {
            self.r.move_(1);
            if self.r.peek(0) == b'>' {
                self.r.move_(1);
                if self.r.peek(0) == b'=' {
                    self.r.move_(1);
                    return GtGtGtEqToken;
                }
                return GtGtGtToken;
            } else if self.r.peek(0) == b'=' {
                self.r.move_(1);
                return GtGtEqToken;
            }
            return GtGtToken;
        }
        op_tokens(c)
    }

    // Go: lex.go:Lexer.consumeIdentifierToken
    fn consume_identifier_token(&mut self) -> bool {
        let c = self.r.peek(0);
        if IDENTIFIER_START_TABLE[c as usize] {
            self.r.move_(1);
        } else if 0xC0 <= c {
            let (r, n) = self.r.peek_rune(0);
            if is_one_of_start(r) {
                self.r.move_(n as isize);
            } else {
                return false;
            }
        } else if !self.consume_unicode_escape() {
            return false;
        }
        loop {
            let c = self.r.peek(0);
            if IDENTIFIER_TABLE[c as usize] {
                self.r.move_(1);
            } else if 0xC0 <= c {
                let (r, n) = self.r.peek_rune(0);
                if r == 0x200C || r == 0x200D || is_one_of_continue(r) {
                    self.r.move_(n as isize);
                } else {
                    break;
                }
            } else if !self.consume_unicode_escape() {
                break;
            }
        }
        true
    }

    // Go: lex.go:Lexer.consumeNumericSeparator
    fn consume_numeric_separator(&mut self, f: fn(&mut Lexer) -> bool) -> bool {
        if self.r.peek(0) != b'_' {
            return false;
        }
        self.r.move_(1);
        if !f(self) {
            self.r.move_(-1);
            return false;
        }
        true
    }

    // Go: lex.go:Lexer.consumeNumericToken
    fn consume_numeric_token(&mut self) -> TokenType {
        // assume to be on 0 1 2 3 4 5 6 7 8 9 .
        let first = self.r.peek(0);
        if first == b'0' {
            self.r.move_(1);
            if self.r.peek(0) == b'x' || self.r.peek(0) == b'X' {
                self.r.move_(1);
                if self.consume_hex_digit() {
                    while self.consume_hex_digit()
                        || self.consume_numeric_separator(Lexer::consume_hex_digit)
                    {
                    }
                    if self.r.peek(0) == b'n' {
                        self.r.move_(1);
                    }
                    return HexadecimalToken;
                }
                self.r.move_(-1);
                return IntegerToken;
            } else if self.r.peek(0) == b'b' || self.r.peek(0) == b'B' {
                self.r.move_(1);
                if self.consume_binary_digit() {
                    while self.consume_binary_digit()
                        || self.consume_numeric_separator(Lexer::consume_binary_digit)
                    {
                    }
                    if self.r.peek(0) == b'n' {
                        self.r.move_(1);
                    }
                    return BinaryToken;
                }
                self.r.move_(-1);
                return IntegerToken;
            } else if self.r.peek(0) == b'o' || self.r.peek(0) == b'O' {
                self.r.move_(1);
                if self.consume_octal_digit() {
                    while self.consume_octal_digit()
                        || self.consume_numeric_separator(Lexer::consume_octal_digit)
                    {
                    }
                    if self.r.peek(0) == b'n' {
                        self.r.move_(1);
                    }
                    return OctalToken;
                }
                self.r.move_(-1);
                return IntegerToken;
            } else if self.r.peek(0) == b'n' {
                self.r.move_(1);
                return IntegerToken;
            } else if b'0' <= self.r.peek(0) && self.r.peek(0) <= b'9' {
                self.err = Some(GoError::Parse(Box::new(new_error_lexer(
                    &self.r,
                    b"legacy octal numbers are not supported".to_vec(),
                ))));
                return ErrorToken;
            }
        } else if first != b'.' {
            while self.consume_digit() || self.consume_numeric_separator(Lexer::consume_digit) {}
        }
        // we have parsed a 0 or an integer number
        let mut c = self.r.peek(0);
        if c == b'.' {
            self.r.move_(1);
            if self.consume_digit() {
                while self.consume_digit() || self.consume_numeric_separator(Lexer::consume_digit) {
                }
                c = self.r.peek(0);
            } else if first == b'.' {
                // number starts with a dot and must be followed by digits
                self.r.move_(-1);
                return ErrorToken; // may be dot or ellipsis
            } else {
                c = self.r.peek(0);
            }
        } else if c == b'n' {
            self.r.move_(1);
            return IntegerToken;
        } else if c != b'e' && c != b'E' {
            return IntegerToken;
        }
        if c == b'e' || c == b'E' {
            self.r.move_(1);
            c = self.r.peek(0);
            if c == b'+' || c == b'-' {
                self.r.move_(1);
            }
            if !self.consume_digit() {
                self.err = Some(GoError::Parse(Box::new(new_error_lexer(
                    &self.r,
                    b"invalid number".to_vec(),
                ))));
                return ErrorToken;
            }
            while self.consume_digit() || self.consume_numeric_separator(Lexer::consume_digit) {}
        }
        DecimalToken
    }

    // Go: lex.go:Lexer.consumeStringToken
    fn consume_string_token(&mut self) -> TokenType {
        // assume to be on ' or "
        let delim = self.r.peek(0);
        self.r.move_(1);
        loop {
            let c = self.r.peek(0);
            if c == delim {
                self.r.move_(1);
                break;
            } else if c == b'\\' {
                self.r.move_(1);
                if !self.consume_line_terminator() {
                    let c = self.r.peek(0);
                    if c == delim || c == b'\\' {
                        self.r.move_(1);
                    }
                }
                continue;
            } else if c == b'\n' || c == b'\r' || c == 0 && self.r.has_err() {
                self.err = Some(GoError::Parse(Box::new(new_error_lexer(
                    &self.r,
                    b"unterminated string literal".to_vec(),
                ))));
                return ErrorToken;
            }
            self.r.move_(1);
        }
        StringToken
    }

    // Go: lex.go:Lexer.consumeRegExpToken
    fn consume_reg_exp_token(&mut self) -> bool {
        // assume to be on /
        self.r.move_(1);
        let mut in_class = false;
        loop {
            let c = self.r.peek(0);
            if !in_class && c == b'/' {
                self.r.move_(1);
                break;
            } else if c == b'[' {
                in_class = true;
            } else if c == b']' {
                in_class = false;
            } else if c == b'\\' {
                self.r.move_(1);
                if self.is_line_terminator() || self.r.peek(0) == 0 && self.r.has_err() {
                    return false;
                }
            } else if self.is_line_terminator() || c == 0 && self.r.has_err() {
                return false;
            }
            self.r.move_(1);
        }
        // flags
        loop {
            let c = self.r.peek(0);
            if IDENTIFIER_TABLE[c as usize] {
                self.r.move_(1);
            } else if 0xC0 <= c {
                let (r, n) = self.r.peek_rune(0);
                if r == 0x200C || r == 0x200D || is_one_of_continue(r) {
                    self.r.move_(n as isize);
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        true
    }

    // Go: lex.go:Lexer.consumeTemplateToken
    fn consume_template_token(&mut self) -> TokenType {
        // assume to be on ` or } when already within template
        let continuation = self.r.peek(0) == b'}';
        self.r.move_(1);
        loop {
            let c = self.r.peek(0);
            if c == b'`' {
                self.template_levels.pop();
                self.r.move_(1);
                if continuation {
                    return TemplateEndToken;
                }
                return TemplateToken;
            } else if c == b'$' && self.r.peek(1) == b'{' {
                self.level += 1;
                self.r.move_(2);
                if continuation {
                    return TemplateMiddleToken;
                }
                return TemplateStartToken;
            } else if c == b'\\' {
                self.r.move_(1);
                let c = self.r.peek(0);
                if c != 0 {
                    self.r.move_(1);
                }
                continue;
            } else if c == 0 && self.r.has_err() {
                self.err = Some(GoError::Parse(Box::new(new_error_lexer(
                    &self.r,
                    b"unterminated template literal".to_vec(),
                ))));
                return ErrorToken;
            }
            self.r.move_(1);
        }
    }
}

// Go: lex.go:opTokens (missing keys give the zero value ErrorToken)
fn op_tokens(c: u8) -> TokenType {
    match c {
        b'=' => EqToken,
        b'!' => NotToken,
        b'<' => LtToken,
        b'>' => GtToken,
        b'+' => AddToken,
        b'-' => SubToken,
        b'*' => MulToken,
        b'/' => DivToken,
        b'%' => ModToken,
        b'&' => BitAndToken,
        b'|' => BitOrToken,
        b'^' => BitXorToken,
        b'~' => BitNotToken,
        b'?' => QuestionToken,
        _ => ErrorToken,
    }
}

// Go: lex.go:opEqTokens
fn op_eq_tokens(c: u8) -> TokenType {
    match c {
        b'=' => EqEqToken,
        b'!' => NotEqToken,
        b'<' => LtEqToken,
        b'>' => GtEqToken,
        b'+' => AddEqToken,
        b'-' => SubEqToken,
        b'*' => MulEqToken,
        b'/' => DivEqToken,
        b'%' => ModEqToken,
        b'&' => BitAndEqToken,
        b'|' => BitOrEqToken,
        b'^' => BitXorEqToken,
        _ => ErrorToken,
    }
}

// Go: lex.go:opOpTokens
fn op_op_tokens(c: u8) -> TokenType {
    match c {
        b'<' => LtLtToken,
        b'+' => IncrToken,
        b'-' => DecrToken,
        b'*' => ExpToken,
        b'&' => AndToken,
        b'|' => OrToken,
        b'?' => NullishToken,
        _ => ErrorToken,
    }
}

// Go: lex.go:opOpEqTokens
fn op_op_eq_tokens(c: u8) -> TokenType {
    match c {
        b'<' => LtLtEqToken,
        b'*' => ExpEqToken,
        b'&' => AndEqToken,
        b'|' => OrEqToken,
        b'?' => NullishEqToken,
        _ => ErrorToken,
    }
}

const fn ident_start_table() -> [bool; 256] {
    let mut t = [false; 256];
    t[b'$' as usize] = true;
    t[b'_' as usize] = true;
    let mut c = b'A';
    while c <= b'Z' {
        t[c as usize] = true;
        c += 1;
    }
    let mut c = b'a';
    while c <= b'z' {
        t[c as usize] = true;
        c += 1;
    }
    t
}

const fn ident_table() -> [bool; 256] {
    let mut t = ident_start_table();
    let mut c = b'0';
    while c <= b'9' {
        t[c as usize] = true;
        c += 1;
    }
    t
}

// Go: lex.go:identifierStartTable ($, A-Z, _, a-z)
pub(crate) static IDENTIFIER_START_TABLE: [bool; 256] = ident_start_table();

// Go: lex.go:identifierTable ($, 0-9, A-Z, _, a-z)
pub(crate) static IDENTIFIER_TABLE: [bool; 256] = ident_table();
