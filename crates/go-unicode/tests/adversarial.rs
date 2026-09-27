//! Adversarial differential test against go1.27.1 (fixture written by
//! `go run ./tools/go-oracle/go-unicode adversarial`):
//!
//! - `struct/...`: exact structure (R16, R32, LatinOffset) of every table,
//!   CaseRanges, TurkishCase/AzeriCase, GraphicRanges/PrintRanges.
//! - `orbit/all`: the SimpleFold orbit walked from every code point.
//! - `<domain>/<func>`: ~55 strings/bytes functions over every code point,
//!   code points in ASCII / Unicode-space / invalid-UTF-8 contexts, and every
//!   short byte string (all of length <= 2, byte classes for lengths 3, 4).
//! - `foldpair/...`: EqualFold of every code point against its fold orbit,
//!   case mappings and neighbours; EqualFold over all pairs of a string set.
//!
//! Each line is an FNV-1a hash (plus a count) of all results, recomputed here
//! and compared with the Go values. Lines are computed in parallel.

use go_unicode::{self as unicode, CaseRange, RangeTable, Rune, bytes, strings, utf8};

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
    fn blob(&mut self, p: &[u8]) {
        self.i32(p.len() as i32);
        self.bytes(p);
    }
    fn bool(&mut self, v: bool) {
        self.byte(v as u8);
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

fn enc_int(i: isize) -> Vec<u8> {
    i.to_string().into_bytes()
}

fn enc_bool(b: bool) -> Vec<u8> {
    if b {
        b"true".to_vec()
    } else {
        b"false".to_vec()
    }
}

fn enc_list<T: AsRef<[u8]>>(l: &[T]) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&(l.len() as u32).to_le_bytes());
    for e in l {
        let e = e.as_ref();
        b.extend_from_slice(&(e.len() as u32).to_le_bytes());
        b.extend_from_slice(e);
    }
    b
}

fn enc_runes(rs: &[Rune]) -> Vec<u8> {
    let mut b = Vec::new();
    for r in rs {
        b.extend_from_slice(&r.to_le_bytes());
    }
    b
}

