//! Shared helpers for the differential tests: fixture loading and the stream
//! serializations, mirrored byte for byte from
//! tools/go-oracle/tdewolff-parse/streams.go.
#![allow(dead_code)]

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::path::PathBuf;

use tdewolff_parse::{GoBytes, GoError, Input, css, html, json, xml};

pub fn fixtures_dir() -> PathBuf {
    if let Ok(d) = std::env::var("TDEWOLFF_PARSE_FIXTURES") {
        return PathBuf::from(d);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Reads a fixture file as tab-separated records (comment lines skipped).
pub fn records(name: &str) -> Vec<Vec<String>> {
    records_at(&fixtures_dir().join(name))
}

/// The `fnfuzz` record set: `$TDEWOLFF_PARSE_FNFUZZ/<name>` or the checked-in
/// tests/fixtures/fnfuzz/<name>.
pub fn fnfuzz_records(name: &str) -> Vec<Vec<String>> {
    let dir = std::env::var("TDEWOLFF_PARSE_FNFUZZ")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fnfuzz")
        });
    records_at(&dir.join(name))
}

/// Reads tab-separated records from a path (comment lines skipped).
pub fn records_at(p: &std::path::Path) -> Vec<Vec<String>> {
    let s = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {}", p.display(), e));
    s.lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| l.split('\t').map(|f| f.to_string()).collect())
        .collect()
}

pub fn unhex(s: &str) -> Vec<u8> {
    assert!(s.len().is_multiple_of(2), "odd hex {}", s);
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

/// Decodes a hex field; `-` is a nil slice (None).
pub fn unhex_opt(s: &str) -> Option<Vec<u8>> {
    if s == "-" { None } else { Some(unhex(s)) }
}

pub fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for c in b {
        write!(s, "{:02x}", c).unwrap();
    }
    s
}

/// Go `hx`: hex, `-` for nil.
pub fn hx(b: &GoBytes) -> String {
    if b.is_nil() {
        "-".to_string()
    } else {
        hex(&b.to_vec())
    }
}

pub fn hx_vec(b: &Option<Vec<u8>>) -> String {
    match b {
        None => "-".to_string(),
        Some(b) => hex(b),
    }
}

/// Go `errStr`.
pub fn err_str(e: &Option<GoError>) -> String {
    match e {
        None => "-".to_string(),
        Some(e) => hex(&e.error_bytes()),
    }
}

/// Go `cp`: `append([]byte(nil), s...)`.
pub fn cp(s: &[u8]) -> GoBytes {
    GoBytes::nil().append(s)
}

fn token_limit(input: &[u8]) -> usize {
    4 * input.len() + 64
}

pub fn html_stream(input: &[u8], tmpl: bool) -> String {
    html_stream_delims(input, if tmpl { Some(html::GO_TEMPLATE) } else { None })
}

/// `html_stream` with the given template delimiters (None: plain lexer).
pub fn html_stream_delims(input: &[u8], delims: Option<[&str; 2]>) -> String {
    let mut sb = String::new();
    let b = cp(input);
    let z = Input::new_bytes(b.clone());
    let mut l = match delims {
        Some(d) => html::Lexer::new_template(z.clone(), d),
        None => html::Lexer::new(z.clone()),
    };
    let limit = token_limit(input);
    let mut i = 0;
    loop {
        let (tt, data) = l.next();
        writeln!(
            sb,
            "{} {} {} {} {} {}",
            z.offset(),
            tt as u32,
            hx(&data),
            hx(&l.text()),
            hx(&l.attr_val()),
            l.has_template() as u8
        )
        .unwrap();
        if tt == html::ErrorToken || i > limit {
            break;
        }
        i += 1;
    }
    writeln!(sb, "err {}", err_str(&l.err())).unwrap();
    z.restore();
    writeln!(sb, "buf {}", hx(&b)).unwrap();
    sb
}

pub fn css_lex_stream(input: &[u8]) -> String {
    let mut sb = String::new();
    let b = cp(input);
    let z = Input::new_bytes(b.clone());
    let mut l = css::Lexer::new(z.clone());
    let limit = token_limit(input);
    let mut i = 0;
    loop {
        let (tt, data) = l.next();
        writeln!(sb, "{} {} {}", z.offset(), tt as u32, hx(&data)).unwrap();
        if tt == css::ErrorToken || i > limit {
            break;
        }
        i += 1;
    }
    writeln!(sb, "err {}", err_str(&l.err())).unwrap();
    z.restore();
    writeln!(sb, "buf {}", hx(&b)).unwrap();
    sb
}

