//! Differential tests against fixtures written by
//! `go run ./tools/go-oracle/xtext-collate fixtures`.

mod common;

use common::*;
use sha2::{Digest, Sha256};
use xtext_collate::language::{self, Confidence};
use xtext_collate::norm::Form;
use xtext_collate::{Buffer, Collator};

fn read(name: &str) -> String {
    std::fs::read_to_string(fixtures_dir().join(name)).unwrap()
}

fn conf_name(c: Confidence) -> &'static str {
    match c {
        Confidence::No => "No",
        Confidence::Low => "Low",
        Confidence::High => "High",
        Confidence::Exact => "Exact",
    }
}

fn decode_input(s: &str) -> String {
    match s.strip_prefix("hex:") {
        Some(h) => String::from_utf8(hex_decode(h)).unwrap(),
        None => s.to_string(),
    }
}

#[test]
fn tags_match_go() {
    check_tag_lines(&read("tags.tsv"));
}

/// Fuzzed tags (tools/go-oracle/xtext-collate tagfuzz), including the
/// regressions: duplicate -u keys / duplicate variants followed by another
/// extension (Go reads the shifted buffer through its stale token slice; the
/// port used to panic) and `-u-rg-XXzzzz` compact tags.
#[test]
fn fuzzed_tags_match_go() {
    check_tag_lines(&read("tags-fuzz.tsv"));
}

/// Tag lines written by `go run ./tools/go-oracle/xtext-collate tagfuzz`
/// (same format as tags.tsv);
/// `XTEXT_COLLATE_TAGS=FILE cargo test --release -- --ignored big_tags`.
#[test]
#[ignore]
fn big_tags() {
    let Ok(paths) = std::env::var("XTEXT_COLLATE_TAGS") else {
        eprintln!("XTEXT_COLLATE_TAGS not set");
        return;
    };
    for p in paths.split(',') {
        check_tag_lines(&std::fs::read_to_string(p).unwrap());
    }
}

fn check_tag_lines(data: &str) {
    let probes = probe_strings();
    let mut n = 0;
    let mut fails = Vec::new();
    for line in data.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        assert_eq!(f.len(), 10, "bad line {line:?}");
        let input = decode_input(f[0]);
        let caught = std::panic::catch_unwind(|| {
            let _ = language::parse(&input);
            let _ = language::DEFAULT.parse(&input);
            language::make(&input)
        });
        if caught.is_err() {
            eprintln!("PANIC on input {input:?}");
            fails.push(format!("PANIC on input {input:?}"));
            continue;
        }
        let mk = language::make(&input);
        let ms = mk.string();
        let same = |x: String| if x == ms { "=".to_string() } else { x };
        let (perr, pstr) = match language::parse(&input) {
            Ok(t) => (String::new(), t.string()),
            Err(e) => {
                // Go returns the partial tag with the error.
                let partial = match language::DEFAULT.parse(&input) {
                    Err((t, _)) => t.string(),
                    Ok(t) => t.string(),
                };
                (e.to_string(), partial)
            }
        };
        let all = language::ALL.canonicalize(&mk).string();
        let (b, conf) = mk.base();
        let parent = mk.parent().string();
        let idx = Collator::match_index(&mk);
        let mut c = Collator::from_tag(&mk, &[]);
        let mut h = Sha256::new();
        for p in &probes {
            let k = c.key_vec(p.as_bytes());
            h.update(&k);
            h.update([0xff]);
            h.update([(c.compare_string(p, "a") + 1) as u8]);
        }
        let ph = hex_encode(&h.finalize()[..4]);
        let got = [
            f[0].to_string(),
            perr,
            same(pstr),
            ms.clone(),
            same(all),
            b.to_string(),
            conf_name(conf).to_string(),
            same(parent),
            idx.to_string(),
            ph,
        ]
        .join("\t");
        n += 1;
        if got != line {
            fails.push(format!("want {line:?}\n got {got:?}"));
        }
    }
    if !fails.is_empty() {
        let k = fails.len();
        panic!(
            "{k}/{n} tag lines differ; first ones:\n{}",
            fails[..k.min(30)].join("\n")
        );
    }
    eprintln!("tags: {n} lines match");
}

