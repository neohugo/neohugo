//! Ports of x/text internal/colltab *_test.go tables (collelem_test.go,
//! contract_test.go, iter_test.go, numeric_test.go, weighter_test.go).

use std::collections::HashMap;

use super::collelem::*;
use super::contract::{CtEntry, scanner};
use super::iter::Iter;
use super::numeric::new_numeric_weighter;
use super::weighter::Weighter;

const DEFAULT_SECONDARY_T: i32 = 0x20;
const DEFAULT_TERTIARY_T: i32 = 0x2;

fn make_ce(w: &[i32]) -> Elem {
    make_elem(w[0], w[1], w[2], w[3] as u8).unwrap_or(Elem(0))
}

fn e(w: &[i32]) -> Elem {
    let dv = [0, DEFAULT_SECONDARY_T, DEFAULT_TERTIARY_T, 0];
    let mut v = w.to_vec();
    v.extend_from_slice(&dv[w.len()..]);
    make_ce(&v)
}

fn make_contract_index(index: i32, n: i32, offset: i32) -> Elem {
    let mut ce = 0xC0000000u32;
    ce += (offset << (4 + 12)) as u32;
    ce += (index << 4) as u32;
    ce += n as u32;
    Elem(ce)
}

// Go: internal/colltab/collelem_test.go:TestColElem
#[test]
fn test_col_elem() {
    // normalCE
    for arg in [
        [0, 0, 0, 0],
        [0, 30, 3, 0],
        [0, 30, 3, 0xFF],
        [100, DEFAULT_SECONDARY_T, DEFAULT_TERTIARY_T, 0],
        [100, DEFAULT_SECONDARY_T, DEFAULT_TERTIARY_T, 0xFF],
        [100, DEFAULT_SECONDARY_T, 3, 0],
        [0x123, DEFAULT_SECONDARY_T, 8, 0xFF],
    ] {
        let ce = make_ce(&arg);
        assert_eq!(ce.ctype(), CeType::Normal);
        assert_eq!(
            [
                ce.primary(),
                ce.secondary(),
                ce.tertiary() as i32,
                ce.ccc() as i32
            ],
            arg,
            "{ce:X?}"
        );
    }
    // contractCE
    for arg in [[0, 0, 0], [1, 1, 1], [1, 15, 1], [4095, 1, 1], [1, 1, 8191]] {
        let ce = make_contract_index(arg[0], arg[1], arg[2]);
        assert_eq!(ce.ctype(), CeType::ContractionIndex);
        let (i, n, o) = split_contract_index(ce);
        assert_eq!([i as i32, n as i32, o as i32], arg);
    }
    // expandCE
    for arg in [0, 5, (1 << 16) - 1] {
        let ce = Elem(0xE0000000 + arg as u32);
        assert_eq!(ce.ctype(), CeType::ExpansionIndex);
        assert_eq!(split_expand_index(ce), arg);
    }
    // decompCE
    for arg in [[0, 0], [1, 1], [0x1F, 0x1F]] {
        let ce = Elem(((arg[1] << 8) + arg[0]) as u32 + 0xF0000000);
        assert_eq!(ce.ctype(), CeType::Decompose);
        let (t1, t2) = split_decompose(ce);
        assert_eq!([t1 as i32, t2 as i32], arg);
    }
}

// Go: internal/colltab/collelem_test.go:TestImplicit
#[test]
fn test_implicit() {
    for (r, p) in [
        (0x33FF, 0x533FF),
        (0x3400, 0x23400),
        (0x4DC0, 0x54DC0),
        (0x4DFF, 0x54DFF),
        (0x4E00, 0x14E00),
        (0x9FCB, 0x19FCB),
        (0xA000, 0x5A000),
        (0xF8FF, 0x5F8FF),
        (0xF900, 0x1F900),
        (0xFA23, 0x1FA23),
        (0xFAD9, 0x1FAD9),
        (0xFB00, 0x5FB00),
        (0x20000, 0x40000),
        (0x2B81C, 0x4B81C),
        (0x10FFFF, 0x15FFFF),
    ] {
        assert_eq!(implicit_primary(r), p, "{r:X}");
    }
}

