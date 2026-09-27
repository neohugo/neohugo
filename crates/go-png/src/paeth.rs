//! Port of go1.27.1 `image/png/paeth.go`.

// Go: image/png/paeth.go:intSize — Go `int` is 64-bit on the golden platform.
const INT_SIZE: u32 = 64;

// Go: image/png/paeth.go:abs
pub(crate) fn abs(x: i64) -> i64 {
    // m := -1 if x < 0. m := 0 otherwise.
    let m = x >> (INT_SIZE - 1);

    // In two's complement representation, the negative number
    // of any number (except the smallest one) can be computed
    // by flipping all the bits and add 1. This is faster than
    // code with a branch.
    // See Hacker's Delight, section 2-4.
    (x ^ m).wrapping_sub(m)
}

/// paeth implements the Paeth filter function, as per the PNG specification.
///
/// Go: image/png/paeth.go:paeth
pub(crate) fn paeth(a: u8, b: u8, c: u8) -> u8 {
    // This is an optimized version of the sample code in the PNG spec.
    // For example, the sample code starts with:
    //	p := int(a) + int(b) - int(c)
    //	pa := abs(p - int(a))
    // but the optimized form uses fewer arithmetic operations:
    //	pa := int(b) - int(c)
    //	pa = abs(pa)
    let mut pc = c as i64;
    let mut pa = b as i64 - pc;
    let mut pb = a as i64 - pc;
    pc = abs(pa + pb);
    pa = abs(pa);
    pb = abs(pb);
    if pa <= pb && pa <= pc {
        return a;
    } else if pb <= pc {
        return b;
    }
    c
}

/// filterPaeth applies the Paeth filter to the cdat slice.
/// cdat is the current row's data, pdat is the previous row's data.
///
/// Go: image/png/paeth.go:filterPaeth
pub(crate) fn filter_paeth(cdat: &mut [u8], pdat: &[u8], bytes_per_pixel: usize) {
    let (mut a, mut b, mut c, mut pa, mut pb, mut pc): (i64, i64, i64, i64, i64, i64);
    for i in 0..bytes_per_pixel {
        a = 0;
        c = 0;
        let mut j = i;
        while j < cdat.len() {
            b = pdat[j] as i64;
            pa = b - c;
            pb = a - c;
            pc = abs(pa + pb);
            pa = abs(pa);
            pb = abs(pb);
            if pa <= pb && pa <= pc {
                // No-op.
            } else if pb <= pc {
                a = b;
            } else {
                a = c;
            }
            a += cdat[j] as i64;
            a &= 0xff;
            cdat[j] = a as u8;
            c = b;
            j += bytes_per_pixel;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Go: image/png/paeth_test.go:slowAbs
    fn slow_abs(x: i64) -> i64 {
        if x < 0 { -x } else { x }
    }

    // Go: image/png/paeth_test.go:slowPaeth
    // slowPaeth is a slow but simple implementation of the Paeth function.
    // It is a straight port of the sample code in the PNG spec, section 9.4.
    fn slow_paeth(a: u8, b: u8, c: u8) -> u8 {
        let p = a as i64 + b as i64 - c as i64;
        let pa = slow_abs(p - a as i64);
        let pb = slow_abs(p - b as i64);
        let pc = slow_abs(p - c as i64);
        if pa <= pb && pa <= pc {
            return a;
        } else if pb <= pc {
            return b;
        }
        c
    }

    // Go: image/png/paeth_test.go:slowFilterPaeth
    // slowFilterPaeth is a slow but simple implementation of func filterPaeth.
    fn slow_filter_paeth(cdat: &mut [u8], pdat: &[u8], bytes_per_pixel: usize) {
        for i in 0..bytes_per_pixel {
            cdat[i] = cdat[i].wrapping_add(paeth(0, pdat[i], 0));
        }
        for i in bytes_per_pixel..cdat.len() {
            cdat[i] = cdat[i].wrapping_add(paeth(
                cdat[i - bytes_per_pixel],
                pdat[i],
                pdat[i - bytes_per_pixel],
            ));
        }
    }

    // Go: image/png/paeth_test.go:TestPaeth
    #[test]
    fn test_paeth() {
        let mut a = 0i32;
        while a < 256 {
            let mut b = 0i32;
            while b < 256 {
                let mut c = 0i32;
                while c < 256 {
                    let got = paeth(a as u8, b as u8, c as u8);
                    let want = slow_paeth(a as u8, b as u8, c as u8);
                    assert_eq!(got, want, "a, b, c = {a}, {b}, {c}");
                    c += 15;
                }
                b += 15;
            }
            a += 15;
        }
    }

    // Go: image/png/paeth_test.go:TestPaethDecode
    #[test]
    fn test_paeth_decode() {
        // A deterministic xorshift stand-in for Go's math/rand (the test only
        // compares two implementations on the same random input).
        let mut s: u64 = 0x9e3779b97f4a7c15;
        let mut next = || {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            s as u8
        };
        let mut pdat0 = [0u8; 32];
        let mut cdat0 = [0u8; 32];
        for bytes_per_pixel in 1..=8 {
            for _ in 0..100 {
                for j in 0..pdat0.len() {
                    pdat0[j] = next();
                    cdat0[j] = next();
                }
                let pdat1 = pdat0;
                let pdat2 = pdat0;
                let mut cdat1 = cdat0;
                let mut cdat2 = cdat0;
                filter_paeth(&mut cdat1, &pdat1, bytes_per_pixel);
                slow_filter_paeth(&mut cdat2, &pdat2, bytes_per_pixel);
                assert_eq!(
                    cdat1, cdat2,
                    "bytesPerPixel: {bytes_per_pixel}\npdat0: {pdat0:x?}\ncdat0: {cdat0:x?}"
                );
            }
        }
    }

    #[test]
    fn test_abs() {
        assert_eq!(abs(0), 0);
        assert_eq!(abs(-5), 5);
        assert_eq!(abs(7), 7);
        assert_eq!(abs(i64::MIN), i64::MIN);
    }
}
