//! go-toml local dates as template values: `AsTime(*time.Location)` through
//! `nh_common::htime` (T01's `AsTimeProvider` contract), `String`, fields, `%v`.

use go_value::Value;
use nh_common::htime::{LocationRef, to_time_in_default_location_e};
use nh_parser::metadecoders::toml;

fn decode(src: &str, key: &str) -> Value {
    let Value::Map(m) = toml::unmarshal(src.as_bytes()).unwrap() else {
        panic!("not a map")
    };
    m.entries.get(key.as_bytes()).unwrap().clone()
}

#[test]
fn local_dates_implement_as_time() {
    let ict = go_time::fixed_zone("ICT", 7 * 3600);
    let d = decode("d = 2020-01-02", "d");
    assert_eq!(d.go_type_name(), "toml.LocalDate");
    let t = to_time_in_default_location_e(&d, &ict).unwrap();
    // 2020-01-02T00:00:00+07:00
    assert_eq!(t.unix_sec, 1577898000);
    assert_eq!(go_time::location_string(t.loc.as_ref()), "ICT");

    let dt = decode("d = 2020-01-02T03:04:05.5", "d");
    let t = to_time_in_default_location_e(&dt, &go_time::utc()).unwrap();
    assert_eq!((t.unix_sec, t.nsec), (1577934245, 500_000_000));

    // LocalTime has no AsTime method.
    let lt = decode("d = 03:04:05", "d");
    let Value::Object(o) = &lt else { panic!() };
    assert!(!o.has_method("AsTime"));
    assert!(to_time_in_default_location_e(&lt, &go_time::utc()).is_err());

    let Value::Object(o) = &dt else { panic!() };
    let r = o
        .call_method(&(), "AsTime", &[Value::object(LocationRef(ict.clone()))])
        .unwrap()
        .unwrap();
    let Value::Time(t) = r else { panic!() };
    assert_eq!(t.unix_sec, 1577934245 - 7 * 3600);
    assert_eq!(
        o.call_method(&(), "String", &[]).unwrap().unwrap(),
        Value::string("2020-01-02T03:04:05.5")
    );
    assert_eq!(o.field("Year"), Some(Value::int(2020)));
    assert_eq!(o.field("Precision"), Some(Value::int(1)));
    assert_eq!(
        go_fmt::sprintf("%v|%s", &[dt.clone(), dt]),
        b"2020-01-02T03:04:05.5|2020-01-02T03:04:05.5".to_vec()
    );
}

#[test]
fn offset_date_times_are_time_values() {
    let v = decode("d = 1979-05-27T00:32:00.999999-07:00", "d");
    let Value::Time(t) = v else { panic!() };
    assert_eq!(t.unix_sec, 296638320);
    assert_eq!(t.nsec, 999_999_000);
    // Go: time.FixedZone("", -25200) prints as "".
    assert_eq!(go_time::location_string(t.loc.as_ref()), "");
    let v = decode("d = 1979-05-27T00:32:00Z", "d");
    let Value::Time(t) = v else { panic!() };
    assert_eq!(go_time::location_string(t.loc.as_ref()), "UTC");
}
