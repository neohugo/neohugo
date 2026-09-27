//! Port of Go's `sort` package (go1.27.1: `src/sort/sort.go`, `slice.go`,
//! `search.go`; the algorithms of `zsortinterface.go`/`zsortfunc.go` live in
//! the crate-private `pdqsort` module).

use crate::pdqsort::{self, LessSwap, bits_len};

/// Go `sort.Interface`. `less` and `swap` take `&mut self` so that
/// implementations may carry mutable state (caches, call counters).
pub trait Interface {
    /// Len is the number of elements in the collection.
    fn len(&self) -> usize;
    /// Less reports whether the element with index i must sort before the
    /// element with index j.
    fn less(&mut self, i: usize, j: usize) -> bool;
    /// Swap swaps the elements with indexes i and j.
    fn swap(&mut self, i: usize, j: usize);
    /// Convenience for `len() == 0`.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Adapter from [`Interface`] (usize) to the Go-int based algorithms.
struct IfaceData<'a, I: Interface + ?Sized>(&'a mut I);

impl<I: Interface + ?Sized> LessSwap for IfaceData<'_, I> {
    #[inline]
    fn less(&mut self, i: isize, j: isize) -> bool {
        self.0.less(i as usize, j as usize)
    }
    #[inline]
    fn swap(&mut self, i: isize, j: isize) {
        self.0.swap(i as usize, j as usize)
    }
}

// Go: sort/sort.go:Sort
/// Sorts data in ascending order as determined by the Less method
/// (pdqsort, not stable). Identical call sequence to Go `sort.Sort`.
pub fn sort<I: Interface + ?Sized>(data: &mut I) {
    let n = data.len() as isize;
    if n <= 1 {
        return;
    }
    let limit = bits_len(n);
    pdqsort::pdqsort(&mut IfaceData(data), 0, n, limit);
}

// Go: sort/sort.go:Stable
/// Sorts data in ascending order as determined by the Less method, while
/// keeping the original order of equal elements (insertion sort blocks of 20
/// + SymMerge). Identical call sequence to Go `sort.Stable`.
pub fn stable<I: Interface + ?Sized>(data: &mut I) {
    let n = data.len() as isize;
    pdqsort::stable(&mut IfaceData(data), n);
}

// Go: sort/sort.go:IsSorted
/// Reports whether data is sorted.
pub fn is_sorted<I: Interface + ?Sized>(data: &mut I) -> bool {
    let n = data.len();
    let mut i = n as isize - 1;
    while i > 0 {
        if data.less(i as usize, i as usize - 1) {
            return false;
        }
        i -= 1;
    }
    true
}

// Go: sort/sort.go:reverse + Reverse
/// Go `sort.Reverse`: the reverse order for data.
pub struct Reverse<I>(pub I);

impl<I: Interface> Interface for Reverse<I> {
    fn len(&self) -> usize {
        self.0.len()
    }
    // Less returns the opposite of the embedded implementation's Less method.
    fn less(&mut self, i: usize, j: usize) -> bool {
        self.0.less(j, i)
    }
    fn swap(&mut self, i: usize, j: usize) {
        self.0.swap(i, j)
    }
}

impl<I: Interface + ?Sized> Interface for &mut I {
    fn len(&self) -> usize {
        (**self).len()
    }
    fn less(&mut self, i: usize, j: usize) -> bool {
        (**self).less(i, j)
    }
    fn swap(&mut self, i: usize, j: usize) {
        (**self).swap(i, j)
    }
}

/// Go `sort.lessSwap` as a public helper: an [`Interface`] built from a
/// length and two closures. When both closures need the same data, capture
/// it through a `RefCell`/`Cell`, or implement [`Interface`] directly.
pub struct LessSwapFuncs<L, S>
where
    L: FnMut(usize, usize) -> bool,
    S: FnMut(usize, usize),
{
    pub len: usize,
    pub less: L,
    pub swap: S,
}

impl<L, S> Interface for LessSwapFuncs<L, S>
where
    L: FnMut(usize, usize) -> bool,
    S: FnMut(usize, usize),
{
    fn len(&self) -> usize {
        self.len
    }
    fn less(&mut self, i: usize, j: usize) -> bool {
        (self.less)(i, j)
    }
    fn swap(&mut self, i: usize, j: usize) {
        (self.swap)(i, j)
    }
}

