//! Port of the sorting part of Go's `slices` package (go1.27.1:
//! `src/slices/sort.go`; the algorithms of `zsortanyfunc.go` and
//! `zsortordered.go` live in the crate-private `pdqsort` module).

use std::cmp::Ordering;

use crate::cmp::{self, Ordered};
use crate::pdqsort::{self, LessSwap, bits_len};

/// Adapter for `pdqsortCmpFunc`/`stableCmpFunc`: less(i, j) = `cmp(data[i], data[j]) < 0`.
struct CmpFuncData<'a, T, F: FnMut(&T, &T) -> Ordering> {
    data: &'a mut [T],
    cmp: F,
}

impl<T, F: FnMut(&T, &T) -> Ordering> LessSwap for CmpFuncData<'_, T, F> {
    #[inline]
    fn less(&mut self, i: isize, j: isize) -> bool {
        (self.cmp)(&self.data[i as usize], &self.data[j as usize]) == Ordering::Less
    }
    #[inline]
    fn swap(&mut self, i: isize, j: isize) {
        self.data.swap(i as usize, j as usize)
    }
}

/// Adapter taking a boolean "less" on elements (same call sequence as
/// `cmp(a, b) < 0`).
struct LessFuncData<'a, T, F: FnMut(&T, &T) -> bool> {
    data: &'a mut [T],
    less: F,
}

impl<T, F: FnMut(&T, &T) -> bool> LessSwap for LessFuncData<'_, T, F> {
    #[inline]
    fn less(&mut self, i: isize, j: isize) -> bool {
        (self.less)(&self.data[i as usize], &self.data[j as usize])
    }
    #[inline]
    fn swap(&mut self, i: isize, j: isize) {
        self.data.swap(i as usize, j as usize)
    }
}

/// Adapter for `pdqsortOrdered`: less(i, j) = `cmp.Less(data[i], data[j])`.
struct OrderedData<'a, T: Ordered>(&'a mut [T]);

impl<T: Ordered> LessSwap for OrderedData<'_, T> {
    #[inline]
    fn less(&mut self, i: isize, j: isize) -> bool {
        cmp::less(&self.0[i as usize], &self.0[j as usize])
    }
    #[inline]
    fn swap(&mut self, i: isize, j: isize) {
        self.0.swap(i as usize, j as usize)
    }
}

// Go: slices/sort.go:Sort
/// Sorts a slice of any ordered type in ascending order. NaNs are ordered
/// before other values; -0.0 and 0.0 are ties (their relative order is what
/// Go's pdqsort produces).
pub fn sort<T: Ordered>(x: &mut [T]) {
    let n = x.len() as isize;
    pdqsort::pdqsort(&mut OrderedData(x), 0, n, bits_len(n));
}

// Go: slices/sort.go:SortFunc
/// Sorts x in ascending order as determined by cmp (not stable).
pub fn sort_func<T, F: FnMut(&T, &T) -> Ordering>(x: &mut [T], cmp: F) {
    let n = x.len() as isize;
    pdqsort::pdqsort(&mut CmpFuncData { data: x, cmp }, 0, n, bits_len(n));
}

// Go: slices/sort.go:SortStableFunc
/// Sorts x while keeping the original order of equal elements.
pub fn sort_stable_func<T, F: FnMut(&T, &T) -> Ordering>(x: &mut [T], cmp: F) {
    let n = x.len() as isize;
    pdqsort::stable(&mut CmpFuncData { data: x, cmp }, n);
}

/// `SortFunc` with a boolean less(a, b) (equivalent to `cmp(a, b) < 0`).
/// Same algorithm and call sequence as Go `sort.Slice`, `sort.Sort` and
/// `slices.SortFunc`.
pub fn sort_less_func<T, F: FnMut(&T, &T) -> bool>(x: &mut [T], less: F) {
    let n = x.len() as isize;
    pdqsort::pdqsort(&mut LessFuncData { data: x, less }, 0, n, bits_len(n));
}

/// `SortStableFunc` with a boolean less(a, b). Same algorithm and call
/// sequence as Go `sort.Stable`, `sort.SliceStable` and `slices.SortStableFunc`.
pub fn sort_stable_less_func<T, F: FnMut(&T, &T) -> bool>(x: &mut [T], less: F) {
    let n = x.len() as isize;
    pdqsort::stable(&mut LessFuncData { data: x, less }, n);
}

// Go: slices/sort.go:IsSorted
/// Reports whether x is sorted in ascending order.
pub fn is_sorted<T: Ordered>(x: &[T]) -> bool {
    let mut i = x.len() as isize - 1;
    while i > 0 {
        if cmp::less(&x[i as usize], &x[i as usize - 1]) {
            return false;
        }
        i -= 1;
    }
    true
}

