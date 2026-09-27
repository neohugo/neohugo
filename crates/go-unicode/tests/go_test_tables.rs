//! Go's own test tables for unicode, unicode/utf8, unicode/utf16, strings and
//! bytes (go1.27.1 `*_test.go`), extracted by `tools/go-oracle/go-unicode
//! gotests` into `tests/fixtures/go_test_tables.txt` and replayed here with
//! the assertions of the Go test that uses each table.

use std::collections::BTreeMap;

use go_unicode::{self as unicode, Rune, bytes, strings, utf8, utf16};

#[derive(Debug, Clone, PartialEq)]
enum V {
    S(Vec<u8>),
    I(i64),
    B(bool),
    Nil,
    D(String),
    L(Vec<V>),
}

impl V {
    fn s(&self) -> &[u8] {
        match self {
            V::S(s) => s,
            V::Nil => &[],
            other => panic!("not a string: {other:?}"),
        }
    }
    fn i(&self) -> i64 {
        match self {
            V::I(i) => *i,
            other => panic!("not an int: {other:?}"),
        }
    }
    fn r(&self) -> Rune {
        self.i() as Rune
    }
    fn b(&self) -> bool {
        match self {
            V::B(b) => *b,
            other => panic!("not a bool: {other:?}"),
        }
    }
    fn d(&self) -> &str {
        match self {
            V::D(d) => d,
            other => panic!("not a name: {other:?}"),
        }
    }
    fn strs(&self) -> Vec<Vec<u8>> {
        match self {
            V::L(l) => l.iter().map(|e| e.s().to_vec()).collect(),
            V::Nil => Vec::new(),
            other => panic!("not a list: {other:?}"),
        }
    }
    fn ints(&self) -> Vec<i64> {
        match self {
            V::L(l) => l.iter().map(|e| e.i()).collect(),
            V::Nil => Vec::new(),
            other => panic!("not a list: {other:?}"),
        }
    }
}

fn unhex(h: &str) -> Vec<u8> {
    (0..h.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&h[i..i + 2], 16).unwrap())
        .collect()
}

fn parse(f: &str) -> V {
    let (tag, rest) = f.split_at(1);
    match tag {
        "s" => V::S(unhex(rest)),
        "i" => V::I(rest.parse().unwrap()),
        "b" => V::B(rest == "T"),
        "n" => V::Nil,
        "d" => V::D(rest.to_string()),
        "L" => {
            if rest.is_empty() {
                V::L(Vec::new())
            } else {
                V::L(rest.split(',').map(parse).collect())
            }
        }
        _ => panic!("bad field {f}"),
    }
}

