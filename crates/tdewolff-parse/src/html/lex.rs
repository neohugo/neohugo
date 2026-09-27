//! Go: parse/html/lex.go — an HTML5 lexer following the specifications at
//! <http://www.w3.org/TR/html5/syntax.html>.

use crate::error::{GoError, new_error_lexer_err};
use crate::gobytes::GoBytes;
use crate::html::hash::{
    Hash, Iframe, Math, Plaintext, Script, Style, Svg, Textarea, Title, Xmp, to_hash,
};
use crate::input::Input;
use crate::util::{copy, to_lower};

/// Go: html.TokenType — the type of token.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TokenType {
    ErrorToken = 0, // extra token when errors occur
    CommentToken,
    DoctypeToken,
    StartTagToken,
    StartTagCloseToken,
    StartTagVoidToken,
    EndTagToken,
    AttributeToken,
    TextToken,
    SvgToken,
    MathToken,
    TemplateToken,
}

pub use TokenType::*;

impl TokenType {
    // Go: parse/html/lex.go:TokenType.String
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
        2 => "Doctype",
        3 => "StartTag",
        4 => "StartTagClose",
        5 => "StartTagVoid",
        6 => "EndTag",
        7 => "Attribute",
        8 => "Text",
        9 => "Svg",
        10 => "Math",
        _ => return format!("Invalid({})", n),
    }
    .to_string()
}

pub const GO_TEMPLATE: [&str; 2] = ["{{", "}}"];
pub const HANDLEBARS_TEMPLATE: [&str; 2] = ["{{", "}}"];
pub const MUSTACHE_TEMPLATE: [&str; 2] = ["{{", "}}"];
pub const EJS_TEMPLATE: [&str; 2] = ["<%", "%>"];
pub const ASP_TEMPLATE: [&str; 2] = ["<%", "%>"];
pub const PHP_TEMPLATE: [&str; 2] = ["<?", "?>"];

/// Go: html.Lexer — the state for the lexer.
pub struct Lexer {
    r: Input,
    tmpl_begin: Vec<u8>,
    tmpl_end: Vec<u8>,
    err: Option<GoError>,

    raw_tag: Hash,
    in_tag: bool,

    text: GoBytes,
    attr_val: GoBytes,
    has_tmpl: bool,
}

// Go: parse/html/lex.go:Lexer.at
fn at(r: &Input, b: &[u8]) -> bool {
    for (i, &c) in b.iter().enumerate() {
        if r.peek(i) != c {
            return false;
        }
    }
    true
}

#[inline]
fn is_letter(c: u8) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_uppercase()
}

impl Lexer {
    // Go: parse/html/lex.go:NewLexer
    /// Returns a new Lexer for a given Input.
    pub fn new(r: Input) -> Lexer {
        Lexer {
            r,
            tmpl_begin: Vec::new(),
            tmpl_end: Vec::new(),
            err: None,
            raw_tag: Hash(0),
            in_tag: false,
            text: GoBytes::nil(),
            attr_val: GoBytes::nil(),
            has_tmpl: false,
        }
    }

    // Go: parse/html/lex.go:NewTemplateLexer
    pub fn new_template(r: Input, tmpl: [&str; 2]) -> Lexer {
        let mut l = Lexer::new(r);
        l.tmpl_begin = tmpl[0].as_bytes().to_vec();
        l.tmpl_end = tmpl[1].as_bytes().to_vec();
        l
    }

    /// The underlying `*parse.Input` (shared handle).
    pub fn input(&self) -> &Input {
        &self.r
    }

    // Go: parse/html/lex.go:Lexer.Err
    /// Returns the error encountered during lexing, this is often io.EOF but
    /// also other errors can be returned.
    pub fn err(&self) -> Option<GoError> {
        if self.err.is_some() {
            return self.err.clone();
        }
        self.r.err()
    }

    // Go: parse/html/lex.go:Lexer.Text
    /// Returns the textual representation of a token. This excludes
    /// delimiters and additional leading/trailing characters.
    pub fn text(&self) -> GoBytes {
        self.text.clone()
    }

