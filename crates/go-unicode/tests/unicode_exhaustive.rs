//! Exhaustive differential test of the unicode tables, predicates and case
//! mappings against go1.27.1: every code point 0..=0x10FFFF plus a set of
//! invalid runes is evaluated, and the FNV-1a hash of the results is compared
//! with `tests/fixtures/unicode_hashes.txt` (written by
//! `tools/go-oracle/go-unicode fixtures`).

use std::collections::HashMap;
use std::sync::Mutex;

use go_unicode::{self as unicode, RangeTable, Rune, utf8, utf16};

const EXTRA_RUNES: [Rune; 10] = [
    -1,
    -2,
    i32::MIN,
    0x110000,
    0x110001,
    0x1FFFFF,
    0x7FFFFFFF,
    -0xFFFD,
    0xFFFFFF,
    0x10FFFF + 0x20,
];

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
    fn bytes(&mut self, p: &[u8]) {
        for &b in p {
            self.byte(b);
        }
    }
    fn i32(&mut self, v: i32) {
        self.bytes(&v.to_le_bytes());
    }
}

fn for_all_runes(mut f: impl FnMut(Rune)) {
    for r in 0..=unicode::MAX_RUNE {
        f(r);
    }
    for r in EXTRA_RUNES {
        f(r);
    }
}

fn pred_line(p: impl Fn(Rune) -> bool) -> (usize, u64) {
    let mut h = Fnv::new();
    let mut n = 0;
    for_all_runes(|r| {
        let v = p(r);
        if v {
            n += 1;
        }
        h.byte(v as u8);
    });
    (n, h.0)
}

fn map_line(m: impl Fn(Rune) -> Rune) -> (usize, u64) {
    let mut h = Fnv::new();
    let mut n = 0;
    for_all_runes(|r| {
        let v = m(r);
        if v != r {
            n += 1;
        }
        h.i32(v);
    });
    (n, h.0)
}

fn load(name: &str) -> Vec<(String, usize, u64)> {
    let path = format!("{}/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), name);
    let text = std::fs::read_to_string(&path).unwrap();
    text.lines()
        .map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            (
                f[0].to_string(),
                f[1].parse().unwrap(),
                u64::from_str_radix(f[2], 16).unwrap(),
            )
        })
        .collect()
}

fn table_for(name: &str) -> Option<&'static RangeTable> {
    let (kind, key) = name.split_once('/')?;
    match kind {
        "Categories" => unicode::lookup(unicode::CATEGORIES, key),
        "Scripts" => unicode::lookup(unicode::SCRIPTS, key),
        "Properties" => unicode::lookup(unicode::PROPERTIES, key),
        "FoldCategory" => unicode::lookup(unicode::FOLD_CATEGORY, key),
        "FoldScript" => unicode::lookup(unicode::FOLD_SCRIPT, key),
        "exported" => lookup_exported(key),
        _ => None,
    }
}

fn lookup_exported(name: &str) -> Option<&'static RangeTable> {
    unicode::ALL_EXPORTED_TABLES
        .iter()
        .find(|(k, _)| *k == name)
        .map(|(_, t)| *t)
}

