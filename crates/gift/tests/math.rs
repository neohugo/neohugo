//! Differential test of the Go math port (gift::gomath) against go1.27.1
//! darwin/arm64: tools/go-oracle/gift `math 20000`.

mod common;

use common::{fixtures_dir, read_tsv};
use gift::gomath;

fn h(s: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(s, 16).unwrap())
}

fn same(got: f64, want: f64) -> bool {
    got.to_bits() == want.to_bits() || (got.is_nan() && want.is_nan())
}

#[test]
fn math_vectors() {
    let rows = read_tsv(&fixtures_dir().join("math.tsv.gz"));
    assert!(rows.len() >= 20000);
    let mut bad = 0;
    let mut nan_payload = 0;
    for row in &rows {
        let (x, y) = (h(&row[0]), h(&row[1]));
        let (s, c) = gomath::sincos(x);
        let got = [
            gomath::exp(x),
            gomath::log(x),
            gomath::pow(x, y),
            gomath::sin(x),
            gomath::cos(x),
            s,
            c,
        ];
        let names = [
            "Exp",
            "Log",
            "Pow",
            "Sin",
            "Cos",
            "Sincos.sin",
            "Sincos.cos",
        ];
        for (k, g) in got.iter().enumerate() {
            let want = h(&row[2 + k]);
            if !same(*g, want) {
                bad += 1;
                if bad < 20 {
                    eprintln!(
                        "{}({:e} [{:016x}], {:e}) = {:e} [{:016x}], want {:e} [{:016x}]",
                        names[k],
                        x,
                        x.to_bits(),
                        y,
                        g,
                        g.to_bits(),
                        want,
                        want.to_bits()
                    );
                }
            } else if g.to_bits() != want.to_bits() {
                nan_payload += 1;
            }
        }
    }
    eprintln!(
        "{} vectors, {} NaN payload differences",
        rows.len(),
        nan_payload
    );
    assert_eq!(bad, 0);
}