    // Go: parse/html/lex.go:Lexer.AttrKey
    /// Returns the attribute key when an AttributeToken was returned from Next.
    pub fn attr_key(&self) -> GoBytes {
        self.text.clone()
    }

    // Go: parse/html/lex.go:Lexer.AttrVal
    /// Returns the attribute value when an AttributeToken was returned from Next.
    pub fn attr_val(&self) -> GoBytes {
        self.attr_val.clone()
    }

    // Go: parse/html/lex.go:Lexer.HasTemplate
    /// Returns true if the token value contains a template.
    pub fn has_template(&self) -> bool {
        self.has_tmpl
    }

    // Go: parse/html/lex.go:Lexer.Next
    /// Returns the next Token. It returns ErrorToken when an error was
    /// encountered. Using Err() one can retrieve the error message.
    pub fn next(&mut self) -> (TokenType, GoBytes) {
        self.text = GoBytes::nil();
        self.has_tmpl = false;
        let mut c: u8;
        if self.in_tag {
            self.attr_val = GoBytes::nil();
            loop {
                // before attribute name state
                c = self.r.peek(0);
                if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0C {
                    self.r.move_(1);
                    continue;
                }
                break;
            }
            if c == 0 && self.r.has_err() {
                return (ErrorToken, GoBytes::nil());
            } else if c != b'>' && (c != b'/' || self.r.peek(1) != b'>') {
                return (AttributeToken, self.shift_attribute());
            }
            self.r.skip();
            self.in_tag = false;
            if c == b'/' {
                self.r.move_(2);
                return (StartTagVoidToken, self.r.shift());
            }
            self.r.move_(1);
            return (StartTagCloseToken, self.r.shift());
        }

        if self.raw_tag != Hash(0) {
            let raw_text = self.shift_raw_text();
            if 0 < raw_text.len() {
                self.text = raw_text.clone();
                self.raw_tag = Hash(0);
                return (TextToken, raw_text);
            }
            self.raw_tag = Hash(0);
        }

        loop {
            c = self.r.peek(0);
            if 0 < self.tmpl_begin.len() && at(&self.r, &self.tmpl_begin) {
                if 0 < self.r.pos() {
                    self.text = self.r.shift();
                    return (TextToken, self.text.clone());
                }
                self.r.move_(self.tmpl_begin.len() as isize);
                self.move_template();
                self.has_tmpl = true;
                return (TemplateToken, self.r.shift());
            } else if c == b'<' {
                c = self.r.peek(1);
                let is_end_tag = c == b'/'
                    && self.r.peek(2) != b'>'
                    && (self.r.peek(2) != 0 || self.r.peek_err(2).is_none());
                if !is_end_tag && !is_letter(c) && c != b'!' && c != b'?' {
                    // not a tag
                    self.r.move_(1);
                } else if 0 < self.r.pos() {
                    // return currently buffered texttoken so that we can return tag next iteration
                    self.text = self.r.shift();
                    return (TextToken, self.text.clone());
                } else if is_end_tag {
                    self.r.move_(2);
                    // only endtags that are not followed by > or EOF arrive here
                    c = self.r.peek(0);
                    if !is_letter(c) {
                        return (CommentToken, self.shift_bogus_comment());
                    }
                    return (EndTagToken, self.shift_end_tag());
                } else if is_letter(c) {
                    self.r.move_(1);
                    self.in_tag = true;
                    return self.shift_start_tag();
                } else if c == b'!' {
                    self.r.move_(2);
                    return self.read_markup();
                } else if c == b'?' {
                    self.r.move_(1);
                    return (CommentToken, self.shift_bogus_comment());
                }
            } else if c == 0 && self.r.has_err() {
                if 0 < self.r.pos() {
                    self.text = self.r.shift();
                    return (TextToken, self.text.clone());
                }
                return (ErrorToken, GoBytes::nil());
            } else {
                self.r.move_(1);
            }
        }
    }

    ////////////////////////////////////////////////////////////////

    // The following functions follow the specifications at https://html.spec.whatwg.org/multipage/parsing.html

