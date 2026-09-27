//! Go: parse/css/lex.go — a CSS3 lexer following the specifications at
//! <http://www.w3.org/TR/css-syntax-3/>.

use crate::error::GoError;
use crate::gobytes::GoBytes;
use crate::input::Input;
use crate::util::equal_fold;

/// Go: css.TokenType — the type of token.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TokenType {
    ErrorToken = 0, // extra token when errors occur
    IdentToken,
    FunctionToken,  // rgb( rgba( ...
    AtKeywordToken, // @abc
    HashToken,      // #abc
    StringToken,
    BadStringToken,
    URLToken,
    BadURLToken,
    DelimToken,            // any unmatched character
    NumberToken,           // 5
    PercentageToken,       // 5%
    DimensionToken,        // 5em
    UnicodeRangeToken,     // U+554A
    IncludeMatchToken,     // ~=
    DashMatchToken,        // |=
    PrefixMatchToken,      // ^=
    SuffixMatchToken,      // $=
    SubstringMatchToken,   // *=
    ColumnToken,           // ||
    WhitespaceToken,       // space \t \r \n \f
    CDOToken,              // <!--
    CDCToken,              // -->
    ColonToken,            // :
    SemicolonToken,        // ;
    CommaToken,            // ,
    LeftBracketToken,      // [
    RightBracketToken,     // ]
    LeftParenthesisToken,  // (
    RightParenthesisToken, // )
    LeftBraceToken,        // {
    RightBraceToken,       // }
    CommentToken,          // extra token for comments
    EmptyToken,
    CustomPropertyNameToken,
    CustomPropertyValueToken,
}

pub use TokenType::*;

impl TokenType {
    // Go: parse/css/lex.go:TokenType.String
    /// Returns the string representation of a TokenType.
    pub fn string(self) -> String {
        token_type_string(self as u32)
    }
}

/// `TokenType(n).String()` for any integer value.
pub fn token_type_string(n: u32) -> String {
    match n {
        0 => "Error",
        1 => "Ident",
        2 => "Function",
        3 => "AtKeyword",
        4 => "Hash",
        5 => "String",
        6 => "BadString",
        7 => "URL",
        8 => "BadURL",
        9 => "Delim",
        10 => "Number",
        11 => "Percentage",
        12 => "Dimension",
        13 => "UnicodeRange",
        14 => "IncludeMatch",
        15 => "DashMatch",
        16 => "PrefixMatch",
        17 => "SuffixMatch",
        18 => "SubstringMatch",
        19 => "Column",
        20 => "Whitespace",
        21 => "CDO",
        22 => "CDC",
        23 => "Colon",
        24 => "Semicolon",
        25 => "Comma",
        26 => "LeftBracket",
        27 => "RightBracket",
        28 => "LeftParenthesis",
        29 => "RightParenthesis",
        30 => "LeftBrace",
        31 => "RightBrace",
        32 => "Comment",
        33 => "Empty",
        34 => "CustomPropertyName",
        35 => "CustomPropertyValue",
        _ => return format!("Invalid({})", n),
    }
    .to_string()
}

/// Go: css.Lexer — the state for the lexer.
pub struct Lexer {
    pub(crate) r: Input,
}

impl Lexer {
    // Go: parse/css/lex.go:NewLexer
    /// Returns a new Lexer for a given Input.
    pub fn new(r: Input) -> Lexer {
        Lexer { r }
    }

    /// The underlying `*parse.Input` (shared handle).
    pub fn input(&self) -> &Input {
        &self.r
    }

    // Go: parse/css/lex.go:Lexer.Err
    /// Returns the error encountered during lexing, this is often io.EOF but
    /// also other errors can be returned.
    pub fn err(&self) -> Option<GoError> {
        self.r.err()
    }

