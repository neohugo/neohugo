//! Go tests whose data is inline in the test function (so the `gotests`
//! extractor does not see it), ported by hand from go1.27.1:
//! strings/replace_test.go (TestReplacer, TestPickAlgorithm,
//! TestGenericTrieBuilding), strings/strings_test.go and bytes/bytes_test.go
//! (TestIndexRune, TestMap, toValidUTF8Tests, TestSpecialCase, TestCutLast,
//! TestCaseConsistency), strings/compare_test.go (compareTests) and
//! unicode/letter_test.go + digit_test.go (TestTurkishCase,
//! TestLetterOptimizations, TestSpecialCaseNoMapping, TestNegativeRune,
//! TestLatinOffset, TestDigitOptimization, TestSimpleFold's -42 case).

use go_unicode::replacer::Replacer;
use go_unicode::{self as unicode, CaseRange, Rune, SpecialCase, bytes, strings, utf8};

fn rep(oldnew: &[&str]) -> Replacer {
    let v: Vec<&[u8]> = oldnew.iter().map(|s| s.as_bytes()).collect();
    Replacer::new(&v)
}

fn repb(oldnew: &[Vec<u8>]) -> Replacer {
    Replacer::new(oldnew)
}

// Go: src/strings/replace_test.go:TestReplacer
#[test]
fn test_replacer() {
    let html_escaper = rep(&[
        "&", "&amp;", "<", "&lt;", ">", "&gt;", "\"", "&quot;", "'", "&apos;",
    ]);
    let html_unescaper = rep(&[
        "&amp;", "&", "&lt;", "<", "&gt;", ">", "&quot;", "\"", "&apos;", "'",
    ]);
    let capital_letters = rep(&["a", "A", "b", "B"]);

    let mut cases: Vec<(&Replacer, Vec<u8>, Vec<u8>)> = Vec::new();
    let str_b = |b: u8| vec![b];

    // inc maps "\x00"->"\x01", ..., "\xff"->"\x00".
    let mut s: Vec<Vec<u8>> = Vec::new();
    for i in 0..256usize {
        s.push(str_b(i as u8));
        s.push(str_b((i + 1) as u8));
    }
    let inc = repb(&s);
    let a1_a2 = rep(&["a", "1", "a", "2"]);

    // repeat maps "a"->"a", "b"->"bb", "c"->"ccc", ...
    let mut s: Vec<Vec<u8>> = Vec::new();
    for i in 0..256isize {
        let mut n = i + 1 - b'a' as isize;
        if n < 1 {
            n = 1;
        }
        s.push(str_b(i as u8));
        s.push(strings::repeat(&str_b(i as u8), n));
    }
    let repeat = repb(&s);
    let a11_a22 = rep(&["a", "11", "a", "22"]);
    let a1_a2_xxx = rep(&["a", "1", "a", "2", "xxx", "xxx"]);
    let a_aa_aaa = rep(&["a", "1", "aa", "2", "aaa", "3"]);
    let aaa_aa_a = rep(&["aaa", "3", "aa", "2", "a", "1"]);
    let gen1 = rep(&[
        "aaa",
        "3[aaa]",
        "aa",
        "2[aa]",
        "a",
        "1[a]",
        "i",
        "i",
        "longerst",
        "most long",
        "longer",
        "medium",
        "long",
        "short",
        "xx",
        "xx",
        "x",
        "X",
        "X",
        "Y",
        "Y",
        "Z",
    ]);
    let gen2 = rep(&["roses", "red", "violets", "blue", "sugar", "sweet"]);
    let gen3 = rep(&[
        "abracadabra",
        "poof",
        "abracadabrakazam",
        "splat",
        "abraham",
        "lincoln",
        "abrasion",
        "scrape",
        "abraham",
        "isaac",
    ]);
    let foo1 = rep(&["foo1", "A", "foo2", "B", "foo3", "C"]);
    let foo2 = rep(&["foo1", "A", "foo2", "B", "foo31", "C", "foo32", "D"]);
    let foo3 = rep(&["foo11", "A", "foo12", "B", "foo31", "C", "foo32", "D"]);
    let foo4 = rep(&["foo12", "B", "foo32", "D"]);
    let all_bytes: Vec<u8> = (0..=255u8).collect();
    let gen_all = repb(&[
        all_bytes.clone(),
        b"[all]".to_vec(),
        b"\xff".to_vec(),
        b"[ff]".to_vec(),
        b"\x00".to_vec(),
        b"[00]".to_vec(),
    ]);
    let blank_to_x1 = rep(&["", "X"]);
    let blank_to_x2 = rep(&["", "X", "", ""]);
    let blank_high_priority = rep(&["", "X", "o", "O"]);
    let blank_low_priority = rep(&["o", "O", "", "X"]);
    let blank_no_op1 = rep(&["", ""]);
    let blank_no_op2 = rep(&["", "", "", "A"]);
    let blank_foo = rep(&["", "X", "foobar", "R", "foobaz", "Z"]);
    let abc_matcher = rep(&["abc", "[match]"]);
    let no_hello = rep(&["Hello", ""]);
    let nop = rep(&[]);

    let c = |r, i: &[u8], o: &[u8]| (r, i.to_vec(), o.to_vec());
    cases.extend([
        c(&capital_letters, b"brad", b"BrAd"),
        c(
            &capital_letters,
            &strings::repeat(b"a", (32 << 10) + 123),
            &strings::repeat(b"A", (32 << 10) + 123),
        ),
        c(&capital_letters, b"", b""),
        c(&inc, b"brad", b"csbe"),
        c(&inc, b"\x00\xff", b"\x01\x00"),
        c(&inc, b"", b""),
        c(&a1_a2, b"brad", b"br1d"),
        c(&html_escaper, b"No changes", b"No changes"),
        c(
            &html_escaper,
            b"I <3 escaping & stuff",
            b"I &lt;3 escaping &amp; stuff",
        ),
        c(&html_escaper, b"&&&", b"&amp;&amp;&amp;"),
        c(&html_escaper, b"", b""),
        c(&repeat, b"brad", b"bbrrrrrrrrrrrrrrrrrradddd"),
        c(&repeat, b"abba", b"abbbba"),
        c(&repeat, b"", b""),
        c(&a11_a22, b"brad", b"br11d"),
        c(&html_unescaper, b"&amp;amp;", b"&amp;"),
        c(
            &html_unescaper,
            b"&lt;b&gt;HTML&apos;s neat&lt;/b&gt;",
            b"<b>HTML's neat</b>",
        ),
        c(&html_unescaper, b"", b""),
        c(&a1_a2_xxx, b"brad", b"br1d"),
        c(&a_aa_aaa, b"aaaa", b"1111"),
        c(&aaa_aa_a, b"aaaa", b"31"),
        c(&gen1, b"fooaaabar", b"foo3[aaa]b1[a]r"),
        c(
            &gen1,
            b"long, longerst, longer",
            b"short, most long, medium",
        ),
        c(&gen1, b"xxxxx", b"xxxxX"),
        c(&gen1, b"XiX", b"YiY"),
        c(&gen1, b"", b""),
        c(
            &gen2,
            b"roses are red, violets are blue...",
            b"red are red, blue are blue...",
        ),
        c(&gen2, b"", b""),
        c(&gen3, b"abracadabrakazam abraham", b"poofkazam lincoln"),
        c(&gen3, b"abrasion abracad", b"scrape abracad"),
        c(&gen3, b"abba abram abrasive", b"abba abram abrasive"),
        c(&gen3, b"", b""),
        c(&foo1, b"fofoofoo12foo32oo", b"fofooA2C2oo"),
        c(&foo1, b"", b""),
        c(&foo2, b"fofoofoo12foo32oo", b"fofooA2Doo"),
        c(&foo2, b"", b""),
        c(&foo3, b"fofoofoo12foo32oo", b"fofooBDoo"),
        c(&foo3, b"", b""),
        c(&foo4, b"fofoofoo12foo32oo", b"fofooBDoo"),
        c(&foo4, b"", b""),
        c(&gen_all, &all_bytes, b"[all]"),
        c(
            &gen_all,
            &[&b"a\xff"[..], &all_bytes, b"\x00"].concat(),
            b"a[ff][all][00]",
        ),
        c(&gen_all, b"", b""),
        c(&blank_to_x1, b"foo", b"XfXoXoX"),
        c(&blank_to_x1, b"", b"X"),
        c(&blank_to_x2, b"foo", b"XfXoXoX"),
        c(&blank_to_x2, b"", b"X"),
        c(&blank_high_priority, b"oo", b"XOXOX"),
        c(&blank_high_priority, b"ii", b"XiXiX"),
        c(&blank_high_priority, b"oiio", b"XOXiXiXOX"),
        c(&blank_high_priority, b"iooi", b"XiXOXOXiX"),
        c(&blank_high_priority, b"", b"X"),
        c(&blank_low_priority, b"oo", b"OOX"),
        c(&blank_low_priority, b"ii", b"XiXiX"),
        c(&blank_low_priority, b"oiio", b"OXiXiOX"),
        c(&blank_low_priority, b"iooi", b"XiOOXiX"),
        c(&blank_low_priority, b"", b"X"),
        c(&blank_no_op1, b"foo", b"foo"),
        c(&blank_no_op1, b"", b""),
        c(&blank_no_op2, b"foo", b"foo"),
        c(&blank_no_op2, b"", b""),
        c(&blank_foo, b"foobarfoobaz", b"XRXZX"),
        c(&blank_foo, b"foobar-foobaz", b"XRX-XZX"),
        c(&blank_foo, b"", b"X"),
        c(&abc_matcher, b"", b""),
        c(&abc_matcher, b"ab", b"ab"),
        c(&abc_matcher, b"abc", b"[match]"),
        c(&abc_matcher, b"abcd", b"[match]d"),
        c(&abc_matcher, b"cabcabcdabca", b"c[match][match]d[match]a"),
        c(&no_hello, b"Hello", b""),
        c(&no_hello, b"Hellox", b"x"),
        c(&no_hello, b"xHello", b"x"),
        c(&no_hello, b"xHellox", b"xx"),
        c(&nop, b"abc", b"abc"),
        c(&nop, b"", b""),
    ]);

    for (i, (r, input, want)) in cases.iter().enumerate() {
        let got = r.replace(input);
        assert_eq!(
            got.as_ref(),
            want.as_slice(),
            "{i}. Replace({:?})",
            String::from_utf8_lossy(input)
        );
        let mut buf = Vec::new();
        let n = r.write_string(&mut buf, input);
        assert_eq!(&buf, want, "{i}. WriteString");
        assert_eq!(n, want.len(), "{i}. WriteString count");
    }
}

