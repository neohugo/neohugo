//! Known vectors: hashstructure_test.go TestHash_golden (FNV), neohugo
//! hashing_test.go, and the image-processing keys of the seeksnack golden
//! build (specs/images.md §4).

use go_hashstructure::hashing::{self, hash_string, hash_string_hex, hash_uint64};
use go_hashstructure::{GoMap, GoStruct, HashValue as H, hash};
use go_value::{FloatKind, IntKind, Map, MapType, UintKind, Value};

fn fnv(v: &H) -> u64 {
    hash(v, None).unwrap()
}

// hashstructure_test.go: TestHash_golden (default FNV hasher).
#[test]
fn golden_fnv() {
    let foo = || H::ptr(H::string("foo"));
    let cases: Vec<(H, u64)> = vec![
        (H::Nil, 12161962213042174405),
        (H::string("foo"), 15621798640163566899),
        (H::int(42), 11375694726533372055),
        (H::Uint(42, UintKind::Uint8), 12638153115695167477),
        (H::Int(42, IntKind::Int16), 590708257076254031),
        (H::Int(42, IntKind::Int32), 843871326190827175),
        (H::Int(42, IntKind::Int64), 11375694726533372055),
        (H::Uint(42, UintKind::Uint16), 590708257076254031),
        (H::Uint(42, UintKind::Uint32), 843871326190827175),
        (H::Uint(42, UintKind::Uint64), 11375694726533372055),
        (H::Float(42.0, FloatKind::F32), 5558953217260120943),
        (H::Float(42.0, FloatKind::F64), 12162027084228238918),
        (H::Complex64(42.0, 0.0), 13187391128804187615),
        (H::Complex128(42.0, 0.0), 4635205179288363782),
        (H::Bool(true), 12638153115695167454),
        (H::Bool(false), 12638153115695167455),
        (H::string_slice(["foo", "bar"]), 18333885979647637445),
        (
            H::any_slice(vec![H::int(1), H::Nil, H::string("foo")]),
            636613494442026145,
        ),
        (
            H::Map(GoMap::new(vec![(H::string("foo"), H::string("bar"))])),
            5334326627423288605,
        ),
        (
            H::Map(GoMap::new(vec![(H::string("foo"), foo())])),
            4615367350888355399,
        ),
        (
            H::Map(GoMap::new(vec![(foo(), H::string("bar"))])),
            5334326627423288605,
        ),
        (
            H::Map(GoMap::new(vec![(
                H::iface(H::string("foo")),
                H::string("bar"),
            )])),
            5334326627423288605,
        ),
        (
            H::Map(GoMap::new(vec![
                (H::iface(H::string("foo")), H::iface(H::string("bar"))),
                (H::iface(H::string("bar")), H::iface(H::int(0))),
            ])),
            10207098687398820730,
        ),
        (
            H::Map(GoMap::new(vec![
                (H::iface(H::string("foo")), H::iface(H::string("bar"))),
                (
                    H::iface(H::string("bar")),
                    H::iface(H::Map(GoMap::new(vec![
                        (H::iface(H::string("foo")), H::iface(H::string("bar"))),
                        (
                            H::iface(H::string("bar")),
                            H::iface(H::Map(GoMap::new(vec![
                                (H::iface(H::string("foo")), H::iface(H::string("bar"))),
                                (
                                    H::iface(H::string("bar")),
                                    H::iface(H::Map(GoMap::new(vec![
                                        (H::iface(foo()), H::iface(H::string("bar"))),
                                        (H::iface(H::string("bar")), H::iface(H::int(0))),
                                    ]))),
                                ),
                            ]))),
                        ),
                    ]))),
                ),
            ])),
            18346441822047112296,
        ),
        (
            H::Struct(
                GoStruct::new("")
                    .field("Foo", H::string("foo"))
                    .field("Bar", H::any_slice(vec![H::Nil, H::Nil, H::Nil])),
            ),
            14887393564066082535,
        ),
    ];
    for (i, (v, want)) in cases.iter().enumerate() {
        assert_eq!(fnv(v), *want, "case {i}: {v:?}");
    }
}

