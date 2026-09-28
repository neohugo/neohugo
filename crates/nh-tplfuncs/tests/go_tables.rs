//! Tables of the Go tests of the data namespaces (`tpl/<ns>/*_test.go`), transcribed with their
//! expected values. The full differential coverage (these inputs included) is `tests/data.rs`.

mod support;

use std::sync::Arc;

use go_value::{GoString, Object, Value};
use nh_deps::deps::Deps;

use support::TestCfg;

fn deps() -> Arc<Deps> {
    Arc::new(Deps::for_tests(TestCfg::new("en")))
}

fn call(o: &dyn Object, m: &str, a: &[Value]) -> go_value::Result<Value> {
    o.call_method(&(), m, a).expect("method exists")
}

fn s(v: &str) -> Value {
    Value::string(v)
}

fn str_of(v: &Value) -> String {
    v.as_go_string().unwrap().to_str_lossy().into_owned()
}

// crypto_test.go: TestMD5, TestSHA1, TestSHA256, TestHMAC.
#[test]
fn crypto() {
    let ns = nh_tplfuncs::crypto::crypto::Namespace::new(deps());
    for (m, input, want) in [
        (
            "MD5",
            "Hello world, gophers!",
            "b3029f756f98f79e7f1b7f1d1f0dd53b",
        ),
        (
            "MD5",
            "Lorem ipsum dolor",
            "06ce65ac476fc656bea3fca5d02cfd81",
        ),
        (
            "SHA1",
            "Hello world, gophers!",
            "c8b5b0e33d408246e30f53e32b8f7627a7a649d4",
        ),
        (
            "SHA1",
            "Lorem ipsum dolor",
            "45f75b844be4d17b3394c6701768daf39419c99b",
        ),
        (
            "SHA256",
            "Hello world, gophers!",
            "6ec43b78da9669f50e4e422575c54bf87536954ccd58280219c393f2ce352b46",
        ),
        (
            "SHA256",
            "Lorem ipsum dolor",
            "9b3e1beb7053e0f900a674dd1c99aca3355e1275e1b03d3cb1bc977f5154e196",
        ),
    ] {
        assert_eq!(
            str_of(&call(&ns, m, &[s(input)]).unwrap()),
            want,
            "{m}({input})"
        );
    }
    // Go: a *testing.T cannot be cast to a string.
    let t = support::decode_value(&serde_json::json!({"t": "*main.tstObj", "id": "o1"}), &[]);
    assert!(call(&ns, "MD5", std::slice::from_ref(&t)).is_err());

    let key = "Secret key";
    let msg = "Hello world, gophers!";
    for (h, enc, want) in [
        ("md5", None, "36eb69b6bf2de96b6856fdee8bf89754"),
        ("sha1", None, "84a76647de6cd47ac6ae4258e3753f711172ce68"),
        (
            "sha256",
            None,
            "b6d11b6c53830b9d87036272ca9fe9d19306b8f9d8aa07b15da27d89e6e34f40",
        ),
        (
            "sha512",
            None,
            "dc3e586cd936865e2abc4c12665e9cc568b2dad714df3c9037cbea159d036cfc4209da9e3fcd30887ff441056941966899f6fb7eec9646ff9ddb592595a8eb7f",
        ),
        ("md5", Some("hex"), "36eb69b6bf2de96b6856fdee8bf89754"),
    ] {
        let mut a = vec![s(h), s(key), s(msg)];
        if let Some(e) = enc {
            a.push(s(e));
        }
        assert_eq!(str_of(&call(&ns, "HMAC", &a).unwrap()), want, "hmac {h}");
    }
    let bin = call(&ns, "HMAC", &[s("md5"), s(key), s(msg), s("binary")]).unwrap();
    assert_eq!(
        bin.as_go_string().unwrap(),
        &GoString::from(&b"6\xebi\xb6\xbf-\xe9khV\xfd\xee\x8b\xf8\x97T"[..])
    );
    assert!(call(&ns, "HMAC", &[s("md5"), s(key), s(msg), s("foo")]).is_err());
    assert!(call(&ns, "HMAC", &[s("md5"), s(key), s(msg), s("")]).is_err());
}

