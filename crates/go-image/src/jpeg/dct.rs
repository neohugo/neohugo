//! Port of Go 1.27.1 `image/jpeg/dct.go`: the integer Loeffler (11-multiply)
//! forward and inverse DCT introduced in Go 1.26.
//!
//! All int32 arithmetic uses wrapping operations, matching Go's defined
//! two's-complement overflow (reachable only with corrupt input).

/// A block is an 8x8 input to a 2D DCT (either the FDCT or IDCT).
///
/// Go: image/jpeg/dct.go:block
pub(crate) type Block = [i32; BLOCK_SIZE];

pub(crate) const BLOCK_SIZE: usize = 8 * 8;

// Constants needed for the implementation.
// These are all 60-bit precision fixed-point constants.
// Go: image/jpeg/dct.go:cos1 etc.
const COS1: u64 = 1130768441178740757; // fix cos 1*pi/16
const SIN1: u64 = 224923827593068887; // fix sin 1*pi/16
const COS3: u64 = 958619196450722178; // fix cos 3*pi/16
const SIN3: u64 = 640528868967736374; // fix sin 3*pi/16
const SQRT2: u64 = 1630477228166597777; // fix sqrt 2
const SQRT2_COS6: u64 = 623956622067911264; // fix (sqrt 2)*cos 6*pi/16
const SQRT2_SIN6: u64 = 1506364539328854985; // fix (sqrt 2)*sin 6*pi/16
const SQRT2INV: u64 = 815238614083298888; // fix 1/sqrt 2
const SQRT2INV_COS6: u64 = 311978311033955632; // fix (1/sqrt 2)*cos 6*pi/16
const SQRT2INV_SIN6: u64 = 753182269664427492; // fix (1/sqrt 2)*sin 6*pi/16

/// c rounds the 60-bit constant x to `bits` bits.
///
/// Go: image/jpeg/dct.go:c
const fn c(x: u64, bits: u32) -> i32 {
    ((x + (1 << (59 - bits))) >> (60 - bits)) as i32
}

/// dctBox implements a 3-multiply, 3-add rotation+scaling.
///
/// Go: image/jpeg/dct.go:dctBox
#[inline(always)]
fn dct_box(x0: i32, x1: i32, kcos: i32, ksin: i32) -> (i32, i32) {
    // y0 = x0*kcos + x1*ksin
    // y1 = -x0*ksin + x1*kcos
    let ksum = kcos.wrapping_mul(x0.wrapping_add(x1));
    let y0 = ksum.wrapping_add(ksin.wrapping_sub(kcos).wrapping_mul(x1));
    let y1 = ksum.wrapping_sub(kcos.wrapping_add(ksin).wrapping_mul(x0));
    (y0, y1)
}

/// fdct implements the forward DCT. Inputs are UQ8.0; outputs are Q13.0.
///
/// Go: image/jpeg/dct.go:fdct
#[inline]
pub(crate) fn fdct(b: &mut Block) {
    fdct_cols(b);
    fdct_rows(b);
}

// butterfly: (a, b) = (a+b, a-b)
#[inline(always)]
fn bf(a: i32, b: i32) -> (i32, i32) {
    (a.wrapping_add(b), a.wrapping_sub(b))
}

