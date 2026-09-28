//! The `autoid` oracle (tools/go-oracle/nh-markup/autoid): `SanitizeAnchorName` for the three
//! auto ID types over a Unicode sweep and random strings (incl. invalid UTF-8).

mod common;

use common::*;
use nh_markup::goldmark::autoid::sanitize_anchor_name_bytes;

const SEPS: [&str; 6] = ["", " ", "-", "_", "\u{a0}", "  "];

/// `sweepChunks` of the Go oracle.
fn sweep_chunks() -> Vec<Vec<u8>> {
    let mut runes: Vec<char> = (0u32..0x3400).filter_map(char::from_u32).collect();
    let mut r = 0x3400u32;
    while r <= 0x10FFFF {
        if let Some(c) = char::from_u32(r) {
            runes.push(c);
        }
        r += 13;
    }
    let mut out = Vec::new();
    let mut i = 0;
    while i * 48 < runes.len() {
        let end = ((i + 1) * 48).min(runes.len());
        let mut b = String::new();
        if i % 5 == 0 {
            b.push_str("\u{3000} ");
        }
        for (j, c) in runes[i * 48..end].iter().enumerate() {
            if j > 0 {
                b.push_str(SEPS[(i + j) % SEPS.len()]);
            }
            b.push(*c);
        }
        if i % 7 == 0 {
            b.push_str(" \u{a0}");
        }
        out.push(b.into_bytes());
        i += 1;
    }
    out
}

#[test]
fn sanitize_anchor_name() {
    let fx = read_fixture("autoid/autoid.json.gz");
    let types: Vec<&str> = fx["id_types"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let sweep = sweep_chunks();
    let inputs: Vec<Vec<u8>> = fx["inputs"].as_array().unwrap().iter().map(bytes).collect();
    let mut failures = Vec::new();
    let mut n = 0;
    for (ti, t) in types.iter().enumerate() {
        let want_sweep = fx["sweep"][ti].as_array().unwrap();
        assert_eq!(want_sweep.len(), sweep.len(), "sweep length");
        for (s, w) in sweep.iter().zip(want_sweep) {
            let got = sanitize_anchor_name_bytes(s, t);
            if got != bytes(w) {
                failures.push(format!(
                    "{t}: {:?}\n got {:?}\nwant {:?}",
                    show(s),
                    show(&got),
                    show(&bytes(w))
                ));
            }
            n += 1;
        }
        let want_random = fx["random"][ti].as_array().unwrap();
        for (s, w) in inputs.iter().zip(want_random) {
            let got = sanitize_anchor_name_bytes(s, t);
            if got != bytes(w) {
                failures.push(format!(
                    "{t}: {:?}\n got {:?}\nwant {:?}",
                    s,
                    show(&got),
                    show(&bytes(w))
                ));
            }
            n += 1;
        }
    }
    eprintln!(
        "autoid: {} of {n} sanitized names identical",
        n - failures.len()
    );
    if !failures.is_empty() {
        for f in failures.iter().take(10) {
            eprintln!("{f}");
        }
        panic!("{} failures", failures.len());
    }
}