    // Go: parse/html/lex.go:Lexer.shiftRawText
    fn shift_raw_text(&mut self) -> GoBytes {
        if self.raw_tag == Plaintext {
            loop {
                if self.r.peek(0) == 0 && self.r.has_err() {
                    return self.r.shift();
                }
                self.r.move_(1);
            }
        } else {
            // RCDATA, RAWTEXT and SCRIPT
            loop {
                let c = self.r.peek(0);
                if c == b'<' {
                    if self.r.peek(1) == b'/' {
                        let mark = self.r.pos();
                        self.r.move_(2);
                        loop {
                            if !is_letter(self.r.peek(0)) {
                                break;
                            }
                            self.r.move_(1);
                        }
                        // copy so that ToLower doesn't change the case of the underlying slice
                        let h = to_hash(&to_lower(copy(&self.r.lexeme().slice_from(mark + 2))));
                        if h == self.raw_tag {
                            self.r.rewind(mark);
                            return self.r.shift();
                        }
                    } else if self.raw_tag == Script
                        && self.r.peek(1) == b'!'
                        && self.r.peek(2) == b'-'
                        && self.r.peek(3) == b'-'
                    {
                        self.r.move_(4);
                        let mut in_script = false;
                        loop {
                            let c = self.r.peek(0);
                            if c == b'-' && self.r.peek(1) == b'-' && self.r.peek(2) == b'>' {
                                self.r.move_(3);
                                break;
                            } else if c == b'<' {
                                let is_end = self.r.peek(1) == b'/';
                                if is_end {
                                    self.r.move_(2);
                                } else {
                                    self.r.move_(1);
                                }
                                let mark = self.r.pos();
                                loop {
                                    if !is_letter(self.r.peek(0)) {
                                        break;
                                    }
                                    self.r.move_(1);
                                }
                                // copy so that ToLower doesn't change the case of the underlying slice
                                let h = to_hash(&to_lower(copy(&self.r.lexeme().slice_from(mark))));
                                if h == Script {
                                    if !is_end {
                                        in_script = true;
                                    } else {
                                        if !in_script {
                                            self.r.rewind(mark - 2);
                                            return self.r.shift();
                                        }
                                        in_script = false;
                                    }
                                }
                            } else if c == 0 && self.r.has_err() {
                                return self.r.shift();
                            } else {
                                self.r.move_(1);
                            }
                        }
                    } else {
                        self.r.move_(1);
                    }
                } else if 0 < self.tmpl_begin.len() && at(&self.r, &self.tmpl_begin) {
                    self.r.move_(self.tmpl_begin.len() as isize);
                    self.move_template();
                    self.has_tmpl = true;
                } else if c == 0 && self.r.has_err() {
                    return self.r.shift();
                } else {
                    self.r.move_(1);
                }
            }
        }
    }

    // Go: parse/html/lex.go:Lexer.readMarkup
    fn read_markup(&mut self) -> (TokenType, GoBytes) {
        if at(&self.r, b"--") {
            self.r.move_(2);
            loop {
                if self.r.peek(0) == 0 && self.r.has_err() {
                    self.text = self.r.lexeme().slice_from(4);
                    return (CommentToken, self.r.shift());
                } else if at(&self.r, b"-->") {
                    self.text = self.r.lexeme().slice_from(4);
                    self.r.move_(3);
                    return (CommentToken, self.r.shift());
                } else if at(&self.r, b"--!>") {
                    self.text = self.r.lexeme().slice_from(4);
                    self.r.move_(4);
                    return (CommentToken, self.r.shift());
                }
                self.r.move_(1);
            }
        } else if at(&self.r, b"[CDATA[") {
            self.r.move_(7);
            loop {
                if self.r.peek(0) == 0 && self.r.has_err() {
                    self.text = self.r.lexeme().slice_from(9);
                    return (TextToken, self.r.shift());
                } else if at(&self.r, b"]]>") {
                    self.text = self.r.lexeme().slice_from(9);
                    self.r.move_(3);
                    return (TextToken, self.r.shift());
                }
                self.r.move_(1);
            }
        } else if self.at_case_insensitive(b"doctype") {
            self.r.move_(7);
            if self.r.peek(0) == b' ' {
                self.r.move_(1);
            }
            loop {
                let c = self.r.peek(0);
                if c == b'>' || c == 0 && self.r.has_err() {
                    self.text = self.r.lexeme().slice_from(9);
                    if c == b'>' {
                        self.r.move_(1);
                    }
                    return (DoctypeToken, self.r.shift());
                }
                self.r.move_(1);
            }
        }
        (CommentToken, self.shift_bogus_comment())
    }

