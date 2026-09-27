//! Port of go1.27.1 `hash/adler32` (the parts used by `compress/zlib`).

// mod is the largest prime that is less than 65536.
const MOD: u32 = 65521;
// nmax is the largest n such that
// 255 * n * (n+1) / 2 + (n+1) * (mod-1) <= 2^32-1.
// It is mentioned in RFC 1950 (search for "5552").
const NMAX: usize = 5552;

/// The low 16 bits are s1, the high 16 bits are s2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Digest(u32);

impl Default for Digest {
    fn default() -> Self {
        Digest::new()
    }
}

impl Digest {
    // Go: hash/adler32/adler32.go:New
    /// New returns a new hash.Hash32 computing the Adler-32 checksum.
    pub fn new() -> Digest {
        Digest(1)
    }

    // Go: hash/adler32/adler32.go:(*digest).Reset
    pub fn reset(&mut self) {
        self.0 = 1;
    }

    // Go: hash/adler32/adler32.go:(*digest).Write
    pub fn write(&mut self, p: &[u8]) {
        self.0 = update(self.0, p);
    }

    // Go: hash/adler32/adler32.go:(*digest).Sum32
    pub fn sum32(&self) -> u32 {
        self.0
    }
}

// Go: hash/adler32/adler32.go:update
/// Add p to the running checksum d.
fn update(d: u32, mut p: &[u8]) -> u32 {
    let (mut s1, mut s2) = (d & 0xffff, d >> 16);
    while !p.is_empty() {
        let mut q: &[u8] = &[];
        if p.len() > NMAX {
            (p, q) = p.split_at(NMAX);
        }
        while p.len() >= 4 {
            s1 = s1.wrapping_add(p[0] as u32);
            s2 = s2.wrapping_add(s1);
            s1 = s1.wrapping_add(p[1] as u32);
            s2 = s2.wrapping_add(s1);
            s1 = s1.wrapping_add(p[2] as u32);
            s2 = s2.wrapping_add(s1);
            s1 = s1.wrapping_add(p[3] as u32);
            s2 = s2.wrapping_add(s1);
            p = &p[4..];
        }
        for &x in p {
            s1 = s1.wrapping_add(x as u32);
            s2 = s2.wrapping_add(s1);
        }
        s1 %= MOD;
        s2 %= MOD;
        p = q;
    }
    s2 << 16 | s1
}

// Go: hash/adler32/adler32.go:Checksum
/// Checksum returns the Adler-32 checksum of data.
pub fn checksum(data: &[u8]) -> u32 {
    update(1, data)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Go: hash/adler32/adler32_test.go:golden (subset)
    #[test]
    fn golden() {
        let cases: &[(u32, &str)] = &[
            (0x00000001, ""),
            (0x00620062, "a"),
            (0x012600c4, "ab"),
            (0x024d0127, "abc"),
            (0x03d8018b, "abcd"),
            (0x05c801f0, "abcde"),
            (0x081e0256, "abcdef"),
            (0x0adb02bd, "abcdefg"),
            (0x0e000325, "abcdefgh"),
            (0x118e038e, "abcdefghi"),
            (0x158603f8, "abcdefghij"),
            (0x3f090f02, "Discard medicine more than two years old."),
            (
                0x46d81477,
                "He who has a shady past knows that nice guys finish last.",
            ),
            (0x40ee0ee1, "I wouldn't marry him with a ten foot pole."),
            (
                0x16661315,
                "Free! Free!/A trip/to Mars/for 900/empty jars/Burma Shave",
            ),
            (
                0x5b2e1480,
                "The days of the digital watch are numbered.  -Tom Stoppard",
            ),
            (0x8c3c09ea, "Nepal premier won't resign."),
            (
                0x45ac18fd,
                "For every action there is an equal and opposite government program.",
            ),
            (
                0x53c61462,
                "His money is twice tainted: 'taint yours and 'taint mine.",
            ),
            (
                0x7e511e63,
                "There is no reason for any individual to have a computer in their home. -Ken Olsen, 1977",
            ),
            (
                0xe4801a6a,
                "It's a tiny change to the code and not completely disgusting. - Bob Manchek",
            ),
            (0x61b507df, "size:  a.out:  bad magic"),
            (
                0xb8631171,
                "The major problem is with sendmail.  -Mark Horton",
            ),
            (
                0x8b5e1904,
                "Give me a rock, paper and scissors and I will move the world.  CCFestoon",
            ),
            (0x7cc6102b, "If the enemy is within range, then so are you."),
            (
                0x700318e7,
                "It's well we cannot hear the screams/That we create in others' dreams.",
            ),
            (
                0x1e601747,
                "You remind me of a TV show, but that's all right: I watch it anyway.",
            ),
            (0xb55b0b09, "C is as portable as Stonehedge!!"),
            (
                0x39111dd0,
                "Even if I could be Shakespeare, I think I should still choose to be Faraday. - A. Huxley",
            ),
            (
                0x91dd304f,
                "The fugacity of a constituent in a mixture of gases at a given temperature is proportional to its mole fraction.  Lewis-Randall Rule",
            ),
            (
                0x2e5d1316,
                "How can you write a big system without C++?  -Paul Glick",
            ),
            (
                0xd0201df6,
                "'Invariant assertions' is the most elegant programming technique!  -Tom Szymanski",
            ),
        ];
        for &(want, input) in cases {
            assert_eq!(checksum(input.as_bytes()), want, "{input:?}");
            let mut d = Digest::new();
            let (a, b) = input.as_bytes().split_at(input.len() / 2);
            d.write(a);
            d.write(b);
            assert_eq!(d.sum32(), want, "{input:?}");
        }
        let rep = |b: u8, n: usize, tail: &[u8]| {
            let mut v = vec![b; n];
            v.extend_from_slice(tail);
            v
        };
        let long_cases: Vec<(u32, Vec<u8>)> = vec![
            (0x211297c8, rep(0xff, 5548, b"8")),
            (0xbaa198c8, rep(0xff, 5549, b"9")),
            (0x553499be, rep(0xff, 5550, b"0")),
            (0xf0c19abe, rep(0xff, 5551, b"1")),
            (0x8d5c9bbe, rep(0xff, 5552, b"2")),
            (0x2af69cbe, rep(0xff, 5553, b"3")),
            (0xc9809dbe, rep(0xff, 5554, b"4")),
            (0x69189ebe, rep(0xff, 5555, b"5")),
            (0x86af0001, rep(0x00, 100_000, b"")),
            (0x79660b4d, rep(b'a', 100_000, b"")),
            (0x110588ee, b"ABCDEFGHIJKLMNOPQRSTUVWXYZ".repeat(10_000)),
        ];
        for (want, input) in long_cases {
            assert_eq!(checksum(&input), want);
        }
        // Large inputs exercise the nmax split.
        let big: Vec<u8> = (0..100_000u32).map(|i| i.wrapping_mul(i) as u8).collect();
        let mut d = Digest::new();
        for c in big.chunks(777) {
            d.write(c);
        }
        assert_eq!(d.sum32(), checksum(&big));
    }
}