// Go: src/strings/replace_test.go:TestPickAlgorithm
#[test]
fn test_pick_algorithm() {
    let html_escaper = rep(&[
        "&", "&amp;", "<", "&lt;", ">", "&gt;", "\"", "&quot;", "'", "&apos;",
    ]);
    let cases: [(Replacer, &str); 6] = [
        (rep(&["a", "A", "b", "B"]), "*strings.byteReplacer"),
        (html_escaper, "*strings.byteStringReplacer"),
        (rep(&["12", "123"]), "*strings.singleStringReplacer"),
        (rep(&["1", "12"]), "*strings.byteStringReplacer"),
        (rep(&["", "X"]), "*strings.genericReplacer"),
        (
            rep(&["a", "1", "b", "12", "cde", "123"]),
            "*strings.genericReplacer",
        ),
    ];
    for (i, (r, want)) in cases.iter().enumerate() {
        assert_eq!(r.algorithm(), *want, "{i}");
    }
    // NewReplacer() with no pairs is a byteReplacer in Go too.
    assert_eq!(rep(&[]).algorithm(), "*strings.byteReplacer");
}

// Go: src/strings/replace_test.go:TestGenericTrieBuilding
#[test]
fn test_generic_trie_building() {
    let cases = [
        (
            "abc;abdef;abdefgh;xx;xy;z",
            "-\na-\n.b-\n..c+\n..d-\n...ef+\n.....gh+\nx-\n.x+\n.y+\nz+\n",
        ),
        (
            "abracadabra;abracadabrakazam;abraham;abrasion",
            "-\na-\n.bra-\n....c-\n.....adabra+\n...........kazam+\n....h-\n.....am+\n....s-\n.....ion+\n",
        ),
        (
            "aaa;aa;a;i;longerst;longer;long;xx;x;X;Y",
            "-\nX+\nY+\na+\n.a+\n..a+\ni+\nl-\n.ong+\n....er+\n......st+\nx+\n.x+\n",
        ),
        ("foo;;foo;foo1", "+\nf-\n.oo+\n...1+\n"),
    ];
    for (input, want) in cases {
        let keys = strings::split(input.as_bytes(), b";");
        let mut args: Vec<&[u8]> = vec![b""; keys.len() * 2];
        for (i, k) in keys.iter().enumerate() {
            args[i * 2] = k;
        }
        let got = Replacer::new(&args).print_trie().unwrap();
        assert_eq!(
            String::from_utf8(got).unwrap(),
            want,
            "PrintTrie({input:?})"
        );
    }
}

