//! Ports of the Go test tables of common/maps (maps, params, scratch, ordered),
//! common/collections (append, slice, stack), common/types, common/predicate and
//! common/types/hstring. (common/math, common/hashing and compare are in math.rs, hashing.rs and
//! compare.rs.)

mod support;

use std::any::Any;
use std::borrow::Cow;
use std::sync::Arc;

use go_value::{
    GoString, HostCtx, IntKind, List, Map, MapType, Object, SafeKind, SliceType, Value,
};
use nh_common::collections::append::append;
use nh_common::collections::slice::{SortedStringSlice, slice, string_slice_to_interface_slice};
use nh_common::collections::stack::Stack;
use nh_common::maps::maps as m;
use nh_common::maps::ordered::Ordered;
use nh_common::maps::params::{self as p, ParamsMergeStrategy};
use nh_common::maps::scratch::Scratch;
use nh_common::predicate;
use nh_common::types::types::{EvictingQueue, LowHigh, new_key_values_strings};
use support::*;

fn s(x: &str) -> Value {
    Value::string(x)
}

fn strs(items: &[&str]) -> Value {
    Value::string_list(items.iter().copied())
}

fn any(items: Vec<Value>) -> Value {
    Value::any_list(items)
}

fn ints(items: &[i64]) -> Value {
    Value::list(
        SliceType::Int,
        items.iter().map(|&i| Value::int(i)).collect(),
    )
}

fn map_of(ty: MapType, entries: Vec<(&str, Value)>) -> Map {
    let mut mm = Map::new(ty);
    for (k, v) in entries {
        mm.insert(k, v);
    }
    mm
}

fn params(entries: Vec<(&str, Value)>) -> Value {
    Value::map(map_of(MapType::Params, entries))
}

fn string_any(entries: Vec<(&str, Value)>) -> Value {
    Value::map(map_of(MapType::StringAny, entries))
}

// ---------------------------------------------------------------------------
// common/maps/maps_test.go

// Go: common/maps/maps_test.go:TestPrepareParams
#[test]
fn test_prepare_params() {
    // Go's map[any]any case ("deF") cannot occur in the value model: YAML maps are
    // map[string]any after metadecoders; the rest of the table is ported.
    let input = map_of(
        MapType::Params,
        vec![
            ("abC", Value::int(32)),
            ("gHi", string_any(vec![("J", Value::int(25))])),
            (
                "jKl",
                Value::map(map_of(MapType::StringString, vec![("M", s("26"))])),
            ),
            (
                "deF",
                string_any(vec![
                    ("23", s("A value")),
                    (
                        "24",
                        string_any(vec![("AbCDe", s("A value")), ("eFgHi", s("Another value"))]),
                    ),
                ]),
            ),
        ],
    );
    let expected = map_of(
        MapType::Params,
        vec![
            ("abc", Value::int(32)),
            (
                "def",
                params(vec![
                    ("23", s("A value")),
                    (
                        "24",
                        params(vec![("abcde", s("A value")), ("efghi", s("Another value"))]),
                    ),
                ]),
            ),
            ("ghi", params(vec![("j", Value::int(25))])),
            ("jkl", params(vec![("m", s("26"))])),
        ],
    );
    let clone = p::prepare_params_clone(&input);
    let mut prepared = input.clone();
    p::prepare_params(&mut prepared);
    assert_eq!(prepared, expected);
    assert_eq!(clone, expected);
}

// Go: common/maps/maps_test.go:TestToSliceStringMap
#[test]
fn test_to_slice_string_map() {
    let one = map_of(MapType::StringAny, vec![("abc", Value::int(123))]);
    let v = Value::list(SliceType::MapStringAny, vec![Value::map(one.clone())]);
    assert_eq!(m::to_slice_string_map(&v).unwrap(), vec![one]);
    let two = map_of(MapType::StringAny, vec![("def", Value::int(456))]);
    let v = any(vec![Value::map(two.clone())]);
    assert_eq!(m::to_slice_string_map(&v).unwrap(), vec![two]);
}

// Go: common/maps/maps_test.go:TestToParamsAndPrepare
#[test]
fn test_to_params_and_prepare() {
    assert!(p::to_params_and_prepare(&string_any(vec![("A", s("av"))])).is_ok());
    let params = p::to_params_and_prepare(&Value::Invalid).unwrap();
    assert_eq!(params, Map::new(MapType::Params));
}

// Go: common/maps/maps_test.go:TestRenameKeys
#[test]
fn test_rename_keys() {
    let mut mm = map_of(
        MapType::StringAny,
        vec![
            ("a", Value::int(32)),
            ("ren1", s("m1")),
            ("ren2", s("m1_2")),
            (
                "sub",
                string_any(vec![(
                    "subsub",
                    string_any(vec![("REN1", s("m2")), ("ren2", s("m2_2"))]),
                )]),
            ),
            (
                "no",
                string_any(vec![("ren1", s("m2")), ("ren2", s("m2_2"))]),
            ),
        ],
    );
    let expected = map_of(
        MapType::StringAny,
        vec![
            ("a", Value::int(32)),
            ("new1", s("m1")),
            ("new2", s("m1_2")),
            (
                "sub",
                string_any(vec![(
                    "subsub",
                    string_any(vec![("new1", s("m2")), ("ren2", s("m2_2"))]),
                )]),
            ),
            (
                "no",
                string_any(vec![("ren1", s("m2")), ("ren2", s("m2_2"))]),
            ),
        ],
    );
    let renamer =
        m::KeyRenamer::new(&["{ren1,sub/*/ren1}", "new1", "{Ren2,sub/ren2}", "new2"]).unwrap();
    renamer.rename(&mut mm);
    assert_eq!(mm, expected);
}

