//! Ports of Go's own tests for the leaf files of html/template:
//! html_test.go, js_test.go, css_test.go, url_test.go, transition_test.go
//! (go1.24 fork, tpl/internal/go_templates/htmltemplate).
//!
//! Go test values the value model cannot write literally are mapped:
//! `struct{ X, Y int }{1, 2}` → a `Kind::Struct` object with those
//! `struct_fields`; `&jsonErrType{}` → a `Kind::Ptr` object whose
//! `marshal_json` fails.

use std::borrow::Cow;
use std::sync::Arc;

use go_value::{FloatKind, GoString, HostCtx, IntKind, Kind, Object, UintKind, Value};

use super::super::content::stringify;
use super::super::context::JsCtx;
use super::super::css::{
    css_escaper, css_value_filter, decode_css, ends_with_css_keyword, hex_decode, is_css_nmchar,
    skip_css_space,
};
use super::super::html::{html_nospace_escaper, strip_tags};
use super::super::js::{
    is_js_type, js_regexp_escaper, js_str_escaper, js_val_escaper, next_js_ctx,
};
use super::super::transition::index_tag_end;
use super::super::url::{srcset_filter_and_escaper, url_escaper, url_normalizer};

fn s(v: &str) -> Vec<Value> {
    vec![Value::string(v)]
}

fn sb(v: &[u8]) -> Vec<Value> {
    vec![Value::String(GoString::from(v))]
}

fn q(b: &[u8]) -> String {
    go_strconv::quote(b)
}

const LOWER7_INPUT: &str = concat!(
    "\x00\x01\x02\x03\x04\x05\x06\x07\x08\t\n\x0b\x0c\r\x0e\x0f",
    "\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f",
    r##" !"#$%&'()*+,-./"##,
    r"0123456789:;<=>?",
    r"@ABCDEFGHIJKLMNO",
    r"PQRSTUVWXYZ[\]^_",
    "`abcdefghijklmno",
    "pqrstuvwxyz{|}~\x7f",
    "\u{00A0}\u{0100}\u{2028}\u{2029}\u{feff}\u{1D11E}"
);

// ---------------------------------------------------------------------------
// html_test.go

