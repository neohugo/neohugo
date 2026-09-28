//! Port of `golang.org/x/net@v0.41.0/html/token.go` (the tokenizer).
//!
//! Go reads from an `io.Reader` into a growing buffer; `html.Parse` is only ever given a
//! `strings.Reader` here, so the port holds the whole input and its spans are absolute offsets
//! (Go's are relative to a buffer it compacts; the bytes they denote are the same). Go lower-cases
//! tag names and converts newlines in place in its buffer; those bytes are never read again, so
//! the port works on copies.

use super::atom::{self, Atom};
use super::escape::{lower, unescape};

/// Go: `html.TokenType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenType {
    /// ErrorToken means that an error occurred during tokenization.
    Error,
    /// TextToken means a text node.
    Text,
    /// A StartTagToken looks like <a>.
    StartTag,
    /// An EndTagToken looks like </a>.
    EndTag,
    /// A SelfClosingTagToken tag looks like <br/>.
    SelfClosingTag,
    /// A CommentToken looks like <!--x-->.
    Comment,
    /// A DoctypeToken looks like <!DOCTYPE x>
    Doctype,
}

/// Go: `html.Attribute` — a namespace-key-value triple. Namespace is non-empty for foreign
/// attributes like xlink, Key is alphabetic, and Val is unescaped.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Attribute {
    pub namespace: Vec<u8>,
    pub key: Vec<u8>,
    pub val: Vec<u8>,
}

/// Go: `html.Token`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub typ: TokenType,
    pub data_atom: Atom,
    pub data: Vec<u8>,
    pub attr: Vec<Attribute>,
}

impl Default for Token {
    fn default() -> Self {
        Token {
            typ: TokenType::Error,
            data_atom: 0,
            data: Vec::new(),
            attr: Vec::new(),
        }
    }
}

/// Go: `span` — a range of bytes in the input. The start is inclusive, the end is exclusive.
#[derive(Clone, Copy, Debug, Default)]
struct Span {
    start: usize,
    end: usize,
}

/// Go: `html.Tokenizer`.
pub struct Tokenizer {
    /// The whole input (Go's `r` + `buf`).
    buf: Vec<u8>,
    /// tt is the TokenType of the current token.
    tt: TokenType,
    /// Go's `err`: set (to io.EOF, the only error of an in-memory reader) once the input is
    /// exhausted; never reset.
    eof: bool,
    /// buf[raw.start:raw.end] holds the raw bytes of the current token.
    raw: Span,
    /// buf[data.start:data.end] holds the raw bytes of the current token's data.
    data: Span,
    /// pendingAttr is the attribute key and value currently being tokenized.
    pending_attr: [Span; 2],
    attr: Vec<[Span; 2]>,
    n_attr_returned: usize,
    /// rawTag is the "script" in "</script>" that closes the next token (lower-cased).
    raw_tag: Vec<u8>,
    /// textIsRaw is whether the current text token's data is not escaped.
    text_is_raw: bool,
    /// convertNUL is whether NUL bytes in the current token's data should be converted into
    /// � replacement characters.
    convert_nul: bool,
    /// allowCDATA is whether CDATA sections are allowed in the current context.
    allow_cdata: bool,
}

/// Go's `' ', '\n', '\r', '\t', '\f'`.
fn is_ws(c: u8) -> bool {
    matches!(c, b' ' | b'\n' | b'\r' | b'\t' | b'\x0c')
}

impl Tokenizer {
    /// Go: `NewTokenizer(r)`.
    // Go: html/token.go:NewTokenizer
    pub fn new(input: &[u8]) -> Tokenizer {
        Tokenizer::new_fragment(input, b"")
    }

    /// Go: `NewTokenizerFragment(r, contextTag)`.
    // Go: html/token.go:NewTokenizerFragment
    pub fn new_fragment(input: &[u8], context_tag: &[u8]) -> Tokenizer {
        let mut z = Tokenizer {
            buf: input.to_vec(),
            tt: TokenType::Error,
            eof: false,
            raw: Span::default(),
            data: Span::default(),
            pending_attr: [Span::default(); 2],
            attr: Vec::new(),
            n_attr_returned: 0,
            raw_tag: Vec::new(),
            text_is_raw: false,
            convert_nul: false,
            allow_cdata: false,
        };
        if !context_tag.is_empty() {
            let s = go_unicode::strings::to_lower(context_tag).into_owned();
            match s.as_slice() {
                b"iframe" | b"noembed" | b"noframes" | b"noscript" | b"plaintext" | b"script"
                | b"style" | b"title" | b"textarea" | b"xmp" => z.raw_tag = s,
                _ => {}
            }
        }
        z
    }

