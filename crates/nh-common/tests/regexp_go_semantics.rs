//! Go `regexp` behaviour that the Rust `regex` crate does not share, pinned with go1.27.1's
//! results (the values below were printed by go1.27.1), and Go's own test tables from
//! `regexp/all_test.go` transcribed literally.
//!
//! | area | `regex` crate | Go (and `goregexp`) |
//! |---|---|---|
//! | `\d \s \w \b \B` | Unicode | ASCII (`\s` = `[\t\n\f\r ]`, no `\v`) |
//! | `(?i)` | Unicode simple case folding of its own Unicode version | `unicode.SimpleFold` orbits (Unicode 17): `k`/`K`/U+212A, `s`/`S`/`ſ`, `σ`/`ς`/`Σ` |
//! | `\p{…}` | its Unicode version (no Garay, Sidetic, …) | Unicode 17.0.0 tables, Go's names and aliases |
//! | invalid UTF-8 input | `&str` only; `bytes::Regex` `.` skips bad bytes | each bad byte is one U+FFFD rune: `.`, `[^a]`, `\x{FFFD}` match it |
//! | empty matches in FindAll/ReplaceAll | its own iteration rules | Go's `matches`/`replaceAll` (advance one rune; no empty match right after a match) |
//! | `ReplaceAll` templates | `$1x` = `${1x}` too, but different name rules | Go's `extract` (`\pL`/`\pN`/`_` names, no leading zeros) |
//! | syntax | accepts `a**`, `a{1001}`, `\1` lookalikes, `\Q` differently | Go's parser and error texts |
//!
//! `goregexp` is a port of Go's package (parser, compiler and engines), so all of these hold
//! by construction; the tests keep them from regressing.

use nh_common::goregexp::{Regexp, quote_meta};

fn all(p: &str, s: &[u8]) -> Vec<Vec<isize>> {
    Regexp::compile(p).unwrap().find_all_submatch_index(s, -1)
}

fn repl(p: &str, s: &[u8]) -> Vec<u8> {
    Regexp::compile(p).unwrap().replace_all(s, b"<$0>")
}

#[test]
fn ascii_perl_classes_and_boundaries() {
    assert!(all(r"\bé", "é".as_bytes()).is_empty());
    assert_eq!(
        all(r"\b\w+\b", "déjà vu".as_bytes()),
        [[0, 1], [3, 4], [7, 9]]
    );
    assert_eq!(all(r"\d+", "١٢٣ 123".as_bytes()), [[7, 10]]);
    assert_eq!(
        repl(r"\s+", "a\u{b}b\u{a0}c\u{2003}d \t\ne".as_bytes()),
        "a\u{b}b\u{a0}c\u{2003}d< \t\n>e".as_bytes()
    );
    assert_eq!(all(r"\w+", "naïve_1".as_bytes()), [[0, 2], [4, 8]]);
    assert_eq!(all(r"[[:word:]]+", "ab_1é".as_bytes()), [[0, 4]]);
    // From nh-config's former goregexp tests.
    let re = Regexp::compile(r"^\w+$").unwrap();
    assert!(re.match_string("GOPATH_1"));
    assert!(!re.match_string("é"));
    let re = Regexp::compile(r"^[\w.-]+\s$").unwrap();
    assert!(re.match_string("a.b- "));
    assert!(!re.match_string("a\u{b}"));
    assert!(Regexp::compile(r"[]a]").unwrap().match_string("]"));
    let re = Regexp::compile(r"(?i)^((HTTPS?|NO)_PROXY|GO\w+)$").unwrap();
    assert!(re.match_string("GOROOT"));
    assert!(re.match_string("https_proxy"));
}

#[test]
fn case_folding() {
    assert_eq!(all("(?i)k", "\u{212a}".as_bytes()), [[0, 3]]);
    assert_eq!(all("(?i)K", "k\u{212a}".as_bytes()), [[0, 1], [1, 4]]);
    assert_eq!(all("(?i)s", "ſ".as_bytes()), [[0, 2]]);
    assert_eq!(all("(?i)σ", "Σς".as_bytes()), [[0, 2], [2, 4]]);
    assert_eq!(all("(?i)ß", "ẞ".as_bytes()), [[0, 3]]);
    assert_eq!(all(r"(?i)\x{212A}", b"kK"), [[0, 1], [1, 2]]);
}