// neohugo common/hashing/hashing_test.go
#[test]
fn neohugo_hashing() {
    assert_eq!(
        hashing::xxhash_from_string("Hello World"),
        7148569436472236994
    );
    let (h, n) =
        hashing::xxhash_from_reader(&mut &b"Hello World"[..], hashing::ReaderKind::WriterTo)
            .unwrap();
    assert_eq!((h, n), (7148569436472236994, 11));
    assert_eq!(
        hashing::xxhash_from_string_hex_encoded("The quick brown fox jumps over the lazy dog"),
        "0b242d361fda71bc"
    );
    assert_eq!(
        hash_string(&[H::string("a"), H::string("b")]).unwrap(),
        "3176555414984061461"
    );
    assert_eq!(
        hash_string(&[H::string("ab")]).unwrap(),
        "7347350983217793633"
    );
    // tstKeyer{"c"}: Key() string → "c"
    let keyer = H::Struct(
        GoStruct::new("tstKeyer")
            .unexported("key", H::string("c"))
            .with_key("c"),
    );
    assert_eq!(
        hash_string(&[H::string("a"), H::string("b"), keyer]).unwrap(),
        "4438730547989914315"
    );
    assert_eq!(
        hashing::md5_from_string_hex_encoded("abc"),
        "900150983cd24fb0d6963f7d28e17f72"
    );
}

// specs/images.md §4.5 (verified against the golden build).
#[test]
fn image_keys() {
    let h = |v: &H| format!("{:x}", hash_uint64(std::slice::from_ref(v)).unwrap());
    assert_eq!(h(&H::int(0)), "34c96acdcadb1bbb");
    assert_eq!(h(&H::string("x")), "5c80c09683041123");
    assert_eq!(h(&H::string_slice(Vec::<&str>::new())), "0");
    assert_eq!(h(&H::Bool(false)), "e934a84adb052768");
    assert_eq!(
        hashing::xxhash_from_string_hex_encoded(""),
        "ef46db3751d8e999"
    );

    // Imaging config SourceHash: HashStringHex(maps.Params) with _merge keys.
    let mut exif = Map::new(MapType::Params);
    exif.insert("_merge", Value::string("none"));
    exif.insert("disabledate", Value::Bool(false));
    exif.insert("disablelatlong", Value::Bool(false));
    exif.insert("excludefields", Value::string(".*"));
    exif.insert("includefields", Value::string(""));
    let mut imaging = Map::new(MapType::Params);
    imaging.insert("_merge", Value::string("none"));
    imaging.insert("exif", Value::map(exif));
    let cfg = hash_string_hex(&[H::Value(Value::map(imaging))]).unwrap();
    assert_eq!(cfg, "4bf645f71319dd1d");

    let key = |opts: &[&str]| hash_string_hex(&[H::string_slice(opts.iter().copied())]).unwrap();
    assert_eq!(key(&["resize", "600x480"]), "7bfd4638d4eb3be2");
    assert_eq!(key(&["resize", "300x240"]), "a08a22b9e9d91e29");
    assert_eq!(key(&["resize", "600x480", "webp"]), "590f9512b18cac1e");
    assert_eq!(key(&["resize", "640x480", "webp"]), "73816495dd661eee");

    let target = |incoming: &str, src: u64, conf_key: &str| {
        hash_string_hex(&[
            H::string(incoming),
            H::uint64(src),
            H::string(conf_key),
            H::string(cfg.as_str()),
        ])
        .unwrap()
    };
    let wm_hash: u64 = 6519743917224815147;
    let wm_hu = target("", wm_hash, &key(&["resize", "600x480"]));
    assert_eq!(wm_hu, "3bf49ff914f6e68c");
    let wm_key = format!("/images/watermark_hu_{wm_hu}.png_{wm_hash}");

    // images.Filter(images.Overlay(wm, 0, 0)): HashString([]gift.Filter{filter{...}})
    let overlay_key = |wm_key: &str| {
        let f = GoStruct::new("filter")
            .field(
                "Options",
                GoStruct::new("filterOpts")
                    .field("Version", H::int(0))
                    .field(
                        "Vals",
                        H::iface(H::any_slice(vec![H::string(wm_key), H::int(0), H::int(0)])),
                    ),
            )
            .field(
                "Filter",
                H::iface(H::Struct(
                    GoStruct::new("overlayFilter")
                        .unexported("src", H::Nil)
                        .unexported("x", H::int(0))
                        .unexported("y", H::int(0)),
                )),
            );
        hash_string(&[H::Slice(Some(vec![H::iface(H::Struct(f))]))]).unwrap()
    };
    let fk = overlay_key(&wm_key);
    assert_eq!(fk, "1682858112077426900");

    let src: u64 = 0x41b9c8214be9d5d7;
    let a = target("", src, &key(&["resize", "600x480"]));
    let b = target(&a, src, &fk);
    let c = target(&b, src, &key(&["resize", "600x480", "webp"]));
    assert_eq!(
        (a.as_str(), b.as_str(), c.as_str()),
        ("143d7e1f185771c7", "c8f2bc05dec496d8", "1e78ebf3348c6618")
    );
}

// go_value::Object conversion: a pointer-to-struct object hashes like the
// equivalent GoStruct behind a pointer; registrations override it.
mod objects {
    use super::*;
    use go_hashstructure::register_object;
    use go_value::{GoString, HostCtx, Object, Result as GoResult};
    use std::any::Any;
    use std::borrow::Cow;

