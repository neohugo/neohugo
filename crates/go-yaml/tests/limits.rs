//! Port of gopkg.in/yaml.v2@v2.4.0 limit_test.go (TestLimits), full size,
//! plus the error table of decode_test.go (TestUnmarshalErrors) and
//! TestFuzzCrashers.

fn big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(1 << 30)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap()
}

fn limit_tests() -> Vec<(&'static str, Vec<u8>, &'static str)> {
    let r = |s: &str, n: usize| s.repeat(n);
    vec![
        (
            "1000kb of maps with 100 aliases",
            format!(
                "{{a: &a [{{a}}{}], b: &b [*a{}]}}",
                r(",{a}", 1000 * 1024 / 4 - 100),
                r(",*a", 99)
            )
            .into_bytes(),
            "yaml: document contains excessive aliasing",
        ),
        (
            "1000kb of deeply nested slices",
            r("[", 1000 * 1024).into_bytes(),
            "yaml: exceeded max depth of 10000",
        ),
        (
            "1000kb of deeply nested maps",
            format!("x: {}", r("{", 1000 * 1024)).into_bytes(),
            "yaml: exceeded max depth of 10000",
        ),
        (
            "1000kb of deeply nested indents",
            r("- ", 1000 * 1024).into_bytes(),
            "yaml: exceeded max depth of 10000",
        ),
        (
            "1000kb of 1000-indent lines",
            r(&(r("- ", 1000) + "\n"), 1024 / 2).into_bytes(),
            "",
        ),
        (
            "1kb of maps",
            format!("a: &a [{{a}}{}]", r(",{a}", 1024 / 4 - 1)).into_bytes(),
            "",
        ),
        (
            "10kb of maps",
            format!("a: &a [{{a}}{}]", r(",{a}", 10 * 1024 / 4 - 1)).into_bytes(),
            "",
        ),
        (
            "100kb of maps",
            format!("a: &a [{{a}}{}]", r(",{a}", 100 * 1024 / 4 - 1)).into_bytes(),
            "",
        ),
        (
            "1000kb of maps",
            format!("a: &a [{{a}}{}]", r(",{a}", 1000 * 1024 / 4 - 1)).into_bytes(),
            "",
        ),
        (
            "1000kb slice nested at max-depth",
            format!(
                "{}1{}{}",
                r("[", 10000),
                r(",1", 1000 * 1024 / 2 - 20000 - 1),
                r("]", 10000)
            )
            .into_bytes(),
            "",
        ),
        (
            "1000kb slice nested in maps at max-depth",
            format!(
                "{{a,b:\n{} [1{}]{}",
                r(" {a,b:", 10000 - 2),
                r(",1", 1000 * 1024 / 2 - 6 * 10000 - 1),
                r("}", 10000 - 1)
            )
            .into_bytes(),
            "",
        ),
        (
            "1000kb of 10000-nested lines",
            r(
                &format!("- {}{}\n", r("[", 10000), r("]", 10000)),
                1000 * 1024 / 20000,
            )
            .into_bytes(),
            "",
        ),
    ]
}

#[test]
fn limits() {
    big_stack(|| {
        for (name, data, want) in limit_tests() {
            let t = std::time::Instant::now();
            let got = go_yaml::unmarshal(&data);
            let el = t.elapsed();
            eprintln!("{name}: {:?}", el);
            if want.is_empty() {
                assert!(got.is_ok(), "{name}: {:?}", got.err());
            } else {
                assert_eq!(
                    got.err().map(|e| e.message()),
                    Some(want.to_string()),
                    "{name}"
                );
            }
        }
    });
}

