//! Differential test of every resampling kernel (gift's five and neohugo's
//! eleven) against go1.27.1 darwin/arm64: tools/go-oracle/gift `kernels`.

mod common;

use common::{fixtures_dir, read_tsv, resampling};

const NAMES: [&str; 16] = [
    "nearestneighbor",
    "box",
    "linear",
    "cubic",
    "lanczos",
    "hermite",
    "mitchellnetravali",
    "catmullrom",
    "bspline",
    "gaussian",
    "hann",
    "hamming",
    "blackman",
    "bartlett",
    "welch",
    "cosine",
];

fn f(s: &str) -> f32 {
    f32::from_bits(u32::from_str_radix(s, 16).unwrap())
}

#[test]
fn kernel_vectors() {
    let rows = read_tsv(&fixtures_dir().join("kernels.tsv.gz"));
    let mut n = 0;
    let mut bad = 0;
    for row in &rows {
        match row[0].as_str() {
            "support" => {
                let r = resampling(&row[1]);
                assert_eq!(r.support().to_bits(), f(&row[2]).to_bits(), "{}", row[1]);
            }
            "k" => {
                let x = f(&row[1]);
                for (i, want) in row[2].split(',').enumerate() {
                    let want = f(want);
                    let got = resampling(NAMES[i]).kernel(x);
                    n += 1;
                    if got.to_bits() != want.to_bits() && !(got.is_nan() && want.is_nan()) {
                        bad += 1;
                        if bad < 30 {
                            eprintln!(
                                "{}({:e} [{:08x}]) = {:e} [{:08x}], want {:e} [{:08x}]",
                                NAMES[i],
                                x,
                                x.to_bits(),
                                got,
                                got.to_bits(),
                                want,
                                want.to_bits()
                            );
                        }
                    }
                }
            }
            _ => panic!("bad row"),
        }
    }
    eprintln!("{} kernel evaluations", n);
    assert!(n >= 20000 * 16);
    assert_eq!(bad, 0);
}