fn probe_strings() -> Vec<String> {
    // Must equal probeStrings in the oracle.
    [
        "a",
        "A",
        "ä",
        "Ä",
        "å",
        "aa",
        "Aa",
        "ae",
        "æ",
        "b",
        "c",
        "č",
        "ch",
        "Ch",
        "CH",
        "cs",
        "ç",
        "d",
        "dz",
        "dž",
        "ǆ",
        "đ",
        "ð",
        "e",
        "é",
        "è",
        "ê",
        "ë",
        "ə",
        "f",
        "g",
        "ğ",
        "gy",
        "h",
        "i",
        "ı",
        "İ",
        "î",
        "j",
        "k",
        "l",
        "ł",
        "ll",
        "ŀl",
        "l·l",
        "lj",
        "m",
        "n",
        "ñ",
        "ng",
        "nj",
        "ny",
        "o",
        "ö",
        "ø",
        "ő",
        "œ",
        "oe",
        "p",
        "q",
        "r",
        "ř",
        "rr",
        "s",
        "ş",
        "š",
        "ß",
        "ss",
        "sz",
        "t",
        "th",
        "þ",
        "u",
        "ü",
        "ű",
        "v",
        "w",
        "x",
        "y",
        "ÿ",
        "z",
        "ž",
        "ʒ",
        "ё",
        "е",
        "й",
        "и",
        "ї",
        "і",
        "ґ",
        "г",
        "ў",
        "у",
        "ع",
        "غ",
        "ه",
        "ی",
        "ي",
        "क",
        "क्ष",
        "ক",
        "ত",
        "ৎ",
        "ಕ",
        "ක",
        "ཀ",
        "ཀྵ",
        "ក",
        "က",
        "ა",
        "ㄱ",
        "가",
        "각",
        "あ",
        "ア",
        "ｱ",
        "ゃ",
        "中",
        "丁",
        "𠀀",
        "一",
        "乙",
        "ﬃ",
        "Ⅳ",
        "①",
        "½",
        "12",
        "2",
        "10",
        "a b",
        "a-b",
        "a_b",
        "-",
        " ",
        "'",
        "’",
        "",
        "ก",
        "ข",
        "เก",
        "กา",
        "ไก",
        "ฤ",
        "ๆ",
        "ฯ",
        "๑",
        "1",
        "١",
        "٠",
        "α",
        "ά",
        "ω",
        "а",
        "я",
        "ﾀ",
        "ｶﾞ",
        "ガ",
        "カ",
        "か",
        "が",
        "ǅ",
        "Ǆ",
        "ĳ",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

struct Digests {
    n: usize,
    nloc: usize,
    rune_step: u32,
    nstress: usize,
    lines: Vec<Vec<String>>,
}

fn digests() -> Digests {
    let data = read("digests.txt");
    let mut d = Digests {
        n: 0,
        nloc: 0,
        rune_step: 1,
        nstress: 0,
        lines: Vec::new(),
    };
    for line in data.lines() {
        let f: Vec<String> = line.split('\t').map(|s| s.to_string()).collect();
        if f[0] == "N" {
            d.n = f[1].parse().unwrap();
            d.nloc = f[2].parse().unwrap();
            d.rune_step = f[3].parse().unwrap();
            d.nstress = f[4].parse().unwrap();
        } else {
            d.lines.push(f);
        }
    }
    d
}

fn check_random(kind: &str, filter: impl Fn(&str) -> bool) {
    let d = digests();
    let main = gen_corpus(1, d.n);
    let loc = gen_corpus(2, d.nloc);
    let stress = gen_stress_corpus(3, d.nstress);
    let mut ok = 0;
    let mut fails = Vec::new();
    for f in &d.lines {
        if f[0] != kind || !filter(&f[1]) {
            continue;
        }
        let (name, tag, opts) = (&f[1], &f[2], &f[3]);
        let corpus = if kind == "S" {
            &stress
        } else if name.starts_with("loc:") {
            &loc
        } else {
            &main
        };
        let mut c = config_collator(tag, opts);
        let (kd, cd) = key_digest(&mut c, corpus);
        if kd == f[4] && cd == f[5] {
            ok += 1;
        } else {
            fails.push(format!(
                "{name}: keys {} cmp {}",
                if kd == f[4] { "ok" } else { "DIFF" },
                if cd == f[5] { "ok" } else { "DIFF" }
            ));
        }
    }
    assert!(
        fails.is_empty(),
        "{} configs differ ({ok} ok):\n{}",
        fails.len(),
        fails.join("\n")
    );
    eprintln!("{ok} random-corpus configs match");
}

#[test]
fn random_corpus_main_configs() {
    check_random("R", |name| !name.starts_with("loc:"));
}

#[test]
fn random_corpus_every_locale() {
    check_random("R", |name| name.starts_with("loc:"));
}

#[test]
fn combining_mark_stress_all_configs() {
    check_random("S", |_| true);
}

/// Large corpora written by `go run ./tools/go-oracle/xtext-collate corpus
/// -out FILE ...` outside the repo; run with
/// `XTEXT_COLLATE_CORPUS=FILE cargo test --release -- --ignored big_corpus`.
#[test]
#[ignore]
fn big_corpus() {
    let Ok(path) = std::env::var("XTEXT_COLLATE_CORPUS") else {
        eprintln!("XTEXT_COLLATE_CORPUS not set");
        return;
    };
    for path in path.split(',') {
        let data = std::fs::read_to_string(path).unwrap();
        let mut strs = Vec::new();
        let (mut ok, mut bad) = (0, Vec::new());
        let mut hashes: Vec<(usize, String, String)> = Vec::new();
        for line in data.lines() {
            let f: Vec<&str> = line.split('\t').collect();
            match f[0] {
                "N" => {
                    let n: usize = f[1].parse().unwrap();
                    let seed: u64 = f[2].parse().unwrap();
                    strs = match f[3] {
                        "random" => gen_corpus(seed, n),
                        "stress" => gen_stress_corpus(seed, n),
                        k => panic!("kind {k}"),
                    };
                }
                "H" => hashes.push((f[1].parse().unwrap(), f[2].to_string(), f[3].to_string())),
                "R" => {
                    let mut c = config_collator(f[2], f[3]);
                    for (i, h, hx) in &hashes {
                        assert_eq!(
                            hex_encode(&strs[*i]),
                            *hx,
                            "corpus generator mismatch at {i}"
                        );
                        let got = key_hash(&c.key_vec(&strs[*i]));
                        if got != *h && bad.len() < 20 {
                            bad.push(format!(
                                "{}: string {i} {:?}",
                                f[1],
                                String::from_utf8_lossy(&strs[*i])
                            ));
                        }
                    }
                    hashes.clear();
                    let (kd, cd) = key_digest(&mut c, &strs);
                    if kd == f[4] && cd == f[5] {
                        ok += 1;
                    } else {
                        bad.push(format!("{}: keys {} cmp {}", f[1], kd == f[4], cd == f[5]));
                    }
                }
                "P" => {
                    let mut c = config_collator(f[2], f[3]);
                    let mut h = Sha256::new();
                    let mut buf = Buffer::new();
                    for r in 0..=0x10FFFFu32 {
                        if (0xD800..=0xDFFF).contains(&r) {
                            continue;
                        }
                        buf.reset();
                        let s = char::from_u32(r).unwrap().to_string();
                        let k = c.key_from_string(&mut buf, &s);
                        put_uvarint(&mut h, k.len() as u64);
                        h.update(k);
                    }
                    if hex_encode(&h.finalize()) == f[4] {
                        ok += 1;
                    } else {
                        bad.push(format!("{}: per-rune keys differ", f[1]));
                    }
                }
                _ => {}
            }
        }
        eprintln!(
            "{path}: {} strings, {ok} digests match, {} problems",
            strs.len(),
            bad.len()
        );
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }
}

#[test]
fn per_rune_keys() {
    let d = digests();
    for f in &d.lines {
        if f[0] != "P" {
            continue;
        }
        let mut c = Collator::new(&f[1]);
        let mut h = Sha256::new();
        let mut buf = Buffer::new();
        let mut r = 0u32;
        while r <= 0x10FFFF {
            if !(0xD800..=0xDFFF).contains(&r) {
                buf.reset();
                let s = char::from_u32(r).unwrap().to_string();
                let k = c.key_from_string(&mut buf, &s);
                put_uvarint(&mut h, k.len() as u64);
                h.update(k);
            }
            r += d.rune_step;
        }
        assert_eq!(
            hex_encode(&h.finalize()),
            f[2],
            "per-rune keys for {}",
            f[1]
        );
    }
}

fn norm_emit(h: &mut Sha256, s: &[u8]) {
    let b2i = |b: bool| b as i64;
    let p = Form::Nfd.properties(s);
    put_varint(h, p.size() as i64);
    put_varint(h, p.ccc() as i64);
    put_varint(h, p.lead_ccc() as i64);
    put_varint(h, p.trail_ccc() as i64);
    put_varint(h, b2i(p.boundary_before()));
    put_varint(h, b2i(p.boundary_after()));
    let d = p.decomposition();
    put_varint(h, d.len() as i64);
    h.update(d);
    let k = Form::Nfkd.properties(s);
    put_varint(h, k.size() as i64);
    let kd = k.decomposition();
    put_varint(h, kd.len() as i64);
    h.update(kd);
    put_varint(h, Form::Nfd.first_boundary(s) as i64);
}

#[test]
fn norm_properties_match_go() {
    let d = digests();
    let f = d.lines.iter().find(|f| f[0] == "M").unwrap();
    let mut h = Sha256::new();
    let mut n = 0usize;
    for r in 0..=0x10FFFFu32 {
        if (0xD800..=0xDFFF).contains(&r) {
            continue;
        }
        let s = char::from_u32(r).unwrap().to_string();
        norm_emit(&mut h, s.as_bytes());
        n += 1;
    }
    for a in 0x80..0x100u32 {
        norm_emit(&mut h, &[a as u8]);
        n += 1;
        for b in 0..0x100u32 {
            norm_emit(&mut h, &[a as u8, b as u8]);
            n += 1;
        }
    }
    for s in gen_corpus(0xB0B, 50000) {
        if s.is_empty() {
            continue;
        }
        norm_emit(&mut h, &s);
        n += 1;
    }
    assert_eq!(n.to_string(), f[1], "norm item count");
    assert_eq!(hex_encode(&h.finalize()), f[2], "norm properties digest");
}

#[test]
fn site_strings_match_go() {
    let strs: Vec<Vec<u8>> = read("site-strings.hex").lines().map(hex_decode).collect();
    let data = read("site-keys.txt");
    let mut lines = data.lines();
    let mut configs = 0;
    while let Some(cl) = lines.next() {
        let f: Vec<&str> = cl.split('\t').collect();
        assert_eq!(f[0], "C");
        let (name, tag, opts) = (f[1], f[2], f.get(3).copied().unwrap_or(""));
        let mut c = config_collator(tag, opts);

        let k = lines.next().unwrap().strip_prefix("K\t").unwrap();
        let want: Vec<&str> = k.split(' ').collect();
        assert_eq!(want.len(), strs.len());
        let mut bad = Vec::new();
        for (i, s) in strs.iter().enumerate() {
            let got = key_hash(&c.key_vec(s));
            if got != want[i] {
                bad.push(format!("{i}: {:?}", String::from_utf8_lossy(s)));
            }
        }
        assert!(
            bad.is_empty(),
            "{name}: {} keys differ: {:?}",
            bad.len(),
            &bad[..bad.len().min(20)]
        );

        let o = lines.next().unwrap().strip_prefix("O\t").unwrap();
        let want_order: Vec<usize> = o.split(' ').map(|x| x.parse().unwrap()).collect();
        let mut idx: Vec<usize> = (0..strs.len()).collect();
        go_sort_stable(&mut idx, |&i, &j| c.compare(&strs[i], &strs[j]) < 0);
        assert_eq!(idx, want_order, "{name}: sort.SliceStable order");

        let dl = lines.next().unwrap();
        let df: Vec<&str> = dl.split('\t').collect();
        let (kd, cd) = key_digest(&mut c, &strs);
        assert_eq!(
            (kd.as_str(), cd.as_str()),
            (df[1], df[2]),
            "{name}: digests"
        );
        configs += 1;
    }
    eprintln!(
        "site strings: {} strings x {configs} configs match",
        strs.len()
    );
}

fn go_sort_stable<T, F: FnMut(&T, &T) -> bool>(x: &mut [T], less: F) {
    go_sort::stable_by(x, less);
}

/// Checks a pairs file written by `go run ./tools/go-oracle/xtext-collate
/// pairs` (adversarial near-equal pairs, see pairs.go): per-pair Compare
/// results (both directions) and key digests for every config.
fn check_pairs_file(path: &std::path::Path) -> (usize, usize) {
    let data = std::fs::read_to_string(path).unwrap();
    let mut pairs: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
    let mut bad = Vec::new();
    let mut configs = 0;
    for line in data.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        match f[0] {
            "P" => pairs.push((hex_decode(f[1]), hex_decode(f[2]))),
            "C" => {
                let (name, tag, opts) = (f[1], f[2], f[3]);
                let mut c = config_collator(tag, opts);
                let mut res = Vec::with_capacity(2 * pairs.len());
                let mut hk = Sha256::new();
                for (a, b) in &pairs {
                    res.push(b'1'.wrapping_add(c.compare(a, b) as u8));
                    res.push(b'1'.wrapping_add(c.compare(b, a) as u8));
                    for s in [a, b] {
                        let k = c.key_vec(s);
                        put_uvarint(&mut hk, k.len() as u64);
                        hk.update(&k);
                    }
                }
                let rd = hex_encode(&Sha256::digest(&res));
                let kd = hex_encode(&hk.finalize());
                configs += 1;
                if rd != f[4] {
                    let mut msg = format!("{name}: compare results differ");
                    if let Some(want) = f.get(6) {
                        let want = want.as_bytes();
                        let mut shown = 0;
                        for (i, (a, b)) in pairs.iter().enumerate() {
                            for d in 0..2 {
                                if res[2 * i + d] != want[2 * i + d] && shown < 10 {
                                    shown += 1;
                                    msg += &format!(
                                        "\n  pair {i} dir {d}: got {} want {}: {:?} vs {:?} ({} / {})",
                                        res[2 * i + d] as char,
                                        want[2 * i + d] as char,
                                        String::from_utf8_lossy(a),
                                        String::from_utf8_lossy(b),
                                        hex_encode(a),
                                        hex_encode(b)
                                    );
                                }
                            }
                        }
                    }
                    bad.push(msg);
                }
                if kd != f[5] {
                    bad.push(format!("{name}: key digest differs"));
                }
            }
            _ => {}
        }
    }
    assert!(
        bad.is_empty(),
        "{} problems ({configs} configs):\n{}",
        bad.len(),
        bad.join("\n")
    );
    (pairs.len(), configs)
}

#[test]
fn adversarial_pairs_match_go() {
    for name in ["pairs.txt", "pairs-numeric.txt"] {
        let (n, c) = check_pairs_file(&fixtures_dir().join(name));
        eprintln!("{name}: {n} adversarial pairs x {c} configs match");
    }
}

/// `XTEXT_COLLATE_PAIRS=FILE[,FILE] cargo test --release -- --ignored big_pairs`.
#[test]
#[ignore]
fn big_pairs() {
    let Ok(paths) = std::env::var("XTEXT_COLLATE_PAIRS") else {
        eprintln!("XTEXT_COLLATE_PAIRS not set");
        return;
    };
    for p in paths.split(',') {
        let (n, c) = check_pairs_file(std::path::Path::new(p));
        eprintln!("{p}: {n} pairs x {c} configs match");
    }
}

/// Checks an enumeration file written by `go run
/// ./tools/go-oracle/xtext-collate enum` (every string of length <= maxlen
/// over the alphabet in the file): key digest and comparison digest per
/// config; with `H` lines, localises the first differing key.
fn check_enum_file(path: &std::path::Path) -> (usize, usize) {
    let data = std::fs::read_to_string(path).unwrap();
    let mut strs: Vec<Vec<u8>> = Vec::new();
    let mut bad = Vec::new();
    let mut configs = 0;
    let mut last: Option<Collator> = None;
    for line in data.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        match f[0] {
            "E" => {
                let max_len: usize = f[1].parse().unwrap();
                let alpha: Vec<Vec<u8>> = f[2].split(',').map(hex_decode).collect();
                strs.clear();
                fn rec(out: &mut Vec<Vec<u8>>, alpha: &[Vec<u8>], prefix: &mut Vec<u8>, n: usize) {
                    if n == 0 {
                        out.push(prefix.clone());
                        return;
                    }
                    for u in alpha {
                        let l = prefix.len();
                        prefix.extend_from_slice(u);
                        rec(out, alpha, prefix, n - 1);
                        prefix.truncate(l);
                    }
                }
                for l in 0..=max_len {
                    rec(&mut strs, &alpha, &mut Vec::new(), l);
                }
            }
            "C" => {
                let (name, tag, opts) = (f[1], f[2], f[3]);
                let mut c = config_collator(tag, opts);
                let mut hk = Sha256::new();
                for s in &strs {
                    let k = c.key_vec(s);
                    put_uvarint(&mut hk, k.len() as u64);
                    hk.update(&k);
                }
                let mut hc = Sha256::new();
                let n = strs.len();
                let mut res = Vec::with_capacity(4096);
                for i in 0..n {
                    let a = &strs[i];
                    let b = &strs[(i + 1) % n];
                    let d = &strs[(i * 7919 + 13) % n];
                    res.push((c.compare(a, b) + 1) as u8);
                    res.push((c.compare(b, a) + 1) as u8);
                    res.push((c.compare(a, d) + 1) as u8);
                    if res.len() >= 4096 {
                        hc.update(&res);
                        res.clear();
                    }
                }
                hc.update(&res);
                let kd = hex_encode(&hk.finalize());
                let cd = hex_encode(&hc.finalize());
                configs += 1;
                if kd != f[4] || cd != f[5] {
                    bad.push(format!(
                        "{name}: keys {} cmp {}",
                        if kd == f[4] { "ok" } else { "DIFF" },
                        if cd == f[5] { "ok" } else { "DIFF" }
                    ));
                }
                last = Some(c);
            }
            "H" => {
                let i: usize = f[2].parse().unwrap();
                let c = last.as_mut().unwrap();
                let got = key_hash(&c.key_vec(&strs[i]));
                if got != f[3] && bad.len() < 40 {
                    bad.push(format!(
                        "{}: string {i} {:?} ({}) key differs",
                        f[1],
                        String::from_utf8_lossy(&strs[i]),
                        hex_encode(&strs[i])
                    ));
                }
            }
            _ => {}
        }
    }
    assert!(
        bad.is_empty(),
        "{} problems ({configs} configs):\n{}",
        bad.len(),
        bad.join("\n")
    );
    (strs.len(), configs)
}