// Go: internal/colltab/collelem_test.go:TestUpdateTertiary
#[test]
fn test_update_tertiary() {
    for (i, (inp, out, t)) in [
        (0x4000FE20u32, 0x0000FE8Au32, 0x0Au8),
        (0x4000FE21, 0x0000FEAA, 0x0A),
        (0x0000FE8B, 0x0000FE83, 0x03),
        (0x82FF0188, 0x9BFF0188, 0x1B),
        (0xAFF0CC02, 0xAFF0CC1B, 0x1B),
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(Elem(inp).update_tertiary(t), Elem(out), "{i}");
    }
}

fn ct(l: u8, h: u8, n: u8, i: u8) -> CtEntry {
    CtEntry { l, h, n, i }
}

// Go: internal/colltab/contract_test.go:TestLookupContraction
#[test]
fn test_lookup_contraction() {
    #[allow(clippy::type_complexity)]
    let tests: Vec<(Vec<(&str, usize, usize)>, usize, Vec<CtEntry>)> = vec![
        (
            vec![
                ("abc", 1, 3),
                ("a", 0, 0),
                ("b", 0, 0),
                ("c", 0, 0),
                ("d", 0, 0),
            ],
            1,
            vec![
                ct(b'a', 0, 1, 0xFF),
                ct(b'b', 0, 1, 0xFF),
                ct(b'c', b'c', 0, 1),
            ],
        ),
        (
            vec![
                ("abc", 1, 3),
                ("abd", 2, 3),
                ("abe", 3, 3),
                ("a", 0, 0),
                ("ab", 0, 0),
                ("d", 0, 0),
                ("f", 0, 0),
            ],
            1,
            vec![
                ct(b'a', 0, 1, 0xFF),
                ct(b'b', 0, 1, 0xFF),
                ct(b'c', b'e', 0, 1),
            ],
        ),
        (
            vec![
                ("abc", 1, 3),
                ("ab", 2, 2),
                ("a", 3, 1),
                ("abcd", 1, 3),
                ("abe", 2, 2),
            ],
            1,
            vec![ct(b'a', 0, 1, 3), ct(b'b', 0, 1, 2), ct(b'c', b'c', 0, 1)],
        ),
        (
            vec![
                ("abc", 1, 3),
                ("abd", 2, 3),
                ("ab", 3, 2),
                ("ac", 4, 2),
                ("a", 5, 1),
                ("b", 6, 1),
                ("ba", 6, 1),
            ],
            2,
            vec![
                ct(b'b', b'b', 0, 6),
                ct(b'a', 0, 2, 5),
                ct(b'c', b'c', 0, 4),
                ct(b'b', 0, 1, 3),
                ct(b'c', b'd', 0, 1),
            ],
        ),
        (
            vec![
                ("bcde", 2, 4),
                ("bc", 7, 2),
                ("ab", 6, 2),
                ("bcd", 5, 3),
                ("abcd", 1, 4),
                ("abc", 4, 3),
                ("bcdf", 3, 4),
            ],
            2,
            vec![
                ct(b'b', 3, 1, 0xFF),
                ct(b'a', 0, 1, 0xFF),
                ct(b'b', 0, 1, 6),
                ct(b'c', 0, 1, 4),
                ct(b'd', b'd', 0, 1),
                ct(b'c', 0, 1, 7),
                ct(b'd', 0, 1, 5),
                ct(b'e', b'f', 0, 2),
            ],
        ),
    ];
    for (i, (lookups, nnode, tries)) in tests.iter().enumerate() {
        for (j, &(s, offset, n)) in lookups.iter().enumerate() {
            for s in [s.to_string(), format!("{s}X")] {
                let mut scan = scanner(tries, 0, *nnode, s.as_bytes());
                scan.scan(0);
                let (o, got_n) = scan.result();
                assert_eq!((o, got_n), (offset, n), "{i}:{j}: {s:?}");
            }
        }
    }
}

