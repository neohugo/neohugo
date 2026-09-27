//! Differential test of the strings/bytes/utf8/utf16/Replacer ports against
//! go1.27.1 on random byte strings (with invalid UTF-8, case-mapping edge
//! cases, Unicode spaces, ...). Vectors are written by
//! `tools/go-oracle/go-unicode fixtures` into
//! `tests/fixtures/strings_vectors.bin.gz`; set `GO_UNICODE_VECTORS` to a
//! larger vector file (same format) to run a bigger corpus.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::io::Read;

use go_unicode::replacer::Replacer;
use go_unicode::{self as unicode, Rune, bytes, strings, utf8, utf16};

struct Rec {
    name: String,
    args: Vec<Vec<u8>>,
    result: Vec<u8>,
}

fn read_u32(p: &[u8], pos: &mut usize) -> usize {
    let v = u32::from_le_bytes(p[*pos..*pos + 4].try_into().unwrap()) as usize;
    *pos += 4;
    v
}

fn read_bytes(p: &[u8], pos: &mut usize) -> Vec<u8> {
    let n = read_u32(p, pos);
    let v = p[*pos..*pos + n].to_vec();
    *pos += n;
    v
}

fn load() -> Vec<Rec> {
    let path = std::env::var("GO_UNICODE_VECTORS").unwrap_or_else(|_| {
        format!(
            "{}/tests/fixtures/strings_vectors.bin.gz",
            env!("CARGO_MANIFEST_DIR")
        )
    });
    load_path(&path)
}

fn load_path(path: &str) -> Vec<Rec> {
    let f = std::fs::File::open(path).unwrap();
    let mut data = Vec::new();
    flate2::read::GzDecoder::new(f)
        .read_to_end(&mut data)
        .unwrap();
    let mut recs = Vec::new();
    let mut pos = 0;
    while pos < data.len() {
        let name = String::from_utf8(read_bytes(&data, &mut pos)).unwrap();
        let nargs = data[pos] as usize;
        pos += 1;
        let args = (0..nargs).map(|_| read_bytes(&data, &mut pos)).collect();
        let result = read_bytes(&data, &mut pos);
        recs.push(Rec { name, args, result });
    }
    recs
}

fn enc_int(i: isize) -> Vec<u8> {
    i.to_string().into_bytes()
}

