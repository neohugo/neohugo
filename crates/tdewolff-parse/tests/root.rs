//! Differential tests for the root package (common.go, util.go, position.go,
//! error.go) against tests/fixtures/root.txt.

mod common;
use common::*;

use std::collections::HashMap;

use tdewolff_parse as parse;
use tdewolff_parse::{EntityMap, GoBytes, NilMap, RevEntityMap};

struct Ents<'a>(&'a HashMap<Vec<u8>, Vec<u8>>);
impl EntityMap for Ents<'_> {
    fn lookup_entity(&self, name: &[u8]) -> Option<&[u8]> {
        self.0.get(name).map(|v| v.as_slice())
    }
}
struct Rev<'a>(&'a HashMap<u8, Vec<u8>>);
impl RevEntityMap for Rev<'_> {
    fn lookup_rev(&self, c: u8) -> Option<&[u8]> {
        self.0.get(&c).map(|v| v.as_slice())
    }
}

fn b(s: &str) -> Vec<u8> {
    unhex_opt(s).unwrap_or_default()
}

fn eq(what: &str, input: &[u8], want: &str, got: String) -> Result<(), String> {
    if want == got {
        Ok(())
    } else {
        Err(format!(
            "{} {:?}: want {} got {}",
            what,
            String::from_utf8_lossy(input),
            want,
            got
        ))
    }
}

fn params_str(p: &Option<parse::Params>) -> String {
    match p {
        None => "-".to_string(),
        Some(p) => {
            let mut v = Vec::new();
            for (k, val) in p {
                v.extend_from_slice(k);
                v.push(0);
                v.extend_from_slice(val);
                v.push(1);
            }
            hex(&v)
        }
    }
}

#[test]
fn root_fixtures() {
    check_root(&records("root.txt"));
}

/// Randomized records from the oracle's `fnfuzz` mode (checked-in small set,
/// or `TDEWOLFF_PARSE_FNFUZZ=<dir>` for a large one).
#[test]
fn root_fnfuzz() {
    check_root(&fnfuzz_records("root.txt"));
}