    /// Go: `AllowCDATA(allowCDATA)`.
    // Go: html/token.go:AllowCDATA
    pub fn allow_cdata(&mut self, allow_cdata: bool) {
        self.allow_cdata = allow_cdata;
    }

    /// Go: `NextIsNotRawText()`.
    // Go: html/token.go:NextIsNotRawText
    pub fn next_is_not_raw_text(&mut self) {
        self.raw_tag.clear();
    }

    /// Go: `Err()` — whether the most recent ErrorToken came from the end of the input.
    // Go: html/token.go:Err
    pub fn err_is_eof(&self) -> bool {
        self.tt == TokenType::Error && self.eof
    }

    /// Go: `readByte()`; at the end of the input it sets the error and returns 0.
    // Go: html/token.go:readByte
    fn read_byte(&mut self) -> u8 {
        if self.raw.end >= self.buf.len() {
            self.eof = true;
            return 0;
        }
        let x = self.buf[self.raw.end];
        self.raw.end += 1;
        x
    }

    // Go: html/token.go:skipWhiteSpace
    fn skip_white_space(&mut self) {
        if self.eof {
            return;
        }
        loop {
            let c = self.read_byte();
            if self.eof {
                return;
            }
            if !is_ws(c) {
                self.raw.end -= 1;
                return;
            }
        }
    }

    /// readRawOrRCDATA reads until the next "</foo>", where "foo" is z.rawTag.
    // Go: html/token.go:readRawOrRCDATA
    fn read_raw_or_rcdata(&mut self) {
        if self.raw_tag == b"script" {
            self.read_script();
            self.text_is_raw = true;
            self.raw_tag.clear();
            return;
        }
        loop {
            let c = self.read_byte();
            if self.eof {
                break;
            }
            if c != b'<' {
                continue;
            }
            let c = self.read_byte();
            if self.eof {
                break;
            }
            if c != b'/' {
                self.raw.end -= 1;
                continue;
            }
            if self.read_raw_end_tag() || self.eof {
                break;
            }
        }
        self.data.end = self.raw.end;
        // A textarea's or title's RCDATA can contain escaped entities.
        self.text_is_raw = self.raw_tag != b"textarea" && self.raw_tag != b"title";
        self.raw_tag.clear();
    }

    /// readRawEndTag attempts to read a tag like "</foo>", where "foo" is z.rawTag. If it
    /// succeeds, it backs up the input position to reconsume the tag and returns true.
    // Go: html/token.go:readRawEndTag
    fn read_raw_end_tag(&mut self) -> bool {
        for i in 0..self.raw_tag.len() {
            let c = self.read_byte();
            if self.eof {
                return false;
            }
            let t = self.raw_tag[i];
            if c != t && c != t.wrapping_sub(b'a' - b'A') {
                self.raw.end -= 1;
                return false;
            }
        }
        let c = self.read_byte();
        if self.eof {
            return false;
        }
        match c {
            b' ' | b'\n' | b'\r' | b'\t' | b'\x0c' | b'/' | b'>' => {
                // The 3 is 2 for the leading "</" plus 1 for the trailing character c.
                self.raw.end -= 3 + self.raw_tag.len();
                return true;
            }
            _ => {}
        }
        self.raw.end -= 1;
        false
    }

