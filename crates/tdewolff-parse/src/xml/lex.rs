//! Go: parse/xml/lex.go — an XML1.0 lexer following the specifications at
//! <http://www.w3.org/TR/xml/>.

use crate::error::{GoError, new_error_lexer_err};
use crate::gobytes::GoBytes;
use crate::input::Input;

/// Go: xml.TokenType — the type of token.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TokenType {
    ErrorToken = 0, // extra token when errors occur
    CommentToken,
    DOCTYPEToken,
    CDATAToken,
    StartTagToken,
    StartTagPIToken,
    StartTagCloseToken,
    StartTagCloseVoidToken,
    StartTagClosePIToken,
    EndTagToken,
    AttributeToken,
    TextToken,
}

pub use TokenType::*;

impl TokenType {
    // Go: parse/xml/lex.go:TokenType.String
    /// Returns the string representation of a TokenType.
    pub fn string(self) -> String {
        token_type_string(self as u32)
    }
}

/// `TokenType(n).String()` for any integer value.
pub fn token_type_string(n: u32) -> String {
    match n {
        0 => "Error",
        1 => "Comment",
        2 => "DOCTYPE",
        3 => "CDATA",
        4 => "StartTag",
        5 => "StartTagPI",
        6 => "StartTagClose",
        7 => "StartTagCloseVoid",
        8 => "StartTagClosePI",
        9 => "EndTag",
        10 => "Attribute",
        11 => "Text",
        _ => return format!("Invalid({})", n),
    }
    .to_string()
}

/// Go: xml.Lexer — the state for the lexer.
pub struct Lexer {
    r: Input,
    err: Option<GoError>,

    in_tag: bool,

    text: GoBytes,
    attr_val: GoBytes,
}

// Go: parse/xml/lex.go:Lexer.at
fn at(r: &Input, b: &[u8]) -> bool {
    for (i, &c) in b.iter().enumerate() {
        if r.peek(i) != c {
            return false;
        }
    }
    true
}

impl Lexer {
    // Go: parse/xml/lex.go:NewLexer
    /// Returns a new Lexer for a given Input.
    pub fn new(r: Input) -> Lexer {
        Lexer {
            r,
            err: None,
            in_tag: false,
            text: GoBytes::nil(),
            attr_val: GoBytes::nil(),
        }
    }

    /// The underlying `*parse.Input` (shared handle).
    pub fn input(&self) -> &Input {
        &self.r
    }

    // Go: parse/xml/lex.go:Lexer.Err
    /// Returns the error encountered during lexing, this is often io.EOF but
    /// also other errors can be returned.
    pub fn err(&self) -> Option<GoError> {
        if self.err.is_some() {
            return self.err.clone();
        }
        self.r.err()
    }

    // Go: parse/xml/lex.go:Lexer.Text
    /// Returns the textual representation of a token. This excludes
    /// delimiters and additional leading/trailing characters.
    pub fn text(&self) -> GoBytes {
        self.text.clone()
    }

    // Go: parse/xml/lex.go:Lexer.AttrVal
    /// Returns the attribute value when an AttributeToken was returned from Next.
    pub fn attr_val(&self) -> GoBytes {
        self.attr_val.clone()
    }

    // Go: parse/xml/lex.go:Lexer.Next
    /// Returns the next Token. It returns ErrorToken when an error was
    /// encountered. Using Err() one can retrieve the error message.
    pub fn next(&mut self) -> (TokenType, GoBytes) {
        self.text = GoBytes::nil();
        let mut c: u8;
        if self.in_tag {
            self.attr_val = GoBytes::nil();
            loop {
                // before attribute name state
                c = self.r.peek(0);
                if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' {
                    self.r.move_(1);
                    continue;
                }
                break;
            }
            if c == 0 {
                if !self.r.has_err() {
                    self.err = Some(new_error_lexer_err(&self.r, "unexpected NULL character"));
                }
                return (ErrorToken, GoBytes::nil());
            } else if c != b'>' && (c != b'/' && c != b'?' || self.r.peek(1) != b'>') {
                return (AttributeToken, self.shift_attribute());
            }
            self.r.skip();
            self.in_tag = false;
            if c == b'/' {
                self.r.move_(2);
                return (StartTagCloseVoidToken, self.r.shift());
            } else if c == b'?' {
                self.r.move_(2);
                return (StartTagClosePIToken, self.r.shift());
            } else {
                self.r.move_(1);
                return (StartTagCloseToken, self.r.shift());
            }
        }

