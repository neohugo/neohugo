//! Differential test of random gift filter chains over generated images of
//! every Go image type: tools/go-oracle/gift `synth`.
//!
//! The checked-in fixture has 8000 cases; set GIFT_SYNTH_BIG to a larger
//! oracle output (plain or .gz) to run more.

mod common;

use std::sync::Arc;

use common::{
    TestImage, digest, fixtures_dir, gen_image, parse_filters, parse_img_spec, pi, read_tsv,
};
use go_image::{Image, pt};

fn no_dumps(n: &str) -> Arc<dyn Image> {
    panic!("no dump {}", n)
}

/// Runs one synth line (Go: main.go:runSynth) and returns the digest.
fn run_line(row: &[String]) -> String {
    let src = gen_image(&parse_img_spec(&row[2]));
    let g = gift::new(parse_filters(&row[5], &no_dumps));
    let mut dst = gen_image(&parse_img_spec(&row[3]));
    let dst_img: &mut dyn go_image::draw::Image = dst.draw_image();
    if row[1] == "drawat" {
        let f: Vec<&str> = row[4].split(',').collect();
        let op = if pi(f[2]) == 1 {
            gift::OVER_OPERATOR
        } else {
            gift::COPY_OPERATOR
        };
        g.draw_at(dst_img, src.image(), pt(pi(f[0]), pi(f[1])), op);
    } else {
        g.draw(dst_img, src.image());
    }
    digest(dst.image())
}

fn run_file(path: &std::path::Path) {
    let rows = read_tsv(path);
    let mut bad = 0;
    for row in &rows {
        let got = run_line(row);
        if got != row[6] {
            bad += 1;
            if bad <= 20 {
                eprintln!(
                    "MISMATCH (oracle: gift one '<line>'):\n{}",
                    row[..6].join("\t")
                );
            }
        }
    }
    eprintln!(
        "{}: {} cases, {} mismatches",
        path.display(),
        rows.len(),
        bad
    );
    assert_eq!(bad, 0);
}

#[test]
fn synth_fixture() {
    run_file(&fixtures_dir().join("synth.tsv.gz"));
}

#[test]
fn synth_big() {
    if let Ok(p) = std::env::var("GIFT_SYNTH_BIG") {
        run_file(std::path::Path::new(&p));
    }
}

#[allow(dead_code)]
fn _unused(_: TestImage) {}