// Go: html_test.go:TestHTMLNospaceEscaper
#[test]
fn test_html_nospace_escaper() {
    let mut input: Vec<u8> = Vec::new();
    input.extend_from_slice(b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\t\n\x0b\x0c\r\x0e\x0f");
    input.extend_from_slice(b"\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f");
    input.extend_from_slice(br##" !"#$%&'()*+,-./"##);
    input.extend_from_slice(b"0123456789:;<=>?");
    input.extend_from_slice(b"@ABCDEFGHIJKLMNO");
    input.extend_from_slice(br"PQRSTUVWXYZ[\]^_");
    input.extend_from_slice(b"`abcdefghijklmno");
    input.extend_from_slice(b"pqrstuvwxyz{|}~\x7f");
    input.extend_from_slice("\u{00A0}\u{0100}\u{2028}\u{2029}\u{feff}\u{fdec}\u{1D11E}".as_bytes());
    input.extend_from_slice(b"erroneous\x960"); // keep at the end

    let mut want: Vec<u8> = Vec::new();
    want.extend_from_slice(b"&#xfffd;\x01\x02\x03\x04\x05\x06\x07");
    want.extend_from_slice(b"\x08&#9;&#10;&#11;&#12;&#13;\x0E\x0F");
    want.extend_from_slice(b"\x10\x11\x12\x13\x14\x15\x16\x17");
    want.extend_from_slice(b"\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f");
    want.extend_from_slice(br"&#32;!&#34;#$%&amp;&#39;()*&#43;,-./");
    want.extend_from_slice(br"0123456789:;&lt;&#61;&gt;?");
    want.extend_from_slice(b"@ABCDEFGHIJKLMNO");
    want.extend_from_slice(br"PQRSTUVWXYZ[\]^_");
    want.extend_from_slice(b"&#96;abcdefghijklmno");
    want.extend_from_slice(b"pqrstuvwxyz{|}~\x7f");
    want.extend_from_slice("\u{00A0}\u{0100}\u{2028}\u{2029}\u{feff}&#xfdec;\u{1D11E}".as_bytes());
    want.extend_from_slice(b"erroneous&#xfffd;0"); // keep at the end

    let got = html_nospace_escaper(&sb(&input));
    assert_eq!(
        got,
        want,
        "encode: want\n\t{}\nbut got\n\t{}",
        q(&want),
        q(&got)
    );

    // r := strings.NewReplacer("\x00", "\ufffd", "\x96", "\ufffd")
    let mut replaced = Vec::new();
    for &c in &input {
        if c == 0 || c == 0x96 {
            replaced.extend_from_slice("\u{fffd}".as_bytes());
        } else {
            replaced.push(c);
        }
    }
    let decoded = go_html::unescape_string_bytes(&got);
    assert_eq!(decoded, replaced, "decode");
}

// Go: html_test.go:TestStripTags
#[test]
fn test_strip_tags() {
    let tests: &[(&str, &str)] = &[
        ("", ""),
        ("Hello, World!", "Hello, World!"),
        ("foo&amp;bar", "foo&amp;bar"),
        (
            r#"Hello <a href="www.example.com/">World</a>!"#,
            "Hello World!",
        ),
        ("Foo <textarea>Bar</textarea> Baz", "Foo Bar Baz"),
        ("Foo <!-- Bar --> Baz", "Foo  Baz"),
        ("<", "<"),
        ("foo < bar", "foo < bar"),
        (
            r#"Foo<script type="text/javascript">alert(1337)</script>Bar"#,
            "FooBar",
        ),
        (r#"Foo<div title="1>2">Bar"#, "FooBar"),
        (r"I <3 Ponies!", r"I <3 Ponies!"),
        (r"<script>foo()</script>", r""),
    ];
    for (input, want) in tests {
        let got = strip_tags(input.as_bytes());
        assert_eq!(got, want.as_bytes(), "{input:?}");
    }
}

// ---------------------------------------------------------------------------
// js_test.go

// Go: js_test.go:TestNextJsCtx
#[test]
fn test_next_js_ctx() {
    use JsCtx::*;
    let tests: &[(JsCtx, &str)] = &[
        // Statement terminators precede regexps.
        (Regexp, ";"),
        // This is not airtight.
        (Regexp, "}"),
        // But member, call, grouping, and array expression terminators
        // precede div ops.
        (DivOp, ")"),
        (DivOp, "]"),
        // At the start of a primary expression, array, or expression
        // statement, expect a regexp.
        (Regexp, "("),
        (Regexp, "["),
        (Regexp, "{"),
        // Assignment operators precede regexps as do all exclusively
        // prefix and binary operators.
        (Regexp, "="),
        (Regexp, "+="),
        (Regexp, "*="),
        (Regexp, "*"),
        (Regexp, "!"),
        // Whether the + or - is infix or prefix, it cannot precede a
        // div op.
        (Regexp, "+"),
        (Regexp, "-"),
        // An incr/decr op precedes a div operator.
        (DivOp, "--"),
        (DivOp, "++"),
        (DivOp, "x--"),
        // When we have many dashes or pluses, then they are grouped
        // left to right.
        (Regexp, "x---"), // A postfix -- then a -.
        // return followed by a slash returns the regexp literal or the
        // slash starts a regexp literal in an expression statement that
        // is dead code.
        (Regexp, "return"),
        (Regexp, "return "),
        (Regexp, "return\t"),
        (Regexp, "return\n"),
        (Regexp, "return\u{2028}"),
        // Identifiers can be divided and cannot validly be preceded by
        // a regular expressions.
        (DivOp, "x"),
        (DivOp, "x "),
        (DivOp, "x\t"),
        (DivOp, "x\n"),
        (DivOp, "x\u{2028}"),
        (DivOp, "preturn"),
        // Numbers precede div ops.
        (DivOp, "0"),
        // Dots that are part of a number are div preceders.
        (DivOp, "0."),
        // Some JS interpreters treat NBSP as a normal space, so
        // we must too in order to properly escape things.
        (Regexp, "=\u{00A0}"),
    ];
    for (want, input) in tests {
        assert_eq!(next_js_ctx(input.as_bytes(), Regexp), *want, "{input:?}");
        assert_eq!(next_js_ctx(input.as_bytes(), DivOp), *want, "{input:?}");
    }
    assert_eq!(next_js_ctx(b"   ", Regexp), Regexp, "Blank tokens");
    assert_eq!(next_js_ctx(b"   ", DivOp), DivOp, "Blank tokens");
}

/// Go: `struct{ X, Y int }{1, 2}`.
struct XyStruct;

impl Object for XyStruct {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("struct { X int; Y int }")
    }
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn has_method(&self, _name: &str) -> bool {
        false
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        _name: &str,
        _args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        None
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(vec![
            (Cow::Borrowed("X"), Value::int(1)),
            (Cow::Borrowed("Y"), Value::int(2)),
        ])
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Go: `*template.jsonErrType`, whose MarshalJSON fails.
struct JsonErrType;

impl Object for JsonErrType {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("*template.jsonErrType")
    }
    fn has_method(&self, _name: &str) -> bool {
        false
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        _name: &str,
        _args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        None
    }
    fn marshal_json(&self) -> Option<go_value::Result<Vec<u8>>> {
        Some(Err(go_value::Error::new(
            "a */ b <script c </script d <!-- e <sCrIpT f </sCrIpT",
        )))
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(vec![])
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// Go: js_test.go:TestJSValEscaper
#[test]
fn test_js_val_escaper() {
    let f32v = |f: f32| Value::Float(go_strconv::internal::f32_to_f64(f), FloatKind::F32);
    let tests: Vec<(Value, &str, bool)> = vec![
        (Value::int(42), " 42 ", false),
        (Value::Uint(42, UintKind::Uint), " 42 ", false),
        (Value::Int(42, IntKind::Int16), " 42 ", false),
        (Value::Uint(42, UintKind::Uint16), " 42 ", false),
        (Value::Int(-42, IntKind::Int32), " -42 ", false),
        (Value::Uint(42, UintKind::Uint32), " 42 ", false),
        (Value::Int(-42, IntKind::Int16), " -42 ", false),
        (Value::Uint(42, UintKind::Uint16), " 42 ", false),
        (Value::Int(-42, IntKind::Int64), " -42 ", false),
        (Value::Uint(42, UintKind::Uint64), " 42 ", false),
        (
            Value::Uint(1 << 53, UintKind::Uint64),
            " 9007199254740992 ",
            false,
        ),
        // ulp(1 << 53) > 1 so this loses precision in JS
        // but it is still a representable integer literal.
        (
            Value::Uint((1 << 53) + 1, UintKind::Uint64),
            " 9007199254740993 ",
            false,
        ),
        (f32v(1.0), " 1 ", false),
        (f32v(-1.0), " -1 ", false),
        (f32v(0.5), " 0.5 ", false),
        (f32v(-0.5), " -0.5 ", false),
        (f32v(1.0 / 256.0), " 0.00390625 ", false),
        (f32v(0.0), " 0 ", false),
        (Value::float64(-0.0), " -0 ", false),
        (Value::float64(1.0), " 1 ", false),
        (Value::float64(-1.0), " -1 ", false),
        (Value::float64(0.5), " 0.5 ", false),
        (Value::float64(-0.5), " -0.5 ", false),
        (Value::float64(0.0), " 0 ", false),
        (Value::float64(-0.0), " -0 ", false),
        (Value::string(""), r#""""#, false),
        (Value::string("foo"), r#""foo""#, false),
        // Newlines.
        (
            Value::string("\r\n\u{2028}\u{2029}"),
            r#""\r\n\u2028\u2029""#,
            false,
        ),
        // "\v" == "v" on IE 6 so use "\u000b" instead.
        (Value::string("\t\x0b"), r#""\t\u000b""#, false),
        (Value::Object(Arc::new(XyStruct)), r#"{"X":1,"Y":2}"#, false),
        (Value::any_list(vec![]), "[]", false),
        (
            Value::any_list(vec![Value::int(42), Value::string("foo"), Value::Invalid]),
            r#"[42,"foo",null]"#,
            false,
        ),
        (
            Value::string_list(["<!--", "</script>", "-->"]),
            r#"["\u003c!--","\u003c/script\u003e","--\u003e"]"#,
            false,
        ),
        (Value::string("<!--"), r#""\u003c!--""#, false),
        (Value::string("-->"), r#""--\u003e""#, false),
        (Value::string("<![CDATA["), r#""\u003c![CDATA[""#, false),
        (Value::string("]]>"), r#""]]\u003e""#, false),
        (Value::string("</script"), r#""\u003c/script""#, false),
        (Value::string("\u{1D11E}"), "\"\u{1D11E}\"", false), // or "\uD834\uDD1E"
        (Value::Invalid, " null ", false),
        (
            Value::Object(Arc::new(JsonErrType)),
            " /* json: error calling MarshalJSON for type *template.jsonErrType: a * / b \\x3Cscript c \\x3C/script d \\x3C!-- e \\x3Cscript f \\x3C/script */null ",
            true,
        ),
    ];

    for (x, js, skip_nest) in tests {
        let got = js_val_escaper(std::slice::from_ref(&x));
        assert_eq!(
            got,
            js.as_bytes(),
            "{x:?}: want\n\t{}\ngot\n\t{}",
            q(js.as_bytes()),
            q(&got)
        );
        if skip_nest {
            continue;
        }
        // Make sure that escaping corner cases are not broken
        // by nesting.
        let a = Value::any_list(vec![x.clone()]);
        let want = format!("[{}]", js.trim());
        let got = js_val_escaper(&[a]);
        assert_eq!(got, want.as_bytes(), "[{x:?}]: got {}", q(&got));
    }
}

// Go: js_test.go:TestJSStrEscaper
#[test]
fn test_js_str_escaper() {
    let tests: &[(&[u8], &[u8])] = &[
        (b"", b""),
        (b"foo", b"foo"),
        (b"\x00", br"\u0000"),
        (b"\t", br"\t"),
        (b"\n", br"\n"),
        (b"\r", br"\r"),
        ("\u{2028}".as_bytes(), br"\u2028"),
        ("\u{2029}".as_bytes(), br"\u2029"),
        (b"\\", br"\\"),
        (b"\\n", br"\\n"),
        (b"foo\r\nbar", br"foo\r\nbar"),
        // Preserve attribute boundaries.
        (b"\"", br"\u0022"),
        (b"'", br"\u0027"),
        // Allow embedding in HTML without further escaping.
        (b"&amp;", br"\u0026amp;"),
        // Prevent breaking out of text node and element boundaries.
        (b"</script>", br"\u003c\/script\u003e"),
        (b"<![CDATA[", br"\u003c![CDATA["),
        (b"]]>", br"]]\u003e"),
        // Escaping text spans.
        (b"<!--", br"\u003c!--"),
        (b"-->", br"--\u003e"),
        // From https://code.google.com/p/doctype/wiki/ArticleUtf7
        (
            b"+ADw-script+AD4-alert(1)+ADw-/script+AD4-",
            br"\u002bADw-script\u002bAD4-alert(1)\u002bADw-\/script\u002bAD4-",
        ),
        // Invalid UTF-8 sequence
        (b"foo\xA0bar", b"foo\xA0bar"),
        // Invalid unicode scalar value.
        (b"foo\xed\xa0\x80bar", b"foo\xed\xa0\x80bar"),
    ];
    for (x, esc) in tests {
        let got = js_str_escaper(&sb(x));
        assert_eq!(got, *esc, "{}: got {}", q(x), q(&got));
    }
}

// Go: js_test.go:TestJSRegexpEscaper
#[test]
fn test_js_regexp_escaper() {
    let tests: &[(&[u8], &[u8])] = &[
        (b"", br"(?:)"),
        (b"foo", br"foo"),
        (b"\x00", br"\u0000"),
        (b"\t", br"\t"),
        (b"\n", br"\n"),
        (b"\r", br"\r"),
        ("\u{2028}".as_bytes(), br"\u2028"),
        ("\u{2029}".as_bytes(), br"\u2029"),
        (b"\\", br"\\"),
        (b"\\n", br"\\n"),
        (b"foo\r\nbar", br"foo\r\nbar"),
        // Preserve attribute boundaries.
        (b"\"", br"\u0022"),
        (b"'", br"\u0027"),
        // Allow embedding in HTML without further escaping.
        (b"&amp;", br"\u0026amp;"),
        // Prevent breaking out of text node and element boundaries.
        (b"</script>", br"\u003c\/script\u003e"),
        (b"<![CDATA[", br"\u003c!\[CDATA\["),
        (b"]]>", br"\]\]\u003e"),
        // Escaping text spans.
        (b"<!--", br"\u003c!\-\-"),
        (b"-->", br"\-\-\u003e"),
        (b"*", br"\*"),
        (b"+", br"\u002b"),
        (b"?", br"\?"),
        (b"[](){}", br"\[\]\(\)\{\}"),
        (b"$foo|x.y", br"\$foo\|x\.y"),
        (b"x^y", br"x\^y"),
    ];
    for (x, esc) in tests {
        let got = js_regexp_escaper(&sb(x));
        assert_eq!(got, *esc, "{}: got {}", q(x), q(&got));
    }
}

// Go: js_test.go:TestEscapersOnLower7AndSelectHighCodepoints
#[test]
fn test_escapers_on_lower7_and_select_high_codepoints() {
    type Esc = fn(&[Value]) -> Vec<u8>;
    let tests: Vec<(&str, Esc, String)> = vec![
        (
            "jsStrEscaper",
            js_str_escaper,
            [
                r"\u0000\u0001\u0002\u0003\u0004\u0005\u0006\u0007",
                r"\u0008\t\n\u000b\f\r\u000e\u000f",
                r"\u0010\u0011\u0012\u0013\u0014\u0015\u0016\u0017",
                r"\u0018\u0019\u001a\u001b\u001c\u001d\u001e\u001f",
                r" !\u0022#$%\u0026\u0027()*\u002b,-.\/",
                r"0123456789:;\u003c=\u003e?",
                r"@ABCDEFGHIJKLMNO",
                r"PQRSTUVWXYZ[\\]^_",
                "\\u0060abcdefghijklmno",
                "pqrstuvwxyz{|}~\u{007f}",
                "\u{00A0}\u{0100}\\u2028\\u2029\u{feff}\u{1D11E}",
            ]
            .concat(),
        ),
        (
            "jsRegexpEscaper",
            js_regexp_escaper,
            [
                r"\u0000\u0001\u0002\u0003\u0004\u0005\u0006\u0007",
                r"\u0008\t\n\u000b\f\r\u000e\u000f",
                r"\u0010\u0011\u0012\u0013\u0014\u0015\u0016\u0017",
                r"\u0018\u0019\u001a\u001b\u001c\u001d\u001e\u001f",
                r" !\u0022#\$%\u0026\u0027\(\)\*\u002b,\-\.\/",
                r"0123456789:;\u003c=\u003e\?",
                r"@ABCDEFGHIJKLMNO",
                r"PQRSTUVWXYZ\[\\\]\^_",
                "`abcdefghijklmno",
                r"pqrstuvwxyz\{\|\}~",
                "\u{007f}",
                "\u{00A0}\u{0100}\\u2028\\u2029\u{feff}\u{1D11E}",
            ]
            .concat(),
        ),
    ];

    for (name, escaper, escaped) in tests {
        let got = escaper(&s(LOWER7_INPUT));
        assert_eq!(
            got,
            escaped.as_bytes(),
            "{name} once: want\n\t{}\ngot\n\t{}",
            q(escaped.as_bytes()),
            q(&got)
        );
        // Escape it rune by rune to make sure that any
        // fast-path checking does not break escaping.
        let mut buf = Vec::new();
        for c in LOWER7_INPUT.chars() {
            buf.extend_from_slice(&escaper(&s(&c.to_string())));
        }
        assert_eq!(buf, escaped.as_bytes(), "{name} rune-wise");
    }
}

// Go: js_test.go:TestIsJsMimeType
#[test]
fn test_is_js_mime_type() {
    let tests: &[(&str, bool)] = &[
        ("application/javascript;version=1.8", true),
        ("application/javascript;version=1.8;foo=bar", true),
        ("application/javascript/version=1.8", false),
        ("text/javascript", true),
        ("application/json", true),
        ("application/ld+json", true),
        ("module", true),
        // go1.24 fork: the empty type is not JS (newer Go says it is).
        ("", false),
    ];
    for (input, out) in tests {
        assert_eq!(is_js_type(input.as_bytes()), *out, "isJSType({input:?})");
    }
}

// ---------------------------------------------------------------------------
// css_test.go

// Go: css_test.go:TestEndsWithCSSKeyword
#[test]
fn test_ends_with_css_keyword() {
    let tests: &[(&str, &str, bool)] = &[
        ("", "url", false),
        ("url", "url", true),
        ("URL", "url", true),
        ("Url", "url", true),
        ("url", "important", false),
        ("important", "important", true),
        ("image-url", "url", false),
        ("imageurl", "url", false),
        ("image url", "url", true),
    ];
    for (css, kw, want) in tests {
        assert_eq!(
            ends_with_css_keyword(css.as_bytes(), kw.as_bytes()),
            *want,
            "css={css}, kw={kw}"
        );
    }
}

// Go: css_test.go:TestIsCSSNmchar
#[test]
fn test_is_css_nmchar() {
    let tests: &[(i32, bool)] = &[
        (0, false),
        ('0' as i32, true),
        ('9' as i32, true),
        ('A' as i32, true),
        ('Z' as i32, true),
        ('a' as i32, true),
        ('z' as i32, true),
        ('_' as i32, true),
        ('-' as i32, true),
        (':' as i32, false),
        (';' as i32, false),
        (' ' as i32, false),
        (0x7f, false),
        (0x80, true),
        (0x1234, true),
        (0xd800, false),
        (0xdc00, false),
        (0xfffe, false),
        (0x10000, true),
        (0x110000, false),
    ];
    for (r, want) in tests {
        assert_eq!(is_css_nmchar(*r), *want, "{r:#x}");
    }
}

// Go: css_test.go:TestDecodeCSS
#[test]
fn test_decode_css() {
    let tests: &[(&str, &str)] = &[
        (r"", r""),
        (r"foo", r"foo"),
        (r"foo\", r"foo"),
        (r"foo\\", r"foo\"),
        (r"\", r""),
        (r"\A", "\n"),
        (r"\a", "\n"),
        (r"\0a", "\n"),
        (r"\00000a", "\n"),
        (r"\000000a", "\u{0000}a"),
        (r"\1234 5", "\u{1234}5"),
        (r"\1234\20 5", "\u{1234} 5"),
        (r"\1234\A 5", "\u{1234}\n5"),
        ("\\1234\t5", "\u{1234}5"),
        ("\\1234\n5", "\u{1234}5"),
        ("\\1234\r\n5", "\u{1234}5"),
        (r"\12345", "\u{12345}"),
        (r"\\", r"\"),
        (r"\\ ", r"\ "),
        (r#"\""#, r#"""#),
        (r"\'", r"'"),
        (r"\.", r"."),
        (r"\. .", r". ."),
        (
            r"The \3c i\3equick\3c/i\3e,\d\A\3cspan style=\27 color:brown\27\3e brown\3c/span\3e  fox jumps\2028over the \3c canine class=\22lazy\22 \3e dog\3c/canine\3e",
            "The <i>quick</i>,\r\n<span style='color:brown'>brown</span> fox jumps\u{2028}over the <canine class=\"lazy\">dog</canine>",
        ),
    ];
    for (css, want) in tests {
        let got1 = decode_css(css.as_bytes());
        assert_eq!(got1, want.as_bytes(), "{css:?}: got {}", q(&got1));
        let recoded = css_escaper(&sb(&got1));
        let got2 = decode_css(&recoded);
        assert_eq!(
            got2,
            want.as_bytes(),
            "{css:?}: escape & decode not dual for {}",
            q(&recoded)
        );
    }
}

// Go: css_test.go:TestHexDecode
#[test]
fn test_hex_decode() {
    let mut i: i64 = 0;
    while i < 0x200000 {
        // 101: coprime with 16
        let s = go_strconv::format_int(i, 16);
        assert_eq!(hex_decode(s.as_bytes()) as i64, i, "{s}");
        let s = s.to_uppercase();
        assert_eq!(hex_decode(s.as_bytes()) as i64, i, "{s}");
        i += 101;
    }
}

// Go: css_test.go:TestSkipCSSSpace
#[test]
fn test_skip_css_space() {
    let tests: &[(&str, &str)] = &[
        ("", ""),
        ("foo", "foo"),
        ("\n", ""),
        ("\r\n", ""),
        ("\r", ""),
        ("\t", ""),
        (" ", ""),
        ("\x0c", ""),
        (" foo", "foo"),
        ("  foo", " foo"),
        (r"\20", r"\20"),
    ];
    for (css, want) in tests {
        assert_eq!(skip_css_space(css.as_bytes()), want.as_bytes(), "{css:?}");
    }
}

// Go: css_test.go:TestCSSEscaper
#[test]
fn test_css_escaper() {
    let want = [
        "\\0\x01\x02\x03\x04\x05\x06\x07",
        "\x08\\9 \\a\x0b\\c \\d\x0E\x0F",
        "\x10\x11\x12\x13\x14\x15\x16\x17",
        "\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f",
        r" !\22#$%\26\27\28\29*\2b,-.\2f ",
        r"0123456789\3a\3b\3c=\3e?",
        r"@ABCDEFGHIJKLMNO",
        r"PQRSTUVWXYZ[\\]^_",
        "`abcdefghijklmno",
        r"pqrstuvwxyz\7b|\7d~",
        "\u{007f}",
        "\u{00A0}\u{0100}\u{2028}\u{2029}\u{feff}\u{1D11E}",
    ]
    .concat();

    let got = css_escaper(&s(LOWER7_INPUT));
    assert_eq!(
        got,
        want.as_bytes(),
        "encode: want\n\t{}\nbut got\n\t{}",
        q(want.as_bytes()),
        q(&got)
    );
    let got = decode_css(&got);
    assert_eq!(got, LOWER7_INPUT.as_bytes(), "decode");
}

// Go: css_test.go:TestCSSValueFilter
#[test]
fn test_css_value_filter() {
    let tests: &[(&[u8], &[u8])] = &[
        (b"", b""),
        (b"foo", b"foo"),
        (b"0", b"0"),
        (b"0px", b"0px"),
        (b"-5px", b"-5px"),
        (b"1.25in", b"1.25in"),
        (b"+.33em", b"+.33em"),
        (b"100%", b"100%"),
        (b"12.5%", b"12.5%"),
        (b".foo", b".foo"),
        (b"#bar", b"#bar"),
        (b"corner-radius", b"corner-radius"),
        (b"-moz-corner-radius", b"-moz-corner-radius"),
        (b"#000", b"#000"),
        (b"#48f", b"#48f"),
        (b"#123456", b"#123456"),
        (b"U+00-FF, U+980-9FF", b"U+00-FF, U+980-9FF"),
        (b"color: red", b"color: red"),
        (b"<!--", b"ZgotmplZ"),
        (b"-->", b"ZgotmplZ"),
        (b"<![CDATA[", b"ZgotmplZ"),
        (b"]]>", b"ZgotmplZ"),
        (b"</style", b"ZgotmplZ"),
        (b"\"", b"ZgotmplZ"),
        (b"'", b"ZgotmplZ"),
        (b"`", b"ZgotmplZ"),
        (b"\x00", b"ZgotmplZ"),
        (b"/* foo */", b"ZgotmplZ"),
        (b"//", b"ZgotmplZ"),
        (b"[href=~", b"ZgotmplZ"),
        (b"expression(alert(1337))", b"ZgotmplZ"),
        (b"-expression(alert(1337))", b"ZgotmplZ"),
        (b"expression", b"ZgotmplZ"),
        (b"Expression", b"ZgotmplZ"),
        (b"EXPRESSION", b"ZgotmplZ"),
        (b"-moz-binding", b"ZgotmplZ"),
        (b"-expr\x00ession(alert(1337))", b"ZgotmplZ"),
        (br"-expr\0ession(alert(1337))", b"ZgotmplZ"),
        (br"-express\69on(alert(1337))", b"ZgotmplZ"),
        (br"-express\69 on(alert(1337))", b"ZgotmplZ"),
        (br"-exp\72 ession(alert(1337))", b"ZgotmplZ"),
        (br"-exp\52 ession(alert(1337))", b"ZgotmplZ"),
        (br"-exp\000052 ession(alert(1337))", b"ZgotmplZ"),
        (br"-expre\0000073sion", b"-expre\x073sion"),
        (br"@import url evil.css", b"ZgotmplZ"),
        (b"<", b"ZgotmplZ"),
        (b">", b"ZgotmplZ"),
    ];
    for (css, want) in tests {
        let got = css_value_filter(&sb(css));
        assert_eq!(got, *want, "{}: got {}", q(css), q(&got));
    }
}

// ---------------------------------------------------------------------------
// url_test.go

// Go: url_test.go:TestURLNormalizer
#[test]
fn test_url_normalizer() {
    let tests: &[(&str, &str)] = &[
        ("", ""),
        (
            "http://example.com:80/foo/bar?q=foo%20&bar=x+y#frag",
            "http://example.com:80/foo/bar?q=foo%20&bar=x+y#frag",
        ),
        (" ", "%20"),
        ("%7c", "%7c"),
        ("%7C", "%7C"),
        ("%2", "%252"),
        ("%", "%25"),
        ("%z", "%25z"),
        ("/foo|bar/%5c\u{1234}", "/foo%7cbar/%5c%e1%88%b4"),
    ];
    for (url, want) in tests {
        let got = url_normalizer(&s(url));
        assert_eq!(got, want.as_bytes(), "{url:?}");
        assert_eq!(
            url_normalizer(&s(want)),
            want.as_bytes(),
            "not idempotent: {want:?}"
        );
    }
}

// Go: url_test.go:TestURLFilters
#[test]
fn test_url_filters() {
    type Esc = fn(&[Value]) -> Vec<u8>;
    let tests: Vec<(&str, Esc, String)> = vec![
        (
            "urlEscaper",
            url_escaper,
            [
                "%00%01%02%03%04%05%06%07%08%09%0a%0b%0c%0d%0e%0f",
                "%10%11%12%13%14%15%16%17%18%19%1a%1b%1c%1d%1e%1f",
                "%20%21%22%23%24%25%26%27%28%29%2a%2b%2c-.%2f",
                "0123456789%3a%3b%3c%3d%3e%3f",
                "%40ABCDEFGHIJKLMNO",
                "PQRSTUVWXYZ%5b%5c%5d%5e_",
                "%60abcdefghijklmno",
                "pqrstuvwxyz%7b%7c%7d~%7f",
                "%c2%a0%c4%80%e2%80%a8%e2%80%a9%ef%bb%bf%f0%9d%84%9e",
            ]
            .concat(),
        ),
        (
            "urlNormalizer",
            url_normalizer,
            [
                "%00%01%02%03%04%05%06%07%08%09%0a%0b%0c%0d%0e%0f",
                "%10%11%12%13%14%15%16%17%18%19%1a%1b%1c%1d%1e%1f",
                "%20!%22#$%25&%27%28%29*+,-./",
                "0123456789:;%3c=%3e?",
                "@ABCDEFGHIJKLMNO",
                "PQRSTUVWXYZ[%5c]%5e_",
                "%60abcdefghijklmno",
                "pqrstuvwxyz%7b%7c%7d~%7f",
                "%c2%a0%c4%80%e2%80%a8%e2%80%a9%ef%bb%bf%f0%9d%84%9e",
            ]
            .concat(),
        ),
    ];
    for (name, escaper, escaped) in tests {
        let got = escaper(&s(LOWER7_INPUT));
        assert_eq!(got, escaped.as_bytes(), "{name}: got {}", q(&got));
    }
}

// Go: url_test.go:TestSrcsetFilter
#[test]
fn test_srcset_filter() {
    let tests: &[(&str, &str, &str)] = &[
        (
            "one ok",
            "http://example.com/img.png",
            "http://example.com/img.png",
        ),
        ("one ok with metadata", " /img.png 200w", " /img.png 200w"),
        ("one bad", "javascript:alert(1) 200w", "#ZgotmplZ"),
        ("two ok", "foo.png, bar.png", "foo.png, bar.png"),
        (
            "left bad",
            "javascript:alert(1), /foo.png",
            "#ZgotmplZ, /foo.png",
        ),
        (
            "right bad",
            "/bogus#, javascript:alert(1)",
            "/bogus#,#ZgotmplZ",
        ),
    ];
    for (name, input, want) in tests {
        let got = srcset_filter_and_escaper(&s(input));
        assert_eq!(
            got,
            want.as_bytes(),
            "{name}: srcsetFilterAndEscaper({input:?})"
        );
    }
}

// ---------------------------------------------------------------------------
// transition_test.go

// Go: transition_test.go:TestFindEndTag
#[test]
fn test_find_end_tag() {
    let tests: &[(&str, &str, isize)] = &[
        ("", "tag", -1),
        ("hello </textarea> hello", "textarea", 6),
        ("hello </TEXTarea> hello", "textarea", 6),
        ("hello </textAREA>", "textarea", 6),
        ("hello </textarea", "textareax", -1),
        ("hello </textarea>", "tag", -1),
        ("hello tag </textarea", "tag", -1),
        ("hello </tag> </other> </textarea> <other>", "textarea", 22),
        ("</textarea> <other>", "textarea", 0),
        ("<div> </div> </TEXTAREA>", "textarea", 13),
        ("<div> </div> </TEXTAREA\t>", "textarea", 13),
        ("<div> </div> </TEXTAREA >", "textarea", 13),
        ("<div> </div> </TEXTAREAfoo", "textarea", -1),
        ("</TEXTAREAfoo </textarea>", "textarea", 14),
        ("<</script >", "script", 1),
        ("</script>", "textarea", -1),
    ];
    for (s, tag, want) in tests {
        assert_eq!(
            index_tag_end(s.as_bytes(), tag.as_bytes()),
            *want,
            "{s:?}/{tag:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Hugo's indirect override (C9) and stringify details.

#[test]
fn stringify_hugo_rules() {
    use super::super::content::ContentType;
    // Missing values print as the empty string (issue 25875).
    assert_eq!(stringify(&[Value::Invalid]), (vec![], ContentType::Plain));
    assert_eq!(
        stringify(&[Value::Invalid, Value::string("a"), Value::Invalid]),
        (b"a".to_vec(), ContentType::Plain)
    );
    // A typed nil pointer is not nil: fmt prints <nil>.
    assert_eq!(
        stringify(&[Value::TypedNil(Arc::from("*int"))]),
        (b"<nil>".to_vec(), ContentType::Plain)
    );
    // An interface-typed nil inside `any` is a nil `any`.
    assert_eq!(
        stringify(&[Value::TypedNil(Arc::from("error"))]),
        (vec![], ContentType::Plain)
    );
    assert_eq!(
        stringify(&[Value::html("<b>")]),
        (b"<b>".to_vec(), ContentType::Html)
    );
}
