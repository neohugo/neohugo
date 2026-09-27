//! Adversarial differential tests: the Rust mirror of
//! tools/go-oracle/go-strconv/adv.go (`oracle -mode advhashes`), checked
//! against tests/fixtures/adv_hashes.txt. Every generator and test body here
//! must stay in lock-step with the Go file.
//!
//! Families (see adv.go for details): exhaustive float32 (`f32all`), every
//! float64 exponent (`f64pow2`), exact decimal ties (`ties`), floats next to
//! short decimals (`nearshort`), exact decimal and hexadecimal midpoints
//! (`midpoints`, `hexmid`), every 0..3-byte string and boundary 4-byte
//! strings through all quoting functions (`quote2/3/4`), full-range escapes
//! (`unqesc`), and exhaustive short / boundary integer strings (`atoiall`,
//! `atoibound`).
//!
//! `adv_hashes` runs everything except the two big exhaustive families,
//! which it samples (11 of 1024 `f32all` chunks, 16 of 256 `quote3` chunks);
//! `adv_hashes_full` (`#[ignore]`, use `--release`) runs all of them.
//! On a mismatch, set GO_STRCONV_DUMP_DIR=<dir> and compare the written
//! dump with `oracle -mode advdump -test <name> -chunk <n>`.

mod common;

use common::*;
use go_strconv as sc;
use go_strconv::internal;

struct AdvDef {
    name: &'static str,
    seed: u64,
    chunks: usize,
    run: fn(&mut Sink, &mut Rng, usize),
}

const ADV_TESTS: &[AdvDef] = &[
    AdvDef {
        name: "f32all",
        seed: 101,
        chunks: 1024,
        run: run_f32all,
    },
    AdvDef {
        name: "f64pow2",
        seed: 102,
        chunks: 32,
        run: run_f64pow2,
    },
    AdvDef {
        name: "ties",
        seed: 103,
        chunks: 64,
        run: run_ties,
    },
    AdvDef {
        name: "nearshort",
        seed: 104,
        chunks: 64,
        run: run_near_short,
    },
    AdvDef {
        name: "midpoints",
        seed: 105,
        chunks: 64,
        run: run_midpoints,
    },
    AdvDef {
        name: "hexmid",
        seed: 106,
        chunks: 32,
        run: run_hex_mid,
    },
    AdvDef {
        name: "quote2",
        seed: 107,
        chunks: 1,
        run: run_quote2,
    },
    AdvDef {
        name: "quote3",
        seed: 108,
        chunks: 256,
        run: run_quote3,
    },
    AdvDef {
        name: "quote4",
        seed: 109,
        chunks: 16,
        run: run_quote4,
    },
    AdvDef {
        name: "unqesc",
        seed: 110,
        chunks: 18,
        run: run_unq_esc,
    },
    AdvDef {
        name: "atoiall",
        seed: 111,
        chunks: 17,
        run: run_atoi_all,
    },
    AdvDef {
        name: "atoibound",
        seed: 112,
        chunks: 4,
        run: run_atoi_bound,
    },
    AdvDef {
        name: "longties",
        seed: 113,
        chunks: 16,
        run: run_long_ties,
    },
    AdvDef {
        name: "fmtbytes",
        seed: 114,
        chunks: 4,
        run: run_fmt_bytes,
    },
    AdvDef {
        name: "complex",
        seed: 115,
        chunks: 8,
        run: run_complex,
    },
    AdvDef {
        name: "wildrunes",
        seed: 116,
        chunks: 16,
        run: run_wild_runes,
    },
];

const FMTS: &[u8] = b"beEfgGxX";

const POW10U: [u64; 20] = {
    let mut t = [1u64; 20];
    let mut i = 1;
    while i < 20 {
        t[i] = t[i - 1] * 10;
        i += 1;
    }
    t
};

/// ParseFloat: value bits (Go returns ±Inf with ErrRange) and error text.
fn parse(s: &mut Sink, st: &[u8], bs: i64) {
    let (v, err) = internal::parse_float(st, bs);
    s.ctx(|| format!("parse {} {}", String::from_utf8_lossy(st), bs));
    s.put_u64(v.to_bits());
    if err.is_none() {
        s.put_str("nil");
    } else {
        s.put_err(sc::parse_float(st, bs).err());
    }
}

// ---------------------------------------------------------------------------
// float formatting

fn run_f32all(s: &mut Sink, _r: &mut Rng, c: usize) {
    let mut b: u32 = (c as u32) << 22;
    loop {
        let f = internal::f32_to_f64(f32::from_bits(b));
        s.ctx(|| format!("value {b:08x}"));
        let e1 = sc::format_float(f, b'e', -1, 32);
        s.put_str(&e1);
        parse(s, e1.as_bytes(), 32);
        s.put_str(sc::format_float(f, b'e', 8, 32));
        s.put_str(sc::format_float(f, b'e', 5, 32));
        if b & ((1 << 22) - 1) == (1 << 22) - 1 {
            break;
        }
        b += 1;
    }
}

