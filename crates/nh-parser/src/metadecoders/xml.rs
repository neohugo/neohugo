//! Port of Go's `encoding/xml` decoder (go1.27.1 `encoding/xml/xml.go`): the subset
//! `clbanning/mxj`'s `NewMapXml` uses, `NewDecoder(bytes.NewReader(doc))` with the default
//! settings (`Strict`, no `AutoClose`, no `Entity` map, no `CharsetReader`, no `DefaultSpace`)
//! and `Decoder.Token`; plus `EscapeText`/`EscapeString` for the encoder.
//!
//! Owner: gaps follow-up of Wave B task T03 (parser-langs).
//!
//! Go strings are bytes here (`Vec<u8>`): names may hold any byte >= 0x80 until `isName`
//! rejects them, and error texts copy input bytes.

use std::collections::HashMap;

use go_unicode::utf8;
use go_unicode::{Range16, RangeTable, Rune};

/// Go: `xml.Name`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Name {
    pub space: Vec<u8>,
    pub local: Vec<u8>,
}

/// Go: `xml.Attr`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attr {
    pub name: Name,
    pub value: Vec<u8>,
}

/// Go: `xml.Token` (the token kinds `rawToken` returns).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    StartElement { name: Name, attr: Vec<Attr> },
    EndElement { name: Name },
    CharData(Vec<u8>),
    Comment(Vec<u8>),
    ProcInst { target: Vec<u8>, inst: Vec<u8> },
    Directive(Vec<u8>),
}

/// The decoder's error (Go's `d.err` values).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// `io.EOF`.
    Eof,
    /// Go: `*xml.SyntaxError`.
    Syntax { msg: Vec<u8>, line: i64 },
    /// A plain `fmt.Errorf` error (unsupported version, encoding without a `CharsetReader`).
    Other(Vec<u8>),
}

impl Error {
    /// Go's `err.Error()` bytes.
    // Go: encoding/xml/xml.go:(*SyntaxError).Error
    pub fn error_bytes(&self) -> Vec<u8> {
        match self {
            Error::Eof => b"EOF".to_vec(),
            Error::Syntax { msg, line } => {
                let mut b = format!("XML syntax error on line {line}: ").into_bytes();
                b.extend_from_slice(msg);
                b
            }
            Error::Other(m) => m.clone(),
        }
    }
}

const XML_URL: &[u8] = b"http://www.w3.org/XML/1998/namespace";
const XMLNS_PREFIX: &[u8] = b"xmlns";
const XML_PREFIX: &[u8] = b"xml";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StkKind {
    Start,
    Ns,
}

struct Stack {
    kind: StkKind,
    name: Name,
    ok: bool,
}

/// Go: `xml.Decoder` over an in-memory document.
pub struct Decoder<'a> {
    data: &'a [u8],
    rpos: usize,
    buf: Vec<u8>,
    stk: Vec<Stack>,
    need_close: bool,
    to_close: Name,
    next_byte: i32,
    ns: HashMap<Vec<u8>, Vec<u8>>,
    err: Option<Error>,
    line: i64,
}