// Go: internal/colltab/iter_test.go:TestDoNorm
#[test]
fn test_do_norm() {
    const DIV: i32 = -1;
    let tests: Vec<(Vec<i32>, Vec<i32>)> = vec![
        (vec![4, DIV, 3], vec![3, 4]),
        (vec![4, DIV, 3, 3, 3], vec![3, 3, 3, 4]),
        (vec![0, 4, DIV, 3], vec![0, 3, 4]),
        (vec![0, 0, 4, 5, DIV, 3, 3], vec![0, 0, 3, 3, 4, 5]),
        (vec![0, 0, 1, 4, 5, DIV, 3, 3], vec![0, 0, 1, 3, 3, 4, 5]),
        (vec![0, 0, 1, 4, 5, DIV, 4, 4], vec![0, 0, 1, 4, 4, 4, 5]),
    ];
    let tw = TestWeighter(HashMap::new());
    for (j, (inp, out)) in tests.iter().enumerate() {
        let mut elems = Vec::new();
        let (mut w, mut p) = (0, 0);
        for (k, &cc) in inp.iter().enumerate() {
            if cc == DIV {
                w = 100;
                p = k;
                continue;
            }
            elems.push(make_ce(&[w, DEFAULT_SECONDARY_T, 2, cc]));
        }
        let mut it = Iter::new(&tw, Vec::new());
        it.elems = elems;
        let ccc = it.elems[p].ccc();
        it.do_norm_for_test(p, ccc);
        let got: Vec<i32> = it.elems.iter().map(|e| e.ccc() as i32).collect();
        assert_eq!(&got, out, "{j}");
    }
}

/// weighter_test.go:testWeighter
struct TestWeighter(HashMap<Vec<u8>, Vec<Elem>>);

impl Weighter for TestWeighter {
    fn append_next(&self, buf: &mut Vec<Elem>, s: &[u8]) -> usize {
        let n = s.len().min(10);
        for i in (1..=n).rev() {
            if let Some(e) = self.0.get(&s[..i]) {
                buf.extend_from_slice(e);
                return i;
            }
        }
        panic!(
            "incomplete testWeighter: could not find {:?}",
            String::from_utf8_lossy(s)
        );
    }
    fn top(&self) -> u32 {
        0
    }
}

fn p(w: &[i32]) -> Vec<Elem> {
    w.iter()
        .map(|&x| make_elem(x, DEFAULT_SECONDARY_T, DEFAULT_TERTIARY_T, 0).unwrap())
        .collect()
}

fn num_weighter() -> TestWeighter {
    let dig_sec = DEFAULT_SECONDARY_T;
    let dig_tert = DEFAULT_TERTIARY_T;
    let t_plus3 = e(&[0, 50, dig_tert + 3]);
    let mut m: HashMap<Vec<u8>, Vec<Elem>> = HashMap::new();
    let mut add = |k: &str, v: Vec<Elem>| {
        m.insert(k.as_bytes().to_vec(), v);
    };
    add("0", p(&[100]));
    add("０", vec![e(&[100, dig_sec, dig_tert + 1])]);
    add("₀", vec![e(&[100, dig_sec, dig_tert + 5])]);
    add("1", p(&[101]));
    add("١", [p(&[101]), vec![t_plus3]].concat());
    add("１", vec![e(&[101, dig_sec, dig_tert + 1])]);
    add("2", p(&[102]));
    add("٢", [p(&[102]), vec![t_plus3]].concat());
    add("２", vec![e(&[102, dig_sec, dig_tert + 3])]);
    add("3", p(&[103]));
    add("4", p(&[104]));
    add("5", p(&[105]));
    add("6", p(&[106]));
    add("7", p(&[107]));
    add("8", p(&[118]));
    add("9", p(&[119]));
    add("٩", [p(&[119]), vec![t_plus3]].concat());
    add("９", vec![e(&[119, dig_sec, dig_tert + 1])]);
    add("₉", vec![e(&[119, dig_sec, dig_tert + 5])]);
    add("a", p(&[5]));
    add("b", p(&[6]));
    add("c", p(&[8, 2]));
    add("klm", p(&[99]));
    add("nop", p(&[121]));
    add("x", p(&[200]));
    add("y", p(&[201]));
    TestWeighter(m)
}