const POW2_MANTS: [u64; 7] = [0, 1, 2, 3, 1 << 51, (1 << 52) - 1, (1 << 52) - 2];
const POW2_F_PRECS: [i64; 9] = [0, 1, 2, 3, 5, 10, 17, 20, 30];

fn run_f64pow2(s: &mut Sink, _r: &mut Rng, c: usize) {
    for exp in (c as u64 * 64)..(c as u64 * 64 + 64) {
        for sign in 0u64..2 {
            for mant in POW2_MANTS {
                let b = sign << 63 | exp << 52 | mant;
                let f = f64::from_bits(b);
                s.ctx(|| format!("value {b:016x}"));
                for &fm in FMTS {
                    s.put_str(sc::format_float(f, fm, -1, 64));
                }
                for prec in 0..=20 {
                    s.put_str(sc::format_float(f, b'e', prec, 64));
                    s.put_str(sc::format_float(f, b'g', prec, 64));
                }
                for prec in POW2_F_PRECS {
                    s.put_str(sc::format_float(f, b'f', prec, 64));
                }
                for prec in 0..=14 {
                    s.put_str(sc::format_float(f, b'x', prec, 64));
                }
                s.put_str(sc::format_float(f, b'e', -1, 32));
                s.put_str(sc::format_float(f, b'g', 9, 32));
                s.put_str(sc::format_float(f, b'x', -1, 32));
                let strs = [
                    sc::format_float(f, b'g', -1, 64),
                    sc::format_float(f, b'x', -1, 64),
                    sc::format_float(f, b'e', 16, 64),
                ];
                for st in &strs {
                    parse(s, st.as_bytes(), 64);
                    parse(s, st.as_bytes(), 32);
                }
            }
        }
    }
}

fn run_ties(s: &mut Sink, _r: &mut Rng, c: usize) {
    let k = c as i64 + 1;
    let scale = f64::from_bits(((1023 - k) as u64) << 52); // 2**-k, exact
    let mut n = 1i64;
    while n <= 2001 {
        let f = n as f64 * scale;
        s.ctx(|| format!("tie {n} {k}"));
        for prec in 0..=50 {
            s.put_str(sc::format_float(f, b'e', prec, 64));
        }
        for prec in 0..=k + 2 {
            s.put_str(sc::format_float(f, b'f', prec, 64));
        }
        for prec in 1..=30 {
            s.put_str(sc::format_float(f, b'g', prec, 64));
        }
        for prec in 0..=30 {
            s.put_str(sc::format_float(f, b'e', prec, 32));
        }
        n += 2;
    }
    let q = c % 16;
    for j in 0u64..1000 {
        let n = (10 * j + 5) * POW10U[q];
        let f = n as f64;
        s.ctx(|| format!("inttie {n}"));
        for prec in 0..=20 {
            s.put_str(sc::format_float(f, b'e', prec, 64));
            s.put_str(sc::format_float(f, b'g', prec + 1, 64));
        }
        s.put_str(sc::format_float(f, b'f', 0, 64));
        s.put_str(sc::format_float(f, b'f', 2, 64));
        for prec in 0..=12 {
            s.put_str(sc::format_float(f, b'e', prec, 32));
        }
    }
}

fn near_short_ops(s: &mut Sink, g: f64, bs: i64, nd: i64) {
    let e1 = sc::format_float(g, b'e', -1, bs);
    s.put_str(&e1);
    parse(s, e1.as_bytes(), bs);
    s.put_str(sc::format_float(g, b'e', nd - 1, bs));
    if nd >= 2 {
        s.put_str(sc::format_float(g, b'e', nd - 2, bs));
    }
    s.put_str(sc::format_float(g, b'g', nd, bs));
    s.put_str(sc::format_float(g, b'f', -1, bs));
}

fn run_near_short(s: &mut Sink, r: &mut Rng, _c: usize) {
    for _ in 0..1 << 12 {
        // The inputs are the floats nearest to short decimals. Rust's own
        // (correctly rounded) parser picks them, independently of the code
        // under test; Go uses its ParseFloat, which is also correctly rounded.
        let nd = 1 + r.intn(17);
        let n = POW10U[nd as usize - 1] + r.next() % (9 * POW10U[nd as usize - 1]);
        let q = r.intn(660) - 350;
        let st = format!("{n}e{q}");
        let f: f64 = st.parse().unwrap();
        s.ctx(|| format!("near {st}"));
        let b0 = f.to_bits();
        for d in -2i64..=2 {
            let b = b0.wrapping_add(d as u64);
            s.ctx(|| format!("value {b:016x}"));
            near_short_ops(s, f64::from_bits(b), 64, nd);
        }
        let nd32 = 1 + r.intn(9);
        let n32 = POW10U[nd32 as usize - 1] + r.next() % (9 * POW10U[nd32 as usize - 1]);
        let q32 = r.intn(100) - 55;
        let st32 = format!("{n32}e{q32}");
        let f32v: f32 = st32.parse().unwrap();
        s.ctx(|| format!("near32 {st32}"));
        let b32 = f32v.to_bits();
        for d in -2i32..=2 {
            let b = b32.wrapping_add(d as u32);
            s.ctx(|| format!("value {b:08x}"));
            near_short_ops(s, internal::f32_to_f64(f32::from_bits(b)), 32, nd32);
        }
    }
}

