//! `text::norm`, `text::xtransform`, `text::runes` and `text::remove_accents`: the differential
//! test against `tools/go-oracle/nh-common/norm` (x/text v0.26.0 `unicode/norm` NFC/NFD,
//! `transform.String`, and neohugo's `text.RemoveAccents`/`RemoveAccentsString`).

mod t02support;

use std::collections::HashSet;

use nh_common::text::{self, norm, xtransform};
use serde_json::Value as J;
use t02support::*;

/// The results of every ported function on `s`, in the fixture's key order.
const KEYS: [&str; 8] = ["nfc", "nfd", "nfcB", "nfdB", "tnfc", "tnfd", "ra", "ras"];

fn run(s: &[u8]) -> [Vec<u8>; 8] {
    let mut nfc_t = norm::NFC;
    let mut nfd_t = norm::NFD;
    let ras = text::remove_accents_string_bytes(s);
    // RemoveAccentsString always yields valid UTF-8 (the `&str` API relies on it).
    assert!(std::str::from_utf8(&ras).is_ok(), "{s:?}: invalid UTF-8");
    [
        norm::NFC.string_bytes(s),
        norm::NFD.string_bytes(s),
        norm::NFC.bytes(s),
        norm::NFD.bytes(s),
        xtransform::string(&mut nfc_t, s).0,
        xtransform::string(&mut nfd_t, s).0,
        text::remove_accents(s),
        ras,
    ]
}

/// The expected results of a case, with the fixture's fallbacks.
fn want(c: &J) -> [Vec<u8>; 8] {
    let input = bytes(&c["in"]);
    let get = |k: &str, fallback: &Vec<u8>| c.get(k).map(bytes).unwrap_or_else(|| fallback.clone());
    let nfc = get("nfc", &input);
    let nfd = get("nfd", &input);
    let ra = get("ra", &input);
    [
        nfc.clone(),
        nfd.clone(),
        get("nfcB", &nfc),
        get("nfdB", &nfd),
        get("tnfc", &nfc),
        get("tnfd", &nfd),
        ra.clone(),
        get("ras", &ra),
    ]
}

fn check(cases: &[J], fails: &mut Vec<String>) -> usize {
    let mut n = 0;
    for c in cases {
        let input = bytes(&c["in"]);
        let got = run(&input);
        let want = want(c);
        for (k, (g, w)) in got.iter().zip(want.iter()).enumerate() {
            n += 1;
            if g != w {
                fails.push(format!(
                    "{}({:?}): want {:?}, got {:?}",
                    KEYS[k],
                    String::from_utf8_lossy(&input),
                    String::from_utf8_lossy(w),
                    String::from_utf8_lossy(g)
                ));
            }
        }
    }
    n
}

fn report(name: &str, n: usize, fails: &[String]) {
    assert!(
        fails.is_empty(),
        "{name}: {} of {n} results differ:\n{}",
        fails.len(),
        fails[..fails.len().min(30)].join("\n")
    );
}

#[test]
fn runes_match_go() {
    let f = fixture("norm/runes.json.gz");
    assert_eq!(f["version"], norm::VERSION);
    let cases = f["cases"].as_array().unwrap();
    let n_single = f["singleRunes"].as_u64().unwrap() as usize;
    let mut fails = Vec::new();
    let mut n = check(cases, &mut fails);

    // Every code point that is not listed is unchanged by every function.
    let listed: HashSet<Vec<u8>> = cases[..n_single].iter().map(|c| bytes(&c["in"])).collect();
    assert_eq!(listed.len(), n_single);
    let mut unchanged = 0;
    for r in 0..=0x10FFFFu32 {
        let Some(ch) = char::from_u32(r) else {
            continue;
        };
        let s = ch.to_string().into_bytes();
        if listed.contains(&s) {
            continue;
        }
        unchanged += 1;
        for (k, g) in run(&s).iter().enumerate() {
            n += 1;
            if *g != s {
                fails.push(format!("{}(U+{r:04X}): want unchanged, got {g:?}", KEYS[k]));
            }
        }
    }
    eprintln!(
        "norm runes: {} cases ({n_single} listed code points) + {unchanged} unchanged code points, {n} results",
        cases.len()
    );
    report("runes", n, &fails);
}

#[test]
fn strings_match_go() {
    let f = fixture("norm/strings.json.gz");
    assert_eq!(f["version"], norm::VERSION);
    let cases = f["cases"].as_array().unwrap();
    let mut fails = Vec::new();
    let n = check(cases, &mut fails);
    eprintln!("norm strings: {} cases, {n} results", cases.len());
    report("strings", n, &fails);
}

// Go: common/text/transform_test.go:TestRemoveAccents
#[test]
fn go_test_remove_accents() {
    assert_eq!(text::remove_accents("Resumé".as_bytes()), b"Resume");
    assert_eq!(text::remove_accents(b"Hugo Rocks!"), b"Hugo Rocks!");
    assert_eq!(text::remove_accents_string("Resumé"), "Resume");
}
