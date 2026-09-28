//! Go's own test of langs/i18n that does not need a site: `TestGetPluralCount`
//! (langs/i18n/i18n_test.go). The site-based tables (`i18nTests`, `TestPlural`, the integration tests) run through the Go
//! oracle in `tests/translate.rs`.

mod common;

use std::sync::Arc;

use go_value::{Map, MapType, Value};
use nh_i18n::i18n::get_plural_count;

fn map(k: &str, v: Value) -> Value {
    let mut m = Map::new(MapType::StringAny);
    m.insert(k, v);
    Value::map(m)
}

fn count_field(v: Value, ptr: bool) -> Value {
    Value::Object(Arc::new(common::GoStruct {
        type_name: "i18n.countField".into(),
        ptr,
        fields: vec![("Count".into(), v)],
        methods: vec![],
    }))
}

fn count_method(ptr: bool) -> Value {
    Value::Object(Arc::new(common::GoStruct {
        type_name: "i18n.countMethod".into(),
        ptr,
        fields: vec![],
        methods: vec![("Count".into(), Value::float64(32.5))],
    }))
}

fn s(v: &str) -> Option<Value> {
    Some(Value::string(v))
}

fn i(v: i64) -> Option<Value> {
    Some(Value::int(v))
}

// Go: langs/i18n/i18n_test.go:TestGetPluralCount
#[test]
fn test_get_plural_count() {
    assert_eq!(get_plural_count(&map("Count", Value::int(32))), i(32));
    assert_eq!(get_plural_count(&map("Count", Value::int(1))), i(1));
    assert_eq!(
        get_plural_count(&map("Count", Value::float64(1.5))),
        s("1.5")
    );
    assert_eq!(
        get_plural_count(&map("Count", Value::string("32"))),
        s("32")
    );
    assert_eq!(
        get_plural_count(&map("Count", Value::string("32.5"))),
        s("32.5")
    );
    assert_eq!(get_plural_count(&map("count", Value::int(32))), i(32));
    assert_eq!(
        get_plural_count(&map("Count", Value::string("32"))),
        s("32")
    );
    assert_eq!(get_plural_count(&map("Counts", Value::int(32))), None);
    assert_eq!(get_plural_count(&Value::string("foo")), None);
    assert_eq!(get_plural_count(&count_field(Value::int(22), false)), i(22));
    assert_eq!(
        get_plural_count(&count_field(Value::float64(1.5), false)),
        s("1.5")
    );
    assert_eq!(get_plural_count(&count_field(Value::int(22), true)), i(22));
    let no_count_field = Value::Object(Arc::new(common::GoStruct {
        type_name: "i18n.noCountField".into(),
        ptr: false,
        fields: vec![("Counts".into(), Value::int(23))],
        methods: vec![],
    }));
    assert_eq!(get_plural_count(&no_count_field), None);
    assert_eq!(get_plural_count(&count_method(false)), s("32.5"));
    assert_eq!(get_plural_count(&count_method(true)), s("32.5"));

    assert_eq!(get_plural_count(&Value::int(1234)), i(1234));
    assert_eq!(get_plural_count(&Value::float64(1234.4)), s("1234.4"));
    assert_eq!(get_plural_count(&Value::float64(1234.0)), s("1234.0"));
    assert_eq!(get_plural_count(&Value::string("1234")), s("1234"));
    assert_eq!(get_plural_count(&Value::string("0.5")), s("0.5"));
    assert_eq!(get_plural_count(&Value::Invalid), None);
}
