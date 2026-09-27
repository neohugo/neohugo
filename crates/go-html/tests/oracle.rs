//! Differential tests against fixtures produced by tools/go-oracle/go-html.

use go_html::{escape_string, escape_string_bytes, unescape_string, unescape_string_bytes};

/// Decodes the oracle's line encoding: printable ASCII verbatim, `\xHH` otherwise.
fn unesc(s: &str) -> Vec<u8> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\' {
            assert_eq!(b[i + 1], b'x', "bad escape in fixture");
            let h = std::str::from_utf8(&b[i + 2..i + 4]).unwrap();
            out.push(u8::from_str_radix(h, 16).unwrap());
            i += 4;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    out
}

fn run_file(path: &str) -> usize {
    let data = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let mut n = 0;
    let mut failures = 0;
    for (lineno, line) in data.lines().enumerate() {
        let f: Vec<&str> = line.split('\t').collect();
        assert_eq!(f.len(), 3, "line {}", lineno + 1);
        let input = unesc(f[0]);
        let want_esc = unesc(f[1]);
        let want_unesc = unesc(f[2]);
        let got_esc = escape_string_bytes(&input);
        let got_unesc = unescape_string_bytes(&input);
        if got_esc != want_esc || got_unesc != want_unesc {
            failures += 1;
            if failures <= 20 {
                eprintln!(
                    "line {}: input {:?}\n  esc   want {:?} got {:?}\n  unesc want {:?} got {:?}",
                    lineno + 1,
                    String::from_utf8_lossy(&input),
                    String::from_utf8_lossy(&want_esc),
                    String::from_utf8_lossy(&got_esc),
                    String::from_utf8_lossy(&want_unesc),
                    String::from_utf8_lossy(&got_unesc)
                );
            }
        }
        if let Ok(s) = std::str::from_utf8(&input) {
            assert_eq!(escape_string(s).as_bytes(), &got_esc[..]);
            assert_eq!(unescape_string(s).as_bytes(), &got_unesc[..]);
        }
        n += 1;
    }
    assert_eq!(failures, 0, "{failures} mismatches out of {n}");
    n
}

#[test]
fn oracle_fixture() {
    let n = run_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/unescape.txt"
    ));
    assert!(n > 20000, "fixture too small: {n}");
}

/// Lines containing '&' from the golden seeksnack HTML output.
#[test]
fn oracle_golden_sample() {
    let n = run_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/golden-sample.txt"
    ));
    assert!(n > 900, "fixture too small: {n}");
}

/// Adversarial sample (`go-html -adv 20000 -seed 5 -every 40`): every prefix
/// of every entity name with assorted suffixes, every no-semicolon entity
/// followed by every byte, glued entity fragments and long numeric
/// references (the longest-prefix and int32 wrap-around rules).
#[test]
fn oracle_adversarial() {
    let n = run_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/adversarial.txt"
    ));
    assert!(n > 8000, "fixture too small: {n}");
}

/// Large scratch corpus: GO_HTML_CORPUS=<file> cargo test --release -- --ignored
#[test]
#[ignore]
fn oracle_corpus() {
    let path = std::env::var("GO_HTML_CORPUS").expect("set GO_HTML_CORPUS");
    let n = run_file(&path);
    eprintln!("{n} cases OK");
}

// Go: html/escape_test.go:unescapeTests
#[test]
fn go_unescape_tests() {
    let tests: &[(&str, &str, &str)] = &[
        ("copy", "A\ttext\nstring", "A\ttext\nstring"),
        ("simple", "&amp; &gt; &lt;", "& > <"),
        ("stringEnd", "&amp &amp", "& &"),
        (
            "multiCodepoint",
            "text &gesl; blah",
            "text \u{22db}\u{fe00} blah",
        ),
        ("decimalEntity", "Delta = &#916; ", "Delta = Δ "),
        (
            "hexadecimalEntity",
            "Lambda = &#x3bb; = &#X3Bb ",
            "Lambda = λ = λ ",
        ),
        (
            "numericEnds",
            "&# &#x &#128;43 &copy = &#169f = &#xa9",
            "&# &#x €43 © = ©f = ©",
        ),
        ("numericReplacements", "Footnote&#x87;", "Footnote‡"),
        ("copySingleAmpersand", "&", "&"),
        ("copyAmpersandNonEntity", "text &test", "text &test"),
        ("copyAmpersandHash", "text &#", "text &#"),
    ];
    for (desc, html, want) in tests {
        assert_eq!(unescape_string(html), *want, "{desc}");
    }
}

// Go: html/escape_test.go:TestUnescapeEscape
#[test]
fn go_unescape_escape() {
    let ss = [
        "",
        "abc def",
        "a & b",
        "a&amp;b",
        "a &amp b",
        "&quot;",
        "\"",
        "\"<&>\"",
        "&quot;&lt;&amp;&gt;&quot;",
        "3&5==1 && 0<1, \"0&lt;1\", a+acute=&aacute;",
        "The special characters are: <, >, &, ' and \"",
    ];
    for s in ss {
        assert_eq!(unescape_string(&escape_string(s)), s);
    }
}

#[test]
fn entity_tables() {
    assert_eq!(go_html::entity(b"amp;"), '&' as i32);
    assert_eq!(go_html::entity(b"amp"), '&' as i32);
    assert_eq!(go_html::entity(b"nGt;"), 0);
    assert_eq!(go_html::entity2(b"gesl;"), [0x22DB, 0xFE00]);
    assert_eq!(go_html::entity2(b"nGt;"), [0, 0]);
    assert_eq!(go_html::entities().count(), 2138);
    assert_eq!(go_html::entities2().count(), 91);
}