// ---------------------------------------------------------------------------
// parsing: exact midpoints

fn gen_mid64(r: &mut Rng) -> u64 {
    const MASK52: u64 = (1 << 52) - 1;
    match r.intn(8) {
        0 | 1 => {
            let mut b = r.next() & !(1 << 63);
            if (b >> 52) & 0x7ff == 0x7ff {
                b &= !(1 << 62);
            }
            b
        }
        2 => {
            let x = r.next();
            x >> (12 + r.intn(52))
        }
        3 => {
            let ef = r.intn(3) as u64;
            ef << 52 | r.next() & MASK52
        }
        4 => {
            let ef = (2046 - r.intn(2)) as u64;
            ef << 52 | (MASK52 - r.next() % 8)
        }
        5 => (r.intn(2047) as u64) << 52,
        6 => {
            let ef = (1023 - 70 + r.intn(141)) as u64;
            ef << 52 | r.next() & MASK52
        }
        _ => {
            let ef = r.intn(2047) as u64;
            let mants = [
                0,
                1,
                2,
                MASK52,
                MASK52 - 1,
                1 << 51,
                (1 << 51) - 1,
                (1 << 51) + 1,
            ];
            ef << 52 | mants[r.intu(8)]
        }
    }
}

fn gen_mid32(r: &mut Rng) -> u32 {
    const MASK23: u32 = (1 << 23) - 1;
    match r.intn(8) {
        0 | 1 => {
            let mut b = (r.next() as u32) & !(1 << 31);
            if (b >> 23) & 0xff == 0xff {
                b &= !(1 << 30);
            }
            b
        }
        2 => {
            let x = r.next();
            (x >> (41 + r.intn(23))) as u32
        }
        3 => {
            let ef = r.intn(3) as u32;
            ef << 23 | (r.next() as u32) & MASK23
        }
        4 => {
            let ef = (254 - r.intn(2)) as u32;
            ef << 23 | (MASK23 - (r.next() % 8) as u32)
        }
        5 => (r.intn(255) as u32) << 23,
        6 => {
            let ef = (127 - 30 + r.intn(61)) as u32;
            ef << 23 | (r.next() as u32) & MASK23
        }
        _ => {
            let ef = r.intn(255) as u32;
            let mants = [
                0,
                1,
                2,
                MASK23,
                MASK23 - 1,
                1 << 22,
                (1 << 22) - 1,
                (1 << 22) + 1,
            ];
            ef << 23 | mants[r.intu(8)]
        }
    }
}

/// Midpoints x * 2**e2 between a float and its neighbours (see adv.go).
fn float_mids(ef: u64, frac: u64, mb: u32, emin: i64) -> Vec<(u64, i64)> {
    let (m, e) = if ef == 0 {
        (frac, emin)
    } else {
        (frac | 1 << mb, ef as i64 + emin - 1)
    };
    let mut mids = vec![(2 * m + 1, e - 1)];
    if m == 0 {
    } else if frac == 0 && ef > 1 {
        mids.push((4 * m - 1, e - 2));
    } else {
        mids.push((2 * m - 1, e - 1));
    }
    mids
}

/// Multiplies the base-1e9 little-endian number l by k (k < 2^32).
fn mul_small(l: &mut Vec<u64>, k: u64) {
    let mut carry = 0u64;
    for x in l.iter_mut() {
        let v = *x * k + carry;
        *x = v % 1_000_000_000;
        carry = v / 1_000_000_000;
    }
    while carry > 0 {
        l.push(carry % 1_000_000_000);
        carry /= 1_000_000_000;
    }
}

/// Digits D and fraction-digit count of x * 2**e2 (value = D / 10**frac).
/// The test's own exact arithmetic (Go uses math/big).
fn exact_decimal(x: u64, e2: i64) -> (String, i64) {
    let mut l = vec![
        x % 1_000_000_000,
        (x / 1_000_000_000) % 1_000_000_000,
        x / 1_000_000_000_000_000_000,
    ];
    let frac = if e2 >= 0 {
        let mut k = e2;
        while k > 0 {
            let step = k.min(30);
            mul_small(&mut l, 1 << step);
            k -= step;
        }
        0
    } else {
        let mut k = -e2;
        while k > 0 {
            let step = k.min(13);
            mul_small(&mut l, 5u64.pow(step as u32));
            k -= step;
        }
        -e2
    };
    while l.len() > 1 && *l.last().unwrap() == 0 {
        l.pop();
    }
    let mut out = l.last().unwrap().to_string();
    for x in l.iter().rev().skip(1) {
        out.push_str(&format!("{x:09}"));
    }
    (out, frac)
}

