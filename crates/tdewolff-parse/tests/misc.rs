//! Differential tests for escaping helpers, perfect hashes, type names and
//! Go's append growth (tests/fixtures/misc.txt), plus hand-ported upstream
//! API tests of the html/css/xml/json packages.

#![allow(clippy::excessive_precision, clippy::type_complexity)]
mod common;
use common::*;

use std::collections::HashMap;

use tdewolff_parse::gobytes::grow_cap;
use tdewolff_parse::{GoBytes, GoError, Input, css, html, json, xml};

fn eq(what: &str, key: &str, want: &str, got: String) -> Result<(), String> {
    if want == got {
        Ok(())
    } else {
        Err(format!("{} {}: want {} got {}", what, key, want, got))
    }
}

#[test]
fn misc_fixtures() {
    check_misc(&records("misc.txt"));
}

/// Randomized records from the oracle's `fnfuzz` mode (checked-in small set,
/// or `TDEWOLFF_PARSE_FNFUZZ=<dir>` for a large one).
#[test]
fn misc_fnfuzz() {
    check_misc(&fnfuzz_records("misc.txt"));
}

fn check_misc(recs: &[Vec<String>]) {
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut hbuf = GoBytes::nil();
    let mut xbuf = GoBytes::nil();
    check_all(recs, |r| {
        *counts.entry(r[0].clone()).or_default() += 1;
        let op = r[0].as_str();
        match op {
            "hescape" => {
                let s = unhex(&r[1]);
                let q: u8 = r[2].parse().unwrap();
                let must = &r[3] == "1";
                let out = html::escape_attr_val(&mut hbuf, &GoBytes::from_slice(&s), q, must);
                eq(
                    op,
                    &format!("{:?} {} {}", String::from_utf8_lossy(&s), q, must),
                    &r[4],
                    hx(&out),
                )
            }
            "xescapeattr" => {
                let s = unhex(&r[1]);
                let out = xml::escape_attr_val(&mut xbuf, &GoBytes::from_slice(&s));
                eq(
                    op,
                    &format!("{:?}", String::from_utf8_lossy(&s)),
                    &r[2],
                    hx(&out),
                )
            }
            "xescapecdata" => {
                let s = unhex(&r[1]);
                let (out, ok) = xml::escape_cdata_val(&mut xbuf, &GoBytes::from_slice(&s));
                eq(
                    op,
                    &format!("{:?}", String::from_utf8_lossy(&s)),
                    &format!("{} {}", r[2], r[3]),
                    format!("{} {}", hx(&out), ok as u8),
                )
            }
            "hhash" => {
                let s = unhex(&r[1]);
                eq(op, &r[1], &r[2], html::to_hash(&s[..]).0.to_string())
            }
            "chash" => {
                let s = unhex(&r[1]);
                eq(op, &r[1], &r[2], css::to_hash(&s[..]).0.to_string())
            }
            "isident" => {
                let s = unhex(&r[1]);
                eq(
                    op,
                    &r[1],
                    &r[2],
                    (css::is_ident(&GoBytes::from_slice(&s)) as u8).to_string(),
                )
            }
            "isurlunquoted" => {
                let s = unhex(&r[1]);
                eq(
                    op,
                    &r[1],
                    &r[2],
                    (css::is_url_unquoted(&GoBytes::from_slice(&s)) as u8).to_string(),
                )
            }
            "hashstring" => {
                let v: u32 = r[2].parse().unwrap();
                let got = if r[1] == "html" {
                    html::Hash(v).string()
                } else {
                    css::Hash(v).string()
                };
                eq(op, &r[2], &r[3], hex(got))
            }
            "typestring" => {
                let n: u32 = r[2].parse().unwrap();
                let got = match r[1].as_str() {
                    "html" => html::token_type_string(n),
                    "css" => css::token_type_string(n),
                    "cssgrammar" => css::grammar_type_string(n),
                    "xml" => xml::token_type_string(n),
                    "json" => json::grammar_type_string(n),
                    "jsonstate" => json::state_string(n),
                    _ => panic!(),
                };
                eq(op, &format!("{} {}", r[1], n), &r[3], hex(got.as_bytes()))
            }
            "csstokenstring" => {
                let t = css::Token::new(css::IdentToken, GoBytes::from_slice(b"data"));
                eq(op, "", &r[1], hex(&t.string()))
            }
            "growcap" => {
                let old: usize = r[1].parse().unwrap();
                let n: usize = r[2].parse().unwrap();
                let s = GoBytes::make(old, old).append(&vec![0u8; n]);
                let got = if old == 0 && n == 0 { 0 } else { s.cap() };
                let _ = grow_cap;
                eq(op, &format!("{} {}", old, n), &r[3], got.to_string())
            }
            "readall" => {
                let n: usize = r[1].parse().unwrap();
                let chunk: usize = r[2].parse().unwrap();
                let eof_with_data = &r[3] == "1";
                let mut rd = ChunkReader {
                    data: vec![0u8; n],
                    pos: 0,
                    chunk,
                    eof_with_data,
                };
                let (b, err) = tdewolff_parse::read_all(&mut rd);
                eq(
                    op,
                    &r[1..4].join(" "),
                    &r[4..7].join(" "),
                    format!("{} {} {}", b.len(), b.cap(), err_str(&err)),
                )
            }
            _ => Err(format!("unknown op {}", op)),
        }
    });
    let mut c: Vec<_> = counts.into_iter().collect();
    c.sort();
    eprintln!("misc ops: {:?}", c);
}