    /// readScript reads until the next </script> tag, following the byzantine rules for
    /// escaping/hiding the closing tag.
    // Go: html/token.go:readScript
    fn read_script(&mut self) {
        #[derive(Clone, Copy)]
        enum St {
            ScriptData,
            ScriptDataLessThanSign,
            ScriptDataEndTagOpen,
            ScriptDataEscapeStart,
            ScriptDataEscapeStartDash,
            ScriptDataEscaped,
            ScriptDataEscapedDash,
            ScriptDataEscapedDashDash,
            ScriptDataEscapedLessThanSign,
            ScriptDataEscapedEndTagOpen,
            ScriptDataDoubleEscapeStart,
            ScriptDataDoubleEscaped,
            ScriptDataDoubleEscapedDash,
            ScriptDataDoubleEscapedDashDash,
            ScriptDataDoubleEscapedLessThanSign,
            ScriptDataDoubleEscapeEnd,
        }
        use St::*;
        let mut st = ScriptData;
        loop {
            match st {
                ScriptData => {
                    let c = self.read_byte();
                    if self.eof {
                        break;
                    }
                    if c == b'<' {
                        st = ScriptDataLessThanSign;
                    }
                }
                ScriptDataLessThanSign => {
                    let c = self.read_byte();
                    if self.eof {
                        break;
                    }
                    match c {
                        b'/' => st = ScriptDataEndTagOpen,
                        b'!' => st = ScriptDataEscapeStart,
                        _ => {
                            self.raw.end -= 1;
                            st = ScriptData;
                        }
                    }
                }
                ScriptDataEndTagOpen => {
                    if self.read_raw_end_tag() || self.eof {
                        break;
                    }
                    st = ScriptData;
                }
                ScriptDataEscapeStart => {
                    let c = self.read_byte();
                    if self.eof {
                        break;
                    }
                    if c == b'-' {
                        st = ScriptDataEscapeStartDash;
                    } else {
                        self.raw.end -= 1;
                        st = ScriptData;
                    }
                }
                ScriptDataEscapeStartDash => {
                    let c = self.read_byte();
                    if self.eof {
                        break;
                    }
                    if c == b'-' {
                        st = ScriptDataEscapedDashDash;
                    } else {
                        self.raw.end -= 1;
                        st = ScriptData;
                    }
                }
                ScriptDataEscaped => {
                    let c = self.read_byte();
                    if self.eof {
                        break;
                    }
                    match c {
                        b'-' => st = ScriptDataEscapedDash,
                        b'<' => st = ScriptDataEscapedLessThanSign,
                        _ => st = ScriptDataEscaped,
                    }
                }
                ScriptDataEscapedDash => {
                    let c = self.read_byte();
                    if self.eof {
                        break;
                    }
                    match c {
                        b'-' => st = ScriptDataEscapedDashDash,
                        b'<' => st = ScriptDataEscapedLessThanSign,
                        _ => st = ScriptDataEscaped,
                    }
                }
                ScriptDataEscapedDashDash => {
                    let c = self.read_byte();
                    if self.eof {
                        break;
                    }
                    match c {
                        b'-' => st = ScriptDataEscapedDashDash,
                        b'<' => st = ScriptDataEscapedLessThanSign,
                        b'>' => st = ScriptData,
                        _ => st = ScriptDataEscaped,
                    }
                }
                ScriptDataEscapedLessThanSign => {
                    let c = self.read_byte();
                    if self.eof {
                        break;
                    }
                    if c == b'/' {
                        st = ScriptDataEscapedEndTagOpen;
                    } else if c.is_ascii_lowercase() || c.is_ascii_uppercase() {
                        st = ScriptDataDoubleEscapeStart;
                    } else {
                        self.raw.end -= 1;
                        st = ScriptData;
                    }
                }
                ScriptDataEscapedEndTagOpen => {
                    if self.read_raw_end_tag() || self.eof {
                        break;
                    }
                    st = ScriptDataEscaped;
                }
                ScriptDataDoubleEscapeStart => {
                    self.raw.end -= 1;
                    let mut matched = true;
                    for i in 0..6 {
                        let c = self.read_byte();
                        if self.eof {
                            self.data.end = self.raw.end;
                            return;
                        }
                        if c != b"script"[i] && c != b"SCRIPT"[i] {
                            self.raw.end -= 1;
                            matched = false;
                            break;
                        }
                    }
                    if !matched {
                        st = ScriptDataEscaped;
                        continue;
                    }
                    let c = self.read_byte();
                    if self.eof {
                        break;
                    }
                    match c {
                        b' ' | b'\n' | b'\r' | b'\t' | b'\x0c' | b'/' | b'>' => {
                            st = ScriptDataDoubleEscaped;
                        }
                        _ => {
                            self.raw.end -= 1;
                            st = ScriptDataEscaped;
                        }
                    }
                }
                ScriptDataDoubleEscaped => {
                    let c = self.read_byte();
                    if self.eof {
                        break;
                    }
                    match c {
                        b'-' => st = ScriptDataDoubleEscapedDash,
                        b'<' => st = ScriptDataDoubleEscapedLessThanSign,
                        _ => st = ScriptDataDoubleEscaped,
                    }
                }
                ScriptDataDoubleEscapedDash => {
                    let c = self.read_byte();
                    if self.eof {
                        break;
                    }
                    match c {
                        b'-' => st = ScriptDataDoubleEscapedDashDash,
                        b'<' => st = ScriptDataDoubleEscapedLessThanSign,
                        _ => st = ScriptDataDoubleEscaped,
                    }
                }
                ScriptDataDoubleEscapedDashDash => {
                    let c = self.read_byte();
                    if self.eof {
                        break;
                    }
                    match c {
                        b'-' => st = ScriptDataDoubleEscapedDashDash,
                        b'<' => st = ScriptDataDoubleEscapedLessThanSign,
                        b'>' => st = ScriptData,
                        _ => st = ScriptDataDoubleEscaped,
                    }
                }
                ScriptDataDoubleEscapedLessThanSign => {
                    let c = self.read_byte();
                    if self.eof {
                        break;
                    }
                    if c == b'/' {
                        st = ScriptDataDoubleEscapeEnd;
                    } else {
                        self.raw.end -= 1;
                        st = ScriptDataDoubleEscaped;
                    }
                }
                ScriptDataDoubleEscapeEnd => {
                    if self.read_raw_end_tag() {
                        self.raw.end += b"</script>".len();
                        st = ScriptDataEscaped;
                        continue;
                    }
                    if self.eof {
                        break;
                    }
                    st = ScriptDataDoubleEscaped;
                }
            }
        }
        // Go: `defer func() { z.data.end = z.raw.end }()`.
        self.data.end = self.raw.end;
    }