fn cat(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

// ---------------------------------------------------------------------------
// struct/...

fn struct_table(h: &mut Fnv, t: &RangeTable) {
    h.i32(t.r16.len() as i32);
    for r in t.r16 {
        h.i32(r.lo as i32);
        h.i32(r.hi as i32);
        h.i32(r.stride as i32);
    }
    h.i32(t.r32.len() as i32);
    for r in t.r32 {
        h.i32(r.lo as i32);
        h.i32(r.hi as i32);
        h.i32(r.stride as i32);
    }
    h.i32(t.latin_offset as i32);
}

fn struct_case_ranges(h: &mut Fnv, crs: &[CaseRange]) {
    h.i32(crs.len() as i32);
    for cr in crs {
        h.i32(cr.lo as i32);
        h.i32(cr.hi as i32);
        for d in cr.delta {
            h.i32(d);
        }
    }
}

fn struct_line(name: &str) -> Option<(usize, u64)> {
    let mut h = Fnv::new();
    let key = name.strip_prefix("struct/")?;
    let n = match key {
        "GraphicRanges" | "PrintRanges" => {
            let l = if key == "GraphicRanges" {
                unicode::GRAPHIC_RANGES
            } else {
                unicode::PRINT_RANGES
            };
            for t in l {
                struct_table(&mut h, t);
            }
            l.len()
        }
        "CaseRanges" => {
            struct_case_ranges(&mut h, unicode::CASE_RANGES);
            unicode::CASE_RANGES.len()
        }
        "TurkishCase" => {
            struct_case_ranges(&mut h, unicode::TURKISH_CASE.0);
            unicode::TURKISH_CASE.0.len()
        }
        "AzeriCase" => {
            struct_case_ranges(&mut h, unicode::AZERI_CASE.0);
            unicode::AZERI_CASE.0.len()
        }
        _ => {
            let (kind, k) = key.split_once('/')?;
            let t = match kind {
                "Categories" => unicode::lookup(unicode::CATEGORIES, k),
                "Scripts" => unicode::lookup(unicode::SCRIPTS, k),
                "Properties" => unicode::lookup(unicode::PROPERTIES, k),
                "FoldCategory" => unicode::lookup(unicode::FOLD_CATEGORY, k),
                "FoldScript" => unicode::lookup(unicode::FOLD_SCRIPT, k),
                "exported" => unicode::ALL_EXPORTED_TABLES
                    .iter()
                    .find(|(n, _)| *n == k)
                    .map(|(_, t)| *t),
                _ => None,
            }?;
            struct_table(&mut h, t);
            t.r16.len() + t.r32.len()
        }
    };
    Some((n, h.0))
}

// ---------------------------------------------------------------------------
// orbit/...

fn orbit(r: Rune) -> Vec<Rune> {
    let mut o = Vec::new();
    let mut x = unicode::simple_fold(r);
    while x != r && o.len() < 16 {
        o.push(x);
        x = unicode::simple_fold(x);
    }
    o
}

fn orbit_line() -> (usize, u64) {
    let mut h = Fnv::new();
    let mut n = 0;
    for_all_runes(|r| {
        let o = orbit(r);
        n += o.len();
        h.i32(o.len() as i32);
        for x in o {
            h.i32(x);
        }
    });
    (n, h.0)
}

// ---------------------------------------------------------------------------
// String domains.

const ADV_BYTES3: [u8; 53] = [
    0x00, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x20, 0x30, 0x41, 0x49, 0x4B, 0x53, 0x5A, 0x5F, 0x61, 0x69,
    0x6B, 0x73, 0x7A, 0x7F, 0x80, 0x84, 0x85, 0x89, 0x8F, 0x90, 0x9E, 0x9F, 0xA0, 0xAA, 0xB0, 0xBF,
    0xC0, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC7, 0xCE, 0xCF, 0xD0, 0xDF, 0xE0, 0xE1, 0xE2, 0xE3, 0xED,
    0xEF, 0xF0, 0xF4, 0xF5, 0xFF,
];

const ADV_BYTES4: [u8; 20] = [
    0x0A, 0x20, 0x41, 0x61, 0x80, 0x85, 0x9F, 0xA0, 0xBF, 0xC0, 0xC2, 0xC4, 0xCE, 0xE0, 0xE1, 0xE2,
    0xED, 0xF0, 0xF4, 0xFF,
];

fn ctx_rune(r: Rune) -> bool {
    !(0x20000..=unicode::MAX_RUNE).contains(&r) || r % 97 == 0
}

fn for_domain(name: &str, mut f: impl FnMut(&[u8])) {
    match name {
        "rune" => for_all_runes(|r| f(&utf8::rune_to_string(r))),
        "ctx1" => for_all_runes(|r| {
            if ctx_rune(r) {
                let s = utf8::rune_to_string(r);
                f(&cat(&[b" ", &s, b"a", &s, "\u{a0}".as_bytes()]));
            }
        }),
        "ctx2" => for_all_runes(|r| {
            if ctx_rune(r) {
                let s = utf8::rune_to_string(r);
                f(&cat(&[
                    "\u{2000}".as_bytes(),
                    &s,
                    b"\xff",
                    &s,
                    b"_",
                    &s,
                    "\u{3000}".as_bytes(),
                ]));
            }
        }),
        "ctx3" => for_all_runes(|r| {
            if ctx_rune(r) {
                let s = utf8::rune_to_string(r);
                f(&cat(&[b"A", &s[..s.len() - 1], b"b", &s[1..], b"Z", &s]));
            }
        }),
        "bytes" => {
            f(b"");
            for a in 0..=255u8 {
                f(&[a]);
            }
            for a in 0..=255u8 {
                for b in 0..=255u8 {
                    f(&[a, b]);
                }
            }
            for a in ADV_BYTES3 {
                for b in ADV_BYTES3 {
                    for c in ADV_BYTES3 {
                        f(&[a, b, c]);
                    }
                }
            }
            for a in ADV_BYTES4 {
                for b in ADV_BYTES4 {
                    for c in ADV_BYTES4 {
                        for d in ADV_BYTES4 {
                            f(&[a, b, c, d]);
                        }
                    }
                }
            }
        }
        _ => panic!("unknown domain {name}"),
    }
}

fn map_fold(r: Rune) -> Rune {
    if unicode::is_space(r) {
        return -1;
    }
    if r == utf8::RUNE_ERROR {
        return unicode::MAX_RUNE;
    }
    unicode::simple_fold(r)
}

const ADV_ANY: &[u8] = b"\xc2\xa0\xffAk";
const ADV_TRIM_L: &[u8] = b" \xc2\xa0\xffA";
const ADV_TRIM_R: &[u8] = b"\xc2\xa0\xff\xe2\x80\x80z";
const ADV_TRIM_BOTH: &[u8] = b"\xff\x80\xe3\x80\x80";
const FFFD: &[u8] = "\u{fffd}".as_bytes();

type StrFn = fn(&[u8]) -> Vec<u8>;

fn adv_func(name: &str) -> Option<StrFn> {
    use unicode::TURKISH_CASE as TR;
    Some(match name {
        "ToUpper" => |s| strings::to_upper(s).into_owned(),
        "ToLower" => |s| strings::to_lower(s).into_owned(),
        "ToTitle" => |s| strings::to_title(s).into_owned(),
        "Title" => |s| strings::title(s).into_owned(),
        "ToUpperSpecialTR" => |s| strings::to_upper_special(TR, s).into_owned(),
        "ToLowerSpecialTR" => |s| strings::to_lower_special(TR, s).into_owned(),
        "ToTitleSpecialTR" => |s| strings::to_title_special(TR, s).into_owned(),
        "TrimSpace" => |s| strings::trim_space(s).to_vec(),
        "Fields" => |s| enc_list(&strings::fields(s)),
        "FieldsFuncPunct" => |s| enc_list(&strings::fields_func(s, unicode::is_punct)),
        "EqualFoldUpper" => |s| enc_bool(strings::equal_fold(s, &strings::to_upper(s))),
        "EqualFoldLower" => |s| enc_bool(strings::equal_fold(&strings::to_lower(s), s)),
        "EqualFoldTitle" => |s| enc_bool(strings::equal_fold(s, &strings::to_title(s))),
        "EqualFoldSimpleFold" => |s| {
            enc_bool(strings::equal_fold(
                s,
                &strings::map(unicode::simple_fold, s),
            ))
        },
        "EqualFoldTurkish" => {
            |s| enc_bool(strings::equal_fold(&strings::to_upper_special(TR, s), s))
        }
        "ToValidUTF8" => |s| strings::to_valid_utf8(s, FFFD).into_owned(),
        "ToValidUTF8Empty" => |s| strings::to_valid_utf8(s, b"").into_owned(),
        "MapIdentity" => |s| strings::map(|r| r, s).into_owned(),
        "MapFold" => |s| strings::map(map_fold, s).into_owned(),
        "TrimFuncLetter" => |s| strings::trim_func(s, unicode::is_letter).to_vec(),
        "TrimLeftSet" => |s| strings::trim_left(s, ADV_TRIM_L).to_vec(),
        "TrimRightSet" => |s| strings::trim_right(s, ADV_TRIM_R).to_vec(),
        "TrimSet" => |s| strings::trim(s, ADV_TRIM_BOTH).to_vec(),
        "IndexRuneError" => |s| enc_int(strings::index_rune(s, utf8::RUNE_ERROR)),
        "IndexFuncUpper" => |s| enc_int(strings::index_func(s, unicode::is_upper)),
        "LastIndexFuncSpace" => |s| enc_int(strings::last_index_func(s, unicode::is_space)),
        "SplitEmpty" => |s| enc_list(&strings::split(s, b"")),
        "SplitN2Empty" => |s| enc_list(&strings::split_n(s, b"", 2)),
        "IndexAny" => |s| enc_int(strings::index_any(s, ADV_ANY)),
        "LastIndexAny" => |s| enc_int(strings::last_index_any(s, ADV_ANY)),
        "RuneCount" => |s| enc_int(utf8::rune_count_in_string(s) as isize),
        "Runes" => |s| enc_runes(&utf8::to_runes(s)),
        "bytes.ToUpper" => |s| bytes::to_upper(s),
        "bytes.ToLower" => |s| bytes::to_lower(s),
        "bytes.ToTitle" => |s| bytes::to_title(s),
        "bytes.Title" => |s| bytes::title(s),
        "bytes.ToUpperSpecialTR" => |s| bytes::to_upper_special(TR, s),
        "bytes.TrimSpace" => |s| bytes::trim_space(s).to_vec(),
        "bytes.Fields" => |s| enc_list(&bytes::fields(s)),
        "bytes.EqualFoldUpper" => |s| enc_bool(bytes::equal_fold(s, &bytes::to_upper(s))),
        "bytes.EqualFoldLower" => |s| enc_bool(bytes::equal_fold(&bytes::to_lower(s), s)),
        "bytes.ToValidUTF8" => |s| bytes::to_valid_utf8(s, FFFD),
        "bytes.MapIdentity" => |s| bytes::map(|r| r, s),
        "bytes.MapFold" => |s| bytes::map(map_fold, s),
        "bytes.Runes" => |s| enc_runes(&bytes::runes(s)),
        "bytes.IndexAny" => |s| enc_int(bytes::index_any(s, ADV_ANY)),
        "bytes.LastIndexAny" => |s| enc_int(bytes::last_index_any(s, ADV_ANY)),
        "bytes.TrimFuncLetter" => |s| bytes::trim_func(s, unicode::is_letter).to_vec(),
        "bytes.TrimSet" => |s| bytes::trim(s, ADV_TRIM_BOTH).to_vec(),
        "bytes.TrimRightSet" => |s| bytes::trim_right(s, ADV_TRIM_R).to_vec(),
        "bytes.IndexRuneError" => |s| enc_int(bytes::index_rune(s, utf8::RUNE_ERROR)),
        "bytes.LastIndexFuncSpace" => |s| enc_int(bytes::last_index_func(s, unicode::is_space)),
        "bytes.SplitEmpty" => |s| enc_list(&bytes::split(s, b"")),
        "bytes.SplitN2Empty" => |s| enc_list(&bytes::split_n(s, b"", 2)),
        _ => return None,
    })
}

fn str_family_line(name: &str) -> Option<(usize, u64)> {
    let (domain, fname) = name.split_once('/')?;
    if !["rune", "ctx1", "ctx2", "ctx3", "bytes"].contains(&domain) {
        return None;
    }
    let f = adv_func(fname)?;
    let mut h = Fnv::new();
    let mut n = 0;
    for_domain(domain, |s| {
        let out = f(s);
        n += out.len();
        h.blob(&out);
    });
    Some((n, h.0))
}

// ---------------------------------------------------------------------------
// foldpair/...

fn fold_partners(r: Rune) -> Vec<Rune> {
    let mut p = orbit(r);
    p.extend_from_slice(&[
        unicode::to_upper(r),
        unicode::to_lower(r),
        unicode::to_title(r),
        unicode::TURKISH_CASE.to_upper(r),
        unicode::TURKISH_CASE.to_lower(r),
        r.wrapping_add(1),
        r.wrapping_sub(1),
        r ^ 0x20,
        r ^ 1,
        0x212A,
        0x17F,
        utf8::RUNE_ERROR,
        'k' as Rune,
        's' as Rune,
    ]);
    p
}

fn fold_strings() -> Vec<Vec<u8>> {
    let mut ss: Vec<Vec<u8>> = (0..=255u8).map(|a| vec![a]).collect();
    let runes: [Rune; 88] = [
        0xB5, 0xC5, 0xDF, 0xE5, 0xFF, 0x130, 0x131, 0x149, 0x17F, 0x178, 0x1C4, 0x1C5, 0x1C6,
        0x1C7, 0x1C8, 0x1C9, 0x1F0, 0x1F1, 0x1F2, 0x1F3, 0x345, 0x390, 0x392, 0x398, 0x399, 0x39A,
        0x39C, 0x3A0, 0x3A1, 0x3A3, 0x3A6, 0x3A9, 0x3B0, 0x3B2, 0x3B8, 0x3B9, 0x3BA, 0x3BC, 0x3C0,
        0x3C1, 0x3C2, 0x3C3, 0x3C6, 0x3C9, 0x3D0, 0x3D1, 0x3D5, 0x3D6, 0x3F0, 0x3F1, 0x3F4, 0x3F5,
        0x412, 0x432, 0x1C80, 0x1C88, 0x1E60, 0x1E61, 0x1E9B, 0x1E9E, 0x1FBE, 0x1FD3, 0x1FE3,
        0x2126, 0x212A, 0x212B, 0x2C2F, 0x2C5F, 0xA64A, 0xA64B, 0xA7AE, 0xAB53, 0xAB70, 0x13A0,
        0xFB05, 0xFB06, 0xFF21, 0xFF41, 0xFFFD, 0x10400, 0x10428, 0x1E900, 0x1E922, 0x10FFFF,
        0x2000, 0x3000, 0x85, 0xA0,
    ];
    for r in runes {
        ss.push(utf8::rune_to_string(r));
    }
    for s in [
        &b"\xc0\xaf"[..],
        b"\xe0\x80\xaf",
        b"\xed\xa0\x80",
        b"\xf4\x90\x80\x80",
        b"\xe2\x84",
        b"\xf0\x9f\x98",
        b"\xc4",
        b"\xce",
        b"\xe1",
        b"\xef\xbf",
        b"Aa",
        b"aA",
        b"kK",
        b"Kk",
        "\u{212a}K".as_bytes(),
        "s\u{17f}".as_bytes(),
        "\u{17f}S".as_bytes(),
        b"\xffa",
        b"a\xff",
        "\u{130}i".as_bytes(),
        "i\u{131}".as_bytes(),
    ] {
        ss.push(s.to_vec());
    }
    ss
}

type FoldFn = fn(&[u8], &[u8]) -> bool;

fn fold_variant(name: &str) -> Option<FoldFn> {
    Some(match name {
        "ab" => |a, b| strings::equal_fold(a, b),
        "ba" => |a, b| strings::equal_fold(b, a),
        "bytes" => |a, b| bytes::equal_fold(a, b),
        "ctx" => |a, b| strings::equal_fold(&cat(&[b"Ab", a, b"\xff"]), &cat(&[b"aB", b, b"\xff"])),
        "long" => |a, b| strings::equal_fold(&cat(&[a, b"x"]), b),
        _ => return None,
    })
}

fn fold_pair_line(name: &str) -> Option<(usize, u64)> {
    let rest = name.strip_prefix("foldpair/")?;
    let (kind, v) = rest.split_once('/')?;
    let f = fold_variant(v)?;
    let mut h = Fnv::new();
    let mut n = 0;
    match kind {
        "rune" => for_all_runes(|r| {
            let a = utf8::rune_to_string(r);
            for x in fold_partners(r) {
                let e = f(&a, &utf8::rune_to_string(x));
                if e {
                    n += 1;
                }
                h.bool(e);
            }
        }),
        "set" => {
            let ss = fold_strings();
            for a in &ss {
                for b in &ss {
                    let e = f(a, b);
                    if e {
                        n += 1;
                    }
                    h.bool(e);
                }
            }
        }
        _ => return None,
    }
    Some((n, h.0))
}

// ---------------------------------------------------------------------------
// long/...

struct XorShift(u64);

impl XorShift {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(2685821657736338717)
    }
    fn intn(&mut self, n: usize) -> usize {
        ((self.next() >> 33) % n as u64) as usize
    }
}