/// Interface-style sort from closures: `sort.Sort(lessSwap{less, swap})`.
pub fn sort_less_swap(
    n: usize,
    less: impl FnMut(usize, usize) -> bool,
    swap: impl FnMut(usize, usize),
) {
    sort(&mut LessSwapFuncs { len: n, less, swap });
}

/// Interface-style stable sort from closures: `sort.Stable(lessSwap{less, swap})`.
pub fn stable_less_swap(
    n: usize,
    less: impl FnMut(usize, usize) -> bool,
    swap: impl FnMut(usize, usize),
) {
    stable(&mut LessSwapFuncs { len: n, less, swap });
}

// Go: sort/sort.go:IntSlice
/// Go `sort.IntSlice` (Go `int` is 64-bit).
pub struct IntSlice<'a>(pub &'a mut [i64]);

impl Interface for IntSlice<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn less(&mut self, i: usize, j: usize) -> bool {
        self.0[i] < self.0[j]
    }
    fn swap(&mut self, i: usize, j: usize) {
        self.0.swap(i, j)
    }
}

impl IntSlice<'_> {
    /// x.Sort() calls Sort(x).
    pub fn sort(&mut self) {
        sort(self)
    }
    /// Go `IntSlice.Search`.
    pub fn search(&self, x: i64) -> usize {
        search_ints(self.0, x)
    }
}

// Go: sort/sort.go:Float64Slice
/// Go `sort.Float64Slice`: NaN values are ordered before other values.
pub struct Float64Slice<'a>(pub &'a mut [f64]);

impl Interface for Float64Slice<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn less(&mut self, i: usize, j: usize) -> bool {
        let (x, y) = (self.0[i], self.0[j]);
        x < y || (x.is_nan() && !y.is_nan())
    }
    fn swap(&mut self, i: usize, j: usize) {
        self.0.swap(i, j)
    }
}

impl Float64Slice<'_> {
    /// x.Sort() calls Sort(x).
    pub fn sort(&mut self) {
        sort(self)
    }
    /// Go `Float64Slice.Search`.
    pub fn search(&self, x: f64) -> usize {
        search_float64s(self.0, x)
    }
}

// Go: sort/sort.go:StringSlice
/// Go `sort.StringSlice` over any byte-string type (bytewise order).
pub struct StringSlice<'a, S: AsRef<[u8]>>(pub &'a mut [S]);

impl<S: AsRef<[u8]>> Interface for StringSlice<'_, S> {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn less(&mut self, i: usize, j: usize) -> bool {
        self.0[i].as_ref() < self.0[j].as_ref()
    }
    fn swap(&mut self, i: usize, j: usize) {
        self.0.swap(i, j)
    }
}

impl<S: AsRef<[u8]>> StringSlice<'_, S> {
    /// x.Sort() calls Sort(x).
    pub fn sort(&mut self) {
        sort(self)
    }
    /// Go `StringSlice.Search`.
    pub fn search(&self, x: &[u8]) -> usize {
        search(self.0.len(), |i| self.0[i].as_ref() >= x)
    }
}

// Go: sort/sort.go:Ints
/// Sorts a slice of ints in increasing order (as of Go 1.22, `slices.Sort`).
pub fn ints(x: &mut [i64]) {
    crate::slices::sort(x)
}

// Go: sort/sort.go:Float64s
/// Sorts a slice of float64s in increasing order, NaNs first
/// (as of Go 1.22, `slices.Sort`).
pub fn float64s(x: &mut [f64]) {
    crate::slices::sort(x)
}

// Go: sort/sort.go:Strings
/// Sorts a slice of strings in increasing order (as of Go 1.22, `slices.Sort`).
pub fn strings<S: crate::cmp::Ordered>(x: &mut [S]) {
    crate::slices::sort(x)
}

// Go: sort/sort.go:IntsAreSorted
pub fn ints_are_sorted(x: &[i64]) -> bool {
    crate::slices::is_sorted(x)
}

// Go: sort/sort.go:Float64sAreSorted
pub fn float64s_are_sorted(x: &[f64]) -> bool {
    crate::slices::is_sorted(x)
}

// Go: sort/sort.go:StringsAreSorted
pub fn strings_are_sorted<S: crate::cmp::Ordered>(x: &[S]) -> bool {
    crate::slices::is_sorted(x)
}

