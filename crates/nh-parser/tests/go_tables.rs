//! Ports of the Go test tables of `parser/metadecoders` (`decoder_test.go`, `format_test.go`),
//! `parser` (`frontmatter_test.go`, `lowercase_camel_json_test.go`) and go-toml's
//! `localtime_test.go`. ORG rows (the stubbed decoder) assert the explicit unsupported error
//! instead.

mod support;

use std::collections::BTreeMap;

use go_value::{GoString, IntKind, Map, MapType, Value};
use nh_parser::frontmatter::{ReplacingJsonMarshaller, interface_to_config};
use nh_parser::metadecoders::decoder::Decoder;
use nh_parser::metadecoders::format::{Format, format_from_string};
use nh_parser::metadecoders::toml::{LocalDate, LocalDateTime, LocalTime};
use serde_json::json;
use support::enc;

fn map(entries: &[(&str, Value)]) -> Value {
    let mut m = BTreeMap::new();
    for (k, v) in entries {
        m.insert(GoString::from(*k), v.clone());
    }
    Value::map(Map::with_entries(MapType::StringAny, m))
}

fn s(v: &str) -> Value {
    Value::string(v)
}

#[test]
fn format_from_string_table() {
    for (s, expect) in [
        ("json", Format::Json),
        ("yaml", Format::Yaml),
        ("yml", Format::Yaml),
        ("xml", Format::Xml),
        ("toml", Format::Toml),
        ("config.toml", Format::Toml),
        ("tOMl", Format::Toml),
        ("org", Format::Org),
        ("foo", Format::Unknown),
    ] {
        assert_eq!(format_from_string(s), expect, "{s}");
    }
}