fn index_rune_cases() -> Vec<(Vec<u8>, Rune, isize)> {
    let r = |s: &str| s.as_bytes().to_vec();
    // "a☺b☻c☹d\xe2\x98�\xff�\xed\xa0\x80"
    let bad = [
        r("a☺b☻c☹d"),
        b"\xe2\x98\xef\xbf\xbd\xff\xef\xbf\xbd\xed\xa0\x80".to_vec(),
    ]
    .concat();
    vec![
        (r(""), 'a' as Rune, -1),
        (r(""), '☺' as Rune, -1),
        (r("foo"), '☹' as Rune, -1),
        (r("foo"), 'o' as Rune, 1),
        (r("foo☺bar"), '☺' as Rune, 3),
        (r("foo☺☻☹bar"), '☹' as Rune, 9),
        (r("a A x"), 'A' as Rune, 2),
        (r("some_text=some_value"), '=' as Rune, 9),
        (r("☺a"), 'a' as Rune, 3),
        (r("a☻☺b"), '☺' as Rune, 4),
        (r("𠀳𠀗𠀾𠁄𠀧𠁆𠁂𠀫𠀖𠀪𠀲𠀴𠁀𠀨𠀿"), '𠀿' as Rune, 56),
        // RuneError should match any invalid UTF-8 byte sequence.
        (r("�"), '�' as Rune, 0),
        (b"\xff".to_vec(), '�' as Rune, 0),
        (r("☻x�"), '�' as Rune, "☻x".len() as isize),
        (
            [r("☻x"), b"\xe2\x98".to_vec()].concat(),
            '�' as Rune,
            "☻x".len() as isize,
        ),
        (
            [r("☻x"), b"\xe2\x98".to_vec(), r("�")].concat(),
            '�' as Rune,
            "☻x".len() as isize,
        ),
        (
            [r("☻x"), b"\xe2\x98x".to_vec()].concat(),
            '�' as Rune,
            "☻x".len() as isize,
        ),
        // Invalid rune values should never match.
        (bad.clone(), -1, -1),
        (bad.clone(), 0xD800, -1),
        (bad, utf8::MAX_RUNE + 1, -1),
        // 2 bytes
        (r("ӆ"), 'ӆ' as Rune, 0),
        (r("a"), 'ӆ' as Rune, -1),
        (r("  ӆ"), 'ӆ' as Rune, 2),
        (r("  a"), 'ӆ' as Rune, -1),
        (r(&("ц".repeat(64) + "ӆ")), 'ӆ' as Rune, 128),
        (r(&"ц".repeat(64)), 'ӆ' as Rune, -1),
        (r(&("Ꙁ".repeat(64) + "Ꚁ")), '䚀' as Rune, -1),
        // 3 bytes
        (r("Ꚁ"), 'Ꚁ' as Rune, 0),
        (r("a"), 'Ꚁ' as Rune, -1),
        (r("  Ꚁ"), 'Ꚁ' as Rune, 2),
        (r("  a"), 'Ꚁ' as Rune, -1),
        (r(&("Ꙁ".repeat(64) + "Ꚁ")), 'Ꚁ' as Rune, 192),
        (r(&("𡋀".repeat(64) + "𡌀")), '𣌀' as Rune, -1),
        // 4 bytes
        (r("𡌀"), '𡌀' as Rune, 0),
        (r("a"), '𡌀' as Rune, -1),
        (r("  𡌀"), '𡌀' as Rune, 2),
        (r("  a"), '𡌀' as Rune, -1),
        (r(&("𡋀".repeat(64) + "𡌀")), '𡌀' as Rune, 256),
        (r(&"𡋀".repeat(64)), '𡌀' as Rune, -1),
        // Cutover in the middle of a rune with runs of equal bytes.
        // (The Ks are U+212A KELVIN SIGN, as in the Go source.)
        (
            r("aaaaa\u{212a}\u{212a}\u{212a}\u{212a}\u{bc104}"),
            0xbc104,
            17,
        ),
        (
            r("aaaaa\u{212a}\u{212a}\u{212a}\u{212a}鄄"),
            '鄄' as Rune,
            17,
        ),
        (
            r("aa\u{212a}\u{212a}\u{212a}\u{212a}\u{212a}a\u{bc104}"),
            0xbc104,
            18,
        ),
        (
            r("aa\u{212a}\u{212a}\u{212a}\u{212a}\u{212a}a鄄"),
            '鄄' as Rune,
            18,
        ),
    ]
}

