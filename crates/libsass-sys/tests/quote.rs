//! `libsasserrors.Error.Error()` formats the file name with `%q`
//! (`strconv.Quote`). This checks the port against Go over every code point:
//! tests/fixtures/sass/quote.tsv (tools/go-oracle/libsass-sys
//! `writeQuoteTable`) holds the length and FNV-1a-64 hash of Go's
//! `strconv.Quote` of each block of 256 code points (surrogates skipped).

use libsass_sys::libsasserrors::go_quote;

fn fnv64(b: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &c in b {
        h ^= c as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

#[test]
fn quote_matches_strconv_quote_for_all_runes() {
    let text = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sass/quote.tsv"),
    )
    .unwrap();
    let mut n = 0;
    let mut fails = Vec::new();
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = line.split('\t').collect();
        let lo: u32 = f[0].parse().unwrap();
        let want_len: usize = f[1].parse().unwrap();
        let want_fnv = u64::from_str_radix(f[2], 16).unwrap();
        let s: String = (lo..lo + 256).filter_map(char::from_u32).collect();
        let q = go_quote(&s);
        if q.len() != want_len || fnv64(q.as_bytes()) != want_fnv {
            fails.push(format!("block U+{lo:04X}: got len {}", q.len()));
        }
        n += 1;
    }
    assert_eq!(n, 0x110000 / 256);
    assert!(fails.is_empty(), "{}", fails.join("\n"));
}

#[test]
fn quote_examples() {
    // fmt.Sprintf("%q", ...) in Go 1.27.
    assert_eq!(go_quote("stdin"), "\"stdin\"");
    assert_eq!(
        go_quote("a\u{0378}\u{061c}\u{fffe}\u{E0001} \u{a0}\u{2028}é日\u{1F600}"),
        "\"a\\u0378\\u061c\\ufffe\\U000e0001 \\u00a0\\u2028é日\u{1F600}\""
    );
    assert_eq!(
        go_quote("\u{7}\u{8}\u{c}\n\r\t\u{b}\u{1}\u{7f}\"\\"),
        "\"\\a\\b\\f\\n\\r\\t\\v\\x01\\x7f\\\"\\\\\""
    );
}
