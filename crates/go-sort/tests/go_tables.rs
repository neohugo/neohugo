//! Ports of the table-driven tests of go1.27.1 `src/sort/search_test.go`,
//! `src/sort/sort_test.go` and `src/slices/sort_test.go` that the oracle
//! fixtures do not already cover verbatim.

// Go's test tables use the literal 3.14 (not PI).
#![allow(clippy::approx_constant)]

use std::cmp::Ordering;

use go_sort::{slices, sort};

fn ord(v: i64) -> Ordering {
    v.cmp(&0)
}

fn str1() -> Vec<&'static str> {
    vec!["foo"]
}

/// The (data, target, wantPos, wantFound) table shared by
/// `sort.TestFind` and `slices.TestBinarySearch`.
fn find_table() -> Vec<(Vec<&'static str>, &'static str, usize, bool)> {
    let str2 = vec!["ab", "ca"];
    let str3 = vec!["mo", "qo", "vo"];
    let str4 = vec!["ab", "ad", "ca", "xy"];
    // slice with repeating elements
    let str_repeats = vec!["ba", "ca", "da", "da", "da", "ka", "ma", "ma", "ta"];
    // slice with all element equal
    let str_same = vec!["xx", "xx", "xx"];
    vec![
        (vec![], "foo", 0, false),
        (vec![], "", 0, false),
        (str1(), "foo", 0, true),
        (str1(), "bar", 0, false),
        (str1(), "zx", 1, false),
        (str2.clone(), "aa", 0, false),
        (str2.clone(), "ab", 0, true),
        (str2.clone(), "ad", 1, false),
        (str2.clone(), "ca", 1, true),
        (str2, "ra", 2, false),
        (str3.clone(), "bb", 0, false),
        (str3.clone(), "mo", 0, true),
        (str3.clone(), "nb", 1, false),
        (str3.clone(), "qo", 1, true),
        (str3.clone(), "tr", 2, false),
        (str3.clone(), "vo", 2, true),
        (str3, "xr", 3, false),
        (str4.clone(), "aa", 0, false),
        (str4.clone(), "ab", 0, true),
        (str4.clone(), "ac", 1, false),
        (str4.clone(), "ad", 1, true),
        (str4.clone(), "ax", 2, false),
        (str4.clone(), "ca", 2, true),
        (str4.clone(), "cc", 3, false),
        (str4.clone(), "dd", 3, false),
        (str4.clone(), "xy", 3, true),
        (str4, "zz", 4, false),
        (str_repeats.clone(), "da", 2, true),
        (str_repeats.clone(), "db", 5, false),
        (str_repeats.clone(), "ma", 6, true),
        (str_repeats, "mb", 8, false),
        (str_same.clone(), "xx", 0, true),
        (str_same.clone(), "ab", 0, false),
        (str_same, "zz", 3, false),
    ]
}

// Go: sort/search_test.go:TestFind
#[test]
fn go_test_find() {
    for (data, target, want_pos, want_found) in find_table() {
        let (pos, found) = sort::find(data.len(), |i| target.cmp(data[i]) as i32);
        assert_eq!(
            (pos, found),
            (want_pos, want_found),
            "Find {target:?} in {data:?}"
        );
    }
}

// Go: sort/search_test.go:TestSearchExhaustive
#[test]
fn go_test_search_exhaustive() {
    for size in 0..=100usize {
        for targ in 0..=size {
            let i = sort::search(size, |i| i >= targ);
            assert_eq!(i, targ, "Search({size}, {targ})");
        }
    }
}

// Go: sort/search_test.go:TestFindExhaustive
#[test]
fn go_test_find_exhaustive() {
    // For each size, we have a (unmaterialized) sequence of integers:
    //   2,4...size*2
    // And we're looking for every possible integer between 1 and size*2 + 1.
    for size in 0..=100i64 {
        for x in 1..=size * 2 + 1 {
            // Encodes the unmaterialized sequence with elem[i] == (i+1)*2
            let (pos, found) = sort::find(size as usize, |i| (x - (i as i64 + 1) * 2) as i32);
            let (want_pos, want_found) = if x % 2 == 0 {
                ((x / 2 - 1) as usize, true)
            } else {
                ((x / 2) as usize, false)
            };
            assert_eq!((pos, found), (want_pos, want_found), "Find({size}, {x})");
        }
    }
}

// Go: sort/search_test.go:TestSearchWrappers
#[test]
fn go_test_search_wrappers() {
    let data: Vec<i64> = vec![-10, -5, 0, 1, 2, 3, 5, 7, 11, 100, 100, 100, 1000, 10000];
    let fdata = [-3.14, 0.0, 1.0, 2.0, 1000.7];
    let sdata = ["f", "foo", "foobar", "x"];
    assert_eq!(sort::search_ints(&data, 11), 8);
    assert_eq!(sort::search_float64s(&fdata, 2.1), 4);
    assert_eq!(sort::search_strings(&sdata, b""), 0);
    let mut d2 = data.clone();
    assert_eq!(sort::IntSlice(&mut d2).search(0), 2);
    let mut f2 = fdata;
    assert_eq!(sort::Float64Slice(&mut f2).search(2.0), 3);
    let mut s2 = sdata;
    assert_eq!(sort::StringSlice(&mut s2).search(b"x"), 3);
}

// Go: sort/sort_test.go:TestBreakPatterns (must not panic, must sort)
#[test]
fn go_test_break_patterns() {
    // Special slice used to trigger breakPatterns.
    let mut data = vec![10i64; 30];
    let n = data.len();
    data[n / 4] = 0;
    data[(n / 4) * 2] = 1;
    data[(n / 4) * 3] = 2;
    sort::sort(&mut sort::IntSlice(&mut data));
    assert!(sort::ints_are_sorted(&data));
}

