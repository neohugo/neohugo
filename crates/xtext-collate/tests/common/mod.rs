//! Shared helpers: the oracle's splitmix64 corpus generator (bit-exact
//! mirror of tools/go-oracle/xtext-collate/fixtures.go), hex, digests.
#![allow(dead_code)]

use sha2::{Digest, Sha256};
use xtext_collate::{Collator, language};

pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn intn(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    pub fn range_rune(&mut self, lo: u32, hi: u32) -> u32 {
        lo + self.intn((hi - lo + 1) as usize) as u32
    }
}

/// Go utf8.AppendRune (invalid scalar -> U+FFFD).
pub fn append_rune(b: &mut Vec<u8>, r: u32) {
    let c = char::from_u32(r).unwrap_or('\u{FFFD}');
    let mut t = [0u8; 4];
    b.extend_from_slice(c.encode_utf8(&mut t).as_bytes());
}

const ASCII_ALNUM: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
const ASCII_PUNCT: &[u8] = b" -_.,'\"!?()&/:;#@%+*=~^`|<>[]{}\\$";
const DIGIT_ZEROS: &[u32] = &[
    0x30, 0x660, 0x6F0, 0x966, 0x9E6, 0xE50, 0xED0, 0xFF10, 0x1D7CE, 0x2080, 0x1369, 0x1040,
];
const CONTRACTION_SEEDS: &[&str] = &[
    "ch",
    "Ch",
    "CH",
    "ll",
    "dz",
    "dž",
    "ŀl",
    "aa",
    "Aa",
    "ng",
    "ny",
    "sz",
    "cs",
    "gy",
    "lj",
    "nj",
    "rr",
    "ij",
    "ae",
    "oe",
    "ss",
    "th",
    "ǆ",
    "l·l",
    "ch\u{0301}",
    "a\u{030a}",
];

fn gen_item(r: &mut Rng, b: &mut Vec<u8>) {
    let k = r.intn(100);
    if k < 20 {
        b.push(ASCII_ALNUM[r.intn(ASCII_ALNUM.len())]);
    } else if k < 25 {
        b.push(ASCII_PUNCT[r.intn(ASCII_PUNCT.len())]);
    } else if k < 45 {
        let c = r.range_rune(0x0E01, 0x0E5B);
        append_rune(b, c);
    } else if k < 53 {
        let c = r.range_rune(0x00C0, 0x024F);
        append_rune(b, c);
    } else if k < 61 {
        let c = r.range_rune(0x0300, 0x036F);
        append_rune(b, c);
    } else if k < 64 {
        let c = r.range_rune(0xAC00, 0xD7A3);
        append_rune(b, c);
    } else if k < 66 {
        let c = r.range_rune(0x1100, 0x11FF);
        append_rune(b, c);
    } else if k < 68 {
        let c = r.range_rune(0x4E00, 0x9FFF);
        append_rune(b, c);
    } else if k < 70 {
        let c = r.range_rune(0xFF01, 0xFF9F);
        append_rune(b, c);
    } else if k < 73 {
        let z = DIGIT_ZEROS[r.intn(DIGIT_ZEROS.len())];
        let d = r.intn(10) as u32;
        append_rune(b, z + d);
    } else if k < 76 {
        let c = r.range_rune(0x0370, 0x04FF);
        append_rune(b, c);
    } else if k < 79 {
        let c = r.range_rune(0x0590, 0x097F);
        append_rune(b, c);
    } else if k < 82 {
        let c = r.range_rune(0x2000, 0x2BFF);
        append_rune(b, c);
    } else if k < 84 {
        let c = r.range_rune(0x1F300, 0x1FAFF);
        append_rune(b, c);
    } else if k < 87 {
        loop {
            let c = r.range_rune(0, 0x10FFFF);
            if !(0xD800..=0xDFFF).contains(&c) {
                append_rune(b, c);
                return;
            }
        }
    } else if k < 89 {
        b.push((0x80 + r.intn(0x80)) as u8);
    } else if k < 91 {
        let mut t = Vec::new();
        let c = r.range_rune(0x0800, 0xFFFF);
        append_rune(&mut t, c);
        let n = t.len();
        let m = 1 + r.intn(n - 1);
        b.extend_from_slice(&t[..m]);
    } else if k < 94 {
        b.push(b"aeiouyAEOnNcCsSzZ"[r.intn(17)]);
        let n = 1 + r.intn(3);
        for _ in 0..n {
            if r.intn(4) == 0 {
                let c = r.range_rune(0x0E31, 0x0E4E);
                append_rune(b, c);
            } else {
                let c = r.range_rune(0x0300, 0x036F);
                append_rune(b, c);
            }
        }
    } else if k < 96 {
        match r.intn(4) {
            0 => {
                let c = r.intn(0x20) as u8;
                b.push(c)
            }
            1 => b.push(0x7F),
            2 => {
                let c = r.range_rune(0x200B, 0x200F);
                append_rune(b, c)
            }
            _ => append_rune(b, 0xFEFF),
        }
    } else if k < 98 {
        b.extend_from_slice(CONTRACTION_SEEDS[r.intn(CONTRACTION_SEEDS.len())].as_bytes());
    } else {
        let c = match r.intn(4) {
            0 => r.range_rune(0x2150, 0x218F),
            1 => r.range_rune(0x3300, 0x33FF),
            2 => r.range_rune(0xFB00, 0xFB06),
            _ => r.range_rune(0x2460, 0x24FF),
        };
        append_rune(b, c);
    }
}