// hash_test.go: TestXxHash; init.go examples of FNV32a.
#[test]
fn hash() {
    let ns = nh_tplfuncs::hash::hash::Namespace::new(deps());
    assert_eq!(
        str_of(
            &call(
                &ns,
                "XxHash",
                &[s("The quick brown fox jumps over the lazy dog")]
            )
            .unwrap()
        ),
        "0b242d361fda71bc"
    );
    assert_eq!(
        call(&ns, "FNV32a", &[s("Hugo Rocks!!")]).unwrap(),
        Value::int(1515779328)
    );
}

// collections_test.go: TestSeq.
#[test]
fn seq() {
    let ns = nh_tplfuncs::collections::collections::Namespace::new(deps());
    let ok: &[(&[Value], &[i64])] = &[
        (
            &[Value::int(-2), Value::int(5)],
            &[-2, -1, 0, 1, 2, 3, 4, 5],
        ),
        (&[Value::int(1), Value::int(2), Value::int(4)], &[1, 3]),
        (&[Value::int(1)], &[1]),
        (&[Value::int(3)], &[1, 2, 3]),
        (&[Value::float64(3.2)], &[1, 2, 3]),
        (&[Value::int(0)], &[]),
        (&[Value::int(-1)], &[-1]),
        (&[Value::int(-3)], &[-1, -2, -3]),
        (&[Value::int(3), Value::int(-2)], &[3, 2, 1, 0, -1, -2]),
        (&[Value::int(6), Value::int(-2), Value::int(2)], &[6, 4, 2]),
    ];
    for (a, want) in ok {
        let got = call(&ns, "Seq", a).unwrap();
        let want = Value::list(
            go_value::SliceType::Int,
            want.iter().map(|i| Value::int(*i)).collect(),
        );
        assert_eq!(got, want, "seq {a:?}");
    }
    let errs: &[&[Value]] = &[
        &[Value::int(1), Value::int(0), Value::int(2)],
        &[Value::int(1), Value::int(-1), Value::int(2)],
        &[Value::int(2), Value::int(1), Value::int(1)],
        &[Value::int(2), Value::int(1), Value::int(1), Value::int(1)],
        &[Value::int(2001)],
        &[],
        &[Value::int(0), Value::int(-1000000)],
    ];
    for a in errs {
        assert!(call(&ns, "Seq", a).is_err(), "seq {a:?}");
    }
}

// math_test.go: TestRound, TestMod, TestModBool.
#[test]
fn math() {
    let ns = nh_tplfuncs::math::math::Namespace::new(deps());
    for (x, want) in [
        (0.1, 0.0),
        (0.5, 1.0),
        (1.1, 1.0),
        (1.5, 2.0),
        (-0.1, 0.0),
        (-0.5, -1.0),
        (-1.1, -1.0),
        (-1.5, -2.0),
    ] {
        // qt.Equals: -0 == 0.
        match call(&ns, "Round", &[Value::float64(x)]).unwrap() {
            Value::Float(f, go_value::FloatKind::F64) => assert!(f == want, "round {x}: {f}"),
            other => panic!("round {x}: {other:?}"),
        }
    }
    assert!(call(&ns, "Round", &[s("abc")]).is_err());
    let i8v = |i| Value::Int(i, go_value::IntKind::Int8);
    let i16v = |i| Value::Int(i, go_value::IntKind::Int16);
    let i32v = |i| Value::Int(i, go_value::IntKind::Int32);
    for (a, b, want) in [
        (Value::int(3), Value::int(2), Some(1)),
        (Value::int(3), Value::int(1), Some(0)),
        (Value::int(3), Value::int(0), None),
        (Value::int(0), Value::int(3), Some(0)),
        (Value::float64(3.1), Value::int(2), Some(1)),
        (Value::int(3), Value::float64(2.1), Some(1)),
        (Value::float64(3.1), Value::float64(2.1), Some(1)),
        (i8v(3), i8v(2), Some(1)),
        (i16v(3), i16v(2), Some(1)),
        (i32v(3), i32v(2), Some(1)),
        (Value::int64(3), Value::int64(2), Some(1)),
        (s("3"), s("2"), Some(1)),
        (s("3.1"), s("2"), Some(1)),
        (s("aaa"), s("0"), None),
        (s("3"), s("aaa"), None),
    ] {
        let got = call(&ns, "Mod", &[a.clone(), b.clone()]);
        match want {
            None => assert!(got.is_err(), "mod {a:?} {b:?}"),
            Some(w) => assert_eq!(got.unwrap(), Value::int64(w), "mod {a:?} {b:?}"),
        }
    }
    assert_eq!(
        call(&ns, "ModBool", &[Value::int(6), Value::int(3)]).unwrap(),
        Value::Bool(true)
    );
}