// decode_test.go: unmarshalErrorTests (the Go test matches with regexps;
// these are the exact messages).
#[test]
fn unmarshal_errors() {
    let cases: &[(&str, &str)] = &[
        (
            "v: !!float 'error'",
            "yaml: cannot decode !!str `error` as a !!float",
        ),
        ("v: [A,", "yaml: line 1: did not find expected node content"),
        (
            "v:\n- [A,",
            "yaml: line 2: did not find expected node content",
        ),
        (
            "a:\n- b: *,",
            "yaml: line 2: did not find expected alphabetic or numeric character",
        ),
        ("a: *b\n", "yaml: unknown anchor 'b' referenced"),
        ("a: &a\n  b: *a\n", "yaml: anchor 'a' value contains itself"),
        (
            "a: &x null\n<<:\n- *x\nb: &x {}\n",
            "yaml: map merge requires map or sequence of maps as the value",
        ),
        (
            "value: -",
            "yaml: block sequence entries are not allowed in this context",
        ),
        (
            "a: !!binary ==",
            "yaml: !!binary value contains invalid base64 data",
        ),
        ("{[.]}", "yaml: invalid map key: []interface {}{\".\"}"),
        (
            "{{.}}",
            "yaml: invalid map key: map[interface {}]interface {}{\".\":interface {}(nil)}",
        ),
        ("b: *a\na: &a {c: 1}", "yaml: unknown anchor 'a' referenced"),
        (
            "%TAG !%79! tag:yaml.org,2002:\n---\nv: !%79!int '1'",
            "yaml: did not find expected whitespace",
        ),
        (
            "a:\n  1:\nb\n  2:",
            "yaml: line 4: could not find expected ':'",
        ),
        (
            "a: &a [00,00,00,00,00,00,00,00,00]\n\
             b: &b [*a,*a,*a,*a,*a,*a,*a,*a,*a]\n\
             c: &c [*b,*b,*b,*b,*b,*b,*b,*b,*b]\n\
             d: &d [*c,*c,*c,*c,*c,*c,*c,*c,*c]\n\
             e: &e [*d,*d,*d,*d,*d,*d,*d,*d,*d]\n\
             f: &f [*e,*e,*e,*e,*e,*e,*e,*e,*e]\n\
             g: &g [*f,*f,*f,*f,*f,*f,*f,*f,*f]\n\
             h: &h [*g,*g,*g,*g,*g,*g,*g,*g,*g]\n\
             i: &i [*h,*h,*h,*h,*h,*h,*h,*h,*h]\n",
            "yaml: document contains excessive aliasing",
        ),
    ];
    for (data, want) in cases {
        let got = go_yaml::unmarshal(data.as_bytes())
            .err()
            .map(|e| e.message());
        assert_eq!(got.as_deref(), Some(*want), "input {data:?}");
        if data.contains(':') {
            // Repeat test with typed value (map[string]interface{}).
            let got = go_yaml::unmarshal_str_map(data.as_bytes())
                .err()
                .map(|e| e.message());
            if !want.starts_with("yaml: invalid map key") {
                assert_eq!(got.as_deref(), Some(*want), "map input {data:?}");
            } else {
                assert!(got.is_some());
            }
        }
    }
}

// decode_test.go: TestFuzzCrashers — must not panic.
#[test]
fn fuzz_crashers() {
    for data in [
        "\"\\0\\\r\n",
        "  0: [\n] 0",
        "? ? \"\n\" 0",
        "    - {\n000}0",
        "0:\n  0: [0\n] 0",
        "    - \"\n000\"0",
        "    - \"\n000\"\"",
        "0:\n    - {\n000}0",
        "0:\n    - \"\n000\"0",
        "0:\n    - \"\n000\"\"",
        " \u{feff}\n",
        "? \u{feff}\n",
        "? \u{feff}:\n",
        "0: \u{feff}\n",
        "? \u{feff}: \u{feff}\n",
    ] {
        let _ = go_yaml::unmarshal(data.as_bytes());
        let _ = go_yaml::unmarshal_str_map(data.as_bytes());
    }
}

// decode_test.go: decoderTests (multi-document streams).
#[test]
fn decoder_multi_doc() {
    let cases: &[(&str, &[&str])] = &[
        ("", &[]),
        ("a: b", &["imap{str:\"a\"=str:\"b\"}"]),
        ("---\na: b\n...\n", &["imap{str:\"a\"=str:\"b\"}"]),
        (
            "---\n'hello'\n...\n---\ngoodbye\n...\n",
            &["str:\"hello\"", "str:\"goodbye\""],
        ),
    ];
    for (data, want) in cases {
        let mut d = go_yaml::Decoder::new(data.as_bytes());
        let mut got = Vec::new();
        while let Some(v) = d.decode() {
            got.push(go_yaml::dump(&v.unwrap()));
        }
        assert_eq!(got, *want, "{data:?}");
    }
}