impl<'a> Decoder<'a> {
    /// Go: `xml.NewDecoder(bytes.NewReader(data))`.
    // Go: encoding/xml/xml.go:NewDecoder
    pub fn new(data: &'a [u8]) -> Decoder<'a> {
        Decoder {
            data,
            rpos: 0,
            buf: Vec::new(),
            stk: Vec::new(),
            need_close: false,
            to_close: Name::default(),
            next_byte: -1,
            ns: HashMap::new(),
            err: None,
            line: 1,
        }
    }

    /// Token returns the next XML token in the input stream. At the end of the input stream,
    /// Token returns `Err(Error::Eof)`.
    // Go: encoding/xml/xml.go:(*Decoder).Token
    pub fn token(&mut self) -> Result<Token, Error> {
        let mut t = match self.raw_token() {
            Ok(t) => t,
            Err(mut err) => {
                if err == Error::Eof && !self.stk.is_empty() {
                    err = self.syntax_error(b"unexpected EOF");
                }
                return Err(err);
            }
        };
        match &mut t {
            Token::StartElement { name, attr } => {
                // In XML name spaces, the translations listed in the
                // attributes apply to the element name and
                // to the other attribute names, so process
                // the translations first.
                for a in attr.iter() {
                    if a.name.space == XMLNS_PREFIX {
                        let v = self.ns.get(&a.name.local).cloned();
                        self.push_ns(&a.name.local, v);
                        self.ns.insert(a.name.local.clone(), a.value.clone());
                    }
                    if a.name.space.is_empty() && a.name.local == XMLNS_PREFIX {
                        // Default space for untagged names
                        let v = self.ns.get(&b""[..]).cloned();
                        self.push_ns(b"", v);
                        self.ns.insert(Vec::new(), a.value.clone());
                    }
                }

                self.push_element(name.clone());
                self.translate(name, true);
                for a in attr.iter_mut() {
                    self.translate(&mut a.name, false);
                }
            }
            Token::EndElement { name } => {
                if !self.pop_element(name) {
                    return Err(self.err.clone().expect("popElement sets d.err"));
                }
            }
            _ => {}
        }
        Ok(t)
    }

    /// Apply name space translation to name n.
    // Go: encoding/xml/xml.go:(*Decoder).translate
    fn translate(&self, n: &mut Name, is_element_name: bool) {
        if n.space == XMLNS_PREFIX || (n.space.is_empty() && !is_element_name) {
            return;
        } else if n.space == XML_PREFIX {
            n.space = XML_URL.to_vec();
        } else if n.space.is_empty() && n.local == XMLNS_PREFIX {
            return;
        }
        if let Some(v) = self.ns.get(&n.space) {
            n.space = v.clone();
        } else if n.space.is_empty() {
            // d.DefaultSpace is "".
            n.space = Vec::new();
        }
    }

    // Go: encoding/xml/xml.go:(*Decoder).pushElement
    fn push_element(&mut self, name: Name) {
        self.stk.push(Stack {
            kind: StkKind::Start,
            name,
            ok: false,
        });
    }

    // Go: encoding/xml/xml.go:(*Decoder).pushNs
    fn push_ns(&mut self, local: &[u8], url: Option<Vec<u8>>) {
        self.stk.push(Stack {
            kind: StkKind::Ns,
            name: Name {
                local: local.to_vec(),
                space: url.clone().unwrap_or_default(),
            },
            ok: url.is_some(),
        });
    }

    /// Creates a SyntaxError with the current line number.
    // Go: encoding/xml/xml.go:(*Decoder).syntaxError
    fn syntax_error(&self, msg: &[u8]) -> Error {
        Error::Syntax {
            msg: msg.to_vec(),
            line: self.line,
        }
    }

    // Go: encoding/xml/xml.go:(*Decoder).popElement
    fn pop_element(&mut self, t: &mut Name) -> bool {
        let s = self.stk.pop();
        let name = t.clone();
        match s {
            Some(s) if s.kind == StkKind::Start => {
                if s.name.local != name.local {
                    let mut m = b"element <".to_vec();
                    m.extend_from_slice(&s.name.local);
                    m.extend_from_slice(b"> closed by </");
                    m.extend_from_slice(&name.local);
                    m.push(b'>');
                    self.err = Some(self.syntax_error(&m));
                    return false;
                } else if s.name.space != name.space {
                    let ns: &[u8] = if name.space.is_empty() {
                        b"\"\""
                    } else {
                        &name.space
                    };
                    let mut m = b"element <".to_vec();
                    m.extend_from_slice(&s.name.local);
                    m.extend_from_slice(b"> in space ");
                    m.extend_from_slice(&s.name.space);
                    m.extend_from_slice(b" closed by </");
                    m.extend_from_slice(&name.local);
                    m.extend_from_slice(b"> in space ");
                    m.extend_from_slice(ns);
                    self.err = Some(self.syntax_error(&m));
                    return false;
                }
            }
            _ => {
                let mut m = b"unexpected end element </".to_vec();
                m.extend_from_slice(&name.local);
                m.push(b'>');
                self.err = Some(self.syntax_error(&m));
                return false;
            }
        }

        self.translate(t, true);

        // Pop stack until a Start or EOF is on the top, undoing the
        // translations that were associated with the element we just closed.
        while let Some(top) = self.stk.last() {
            if top.kind == StkKind::Start {
                break;
            }
            let s = self.stk.pop().unwrap();
            if s.ok {
                self.ns.insert(s.name.local, s.name.space);
            } else {
                self.ns.remove(&s.name.local);
            }
        }

        true
    }

    fn fail<T>(&mut self, e: Error) -> Result<T, Error> {
        self.err = Some(e.clone());
        Err(e)
    }

    fn cur_err(&self) -> Error {
        self.err.clone().expect("d.err is set")
    }

    // Go: encoding/xml/xml.go:(*Decoder).rawToken
    fn raw_token(&mut self) -> Result<Token, Error> {
        if let Some(e) = &self.err {
            return Err(e.clone());
        }
        if self.need_close {
            // The last element we read was self-closing and
            // we returned just the StartElement half.
            // Return the EndElement half now.
            self.need_close = false;
            return Ok(Token::EndElement {
                name: std::mem::take(&mut self.to_close),
            });
        }

        let Some(b) = self.getc() else {
            return Err(self.cur_err());
        };

        if b != b'<' {
            // Text section.
            self.ungetc(b);
            let Some(data) = self.text(-1, false) else {
                return Err(self.cur_err());
            };
            return Ok(Token::CharData(data));
        }

        let Some(mut b) = self.mustgetc() else {
            return Err(self.cur_err());
        };
        match b {
            b'/' => {
                // </: End element
                let Some(name) = self.nsname() else {
                    if self.err.is_none() {
                        let e = self.syntax_error(b"expected element name after </");
                        return self.fail(e);
                    }
                    return Err(self.cur_err());
                };
                self.space();
                let Some(b) = self.mustgetc() else {
                    return Err(self.cur_err());
                };
                if b != b'>' {
                    let mut m = b"invalid characters between </".to_vec();
                    m.extend_from_slice(&name.local);
                    m.extend_from_slice(b" and >");
                    let e = self.syntax_error(&m);
                    return self.fail(e);
                }
                return Ok(Token::EndElement { name });
            }
            b'?' => {
                // <?: Processing instruction.
                let Some(target) = self.name() else {
                    if self.err.is_none() {
                        let e = self.syntax_error(b"expected target name after <?");
                        return self.fail(e);
                    }
                    return Err(self.cur_err());
                };
                self.space();
                self.buf.clear();
                let mut b0 = 0u8;
                loop {
                    let Some(b) = self.mustgetc() else {
                        return Err(self.cur_err());
                    };
                    self.buf.push(b);
                    if b0 == b'?' && b == b'>' {
                        break;
                    }
                    b0 = b;
                }
                let data = self.buf[..self.buf.len() - 2].to_vec(); // chop ?>

                if target == b"xml" {
                    let content = &data;
                    let ver = proc_inst(b"version", content);
                    if !ver.is_empty() && ver != b"1.0" {
                        let mut m = b"xml: unsupported version ".to_vec();
                        m.extend_from_slice(go_strconv::quote(ver).as_bytes());
                        m.extend_from_slice(b"; only version 1.0 is supported");
                        return self.fail(Error::Other(m));
                    }
                    let enc = proc_inst(b"encoding", content);
                    if !enc.is_empty()
                        && enc != b"utf-8"
                        && enc != b"UTF-8"
                        && !go_unicode::bytes::equal_fold(enc, b"utf-8")
                    {
                        // d.CharsetReader is nil (mxj.XmlCharsetReader).
                        let mut m = b"xml: encoding ".to_vec();
                        m.extend_from_slice(go_strconv::quote(enc).as_bytes());
                        m.extend_from_slice(b" declared but Decoder.CharsetReader is nil");
                        return self.fail(Error::Other(m));
                    }
                }
                return Ok(Token::ProcInst { target, inst: data });
            }
            b'!' => {
                // <!: Maybe comment, maybe CDATA.
                let Some(b2) = self.mustgetc() else {
                    return Err(self.cur_err());
                };
                b = b2;
                match b {
                    b'-' => {
                        // <!-
                        // Probably <!-- for a comment.
                        let Some(b) = self.mustgetc() else {
                            return Err(self.cur_err());
                        };
                        if b != b'-' {
                            let e = self.syntax_error(b"invalid sequence <!- not part of <!--");
                            return self.fail(e);
                        }
                        // Look for terminator.
                        self.buf.clear();
                        let (mut b0, mut b1) = (0u8, 0u8);
                        loop {
                            let Some(b) = self.mustgetc() else {
                                return Err(self.cur_err());
                            };
                            self.buf.push(b);
                            if b0 == b'-' && b1 == b'-' {
                                if b != b'>' {
                                    let e = self.syntax_error(
                                        b"invalid sequence \"--\" not allowed in comments",
                                    );
                                    return self.fail(e);
                                }
                                break;
                            }
                            b0 = b1;
                            b1 = b;
                        }
                        let data = self.buf[..self.buf.len() - 3].to_vec(); // chop -->
                        return Ok(Token::Comment(data));
                    }
                    b'[' => {
                        // <![
                        // Probably <![CDATA[.
                        for i in 0..6 {
                            let Some(b) = self.mustgetc() else {
                                return Err(self.cur_err());
                            };
                            if b != b"CDATA["[i] {
                                let e = self.syntax_error(b"invalid <![ sequence");
                                return self.fail(e);
                            }
                        }
                        // Have <![CDATA[.  Read text until ]]>.
                        let Some(data) = self.text(-1, true) else {
                            return Err(self.cur_err());
                        };
                        return Ok(Token::CharData(data));
                    }
                    _ => {}
                }

                // Probably a directive: <!DOCTYPE ...>, <!ENTITY ...>, etc.
                // We don't care, but accumulate for caller. Quoted angle
                // brackets do not count for nesting.
                self.buf.clear();
                self.buf.push(b);
                let mut inquote = 0u8;
                let mut depth = 0i64;
                loop {
                    let Some(nb) = self.mustgetc() else {
                        return Err(self.cur_err());
                    };
                    b = nb;
                    if inquote == 0 && b == b'>' && depth == 0 {
                        break;
                    }
                    // HandleB:
                    'handle_b: loop {
                        self.buf.push(b);
                        if b == inquote {
                            inquote = 0;
                        } else if inquote != 0 {
                            // in quotes, no special action
                        } else if b == b'\'' || b == b'"' {
                            inquote = b;
                        } else if b == b'>' && inquote == 0 {
                            depth -= 1;
                        } else if b == b'<' && inquote == 0 {
                            // Look for <!-- to begin comment.
                            let s = b"!--";
                            for i in 0..s.len() {
                                let Some(nb) = self.mustgetc() else {
                                    return Err(self.cur_err());
                                };
                                b = nb;
                                if b != s[i] {
                                    for &c in &s[..i] {
                                        self.buf.push(c);
                                    }
                                    depth += 1;
                                    continue 'handle_b;
                                }
                            }

                            // Remove < that was written above.
                            self.buf.pop();

                            // Look for terminator.
                            let (mut b0, mut b1) = (0u8, 0u8);
                            loop {
                                let Some(nb) = self.mustgetc() else {
                                    return Err(self.cur_err());
                                };
                                b = nb;
                                if b0 == b'-' && b1 == b'-' && b == b'>' {
                                    break;
                                }
                                b0 = b1;
                                b1 = b;
                            }

                            // Replace the comment with a space in the returned Directive
                            // body, so that markup parts that were separated by the comment
                            // (like a "<" and a "!") don't get joined when re-encoding the
                            // Directive, taking new semantic meaning.
                            self.buf.push(b' ');
                        }
                        break;
                    }
                }
                return Ok(Token::Directive(self.buf.clone()));
            }
            _ => {}
        }

        // Must be an open element like <a href="foo">
        self.ungetc(b);

        let mut empty = false;
        let Some(name) = self.nsname() else {
            if self.err.is_none() {
                let e = self.syntax_error(b"expected element name after <");
                return self.fail(e);
            }
            return Err(self.cur_err());
        };

        let mut attr: Vec<Attr> = Vec::new();
        loop {
            self.space();
            let Some(mut b) = self.mustgetc() else {
                return Err(self.cur_err());
            };
            if b == b'/' {
                empty = true;
                let Some(nb) = self.mustgetc() else {
                    return Err(self.cur_err());
                };
                b = nb;
                if b != b'>' {
                    let e = self.syntax_error(b"expected /> in element");
                    return self.fail(e);
                }
                break;
            }
            if b == b'>' {
                break;
            }
            self.ungetc(b);

            let Some(aname) = self.nsname() else {
                if self.err.is_none() {
                    let e = self.syntax_error(b"expected attribute name in element");
                    return self.fail(e);
                }
                return Err(self.cur_err());
            };
            self.space();
            let Some(b) = self.mustgetc() else {
                return Err(self.cur_err());
            };
            if b != b'=' {
                // d.Strict
                let e = self.syntax_error(b"attribute name without = in element");
                return self.fail(e);
            }
            self.space();
            let Some(data) = self.attrval() else {
                return Err(self.cur_err());
            };
            attr.push(Attr {
                name: aname,
                value: data,
            });
        }
        if empty {
            self.need_close = true;
            self.to_close = name.clone();
        }
        Ok(Token::StartElement { name, attr })
    }