const LONG_PIECES: [&[u8]; 36] = [
    b"a",
    b"b",
    b"ab",
    b"aab",
    b"A",
    b"K",
    b"k",
    b" ",
    b"\t",
    "\u{a0}".as_bytes(),
    "\u{3000}".as_bytes(),
    "\u{446}".as_bytes(),
    "\u{4c6}".as_bytes(),
    "\u{a640}".as_bytes(),
    "\u{a680}".as_bytes(),
    "\u{4680}".as_bytes(),
    "\u{212c0}".as_bytes(),
    "\u{21300}".as_bytes(),
    "\u{23300}".as_bytes(),
    b"\xe2\x98",
    "\u{263a}".as_bytes(),
    b"\xff",
    b"\xed\xa0\x80",
    "\u{212a}".as_bytes(),
    "\u{17f}".as_bytes(),
    "\u{df}".as_bytes(),
    "\u{130}".as_bytes(),
    "\u{131}".as_bytes(),
    "\u{3a3}".as_bytes(),
    "\u{3c2}".as_bytes(),
    "\u{3c3}".as_bytes(),
    "\u{9104}".as_bytes(),
    "\u{bc104}".as_bytes(),
    b"x",
    b"_",
    b".",
];

struct LongCase {
    h: Vec<u8>,
    alpha: Vec<&'static [u8]>,
    needles: Vec<Vec<u8>>,
}