// Go: slices/sort.go:IsSortedFunc
/// Reports whether x is sorted in ascending order, with cmp as the
/// comparison function.
pub fn is_sorted_func<T, F: FnMut(&T, &T) -> Ordering>(x: &[T], mut cmp: F) -> bool {
    let mut i = x.len() as isize - 1;
    while i > 0 {
        if cmp(&x[i as usize], &x[i as usize - 1]) == Ordering::Less {
            return false;
        }
        i -= 1;
    }
    true
}

// Go: slices/sort.go:Min
/// Returns the minimal value in x (NaN-propagating for floats).
/// Panics if x is empty, like Go.
pub fn min<T: Ordered + Clone>(x: &[T]) -> T {
    if x.is_empty() {
        panic!("slices.Min: empty list");
    }
    let mut m = x[0].clone();
    for v in &x[1..] {
        m = m.go_min(v.clone());
    }
    m
}

// Go: slices/sort.go:MinFunc
/// Returns the first minimal value in x according to cmp. Panics if empty.
pub fn min_func<T: Clone, F: FnMut(&T, &T) -> Ordering>(x: &[T], mut cmp: F) -> T {
    if x.is_empty() {
        panic!("slices.MinFunc: empty list");
    }
    let mut m = &x[0];
    for v in &x[1..] {
        if cmp(v, m) == Ordering::Less {
            m = v;
        }
    }
    m.clone()
}

// Go: slices/sort.go:Max
/// Returns the maximal value in x (NaN-propagating for floats).
/// Panics if x is empty, like Go.
pub fn max<T: Ordered + Clone>(x: &[T]) -> T {
    if x.is_empty() {
        panic!("slices.Max: empty list");
    }
    let mut m = x[0].clone();
    for v in &x[1..] {
        m = m.go_max(v.clone());
    }
    m
}

// Go: slices/sort.go:MaxFunc
/// Returns the first maximal value in x according to cmp. Panics if empty.
pub fn max_func<T: Clone, F: FnMut(&T, &T) -> Ordering>(x: &[T], mut cmp: F) -> T {
    if x.is_empty() {
        panic!("slices.MaxFunc: empty list");
    }
    let mut m = &x[0];
    for v in &x[1..] {
        if cmp(v, m) == Ordering::Greater {
            m = v;
        }
    }
    m.clone()
}

// Go: slices/sort.go:BinarySearch
/// Searches for target in a sorted slice and returns the earliest position
/// where target is found, or the position where it would appear, and whether
/// it was found.
pub fn binary_search<T: Ordered>(x: &[T], target: &T) -> (usize, bool) {
    // Inlining is faster than calling BinarySearchFunc with a lambda.
    let n = x.len() as isize;
    // Define x[-1] < target and x[n] >= target.
    // Invariant: x[i-1] < target, x[j] >= target.
    let (mut i, mut j) = (0isize, n);
    while i < j {
        let h = ((i + j) as usize >> 1) as isize; // avoid overflow when computing h
        // i ≤ h < j
        if cmp::less(&x[h as usize], target) {
            i = h + 1; // preserves x[i-1] < target
        } else {
            j = h; // preserves x[j] >= target
        }
    }
    // i == j, x[i-1] < target, and x[j] (= x[i]) >= target  =>  answer is i.
    let found = i < n && {
        let xi = &x[i as usize];
        xi.go_eq(target) || (xi.go_is_nan() && target.go_is_nan())
    };
    (i as usize, found)
}

// Go: slices/sort.go:BinarySearchFunc
/// Like [`binary_search`] with a custom comparison of element and target.
pub fn binary_search_func<T, U: ?Sized, F: FnMut(&T, &U) -> Ordering>(
    x: &[T],
    target: &U,
    mut cmp: F,
) -> (usize, bool) {
    let n = x.len() as isize;
    // Define cmp(x[-1], target) < 0 and cmp(x[n], target) >= 0 .
    // Invariant: cmp(x[i - 1], target) < 0, cmp(x[j], target) >= 0.
    let (mut i, mut j) = (0isize, n);
    while i < j {
        let h = ((i + j) as usize >> 1) as isize; // avoid overflow when computing h
        // i ≤ h < j
        if cmp(&x[h as usize], target) == Ordering::Less {
            i = h + 1; // preserves cmp(x[i - 1], target) < 0
        } else {
            j = h; // preserves cmp(x[j], target) >= 0
        }
    }
    // i == j, cmp(x[i-1], target) < 0, and cmp(x[j], target) (= cmp(x[i], target)) >= 0  =>  answer is i.
    let found = i < n && cmp(&x[i as usize], target) == Ordering::Equal;
    (i as usize, found)
}
