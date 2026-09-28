//! Port of `compare/compare.go`, `compare/compare_strings.go`.
//!
//! Owner: Wave B task T01 (common-values).

//! Go `compare` package: `Strings`/`LessStrings` (ASCII + simple-fold case-insensitive compare
//! with a byte-order tiebreak) and the interfaces tpl/compare checks by method name.

use std::cmp::Ordering;

use go_unicode::utf8;
use go_value::{Kind, Value};

/// Go: `compare.Eqer` is `Eq(other any) bool`; `compare.ProbablyEqer` is `ProbablyEq(other any) bool`;
/// `compare.Comparer` is `Compare(other any) int`. In Rust these are method-name checks on
/// `go_value::Object` (`has_method("Eq")`), see nh_common::object docs.
pub const EQER_METHOD: &str = "Eq";
pub const PROBABLY_EQER_METHOD: &str = "ProbablyEq";
pub const COMPARER_METHOD: &str = "Compare";

/// Calls the one-argument boolean method `name` (`Eq`/`ProbablyEq`) of an object, if it has it.
fn call_bool_method(v: &Value, name: &str, other: &Value) -> Option<bool> {
    let Value::Object(o) = v else { return None };
    if !o.has_method(name) {
        return None;
    }
    match o.call_method(&(), name, std::slice::from_ref(other)) {
        Some(Ok(Value::Bool(b))) => Some(b),
        _ => Some(false),
    }
}

// Go: compare/compare.go:Eq (the `v1 == v2` interface comparison)
/// Go's `==` on two `any` values: equal dynamic types and equal values. Objects compare by
/// identity (pointers) — struct-kind objects without identity semantics are compared by
/// identity too. Go panics comparing uncomparable types (slices, maps, funcs); this returns
/// `Err` with Go's runtime error text for those.
pub fn interface_equal(v1: &Value, v2: &Value) -> Result<bool, crate::herrors::Error> {
    let t1 = crate::hreflect::type_of(v1);
    let t2 = crate::hreflect::type_of(v2);
    if t1 != t2 {
        return Ok(false);
    }
    let uncomparable = |t: &str| {
        Err(crate::herrors::Error::new(format!(
            "runtime error: comparing uncomparable type {t}"
        )))
    };
    Ok(match (v1, v2) {
        (Value::Invalid, _) | (_, Value::Invalid) => v1.is_invalid() && v2.is_invalid(),
        (Value::TypedNil(t), _) => {
            if matches!(
                go_value::typed_nil_kind(t),
                go_value::NilKind::Slice | go_value::NilKind::Map | go_value::NilKind::Func
            ) {
                return uncomparable(t);
            }
            v2.is_nil()
        }
        (_, Value::TypedNil(t)) => {
            if matches!(
                go_value::typed_nil_kind(t),
                go_value::NilKind::Slice | go_value::NilKind::Map | go_value::NilKind::Func
            ) {
                return uncomparable(t);
            }
            false
        }
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Int(a, _), Value::Int(b, _)) => a == b,
        (Value::Uint(a, _), Value::Uint(b, _)) => a == b,
        (Value::Float(a, _), Value::Float(b, _)) => a == b,
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Safe(_, a), Value::Safe(_, b)) => a == b,
        // time.Time is a comparable struct: wall, ext and loc pointer are compared.
        (Value::Time(a), Value::Time(b)) => {
            a.unix_sec == b.unix_sec
                && a.nsec == b.nsec
                && match (&a.loc, &b.loc) {
                    (None, None) => true,
                    (Some(x), Some(y)) => std::sync::Arc::ptr_eq(x, y),
                    _ => false,
                }
        }
        (Value::List(_), _) => return uncomparable(&v1.go_type_name()),
        (Value::Map(_), _) => return uncomparable(&v1.go_type_name()),
        (Value::Object(a), Value::Object(b)) => {
            if let (Some(ua), Some(ub)) = (a.underlying(), b.underlying()) {
                return interface_equal(&ua, &ub);
            }
            match a.kind() {
                Kind::Slice | Kind::Map | Kind::Func => {
                    return uncomparable(&v1.go_type_name());
                }
                _ => a.identity() == b.identity(),
            }
        }
        _ => false,
    })
}

// Go: compare/compare.go:Eq
/// Eq: nil only equals nil; an `Eqer` (an object with an `Eq` method) decides; else Go's `==`
/// ([`interface_equal`]; uncomparable values report `false` here, where Go panics).
pub fn eq(v1: &Value, v2: &Value) -> bool {
    if v1.is_invalid() || v2.is_invalid() {
        return v1.is_invalid() && v2.is_invalid();
    }
    if let Some(b) = call_bool_method(v1, EQER_METHOD, v2) {
        return b;
    }
    interface_equal(v1, v2).unwrap_or(false)
}