// Go: src/strings/strings_test.go:TestIndexRune, src/bytes/bytes_test.go:TestIndexRune
#[test]
fn test_index_rune() {
    for (input, r, want) in index_rune_cases() {
        assert_eq!(
            strings::index_rune(&input, r),
            want,
            "strings {input:?} {r:#x}"
        );
        assert_eq!(bytes::index_rune(&input, r), want, "bytes {input:?} {r:#x}");
    }
    let haystack = ["test".to_string(), "𡋀".repeat(32), "𡌀".to_string()].concat();
    assert_eq!(strings::index_rune(haystack.as_bytes(), 's' as Rune), 2);
    assert_eq!(strings::index_rune(haystack.as_bytes(), '𡌀' as Rune), 132);
    assert_eq!(bytes::index_rune("test世界".as_bytes(), '世' as Rune), 4);
}

fn ten_runes(r: Rune) -> Vec<u8> {
    let mut v = Vec::new();
    for _ in 0..10 {
        utf8::append_rune(&mut v, r);
    }
    v
}

fn rot13(r: Rune) -> Rune {
    let step = 13;
    if r >= 'a' as Rune && r <= 'z' as Rune {
        return ((r - 'a' as Rune + step) % 26) + 'a' as Rune;
    }
    if r >= 'A' as Rune && r <= 'Z' as Rune {
        return ((r - 'A' as Rune + step) % 26) + 'A' as Rune;
    }
    r
}

