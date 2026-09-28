//! Red-team differential tests (tools/go-oracle/gift/redteam.go): synth-format
//! lines over every image kind (sub-images, type-hiding wrappers, every
//! YCbCr/NYCbCrA ratio, palettes of every colour kind and > 256 entries),
//! extreme filter parameters (NaN, ±Inf, huge, denormal, -0), long chains,
//! resampling extremes, DrawAt into every destination kind, Bounds of chains
//! with extreme parameters, and cases where Go panics (expected `panic`).
//!
//! The checked-in `redteam.tsv.gz` holds a sample of every mode plus the
//! regression and FMA-witness cases; set GIFT_REDTEAM_BIG to a
//! comma-separated list of larger oracle outputs (plain or .gz) to run more.

mod common;

use common::{fixtures_dir, read_tsv, run_redteam_line};

fn run_file(path: &std::path::Path) {
    run_rows(path, &|_| true);
}

fn run_rows(path: &std::path::Path, keep: &dyn Fn(&[String]) -> bool) {
    let rows: Vec<Vec<String>> = read_tsv(path).into_iter().filter(|r| keep(r)).collect();
    assert!(!rows.is_empty());
    // Silence the expected panics (the result is compared, not printed),
    // unless GIFT_REDTEAM_VERBOSE is set.
    let hook = std::panic::take_hook();
    if std::env::var_os("GIFT_REDTEAM_VERBOSE").is_none() {
        std::panic::set_hook(Box::new(|_| {}));
    }
    let mut bad = Vec::new();
    for row in &rows {
        let got = run_redteam_line(row);
        if got != row[6] {
            bad.push((row[..6].join("\t"), row[6].clone(), got));
        }
    }
    std::panic::set_hook(hook);
    for (line, want, got) in bad.iter().take(20) {
        eprintln!("MISMATCH (oracle: gift rtone '<line>'): want {want} got {got}\n{line}");
    }
    eprintln!(
        "{}: {} cases, {} mismatches",
        path.display(),
        rows.len(),
        bad.len()
    );
    assert!(bad.is_empty());
}

#[test]
fn redteam_fixture() {
    run_file(&fixtures_dir().join("redteam.tsv.gz"));
}

/// Red-team regression: transform.go:anchorPt's `b.Max.X - w` and
/// `b.Min.X + (b.Dx()-w)/2` wrap in Go (CropToSize with huge sizes, or a
/// source rect whose Dx overflows); the port panicked with overflow checks.
#[test]
fn regression_crop_to_size_anchor_wraps() {
    run_rows(&fixtures_dir().join("regressions.tsv"), &|r| {
        r[1] == "bounds"
    });
}

/// Red-team regression: gift's coordinate arithmetic (`dstb.Min.X + x -
/// srcb.Min.X`, `dstb.Min.X + srcb.Max.X - srcx - 1`, `Min.X - kradius`,
/// `dstb.Min.Y + h`, splitRange, setPixelRow/Column) wraps in Go for
/// images and DrawAt points near the ends of the int range; the port
/// panicked with overflow checks. One case per site (copyimage, colorchan,
/// colorFilter, convolution + 1-D passes, rank, resize H/V/nearest,
/// transforms, crop, interpolation, setter columns).
#[test]
fn regression_far_origin_coordinates_wrap() {
    run_rows(&fixtures_dir().join("regressions.tsv"), &|r| {
        r[1] != "bounds"
    });
}

#[test]
fn redteam_big() {
    if let Ok(list) = std::env::var("GIFT_REDTEAM_BIG") {
        for p in list.split(',').filter(|p| !p.is_empty()) {
            run_file(std::path::Path::new(p));
        }
    }
}