// Go: common/maps/maps_test.go:TestLookupEqualFold
#[test]
fn test_lookup_equal_fold() {
    let m1 = map_of(MapType::StringAny, vec![("a", s("av")), ("B", s("bv"))]);
    let (v, k) = m::lookup_equal_fold(&m1, b"b").unwrap();
    assert_eq!((v, k.as_bytes()), (&s("bv"), &b"B"[..]));
    let m2 = map_of(MapType::StringString, vec![("a", s("av")), ("B", s("bv"))]);
    let (v, k) = m::lookup_equal_fold(&m2, b"b").unwrap();
    assert_eq!((v, k.as_bytes()), (&s("bv"), &b"B"[..]));
}

// ---------------------------------------------------------------------------
// common/maps/params_test.go

// Go: common/maps/params_test.go:TestGetNestedParam
#[test]
fn test_get_nested_param() {
    let mm = map_of(
        MapType::StringAny,
        vec![
            ("string", s("value")),
            ("first", Value::int(1)),
            ("with_underscore", Value::int(2)),
            (
                "nested",
                string_any(vec![
                    ("color", s("blue")),
                    ("nestednested", string_any(vec![("color", s("green"))])),
                ]),
            ),
        ],
    );
    let must =
        |key: &str, sep: &str| p::get_nested_param(key.as_bytes(), sep.as_bytes(), &[&mm]).unwrap();
    assert_eq!(must("first", "_"), Value::int(1));
    assert_eq!(must("First", "_"), Value::int(1));
    assert_eq!(must("with_underscore", "_"), Value::int(2));
    assert_eq!(must("nested_color", "_"), s("blue"));
    assert_eq!(must("nested.nestednested.color", "."), s("green"));
    assert_eq!(must("string.name", "."), Value::Invalid);
    assert_eq!(must("nested.foo", "."), Value::Invalid);
}

// Go: common/maps/params_test.go:TestGetNestedParamFnNestedNewKey
#[test]
fn test_get_nested_param_fn_nested_new_key() {
    let nested = map_of(MapType::StringAny, vec![("color", s("blue"))]);
    let mm = map_of(
        MapType::StringAny,
        vec![("nested", Value::map(nested.clone()))],
    );
    let (existing, nested_key, owner) = p::get_nested_param_fn(b"nested.new", b".", |k| {
        mm.get(k).cloned().unwrap_or(Value::Invalid)
    })
    .unwrap();
    assert_eq!(existing, Value::Invalid);
    assert_eq!(nested_key.as_bytes(), b"new");
    assert_eq!(*owner.unwrap(), nested);
}

/// Maps with merge-strategy objects compare by their encoding (objects compare by identity).
fn enc(m: &Map) -> serde_json::Value {
    encode(&Value::map(m.clone()))
}

fn params_pair() -> (Map, Map) {
    let p1 = map_of(
        MapType::Params,
        vec![
            ("a", s("av")),
            ("c", s("cv")),
            (
                "nested",
                params(vec![("al2", s("al2v")), ("cl2", s("cl2v"))]),
            ),
        ],
    );
    let p2 = map_of(
        MapType::Params,
        vec![
            ("b", s("bv")),
            ("a", s("abv")),
            (
                "nested",
                params(vec![("bl2", s("bl2v")), ("al2", s("al2bv"))]),
            ),
            (p::MERGE_STRATEGY_KEY, ParamsMergeStrategy::Deep.value()),
        ],
    );
    (p1, p2)
}