    // Go: parse/html/lex.go:Lexer.shiftBogusComment
    fn shift_bogus_comment(&mut self) -> GoBytes {
        loop {
            let c = self.r.peek(0);
            if c == b'>' {
                self.text = self.r.lexeme().slice_from(2);
                self.r.move_(1);
                return self.r.shift();
            } else if c == 0 && self.r.has_err() {
                self.text = self.r.lexeme().slice_from(2);
                return self.r.shift();
            }
            self.r.move_(1);
        }
    }

    // Go: parse/html/lex.go:Lexer.shiftStartTag
    fn shift_start_tag(&mut self) -> (TokenType, GoBytes) {
        loop {
            // spec says only a-zA-Z0-9, but we're lenient here
            let c = self.r.peek(0);
            if c == b' '
                || c == b'>'
                || c == b'/' && self.r.peek(1) == b'>'
                || c == b'\t'
                || c == b'\n'
                || c == b'\r'
                || c == 0x0C
                || c == 0 && self.r.has_err()
                || 0 < self.tmpl_begin.len() && at(&self.r, &self.tmpl_begin)
            {
                break;
            }
            self.r.move_(1);
        }
        self.text = to_lower(self.r.lexeme().slice_from(1));
        let h = to_hash(&self.text);
        if h == Textarea
            || h == Title
            || h == Style
            || h == Xmp
            || h == Iframe
            || h == Script
            || h == Plaintext
            || h == Svg
            || h == Math
        {
            if h == Svg || h == Math {
                let data = self.shift_xml(h);
                if self.err.is_some() {
                    return (ErrorToken, GoBytes::nil());
                }

                self.in_tag = false;
                if h == Svg {
                    return (SvgToken, data);
                }
                return (MathToken, data);
            }
            self.raw_tag = h;
        }
        (StartTagToken, self.r.shift())
    }

