//! Regression tests from the second red-team pass over the port. The
//! expected bytes and messages were printed by go1.27.1 (`encoding/json`,
//! jsonv2-backed); each test names the Go program's output it quotes.

use go_json::{JsonField, JsonStruct};
use go_value::{Map, MapType, Value};

/// go1.27.1's `Indent` (v2_indent.go:appendIndent) never returns when the
/// prefix or indent holds a byte other than space and tab, the indent is
/// empty, and the bytes its deferred placeholder fix-up walks hold a '\n'
/// followed by more spaces than the prefix is long: `for len(spaces) > 0 {
/// spaces = spaces[copy(spaces, invalidIndent):] }` makes no progress. On a
/// syntax error the fix-up walks `src` itself, because
/// `jsontext.AppendFormat` returns `append(dst, src...)` with the error.
/// This is what made `go-oracle/go-json -mode advtext` hang.
///
/// The port returns what Go computes before the fix-up: the syntax error
/// (Go returns it, with `dst` unchanged, for the same call with the prefix
/// replaced by spaces), or, for a valid `src`, the output with the fill
/// stopped after the prefix.
#[test]
fn indent_inputs_that_hang_go_terminate() {
    // Go: Indent(&b, src, " ", "") etc. for the errors.
    let errors: [(&[u8], &str, &str, i64); 3] = [
        (
            b"[\n  1,]",
            "a",
            "invalid character ']' looking for beginning of value",
            7,
        ),
        (
            b"\n  x",
            "a",
            "invalid character 'x' looking for beginning of value",
            4,
        ),
        (
            b"{\"a\":\n   tru}",
            "<>",
            "invalid character '}' in literal true (expecting 'e')",
            13,
        ),
    ];
    for (src, prefix, msg, off) in errors {
        let mut dst = b"pre".to_vec();
        let err = go_json::indent(&mut dst, src, prefix, "").unwrap_err();
        assert_eq!(err.to_string(), msg);
        assert_eq!(err.offset(), Some(off));
        assert_eq!(dst, b"pre");
    }

    let mut dst = b"pre".to_vec();
    go_json::indent(&mut dst, b"1\n  ", "a", "").unwrap();
    assert_eq!(dst, b"pre1\na ");

    // Neighbouring cases that do return in Go.
    let ok: [(&[u8], &str, &str, &[u8]); 3] = [
        (b"[1]\n \n", "abc", "", b"pre[\nabc1\nabc]\na\n"),
        (b"[1]\n   ", "ab", "c", b"pre[\nabc1\nab]\nabc"),
        (
            b"{\"a\":[1,{}]}\r\n\t  \n    ",
            "<>",
            "\u{2028}",
            b"pre{\n<>\xe2\x80\xa8\"a\": [\n<>\xe2\x80\xa8\xe2\x80\xa81,\n<>\xe2\x80\xa8\xe2\x80\xa8{}\n<>\xe2\x80\xa8]\n<>}\r\n\t  \n<>\xe2\x80",
        ),
    ];
    for (src, prefix, indent, want) in ok {
        let mut dst = b"pre".to_vec();
        go_json::indent(&mut dst, src, prefix, indent).unwrap();
        assert_eq!(dst, want);
    }
}

fn nest(depth: usize, inner: Value) -> Value {
    let mut v = inner;
    for _ in 0..depth {
        v = Value::any_list(vec![v]);
    }
    v
}

fn nest_map(depth: usize, inner: Value) -> Value {
    let mut v = inner;
    for _ in 0..depth {
        let mut m = Map::new(MapType::StringAny);
        m.entries.insert("a".into(), v);
        v = Value::map(m);
    }
    v
}