// Go: common/maps/params_test.go:TestParamsSetAndMerge
#[test]
fn test_params_set_and_merge() {
    let (mut p1, p2) = params_pair();
    p::set_params(&mut p1, &p2);
    assert_eq!(
        enc(&p1),
        enc(&map_of(
            MapType::Params,
            vec![
                ("a", s("abv")),
                ("c", s("cv")),
                (
                    "nested",
                    params(vec![
                        ("al2", s("al2bv")),
                        ("cl2", s("cl2v")),
                        ("bl2", s("bl2v"))
                    ])
                ),
                ("b", s("bv")),
                (p::MERGE_STRATEGY_KEY, ParamsMergeStrategy::Deep.value()),
            ]
        ))
    );

    let (mut p1, p2) = params_pair();
    p::merge_params_with_strategy("", &mut p1, &p2);
    // Default is to do a shallow merge.
    assert_eq!(
        enc(&p1),
        enc(&map_of(
            MapType::Params,
            vec![
                ("c", s("cv")),
                (
                    "nested",
                    params(vec![("al2", s("al2v")), ("cl2", s("cl2v"))])
                ),
                ("b", s("bv")),
                ("a", s("av")),
            ]
        ))
    );

    let (mut p1, p2) = params_pair();
    p::set_merge_strategy(&mut p1, ParamsMergeStrategy::None);
    p::merge_params_with_strategy("", &mut p1, &p2);
    p::delete_merge_strategy(&mut p1);
    assert_eq!(
        enc(&p1),
        enc(&map_of(
            MapType::Params,
            vec![
                ("a", s("av")),
                ("c", s("cv")),
                (
                    "nested",
                    params(vec![("al2", s("al2v")), ("cl2", s("cl2v"))])
                ),
            ]
        ))
    );

    let (mut p1, p2) = params_pair();
    p::set_merge_strategy(&mut p1, ParamsMergeStrategy::Shallow);
    p::merge_params_with_strategy("", &mut p1, &p2);
    p::delete_merge_strategy(&mut p1);
    assert_eq!(
        enc(&p1),
        enc(&map_of(
            MapType::Params,
            vec![
                ("a", s("av")),
                ("c", s("cv")),
                (
                    "nested",
                    params(vec![("al2", s("al2v")), ("cl2", s("cl2v"))])
                ),
                ("b", s("bv")),
            ]
        ))
    );

    let (mut p1, p2) = params_pair();
    p::set_merge_strategy(&mut p1, ParamsMergeStrategy::Deep);
    p::merge_params_with_strategy("", &mut p1, &p2);
    p::delete_merge_strategy(&mut p1);
    assert_eq!(
        enc(&p1),
        enc(&map_of(
            MapType::Params,
            vec![
                (
                    "nested",
                    params(vec![
                        ("al2", s("al2v")),
                        ("cl2", s("cl2v")),
                        ("bl2", s("bl2v"))
                    ])
                ),
                ("b", s("bv")),
                ("a", s("av")),
                ("c", s("cv")),
            ]
        ))
    );
}

// Go: common/maps/params_test.go:TestParamsIsZero
#[test]
fn test_params_is_zero() {
    assert!(p::params_is_zero(&Map::new(MapType::Params)));
    assert!(p::params_is_zero(&map_of(MapType::Params, vec![])));
    assert!(!p::params_is_zero(&map_of(
        MapType::Params,
        vec![("foo", s("bar"))]
    )));
    assert!(!p::params_is_zero(&map_of(
        MapType::Params,
        vec![("_merge", s("foo")), ("foo", s("bar"))]
    )));
    assert!(p::params_is_zero(&map_of(
        MapType::Params,
        vec![("_merge", s("foo"))]
    )));
    // A nil Params is zero through the template truth test too.
    assert!(!nh_common::hreflect::is_truthful(&Value::TypedNil(
        Arc::from("maps.Params")
    )));
}

/// The `maps.Params` methods as the named-type registry dispatches them.
#[test]
fn params_methods() {
    let mut reg = nh_common::object::NamedTypeRegistry::new();
    reg.register("maps.Params", p::PARAMS_METHODS);
    let v = params(vec![
        ("_merge", ParamsMergeStrategy::None.value()),
        ("a", params(vec![("b", s("x"))])),
    ]);
    assert!(reg.has_method(&v, "IsZero"));
    assert!(reg.has_method(&v, "GetNested"));
    assert!(!reg.has_method(&v, "isZero"));
    assert!(!reg.has_method(&v, "a"));
    assert_eq!(
        reg.call(&(), &v, "IsZero", &[]).unwrap().unwrap(),
        Value::Bool(false)
    );
    assert_eq!(
        reg.call(&(), &v, "GetNested", &[s("A"), s("B")])
            .unwrap()
            .unwrap(),
        s("x")
    );
    assert_eq!(
        reg.call(&(), &v, "GetNested", &[s("missing")])
            .unwrap()
            .unwrap(),
        Value::Invalid
    );
    assert_eq!(
        reg.call(&(), &v, "DeleteMergeStrategy", &[])
            .unwrap()
            .unwrap(),
        Value::Bool(true)
    );
    let e = reg
        .call(&(), &v, "GetMergeStrategy", &[])
        .unwrap()
        .unwrap_err();
    assert_eq!(
        e.message(),
        "can't call method/function \"GetMergeStrategy\" with 2 results"
    );
    let e = reg
        .call(&(), &v, "SetMergeStrategy", &[s("deep")])
        .unwrap()
        .unwrap_err();
    assert_eq!(
        e.message(),
        "can't call method/function \"SetMergeStrategy\" with 0 results"
    );
    // Case-insensitive key lookup with nil values reported as missing.
    let pm = map_of(
        MapType::Params,
        vec![("title", s("T")), ("nil", Value::Invalid)],
    );
    assert_eq!(p::params_get(&pm, b"TITLE"), s("T"));
    assert_eq!(p::params_get(&pm, b"Nil"), Value::Invalid);
    assert_eq!(p::params_get(&pm, b"missing"), Value::Invalid);
    // The merge strategy prints like Go's named string.
    assert_eq!(
        go_fmt::sprintf(
            "%v|%s|%q|%T",
            &[
                ParamsMergeStrategy::Deep.value(),
                ParamsMergeStrategy::Deep.value(),
                ParamsMergeStrategy::Deep.value(),
                ParamsMergeStrategy::Deep.value()
            ]
        ),
        b"deep|deep|\"deep\"|maps.ParamsMergeStrategy".to_vec()
    );
}

// ---------------------------------------------------------------------------
// common/maps/scratch_test.go

