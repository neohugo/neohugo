//! Ports of the Go test tables of cache/dynacache, common/herrors and identity (the parts a
//! one-shot build uses), plus behaviour tests for the lock-free caches, the clocks and the
//! reflect type helpers.

use std::sync::Arc;

use go_time::GoTimeExt;
use go_value::{GoString, Map, MapType, SliceType, Value};
use nh_common::dynacache::{
    Cache, ClearWhen, Options, OptionsPartition, calculate_max_size_per_partition, clean_key,
    get_or_create_partition,
};
use nh_common::herrors::{self, Error, ErrorKind};
use nh_common::hreflect;
use nh_common::htime::{self, Clock, StartClock};
use nh_common::identity::{self, IncrementByOne, Incrementer};
use nh_common::maps::cache::Cache as MapsCache;

// Go: cache/dynacache/dynacache_test.go:TestCache
#[test]
fn test_cache() {
    let cache = Cache::new(Options::default());
    let opts = OptionsPartition {
        weight: 30,
        clear_when: ClearWhen::Unset,
    };
    let p1 = get_or_create_partition::<String, i64>(&cache, "/aaaa/bbbb", opts);
    let p2 = get_or_create_partition::<String, i64>(&cache, "/aaaa/bbbb", opts);
    assert!(Arc::ptr_eq(&p1, &p2));
    let bad_name = std::panic::catch_unwind(|| {
        get_or_create_partition::<String, i64>(&cache, "foo bar", opts);
    });
    assert!(bad_name.is_err());
    let bad_weight = std::panic::catch_unwind(|| {
        get_or_create_partition::<String, i64>(
            &cache,
            "/aaaa/cccc",
            OptionsPartition {
                weight: 1234,
                ..opts
            },
        );
    });
    assert!(bad_weight.is_err());
    let p3 = get_or_create_partition::<String, i64>(&cache, "/aaaa/cccc", opts);
    assert!(!Arc::ptr_eq(&p3, &p1));
    // Go's partition names (hugolib/content_map_page.go).
    for name in [
        "/pag1/0",
        "/cont/ren/1",
        "/cont/toc/0",
        "/imgs",
        "/tmpl/openapi3",
        "/ress/12",
    ] {
        get_or_create_partition::<String, i64>(&cache, name, opts);
    }
    for bad in ["/abc", "/abcd/", "/abcd//x", "/abcd/x/y/z", "abcd", "/ab-d"] {
        let r = std::panic::catch_unwind(|| {
            get_or_create_partition::<String, i64>(&cache, bad, opts);
        });
        assert!(r.is_err(), "{bad}");
    }
}

// Go: cache/dynacache/dynacache_test.go:TestCalculateMaxSizePerPartition
#[test]
fn test_calculate_max_size_per_partition() {
    assert_eq!(calculate_max_size_per_partition(1000, 500, 5), 200);
    assert_eq!(calculate_max_size_per_partition(1000, 250, 5), 400);
    assert!(std::panic::catch_unwind(|| calculate_max_size_per_partition(1000, 250, 0)).is_err());
    assert!(std::panic::catch_unwind(|| calculate_max_size_per_partition(1000, 0, 1)).is_err());
}

// Go: cache/dynacache/dynacache_test.go:TestCleanKey
#[test]
fn test_clean_key() {
    assert_eq!(clean_key("a/b/c"), "/a/b/c");
    assert_eq!(clean_key("/a/b/c"), "/a/b/c");
    assert_eq!(clean_key("a/b/c/"), "/a/b/c");
    assert_eq!(clean_key("/a/b/c/"), "/a/b/c");
    assert_eq!(clean_key(""), "/");
    assert_eq!(clean_key("a/../../b//c/."), "/b/c");
}

