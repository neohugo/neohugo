//! Ports of `langs/language_test.go` (TestCollator) and checks of the `*langs.Language`
//! template API and the `Languages` helpers.

use std::sync::Arc;

use go_value::{Object, Value};
use nh_langs::config::LanguageConfig;
use nh_langs::language::{Language, LanguageObject, as_index_set, as_set, languages_to_value};

#[test]
fn collator_is_safe_to_share() {
    let l = Language::new("en", "en", "", LanguageConfig::default()).unwrap();
    let coll = l.collator1().clone();
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let coll = coll.clone();
            std::thread::spawn(move || {
                let mut c = coll.lock();
                for _ in 0..10 {
                    assert_eq!(c.compare_strings(b"abc", b"def"), -1);
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn language_template_api() {
    let cfg = LanguageConfig {
        language_name: "🇹🇭 ไทย".into(),
        language_code: String::new(),
        title: "T".into(),
        language_direction: "ltr".into(),
        weight: 2,
        disabled: false,
    };
    let th = Language::new("th", "en", "", cfg).unwrap();
    let en = Language::new(
        "en",
        "en",
        "",
        LanguageConfig {
            language_code: "en-US".into(),
            weight: 1,
            ..LanguageConfig::default()
        },
    )
    .unwrap();
    assert_eq!(th.language_code(), "th");
    assert_eq!(en.language_code(), "en-US");
    assert_eq!(th.location().name, "UTC");
    assert_eq!(th.translator().locale(), "th");
    assert_eq!(th.collator1().tag, "th");

    let o = LanguageObject(th.clone());
    assert_eq!(o.type_name(), "*langs.Language");
    assert_eq!(o.field("Lang"), Some(Value::string("th")));
    assert_eq!(o.field("Weight"), Some(Value::int(2)));
    assert_eq!(
        o.call_method(&(), "LanguageCode", &[]).unwrap().unwrap(),
        Value::string("th")
    );
    assert_eq!(
        o.call_method(&(), "String", &[]).unwrap().unwrap(),
        Value::string("th")
    );
    assert_eq!(
        go_fmt::sprint(&[Value::object(LanguageObject(th.clone()))]),
        b"th"
    );

    let ls: Vec<Arc<Language>> = vec![en, th];
    assert_eq!(as_index_set(&ls).get("th"), Some(&1));
    assert_eq!(as_set(&ls).len(), 2);
    let v = languages_to_value(&ls);
    assert_eq!(v.go_type_name(), "langs.Languages");
    let Value::List(l) = &v else { panic!() };
    assert!(nh_langs::language::language_from_value(&l.items[1]).is_some());
}

#[test]
fn bad_time_zone() {
    let e = Language::new("en", "en", "Nowhere/Nothing", LanguageConfig::default())
        .err()
        .unwrap();
    assert!(
        e.message()
            .starts_with("invalid timeZone for language \"en\": "),
        "{e}"
    );
}
