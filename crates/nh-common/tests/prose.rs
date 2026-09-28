//! Differential tests for `nh_common::prose` (jdkato/prose transform/title.go) against
//! `tests/fixtures/prose/title.json`, written by `tools/go-oracle/nh-common/prose`.

use nh_common::flect;
use nh_common::prose::{TitleConverter, TitleStyle};
use serde_json::Value as J;

fn fixture() -> J {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/prose/title.json"
    );
    let b = std::fs::read(path).expect("read fixture");
    serde_json::from_slice(&b).expect("parse fixture")
}

/// A fixture string: a JSON string, or `{"hex": ...}` for invalid UTF-8.
fn bytes(v: &J) -> Vec<u8> {
    match v {
        J::String(s) => s.as_bytes().to_vec(),
        J::Object(o) => hex(o["hex"].as_str().expect("hex")),
        other => panic!("not a string: {other}"),
    }
}

/// A fixture result: a string, or `{"panic": msg}`.
fn result(v: &J) -> Result<Vec<u8>, String> {
    match v {
        J::Object(o) if o.contains_key("panic") => Err(o["panic"].as_str().unwrap().to_string()),
        _ => Ok(bytes(v)),
    }
}

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn show(r: &Result<Vec<u8>, String>) -> String {
    match r {
        Ok(b) => format!("{:?}", String::from_utf8_lossy(b)),
        Err(e) => format!("panic({e})"),
    }
}

#[test]
fn title_matches_go() {
    let f = fixture();
    let ap = TitleConverter::new(TitleStyle::Ap);
    let chicago = TitleConverter::new(TitleStyle::Chicago);
    let cases = f["cases"].as_array().unwrap();
    assert!(cases.len() > 3000, "corpus too small: {}", cases.len());

    let mut bad = Vec::new();
    for c in cases {
        let input = bytes(&c["in"]);
        let run = |tc: &TitleConverter, s: &[u8]| {
            tc.try_title_bytes(s).map_err(|e| e.message().to_string())
        };
        let checks = [
            ("ap", run(&ap, &input)),
            ("chicago", run(&chicago, &input)),
            ("create_title", run(&ap, &input)),
            ("ap_pluralize", run(&ap, &flect::pluralize_bytes(&input))),
        ];
        for (name, got) in checks {
            let want = result(&c[name]);
            if got != want {
                bad.push(format!(
                    "{name}({:?}): got {} want {}",
                    String::from_utf8_lossy(&input),
                    show(&got),
                    show(&want)
                ));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "{} mismatches:\n{}",
        bad.len(),
        bad[..bad.len().min(40)].join("\n")
    );
}

/// prose's own `TestTitle` table (testdata/title.json), as recorded by the oracle.
#[test]
fn upstream_title_table() {
    let f = fixture();
    let ap = TitleConverter::new(TitleStyle::Ap);
    let table = f["upstream"].as_array().unwrap();
    assert!(!table.is_empty());
    for c in table {
        let input = c["in"].as_str().unwrap();
        let want = c["expect"].as_str().unwrap();
        assert_eq!(ap.title(input), want, "Title({input:?})");
    }
}

/// CM §0.10: auto section titles of seeksnack.
#[test]
fn section_titles() {
    let ap = TitleConverter::new(TitleStyle::Ap);
    assert_eq!(flect::pluralize("biscuit-roll"), "biscuit-rolls");
    assert_eq!(ap.title("biscuit-rolls"), "Biscuit-Rolls");
    assert_eq!(ap.title(&flect::pluralize("candy")), "Candies");
    assert_eq!(ap.title(&flect::pluralize("corn-chips")), "Corn-Chips");
    assert_eq!(ap.title("potato-chips"), "Potato-Chips");
    assert_eq!(ap.title("INS 160a (I)"), "INS 160a (I)");
    assert_eq!(ap.title("lay's"), "Lay's");
    assert_eq!(ap.title("เลย์ สแตคส์ "), "เลย์ สแตคส์ ");
    assert_eq!(ap.title(""), "");
}