    // Go: encoding/xml/xml.go:(*Decoder).attrval
    fn attrval(&mut self) -> Option<Vec<u8>> {
        let b = self.mustgetc()?;
        // Handle quoted attribute values
        if b == b'"' || b == b'\'' {
            return self.text(b as i32, false);
        }
        // Handle unquoted attribute values for strict parsers
        self.err = Some(self.syntax_error(b"unquoted or missing attribute value in element"));
        None
    }

    /// Skip spaces if any
    // Go: encoding/xml/xml.go:(*Decoder).space
    fn space(&mut self) {
        loop {
            let Some(b) = self.getc() else {
                return;
            };
            match b {
                b' ' | b'\r' | b'\n' | b'\t' => {}
                _ => {
                    self.ungetc(b);
                    return;
                }
            }
        }
    }

    /// Read a single byte. If there is no byte to read, return None and leave the error in
    /// d.err. Maintain line number.
    // Go: encoding/xml/xml.go:(*Decoder).getc
    fn getc(&mut self) -> Option<u8> {
        if self.err.is_some() {
            return None;
        }
        let b;
        if self.next_byte >= 0 {
            b = self.next_byte as u8;
            self.next_byte = -1;
        } else {
            if self.rpos >= self.data.len() {
                self.err = Some(Error::Eof);
                return None;
            }
            b = self.data[self.rpos];
            self.rpos += 1;
        }
        if b == b'\n' {
            self.line += 1;
        }
        Some(b)
    }

    /// Must read a single byte. If there is no byte to read, set d.err to
    /// SyntaxError("unexpected EOF") and return None.
    // Go: encoding/xml/xml.go:(*Decoder).mustgetc
    fn mustgetc(&mut self) -> Option<u8> {
        let b = self.getc();
        if b.is_none() && self.err == Some(Error::Eof) {
            self.err = Some(self.syntax_error(b"unexpected EOF"));
        }
        b
    }

    /// Unread a single byte.
    // Go: encoding/xml/xml.go:(*Decoder).ungetc
    fn ungetc(&mut self, b: u8) {
        if b == b'\n' {
            self.line -= 1;
        }
        self.next_byte = b as i32;
    }

    /// Read plain text section (XML calls it character data).
    /// If quote >= 0, we are in a quoted string and need to find the matching quote.
    /// If cdata == true, we are in a <![CDATA[ section and need to find ]]>.
    /// On failure return None and leave the error in d.err.
    // Go: encoding/xml/xml.go:(*Decoder).text
    fn text(&mut self, quote: i32, cdata: bool) -> Option<Vec<u8>> {
        let (mut b0, mut b1) = (0u8, 0u8);
        let mut trunc = 0;
        self.buf.clear();
        'input: loop {
            let Some(mut b) = self.getc() else {
                if cdata {
                    if self.err == Some(Error::Eof) {
                        self.err = Some(self.syntax_error(b"unexpected EOF in CDATA section"));
                    }
                    return None;
                }
                break 'input;
            };

            // <![CDATA[ section ends with ]]>.
            // It is an error for ]]> to appear in ordinary text,
            // but it is allowed in quoted strings.
            if quote < 0 && b0 == b']' && b1 == b']' && b == b'>' {
                if cdata {
                    trunc = 2;
                    break 'input;
                }
                self.err = Some(self.syntax_error(b"unescaped ]]> not in CDATA section"));
                return None;
            }

            // Stop reading text if we see a <.
            if b == b'<' && !cdata {
                if quote >= 0 {
                    self.err = Some(self.syntax_error(b"unescaped < inside quoted string"));
                    return None;
                }
                self.ungetc(b'<');
                break 'input;
            }
            if quote >= 0 && b as i32 == quote {
                break 'input;
            }
            if b == b'&' && !cdata {
                // Read escaped character expression up to semicolon.
                // XML in all its glory allows a document to define and use
                // its own character names with <!ENTITY ...> directives.
                // Parsers are required to recognize lt, gt, amp, apos, and quot
                // even if they have not been declared.
                let before = self.buf.len();
                self.buf.push(b'&');
                let mut text: Vec<u8> = Vec::new();
                let mut have_text = false;
                b = self.mustgetc()?;
                if b == b'#' {
                    self.buf.push(b);
                    b = self.mustgetc()?;
                    let mut base = 10;
                    if b == b'x' {
                        base = 16;
                        self.buf.push(b);
                        b = self.mustgetc()?;
                    }
                    let start = self.buf.len();
                    while b.is_ascii_digit()
                        || base == 16 && (b'a'..=b'f').contains(&b)
                        || base == 16 && (b'A'..=b'F').contains(&b)
                    {
                        self.buf.push(b);
                        b = self.mustgetc()?;
                    }
                    if b != b';' {
                        self.ungetc(b);
                    } else {
                        let s = self.buf[start..].to_vec();
                        self.buf.push(b';');
                        if let Ok(n) = go_strconv::parse_uint(&s, base, 64)
                            && n <= utf8::MAX_RUNE as u64
                        {
                            text = rune_to_string(n as Rune);
                            have_text = true;
                        }
                    }
                } else {
                    self.ungetc(b);
                    if !self.read_name() && self.err.is_some() {
                        return None;
                    }
                    b = self.mustgetc()?;
                    if b != b';' {
                        self.ungetc(b);
                    } else {
                        let name = self.buf[before + 1..].to_vec();
                        self.buf.push(b';');
                        if is_name(&name) {
                            let r = match &name[..] {
                                b"lt" => Some(b'<'),
                                b"gt" => Some(b'>'),
                                b"amp" => Some(b'&'),
                                b"apos" => Some(b'\''),
                                b"quot" => Some(b'"'),
                                _ => None,
                            };
                            if let Some(r) = r {
                                text = vec![r];
                                have_text = true;
                            }
                            // (d.Entity is nil.)
                        }
                    }
                }

                if have_text {
                    self.buf.truncate(before);
                    self.buf.extend_from_slice(&text);
                    b0 = 0;
                    b1 = 0;
                    continue 'input;
                }
                // (d.Strict)
                let mut ent = self.buf[before..].to_vec();
                if ent[ent.len() - 1] != b';' {
                    ent.extend_from_slice(b" (no semicolon)");
                }
                let mut m = b"invalid character entity ".to_vec();
                m.extend_from_slice(&ent);
                self.err = Some(self.syntax_error(&m));
                return None;
            }

            // We must rewrite unescaped \r and \r\n into \n.
            if b == b'\r' {
                self.buf.push(b'\n');
            } else if b1 == b'\r' && b == b'\n' {
                // Skip \r\n--we already wrote \n.
            } else {
                self.buf.push(b);
            }

            b0 = b1;
            b1 = b;
        }
        let data = self.buf[..self.buf.len() - trunc].to_vec();