#[test]
fn unicode_17_tables() {
    assert_eq!(
        all(r"\p{Garay}+", "\u{10D40}\u{10D41}".as_bytes()),
        [[0, 8]]
    );
    assert_eq!(all(r"\p{Sidetic}", "\u{10940}".as_bytes()), [[0, 4]]);
    assert_eq!(all(r"\p{Tolong_Siki}", "\u{11DB0}".as_bytes()), [[0, 4]]);
    assert_eq!(all(r"\p{Beria_Erfe}", "\u{16EA0}".as_bytes()), [[0, 4]]);
    assert_eq!(all(r"\p{L}", "\u{10D4A}".as_bytes()), [[0, 4]]);
    assert_eq!(all(r"\p{Nd}", "\u{11DE0}".as_bytes()), [[0, 4]]);
    assert_eq!(all(r"\pN+", "½Ⅻ٣".as_bytes()), [[0, 7]]);
    assert!(Regexp::compile(r"^\p{^L}$").unwrap().match_string("1"));
}

#[test]
fn invalid_utf8_input() {
    assert_eq!(all(".", b"\xff"), [[0, 1]]);
    assert_eq!(all(r"\x{FFFD}", b"a\xffb\xef\xbf\xbd"), [[1, 2], [3, 6]]);
    assert_eq!(all("[^a]", b"\xe0\x80"), [[0, 1], [1, 2]]);
    assert_eq!(all("a.b", b"a\xffb"), [[0, 3]]);
    assert_eq!(all(r"\PL", b"\xff"), [[0, 1]]);
    assert_eq!(all("x*", b"\xffa"), [[0, 0], [1, 1], [2, 2]]);
    assert_eq!(repl(".", b"\xff"), b"<\xff>");
}

#[test]
fn empty_matches_and_leftmost_first() {
    assert_eq!(all("a*", b"baaab"), [[0, 0], [1, 4], [5, 5]]);
    assert_eq!(repl("a*", b"baaab"), b"<>b<aaa>b<>");
    assert_eq!(all("", "日本".as_bytes()), [[0, 0], [3, 3], [6, 6]]);
    assert_eq!(all("a|", b"ab"), [[0, 1], [2, 2]]);
    assert_eq!(all("(a|ab)(c|bcd)", b"abcd"), [[0, 4, 0, 1, 1, 4]]);
    assert_eq!(all("(?U)a+", b"aaa"), [[0, 1], [1, 2], [2, 3]]);
    assert_eq!(all("(?m)^", b"a\nb\r\nc"), [[0, 0], [2, 2], [5, 5]]);
    assert_eq!(all("(?m)$", b"a\nb\r\nc"), [[1, 1], [4, 4], [6, 6]]);
    assert_eq!(all("$", b"a\n"), [[2, 2]]);
    assert_eq!(all(r"\z", b"a\n"), [[2, 2]]);
}