// safe_test.go: every func wraps cast.ToString in its html/template type.
#[test]
fn safe() {
    let ns = nh_tplfuncs::safe::safe::Namespace::new(deps());
    for (m, t) in [
        ("CSS", "template.CSS"),
        ("HTML", "template.HTML"),
        ("HTMLAttr", "template.HTMLAttr"),
        ("JS", "template.JS"),
        ("JSStr", "template.JSStr"),
        ("URL", "template.URL"),
    ] {
        let v = call(&ns, m, &[s("a<b>")]).unwrap();
        assert_eq!(v.go_type_name(), t);
        assert_eq!(str_of(&v), "a<b>");
    }
}

// encoding_test.go: TestBase64Decode, TestBase64Encode, TestJsonify.
#[test]
fn encoding() {
    let ns = nh_tplfuncs::encoding::encoding::Namespace::new(deps());
    assert_eq!(
        str_of(&call(&ns, "Base64Decode", &[s("YWJjMTIzIT8kKiYoKSctPUB+")]).unwrap()),
        "abc123!?$*&()'-=@~"
    );
    assert!(call(&ns, "Base64Decode", &[s("YWJjMTIzIT8kKiYoKSctPUB")]).is_err());
    assert_eq!(
        str_of(&call(&ns, "Base64Encode", &[s("YWJjMTIzIT8kKiYoKSctPUB+")]).unwrap()),
        "WVdKak1USXpJVDhrS2lZb0tTY3RQVUIr"
    );
    let v = call(&ns, "Jsonify", &[Value::string_list(["a", "b"])]).unwrap();
    assert_eq!(v, Value::html("[\"a\",\"b\"]"));
}

// cast_test.go: TestToInt, TestToString, TestToFloat (html/template types become strings).
#[test]
fn cast() {
    let ns = nh_tplfuncs::cast::cast::Namespace::new(deps());
    assert_eq!(call(&ns, "ToInt", &[s("1")]).unwrap(), Value::int(1));
    assert_eq!(
        call(&ns, "ToInt", &[Value::html("4")]).unwrap(),
        Value::int(4)
    );
    assert!(call(&ns, "ToInt", &[s("a")]).is_err());
    assert_eq!(call(&ns, "ToString", &[Value::int(1)]).unwrap(), s("1"));
    assert_eq!(
        call(&ns, "ToFloat", &[Value::html("4.5")]).unwrap(),
        Value::float64(4.5)
    );
    assert_eq!(
        call(&ns, "ToFloat", &[Value::int(1)]).unwrap(),
        Value::float64(1.0)
    );
}

// reflect_test.go: TestIsMap, TestIsSlice.
#[test]
fn reflect() {
    let ns = nh_tplfuncs::reflect::reflect::Namespace::new(deps());
    let m = Value::map(go_value::Map::new(go_value::MapType::StringAny));
    let l = Value::string_list(["a"]);
    assert_eq!(
        call(&ns, "IsMap", std::slice::from_ref(&m)).unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        call(&ns, "IsMap", std::slice::from_ref(&l)).unwrap(),
        Value::Bool(false)
    );
    assert_eq!(call(&ns, "IsSlice", &[l]).unwrap(), Value::Bool(true));
    assert_eq!(call(&ns, "IsSlice", &[m]).unwrap(), Value::Bool(false));
}