// Go: common/maps/scratch_test.go:TestScratchAdd
#[test]
fn test_scratch_add() {
    let scratch = Arc::new(Scratch::new());
    let key = |k: &str| GoString::from(k);
    scratch.add(&(), &key("int1"), Value::int(10)).unwrap();
    scratch.add(&(), &key("int1"), Value::int(20)).unwrap();
    scratch.add(&(), &key("int2"), Value::int(20)).unwrap();
    assert_eq!(scratch.get(&key("int1")), Value::int64(30));
    assert_eq!(scratch.get(&key("int2")), Value::int(20));

    scratch
        .add(&(), &key("float1"), Value::float64(10.5))
        .unwrap();
    scratch
        .add(&(), &key("float1"), Value::float64(20.1))
        .unwrap();
    assert_eq!(scratch.get(&key("float1")), Value::float64(30.6));

    scratch.add(&(), &key("string1"), s("Hello ")).unwrap();
    scratch.add(&(), &key("string1"), s("big ")).unwrap();
    scratch.add(&(), &key("string1"), s("World!")).unwrap();
    assert_eq!(scratch.get(&key("string1")), s("Hello big World!"));

    scratch
        .add(&(), &key("scratch"), Value::Object(scratch.clone()))
        .unwrap();
    assert!(
        scratch
            .add(&(), &key("scratch"), Value::Object(scratch.clone()))
            .is_err()
    );
    assert_eq!(scratch.values().as_map().unwrap().len(), 5);
    // Break the reference cycle.
    scratch.delete(&key("scratch"));
}

// Go: common/maps/scratch_test.go:TestScratchAddSlice
#[test]
fn test_scratch_add_slice() {
    let scratch = Scratch::new();
    let k = GoString::from("intSlice");
    scratch.add(&(), &k, ints(&[1, 2])).unwrap();
    scratch.add(&(), &k, Value::int(3)).unwrap();
    assert_eq!(scratch.get(&k), ints(&[1, 2, 3]));
    scratch.add(&(), &k, ints(&[4, 5])).unwrap();
    assert_eq!(scratch.get(&k), ints(&[1, 2, 3, 4, 5]));
}

// Go: common/maps/scratch_test.go:TestScratchAddTypedSliceToInterfaceSlice
#[test]
fn test_scratch_add_typed_slice_to_interface_slice() {
    let scratch = Scratch::new();
    let k = GoString::from("slice");
    scratch.set(&k, any(vec![]));
    scratch.add(&(), &k, ints(&[1, 2])).unwrap();
    assert_eq!(scratch.get(&k), ints(&[1, 2]));
}

// Go: common/maps/scratch_test.go:TestScratchAddDifferentTypedSliceToInterfaceSlice
#[test]
fn test_scratch_add_different_typed_slice_to_interface_slice() {
    let scratch = Scratch::new();
    let k = GoString::from("slice");
    scratch.set(&k, strs(&["foo"]));
    scratch.add(&(), &k, ints(&[1, 2])).unwrap();
    assert_eq!(
        scratch.get(&k),
        any(vec![s("foo"), Value::int(1), Value::int(2)])
    );
}

// Go: common/maps/scratch_test.go:TestScratchSet
#[test]
fn test_scratch_set() {
    let scratch = Scratch::new();
    scratch.set(&GoString::from("key"), s("val"));
    assert_eq!(scratch.get(&GoString::from("key")), s("val"));
}

// Go: common/maps/scratch_test.go:TestScratchDelete
#[test]
fn test_scratch_delete() {
    let scratch = Scratch::new();
    let k = GoString::from("key");
    scratch.set(&k, s("val"));
    scratch.delete(&k);
    scratch.add(&(), &k, s("Lucy Parsons")).unwrap();
    assert_eq!(scratch.get(&k), s("Lucy Parsons"));
}