    /// readComment reads the next comment token starting with "<!--". The opening "<!--" has
    /// already been consumed.
    // Go: html/token.go:readComment
    fn read_comment(&mut self) {
        self.data.start = self.raw.end;
        self.read_comment_body();
        // Go: the deferred fix-up.
        if self.data.end < self.data.start {
            // It's a comment with no data, like <!-->.
            self.data.end = self.data.start;
        }
    }

    fn read_comment_body(&mut self) {
        let mut dash_count = 0;
        let mut beginning = true;
        loop {
            let c = self.read_byte();
            if self.eof {
                self.data.end = self.calculate_abrupt_comment_data_end();
                return;
            }
            match c {
                b'-' => {
                    dash_count += 1;
                    continue;
                }
                b'>' => {
                    if dash_count >= 2 || beginning {
                        // Go: z.raw.end - len("-->"), which may be below data.start (fixed
                        // up by the caller); "<!--" plus this '>' were read, so it is >= 2.
                        self.data.end = self.raw.end - 3;
                        return;
                    }
                }
                b'!' => {
                    if dash_count >= 2 {
                        let c = self.read_byte();
                        if self.eof {
                            self.data.end = self.calculate_abrupt_comment_data_end();
                            return;
                        } else if c == b'>' {
                            self.data.end = self.raw.end - b"--!>".len();
                            return;
                        } else if c == b'-' {
                            dash_count = 1;
                            beginning = false;
                            continue;
                        }
                    }
                }
                _ => {}
            }
            dash_count = 0;
            beginning = false;
        }
    }

    // Go: html/token.go:calculateAbruptCommentDataEnd
    fn calculate_abrupt_comment_data_end(&self) -> usize {
        let mut raw = &self.buf[self.raw.start..self.raw.end];
        const PREFIX_LEN: usize = 4; // len("<!--")
        if raw.len() >= PREFIX_LEN {
            raw = &raw[PREFIX_LEN..];
            if raw.ends_with(b"--!") {
                return self.raw.end - 3;
            } else if raw.ends_with(b"--") {
                return self.raw.end - 2;
            } else if raw.ends_with(b"-") {
                return self.raw.end - 1;
            }
        }
        self.raw.end
    }