/// First writer wins; `create` runs without the lock (it may use the same partition) and
/// errors are not cached.
#[test]
fn partition_get_or_create() {
    let cache = Cache::new(Options::default());
    let p = get_or_create_partition::<String, String>(
        &cache,
        "/test/p",
        OptionsPartition {
            weight: 10,
            clear_when: ClearWhen::OnRebuild,
        },
    );
    let v = p
        .get_or_create("a".into(), |k| {
            // Re-entrant use of the same partition with another key.
            let inner = p.get_or_create("b".into(), |_| Ok("inner".to_string()))?;
            // A racing writer stores "a" first.
            p.set(k.clone(), "first".into());
            Ok(format!("computed-{inner}"))
        })
        .unwrap();
    assert_eq!(v, "first");
    assert_eq!(p.get(&"b".to_string()), Some("inner".into()));
    let e = p.get_or_create("c".into(), |_| Err(Error::new("boom")));
    assert!(e.is_err());
    assert_eq!(p.get(&"c".to_string()), None);
    assert_eq!(
        p.get_or_create("c".into(), |_| Ok("ok".into())).unwrap(),
        "ok"
    );
    assert_eq!(
        p.get_or_create_with_timeout("d".into(), go_time::Duration(1), |_| Ok("d".into()))
            .unwrap(),
        "d"
    );
    let mut keys = p.keys();
    keys.sort();
    assert_eq!(keys, ["a", "b", "c", "d"]);
}

#[test]
fn maps_cache() {
    let c: MapsCache<String, i64> = MapsCache::new();
    assert_eq!(c.get_or_create("a".into(), || Ok(1)).unwrap(), 1);
    assert_eq!(c.get_or_create("a".into(), || Ok(2)).unwrap(), 1);
    assert!(
        c.get_or_create("b".into(), || Err(Error::new("x")))
            .is_err()
    );
    assert!(!c.contains(&"b".to_string()));
    c.set_if_absent("a".into(), 3);
    assert_eq!(c.get(&"a".to_string()), Some(1));
    let v = c
        .init_and_get(&"z".to_string(), |m| {
            m.insert("z".into(), 26);
            Ok(())
        })
        .unwrap();
    assert_eq!(v, Some(26));
    // Initialised once.
    let v = c
        .init_and_get(&"y".to_string(), |m| {
            m.insert("y".into(), 25);
            Ok(())
        })
        .unwrap();
    assert_eq!(v, None);
    assert_eq!(c.drain().len(), 2);
    assert_eq!(c.len(), 0);
}

// Go: common/herrors/errors_test.go:TestIsFeatureNotAvailableError
#[test]
fn test_is_feature_not_available_error() {
    assert!(herrors::is_feature_not_available(
        &herrors::err_feature_not_available()
    ));
    assert!(!herrors::is_feature_not_available(&Error::new("asdf")));
    let e: Error = std::io::Error::from(std::io::ErrorKind::NotFound).into();
    assert!(herrors::is_not_exist(&e));
    assert!(!herrors::is_not_exist(&Error::new("foo")));
    assert!(herrors::is_not_exist(&e.clone().wrap("foo")));
    let t = Error::timeout(go_time::Duration(30_000_000_000));
    assert_eq!(t.message(), "timeout after 30s");
    assert!(herrors::is_timeout_error(&t));
    assert_eq!(t.kind(), ErrorKind::Timeout);
}

