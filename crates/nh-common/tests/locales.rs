//! Differential tests for `nh_common::locales` (localescompressed en/th) against
//! `tests/fixtures/locales/locales.json`, written by `tools/go-oracle/nh-common/locales` built for
//! arm64 (identical to the amd64 output).

use std::sync::Arc;

use go_time::GoTimeExt;
use go_value::Location;
use nh_common::locales::{Translator, get_currency, get_translator};
use serde_json::Value as J;

fn fixture() -> J {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/locales/locales.json"
    );
    let b = std::fs::read(path).expect("read fixture");
    serde_json::from_slice(&b).expect("parse fixture")
}

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

/// A fixture result: a string, or `{"panic": msg}`.
fn result(v: &J) -> Result<String, String> {
    match v {
        J::String(s) => Ok(s.clone()),
        J::Object(o) => Err(o["panic"].as_str().unwrap().to_string()),
        other => panic!("bad result {other}"),
    }
}

fn zones(f: &J) -> Vec<Arc<Location>> {
    f["zones"]
        .as_array()
        .unwrap()
        .iter()
        .map(|z| {
            let name = z["name"].as_str().unwrap();
            match z["kind"].as_str().unwrap() {
                "utc" => go_time::utc(),
                "fixed" => go_time::fixed_zone(name, z["offset"].as_i64().unwrap()),
                "tzdata" => {
                    go_time::load_location_from_tz_data(name, &hex(z["tzdata"].as_str().unwrap()))
                        .unwrap()
                }
                k => panic!("zone kind {k}"),
            }
        })
        .collect()
}

fn translators() -> [(&'static str, Arc<dyn Translator>); 2] {
    [
        ("en", get_translator("en").unwrap().unwrap()),
        ("th", get_translator("th").unwrap().unwrap()),
    ]
}

fn err_msg(r: nh_common::Result<String>) -> Result<String, String> {
    r.map_err(|e| e.message().to_string())
}

#[test]
fn dates_and_times_match_go() {
    let f = fixture();
    let zones = zones(&f);
    let trs = translators();
    let mut bad = Vec::new();
    let mut n = 0;
    for c in f["cases"].as_array().unwrap() {
        if c["kind"] != "time" {
            continue;
        }
        n += 1;
        let loc = &zones[c["zone"].as_u64().unwrap() as usize];
        let t = go_time::unix(c["unix"].as_i64().unwrap(), c["nsec"].as_i64().unwrap()).in_loc(loc);
        for (l, tr) in &trs {
            let want = &c[*l];
            let got = [
                ("date_short", tr.fmt_date_short(&t)),
                ("date_medium", tr.fmt_date_medium(&t)),
                ("date_long", tr.fmt_date_long(&t)),
                ("date_full", tr.fmt_date_full(&t)),
                ("time_short", tr.fmt_time_short(&t)),
                ("time_medium", tr.fmt_time_medium(&t)),
                ("time_long", tr.fmt_time_long(&t)),
                ("time_full", tr.fmt_time_full(&t)),
            ];
            for (name, got) in got {
                if want[name].as_str() != Some(got.as_str()) {
                    bad.push(format!(
                        "{l} {name} {} ({}): got {got:?} want {}",
                        t.string(),
                        loc.name,
                        want[name]
                    ));
                }
            }
        }
    }
    assert!(n > 700, "too few time cases: {n}");
    assert!(
        bad.is_empty(),
        "{} mismatches:\n{}",
        bad.len(),
        bad[..bad.len().min(40)].join("\n")
    );
}

#[test]
fn numbers_match_go() {
    let f = fixture();
    let trs = translators();
    let mut bad = Vec::new();
    let mut n = 0;
    for c in f["cases"].as_array().unwrap() {
        if c["kind"] != "number" {
            continue;
        }
        n += 1;
        let num = f64::from_bits(u64::from_str_radix(c["n"].as_str().unwrap(), 16).unwrap());
        let v = c["v"].as_u64().unwrap();
        for (l, tr) in &trs {
            let want = c[*l].as_object().unwrap();
            for (key, w) in want {
                let got: Result<String, String> = match key.as_str() {
                    "number" => err_msg(tr.try_fmt_number(num, v)),
                    "percent" => Ok(tr.fmt_percent(num, v)),
                    "cardinal" => Ok(tr.cardinal_plural_rule(num, v).string().to_string()),
                    "ordinal" => Ok(tr.ordinal_plural_rule(num, v).string().to_string()),
                    "range" => Ok(tr.range_plural_rule(num, v, num, v).string().to_string()),
                    k => {
                        let (kind, cur) = k.split_once(':').unwrap();
                        match kind {
                            "currency" => err_msg(tr.try_fmt_currency(num, v, cur)),
                            "accounting" => err_msg(tr.try_fmt_accounting(num, v, cur)),
                            _ => panic!("unknown key {k}"),
                        }
                    }
                };
                let want = if key == "cardinal" || key == "ordinal" || key == "range" {
                    Ok(w.as_str().unwrap().to_string())
                } else {
                    result(w)
                };
                if got != want {
                    bad.push(format!(
                        "{l} {key}({num:?}, {v}): got {got:?} want {want:?}"
                    ));
                }
            }
        }
    }
    assert!(n > 800, "too few number cases: {n}");
    assert!(
        bad.is_empty(),
        "{} mismatches:\n{}",
        bad.len(),
        bad[..bad.len().min(40)].join("\n")
    );
}

#[test]
fn panicking_formats_panic_like_go() {
    let (_, en) = &translators()[0];
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| en.fmt_number(f64::NAN, 3)));
    let msg = r.err().and_then(|p| p.downcast_ref::<String>().cloned());
    assert_eq!(
        msg.as_deref(),
        Some("runtime error: slice bounds out of range [:-1]")
    );
    assert_eq!(en.fmt_number(1234567.891, 2), "1,234,567.89");
    // strconv rounds half to even: 1234.5 -> "1234".
    assert_eq!(en.fmt_currency(-1234.5, 0, "usd"), "-$1,234.00");
    assert_eq!(en.fmt_accounting(-1234.5, 1, "USD"), "($1,234.50)");
}