pub fn css_parse_stream(input: &[u8], inline: bool) -> String {
    let mut sb = String::new();
    let b = cp(input);
    let z = Input::new_bytes(b.clone());
    let mut p = css::Parser::new(z.clone(), inline);
    let limit = token_limit(input);
    let mut i = 0;
    loop {
        let (gt, tt, data) = p.next();
        write!(
            sb,
            "{} {} {} {} {}",
            p.offset(),
            gt as u32,
            tt as u32,
            hx(&data),
            p.values().len()
        )
        .unwrap();
        for v in p.values() {
            write!(sb, " {}:{}", v.token_type as u32, hx(&v.data)).unwrap();
        }
        write!(sb, " {}", p.has_parse_error() as u8).unwrap();
        let err = p.err();
        if gt == css::ErrorGrammar {
            write!(sb, " {}", err_str(&err)).unwrap();
        }
        sb.push('\n');
        if gt == css::ErrorGrammar && err == Some(GoError::Eof) || i > limit {
            break;
        }
        i += 1;
    }
    z.restore();
    writeln!(sb, "buf {}", hx(&b)).unwrap();
    sb
}

pub fn xml_stream(input: &[u8]) -> String {
    let mut sb = String::new();
    let b = cp(input);
    let z = Input::new_bytes(b.clone());
    let mut l = xml::Lexer::new(z.clone());
    let limit = token_limit(input);
    let mut i = 0;
    loop {
        let (tt, data) = l.next();
        writeln!(
            sb,
            "{} {} {} {} {}",
            z.offset(),
            tt as u32,
            hx(&data),
            hx(&l.text()),
            hx(&l.attr_val())
        )
        .unwrap();
        if tt == xml::ErrorToken || i > limit {
            break;
        }
        i += 1;
    }
    writeln!(sb, "err {}", err_str(&l.err())).unwrap();
    z.restore();
    writeln!(sb, "buf {}", hx(&b)).unwrap();
    sb
}

pub fn json_stream(input: &[u8]) -> String {
    let mut sb = String::new();
    let b = cp(input);
    let z = Input::new_bytes(b.clone());
    let mut p = json::Parser::new(z.clone());
    let limit = token_limit(input);
    let mut i = 0;
    loop {
        let (gt, data) = p.next();
        writeln!(
            sb,
            "{} {} {} {}",
            z.offset(),
            gt as u32,
            hx(&data),
            p.state() as u32
        )
        .unwrap();
        if gt == json::ErrorGrammar || i > limit {
            break;
        }
        i += 1;
    }
    writeln!(sb, "err {}", err_str(&p.err())).unwrap();
    z.restore();
    writeln!(sb, "buf {}", hx(&b)).unwrap();
    sb
}

pub fn stream_for(kind: &str, input: &[u8]) -> String {
    match kind {
        "html" => html_stream(input, false),
        "htmltmpl" => html_stream(input, true),
        "htmlphp" => html_stream_delims(input, Some(html::PHP_TEMPLATE)),
        "csslex" => css_lex_stream(input),
        "css" => css_parse_stream(input, false),
        "cssinline" => css_parse_stream(input, true),
        "xml" => xml_stream(input),
        "json" => json_stream(input),
        _ => panic!("unknown kind {}", kind),
    }
}

pub fn fnv64a(b: &[u8]) -> u64 {
    let mut h: u64 = 14695981039346656037;
    for &c in b {
        h ^= c as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}

/// Shows the first differing line of two streams.
pub fn first_diff(a: &str, b: &str) -> String {
    for (i, (x, y)) in a.lines().zip(b.lines()).enumerate() {
        if x != y {
            return format!("line {}:\n  go:   {}\n  rust: {}", i, x, y);
        }
    }
    format!(
        "length differs: go {} lines, rust {} lines",
        a.lines().count(),
        b.lines().count()
    )
}

/// Named entity maps from entities.txt: (entities, rev entities or None for nil).
pub type EntityMaps = BTreeMap<String, (HashMap<Vec<u8>, Vec<u8>>, Option<HashMap<u8, Vec<u8>>>)>;

pub fn entity_maps() -> EntityMaps {
    let mut maps: EntityMaps = BTreeMap::new();
    for r in records("entities.txt") {
        let e = maps
            .entry(r[1].clone())
            .or_insert_with(|| (HashMap::new(), Some(HashMap::new())));
        match r[0].as_str() {
            "ent" => {
                e.0.insert(unhex(&r[2]), unhex(&r[3]));
            }
            "revnil" => e.1 = None,
            "rev" => {
                e.1.get_or_insert_with(HashMap::new)
                    .insert(r[2].parse::<u8>().unwrap(), unhex(&r[3]));
            }
            _ => panic!("bad record"),
        }
    }
    maps
}

/// Runs `check` over all records, collecting failures (reports up to 20).
pub fn check_all<F: FnMut(&[String]) -> Result<(), String>>(
    recs: &[Vec<String>],
    mut check: F,
) -> usize {
    let mut fails = Vec::new();
    for r in recs {
        if let Err(e) = check(r) {
            fails.push(e);
        }
    }
    for f in fails.iter().take(20) {
        eprintln!("FAIL: {}", f);
    }
    assert!(
        fails.is_empty(),
        "{} of {} records failed",
        fails.len(),
        recs.len()
    );
    recs.len()
}