// Go: internal/colltab/numeric_test.go:TestNumericAppendNext
#[test]
fn test_numeric_append_next() {
    let t_plus3 = e(&[0, 50, DEFAULT_TERTIARY_T + 3]);
    let tests: Vec<(&str, Vec<Elem>)> = vec![
        ("a", p(&[5])),
        ("klm", p(&[99])),
        ("aa", p(&[5, 5])),
        ("1", p(&[120, 1, 101])),
        ("0", p(&[120, 0])),
        ("01", p(&[120, 1, 101])),
        ("0001", p(&[120, 1, 101])),
        ("10", p(&[120, 2, 101, 100])),
        ("99", p(&[120, 2, 119, 119])),
        ("9999", p(&[120, 4, 119, 119, 119, 119])),
        ("1a", p(&[120, 1, 101, 5])),
        ("0b", p(&[120, 0, 6])),
        ("01c", p(&[120, 1, 101, 8, 2])),
        ("10x", p(&[120, 2, 101, 100, 200])),
        ("99y", p(&[120, 2, 119, 119, 201])),
        ("9999nop", p(&[120, 4, 119, 119, 119, 119, 121])),
        (
            "١٢٩",
            vec![
                e(&[120]),
                e(&[3]),
                e(&[101]),
                t_plus3,
                e(&[102]),
                t_plus3,
                e(&[119]),
                t_plus3,
            ],
        ),
        (
            "１２９",
            vec![
                e(&[120]),
                e(&[3]),
                e(&[101, DEFAULT_SECONDARY_T, DEFAULT_TERTIARY_T + 1]),
                e(&[102, DEFAULT_SECONDARY_T, DEFAULT_TERTIARY_T + 3]),
                e(&[119, DEFAULT_SECONDARY_T, DEFAULT_TERTIARY_T + 1]),
            ],
        ),
        ("a10", p(&[5, 120, 2, 101, 100])),
    ];
    for (inp, want) in tests {
        let nw = new_numeric_weighter(Box::new(num_weighter()));
        let b = inp.as_bytes();
        let mut got = Vec::new();
        let mut n = 0;
        while n < b.len() {
            n += nw.append_next(&mut got, &b[n..]);
        }
        assert_eq!(got, want, "AppendNext({inp:?})");
    }
}

// Go: internal/colltab/numeric_test.go:TestNumericOverflow
#[test]
fn test_numeric_overflow() {
    let max_digits = (1usize << 21) - 1;
    let many = "9".repeat(max_digits + 1) + "a";
    let nw = new_numeric_weighter(Box::new(num_weighter()));
    let mut got = Vec::new();
    let n = nw.append_next(&mut got, many.as_bytes());
    assert_eq!(n, max_digits);
    assert_eq!(got[1].primary(), max_digits as i32);
}

