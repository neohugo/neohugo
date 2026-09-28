//! Differential test against `tools/go-oracle/nh-hugofs/nfc`: Go's `norm.NFC.String` (the darwin
//! file-name normalization) on every code point and on decomposed, Hangul, Thai and random
//! strings. Runs on every OS.

mod support;

use std::collections::HashSet;

use nh_hugofs::nfc::nfc_string;
use serde_json::Value as J;
use support::*;

fn str_of(v: &J) -> String {
    match v {
        J::String(s) => s.clone(),
        J::Object(o) => {
            let h = o["hex"].as_str().unwrap();
            let b: Vec<u8> = (0..h.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&h[i..i + 2], 16).unwrap())
                .collect();
            String::from_utf8(b).expect("the nfc fixture has valid UTF-8 only")
        }
        _ => panic!("{v}"),
    }
}

#[test]
fn nfc_matches_go() {
    let fixture = load_fixture(&fixture_dir("nfc").join("nfc.json.gz"));
    assert_eq!(fixture["version"], "15.0.0");
    let mut listed = HashSet::new();
    let mut failures = Vec::new();
    let cases = fixture["cases"].as_array().unwrap();
    for c in cases {
        let input = str_of(&c["in"]);
        let want = match c.get("out") {
            Some(o) => str_of(o),
            None => input.clone(),
        };
        let mut it = input.chars();
        if let (Some(ch), None) = (it.next(), it.next()) {
            listed.insert(ch);
        }
        let got = nfc_string(&input);
        if got != want {
            failures.push(format!("{input:?}: want {want:?}, got {got:?}"));
        }
    }
    // Every code point the fixture does not list is its own NFC.
    let mut n = cases.len();
    for r in 0..=0x10FFFFu32 {
        let Some(ch) = char::from_u32(r) else {
            continue;
        };
        if listed.contains(&ch) {
            continue;
        }
        n += 1;
        let s = ch.to_string();
        let got = nfc_string(&s);
        if got != s {
            failures.push(format!("U+{r:04X}: want unchanged, got {got:?}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {n} differ:\n{}",
        failures.len(),
        failures[..failures.len().min(20)].join("\n")
    );
}
