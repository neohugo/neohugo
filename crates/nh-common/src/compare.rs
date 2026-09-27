//! Port of `compare/compare.go`, `compare/compare_strings.go`.
//!
//! Owner: Wave B task T01 (common-values).


//! Go `compare` package: `Strings`/`LessStrings` (ASCII + simple-fold case-insensitive compare
//! with byte-order tiebreak) and the interfaces tpl/compare checks by method name.

use std::cmp::Ordering;

use go_value::Value;

/// Go: `compare.Eqer` is `Eq(other any) bool`; `compare.ProbablyEqer` is `ProbablyEq(other any) bool`;
/// `compare.Comparer` is `Compare(other any) int`. In Rust these are method-name checks on
/// `go_value::Object` (`has_method("Eq")`), see nh_common::object docs.
pub const EQER_METHOD: &str = "Eq";
pub const PROBABLY_EQER_METHOD: &str = "ProbablyEq";
pub const COMPARER_METHOD: &str = "Compare";

/// Go: `compare.Eq(v1, v2 any) bool` (used by internal Hugo code, not the template `eq`).
// Go: compare/compare.go:Eq
pub fn eq(v1: &Value, v2: &Value) -> bool {
    todo!()
}

/// Go: `compare.Strings(s, t string) int` — case-insensitive (simple fold) compare, then bytes.
// Go: compare/compare_strings.go:Strings
pub fn strings(s: &[u8], t: &[u8]) -> i32 {
    todo!("port compareFold exactly (ASCII fast path + unicode.SimpleFold)")
}

/// Go: `compare.LessStrings(s, t)`.
// Go: compare/compare_strings.go:LessStrings
pub fn less_strings(s: &[u8], t: &[u8]) -> bool {
    strings(s, t) < 0
}

/// Ordering adapter for `strings`.
pub fn strings_ordering(s: &[u8], t: &[u8]) -> Ordering {
    strings(s, t).cmp(&0)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: compare/compare.go (67 lines; 1/2 funcs executed)
//   types: Eqer, ProbablyEqer, Comparer
// EX L44-54: Eq(v1, v2 any) bool
//    L57-67: ProbablyEq(v1, v2 any) bool
// Source: compare/compare_strings.go (113 lines; 3/3 funcs executed)
// EX L23-32: Strings(s, t string) int
// EX L36-108: compareFold(s, t string) int
// EX L111-113: LessStrings(s, t string) bool
// ---------------------------------------------------------------------------