    /// readUntilCloseAngle reads until the next ">".
    // Go: html/token.go:readUntilCloseAngle
    fn read_until_close_angle(&mut self) {
        self.data.start = self.raw.end;
        loop {
            let c = self.read_byte();
            if self.eof {
                self.data.end = self.raw.end;
                return;
            }
            if c == b'>' {
                self.data.end = self.raw.end - 1;
                return;
            }
        }
    }

    /// readMarkupDeclaration reads the next token starting with "<!". It might be a
    /// "<!--comment-->", a "<!DOCTYPE foo>", a "<![CDATA[section]]>" or "<!a bogus comment".
    // Go: html/token.go:readMarkupDeclaration
    fn read_markup_declaration(&mut self) -> TokenType {
        self.data.start = self.raw.end;
        let mut c = [0u8; 2];
        for ci in &mut c {
            *ci = self.read_byte();
            if self.eof {
                self.data.end = self.raw.end;
                return TokenType::Comment;
            }
        }
        if c[0] == b'-' && c[1] == b'-' {
            self.read_comment();
            return TokenType::Comment;
        }
        self.raw.end -= 2;
        if self.read_doctype() {
            return TokenType::Doctype;
        }
        if self.allow_cdata && self.read_cdata() {
            self.convert_nul = true;
            return TokenType::Text;
        }
        // It's a bogus comment.
        self.read_until_close_angle();
        TokenType::Comment
    }

    /// readDoctype attempts to read a doctype declaration and returns true if successful.
    // Go: html/token.go:readDoctype
    fn read_doctype(&mut self) -> bool {
        const S: &[u8] = b"DOCTYPE";
        for &si in S {
            let c = self.read_byte();
            if self.eof {
                self.data.end = self.raw.end;
                return false;
            }
            if c != si && c != si + (b'a' - b'A') {
                // Back up to read the fragment of "DOCTYPE" again.
                self.raw.end = self.data.start;
                return false;
            }
        }
        self.skip_white_space();
        if self.eof {
            self.data.start = self.raw.end;
            self.data.end = self.raw.end;
            return true;
        }
        self.read_until_close_angle();
        true
    }

    /// readCDATA attempts to read a CDATA section and returns true if successful.
    // Go: html/token.go:readCDATA
    fn read_cdata(&mut self) -> bool {
        const S: &[u8] = b"[CDATA[";
        for &si in S {
            let c = self.read_byte();
            if self.eof {
                self.data.end = self.raw.end;
                return false;
            }
            if c != si {
                // Back up to read the fragment of "[CDATA[" again.
                self.raw.end = self.data.start;
                return false;
            }
        }
        self.data.start = self.raw.end;
        let mut brackets = 0;
        loop {
            let c = self.read_byte();
            if self.eof {
                self.data.end = self.raw.end;
                return true;
            }
            match c {
                b']' => brackets += 1,
                b'>' => {
                    if brackets >= 2 {
                        self.data.end = self.raw.end - b"]]>".len();
                        return true;
                    }
                    brackets = 0;
                }
                _ => brackets = 0,
            }
        }
    }

    /// startTagIn returns whether the start tag in buf[data] case-insensitively matches any
    /// element of ss.
    // Go: html/token.go:startTagIn
    fn start_tag_in(&self, ss: &[&[u8]]) -> bool {
        let d = &self.buf[self.data.start..self.data.end];
        ss.iter()
            .any(|s| d.len() == s.len() && d.to_ascii_lowercase() == *s)
    }