fn adv_plain(d: &str, p: i64) -> String {
    let n = d.len() as i64;
    if p <= 0 {
        format!("0.{}{}", "0".repeat((-p) as usize), d)
    } else if p >= n {
        format!("{}{}", d, "0".repeat((p - n) as usize))
    } else {
        format!("{}.{}", &d[..p as usize], &d[p as usize..])
    }
}

fn adv_sci(d: &str, p: i64, k: usize, ec: &str) -> String {
    let k = k.min(d.len());
    let m = &d[..k];
    let mut b = String::new();
    b.push_str(&m[..1]);
    if m.len() > 1 {
        b.push('.');
        b.push_str(&m[1..]);
    }
    b.push_str(ec);
    b.push_str(&(p - 1).to_string());
    b
}

fn adv_underscored(d: &str, p: i64) -> String {
    let d = d.as_bytes();
    let mut b = String::new();
    b.push(d[0] as char);
    if d.len() > 1 {
        b.push('.');
        for i in 1..d.len() {
            b.push(d[i] as char);
            if i % 4 == 0 && i + 1 < d.len() {
                b.push('_');
            }
        }
    }
    b.push('e');
    b.push_str(&(p - 1).to_string());
    b
}

fn adv_decrement(d: &str) -> String {
    let mut b = d.as_bytes().to_vec();
    for i in (0..b.len()).rev() {
        if b[i] > b'0' {
            b[i] -= 1;
            break;
        }
        b[i] = b'9';
    }
    String::from_utf8(b).unwrap()
}

fn mid_forms(d: &str, frac: i64) -> Vec<String> {
    let n = d.len() as i64;
    let p = n - frac;
    let plain = adv_plain(d, p);
    let up = if frac == 0 {
        format!("{plain}.000001")
    } else {
        format!("{plain}000001")
    };
    let down = adv_plain(&(adv_decrement(d) + "999999"), p);
    let mut forms = vec![plain.clone(), adv_sci(d, p, n as usize, "e"), up, down];
    for k in [17, 19, 20, 25] {
        if k < n {
            forms.push(adv_sci(d, p, k as usize, "e"));
        }
    }
    let mut padded = format!("+000{plain}");
    if frac > 0 {
        padded.push_str("000");
    }
    forms.push(adv_underscored(d, p));
    forms.push(padded);
    forms.push(format!("-{}", adv_sci(d, p, n as usize, "E")));
    if p >= 1 {
        forms.push(adv_sci(d, p, n as usize, "E+"));
    }
    forms
}

fn run_midpoints(s: &mut Sink, r: &mut Rng, _c: usize) {
    let emit = |s: &mut Sink, mids: Vec<(u64, i64)>| {
        for (x, e2) in mids {
            let (d, frac) = exact_decimal(x, e2);
            s.ctx(|| format!("mid {x} {e2}"));
            for st in mid_forms(&d, frac) {
                parse(s, st.as_bytes(), 64);
                parse(s, st.as_bytes(), 32);
            }
        }
    };
    for _ in 0..256 {
        let b = gen_mid64(r);
        s.ctx(|| format!("value {b:016x}"));
        emit(
            s,
            float_mids((b >> 52) & 0x7ff, b & ((1 << 52) - 1), 52, -1074),
        );
    }
    for _ in 0..128 {
        let b = gen_mid32(r);
        s.ctx(|| format!("value32 {b:08x}"));
        emit(
            s,
            float_mids(
                ((b >> 23) & 0xff) as u64,
                (b & ((1 << 23) - 1)) as u64,
                23,
                -149,
            ),
        );
    }
}

// ---------------------------------------------------------------------------
// parsing: hexadecimal midpoints

fn adv_underscored_hex(h: &str) -> String {
    let h = h.as_bytes();
    let mut b = String::new();
    for i in 0..h.len() {
        b.push(h[i] as char);
        if i % 2 == 1 && i + 1 < h.len() {
            b.push('_');
        }
    }
    b
}

fn hex_forms(x: u64, e2: i64) -> Vec<String> {
    let h = format!("{x:x}");
    let pm = |e: i64| format!("p{e}");
    let mut forms = vec![
        format!("0x{h}{}", pm(e2)),
        format!("0x{h}000{}", pm(e2 - 12)),
        format!("0x{h}0000001{}", pm(e2 - 28)),
        format!("0x{:x}{}", x * 16 - 1, pm(e2 - 4)),
    ];
    if h.len() > 1 {
        forms.push(format!(
            "0x{}.{}{}",
            &h[..1],
            &h[1..],
            pm(e2 + 4 * (h.len() as i64 - 1))
        ));
    } else {
        forms.push(format!("0x{h}.{}", pm(e2)));
    }
    let exp = if e2 >= 0 {
        format!("+{e2}")
    } else {
        e2.to_string()
    };
    forms.push(format!("0x_{}{}", adv_underscored_hex(&h), pm(e2)));
    forms.push(format!("-0X{}P{}", h.to_ascii_uppercase(), exp));
    forms.push(format!("0x0.00{h}{}", pm(e2 + 4 * (2 + h.len() as i64))));
    forms.push(format!("0x{h}{}", pm(e2 - 5000)));
    forms.push(format!("0x{h}{}", pm(e2 + 5000)));
    forms
}