    // Go: parse/css/lex.go:Lexer.Next
    /// Returns the next Token. It returns ErrorToken when an error was
    /// encountered. Using Err() one can retrieve the error message.
    pub fn next(&mut self) -> (TokenType, GoBytes) {
        match self.r.peek(0) {
            b' ' | b'\t' | b'\n' | b'\r' | 0x0C => {
                self.r.move_(1);
                while self.consume_whitespace() {}
                return (WhitespaceToken, self.r.shift());
            }
            b':' => {
                self.r.move_(1);
                return (ColonToken, self.r.shift());
            }
            b';' => {
                self.r.move_(1);
                return (SemicolonToken, self.r.shift());
            }
            b',' => {
                self.r.move_(1);
                return (CommaToken, self.r.shift());
            }
            b'(' | b')' | b'[' | b']' | b'{' | b'}' => {
                let t = self.consume_bracket();
                if t != ErrorToken {
                    return (t, self.r.shift());
                }
            }
            b'#' => {
                if self.consume_hash_token() {
                    return (HashToken, self.r.shift());
                }
            }
            b'"' | b'\'' => {
                let t = self.consume_string();
                if t != ErrorToken {
                    return (t, self.r.shift());
                }
            }
            b'.' | b'+' => {
                let t = self.consume_numeric();
                if t != ErrorToken {
                    return (t, self.r.shift());
                }
            }
            b'-' => {
                let t = self.consume_numeric();
                if t != ErrorToken {
                    return (t, self.r.shift());
                }
                let t = self.consume_identlike();
                if t != ErrorToken {
                    return (t, self.r.shift());
                } else if self.consume_cdc_token() {
                    return (CDCToken, self.r.shift());
                } else if self.consume_custom_variable_token() {
                    return (CustomPropertyNameToken, self.r.shift());
                }
            }
            b'@' => {
                if self.consume_at_keyword_token() {
                    return (AtKeywordToken, self.r.shift());
                }
            }
            b'$' | b'*' | b'^' | b'~' => {
                let t = self.consume_match();
                if t != ErrorToken {
                    return (t, self.r.shift());
                }
            }
            b'/' => {
                if self.consume_comment() {
                    return (CommentToken, self.r.shift());
                }
            }
            b'<' => {
                if self.consume_cdo_token() {
                    return (CDOToken, self.r.shift());
                }
            }
            b'\\' => {
                let t = self.consume_identlike();
                if t != ErrorToken {
                    return (t, self.r.shift());
                }
            }
            b'u' | b'U' => {
                if self.consume_unicode_range_token() {
                    return (UnicodeRangeToken, self.r.shift());
                }
                let t = self.consume_identlike();
                if t != ErrorToken {
                    return (t, self.r.shift());
                }
            }
            b'|' => {
                let t = self.consume_match();
                if t != ErrorToken {
                    return (t, self.r.shift());
                } else if self.consume_column_token() {
                    return (ColumnToken, self.r.shift());
                }
            }
            0 => {
                if self.r.has_err() {
                    return (ErrorToken, GoBytes::nil());
                }
            }
            _ => {
                let t = self.consume_numeric();
                if t != ErrorToken {
                    return (t, self.r.shift());
                }
                let t = self.consume_identlike();
                if t != ErrorToken {
                    return (t, self.r.shift());
                }
            }
        }
        // can't be rune because consumeIdentlike consumes that as an identifier
        self.r.move_(1);
        (DelimToken, self.r.shift())
    }

    ////////////////////////////////////////////////////////////////

    // The following functions follow the railroad diagrams in http://www.w3.org/TR/css3-syntax/

    // Go: parse/css/lex.go:Lexer.consumeByte
    fn consume_byte(&mut self, c: u8) -> bool {
        if self.r.peek(0) == c {
            self.r.move_(1);
            return true;
        }
        false
    }

    // Go: parse/css/lex.go:Lexer.consumeComment
    fn consume_comment(&mut self) -> bool {
        if self.r.peek(0) != b'/' || self.r.peek(1) != b'*' {
            return false;
        }
        self.r.move_(2);
        loop {
            let c = self.r.peek(0);
            if c == 0 && self.r.has_err() {
                break;
            } else if c == b'*' && self.r.peek(1) == b'/' {
                self.r.move_(2);
                return true;
            }
            self.r.move_(1);
        }
        true
    }

    // Go: parse/css/lex.go:Lexer.consumeNewline
    fn consume_newline(&mut self) -> bool {
        let c = self.r.peek(0);
        if c == b'\n' || c == 0x0C {
            self.r.move_(1);
            return true;
        } else if c == b'\r' {
            if self.r.peek(1) == b'\n' {
                self.r.move_(2);
            } else {
                self.r.move_(1);
            }
            return true;
        }
        false
    }