/// fdctCols applies the 1D DCT to the columns of b.
/// Inputs are UQ8.0 in [0,255] but interpreted as [-128,127].
/// Outputs are Q10.18.
///
/// Go: image/jpeg/dct.go:fdctCols
#[inline]
fn fdct_cols(b: &mut Block) {
    for i in 0..8 {
        let mut x0 = b[0 * 8 + i];
        let mut x1 = b[1 * 8 + i];
        let mut x2 = b[2 * 8 + i];
        let mut x3 = b[3 * 8 + i];
        let mut x4 = b[4 * 8 + i];
        let mut x5 = b[5 * 8 + i];
        let mut x6 = b[6 * 8 + i];
        let mut x7 = b[7 * 8 + i];

        // Stage 1: four butterflies.
        (x0, x7) = bf(x0, x7);
        (x1, x6) = bf(x1, x6);
        (x2, x5) = bf(x2, x5);
        (x3, x4) = bf(x3, x4);

        // Stage 2: two boxes and two butterflies.
        (x4, x7) = dct_box(x4, x7, c(COS3, 18), c(SIN3, 18));
        (x5, x6) = dct_box(x5, x6, c(COS1, 18), c(SIN1, 18));

        (x0, x3) = bf(x0, x3);
        (x1, x2) = bf(x1, x2);

        // Stage 3: one box and three butterflies.
        (x2, x3) = dct_box(x2, x3, c(SQRT2_COS6, 18), c(SQRT2_SIN6, 18));

        (x0, x1) = bf(x0, x1);

        // Store x0, x1, x2, x3 to their permuted targets.
        b[0 * 8 + i] = x0.wrapping_sub(128 * 8).wrapping_shl(18);
        b[4 * 8 + i] = x1.wrapping_shl(18);
        b[2 * 8 + i] = x2;
        b[6 * 8 + i] = x3;

        (x4, x6) = bf(x4, x6);
        (x7, x5) = bf(x7, x5);

        // Stage 4: two √2 scalings and one butterfly.
        x5 = (x5 >> 12).wrapping_mul(c(SQRT2, 12));
        x6 = (x6 >> 12).wrapping_mul(c(SQRT2, 12));
        (x7, x4) = bf(x7, x4);

        // Store x4 x5 x6 x7 to their permuted targets.
        b[1 * 8 + i] = x7;
        b[3 * 8 + i] = x5;
        b[5 * 8 + i] = x6;
        b[7 * 8 + i] = x4;
    }
}

/// fdctRows applies the 1D DCT to the rows of b.
/// Inputs are Q10.18; outputs are Q13.0.
///
/// Go: image/jpeg/dct.go:fdctRows
#[inline]
fn fdct_rows(b: &mut Block) {
    for i in 0..8 {
        let x = &mut b[8 * i..8 * i + 8];
        let mut x0 = x[0];
        let mut x1 = x[1];
        let mut x2 = x[2];
        let mut x3 = x[3];
        let mut x4 = x[4];
        let mut x5 = x[5];
        let mut x6 = x[6];
        let mut x7 = x[7];

        // Stage 1: four butterflies.
        (x0, x7) = bf(x0, x7);
        (x1, x6) = bf(x1, x6);
        (x2, x5) = bf(x2, x5);
        (x3, x4) = bf(x3, x4);

        // Stage 2: two boxes and two butterflies.
        (x4, x7) = dct_box(x4 >> 14, x7 >> 14, c(COS3, 14), c(SIN3, 14));
        (x5, x6) = dct_box(x5 >> 14, x6 >> 14, c(COS1, 14), c(SIN1, 14));
        (x0, x3) = bf(x0, x3);
        (x1, x2) = bf(x1, x2);

        // Stage 3: one box and three butterflies.
        (x2, x3) = dct_box(x2 >> 14, x3 >> 14, c(SQRT2_COS6, 14), c(SQRT2_SIN6, 14));
        (x0, x1) = bf(x0, x1);
        (x4, x6) = bf(x4, x6);
        (x7, x5) = bf(x7, x5);

        // Stage 4: two √2 scalings and one butterfly.
        x5 = (x5 >> 14).wrapping_mul(c(SQRT2, 14));
        x6 = (x6 >> 14).wrapping_mul(c(SQRT2, 14));
        (x7, x4) = bf(x7, x4);

        // Cut from Q13.18 to Q13.0.
        x0 = x0.wrapping_add(1 << 17) >> 18;
        x1 = x1.wrapping_add(1 << 17) >> 18;
        x2 = x2.wrapping_add(1 << 17) >> 18;
        x3 = x3.wrapping_add(1 << 17) >> 18;
        x4 = x4.wrapping_add(1 << 17) >> 18;
        x5 = x5.wrapping_add(1 << 17) >> 18;
        x6 = x6.wrapping_add(1 << 17) >> 18;
        x7 = x7.wrapping_add(1 << 17) >> 18;

        x[0] = x0;
        x[1] = x7;
        x[2] = x2;
        x[3] = x5;
        x[4] = x1;
        x[5] = x6;
        x[6] = x3;
        x[7] = x4;
    }
}