// Go: src/strings/strings_test.go:TestMap (and bytes TestMap)
#[test]
fn test_map() {
    let a = ten_runes('a' as Rune);
    // 1. Grow.
    let max_rune = |_| unicode::MAX_RUNE;
    assert_eq!(
        strings::map(max_rune, &a).as_ref(),
        ten_runes(unicode::MAX_RUNE)
    );
    assert_eq!(bytes::map(max_rune, &a), ten_runes(unicode::MAX_RUNE));
    // 2. Shrink
    let min_rune = |_| 'a' as Rune;
    assert_eq!(
        strings::map(min_rune, &ten_runes(unicode::MAX_RUNE)).as_ref(),
        a
    );
    assert_eq!(bytes::map(min_rune, &ten_runes(unicode::MAX_RUNE)), a);
    // 3. Rot13
    assert_eq!(strings::map(rot13, b"a to zed").as_ref(), b"n gb mrq");
    assert_eq!(bytes::map(rot13, b"a to zed"), b"n gb mrq");
    // 4. Rot13^2
    assert_eq!(
        strings::map(rot13, &strings::map(rot13, b"a to zed")).as_ref(),
        b"a to zed"
    );
    // 5. Drop
    let drop_not_latin = |r| {
        if unicode::is(unicode::LATIN, r) {
            r
        } else {
            -1
        }
    };
    assert_eq!(
        strings::map(drop_not_latin, "Hello, 세계".as_bytes()).as_ref(),
        b"Hello"
    );
    assert_eq!(
        bytes::map(drop_not_latin, "Hello, 세계".as_bytes()),
        b"Hello"
    );
    // 6. Identity: no copy.
    let orig = b"Input string that we expect not to be copied.";
    assert!(matches!(
        strings::map(|r| r, orig),
        std::borrow::Cow::Borrowed(_)
    ));
    // 7. Handle invalid UTF-8 sequence
    let replace_not_latin = |r| {
        if unicode::is(unicode::LATIN, r) {
            r
        } else {
            utf8::RUNE_ERROR
        }
    };
    assert_eq!(
        strings::map(replace_not_latin, b"Hello\xadWorld").as_ref(),
        "Hello\u{FFFD}World".as_bytes()
    );
    assert_eq!(
        bytes::map(replace_not_latin, b"Hello\xadWorld"),
        "Hello\u{FFFD}World".as_bytes()
    );
    // 8. Check utf8.RuneSelf and utf8.MaxRune encoding
    let encode = |r| match r {
        utf8::RUNE_SELF => unicode::MAX_RUNE,
        unicode::MAX_RUNE => utf8::RUNE_SELF,
        _ => r,
    };
    let s = [
        utf8::rune_to_string(utf8::RUNE_SELF),
        utf8::rune_to_string(utf8::MAX_RUNE),
    ]
    .concat();
    let r = [
        utf8::rune_to_string(utf8::MAX_RUNE),
        utf8::rune_to_string(utf8::RUNE_SELF),
    ]
    .concat();
    assert_eq!(strings::map(encode, &s).as_ref(), r);
    assert_eq!(strings::map(encode, &r).as_ref(), s);
    assert_eq!(bytes::map(encode, &s), r);
    // 9. Check mapping occurs in the front, middle and back
    let trim_spaces = |r| if unicode::is_space(r) { -1 } else { r };
    assert_eq!(
        strings::map(trim_spaces, b"   abc    123   ").as_ref(),
        b"abc123"
    );
    assert_eq!(bytes::map(trim_spaces, b"   abc    123   "), b"abc123");
}