fn enc_usize(i: usize) -> Vec<u8> {
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

fn parse_int(a: &[u8]) -> isize {
    std::str::from_utf8(a).unwrap().parse().unwrap()
}

fn pred(name: &[u8]) -> fn(Rune) -> bool {
    match name {
        b"IsSpace" => unicode::is_space,
        b"IsPunct" => unicode::is_punct,
        b"IsLetter" => unicode::is_letter,
        b"IsDigit" => unicode::is_digit,
        b"IsUpper" => unicode::is_upper,
        b"IsError" => |r| r == utf8::RUNE_ERROR,
        _ => panic!("unknown predicate {}", String::from_utf8_lossy(name)),
    }
}

fn mapping(name: &[u8]) -> fn(Rune) -> Rune {
    match name {
        b"identity" => |r| r,
        b"weird" => |r| {
            if ('0' as Rune..='9' as Rune).contains(&r) {
                -1
            } else if r == 'a' as Rune {
                0xE4
            } else if r == utf8::RUNE_ERROR {
                '?' as Rune
            } else if r == 'x' as Rune {
                0xD800
            } else if r == 'y' as Rune {
                0x110000
            } else if r == 'b' as Rune {
                utf8::RUNE_ERROR
            } else if r == ' ' as Rune {
                -5
            } else {
                unicode::to_upper(r)
            }
        },
        b"toError" => |r| if r >= 0x80 { utf8::RUNE_ERROR } else { r },
        _ => panic!("unknown mapping {}", String::from_utf8_lossy(name)),
    }
}

/// Checks that a Cow result borrows its input exactly when Go would return
/// the input unchanged (same bytes).
#[allow(clippy::ptr_arg)]
fn cow_ok(c: &Cow<'_, [u8]>, input: &[u8]) -> bool {
    match c {
        Cow::Borrowed(b) => *b == input,
        Cow::Owned(_) => true,
    }
}

fn run(r: &Rec) -> Vec<u8> {
    let a = &r.args;
    let s = a[0].as_slice();
    let arg = |i: usize| a[i].as_slice();
    match r.name.as_str() {
        "utf8.RuneCount" => enc_usize(utf8::rune_count(s)),
        "utf8.RuneCountInString" => enc_usize(utf8::rune_count_in_string(s)),
        "utf8.Valid" => enc_bool(utf8::valid(s)),
        "utf8.ValidString" => enc_bool(utf8::valid_string(s)),
        "[]rune" => enc_runes(&utf8::to_runes(s)),
        "string([]rune)" => {
            let mut b = s.to_vec();
            b.extend(utf8::rune_to_string(parse_int(arg(1)) as Rune));
            utf8::from_runes(&utf8::to_runes(&b))
        }
        "string(runes)" => {
            let r = parse_int(arg(1)) as Rune;
            utf8::from_runes(&[r, 'x' as Rune, r])
        }
        "range" => {
            let mut b = Vec::new();
            for (i, c) in utf8::runes(s) {
                b.extend_from_slice(i.to_string().as_bytes());
                b.push(b':');
                b.extend_from_slice(c.to_string().as_bytes());
                b.push(b' ');
            }
            b
        }
        "utf16.Encode" => {
            let mut b = Vec::new();
            for x in utf16::encode(&utf8::to_runes(s)) {
                b.extend_from_slice(&x.to_le_bytes());
            }
            b
        }
        "utf16.Decode" => {
            let u: Vec<u16> = s
                .chunks(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            enc_runes(&utf16::decode(&u))
        }

        // strings
        "strings.Count" => enc_usize(strings::count(s, arg(1))),
        "strings.Contains" => enc_bool(strings::contains(s, arg(1))),
        "strings.ContainsAny" => enc_bool(strings::contains_any(s, arg(1))),
        "strings.ContainsRune" => enc_bool(strings::contains_rune(s, parse_int(arg(1)) as Rune)),
        "strings.ContainsFunc" => enc_bool(strings::contains_func(s, pred(arg(1)))),
        "strings.Index" => enc_int(strings::index(s, arg(1))),
        "strings.LastIndex" => enc_int(strings::last_index(s, arg(1))),
        "strings.IndexByte" => enc_int(strings::index_byte(s, arg(1)[0])),
        "strings.LastIndexByte" => enc_int(strings::last_index_byte(s, arg(1)[0])),
        "strings.IndexRune" => enc_int(strings::index_rune(s, parse_int(arg(1)) as Rune)),
        "strings.IndexAny" => enc_int(strings::index_any(s, arg(1))),
        "strings.LastIndexAny" => enc_int(strings::last_index_any(s, arg(1))),
        "strings.SplitN" => enc_list(&strings::split_n(s, arg(1), parse_int(arg(2)))),
        "strings.SplitAfterN" => enc_list(&strings::split_after_n(s, arg(1), parse_int(arg(2)))),
        "strings.Split" => enc_list(&strings::split(s, arg(1))),
        "strings.SplitAfter" => enc_list(&strings::split_after(s, arg(1))),
        "strings.Fields" => enc_list(&strings::fields(s)),
        "strings.FieldsFunc" => enc_list(&strings::fields_func(s, pred(arg(1)))),
        "strings.Join" => strings::join(&strings::split(s, arg(2)), arg(1)),
        "strings.HasPrefix" => enc_bool(strings::has_prefix(s, arg(1))),
        "strings.HasSuffix" => enc_bool(strings::has_suffix(s, arg(1))),
        "strings.Map" => {
            let c = strings::map(mapping(arg(1)), s);
            assert!(cow_ok(&c, s));
            c.into_owned()
        }
        "strings.Repeat" => strings::repeat(s, parse_int(arg(1))),
        "strings.ToUpper" => {
            let c = strings::to_upper(s);
            assert!(cow_ok(&c, s));
            c.into_owned()
        }
        "strings.ToLower" => {
            let c = strings::to_lower(s);
            assert!(cow_ok(&c, s));
            c.into_owned()
        }
        "strings.ToTitle" => strings::to_title(s).into_owned(),
        "strings.ToUpperSpecial" => {
            strings::to_upper_special(unicode::TURKISH_CASE, s).into_owned()
        }
        "strings.ToLowerSpecial" => {
            strings::to_lower_special(unicode::TURKISH_CASE, s).into_owned()
        }
        "strings.ToTitleSpecial" => {
            strings::to_title_special(unicode::TURKISH_CASE, s).into_owned()
        }
        "strings.ToValidUTF8" => {
            let c = strings::to_valid_utf8(s, arg(1));
            assert!(cow_ok(&c, s));
            c.into_owned()
        }
        "strings.Title" => strings::title(s).into_owned(),
        "strings.TrimLeftFunc" => strings::trim_left_func(s, pred(arg(1))).to_vec(),
        "strings.TrimRightFunc" => strings::trim_right_func(s, pred(arg(1))).to_vec(),
        "strings.TrimFunc" => strings::trim_func(s, pred(arg(1))).to_vec(),
        "strings.IndexFunc" => enc_int(strings::index_func(s, pred(arg(1)))),
        "strings.LastIndexFunc" => enc_int(strings::last_index_func(s, pred(arg(1)))),
        "strings.Trim" => strings::trim(s, arg(1)).to_vec(),
        "strings.TrimLeft" => strings::trim_left(s, arg(1)).to_vec(),
        "strings.TrimRight" => strings::trim_right(s, arg(1)).to_vec(),
        "strings.TrimSpace" => strings::trim_space(s).to_vec(),
        "strings.TrimPrefix" => strings::trim_prefix(s, arg(1)).to_vec(),
        "strings.TrimSuffix" => strings::trim_suffix(s, arg(1)).to_vec(),
        "strings.Replace" => strings::replace(s, arg(1), arg(2), parse_int(arg(3))).into_owned(),
        "strings.ReplaceAll" => strings::replace_all(s, arg(1), arg(2)).into_owned(),
        "strings.EqualFold" => enc_bool(strings::equal_fold(s, arg(1))),
        "strings.EqualFold/upper" => enc_bool(strings::equal_fold(s, &strings::to_upper(s))),
        "strings.EqualFold/lower" => enc_bool(strings::equal_fold(&strings::to_lower(s), s)),
        "strings.Cut" => {
            let (b, af, found) = strings::cut(s, arg(1));
            enc_list(&[b, af, &enc_bool(found)])
        }
        "strings.CutLast" => {
            let (b, af, found) = strings::cut_last(s, arg(1));
            enc_list(&[b, af, &enc_bool(found)])
        }
        "strings.CutPrefix" => {
            let (af, found) = strings::cut_prefix(s, arg(1));
            enc_list(&[af, &enc_bool(found)])
        }
        "strings.CutSuffix" => {
            let (b, found) = strings::cut_suffix(s, arg(1));
            enc_list(&[b, &enc_bool(found)])
        }
        "strings.Compare" => enc_int(strings::compare(s, arg(1))),

        // bytes
        "bytes.Count" => enc_usize(bytes::count(s, arg(1))),
        "bytes.Contains" => enc_bool(bytes::contains(s, arg(1))),
        "bytes.ContainsAny" => enc_bool(bytes::contains_any(s, arg(1))),
        "bytes.ContainsRune" => enc_bool(bytes::contains_rune(s, parse_int(arg(1)) as Rune)),
        "bytes.ContainsFunc" => enc_bool(bytes::contains_func(s, pred(arg(1)))),
        "bytes.Index" => enc_int(bytes::index(s, arg(1))),
        "bytes.LastIndex" => enc_int(bytes::last_index(s, arg(1))),
        "bytes.IndexByte" => enc_int(bytes::index_byte(s, arg(1)[0])),
        "bytes.LastIndexByte" => enc_int(bytes::last_index_byte(s, arg(1)[0])),
        "bytes.IndexRune" => enc_int(bytes::index_rune(s, parse_int(arg(1)) as Rune)),
        "bytes.IndexAny" => enc_int(bytes::index_any(s, arg(1))),
        "bytes.LastIndexAny" => enc_int(bytes::last_index_any(s, arg(1))),
        "bytes.SplitN" => enc_list(&bytes::split_n(s, arg(1), parse_int(arg(2)))),
        "bytes.SplitAfterN" => enc_list(&bytes::split_after_n(s, arg(1), parse_int(arg(2)))),
        "bytes.Split" => enc_list(&bytes::split(s, arg(1))),
        "bytes.SplitAfter" => enc_list(&bytes::split_after(s, arg(1))),
        "bytes.Fields" => enc_list(&bytes::fields(s)),
        "bytes.FieldsFunc" => enc_list(&bytes::fields_func(s, pred(arg(1)))),
        "bytes.Join" => bytes::join(&bytes::split(s, arg(2)), arg(1)),
        "bytes.Map" => bytes::map(mapping(arg(1)), s),
        "bytes.Repeat" => bytes::repeat(s, parse_int(arg(1))),
        "bytes.ToUpper" => bytes::to_upper(s),
        "bytes.ToLower" => bytes::to_lower(s),
        "bytes.ToTitle" => bytes::to_title(s),
        "bytes.ToUpperSpecial" => bytes::to_upper_special(unicode::TURKISH_CASE, s),
        "bytes.ToLowerSpecial" => bytes::to_lower_special(unicode::TURKISH_CASE, s),
        "bytes.ToTitleSpecial" => bytes::to_title_special(unicode::TURKISH_CASE, s),
        "bytes.ToValidUTF8" => bytes::to_valid_utf8(s, arg(1)),
        "bytes.Title" => bytes::title(s),
        "bytes.TrimLeftFunc" => bytes::trim_left_func(s, pred(arg(1))).to_vec(),
        "bytes.TrimRightFunc" => bytes::trim_right_func(s, pred(arg(1))).to_vec(),
        "bytes.TrimFunc" => bytes::trim_func(s, pred(arg(1))).to_vec(),
        "bytes.IndexFunc" => enc_int(bytes::index_func(s, pred(arg(1)))),
        "bytes.LastIndexFunc" => enc_int(bytes::last_index_func(s, pred(arg(1)))),
        "bytes.Trim" => bytes::trim(s, arg(1)).to_vec(),
        "bytes.TrimLeft" => bytes::trim_left(s, arg(1)).to_vec(),
        "bytes.TrimRight" => bytes::trim_right(s, arg(1)).to_vec(),
        "bytes.TrimSpace" => bytes::trim_space(s).to_vec(),
        "bytes.TrimPrefix" => bytes::trim_prefix(s, arg(1)).to_vec(),
        "bytes.TrimSuffix" => bytes::trim_suffix(s, arg(1)).to_vec(),
        "bytes.Runes" => enc_runes(&bytes::runes(s)),
        "bytes.Replace" => bytes::replace(s, arg(1), arg(2), parse_int(arg(3))),
        "bytes.ReplaceAll" => bytes::replace_all(s, arg(1), arg(2)),
        "bytes.EqualFold" => enc_bool(bytes::equal_fold(s, arg(1))),
        "bytes.EqualFold/upper" => enc_bool(bytes::equal_fold(s, &bytes::to_upper(s))),
        "bytes.Equal" => enc_bool(bytes::equal(s, arg(1))),
        "bytes.Compare" => enc_int(bytes::compare(s, arg(1))),
        "bytes.Cut" => {
            let (b, af, found) = bytes::cut(s, arg(1));
            enc_list(&[b, af, &enc_bool(found)])
        }
        "bytes.CutLast" => {
            let (b, af, found) = bytes::cut_last(s, arg(1));
            enc_list(&[b, af, &enc_bool(found)])
        }
        "bytes.CutPrefix" => {
            let (af, found) = bytes::cut_prefix(s, arg(1));
            enc_list(&[af, &enc_bool(found)])
        }
        "bytes.CutSuffix" => {
            let (b, found) = bytes::cut_suffix(s, arg(1));
            enc_list(&[b, &enc_bool(found)])
        }

        "strings.NewReplacer" => {
            let rep = Replacer::new(&a[1..]);
            let out = rep.replace(s);
            assert!(cow_ok(&out, s));
            let mut w = Vec::new();
            let n = rep.write_string(&mut w, s);
            assert_eq!(n, w.len());
            assert_eq!(w, out.as_ref(), "WriteString != Replace");
            out.into_owned()
        }
        other => panic!("unknown vector function {other}"),
    }
}

#[test]
fn strings_bytes_vectors_against_go() {
    check(&load());
}

/// The `-rich` corpus (random code points from all planes, fold-orbit runes,
/// spaces/separators, random bytes, truncated and invalid encodings), written
/// by `fixtures -only vectors -rich -n 900 -seed 20260928 -vec
/// strings_vectors_rich.bin.gz`.
#[test]
fn strings_bytes_rich_vectors_against_go() {
    check(&load_path(&format!(
        "{}/tests/fixtures/strings_vectors_rich.bin.gz",
        env!("CARGO_MANIFEST_DIR")
    )));
}

fn check(recs: &[Rec]) {
    assert!(recs.len() > 10_000);
    let mut per_fn: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    let mut failures = Vec::new();
    for r in recs {
        let got = run(r);
        let e = per_fn.entry(r.name.as_str()).or_default();
        e.0 += 1;
        if got != r.result {
            e.1 += 1;
            if failures.len() < 30 {
                failures.push(format!(
                    "{}({:?}) = {:?}, want {:?}",
                    r.name,
                    r.args
                        .iter()
                        .map(|x| String::from_utf8_lossy(x).into_owned())
                        .collect::<Vec<_>>(),
                    String::from_utf8_lossy(&got),
                    String::from_utf8_lossy(&r.result)
                ));
            }
        }
    }
    let bad: Vec<_> = per_fn.iter().filter(|(_, v)| v.1 > 0).collect();
    assert!(
        failures.is_empty(),
        "failing functions {bad:?}\nfirst failures:\n{}",
        failures.join("\n")
    );
    // Every function family is exercised.
    assert!(per_fn.len() > 100, "only {} functions", per_fn.len());
}
