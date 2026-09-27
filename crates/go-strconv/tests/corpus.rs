//! Real-site corpus checks (tools/go-oracle/go-strconv/corpus.go,
//! `oracle -mode corpus`).
//!
//! `corpus_numbers`: every numeric-looking token in the seeksnack sources
//! and in the golden Go build output (5,648 tokens) through ParseFloat
//! (64/32), FormatFloat of the result, ParseInt, Atoi and ParseUint.
//!
//! `corpus_quote`: every line of every seeksnack text source through the
//! quoting functions, one hash per file. It needs the site sources:
//! GO_STRCONV_SITE=<pristine-seeksnack dir> (skipped otherwise).

mod common;

use common::*;
use go_strconv as sc;
use go_strconv::internal;

fn code(e: Option<internal::Error>) -> &'static str {
    match e {
        None => "nil",
        Some(internal::Error::Range) => "range",
        Some(internal::Error::Syntax) => "syntax",
        Some(_) => "other",
    }
}

#[test]
fn corpus_numbers() {
    let mut n = 0;
    let mut bad = 0;
    for l in fixture_lines("corpus_numbers.txt") {
        let f: Vec<&str> = l.split('\t').collect();
        assert_eq!(f.len(), 18, "{l}");
        let t = f[0].as_bytes();
        let (v64, e64) = internal::parse_float(t, 64);
        let (v32, e32) = internal::parse_float(t, 32);
        let (pi10, epi10) = internal::parse_int(t, 10, 64);
        let (pi0, epi0) = internal::parse_int(t, 0, 64);
        let (a, ea) = internal::atoi(t);
        let (pu0, epu0) = internal::parse_uint(t, 0, 32);
        let got = [
            f[0].to_string(),
            format!("{:016x}", v64.to_bits()),
            code(e64).to_string(),
            format!("{:016x}", v32.to_bits()),
            code(e32).to_string(),
            sc::format_float(v64, b'g', -1, 64),
            sc::format_float(v64, b'f', -1, 64),
            sc::format_float(v64, b'e', -1, 64),
            sc::format_float(v64, b'f', 2, 64),
            sc::format_float(v32, b'g', -1, 32),
            pi10.to_string(),
            code(epi10).to_string(),
            pi0.to_string(),
            code(epi0).to_string(),
            a.to_string(),
            code(ea).to_string(),
            pu0.to_string(),
            code(epu0).to_string(),
        ];
        if got.iter().map(|s| s.as_str()).ne(f.iter().copied()) {
            bad += 1;
            if bad < 20 {
                eprintln!("got  {}\nwant {}", got.join("\t"), l);
            }
        }
        n += 1;
    }
    assert_eq!(bad, 0, "{bad} of {n} corpus tokens differ");
    assert!(n > 5000, "{n}");
}

/// bufio.ScanLines: split on '\n', drop one trailing '\r', no empty final
/// line after a trailing newline.
fn scan_lines(data: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut rest = data;
    while !rest.is_empty() {
        let (line, next) = match rest.iter().position(|&c| c == b'\n') {
            Some(i) => (&rest[..i], &rest[i + 1..]),
            None => (rest, &rest[rest.len()..]),
        };
        out.push(line.strip_suffix(b"\r").unwrap_or(line));
        rest = next;
    }
    out
}

fn quote_file(s: &mut Sink, data: &[u8]) {
    s.put_str(sc::quote(data));
    s.put_str(sc::quote_to_ascii(data));
    for line in scan_lines(data) {
        let q = sc::quote(line);
        s.put_str(&q);
        s.put_str(sc::quote_to_ascii(line));
        s.put_str(sc::quote_to_graphic(line));
        s.put_bool(sc::can_backquote(line));
        let u = sc::unquote(&q);
        s.put_str(u.as_deref().unwrap_or(b""));
        s.put_err(u.as_ref().err());
        let u = sc::unquote(line);
        s.put_str(u.as_deref().unwrap_or(b""));
        s.put_err(u.as_ref().err());
        let p = sc::quoted_prefix(line);
        s.put_str(p.unwrap_or(b""));
        s.put_err(p.err());
    }
}

#[test]
fn corpus_quote() {
    let Ok(site) = std::env::var("GO_STRCONV_SITE") else {
        eprintln!("corpus_quote skipped: set GO_STRCONV_SITE=<seeksnack source dir>");
        return;
    };
    let mut n = 0;
    let mut bytes = 0;
    for l in fixture_lines("corpus_quote.txt") {
        let f: Vec<&str> = l.split('\t').collect();
        let data = std::fs::read(format!("{site}/{}", f[0]))
            .unwrap_or_else(|e| panic!("{site}/{}: {e}", f[0]));
        assert_eq!(data.len().to_string(), f[1], "{}: file size differs", f[0]);
        let mut s = Sink::new(false);
        quote_file(&mut s, &data);
        assert_eq!(format!("{:016x}", s.h), f[2], "{}", f[0]);
        n += 1;
        bytes += data.len();
    }
    eprintln!("corpus_quote: {n} files, {bytes} bytes match");
    assert!(n > 300);
}
