//! Every pixel setter (NRGBA, NRGBA64, RGBA, RGBA64, Gray, Gray16, Paletted
//! and the generic Set path of CMYK, Alpha, Alpha16) fed with 65536
//! adversarial float32 values (rounding boundaries of f32u8/f32u16 +-4 ulps,
//! out-of-range, NaN, infinities, denormals) through ColorFunc, plus large
//! DrawAt Over cases over every destination type
//! (tools/go-oracle/gift `setter`).

mod common;

use std::sync::Arc;

use common::{digest, fixtures_dir, gen_image, parse_filters, parse_img_spec, pi, read_tsv};
use go_image::{Image, pt};

fn no_dumps(n: &str) -> Arc<dyn Image> {
    panic!("no dump {}", n)
}

#[test]
fn setter_cases() {
    let rows = read_tsv(&fixtures_dir().join("setter.tsv.gz"));
    let mut bad = 0;
    for row in &rows {
        let src = gen_image(&parse_img_spec(&row[2]));
        let g = gift::new(parse_filters(&row[5], &no_dumps));
        let mut dst = gen_image(&parse_img_spec(&row[3]));
        if row[1] == "drawat" {
            let f: Vec<&str> = row[4].split(',').collect();
            let op = if pi(f[2]) == 1 {
                gift::OVER_OPERATOR
            } else {
                gift::COPY_OPERATOR
            };
            g.draw_at(dst.draw_image(), src.image(), pt(pi(f[0]), pi(f[1])), op);
        } else {
            g.draw(dst.draw_image(), src.image());
        }
        if digest(dst.image()) != row[6] {
            bad += 1;
            eprintln!("MISMATCH {}", row[..6].join("\t"));
        }
    }
    eprintln!("{} setter/over cases, {} mismatches", rows.len(), bad);
    assert!(rows.len() >= 60);
    assert_eq!(bad, 0);
}