fn run_hex_mid(s: &mut Sink, r: &mut Rng, _c: usize) {
    let emit = |s: &mut Sink, mids: Vec<(u64, i64)>| {
        for (x, e2) in mids {
            s.ctx(|| format!("mid {x} {e2}"));
            for st in hex_forms(x, e2) {
                parse(s, st.as_bytes(), 64);
                parse(s, st.as_bytes(), 32);
            }
        }
    };
    for _ in 0..1024 {
        let b = gen_mid64(r);
        s.ctx(|| format!("value {b:016x}"));
        emit(
            s,
            float_mids((b >> 52) & 0x7ff, b & ((1 << 52) - 1), 52, -1074),
        );
        let b32 = gen_mid32(r);
        s.ctx(|| format!("value32 {b32:08x}"));
        emit(
            s,
            float_mids(
                ((b32 >> 23) & 0xff) as u64,
                (b32 & ((1 << 23) - 1)) as u64,
                23,
                -149,
            ),
        );
    }
}

// ---------------------------------------------------------------------------
// quoting

fn run_quote2(s: &mut Sink, _r: &mut Rng, _c: usize) {
    quote_ops(s, b"");
    for a in 0..=255u8 {
        quote_ops(s, &[a]);
    }
    for a in 0..=255u8 {
        for b in 0..=255u8 {
            quote_ops(s, &[a, b]);
        }
    }
}

fn run_quote3(s: &mut Sink, _r: &mut Rng, c: usize) {
    for b in 0..=255u8 {
        for d in 0..=255u8 {
            quote_ops(s, &[c as u8, b, d]);
        }
    }
}

const QUOTE4_SET: [u8; 16] = [
    0x00, 0x0a, 0x22, 0x27, 0x30, 0x5c, 0x60, 0x78, 0x7f, 0x80, 0x8f, 0x90, 0x9f, 0xa0, 0xbf, 0xc0,
];

fn run_quote4(s: &mut Sink, _r: &mut Rng, c: usize) {
    for a in c * 16..c * 16 + 16 {
        for b in QUOTE4_SET {
            for d in QUOTE4_SET {
                for e in QUOTE4_SET {
                    quote_ops(s, &[a as u8, b, d, e]);
                }
            }
        }
    }
}

fn unq(s: &mut Sink, st: &[u8]) {
    let u = sc::unquote(st);
    s.ctx(|| format!("unquote {}", hex(st)));
    s.put_str(u.as_deref().unwrap_or(b""));
    s.put_err(u.as_ref().err());
}