    // Go: parse/css/lex.go:Lexer.consumeWhitespace
    fn consume_whitespace(&mut self) -> bool {
        let c = self.r.peek(0);
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0C {
            self.r.move_(1);
            return true;
        }
        false
    }

    // Go: parse/css/lex.go:Lexer.consumeDigit
    fn consume_digit(&mut self) -> bool {
        let c = self.r.peek(0);
        if c.is_ascii_digit() {
            self.r.move_(1);
            return true;
        }
        false
    }

    // Go: parse/css/lex.go:Lexer.consumeHexDigit
    fn consume_hex_digit(&mut self) -> bool {
        let c = self.r.peek(0);
        if c.is_ascii_hexdigit() {
            self.r.move_(1);
            return true;
        }
        false
    }

    // Go: parse/css/lex.go:Lexer.consumeEscape
    fn consume_escape(&mut self) -> bool {
        if self.r.peek(0) != b'\\' {
            return false;
        }
        let mark = self.r.pos();
        self.r.move_(1);
        if self.consume_newline() {
            self.r.rewind(mark);
            return false;
        } else if self.consume_hex_digit() {
            for _k in 1..6 {
                if !self.consume_hex_digit() {
                    break;
                }
            }
            self.consume_whitespace();
            return true;
        } else {
            let c = self.r.peek(0);
            if c >= 0xC0 {
                let (_, n) = self.r.peek_rune(0);
                self.r.move_(n as isize);
                return true;
            } else if c == 0 && self.r.has_err() {
                self.r.rewind(mark);
                return false;
            }
        }
        self.r.move_(1);
        true
    }

    // Go: parse/css/lex.go:Lexer.consumeIdentToken
    pub(crate) fn consume_ident_token(&mut self) -> bool {
        let mark = self.r.pos();
        if self.r.peek(0) == b'-' {
            self.r.move_(1);
        }
        let c = self.r.peek(0);
        if !(c.is_ascii_lowercase() || c.is_ascii_uppercase() || c == b'_' || c >= 0x80) {
            if c != b'\\' || !self.consume_escape() {
                self.r.rewind(mark);
                return false;
            }
        } else {
            self.r.move_(1);
        }
        loop {
            let c = self.r.peek(0);
            if !(c.is_ascii_lowercase()
                || c.is_ascii_uppercase()
                || c.is_ascii_digit()
                || c == b'_'
                || c == b'-'
                || c >= 0x80)
            {
                if c != b'\\' || !self.consume_escape() {
                    break;
                }
            } else {
                self.r.move_(1);
            }
        }
        true
    }

    // Go: parse/css/lex.go:Lexer.consumeCustomVariableToken
    /// support custom variables, https://www.w3.org/TR/css-variables-1/
    fn consume_custom_variable_token(&mut self) -> bool {
        // expect to be on a '-'
        self.r.move_(1);
        if self.r.peek(0) != b'-' {
            self.r.move_(-1);
            return false;
        }
        if !self.consume_ident_token() {
            self.r.move_(-1);
            return false;
        }
        true
    }

    // Go: parse/css/lex.go:Lexer.consumeAtKeywordToken
    fn consume_at_keyword_token(&mut self) -> bool {
        // expect to be on an '@'
        self.r.move_(1);
        if !self.consume_ident_token() {
            self.r.move_(-1);
            return false;
        }
        true
    }

    // Go: parse/css/lex.go:Lexer.consumeHashToken
    fn consume_hash_token(&mut self) -> bool {
        // expect to be on a '#'
        let mark = self.r.pos();
        self.r.move_(1);
        let c = self.r.peek(0);
        if !(c.is_ascii_lowercase()
            || c.is_ascii_uppercase()
            || c.is_ascii_digit()
            || c == b'_'
            || c == b'-'
            || c >= 0x80)
        {
            if c != b'\\' || !self.consume_escape() {
                self.r.rewind(mark);
                return false;
            }
        } else {
            self.r.move_(1);
        }
        loop {
            let c = self.r.peek(0);
            if !(c.is_ascii_lowercase()
                || c.is_ascii_uppercase()
                || c.is_ascii_digit()
                || c == b'_'
                || c == b'-'
                || c >= 0x80)
            {
                if c != b'\\' || !self.consume_escape() {
                    break;
                }
            } else {
                self.r.move_(1);
            }
        }
        true
    }