fn check_root(recs: &[Vec<String>]) {
    let maps = entity_maps();
    let mut counts: HashMap<String, usize> = HashMap::new();
    check_all(recs, |r| {
        *counts.entry(r[0].clone()).or_default() += 1;
        match r[0].as_str() {
            "number" => {
                let input = b(&r[1]);
                eq(
                    "number",
                    &input,
                    &r[2],
                    parse::number(&input[..]).to_string(),
                )
            }
            "dimension" => {
                let input = b(&r[1]);
                let (n, u) = parse::dimension(&input[..]);
                eq(
                    "dimension",
                    &input,
                    &format!("{} {}", r[2], r[3]),
                    format!("{} {}", n, u),
                )
            }
            "mediatype" => {
                let input = b(&r[1]);
                let (mt, params) = parse::mediatype(&cp(&input));
                eq(
                    "mediatype",
                    &input,
                    &format!("{} {}", r[2], r[3]),
                    format!("{} {}", hx(&mt), params_str(&params)),
                )
            }
            "datauri" => {
                let input = b(&r[1]);
                let buf = cp(&input);
                let got = match parse::data_uri(&buf) {
                    Ok((m, d)) => format!("{} {} -", hx(&m), hx(&d)),
                    Err(e) => format!("- - {}", err_str(&Some(e))),
                };
                eq(
                    "datauri",
                    &input,
                    &format!("{} {} {} {}", r[2], r[3], r[4], r[5]),
                    format!("{} {}", got, hx(&buf)),
                )
            }
            "quoteentity" => {
                let input = b(&r[1]);
                let (q, n) = parse::quote_entity(&input[..]);
                eq(
                    "quoteentity",
                    &input,
                    &format!("{} {}", r[2], r[3]),
                    format!("{} {}", q, n),
                )
            }
            "rmw" => {
                let input = b(&r[1]);
                let buf = cp(&input);
                let out = parse::replace_multiple_whitespace(buf.clone());
                eq(
                    "rmw",
                    &input,
                    &format!("{} {}", r[2], r[3]),
                    format!("{} {}", hx(&out), hx(&buf)),
                )
            }
            "rent" | "rmwe" => {
                let input = b(&r[2]);
                let (ents, rev) = &maps[&r[1]];
                let ents = Ents(ents);
                let rev_holder;
                let rev: &dyn RevEntityMap = match rev {
                    Some(m) => {
                        rev_holder = Rev(m);
                        &rev_holder
                    }
                    None => &NilMap,
                };
                let buf = cp(&input);
                let out = if r[0] == "rent" {
                    parse::replace_entities(buf.clone(), &ents, rev)
                } else {
                    parse::replace_multiple_whitespace_and_entities(buf.clone(), &ents, rev)
                };
                eq(
                    &format!("{} {}", r[0], r[1]),
                    &input,
                    &format!("{} {}", r[3], r[4]),
                    format!("{} {}", hx(&out), hx(&buf)),
                )
            }
            "decodeurl" => {
                let input = b(&r[1]);
                let buf = cp(&input);
                let out = parse::decode_url(buf.clone());
                eq(
                    "decodeurl",
                    &input,
                    &format!("{} {}", r[2], r[3]),
                    format!("{} {}", hx(&out), hx(&buf)),
                )
            }
            "encodeurl" => {
                let input = b(&r[2]);
                let table = if r[1] == "url" {
                    &parse::URL_ENCODING_TABLE
                } else {
                    &parse::DATA_URI_ENCODING_TABLE
                };
                let buf = cp(&input);
                let out = parse::encode_url(buf.clone(), table);
                eq(
                    "encodeurl",
                    &input,
                    &format!("{} {}", r[3], r[4]),
                    format!("{} {}", hx(&out), hx(&buf)),
                )
            }
            "appendescape" => {
                let input = b(&r[1]);
                let out =
                    parse::append_escape(GoBytes::from_slice(b"pre"), &input[..], b"\"'", b'\\');
                eq("appendescape", &input, &r[2], hx(&out))
            }
            "trim" => {
                let input = b(&r[1]);
                eq(
                    "trim",
                    &input,
                    &r[2],
                    hx(&parse::trim_whitespace(&cp(&input))),
                )
            }
            "tolower" => {
                let input = b(&r[1]);
                eq("tolower", &input, &r[2], hx(&parse::to_lower(cp(&input))))
            }
            "isallws" => {
                let input = b(&r[1]);
                eq(
                    "isallws",
                    &input,
                    &r[2],
                    (parse::is_all_whitespace(&input[..]) as u8).to_string(),
                )
            }
            "equalfold" => {
                let input = b(&r[1]);
                let target = b(&r[2]);
                eq(
                    "equalfold",
                    &input,
                    &r[3],
                    (parse::equal_fold(&input[..], &target[..]) as u8).to_string(),
                )
            }
            "position" => {
                let off: isize = r[1].parse().unwrap();
                let input = b(&r[2]);
                let mut rd = parse::buffer::Reader::new(cp(&input));
                let (line, col, context) = parse::position(Some(&mut rd), off);
                eq(
                    "position",
                    &input,
                    &format!("{} {} {}", r[3], r[4], r[5]),
                    format!("{} {} {}", line, col, hex(&context)),
                )
            }
            "newerror" => {
                let input = b(&r[1]);
                let mut rd = parse::buffer::Reader::new(cp(&input));
                let e = parse::new_error(
                    Some(&mut rd),
                    (input.len() / 2) as isize,
                    b"message 5".to_vec(),
                );
                eq("newerror", &input, &r[2], hex(&e.error_bytes()))
            }
            "printable" => {
                let rr: i64 = r[1].parse().unwrap();
                let got = format!(
                    "{} {}",
                    hex(&parse::printable(rr as i32)),
                    parse::utf8::is_graphic(rr as i32) as u8
                );
                eq(
                    "printable",
                    r[1].as_bytes(),
                    &format!("{} {}", r[2], r[3]),
                    got,
                )
            }
            "table" => {
                let t = if r[1] == "url" {
                    &parse::URL_ENCODING_TABLE
                } else {
                    &parse::DATA_URI_ENCODING_TABLE
                };
                let got: String = t.iter().map(|&x| if x { '1' } else { '0' }).collect();
                eq("table", r[1].as_bytes(), &r[2], got)
            }
            op => Err(format!("unknown op {}", op)),
        }
    });
    let mut c: Vec<_> = counts.into_iter().collect();
    c.sort();
    eprintln!("root ops: {:?}", c);
}