fn long_cases() -> Vec<LongCase> {
    let mut rng = XorShift(0x9E3779B97F4A7C15);
    let mut cs = Vec::new();
    for k in 0..300 {
        let m = 2 + rng.intn(4);
        let alpha: Vec<&'static [u8]> = (0..m)
            .map(|_| LONG_PIECES[rng.intn(LONG_PIECES.len())])
            .collect();
        let max_n = if k >= 250 { 6000 } else { 400 };
        let n = 1 + rng.intn(max_n);
        let mut h = Vec::new();
        for _ in 0..n {
            h.extend_from_slice(alpha[rng.intn(m)]);
        }
        let a = rng.intn(h.len());
        let l = 2 + rng.intn(11);
        let n0 = cat(&[alpha[0], alpha[1]]);
        let n1 = h[a..(a + l).min(h.len())].to_vec();
        let (x, y, z) = (rng.intn(m), rng.intn(m), rng.intn(m));
        let n2 = cat(&[alpha[x], alpha[y], alpha[z]]);
        let n3 = cat(&[b"Q", alpha[0]]);
        cs.push(LongCase {
            h,
            alpha,
            needles: vec![n0, n1, n2, n3],
        });
    }
    cs
}

fn long_runes() -> Vec<Rune> {
    let mut rs: Vec<Rune> = LONG_PIECES
        .iter()
        .map(|p| utf8::decode_rune_in_string(p).0)
        .collect();
    rs.extend_from_slice(&[-1, 0xD800, utf8::MAX_RUNE + 1, 'Q' as Rune]);
    rs
}

