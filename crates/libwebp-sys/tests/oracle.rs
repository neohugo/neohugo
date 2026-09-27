//! Differential tests against github.com/bep/gowebp@v0.3.0 (fixtures from
//! tools/go-oracle/libwebp-sys).

mod common;

use std::path::{Path, PathBuf};

use common::{Pack, check};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/webp")
}

fn run_pack(pack: &Pack, threads: usize) -> (usize, Vec<String>) {
    let n = pack.len();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let fails = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if i >= n {
                        break;
                    }
                    let c = pack.case(i);
                    if let Some(f) = check(&c) {
                        fails.lock().unwrap().push(f);
                    }
                }
            });
        }
    });
    let mut f = fails.into_inner().unwrap();
    f.sort();
    (n, f)
}

#[test]
fn checked_in_fixtures_match_go() {
    let pack = Pack::open(&fixtures());
    assert!(pack.len() > 900);
    let (n, fails) = run_pack(&pack, 8);
    assert!(
        fails.is_empty(),
        "{} of {} cases differ:\n{}",
        fails.len(),
        n,
        fails.join("\n")
    );
}

/// Same fixtures, single-threaded (the DSP init path runs before any
/// concurrency in the other test only if it runs first; this covers both).
#[test]
fn checked_in_fixtures_match_go_serial() {
    let pack = Pack::open(&fixtures());
    let (n, fails) = run_pack(&pack, 1);
    assert!(
        fails.is_empty(),
        "{} of {} cases differ:\n{}",
        fails.len(),
        n,
        fails.join("\n")
    );
}

/// Every WebP in the golden seeksnack build (807 files). The pack (~420 MB)
/// is produced by `tools/go-oracle/libwebp-sys -mode golden` and lives
/// outside the repository; set LIBWEBP_GOLDEN_PACK to its directory.
#[test]
fn golden_seeksnack_webps() {
    let Ok(dir) = std::env::var("LIBWEBP_GOLDEN_PACK") else {
        eprintln!("LIBWEBP_GOLDEN_PACK not set; skipping");
        return;
    };
    let pack = Pack::open(Path::new(&dir));
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let (n, fails) = run_pack(&pack, threads);
    eprintln!("golden webps: {} cases, {} differ", n, fails.len());
    assert!(
        fails.is_empty(),
        "{} of {} cases differ:\n{}",
        fails.len(),
        n,
        fails.join("\n")
    );
}

/// Adversarial geometry (`-mode edge`): negative / truncated strides,
/// allocation failures and over-size gray images that libwebp rejects before
/// reading the pixels, negative `Rect.Min`/`Max`, overlapping gray rows, odd
/// sizes around the macroblock size.
#[test]
fn edge_fixtures_match_go() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/webp-edge");
    let pack = Pack::open(&dir);
    assert!(pack.len() >= 50);
    let (n, fails) = run_pack(&pack, 4);
    assert!(
        fails.is_empty(),
        "{} of {} cases differ:\n{}",
        fails.len(),
        n,
        fails.join("\n")
    );
}