pub fn gen_string(r: &mut Rng) -> Vec<u8> {
    let n = r.intn(12);
    let mut b = Vec::new();
    for _ in 0..n {
        gen_item(r, &mut b);
    }
    b
}

pub fn gen_corpus(seed: u64, n: usize) -> Vec<Vec<u8>> {
    let mut r = Rng(seed);
    (0..n).map(|_| gen_string(&mut r)).collect()
}

pub fn hex_decode(s: &str) -> Vec<u8> {
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}

pub fn hex_encode(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub fn put_uvarint(h: &mut Sha256, mut v: u64) {
    let mut buf = Vec::new();
    while v >= 0x80 {
        buf.push(v as u8 | 0x80);
        v >>= 7;
    }
    buf.push(v as u8);
    h.update(&buf);
}

pub fn put_varint(h: &mut Sha256, v: i64) {
    let ux = ((v << 1) ^ (v >> 63)) as u64;
    put_uvarint(h, ux);
}

pub fn key_hash(k: &[u8]) -> String {
    let d = Sha256::digest(k);
    hex_encode(&d[..8])
}

pub fn fixtures_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Builds the collator of an oracle config line (`name \t tag \t opts`).
pub fn config_collator(tag: &str, opts: &str) -> Collator {
    let mut o = Vec::new();
    for n in opts.split(',').filter(|s| !s.is_empty()) {
        o.push(match n {
            "loose" => xtext_collate::LOOSE,
            "ignorecase" => xtext_collate::IGNORE_CASE,
            "ignorediacritics" => xtext_collate::IGNORE_DIACRITICS,
            "ignorewidth" => xtext_collate::IGNORE_WIDTH,
            "force" => xtext_collate::FORCE,
            "numeric" => xtext_collate::NUMERIC,
            _ => panic!("unknown option {n}"),
        });
    }
    Collator::from_tag(&language::make(tag), &o)
}

/// The oracle's keyDigest.
pub fn key_digest(c: &mut Collator, strs: &[Vec<u8>]) -> (String, String) {
    let mut hk = Sha256::new();
    let mut hc = Sha256::new();
    for s in strs {
        let k = c.key_vec(s);
        put_uvarint(&mut hk, k.len() as u64);
        hk.update(&k);
    }
    let n = strs.len();
    for i in 0..n {
        let a = &strs[i];
        let b = &strs[(i + 1) % n];
        let d = &strs[(i * 7919 + 13) % n];
        hc.update([
            (c.compare(a, b) + 1) as u8,
            (c.compare(a, d) + 1) as u8,
            (c.compare(d, a) + 1) as u8,
        ]);
    }
    (hex_encode(&hk.finalize()), hex_encode(&hc.finalize()))
}

const STRESS_STARTERS: &[&str] = &[
    "a",
    "A",
    "l",
    "L",
    "c",
    "C",
    "e",
    "o",
    "u",
    "ı",
    "i",
    "z",
    "s",
    "n",
    "y",
    "ร",
    "ก",
    "เ",
    "ཀ",
    "ྐ",
    "क",
    "ल",
    "ل",
    "ا",
    "ㄱ",
    "가",
    "ᄀ",
    "ᅡ",
    "ch",
    "ll",
    "a\u{0308}",
    "\u{0301}",
    "",
    "1",
    "0",
];

const STRESS_MARKS: &[u32] = &[
    0x0300, 0x0301, 0x0302, 0x0308, 0x030A, 0x030C, 0x0327, 0x0328, 0x0323, 0x0331, 0x031B, 0x0345,
    0x0335, 0x05B0, 0x05B4, 0x0E31, 0x0E38, 0x0E39, 0x0E47, 0x0E48, 0x0E49, 0x0E4C, 0x0F71, 0x0F72,
    0x0F74, 0x0F80, 0x1DCE, 0x093C, 0x094D, 0x0B3C, 0x0B4D, 0x3099, 0x309A, 0x0FB5, 0x0FB7, 0x0F90,
    0x0FB3, 0x0F81, 0x0F73, 0x0F75,
];

/// Mirror of the oracle's genStress.
pub fn gen_stress(r: &mut Rng) -> Vec<u8> {
    let mut b = Vec::new();
    let segs = 1 + r.intn(3);
    for _ in 0..segs {
        b.extend_from_slice(STRESS_STARTERS[r.intn(STRESS_STARTERS.len())].as_bytes());
        let m = r.intn(70);
        for _ in 0..m {
            match r.intn(40) {
                0 => {
                    let c = (0x80 + r.intn(0x40)) as u8;
                    b.push(c)
                }
                1 => b.extend_from_slice(STRESS_STARTERS[r.intn(STRESS_STARTERS.len())].as_bytes()),
                _ => {
                    let c = STRESS_MARKS[r.intn(STRESS_MARKS.len())];
                    append_rune(&mut b, c)
                }
            }
        }
    }
    b
}

pub fn gen_stress_corpus(seed: u64, n: usize) -> Vec<Vec<u8>> {
    let mut r = Rng(seed);
    (0..n).map(|_| gen_stress(&mut r)).collect()
}