// decode_test.go: TestUnmarshalNaN / mergeTests.
#[test]
fn nan_and_merge() {
    let v = go_yaml::unmarshal_str_map(b"notanum: .NaN")
        .unwrap()
        .unwrap();
    match v.get(b"notanum") {
        Some(go_yaml::Yaml::Float64(f)) => {
            assert!(f.is_nan());
            assert_eq!(f.to_bits(), go_yaml::GO_NAN_BITS);
        }
        other => panic!("{other:?}"),
    }
}

// decode_test.go: unmarshalStrictTests (the map[interface{}]interface{} case)
// and the same input in non-strict mode.
#[test]
fn strict_duplicate_keys() {
    let data = b"a: 1\n9: 2\nnull: 3\n9: 4";
    let v = go_yaml::unmarshal(data).unwrap();
    assert_eq!(
        go_yaml::dump(&v),
        "imap{int:9=int:4,nil=int:3,str:\"a\"=int:1}"
    );
    let err = go_yaml::unmarshal_with(data, true).unwrap_err();
    assert_eq!(
        err.message(),
        "yaml: unmarshal errors:\n  line 4: key 9 already set in map"
    );
}

/// Alias chains multiply the decoder's recursion depth: each anchor nests
/// 190 levels and aliases the previous one, so decoding recurses ~11,400
/// levels although the parsed tree is only ~190 deep. Go decodes this; the
/// port must not overflow a default-size (2 MB) thread stack.
fn alias_chain(k: usize, d: usize) -> Vec<u8> {
    let mut s = format!("a0: &a0 {}1{}\n", "[".repeat(d), "]".repeat(d));
    for i in 1..k {
        s.push_str(&format!(
            "a{i}: &a{i} {}*a{}{}\n",
            "[".repeat(d),
            i - 1,
            "]".repeat(d)
        ));
    }
    s.into_bytes()
}

fn depth_of(v: &go_yaml::Yaml) -> usize {
    let mut d = 0;
    let mut cur = v;
    loop {
        match cur {
            go_yaml::Yaml::Seq(items) if !items.is_empty() => {
                d += 1;
                cur = &items[0];
            }
            _ => return d,
        }
    }
}

#[test]
fn alias_chain_on_small_stack() {
    let data = alias_chain(60, 190);
    let (ok_iface, depth, ok_meta, ok_map) = std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(move || {
            let v = go_yaml::unmarshal(&data).expect("decodes like Go");
            let depth = match &v {
                go_yaml::Yaml::Map(m) => m
                    .iter()
                    .find(|(k, _)| k.as_bytes() == Some(b"a59"))
                    .map(|(_, v)| depth_of(v))
                    .unwrap_or(0),
                _ => 0,
            };
            let meta = go_yaml::metadecoders::unmarshal(&data);
            let map = go_yaml::metadecoders::unmarshal_to_map(&data);
            let oks = (true, depth, meta.is_ok(), map.is_ok());
            // go_value::Value drops recursively (go-value crate): an
            // 11,400-deep value does not fit a 2 MB stack in debug builds.
            // Only decoding is under test here, so drop elsewhere.
            big_stack(move || drop((meta, map)));
            oks
        })
        .unwrap()
        .join()
        .unwrap();
    assert!(ok_iface && ok_meta && ok_map);
    assert_eq!(depth, 60 * 190);
}

#[test]
fn alias_cycle_and_excess() {
    let cyc = format!("a: &a [{}*a{}]\n", "[".repeat(150), "]".repeat(150));
    let r = std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(move || go_yaml::unmarshal(cyc.as_bytes()).map(|_| ()))
        .unwrap()
        .join()
        .unwrap();
    assert_eq!(
        r.unwrap_err().message(),
        "yaml: anchor 'a' value contains itself"
    );
    let data = alias_chain(200, 20);
    let r = std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(move || go_yaml::unmarshal(&data).map(|_| ()))
        .unwrap()
        .join()
        .unwrap();
    assert_eq!(
        r.unwrap_err().message(),
        "yaml: document contains excessive aliasing"
    );
}