/// Marshal enforces jsontext's 10000-level limit when it opens an array or
/// object, but empty maps and slices are written without opening one. Go:
/// `json.Marshal` of the nested `[]any`/`map[string]any` values below
/// (go1.27.1 output: error text and SyntaxError.Offset, or the length).
#[test]
fn marshal_depth_limit() {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            let err = |r: Result<Vec<u8>, go_json::Error>| {
                let e = r.unwrap_err();
                (e.to_string(), e.offset())
            };
            let len = |r: Result<Vec<u8>, go_json::Error>| r.unwrap().len();

            assert_eq!(len(go_json::marshal(&nest(10000, Value::int(1)))), 20001);
            assert_eq!(
                err(go_json::marshal(&nest(10001, Value::int(1)))),
                ("exceeded max depth".to_string(), Some(10001))
            );
            assert_eq!(
                err(go_json::marshal(&nest_map(10001, Value::int(1)))),
                ("exceeded max depth".to_string(), Some(50001))
            );
            assert_eq!(
                err(go_json::marshal_indent(
                    &nest(10001, Value::int(1)),
                    "",
                    " "
                )),
                ("exceeded max depth".to_string(), Some(10001))
            );
            // json.RawMessage("[1]") at depth 10001.
            let raw = Value::object(go_json::RawMessage(Some(b"[1]".to_vec())));
            let e = go_json::marshal(&nest(10000, raw)).unwrap_err();
            assert_eq!(
                e.to_string(),
                "json: error calling MarshalJSON for type *jsontext.Value: exceeded max depth"
            );
            // Empty maps and slices do not open a level.
            let empty_map = Value::map(Map::new(MapType::StringAny));
            assert_eq!(len(go_json::marshal(&nest(10000, empty_map))), 20002);
            assert_eq!(
                len(go_json::marshal(&nest(10000, Value::any_list(vec![])))),
                20002
            );
            // An empty struct does.
            let empty_struct = Value::object(JsonStruct {
                type_name: "struct {}".into(),
                fields: vec![],
            });
            assert_eq!(
                err(go_json::marshal(&nest(10000, empty_struct))),
                ("exceeded max depth".to_string(), Some(10001))
            );
        })
        .unwrap()
        .join()
        .unwrap();
}

fn params(entries: &[(&str, Value)]) -> Value {
    let mut m = Map::new(MapType::Params);
    for (k, v) in entries {
        m.entries.insert((*k).into(), v.clone());
    }
    Value::map(m)
}

/// `omitzero` calls `IsZero` when the field's type has one, and Hugo's
/// `maps.Params` does: it is zero when empty or when its only key is
/// "_merge". `omitempty` (v1 semantics) only looks at the length, and an
/// interface-typed field is only zero when nil.
///
/// Go:
///
/// ```go
/// type S struct {
///     Z  maps.Params `json:"z,omitzero"`
///     E  maps.Params `json:"e,omitempty"`
///     ZA any         `json:"za,omitzero"`
///     EA any         `json:"ea,omitempty"`
/// }
/// json.Marshal(S{p, p, p, p})
/// ```
#[test]
fn params_omitzero_calls_is_zero() {
    let cases: Vec<(Value, &str)> = vec![
        (
            Value::TypedNil("maps.Params".into()),
            r#"{"za":null,"ea":null}"#,
        ),
        (params(&[]), r#"{"za":{},"ea":{}}"#),
        (
            params(&[("_merge", Value::string("deep"))]),
            r#"{"e":{"_merge":"deep"},"za":{"_merge":"deep"},"ea":{"_merge":"deep"}}"#,
        ),
        (
            params(&[("a", Value::int(1))]),
            r#"{"z":{"a":1},"e":{"a":1},"za":{"a":1},"ea":{"a":1}}"#,
        ),
        (
            params(&[("_merge", Value::int(1)), ("a", Value::int(2))]),
            r#"{"z":{"_merge":1,"a":2},"e":{"_merge":1,"a":2},"za":{"_merge":1,"a":2},"ea":{"_merge":1,"a":2}}"#,
        ),
        (
            params(&[("_MERGE", Value::int(1))]),
            r#"{"z":{"_MERGE":1},"e":{"_MERGE":1},"za":{"_MERGE":1},"ea":{"_MERGE":1}}"#,
        ),
    ];
    for (p, want) in cases {
        let s = JsonStruct {
            type_name: "main.S".into(),
            fields: vec![
                JsonField::new("z", p.clone()).omit_zero(),
                JsonField::new("e", p.clone()).omit_empty(),
                JsonField::new("za", p.clone())
                    .omit_zero()
                    .interface_typed(),
                JsonField::new("ea", p.clone())
                    .omit_empty()
                    .interface_typed(),
            ],
        };
        let got = go_json::marshal(&Value::object(s)).unwrap();
        assert_eq!(String::from_utf8(got).unwrap(), want, "{p:?}");
    }
}
