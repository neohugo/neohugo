//! Port of Go's sorting algorithms (go1.27.1) with identical element order
//! for comparators with ties (and identical less/swap call sequences, so even
//! inconsistent comparators give Go's result).
//!
//! * [`sort`] — package `sort`: [`sort::sort`] (pdqsort), [`sort::stable`]
//!   (insertion sort + SymMerge), [`sort::slice`], [`sort::slice_stable`],
//!   [`sort::is_sorted`], [`sort::Reverse`], `IntSlice`/`Float64Slice`/
//!   `StringSlice`, [`sort::search`], [`sort::find`], `search_ints`, ...
//! * [`slices`] — package `slices` sorting: [`slices::sort`] (cmp.Ordered),
//!   [`slices::sort_func`], [`slices::sort_stable_func`], `is_sorted*`,
//!   `min*`/`max*`, `binary_search*`.
//! * [`cmp`] — package `cmp`: [`cmp::Ordered`], [`cmp::less`], [`cmp::compare`].
//!
//! Convenience entry points: [`sort_by`] / [`stable_by`] take a boolean
//! `less(&a, &b)`; [`sort::sort_less_swap`] / [`sort::stable_less_swap`] take
//! `len` + `less(i, j)` + `swap(i, j)` closures.
//!
//! All Go variants (`sort.Sort`, `sort.Slice`, `slices.SortFunc`,
//! `slices.Sort`) run the same generated algorithm, so e.g. [`sort_by`] gives
//! exactly the order of `sort.Slice(x, func(i, j int) bool { return less(x[i], x[j]) })`.

pub mod cmp;
mod pdqsort;
pub mod slices;
pub mod sort;

/// Unstable sort with Go's pdqsort: the same result as Go `sort.Slice`,
/// `sort.Sort` or `slices.SortFunc` with the equivalent comparator.
pub fn sort_by<T, F: FnMut(&T, &T) -> bool>(x: &mut [T], less: F) {
    slices::sort_less_func(x, less)
}

/// Stable sort with Go's insertion sort + SymMerge: the same result as Go
/// `sort.Stable`, `sort.SliceStable` or `slices.SortStableFunc` with the
/// equivalent comparator.
pub fn stable_by<T, F: FnMut(&T, &T) -> bool>(x: &mut [T], less: F) {
    slices::sort_stable_less_func(x, less)
}