    struct Page {
        title: &'static str,
        weight: i64,
    }

    impl Object for Page {
        fn type_name(&self) -> Cow<'_, str> {
            Cow::Borrowed("*hugolib.pageState")
        }
        fn has_method(&self, _name: &str) -> bool {
            false
        }
        fn call_method(
            &self,
            _ctx: HostCtx<'_>,
            _name: &str,
            _args: &[Value],
        ) -> Option<GoResult<Value>> {
            None
        }
        fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
            Some(vec![
                (Cow::Borrowed("Title"), Value::string(self.title)),
                (Cow::Borrowed("Weight"), Value::int(self.weight)),
            ])
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    struct Keyed;
    impl Object for Keyed {
        fn type_name(&self) -> Cow<'_, str> {
            Cow::Borrowed("*resources.genericResource")
        }
        fn has_method(&self, _name: &str) -> bool {
            false
        }
        fn call_method(
            &self,
            _ctx: HostCtx<'_>,
            _name: &str,
            _args: &[Value],
        ) -> Option<GoResult<Value>> {
            None
        }
        fn hash_key(&self) -> Option<GoString> {
            Some("/images/a.png".into())
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    struct Registered;
    impl Object for Registered {
        fn type_name(&self) -> Cow<'_, str> {
            Cow::Borrowed("*x.registered")
        }
        fn has_method(&self, _name: &str) -> bool {
            false
        }
        fn call_method(
            &self,
            _ctx: HostCtx<'_>,
            _name: &str,
            _args: &[Value],
        ) -> Option<GoResult<Value>> {
            None
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn object_as_struct() {
        let o = H::Value(Value::object(Page {
            title: "x",
            weight: 3,
        }));
        let s = H::ptr(H::Struct(
            GoStruct::new("pageState")
                .field("Title", H::string("x"))
                .field("Weight", H::int(3)),
        ));
        assert_eq!(
            hash_uint64(std::slice::from_ref(&o)).unwrap(),
            hash_uint64(std::slice::from_ref(&s)).unwrap()
        );
        // inside a []interface{} and a map[string]interface{}
        let l = H::Value(Value::any_list(vec![Value::object(Page {
            title: "x",
            weight: 3,
        })]));
        assert_eq!(
            hash_uint64(&[l]).unwrap(),
            hash_uint64(&[H::any_slice(vec![s])]).unwrap()
        );
    }

    #[test]
    fn keyer_object() {
        // toHashable: Key() providers are replaced by their key.
        let o = H::Value(Value::object(Keyed));
        assert_eq!(
            hash_string(std::slice::from_ref(&o)).unwrap(),
            hash_string(&[H::string("/images/a.png")]).unwrap()
        );
        assert_eq!(
            hash_string(&[H::string("a"), o]).unwrap(),
            hash_string(&[H::string("a"), H::string("/images/a.png")]).unwrap()
        );
    }

    #[test]
    fn registered_object() {
        register_object::<Registered>(|_| {
            H::Struct(GoStruct::new("registered").tagged("A", r#"hash:"ignore""#, H::int(1)))
        });
        let o = H::Value(Value::object(Registered));
        let want = H::Struct(GoStruct::new("registered"));
        assert_eq!(hash_uint64(&[o]).unwrap(), hash_uint64(&[want]).unwrap());
    }

    #[test]
    fn typed_nils() {
        // hashing.HashString(uri, map[string]any(nil)) — the GetRemote key.
        let a = hash_string(&[
            H::string("u"),
            H::Value(Value::TypedNil("map[string]interface {}".into())),
        ])
        .unwrap();
        assert_eq!(a, "17831426688197458792");
        // nil slice hashes as 0 (empty), nil pointer as int(0)
        assert_eq!(
            hash(&H::Value(Value::TypedNil("[]string".into())), None).unwrap(),
            0
        );
        assert_eq!(
            hash_uint64(&[H::Value(Value::TypedNil("*page.Page".into()))]).unwrap(),
            hash_uint64(&[H::int(0)]).unwrap()
        );
    }

    #[test]
    fn value_kinds() {
        // template.HTML hashes like a string; float32 as 4 bytes; int64 like int.
        assert_eq!(
            hash_uint64(&[H::Value(Value::html("x"))]).unwrap(),
            hash_uint64(&[H::string("x")]).unwrap()
        );
        assert_eq!(
            hash(&H::Value(Value::Float(42.0, FloatKind::F32)), None).unwrap(),
            5558953217260120943
        );
        assert_eq!(
            hash(&H::Value(Value::int64(42)), None).unwrap(),
            11375694726533372055
        );
    }
}
