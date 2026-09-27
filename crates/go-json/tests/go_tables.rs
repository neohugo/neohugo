//! Tests ported from Go's own encoding/json test tables (go1.27.1,
//! jsonv2 build: v2_scanner_test.go, v2_stream_test.go, v2_encode_test.go).

use std::borrow::Cow;

use go_json::{Decoder, Encoder, JsonField, JsonStruct, Token};
use go_value::{GoString, Kind, Map, MapType, Object, Value};

// Go: v2_scanner_test.go:TestValid
#[test]
fn valid() {
    let tests: &[(&str, bool)] = &[
        ("foo", false),
        ("}{", false),
        ("{]", false),
        ("{}", true),
        (r#"{"foo":"bar"}"#, true),
        (r#"{"foo":"bar","bar":{"baz":["qux"]}}"#, true),
    ];
    for &(data, ok) in tests {
        assert_eq!(go_json::valid(data.as_bytes()), ok, "Valid(`{data}`)");
    }
}

// Go: v2_scanner_test.go:TestCompactAndIndent
#[test]
fn compact_and_indent() {
    let tests: &[(&str, &str)] = &[
        ("1", "1"),
        ("{}", "{}"),
        ("[]", "[]"),
        (r#"{"":2}"#, "{\n\t\"\": 2\n}"),
        ("[3]", "[\n\t3\n]"),
        ("[1,2,3]", "[\n\t1,\n\t2,\n\t3\n]"),
        (r#"{"x":1}"#, "{\n\t\"x\": 1\n}"),
        (
            r#"[true,false,null,"x",1,1.5,0,-5e+2]"#,
            "[\n\ttrue,\n\tfalse,\n\tnull,\n\t\"x\",\n\t1,\n\t1.5,\n\t0,\n\t-5e+2\n]",
        ),
        (
            "{\"\":\"<>&\u{2028}\u{2029}\"}",
            "{\n\t\"\": \"<>&\u{2028}\u{2029}\"\n}",
        ), // See golang.org/issue/34070
        ("null", "null \n\r\t"), // See golang.org/issue/13520 and golang.org/issue/74806
    ];
    for &(compact, indent) in tests {
        let mut buf = Vec::new();
        go_json::compact(&mut buf, compact.as_bytes()).unwrap();
        assert_eq!(buf, compact.as_bytes());

        let mut buf = Vec::new();
        go_json::compact(&mut buf, indent.as_bytes()).unwrap();
        assert_eq!(buf, compact.as_bytes());

        let mut buf = Vec::new();
        go_json::indent(&mut buf, indent.as_bytes(), "", "\t").unwrap();
        assert_eq!(buf, indent.as_bytes());

        let mut buf = Vec::new();
        go_json::indent(&mut buf, compact.as_bytes(), "", "\t").unwrap();
        assert_eq!(
            buf,
            indent.trim_end_matches([' ', '\n', '\r', '\t']).as_bytes()
        );
    }
}

// Go: v2_scanner_test.go:TestCompactSeparators
#[test]
fn compact_separators() {
    // U+2028 and U+2029 should be escaped inside strings.
    // They should not appear outside strings.
    let tests: &[(&str, &str)] = &[
        ("{\"\u{2028}\": 1}", "{\"\u{2028}\":1}"),
        ("{\"\u{2029}\" :2}", "{\"\u{2029}\":2}"),
    ];
    for &(input, compact) in tests {
        let mut buf = Vec::new();
        go_json::compact(&mut buf, input.as_bytes()).unwrap();
        assert_eq!(buf, compact.as_bytes());
    }
}

// Go: v2_scanner_test.go:TestIndentErrors
#[test]
fn indent_errors() {
    let tests: &[(&str, &str, i64)] = &[
        (
            r#"{"X": "foo", "Y"}"#,
            "invalid character '}' after object key",
            r#"{"X": "foo", "Y"}"#.len() as i64,
        ),
        (
            r#"{"X": "foo" "Y": "bar"}"#,
            "invalid character '\"' after object key:value pair",
            r#"{"X": "foo" ""#.len() as i64,
        ),
    ];
    for &(input, msg, offset) in tests {
        let mut buf = Vec::new();
        let err = go_json::indent(&mut buf, input.as_bytes(), "", "").unwrap_err();
        assert_eq!(
            err,
            go_json::Error::Syntax(go_json::SyntaxError {
                msg: msg.to_string(),
                offset
            })
        );
    }
}

/// Go: v2_stream_test.go:streamTest
#[allow(clippy::approx_constant)] // Go's test value 3.14
fn stream_test() -> Vec<Value> {
    let mut m = Map::new(MapType::StringAny);
    m.insert("\u{212A}", Value::from("Kelvin"));
    m.insert("\u{00DF}", Value::from("long s"));
    vec![
        Value::float64(0.1),
        Value::from("hello"),
        Value::Invalid,
        Value::Bool(true),
        Value::Bool(false),
        Value::any_list(vec![Value::from("a"), Value::from("b"), Value::from("c")]),
        Value::map(m),
        Value::float64(3.14), // another value to make sure something can follow map
    ]
}

const STREAM_ENCODED: &str = "0.1\n\"hello\"\nnull\ntrue\nfalse\n[\"a\",\"b\",\"c\"]\n{\"\u{00DF}\":\"long s\",\"\u{212A}\":\"Kelvin\"}\n3.14\n";

/// Go: v2_stream_test.go:nlines
fn nlines(s: &str, n: usize) -> &str {
    if n == 0 {
        return "";
    }
    let mut count = 0;
    for (i, c) in s.char_indices() {
        if c == '\n' {
            count += 1;
            if count == n {
                return &s[..i + 1];
            }
        }
    }
    s
}

// Go: v2_stream_test.go:TestEncoder
#[test]
fn encoder() {
    let st = stream_test();
    for i in 0..=st.len() {
        let mut enc = Encoder::new(Vec::new());
        // Check that enc.SetIndent("", "") turns off indentation.
        enc.set_indent(">", ".");
        enc.set_indent("", "");
        for v in &st[..i] {
            enc.encode(v).unwrap();
        }
        assert_eq!(
            String::from_utf8(enc.into_inner()).unwrap(),
            nlines(STREAM_ENCODED, i)
        );
    }
}

// Go: v2_stream_test.go:TestEncoderIndent
#[test]
fn encoder_indent() {
    let want = "0.1\n\"hello\"\nnull\ntrue\nfalse\n[\n>.\"a\",\n>.\"b\",\n>.\"c\"\n>]\n{\n>.\"\u{00DF}\": \"long s\",\n>.\"\u{212A}\": \"Kelvin\"\n>}\n3.14\n";
    let mut enc = Encoder::new(Vec::new());
    enc.set_indent(">", ".");
    for v in &stream_test() {
        enc.encode(v).unwrap();
    }
    assert_eq!(String::from_utf8(enc.into_inner()).unwrap(), want);
}

/// Go's strMarshaler / strPtrMarshaler: MarshalJSON returns the string.
struct StrMarshaler(&'static str, &'static str);

impl Object for StrMarshaler {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed(self.1)
    }
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn has_method(&self, _: &str) -> bool {
        false
    }
    fn call_method(
        &self,
        _: go_value::HostCtx<'_>,
        _: &str,
        _: &[Value],
    ) -> Option<go_value::Result<Value>> {
        None
    }
    fn marshal_json(&self) -> Option<go_value::Result<Vec<u8>>> {
        Some(Ok(self.0.as_bytes().to_vec()))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// Go: v2_stream_test.go:TestEncoderSetEscapeHTML
#[test]
fn encoder_set_escape_html() {
    let tag_struct = JsonStruct::new(
        "struct { Valid int \"json:\\\"<>&#! \\\"\"; Invalid int \"json:\\\"\\\\\\\\\\\"\" }",
        vec![
            JsonField::new("<>&#! ", Value::int(0)),
            JsonField::new("Invalid", Value::int(0)),
        ],
    );
    let marshaler_struct = JsonStruct::new(
        "struct { NonPtr json.strMarshaler; Ptr json.strPtrMarshaler }",
        vec![
            JsonField::new(
                "NonPtr",
                Value::object(StrMarshaler("\"<str>\"", "json.strMarshaler")),
            ),
            JsonField::new(
                "Ptr",
                Value::object(StrMarshaler("\"<str>\"", "json.strPtrMarshaler")),
            ),
        ],
    );
    let string_option = JsonStruct::new(
        "struct { Bar string \"json:\\\"bar,string\\\"\" }",
        vec![JsonField::new("bar", Value::from("<html>foobar</html>")).string()],
    );
    let tests: Vec<(Value, &str, &str)> = vec![
        (Value::from("<&>"), r#""\u003c\u0026\u003e""#, r#""<&>""#),
        (
            Value::object(tag_struct),
            r#"{"\u003c\u003e\u0026#! ":0,"Invalid":0}"#,
            r#"{"<>&#! ":0,"Invalid":0}"#,
        ),
        (
            Value::object(marshaler_struct),
            r#"{"NonPtr":"\u003cstr\u003e","Ptr":"\u003cstr\u003e"}"#,
            r#"{"NonPtr":"<str>","Ptr":"<str>"}"#,
        ),
        (
            Value::object(string_option),
            r#"{"bar":"\"\\u003chtml\\u003efoobar\\u003c/html\\u003e\""}"#,
            r#"{"bar":"\"<html>foobar</html>\""}"#,
        ),
    ];
    for (v, want_escape, want) in tests {
        let mut enc = Encoder::new(Vec::new());
        enc.encode(&v).unwrap();
        assert_eq!(
            String::from_utf8(enc.into_inner()).unwrap().trim(),
            want_escape
        );
        let mut enc = Encoder::new(Vec::new());
        enc.set_escape_html(false);
        enc.encode(&v).unwrap();
        assert_eq!(String::from_utf8(enc.into_inner()).unwrap().trim(), want);
    }
}

// Go: v2_encode_test.go:TestHTMLEscape
#[test]
fn html_escape() {
    let m = b"{\"M\":\"<html>foo &\xe2\x80\xa8 \xe2\x80\xa9</html>\"}";
    let want = r#"{"M":"\u003chtml\u003efoo \u0026\u2028 \u2029\u003c/html\u003e"}"#;
    let mut b = Vec::new();
    go_json::html_escape(&mut b, m);
    assert_eq!(b, want.as_bytes());
}

// Go: v2_encode_test.go:TestEncodeString
#[test]
fn encode_string() {
    for c in 0u8..0x20 {
        let want = match c {
            0x08 => r#""\b""#.to_string(),
            0x09 => r#""\t""#.to_string(),
            0x0a => r#""\n""#.to_string(),
            0x0c => r#""\f""#.to_string(),
            0x0d => r#""\r""#.to_string(),
            _ => format!("\"\\u00{:02x}\"", c),
        };
        let b = go_json::marshal(&Value::String(GoString::from(vec![c]))).unwrap();
        assert_eq!(String::from_utf8(b).unwrap(), want);
    }
}

/// Expected results for TestDecodeInStream.
enum Want {
    Tok(Token),
    Decode(Value),
    /// A syntax error from Token.
    Err(&'static str, i64),
    /// A syntax error from Decode.
    DecodeErr(&'static str, i64),
}

fn delim(c: u8) -> Want {
    Want::Tok(Token::Delim(c))
}

fn tok_f(f: f64) -> Want {
    Want::Tok(Token::Value(Value::float64(f)))
}

fn tok_s(s: &str) -> Want {
    Want::Tok(Token::Value(Value::from(s)))
}

fn obj_a(f: f64) -> Value {
    let mut m = Map::new(MapType::StringAny);
    m.insert("a", Value::float64(f));
    Value::map(m)
}

// Go: v2_stream_test.go:TestDecodeInStream
#[test]
fn decode_in_stream() {
    let long = "a".repeat(513);
    let long_json = format!("{{ \"{}\" 1 }}", long);
    let tests: Vec<(String, Vec<Want>)> = vec![
        // streaming token cases
        ("10".into(), vec![tok_f(10.0)]),
        (" [10] ".into(), vec![delim(b'['), tok_f(10.0), delim(b']')]),
        (
            r#" [false,10,"b"] "#.into(),
            vec![
                delim(b'['),
                Want::Tok(Token::Value(Value::Bool(false))),
                tok_f(10.0),
                tok_s("b"),
                delim(b']'),
            ],
        ),
        (
            r#"{ "a": 1 }"#.into(),
            vec![delim(b'{'), tok_s("a"), tok_f(1.0), delim(b'}')],
        ),
        (
            r#"{"a": 1, "b":"3"}"#.into(),
            vec![
                delim(b'{'),
                tok_s("a"),
                tok_f(1.0),
                tok_s("b"),
                tok_s("3"),
                delim(b'}'),
            ],
        ),
        (
            r#" [{"a": 1},{"a": 2}] "#.into(),
            vec![
                delim(b'['),
                delim(b'{'),
                tok_s("a"),
                tok_f(1.0),
                delim(b'}'),
                delim(b'{'),
                tok_s("a"),
                tok_f(2.0),
                delim(b'}'),
                delim(b']'),
            ],
        ),
        (
            r#"{"obj": {"a": 1}}"#.into(),
            vec![
                delim(b'{'),
                tok_s("obj"),
                delim(b'{'),
                tok_s("a"),
                tok_f(1.0),
                delim(b'}'),
                delim(b'}'),
            ],
        ),
        (
            r#"{"obj": [{"a": 1}]}"#.into(),
            vec![
                delim(b'{'),
                tok_s("obj"),
                delim(b'['),
                delim(b'{'),
                tok_s("a"),
                tok_f(1.0),
                delim(b'}'),
                delim(b']'),
                delim(b'}'),
            ],
        ),
        // streaming tokens with intermittent Decode()
        (
            r#"{ "a": 1 }"#.into(),
            vec![
                delim(b'{'),
                tok_s("a"),
                Want::Decode(Value::float64(1.0)),
                delim(b'}'),
            ],
        ),
        (
            r#" [ { "a" : 1 } ] "#.into(),
            vec![delim(b'['), Want::Decode(obj_a(1.0)), delim(b']')],
        ),
        (
            r#" [{"a": 1},{"a": 2}] "#.into(),
            vec![
                delim(b'['),
                Want::Decode(obj_a(1.0)),
                Want::Decode(obj_a(2.0)),
                delim(b']'),
            ],
        ),
        (
            r#"{ "obj" : [ { "a" : 1 } ] }"#.into(),
            vec![
                delim(b'{'),
                tok_s("obj"),
                delim(b'['),
                Want::Decode(obj_a(1.0)),
                delim(b']'),
                delim(b'}'),
            ],
        ),
        (
            r#"{"obj": {"a": 1}}"#.into(),
            vec![
                delim(b'{'),
                tok_s("obj"),
                Want::Decode(obj_a(1.0)),
                delim(b'}'),
            ],
        ),
        (
            r#"{"obj": [{"a": 1}]}"#.into(),
            vec![
                delim(b'{'),
                tok_s("obj"),
                Want::Decode(Value::any_list(vec![obj_a(1.0)])),
                delim(b'}'),
            ],
        ),
        (
            r#" [{"a": 1} {"a": 2}] "#.into(),
            vec![
                delim(b'['),
                Want::Decode(obj_a(1.0)),
                Want::DecodeErr(
                    "invalid character '{' after array element",
                    r#" [{"a": 1} {"#.len() as i64,
                ),
            ],
        ),
        (
            long_json.clone(),
            vec![
                delim(b'{'),
                tok_s(&long),
                Want::DecodeErr(
                    "invalid character '1' after object key",
                    (r#"{ ""#.len() + 513 + r#"" 1"#.len()) as i64,
                ),
            ],
        ),
        (
            r#"{ "\a" }"#.into(),
            vec![
                delim(b'{'),
                Want::Err(
                    "invalid escape sequence `\\a` in string",
                    r#"{ "\a"#.len() as i64,
                ),
            ],
        ),
        (
            r#" \a"#.into(),
            vec![Want::Err(
                "invalid character '\\\\' looking for beginning of value",
                r#" \"#.len() as i64,
            )],
        ),
        (
            ",".into(),
            vec![Want::Err(
                "invalid character ',' looking for beginning of value",
                1,
            )],
        ),
    ];
    for (json, exp) in tests {
        let mut dec = Decoder::new(json.as_bytes());
        for (i, want) in exp.iter().enumerate() {
            let want_more = !matches!(want, Want::Tok(Token::Delim(b']' | b'}')));
            assert_eq!(dec.more(), want_more, "{json}: More() before token {i}");
            let got = match want {
                Want::Decode(_) | Want::DecodeErr(..) => dec.decode().map(Token::Value),
                _ => dec.token(),
            };
            match want {
                Want::Err(msg, off) | Want::DecodeErr(msg, off) => {
                    let err = got.unwrap_err();
                    assert_eq!(
                        err,
                        go_json::Error::Syntax(go_json::SyntaxError {
                            msg: msg.to_string(),
                            offset: *off
                        }),
                        "{json}"
                    );
                    break;
                }
                Want::Tok(t) => assert_eq!(&got.unwrap(), t, "{json}: token {i}"),
                Want::Decode(v) => {
                    assert_eq!(got.unwrap(), Token::Value(v.clone()), "{json}: token {i}")
                }
            }
        }
    }
}