    // Go: parse/css/lex.go:Lexer.consumeNumberToken
    fn consume_number_token(&mut self) -> bool {
        let mut mark = self.r.pos();
        let mut c = self.r.peek(0);
        if c == b'+' || c == b'-' {
            self.r.move_(1);
        }
        let first_digit = self.consume_digit();
        if first_digit {
            while self.consume_digit() {}
        }
        if self.r.peek(0) == b'.' {
            self.r.move_(1);
            if self.consume_digit() {
                while self.consume_digit() {}
            } else if first_digit {
                // . could belong to the next token
                self.r.move_(-1);
                return true;
            } else {
                self.r.rewind(mark);
                return false;
            }
        } else if !first_digit {
            self.r.rewind(mark);
            return false;
        }
        mark = self.r.pos();
        c = self.r.peek(0);
        if c == b'e' || c == b'E' {
            self.r.move_(1);
            c = self.r.peek(0);
            if c == b'+' || c == b'-' {
                self.r.move_(1);
            }
            if !self.consume_digit() {
                // e could belong to next token
                self.r.rewind(mark);
                return true;
            }
            while self.consume_digit() {}
        }
        true
    }

    // Go: parse/css/lex.go:Lexer.consumeUnicodeRangeToken
    fn consume_unicode_range_token(&mut self) -> bool {
        let c = self.r.peek(0);
        if (c != b'u' && c != b'U') || self.r.peek(1) != b'+' {
            return false;
        }
        let mark = self.r.pos();
        self.r.move_(2);

        // consume up to 6 hexDigits
        let mut k = 0;
        while self.consume_hex_digit() {
            k += 1;
        }

        // either a minus or a question mark or the end is expected
        if self.consume_byte(b'-') {
            if k == 0 || 6 < k {
                self.r.rewind(mark);
                return false;
            }

            // consume another up to 6 hexDigits
            if self.consume_hex_digit() {
                k = 1;
                while self.consume_hex_digit() {
                    k += 1;
                }
            } else {
                self.r.rewind(mark);
                return false;
            }
        } else if self.consume_byte(b'?') {
            // could be filled up to 6 characters with question marks or else regular hexDigits
            k += 1;
            while self.consume_byte(b'?') {
                k += 1;
            }
        }
        if k == 0 || 6 < k {
            self.r.rewind(mark);
            return false;
        }
        true
    }

    // Go: parse/css/lex.go:Lexer.consumeColumnToken
    fn consume_column_token(&mut self) -> bool {
        if self.r.peek(0) == b'|' && self.r.peek(1) == b'|' {
            self.r.move_(2);
            return true;
        }
        false
    }

    // Go: parse/css/lex.go:Lexer.consumeCDOToken
    fn consume_cdo_token(&mut self) -> bool {
        if self.r.peek(0) == b'<'
            && self.r.peek(1) == b'!'
            && self.r.peek(2) == b'-'
            && self.r.peek(3) == b'-'
        {
            self.r.move_(4);
            return true;
        }
        false
    }

    // Go: parse/css/lex.go:Lexer.consumeCDCToken
    fn consume_cdc_token(&mut self) -> bool {
        if self.r.peek(0) == b'-' && self.r.peek(1) == b'-' && self.r.peek(2) == b'>' {
            self.r.move_(3);
            return true;
        }
        false
    }

    ////////////////////////////////////////////////////////////////

    // Go: parse/css/lex.go:Lexer.consumeMatch
    /// consumes any MatchToken.
    fn consume_match(&mut self) -> TokenType {
        if self.r.peek(1) == b'=' {
            match self.r.peek(0) {
                b'~' => {
                    self.r.move_(2);
                    return IncludeMatchToken;
                }
                b'|' => {
                    self.r.move_(2);
                    return DashMatchToken;
                }
                b'^' => {
                    self.r.move_(2);
                    return PrefixMatchToken;
                }
                b'$' => {
                    self.r.move_(2);
                    return SuffixMatchToken;
                }
                b'*' => {
                    self.r.move_(2);
                    return SubstringMatchToken;
                }
                _ => {}
            }
        }
        ErrorToken
    }

