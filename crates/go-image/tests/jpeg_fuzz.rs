//! Differential test: decode deterministically mutated JPEGs (bit flips,
//! stray 0xff, truncation, deletions, insertions) and compare DecodeConfig,
//! the error string or the full decoded digest with the Go oracle
//! (`go-image fuzz`).

mod common;

use common::*;
use go_image::jpeg;

fn bases() -> Vec<Vec<u8>> {
    // Same order as the oracle's (sorted) command line.
    let mut names: Vec<String> = std::fs::read_dir(fixtures_dir().join("gotestdata"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n.ends_with(".jpeg"))
        .map(|n| format!("gotestdata/{}", n))
        .collect();
    names.push("site/content_pretzels_combos-pizzeria-pretzel_combos-pipr.jpg".to_string());
    names.push("site/content_potato-crisps_pringles-paprika_pringles-paprika.jpg".to_string());
    names.push("site/content_cookies_alices-pineapple-pastry_600x200.jpg".to_string());
    // The oracle was run with `ls`-sorted paths.
    names.sort();
    names
        .iter()
        .map(|n| std::fs::read(fixtures_dir().join(n)).unwrap())
        .collect()
}

fn run(rows: &[Vec<String>]) -> usize {
    let bases = bases();
    assert_eq!(bases.len(), 33);
    let mut failures = 0;
    for want in rows {
        let seed: u64 = want[0].parse().unwrap();
        let mut r = Rng::new(seed.wrapping_mul(31).wrapping_add(1));
        let bi = r.intn(bases.len() as i64) as usize;
        let mut b = bases[bi].clone();
        let nm = 1 + r.intn(4);
        for _k in 0..nm {
            if b.is_empty() {
                break;
            }
            match r.intn(5) {
                0 => {
                    let i = r.intn(b.len() as i64) as usize;
                    b[i] ^= (1 + r.intn(255)) as u8;
                }
                1 => {
                    let i = r.intn(b.len() as i64) as usize;
                    b[i] = 0xff;
                }
                2 => {
                    let n = r.intn(b.len() as i64) as usize;
                    b.truncate(n);
                }
                3 => {
                    let i = r.intn(b.len() as i64) as usize;
                    let mut m = r.intn(16) as usize;
                    if i + m > b.len() {
                        m = b.len() - i;
                    }
                    b.drain(i..i + m);
                }
                _ => {
                    let i = r.intn(b.len() as i64 + 1) as usize;
                    let m = 1 + r.intn(4);
                    let mut ins = Vec::new();
                    for _ in 0..m {
                        ins.push(r.byte());
                    }
                    b.splice(i..i, ins);
                }
            }
        }
        let mut got = vec![seed.to_string(), bi.to_string(), sha16(&b)];
        match jpeg::decode_config(&mut &b[..]) {
            Err(e) => got.push(format!("err:{}", e)),
            Ok(c) => {
                got.push(format!(
                    "{},{},{}",
                    model_name(&c.color_model),
                    c.width,
                    c.height
                ));
                if c.width * c.height > 4 << 20 {
                    got.push("skip".to_string());
                    if &got != want {
                        failures += 1;
                        eprintln!("MISMATCH got {:?}\n         want {:?}", got, want);
                    }
                    continue;
                }
            }
        }
        match jpeg::decode(&mut &b[..]) {
            Err(e) => got.push(format!("err:{}", e)),
            Ok(m) => got.push(img_digest(m.as_ref())),
        }
        if &got != want {
            failures += 1;
            if failures < 20 {
                eprintln!("MISMATCH got {:?}\n         want {:?}", got, want);
            }
        }
    }
    failures
}

#[test]
fn fuzz_decode() {
    let rows = read_tsv("fuzz.tsv");
    assert_eq!(rows.len(), 5000);
    let failures = run(&rows);
    assert_eq!(
        failures,
        0,
        "{} of {} mutated files differ",
        failures,
        rows.len()
    );
}

/// Larger corpus kept outside the repository (GO_IMAGE_FUZZ_BIG=<path to
/// `go-image fuzz 100000 ...` output>).
#[test]
fn fuzz_decode_big() {
    let Ok(path) = std::env::var("GO_IMAGE_FUZZ_BIG") else {
        return;
    };
    let rows = parse_tsv(&std::fs::read_to_string(path).unwrap());
    let failures = run(&rows);
    assert_eq!(
        failures,
        0,
        "{} of {} mutated files differ",
        failures,
        rows.len()
    );
}