fn long_func(name: &str, c: &LongCase, h: &mut Fnv) -> Option<()> {
    use go_unicode::replacer::Replacer;
    let s = c.h.as_slice();
    match name {
        "IndexRune" => {
            for r in long_runes() {
                h.i32(strings::index_rune(s, r) as i32);
                h.i32(bytes::index_rune(s, r) as i32);
            }
        }
        "Index" => {
            for n in &c.needles {
                h.i32(strings::index(s, n) as i32);
                h.i32(strings::last_index(s, n) as i32);
                h.i32(strings::count(s, n) as i32);
                h.i32(bytes::index(s, n) as i32);
                h.i32(bytes::last_index(s, n) as i32);
                h.i32(bytes::count(s, n) as i32);
            }
        }
        "IndexAny" => {
            let set = c.alpha[1..].concat();
            for cs in [&set[..], c.alpha[0], b"Q\xff", "\u{a0}\u{3000}".as_bytes()] {
                h.i32(strings::index_any(s, cs) as i32);
                h.i32(strings::last_index_any(s, cs) as i32);
                h.i32(bytes::index_any(s, cs) as i32);
                h.i32(bytes::last_index_any(s, cs) as i32);
            }
        }
        "Replace" => {
            for n in &c.needles {
                h.blob(&strings::replace(s, n, b"Z", -1));
                h.blob(&strings::replace(s, n, b"", 3));
                h.blob(&bytes::replace(s, n, "\u{e9}".as_bytes(), -1));
            }
        }
        "Split" => {
            for n in &c.needles {
                h.blob(&enc_list(&strings::split_n(s, n, 5)));
                h.i32(strings::split(s, n).len() as i32);
                h.i32(strings::split_after(s, n).len() as i32);
                h.i32(bytes::split(s, n).len() as i32);
            }
        }
        "Replacer" => {
            let mut on: Vec<Vec<u8>> = Vec::new();
            for (i, a) in c.alpha.iter().enumerate() {
                on.push(a.to_vec());
                on.push(b"<".repeat(i));
            }
            h.blob(&Replacer::new(&on).replace(s));
            if c.needles[1].len() > 1 {
                h.blob(&Replacer::new(&[&c.needles[1][..], b"[m]"]).replace(s));
            }
            if c.needles[0].len() > 1 {
                h.blob(&Replacer::new(&[&c.needles[0][..], b""]).replace(s));
            }
            let head = &s[..s.len().min(300)];
            h.blob(&Replacer::new(&[&b""[..], b"|", &c.needles[0], b"#"]).replace(head));
        }
        "Case" => {
            h.blob(&strings::to_upper(s));
            h.blob(&strings::to_lower(s));
            h.blob(&strings::title(s));
            h.blob(&bytes::to_title(s));
            h.bool(strings::equal_fold(s, &strings::to_upper(s)));
            h.bool(strings::equal_fold(&strings::to_lower(s), s));
            h.bool(bytes::equal_fold(s, &strings::to_title(s)));
        }
        "Space" => {
            h.blob(strings::trim_space(s));
            h.blob(&enc_list(&strings::fields(s)));
            h.blob(bytes::trim_space(s));
            h.blob(&enc_list(&bytes::fields(s)));
            h.blob(&strings::to_valid_utf8(s, b"?"));
            h.blob(&strings::map(map_fold, s));
        }
        _ => return None,
    }
    Some(())
}