/// An `io.Reader` without `Bytes()` returning at most `chunk` bytes per call
/// (mirrors `chunkReader` in the oracle).
struct ChunkReader {
    data: Vec<u8>,
    pos: usize,
    chunk: usize,
    eof_with_data: bool,
}

impl tdewolff_parse::GoReader for ChunkReader {
    fn read(&mut self, p: &mut [u8]) -> (usize, Option<GoError>) {
        if self.pos >= self.data.len() {
            return (0, Some(GoError::Eof));
        }
        let n = p.len().min(self.chunk).min(self.data.len() - self.pos);
        p[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
        self.pos += n;
        if self.eof_with_data && self.pos >= self.data.len() {
            return (n, Some(GoError::Eof));
        }
        (n, None)
    }
}

// Hand-ported upstream API tests.

#[test]
fn html_text_and_attr_val() {
    let mut l = html::Lexer::new(Input::new_string(
        br#"<div attr="val" >text<!--comment--><!DOCTYPE doctype><![CDATA[cdata]]><script>js</script><svg>image</svg>"#,
    ));
    let exp: &[(&[u8], Option<&[u8]>, Option<&[u8]>)] = &[
        (b"<div", Some(b"div"), None),
        (br#" attr="val""#, Some(b"attr"), Some(br#""val""#)),
        (b">", None, None),
        (b"text", Some(b"text"), None),
        (b"<!--comment-->", Some(b"comment"), None),
        (b"<!DOCTYPE doctype>", Some(b" doctype"), None),
        (b"<![CDATA[cdata]]>", Some(b"cdata"), None),
        (b"<script", Some(b"script"), None),
        (b">", None, None),
        (b"js", Some(b"js"), None),
        (b"</script>", Some(b"script"), None),
        (b"<svg>image</svg>", Some(b"svg"), None),
    ];
    for (data, text, attr) in exp {
        let (_, d) = l.next();
        assert_eq!(d.to_vec(), data.to_vec());
        match text {
            None => assert!(l.text().is_nil()),
            Some(t) => assert_eq!(l.text().to_vec(), t.to_vec()),
        }
        match attr {
            None => assert!(l.attr_val().is_nil()),
            Some(a) => assert_eq!(l.attr_val().to_vec(), a.to_vec()),
        }
    }
}

#[test]
fn html_offset() {
    let z = Input::new_string(br#"<div attr="val">text</div>"#);
    let mut l = html::Lexer::new(z.clone());
    assert_eq!(z.offset(), 0);
    for want in [4, 15, 16, 20, 26] {
        l.next();
        assert_eq!(z.offset(), want);
    }
}

#[test]
fn html_errors() {
    for (s, col) in [
        (&b"<svg>\x00</svg>"[..], 6isize),
        (&b"<svg></svg\x00>"[..], 11),
    ] {
        let mut l = html::Lexer::new(Input::new_string(s));
        loop {
            let (tt, _) = l.next();
            if tt == html::ErrorToken {
                match l.err() {
                    Some(GoError::Parse(e)) => assert_eq!(e.position().1, col),
                    e => panic!("bad error {:?}", e),
                }
                break;
            }
        }
    }
}

#[test]
fn html_lowercases_in_place() {
    // the lexer lowercases tag names, attribute names and end tags in the
    // shared input buffer (the minifier relies on seeing those bytes)
    let b = GoBytes::nil().append(b"<DIV CLASS=X></DIV>");
    let z = Input::new_bytes(b.clone());
    let mut l = html::Lexer::new(z.clone());
    while l.next().0 != html::ErrorToken {}
    z.restore();
    assert_eq!(b.to_vec(), b"<div class=X></div>".to_vec());
}

#[test]
fn xml_attr_whitespace_in_place() {
    let b = GoBytes::nil().append(b"<a b=\"x\ty\nz\"/>");
    let z = Input::new_bytes(b.clone());
    let mut l = xml::Lexer::new(z.clone());
    while l.next().0 != xml::ErrorToken {}
    z.restore();
    assert_eq!(b.to_vec(), b"<a b=\"x y z\"/>".to_vec());
}

#[test]
fn css_offset_and_parse_offset() {
    let z = Input::new_string(b"div{background:url(link);}");
    let mut l = css::Lexer::new(z.clone());
    assert_eq!(z.offset(), 0);
    for want in [3, 4, 14, 15, 24, 25, 26] {
        l.next();
        assert_eq!(z.offset(), want);
    }
    assert_eq!(
        css::Lexer::new(Input::new_string(b"x")).consume_bracket(),
        css::ErrorToken
    );

    let z = Input::new_string(b"div{background:url(link);}");
    let mut p = css::Parser::new(z.clone(), false);
    assert_eq!(z.offset(), 0);
    for want in [4, 25, 26] {
        p.next();
        assert_eq!(z.offset(), want);
    }
}

#[test]
fn css_parse_errors() {
    let tests: &[(bool, &[u8], isize)] = &[
        (false, b"}", 2),
        (true, b"}", 1),
        (false, b"selector", 9),
        (true, b"color 0", 7),
        (true, b"--color 0", 9),
        (true, b"--custom-variable:0", 0),
    ];
    for &(inline, s, col) in tests {
        let mut p = css::Parser::new(Input::new_string(s), inline);
        loop {
            let (gt, _, _) = p.next();
            if gt == css::ErrorGrammar {
                if col == 0 {
                    assert_eq!(p.err(), Some(GoError::Eof));
                } else {
                    assert!(p.has_parse_error());
                    match p.err() {
                        Some(GoError::Parse(e)) => assert_eq!(e.position().1, col),
                        e => panic!("bad error {:?}", e),
                    }
                }
                break;
            }
        }
    }
}

#[test]
fn json_states_and_offset() {
    let tests: &[(&[u8], &[json::State])] = &[
        (b"null", &[json::ValueState]),
        (
            b"[null]",
            &[json::ArrayState, json::ArrayState, json::ValueState],
        ),
        (
            b"{\"\":null}",
            &[
                json::ObjectKeyState,
                json::ObjectValueState,
                json::ObjectKeyState,
                json::ValueState,
            ],
        ),
    ];
    for &(s, states) in tests {
        let mut p = json::Parser::new(Input::new_string(s));
        for &st in states {
            let (gt, _) = p.next();
            if gt == json::ErrorGrammar {
                break;
            }
            assert_eq!(p.state(), st);
        }
    }
}

#[test]
fn buffer_reader_writer() {
    use tdewolff_parse::buffer::{Reader, Writer};
    let s = b"Lorem ipsum";
    let mut r = Reader::new(GoBytes::from_slice(s));
    assert_eq!(r.len(), s.len());
    let mut b = [0u8; 3];
    assert_eq!(r.read(&mut b), (3, None));
    assert_eq!(&b, b"Lor");
    let mut rest = [0u8; 20];
    assert_eq!(r.read(&mut rest), (8, None));
    assert_eq!(r.read(&mut rest), (0, Some(GoError::Eof)));
    assert_eq!(r.read(&mut []), (0, None));
    r.reset();
    assert_eq!(r.bytes().to_vec(), s.to_vec());

    let mut w = Writer::new(GoBytes::make(0, 3));
    assert_eq!(w.write(b"abc"), (3, None));
    assert_eq!(w.write(b"defg"), (4, None));
    assert_eq!(w.bytes().cap(), 2 * 3 + 4);
    assert_eq!(w.bytes().to_vec(), b"abcdefg".to_vec());
    let before = w.bytes();
    w.reset();
    assert_eq!(w.len(), 0);
    w.write(b"XY");
    // Go: bytes returned before Reset alias the reused buffer
    assert_eq!(before.to_vec(), b"XYcdefg".to_vec());
    let mut sw = Writer::new_static(GoBytes::make(0, 2));
    assert_eq!(sw.write(b"abc"), (0, Some(GoError::Eof)));
    assert_eq!(sw.close(), Some(GoError::Eof));
}

#[test]
fn buffer_lexer() {
    use tdewolff_parse::buffer::Lexer;
    let s = b"Lorem ipsum dolor sit amet, consectetur adipiscing elit.";
    let mut z = Lexer::new_bytes(GoBytes::from_slice(s));
    assert_eq!(z.bytes().to_vec(), s.to_vec());
    assert_eq!(z.err(), None);
    assert_eq!(z.peek(0), b'L');
    z.move_(1);
    assert_eq!(z.peek(0), b'o');
    z.rewind(6);
    assert_eq!(z.peek(0), b'i');
    assert_eq!(z.offset(), 6);
    assert_eq!(z.lexeme().to_vec(), b"Lorem ".to_vec());
    assert_eq!(z.shift().to_vec(), b"Lorem ".to_vec());
    assert_eq!(z.pos(), 0);
    z.move_((s.len() - 7) as isize);
    assert_eq!(z.err(), None);
    z.skip();
    z.move_(1);
    assert_eq!(z.err(), Some(GoError::Eof));
    z.move_(-1);
    assert_eq!(z.err(), None);
    z.reset();
    assert_eq!(z.peek(0), b'L');

    let z = Lexer::new_bytes(GoBytes::from_slice("aæ†\u{100000}".as_bytes()));
    assert_eq!(z.peek_rune(0), ('a' as i32, 1));
    assert_eq!(z.peek_rune(1), ('æ' as i32, 2));
    assert_eq!(z.peek_rune(3), ('†' as i32, 3));
    assert_eq!(z.peek_rune(6), (0x100000, 4));
    let z = Lexer::new_bytes(GoBytes::from_slice(b"\xF0"));
    assert_eq!(z.peek_rune(0), (0xF0, 1));

    let b = GoBytes::from_slice(b"abcd");
    let mut z = Lexer::new_bytes(b.slice_to(2));
    assert_eq!(b.to_vec(), b"ab\0d".to_vec());
    z.restore();
    assert_eq!(b.to_vec(), b"abcd".to_vec());
}

struct PlainReader {
    data: Vec<u8>,
    pos: usize,
}

impl tdewolff_parse::GoReader for PlainReader {
    fn read(&mut self, p: &mut [u8]) -> (usize, Option<GoError>) {
        if self.pos >= self.data.len() {
            return (0, Some(GoError::Eof));
        }
        let n = p.len().min(self.data.len() - self.pos);
        p[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
        self.pos += n;
        (n, None)
    }
}

fn plain(s: &[u8]) -> PlainReader {
    PlainReader {
        data: s.to_vec(),
        pos: 0,
    }
}

#[test]
fn buffer_stream_lexer() {
    use tdewolff_parse::buffer::{Reader, StreamLexer};
    let s = b"Lorem ipsum dolor sit amet, consectetur adipiscing elit.";
    let mut r = Reader::new(GoBytes::from_slice(s));
    let mut z = StreamLexer::new(&mut r);
    assert_eq!(z.err(), None);
    assert_eq!(z.pos(), 0);
    assert_eq!(z.peek(0), b'L');
    assert_eq!(z.peek(1), b'o');
    z.move_(1);
    assert_eq!(z.peek(0), b'o');
    z.rewind(6);
    assert_eq!(z.peek(0), b'i');
    assert_eq!(z.lexeme().to_vec(), b"Lorem ".to_vec());
    assert_eq!(z.shift().to_vec(), b"Lorem ".to_vec());
    assert_eq!(z.shift_len(), 6);
    assert_eq!(z.pos(), 0);
    z.move_((s.len() - 7) as isize);
    assert_eq!(z.err(), None);
    z.skip();
    z.move_(1);
    assert_eq!(z.err(), Some(GoError::Eof));
    z.move_(-1);
    assert_eq!(z.err(), None);
    z.free(0);

    let mut pr = plain(s);
    let mut z = StreamLexer::new_size(&mut pr, 5);
    z.move_(6);
    assert_eq!(z.shift().to_vec(), b"Lorem ".to_vec());
    assert_eq!(z.shift_len(), 6);

    let s = b"abcdefghijklm";
    let mut pr = plain(s);
    assert_eq!(StreamLexer::new_size(&mut pr, 4).peek(8), b'i');
    let mut pr = plain(s);
    assert_eq!(StreamLexer::new_size(&mut pr, 4).peek(12), b'm');
    let mut pr = plain(s);
    assert_eq!(StreamLexer::new_size(&mut pr, 0).peek(4), b'e');
    let mut pr = plain(s);
    assert_eq!(StreamLexer::new_size(&mut pr, 13).peek(13), 0);

    let mut r = Reader::new(GoBytes::from_slice("aæ†\u{100000}".as_bytes()));
    let mut z = StreamLexer::new(&mut r);
    assert_eq!(z.peek_rune(0), ('a' as i32, 1));
    assert_eq!(z.peek_rune(1), ('æ' as i32, 2));
    assert_eq!(z.peek_rune(3), ('†' as i32, 3));
    assert_eq!(z.peek_rune(6), (0x100000, 4));

    let mut r = Reader::new(GoBytes::from_slice(b"\xF0"));
    let mut z = StreamLexer::new(&mut r);
    assert_eq!(z.peek_rune(0), (0, 4));

    let mut pr = plain(b"");
    let mut z = StreamLexer::new(&mut pr);
    assert_eq!(z.peek(0), 0);
    assert_eq!(z.err(), Some(GoError::Eof));
    assert_eq!(z.peek(0), 0);

    // many small reads through a growing, recycled buffer
    let long: Vec<u8> = (0..20000u32).map(|i| b'a' + (i % 26) as u8).collect();
    let mut pr = plain(&long);
    let mut z = StreamLexer::new_size(&mut pr, 16);
    let mut out = Vec::new();
    loop {
        let c = z.peek(0);
        if c == 0 {
            break;
        }
        z.move_(1);
        if z.pos() == 7 {
            z.shift().write_to(&mut out);
            let n = z.shift_len();
            z.free(n);
        }
    }
    z.shift().write_to(&mut out);
    assert_eq!(out, long);
}