    /// readStartTag reads the next start tag token. The opening "<a" has already been consumed.
    // Go: html/token.go:readStartTag
    fn read_start_tag(&mut self) -> TokenType {
        self.read_tag(true);
        if self.eof {
            return TokenType::Error;
        }
        // Several tags flag the tokenizer's next token as raw.
        let c = self.buf[self.data.start].to_ascii_lowercase();
        let raw = match c {
            b'i' => self.start_tag_in(&[b"iframe"]),
            b'n' => self.start_tag_in(&[b"noembed", b"noframes", b"noscript"]),
            b'p' => self.start_tag_in(&[b"plaintext"]),
            b's' => self.start_tag_in(&[b"script", b"style"]),
            b't' => self.start_tag_in(&[b"textarea", b"title"]),
            b'x' => self.start_tag_in(&[b"xmp"]),
            _ => false,
        };
        if raw {
            // strings.ToLower of an ASCII name (startTagIn matched it).
            self.raw_tag = self.buf[self.data.start..self.data.end].to_ascii_lowercase();
        }
        // Look for a self-closing token (e.g. <br/>): the last non-bracket character of the tag
        // must not be the last non-quote character of the last attribute (<p a=/>).
        let n_attrs = self.attr.len();
        if !self.eof
            && self.buf[self.raw.end - 2] == b'/'
            && (n_attrs == 0
                || self.raw.end as isize - 2 != self.attr[n_attrs - 1][1].end as isize - 1)
        {
            return TokenType::SelfClosingTag;
        }
        TokenType::StartTag
    }

    /// readTag reads the next tag token and its attributes. If saveAttr, those attributes are
    /// saved in z.attr, otherwise z.attr is set to an empty slice.
    // Go: html/token.go:readTag
    fn read_tag(&mut self, save_attr: bool) {
        self.attr.clear();
        self.n_attr_returned = 0;
        // Read the tag name and attribute key/value pairs.
        self.read_tag_name();
        self.skip_white_space();
        if self.eof {
            return;
        }
        loop {
            let c = self.read_byte();
            if self.eof || c == b'>' {
                break;
            }
            self.raw.end -= 1;
            self.read_tag_attr_key();
            self.read_tag_attr_val();
            // Save pendingAttr if saveAttr and that attribute has a non-empty key.
            if save_attr && self.pending_attr[0].start != self.pending_attr[0].end {
                self.attr.push(self.pending_attr);
            }
            self.skip_white_space();
            if self.eof {
                break;
            }
        }
    }

    /// readTagName sets z.data to the "div" in "<div k=v>".
    // Go: html/token.go:readTagName
    fn read_tag_name(&mut self) {
        self.data.start = self.raw.end - 1;
        loop {
            let c = self.read_byte();
            if self.eof {
                self.data.end = self.raw.end;
                return;
            }
            match c {
                b' ' | b'\n' | b'\r' | b'\t' | b'\x0c' => {
                    self.data.end = self.raw.end - 1;
                    return;
                }
                b'/' | b'>' => {
                    self.raw.end -= 1;
                    self.data.end = self.raw.end;
                    return;
                }
                _ => {}
            }
        }
    }

    /// readTagAttrKey sets z.pendingAttr[0] to the "k" in "<div k=v>".
    // Go: html/token.go:readTagAttrKey
    fn read_tag_attr_key(&mut self) {
        self.pending_attr[0].start = self.raw.end;
        loop {
            let c = self.read_byte();
            if self.eof {
                self.pending_attr[0].end = self.raw.end;
                return;
            }
            match c {
                b'=' if self.pending_attr[0].start + 1 == self.raw.end => {
                    // WHATWG 13.2.5.32, if we see an equals sign before the attribute name
                    // begins, we treat it as a character in the attribute name and continue.
                    continue;
                }
                b'=' | b' ' | b'\n' | b'\r' | b'\t' | b'\x0c' | b'/' | b'>' => {
                    // WHATWG 13.2.5.33 Attribute name state
                    // We need to reconsume the char in the after attribute name state to support
                    // the / character
                    self.raw.end -= 1;
                    self.pending_attr[0].end = self.raw.end;
                    return;
                }
                _ => {}
            }
        }
    }