        // Inspect each rune for being a disallowed character.
        let mut buf = &data[..];
        while !buf.is_empty() {
            let (r, size) = utf8::decode_rune(buf);
            if r == utf8::RUNE_ERROR && size == 1 {
                self.err = Some(self.syntax_error(b"invalid UTF-8"));
                return None;
            }
            buf = &buf[size..];
            if !is_in_character_range(r) {
                let m = format!("illegal character code {}", go_u(r));
                self.err = Some(self.syntax_error(m.as_bytes()));
                return None;
            }
        }

        Some(data)
    }

    /// Get name space name: name with a : stuck in the middle.
    /// The part before the : is the name space identifier.
    // Go: encoding/xml/xml.go:(*Decoder).nsname
    fn nsname(&mut self) -> Option<Name> {
        let s = self.name()?;
        let colons = s.iter().filter(|&&c| c == b':').count();
        if colons > 1 {
            return None;
        }
        let mut name = Name::default();
        match s.iter().position(|&c| c == b':') {
            Some(i) if i > 0 && i + 1 < s.len() => {
                name.space = s[..i].to_vec();
                name.local = s[i + 1..].to_vec();
            }
            _ => name.local = s,
        }
        Some(name)
    }

    /// Get name: /first(first|second)*/
    /// Do not set d.err if the name is missing (unless unexpected EOF is received):
    /// let the caller provide better context.
    // Go: encoding/xml/xml.go:(*Decoder).name
    fn name(&mut self) -> Option<Vec<u8>> {
        self.buf.clear();
        if !self.read_name() {
            return None;
        }

        // Now we check the characters.
        if !is_name(&self.buf) {
            let mut m = b"invalid XML name: ".to_vec();
            m.extend_from_slice(&self.buf);
            self.err = Some(self.syntax_error(&m));
            return None;
        }
        Some(self.buf.clone())
    }

    /// Read a name and append its bytes to d.buf.
    /// The name is delimited by any single-byte character not valid in names.
    /// All multi-byte characters are accepted; the caller must check their validity.
    // Go: encoding/xml/xml.go:(*Decoder).readName
    fn read_name(&mut self) -> bool {
        let Some(b) = self.mustgetc() else {
            return false;
        };
        if b < utf8::RUNE_SELF as u8 && !is_name_byte(b) {
            self.ungetc(b);
            return false;
        }
        self.buf.push(b);

        loop {
            let Some(b) = self.mustgetc() else {
                return false;
            };
            if b < utf8::RUNE_SELF as u8 && !is_name_byte(b) {
                self.ungetc(b);
                break;
            }
            self.buf.push(b);
        }
        true
    }
}

/// Go's `%U` of a rune.
fn go_u(r: Rune) -> String {
    if r < 0 {
        return format!("U+{:04X}", r as u32);
    }
    format!("U+{r:04X}")
}

fn rune_to_string(r: Rune) -> Vec<u8> {
    utf8::rune_to_string(r)
}

/// Decide whether the given rune is in the XML Character Range, per
/// the Char production of https://www.xml.com/axml/testaxml.htm,
/// Section 2.2 Characters.
// Go: encoding/xml/xml.go:isInCharacterRange
pub fn is_in_character_range(r: Rune) -> bool {
    r == 0x09
        || r == 0x0A
        || r == 0x0D
        || (0x20..=0xD7FF).contains(&r)
        || (0xE000..=0xFFFD).contains(&r)
        || (0x10000..=0x10FFFF).contains(&r)
}

// Go: encoding/xml/xml.go:isNameByte
fn is_name_byte(c: u8) -> bool {
    c.is_ascii_uppercase()
        || c.is_ascii_lowercase()
        || c.is_ascii_digit()
        || c == b'_'
        || c == b':'
        || c == b'.'
        || c == b'-'
}

// Go: encoding/xml/xml.go:isName
fn is_name(s: &[u8]) -> bool {
    if s.is_empty() {
        return false;
    }
    let (c, mut n) = utf8::decode_rune(s);
    if c == utf8::RUNE_ERROR && n == 1 {
        return false;
    }
    if !go_unicode::is(&FIRST, c) {
        return false;
    }
    let mut s = s;
    while n < s.len() {
        s = &s[n..];
        let (c, n2) = utf8::decode_rune(s);
        n = n2;
        if c == utf8::RUNE_ERROR && n == 1 {
            return false;
        }
        if !go_unicode::is(&FIRST, c) && !go_unicode::is(&SECOND, c) {
            return false;
        }
    }
    true
}

/// procInst parses the `param="..."` or `param='...'`
/// value out of the provided string, returning "" if not found.
// Go: encoding/xml/xml.go:procInst
fn proc_inst<'s>(param: &[u8], s: &'s [u8]) -> &'s [u8] {
    // TODO: this parsing is somewhat lame and not exact.
    // It works for all actual cases, though.
    let mut param = param.to_vec();
    param.push(b'=');
    let lenp = param.len();
    let mut i = 0;
    let mut sep = 0u8;
    while i < s.len() {
        let sub = &s[i..];
        let k = match find(sub, &param) {
            Some(k) if lenp + k < sub.len() => k,
            _ => return b"",
        };
        i += lenp + k + 1;
        let c = sub[lenp + k];
        if c == b'\'' || c == b'"' {
            sep = c;
            break;
        }
    }
    if sep == 0 {
        return b"";
    }
    match s[i..].iter().position(|&c| c == sep) {
        Some(j) => &s[i..i + j],
        None => b"",
    }
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

const ESC_QUOT: &[u8] = b"&#34;"; // shorter than "&quot;"
const ESC_APOS: &[u8] = b"&#39;"; // shorter than "&apos;"
const ESC_AMP: &[u8] = b"&amp;";
const ESC_LT: &[u8] = b"&lt;";
const ESC_GT: &[u8] = b"&gt;";
const ESC_TAB: &[u8] = b"&#x9;";
const ESC_NL: &[u8] = b"&#xA;";
const ESC_CR: &[u8] = b"&#xD;";
const ESC_FFFD: &[u8] = "\u{FFFD}".as_bytes(); // Unicode replacement character

/// EscapeText/EscapeString: the properly escaped XML equivalent of the plain text data s
/// (`escapeNewline` true).
// Go: encoding/xml/xml.go:escapeText
pub fn escape_text(w: &mut Vec<u8>, s: &[u8]) {
    let mut last = 0;
    let mut i = 0;
    while i < s.len() {
        let (r, width) = utf8::decode_rune(&s[i..]);
        i += width;
        let esc = match r {
            0x22 => ESC_QUOT,
            0x27 => ESC_APOS,
            0x26 => ESC_AMP,
            0x3C => ESC_LT,
            0x3E => ESC_GT,
            0x09 => ESC_TAB,
            0x0A => ESC_NL,
            0x0D => ESC_CR,
            _ => {
                if !is_in_character_range(r) || (r == 0xFFFD && width == 1) {
                    ESC_FFFD
                } else {
                    continue;
                }
            }
        };
        w.extend_from_slice(&s[last..i - width]);
        w.extend_from_slice(esc);
        last = i;
    }
    w.extend_from_slice(&s[last..]);
}

