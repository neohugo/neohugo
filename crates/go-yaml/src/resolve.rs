//! Implicit and explicit scalar tag resolution (YAML 1.1 rules).
//!
//! Go: gopkg.in/yaml.v2@v2.4.0 resolve.go.

use crate::gostd::{parse_float, parse_int, parse_timestamp_ok, parse_uint};
use crate::yamlh::*;

/// The value a scalar resolves to (Go: the `interface{}` returned by
/// `resolve`). A resolved `time.Time` is represented by `Timestamp`: yaml.v2
/// never stores it into the targets this crate supports (it keeps the
/// original string for `interface{}` and `string` targets).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Resolved {
    Nil,
    Bool(bool),
    /// Go `int`.
    Int(i64),
    /// Go `uint64`.
    Uint64(u64),
    Float64(f64),
    Str(Vec<u8>),
    Timestamp,
}

/// Go's `math.NaN()` bit pattern (0x7FF8000000000001), which differs from
/// Rust's `f64::NAN`. It matters when the value is hashed.
pub const GO_NAN_BITS: u64 = 0x7FF8000000000001;

pub(crate) fn go_nan() -> f64 {
    f64::from_bits(GO_NAN_BITS)
}

// Go: resolve.go:init resolveTable
fn resolve_table(c: u8) -> u8 {
    match c {
        b'+' | b'-' => b'S', // Sign
        b'0'..=b'9' => b'D', // Digit
        b'y' | b'Y' | b'n' | b'N' | b't' | b'T' | b'f' | b'F' | b'o' | b'O' | b'~' => b'M', // In map
        b'.' => b'.', // Float (potentially in map)
        _ => 0,
    }
}

// Go: resolve.go:init resolveMap
fn resolve_map(s: &[u8]) -> Option<(&'static str, Resolved)> {
    let r = match s {
        b"y" | b"Y" | b"yes" | b"Yes" | b"YES" => (BOOL_TAG, Resolved::Bool(true)),
        b"true" | b"True" | b"TRUE" => (BOOL_TAG, Resolved::Bool(true)),
        b"on" | b"On" | b"ON" => (BOOL_TAG, Resolved::Bool(true)),
        b"n" | b"N" | b"no" | b"No" | b"NO" => (BOOL_TAG, Resolved::Bool(false)),
        b"false" | b"False" | b"FALSE" => (BOOL_TAG, Resolved::Bool(false)),
        b"off" | b"Off" | b"OFF" => (BOOL_TAG, Resolved::Bool(false)),
        b"" | b"~" | b"null" | b"Null" | b"NULL" => (NULL_TAG, Resolved::Nil),
        b".nan" | b".NaN" | b".NAN" => (FLOAT_TAG, Resolved::Float64(go_nan())),
        b".inf" | b".Inf" | b".INF" => (FLOAT_TAG, Resolved::Float64(f64::INFINITY)),
        b"+.inf" | b"+.Inf" | b"+.INF" => (FLOAT_TAG, Resolved::Float64(f64::INFINITY)),
        b"-.inf" | b"-.Inf" | b"-.INF" => (FLOAT_TAG, Resolved::Float64(f64::NEG_INFINITY)),
        b"<<" => (MERGE_TAG, Resolved::Str(b"<<".to_vec())),
        _ => return None,
    };
    Some(r)
}

const LONG_TAG_PREFIX: &str = "tag:yaml.org,2002:";

// Go: resolve.go:shortTag
pub(crate) fn short_tag(tag: &str) -> String {
    if let Some(rest) = tag.strip_prefix(LONG_TAG_PREFIX) {
        return format!("!!{rest}");
    }
    tag.to_string()
}

// Go: resolve.go:resolvableTag
fn resolvable_tag(tag: &str) -> bool {
    matches!(
        tag,
        "" | STR_TAG | BOOL_TAG | INT_TAG | FLOAT_TAG | NULL_TAG | TIMESTAMP_TAG
    )
}

// Go: resolve.go:yamlStyleFloat
// `^[-+]?(\.[0-9]+|[0-9]+(\.[0-9]*)?)([eE][-+]?[0-9]+)?$`
fn yaml_style_float(s: &[u8]) -> bool {
    let mut i = 0;
    if i < s.len() && (s[i] == b'-' || s[i] == b'+') {
        i += 1;
    }
    let digits = |i: &mut usize| {
        let st = *i;
        while *i < s.len() && s[*i].is_ascii_digit() {
            *i += 1;
        }
        *i - st
    };
    if i < s.len() && s[i] == b'.' {
        i += 1;
        if digits(&mut i) == 0 {
            return false;
        }
    } else {
        if digits(&mut i) == 0 {
            return false;
        }
        if i < s.len() && s[i] == b'.' {
            i += 1;
            digits(&mut i);
        }
    }
    if i < s.len() && (s[i] == b'e' || s[i] == b'E') {
        i += 1;
        if i < s.len() && (s[i] == b'-' || s[i] == b'+') {
            i += 1;
        }
        if digits(&mut i) == 0 {
            return false;
        }
    }
    i == s.len()
}

/// Go: `int(intv)` result of the resolver: `if intv == int64(int(intv))`
/// always holds with 64-bit `int`.
fn int_result(intv: i64) -> (&'static str, Resolved) {
    (INT_TAG, Resolved::Int(intv))
}

/// The error raised by the deferred check in `resolve` (Go: failf).
#[derive(Debug)]
pub(crate) struct ResolveError(pub Vec<u8>);