    /// readTagAttrVal sets z.pendingAttr[1] to the "v" in "<div k=v>".
    // Go: html/token.go:readTagAttrVal
    fn read_tag_attr_val(&mut self) {
        self.pending_attr[1].start = self.raw.end;
        self.pending_attr[1].end = self.raw.end;
        self.skip_white_space();
        if self.eof {
            return;
        }
        let c = self.read_byte();
        if self.eof {
            return;
        }
        if c == b'/' {
            // WHATWG 13.2.5.34 After attribute name state
            // U+002F SOLIDUS (/) - Switch to the self-closing start tag state.
            return;
        }
        if c != b'=' {
            self.raw.end -= 1;
            return;
        }
        self.skip_white_space();
        if self.eof {
            return;
        }
        let quote = self.read_byte();
        if self.eof {
            return;
        }
        match quote {
            b'>' => {
                self.raw.end -= 1;
            }
            b'\'' | b'"' => {
                self.pending_attr[1].start = self.raw.end;
                loop {
                    let c = self.read_byte();
                    if self.eof {
                        self.pending_attr[1].end = self.raw.end;
                        return;
                    }
                    if c == quote {
                        self.pending_attr[1].end = self.raw.end - 1;
                        return;
                    }
                }
            }
            _ => {
                self.pending_attr[1].start = self.raw.end - 1;
                loop {
                    let c = self.read_byte();
                    if self.eof {
                        self.pending_attr[1].end = self.raw.end;
                        return;
                    }
                    match c {
                        b' ' | b'\n' | b'\r' | b'\t' | b'\x0c' => {
                            self.pending_attr[1].end = self.raw.end - 1;
                            return;
                        }
                        b'>' => {
                            self.raw.end -= 1;
                            self.pending_attr[1].end = self.raw.end;
                            return;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Next scans the next token and returns its type.
    // Go: html/token.go:Next
    pub fn next(&mut self) -> TokenType {
        self.raw.start = self.raw.end;
        self.data.start = self.raw.end;
        self.data.end = self.raw.end;
        if self.eof {
            self.tt = TokenType::Error;
            return self.tt;
        }
        if !self.raw_tag.is_empty() {
            if self.raw_tag == b"plaintext" {
                // Read everything up to EOF.
                while !self.eof {
                    self.read_byte();
                }
                self.data.end = self.raw.end;
                self.text_is_raw = true;
            } else {
                self.read_raw_or_rcdata();
            }
            if self.data.end > self.data.start {
                self.tt = TokenType::Text;
                self.convert_nul = true;
                return self.tt;
            }
        }
        self.text_is_raw = false;
        self.convert_nul = false;

        loop {
            let c = self.read_byte();
            if self.eof {
                break;
            }
            if c != b'<' {
                continue;
            }

            // Check if the '<' we have just read is part of a tag, comment or doctype. If not,
            // it's part of the accumulated text token.
            let c = self.read_byte();
            if self.eof {
                break;
            }
            let token_type = if c.is_ascii_lowercase() || c.is_ascii_uppercase() {
                TokenType::StartTag
            } else if c == b'/' {
                TokenType::EndTag
            } else if c == b'!' || c == b'?' {
                // We use CommentToken to mean any of "<!--actual comments-->",
                // "<!DOCTYPE declarations>" and "<?xml processing instructions?>".
                TokenType::Comment
            } else {
                // Reconsume the current character.
                self.raw.end -= 1;
                continue;
            };

            // We have a non-text token, but we might have accumulated some text before that.
            // If so, we return the text first, and return the non-text token on the subsequent
            // call to Next.
            let x = self.raw.end - 2;
            if self.raw.start < x {
                self.raw.end = x;
                self.data.end = x;
                self.tt = TokenType::Text;
                return self.tt;
            }
            match token_type {
                TokenType::StartTag => {
                    self.tt = self.read_start_tag();
                    return self.tt;
                }
                TokenType::EndTag => {
                    let c = self.read_byte();
                    if self.eof {
                        break;
                    }
                    if c == b'>' {
                        // "</>" does not generate a token at all. Generate an empty comment to
                        // allow passthrough clients to pick up the data using Raw.
                        self.tt = TokenType::Comment;
                        return self.tt;
                    }
                    if c.is_ascii_lowercase() || c.is_ascii_uppercase() {
                        self.read_tag(false);
                        if self.eof {
                            self.tt = TokenType::Error;
                        } else {
                            self.tt = TokenType::EndTag;
                        }
                        return self.tt;
                    }
                    self.raw.end -= 1;
                    self.read_until_close_angle();
                    self.tt = TokenType::Comment;
                    return self.tt;
                }
                _ => {
                    if c == b'!' {
                        self.tt = self.read_markup_declaration();
                        return self.tt;
                    }
                    self.raw.end -= 1;
                    self.read_until_close_angle();
                    self.tt = TokenType::Comment;
                    return self.tt;
                }
            }
        }
        if self.raw.start < self.raw.end {
            self.data.end = self.raw.end;
            self.tt = TokenType::Text;
            return self.tt;
        }
        self.tt = TokenType::Error;
        self.tt
    }

    /// Raw returns the unmodified text of the current token.
    // Go: html/token.go:Raw
    pub fn raw(&self) -> &[u8] {
        &self.buf[self.raw.start..self.raw.end]
    }

    /// Text returns the unescaped text of a text, comment or doctype token.
    // Go: html/token.go:Text
    pub fn text(&mut self) -> Option<Vec<u8>> {
        match self.tt {
            TokenType::Text | TokenType::Comment | TokenType::Doctype => {
                let mut s = convert_newlines(&self.buf[self.data.start..self.data.end]);
                self.data.start = self.raw.end;
                self.data.end = self.raw.end;
                if (self.convert_nul || self.tt == TokenType::Comment) && s.contains(&0) {
                    s = go_unicode::bytes::replace_all(&s, b"\x00", "\u{fffd}".as_bytes());
                }
                if !self.text_is_raw {
                    s = unescape(&s, false);
                }
                Some(s)
            }
            _ => None,
        }
    }

    /// TagName returns the lower-cased name of a tag token and whether the tag has attributes.
    // Go: html/token.go:TagName
    pub fn tag_name(&mut self) -> (Option<Vec<u8>>, bool) {
        if self.data.start < self.data.end {
            match self.tt {
                TokenType::StartTag | TokenType::EndTag | TokenType::SelfClosingTag => {
                    let s = lower(&self.buf[self.data.start..self.data.end]);
                    self.data.start = self.raw.end;
                    self.data.end = self.raw.end;
                    return (Some(s), self.n_attr_returned < self.attr.len());
                }
                _ => {}
            }
        }
        (None, false)
    }

    /// TagAttr returns the lower-cased key and unescaped value of the next unparsed attribute
    /// for the current tag token and whether there are more attributes.
    // Go: html/token.go:TagAttr
    pub fn tag_attr(&mut self) -> (Vec<u8>, Vec<u8>, bool) {
        if self.n_attr_returned < self.attr.len() {
            match self.tt {
                TokenType::StartTag | TokenType::SelfClosingTag => {
                    let x = self.attr[self.n_attr_returned];
                    self.n_attr_returned += 1;
                    let key = lower(&self.buf[x[0].start..x[0].end]);
                    let val = unescape(&convert_newlines(&self.buf[x[1].start..x[1].end]), true);
                    return (key, val, self.n_attr_returned < self.attr.len());
                }
                _ => {}
            }
        }
        (Vec::new(), Vec::new(), false)
    }

    /// Token returns the current Token.
    // Go: html/token.go:Token
    pub fn token(&mut self) -> Token {
        let mut t = Token {
            typ: self.tt,
            ..Default::default()
        };
        match self.tt {
            TokenType::Text | TokenType::Comment | TokenType::Doctype => {
                t.data = self.text().unwrap_or_default();
            }
            TokenType::StartTag | TokenType::SelfClosingTag | TokenType::EndTag => {
                let (name, mut more_attr) = self.tag_name();
                while more_attr {
                    let (key, val, more) = self.tag_attr();
                    more_attr = more;
                    t.attr.push(Attribute {
                        namespace: Vec::new(),
                        key: atom::string_of(&key),
                        val,
                    });
                }
                let name = name.unwrap_or_default();
                let a = atom::lookup(&name);
                if a != 0 {
                    t.data_atom = a;
                    t.data = atom::string(a).to_vec();
                } else {
                    t.data_atom = 0;
                    t.data = name;
                }
            }
            TokenType::Error => {}
        }
        t
    }
}

/// convertNewlines converts "\r" and "\r\n" in s to "\n".
// Go: html/token.go:convertNewlines
pub fn convert_newlines(s: &[u8]) -> Vec<u8> {
    if !s.contains(&b'\r') {
        return s.to_vec();
    }
    let mut out = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        if s[i] == b'\r' {
            if i + 1 < s.len() && s[i + 1] == b'\n' {
                i += 1;
            }
            out.push(b'\n');
        } else {
            out.push(s[i]);
        }
        i += 1;
    }
    out
}