/// Go: encoding/xml/xml.go:first
pub(crate) static FIRST: RangeTable = RangeTable {
    r16: &[
        Range16 {
            lo: 0x003A,
            hi: 0x003A,
            stride: 1,
        },
        Range16 {
            lo: 0x0041,
            hi: 0x005A,
            stride: 1,
        },
        Range16 {
            lo: 0x005F,
            hi: 0x005F,
            stride: 1,
        },
        Range16 {
            lo: 0x0061,
            hi: 0x007A,
            stride: 1,
        },
        Range16 {
            lo: 0x00C0,
            hi: 0x00D6,
            stride: 1,
        },
        Range16 {
            lo: 0x00D8,
            hi: 0x00F6,
            stride: 1,
        },
        Range16 {
            lo: 0x00F8,
            hi: 0x00FF,
            stride: 1,
        },
        Range16 {
            lo: 0x0100,
            hi: 0x0131,
            stride: 1,
        },
        Range16 {
            lo: 0x0134,
            hi: 0x013E,
            stride: 1,
        },
        Range16 {
            lo: 0x0141,
            hi: 0x0148,
            stride: 1,
        },
        Range16 {
            lo: 0x014A,
            hi: 0x017E,
            stride: 1,
        },
        Range16 {
            lo: 0x0180,
            hi: 0x01C3,
            stride: 1,
        },
        Range16 {
            lo: 0x01CD,
            hi: 0x01F0,
            stride: 1,
        },
        Range16 {
            lo: 0x01F4,
            hi: 0x01F5,
            stride: 1,
        },
        Range16 {
            lo: 0x01FA,
            hi: 0x0217,
            stride: 1,
        },
        Range16 {
            lo: 0x0250,
            hi: 0x02A8,
            stride: 1,
        },
        Range16 {
            lo: 0x02BB,
            hi: 0x02C1,
            stride: 1,
        },
        Range16 {
            lo: 0x0386,
            hi: 0x0386,
            stride: 1,
        },
        Range16 {
            lo: 0x0388,
            hi: 0x038A,
            stride: 1,
        },
        Range16 {
            lo: 0x038C,
            hi: 0x038C,
            stride: 1,
        },
        Range16 {
            lo: 0x038E,
            hi: 0x03A1,
            stride: 1,
        },
        Range16 {
            lo: 0x03A3,
            hi: 0x03CE,
            stride: 1,
        },
        Range16 {
            lo: 0x03D0,
            hi: 0x03D6,
            stride: 1,
        },
        Range16 {
            lo: 0x03DA,
            hi: 0x03E0,
            stride: 2,
        },
        Range16 {
            lo: 0x03E2,
            hi: 0x03F3,
            stride: 1,
        },
        Range16 {
            lo: 0x0401,
            hi: 0x040C,
            stride: 1,
        },
        Range16 {
            lo: 0x040E,
            hi: 0x044F,
            stride: 1,
        },
        Range16 {
            lo: 0x0451,
            hi: 0x045C,
            stride: 1,
        },
        Range16 {
            lo: 0x045E,
            hi: 0x0481,
            stride: 1,
        },
        Range16 {
            lo: 0x0490,
            hi: 0x04C4,
            stride: 1,
        },
        Range16 {
            lo: 0x04C7,
            hi: 0x04C8,
            stride: 1,
        },
        Range16 {
            lo: 0x04CB,
            hi: 0x04CC,
            stride: 1,
        },
        Range16 {
            lo: 0x04D0,
            hi: 0x04EB,
            stride: 1,
        },
        Range16 {
            lo: 0x04EE,
            hi: 0x04F5,
            stride: 1,
        },
        Range16 {
            lo: 0x04F8,
            hi: 0x04F9,
            stride: 1,
        },
        Range16 {
            lo: 0x0531,
            hi: 0x0556,
            stride: 1,
        },
        Range16 {
            lo: 0x0559,
            hi: 0x0559,
            stride: 1,
        },
        Range16 {
            lo: 0x0561,
            hi: 0x0586,
            stride: 1,
        },
        Range16 {
            lo: 0x05D0,
            hi: 0x05EA,
            stride: 1,
        },
        Range16 {
            lo: 0x05F0,
            hi: 0x05F2,
            stride: 1,
        },
        Range16 {
            lo: 0x0621,
            hi: 0x063A,
            stride: 1,
        },
        Range16 {
            lo: 0x0641,
            hi: 0x064A,
            stride: 1,
        },
        Range16 {
            lo: 0x0671,
            hi: 0x06B7,
            stride: 1,
        },
        Range16 {
            lo: 0x06BA,
            hi: 0x06BE,
            stride: 1,
        },
        Range16 {
            lo: 0x06C0,
            hi: 0x06CE,
            stride: 1,
        },
        Range16 {
            lo: 0x06D0,
            hi: 0x06D3,
            stride: 1,
        },
        Range16 {
            lo: 0x06D5,
            hi: 0x06D5,
            stride: 1,
        },
        Range16 {
            lo: 0x06E5,
            hi: 0x06E6,
            stride: 1,
        },
        Range16 {
            lo: 0x0905,
            hi: 0x0939,
            stride: 1,
        },
        Range16 {
            lo: 0x093D,
            hi: 0x093D,
            stride: 1,
        },
        Range16 {
            lo: 0x0958,
            hi: 0x0961,
            stride: 1,
        },
        Range16 {
            lo: 0x0985,
            hi: 0x098C,
            stride: 1,
        },
        Range16 {
            lo: 0x098F,
            hi: 0x0990,
            stride: 1,
        },
        Range16 {
            lo: 0x0993,
            hi: 0x09A8,
            stride: 1,
        },
        Range16 {
            lo: 0x09AA,
            hi: 0x09B0,
            stride: 1,
        },
        Range16 {
            lo: 0x09B2,
            hi: 0x09B2,
            stride: 1,
        },
        Range16 {
            lo: 0x09B6,
            hi: 0x09B9,
            stride: 1,
        },
        Range16 {
            lo: 0x09DC,
            hi: 0x09DD,
            stride: 1,
        },
        Range16 {
            lo: 0x09DF,
            hi: 0x09E1,
            stride: 1,
        },
        Range16 {
            lo: 0x09F0,
            hi: 0x09F1,
            stride: 1,
        },
        Range16 {
            lo: 0x0A05,
            hi: 0x0A0A,
            stride: 1,
        },
        Range16 {
            lo: 0x0A0F,
            hi: 0x0A10,
            stride: 1,
        },
        Range16 {
            lo: 0x0A13,
            hi: 0x0A28,
            stride: 1,
        },
        Range16 {
            lo: 0x0A2A,
            hi: 0x0A30,
            stride: 1,
        },
        Range16 {
            lo: 0x0A32,
            hi: 0x0A33,
            stride: 1,
        },
        Range16 {
            lo: 0x0A35,
            hi: 0x0A36,
            stride: 1,
        },
        Range16 {
            lo: 0x0A38,
            hi: 0x0A39,
            stride: 1,
        },
        Range16 {
            lo: 0x0A59,
            hi: 0x0A5C,
            stride: 1,
        },
        Range16 {
            lo: 0x0A5E,
            hi: 0x0A5E,
            stride: 1,
        },
        Range16 {
            lo: 0x0A72,
            hi: 0x0A74,
            stride: 1,
        },
        Range16 {
            lo: 0x0A85,
            hi: 0x0A8B,
            stride: 1,
        },
        Range16 {
            lo: 0x0A8D,
            hi: 0x0A8D,
            stride: 1,
        },
        Range16 {
            lo: 0x0A8F,
            hi: 0x0A91,
            stride: 1,
        },
        Range16 {
            lo: 0x0A93,
            hi: 0x0AA8,
            stride: 1,
        },
        Range16 {
            lo: 0x0AAA,
            hi: 0x0AB0,
            stride: 1,
        },
        Range16 {
            lo: 0x0AB2,
            hi: 0x0AB3,
            stride: 1,
        },
        Range16 {
            lo: 0x0AB5,
            hi: 0x0AB9,
            stride: 1,
        },
        Range16 {
            lo: 0x0B05,
            hi: 0x0B0C,
            stride: 1,
        },
        Range16 {
            lo: 0x0B0F,
            hi: 0x0B10,
            stride: 1,
        },
        Range16 {
            lo: 0x0B13,
            hi: 0x0B28,
            stride: 1,
        },
        Range16 {
            lo: 0x0B2A,
            hi: 0x0B30,
            stride: 1,
        },
        Range16 {
            lo: 0x0B32,
            hi: 0x0B33,
            stride: 1,
        },
        Range16 {
            lo: 0x0B36,
            hi: 0x0B39,
            stride: 1,
        },
        Range16 {
            lo: 0x0B3D,
            hi: 0x0B3D,
            stride: 1,
        },
        Range16 {
            lo: 0x0B5C,
            hi: 0x0B5D,
            stride: 1,
        },
        Range16 {
            lo: 0x0B5F,
            hi: 0x0B61,
            stride: 1,
        },
        Range16 {
            lo: 0x0B85,
            hi: 0x0B8A,
            stride: 1,
        },
        Range16 {
            lo: 0x0B8E,
            hi: 0x0B90,
            stride: 1,
        },
        Range16 {
            lo: 0x0B92,
            hi: 0x0B95,
            stride: 1,
        },
        Range16 {
            lo: 0x0B99,
            hi: 0x0B9A,
            stride: 1,
        },
        Range16 {
            lo: 0x0B9C,
            hi: 0x0B9C,
            stride: 1,
        },
        Range16 {
            lo: 0x0B9E,
            hi: 0x0B9F,
            stride: 1,
        },
        Range16 {
            lo: 0x0BA3,
            hi: 0x0BA4,
            stride: 1,
        },
        Range16 {
            lo: 0x0BA8,
            hi: 0x0BAA,
            stride: 1,
        },
        Range16 {
            lo: 0x0BAE,
            hi: 0x0BB5,
            stride: 1,
        },
        Range16 {
            lo: 0x0BB7,
            hi: 0x0BB9,
            stride: 1,
        },
        Range16 {
            lo: 0x0C05,
            hi: 0x0C0C,
            stride: 1,
        },
        Range16 {
            lo: 0x0C0E,
            hi: 0x0C10,
            stride: 1,
        },
        Range16 {
            lo: 0x0C12,
            hi: 0x0C28,
            stride: 1,
        },
        Range16 {
            lo: 0x0C2A,
            hi: 0x0C33,
            stride: 1,
        },
        Range16 {
            lo: 0x0C35,
            hi: 0x0C39,
            stride: 1,
        },
        Range16 {
            lo: 0x0C60,
            hi: 0x0C61,
            stride: 1,
        },
        Range16 {
            lo: 0x0C85,
            hi: 0x0C8C,
            stride: 1,
        },
        Range16 {
            lo: 0x0C8E,
            hi: 0x0C90,
            stride: 1,
        },
        Range16 {
            lo: 0x0C92,
            hi: 0x0CA8,
            stride: 1,
        },
        Range16 {
            lo: 0x0CAA,
            hi: 0x0CB3,
            stride: 1,
        },
        Range16 {
            lo: 0x0CB5,
            hi: 0x0CB9,
            stride: 1,
        },
        Range16 {
            lo: 0x0CDE,
            hi: 0x0CDE,
            stride: 1,
        },
        Range16 {
            lo: 0x0CE0,
            hi: 0x0CE1,
            stride: 1,
        },
        Range16 {
            lo: 0x0D05,
            hi: 0x0D0C,
            stride: 1,
        },
        Range16 {
            lo: 0x0D0E,
            hi: 0x0D10,
            stride: 1,
        },
        Range16 {
            lo: 0x0D12,
            hi: 0x0D28,
            stride: 1,
        },
        Range16 {
            lo: 0x0D2A,
            hi: 0x0D39,
            stride: 1,
        },
        Range16 {
            lo: 0x0D60,
            hi: 0x0D61,
            stride: 1,
        },
        Range16 {
            lo: 0x0E01,
            hi: 0x0E2E,
            stride: 1,
        },
        Range16 {
            lo: 0x0E30,
            hi: 0x0E30,
            stride: 1,
        },
        Range16 {
            lo: 0x0E32,
            hi: 0x0E33,
            stride: 1,
        },
        Range16 {
            lo: 0x0E40,
            hi: 0x0E45,
            stride: 1,
        },
        Range16 {
            lo: 0x0E81,
            hi: 0x0E82,
            stride: 1,
        },
        Range16 {
            lo: 0x0E84,
            hi: 0x0E84,
            stride: 1,
        },
        Range16 {
            lo: 0x0E87,
            hi: 0x0E88,
            stride: 1,
        },
        Range16 {
            lo: 0x0E8A,
            hi: 0x0E8D,
            stride: 3,
        },
        Range16 {
            lo: 0x0E94,
            hi: 0x0E97,
            stride: 1,
        },
        Range16 {
            lo: 0x0E99,
            hi: 0x0E9F,
            stride: 1,
        },
        Range16 {
            lo: 0x0EA1,
            hi: 0x0EA3,
            stride: 1,
        },
        Range16 {
            lo: 0x0EA5,
            hi: 0x0EA7,
            stride: 2,
        },
        Range16 {
            lo: 0x0EAA,
            hi: 0x0EAB,
            stride: 1,
        },
        Range16 {
            lo: 0x0EAD,
            hi: 0x0EAE,
            stride: 1,
        },
        Range16 {
            lo: 0x0EB0,
            hi: 0x0EB0,
            stride: 1,
        },
        Range16 {
            lo: 0x0EB2,
            hi: 0x0EB3,
            stride: 1,
        },
        Range16 {
            lo: 0x0EBD,
            hi: 0x0EBD,
            stride: 1,
        },
        Range16 {
            lo: 0x0EC0,
            hi: 0x0EC4,
            stride: 1,
        },
        Range16 {
            lo: 0x0F40,
            hi: 0x0F47,
            stride: 1,
        },
        Range16 {
            lo: 0x0F49,
            hi: 0x0F69,
            stride: 1,
        },
        Range16 {
            lo: 0x10A0,
            hi: 0x10C5,
            stride: 1,
        },
        Range16 {
            lo: 0x10D0,
            hi: 0x10F6,
            stride: 1,
        },
        Range16 {
            lo: 0x1100,
            hi: 0x1100,
            stride: 1,
        },
        Range16 {
            lo: 0x1102,
            hi: 0x1103,
            stride: 1,
        },
        Range16 {
            lo: 0x1105,
            hi: 0x1107,
            stride: 1,
        },
        Range16 {
            lo: 0x1109,
            hi: 0x1109,
            stride: 1,
        },
        Range16 {
            lo: 0x110B,
            hi: 0x110C,
            stride: 1,
        },
        Range16 {
            lo: 0x110E,
            hi: 0x1112,
            stride: 1,
        },
        Range16 {
            lo: 0x113C,
            hi: 0x1140,
            stride: 2,
        },
        Range16 {
            lo: 0x114C,
            hi: 0x1150,
            stride: 2,
        },
        Range16 {
            lo: 0x1154,
            hi: 0x1155,
            stride: 1,
        },
        Range16 {
            lo: 0x1159,
            hi: 0x1159,
            stride: 1,
        },
        Range16 {
            lo: 0x115F,
            hi: 0x1161,
            stride: 1,
        },
        Range16 {
            lo: 0x1163,
            hi: 0x1169,
            stride: 2,
        },
        Range16 {
            lo: 0x116D,
            hi: 0x116E,
            stride: 1,
        },
        Range16 {
            lo: 0x1172,
            hi: 0x1173,
            stride: 1,
        },
        Range16 {
            lo: 0x11AE,
            hi: 0x11AF,
            stride: 1,
        },
        Range16 {
            lo: 0x11B7,
            hi: 0x11B8,
            stride: 1,
        },
        Range16 {
            lo: 0x11BA,
            hi: 0x11BA,
            stride: 1,
        },
        Range16 {
            lo: 0x11BC,
            hi: 0x11C2,
            stride: 1,
        },
        Range16 {
            lo: 0x11F9,
            hi: 0x11F9,
            stride: 1,
        },
        Range16 {
            lo: 0x1E00,
            hi: 0x1E9B,
            stride: 1,
        },
        Range16 {
            lo: 0x1EA0,
            hi: 0x1EF9,
            stride: 1,
        },
        Range16 {
            lo: 0x1F00,
            hi: 0x1F15,
            stride: 1,
        },
        Range16 {
            lo: 0x1F18,
            hi: 0x1F1D,
            stride: 1,
        },
        Range16 {
            lo: 0x1F20,
            hi: 0x1F45,
            stride: 1,
        },
        Range16 {
            lo: 0x1F48,
            hi: 0x1F4D,
            stride: 1,
        },
        Range16 {
            lo: 0x1F50,
            hi: 0x1F57,
            stride: 1,
        },
        Range16 {
            lo: 0x1F5D,
            hi: 0x1F5D,
            stride: 1,
        },
        Range16 {
            lo: 0x1F5F,
            hi: 0x1F7D,
            stride: 1,
        },
        Range16 {
            lo: 0x1F80,
            hi: 0x1FB4,
            stride: 1,
        },
        Range16 {
            lo: 0x1FB6,
            hi: 0x1FBC,
            stride: 1,
        },
        Range16 {
            lo: 0x1FBE,
            hi: 0x1FBE,
            stride: 1,
        },
        Range16 {
            lo: 0x1FC2,
            hi: 0x1FC4,
            stride: 1,
        },
        Range16 {
            lo: 0x1FC6,
            hi: 0x1FCC,
            stride: 1,
        },
        Range16 {
            lo: 0x1FD0,
            hi: 0x1FD3,
            stride: 1,
        },
        Range16 {
            lo: 0x1FD6,
            hi: 0x1FDB,
            stride: 1,
        },
        Range16 {
            lo: 0x1FE0,
            hi: 0x1FEC,
            stride: 1,
        },
        Range16 {
            lo: 0x1FF2,
            hi: 0x1FF4,
            stride: 1,
        },
        Range16 {
            lo: 0x1FF6,
            hi: 0x1FFC,
            stride: 1,
        },
        Range16 {
            lo: 0x2126,
            hi: 0x2126,
            stride: 1,
        },
        Range16 {
            lo: 0x212A,
            hi: 0x212B,
            stride: 1,
        },
        Range16 {
            lo: 0x212E,
            hi: 0x212E,
            stride: 1,
        },
        Range16 {
            lo: 0x2180,
            hi: 0x2182,
            stride: 1,
        },
        Range16 {
            lo: 0x3007,
            hi: 0x3007,
            stride: 1,
        },
        Range16 {
            lo: 0x3021,
            hi: 0x3029,
            stride: 1,
        },
        Range16 {
            lo: 0x3041,
            hi: 0x3094,
            stride: 1,
        },
        Range16 {
            lo: 0x30A1,
            hi: 0x30FA,
            stride: 1,
        },
        Range16 {
            lo: 0x3105,
            hi: 0x312C,
            stride: 1,
        },
        Range16 {
            lo: 0x4E00,
            hi: 0x9FA5,
            stride: 1,
        },
        Range16 {
            lo: 0xAC00,
            hi: 0xD7A3,
            stride: 1,
        },
    ],
    r32: &[],
    latin_offset: 0,
};