// Go: src/strings/strings_test.go:toValidUTF8Tests (same table in bytes)
#[test]
fn test_to_valid_utf8() {
    let cases: [(&[u8], &str, &str); 14] = [
        (b"", "\u{FFFD}", ""),
        (b"abc", "\u{FFFD}", "abc"),
        ("\u{FDDD}".as_bytes(), "\u{FFFD}", "\u{FDDD}"),
        (b"a\xffb", "\u{FFFD}", "a\u{FFFD}b"),
        (b"a\xffb\xef\xbf\xbd", "X", "aXb\u{FFFD}"),
        (
            b"a\xe2\x98\xba\xffb\xe2\x98\xba\xc0\xafc\xe2\x98\xba\xff",
            "",
            "a☺b☺c☺",
        ),
        (
            b"a\xe2\x98\xba\xffb\xe2\x98\xba\xc0\xafc\xe2\x98\xba\xff",
            "日本語",
            "a☺日本語b☺日本語c☺日本語",
        ),
        (b"\xC0\xAF", "\u{FFFD}", "\u{FFFD}"),
        (b"\xE0\x80\xAF", "\u{FFFD}", "\u{FFFD}"),
        (b"\xed\xa0\x80", "abc", "abc"),
        (b"\xed\xbf\xbf", "\u{FFFD}", "\u{FFFD}"),
        (b"\xF0\x80\x80\xaf", "☺", "☺"),
        (b"\xF8\x80\x80\x80\xAF", "\u{FFFD}", "\u{FFFD}"),
        (b"\xFC\x80\x80\x80\x80\xAF", "\u{FFFD}", "\u{FFFD}"),
    ];
    for (input, repl, out) in cases {
        assert_eq!(
            strings::to_valid_utf8(input, repl.as_bytes()).as_ref(),
            out.as_bytes(),
            "strings {input:?}"
        );
        assert_eq!(
            bytes::to_valid_utf8(input, repl.as_bytes()),
            out.as_bytes(),
            "bytes {input:?}"
        );
    }
}

// Go: src/strings/strings_test.go:TestSpecialCase
#[test]
fn test_special_case() {
    let lower = "abcçdefgğhıijklmnoöprsştuüvyz".as_bytes();
    let upper = "ABCÇDEFGĞHIİJKLMNOÖPRSŞTUÜVYZ".as_bytes();
    let tr = unicode::TURKISH_CASE;
    assert_eq!(strings::to_upper_special(tr, upper).as_ref(), upper);
    assert_eq!(strings::to_upper_special(tr, lower).as_ref(), upper);
    assert_eq!(strings::to_lower_special(tr, lower).as_ref(), lower);
    assert_eq!(strings::to_lower_special(tr, upper).as_ref(), lower);
    assert_eq!(bytes::to_upper_special(tr, lower), upper);
    assert_eq!(bytes::to_lower_special(tr, upper), lower);
}

// Go: src/strings/strings_test.go:TestCutLast (and bytes TestCutLast)
#[test]
fn test_cut_last() {
    let cases: [(&str, &str, &str, &str, bool); 7] = [
        ("a/b/c", "/", "a/b", "c", true),
        ("a//b//c", "//", "a//b", "c", true),
        ("abc", "/", "abc", "", false),
        ("abc", "", "abc", "", true),
        ("", "", "", "", true),
        ("/abc", "/", "", "abc", true),
        ("abc/", "/", "abc", "", true),
    ];
    for (s, sep, before, after, found) in cases {
        let got = strings::cut_last(s.as_bytes(), sep.as_bytes());
        assert_eq!(
            got,
            (before.as_bytes(), after.as_bytes(), found),
            "{s:?} {sep:?}"
        );
        let got = bytes::cut_last(s.as_bytes(), sep.as_bytes());
        assert_eq!(
            got,
            (before.as_bytes(), after.as_bytes(), found),
            "{s:?} {sep:?}"
        );
    }
}

