//! Differential test over every seeksnack image (content/ and assets/, 603
//! JPEGs and PNGs decoded by Go and dumped as raw pixels by
//! `tools/go-oracle/gift site <siteRoot> <outDir>`): the Hugo pipeline
//! (box resizes to 600x480, 300x240, 600x200, 128x128, 32x32, same-size
//! copies, watermark overlays at the original size and after 600x480 /
//! 300x240 resizes) plus two resizes with every resampling filter, and for
//! every 8th image the full filter table.
//!
//! The corpus lives outside the repository (about 290 MB). The test reads it
//! from $GIFT_SITE_DIR and is skipped (passes with a note) when the variable
//! is unset or the directory has no ops.tsv.

mod common;

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use common::{digest, load_dump, read_tsv, run_real_op};
use go_image::Image;

#[test]
fn site_corpus() {
    let Ok(dir) = std::env::var("GIFT_SITE_DIR") else {
        eprintln!("GIFT_SITE_DIR not set; site corpus test skipped");
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    if !dir.join("ops.tsv").exists() {
        eprintln!("site corpus not found in {}; skipped", dir.display());
        return;
    }
    let index = read_tsv(&dir.join("index.tsv"));
    let wm_name = index
        .iter()
        .find(|r| r[1] == "assets/images/watermark.png")
        .map(|r| r[0].clone())
        .expect("watermark in index");
    let wm: Arc<dyn Image> = Arc::from(load_dump(&dir.join("imgs").join(format!("{wm_name}.gz"))));

    let mut ops: BTreeMap<usize, Vec<Vec<String>>> = BTreeMap::new();
    for row in read_tsv(&dir.join("ops.tsv")) {
        ops.entry(row[0].parse().unwrap()).or_default().push(row);
    }
    let jobs: Vec<(usize, Vec<Vec<String>>)> = ops.into_iter().collect();
    let next = AtomicUsize::new(0);
    let total = AtomicUsize::new(0);
    let failures: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                let lookup = |n: &str| -> Arc<dyn Image> {
                    assert_eq!(n, wm_name);
                    wm.clone()
                };
                loop {
                    let j = next.fetch_add(1, Ordering::SeqCst);
                    if j >= jobs.len() {
                        break;
                    }
                    let (n, rows) = &jobs[j];
                    let src = load_dump(&dir.join("imgs").join(format!("{n}.gz")));
                    for row in rows {
                        let out = run_real_op(&*src, &row[1], &row[2], &lookup);
                        total.fetch_add(1, Ordering::SeqCst);
                        if digest(out.as_ref()) != row[3] {
                            failures.lock().unwrap().push(format!(
                                "MISMATCH image {} ({}) {} {}",
                                n, index[*n][1], row[1], row[2]
                            ));
                        }
                    }
                }
            });
        }
    });

    let failures = failures.into_inner().unwrap();
    for f in failures.iter().take(30) {
        eprintln!("{f}");
    }
    eprintln!(
        "site corpus: {} images, {} ops, {} mismatches",
        jobs.len(),
        total.load(Ordering::SeqCst),
        failures.len()
    );
    assert!(failures.is_empty());
}
