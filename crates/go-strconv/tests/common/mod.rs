//! Rust mirror of tools/go-oracle/go-strconv/{gen.go,tests.go}.
//! Every generator and test body here must stay in lock-step with the Go
//! oracle; the chunk hashes in tests/fixtures/hashes.txt come from it.

#![allow(dead_code)]
#![allow(clippy::needless_range_loop)]

use go_strconv as sc;
use go_strconv::internal;
use std::fmt::Write as _;

pub fn fixture(name: &str) -> String {
    let p = format!("{}/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{p}: {e}"))
}

pub fn fixture_lines(name: &str) -> Vec<String> {
    fixture(name)
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.to_string())
        .collect()
}

pub fn unhex(s: &str) -> Vec<u8> {
    assert!(s.len().is_multiple_of(2), "bad hex {s}");
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}

pub fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for c in b {
        write!(s, "{c:02x}").unwrap();
    }
    s
}

// ---------------------------------------------------------------------------
// gen.go

/// SplitMix64.
pub struct Rng {
    pub s: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng { s: seed }
    }
    pub fn next(&mut self) -> u64 {
        self.s = self.s.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.s;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn intn(&mut self, n: i64) -> i64 {
        (self.next() % n as u64) as i64
    }
    pub fn intu(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

pub fn chunk_rng(seed: u64, c: usize) -> Rng {
    Rng::new(seed.wrapping_mul(1000003).wrapping_add(c as u64))
}

const POW10U: [u64; 20] = [
    1,
    10,
    100,
    1000,
    10000,
    100000,
    1000000,
    10000000,
    100000000,
    1000000000,
    10000000000,
    100000000000,
    1000000000000,
    10000000000000,
    100000000000000,
    1000000000000000,
    10000000000000000,
    100000000000000000,
    1000000000000000000,
    10000000000000000000,
];

const POW10F: [f64; 23] = [
    1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16,
    1e17, 1e18, 1e19, 1e20, 1e21, 1e22,
];

const POW10F32: [f32; 11] = [1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10];

pub fn gen_f64(r: &mut Rng) -> u64 {
    match r.intn(10) {
        0..=2 => r.next(),
        3 => {
            let sign = r.next() & 1;
            let exp = (1023 - 70 + r.intn(141)) as u64;
            let mant = r.next() & ((1 << 52) - 1);
            sign << 63 | exp << 52 | mant
        }
        4 => {
            let sign = r.next() & 1;
            let sh = r.intn(64) as u32;
            let mant = (r.next() >> sh) & ((1 << 52) - 1);
            let exp = r.intn(3) as u64;
            sign << 63 | exp << 52 | mant
        }
        5 => {
            let k = 1 + r.intu(17);
            let n = r.next() % POW10U[k];
            let e = r.intu(23);
            let mut f = n as f64;
            if r.next() & 1 == 0 {
                f /= POW10F[e];
            } else {
                f *= POW10F[e];
            }
            if r.next() & 1 == 1 {
                f = -f;
            }
            f.to_bits()
        }
        6 => {
            let sign = r.next() & 1;
            let exp = r.intn(2047) as u64;
            let a = r.next();
            let b = r.next();
            let c = r.next();
            let mut mant = a & b & c & ((1 << 52) - 1);
            let sh = r.intn(53) as u32;
            mant &= u64::MAX << sh;
            sign << 63 | exp << 52 | mant
        }
        7 => {
            let sign = r.next() & 1;
            let exp = r.intn(2048) as u64;
            let mants: [u64; 8] = [
                0,
                1,
                2,
                (1 << 52) - 1,
                (1 << 52) - 2,
                1 << 51,
                (1 << 51) - 1,
                (1 << 51) + 1,
            ];
            sign << 63 | exp << 52 | mants[r.intu(8)]
        }
        8 => {
            let sh = r.intn(64) as u32;
            let n = r.next() >> sh;
            let mut f = n as f64;
            if r.next() & 1 == 1 {
                f = -f;
            }
            f.to_bits()
        }
        _ => internal::f32_to_f64(f32::from_bits(r.next() as u32)).to_bits(),
    }
}

pub fn gen_f32(r: &mut Rng) -> u32 {
    match r.intn(8) {
        0..=2 => r.next() as u32,
        3 => {
            let sign = (r.next() & 1) as u32;
            let exp = (127 - 30 + r.intn(61)) as u32;
            let mant = (r.next() as u32) & ((1 << 23) - 1);
            sign << 31 | exp << 23 | mant
        }
        4 => {
            let sign = (r.next() & 1) as u32;
            let sh = r.intn(64) as u32;
            let mant = ((r.next() >> sh) as u32) & ((1 << 23) - 1);
            let exp = r.intn(3) as u32;
            sign << 31 | exp << 23 | mant
        }
        5 => {
            let k = 1 + r.intu(9);
            let n = r.next() % POW10U[k];
            let e = r.intu(11);
            let mut f = n as f32;
            if r.next() & 1 == 0 {
                f /= POW10F32[e];
            } else {
                f *= POW10F32[e];
            }
            if r.next() & 1 == 1 {
                f = -f;
            }
            f.to_bits()
        }
        6 => {
            let sign = (r.next() & 1) as u32;
            let exp = r.intn(255) as u32;
            let a = r.next();
            let b = r.next();
            let c = r.next();
            let mut mant = ((a & b & c) as u32) & ((1 << 23) - 1);
            let sh = r.intn(24) as u32;
            mant &= u32::MAX << sh;
            sign << 31 | exp << 23 | mant
        }
        _ => {
            let sign = (r.next() & 1) as u32;
            let exp = r.intn(256) as u32;
            let mants: [u32; 8] = [
                0,
                1,
                2,
                (1 << 23) - 1,
                (1 << 23) - 2,
                1 << 22,
                (1 << 22) - 1,
                (1 << 22) + 1,
            ];
            sign << 31 | exp << 23 | mants[r.intu(8)]
        }
    }
}

const DEC_DIGITS: &[u8] = b"0123456789";
const HEX_DIGITS: &[u8] = b"0123456789abcdefABCDEF";

fn gen_digits(r: &mut Rng, b: &mut Vec<u8>, n: i64) {
    let mode = r.intn(4);
    for i in 0..n {
        let c = match mode {
            0 => DEC_DIGITS[r.intu(10)],
            1 => {
                if r.intn(8) == 0 {
                    DEC_DIGITS[r.intu(10)]
                } else {
                    b'9'
                }
            }
            2 => {
                if r.intn(8) == 0 {
                    DEC_DIGITS[r.intu(10)]
                } else {
                    b'0'
                }
            }
            _ => {
                if i == 0 || i >= n - 2 {
                    DEC_DIGITS[r.intu(10)]
                } else {
                    b'0'
                }
            }
        };
        b.push(c);
    }
}

fn gen_sign(r: &mut Rng, b: &mut Vec<u8>) {
    match r.intn(3) {
        0 => b.push(b'+'),
        1 => b.push(b'-'),
        _ => {}
    }
}

fn gen_exp(r: &mut Rng, b: &mut Vec<u8>) {
    gen_sign(r, b);
    let n = match r.intn(4) {
        0 => 1,
        1 => 2,
        2 => 3,
        _ => 1 + r.intn(22),
    };
    for _ in 0..n {
        b.push(DEC_DIGITS[r.intu(10)]);
    }
}

const SPECIAL_WORDS: [&str; 10] = [
    "inf",
    "infinity",
    "nan",
    "infin",
    "infinit",
    "infinityx",
    "na",
    "in",
    "i",
    "n",
];

const JUNK_CHARS: &[u8] = b"x.e+-_ pP0i";

fn mutate(r: &mut Rng, mut b: Vec<u8>) -> Vec<u8> {
    if r.intn(10) == 0 {
        let k = 1 + r.intn(3);
        for _ in 0..k {
            let pos = r.intu(b.len() + 1);
            b.insert(pos, b'_');
        }
    }
    if r.intn(20) == 0 {
        let pos = r.intu(b.len() + 1);
        let c = JUNK_CHARS[r.intu(JUNK_CHARS.len())];
        b.insert(pos, c);
    }
    if r.intn(30) == 0 {
        let n = r.intu(b.len() + 1);
        b.truncate(n);
    }
    b
}

pub fn gen_num_string(r: &mut Rng) -> Vec<u8> {
    let mut b = Vec::new();
    match r.intn(20) {
        0 => {
            let w = SPECIAL_WORDS[r.intu(SPECIAL_WORDS.len())].as_bytes();
            gen_sign(r, &mut b);
            for &c0 in w {
                let mut c = c0;
                if r.next() & 1 == 1 {
                    c -= b'a' - b'A';
                }
                b.push(c);
            }
            mutate(r, b)
        }
        1 | 2 => {
            gen_sign(r, &mut b);
            b.push(b'0');
            if r.next() & 1 == 0 {
                b.push(b'x');
            } else {
                b.push(b'X');
            }
            let nd = r.intn(20);
            for _ in 0..nd {
                b.push(HEX_DIGITS[r.intu(HEX_DIGITS.len())]);
            }
            if r.intn(4) != 0 {
                b.push(b'.');
                let nf = r.intn(20);
                for _ in 0..nf {
                    b.push(HEX_DIGITS[r.intu(HEX_DIGITS.len())]);
                }
            }
            if r.intn(8) != 0 {
                if r.next() & 1 == 0 {
                    b.push(b'p');
                } else {
                    b.push(b'P');
                }
                gen_exp(r, &mut b);
            }
            mutate(r, b)
        }
        _ => {
            gen_sign(r, &mut b);
            let nd = if r.intn(50) == 0 {
                700 + r.intn(400)
            } else if r.intn(4) == 0 {
                r.intn(40)
            } else {
                r.intn(20)
            };
            gen_digits(r, &mut b, nd);
            if r.intn(3) != 0 {
                b.push(b'.');
                let n = r.intn(25);
                gen_digits(r, &mut b, n);
            }
            if r.intn(3) != 0 {
                if r.next() & 1 == 0 {
                    b.push(b'e');
                } else {
                    b.push(b'E');
                }
                gen_exp(r, &mut b);
            }
            mutate(r, b)
        }
    }
}

const INT_CHARS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCXYZ";

pub fn gen_int_string(r: &mut Rng) -> Vec<u8> {
    let mut b = Vec::new();
    gen_sign(r, &mut b);
    match r.intn(8) {
        0 => {
            b.push(b'0');
            b.push(b"xX"[r.intu(2)]);
        }
        1 => {
            b.push(b'0');
            b.push(b"bB"[r.intu(2)]);
        }
        2 => {
            b.push(b'0');
            b.push(b"oO"[r.intu(2)]);
        }
        3 => b.push(b'0'),
        _ => {}
    }
    let n = if r.intn(8) == 0 {
        r.intn(70)
    } else {
        r.intn(22)
    };
    let mode = r.intn(4);
    for _ in 0..n {
        let c = match mode {
            0 => DEC_DIGITS[r.intu(10)],
            1 => HEX_DIGITS[r.intu(HEX_DIGITS.len())],
            2 => INT_CHARS[r.intu(INT_CHARS.len())],
            _ => b"01"[r.intu(2)],
        };
        b.push(c);
    }
    mutate(r, b)
}

const ESCAPE_SNIPPETS: [&str; 26] = [
    r"\x41",
    r"\xff",
    r"\x4",
    r"\xg0",
    r"é",
    r"\u12",
    r"\ud800",
    r"\U0001F600",
    r"\U00110000",
    r"\U0010ffff",
    r"\377",
    r"\400",
    r"\12",
    r"\8",
    r"\n",
    r"\t",
    r"\a",
    r"\b",
    r"\f",
    r"\r",
    r"\v",
    r"\\",
    r"\'",
    r#"\""#,
    r"\q",
    r"\",
];

const INTERESTING_RUNES: [i32; 21] = [
    0xa0, 0xad, 0x85, 0x2000, 0x2028, 0x2029, 0x3000, 0xfeff, 0xe000, 0xfffd, 0xffff, 0x10ffff,
    0x1f600, 0x1680, 0x180e, 0x200b, 0x0378, 0x061c, 0xd7ff, 0xe0001, 0x10000,
];

/// Go utf8.AppendRune (reference encoder for the tests).
pub fn append_rune(p: &mut Vec<u8>, r: i32) {
    let i = r as u32;
    if i <= 0x7f {
        p.push(r as u8);
    } else if i <= 0x7ff {
        p.push(0xC0 | (r >> 6) as u8);
        p.push(0x80 | (r as u8) & 0x3F);
    } else if i < 0xD800 || (0xDFFF < i && i <= 0xFFFF) {
        p.push(0xE0 | (r >> 12) as u8);
        p.push(0x80 | ((r >> 6) as u8) & 0x3F);
        p.push(0x80 | (r as u8) & 0x3F);
    } else if i > 0xFFFF && i <= 0x10FFFF {
        p.push(0xF0 | (r >> 18) as u8);
        p.push(0x80 | ((r >> 12) as u8) & 0x3F);
        p.push(0x80 | ((r >> 6) as u8) & 0x3F);
        p.push(0x80 | (r as u8) & 0x3F);
    } else {
        p.extend_from_slice(&[0xEF, 0xBF, 0xBD]);
    }
}

pub fn gen_bytes(r: &mut Rng) -> Vec<u8> {
    let mut b = Vec::new();
    let n = if r.intn(10) == 0 {
        r.intn(200)
    } else {
        r.intn(20)
    };
    for _ in 0..n {
        match r.intn(8) {
            0 | 1 => b.push((32 + r.intn(95)) as u8),
            2 => {
                if r.intn(10) == 0 {
                    b.push(0x7f);
                } else {
                    b.push(r.intn(32) as u8);
                }
            }
            3 => b.push(r.intn(256) as u8),
            4 => {
                let x = r.intn(0x110000) as i32;
                append_rune(&mut b, x);
            }
            5 => b.push(b"\"'`\\"[r.intu(4)]),
            6 => b.extend_from_slice(ESCAPE_SNIPPETS[r.intu(ESCAPE_SNIPPETS.len())].as_bytes()),
            _ => {
                let x = INTERESTING_RUNES[r.intu(INTERESTING_RUNES.len())];
                append_rune(&mut b, x);
            }
        }
    }
    b
}

// ---------------------------------------------------------------------------
// tests.go

pub struct Sink {
    pub h: u64,
    pub dump: Option<String>,
}

impl Sink {
    pub fn new(dump: bool) -> Sink {
        Sink {
            h: 14695981039346656037,
            dump: if dump { Some(String::new()) } else { None },
        }
    }

    #[inline]
    fn byte1(&mut self, c: u8) {
        self.h ^= c as u64;
        self.h = self.h.wrapping_mul(1099511628211);
    }

    pub fn put(&mut self, b: &[u8]) {
        for c in (b.len() as u32).to_le_bytes() {
            self.byte1(c);
        }
        for &c in b {
            self.byte1(c);
        }
        if let Some(d) = &mut self.dump {
            writeln!(d, "  {}", hex(b)).unwrap();
        }
    }

    pub fn put_str(&mut self, s: impl AsRef<[u8]>) {
        self.put(s.as_ref())
    }

    pub fn put_u64(&mut self, v: u64) {
        self.put(&v.to_le_bytes())
    }

    pub fn put_bool(&mut self, v: bool) {
        self.put(&[v as u8])
    }

    pub fn put_err<E: std::fmt::Display>(&mut self, err: Option<E>) {
        match err {
            None => self.put_str("nil"),
            Some(e) => self.put_str(e.to_string()),
        }
    }

    pub fn is_dump(&self) -> bool {
        self.dump.is_some()
    }

    pub fn ctx(&mut self, line: impl FnOnce() -> String) {
        if let Some(d) = &mut self.dump {
            d.push_str(&line());
            d.push('\n');
        }
    }
}

pub struct TestDef {
    pub name: &'static str,
    pub seed: u64,
    pub chunks: usize,
    pub run: fn(&mut Sink, &mut Rng, usize),
    /// run with internal/strconv.optimize = false
    pub slow: bool,
}

pub const CHUNK_N: usize = 1 << 14;

pub const TESTS: &[TestDef] = &[
    TestDef {
        name: "ftoa64",
        seed: 1,
        chunks: 64,
        run: run_ftoa64,
        slow: false,
    },
    TestDef {
        name: "ftoa32",
        seed: 2,
        chunks: 64,
        run: run_ftoa32,
        slow: false,
    },
    TestDef {
        name: "ftoa64as32",
        seed: 3,
        chunks: 4,
        run: run_ftoa64as32,
        slow: false,
    },
    TestDef {
        name: "ftoagrid64",
        seed: 4,
        chunks: 4,
        run: run_grid64,
        slow: false,
    },
    TestDef {
        name: "ftoagrid32",
        seed: 5,
        chunks: 4,
        run: run_grid32,
        slow: false,
    },
    TestDef {
        name: "atofgen",
        seed: 6,
        chunks: 64,
        run: run_atof_gen,
        slow: false,
    },
    TestDef {
        name: "atoigen",
        seed: 7,
        chunks: 32,
        run: run_atoi_gen,
        slow: false,
    },
    TestDef {
        name: "quoteall",
        seed: 8,
        chunks: 0x111,
        run: run_quote_all,
        slow: false,
    },
    TestDef {
        name: "quotegen",
        seed: 9,
        chunks: 32,
        run: run_quote_gen,
        slow: false,
    },
    TestDef {
        name: "nanconv",
        seed: 10,
        chunks: 1,
        run: run_nan_conv,
        slow: false,
    },
    TestDef {
        name: "ftoaslow64",
        seed: 11,
        chunks: 16,
        run: run_ftoa_slow64,
        slow: true,
    },
    TestDef {
        name: "ftoaslow32",
        seed: 12,
        chunks: 8,
        run: run_ftoa_slow32,
        slow: true,
    },
    TestDef {
        name: "atofslow",
        seed: 13,
        chunks: 16,
        run: run_atof_slow,
        slow: true,
    },
];

const FMTS: &[u8] = b"beEfgGxX";

/// Error of a Result as Option for put_err.
fn e<T, E>(r: &Result<T, E>) -> Option<&E> {
    r.as_ref().err()
}

fn ftoa_ops(s: &mut Sink, r: &mut Rng, f: f64, bit_size: i64) {
    for &fm in FMTS {
        let precs: Vec<i64> = match fm {
            b'b' => vec![-1],
            b'e' | b'E' | b'g' | b'G' => {
                let p1 = r.intn(25);
                let p2 = r.intn(18);
                vec![-1, p1, p2]
            }
            b'f' => {
                let p1 = r.intn(30);
                let p2 = r.intn(8);
                vec![-1, p1, p2]
            }
            _ => {
                let p1 = r.intn(18) - 1;
                let p2 = r.intn(16);
                vec![-1, p1, p2]
            }
        };
        for prec in precs {
            s.ctx(|| format!("fmt {} {}", fm as char, prec));
            s.put_str(sc::format_float(f, fm, prec, bit_size));
        }
    }
    // round trips
    let p1 = r.intn(40);
    let p2 = r.intn(20);
    let p3 = r.intn(16) - 1;
    let strs = [
        sc::format_float(f, b'g', -1, bit_size),
        sc::format_float(f, b'e', p1, bit_size),
        sc::format_float(f, b'f', p2, bit_size),
        sc::format_float(f, b'x', p3, bit_size),
    ];
    for st in &strs {
        for bs in [64, 32] {
            // Go returns the value even on error: use the internal API.
            let (v, err) = internal::parse_float(st.as_bytes(), bs);
            s.ctx(|| format!("parse {st} {bs}"));
            s.put_u64(v.to_bits());
            s.put_err(sc::parse_float(st, bs).err());
            debug_assert_eq!(err.is_some(), sc::parse_float(st, bs).is_err());
        }
    }
}

fn run_ftoa64(s: &mut Sink, r: &mut Rng, _c: usize) {
    for _ in 0..CHUNK_N {
        let b = gen_f64(r);
        s.ctx(|| format!("value {b:016x}"));
        ftoa_ops(s, r, f64::from_bits(b), 64);
    }
}

fn run_ftoa32(s: &mut Sink, r: &mut Rng, _c: usize) {
    for _ in 0..CHUNK_N {
        let b = gen_f32(r);
        s.ctx(|| format!("value {b:08x}"));
        ftoa_ops(s, r, internal::f32_to_f64(f32::from_bits(b)), 32);
    }
}

fn run_ftoa64as32(s: &mut Sink, r: &mut Rng, _c: usize) {
    for _ in 0..CHUNK_N {
        let b = gen_f64(r);
        s.ctx(|| format!("value {b:016x}"));
        ftoa_ops(s, r, f64::from_bits(b), 32);
    }
}

pub fn prec_grid() -> Vec<i64> {
    let mut p: Vec<i64> = (-1..=40).collect();
    p.extend_from_slice(&[
        50, 60, 70, 80, 100, 150, 200, 300, 400, 500, 767, 800, 1074, 1100,
    ]);
    p
}

const GRID_N: usize = 1 << 10;

fn grid_ops(s: &mut Sink, f: f64, bit_size: i64, grid: &[i64]) {
    for &fm in FMTS {
        for &prec in grid {
            s.ctx(|| format!("fmt {} {}", fm as char, prec));
            s.put_str(sc::format_float(f, fm, prec, bit_size));
        }
    }
}

fn run_grid64(s: &mut Sink, r: &mut Rng, _c: usize) {
    let grid = prec_grid();
    for _ in 0..GRID_N {
        let b = gen_f64(r);
        s.ctx(|| format!("value {b:016x}"));
        grid_ops(s, f64::from_bits(b), 64, &grid);
    }
}

fn run_grid32(s: &mut Sink, r: &mut Rng, _c: usize) {
    let grid = prec_grid();
    for _ in 0..GRID_N {
        let b = gen_f32(r);
        s.ctx(|| format!("value {b:08x}"));
        grid_ops(s, internal::f32_to_f64(f32::from_bits(b)), 32, &grid);
    }
}

fn run_atof_gen(s: &mut Sink, r: &mut Rng, _c: usize) {
    for i in 0..CHUNK_N {
        let st = gen_num_string(r);
        s.ctx(|| format!("input {}", hex(&st)));
        for bs in [64, 32] {
            let (v, _) = internal::parse_float(&st, bs);
            s.put_u64(v.to_bits());
            s.put_err(e(&sc::parse_float(&st, bs)));
        }
        if i % 4 == 0 {
            // complex
            let st2 = gen_num_string(r);
            let mut c = Vec::new();
            let paren = r.intn(4) == 0;
            if paren {
                c.push(b'(');
            }
            c.extend_from_slice(&st);
            match r.intn(3) {
                0 => c.push(b'+'),
                1 => c.push(b'-'),
                _ => {}
            }
            c.extend_from_slice(&st2);
            if r.intn(5) != 0 {
                c.push(b'i');
            }
            if paren {
                c.push(b')');
            }
            s.ctx(|| format!("complex {}", hex(&c)));
            for bs in [128, 64] {
                let (v, _) = internal::parse_complex(&c, bs);
                s.put_u64(v.0.to_bits());
                s.put_u64(v.1.to_bits());
                s.put_err(e(&sc::parse_complex(&c, bs)));
            }
        }
    }
}

const INT_BASE_BITS: [(i64, i64); 9] = [
    (0, 0),
    (0, 64),
    (10, 0),
    (10, 32),
    (16, 64),
    (2, 64),
    (8, 16),
    (36, 64),
    (0, 8),
];

fn run_atoi_gen(s: &mut Sink, r: &mut Rng, _c: usize) {
    for _ in 0..CHUNK_N {
        let st = gen_int_string(r);
        s.ctx(|| format!("input {}", hex(&st)));
        let rb = r.intn(40) - 1;
        let rbits = r.intn(70) - 2;
        for j in 0..=INT_BASE_BITS.len() {
            let (base, bits) = if j < INT_BASE_BITS.len() {
                INT_BASE_BITS[j]
            } else {
                (rb, rbits)
            };
            let (v, _) = internal::parse_int(&st, base, bits);
            s.put_u64(v as u64);
            s.put_err(e(&sc::parse_int(&st, base, bits)));
            let (u, _) = internal::parse_uint(&st, base, bits);
            s.put_u64(u);
            s.put_err(e(&sc::parse_uint(&st, base, bits)));
        }
        let (v, _) = internal::atoi(&st);
        s.put_u64(v as u64);
        s.put_err(e(&sc::atoi(&st)));
        let (b, _) = internal::parse_bool(&st);
        s.put_bool(b);
        s.put_err(e(&sc::parse_bool(&st)));
        // formatting of a random integer in a random base
        // Go: n := r.next() >> uint(r.intn(64)) -- operands evaluate left to right.
        let a = r.next();
        let sh = r.intn(64) as u32;
        let n = a >> sh;
        let base = 2 + r.intn(35);
        s.put_str(sc::format_int(n as i64, base));
        s.put_str(sc::format_uint(n, base));
        s.put_str(sc::format_int(n as i64, 10));
        s.put_str(sc::itoa((n as i64).wrapping_neg()));
    }
}

pub fn quote_ops(s: &mut Sink, st: &[u8]) {
    s.put_str(sc::quote(st));
    s.put_str(sc::quote_to_ascii(st));
    s.put_str(sc::quote_to_graphic(st));
    s.put_bool(sc::can_backquote(st));
    for q in [&b""[..], b"\"", b"'", b"`"] {
        let mut w = q.to_vec();
        w.extend_from_slice(st);
        w.extend_from_slice(q);
        let u = sc::unquote(&w);
        s.put_str(u.as_deref().unwrap_or(b""));
        s.put_err(e(&u));
        w.extend_from_slice(b"tail");
        let p = sc::quoted_prefix(&w);
        s.put_str(p.unwrap_or(b""));
        s.put_err(e(&p));
    }
    for q in [0u8, b'\'', b'"'] {
        match sc::unquote_char(st, q) {
            Ok((v, mb, tail)) => {
                s.put_u64(v as i64 as u64);
                s.put_bool(mb);
                s.put_u64(tail.len() as u64);
                s.put_err(None::<sc::Error>);
            }
            Err(err) => {
                s.put_u64(0);
                s.put_bool(false);
                s.put_u64(0);
                s.put_err(Some(err));
            }
        }
    }
}

fn run_quote_all(s: &mut Sink, _r: &mut Rng, c: usize) {
    let runes: Vec<i32> = if c < 0x110 {
        ((c << 12) as i32..((c + 1) << 12) as i32).collect()
    } else {
        vec![
            -1,
            -2,
            i32::MIN,
            0x110000,
            0x110001,
            i32::MAX,
            0x7fffffff - 1,
            -0x10000,
        ]
    };
    for x in runes {
        s.ctx(|| format!("rune {x}"));
        s.put_str(sc::quote_rune(x));
        s.put_str(sc::quote_rune_to_ascii(x));
        s.put_str(sc::quote_rune_to_graphic(x));
        s.put_bool(sc::is_print(x));
        s.put_bool(sc::is_graphic(x));
        let mut st = Vec::new();
        append_rune(&mut st, x);
        quote_ops(s, &st);
        let u = sc::unquote(sc::quote_rune(x));
        s.put_str(u.as_deref().unwrap_or(b""));
        s.put_err(e(&u));
        let u = sc::unquote(sc::quote_to_ascii(&st));
        s.put_str(u.as_deref().unwrap_or(b""));
        s.put_err(e(&u));
    }
}

fn run_quote_gen(s: &mut Sink, r: &mut Rng, _c: usize) {
    for _ in 0..CHUNK_N {
        let st = gen_bytes(r);
        s.ctx(|| format!("input {}", hex(&st)));
        quote_ops(s, &st);
    }
}

fn run_nan_conv(s: &mut Sink, r: &mut Rng, _c: usize) {
    for _ in 0..CHUNK_N {
        let sign = r.next() & 1;
        let mut mant = r.next() & ((1 << 52) - 1);
        if mant == 0 {
            mant = 1;
        }
        let f = f64::from_bits(sign << 63 | 0x7ff << 52 | mant);
        let f32v = internal::f64_to_f32(f);
        s.put_u64(f32v.to_bits() as u64);
        s.put_u64(internal::f32_to_f64(f32v).to_bits());
        let mut b32 = r.next() as u32;
        b32 |= 0x7f800000;
        if b32 & ((1 << 23) - 1) == 0 {
            b32 |= 1;
        }
        s.put_u64(internal::f32_to_f64(f32::from_bits(b32)).to_bits());
        // ordinary values f64 -> f32 rounding
        let g = f64::from_bits(gen_f64(r));
        s.put_u64(internal::f64_to_f32(g).to_bits() as u64);
    }
}

const SLOW_N: usize = 1 << 12;

fn slow_ftoa_ops(s: &mut Sink, r: &mut Rng, f: f64, bit_size: i64, grid: &[i64]) {
    for &fm in FMTS {
        if fm == b'b' || fm == b'x' || fm == b'X' {
            continue;
        }
        let p1 = r.intn(25);
        let p2 = r.intn(18);
        let p3 = grid[r.intu(grid.len())];
        for prec in [p1, p2, p3] {
            s.ctx(|| format!("fmt {} {}", fm as char, prec));
            s.put_str(sc::format_float(f, fm, prec, bit_size));
        }
    }
}

fn run_ftoa_slow64(s: &mut Sink, r: &mut Rng, _c: usize) {
    let grid = prec_grid();
    for _ in 0..SLOW_N {
        let b = gen_f64(r);
        s.ctx(|| format!("value {b:016x}"));
        slow_ftoa_ops(s, r, f64::from_bits(b), 64, &grid);
    }
}

fn run_ftoa_slow32(s: &mut Sink, r: &mut Rng, _c: usize) {
    let grid = prec_grid();
    for _ in 0..SLOW_N {
        let b = gen_f32(r);
        s.ctx(|| format!("value {b:08x}"));
        slow_ftoa_ops(s, r, internal::f32_to_f64(f32::from_bits(b)), 32, &grid);
    }
}

fn run_atof_slow(s: &mut Sink, r: &mut Rng, _c: usize) {
    for _ in 0..SLOW_N {
        let st = gen_num_string(r);
        s.ctx(|| format!("input {}", hex(&st)));
        for bs in [64, 32] {
            let (v, _) = internal::parse_float(&st, bs);
            s.put_u64(v.to_bits());
            s.put_err(e(&sc::parse_float(&st, bs)));
        }
        let f = f64::from_bits(gen_f64(r));
        let p = r.intn(30);
        let st2 = sc::format_float(f, b'e', p, 64);
        s.ctx(|| format!("input2 {st2}"));
        for bs in [64, 32] {
            let (v, _) = internal::parse_float(st2.as_bytes(), bs);
            s.put_u64(v.to_bits());
            s.put_err(e(&sc::parse_float(&st2, bs)));
        }
    }
}

/// Runs chunk c of test t and returns its hash (and the dump if requested).
pub fn run_chunk(t: &TestDef, c: usize, dump: bool) -> (u64, Option<String>) {
    let mut s = Sink::new(dump);
    let mut r = chunk_rng(t.seed, c);
    let old = internal::set_optimize(!t.slow);
    (t.run)(&mut s, &mut r, c);
    internal::set_optimize(old);
    if let Some(d) = &mut s.dump {
        writeln!(d, "hash {:016x}", s.h).unwrap();
    }
    (s.h, s.dump)
}

/// Parses an oracle hashes file: (test, chunk) -> hash.
pub fn expected_hashes(data: &str) -> std::collections::HashMap<(String, usize), u64> {
    let mut m = std::collections::HashMap::new();
    for l in data
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let f: Vec<&str> = l.split(' ').collect();
        m.insert(
            (f[0].to_string(), f[1].parse().unwrap()),
            u64::from_str_radix(f[2], 16).unwrap(),
        );
    }
    m
}