// Go: src/strings/compare_test.go:TestCompare
#[test]
fn test_compare() {
    let cases: [(&str, &str, isize); 13] = [
        ("", "", 0),
        ("a", "", 1),
        ("", "a", -1),
        ("abc", "abc", 0),
        ("ab", "abc", -1),
        ("abc", "ab", 1),
        ("x", "ab", 1),
        ("ab", "x", -1),
        ("x", "a", 1),
        ("b", "x", -1),
        ("abcdefgh", "abcdefgh", 0),
        ("abcdefghi", "abcdefghi", 0),
        ("abcdefghi", "abcdefghj", -1),
    ];
    for (a, b, want) in cases {
        for offset in 0..=16 {
            let shifted = ["*".repeat(offset), b.to_string()].concat();
            let shifted = &shifted.as_bytes()[offset..];
            assert_eq!(strings::compare(a.as_bytes(), shifted), want);
            assert_eq!(bytes::compare(a.as_bytes(), shifted), want);
        }
    }
}

// Go: src/strings/strings_test.go:TestCaseConsistency
#[test]
fn test_case_consistency() {
    let a: Vec<Rune> = (0..=unicode::MAX_RUNE).collect();
    let s = utf8::from_runes(&a);
    let upper = strings::to_upper(&s).into_owned();
    let lower = strings::to_lower(&s).into_owned();
    assert_eq!(utf8::rune_count_in_string(&upper), a.len());
    assert_eq!(utf8::rune_count_in_string(&lower), a.len());
    assert_eq!(strings::to_upper(&upper).as_ref(), upper.as_slice());
    assert_eq!(strings::to_lower(&lower).as_ref(), lower.as_slice());
}

// Go: src/unicode/letter_test.go:TestTurkishCase
#[test]
fn test_turkish_case() {
    let lower: Vec<Rune> = "abcçdefgğhıijklmnoöprsştuüvyz"
        .chars()
        .map(|c| c as Rune)
        .collect();
    let upper: Vec<Rune> = "ABCÇDEFGĞHIİJKLMNOÖPRSŞTUÜVYZ"
        .chars()
        .map(|c| c as Rune)
        .collect();
    let tr = unicode::TURKISH_CASE;
    for (i, &l) in lower.iter().enumerate() {
        let u = upper[i];
        assert_eq!(tr.to_lower(l), l);
        assert_eq!(tr.to_upper(u), u);
        assert_eq!(tr.to_upper(l), u);
        assert_eq!(tr.to_lower(u), l);
        assert_eq!(tr.to_title(u), u);
        assert_eq!(tr.to_title(l), u);
    }
}

// Go: src/unicode/letter_test.go:TestLetterOptimizations, digit_test.go:TestDigitOptimization
#[test]
fn test_letter_optimizations() {
    for i in 0..=unicode::MAX_LATIN1 {
        assert_eq!(unicode::is(unicode::LETTER, i), unicode::is_letter(i));
        assert_eq!(unicode::is(unicode::UPPER, i), unicode::is_upper(i));
        assert_eq!(unicode::is(unicode::LOWER, i), unicode::is_lower(i));
        assert_eq!(unicode::is(unicode::TITLE, i), unicode::is_title(i));
        assert_eq!(unicode::is(unicode::WHITE_SPACE, i), unicode::is_space(i));
        assert_eq!(unicode::to(unicode::UPPER_CASE, i), unicode::to_upper(i));
        assert_eq!(unicode::to(unicode::LOWER_CASE, i), unicode::to_lower(i));
        assert_eq!(unicode::to(unicode::TITLE_CASE, i), unicode::to_title(i));
        assert_eq!(unicode::is(unicode::DIGIT, i), unicode::is_digit(i));
    }
}

// Go: src/unicode/letter_test.go:TestLatinOffset
#[test]
fn test_latin_offset() {
    for m in [
        unicode::CATEGORIES,
        unicode::FOLD_CATEGORY,
        unicode::FOLD_SCRIPT,
        unicode::PROPERTIES,
        unicode::SCRIPTS,
    ] {
        for (name, tab) in m {
            let mut i = 0;
            while i < tab.r16.len() && tab.r16[i].hi as Rune <= unicode::MAX_LATIN1 {
                i += 1;
            }
            assert_eq!(tab.latin_offset, i, "{name}");
        }
    }
}

