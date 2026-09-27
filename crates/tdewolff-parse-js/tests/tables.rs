//! Exhaustive checks against `tables.txt.gz` (Go oracle `tables` command):
//! `TokenType.String` for all 65536 values, `OpPrec`/`DeclType` strings,
//! `IsIdentifierStart/Continue/End` and the lexer's token streams around
//! every code point (incl. surrogates and invalid UTF-8),
//! `AsIdentifierName`/`AsDecimalLiteral` over all short strings of an
//! interesting alphabet, and the tie order of `sort.Sort(VarsByUses)` (the
//! minifier's renaming order).

mod common;

use std::collections::HashMap;
use std::io::Read;

use common::*;
use tdewolff_parse::{GoBytes, GoError, Input};
use tdewolff_parse_js::*;

fn sections() -> HashMap<String, Vec<String>> {
    let p = fixtures_dir().join("tables.txt.gz");
    let f = std::fs::File::open(&p).unwrap_or_else(|e| panic!("{}: {}", p.display(), e));
    let mut s = String::new();
    flate2::read::GzDecoder::new(f)
        .read_to_string(&mut s)
        .unwrap();
    let mut m: HashMap<String, Vec<String>> = HashMap::new();
    let mut cur = String::new();
    for line in s.lines() {
        if let Some(name) = line.strip_prefix('#') {
            cur = name.to_string();
            m.entry(cur.clone()).or_default();
        } else {
            m.get_mut(&cur).unwrap().push(line.to_string());
        }
    }
    m
}

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn b01(b: bool) -> &'static str {
    if b { "1" } else { "0" }
}

// mirrors tables.go:encRune
fn enc_rune(r: u32) -> Vec<u8> {
    if r < 0x80 {
        vec![r as u8]
    } else if r < 0x800 {
        vec![0xC0 | (r >> 6) as u8, 0x80 | (r as u8 & 0x3F)]
    } else if r < 0x10000 {
        vec![
            0xE0 | (r >> 12) as u8,
            0x80 | ((r >> 6) as u8 & 0x3F),
            0x80 | (r as u8 & 0x3F),
        ]
    } else {
        vec![
            0xF0 | (r >> 18) as u8,
            0x80 | ((r >> 12) as u8 & 0x3F),
            0x80 | ((r >> 6) as u8 & 0x3F),
            0x80 | (r as u8 & 0x3F),
        ]
    }
}

// mirrors tables.go:tokSig
fn tok_sig(src: &[u8], re: bool) -> String {
    let mut parts: Vec<String> = Vec::new();
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut l = Lexer::new(Input::new_bytes(GoBytes::from_slice(src)));
        for i in 0..6 {
            let (mut tt, mut data) = l.next();
            if re && i == 0 && tt == DivToken {
                (tt, data) = l.reg_exp();
            }
            let mut p = format!("{}:{}", tt.0, data.len());
            if tt == ErrorToken {
                if matches!(l.err(), Some(GoError::Eof)) {
                    parts.push(p);
                    break;
                }
                p.push('E');
            }
            parts.push(p);
        }
    }));
    if r.is_err() {
        // Go's deferred `s += "P"` runs before the named result is assigned
        return "P".to_string();
    }
    parts.join(",")
}

// mirrors tables.go:bytesSig
fn bytes_sig(b: &[u8]) -> String {
    let mut s = String::new();
    s.push_str(b01(is_identifier_start(b)));
    s.push_str(b01(is_identifier_continue(b)));
    s.push_str(b01(is_identifier_end(b)));
    let cat = |parts: &[&[u8]]| -> Vec<u8> { parts.concat() };
    for input in [
        b.to_vec(),
        cat(&[b"a", b, b"a"]),
        cat(&[b" ", b]),
        cat(&[b"#", b]),
        cat(&[b"\n-->", b]),
        cat(&[b"'", b, b"'"]),
        cat(&[b"`", b, b"`"]),
    ] {
        s.push('|');
        s.push_str(&tok_sig(&input, false));
    }
    s.push('|');
    s.push_str(&tok_sig(&cat(&[b"/x/", b]), true));
    s
}