// Hand-ported API tests from upstream (input_test.go, util_test.go,
// error_test.go, position_test.go) that check aliasing and state.

#[test]
fn input_basic() {
    let s = b"Lorem ipsum dolor sit amet, consectetur adipiscing elit.";
    let mut rd = parse::buffer::Reader::new(GoBytes::from_slice(s));
    let z = parse::Input::new(Some(&mut rd));
    assert_eq!(z.bytes().to_vec(), s.to_vec());
    assert_eq!(z.err(), None);
    assert_eq!(z.pos(), 0);
    assert_eq!(z.peek(0), b'L');
    assert_eq!(z.peek(1), b'o');
    z.move_(1);
    assert_eq!(z.peek(0), b'o');
    assert_eq!(z.peek(1), b'r');
    z.rewind(6);
    assert_eq!(z.peek(0), b'i');
    assert_eq!(z.peek(1), b'p');
    assert_eq!(z.offset(), 6);
    assert_eq!(z.lexeme().to_vec(), b"Lorem ".to_vec());
    assert_eq!(z.shift().to_vec(), b"Lorem ".to_vec());
    assert_eq!(z.pos(), 0);
    assert_eq!(z.peek(0), b'i');
    assert_eq!(z.err(), None);
    z.move_((s.len() - 6 - 1) as isize);
    assert_eq!(z.err(), None);
    z.skip();
    assert_eq!(z.pos(), 0);
    z.move_(1);
    assert_eq!(z.err(), Some(parse::GoError::Eof));
    z.move_(-1);
    assert_eq!(z.err(), None);
    z.reset();
    assert_eq!(z.peek(0), b'L');
    assert_eq!(z.len(), s.len());
}

#[test]
fn input_peek_at() {
    // Go `Peek(-1)`/`Peek(-2)` as used by the js lexer
    let z = parse::Input::new_string(b"a/=b");
    z.move_(3);
    assert_eq!(z.peek_at(-1), b'=');
    assert_eq!(z.peek_at(-2), b'/');
    assert_eq!(z.peek_at(-3), b'a');
    assert_eq!(z.peek_at(0), b'b');
    assert_eq!(z.peek_at(1), 0);
    assert_eq!(z.peek_at(0), z.peek(0));
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| z.peek_at(-4)));
    assert!(r.is_err(), "out of range panics like Go");
}

#[test]
fn input_runes() {
    let z = parse::Input::new_string("aæ†\u{100000}".as_bytes());
    assert_eq!(z.peek_rune(0), ('a' as i32, 1));
    assert_eq!(z.peek_rune(1), ('æ' as i32, 2));
    assert_eq!(z.peek_rune(3), ('†' as i32, 3));
    assert_eq!(z.peek_rune(6), (0x100000, 4));
    let z = parse::Input::new_string(b"\xF0");
    assert_eq!(z.peek_rune(0), (0xF0, 1));
}