// Go: common/maps/scratch_test.go:TestScratchInParallel
#[test]
fn test_scratch_in_parallel() {
    let scratch = Arc::new(Scratch::new());
    let key = GoString::from("counter");
    scratch.set(&key, Value::int64(1));
    let handles: Vec<_> = (1..=10)
        .map(|j| {
            let scratch = scratch.clone();
            let key = key.clone();
            std::thread::spawn(move || {
                for k in 0..10 {
                    let new_val = Value::int64(k + j);
                    scratch.add(&(), &key, new_val.clone()).unwrap();
                    scratch.set(&key, new_val);
                    match scratch.get(&key) {
                        Value::Int(counter, IntKind::Int64) => assert!(counter >= 1),
                        other => panic!("got {other:?}"),
                    }
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
}

// Go: common/maps/scratch_test.go:TestScratchGet
#[test]
fn test_scratch_get() {
    assert_eq!(
        Scratch::new().get(&GoString::from("nothing")),
        Value::Invalid
    );
}

// Go: common/maps/scratch_test.go:TestScratchSetInMap
#[test]
fn test_scratch_set_in_map() {
    let scratch = Scratch::new();
    let k = GoString::from("key");
    for (mk, v) in [
        ("lux", "Lux"),
        ("abc", "Abc"),
        ("zyx", "Zyx"),
        ("abc", "Abc (updated)"),
        ("def", "Def"),
    ] {
        scratch.set_in_map(&k, &GoString::from(mk), s(v)).unwrap();
    }
    assert_eq!(
        scratch.get_sorted_map_values(&k).unwrap(),
        any(vec![s("Abc (updated)"), s("Def"), s("Lux"), s("Zyx")])
    );
}

// Go: common/maps/scratch_test.go:TestScratchDeleteInMap
#[test]
fn test_scratch_delete_in_map() {
    let scratch = Scratch::new();
    let k = GoString::from("key");
    let g = GoString::from;
    scratch.set_in_map(&k, &g("lux"), s("Lux")).unwrap();
    scratch.set_in_map(&k, &g("abc"), s("Abc")).unwrap();
    scratch.set_in_map(&k, &g("zyx"), s("Zyx")).unwrap();
    scratch.delete_in_map(&k, &g("abc")).unwrap();
    scratch.set_in_map(&k, &g("def"), s("Def")).unwrap();
    scratch.delete_in_map(&k, &g("lmn")).unwrap(); // Do nothing
    assert_eq!(
        scratch.get_sorted_map_values(&k).unwrap(),
        any(vec![s("Def"), s("Lux"), s("Zyx")])
    );
}

// Go: common/maps/scratch_test.go:TestScratchGetSortedMapValues
#[test]
fn test_scratch_get_sorted_map_values() {
    assert_eq!(
        Scratch::new()
            .get_sorted_map_values(&GoString::from("nothing"))
            .unwrap(),
        Value::Invalid
    );
}

/// The Scratch method table as templates see it.
#[test]
fn scratch_methods() {
    let v = Value::object(Scratch::new());
    let o = v.as_object().unwrap();
    assert_eq!(o.type_name(), "*maps.Scratch");
    for name in [
        "Add",
        "Set",
        "Delete",
        "Get",
        "Values",
        "SetInMap",
        "DeleteInMap",
        "GetSortedMapValues",
    ] {
        assert!(o.has_method(name), "{name}");
    }
    assert_eq!(Scratch::GO_METHODS.len(), 8);
    assert!(!o.has_method("get"));
    let ret = o
        .call_method(&(), "Set", &[s("k"), Value::int(1)])
        .unwrap()
        .unwrap();
    assert_eq!(ret, s(""));
    let e = o.call_method(&(), "Set", &[s("k")]).unwrap().unwrap_err();
    assert_eq!(e.message(), "wrong number of args for Set: want 2 got 1");
    let e = o
        .call_method(&(), "Get", &[Value::html("k")])
        .unwrap()
        .unwrap_err();
    assert_eq!(
        e.message(),
        "wrong type for value; expected string; got template.HTML"
    );
    assert_eq!(
        o.call_method(&(), "Get", &[s("k")]).unwrap().unwrap(),
        Value::int(1)
    );
    // Add of a number to an existing int gives an int64.
    o.call_method(&(), "Add", &[s("k"), Value::int(2)])
        .unwrap()
        .unwrap();
    assert_eq!(
        o.call_method(&(), "Get", &[s("k")]).unwrap().unwrap(),
        Value::int64(3)
    );
}

// ---------------------------------------------------------------------------
// common/maps/ordered_test.go

// Go: common/maps/ordered_test.go:TestOrdered
#[test]
fn test_ordered() {
    let mut mm: Ordered<String, i64> = Ordered::new();
    mm.set("a".into(), 1);
    mm.set("b".into(), 2);
    mm.set("c".into(), 3);
    assert_eq!(mm.keys(), ["a", "b", "c"]);
    assert_eq!(mm.values(), [1, 2, 3]);
    assert_eq!(mm.get(&"b".to_string()), Some(&2));
    mm.set("b".into(), 22);
    assert_eq!(mm.keys(), ["a", "b", "c"]);
    assert_eq!(mm.values(), [1, 22, 3]);
    mm.delete(&"b".to_string());
    assert_eq!(mm.keys(), ["a", "c"]);
    assert_eq!(mm.values(), [1, 3]);
}

// Go: common/maps/ordered_test.go:TestOrderedHash
#[test]
fn test_ordered_hash() {
    use go_hashstructure::HashValue as H;
    let h =
        |mm: &Ordered<String, i64>| mm.hash(|k, v| (H::string(k.as_str()), H::int(*v))).unwrap();
    let mut mm: Ordered<String, i64> = Ordered::new();
    mm.set("a".into(), 1);
    mm.set("b".into(), 2);
    mm.set("c".into(), 3);
    let h1 = h(&mm);
    mm.set("d".into(), 4);
    let h2 = h(&mm);
    assert_ne!(h1, h2);
    let mut mm: Ordered<String, i64> = Ordered::new();
    mm.set("b".into(), 2);
    mm.set("a".into(), 1);
    mm.set("c".into(), 3);
    // Order does not matter.
    assert_eq!(h(&mm), h1);
}

// ---------------------------------------------------------------------------
// common/collections/append_test.go

fn slicers(ty: &str, ids: &[&str]) -> Value {
    Value::List(Arc::new(List::new(
        SliceType::Named(Arc::from(ty)),
        ids.iter()
            .map(|id| Value::object(Pg(id.to_string())))
            .collect(),
    )))
}

fn pg(id: &str) -> Value {
    Value::object(Pg(id.to_string()))
}

// Go: common/collections/append_test.go:TestAppend
#[test]
fn test_append() {
    init();
    let html = |x: &str| Value::Safe(SafeKind::Html, GoString::from(x));
    // (start, addend, expected; None = error). Go's tstSlicers/*tstSlicer are the oracle's
    // main.pgs/*main.pg here; the &tstSlicers (pointer to slice) case has no value-model
    // equivalent.
    let table: Vec<(Value, Vec<Value>, Option<Value>)> = vec![
        (
            strs(&["a", "b"]),
            vec![s("c")],
            Some(strs(&["a", "b", "c"])),
        ),
        (
            strs(&["a", "b"]),
            vec![s("c"), s("d"), s("e")],
            Some(strs(&["a", "b", "c", "d", "e"])),
        ),
        (
            strs(&["a", "b"]),
            vec![strs(&["c", "d", "e"])],
            Some(strs(&["a", "b", "c", "d", "e"])),
        ),
        (
            strs(&["a"]),
            vec![s("b"), html("c")],
            Some(any(vec![s("a"), s("b"), html("c")])),
        ),
        (
            Value::Invalid,
            vec![s("a"), s("b")],
            Some(strs(&["a", "b"])),
        ),
        (
            Value::Invalid,
            vec![Value::Invalid],
            Some(any(vec![Value::Invalid])),
        ),
        (
            any(vec![]),
            vec![strs(&["c", "d", "e"])],
            Some(strs(&["c", "d", "e"])),
        ),
        (
            slicers("main.pgs", &["a", "b"]),
            vec![pg("c")],
            Some(slicers("main.pgs", &["a", "b", "c"])),
        ),
        (
            strs(&["a", "b"]),
            vec![slicers("main.pgs", &["a", "b"])],
            Some(any(vec![s("a"), s("b"), pg("a"), pg("b")])),
        ),
        (
            strs(&["a", "b"]),
            vec![pg("a")],
            Some(any(vec![s("a"), s("b"), pg("a")])),
        ),
        // Errors
        (s(""), vec![strs(&["a", "b"])], None),
        // No string concatenation.
        (s("ab"), vec![s("c")], None),
        (
            strs(&["a", "b"]),
            vec![Value::Invalid],
            Some(any(vec![s("a"), s("b"), Value::Invalid])),
        ),
        (
            strs(&["a", "b"]),
            vec![Value::Invalid, s("d"), Value::Invalid],
            Some(any(vec![
                s("a"),
                s("b"),
                Value::Invalid,
                s("d"),
                Value::Invalid,
            ])),
        ),
        (
            any(vec![s("a"), Value::Invalid, s("c")]),
            vec![s("d"), Value::Invalid, s("f")],
            Some(any(vec![
                s("a"),
                Value::Invalid,
                s("c"),
                s("d"),
                Value::Invalid,
                s("f"),
            ])),
        ),
        (strs(&["a", "b"]), vec![], Some(strs(&["a", "b"]))),
    ];
    for (i, (start, addend, expected)) in table.into_iter().enumerate() {
        let got = append(&(), &start, &addend);
        match expected {
            None => assert!(got.is_err(), "[{i}] {got:?}"),
            Some(e) => {
                let got = got.unwrap();
                assert_eq!(encode(&got), encode(&e), "[{i}]");
            }
        }
    }
}

// Go: common/collections/append_test.go:TestAppendToMultiDimensionalSlice
#[test]
fn test_append_to_multi_dimensional_slice() {
    let ss = |rows: &[&[&str]]| {
        Value::list(
            SliceType::Named(Arc::from("[][]string")),
            rows.iter().map(|r| strs(r)).collect(),
        )
    };
    let got = append(&(), &ss(&[&["a", "b"]]), &[strs(&["c", "d"])]).unwrap();
    assert_eq!(got, ss(&[&["a", "b"], &["c", "d"]]));
    let got = append(
        &(),
        &ss(&[&["a", "b"]]),
        &[strs(&["c", "d"]), strs(&["e", "f"])],
    )
    .unwrap();
    assert_eq!(got, ss(&[&["a", "b"], &["c", "d"], &["e", "f"]]));
    let e = append(&(), &ss(&[&["a", "b"]]), &[ints(&[1, 2])]).unwrap_err();
    assert_eq!(e.message(), "cannot append slice of int to slice of string");
}

// Go: common/collections/append_test.go:TestAppendShouldMakeACopyOfTheInputSlice
#[test]
fn test_append_should_make_a_copy_of_the_input_slice() {
    let slice = strs(&["a", "b"]);
    let result = append(&(), &slice, &[s("c")]).unwrap();
    assert_eq!(result, strs(&["a", "b", "c"]));
    assert_eq!(slice, strs(&["a", "b"]));
}

// ---------------------------------------------------------------------------
// common/collections/slice_test.go

/// Go's tstSlicerIn1/tstSlicerIn2: Slicers whose Slice returns testSlicerInterfaces.
struct SlicerIn(&'static str, String);

impl Object for SlicerIn {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed(self.0)
    }
    fn has_method(&self, name: &str) -> bool {
        matches!(name, "Slice" | "Name")
    }
    fn call_method(
        &self,
        _: HostCtx<'_>,
        name: &str,
        args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        match name {
            "Name" => Some(Ok(Value::string(self.1.as_str()))),
            "Slice" => {
                let items = args[0].as_list().unwrap().items.clone();
                if items.iter().all(|v| v.downcast::<SlicerIn>().is_some()) {
                    Some(Ok(Value::List(Arc::new(List::new(
                        SliceType::Named(Arc::from("collections.testSlicerInterfaces")),
                        items,
                    )))))
                } else {
                    Some(Err(go_value::Error::new("invalid type")))
                }
            }
            _ => None,
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// Go: common/collections/slice_test.go:TestSlice
#[test]
fn test_slice() {
    init();
    let in1 = |n: &str| Value::object(SlicerIn("*collections.tstSlicerIn1", n.to_string()));
    let in2 = |n: &str| Value::object(SlicerIn("*collections.tstSlicerIn2", n.to_string()));
    let table: Vec<(Vec<Value>, Value)> = vec![
        (vec![s("a"), s("b")], strs(&["a", "b"])),
        (vec![pg("a"), pg("b")], slicers("main.pgs", &["a", "b"])),
        (vec![pg("a"), s("b")], any(vec![pg("a"), s("b")])),
        (vec![], any(vec![])),
        (vec![Value::Invalid], any(vec![Value::Invalid])),
        (
            vec![Value::int(5), s("b")],
            any(vec![Value::int(5), s("b")]),
        ),
    ];
    for (i, (args, expected)) in table.into_iter().enumerate() {
        assert_eq!(encode(&slice(&(), &args)), encode(&expected), "[{i}]");
    }
    let got = slice(&(), &[in1("a"), in2("b")]);
    let l = got.as_list().unwrap();
    assert_eq!(l.ty.go_name(), "collections.testSlicerInterfaces");
    assert_eq!(l.len(), 2);
    let got = slice(&(), &[in1("a"), pg("b")]);
    assert_eq!(got.as_list().unwrap().ty, SliceType::Any);
}

// Go: common/collections/slice_test.go:TestSortedStringSlice
#[test]
fn test_sorted_string_slice() {
    let ss = SortedStringSlice(
        ["a", "b", "b", "b", "c", "d"]
            .iter()
            .map(|x| GoString::from(*x))
            .collect(),
    );
    assert!(ss.contains(b"a"));
    assert!(ss.contains(b"b"));
    assert!(!ss.contains(b"z"));
    assert_eq!(ss.count(b"b"), 3);
    assert_eq!(ss.count(b"z"), 0);
    assert_eq!(ss.count(b"a"), 1);
}

// Go: common/collections/slice_test.go:TestStringSliceToInterfaceSlice
#[test]
fn test_string_slice_to_interface_slice() {
    for input in [&[][..], &["hello"][..], &["a", "b", "c"][..]] {
        let ss: Vec<GoString> = input.iter().map(|x| GoString::from(*x)).collect();
        let want: Vec<Value> = input.iter().map(|x| s(x)).collect();
        assert_eq!(string_slice_to_interface_slice(&ss), want);
    }
}

// ---------------------------------------------------------------------------
// common/collections/stack_test.go

// Go: common/collections/stack_test.go:TestStackBasic
#[test]
fn test_stack_basic() {
    let st: Stack<i64> = Stack::new();
    assert_eq!(st.len(), 0);
    st.push(1);
    st.push(2);
    st.push(3);
    assert_eq!(st.len(), 3);
    assert_eq!(st.peek(), Some(3));
    assert_eq!(st.pop(), Some(3));
    assert_eq!(st.len(), 2);
    st.pop();
    st.pop();
    assert_eq!(st.pop(), None);
}

// Go: common/collections/stack_test.go:TestStackDrain
#[test]
fn test_stack_drain() {
    let st: Stack<&str> = Stack::new();
    st.push("a");
    st.push("b");
    assert_eq!(st.drain(), ["a", "b"]);
    assert_eq!(st.len(), 0);
}

// Go: common/collections/stack_test.go:TestStackDrainMatching
#[test]
fn test_stack_drain_matching() {
    let st: Stack<i64> = Stack::new();
    for i in 1..=4 {
        st.push(i);
    }
    assert_eq!(st.drain_matching(|v| v % 2 == 0), [4, 2]);
    assert_eq!(st.drain(), [1, 3]);
}

// ---------------------------------------------------------------------------
// common/types and common/predicate

// Go: common/types/types_test.go:TestKeyValues
#[test]
fn test_key_values() {
    let kv = new_key_values_strings("key", &["a1", "a2"]);
    assert_eq!(kv.key_string(), "key");
    assert_eq!(kv.values, vec![s("a1"), s("a2")]);
    assert_eq!(kv.string(), b"key: [a1 a2]".to_vec());
}

// Go: common/types/types_test.go:TestLowHigh
#[test]
fn test_low_high() {
    let lh = LowHigh { low: 2, high: 10 };
    let src = b"abcdefghijklmnopqrstuvwxyz";
    assert!(!lh.is_zero());
    assert_eq!(lh.value(src), b"cdefghij");
}

// Go: common/types/convert_test.go:TestToStringSlicePreserveString
#[test]
fn test_to_string_slice_preserve_string() {
    use nh_common::types::convert::to_string_slice_preserve_string as f;
    let g = |x: &[&str]| x.iter().map(|y| GoString::from(*y)).collect::<Vec<_>>();
    assert_eq!(f(&s("Hugo")), g(&["Hugo"]));
    assert_eq!(f(&any(vec![s("A"), s("B")])), g(&["A", "B"]));
    assert_eq!(f(&ints(&[1, 3])), g(&["1", "3"]));
    assert_eq!(f(&Value::Invalid), g(&[]));
}

// Go: common/types/convert_test.go:TestToString
#[test]
fn test_to_string() {
    use nh_common::types::convert::to_string;
    let bytes = Value::list(
        SliceType::Uint8,
        b"Hugo"
            .iter()
            .map(|&b| Value::Uint(b as u64, go_value::UintKind::Uint8))
            .collect(),
    );
    assert_eq!(to_string(&bytes), "Hugo");
    let raw = Value::object(go_json::RawMessage(Some(b"Hugo".to_vec())));
    assert_eq!(to_string(&raw), "Hugo");
}

// Go: common/types/convert_test.go:TestToDuration
#[test]
fn test_to_duration() {
    use nh_common::types::convert::to_duration;
    assert_eq!(to_duration(&s("200ms")), go_time::Duration(200_000_000));
    assert_eq!(to_duration(&s("200")), go_time::Duration(200_000_000));
    assert_eq!(to_duration(&s("4m")), go_time::Duration(240_000_000_000));
    assert_eq!(to_duration(&s("asdfadf")), go_time::Duration(0));
}

// Go: common/types/evictingqueue_test.go:TestEvictingStringQueue
#[test]
fn test_evicting_string_queue() {
    let queue: EvictingQueue<String> = EvictingQueue::new(3);
    assert_eq!(queue.peek(), "");
    queue.add("a".into());
    queue.add("b".into());
    queue.add("a".into());
    assert_eq!(queue.peek(), "b");
    queue.add("b".into());
    assert_eq!(queue.peek(), "b");
    queue.add("a".into());
    queue.add("b".into());
    assert!(queue.contains(&"a".to_string()));
    assert!(!queue.contains(&"foo".to_string()));
    assert_eq!(queue.peek_all(), ["b", "a"]);
    assert_eq!(queue.peek(), "b");
    queue.add("c".into());
    queue.add("d".into());
    // Overflowed, a should now be removed.
    assert_eq!(queue.peek_all(), ["d", "c", "b"]);
    assert_eq!(queue.peek_all_set().len(), 3);
    assert!(queue.peek_all_set().contains("c"));
}

// Go: common/types/hstring/stringtypes_test.go:TestRenderedString
#[test]
fn test_rendered_string() {
    use nh_common::types::hstring::Html;
    let h = Html(GoString::from("Hugo"));
    // Validate that it will behave like a string in Hugo settings.
    assert_eq!(nh_common::cast::caste::to_string(&h.value()), "Hugo");
    assert_eq!(h.printable_value(), Value::html("Hugo"));
    // A named string type prints and compares through its underlying string.
    assert_eq!(
        go_fmt::sprintf(
            "%v|%s|%q|%T|%d",
            &[h.value(), h.value(), h.value(), h.value(), h.value()]
        ),
        b"Hugo|Hugo|\"Hugo\"|hstring.HTML|%!d(hstring.HTML=Hugo)".to_vec()
    );
    assert!(nh_common::hreflect::is_truthful(&h.value()));
    assert!(!nh_common::hreflect::is_truthful(
        &Html(GoString::empty()).value()
    ));
}

fn int_p1(i: &i64) -> bool {
    *i == 10 || *i == 1
}

fn int_p2(i: &i64) -> bool {
    *i == 10 || *i == 2
}

// Go: common/predicate/predicate_test.go:TestAdd
#[test]
fn test_predicate_add() {
    let p: predicate::P<i64> = predicate::new(int_p1);
    assert!(p(&1));
    assert!(!p(&2));
    let neg = predicate::negate(p.clone());
    assert!(!neg(&1));
    assert!(neg(&2));
    let and = predicate::and(Some(p.clone()), vec![predicate::new(int_p2)]);
    assert!(!and(&1));
    assert!(!and(&2));
    assert!(and(&10));
    let or = predicate::or(Some(p), vec![predicate::new(int_p2)]);
    assert!(or(&1));
    assert!(or(&2));
    assert!(or(&10));
    assert!(!or(&11));
}

// Go: common/predicate/predicate_test.go:TestFilter
#[test]
fn test_predicate_filter() {
    let p = predicate::or(Some(predicate::new(int_p1)), vec![predicate::new(int_p2)]);
    let mut v = vec![1, 2, 3, 4, 1, 6, 7, 8, 2];
    predicate::filter(&p, &mut v);
    assert_eq!(v, [1, 2, 1, 2]);
}

// Go: common/predicate/predicate_test.go:TestFilterCopy
#[test]
fn test_predicate_filter_copy() {
    let p = predicate::or(Some(predicate::new(int_p1)), vec![predicate::new(int_p2)]);
    let v = vec![1, 2, 3, 4, 1, 6, 7, 8, 2];
    assert_eq!(predicate::filter_copy(&p, &v), [1, 2, 1, 2]);
    assert_eq!(v, [1, 2, 3, 4, 1, 6, 7, 8, 2]);
}