/// Go: encoding/xml/xml.go:second
pub(crate) static SECOND: RangeTable = RangeTable {
    r16: &[
        Range16 {
            lo: 0x002D,
            hi: 0x002E,
            stride: 1,
        },
        Range16 {
            lo: 0x0030,
            hi: 0x0039,
            stride: 1,
        },
        Range16 {
            lo: 0x00B7,
            hi: 0x00B7,
            stride: 1,
        },
        Range16 {
            lo: 0x02D0,
            hi: 0x02D1,
            stride: 1,
        },
        Range16 {
            lo: 0x0300,
            hi: 0x0345,
            stride: 1,
        },
        Range16 {
            lo: 0x0360,
            hi: 0x0361,
            stride: 1,
        },
        Range16 {
            lo: 0x0387,
            hi: 0x0387,
            stride: 1,
        },
        Range16 {
            lo: 0x0483,
            hi: 0x0486,
            stride: 1,
        },
        Range16 {
            lo: 0x0591,
            hi: 0x05A1,
            stride: 1,
        },
        Range16 {
            lo: 0x05A3,
            hi: 0x05B9,
            stride: 1,
        },
        Range16 {
            lo: 0x05BB,
            hi: 0x05BD,
            stride: 1,
        },
        Range16 {
            lo: 0x05BF,
            hi: 0x05BF,
            stride: 1,
        },
        Range16 {
            lo: 0x05C1,
            hi: 0x05C2,
            stride: 1,
        },
        Range16 {
            lo: 0x064B,
            hi: 0x0652,
            stride: 1,
        },
        Range16 {
            lo: 0x0660,
            hi: 0x0669,
            stride: 1,
        },
        Range16 {
            lo: 0x0670,
            hi: 0x0670,
            stride: 1,
        },
        Range16 {
            lo: 0x06D6,
            hi: 0x06DC,
            stride: 1,
        },
        Range16 {
            lo: 0x06DD,
            hi: 0x06DF,
            stride: 1,
        },
        Range16 {
            lo: 0x06E0,
            hi: 0x06E4,
            stride: 1,
        },
        Range16 {
            lo: 0x06E7,
            hi: 0x06E8,
            stride: 1,
        },
        Range16 {
            lo: 0x06EA,
            hi: 0x06ED,
            stride: 1,
        },
        Range16 {
            lo: 0x06F0,
            hi: 0x06F9,
            stride: 1,
        },
        Range16 {
            lo: 0x0901,
            hi: 0x0903,
            stride: 1,
        },
        Range16 {
            lo: 0x093C,
            hi: 0x093C,
            stride: 1,
        },
        Range16 {
            lo: 0x093E,
            hi: 0x094C,
            stride: 1,
        },
        Range16 {
            lo: 0x094D,
            hi: 0x094D,
            stride: 1,
        },
        Range16 {
            lo: 0x0951,
            hi: 0x0954,
            stride: 1,
        },
        Range16 {
            lo: 0x0962,
            hi: 0x0963,
            stride: 1,
        },
        Range16 {
            lo: 0x0966,
            hi: 0x096F,
            stride: 1,
        },
        Range16 {
            lo: 0x0981,
            hi: 0x0983,
            stride: 1,
        },
        Range16 {
            lo: 0x09BC,
            hi: 0x09BC,
            stride: 1,
        },
        Range16 {
            lo: 0x09BE,
            hi: 0x09BF,
            stride: 1,
        },
        Range16 {
            lo: 0x09C0,
            hi: 0x09C4,
            stride: 1,
        },
        Range16 {
            lo: 0x09C7,
            hi: 0x09C8,
            stride: 1,
        },
        Range16 {
            lo: 0x09CB,
            hi: 0x09CD,
            stride: 1,
        },
        Range16 {
            lo: 0x09D7,
            hi: 0x09D7,
            stride: 1,
        },
        Range16 {
            lo: 0x09E2,
            hi: 0x09E3,
            stride: 1,
        },
        Range16 {
            lo: 0x09E6,
            hi: 0x09EF,
            stride: 1,
        },
        Range16 {
            lo: 0x0A3E,
            hi: 0x0A3F,
            stride: 1,
        },
        Range16 {
            lo: 0x0A40,
            hi: 0x0A42,
            stride: 1,
        },
        Range16 {
            lo: 0x0A47,
            hi: 0x0A48,
            stride: 1,
        },
        Range16 {
            lo: 0x0A4B,
            hi: 0x0A4D,
            stride: 1,
        },
        Range16 {
            lo: 0x0A66,
            hi: 0x0A6F,
            stride: 1,
        },
        Range16 {
            lo: 0x0A70,
            hi: 0x0A71,
            stride: 1,
        },
        Range16 {
            lo: 0x0A81,
            hi: 0x0A83,
            stride: 1,
        },
        Range16 {
            lo: 0x0ABC,
            hi: 0x0ABC,
            stride: 1,
        },
        Range16 {
            lo: 0x0ABE,
            hi: 0x0AC5,
            stride: 1,
        },
        Range16 {
            lo: 0x0AC7,
            hi: 0x0AC9,
            stride: 1,
        },
        Range16 {
            lo: 0x0ACB,
            hi: 0x0ACD,
            stride: 1,
        },
        Range16 {
            lo: 0x0AE6,
            hi: 0x0AEF,
            stride: 1,
        },
        Range16 {
            lo: 0x0B01,
            hi: 0x0B03,
            stride: 1,
        },
        Range16 {
            lo: 0x0B3C,
            hi: 0x0B3C,
            stride: 1,
        },
        Range16 {
            lo: 0x0B3E,
            hi: 0x0B43,
            stride: 1,
        },
        Range16 {
            lo: 0x0B47,
            hi: 0x0B48,
            stride: 1,
        },
        Range16 {
            lo: 0x0B4B,
            hi: 0x0B4D,
            stride: 1,
        },
        Range16 {
            lo: 0x0B56,
            hi: 0x0B57,
            stride: 1,
        },
        Range16 {
            lo: 0x0B66,
            hi: 0x0B6F,
            stride: 1,
        },
        Range16 {
            lo: 0x0B82,
            hi: 0x0B83,
            stride: 1,
        },
        Range16 {
            lo: 0x0BBE,
            hi: 0x0BC2,
            stride: 1,
        },
        Range16 {
            lo: 0x0BC6,
            hi: 0x0BC8,
            stride: 1,
        },
        Range16 {
            lo: 0x0BCA,
            hi: 0x0BCD,
            stride: 1,
        },
        Range16 {
            lo: 0x0BD7,
            hi: 0x0BD7,
            stride: 1,
        },
        Range16 {
            lo: 0x0BE7,
            hi: 0x0BEF,
            stride: 1,
        },
        Range16 {
            lo: 0x0C01,
            hi: 0x0C03,
            stride: 1,
        },
        Range16 {
            lo: 0x0C3E,
            hi: 0x0C44,
            stride: 1,
        },
        Range16 {
            lo: 0x0C46,
            hi: 0x0C48,
            stride: 1,
        },
        Range16 {
            lo: 0x0C4A,
            hi: 0x0C4D,
            stride: 1,
        },
        Range16 {
            lo: 0x0C55,
            hi: 0x0C56,
            stride: 1,
        },
        Range16 {
            lo: 0x0C66,
            hi: 0x0C6F,
            stride: 1,
        },
        Range16 {
            lo: 0x0C82,
            hi: 0x0C83,
            stride: 1,
        },
        Range16 {
            lo: 0x0CBE,
            hi: 0x0CC4,
            stride: 1,
        },
        Range16 {
            lo: 0x0CC6,
            hi: 0x0CC8,
            stride: 1,
        },
        Range16 {
            lo: 0x0CCA,
            hi: 0x0CCD,
            stride: 1,
        },
        Range16 {
            lo: 0x0CD5,
            hi: 0x0CD6,
            stride: 1,
        },
        Range16 {
            lo: 0x0CE6,
            hi: 0x0CEF,
            stride: 1,
        },
        Range16 {
            lo: 0x0D02,
            hi: 0x0D03,
            stride: 1,
        },
        Range16 {
            lo: 0x0D3E,
            hi: 0x0D43,
            stride: 1,
        },
        Range16 {
            lo: 0x0D46,
            hi: 0x0D48,
            stride: 1,
        },
        Range16 {
            lo: 0x0D4A,
            hi: 0x0D4D,
            stride: 1,
        },
        Range16 {
            lo: 0x0D57,
            hi: 0x0D57,
            stride: 1,
        },
        Range16 {
            lo: 0x0D66,
            hi: 0x0D6F,
            stride: 1,
        },
        Range16 {
            lo: 0x0E31,
            hi: 0x0E31,
            stride: 1,
        },
        Range16 {
            lo: 0x0E34,
            hi: 0x0E3A,
            stride: 1,
        },
        Range16 {
            lo: 0x0E46,
            hi: 0x0E46,
            stride: 1,
        },
        Range16 {
            lo: 0x0E47,
            hi: 0x0E4E,
            stride: 1,
        },
        Range16 {
            lo: 0x0E50,
            hi: 0x0E59,
            stride: 1,
        },
        Range16 {
            lo: 0x0EB1,
            hi: 0x0EB1,
            stride: 1,
        },
        Range16 {
            lo: 0x0EB4,
            hi: 0x0EB9,
            stride: 1,
        },
        Range16 {
            lo: 0x0EBB,
            hi: 0x0EBC,
            stride: 1,
        },
        Range16 {
            lo: 0x0EC6,
            hi: 0x0EC6,
            stride: 1,
        },
        Range16 {
            lo: 0x0EC8,
            hi: 0x0ECD,
            stride: 1,
        },
        Range16 {
            lo: 0x0ED0,
            hi: 0x0ED9,
            stride: 1,
        },
        Range16 {
            lo: 0x0F18,
            hi: 0x0F19,
            stride: 1,
        },
        Range16 {
            lo: 0x0F20,
            hi: 0x0F29,
            stride: 1,
        },
        Range16 {
            lo: 0x0F35,
            hi: 0x0F39,
            stride: 2,
        },
        Range16 {
            lo: 0x0F3E,
            hi: 0x0F3F,
            stride: 1,
        },
        Range16 {
            lo: 0x0F71,
            hi: 0x0F84,
            stride: 1,
        },
        Range16 {
            lo: 0x0F86,
            hi: 0x0F8B,
            stride: 1,
        },
        Range16 {
            lo: 0x0F90,
            hi: 0x0F95,
            stride: 1,
        },
        Range16 {
            lo: 0x0F97,
            hi: 0x0F97,
            stride: 1,
        },
        Range16 {
            lo: 0x0F99,
            hi: 0x0FAD,
            stride: 1,
        },
        Range16 {
            lo: 0x0FB1,
            hi: 0x0FB7,
            stride: 1,
        },
        Range16 {
            lo: 0x0FB9,
            hi: 0x0FB9,
            stride: 1,
        },
        Range16 {
            lo: 0x20D0,
            hi: 0x20DC,
            stride: 1,
        },
        Range16 {
            lo: 0x302A,
            hi: 0x302F,
            stride: 1,
        },
        Range16 {
            lo: 0x3031,
            hi: 0x3035,
            stride: 1,
        },
        Range16 {
            lo: 0x3099,
            hi: 0x309A,
            stride: 1,
        },
        Range16 {
            lo: 0x309D,
            hi: 0x309E,
            stride: 1,
        },
        Range16 {
            lo: 0x30FC,
            hi: 0x30FE,
            stride: 1,
        },
    ],
    r32: &[],
    latin_offset: 0,
};

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (go1.27.1 encoding/xml/xml.go, the decoder subset mxj uses)
// OK (*SyntaxError).Error, NewDecoder, (*Decoder).Token, translate, pushElement, pushNs,
//    syntaxError, popElement, rawToken, attrval, space, getc, mustgetc, ungetc, text,
//    isInCharacterRange, nsname, name, readName, isNameByte, isName, first, second,
//    procInst, escapeText (Strict mode only; no AutoClose, Entity, CharsetReader)
// ---------------------------------------------------------------------------