fn strs(v: &J) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect()
}

fn own(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn names_match_go() {
    let f = fixture();
    let trs = translators();
    let mut n = 0;
    for c in f["cases"].as_array().unwrap() {
        if c["kind"] != "names" {
            continue;
        }
        n += 1;
        let l = c["locale"].as_str().unwrap();
        let tr = &trs.iter().find(|(x, _)| *x == l).unwrap().1;
        assert_eq!(tr.locale(), c["Locale"].as_str().unwrap());
        let rules = |rs: &[nh_common::locales::PluralRule]| {
            rs.iter()
                .map(|r| r.string().to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            rules(tr.plurals_cardinal()),
            strs(&c["PluralsCardinal"]),
            "{l}"
        );
        assert_eq!(
            rules(tr.plurals_ordinal()),
            strs(&c["PluralsOrdinal"]),
            "{l}"
        );
        assert_eq!(rules(tr.plurals_range()), strs(&c["PluralsRange"]), "{l}");
        assert_eq!(
            own(tr.months_abbreviated()),
            strs(&c["MonthsAbbreviated"]),
            "{l}"
        );
        assert_eq!(own(tr.months_narrow()), strs(&c["MonthsNarrow"]), "{l}");
        assert_eq!(own(tr.months_wide()), strs(&c["MonthsWide"]), "{l}");
        assert_eq!(
            own(tr.weekdays_abbreviated()),
            strs(&c["WeekdaysAbbreviated"]),
            "{l}"
        );
        assert_eq!(own(tr.weekdays_narrow()), strs(&c["WeekdaysNarrow"]), "{l}");
        assert_eq!(own(tr.weekdays_short()), strs(&c["WeekdaysShort"]), "{l}");
        assert_eq!(own(tr.weekdays_wide()), strs(&c["WeekdaysWide"]), "{l}");
        let months = |f: &dyn Fn(u32) -> String| (1..=12).map(f).collect::<Vec<_>>();
        let days = |f: &dyn Fn(u32) -> String| (0..=6).map(f).collect::<Vec<_>>();
        assert_eq!(
            months(&|m| tr.month_abbreviated(m).to_string()),
            strs(&c["MonthAbbreviated"]),
            "{l}"
        );
        assert_eq!(
            months(&|m| tr.month_narrow(m).to_string()),
            strs(&c["MonthNarrow"]),
            "{l}"
        );
        assert_eq!(
            months(&|m| tr.month_wide(m).to_string()),
            strs(&c["MonthWide"]),
            "{l}"
        );
        assert_eq!(
            days(&|d| tr.weekday_abbreviated(d).to_string()),
            strs(&c["WeekdayAbbreviated"]),
            "{l}"
        );
        assert_eq!(
            days(&|d| tr.weekday_narrow(d).to_string()),
            strs(&c["WeekdayNarrow"]),
            "{l}"
        );
        assert_eq!(
            days(&|d| tr.weekday_short(d).to_string()),
            strs(&c["WeekdayShort"]),
            "{l}"
        );
        assert_eq!(
            days(&|d| tr.weekday_wide(d).to_string()),
            strs(&c["WeekdayWide"]),
            "{l}"
        );
    }
    assert_eq!(n, 2);
}

#[test]
fn get_translator_matches_go() {
    let f = fixture();
    let mut n = 0;
    for c in f["cases"].as_array().unwrap() {
        if c["kind"] != "translator" {
            continue;
        }
        n += 1;
        let key = c["key"].as_str().unwrap();
        let found = c["found"].as_bool().unwrap();
        match get_translator(key) {
            Ok(Some(t)) => {
                assert!(found, "{key:?}");
                assert_eq!(t.locale(), c["locale"].as_str().unwrap(), "{key:?}");
            }
            Ok(None) => assert!(!found, "{key:?} not found, Go has it"),
            Err(e) => {
                // A locale Go has that the port does not: an explicit error, never a fallback.
                assert!(found, "{key:?}: {}", e.message());
                assert!(
                    e.message().starts_with("neohugo-rs: locale"),
                    "{}",
                    e.message()
                );
                assert!(e.message().contains("is not supported"), "{}", e.message());
            }
        }
    }
    assert!(n > 10);
    // Translators are shared, like Go's cache.
    let a = get_translator("en").unwrap().unwrap();
    let b = get_translator("EN").unwrap().unwrap();
    assert!(Arc::ptr_eq(&a, &b));
}

#[test]
fn get_currency_matches_go() {
    let f = fixture();
    let mut n = 0;
    for c in f["cases"].as_array().unwrap() {
        if c["kind"] != "currency" {
            continue;
        }
        n += 1;
        let key = c["key"].as_str().unwrap();
        assert_eq!(
            get_currency(key),
            c["type"].as_i64().unwrap(),
            "GetCurrency({key:?})"
        );
    }
    assert!(n > 10);
}