// Go: common/herrors/file_error_test.go:TestNewFileError (the message part)
#[test]
fn test_new_file_error() {
    let fe = herrors::new_file_error_from_name(Error::new("bar"), "foo.html");
    assert_eq!(fe.to_string(), r#""foo.html:1:1": bar"#);
}

// Go: common/herrors/file_error_test.go:TestNewFileErrorExtractFromMessage
#[test]
fn test_new_file_error_extract_from_message() {
    for (msg, line, col) in [
        ("no line number for you", 1, 1),
        (
            r#"template: _default/single.html:4:15: executing "_default/single.html" at <.Titles>: can't evaluate field Titles in type *hugolib.PageOutput"#,
            4,
            15,
        ),
        (
            "parse failed: template: _default/bundle-resource-meta.html:11: unexpected in operand",
            11,
            1,
        ),
        (
            r#"failed:: template: _default/bundle-resource-meta.html:2:7: executing "main" at <.Titles>"#,
            2,
            7,
        ),
        (
            r#"failed to load translations: (6, 7): was expecting token =, but got "g" instead"#,
            6,
            7,
        ),
        (
            r#"execute of template failed: template: index.html:2:5: executing "index.html" at <partial "foo.html" .>: error calling partial: "/layouts/partials/foo.html:3:6": execute of template failed: template: partials/foo.html:3:6: executing "partials/foo.html" at <.ThisDoesNotExist>: can't evaluate field ThisDoesNotExist in type *hugolib.pageStat"#,
            2,
            5,
        ),
        ("yaml: line 12: did not find expected key", 12, 1),
    ] {
        let fe = herrors::new_file_error_from_name(Error::new(msg), "test.txt");
        let pos = fe.pos().unwrap();
        assert_eq!((pos.line, pos.column), (line, col), "{msg}");
    }
}

#[test]
fn improve_render_err() {
    let e = Error::new(
        r#"template: a.html:3:4: executing "a.html" at <.Page.Parent.Title>: error calling Title: runtime error: invalid memory address or nil pointer dereference"#,
    );
    assert_eq!(
        herrors::improve_render_err(e).message(),
        r#"template: a.html:3:4: executing "a.html" – Parent is nil; wrap it in if or with: {{ with .Page.Parent }}{{ .Title }}{{ end }}"#
    );
    let e = Error::new(r#"template: x: executing "__hdeferred/abc" at <.X>: bad"#);
    assert_eq!(
        herrors::improve_render_err(e).message(),
        "template: x: executing at <.X>: bad"
    );
}

// Go: identity/identity.go:CleanString
#[test]
fn identity_clean_string() {
    assert_eq!(identity::clean_string(b"/A/b/C/"), "/a/b/c");
    assert_eq!(identity::clean_string(b""), "/.");
    assert_eq!(identity::clean_string(b"a//b/../c"), "/a/c");
    assert_eq!(
        identity::clean_string_identity(b"Foo").identifier_base(),
        "/foo"
    );
    let inc = IncrementByOne::default();
    assert_eq!((inc.incr(), inc.incr()), (1, 2));
}

/// `--clock`: the system clock shifted to start at t, in the Local location.
#[test]
fn start_clock() {
    let t = go_time::date(2026, go_time::Month(9), 27, 12, 0, 0, 0, &go_time::utc());
    let c = StartClock::new(&t);
    let now = c.now();
    let d = now.sub(&t);
    assert!(d.0 >= 0 && d.0 < 60_000_000_000, "{d:?}");
    assert!(go_time::is_local_loc(&now.go_location()));
    assert_eq!(now.year(), 2026);
    // The process clock defaults to the system clock.
    let sys = htime::now();
    assert!(sys.year() >= 2024);
}

/// The reflect type helpers behind Append/Slice.
#[test]
fn reflect_types() {
    hreflect::register_interface("test.Iface", |t| t == "*test.impl" || t == "test.Iface");
    assert!(hreflect::type_assignable_to("*test.impl", "test.Iface"));
    assert!(hreflect::type_assignable_to("string", "interface {}"));
    assert!(!hreflect::type_assignable_to("template.HTML", "string"));
    assert!(hreflect::type_assignable_to(
        "maps.Params",
        "map[string]interface {}"
    ));
    assert!(hreflect::type_assignable_to(
        "map[string]interface {}",
        "maps.Params"
    ));
    assert!(!hreflect::type_assignable_to("maps.Params", "page.Data"));
    assert!(hreflect::type_assignable_to("[]page.Page", "page.Pages"));
    assert!(!hreflect::type_assignable_to(
        "[]interface {}",
        "page.Pages"
    ));
    assert_eq!(
        hreflect::elem_type("page.Pages").as_deref(),
        Some("page.Page")
    );
    assert_eq!(
        hreflect::elem_type("[][]string").as_deref(),
        Some("[]string")
    );
    assert_eq!(
        hreflect::elem_type("map[string]int").as_deref(),
        Some("int")
    );
    assert_eq!(hreflect::slice_of("string"), SliceType::String);
    assert_eq!(
        hreflect::slice_of("*hugolib.pageState"),
        SliceType::Named(Arc::from("[]*hugolib.pageState"))
    );
    // A Params stored into a []map[string]interface {} slot becomes a map[string]interface {}.
    let pv = Value::map(Map::new(MapType::Params));
    let stored = hreflect::assign_to(pv, "map[string]interface {}");
    assert_eq!(stored.go_type_name(), "map[string]interface {}");
    assert_eq!(hreflect::type_of(&Value::Invalid), None);
    assert_eq!(
        hreflect::type_of(&Value::TypedNil(Arc::from("error"))),
        None
    );
    assert_eq!(
        hreflect::type_of(&Value::TypedNil(Arc::from("[]string"))).as_deref(),
        Some("[]string")
    );
    // AsTime: time.Time as it is; AsTimeProvider objects through their AsTime method.
    let loc = go_time::utc();
    let t = Value::Time(go_time::unix(0, 0));
    assert!(hreflect::as_time(&t, &loc).is_some());
    assert!(hreflect::as_time(&Value::string("2020-01-01"), &loc).is_none());
    let _ = GoString::empty();
}
