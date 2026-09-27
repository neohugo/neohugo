//! Port of the `time.Time` marshaler of `encoding/json/v2/arshal_time.go`
//! (go1.27.1), for the v1 default (RFC 3339 with nanoseconds). Format tags
//! and `time.Duration` are not reachable from the v1 API with `Value`s.

use go_time::GoTimeExt as _;
use go_value::Time;

use super::Enc;
use super::errors::new_marshal_error_before;
use crate::goerr::{Err, MarshalerErr};
use crate::jsonflags;

// Go: arshal_time.go:makeTimeArshaler (marshal, time.Time)
pub(crate) fn marshal_time(enc: &mut Enc, tt: &Time) -> Option<Err> {
    let t = "time.Time";
    // (no `format` tag: base 0, RFC 3339)
    if enc.opts.flags.get(jsonflags::STRING_TAG)
        && !enc
            .opts
            .flags
            .get(jsonflags::REPORT_ERRORS_WITH_LEGACY_SEMANTICS)
    {
        return Some(new_marshal_error_before(
            enc,
            t,
            Some(Err::InvalidStringTag),
        ));
    }
    // k is always '"' for the RFC 3339 representation.
    let err = enc.append_raw(b'"', true, &mut |b: &mut Vec<u8>| append_marshal(tt, b));
    if let Some(err) = err {
        if enc
            .opts
            .flags
            .get(jsonflags::REPORT_ERRORS_WITH_LEGACY_SEMANTICS)
        {
            // Go: internal.NewMarshalerError(va.Addr().Interface(), err, "MarshalJSON")
            return Some(Err::Marshaler(Box::new(MarshalerErr {
                type_name: "*time.Time".to_string(),
                err,
                source_func: "MarshalJSON",
            })));
        }
        return Some(err);
    }
    None
}

// Go: arshal_time.go:timeArshaler.appendMarshal (base 0)
fn append_marshal(tt: &Time, b: &mut Vec<u8>) -> Option<Err> {
    let n0 = b.len();
    tt.append_format(b, go_time::RFC3339_NANO.as_bytes());
    // Not all Go timestamps can be represented as valid RFC 3339.
    // Explicitly check for these edge cases.
    // See https://go.dev/issue/4556 and https://go.dev/issue/54580.
    let s = &b[n0..];
    if s[4] != b'-' {
        // year must be exactly 4 digits wide
        return Some(Err::Msg("year outside of range [0,9999]".to_string()));
    } else if s[s.len() - 1] != b'Z' {
        let c = s[s.len() - "Z07:00".len()];
        if c.is_ascii_digit() || parse_dec2(&s[s.len() - "07:00".len()..]) >= 24 {
            return Some(Err::Msg(
                "timezone hour outside of range [0,23]".to_string(),
            ));
        }
    }
    None
}

// Go: arshal_time.go:parseDec2
/// parseDec2 parses b as an unsigned, base-10, 2-digit number.
/// The result is undefined if digits are not base-10.
fn parse_dec2(b: &[u8]) -> u8 {
    if b.len() < 2 {
        return 0;
    }
    10u8.wrapping_mul(b[0].wrapping_sub(b'0'))
        .wrapping_add(b[1].wrapping_sub(b'0'))
}
