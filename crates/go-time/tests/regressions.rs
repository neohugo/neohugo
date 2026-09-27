//! Regression tests for discrepancies found by the adversarial verification
//! pass (expected values from go1.27.1).

use go_time::{Duration, GoTimeExt};

/// `Truncate`/`Round` when Go's internal seconds are MinInt64: `div` negates
/// the seconds (wrapping) and then computes `d - r` with a negative `r`,
/// which wraps in Go (it panicked in debug builds of the port).
/// Rows: (unix, nsec, d, Truncate unix, Truncate nsec, Round unix, Round nsec).
#[test]
fn div_internal_min_seconds() {
    let rows: &[(i64, i64, i64, i64, i64, i64, i64)] = &[
        (
            9223371974719179008,
            0,
            1000000000,
            9223371974719179008,
            0,
            9223371974719179008,
            0,
        ),
        (
            9223371974719179008,
            0,
            2000000000,
            9223371974719179008,
            0,
            9223371974719179008,
            0,
        ),
        (
            9223371974719179008,
            0,
            3000000000,
            9223371974719179009,
            0,
            9223371974719179009,
            0,
        ),
        (
            9223371974719179008,
            0,
            7000000000,
            9223371974719179009,
            0,
            9223371974719179009,
            0,
        ),
        (
            9223371974719179008,
            0,
            9223372036000000000,
            9223371983087775237,
            709551616,
            9223371983087775237,
            709551616,
        ),
        (
            9223371974719179008,
            0,
            1,
            9223371974719179008,
            0,
            9223371974719179008,
            0,
        ),
        (
            9223371974719179008,
            0,
            3,
            9223371974719179009,
            999999999,
            9223371974719179009,
            999999999,
        ),
        (
            9223371974719179008,
            0,
            4611686018427387905,
            9223371974719179009,
            0,
            9223371974719179009,
            0,
        ),
        (
            9223371974719179008,
            0,
            9223372036854775807,
            9223371974719179009,
            145224193,
            9223371974719179009,
            0,
        ),
        (
            9223371974719179008,
            1,
            1000000000,
            9223371974719179008,
            0,
            9223371974719179008,
            0,
        ),
        (
            9223371974719179008,
            1,
            2000000000,
            9223371974719179008,
            0,
            9223371974719179008,
            0,
        ),
        (
            9223371974719179008,
            1,
            3000000000,
            9223371974719179009,
            0,
            9223371974719179009,
            0,
        ),
        (
            9223371974719179008,
            1,
            7000000000,
            9223371974719179009,
            0,
            9223371974719179009,
            0,
        ),
        (
            9223371974719179008,
            1,
            9223372036000000000,
            9223371974719179009,
            0,
            9223371975573954816,
            0,
        ),
        (
            9223371974719179008,
            1,
            1,
            9223371974719179008,
            1,
            9223371974719179008,
            1,
        ),
        (
            9223371974719179008,
            1,
            3,
            9223371974719179009,
            999999999,
            9223371974719179008,
            2,
        ),
        (
            9223371974719179008,
            1,
            4611686018427387905,
            9223371974719179009,
            0,
            9223371974719179009,
            0,
        ),
        (
            9223371974719179008,
            1,
            9223372036854775807,
            9223371974719179009,
            145224193,
            9223371974719179009,
            0,
        ),
    ];
    for &(unix, ns, d, tu, tn, ru, rn) in rows {
        let t = go_time::unix(unix, ns).utc();
        let tr = t.truncate(Duration(d));
        let ro = t.round(Duration(d));
        assert_eq!(
            (tr.go_unix(), tr.nanosecond(), ro.go_unix(), ro.nanosecond()),
            (tu, tn, ru, rn),
            "unix={} ns={} d={}",
            unix,
            ns,
            d
        );
    }
}

/// `time.Local = time.UTC`: Local *is* `&utcLoc`, so times carrying it are
/// UTC for `MarshalBinary` (offset -1) and `GoString` ("time.UTC"). With
/// go_value's shared UTC as Local the port must behave the same (it used to
/// treat it as a zone-less Local: offset 0, "time.Local").
/// Go: `time.Local = time.UTC; t := time.Unix(0, 0)`:
/// MarshalBinary = 010000000e7791f70000000000ffff, GoString =
/// time.Date(1970, time.January, 1, 0, 0, 0, 0, time.UTC).
#[test]
fn local_is_go_value_utc() {
    go_time::set_local(go_value::Location::utc());
    let t = go_time::unix(0, 0);
    let mb = t.marshal_binary().unwrap();
    let hex: String = mb.iter().map(|b| format!("{:02x}", b)).collect();
    assert_eq!(hex, "010000000e7791f70000000000ffff");
    assert_eq!(
        t.go_string(),
        "time.Date(1970, time.January, 1, 0, 0, 0, 0, time.UTC)"
    );
    assert_eq!(t.string(), "1970-01-01 00:00:00 +0000 UTC");
    // initLocal's UTC fallback is a distinct zone-less "UTC" Location that is
    // Local, not UTC: offset 0 and "time.Local".
    let fallback = std::sync::Arc::new(go_time::init_local_from_env(Some(""), None));
    go_time::set_local(fallback);
    let t = go_time::unix(0, 0);
    let hex: String = t
        .marshal_binary()
        .unwrap()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();
    assert_eq!(hex, "010000000e7791f700000000000000");
    assert_eq!(
        t.go_string(),
        "time.Date(1970, time.January, 1, 0, 0, 0, 0, time.Local)"
    );
}