/// idct implements the inverse DCT. Inputs are UQ8.0; outputs are Q10.3.
///
/// Go: image/jpeg/dct.go:idct
#[inline]
pub(crate) fn idct(b: &mut Block) {
    // A 2D IDCT is a 1D IDCT on rows followed by columns.
    idct_rows(b);
    idct_cols(b);
}

/// idctRows applies the 1D IDCT to the rows of b.
/// Inputs are UQ8.0; outputs are Q9.20.
///
/// Go: image/jpeg/dct.go:idctRows
#[inline]
fn idct_rows(b: &mut Block) {
    for i in 0..8 {
        let x = &mut b[8 * i..8 * i + 8];
        let mut x0 = x[0];
        let mut x7 = x[1];
        let mut x2 = x[2];
        let mut x5 = x[3];
        let mut x1 = x[4];
        let mut x6 = x[5];
        let mut x3 = x[6];
        let mut x4 = x[7];

        // Run FDCT backward.

        // Stages 4, 3, 2: x0, x1, x2, x3.

        x0 = x0.wrapping_shl(17);
        x1 = x1.wrapping_shl(17);
        (x0, x1) = bf(x0, x1);

        // Note: (1/sqrt 2)*((cos 6*pi/16)+(sin 6*pi/16)) < 0.924, so no new high bit.
        (x2, x3) = dct_box(x2, x3, c(SQRT2INV_COS6, 18), -c(SQRT2INV_SIN6, 18));
        (x1, x2) = bf(x1, x2);
        (x0, x3) = bf(x0, x3);

        // Stages 4, 3, 2: x4, x5, x6, x7.

        x4 = x4.wrapping_shl(7);
        x7 = x7.wrapping_shl(7);
        (x7, x4) = bf(x7, x4);

        x6 = x6.wrapping_mul(c(SQRT2INV, 8));
        x5 = x5.wrapping_mul(c(SQRT2INV, 8));

        (x7, x5) = bf(x7, x5);
        (x4, x6) = bf(x4, x6);

        (x4, x7) = dct_box(x4 >> 2, x7 >> 2, c(COS3, 12), -c(SIN3, 12));
        (x5, x6) = dct_box(x5 >> 2, x6 >> 2, c(COS1, 12), -c(SIN1, 12));

        // Stage 1.

        (x0, x7) = bf(x0, x7);
        (x1, x6) = bf(x1, x6);
        (x2, x5) = bf(x2, x5);
        (x3, x4) = bf(x3, x4);

        x[0] = x0;
        x[1] = x1;
        x[2] = x2;
        x[3] = x3;
        x[4] = x4;
        x[5] = x5;
        x[6] = x6;
        x[7] = x7;
    }
}

/// idctCols applies the 1D IDCT to the columns of b.
/// Inputs are Q9.20. Outputs are Q10.3. That is, the result is the IDCT*8.
///
/// Go: image/jpeg/dct.go:idctCols
#[inline]
fn idct_cols(b: &mut Block) {
    for i in 0..8 {
        let mut x0 = b[0 * 8 + i];
        let mut x7 = b[1 * 8 + i];
        let mut x2 = b[2 * 8 + i];
        let mut x5 = b[3 * 8 + i];
        let mut x1 = b[4 * 8 + i];
        let mut x6 = b[5 * 8 + i];
        let mut x3 = b[6 * 8 + i];
        let mut x4 = b[7 * 8 + i];

        // Start by adding 0.5 to x0 (the incoming DC signal).
        // The butterflies will add it to all the other values,
        // and then the final shifts will round properly.
        x0 = x0.wrapping_add(1 << 19);

        // Stages 4, 3, 2: x0, x1, x2, x3.

        (x0, x1) = (x0.wrapping_add(x1) >> 2, x0.wrapping_sub(x1) >> 2);
        // Note: (1/sqrt 2)*((cos 6*pi/16)+(sin 6*pi/16)) < 1, so no new high bit.
        (x2, x3) = dct_box(
            x2 >> 13,
            x3 >> 13,
            c(SQRT2INV_COS6, 12),
            -c(SQRT2INV_SIN6, 12),
        );

        (x1, x2) = bf(x1, x2);
        (x0, x3) = bf(x0, x3);

        // Stages 4, 3, 2: x4, x5, x6, x7.

        (x7, x4) = bf(x7, x4);

        x5 = (x5 >> 13).wrapping_mul(c(SQRT2INV, 14));
        x6 = (x6 >> 13).wrapping_mul(c(SQRT2INV, 14));

        (x7, x5) = bf(x7, x5);
        (x4, x6) = bf(x4, x6);

        (x4, x7) = dct_box(x4 >> 14, x7 >> 14, c(COS3, 12), -c(SIN3, 12));
        (x5, x6) = dct_box(x5 >> 14, x6 >> 14, c(COS1, 12), -c(SIN1, 12));

        (x0, x7) = bf(x0, x7);
        (x1, x6) = bf(x1, x6);
        (x2, x5) = bf(x2, x5);
        (x3, x4) = bf(x3, x4);

        x0 >>= 18;
        x1 >>= 18;
        x2 >>= 18;
        x3 >>= 18;
        x4 >>= 18;
        x5 >>= 18;
        x6 >>= 18;
        x7 >>= 18;

        b[0 * 8 + i] = x0;
        b[1 * 8 + i] = x1;
        b[2 * 8 + i] = x2;
        b[3 * 8 + i] = x3;
        b[4 * 8 + i] = x4;
        b[5 * 8 + i] = x5;
        b[6 * 8 + i] = x6;
        b[7 * 8 + i] = x7;
    }
}

