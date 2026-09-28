//! `libsasserrors.JsonToError` against Go (go1.27.1: encoding/json on the
//! jsonv2 implementation with the v1 options): tests/fixtures/sass/json.rec.zz
//! (tools/go-oracle/libsass-sys `-json`) holds LibSass-like error documents
//! in many spellings, type mismatches, duplicate and case-folded keys, string
//! escapes and surrogates, invalid UTF-8, number edge cases, nesting-depth
//! limits and random mutations, with the decoded fields and `Error()`.
//!
//! `LIBSASS_JSON_CASES=<path>` checks an additional (larger) corpus.

mod common;

use std::path::Path;

use common::read_records;
use libsass_sys::json_to_error;

fn int(v: &[u8]) -> i64 {
    std::str::from_utf8(v).unwrap().parse().unwrap()
}

fn run(path: &Path) -> (usize, Vec<String>) {
    let recs = read_records(path);
    let mut fails = Vec::new();
    let mut n = 0;
    for c in recs.chunks(7) {
        let keys: Vec<&str> = c.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(
            keys,
            [
                "json", "status", "column", "file", "line", "message", "error"
            ]
        );
        n += 1;
        let e = json_to_error(&c[0].1);
        let got = (
            e.status,
            e.column,
            e.file.as_bytes().to_vec(),
            e.line,
            e.message.as_bytes().to_vec(),
            e.to_string().into_bytes(),
        );
        let want = (
            int(&c[1].1),
            int(&c[2].1),
            c[3].1.clone(),
            int(&c[4].1),
            c[5].1.clone(),
            c[6].1.clone(),
        );
        if got != want {
            let json = String::from_utf8_lossy(&c[0].1);
            let json = if json.len() > 300 {
                format!("{}...[{} bytes]", &json[..300], c[0].1.len())
            } else {
                json.into_owned()
            };
            fails.push(format!(
                "{json:?}:\n  got  {:?}\n  want {:?}",
                (
                    got.0,
                    got.1,
                    String::from_utf8_lossy(&got.2),
                    got.3,
                    String::from_utf8_lossy(&got.4)
                ),
                (
                    want.0,
                    want.1,
                    String::from_utf8_lossy(&want.2),
                    want.3,
                    String::from_utf8_lossy(&want.4)
                )
            ));
        }
    }
    (n, fails)
}

#[test]
fn json_to_error_matches_go() {
    let (n, fails) =
        run(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sass/json.rec.zz"));
    assert!(n >= 3000, "{n}");
    assert!(
        fails.is_empty(),
        "{} of {n} documents differ:\n{}",
        fails.len(),
        fails.join("\n")
    );
}

/// Red-team regression: the validator used to recurse once per nesting
/// level and overflowed the stack (process abort) on the 10000-level
/// documents of json.rec.zz in a debug build. Go decodes them (the file is
/// kept, the nesting-depth limit decides validity).
#[test]
fn deep_nesting_on_a_small_stack() {
    let h = std::thread::Builder::new()
        .stack_size(64 << 10)
        .spawn(|| {
            for (d, valid) in [(9999, true), (10000, false)] {
                let doc = format!(
                    "{{\"file\": \"x\", \"deep\": {}{}}}",
                    "[".repeat(d),
                    "]".repeat(d)
                );
                let e = json_to_error(doc.as_bytes());
                assert_eq!(e.file == "x", valid, "depth {d}");
                let doc = format!(
                    "{{\"file\": \"x\", \"deep\": {}1{}}}",
                    "{\"a\":".repeat(d),
                    "}".repeat(d)
                );
                assert_eq!(
                    json_to_error(doc.as_bytes()).file == "x",
                    valid,
                    "depth {d}"
                );
            }
        })
        .unwrap();
    h.join().unwrap();
}

#[test]
fn json_to_error_external_corpus() {
    let Ok(p) = std::env::var("LIBSASS_JSON_CASES") else {
        return;
    };
    let (n, fails) = run(Path::new(&p));
    eprintln!("json external: {n} documents, {} differ", fails.len());
    assert!(
        fails.is_empty(),
        "{} of {n} documents differ:\n{}",
        fails.len(),
        fails.join("\n")
    );
}
