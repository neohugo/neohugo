//! Ports of the go1.27.1 `encoding/json/internal/jsonwire` test tables:
//! `wire_test.go` (TestQuoteRune, TestTruncatePointer), `decode_test.go`
//! (TestConsumeWhitespace, TestConsumeLiteral, TestConsumeString,
//! TestConsumeNumber, TestParseHexUint16) and `encode_test.go`
//! (TestAppendQuote, TestAppendNumber).

use super::decode::{STRING_NON_CANONICAL_FOR_TESTS as NC, STRING_NON_VERBATIM_FOR_TESTS as NV};
use super::*;
use crate::goerr::Err;
use crate::jsonflags::{self, Flags};

/// Concatenates byte pieces (Go string literals mixing `\xNN` and `\uNNNN`).
fn cat(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

fn ice(what: &[u8], where_: &str) -> Err {
    new_invalid_character_error(what, where_)
}

fn iese(what: &[u8]) -> Err {
    new_invalid_escape_sequence_error(what)
}

// Go: wire_test.go:TestQuoteRune
#[test]
fn quote_rune_table() {
    let tests: &[(&[u8], &str)] = &[
        (b"x", "'x'"),
        (b"\n", "'\\n'"),
        (b"'", "'\\''"),
        (b"\xff", "'\\xff'"),
        ("💩".as_bytes(), "'💩'"),
        (&"💩".as_bytes()[..1], "'\\xf0'"),
        ("\u{ffff}".as_bytes(), "'\\uffff'"),
        ("\u{101234}".as_bytes(), "'\\U00101234'"),
    ];
    for (input, want) in tests {
        assert_eq!(quote_rune(input), *want, "QuoteRune({:?})", input);
    }
}

// Go: wire_test.go:TestTruncatePointer
#[test]
fn truncate_pointer_table() {
    let tests = [
        ("hello", "hello"),
        ("/a/b/c", "/a/b/c"),
        ("/a/b/c/d/e/f/g", "/a/b/…/f/g"),
        ("supercalifragilisticexpialidocious", "super…cious"),
        (
            "/supercalifragilisticexpialidocious/supercalifragilisticexpialidocious",
            "/supe…/…cious",
        ),
        (
            "/supercalifragilisticexpialidocious/supercalifragilisticexpialidocious/supercalifragilisticexpialidocious",
            "/supe…/…/…cious",
        ),
        (
            "/a/supercalifragilisticexpialidocious/supercalifragilisticexpialidocious",
            "/a/…/…cious",
        ),
        (
            "/supercalifragilisticexpialidocious/supercalifragilisticexpialidocious/b",
            "/supe…/…/b",
        ),
        ("/fizz/buzz/bazz", "/fizz/…/bazz"),
        ("/fizz/buzz/bazz/razz", "/fizz/…/razz"),
        ("/////////////////////////////", "/////…/////"),
        ("/🎄❤️✨/🎁✅😊/🎅🔥⭐", "/🎄…/…/…⭐"),
    ];
    for (input, want) in tests {
        let got = truncate_pointer(input.as_bytes(), 10);
        assert_eq!(
            String::from_utf8_lossy(&got),
            want,
            "TruncatePointer({input:?})"
        );
    }
}

// Go: decode_test.go:TestConsumeWhitespace
#[test]
fn consume_whitespace_table() {
    let tests: &[(&[u8], usize)] = &[
        (b"", 0),
        (b"a", 0),
        (b" a", 1),
        (b" a ", 1),
        (b" \n\r\ta", 4),
        (b" \n\r\t \n\r\t \n\r\t \n\r\t", 16),
        ("\u{00a0}".as_bytes(), 0), // non-breaking space is not JSON whitespace
    ];
    for (input, want) in tests {
        assert_eq!(
            consume_whitespace(input),
            *want,
            "ConsumeWhitespace({input:?})"
        );
    }
}

// Go: decode_test.go:TestConsumeLiteral
#[test]
fn consume_literal_table() {
    let ue = || Some(Err::UnexpectedEof);
    let tests: Vec<(&str, &str, usize, Option<Err>)> = vec![
        ("null", "", 0, ue()),
        ("null", "n", 1, ue()),
        ("null", "nu", 2, ue()),
        ("null", "nul", 3, ue()),
        ("null", "null", 4, None),
        ("null", "nullx", 4, None),
        (
            "null",
            "x",
            0,
            Some(ice(b"x", "in literal null (expecting 'n')")),
        ),
        (
            "null",
            "nuxx",
            2,
            Some(ice(b"x", "in literal null (expecting 'l')")),
        ),
        ("false", "", 0, ue()),
        ("false", "f", 1, ue()),
        ("false", "fa", 2, ue()),
        ("false", "fal", 3, ue()),
        ("false", "fals", 4, ue()),
        ("false", "false", 5, None),
        ("false", "falsex", 5, None),
        (
            "false",
            "x",
            0,
            Some(ice(b"x", "in literal false (expecting 'f')")),
        ),
        (
            "false",
            "falsx",
            4,
            Some(ice(b"x", "in literal false (expecting 'e')")),
        ),
        ("true", "", 0, ue()),
        ("true", "t", 1, ue()),
        ("true", "tr", 2, ue()),
        ("true", "tru", 3, ue()),
        ("true", "true", 4, None),
        ("true", "truex", 4, None),
        (
            "true",
            "x",
            0,
            Some(ice(b"x", "in literal true (expecting 't')")),
        ),
        (
            "true",
            "trux",
            3,
            Some(ice(b"x", "in literal true (expecting 'e')")),
        ),
    ];
    for (lit, input, want, want_err) in tests {
        let got = match lit {
            "null" => consume_null(input.as_bytes()),
            "false" => consume_false(input.as_bytes()),
            _ => consume_true(input.as_bytes()),
        };
        if want_err.is_none() {
            assert_eq!(got, want, "Consume{lit}({input:?})");
        } else {
            assert_eq!(got, 0, "Consume{lit}({input:?})");
        }
        let (got, got_err) = consume_literal(input.as_bytes(), lit);
        assert_eq!(
            (got, &got_err),
            (want, &want_err),
            "ConsumeLiteral({input:?}, {lit:?})"
        );
    }
}

/// `errPrev` in Go's TestConsumeString table.
#[derive(Clone)]
enum E {
    Nil,
    Prev,
    Is(Err),
}

fn resolve(e: E, prev: &Option<Err>) -> Option<Err> {
    match e {
        E::Nil => None,
        E::Prev => prev.clone(),
        E::Is(err) => Some(err),
    }
}

// Go: decode_test.go:TestConsumeString
#[test]
fn consume_string_table() {
    use E::{Is, Nil, Prev};
    let ue = || Is(Err::UnexpectedEof);
    let iu = || Is(Err::InvalidUtf8);
    let fffd: &[u8] = "\u{fffd}".as_bytes();
    #[allow(clippy::type_complexity)]
    let tests: Vec<(Vec<u8>, bool, usize, usize, u32, Vec<u8>, E, E, E)> = vec![
        (b"".to_vec(), false, 0, 0, 0, b"".to_vec(), ue(), Prev, Prev),
        (
            b"\"".to_vec(),
            false,
            1,
            1,
            0,
            b"".to_vec(),
            ue(),
            Prev,
            Prev,
        ),
        (b"\"\"".to_vec(), true, 2, 2, 0, b"".to_vec(), Nil, Nil, Nil),
        (
            b"\"\"x".to_vec(),
            true,
            2,
            2,
            0,
            b"".to_vec(),
            Nil,
            Nil,
            Is(ice(b"x", "after string value")),
        ),
        (
            b" \"\"x".to_vec(),
            false,
            0,
            0,
            0,
            b"".to_vec(),
            Is(ice(b" ", "at start of string (expecting '\"')")),
            Prev,
            Prev,
        ),
        (
            b"\"hello".to_vec(),
            false,
            6,
            6,
            0,
            b"hello".to_vec(),
            ue(),
            Prev,
            Prev,
        ),
        (
            b"\"hello\"".to_vec(),
            true,
            7,
            7,
            0,
            b"hello".to_vec(),
            Nil,
            Nil,
            Nil,
        ),
        (
            b"\"\x00\"".to_vec(),
            false,
            1,
            1,
            NV | NC,
            b"".to_vec(),
            Is(ice(b"\x00", "in string (expecting non-control character)")),
            Prev,
            Prev,
        ),
        (
            b"\"\\u0000\"".to_vec(),
            false,
            8,
            8,
            NV,
            b"\x00".to_vec(),
            Nil,
            Nil,
            Nil,
        ),
        (
            b"\"\x1f\"".to_vec(),
            false,
            1,
            1,
            NV | NC,
            b"".to_vec(),
            Is(ice(b"\x1f", "in string (expecting non-control character)")),
            Prev,
            Prev,
        ),
        (
            b"\"\\u001f\"".to_vec(),
            false,
            8,
            8,
            NV,
            b"\x1f".to_vec(),
            Nil,
            Nil,
            Nil,
        ),
        (
            b"\"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz\"".to_vec(),
            true,
            54,
            54,
            0,
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz".to_vec(),
            Nil,
            Nil,
            Nil,
        ),
        (
            b"\" !#$%'()*+,-./0123456789:;=?@[]^_`{|}~\x7f\"".to_vec(),
            true,
            41,
            41,
            0,
            b" !#$%'()*+,-./0123456789:;=?@[]^_`{|}~\x7f".to_vec(),
            Nil,
            Nil,
            Nil,
        ),
        (
            b"\"&\"".to_vec(),
            false,
            3,
            3,
            0,
            b"&".to_vec(),
            Nil,
            Nil,
            Nil,
        ),
        (
            b"\"<\"".to_vec(),
            false,
            3,
            3,
            0,
            b"<".to_vec(),
            Nil,
            Nil,
            Nil,
        ),
        (
            b"\">\"".to_vec(),
            false,
            3,
            3,
            0,
            b">".to_vec(),
            Nil,
            Nil,
            Nil,
        ),
        (
            b"\"x\x80\"".to_vec(),
            false,
            4,
            2,
            NV | NC,
            cat(&[b"x", fffd]),
            Nil,
            iu(),
            Prev,
        ),
        (
            b"\"x\xff\"".to_vec(),
            false,
            4,
            2,
            NV | NC,
            cat(&[b"x", fffd]),
            Nil,
            iu(),
            Prev,
        ),
        (
            b"\"x\xc0".to_vec(),
            false,
            3,
            2,
            NV | NC,
            cat(&[b"x", fffd]),
            ue(),
            iu(),
            ue(),
        ),
        (
            b"\"x\xc0\x80\"".to_vec(),
            false,
            5,
            2,
            NV | NC,
            cat(&[b"x", fffd, fffd]),
            Nil,
            iu(),
            Prev,
        ),
        (
            b"\"x\xe0".to_vec(),
            false,
            2,
            2,
            0,
            b"x".to_vec(),
            ue(),
            Prev,
            Prev,
        ),
        (
            b"\"x\xe0\x80".to_vec(),
            false,
            4,
            2,
            NV | NC,
            cat(&[b"x", fffd, fffd]),
            ue(),
            iu(),
            ue(),
        ),
        (
            b"\"x\xe0\x80\x80\"".to_vec(),
            false,
            6,
            2,
            NV | NC,
            cat(&[b"x", fffd, fffd, fffd]),
            Nil,
            iu(),
            Prev,
        ),
        (
            b"\"x\xf0".to_vec(),
            false,
            2,
            2,
            0,
            b"x".to_vec(),
            ue(),
            Prev,
            Prev,
        ),
        (
            b"\"x\xf0\x80".to_vec(),
            false,
            4,
            2,
            NV | NC,
            cat(&[b"x", fffd, fffd]),
            ue(),
            iu(),
            ue(),
        ),
        (
            b"\"x\xf0\x80\x80".to_vec(),
            false,
            5,
            2,
            NV | NC,
            cat(&[b"x", fffd, fffd, fffd]),
            ue(),
            iu(),
            ue(),
        ),
        (
            b"\"x\xf0\x80\x80\x80\"".to_vec(),
            false,
            7,
            2,
            NV | NC,
            cat(&[b"x", fffd, fffd, fffd, fffd]),
            Nil,
            iu(),
            Prev,
        ),
        (
            b"\"x\xed\xba\xad\"".to_vec(),
            false,
            6,
            2,
            NV | NC,
            cat(&[b"x", fffd, fffd, fffd]),
            Nil,
            iu(),
            Prev,
        ),
        (
            cat(&[
                b"\"",
                "\u{0080}\u{00f6}\u{20ac}\u{d799}\u{e000}\u{fb33}\u{fffd}\u{1f602}".as_bytes(),
                b"\"",
            ]),
            false,
            25,
            25,
            0,
            "\u{0080}\u{00f6}\u{20ac}\u{d799}\u{e000}\u{fb33}\u{fffd}\u{1f602}"
                .as_bytes()
                .to_vec(),
            Nil,
            Nil,
            Nil,
        ),
        (
            "\"¢\"".as_bytes()[..2].to_vec(),
            false,
            1,
            1,
            0,
            b"".to_vec(),
            ue(),
            Prev,
            Prev,
        ),
        (
            "\"¢\"".as_bytes()[..3].to_vec(),
            false,
            3,
            3,
            0,
            "¢".as_bytes().to_vec(),
            ue(),
            Prev,
            Prev,
        ),
        (
            "\"¢\"".as_bytes()[..4].to_vec(),
            false,
            4,
            4,
            0,
            "¢".as_bytes().to_vec(),
            Nil,
            Nil,
            Nil,
        ),
        (
            "\"€\"".as_bytes()[..2].to_vec(),
            false,
            1,
            1,
            0,
            b"".to_vec(),
            ue(),
            Prev,
            Prev,
        ),
        (
            "\"€\"".as_bytes()[..3].to_vec(),
            false,
            1,
            1,
            0,
            b"".to_vec(),
            ue(),
            Prev,
            Prev,
        ),
        (
            "\"€\"".as_bytes()[..4].to_vec(),
            false,
            4,
            4,
            0,
            "€".as_bytes().to_vec(),
            ue(),
            Prev,
            Prev,
        ),
        (
            "\"€\"".as_bytes()[..5].to_vec(),
            false,
            5,
            5,
            0,
            "€".as_bytes().to_vec(),
            Nil,
            Nil,
            Nil,
        ),
        (
            "\"𐍈\"".as_bytes()[..2].to_vec(),
            false,
            1,
            1,
            0,
            b"".to_vec(),
            ue(),
            Prev,
            Prev,
        ),
        (
            "\"𐍈\"".as_bytes()[..3].to_vec(),
            false,
            1,
            1,
            0,
            b"".to_vec(),
            ue(),
            Prev,
            Prev,
        ),
        (
            "\"𐍈\"".as_bytes()[..4].to_vec(),
            false,
            1,
            1,
            0,
            b"".to_vec(),
            ue(),
            Prev,
            Prev,
        ),
        (
            "\"𐍈\"".as_bytes()[..5].to_vec(),
            false,
            5,
            5,
            0,
            "𐍈".as_bytes().to_vec(),
            ue(),
            Prev,
            Prev,
        ),
        (
            "\"𐍈\"".as_bytes()[..6].to_vec(),
            false,
            6,
            6,
            0,
            "𐍈".as_bytes().to_vec(),
            Nil,
            Nil,
            Nil,
        ),
        (
            b"\"x\\".to_vec(),
            false,
            2,
            2,
            NV,
            b"x".to_vec(),
            ue(),
            Prev,
            Prev,
        ),
        (
            b"\"x\\\"".to_vec(),
            false,
            4,
            4,
            NV,
            b"x\"".to_vec(),
            ue(),
            Prev,
            Prev,
        ),
        (
            b"\"x\\x\"".to_vec(),
            false,
            2,
            2,
            NV | NC,
            b"x".to_vec(),
            Is(iese(b"\\x")),
            Prev,
            Prev,
        ),
        (
            b"\"\\\"\\\\\\b\\f\\n\\r\\t\"".to_vec(),
            false,
            16,
            16,
            NV,
            b"\"\\\x08\x0c\n\r\t".to_vec(),
            Nil,
            Nil,
            Nil,
        ),
        (
            b"\"/\"".to_vec(),
            true,
            3,
            3,
            0,
            b"/".to_vec(),
            Nil,
            Nil,
            Nil,
        ),
        (
            b"\"\\/\"".to_vec(),
            false,
            4,
            4,
            NV | NC,
            b"/".to_vec(),
            Nil,
            Nil,
            Nil,
        ),
        (
            b"\"\\u002f\"".to_vec(),
            false,
            8,
            8,
            NV | NC,
            b"/".to_vec(),
            Nil,
            Nil,
            Nil,
        ),
        (
            b"\"\\u".to_vec(),
            false,
            1,
            1,
            NV,
            b"".to_vec(),
            ue(),
            Prev,
            Prev,
        ),
        (
            b"\"\\uf".to_vec(),
            false,
            1,
            1,
            NV,
            b"".to_vec(),
            ue(),
            Prev,
            Prev,
        ),
        (
            b"\"\\uff".to_vec(),
            false,
            1,
            1,
            NV,
            b"".to_vec(),
            ue(),
            Prev,
            Prev,
        ),
        (
            b"\"\\ufff".to_vec(),
            false,
            1,
            1,
            NV,
            b"".to_vec(),
            ue(),
            Prev,
            Prev,
        ),
        (
            b"\"\\ufffd".to_vec(),
            false,
            7,
            7,
            NV | NC,
            fffd.to_vec(),
            ue(),
            Prev,
            Prev,
        ),
        (
            b"\"\\ufffd\"".to_vec(),
            false,
            8,
            8,
            NV | NC,
            fffd.to_vec(),
            Nil,
            Nil,
            Nil,
        ),
        (
            b"\"\\uABCD\"".to_vec(),
            false,
            8,
            8,
            NV | NC,
            "\u{abcd}".as_bytes().to_vec(),
            Nil,
            Nil,
            Nil,
        ),
        (
            b"\"\\uefX0\"".to_vec(),
            false,
            1,
            1,
            NV | NC,
            b"".to_vec(),
            Is(iese(b"\\uefX0")),
            Prev,
            Prev,
        ),
        (
            b"\"\\uDEAD".to_vec(),
            false,
            7,
            1,
            NV | NC,
            fffd.to_vec(),
            ue(),
            Prev,
            Prev,
        ),
        (
            b"\"\\uDEAD\"".to_vec(),
            false,
            8,
            1,
            NV | NC,
            fffd.to_vec(),
            Nil,
            Is(iese(b"\\uDEAD\"")),
            Prev,
        ),
        (
            b"\"\\uDEAD______\"".to_vec(),
            false,
            14,
            1,
            NV | NC,
            cat(&[fffd, b"______"]),
            Nil,
            Is(iese(b"\\uDEAD______")),
            Prev,
        ),
        (
            b"\"\\uDEAD\\uXXXX\"".to_vec(),
            false,
            7,
            1,
            NV | NC,
            fffd.to_vec(),
            Is(iese(b"\\uXXXX")),
            Is(iese(b"\\uDEAD\\uXXXX")),
            Is(iese(b"\\uXXXX")),
        ),
        (
            b"\"\\uDEAD\\uBEEF\"".to_vec(),
            false,
            14,
            1,
            NV | NC,
            cat(&[fffd, "\u{beef}".as_bytes()]),
            Nil,
            Is(iese(b"\\uDEAD\\uBEEF")),
            Prev,
        ),
        (
            b"\"\\uD800\\udea".to_vec(),
            false,
            7,
            1,
            NV | NC,
            fffd.to_vec(),
            ue(),
            Prev,
            Prev,
        ),
        (
            b"\"\\uD800\\udb".to_vec(),
            false,
            7,
            1,
            NV | NC,
            fffd.to_vec(),
            ue(),
            Is(iese(b"\\uD800\\udb")),
            ue(),
        ),
        (
            b"\"\\uD800\\udead\"".to_vec(),
            false,
            14,
            14,
            NV | NC,
            "\u{102ad}".as_bytes().to_vec(),
            Nil,
            Nil,
            Nil,
        ),
        (
            b"\"\\u0022\\u005c\\u002f\\u0008\\u000c\\u000a\\u000d\\u0009\"".to_vec(),
            false,
            50,
            50,
            NV | NC,
            b"\"\\/\x08\x0c\n\r\t".to_vec(),
            Nil,
            Nil,
            Nil,
        ),
        (
            b"\"\\u0080\\u00f6\\u20ac\\ud799\\ue000\\ufb33\\ufffd\\ud83d\\ude02\"".to_vec(),
            false,
            56,
            56,
            NV | NC,
            "\u{0080}\u{00f6}\u{20ac}\u{d799}\u{e000}\u{fb33}\u{fffd}\u{1f602}"
                .as_bytes()
                .to_vec(),
            Nil,
            Nil,
            Nil,
        ),
    ];
    for (
        input,
        simple,
        want,
        want_utf8,
        want_flags,
        want_unquote,
        want_err,
        want_err_utf8,
        want_err_unquote,
    ) in tests
    {
        let want_err = resolve(want_err, &None);
        let want_err_utf8 = resolve(want_err_utf8, &want_err);
        let want_err_unquote = resolve(want_err_unquote, &want_err_utf8);

        let got = consume_simple_string(&input);
        if simple {
            assert_eq!(got, want, "consumeSimpleString({input:?})");
        } else {
            assert_eq!(got, 0, "consumeSimpleString({input:?})");
        }

        let mut got_flags = ValueFlags::default();
        let (got, got_err) = consume_string(&mut got_flags, &input, false);
        assert_eq!(
            got_flags,
            ValueFlags::from_bits_for_tests(want_flags),
            "consumeString({input:?}, false) flags"
        );
        assert_eq!(
            (got, &got_err),
            (want, &want_err),
            "consumeString({input:?}, false)"
        );

        let (got, got_err) = consume_string(&mut got_flags, &input, true);
        assert_eq!(
            (got, &got_err),
            (want_utf8, &want_err_utf8),
            "consumeString({input:?}, true)"
        );

        let mut got_unquote = Vec::new();
        let got_err = append_unquote(&mut got_unquote, &input);
        assert_eq!(
            (&got_unquote, &got_err),
            (&want_unquote, &want_err_unquote),
            "AppendUnquote({input:?})"
        );
    }
}

// Go: decode_test.go:TestConsumeNumber
#[test]
fn consume_number_table() {
    let ue = || Some(Err::UnexpectedEof);
    let d = "in number (expecting digit)";
    let big = "9876543210".repeat(1000);
    let neg_big = format!("-{big}");
    let tests: Vec<(&str, bool, usize, Option<Err>)> = vec![
        ("", false, 0, ue()),
        ("\"NaN\"", false, 0, Some(ice(b"\"", d))),
        ("\"Infinity\"", false, 0, Some(ice(b"\"", d))),
        ("\"-Infinity\"", false, 0, Some(ice(b"\"", d))),
        (".0", false, 0, Some(ice(b".", d))),
        ("0", true, 1, None),
        ("-0", false, 2, None),
        ("+0", false, 0, Some(ice(b"+", d))),
        ("1", true, 1, None),
        ("-1", false, 2, None),
        ("00", true, 1, None),
        ("-00", false, 2, None),
        ("01", true, 1, None),
        ("-01", false, 2, None),
        ("0i", true, 1, None),
        ("-0i", false, 2, None),
        ("0f", true, 1, None),
        ("-0f", false, 2, None),
        ("9876543210", true, 10, None),
        ("-9876543210", false, 11, None),
        ("9876543210x", true, 10, None),
        ("-9876543210x", false, 11, None),
        (" 9876543210", true, 0, Some(ice(b" ", d))),
        ("- 9876543210", false, 1, Some(ice(b" ", d))),
        (&big, true, 10000, None),
        (&neg_big, false, 1 + 10000, None),
        ("0.", false, 1, ue()),
        ("-0.", false, 2, ue()),
        ("0e", false, 1, ue()),
        ("-0e", false, 2, ue()),
        ("0E", false, 1, ue()),
        ("-0E", false, 2, ue()),
        ("0.0", false, 3, None),
        ("-0.0", false, 4, None),
        ("0e0", false, 3, None),
        ("-0e0", false, 4, None),
        ("0E0", false, 3, None),
        ("-0E0", false, 4, None),
        ("0.0123456789", false, 12, None),
        ("-0.0123456789", false, 13, None),
        ("1.f", false, 2, Some(ice(b"f", d))),
        ("-1.f", false, 3, Some(ice(b"f", d))),
        ("1.e", false, 2, Some(ice(b"e", d))),
        ("-1.e", false, 3, Some(ice(b"e", d))),
        ("1e0", false, 3, None),
        ("-1e0", false, 4, None),
        ("1E0", false, 3, None),
        ("-1E0", false, 4, None),
        ("1Ex", false, 2, Some(ice(b"x", d))),
        ("-1Ex", false, 3, Some(ice(b"x", d))),
        ("1e-0", false, 4, None),
        ("-1e-0", false, 5, None),
        ("1e+0", false, 4, None),
        ("-1e+0", false, 5, None),
        ("1E-0", false, 4, None),
        ("-1E-0", false, 5, None),
        ("1E+0", false, 4, None),
        ("-1E+0", false, 5, None),
        ("1E+00500", false, 8, None),
        ("-1E+00500", false, 9, None),
        ("1E+00500x", false, 8, None),
        ("-1E+00500x", false, 9, None),
        ("9876543210.0123456789e+01234589x", false, 31, None),
        ("-9876543210.0123456789e+01234589x", false, 32, None),
        ("1_000_000", true, 1, None),
        ("0x12ef", true, 1, None),
        ("0x1p-2", true, 1, None),
    ];
    for (input, simple, want, want_err) in tests {
        let got = consume_simple_number(input.as_bytes());
        if simple {
            assert_eq!(got, want, "ConsumeSimpleNumber({input:?})");
        } else {
            assert_eq!(got, 0, "ConsumeSimpleNumber({input:?})");
        }
        let (got, got_err) = consume_number(input.as_bytes());
        assert_eq!(
            (got, &got_err),
            (want, &want_err),
            "ConsumeNumber({input:?})"
        );
    }
}

// Go: decode_test.go:TestParseHexUint16
#[test]
fn parse_hex_uint16_table() {
    let tests: &[(&str, u16, bool)] = &[
        ("", 0, false),
        ("a", 0, false),
        ("ab", 0, false),
        ("abc", 0, false),
        ("abcd", 0xabcd, true),
        ("abcde", 0, false),
        ("9eA1", 0x9ea1, true),
        ("gggg", 0, false),
        ("0000", 0x0000, true),
        ("1234", 0x1234, true),
    ];
    for (input, want, want_ok) in tests {
        assert_eq!(
            super::decode::parse_hex_uint16_for_tests(input.as_bytes()),
            (*want, *want_ok),
            "parseHexUint16({input:?})"
        );
    }
}

// Go: encode_test.go:TestAppendQuote
#[test]
fn append_quote_table() {
    let fffd: &[u8] = "\u{fffd}".as_bytes();
    let q = |parts: &[&[u8]]| -> Vec<u8> {
        let mut v = vec![b'"'];
        v.extend_from_slice(&parts.concat());
        v.push(b'"');
        v
    };
    let ascii: &[u8] = b" !#$%&'()*+,-./0123456789:;<=>?@[]^_`{|}~\x7f";
    let iu = || Some(Err::InvalidUtf8);
    #[allow(clippy::type_complexity)]
    let tests: Vec<(Vec<u8>, u64, Vec<u8>, Option<Err>)> = vec![
        (b"".to_vec(), 0, b"\"\"".to_vec(), None),
        (b"hello".to_vec(), 0, b"\"hello\"".to_vec(), None),
        (b"\x00".to_vec(), 0, b"\"\\u0000\"".to_vec(), None),
        (b"\x1f".to_vec(), 0, b"\"\\u001f\"".to_vec(), None),
        (
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz".to_vec(),
            0,
            b"\"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz\"".to_vec(),
            None,
        ),
        (ascii.to_vec(), 0, q(&[ascii]), None),
        (
            ascii.to_vec(),
            jsonflags::ESCAPE_FOR_HTML,
            b"\" !#$%\\u0026'()*+,-./0123456789:;\\u003c=\\u003e?@[]^_`{|}~\x7f\"".to_vec(),
            None,
        ),
        (ascii.to_vec(), jsonflags::ESCAPE_FOR_JS, q(&[ascii]), None),
        (
            "\u{2027}\u{2028}\u{2029}\u{2030}".as_bytes().to_vec(),
            0,
            q(&["\u{2027}\u{2028}\u{2029}\u{2030}".as_bytes()]),
            None,
        ),
        (
            "\u{2027}\u{2028}\u{2029}\u{2030}".as_bytes().to_vec(),
            jsonflags::ESCAPE_FOR_HTML,
            q(&["\u{2027}\u{2028}\u{2029}\u{2030}".as_bytes()]),
            None,
        ),
        (
            "\u{2027}\u{2028}\u{2029}\u{2030}".as_bytes().to_vec(),
            jsonflags::ESCAPE_FOR_JS,
            q(&["\u{2027}".as_bytes(), b"\\u2028\\u2029", "\u{2030}".as_bytes()]),
            None,
        ),
        (cat(&[b"x\x80", fffd]), 0, q(&[b"x", fffd, fffd]), iu()),
        (cat(&[b"x\xff", fffd]), 0, q(&[b"x", fffd, fffd]), iu()),
        (b"x\xc0".to_vec(), 0, q(&[b"x", fffd]), iu()),
        (b"x\xc0\x80".to_vec(), 0, q(&[b"x", fffd, fffd]), iu()),
        (b"x\xe0".to_vec(), 0, q(&[b"x", fffd]), iu()),
        (b"x\xe0\x80".to_vec(), 0, q(&[b"x", fffd, fffd]), iu()),
        (b"x\xe0\x80\x80".to_vec(), 0, q(&[b"x", fffd, fffd, fffd]), iu()),
        (b"x\xf0".to_vec(), 0, q(&[b"x", fffd]), iu()),
        (b"x\xf0\x80".to_vec(), 0, q(&[b"x", fffd, fffd]), iu()),
        (b"x\xf0\x80\x80".to_vec(), 0, q(&[b"x", fffd, fffd, fffd]), iu()),
        (b"x\xf0\x80\x80\x80".to_vec(), 0, q(&[b"x", fffd, fffd, fffd, fffd]), iu()),
        (b"x\xed\xba\xad".to_vec(), 0, q(&[b"x", fffd, fffd, fffd]), iu()),
        (
            b"\"\\/\x08\x0c\n\r\t".to_vec(),
            0,
            b"\"\\\"\\\\/\\b\\f\\n\\r\\t\"".to_vec(),
            None,
        ),
        (
            "٩(-̮̮̃-̃)۶ ٩(●̮̮̃•̃)۶ ٩(͡๏̯͡๏)۶ ٩(-̮̮̃•̃).".as_bytes().to_vec(),
            0,
            q(&["٩(-̮̮̃-̃)۶ ٩(●̮̮̃•̃)۶ ٩(͡๏̯͡๏)۶ ٩(-̮̮̃•̃).".as_bytes()]),
            None,
        ),
        (
            "\u{0080}\u{00f6}\u{20ac}\u{d799}\u{e000}\u{fb33}\u{fffd}\u{1f602}".as_bytes().to_vec(),
            0,
            q(&["\u{0080}\u{00f6}\u{20ac}\u{d799}\u{e000}\u{fb33}\u{fffd}\u{1f602}".as_bytes()]),
            None,
        ),
        (
            "\u{0000}\u{001f}\u{0020}\u{0022}\u{0026}\u{003c}\u{003e}\u{005c}\u{007f}\u{0080}\u{2028}\u{2029}\u{fffd}\u{1f602}"
                .as_bytes()
                .to_vec(),
            0,
            q(&[
                b"\\u0000\\u001f \\\"&<>\\\\\x7f",
                "\u{0080}\u{2028}\u{2029}\u{fffd}\u{1f602}".as_bytes(),
            ]),
            None,
        ),
    ];
    for (input, flag_bits, want, want_err_utf8) in tests {
        let mut flags = Flags::default();
        if flag_bits != 0 {
            flags.set(flag_bits | 1);
        }

        flags.set(jsonflags::ALLOW_INVALID_UTF8 | 1);
        let mut got = Vec::new();
        let got_err = append_quote(&mut got, &input, &flags);
        assert_eq!((&got, &got_err), (&want, &None), "AppendQuote({input:?})");

        flags.set(jsonflags::ALLOW_INVALID_UTF8);
        let mut got = Vec::new();
        let got_err = append_quote(&mut got, &input, &flags);
        match &want_err_utf8 {
            None => assert_eq!((&got, &got_err), (&want, &None), "AppendQuote({input:?})"),
            Some(_) => {
                assert!(want.starts_with(&got), "AppendQuote({input:?}) = {got:?}");
                assert_eq!(got_err, want_err_utf8, "AppendQuote({input:?})");
            }
        }
    }
}

// Go: encode_test.go:TestAppendNumber
#[test]
fn append_number_table() {
    let f32b = |b: u32| f32::from_bits(b) as f64;
    let tests: Vec<(f64, &str, &str)> = vec![
        (std::f64::consts::E, "2.7182817", "2.718281828459045"),
        (std::f64::consts::PI, "3.1415927", "3.141592653589793"),
        (1.401298464324817e-45, "1e-45", "1.401298464324817e-45"), // math.SmallestNonzeroFloat32
        (5e-324, "0", "5e-324"),                                   // math.SmallestNonzeroFloat64
        (f32::MAX as f64, "3.4028235e+38", "3.4028234663852886e+38"),
        (f64::MAX, "", "1.7976931348623157e+308"),
        (0.1111111111111111, "0.11111111", "0.1111111111111111"),
        (0.2222222222222222, "0.22222222", "0.2222222222222222"),
        (0.3333333333333333, "0.33333334", "0.3333333333333333"),
        (0.4444444444444444, "0.44444445", "0.4444444444444444"),
        (0.5555555555555555, "0.5555556", "0.5555555555555555"),
        (0.6666666666666666, "0.6666667", "0.6666666666666666"),
        (0.7777777777777777, "0.7777778", "0.7777777777777777"),
        (0.8888888888888888, "0.8888889", "0.8888888888888888"),
        (0.9999999999999999, "1", "0.9999999999999999"),
        // RFC 8785, appendix B.
        (f64::from_bits(0x0000000000000000), "0", "0"),
        (f64::from_bits(0x8000000000000000), "-0", "-0"),
        (f64::from_bits(0x0000000000000001), "0", "5e-324"),
        (f64::from_bits(0x8000000000000001), "-0", "-5e-324"),
        (
            f64::from_bits(0x7fefffffffffffff),
            "",
            "1.7976931348623157e+308",
        ),
        (
            f64::from_bits(0xffefffffffffffff),
            "",
            "-1.7976931348623157e+308",
        ),
        (
            f64::from_bits(0x4340000000000000),
            "9007199000000000",
            "9007199254740992",
        ),
        (
            f64::from_bits(0xc340000000000000),
            "-9007199000000000",
            "-9007199254740992",
        ),
        (
            f64::from_bits(0x4430000000000000),
            "295147900000000000000",
            "295147905179352830000",
        ),
        (
            f64::from_bits(0x44b52d02c7e14af5),
            "1e+23",
            "9.999999999999997e+22",
        ),
        (f64::from_bits(0x44b52d02c7e14af6), "1e+23", "1e+23"),
        (
            f64::from_bits(0x44b52d02c7e14af7),
            "1e+23",
            "1.0000000000000001e+23",
        ),
        (
            f64::from_bits(0x444b1ae4d6e2ef4e),
            "1e+21",
            "999999999999999700000",
        ),
        (
            f64::from_bits(0x444b1ae4d6e2ef4f),
            "1e+21",
            "999999999999999900000",
        ),
        (f64::from_bits(0x444b1ae4d6e2ef50), "1e+21", "1e+21"),
        (
            f64::from_bits(0x3eb0c6f7a0b5ed8c),
            "0.000001",
            "9.999999999999997e-7",
        ),
        (f64::from_bits(0x3eb0c6f7a0b5ed8d), "0.000001", "0.000001"),
        (
            f64::from_bits(0x41b3de4355555553),
            "333333340",
            "333333333.3333332",
        ),
        (
            f64::from_bits(0x41b3de4355555554),
            "333333340",
            "333333333.33333325",
        ),
        (
            f64::from_bits(0x41b3de4355555555),
            "333333340",
            "333333333.3333333",
        ),
        (
            f64::from_bits(0x41b3de4355555556),
            "333333340",
            "333333333.3333334",
        ),
        (
            f64::from_bits(0x41b3de4355555557),
            "333333340",
            "333333333.33333343",
        ),
        (
            f64::from_bits(0xbecbf647612f3696),
            "-0.0000033333333",
            "-0.0000033333333333333333",
        ),
        (
            f64::from_bits(0x43143ff3c1cb0959),
            "1424953900000000",
            "1424953923781206.2",
        ),
        // RFC 8785, appendix B, modified for 32-bit behavior.
        (f32b(0x65a96815), "9.999999e+22", "9.999998877476383e+22"),
        (f32b(0x65a96816), "1e+23", "9.999999778196308e+22"),
        (f32b(0x65a96817), "1.0000001e+23", "1.0000000678916234e+23"),
        (
            f32b(0x6258d725),
            "999999900000000000000",
            "999999879303389000000",
        ),
        (
            f32b(0x6258d726),
            "999999950000000000000",
            "999999949672133200000",
        ),
        (f32b(0x6258d727), "1e+21", "1.0000000200408773e+21"),
        (f32b(0x6258d728), "1.0000001e+21", "1.0000000904096215e+21"),
        (f32b(0x358637bc), "9.999999e-7", "9.99999883788405e-7"),
        (f32b(0x358637bd), "0.000001", "9.999999974752427e-7"),
        (
            f32b(0x358637be),
            "0.0000010000001",
            "0.0000010000001111620804",
        ),
    ];
    for (input, want32, want64) in tests {
        if !want32.is_empty() {
            let mut got = Vec::new();
            append_float(&mut got, input, 32);
            assert_eq!(
                String::from_utf8(got).unwrap(),
                want32,
                "AppendFloat({input:?}, 32)"
            );
        }
        if !want64.is_empty() {
            let mut got = Vec::new();
            append_float(&mut got, input, 64);
            assert_eq!(
                String::from_utf8(got).unwrap(),
                want64,
                "AppendFloat({input:?}, 64)"
            );
        }
    }
}