#[test]
fn tables() {
    // the lexer panics (like Go) on some inputs; keep the output quiet
    std::panic::set_hook(Box::new(|_| {}));
    let m = sections();
    let mut fails = Vec::new();

    // TokenType.String
    let mut named = HashMap::new();
    for l in &m["tokens"] {
        let (n, s) = l.split_once(' ').unwrap();
        named.insert(n.parse::<u32>().unwrap(), s.to_string());
    }
    for i in 0..=u16::MAX {
        let tt = TokenType(i);
        let want = named
            .get(&(i as u32))
            .cloned()
            .unwrap_or_else(|| format!("Invalid({})", i));
        if tt.string() != want || tt.bytes().is_some() != named.contains_key(&(i as u32)) {
            fails.push(format!("token {}: {:?} want {:?}", i, tt.string(), want));
        }
    }

    for l in &m["opprec"] {
        let (n, s) = l.split_once(' ').unwrap();
        let got = OpPrec(n.parse().unwrap()).string();
        if got != s {
            fails.push(format!("opprec {}: {:?} want {:?}", n, got, s));
        }
    }
    for l in &m["decltype"] {
        let (n, s) = l.split_once(' ').unwrap();
        let got = DeclType(n.parse().unwrap()).string();
        if got != s {
            fails.push(format!("decltype {}: {:?} want {:?}", n, got, s));
        }
    }

    // every code point
    let mut covered = 0u32;
    for l in &m["runes"] {
        let mut it = l.splitn(3, ' ');
        let lo = u32::from_str_radix(it.next().unwrap(), 16).unwrap();
        let hi = u32::from_str_radix(it.next().unwrap(), 16).unwrap();
        let sig = it.next().unwrap();
        assert_eq!(lo, covered);
        for r in lo..=hi {
            let got = bytes_sig(&enc_rune(r));
            if got != sig {
                fails.push(format!("rune {:x}: {} want {}", r, got, sig));
            }
        }
        covered = hi + 1;
    }
    assert_eq!(covered, 0x110000);

    for l in &m["bytes"] {
        let (h, sig) = l.split_once(' ').unwrap();
        let got = bytes_sig(&hex(h));
        if got != sig {
            fails.push(format!("bytes {}: {} want {}", h, got, sig));
        }
    }

    for l in &m["asname"] {
        let (h, want) = l.split_once(' ').unwrap();
        let b = hex(h);
        let got = format!(
            "{}{}",
            b01(as_identifier_name(&b[..])),
            b01(as_decimal_literal(&b[..]))
        );
        if got != want {
            fails.push(format!("asname {}: {} want {}", h, got, want));
        }
    }

    // sort.Sort(VarsByUses)
    for l in &m["sort"] {
        let (uses, perm) = l.split_once('|').unwrap();
        let uses: Vec<u16> = if uses.is_empty() {
            vec![]
        } else {
            uses.split(',').map(|u| u.parse().unwrap()).collect()
        };
        let mut ast = Ast::new();
        let mut vs: Vec<NodeId> = uses
            .iter()
            .enumerate()
            .map(|(i, &u)| {
                ast.alloc_var(
                    GoBytes::from_slice(i.to_string().as_bytes()),
                    NodeId::NIL,
                    u,
                    NoDecl,
                )
            })
            .collect();
        sort_vars_by_uses(&ast.nodes, &mut vs);
        let got: Vec<String> = vs
            .iter()
            .map(|&v| ast.var(v).data.to_string_lossy())
            .collect();
        if got.join(",") != perm {
            fails.push(format!("sort {:?}: {} want {}", uses, got.join(","), perm));
        }
    }

    let _ = std::panic::take_hook();
    for f in fails.iter().take(30) {
        eprintln!("{}", f);
    }
    assert!(fails.is_empty(), "{} table entries differ", fails.len());
}