// Go: src/unicode/letter_test.go:TestSpecialCaseNoMapping
#[test]
fn test_special_case_no_mapping() {
    let no_change_for_capital_a = [CaseRange {
        lo: 'A' as u32,
        hi: 'A' as u32,
        delta: [0, 0, 0],
    }];
    let got = strings::to_lower_special(SpecialCase(&no_change_for_capital_a), b"ABC");
    assert_eq!(got.as_ref(), b"Abc");
}

// Go: src/unicode/letter_test.go:TestNegativeRune
#[test]
fn test_negative_rune() {
    let non_latin1: [u32; 8] = [
        0x0100, 0x0101, 0x01C5, 0x0300, 0x0660, 0x037E, 0x02C2, 0x1680,
    ];
    for i in 0..(unicode::MAX_LATIN1 as usize + non_latin1.len()) {
        let mut base = i as u32;
        if i >= unicode::MAX_LATIN1 as usize {
            base = non_latin1[i - unicode::MAX_LATIN1 as usize];
        }
        let r = base.wrapping_sub(1 << 31) as Rune;
        assert!(!unicode::is(unicode::LETTER, r));
        assert!(!unicode::is_control(r));
        assert!(!unicode::is_digit(r));
        assert!(!unicode::is_graphic(r));
        assert!(!unicode::is_letter(r));
        assert!(!unicode::is_lower(r));
        assert!(!unicode::is_mark(r));
        assert!(!unicode::is_number(r));
        assert!(!unicode::is_print(r));
        assert!(!unicode::is_punct(r));
        assert!(!unicode::is_space(r));
        assert!(!unicode::is_symbol(r));
        assert!(!unicode::is_title(r));
        assert!(!unicode::is_upper(r));
    }
}

// Go: src/unicode/letter_test.go:TestSimpleFold (the non-table part)
#[test]
fn test_simple_fold_negative() {
    assert_eq!(unicode::simple_fold(-42), -42);
}

// Go: src/strings/strings_test.go:RepeatTests and src/bytes/bytes_test.go:RepeatTests,
// the rows the `gotests` extractor skips (built with make / package-level
// strings).
#[test]
fn test_repeat_long_rows() {
    let long_spaces = vec![b' '; 200];
    let mut long_string = vec![b'a'];
    long_string.extend(std::iter::repeat_n(0u8, 1 << 16));
    long_string.push(b'z');
    let zeros = vec![0u8; 1 << 16];
    let cases: [(&[u8], Vec<u8>, isize); 3] = [
        (b" ", long_spaces.clone(), long_spaces.len() as isize),
        (b"\x00", zeros, 1 << 16),
        (
            &long_string,
            [long_string.clone(), long_string.clone()].concat(),
            2,
        ),
    ];
    for (i, (input, out, count)) in cases.iter().enumerate() {
        assert_eq!(&strings::repeat(input, *count), out, "strings row {i}");
        if i > 0 {
            // bytes.RepeatTests has only the last two rows.
            assert_eq!(&bytes::repeat(input, *count), out, "bytes row {i}");
        }
    }
}

// Go: src/strings/strings_test.go:TestRepeatCatchesOverflow (and bytes)
#[test]
fn test_repeat_catches_overflow() {
    let max_int = isize::MAX;
    let cases: [(Vec<u8>, isize, &str); 8] = [
        (b"--".to_vec(), -2147483647, "negative"),
        (b"".to_vec(), max_int, ""),
        (b"-".to_vec(), 10, ""),
        (b"gopher".to_vec(), 0, ""),
        (b"-".to_vec(), -1, "negative"),
        (b"--".to_vec(), -102, "negative"),
        (vec![0u8; 255], (usize::MAX / 255 + 1) as isize, "overflow"),
        // 64-bit
        (b"-".to_vec(), max_int, "out of range"),
    ];
    for (i, (s, count, err)) in cases.iter().enumerate() {
        for which in ["strings", "bytes"] {
            let r = std::panic::catch_unwind(|| {
                if which == "strings" {
                    strings::repeat(s, *count)
                } else {
                    bytes::repeat(s, *count)
                }
            });
            match r {
                Ok(_) => assert!(err.is_empty(), "{which} #{i} did not panic, want {err:?}"),
                Err(e) => {
                    let msg = e
                        .downcast_ref::<&str>()
                        .map(|s| s.to_string())
                        .or_else(|| e.downcast_ref::<String>().cloned())
                        .unwrap_or_default();
                    assert!(
                        !err.is_empty() && msg.contains(err),
                        "{which} #{i} panicked {msg:?}, want {err:?}"
                    );
                }
            }
        }
    }
}
