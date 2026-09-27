//! Differential test on real seeksnack images (decoded by Go image/jpeg and
//! image/png and checked in as raw pixel dumps, so no codec is involved):
//! compact crops of JPEGs with 4:4:4, 4:2:0 and 4:2:2 chroma (odd origins),
//! the NRGBA watermark, an RGB PNG (*image.RGBA), a paletted PNG with tRNS,
//! RGBA PNGs (*image.NRGBA) and gift's own testdata/src.png. Each image runs
//! the Hugo pipeline ops (box resizes, same-size copy, watermark overlays)
//! plus every resampling filter and the other gift filters
//! (tools/go-oracle/gift `realfix`).

mod common;

use std::collections::HashMap;
use std::sync::Arc;

use common::{digest, fixtures_dir, load_dump, read_tsv, run_real_op};
use go_image::Image;

#[test]
fn real_images() {
    let rows = read_tsv(&fixtures_dir().join("real.tsv.gz"));
    let mut dumps: HashMap<String, Arc<dyn Image>> = HashMap::new();
    let mut load = |name: &str| -> Arc<dyn Image> {
        dumps
            .entry(name.to_string())
            .or_insert_with(|| {
                Arc::from(load_dump(
                    &fixtures_dir().join("real").join(format!("{name}.gz")),
                ))
            })
            .clone()
    };
    let wm = load("watermark_full");
    let lookup = |n: &str| -> Arc<dyn Image> {
        assert_eq!(n, "watermark_full");
        wm.clone()
    };
    let mut bad = 0;
    let mut per_image: HashMap<String, usize> = HashMap::new();
    for row in &rows {
        let src = load(&row[0]);
        let out = run_real_op(&*src, &row[1], &row[2], &lookup);
        *per_image.entry(row[0].clone()).or_default() += 1;
        if digest(out.as_ref()) != row[3] {
            bad += 1;
            if bad <= 20 {
                eprintln!("MISMATCH {} {} {}", row[0], row[1], row[2]);
            }
        }
    }
    eprintln!(
        "{} ops on {} images, {} mismatches",
        rows.len(),
        per_image.len(),
        bad
    );
    assert!(rows.len() > 1000);
    assert_eq!(bad, 0);
}