// Go: internal/colltab/trie_test.go:TestLookupTrie
#[test]
fn test_lookup_trie() {
    use super::trie::Trie;
    const TX: u8 = 0x80;
    const T2: u8 = 0xC0;
    const T3: u8 = 0xE0;
    const T4: u8 = 0xF0;
    const T5: u8 = 0xF8;
    const T6: u8 = 0xFC;
    let test_runes: [u32; 17] = [
        0x01, 0x0C, 0x7F, // 1-byte sequences
        0x80, 0x100, 0x7FF, // 2-byte sequences
        0x800, 0x999, 0xFFFF, // 3-byte sequences
        0x10000, 0x10101, 0x10FFFF, // 4-byte sequences
        0x200, 0x201, 0x202, 0x210, 0x215, // five entries in one sparse block
    ];
    let tests: Vec<(usize, Vec<u8>)> = vec![
        // illegal runes
        (1, vec![0x80]),
        (1, vec![0xFF]),
        (1, vec![T2, TX - 1]),
        (1, vec![T2, T2]),
        (2, vec![T3, TX, TX - 1]),
        (2, vec![T3, TX, T2]),
        (1, vec![T3, TX - 1, TX]),
        (3, vec![T4, TX, TX, TX - 1]),
        (3, vec![T4, TX, TX, T2]),
        (1, vec![T4, T2, TX, TX - 1]),
        (2, vec![T4, TX, T2, TX - 1]),
        // short runes
        (0, vec![T2]),
        (0, vec![T3, TX]),
        (0, vec![T4, TX, TX]),
        // we only support UTF-8 up to utf8.UTFMax bytes (4 bytes)
        (1, vec![T5, TX, TX, TX, TX]),
        (1, vec![T6, TX, TX, TX, TX, TX]),
    ];
    let mut values = vec![0u32; 832];
    for (i, v) in [
        (0x000c, 0x01),
        (0x007f, 0x02),
        (0x00c0, 0x03),
        (0x0100, 0x04),
        (0x0140, 0x0c),
        (0x0141, 0x0d),
        (0x0142, 0x0e),
        (0x0150, 0x0f),
        (0x0155, 0x10),
        (0x01bf, 0x05),
        (0x01c0, 0x06),
        (0x0219, 0x07),
        (0x027f, 0x08),
        (0x0280, 0x09),
        (0x02c1, 0x0a),
        (0x033f, 0x0b),
    ] {
        values[i] = v;
    }
    let mut lookup = vec![0u16; 640];
    for (i, v) in [
        (0x0e0, 0x05),
        (0x0e6, 0x06),
        (0x13f, 0x07),
        (0x140, 0x08),
        (0x144, 0x09),
        (0x190, 0x03),
        (0x1ff, 0x0a),
        (0x20f, 0x05),
        (0x242, 0x01),
        (0x244, 0x02),
        (0x248, 0x03),
        (0x25f, 0x04),
        (0x260, 0x01),
        (0x26f, 0x02),
        (0x270, 0x04),
        (0x274, 0x06),
    ] {
        lookup[i] = v;
    }
    let values: &'static [u32] = Box::leak(values.into_boxed_slice());
    let lookup: &'static [u16] = Box::leak(lookup.into_boxed_slice());
    let trie = Trie {
        index0: &lookup[6 * 64..],
        values0: values,
        index: lookup,
        values,
    };
    for (i, &r) in test_runes.iter().enumerate() {
        let mut buf = [0u8; 4];
        let n = crate::goutf8::encode_rune(&mut buf, r as i32);
        let b = &buf[..n];
        let (v, sz) = trie.lookup(b);
        assert_eq!(v.0 as usize, i, "lookup({r:#x}): value");
        assert_eq!(sz, b.len(), "lookup({r:#x}): size");
    }
    for (i, (size, bytes)) in tests.iter().enumerate() {
        let (v, sz) = trie.lookup(bytes);
        assert_eq!(v.0, 0, "illegal rune case {i}: value");
        assert_eq!(sz, *size, "illegal rune case {i}: size");
    }
}

// Go: internal/colltab/colltab_test.go:TestMatchLang
#[test]
fn test_match_lang() {
    use crate::language;
    let p = |s: &str| language::parse(s).unwrap();
    let tags = vec![
        language::und(),
        p("bs"),
        p("de"),
        p("en"),
        p("en-US"),
        p("en-US-u-va-posix"),
        p("pt"),
        p("sr"),
        p("sr-Latn"),
        p("zh"),
        p("zh-u-co-stroke"),
        p("zh-Hant-u-co-pinyin"),
        p("zh-Hant"),
    ];
    for (i, (x, t)) in [
        (0, "und"),
        (0, "fa"), // Default to first element when no match.
        (3, "en"),
        (4, "en-US"),
        (5, "en-US-u-va-posix"),   // Ext. variant match.
        (4, "en-US-u-va-noposix"), // Ext. variant mismatch.
        (3, "en-UK-u-va-noposix"), // Ext. variant mismatch.
        (7, "sr"),
        (0, "hr"),      // Don't match to close language!
        (0, "gsw"),     // Don't match to close language!
        (1, "bs-Cyrl"), // Odd, but correct.
        (1, "bs-Latn"), // Estimated script drops.
        (8, "sr-Latn"),
        (9, "zh"),
        (9, "zh-Hans"),
        (12, "zh-Hant"),
        (11, "zh-Hant-u-co-pinyin"),
        (10, "zh-Hant-u-co-stroke"),
        // There is no "phonebk" sorting order for zh-Hant, so use default.
        (12, "zh-Hant-u-co-phonebk"),
        (10, "zh-u-co-stroke"),
        (12, "und-TW"),     // Infer script and language.
        (12, "und-HK"),     // Infer script and language.
        (6, "und-BR"),      // Infer script and language.
        (6, "und-PT"),      // Infer script and language.
        (2, "und-Latn-DE"), // Infer language.
        (0, "und-Jpan-BR"), // Infers "ja", so no match.
        (0, "zu"),          // No match past index.
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(super::match_lang(&p(t), &tags), x, "{i}: MatchLang({t:?})");
    }
}
