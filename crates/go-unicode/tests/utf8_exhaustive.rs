//! Exhaustive differential test of utf8 decoding against go1.27.1: every
//! 1- and 2-byte sequence, every 3-byte sequence with a non-ASCII first byte,
//! and 4-/5-byte sequences over byte boundary classes. For each sequence
//! DecodeRune, DecodeLastRune, FullRune, Valid, RuneCount and the
//! `for i, r := range s` iteration are folded into an FNV-1a hash compared
//! with `tests/fixtures/utf8_hashes.txt`.

use go_unicode::utf8;

struct Fnv(u64);

impl Fnv {
    fn new() -> Self {
        Fnv(0xcbf29ce484222325)
    }
    #[inline]
    fn byte(&mut self, b: u8) {
        self.0 ^= b as u64;
        self.0 = self.0.wrapping_mul(0x100000001b3);
    }
    fn i32(&mut self, v: i32) {
        for b in v.to_le_bytes() {
            self.byte(b);
        }
    }
}

fn decode_hash(h: &mut Fnv, p: &[u8]) {
    let (r, size) = utf8::decode_rune(p);
    h.i32(r);
    h.byte(size as u8);
    assert_eq!(utf8::decode_rune_in_string(p), (r, size));
    let (r, size) = utf8::decode_last_rune(p);
    h.i32(r);
    h.byte(size as u8);
    assert_eq!(utf8::decode_last_rune_in_string(p), (r, size));
    let full = utf8::full_rune(p);
    assert_eq!(utf8::full_rune_in_string(p), full);
    h.byte(full as u8);
    let valid = utf8::valid(p);
    assert_eq!(utf8::valid_string(p), valid);
    assert_eq!(std::str::from_utf8(p).is_ok(), valid);
    h.byte(valid as u8);
    let n = utf8::rune_count(p);
    assert_eq!(utf8::rune_count_in_string(p), n);
    h.byte(n as u8);
    for (i, c) in utf8::runes(p) {
        h.byte(i as u8);
        h.i32(c);
    }
    h.byte(0xAA);
}

const BYTE_CLASSES: [u8; 18] = [
    0x00, 0x41, 0x7F, 0x80, 0x8F, 0x90, 0x9F, 0xA0, 0xBF, 0xC0, 0xC2, 0xDF, 0xE0, 0xED, 0xF0, 0xF4,
    0xF5, 0xFF,
];

fn compute(name: &str) -> (usize, u64) {
    let mut h = Fnv::new();
    let mut n = 0;
    match name {
        "decode/len1" => {
            for a in 0..=255u8 {
                decode_hash(&mut h, &[a]);
                n += 1;
            }
        }
        "decode/len2" => {
            for a in 0..=255u8 {
                for b in 0..=255u8 {
                    decode_hash(&mut h, &[a, b]);
                    n += 1;
                }
            }
        }
        "decode/len3hi" => {
            for a in 0x80..=255u8 {
                for b in 0..=255u8 {
                    for c in 0..=255u8 {
                        decode_hash(&mut h, &[a, b, c]);
                        n += 1;
                    }
                }
            }
        }
        "decode/len4" => {
            for a in 0xC0..=255u8 {
                for b in 0..=255u8 {
                    for c in BYTE_CLASSES {
                        for d in BYTE_CLASSES {
                            decode_hash(&mut h, &[a, b, c, d]);
                            n += 1;
                        }
                    }
                }
            }
        }
        "decode/len5" => {
            for a in 0xC0..=255u8 {
                for b in BYTE_CLASSES {
                    for c in BYTE_CLASSES {
                        for d in BYTE_CLASSES {
                            for e in BYTE_CLASSES {
                                decode_hash(&mut h, &[a, b, c, d, e]);
                                n += 1;
                            }
                        }
                    }
                }
            }
        }
        other => panic!("unknown line {other}"),
    }
    (n, h.0)
}

#[test]
fn utf8_decode_exhaustive_against_go() {
    let path = format!(
        "{}/tests/fixtures/utf8_hashes.txt",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(path).unwrap();
    let lines: Vec<(String, usize, u64)> = text
        .lines()
        .map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            (
                f[0].to_string(),
                f[1].parse().unwrap(),
                u64::from_str_radix(f[2], 16).unwrap(),
            )
        })
        .collect();
    assert_eq!(lines.len(), 5);
    std::thread::scope(|sc| {
        let handles: Vec<_> = lines
            .iter()
            .map(|(name, count, hash)| {
                sc.spawn(move || {
                    let got = compute(name);
                    assert_eq!(got, (*count, *hash), "{name}");
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
    });
}
