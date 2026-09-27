//! Differential tests: decode real JPEGs (Go's image/testdata, checked-in
//! seeksnack samples and — when available — every seeksnack JPEG) and
//! compare DecodeConfig, decoded type/geometry/buffers, the q75 re-encoding,
//! the draw.Draw(RGBA, Src) conversion and its q75 encoding with the Go
//! oracle (`go-image decode-files`).

mod common;

use common::*;
use go_image::draw::{self, Op};
use go_image::jpeg;
use go_image::{Image, RGBA};

fn enc_sha(m: &dyn Image, q: i64) -> String {
    let mut buf = Vec::new();
    match jpeg::encode(&mut buf, m, Some(&jpeg::Options { quality: q })) {
        Ok(()) => sha(&buf),
        Err(e) => format!("err:{}", e),
    }
}

/// Computes the oracle's decode-files record for one file.
fn record(rel: &str, data: &[u8]) -> Vec<String> {
    let cfg_str = match jpeg::decode_config(&mut &data[..]) {
        Err(e) => format!("err:{}", e),
        Ok(c) => format!("{},{},{}", model_name(&c.color_model), c.width, c.height),
    };
    let m = match jpeg::decode(&mut &data[..]) {
        Err(e) => return vec![rel.to_string(), cfg_str, format!("err:{}", e)],
        Ok(m) => m,
    };
    let b = m.bounds();
    let mut rgba = RGBA::new(b);
    draw::draw(&mut rgba, b, m.as_ref(), b.min, Op::Src);
    vec![
        rel.to_string(),
        cfg_str,
        img_digest(m.as_ref()),
        enc_sha(m.as_ref(), 75),
        sha(&rgba.pix),
        enc_sha(&rgba, 75),
    ]
}

fn check(root: &std::path::Path, rows: &[Vec<String>]) -> usize {
    let mut failures = 0;
    for want in rows {
        let data = std::fs::read(root.join(&want[0])).unwrap();
        let got = record(&want[0], &data);
        if &got != want {
            failures += 1;
            eprintln!("MISMATCH {}", want[0]);
            for (i, (g, w)) in got.iter().zip(want.iter()).enumerate() {
                if g != w {
                    eprintln!("  field {}: got {} want {}", i, g, w);
                }
            }
            if got.len() != want.len() {
                eprintln!("  len got {} want {}", got.len(), want.len());
            }
        }
    }
    failures
}

#[test]
fn decode_checked_in_fixtures() {
    let rows = read_tsv("fixtures-decode.tsv");
    assert_eq!(rows.len(), 40);
    let failures = check(&fixtures_dir(), &rows);
    assert_eq!(failures, 0, "{} of {} files differ", failures, rows.len());
}

/// All 538 seeksnack JPEGs. The corpus lives outside the repository; the test
/// is skipped when it is not present (override with GO_IMAGE_SITE_ROOT).
#[test]
fn decode_seeksnack_corpus() {
    let root = std::env::var("GO_IMAGE_SITE_ROOT").unwrap_or_else(|_| {
        "/private/tmp/claude-501/-Users-blackb1rd-git-github-org-neohugo/0835ef6d-534f-4ac6-a7eb-50e9240d6eb1/scratchpad/canon/pristine-seeksnack".to_string()
    });
    let root = std::path::PathBuf::from(root);
    if !root.join("content").exists() {
        eprintln!("seeksnack corpus not found at {}; skipping", root.display());
        return;
    }
    let rows = read_tsv("site-decode.tsv");
    assert_eq!(rows.len(), 538);
    let failures = check(&root, &rows);
    assert_eq!(failures, 0, "{} of {} files differ", failures, rows.len());
}