fn run_unq_esc(s: &mut Sink, _r: &mut Rng, c: usize) {
    if c < 17 {
        for v in (c << 16) as u32..((c + 1) << 16) as u32 {
            unq(s, format!(r#""\U{v:08x}""#).as_bytes());
            unq(s, format!(r#"'\U{v:08X}'"#).as_bytes());
            unq(s, format!(r#""a\U{v:08x}b""#).as_bytes());
            if v < 0x10000 {
                unq(s, format!(r#""\u{v:04x}""#).as_bytes());
                unq(s, format!(r#"'\u{v:04X}'"#).as_bytes());
                unq(s, format!(r#"`\u{v:04x}`"#).as_bytes());
            }
            let inp = format!(r#"\U{v:08x}z"#);
            match sc::unquote_char(inp.as_bytes(), b'"') {
                Ok((val, mb, tail)) => {
                    s.put_u64(val as i64 as u64);
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
        return;
    }
    for v in 0..=255u8 {
        let b = [v];
        let cat = |parts: &[&[u8]]| parts.concat();
        unq(s, format!(r#""\x{v:02x}""#).as_bytes());
        unq(s, format!(r#""\x{v:02X}""#).as_bytes());
        unq(s, format!(r#"'\x{v:02x}'"#).as_bytes());
        unq(s, &cat(&[b"\"\\", &b, b"\""]));
        unq(s, &cat(&[b"'\\", &b, b"'"]));
        unq(s, &cat(&[b"\"\\", &b, b"00\""]));
        unq(s, &cat(&[b"\"\\x", &b, b"0\""]));
        unq(s, &cat(&[b"\"\\u00", &b, b"0\""]));
    }
    for v in 0..512u32 {
        unq(s, format!(r#""\{v:03o}""#).as_bytes());
        unq(s, format!(r#"'\{v:03o}'"#).as_bytes());
    }
}

// ---------------------------------------------------------------------------
// integers

const ATOI_ALPHABET: &[u8; 16] = b"01789abfxXoO_+-z";
const ATOI_BASES: [i64; 11] = [-1, 0, 1, 2, 3, 8, 10, 16, 35, 36, 37];
const ATOI_BITS: [i64; 11] = [-1, 0, 1, 7, 8, 9, 16, 32, 63, 64, 65];

fn put_int(s: &mut Sink, v: (i64, Option<internal::Error>), pubr: impl FnOnce() -> String) {
    s.put_u64(v.0 as u64);
    if v.1.is_none() {
        s.put_str("nil");
    } else {
        s.put_str(pubr());
    }
}

fn put_uint(s: &mut Sink, v: (u64, Option<internal::Error>), pubr: impl FnOnce() -> String) {
    s.put_u64(v.0);
    if v.1.is_none() {
        s.put_str("nil");
    } else {
        s.put_str(pubr());
    }
}

fn atoi_ops(s: &mut Sink, st: &[u8]) {
    s.ctx(|| format!("input {:?}", String::from_utf8_lossy(st)));
    for base in ATOI_BASES {
        for bits in ATOI_BITS {
            put_int(s, internal::parse_int(st, base, bits), || {
                sc::parse_int(st, base, bits).unwrap_err().to_string()
            });
            put_uint(s, internal::parse_uint(st, base, bits), || {
                sc::parse_uint(st, base, bits).unwrap_err().to_string()
            });
        }
    }
    put_int(s, internal::atoi(st), || {
        sc::atoi(st).unwrap_err().to_string()
    });
    let (b, err) = internal::parse_bool(st);
    s.put_bool(b);
    if err.is_none() {
        s.put_str("nil");
    } else {
        s.put_str(sc::parse_bool(st).unwrap_err().to_string());
    }
}

fn run_atoi_all(s: &mut Sink, _r: &mut Rng, c: usize) {
    let a = ATOI_ALPHABET;
    if c < 16 {
        for i in 0..16 * 16 * 16 {
            let st = [a[c], a[i >> 8], a[(i >> 4) & 15], a[i & 15]];
            atoi_ops(s, &st);
        }
        return;
    }
    atoi_ops(s, b"");
    for n in 1..=3usize {
        let total = 1usize << (4 * n);
        for i in 0..total {
            let st: Vec<u8> = (0..n).map(|j| a[(i >> (4 * (n - 1 - j))) & 15]).collect();
            atoi_ops(s, &st);
        }
    }
}

/// big.Int.Text(base) for an i128.
fn i128_text(v: i128, base: u32) -> String {
    let digits = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let neg = v < 0;
    let mut u = v.unsigned_abs();
    let mut out = Vec::new();
    loop {
        out.push(digits[(u % base as u128) as usize]);
        u /= base as u128;
        if u == 0 {
            break;
        }
    }
    if neg {
        out.push(b'-');
    }
    out.reverse();
    String::from_utf8(out).unwrap()
}

const BOUND_BASES: [u32; 5] = [2, 8, 10, 16, 36];

fn run_atoi_bound(s: &mut Sink, _r: &mut Rng, c: usize) {
    for bits in (c as i64 * 16 + 1)..=(c as i64 * 16 + 16) {
        for sh in [bits - 1, bits] {
            for d in -2i128..=2 {
                for neg in [false, true] {
                    let mut v = (1i128 << sh) + d;
                    if neg {
                        v = -v;
                    }
                    for base in BOUND_BASES {
                        let mut digits = i128_text(v, base);
                        let mut sign = "";
                        if digits.starts_with('-') {
                            sign = "-";
                            digits.remove(0);
                        }
                        let prefix = match base {
                            2 => "0b",
                            8 => "0o",
                            16 => "0x",
                            _ => "",
                        };
                        let mut strs = vec![format!("{sign}{digits}")];
                        if !prefix.is_empty() || base == 10 {
                            let us = adv_underscored_hex(&digits);
                            strs.push(format!("{sign}{prefix}{digits}"));
                            strs.push(format!("{sign}{prefix}_{us}"));
                            strs.push(format!("{sign}{prefix}{us}_"));
                        }
                        for st in &strs {
                            let st = st.as_bytes();
                            s.ctx(|| {
                                format!(
                                    "bound {} base {base} bits {bits}",
                                    String::from_utf8_lossy(st)
                                )
                            });
                            for b in [base as i64, 0] {
                                for bs in [bits, 64, 0] {
                                    put_int(s, internal::parse_int(st, b, bs), || {
                                        sc::parse_int(st, b, bs).unwrap_err().to_string()
                                    });
                                    put_uint(s, internal::parse_uint(st, b, bs), || {
                                        sc::parse_uint(st, b, bs).unwrap_err().to_string()
                                    });
                                }
                            }
                            put_int(s, internal::atoi(st), || {
                                sc::atoi(st).unwrap_err().to_string()
                            });
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// round 2

fn long_tie_forms(d: &str, frac: i64) -> Vec<String> {
    let p = d.len() as i64 - frac;
    let plain = adv_plain(d, p);
    let dot = if frac == 0 { "." } else { "" };
    let z820 = "0".repeat(820);
    vec![
        format!("{plain}{dot}{z820}1"),
        format!("{plain}{dot}{z820}"),
        adv_plain(&(adv_decrement(d) + &"9".repeat(820)), p),
        format!("{}{plain}{dot}1", "0".repeat(790)),
        format!("-{plain}{dot}{z820}1"),
        format!("{}.{}{}1e{}", &d[..1], &d[1..], "0".repeat(850), p - 1),
    ]
}

fn run_long_ties(s: &mut Sink, r: &mut Rng, _c: usize) {
    let emit = |s: &mut Sink, mids: Vec<(u64, i64)>| {
        for (x, e2) in mids {
            let (d, frac) = exact_decimal(x, e2);
            s.ctx(|| format!("mid {x} {e2}"));
            for st in long_tie_forms(&d, frac) {
                parse(s, st.as_bytes(), 64);
                parse(s, st.as_bytes(), 32);
            }
        }
    };
    for _ in 0..64 {
        let b = gen_mid64(r);
        s.ctx(|| format!("value {b:016x}"));
        emit(
            s,
            float_mids((b >> 52) & 0x7ff, b & ((1 << 52) - 1), 52, -1074),
        );
        let b32 = gen_mid32(r);
        s.ctx(|| format!("value32 {b32:08x}"));
        emit(
            s,
            float_mids(
                ((b32 >> 23) & 0xff) as u64,
                (b32 & ((1 << 23) - 1)) as u64,
                23,
                -149,
            ),
        );
    }
}

const FMT_BYTES_PRECS: [i64; 10] = [-5, -1, 0, 1, 5, 17, 18, 30, 400, 1100];

fn run_fmt_bytes(s: &mut Sink, r: &mut Rng, c: usize) {
    for fm in c * 64..c * 64 + 64 {
        for _ in 0..32 {
            let f = f64::from_bits(gen_f64(r));
            for prec in FMT_BYTES_PRECS {
                for bs in [64, 32] {
                    s.ctx(|| format!("fmt {fm} value {:016x} prec {prec} bs {bs}", f.to_bits()));
                    let mut dst = b"x".to_vec();
                    sc::append_float(&mut dst, f, fm as u8, prec, bs);
                    if fm < 0x80 {
                        // FormatFloat must agree with AppendFloat for ASCII fmt bytes.
                        assert_eq!(
                            sc::format_float(f, fm as u8, prec, bs).as_bytes(),
                            &dst[1..]
                        );
                    }
                    s.put(&dst);
                }
            }
        }
    }
}

fn run_complex(s: &mut Sink, r: &mut Rng, _c: usize) {
    for _ in 0..1 << 11 {
        let re = f64::from_bits(gen_f64(r));
        let im = f64::from_bits(gen_f64(r));
        s.ctx(|| format!("complex {:016x} {:016x}", re.to_bits(), im.to_bits()));
        for &fm in FMTS {
            for prec in [-1, 0, 3, 17] {
                for bs in [128, 64] {
                    let st = sc::format_complex((re, im), fm, prec, bs);
                    s.put_str(&st);
                    if prec == -1 && fm != b'b' {
                        let (p, err) = internal::parse_complex(st.as_bytes(), bs);
                        s.put_u64(p.0.to_bits());
                        s.put_u64(p.1.to_bits());
                        if err.is_none() {
                            s.put_str("nil");
                        } else {
                            s.put_err(sc::parse_complex(&st, bs).err());
                        }
                    }
                }
            }
        }
    }
}

const WILD_LOWS: [u32; 24] = [
    0x0000, 0x0020, 0x0041, 0x007e, 0x007f, 0x00a0, 0x00ad, 0x0378, 0x1680, 0x2000, 0x200a, 0x2028,
    0x202f, 0x205f, 0x3000, 0xd800, 0xdfff, 0xe000, 0xf8ff, 0xfeff, 0xfffd, 0xffff, 0x0085, 0x00ff,
];

fn run_wild_runes(s: &mut Sink, _r: &mut Rng, c: usize) {
    for hi in (c * 4096) as u32..(c * 4096 + 4096) as u32 {
        for lo in WILD_LOWS {
            let x = (hi << 16 | lo) as i32;
            s.ctx(|| format!("rune {x}"));
            s.put_bool(sc::is_print(x));
            s.put_bool(sc::is_graphic(x));
            s.put_str(sc::quote_rune(x));
            s.put_str(sc::quote_rune_to_ascii(x));
            s.put_str(sc::quote_rune_to_graphic(x));
        }
    }
}

// ---------------------------------------------------------------------------
// driver

fn run_adv_chunk(t: &AdvDef, c: usize, dump: bool) -> (u64, Option<String>) {
    let mut s = Sink::new(dump);
    let mut r = chunk_rng(t.seed, c);
    (t.run)(&mut s, &mut r, c);
    if let Some(d) = &mut s.dump {
        use std::fmt::Write as _;
        writeln!(d, "hash {:016x}", s.h).unwrap();
    }
    (s.h, s.dump)
}

fn check(filter: impl Fn(&str, usize) -> bool) {
    let expected = expected_hashes(&fixture("adv_hashes.txt"));
    let want: usize = ADV_TESTS.iter().map(|t| t.chunks).sum();
    assert_eq!(want, expected.len(), "fixture/test definition mismatch");
    let mut jobs: Vec<(usize, usize, u64)> = Vec::new();
    for (ti, t) in ADV_TESTS.iter().enumerate() {
        for c in 0..t.chunks {
            if filter(t.name, c) {
                let h = *expected
                    .get(&(t.name.to_string(), c))
                    .unwrap_or_else(|| panic!("missing {} {c}", t.name));
                jobs.push((ti, c, h));
            }
        }
    }
    // Longest chunks first for better load balance.
    jobs.sort_by_key(|&(ti, c, _)| (ti != 0, ti, c));
    let next = std::sync::atomic::AtomicUsize::new(0);
    let failures = std::sync::Mutex::new(Vec::new());
    let nthreads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    std::thread::scope(|scope| {
        for _ in 0..nthreads {
            scope.spawn(|| {
                loop {
                    let j = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if j >= jobs.len() {
                        break;
                    }
                    let (ti, c, want) = jobs[j];
                    let (h, _) = run_adv_chunk(&ADV_TESTS[ti], c, false);
                    if h != want {
                        failures
                            .lock()
                            .unwrap()
                            .push((ADV_TESTS[ti].name, c, h, want));
                    }
                }
            });
        }
    });
    let mut failures = failures.into_inner().unwrap();
    failures.sort();
    if !failures.is_empty() {
        if let Ok(dir) = std::env::var("GO_STRCONV_DUMP_DIR") {
            for &(name, c, _, _) in failures.iter().take(5) {
                let t = ADV_TESTS.iter().find(|t| t.name == name).unwrap();
                let (_, d) = run_adv_chunk(t, c, true);
                let p = format!("{dir}/rust-adv-{name}-{c}.txt");
                std::fs::write(&p, d.unwrap()).unwrap();
                eprintln!("wrote {p}");
            }
        }
        for f in &failures {
            eprintln!(
                "MISMATCH test={} chunk={} got={:016x} want={:016x}",
                f.0, f.1, f.2, f.3
            );
        }
        panic!("{} of {} chunks mismatched", failures.len(), jobs.len());
    }
    eprintln!("all {} adversarial chunks match", jobs.len());
}

/// First bytes of the sampled `quote3` chunks: ASCII specials, the UTF-8
/// lead-byte boundaries and invalid lead bytes.
const QUOTE3_SAMPLE: [usize; 16] = [
    0x00, 0x22, 0x27, 0x5c, 0x60, 0x7f, 0x80, 0xc2, 0xdf, 0xe0, 0xed, 0xef, 0xf0, 0xf4, 0xf5, 0xff,
];

#[test]
fn adv_hashes() {
    check(|name, c| match name {
        "f32all" => c % 128 == 0 || c == 511 || c == 512 || c == 1023,
        "quote3" => QUOTE3_SAMPLE.contains(&c),
        _ => true,
    });
}

/// Everything, including all 2^32 float32 values and all 2^24 3-byte
/// strings. Run with `cargo test --release --test adv -- --ignored`.
#[test]
#[ignore]
fn adv_hashes_full() {
    check(|_, _| true);
}

/// Only the named family: GO_STRCONV_ADV=<name> cargo test --release --test adv adv_hashes_one -- --ignored
#[test]
#[ignore]
fn adv_hashes_one() {
    let name = std::env::var("GO_STRCONV_ADV").expect("set GO_STRCONV_ADV=<family>");
    check(|n, _| n == name);
}

/// Go: `strconv.AppendFloat(nil, 1, 0xff, -1, 64)` is `"%\xff"` (bytes 25 ff),
/// while NaN/Inf ignore fmt. `format_float` cannot return invalid UTF-8.
#[test]
fn non_ascii_fmt_byte() {
    let mut v = Vec::new();
    sc::append_float(&mut v, 1.0, 0xff, -1, 64);
    assert_eq!(v, b"%\xff");
    let mut v = Vec::new();
    sc::append_float(&mut v, 1.0, 0x80, 3, 32);
    assert_eq!(v, b"%\x80");
    assert_eq!(sc::format_float(f64::NAN, 0xff, -1, 64), "NaN");
    assert_eq!(sc::format_float(f64::NEG_INFINITY, 0xff, 2, 32), "-Inf");
    assert!(std::panic::catch_unwind(|| sc::format_float(1.0, 0xff, -1, 64)).is_err());
}