#[test]
fn syntax_quirks() {
    assert_eq!(all("a{,2}", b"a{,2}"), [[0, 5]]);
    assert_eq!(all(r"\Q*\E+", b"***"), [[0, 3]]);
    assert_eq!(all(r"\101\x41\x{41}", b"AAA"), [[0, 3]]);
    assert!(Regexp::compile(r"\Q.*\E").unwrap().match_string("x.*"));
    assert!(Regexp::compile("(?P<n>a)(?P<n>b)").is_ok());
    assert!(Regexp::compile("(?<n>a)").is_ok());
    for (p, want) in [
        (
            "a**",
            "error parsing regexp: invalid nested repetition operator: `**`",
        ),
        (
            "a{1001}",
            "error parsing regexp: invalid repeat count: `{1001}`",
        ),
        (
            r"\1",
            r"error parsing regexp: invalid escape sequence: `\1`",
        ),
        ("[a", "error parsing regexp: missing closing ]: `[a`"),
        ("(", "error parsing regexp: missing closing ): `(`"),
        (")", "error parsing regexp: unexpected ): `)`"),
        (
            "x{2}{3}",
            "error parsing regexp: invalid nested repetition operator: `{2}{3}`",
        ),
        (
            r"\8",
            r"error parsing regexp: invalid escape sequence: `\8`",
        ),
        (
            r"\p{Foo}",
            r"error parsing regexp: invalid character class range: `\p{Foo}`",
        ),
        (
            "(?i)(?P<N>a",
            "error parsing regexp: missing closing ): `(?i)(?P<N>a`",
        ),
    ] {
        assert_eq!(Regexp::compile(p).unwrap_err().to_string(), want, "{p}");
    }
    assert_eq!(
        Regexp::compile_bytes(b"\xff").unwrap_err().as_bytes(),
        b"error parsing regexp: invalid UTF-8: `\xff`"
    );
}

#[test]
fn expand_templates() {
    let re = Regexp::compile(r"(?P<first>\w+) (\w+)").unwrap();
    for (t, want) in [
        ("$1x", ""),
        ("${1}x", "hellox"),
        ("$2$1", "worldhello"),
        ("$first-$2", "hello-world"),
        ("${first}_", "hello_"),
        ("$$1", "$1"),
        ("$", "$"),
        ("$10", ""),
        ("$01", ""),
        ("${01}", ""),
        ("$ſ", ""),
    ] {
        assert_eq!(re.replace_all_string("hello world", t), want, "{t}");
    }
}