#[test]
fn format_from_content_string_table() {
    let d = Decoder::default();
    for (data, expect) in [
        (r#"foo = "bar""#, Format::Toml),
        (r#"   foo = "bar""#, Format::Toml),
        (r#"foo="bar""#, Format::Toml),
        (r#"foo: "bar""#, Format::Yaml),
        (r#"foo:"bar""#, Format::Yaml),
        (r#"{ "foo": "bar""#, Format::Json),
        (r#"a,b,c""#, Format::Csv),
        (r#"<foo>bar</foo>""#, Format::Xml),
        ("asdfasdf", Format::Unknown),
        ("", Format::Unknown),
    ] {
        assert_eq!(
            d.format_from_content_string(data.as_bytes()),
            expect,
            "{data}"
        );
    }
}

#[test]
fn unmarshal_to_map_table() {
    let d = Decoder::default();
    let expect = map(&[("a", s("b"))]);
    let ok_cases = [
        (r#"a = "b""#, Format::Toml, expect.clone()),
        (r#"a: "b""#, Format::Yaml, expect.clone()),
        // Make sure we get all string keys, even for YAML
        (
            "a: Easy!\nb:\n  c: 2\n  d: [3, 4]",
            Format::Yaml,
            map(&[
                ("a", s("Easy!")),
                (
                    "b",
                    map(&[
                        ("c", Value::int(2)),
                        ("d", Value::any_list(vec![Value::int(3), Value::int(4)])),
                    ]),
                ),
            ]),
        ),
        (
            "a:\n  true: 1\n  false: 2",
            Format::Yaml,
            map(&[(
                "a",
                map(&[("true", Value::int(1)), ("false", Value::int(2))]),
            )]),
        ),
        (r#"{ "a": "b" }"#, Format::Json, expect.clone()),
    ];
    for (data, f, want) in ok_cases {
        let m = d.unmarshal_to_map(data.as_bytes(), f).unwrap();
        assert_eq!(enc(&Value::map(m)), enc(&want), "{data}");
    }
    let m = d
        .unmarshal_to_map(b"<root><a>b</a></root>", Format::Xml)
        .unwrap();
    assert_eq!(enc(&Value::map(m)), enc(&expect));
    // Stubbed decoder.
    let e = d.unmarshal_to_map(b"#+a: b", Format::Org).unwrap_err();
    assert!(e.message().starts_with("neohugo-rs:"), "{e}");
    // errors
    for (data, f) in [
        ("a = b", Format::Toml),
        ("a,b,c", Format::Csv),
        ("<root>just a string</root>", Format::Xml),
    ] {
        assert!(d.unmarshal_to_map(data.as_bytes(), f).is_err(), "{data}");
    }
}

// Go: parser/metadecoders/decoder_test.go:TestUnmarshalXML
#[test]
fn unmarshal_xml() {
    let xml_doc = r#"<?xml version="1.0" encoding="utf-8" standalone="yes"?>
	<rss version="2.0"
		xmlns:atom="http://www.w3.org/2005/Atom">
		<channel>
			<title>Example feed</title>
			<link>https://example.com/</link>
			<description>Example feed</description>
			<generator>Hugo -- gohugo.io</generator>
			<language>en-us</language>
			<copyright>Example</copyright>
			<lastBuildDate>Fri, 08 Jan 2021 14:44:10 +0000</lastBuildDate>
			<atom:link href="https://example.com/feed.xml" rel="self" type="application/rss+xml"/>
			<item>
				<title>Example title</title>
				<link>https://example.com/2021/11/30/example-title/</link>
				<pubDate>Tue, 30 Nov 2021 15:00:00 +0000</pubDate>
				<guid>https://example.com/2021/11/30/example-title/</guid>
				<description>Example description</description>
			</item>
		</channel>
	</rss>"#;
    let expect = map(&[
        ("-atom", s("http://www.w3.org/2005/Atom")),
        ("-version", s("2.0")),
        (
            "channel",
            map(&[
                ("copyright", s("Example")),
                ("description", s("Example feed")),
                ("generator", s("Hugo -- gohugo.io")),
                (
                    "item",
                    map(&[
                        ("description", s("Example description")),
                        ("guid", s("https://example.com/2021/11/30/example-title/")),
                        ("link", s("https://example.com/2021/11/30/example-title/")),
                        ("pubDate", s("Tue, 30 Nov 2021 15:00:00 +0000")),
                        ("title", s("Example title")),
                    ]),
                ),
                ("language", s("en-us")),
                ("lastBuildDate", s("Fri, 08 Jan 2021 14:44:10 +0000")),
                (
                    "link",
                    Value::any_list(vec![
                        s("https://example.com/"),
                        map(&[
                            ("-href", s("https://example.com/feed.xml")),
                            ("-rel", s("self")),
                            ("-type", s("application/rss+xml")),
                        ]),
                    ]),
                ),
                ("title", s("Example feed")),
            ]),
        ),
    ]);
    let m = Decoder::default()
        .unmarshal(xml_doc.as_bytes(), Format::Xml)
        .unwrap();
    assert_eq!(enc(&m), enc(&expect));
}

#[test]
fn unmarshal_to_interface_table() {
    let d = Decoder::default();
    let expect = map(&[("a", s("b"))]);
    let cases = [
        (
            r#"[ "Brecker", "Blake", "Redman" ]"#,
            Format::Json,
            Value::any_list(vec![s("Brecker"), s("Blake"), s("Redman")]),
        ),
        (r#"{ "a": "b" }"#, Format::Json, expect.clone()),
        ("", Format::Json, map(&[])),
        (r#"a = "b""#, Format::Toml, expect.clone()),
        (r#"a: "b""#, Format::Yaml, expect.clone()),
        (
            "a: Easy!\nb:\n  c: 2\n  d: [3, 4]",
            Format::Yaml,
            map(&[
                ("a", s("Easy!")),
                (
                    "b",
                    map(&[
                        ("c", Value::int(2)),
                        ("d", Value::any_list(vec![Value::int(3), Value::int(4)])),
                    ]),
                ),
            ]),
        ),
    ];
    for (data, f, want) in cases {
        let v = d.unmarshal(data.as_bytes(), f).unwrap();
        assert_eq!(enc(&v), enc(&want), "{data}");
    }
    // errors
    assert!(d.unmarshal(br#"a = ""#, Format::Toml).is_err());
    let v = d.unmarshal(b"<root><a>b</a></root>", Format::Xml).unwrap();
    assert_eq!(enc(&v), enc(&expect));
    let v = d.unmarshal(b"a,b,c", Format::Csv).unwrap();
    assert_eq!(
        enc(&v),
        json!({"t": "[][]string", "items": [{"t": "[]string", "items": [
            {"t": "string", "s": "a"}, {"t": "string", "s": "b"}, {"t": "string", "s": "c"}]}]})
    );
    // Stubbed decoder.
    let e = d.unmarshal(b"#+a: b", Format::Org).unwrap_err();
    assert!(e.message().starts_with("neohugo-rs:"), "{e}");
}

#[test]
fn unmarshal_string_to_table() {
    let d = Decoder::default();
    let cases = [
        ("a string", s("string"), s("a string")),
        (
            r#"{ "a": "b" }"#,
            Value::map(Map::new(MapType::StringAny)),
            map(&[("a", s("b"))]),
        ),
        ("32", Value::int64(1234), Value::int64(32)),
        ("32", Value::int(1234), Value::int(32)),
        (
            "3.14159",
            Value::float64(1.0),
            Value::float64("3.14159".parse().unwrap()),
        ),
        (
            "[3,7,9]",
            Value::any_list(vec![]),
            Value::any_list(vec![Value::int(3), Value::int(7), Value::int(9)]),
        ),
        (
            "[3.1,7.2,9.3]",
            Value::any_list(vec![]),
            Value::any_list(vec![
                Value::float64(3.1),
                Value::float64(7.2),
                Value::float64(9.3),
            ]),
        ),
    ];
    for (data, to, want) in cases {
        let v = d.unmarshal_string_to(data, &to).unwrap();
        assert_eq!(enc(&v), enc(&want), "{data}");
    }
    let e = d
        .unmarshal_string_to("1", &Value::Int(1, IntKind::Int32))
        .unwrap_err();
    assert_eq!(e.message(), "unmarshal: int32 not supported");
}

#[test]
fn stringify_yaml_map_keys() {
    // Go's TestStringifyYAMLMapKeys through the YAML decoder (go_value maps always have
    // string keys; go-yaml's to_value applies stringifyMapKeys).
    let d = Decoder::default();
    let cases = [
        (
            "a: 1\nb: 2",
            map(&[("a", Value::int(1)), ("b", Value::int(2))]),
        ),
        (
            "a: [1, {b: 2}]",
            map(&[(
                "a",
                Value::any_list(vec![Value::int(1), map(&[("b", Value::int(2))])]),
            )]),
        ),
        (
            "x: {true: 1, b: false}",
            map(&[(
                "x",
                map(&[("true", Value::int(1)), ("b", Value::Bool(false))]),
            )]),
        ),
        (
            "x: {1: a, 2: b}",
            map(&[("x", map(&[("1", s("a")), ("2", s("b"))]))]),
        ),
        (
            "- {1: a, 2: b}",
            Value::any_list(vec![map(&[("1", s("a")), ("2", s("b"))])]),
        ),
    ];
    for (data, want) in cases {
        let v = d.unmarshal(data.as_bytes(), Format::Yaml).unwrap();
        assert_eq!(enc(&v), enc(&want), "{data}");
    }
}

#[test]
fn interface_to_config_table() {
    let cases: Vec<(Value, Format, Option<&[u8]>)> = vec![
        // TOML
        (map(&[]), Format::Toml, Some(b"")),
        (
            map(&[("title", s("test' 1"))]),
            Format::Toml,
            Some(b"title = \"test' 1\"\n"),
        ),
        // YAML
        (map(&[]), Format::Yaml, Some(b"{}\n")),
        (
            map(&[("title", s("test 1"))]),
            Format::Yaml,
            Some(b"title: test 1\n"),
        ),
        // JSON
        (map(&[]), Format::Json, Some(b"{}\n")),
        (
            map(&[("title", s("test 1"))]),
            Format::Json,
            Some(b"{\n   \"title\": \"test 1\"\n}\n"),
        ),
        // Errors
        (Value::Invalid, Format::Toml, None),
        (map(&[]), Format::Unknown, None),
    ];
    for (input, f, want) in cases {
        let mut buf = Vec::new();
        let r = interface_to_config(&input, f, &mut buf);
        match want {
            Some(w) => {
                r.unwrap();
                assert_eq!(buf, w);
            }
            None => assert!(r.is_err()),
        }
    }
}

#[test]
fn replacing_json_marshaller() {
    let m = map(&[
        ("foo", s("bar")),
        ("baz", Value::int(42)),
        ("zeroInt1", Value::int(0)),
        ("zeroInt2", Value::int(0)),
        ("zeroFloat", Value::float64(0.0)),
        ("zeroString", s("")),
        ("zeroBool", Value::Bool(false)),
        ("nil", Value::Invalid),
    ]);
    let marshaller = ReplacingJsonMarshaller {
        value: m,
        keys_to_lower: true,
        omit_empty: true,
    };
    let b = marshaller.marshal_json().unwrap();
    assert_eq!(String::from_utf8(b).unwrap(), r#"{"baz":42,"foo":"bar"}"#);
}

#[test]
fn toml_localtime_table() {
    // go-toml localtime_test.go
    let d = LocalDate {
        year: 2021,
        month: 6,
        day: 8,
    };
    assert_eq!(d.string(), "2021-06-08");
    let t = LocalTime {
        hour: 20,
        minute: 12,
        second: 1,
        nanosecond: 2,
        precision: 0,
    };
    assert_eq!(t.string(), "20:12:01.000000002");
    let t = LocalTime {
        hour: 20,
        minute: 12,
        second: 1,
        nanosecond: 2,
        precision: 3,
    };
    assert_eq!(t.string(), "20:12:01.000");
    let t = LocalTime {
        hour: 20,
        minute: 12,
        second: 1,
        nanosecond: 100_000_000,
        precision: 0,
    };
    assert_eq!(t.string(), "20:12:01.1");
    let dt = LocalDateTime {
        date: d,
        time: LocalTime {
            hour: 20,
            minute: 12,
            second: 1,
            nanosecond: 2,
            precision: 0,
        },
    };
    assert_eq!(dt.string(), "2021-06-08T20:12:01.000000002");
    let tm = dt.as_time(&go_time::utc());
    assert_eq!(
        support::enc_time(&tm),
        json!({"t": "time.Time", "unix": 1623183121, "nsec": 2, "loc": "UTC", "abbr": "UTC", "off": 0})
    );
}
