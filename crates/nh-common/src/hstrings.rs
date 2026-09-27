//! Port of `common/hstrings/strings.go`.
//!
//! Owner: Wave B task T02 (common-paths-text).


/// Go: `hstrings.StringEqualFold` — a string compared case-insensitively.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StringEqualFold(pub String);

/// Go: `hstrings.EqualAny(a string, b ...string) bool`.
pub fn equal_any(a: &str, b: &[&str]) -> bool {
    b.iter().any(|x| *x == a)
}

/// Go: `hstrings.InSlice` / `InSlicEqualFold` (EqualFold uses Go simple folding).
pub fn in_slice_equal_fold(arr: &[&str], el: &str) -> bool {
    todo!("go-unicode EqualFold")
}

/// Go: `hstrings.ToString(v any) (string, bool)`.
pub fn to_string(v: &go_value::Value) -> Option<go_value::GoString> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/hstrings/strings.go (134 lines; 0/11 funcs executed)
//   types: StringEqualFold, regexpCache, (group)
//    L33-35: (s StringEqualFold) EqualFold(s2 string) bool
//    L37-39: (s StringEqualFold) String() string
//    L41-50: (s StringEqualFold) Eq(s2 any) bool
//    L53-55: EqualAny(a string, b ...string) bool
//    L63-75: (rc *regexpCache) getOrCompileRegexp(pattern string) (re *regexp.Regexp, err error)
//    L77-82: (rc *regexpCache) get(key string) (re *regexp.Regexp, ok bool)
//    L84-88: (rc *regexpCache) set(key string, re *regexp.Regexp)
//    L95-97: GetOrCompileRegexp(pattern string) (re *regexp.Regexp, err error)
//    L101-103: InSlice(arr []string, el string) bool
//    L108-115: InSlicEqualFold(arr []string, el string) bool
//    L121-129: ToString(v any) (string, bool)
// ---------------------------------------------------------------------------
