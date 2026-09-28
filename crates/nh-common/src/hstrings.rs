//! Port of `common/hstrings/strings.go`.
//!
//! Owner: Wave B task T02 (common-paths-text).

use go_unicode::strings::equal_fold;
use go_value::{GoString, Value};

use crate::herrors::{Error, Result};

/// Go: `hstrings.StringEqualFold` — a string that implements `compare.Eqer` and considers two
/// strings equal if they are equal when folded to lower case (used by the `eq` template function).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StringEqualFold(pub String);

impl StringEqualFold {
    // Go: common/hstrings/strings.go:EqualFold
    pub fn equal_fold(&self, s2: &str) -> bool {
        equal_fold(self.0.as_bytes(), s2.as_bytes())
    }

    // Go: common/hstrings/strings.go:String
    pub fn string(&self) -> &str {
        &self.0
    }

    /// Go: `Eq(s2 any)` — a string, or a `fmt.Stringer`'s `String()`, compared with EqualFold.
    // Go: common/hstrings/strings.go:Eq
    pub fn eq_any(&self, s2: &Value) -> bool {
        match string_or_stringer(s2) {
            Some(s) => equal_fold(self.0.as_bytes(), s.as_bytes()),
            None => false,
        }
    }
}

/// The Go type switch `case string: ...; case fmt.Stringer: ...`.
fn string_or_stringer(v: &Value) -> Option<GoString> {
    match v {
        Value::String(s) => Some(s.clone()),
        // time.Time implements fmt.Stringer.
        Value::Time(t) => Some(go_time::GoTimeExt::string(t).into()),
        Value::Object(o) => {
            // json.Number implements fmt.Stringer.
            if let Some(n) = o.as_any().downcast_ref::<go_json::Number>() {
                return Some(n.0.clone());
            }
            o.go_string()
        }
        _ => None,
    }
}

/// Go: `hstrings.EqualAny(a string, b ...string) bool`.
// Go: common/hstrings/strings.go:EqualAny
pub fn equal_any(a: &str, b: &[&str]) -> bool {
    b.contains(&a)
}

/// Go: `hstrings.GetOrCompileRegexp`: the compiled Go regexp for the pattern, from a
/// process-wide cache (Go's `regexpCache`: compiled once per pattern, errors are not cached).
/// The error is Go's `*syntax.Error` text.
// Go: common/hstrings/strings.go:GetOrCompileRegexp
pub fn get_or_compile_regexp(pattern: impl AsRef<[u8]>) -> Result<crate::goregexp::Regexp> {
    crate::goregexp::get_or_compile(pattern.as_ref()).map_err(|e| Error::new(e.error()))
}

/// Go: `hstrings.InSlice` — whether `el` is an element of `arr`.
// Go: common/hstrings/strings.go:InSlice
pub fn in_slice(arr: &[&str], el: &str) -> bool {
    arr.contains(&el)
}

/// Go: `hstrings.InSlicEqualFold` (`strings.EqualFold`, Go simple folding).
// Go: common/hstrings/strings.go:InSlicEqualFold
pub fn in_slice_equal_fold(arr: &[&str], el: &str) -> bool {
    for v in arr {
        if equal_fold(v.as_bytes(), el.as_bytes()) {
            return true;
        }
    }
    false
}

/// Go: `hstrings.ToString(v any) (string, bool)` — a string, or a `fmt.Stringer`'s `String()`;
/// stricter than `cast.ToString` (no numbers).
// Go: common/hstrings/strings.go:ToString
pub fn to_string(v: &Value) -> Option<GoString> {
    string_or_stringer(v)
}

/// Go: `hstrings.Strings2`.
pub type Strings2 = [String; 2];
/// Go: `hstrings.Strings3`.
pub type Strings3 = [String; 3];

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/hstrings/strings.go (134 lines; 0/11 funcs executed)
//   types: StringEqualFold, regexpCache, (group)
// OK L33-35: (s StringEqualFold) EqualFold(s2 string) bool
// OK L37-39: (s StringEqualFold) String() string
// OK L41-50: (s StringEqualFold) Eq(s2 any) bool
// OK L53-55: EqualAny(a string, b ...string) bool
// OK L63-75: (rc *regexpCache) getOrCompileRegexp(pattern string) (re *regexp.Regexp, err error)
// OK L77-82: (rc *regexpCache) get(key string) (re *regexp.Regexp, ok bool)
// OK L84-88: (rc *regexpCache) set(key string, re *regexp.Regexp)
// OK L95-97: GetOrCompileRegexp(pattern string) (re *regexp.Regexp, err error)
// OK L101-103: InSlice(arr []string, el string) bool
// OK L108-115: InSlicEqualFold(arr []string, el string) bool
// OK L121-129: ToString(v any) (string, bool)
// ---------------------------------------------------------------------------