fn long_line(name: &str) -> Option<(usize, u64)> {
    let fname = name.strip_prefix("long/")?;
    let mut h = Fnv::new();
    let mut n = 0;
    for c in long_cases() {
        long_func(fname, &c, &mut h)?;
        n += c.h.len();
    }
    Some((n, h.0))
}

// ---------------------------------------------------------------------------

fn compute(name: &str) -> Option<(usize, u64)> {
    if name.starts_with("long/") {
        return long_line(name);
    }
    if name.starts_with("struct/") {
        return struct_line(name);
    }
    if name == "orbit/all" {
        return Some(orbit_line());
    }
    if name.starts_with("foldpair/") {
        return fold_pair_line(name);
    }
    str_family_line(name)
}

fn load() -> Vec<(String, usize, u64)> {
    let path = format!(
        "{}/tests/fixtures/adversarial_hashes.txt",
        env!("CARGO_MANIFEST_DIR")
    );
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

#[test]
fn adversarial_against_go() {
    let lines = load();
    assert!(lines.len() > 810, "fixture has {} lines", lines.len());
    // Sanity checks that the mirrored domains match Go's sizes.
    assert_eq!(fold_strings().len(), 256 + 88 + 21);
    let failures = std::sync::Mutex::new(Vec::new());
    let next = std::sync::atomic::AtomicUsize::new(0);
    // Heaviest lines first (they are listed after the cheap struct lines).
    let mut order: Vec<usize> = (0..lines.len()).collect();
    order.sort_by_key(|&i| {
        let n = &lines[i].0;
        if n.starts_with("rune/") || n.starts_with("foldpair/rune/") {
            0
        } else if n.starts_with("struct/") {
            2
        } else {
            1
        }
    });
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    std::thread::scope(|sc| {
        for _ in 0..threads {
            sc.spawn(|| {
                loop {
                    let k = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if k >= order.len() {
                        break;
                    }
                    let (name, count, hash) = &lines[order[k]];
                    match compute(name) {
                        None => failures.lock().unwrap().push(format!("{name}: unknown line")),
                        Some(got) if got != (*count, *hash) => {
                            failures.lock().unwrap().push(format!(
                                "{name}: got count {} hash {:016x}, want count {count} hash {hash:016x}",
                                got.0, got.1
                            ))
                        }
                        Some(_) => {}
                    }
                }
            });
        }
    });
    let mut failures = failures.into_inner().unwrap();
    failures.sort();
    assert!(
        failures.is_empty(),
        "{} of {} lines differ from Go:\n{}",
        failures.len(),
        lines.len(),
        failures.join("\n")
    );
}

/// The `&str` conveniences of `strings` agree with the byte functions they
/// wrap (every ctx1 string is valid UTF-8).
#[test]
fn str_wrappers_match_byte_functions() {
    let mut n = 0;
    for_domain("ctx1", |b| {
        let s = std::str::from_utf8(b).expect("ctx1 strings are valid UTF-8");
        assert_eq!(strings::to_lower_str(s).as_bytes(), &*strings::to_lower(b));
        assert_eq!(strings::to_upper_str(s).as_bytes(), &*strings::to_upper(b));
        assert_eq!(strings::to_title_str(s).as_bytes(), &*strings::to_title(b));
        assert_eq!(strings::title_str(s).as_bytes(), &*strings::title(b));
        assert_eq!(
            strings::map_str(map_fold, s).as_bytes(),
            &*strings::map(map_fold, b)
        );
        assert_eq!(
            strings::trim_space_str(s).as_bytes(),
            strings::trim_space(b)
        );
        assert_eq!(
            strings::trim_func_str(s, unicode::is_letter).as_bytes(),
            strings::trim_func(b, unicode::is_letter)
        );
        assert_eq!(
            strings::trim_left_func_str(s, unicode::is_space).as_bytes(),
            strings::trim_left_func(b, unicode::is_space)
        );
        assert_eq!(
            strings::trim_right_func_str(s, unicode::is_space).as_bytes(),
            strings::trim_right_func(b, unicode::is_space)
        );
        let f: Vec<&[u8]> = strings::fields_str(s)
            .iter()
            .map(|x| x.as_bytes())
            .collect();
        assert_eq!(f, strings::fields(b));
        let f: Vec<&[u8]> = strings::fields_func_str(s, unicode::is_punct)
            .iter()
            .map(|x| x.as_bytes())
            .collect();
        assert_eq!(f, strings::fields_func(b, unicode::is_punct));
        let up = strings::to_upper_str(s);
        assert_eq!(
            strings::equal_fold_str(s, &up),
            strings::equal_fold(b, up.as_bytes())
        );
        n += 1;
    });
    assert!(n > 140_000);
}