// Go: sort/sort_test.go:TestReverseSortIntSlice
#[test]
fn go_test_reverse_sort_int_slice() {
    let ints: Vec<i64> = vec![
        74, 59, 238, -784, 9845, 959, 905, 0, 0, 42, 7586, -5467984, 7586,
    ];
    let mut data = ints.clone();
    let mut data1 = ints.clone();
    sort::sort(&mut sort::IntSlice(&mut data));
    sort::sort(&mut sort::Reverse(sort::IntSlice(&mut data1)));
    for i in 0..data.len() {
        assert_eq!(
            data[i],
            data1[data.len() - i - 1],
            "reverse sort didn't sort"
        );
    }
}

// Go: slices/sort_test.go:TestMinMax
#[test]
fn go_test_min_max() {
    let int_cmp = |a: &i64, b: &i64| ord(a - b);
    let tests: &[(&[i64], i64, i64)] = &[
        (&[7], 7, 7),
        (&[1, 2], 1, 2),
        (&[2, 1], 1, 2),
        (&[1, 2, 3], 1, 3),
        (&[3, 2, 1], 1, 3),
        (&[2, 1, 3], 1, 3),
        (&[2, 2, 3], 2, 3),
        (&[3, 2, 3], 2, 3),
        (&[0, 2, -9], -9, 2),
    ];
    for &(data, want_min, want_max) in tests {
        assert_eq!(slices::min(data), want_min, "Min {data:?}");
        assert_eq!(
            slices::min_func(data, int_cmp),
            want_min,
            "MinFunc {data:?}"
        );
        assert_eq!(slices::max(data), want_max, "Max {data:?}");
        assert_eq!(
            slices::max_func(data, int_cmp),
            want_max,
            "MaxFunc {data:?}"
        );
    }

    // type S struct { a int; b string }; cmpS compares a only.
    let svals = [(1, "a"), (2, "a"), (1, "b"), (2, "b")];
    let cmp_s = |x: &(i64, &str), y: &(i64, &str)| x.0.cmp(&y.0);
    assert_eq!(slices::min_func(&svals, cmp_s), (1, "a"));
    assert_eq!(slices::max_func(&svals, cmp_s), (2, "a"));
}

// Go: slices/sort_test.go:TestMinMaxNaNs
#[test]
fn go_test_min_max_nans() {
    let fs = [1.0, 999.9, 3.14, -400.4, -5.14];
    assert_eq!(slices::min(&fs), -400.4);
    assert_eq!(slices::max(&fs), 999.9);
    // No matter which element of fs is replaced with a NaN, both Min and Max
    // should propagate the NaN to their output.
    for i in 0..fs.len() {
        let mut testfs = fs;
        testfs[i] = f64::NAN;
        assert!(slices::min(&testfs).is_nan());
        assert!(slices::max(&testfs).is_nan());
    }
}

// Go: slices/sort_test.go:TestMinMaxPanics
#[test]
fn go_test_min_max_panics() {
    let empty: [i64; 0] = [];
    assert!(std::panic::catch_unwind(|| slices::min(&empty)).is_err());
    assert!(std::panic::catch_unwind(|| slices::max(&empty)).is_err());
    assert!(std::panic::catch_unwind(|| slices::min_func(&empty, |a, b| a.cmp(b))).is_err());
    assert!(std::panic::catch_unwind(|| slices::max_func(&empty, |a, b| a.cmp(b))).is_err());
}

// Go: slices/sort_test.go:TestBinarySearch
#[test]
fn go_test_binary_search() {
    for (data, target, want_pos, want_found) in find_table() {
        assert_eq!(
            slices::binary_search(&data, &target),
            (want_pos, want_found),
            "BinarySearch {target:?} in {data:?}"
        );
        assert_eq!(
            slices::binary_search_func(&data, &target, |a, b| a.cmp(b)),
            (want_pos, want_found),
            "BinarySearchFunc {target:?} in {data:?}"
        );
    }
}

// Go: slices/sort_test.go:TestBinarySearchInts
#[test]
fn go_test_binary_search_ints() {
    let data: [i64; 8] = [20, 30, 40, 50, 60, 70, 80, 90];
    for (target, want_pos, want_found) in
        [(20, 0, true), (23, 1, false), (43, 3, false), (80, 6, true)]
    {
        assert_eq!(
            slices::binary_search(&data, &target),
            (want_pos, want_found)
        );
        assert_eq!(
            slices::binary_search_func(&data, &target, |a, b| ord(a - b)),
            (want_pos, want_found)
        );
    }
}

// Go: slices/sort_test.go:TestBinarySearchFloats
#[test]
fn go_test_binary_search_floats() {
    let data = [f64::NAN, -0.25, 0.0, 1.4];
    for (target, want_pos, want_found) in [
        (f64::NAN, 0, true),
        (f64::NEG_INFINITY, 1, false),
        (-0.25, 1, true),
        (0.0, 2, true),
        (1.4, 3, true),
        (1.5, 4, false),
    ] {
        assert_eq!(
            slices::binary_search(&data, &target),
            (want_pos, want_found),
            "{target}"
        );
    }
}

// Go: slices/sort_test.go:TestBinarySearchFunc
#[test]
fn go_test_binary_search_func() {
    let data = [1i64, 10, 11, 2]; // sorted lexicographically
    let (pos, found) = slices::binary_search_func(&data, "2", |a, b| a.to_string().as_str().cmp(b));
    assert_eq!((pos, found), (3, true));
}