        loop {
            c = self.r.peek(0);
            if c == b'<' {
                if self.r.pos() > 0 {
                    self.text = self.r.shift();
                    return (TextToken, self.text.clone());
                }
                c = self.r.peek(1);
                if c == b'/' {
                    self.r.move_(2);
                    return (EndTagToken, self.shift_end_tag());
                } else if c == b'!' {
                    self.r.move_(2);
                    if at(&self.r, b"--") {
                        self.r.move_(2);
                        return (CommentToken, self.shift_comment_text());
                    } else if at(&self.r, b"[CDATA[") {
                        self.r.move_(7);
                        return (CDATAToken, self.shift_cdata_text());
                    } else if at(&self.r, b"DOCTYPE") {
                        self.r.move_(7);
                        return (DOCTYPEToken, self.shift_doctype_text());
                    }
                    self.r.move_(-2);
                } else if c == b'?' {
                    self.r.move_(2);
                    self.in_tag = true;
                    return (StartTagPIToken, self.shift_start_tag());
                }
                self.r.move_(1);
                self.in_tag = true;
                return (StartTagToken, self.shift_start_tag());
            } else if c == 0 {
                if self.r.pos() > 0 {
                    self.text = self.r.shift();
                    return (TextToken, self.text.clone());
                }
                if !self.r.has_err() {
                    self.err = Some(new_error_lexer_err(&self.r, "unexpected NULL character"));
                }
                return (ErrorToken, GoBytes::nil());
            }
            self.r.move_(1);
        }
    }

    ////////////////////////////////////////////////////////////////

    // The following functions follow the specifications at http://www.w3.org/html/wg/drafts/html/master/syntax.html

    // Go: parse/xml/lex.go:Lexer.shiftDOCTYPEText
    fn shift_doctype_text(&mut self) -> GoBytes {
        let mut in_string = false;
        let mut in_brackets = false;
        loop {
            let c = self.r.peek(0);
            if c == b'"' {
                in_string = !in_string;
            } else if (c == b'[' || c == b']') && !in_string {
                in_brackets = c == b'[';
            } else if c == b'>' && !in_string && !in_brackets {
                self.text = self.r.lexeme().slice_from(9);
                self.r.move_(1);
                return self.r.shift();
            } else if c == 0 {
                self.text = self.r.lexeme().slice_from(9);
                return self.r.shift();
            }
            self.r.move_(1);
        }
    }

    // Go: parse/xml/lex.go:Lexer.shiftCDATAText
    fn shift_cdata_text(&mut self) -> GoBytes {
        loop {
            let c = self.r.peek(0);
            if c == b']' && self.r.peek(1) == b']' && self.r.peek(2) == b'>' {
                self.text = self.r.lexeme().slice_from(9);
                self.r.move_(3);
                return self.r.shift();
            } else if c == 0 {
                self.text = self.r.lexeme().slice_from(9);
                return self.r.shift();
            }
            self.r.move_(1);
        }
    }

    // Go: parse/xml/lex.go:Lexer.shiftCommentText
    fn shift_comment_text(&mut self) -> GoBytes {
        loop {
            let c = self.r.peek(0);
            if c == b'-' && self.r.peek(1) == b'-' && self.r.peek(2) == b'>' {
                self.text = self.r.lexeme().slice_from(4);
                self.r.move_(3);
                return self.r.shift();
            } else if c == 0 {
                return self.r.shift();
            }
            self.r.move_(1);
        }
    }

    // Go: parse/xml/lex.go:Lexer.shiftStartTag
    fn shift_start_tag(&mut self) -> GoBytes {
        let name_start = self.r.pos();
        loop {
            let c = self.r.peek(0);
            if c == b' '
                || c == b'>'
                || (c == b'/' || c == b'?') && self.r.peek(1) == b'>'
                || c == b'\t'
                || c == b'\n'
                || c == b'\r'
                || c == 0
            {
                break;
            }
            self.r.move_(1);
        }
        self.text = self.r.lexeme().slice_from(name_start);
        self.r.shift()
    }

    // Go: parse/xml/lex.go:Lexer.shiftAttribute
    fn shift_attribute(&mut self) -> GoBytes {
        let name_start = self.r.pos();
        let mut c: u8;
        loop {
            // attribute name state
            c = self.r.peek(0);
            if c == b' '
                || c == b'='
                || c == b'>'
                || (c == b'/' || c == b'?') && self.r.peek(1) == b'>'
                || c == b'\t'
                || c == b'\n'
                || c == b'\r'
                || c == 0
            {
                break;
            }
            self.r.move_(1);
        }
        let name_end = self.r.pos();
        loop {
            // after attribute name state
            c = self.r.peek(0);
            if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' {
                self.r.move_(1);
                continue;
            }
            break;
        }
        if c == b'=' {
            self.r.move_(1);
            loop {
                // before attribute value state
                c = self.r.peek(0);
                if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' {
                    self.r.move_(1);
                    continue;
                }
                break;
            }
            let attr_pos = self.r.pos();
            let delim = c;
            if delim == b'"' || delim == b'\'' {
                // attribute value single- and double-quoted state
                self.r.move_(1);
                loop {
                    c = self.r.peek(0);
                    if c == delim {
                        self.r.move_(1);
                        break;
                    } else if c == 0 {
                        break;
                    }
                    self.r.move_(1);
                    if c == b'\t' || c == b'\n' || c == b'\r' {
                        // in place: rewrites the input buffer
                        self.r.lexeme().set(self.r.pos() - 1, b' ');
                    }
                }
            } else {
                // attribute value unquoted state
                loop {
                    c = self.r.peek(0);
                    if c == b' '
                        || c == b'>'
                        || (c == b'/' || c == b'?') && self.r.peek(1) == b'>'
                        || c == b'\t'
                        || c == b'\n'
                        || c == b'\r'
                        || c == 0
                    {
                        break;
                    }
                    self.r.move_(1);
                }
            }
            self.attr_val = self.r.lexeme().slice_from(attr_pos);
        } else {
            self.r.rewind(name_end);
            self.attr_val = GoBytes::nil();
        }
        self.text = self.r.lexeme().slice(name_start, name_end);
        self.r.shift()
    }

    // Go: parse/xml/lex.go:Lexer.shiftEndTag
    fn shift_end_tag(&mut self) -> GoBytes {
        loop {
            let c = self.r.peek(0);
            if c == b'>' {
                self.text = self.r.lexeme().slice_from(2);
                self.r.move_(1);
                break;
            } else if c == 0 {
                self.text = self.r.lexeme().slice_from(2);
                break;
            }
            self.r.move_(1);
        }

        let mut end = self.text.len();
        while end > 0 {
            let c = self.text.at(end - 1);
            if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' {
                end -= 1;
                continue;
            }
            break;
        }
        self.text = self.text.slice_to(end);
        self.r.shift()
    }
}
