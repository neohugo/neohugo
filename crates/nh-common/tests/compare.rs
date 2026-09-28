//! `compare`: the Go test tables (compare/compare_strings_test.go) and the differential test
//! against `tools/go-oracle/nh-common/compare` (Strings over the string corpus, the stable sort
//! by LessStrings, Eq/ProbablyEq over typed values).

mod support;

use std::any::Any;
use std::borrow::Cow;

use go_value::{HostCtx, Object, Value};
use nh_common::compare;
use support::*;

// Go: compare/compare_strings_test.go:TestCompare
#[test]
fn go_test_compare() {
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
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        };
        // compareFold is private; Strings equals it whenever the fold result is not 0.
        let got = compare::strings(a.as_bytes(), b.as_bytes());
        if expect != 0 {
            assert_eq!(got, expect, "{a:?} {b:?}");
        }
    }
}

// Go: compare/compare_strings_test.go:TestLexicographicSort
#[test]
fn go_test_lexicographic_sort() {
    let mut s = vec!["b", "Bz", "ba", "A", "Ba", "ba"];
    go_sort::sort::slice(&mut s, |s, i, j| {
        compare::less_strings(s[i].as_bytes(), s[j].as_bytes())
    });
    assert_eq!(s, ["A", "b", "Ba", "ba", "ba", "Bz"]);
}

/// The oracle's `*main.eqer` (compare.Eqer + compare.ProbablyEqer).
struct Eqer(String);

impl Object for Eqer {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("*main.eqer")
    }
    fn has_method(&self, name: &str) -> bool {
        matches!(name, "Eq" | "ProbablyEq")
    }
    fn call_method(
        &self,
        _: HostCtx<'_>,
        name: &str,
        args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        let other = &args[0];
        let same_id = || other.downcast::<Eqer>().is_some_and(|o| o.0 == self.0);
        match name {
            "Eq" => Some(Ok(Value::Bool(same_id()))),
            "ProbablyEq" => Some(Ok(Value::Bool(match other {
                Value::String(s) => s.as_bytes() == self.0.as_bytes(),
                _ => other.downcast::<Eqer>().is_some(),
            }))),
            _ => None,
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[test]
fn oracle() {
    init();
    let fx = fixture("compare/compare.json.gz");
    let strs: Vec<Vec<u8>> = fx["strings"]
        .as_array()
        .unwrap()
        .iter()
        .map(bytes)
        .collect();

    // The stable sort by LessStrings.
    let sorted: Vec<usize> = fx["sorted"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i.as_u64().unwrap() as usize)
        .collect();
    let mut order: Vec<usize> = (0..sorted.len()).collect();
    go_sort::sort::slice_stable(&mut order, |o, i, j| {
        compare::less_strings(&strs[o[i]], &strs[o[j]])
    });
    assert_eq!(order, sorted, "sort.SliceStable by LessStrings");

    let mut n = 0;
    let mut bad = Vec::new();
    for c in fx["cases"].as_array().unwrap() {
        let a = &strs[c["a"].as_u64().unwrap() as usize];
        let b = &strs[c["b"].as_u64().unwrap() as usize];
        let want = c["r"].as_i64().unwrap() as i32;
        n += 1;
        let got = compare::strings(a, b);
        if got != want {
            bad.push(format!(
                "Strings({:?}, {:?}) = {got}, want {want}",
                String::from_utf8_lossy(a),
                String::from_utf8_lossy(b)
            ));
        }
        if compare::less_strings(a, b) != (want < 0) {
            bad.push(format!("LessStrings({a:?}, {b:?})"));
        }
    }

    // Eq / ProbablyEq. The two *eqer values with the same id are distinct pointers.
    let values: Vec<Value> = fx["values"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            if v["t"] == "*main.eqer" {
                Value::object(Eqer(v["id"].as_str().unwrap().to_string()))
            } else {
                decode(v)
            }
        })
        .collect();
    for e in fx["eq"].as_array().unwrap() {
        let a = &values[e[0].as_u64().unwrap() as usize];
        let b = &values[e[1].as_u64().unwrap() as usize];
        n += 1;
        let (eq, peq) = (compare::eq(a, b), compare::probably_eq(a, b));
        if eq != e[2].as_bool().unwrap() || peq != e[3].as_bool().unwrap() {
            bad.push(format!(
                "Eq({a:?}, {b:?}) = {eq}/{peq}, want {}/{}",
                e[2], e[3]
            ));
        }
    }

    eprintln!("compare: {n} checks, {} mismatches", bad.len());
    for b in bad.iter().take(20) {
        eprintln!("{b}");
    }
    assert!(n > 40000, "too few checks: {n}");
    assert!(bad.is_empty());
}