#[test]
fn input_empty_and_error() {
    let z = parse::Input::new_string(b"");
    assert_eq!(z.peek(0), 0);
    assert!(z.bytes().is_empty());
    assert_eq!(z.err(), Some(parse::GoError::Eof));

    struct ErrReader;
    impl parse::GoReader for ErrReader {
        fn read(&mut self, _: &mut [u8]) -> (usize, Option<parse::GoError>) {
            (0, Some(parse::GoError::Other(b"plain error".to_vec())))
        }
    }
    let z = parse::Input::new(Some(&mut ErrReader));
    assert_eq!(z.peek(0), 0);
    assert_eq!(
        z.err(),
        Some(parse::GoError::Other(b"plain error".to_vec()))
    );
    assert_eq!(z.peek(0), 0);

    let mut r = parse::IoReader(&b"io reader"[..]);
    let z = parse::Input::new(Some(&mut r));
    assert_eq!(z.bytes().to_vec(), b"io reader".to_vec());
}

#[test]
fn input_restore() {
    let b = GoBytes::from_slice(b"abcd");
    let z = parse::Input::new_bytes(b.slice_to(2));
    assert_eq!(z.peek(2), 0, "must have terminating NULL");
    assert_eq!(
        b.to_vec(),
        b"ab\0d".to_vec(),
        "terminating NULL overwrites underlying buffer"
    );
    z.restore();
    assert_eq!(
        b.to_vec(),
        b"abcd".to_vec(),
        "terminating NULL has been restored"
    );

    let b = GoBytes::from_slice(b"test");
    let z = parse::Input::new_bytes(b.clone());
    assert_eq!(z.peek(4), 0);
    assert!(!z.bytes().same_array(&b), "no spare capacity: copied");
}

#[test]
fn util_copy_and_to_lower_alias() {
    let foo = GoBytes::from_slice(b"abc");
    let bar = parse::copy(&foo);
    foo.set(0, b'b');
    assert_eq!(foo.to_vec(), b"bbc".to_vec());
    assert_eq!(bar.to_vec(), b"abc".to_vec());

    let foo = GoBytes::from_slice(b"Abc");
    let bar = parse::to_lower(foo.clone());
    bar.set(1, b'B');
    assert_eq!(foo.to_vec(), b"aBc".to_vec());
    assert_eq!(bar.to_vec(), b"aBc".to_vec());
}

#[test]
fn util_misc() {
    assert!(parse::equal_fold(b"Abc", b"abc"));
    assert!(!parse::equal_fold(b"Abcd", b"abc"));
    assert!(!parse::equal_fold(b"Bbc", b"abc"));
    assert!(!parse::equal_fold(b"[]", b"{}"));
    assert!(parse::is_all_whitespace(b"\t \r\n\x0c"));
    assert!(!parse::is_all_whitespace(b"\t \r\n\x0cx"));
    let printable: Vec<u8> = "a\x00\x7f\u{0800}\u{200F}"
        .chars()
        .flat_map(|c| parse::printable(c as i32))
        .collect();
    assert_eq!(printable, "a0x000x7F\u{0800}U+200F".as_bytes());
}

#[test]
fn error_messages() {
    let mut rd = parse::buffer::Reader::new(GoBytes::from_slice(b"buffer"));
    let err = parse::new_error(Some(&mut rd), 3, b"message".to_vec());
    let (line, column, context) = err.position();
    assert_eq!((line, column), (1, 4));
    assert_eq!(context, b"    1: buffer\n          ^");
    assert_eq!(
        err.error_bytes(),
        b"message on line 1 and column 4\n    1: buffer\n          ^".to_vec()
    );

    let l = parse::Input::new_string(b"buffer");
    l.move_(3);
    let err = parse::new_error_lexer(&l, b"message".to_vec());
    assert_eq!(err.position().1, 4);
    assert_eq!(
        err.error_bytes(),
        b"message on line 1 and column 4\n    1: buffer\n          ^".to_vec()
    );
}

#[test]
fn indenter() {
    use std::io::Write;
    let mut out = Vec::new();
    {
        let mut w = parse::Indenter::new(&mut out, 2);
        assert_eq!(w.indent(), 2);
        w.write_all(b"a\nb\n").unwrap();
    }
    assert_eq!(out, b"a\n  b\n  ".to_vec());
}