fn compute(name: &str) -> Option<(usize, u64)> {
    Some(match name {
        "pred/IsLetter" => pred_line(unicode::is_letter),
        "pred/IsDigit" => pred_line(unicode::is_digit),
        "pred/IsNumber" => pred_line(unicode::is_number),
        "pred/IsMark" => pred_line(unicode::is_mark),
        "pred/IsSpace" => pred_line(unicode::is_space),
        "pred/IsPunct" => pred_line(unicode::is_punct),
        "pred/IsSymbol" => pred_line(unicode::is_symbol),
        "pred/IsPrint" => pred_line(unicode::is_print),
        "pred/IsGraphic" => pred_line(unicode::is_graphic),
        "pred/IsControl" => pred_line(unicode::is_control),
        "pred/IsUpper" => pred_line(unicode::is_upper),
        "pred/IsLower" => pred_line(unicode::is_lower),
        "pred/IsTitle" => pred_line(unicode::is_title),
        "pred/utf16.IsSurrogate" => pred_line(utf16::is_surrogate),
        "pred/utf8.ValidRune" => pred_line(utf8::valid_rune),
        "pred/In/Graphic" => pred_line(|r| unicode::r#in(r, unicode::GRAPHIC_RANGES)),
        "pred/In/Print" => pred_line(|r| unicode::r#in(r, unicode::PRINT_RANGES)),
        "pred/In/Lu,Nd,Han" => {
            pred_line(|r| unicode::r#in(r, &[unicode::LU, unicode::ND, unicode::HAN]))
        }
        "pred/IsOneOf/Graphic" => pred_line(|r| unicode::is_one_of(unicode::GRAPHIC_RANGES, r)),
        "pred/IsOneOf/empty" => pred_line(|r| unicode::is_one_of(&[], r)),
        "map/ToUpper" => map_line(unicode::to_upper),
        "map/ToLower" => map_line(unicode::to_lower),
        "map/ToTitle" => map_line(unicode::to_title),
        "map/SimpleFold" => map_line(unicode::simple_fold),
        "map/To/-1" => map_line(|r| unicode::to(-1, r)),
        "map/To/0" => map_line(|r| unicode::to(0, r)),
        "map/To/1" => map_line(|r| unicode::to(1, r)),
        "map/To/2" => map_line(|r| unicode::to(2, r)),
        "map/To/3" => map_line(|r| unicode::to(3, r)),
        "map/TurkishCase.ToUpper" => map_line(|r| unicode::TURKISH_CASE.to_upper(r)),
        "map/TurkishCase.ToLower" => map_line(|r| unicode::TURKISH_CASE.to_lower(r)),
        "map/TurkishCase.ToTitle" => map_line(|r| unicode::TURKISH_CASE.to_title(r)),
        "map/AzeriCase.ToUpper" => map_line(|r| unicode::AZERI_CASE.to_upper(r)),
        "map/utf8.RuneLen" => map_line(|r| utf8::rune_len(r) as Rune),
        "map/utf16.RuneLen" => map_line(|r| utf16::rune_len(r) as Rune),
        "map/utf16.EncodeRune.1" => map_line(|r| utf16::encode_rune(r).0),
        "map/utf16.EncodeRune.2" => map_line(|r| utf16::encode_rune(r).1),
        "enc/utf8.AppendRune" | "enc/string(rune)" => {
            let mut h = Fnv::new();
            let mut n = 0;
            let mut b = Vec::new();
            for_all_runes(|r| {
                b.clear();
                utf8::append_rune(&mut b, r);
                let mut p = [0u8; 4];
                let k = utf8::encode_rune(&mut p, r);
                assert_eq!(&p[..k], &b[..], "encode_rune != append_rune for {r:#x}");
                if name == "enc/string(rune)" {
                    assert_eq!(utf8::rune_to_string(r), b);
                }
                n += b.len();
                h.byte(b.len() as u8);
                h.bytes(&b);
            });
            (n, h.0)
        }
        "enc/utf16.AppendRune" => {
            let mut h = Fnv::new();
            let mut n = 0;
            let mut a = Vec::new();
            for_all_runes(|r| {
                a.clear();
                utf16::append_rune(&mut a, r);
                assert_eq!(utf16::encode(&[r]), a);
                n += a.len();
                h.byte(a.len() as u8);
                for &u in &a {
                    h.byte(u as u8);
                    h.byte((u >> 8) as u8);
                }
            });
            (n, h.0)
        }
        "utf16.DecodeRune/window3" => {
            let mut vals: Vec<Rune> = (0xD700..0xE100).step_by(3).collect();
            vals.extend_from_slice(&[
                -1, 0, 0x41, 0xD800, 0xDBFF, 0xDC00, 0xDFFF, 0xE000, 0x10000, 0x10FFFF, 0x110000,
            ]);
            let mut h = Fnv::new();
            let mut n = 0;
            for &a in &vals {
                for &b in &vals {
                    let v = utf16::decode_rune(a, b);
                    if v != unicode::REPLACEMENT_CHAR {
                        n += 1;
                    }
                    h.i32(v);
                }
            }
            (n, h.0)
        }
        _ => {
            let t = table_for(name)?;
            pred_line(|r| unicode::is(t, r))
        }
    })
}

#[test]
fn exhaustive_against_go() {
    let lines = load("unicode_hashes.txt");
    assert!(lines.len() > 500);
    // Identical tables (aliases) hash identically; compute each distinct
    // table once, in parallel.
    let cache: Mutex<HashMap<usize, (usize, u64)>> = Mutex::new(HashMap::new());
    let failures: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let next = std::sync::atomic::AtomicUsize::new(0);
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    std::thread::scope(|sc| {
        for _ in 0..threads {
            sc.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if i >= lines.len() {
                        break;
                    }
                    let (name, count, hash) = &lines[i];
                    let got = if let Some(t) = table_for(name) {
                        let key = t as *const RangeTable as usize;
                        let cached = cache.lock().unwrap().get(&key).copied();
                        match cached {
                            Some(v) => v,
                            None => {
                                let v = pred_line(|r| unicode::is(t, r));
                                cache.lock().unwrap().insert(key, v);
                                v
                            }
                        }
                    } else {
                        match compute(name) {
                            Some(v) => v,
                            None => {
                                failures.lock().unwrap().push(format!("{name}: unknown"));
                                continue;
                            }
                        }
                    };
                    if got != (*count, *hash) {
                        failures.lock().unwrap().push(format!(
                            "{name}: got count {} hash {:016x}, want count {count} hash {hash:016x}",
                            got.0, got.1
                        ));
                    }
                }
            });
        }
    });
    let failures = failures.into_inner().unwrap();
    assert!(
        failures.is_empty(),
        "{} mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn meta_against_go() {
    let path = format!(
        "{}/tests/fixtures/unicode_meta.txt",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(path).unwrap();
    let mut aliases = 0;
    for l in text.lines() {
        let f: Vec<&str> = l.split('\t').collect();
        match f[0] {
            "Version" => assert_eq!(unicode::VERSION, f[1]),
            "CategoryAliases" => {
                aliases += 1;
                assert_eq!(unicode::lookup(unicode::CATEGORY_ALIASES, f[1]), Some(f[2]));
            }
            "Categories" => assert_eq!(unicode::CATEGORIES.len().to_string(), f[1]),
            "Scripts" => assert_eq!(unicode::SCRIPTS.len().to_string(), f[1]),
            "Properties" => assert_eq!(unicode::PROPERTIES.len().to_string(), f[1]),
            "FoldCategory" => assert_eq!(unicode::FOLD_CATEGORY.len().to_string(), f[1]),
            "FoldScript" => assert_eq!(unicode::FOLD_SCRIPT.len().to_string(), f[1]),
            "exported" => assert_eq!(unicode::ALL_EXPORTED_TABLES.len().to_string(), f[1]),
            "CaseRanges" => assert_eq!(unicode::CASE_RANGES.len().to_string(), f[1]),
            other => panic!("unknown meta line {other}"),
        }
    }
    assert_eq!(aliases, unicode::CATEGORY_ALIASES.len());
    // Every map is sorted for binary search.
    for m in [
        unicode::CATEGORIES,
        unicode::SCRIPTS,
        unicode::PROPERTIES,
        unicode::FOLD_CATEGORY,
        unicode::FOLD_SCRIPT,
    ] {
        assert!(m.windows(2).all(|w| w[0].0.as_bytes() < w[1].0.as_bytes()));
        for (k, t) in m {
            assert!(std::ptr::eq(unicode::lookup(m, k).unwrap(), *t));
        }
    }
    assert_eq!(unicode::lookup(unicode::CATEGORIES, "Nope"), None);
}