#[cfg(test)]
mod tests {
    //! Differential test against the oracle's `dct` command, which runs a
    //! verbatim copy of go1.27.1's dct.go on random blocks of five
    //! magnitude classes (pixels, sparse/large/arbitrary/small coefficients).
    use super::*;
    use sha2::{Digest, Sha256};

    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
            z ^ (z >> 31)
        }
        fn intn(&mut self, n: i64) -> i64 {
            (self.next() % n as u64) as i64
        }
    }

    fn sha16(b: &[u8]) -> String {
        Sha256::digest(b)
            .iter()
            .map(|x| format!("{:02x}", x))
            .collect::<String>()[..16]
            .to_string()
    }

    #[test]
    fn dct_matches_go() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dct.tsv");
        let data = std::fs::read_to_string(path).unwrap();
        assert_eq!(check(&data), 5000);
    }

    /// Larger corpus kept outside the repository (GO_IMAGE_DCT_BIG=<path to
    /// `go-image dct N` output, optionally with GO_IMAGE_SEED0 set>).
    #[test]
    fn dct_matches_go_big() {
        let Ok(path) = std::env::var("GO_IMAGE_DCT_BIG") else {
            return;
        };
        let data = std::fs::read_to_string(path).unwrap();
        let n = check(&data);
        eprintln!("dct: {} blocks match", n);
    }

    /// Checks every row of an oracle `dct` output; returns the row count.
    fn check(data: &str) -> usize {
        let mut n = 0;
        for line in data.lines() {
            let f: Vec<&str> = line.split('\t').collect();
            let seed: u64 = f[0].parse().unwrap();
            let mut r = Rng(seed.wrapping_mul(101).wrapping_add(7));
            let mut b: Block = [0; BLOCK_SIZE];
            let class = r.intn(5);
            for x in b.iter_mut() {
                *x = match class {
                    0 => r.next() as u8 as i32,
                    1 => {
                        if r.intn(4) == 0 {
                            r.intn(4096) as i32 - 2048
                        } else {
                            0
                        }
                    }
                    2 => r.intn(1 << 24) as i32 - (1 << 23),
                    3 => r.next() as u32 as i32,
                    _ => r.intn(64) as i32 - 32,
                };
            }
            let (mut fb, mut ib) = (b, b);
            fdct(&mut fb);
            idct(&mut ib);
            let fbytes: Vec<u8> = fb.iter().flat_map(|v| (*v as u32).to_le_bytes()).collect();
            let ibytes: Vec<u8> = ib.iter().flat_map(|v| (*v as u32).to_le_bytes()).collect();
            assert_eq!(f[1], class.to_string(), "seed {}", seed);
            assert_eq!(f[2], sha16(&fbytes), "fdct seed {} class {}", seed, class);
            assert_eq!(f[3], sha16(&ibytes), "idct seed {} class {}", seed, class);
            n += 1;
        }
        n
    }
}
