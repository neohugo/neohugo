//! Nesting limits and uint16 wrap-around (Go oracle `limits`,
//! tools/go-oracle/tdewolff-parse-js/limits.go): inputs at, just below and
//! over the parser's 1000-statement/1000-expression nesting limits for 52
//! nesting shapes, and inputs whose `Var.Uses`, `NumForDecls`,
//! `NumFuncArgs` or `NumArgUses` wrap around 65535. The fixture stores the
//! input specs and the FNV digests of every mode ("-" where Go's `String()`
//! is too slow: `ExprStmt.String` evaluates its value twice, which is
//! exponential in the nesting depth).

mod common;

use common::*;

/// Go `strconv.Unquote` of a `strconv.Quote`d string (the escapes it
/// produces for the ASCII specs).
fn unquote(s: &str) -> String {
    let b = s.as_bytes();
    assert!(
        b.len() >= 2 && b[0] == b'"' && b[b.len() - 1] == b'"',
        "{}",
        s
    );
    let mut out = String::new();
    let mut i = 1;
    while i < b.len() - 1 {
        if b[i] == b'\\' {
            i += 1;
            match b[i] {
                b'n' => out.push('\n'),
                b't' => out.push('\t'),
                b'r' => out.push('\r'),
                b'\\' => out.push('\\'),
                b'"' => out.push('"'),
                c => panic!("unsupported escape \\{} in {}", c as char, s),
            }
        } else {
            out.push(b[i] as char);
        }
        i += 1;
    }
    out
}

// Go: limits.go:limitSpec.build
fn build(f: &[String], n: usize) -> Vec<u8> {
    let rep = |u: &str| -> String {
        if u.contains("%d") {
            (0..n).map(|i| u.replace("%d", &i.to_string())).collect()
        } else {
            u.repeat(n)
        }
    };
    let mut s = f[0].clone();
    s.push_str(&rep(&f[1]));
    s.push_str(&f[2]);
    s.push_str(&rep(&f[3]));
    s.push_str(&f[4]);
    s.into_bytes()
}

/// Checks the fixture lines whose repeated unit does (`distinct`) or does
/// not contain `%d`: 65536 distinct names are quadratic to declare (Go's
/// linear scope lookups, kept by the port), too slow for a debug build.
fn check_limits(distinct: bool) {
    let tsv = std::fs::read_to_string(fixtures_dir().join("limits.tsv")).unwrap();
    let (n, fails) = big_stack(move || {
        let mut n = 0;
        let mut fails = Vec::new();
        for line in tsv.lines().filter(|l| !l.starts_with('#')) {
            let cols: Vec<&str> = line.split('\t').collect();
            assert_eq!(cols.len(), 6 + MODES.len(), "{}", line);
            let spec: Vec<String> = cols[..5].iter().map(|c| unquote(c)).collect();
            if spec[1].contains("%d") != distinct {
                continue;
            }
            let count: usize = cols[5].parse().unwrap();
            let src = build(&spec, count);
            for (k, mode) in MODES.iter().enumerate() {
                if cols[6 + k] == "-" {
                    continue;
                }
                n += 1;
                if digest(&run_mode(mode, &src)) != cols[6 + k] {
                    fails.push(format!("{} {:?} x{}", mode, spec, count));
                }
            }
        }
        (n, fails)
    });
    for f in fails.iter().take(20) {
        eprintln!("{}", f);
    }
    assert!(fails.is_empty(), "{} of {} checks differ", fails.len(), n);
    eprintln!("limits: {} checks identical", n);
}

#[test]
fn limits() {
    check_limits(false);
}

/// `cargo test --release --test limits -- --ignored`
#[test]
#[ignore]
fn limits_distinct_names() {
    check_limits(true);
}