    // Go: parse/html/lex.go:Lexer.shiftAttribute
    fn shift_attribute(&mut self) -> GoBytes {
        let name_start = self.r.pos();
        let mut c: u8;
        if 0 < self.tmpl_begin.len() && at(&self.r, &self.tmpl_begin) {
            self.r.move_(self.tmpl_begin.len() as isize);
            self.move_template();
            self.has_tmpl = true;
        }
        loop {
            // attribute name state
            c = self.r.peek(0);
            if c == b' '
                || c == b'='
                || c == b'>'
                || c == b'/' && self.r.peek(1) == b'>'
                || c == b'\t'
                || c == b'\n'
                || c == b'\r'
                || c == 0x0C
                || c == 0 && self.r.has_err()
            {
                break;
            }
            self.r.move_(1);
        }
        let name_end = self.r.pos();
        loop {
            // after attribute name state
            c = self.r.peek(0);
            if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0C {
                self.r.move_(1);
                continue;
            }
            break;
        }
        let name_has_tmpl = self.has_tmpl;
        if c == b'=' {
            self.r.move_(1);
            loop {
                // before attribute value state
                c = self.r.peek(0);
                if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0C {
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
                    let c = self.r.peek(0);
                    if c == delim {
                        self.r.move_(1);
                        break;
                    } else if 0 < self.tmpl_begin.len() && at(&self.r, &self.tmpl_begin) {
                        self.r.move_(self.tmpl_begin.len() as isize);
                        self.move_template();
                        self.has_tmpl = true;
                    } else if c == 0 && self.r.has_err() {
                        break;
                    } else {
                        self.r.move_(1);
                    }
                }
            } else if 0 < self.tmpl_begin.len() && at(&self.r, &self.tmpl_begin) {
                self.r.move_(self.tmpl_begin.len() as isize);
                self.move_template();
                self.has_tmpl = true;
            } else {
                // attribute value unquoted state
                loop {
                    let c = self.r.peek(0);
                    if c == b' '
                        || c == b'>'
                        || c == b'\t'
                        || c == b'\n'
                        || c == b'\r'
                        || c == 0x0C
                        || c == 0 && self.r.has_err()
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
        if 0 < self.tmpl_begin.len() && at(&self.r, &self.tmpl_begin) {
            self.r.move_(self.tmpl_begin.len() as isize);
            self.move_template();
            self.has_tmpl = true;
        }
        self.text = self.r.lexeme().slice(name_start, name_end);
        if !name_has_tmpl {
            self.text = to_lower(self.text.clone());
        }
        self.r.shift()
    }

    // Go: parse/html/lex.go:Lexer.shiftEndTag
    fn shift_end_tag(&mut self) -> GoBytes {
        loop {
            let c = self.r.peek(0);
            if c == b'>' {
                self.text = self.r.lexeme().slice_from(2);
                self.r.move_(1);
                break;
            } else if c == 0 && self.r.has_err() {
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
        to_lower(self.r.shift())
    }

    // Go: parse/html/lex.go:Lexer.shiftXML
    /// Parses the content of a svg or math tag according to the XML 1.1
    /// specifications, including the tag itself. So far we have already parsed
    /// `<svg` or `<math`.
    fn shift_xml(&mut self, raw_tag: Hash) -> GoBytes {
        let mut in_quote = false;
        loop {
            let c = self.r.peek(0);
            if c == b'"' {
                in_quote = !in_quote;
                self.r.move_(1);
            } else if c == b'<' && !in_quote && self.r.peek(1) == b'/' {
                let mark = self.r.pos();
                self.r.move_(2);
                loop {
                    if !is_letter(self.r.peek(0)) {
                        break;
                    }
                    self.r.move_(1);
                }
                // copy so that ToLower doesn't change the case of the underlying slice
                let h = to_hash(&to_lower(copy(&self.r.lexeme().slice_from(mark + 2))));
                if h == raw_tag {
                    break;
                }
            } else if c == 0 {
                if !self.r.has_err() {
                    self.err = Some(new_error_lexer_err(&self.r, "unexpected NULL character"));
                }
                return self.r.shift();
            } else {
                self.r.move_(1);
            }
        }

        loop {
            let c = self.r.peek(0);
            if c == b'>' {
                self.r.move_(1);
                break;
            } else if c == 0 {
                if !self.r.has_err() {
                    self.err = Some(new_error_lexer_err(&self.r, "unexpected NULL character"));
                }
                return self.r.shift();
            }
            self.r.move_(1);
        }
        self.r.shift()
    }

    // Go: parse/html/lex.go:Lexer.moveTemplate
    fn move_template(&mut self) {
        loop {
            let c = self.r.peek(0);
            if c == 0 && self.r.has_err() {
                return;
            } else if at(&self.r, &self.tmpl_end) {
                self.r.move_(self.tmpl_end.len() as isize);
                return;
            } else if c == b'"' || c == b'\'' {
                self.r.move_(1);
                let mut escape = false;
                loop {
                    let c2 = self.r.peek(0);
                    if c2 == 0 && self.r.has_err() {
                        return;
                    } else if !escape && c2 == c {
                        self.r.move_(1);
                        break;
                    } else if c2 == b'\\' {
                        escape = !escape;
                    } else {
                        escape = false;
                    }
                    self.r.move_(1);
                }
            } else {
                self.r.move_(1);
            }
        }
    }

    ////////////////////////////////////////////////////////////////

    // Go: parse/html/lex.go:Lexer.atCaseInsensitive
    fn at_case_insensitive(&self, b: &[u8]) -> bool {
        for (i, &c) in b.iter().enumerate() {
            if self.r.peek(i) != c && self.r.peek(i).wrapping_add(b'a' - b'A') != c {
                return false;
            }
        }
        true
    }
}