// Go: compare/compare.go:ProbablyEq
/// ProbablyEq: [`eq`], else a `ProbablyEqer`'s answer.
pub fn probably_eq(v1: &Value, v2: &Value) -> bool {
    if eq(v1, v2) {
        return true;
    }
    call_bool_method(v1, PROBABLY_EQER_METHOD, v2).unwrap_or(false)
}

// Go: compare/compare_strings.go:Strings
/// Strings returns an integer comparing two strings lexicographically, case-insensitively
/// (Unicode simple folding), with a byte-order tiebreak ("B" and "b" differ).
pub fn strings(s: &[u8], t: &[u8]) -> i32 {
    let c = compare_fold(s, t);

    if c == 0 {
        // "B" and "b" would be the same so we need a tiebreaker.
        return match s.cmp(t) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        };
    }

    c
}

// Go: compare/compare_strings.go:compareFold
/// This function is derived from strings.EqualFold in Go's stdlib.
fn compare_fold(mut s: &[u8], mut t: &[u8]) -> i32 {
    while !s.is_empty() && !t.is_empty() {
        let (mut sr, mut tr): (i32, i32);
        if (s[0] as i32) < utf8::RUNE_SELF {
            sr = s[0] as i32;
            s = &s[1..];
        } else {
            let (r, size) = utf8::decode_rune_in_string(s);
            sr = r;
            s = &s[size..];
        }
        if (t[0] as i32) < utf8::RUNE_SELF {
            tr = t[0] as i32;
            t = &t[1..];
        } else {
            let (r, size) = utf8::decode_rune_in_string(t);
            tr = r;
            t = &t[size..];
        }

        if tr == sr {
            continue;
        }

        let mut c = 1;
        if tr < sr {
            std::mem::swap(&mut tr, &mut sr);
            c = -c;
        }

        //  ASCII only.
        if tr < utf8::RUNE_SELF && sr >= 'A' as i32 && sr <= 'Z' as i32 {
            if tr <= 'Z' as i32 {
                // Same case.
                return -c;
            }

            let diff = tr - (sr + 'a' as i32 - 'A' as i32);

            if diff == 0 {
                continue;
            }

            if diff < 0 {
                return c;
            }

            if diff > 0 {
                return -c;
            }
        }

        // Unicode.
        let mut r = go_unicode::simple_fold(sr);
        while r != sr && r < tr {
            r = go_unicode::simple_fold(r);
        }

        if r == tr {
            continue;
        }

        return -c;
    }

    if s.is_empty() && t.is_empty() {
        return 0;
    }

    if s.is_empty() {
        return -1;
    }

    1
}

// Go: compare/compare_strings.go:LessStrings
/// LessStrings returns whether s is less than t lexicographically.
pub fn less_strings(s: &[u8], t: &[u8]) -> bool {
    strings(s, t) < 0
}

/// Ordering adapter for `strings`.
pub fn strings_ordering(s: &[u8], t: &[u8]) -> Ordering {
    strings(s, t).cmp(&0)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Go: compare/compare_strings_test.go:TestCompare
    #[test]
    fn test_compare() {
        let lower = |s: &str| go_unicode::strings::to_lower(s.as_bytes()).into_owned();
        for (a, b) in [
            ("a", "a"),
            ("A", "a"),
            ("Ab", "Ac"),
            ("az", "Za"),
            ("C", "D"),
            ("B", "a"),
            ("C", ""),
            ("", ""),
            ("αβδC", "ΑΒΔD"),
            ("αβδC", "ΑΒΔ"),
            ("αβδ", "ΑΒΔD"),
            ("αβδ", "ΑΒΔ"),
            ("β", "δ"),
            ("好", "好"),
        ] {
            let expect = match lower(a).cmp(&lower(b)) {
                Ordering::Less => -1,
                Ordering::Equal => 0,
                Ordering::Greater => 1,
            };
            assert_eq!(
                compare_fold(a.as_bytes(), b.as_bytes()),
                expect,
                "{a:?} {b:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: compare/compare.go (67 lines; 1/2 funcs executed)
//   types: Eqer, ProbablyEqer, Comparer
// OK L44-54: Eq(v1, v2 any) bool
// OK L57-67: ProbablyEq(v1, v2 any) bool
// Source: compare/compare_strings.go (113 lines; 3/3 funcs executed)
// OK L23-32: Strings(s, t string) int
// OK L36-108: compareFold(s, t string) int
// OK L111-113: LessStrings(s, t string) bool
// ---------------------------------------------------------------------------
