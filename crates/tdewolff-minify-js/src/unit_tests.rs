//! The upstream unit tests of crate-private functions, ported:
//! `util_test.go` (`TestBinaryNumber`, `TestOctalNumber`,
//! `TestHexadecimalNumber`, `TestString`, `TestHasSideEffects`) and
//! `js_test.go:TestRenamerIndices`.

use tdewolff_parse::{GoBytes, Input};
use tdewolff_parse_js::{Node, Options, parse};

use crate::util::*;
use crate::vars::Renamer;

#[allow(clippy::all)]
mod tables {
    include!("../tests/upstream_tables/mod.rs");
}

fn b(s: &str) -> GoBytes {
    GoBytes::from_slice(s.as_bytes())
}

// Go: util_test.go:TestBinaryNumber
#[test]
fn test_binary_number() {
    assert_eq!(binary_number(b("0b0"), 0).to_vec(), b"0");
    assert_eq!(binary_number(b("0b1"), 0).to_vec(), b"1");
    assert_eq!(binary_number(b("0b1001"), 0).to_vec(), b"9");
    assert_eq!(
        binary_number(
            b("0b100000000000000000000000000000000000000000000000000000000000000"),
            0
        )
        .to_vec(),
        b"4611686018427387904"
    );
    assert_eq!(
        binary_number(
            b("0b1000000000000000000000000000000000000000000000000000000000000000"),
            0
        )
        .to_vec(),
        b"0b1000000000000000000000000000000000000000000000000000000000000000"
    );
}

// Go: util_test.go:TestOctalNumber
#[test]
fn test_octal_number() {
    assert_eq!(octal_number(b("0o0"), 0).to_vec(), b"0");
    assert_eq!(octal_number(b("0o1"), 0).to_vec(), b"1");
    assert_eq!(octal_number(b("0o775"), 0).to_vec(), b"509");
    assert_eq!(
        octal_number(b("0o100000000000000000000"), 0).to_vec(),
        b"1152921504606846976"
    );
    assert_eq!(
        octal_number(b("0o1000000000000000000000"), 0).to_vec(),
        b"0o1000000000000000000000"
    );
}

// Go: util_test.go:TestHexadecimalNumber
#[test]
fn test_hexadecimal_number() {
    assert_eq!(hexadecimal_number(b("0x0"), 0).to_vec(), b"0");
    assert_eq!(hexadecimal_number(b("0x1"), 0).to_vec(), b"1");
    assert_eq!(hexadecimal_number(b("0xFE"), 0).to_vec(), b"254");
    assert_eq!(
        hexadecimal_number(b("0x1000000000"), 0).to_vec(),
        b"68719476736"
    );
    assert_eq!(
        hexadecimal_number(b("0xd000000000"), 0).to_vec(),
        b"893353197568"
    );
    assert_eq!(
        hexadecimal_number(b("0xe000000000"), 0).to_vec(),
        b"0xe000000000"
    );
    assert_eq!(
        hexadecimal_number(b("0xE000000000"), 0).to_vec(),
        b"0xE000000000"
    );
    assert_eq!(
        hexadecimal_number(b("0x10000000000"), 0).to_vec(),
        b"0x10000000000"
    );
}

// Go: util_test.go:TestString
#[test]
fn test_string() {
    for (s, expected) in tables::TEST_STRING {
        let got = minify_string(GoBytes::from_slice(s), true).to_vec();
        assert_eq!(got, expected.to_vec(), "{:?}", String::from_utf8_lossy(s));
    }
}

// Go: util_test.go:TestHasSideEffects
#[test]
fn test_has_side_effects() {
    for &(js, has) in tables::TEST_HAS_SIDE_EFFECTS {
        let ast = parse(&Input::new_string(js), Options::default()).unwrap();
        let expr = match ast.node(ast.list()[0]) {
            Node::ExprStmt(s) => s.value,
            _ => panic!("not an ExprStmt"),
        };
        assert_eq!(
            has_side_effects(&ast, expr),
            has,
            "{:?}",
            String::from_utf8_lossy(js)
        );
    }
}

// Go: js_test.go:TestRenamerIndices
#[test]
fn test_renamer_indices() {
    let renamer = Renamer::new(true, true);
    for i in [0usize, 1, 2, 53, 54, 55, 117, 118] {
        let name = renamer.get_name(b(" "), i);
        let j = renamer.get_index(&name.to_vec());
        assert_eq!(j, i as i64, "{}", name.to_string_lossy());
    }
    for i in 0..100000usize {
        let name = renamer.get_name(b(" "), i);
        let j = renamer.get_index(&name.to_vec());
        assert_eq!(j, i as i64, "{}", name.to_string_lossy());
    }
}

/// `getName` writes into the name's own bytes (shared with every alias of
/// the input) and only reallocates when the capacity is too small.
#[test]
fn get_name_in_place() {
    let renamer = Renamer::new(true, true);
    let input = GoBytes::from_slice(b"xyz+abc");
    let name = input.slice3(0, 3, 3);
    let got = renamer.get_name(name, 0);
    assert!(got.same_array(&input));
    assert_eq!(input.to_vec(), b"eyz+abc");
    let name = input.slice3(4, 5, 5);
    let got = renamer.get_name(name, 60); // two characters, capacity 1
    assert!(!got.same_array(&input));
    assert_eq!(got.to_vec(), b"ae");
    assert_eq!(input.to_vec(), b"eyz+abc");
}