/// Adapter for Go `sort.Slice`: `less(x, i, j)` sees the current slice.
struct SliceIdx<'a, T, F: FnMut(&[T], usize, usize) -> bool> {
    x: &'a mut [T],
    less: F,
}

impl<T, F: FnMut(&[T], usize, usize) -> bool> LessSwap for SliceIdx<'_, T, F> {
    #[inline]
    fn less(&mut self, i: isize, j: isize) -> bool {
        (self.less)(self.x, i as usize, j as usize)
    }
    #[inline]
    fn swap(&mut self, i: isize, j: isize) {
        self.x.swap(i as usize, j as usize)
    }
}

// Go: sort/slice.go:Slice
/// Go `sort.Slice(x, less)`. The less closure receives the slice (in its
/// current, partially sorted state) and two indices, as a Go closure over `x`
/// would see it.
pub fn slice<T, F: FnMut(&[T], usize, usize) -> bool>(x: &mut [T], less: F) {
    let length = x.len() as isize;
    let limit = bits_len(length);
    pdqsort::pdqsort(&mut SliceIdx { x, less }, 0, length, limit);
}

// Go: sort/slice.go:SliceStable
/// Go `sort.SliceStable(x, less)`.
pub fn slice_stable<T, F: FnMut(&[T], usize, usize) -> bool>(x: &mut [T], less: F) {
    let n = x.len() as isize;
    pdqsort::stable(&mut SliceIdx { x, less }, n);
}

// Go: sort/slice.go:SliceIsSorted
/// Go `sort.SliceIsSorted(x, less)`.
pub fn slice_is_sorted<T, F: FnMut(&[T], usize, usize) -> bool>(x: &[T], mut less: F) -> bool {
    let n = x.len() as isize;
    let mut i = n - 1;
    while i > 0 {
        if less(x, i as usize, i as usize - 1) {
            return false;
        }
        i -= 1;
    }
    true
}

// Go: sort/search.go:Search
/// Binary search for the smallest index i in [0, n) at which f(i) is true,
/// assuming f(i) == true implies f(i+1) == true. Returns n if none.
pub fn search(n: usize, mut f: impl FnMut(usize) -> bool) -> usize {
    // Define f(-1) == false and f(n) == true.
    // Invariant: f(i-1) == false, f(j) == true.
    let (mut i, mut j) = (0isize, n as isize);
    while i < j {
        let h = ((i + j) as usize >> 1) as isize; // avoid overflow when computing h
        // i ≤ h < j
        if !f(h as usize) {
            i = h + 1; // preserves f(i-1) == false
        } else {
            j = h; // preserves f(j) == true
        }
    }
    // i == j, f(i-1) == false, and f(j) (= f(i)) == true  =>  answer is i.
    i as usize
}

// Go: sort/search.go:Find
/// Binary search for the smallest index i in [0, n) at which cmp(i) <= 0;
/// found reports whether cmp(i) == 0.
pub fn find(n: usize, mut cmp: impl FnMut(usize) -> i32) -> (usize, bool) {
    // The invariants here are similar to the ones in Search.
    // Define cmp(-1) > 0 and cmp(n) <= 0
    // Invariant: cmp(i-1) > 0, cmp(j) <= 0
    let (mut i, mut j) = (0isize, n as isize);
    while i < j {
        let h = ((i + j) as usize >> 1) as isize; // avoid overflow when computing h
        // i ≤ h < j
        if cmp(h as usize) > 0 {
            i = h + 1; // preserves cmp(i-1) > 0
        } else {
            j = h; // preserves cmp(j) <= 0
        }
    }
    // i == j, cmp(i-1) > 0 and cmp(j) <= 0
    let i = i as usize;
    (i, i < n && cmp(i) == 0)
}

// Go: sort/search.go:SearchInts
pub fn search_ints(a: &[i64], x: i64) -> usize {
    search(a.len(), |i| a[i] >= x)
}

// Go: sort/search.go:SearchFloat64s
pub fn search_float64s(a: &[f64], x: f64) -> usize {
    search(a.len(), |i| a[i] >= x)
}

// Go: sort/search.go:SearchStrings
pub fn search_strings<S: AsRef<[u8]>>(a: &[S], x: &[u8]) -> usize {
    search(a.len(), |i| a[i].as_ref() >= x)
}