// Go: regexp/all_test.go:goodRe, badRe (TestGoodCompile, TestBadCompile)
#[test]
fn go_good_and_bad_compile() {
    for p in [
        "",
        ".",
        "^.$",
        "a",
        "a*",
        "a+",
        "a?",
        "a|b",
        "a*|b*",
        "(a*|b)(c*|d)",
        "[a-z]",
        r"[a-abc-c\-\]\[]",
        "[a-z]+",
        "[abc]",
        "[^1234]",
        r"[^\n]",
        r"\!\\",
    ] {
        assert!(Regexp::compile(p).is_ok(), "{p}");
    }
    let large = r"\pL".repeat(27000);
    for (p, err) in [
        ("*", "missing argument to repetition operator: `*`"),
        ("+", "missing argument to repetition operator: `+`"),
        ("?", "missing argument to repetition operator: `?`"),
        ("(abc", "missing closing ): `(abc`"),
        ("abc)", "unexpected ): `abc)`"),
        ("x[a-z", "missing closing ]: `[a-z`"),
        ("[z-a]", "invalid character class range: `z-a`"),
        (r"abc\", "trailing backslash at end of expression"),
        ("a**", "invalid nested repetition operator: `**`"),
        ("a*+", "invalid nested repetition operator: `*+`"),
        (r"\x", r"invalid escape sequence: `\x`"),
        (large.as_str(), "expression too large"),
    ] {
        let e = Regexp::compile(p).unwrap_err().to_string();
        assert!(e.contains(err), "{p}: {e}");
    }
}

// Go: regexp/all_test.go:replaceTests, replaceLiteralTests, replaceFuncTests
#[test]
fn go_replace_tests() {
    let replace = [
        ("", "", "", ""),
        ("", "x", "", "x"),
        ("", "", "abc", "abc"),
        ("", "x", "abc", "xaxbxcx"),
        ("b", "", "", ""),
        ("b", "x", "", ""),
        ("b", "", "abc", "ac"),
        ("b", "x", "abc", "axc"),
        ("y", "", "", ""),
        ("y", "x", "", ""),
        ("y", "", "abc", "abc"),
        ("y", "x", "abc", "abc"),
        ("[a-c]*", "x", "\u{65e5}", "x\u{65e5}x"),
        ("[^\u{65e5}]", "x", "abc\u{65e5}def", "xxx\u{65e5}xxx"),
        ("^[a-c]*", "x", "abcdabc", "xdabc"),
        ("[a-c]*$", "x", "abcdabc", "abcdx"),
        ("^[a-c]*$", "x", "abcdabc", "abcdabc"),
        ("^[a-c]*", "x", "abc", "x"),
        ("[a-c]*$", "x", "abc", "x"),
        ("^[a-c]*$", "x", "abc", "x"),
        ("^[a-c]*", "x", "dabce", "xdabce"),
        ("[a-c]*$", "x", "dabce", "dabcex"),
        ("^[a-c]*$", "x", "dabce", "dabce"),
        ("^[a-c]*", "x", "", "x"),
        ("[a-c]*$", "x", "", "x"),
        ("^[a-c]*$", "x", "", "x"),
        ("^[a-c]+", "x", "abcdabc", "xdabc"),
        ("[a-c]+$", "x", "abcdabc", "abcdx"),
        ("^[a-c]+$", "x", "abcdabc", "abcdabc"),
        ("^[a-c]+", "x", "abc", "x"),
        ("[a-c]+$", "x", "abc", "x"),
        ("^[a-c]+$", "x", "abc", "x"),
        ("^[a-c]+", "x", "dabce", "dabce"),
        ("[a-c]+$", "x", "dabce", "dabce"),
        ("^[a-c]+$", "x", "dabce", "dabce"),
        ("^[a-c]+", "x", "", ""),
        ("[a-c]+$", "x", "", ""),
        ("^[a-c]+$", "x", "", ""),
        ("abc", "def", "abcdefg", "defdefg"),
        ("bc", "BC", "abcbcdcdedef", "aBCBCdcdedef"),
        ("abc", "", "abcdabc", "d"),
        ("x", "xXx", "xxxXxxx", "xXxxXxxXxXxXxxXxxXx"),
        ("abc", "d", "", ""),
        ("abc", "d", "abc", "d"),
        (".+", "x", "abc", "x"),
        ("[a-c]*", "x", "def", "xdxexfx"),
        ("[a-c]+", "x", "abcbcdcdedef", "xdxdedef"),
        ("[a-c]*", "x", "abcbcdcdedef", "xdxdxexdxexfx"),
        ("a+", "($0)", "banana", "b(a)n(a)n(a)"),
        ("a+", "(${0})", "banana", "b(a)n(a)n(a)"),
        ("a+", "(${0})$0", "banana", "b(a)an(a)an(a)a"),
        (
            "hello, (.+)",
            "goodbye, ${1}",
            "hello, world",
            "goodbye, world",
        ),
        ("hello, (.+)", "goodbye, $1x", "hello, world", "goodbye, "),
        (
            "hello, (.+)",
            "goodbye, ${1}x",
            "hello, world",
            "goodbye, worldx",
        ),
        (
            "hello, (.+)",
            "<$0><$1><$2><$3>",
            "hello, world",
            "<hello, world><world><><>",
        ),
        (
            "hello, (?P<noun>.+)",
            "goodbye, $noun!",
            "hello, world",
            "goodbye, world!",
        ),
        (
            "hello, (?P<noun>.+)",
            "goodbye, ${noun}",
            "hello, world",
            "goodbye, world",
        ),
        ("(?P<x>hi)|(?P<x>bye)", "$x$x$x", "hi", "hihihi"),
        ("(?P<x>hi)|(?P<x>bye)", "$x$x$x", "bye", "byebyebye"),
        ("(?P<x>hi)|(?P<x>bye)", "$xyz", "hi", ""),
        ("(?P<x>hi)|(?P<x>bye)", "${x}yz", "hi", "hiyz"),
        ("(?P<x>hi)|(?P<x>bye)", "hello $$x", "hi", "hello $x"),
        ("a+", "${oops", "aaa", "${oops"),
        ("a+", "$$", "aaa", "$"),
        ("a+", "$", "aaa", "$"),
        ("(x)?", "$1", "123", "123"),
        ("abc", "$1", "123", "123"),
        ("(a)(b){0}(c)", ".$1|$3.", "xacxacx", "x.a|c.x.a|c.x"),
        ("(a)(((b))){0}c", ".$1.", "xacxacx", "x.a.x.a.x"),
        (
            "((a(b){0}){3}){5}(h)",
            "y caramb$2",
            "say aaaaaaaaaaaaaaaah",
            "say ay caramba",
        ),
        (
            "((a(b){0}){3}){5}h",
            "y caramb$2",
            "say aaaaaaaaaaaaaaaah",
            "say ay caramba",
        ),
    ];
    for (p, r, input, out) in replace {
        let re = Regexp::compile(p).unwrap();
        assert_eq!(re.replace_all_string(input, r), out, "{p} {r} {input}");
        assert_eq!(
            re.replace_all(input.as_bytes(), r.as_bytes()),
            out.as_bytes()
        );
    }
    let literal = [
        ("a+", "($0)", "banana", "b($0)n($0)n($0)"),
        ("a+", "(${0})", "banana", "b(${0})n(${0})n(${0})"),
        ("a+", "(${0})$0", "banana", "b(${0})$0n(${0})$0n(${0})$0"),
        (
            "hello, (.+)",
            "goodbye, ${1}",
            "hello, world",
            "goodbye, ${1}",
        ),
        (
            "hello, (?P<noun>.+)",
            "goodbye, $noun!",
            "hello, world",
            "goodbye, $noun!",
        ),
        ("(?P<x>hi)|(?P<x>bye)", "$x$x$x", "hi", "$x$x$x"),
        ("(?P<x>hi)|(?P<x>bye)", "hello $$x", "hi", "hello $$x"),
        ("a+", "${oops", "aaa", "${oops"),
        ("a+", "$$", "aaa", "$$"),
        ("a+", "$", "aaa", "$"),
    ];
    for (p, r, input, out) in literal {
        let re = Regexp::compile(p).unwrap();
        assert_eq!(re.replace_all_literal_string(input, r), out, "{p} {r}");
    }
    for (p, input, out) in [
        ("[a-c]", "defabcdef", "defxayxbyxcydef"),
        ("[a-c]+", "defabcdef", "defxabcydef"),
        ("[a-c]*", "defabcdef", "xydxyexyfxabcydxyexyfxy"),
    ] {
        let re = Regexp::compile(p).unwrap();
        let got = re.replace_all_func(input.as_bytes(), |m| {
            let mut v = b"x".to_vec();
            v.extend_from_slice(m);
            v.push(b'y');
            v
        });
        assert_eq!(got, out.as_bytes(), "{p}");
    }
}

// Go: regexp/all_test.go:splitTests
#[test]
fn go_split_tests() {
    let tests: &[(&str, &str, isize, Option<&[&str]>)] = &[
        ("foo:and:bar", ":", -1, Some(&["foo", "and", "bar"])),
        ("foo:and:bar", ":", 1, Some(&["foo:and:bar"])),
        ("foo:and:bar", ":", 2, Some(&["foo", "and:bar"])),
        ("foo:and:bar", "foo", -1, Some(&["", ":and:bar"])),
        ("foo:and:bar", "bar", -1, Some(&["foo:and:", ""])),
        ("foo:and:bar", "baz", -1, Some(&["foo:and:bar"])),
        ("baabaab", "a", -1, Some(&["b", "", "b", "", "b"])),
        ("baabaab", "a*", -1, Some(&["b", "b", "b"])),
        ("baabaab", "ba*", -1, Some(&["", "", "", ""])),
        ("foobar", "f*b*", -1, Some(&["", "o", "o", "a", "r"])),
        ("foobar", "f+.*b+", -1, Some(&["", "ar"])),
        ("foobooboar", "o{2}", -1, Some(&["f", "b", "boar"])),
        ("a,b,c,d,e,f", ",", 3, Some(&["a", "b", "c,d,e,f"])),
        ("a,b,c,d,e,f", ",", 0, None),
        (",", ",", -1, Some(&["", ""])),
        (",,,", ",", -1, Some(&["", "", "", ""])),
        ("", ",", -1, Some(&[""])),
        ("", ".*", -1, Some(&[""])),
        ("", ".+", -1, Some(&[""])),
        ("", "", -1, Some(&[])),
        ("foobar", "", -1, Some(&["f", "o", "o", "b", "a", "r"])),
        (
            "abaabaccadaaae",
            "a*",
            5,
            Some(&["", "b", "b", "c", "cadaaae"]),
        ),
        (":x:y:z:", ":", -1, Some(&["", "x", "y", "z", ""])),
    ];
    for (s, r, n, out) in tests {
        let re = Regexp::compile(r).unwrap();
        let got = re.split_string(s, *n);
        assert_eq!(got, out.unwrap_or(&[]), "{s} {r} {n}");
    }
}

// Go: regexp/all_test.go:metaTests, literalPrefixTests (TestQuoteMeta, TestLiteralPrefix)
#[test]
fn go_meta_tests() {
    let meta = [
        ("", "", "", true),
        ("foo", "foo", "foo", true),
        ("日本語+", r"日本語\+", "日本語", false),
        (r"foo\.\$", r"foo\\\.\\\$", "foo.$", true), // has meta but no operator
        (r"foo.\$", r"foo\.\\\$", "foo", false),     // has escaped operators and real operators
        (
            r"!@#$%^&*()_+-=[{]}\|,<.>/?~",
            r"!@#\$%\^&\*\(\)_\+-=\[\{\]\}\\\|,<\.>/\?~",
            "!@#",
            false,
        ),
    ];
    for (pattern, output, _, _) in meta {
        // Verify that QuoteMeta returns the expected string.
        let quoted = quote_meta(pattern.as_bytes());
        assert_eq!(quoted, output.as_bytes(), "{pattern}");
        // Verify that the quoted string is in fact treated as expected by Compile.
        if !pattern.is_empty() {
            let re = Regexp::compile_bytes(&quoted).unwrap();
            let src = format!("abc{pattern}def");
            assert_eq!(re.replace_all_string(&src, "xyz"), "abcxyzdef", "{pattern}");
        }
    }
    let literal_prefix = [
        // See golang.org/issue/11175. output is unused.
        ("^0^0$", "", "0", false),
        ("^0^", "", "", false),
        ("^0$", "", "0", true),
        ("$0^", "", "", false),
        ("$0$", "", "", false),
        ("^^0$$", "", "", false),
        ("^$^$", "", "", false),
        ("$$0^^", "", "", false),
        (r"a\x{fffd}b", "", "a", false),
        (r"\x{fffd}b", "", "", false),
        ("\u{fffd}", "", "", false),
    ];
    for (pattern, _, literal, is_literal) in meta.into_iter().chain(literal_prefix) {
        // Literal method needs to scan the pattern.
        let re = Regexp::must_compile(pattern);
        let (str, complete) = re.literal_prefix();
        assert_eq!(complete, is_literal, "{pattern}");
        assert_eq!(str, literal.as_bytes(), "{pattern}");
    }
}

/// The deepest trees Go accepts (`x{0,1000}` simplifies to 1,000 nested `(x(x(…)?)?)?`, and
/// 999 nested groups) compile and run on a default 2 MiB test thread.
#[test]
fn deep_expressions() {
    let input = "a".repeat(1200);
    let re = Regexp::compile("a{0,1000}").unwrap();
    assert_eq!(
        re.find_all_index(input.as_bytes(), -1),
        [[0, 1000], [1000, 1200]]
    );
    let re = Regexp::compile("(?:(a){0,1000})b").unwrap();
    assert_eq!(re.find_submatch_index(b"aab"), Some(vec![0, 3, 1, 2]));
    // 999 groups around a literal: height 1,000 (Go's maxHeight).
    let nested = format!("{}a{}", "(".repeat(999), ")".repeat(999));
    let re = Regexp::compile(&nested).unwrap();
    assert_eq!(re.num_subexp(), 999);
    assert!(re.match_string("xa"));
    let too_deep = format!("{}a{}", "(".repeat(1000), ")".repeat(1000));
    assert_eq!(
        Regexp::compile(&too_deep).unwrap_err().code,
        nh_common::goregexp::syntax::ErrorCode::NestingDepth
    );
}