fn load() -> BTreeMap<String, Vec<Vec<V>>> {
    let path = format!(
        "{}/tests/fixtures/go_test_tables.txt",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(path).unwrap();
    let mut m: BTreeMap<String, Vec<Vec<V>>> = BTreeMap::new();
    for line in text.lines() {
        if line.starts_with('#') {
            continue;
        }
        let mut it = line.split('\t');
        let name = it.next().unwrap().to_string();
        let row: Vec<V> = it.map(parse).collect();
        m.entry(name).or_default().push(row);
    }
    m
}

fn pred(name: &str) -> Box<dyn Fn(Rune) -> bool> {
    if let Some(inner) = name.strip_prefix("not ") {
        let p = pred(inner);
        return Box::new(move |r| !p(r));
    }
    match name {
        "IsSpace" => Box::new(unicode::is_space),
        "IsDigit" => Box::new(unicode::is_digit),
        "IsUpper" => Box::new(unicode::is_upper),
        "IsValidRune" => Box::new(|r| r != utf8::RUNE_ERROR),
        other => panic!("unknown predicate {other}"),
    }
}

fn q(b: &[u8]) -> String {
    format!("{:?}", String::from_utf8_lossy(b))
}

#[test]
fn go_test_tables() {
    let m = load();
    let t = |name: &str| -> &Vec<Vec<V>> {
        m.get(name)
            .unwrap_or_else(|| panic!("missing table {name}"))
    };
    let mut checked = 0usize;
    let mut fails: Vec<String> = Vec::new();
    let mut check = |ok: bool, msg: String| {
        checked += 1;
        if !ok {
            fails.push(msg);
        }
    };

    // ---- unicode ----
    for row in t("unicode.upperTest") {
        let r = row[0].r();
        check(unicode::is_letter(r), format!("IsLetter({r:#x})"));
        check(unicode::is_upper(r), format!("IsUpper({r:#x})"));
    }
    for row in t("unicode.notupperTest") {
        let r = row[0].r();
        check(!unicode::is_upper(r), format!("!IsUpper({r:#x})"));
    }
    for row in t("unicode.letterTest") {
        let r = row[0].r();
        check(unicode::is_letter(r), format!("IsLetter({r:#x})"));
        check(!unicode::is_space(r), format!("!IsSpace({r:#x})"));
    }
    for row in t("unicode.notletterTest") {
        let r = row[0].r();
        check(!unicode::is_letter(r), format!("!IsLetter({r:#x})"));
        check(!unicode::is_upper(r), format!("!IsUpper({r:#x})"));
    }
    for row in t("unicode.spaceTest") {
        let r = row[0].r();
        check(unicode::is_space(r), format!("IsSpace({r:#x})"));
    }
    for row in t("unicode.caseTest") {
        let (cas, inp, out) = (row[0].i(), row[1].r(), row[2].r());
        check(unicode::to(cas, inp) == out, format!("To({cas}, {inp:#x})"));
        let direct = match cas {
            unicode::UPPER_CASE => Some(unicode::to_upper(inp)),
            unicode::LOWER_CASE => Some(unicode::to_lower(inp)),
            unicode::TITLE_CASE => Some(unicode::to_title(inp)),
            _ => None,
        };
        if let Some(d) = direct {
            check(d == out, format!("To*Case({cas}, {inp:#x}) = {d:#x}"));
        }
    }
    for row in t("unicode.simpleFoldTests") {
        let cycle = utf8::to_runes(row[0].s());
        let mut r = *cycle.last().unwrap();
        for &out in &cycle {
            let got = unicode::simple_fold(r);
            check(
                got == out,
                format!("SimpleFold({r:#x}) = {got:#x}, want {out:#x}"),
            );
            r = out;
        }
    }
    check(unicode::simple_fold(-42) == -42, "SimpleFold(-42)".into());
    for row in t("unicode.testDigit") {
        let r = row[0].r();
        check(unicode::is_digit(r), format!("IsDigit({r:#x})"));
    }
    for row in t("unicode.testLetter") {
        let r = row[0].r();
        check(!unicode::is_digit(r), format!("!IsDigit({r:#x})"));
    }
    for row in t("unicode.inCategoryTest") {
        let (r, cat) = (row[0].r(), String::from_utf8(row[1].s().to_vec()).unwrap());
        let tab = unicode::lookup(unicode::CATEGORIES, &cat).unwrap();
        check(
            unicode::is(tab, r),
            format!("Is(Categories[{cat}], {r:#x})"),
        );
    }
    for row in t("unicode.inPropTest") {
        let (r, prop) = (row[0].r(), String::from_utf8(row[1].s().to_vec()).unwrap());
        let tab = unicode::lookup(unicode::PROPERTIES, &prop).unwrap();
        check(
            unicode::is(tab, r),
            format!("Is(Properties[{prop}], {r:#x})"),
        );
    }
    // TestTurkishCase (letter_test.go).
    {
        let lower: Vec<Rune> = "abcçdefgğhıijklmnoöprsştuüvyz"
            .chars()
            .map(|c| c as Rune)
            .collect();
        let upper: Vec<Rune> = "ABCÇDEFGĞHIİJKLMNOÖPRSŞTUÜVYZ"
            .chars()
            .map(|c| c as Rune)
            .collect();
        for (i, &l) in lower.iter().enumerate() {
            let u = upper[i];
            let tc = unicode::TURKISH_CASE;
            check(tc.to_lower(l) == l, format!("turkish lower({l:#x})"));
            check(tc.to_upper(u) == u, format!("turkish upper({u:#x})"));
            check(tc.to_upper(l) == u, format!("turkish upper(lower {l:#x})"));
            check(tc.to_lower(u) == l, format!("turkish lower(upper {u:#x})"));
            check(tc.to_title(u) == u, format!("turkish title({u:#x})"));
            check(tc.to_title(l) == u, format!("turkish title(lower {l:#x})"));
        }
    }

    // ---- utf8 ----
    for (table, surrogate) in [("utf8.utf8map", false), ("utf8.surrogateMap", true)] {
        for row in t(table) {
            let (r, s) = (row[0].r(), row[1].s());
            if !surrogate {
                let mut b = [0u8; 4];
                let n = utf8::encode_rune(&mut b, r);
                check(&b[..n] == s, format!("EncodeRune({r:#x})"));
                let mut v = Vec::new();
                utf8::append_rune(&mut v, r);
                check(v == s, format!("AppendRune({r:#x})"));
                check(utf8::full_rune(s), format!("FullRune({})", q(s)));
                check(
                    !utf8::full_rune(&s[..s.len() - 1]),
                    format!("!FullRune({})", q(s)),
                );
                check(
                    utf8::decode_rune(s) == (r, s.len()),
                    format!("DecodeRune({})", q(s)),
                );
                let mut padded = s.to_vec();
                padded.push(0);
                check(
                    utf8::decode_rune(&padded) == (r, s.len()),
                    format!("DecodeRune padded({})", q(s)),
                );
                check(
                    utf8::decode_last_rune(s) == (r, s.len()),
                    format!("DecodeLastRune({})", q(s)),
                );
                // Make sure bad sequences fail.
                let wantsize = if s.len() == 1 { 0 } else { 1 };
                let short = &s[..s.len() - 1];
                check(
                    utf8::decode_rune(short) == (utf8::RUNE_ERROR, wantsize),
                    format!("DecodeRune(short {})", q(s)),
                );
                check(
                    utf8::rune_len(r) == s.len() as isize,
                    format!("RuneLen({r:#x})"),
                );
            } else {
                check(
                    utf8::decode_rune(s) == (utf8::RUNE_ERROR, 1),
                    format!("surrogate DecodeRune({})", q(s)),
                );
                check(
                    utf8::decode_last_rune(s) == (utf8::RUNE_ERROR, 1),
                    format!("surrogate DecodeLastRune({})", q(s)),
                );
            }
        }
    }
    for row in t("utf8.testStrings") {
        // TestSequencing-style consistency: forward and backward decoding agree.
        let s = row[0].s();
        let fwd: Vec<(usize, Rune)> = utf8::runes(s).collect();
        let mut back = Vec::new();
        let mut end = s.len();
        while end > 0 {
            let (r, size) = utf8::decode_last_rune(&s[..end]);
            end -= size;
            back.push((end, r));
        }
        back.reverse();
        check(fwd == back, format!("forward/backward decode of {}", q(s)));
    }
    for row in t("utf8.invalidSequenceTests") {
        let s = row[0].s();
        check(
            utf8::decode_rune(s).0 == utf8::RUNE_ERROR,
            format!("DecodeRune({})", q(s)),
        );
        check(
            utf8::runes(s).next().unwrap().1 == utf8::RUNE_ERROR,
            format!("range({})", q(s)),
        );
        // DecodeLastRune of the sequence without its last byte? (Go checks
        // the first rune only.)
    }
    for row in t("utf8.runecounttests") {
        let (s, n) = (row[0].s(), row[1].i() as usize);
        check(utf8::rune_count(s) == n, format!("RuneCount({})", q(s)));
        check(
            utf8::rune_count_in_string(s) == n,
            format!("RuneCountInString({})", q(s)),
        );
    }
    for row in t("utf8.runelentests") {
        let (r, n) = (row[0].r(), row[1].i() as isize);
        check(utf8::rune_len(r) == n, format!("RuneLen({r:#x})"));
    }
    for row in t("utf8.validTests") {
        let (s, ok) = (row[0].s(), row[1].b());
        check(utf8::valid(s) == ok, format!("Valid({})", q(s)));
        check(
            utf8::valid_string(s) == ok,
            format!("ValidString({})", q(s)),
        );
    }
    for i in 0..100 {
        // validTests init() rows.
        let a = "a".repeat(i);
        let b = "b".repeat(i);
        for (s, ok) in [
            (a.clone().into_bytes(), true),
            (format!("{a}Ж").into_bytes(), true),
            ([a.as_bytes(), b"\xe2"].concat(), false),
            (format!("{a}Ж{b}").into_bytes(), true),
            ([a.as_bytes(), b"\xe2", b.as_bytes()].concat(), false),
        ] {
            check(utf8::valid(&s) == ok, format!("Valid(init row {i})"));
        }
    }
    for row in t("utf8.validrunetests") {
        let (r, ok) = (row[0].r(), row[1].b());
        check(utf8::valid_rune(r) == ok, format!("ValidRune({r:#x})"));
    }

    // ---- utf16 ----
    for row in t("utf16.encodeTests") {
        let inp: Vec<Rune> = row[0].ints().iter().map(|&x| x as Rune).collect();
        let out: Vec<u16> = row[1].ints().iter().map(|&x| x as u16).collect();
        check(utf16::encode(&inp) == out, format!("utf16.Encode({inp:?})"));
        let mut a = Vec::new();
        for &r in &inp {
            utf16::append_rune(&mut a, r);
        }
        check(a == out, format!("utf16.AppendRune({inp:?})"));
    }
    for row in t("utf16.decodeTests") {
        let inp: Vec<u16> = row[0].ints().iter().map(|&x| x as u16).collect();
        let out: Vec<Rune> = row[1].ints().iter().map(|&x| x as Rune).collect();
        check(utf16::decode(&inp) == out, format!("utf16.Decode({inp:?})"));
    }
    for row in t("utf16.decodeRuneTests") {
        let (r1, r2, want) = (row[0].r(), row[1].r(), row[2].r());
        check(
            utf16::decode_rune(r1, r2) == want,
            format!("utf16.DecodeRune({r1:#x}, {r2:#x})"),
        );
    }
    for row in t("utf16.surrogateTests") {
        let (r, want) = (row[0].r(), row[1].b());
        check(
            utf16::is_surrogate(r) == want,
            format!("utf16.IsSurrogate({r:#x})"),
        );
    }

    // ---- strings and bytes (shared table shapes) ----
    for pkg in ["strings", "bytes"] {
        let is_bytes = pkg == "bytes";
        let tn = |n: &str| format!("{pkg}.{n}");
        for row in t(&tn("indexTests")) {
            let (s, sep, out) = (row[0].s(), row[1].s(), row[2].i() as isize);
            let got = if is_bytes {
                bytes::index(s, sep)
            } else {
                strings::index(s, sep)
            };
            check(got == out, format!("{pkg}.Index({}, {})", q(s), q(sep)));
        }
        for row in t(&tn("lastIndexTests")) {
            let (s, sep, out) = (row[0].s(), row[1].s(), row[2].i() as isize);
            let got = if is_bytes {
                bytes::last_index(s, sep)
            } else {
                strings::last_index(s, sep)
            };
            check(got == out, format!("{pkg}.LastIndex({}, {})", q(s), q(sep)));
        }
        for row in t(&tn("indexAnyTests")) {
            let (s, sep, out) = (row[0].s(), row[1].s(), row[2].i() as isize);
            let got = if is_bytes {
                bytes::index_any(s, sep)
            } else {
                strings::index_any(s, sep)
            };
            check(got == out, format!("{pkg}.IndexAny({}, {})", q(s), q(sep)));
        }
        for row in t(&tn("lastIndexAnyTests")) {
            let (s, sep, out) = (row[0].s(), row[1].s(), row[2].i() as isize);
            let got = if is_bytes {
                bytes::last_index_any(s, sep)
            } else {
                strings::last_index_any(s, sep)
            };
            check(
                got == out,
                format!("{pkg}.LastIndexAny({}, {})", q(s), q(sep)),
            );
        }
        for (table, after) in [("splittests", false), ("splitaftertests", true)] {
            for row in t(&tn(table)) {
                let (s, sep, n, want) =
                    (row[0].s(), row[1].s(), row[2].i() as isize, row[3].strs());
                let got: Vec<Vec<u8>> = match (is_bytes, after) {
                    (false, false) => strings::split_n(s, sep, n),
                    (false, true) => strings::split_after_n(s, sep, n),
                    (true, false) => bytes::split_n(s, sep, n),
                    (true, true) => bytes::split_after_n(s, sep, n),
                }
                .into_iter()
                .map(|x| x.to_vec())
                .collect();
                check(
                    got == want,
                    format!("{pkg}.{table}({}, {}, {n})", q(s), q(sep)),
                );
                if n == 0 {
                    continue;
                }
                let join_sep: &[u8] = if after { b"" } else { sep };
                let joined = if is_bytes {
                    bytes::join(&got, join_sep)
                } else {
                    strings::join(&got, join_sep)
                };
                check(joined == s, format!("{pkg}.Join({table}({}))", q(s)));
                if n < 0 {
                    let all: Vec<Vec<u8>> = match (is_bytes, after) {
                        (false, false) => strings::split(s, sep),
                        (false, true) => strings::split_after(s, sep),
                        (true, false) => bytes::split(s, sep),
                        (true, true) => bytes::split_after(s, sep),
                    }
                    .into_iter()
                    .map(|x| x.to_vec())
                    .collect();
                    check(all == got, format!("{pkg}.Split vs SplitN({})", q(s)));
                }
            }
        }
        for row in t(&tn("fieldstests")) {
            let (s, want) = (row[0].s(), row[1].strs());
            let got: Vec<Vec<u8>> = if is_bytes {
                bytes::fields(s)
            } else {
                strings::fields(s)
            }
            .into_iter()
            .map(|x| x.to_vec())
            .collect();
            check(got == want, format!("{pkg}.Fields({})", q(s)));
            let got2: Vec<Vec<u8>> = if is_bytes {
                bytes::fields_func(s, unicode::is_space)
            } else {
                strings::fields_func(s, unicode::is_space)
            }
            .into_iter()
            .map(|x| x.to_vec())
            .collect();
            check(got2 == want, format!("{pkg}.FieldsFunc({}, IsSpace)", q(s)));
        }
        for (table, f) in [
            ("upperTests", 0),
            ("lowerTests", 1),
            ("trimSpaceTests", 2),
            ("TitleTests", 3),
        ] {
            for row in t(&tn(table)) {
                let (s, want) = (row[0].s(), row[1].s());
                let got: Vec<u8> = match (is_bytes, f) {
                    (false, 0) => strings::to_upper(s).into_owned(),
                    (false, 1) => strings::to_lower(s).into_owned(),
                    (false, 2) => strings::trim_space(s).to_vec(),
                    (false, _) => strings::title(s).into_owned(),
                    (true, 0) => bytes::to_upper(s),
                    (true, 1) => bytes::to_lower(s),
                    (true, 2) => bytes::trim_space(s).to_vec(),
                    (true, _) => bytes::title(s),
                };
                check(got == want, format!("{pkg}.{table}({})", q(s)));
            }
        }
        for row in t(&tn("RepeatTests")) {
            // strings: {in, out, count}; bytes: {in, out, count}.
            let (s, want, count) = (row[0].s(), row[1].s(), row[2].i() as isize);
            let got = if is_bytes {
                bytes::repeat(s, count)
            } else {
                strings::repeat(s, count)
            };
            check(got == want, format!("{pkg}.Repeat({}, {count})", q(s)));
        }
        for row in t(&tn("RunesTests")) {
            let (s, want, lossy) = (row[0].s(), row[1].ints(), row[2].b());
            let want: Vec<Rune> = want.iter().map(|&x| x as Rune).collect();
            let got = if is_bytes {
                bytes::runes(s)
            } else {
                utf8::to_runes(s)
            };
            check(got == want, format!("{pkg}.Runes({})", q(s)));
            if !lossy {
                check(
                    utf8::from_runes(&got) == s,
                    format!("string([]rune({}))", q(s)),
                );
            }
        }
        for row in t(&tn("trimTests")) {
            let (f, s, arg, want) = (row[0].s(), row[1].s(), row[2].s(), row[3].s());
            let got: &[u8] = match (is_bytes, f) {
                (false, b"Trim") => strings::trim(s, arg),
                (false, b"TrimLeft") => strings::trim_left(s, arg),
                (false, b"TrimRight") => strings::trim_right(s, arg),
                (false, b"TrimPrefix") => strings::trim_prefix(s, arg),
                (false, b"TrimSuffix") => strings::trim_suffix(s, arg),
                (true, b"Trim") => bytes::trim(s, arg),
                (true, b"TrimLeft") => bytes::trim_left(s, arg),
                (true, b"TrimRight") => bytes::trim_right(s, arg),
                (true, b"TrimPrefix") => bytes::trim_prefix(s, arg),
                (true, b"TrimSuffix") => bytes::trim_suffix(s, arg),
                _ => panic!("unknown trim function {}", q(f)),
            };
            check(got == want, format!("{pkg}.{}({}, {})", q(f), q(s), q(arg)));
        }
        for row in t(&tn("trimFuncTests")) {
            let p = pred(row[0].d());
            let s = row[1].s();
            let (trim, left, right) = (row[2].s(), row[3].s(), row[4].s());
            if is_bytes {
                check(
                    bytes::trim_func(s, &p) == trim,
                    format!("bytes.TrimFunc({})", q(s)),
                );
                check(
                    bytes::trim_left_func(s, &p) == left,
                    format!("bytes.TrimLeftFunc({})", q(s)),
                );
                check(
                    bytes::trim_right_func(s, &p) == right,
                    format!("bytes.TrimRightFunc({})", q(s)),
                );
            } else {
                check(
                    strings::trim_func(s, &p) == trim,
                    format!("strings.TrimFunc({})", q(s)),
                );
                check(
                    strings::trim_left_func(s, &p) == left,
                    format!("strings.TrimLeftFunc({})", q(s)),
                );
                check(
                    strings::trim_right_func(s, &p) == right,
                    format!("strings.TrimRightFunc({})", q(s)),
                );
            }
        }
        for row in t(&tn("indexFuncTests")) {
            let s = row[0].s();
            let p = pred(row[1].d());
            let (first, last) = (row[2].i() as isize, row[3].i() as isize);
            let (f, l) = if is_bytes {
                (bytes::index_func(s, &p), bytes::last_index_func(s, &p))
            } else {
                (strings::index_func(s, &p), strings::last_index_func(s, &p))
            };
            check(f == first, format!("{pkg}.IndexFunc({})", q(s)));
            check(l == last, format!("{pkg}.LastIndexFunc({})", q(s)));
        }
        for row in t(&tn("ReplaceTests")) {
            let (s, old, new, n, want) = (
                row[0].s(),
                row[1].s(),
                row[2].s(),
                row[3].i() as isize,
                row[4].s(),
            );
            let got = if is_bytes {
                bytes::replace(s, old, new, n)
            } else {
                strings::replace(s, old, new, n).into_owned()
            };
            check(
                got == want,
                format!("{pkg}.Replace({}, {}, {}, {n})", q(s), q(old), q(new)),
            );
            if n == -1 {
                let all = if is_bytes {
                    bytes::replace_all(s, old, new)
                } else {
                    strings::replace_all(s, old, new).into_owned()
                };
                check(all == want, format!("{pkg}.ReplaceAll({})", q(s)));
            }
        }
        for row in t(&tn("EqualFoldTests")) {
            let (s, u, out) = (row[0].s(), row[1].s(), row[2].b());
            let f = if is_bytes {
                bytes::equal_fold
            } else {
                strings::equal_fold
            };
            check(
                f(s, u) == out,
                format!("{pkg}.EqualFold({}, {})", q(s), q(u)),
            );
            check(
                f(u, s) == out,
                format!("{pkg}.EqualFold({}, {})", q(u), q(s)),
            );
        }
        for row in t(&tn("cutTests")) {
            let (s, sep, before, after, found) =
                (row[0].s(), row[1].s(), row[2].s(), row[3].s(), row[4].b());
            let got = if is_bytes {
                bytes::cut(s, sep)
            } else {
                strings::cut(s, sep)
            };
            check(
                got == (before, after, found),
                format!("{pkg}.Cut({}, {})", q(s), q(sep)),
            );
        }
        for row in t(&tn("cutPrefixTests")) {
            let (s, sep, after, found) = (row[0].s(), row[1].s(), row[2].s(), row[3].b());
            let got = if is_bytes {
                bytes::cut_prefix(s, sep)
            } else {
                strings::cut_prefix(s, sep)
            };
            check(
                got == (after, found),
                format!("{pkg}.CutPrefix({}, {})", q(s), q(sep)),
            );
        }
        for row in t(&tn("cutSuffixTests")) {
            let (s, sep, before, found) = (row[0].s(), row[1].s(), row[2].s(), row[3].b());
            let got = if is_bytes {
                bytes::cut_suffix(s, sep)
            } else {
                strings::cut_suffix(s, sep)
            };
            check(
                got == (before, found),
                format!("{pkg}.CutSuffix({}, {})", q(s), q(sep)),
            );
        }
        for row in t(&tn("ContainsAnyTests")) {
            let (s, chars, want) = (row[0].s(), row[1].s(), row[2].b());
            let got = if is_bytes {
                bytes::contains_any(s, chars)
            } else {
                strings::contains_any(s, chars)
            };
            check(
                got == want,
                format!("{pkg}.ContainsAny({}, {})", q(s), q(chars)),
            );
        }
        for row in t(&tn("ContainsRuneTests")) {
            let (s, r, want) = (row[0].s(), row[1].r(), row[2].b());
            let got = if is_bytes {
                bytes::contains_rune(s, r)
            } else {
                strings::contains_rune(s, r)
            };
            check(got == want, format!("{pkg}.ContainsRune({}, {r:#x})", q(s)));
        }
    }
    for row in t("strings.FieldsFuncTests") {
        let (s, want) = (row[0].s(), row[1].strs());
        let got: Vec<Vec<u8>> = strings::fields_func(s, |c| c == 'X' as Rune)
            .into_iter()
            .map(|x| x.to_vec())
            .collect();
        check(got == want, format!("strings.FieldsFunc({}, =='X')", q(s)));
    }
    for row in t("strings.ContainsTests") {
        let (s, sub, want) = (row[0].s(), row[1].s(), row[2].b());
        check(
            strings::contains(s, sub) == want,
            format!("strings.Contains({}, {})", q(s), q(sub)),
        );
    }
    for row in t("bytes.containsTests") {
        let (s, sub, want) = (row[0].s(), row[1].s(), row[2].b());
        check(
            bytes::contains(s, sub) == want,
            format!("bytes.Contains({}, {})", q(s), q(sub)),
        );
    }
    for row in t("strings.CountTests") {
        let (s, sep, n) = (row[0].s(), row[1].s(), row[2].i() as usize);
        check(
            strings::count(s, sep) == n,
            format!("strings.Count({}, {})", q(s), q(sep)),
        );
        check(
            bytes::count(s, sep) == n,
            format!("bytes.Count({}, {})", q(s), q(sep)),
        );
    }
    for row in t("bytes.ToTitleTests") {
        let (s, want) = (row[0].s(), row[1].s());
        check(
            bytes::to_title(s) == want,
            format!("bytes.ToTitle({})", q(s)),
        );
        check(
            strings::to_title(s).as_ref() == want,
            format!("strings.ToTitle({})", q(s)),
        );
    }

    // TestSpecialCase (strings_test.go).
    {
        let lower = "abcçdefgğhıijklmnoöprsştuüvyz".as_bytes();
        let upper = "ABCÇDEFGĞHIİJKLMNOÖPRSŞTUÜVYZ".as_bytes();
        let tc = unicode::TURKISH_CASE;
        check(
            strings::to_upper_special(tc, upper).as_ref() == upper,
            "ToUpperSpecial(upper)".into(),
        );
        check(
            strings::to_upper_special(tc, lower).as_ref() == upper,
            "ToUpperSpecial(lower)".into(),
        );
        check(
            strings::to_lower_special(tc, lower).as_ref() == lower,
            "ToLowerSpecial(lower)".into(),
        );
        check(
            strings::to_lower_special(tc, upper).as_ref() == lower,
            "ToLowerSpecial(upper)".into(),
        );
    }

    assert!(
        fails.is_empty(),
        "{} of {checked} checks failed:\n{}",
        fails.len(),
        fails.join("\n")
    );
    assert!(checked > 1500, "only {checked} checks");
}