// Go: resolve.go:resolve
//
// Returns the resolved tag (as an owned String, since explicit tags can be
// arbitrary) and value.
pub(crate) fn resolve(tag: &str, input: &[u8]) -> Result<(String, Resolved), ResolveError> {
    if !resolvable_tag(tag) {
        return Ok((tag.to_string(), Resolved::Str(input.to_vec())));
    }

    let (rtag, out) = resolve_inner(tag, input);

    // Go: the deferred function.
    if tag.is_empty() || tag == rtag || tag == STR_TAG || tag == BINARY_TAG {
        return Ok((rtag.to_string(), out));
    }
    if tag == FLOAT_TAG
        && rtag == INT_TAG
        && let Resolved::Int(v) = out
    {
        return Ok((FLOAT_TAG.to_string(), Resolved::Float64(v as f64)));
    }
    let mut msg = format!("cannot decode {} `", short_tag(rtag)).into_bytes();
    msg.extend_from_slice(input);
    msg.extend_from_slice(format!("` as a {}", short_tag(tag)).as_bytes());
    Err(ResolveError(msg))
}

fn resolve_inner(tag: &str, input: &[u8]) -> (&'static str, Resolved) {
    // Any data is accepted as a !!str or !!binary.
    // Otherwise, the prefix is enough of a hint about what it might be.
    let mut hint = b'N';
    if !input.is_empty() {
        hint = resolve_table(input[0]);
    }
    if hint != 0 && tag != STR_TAG && tag != BINARY_TAG {
        // Handle things we can lookup in a map.
        if let Some(item) = resolve_map(input) {
            return item;
        }

        // Base 60 floats are a bad idea, were dropped in YAML 1.2, and
        // are purposefully unsupported here. They're still quoted on
        // the way out for compatibility with other parser, though.

        match hint {
            b'M' => {
                // We've already checked the map above.
            }
            b'.' => {
                // Not in the map, so maybe a normal float.
                if let Ok(floatv) = parse_float(input) {
                    return (FLOAT_TAG, Resolved::Float64(floatv));
                }
            }
            b'D' | b'S' => {
                // Int, float, or timestamp.
                // Only try values as a timestamp if the value is unquoted or there's an explicit
                // !!timestamp tag.
                if (tag.is_empty() || tag == TIMESTAMP_TAG) && parse_timestamp_ok(input) {
                    return (TIMESTAMP_TAG, Resolved::Timestamp);
                }

                let plain: Vec<u8> = input.iter().copied().filter(|&c| c != b'_').collect();
                if let Ok(intv) = parse_int(&plain, 0) {
                    return int_result(intv);
                }
                if let Ok(uintv) = parse_uint(&plain, 0) {
                    return (INT_TAG, Resolved::Uint64(uintv));
                }
                if yaml_style_float(&plain)
                    && let Ok(floatv) = parse_float(&plain)
                {
                    return (FLOAT_TAG, Resolved::Float64(floatv));
                }
                if plain.starts_with(b"0b") {
                    if let Ok(intv) = parse_int(&plain[2..], 2) {
                        return int_result(intv);
                    }
                    if let Ok(uintv) = parse_uint(&plain[2..], 2) {
                        return (INT_TAG, Resolved::Uint64(uintv));
                    }
                } else if plain.starts_with(b"-0b") {
                    let mut s = b"-".to_vec();
                    s.extend_from_slice(&plain[3..]);
                    if let Ok(intv) = parse_int(&s, 2) {
                        return int_result(intv);
                    }
                }
            }
            _ => {
                // Go: panic("resolveTable item not yet handled: ...") — unreachable.
            }
        }
    }
    (STR_TAG, Resolved::Str(input.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(s: &str) -> Resolved {
        resolve("", s.as_bytes()).unwrap().1
    }

    #[test]
    fn plain_scalars() {
        assert_eq!(r("yes"), Resolved::Bool(true));
        assert_eq!(r("Off"), Resolved::Bool(false));
        assert_eq!(r("~"), Resolved::Nil);
        assert_eq!(r(""), Resolved::Nil);
        assert_eq!(r("010"), Resolved::Int(8));
        assert_eq!(r("08"), Resolved::Float64(8.0));
        assert_eq!(r("0b101"), Resolved::Int(5));
        assert_eq!(r("0b-101"), Resolved::Int(-5));
        assert_eq!(r("1_000"), Resolved::Int(1000));
        assert_eq!(r("18446744073709551615"), Resolved::Uint64(u64::MAX));
        assert_eq!(r("1e400"), Resolved::Str(b"1e400".to_vec()));
        assert_eq!(r("2001-12-14"), Resolved::Timestamp);
        assert_eq!(r(".5"), Resolved::Float64(0.5));
        assert_eq!(r("+.INF"), Resolved::Float64(f64::INFINITY));
        assert_eq!(r("hello"), Resolved::Str(b"hello".to_vec()));
    }

    #[test]
    fn explicit_tags() {
        assert_eq!(resolve(FLOAT_TAG, b"1").unwrap().1, Resolved::Float64(1.0));
        assert!(resolve(FLOAT_TAG, b"error").is_err());
        assert_eq!(
            resolve(FLOAT_TAG, b"error").unwrap_err().0,
            b"cannot decode !!str `error` as a !!float"
        );
        assert_eq!(
            resolve(STR_TAG, b"12").unwrap().1,
            Resolved::Str(b"12".to_vec())
        );
        assert_eq!(
            resolve("!foo", b"12").unwrap().1,
            Resolved::Str(b"12".to_vec())
        );
    }
}