#[test]
fn exhaustive_enumerations_match_go() {
    for name in ["enum-thai.txt", "enum-latin.txt", "enum-marks.txt"] {
        let (n, c) = check_enum_file(&fixtures_dir().join(name));
        eprintln!("{name}: {n} strings x {c} configs match");
    }
}

/// `XTEXT_COLLATE_ENUM=FILE[,FILE] cargo test --release -- --ignored big_enum`.
#[test]
#[ignore]
fn big_enum() {
    let Ok(paths) = std::env::var("XTEXT_COLLATE_ENUM") else {
        eprintln!("XTEXT_COLLATE_ENUM not set");
        return;
    };
    for p in paths.split(',') {
        let (n, c) = check_enum_file(std::path::Path::new(p));
        eprintln!("{p}: {n} strings x {c} configs match");
    }
}

/// Lines written by `go run ./tools/go-oracle/xtext-collate settype`:
/// `Tag.SetTypeForKey` over fuzzed tags (Go's in-place scanner aliasing
/// included).
fn check_settype_lines(data: &str) -> usize {
    let mut n = 0;
    let mut fails = Vec::new();
    for line in data.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        assert_eq!(f.len(), 6, "bad line {line:?}");
        let input = decode_input(f[0]);
        let value = if f[2].is_empty() {
            String::new()
        } else {
            decode_input(f[2])
        };
        let t = language::make(&input);
        let (res, err) = t.set_type_for_key(f[1], &value);
        let es = err.map(|e| e.to_string()).unwrap_or_default();
        let got = [
            f[0].to_string(),
            f[1].to_string(),
            f[2].to_string(),
            res.string(),
            es,
            res.type_for_key(f[1]),
        ]
        .join("\t");
        n += 1;
        if got != line && fails.len() < 30 {
            fails.push(format!("want {line:?}\n got {got:?}"));
        }
    }
    assert!(
        fails.is_empty(),
        "SetTypeForKey differs:\n{}",
        fails.join("\n")
    );
    n
}

#[test]
fn set_type_for_key_matches_go() {
    let n = check_settype_lines(&read("settype.tsv"));
    eprintln!("settype: {n} lines match");
}

/// `XTEXT_COLLATE_SETTYPE=FILE cargo test --release -- --ignored big_settype`.
#[test]
#[ignore]
fn big_settype() {
    let Ok(paths) = std::env::var("XTEXT_COLLATE_SETTYPE") else {
        eprintln!("XTEXT_COLLATE_SETTYPE not set");
        return;
    };
    for p in paths.split(',') {
        let n = check_settype_lines(&std::fs::read_to_string(p).unwrap());
        eprintln!("{p}: {n} lines match");
    }
}
