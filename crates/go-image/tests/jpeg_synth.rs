//! Differential test: synthetic images of every image type (random and
//! smooth content, odd sizes, non-zero origins) encoded at every quality
//! 1..100, plus nil/out-of-range options and a q75 round trip, against the
//! Go oracle (`go-image synth`).

mod common;

use common::*;
use go_image::jpeg;

#[test]
fn synth_encode_all_qualities() {
    let rows = read_tsv("synth.tsv");
    assert_eq!(rows.len(), 300);
    let failures = run(&rows);
    assert_eq!(
        failures,
        0,
        "{} of {} synthetic images differ",
        failures,
        rows.len()
    );
}

/// Larger corpus kept outside the repository (GO_IMAGE_SYNTH_BIG=<path to
/// `go-image synth N` output>).
#[test]
fn synth_encode_all_qualities_big() {
    let Ok(path) = std::env::var("GO_IMAGE_SYNTH_BIG") else {
        return;
    };
    let rows = parse_tsv(&std::fs::read_to_string(path).unwrap());
    let failures = run(&rows);
    assert_eq!(
        failures,
        0,
        "{} of {} synthetic images differ",
        failures,
        rows.len()
    );
}

fn run(rows: &[Vec<String>]) -> usize {
    let mut failures = 0;
    for want in rows {
        let seed: u64 = want[0].parse().unwrap();
        let mut r = Rng::new(seed.wrapping_mul(1000003).wrapping_add(17));
        let kind = r.intn(NUM_KINDS);
        let mut max_size = 40;
        if r.intn(5) == 0 {
            max_size = 300;
        }
        let rr = rand_rect(&mut r, max_size, 10);
        let g = gen_image(&mut r, kind, rr);
        let m = g.as_image();
        let mut hs = Vec::new();
        let mut q75 = None;
        for q in 1..=100 {
            let mut buf = Vec::new();
            match jpeg::encode(&mut buf, m, Some(&jpeg::Options { quality: q })) {
                Err(_) => hs.push("err".to_string()),
                Ok(()) => {
                    hs.push(sha16(&buf));
                    if q == 75 {
                        q75 = Some(buf);
                    }
                }
            }
        }
        let rt = match &q75 {
            None => "none".to_string(),
            Some(b) => match jpeg::decode(&mut &b[..]) {
                Err(e) => format!("err:{}", e),
                Ok(d) => img_digest(d.as_ref()),
            },
        };
        let mut bnil = Vec::new();
        let mut b0 = Vec::new();
        let mut b200 = Vec::new();
        let _ = jpeg::encode(&mut bnil, m, None);
        let _ = jpeg::encode(&mut b0, m, Some(&jpeg::Options { quality: -5 }));
        let _ = jpeg::encode(&mut b200, m, Some(&jpeg::Options { quality: 200 }));
        let got = vec![
            seed.to_string(),
            kind.to_string(),
            rect_str(m.bounds()),
            hs.join(","),
            rt,
            sha16(&bnil),
            sha16(&b0),
            sha16(&b200),
        ];
        if &got != want {
            failures += 1;
            eprintln!("MISMATCH seed {} kind {}", seed, kind);
            for (i, (g, w)) in got.iter().zip(want.iter()).enumerate() {
                if g != w {
                    eprintln!("  field {}: got {}\n            want {}", i, g, w);
                }
            }
        }
    }
    failures
}