    // Go: parse/css/lex.go:Lexer.consumeBracket
    /// consumes any bracket token.
    pub fn consume_bracket(&mut self) -> TokenType {
        match self.r.peek(0) {
            b'(' => {
                self.r.move_(1);
                LeftParenthesisToken
            }
            b')' => {
                self.r.move_(1);
                RightParenthesisToken
            }
            b'[' => {
                self.r.move_(1);
                LeftBracketToken
            }
            b']' => {
                self.r.move_(1);
                RightBracketToken
            }
            b'{' => {
                self.r.move_(1);
                LeftBraceToken
            }
            b'}' => {
                self.r.move_(1);
                RightBraceToken
            }
            _ => ErrorToken,
        }
    }

    // Go: parse/css/lex.go:Lexer.consumeNumeric
    /// consumes NumberToken, PercentageToken or DimensionToken.
    fn consume_numeric(&mut self) -> TokenType {
        if self.consume_number_token() {
            if self.consume_byte(b'%') {
                return PercentageToken;
            } else if self.consume_ident_token() {
                return DimensionToken;
            }
            return NumberToken;
        }
        ErrorToken
    }

    // Go: parse/css/lex.go:Lexer.consumeString
    /// consumes a string and may return BadStringToken when a newline is
    /// encountered.
    fn consume_string(&mut self) -> TokenType {
        // assume to be on " or '
        let delim = self.r.peek(0);
        self.r.move_(1);
        loop {
            let c = self.r.peek(0);
            if c == 0 && self.r.has_err() {
                break;
            } else if c == b'\n' || c == b'\r' || c == 0x0C {
                self.r.move_(1);
                return BadStringToken;
            } else if c == delim {
                self.r.move_(1);
                break;
            } else if c == b'\\' {
                if !self.consume_escape() {
                    // either newline or EOF after backslash
                    self.r.move_(1);
                    self.consume_newline();
                }
            } else {
                self.r.move_(1);
            }
        }
        StringToken
    }

    // Go: parse/css/lex.go:Lexer.consumeUnquotedURL
    pub(crate) fn consume_unquoted_url(&mut self) -> bool {
        loop {
            let c = self.r.peek(0);
            if c == 0 && self.r.has_err() || c == b')' {
                break;
            } else if c == b'"'
                || c == b'\''
                || c == b'('
                || c == b'\\'
                || c == b' '
                || c <= 0x1F
                || c == 0x7F
            {
                if c != b'\\' || !self.consume_escape() {
                    return false;
                }
            } else {
                self.r.move_(1);
            }
        }
        true
    }

    // Go: parse/css/lex.go:Lexer.consumeRemnantsBadURL
    /// consumes bytes of a BadUrlToken so that normal tokenization may
    /// continue.
    fn consume_remnants_bad_url(&mut self) {
        loop {
            if self.consume_byte(b')') || self.r.has_err() {
                break;
            } else if !self.consume_escape() {
                self.r.move_(1);
            }
        }
    }

    // Go: parse/css/lex.go:Lexer.consumeIdentlike
    /// consumes IdentToken, FunctionToken or UrlToken.
    fn consume_identlike(&mut self) -> TokenType {
        if self.consume_ident_token() {
            if self.r.peek(0) != b'(' {
                return IdentToken;
            }
            // bytes.Replace(l.r.Lexeme(), []byte{'\\'}, nil, -1)
            let lexeme: Vec<u8> = self.r.lexeme().iter().filter(|&c| c != b'\\').collect();
            if !equal_fold(&lexeme, b"url") {
                self.r.move_(1);
                return FunctionToken;
            }
            self.r.move_(1);

            // consume url
            while self.consume_whitespace() {}
            let c = self.r.peek(0);
            if c == b'"' || c == b'\'' {
                if self.consume_string() == BadStringToken {
                    self.consume_remnants_bad_url();
                    return BadURLToken;
                }
            } else if !self.consume_unquoted_url() && !self.consume_whitespace() {
                // if unquoted URL fails due to encountering whitespace, continue
                self.consume_remnants_bad_url();
                return BadURLToken;
            }
            while self.consume_whitespace() {}
            if !self.consume_byte(b')') && self.r.err() != Some(GoError::Eof) {
                self.consume_remnants_bad_url();
                return BadURLToken;
            }
            return URLToken;
        }
        ErrorToken
    }
}
