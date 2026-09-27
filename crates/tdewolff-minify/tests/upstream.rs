//! Upstream `_test.go` tables of github.com/tdewolff/minify/v2@v2.23.8,
//! extracted literally by tools/go-oracle/tdewolff-minify (upstream.txt.gz:
//! input, upstream expected value, and the Go output of the same setup
//! without a JS minifier), plus the hand-ported non-table tests.
//!
//! Every row must equal the Go output; rows whose Go output equals the
//! upstream expectation (all rows that do not need the JS minifier) thereby
//! also equal the upstream expectation.

#![allow(clippy::needless_range_loop)]

mod common;

use std::sync::Arc;

use common::configs::{config, copy_func, err_plain, special_config};
use common::*;
use tdewolff_minify::{
    GoBytes, GoError, GoReader, M, Minifier, Params, Regexp, Url, Writer, css, data_uri, decimal,
    err_not_exist, html, is_err_not_exist, json, mediatype, number, svg, xml,
};

type Run = Box<dyn FnMut(i64, Option<Url>, &[u8]) -> (Vec<u8>, Option<GoError>)>;

/// `mf.Minify(m, w, bytes.NewBufferString(in), params)`
fn minifier_run(m: M, mf: Box<dyn Minifier>, params: Option<Params>) -> Run {
    Box::new(move |_, _, input| {
        let mut out = Vec::new();
        let mut r = tdewolff_parse::buffer::Reader::new(cp(input));
        let err = mf.minify(&m, &mut out, &mut r, params.as_ref()).err();
        (out, err)
    })
}

fn inline() -> Option<Params> {
    let mut p = Params::new();
    p.insert(b"inline".to_vec(), b"1".to_vec());
    Some(p)
}

fn upstream_run(name: &str) -> Run {
    match name {
        "html/TestHTML" => minifier_run(
            special_config("t-copycssjs"),
            Box::new(html::Minifier::default()),
            None,
        ),
        "html/TestHTMLCSSJS" => minifier_run(
            special_config("t-htmlcsssvg"),
            Box::new(html::Minifier::default()),
            None,
        ),
        "html/TestHTMLKeepEndTags" => minifier_run(
            M::new(),
            Box::new(html::Minifier {
                keep_end_tags: true,
                ..Default::default()
            }),
            None,
        ),
        "html/TestHTMLKeepSpecialComments" => minifier_run(
            M::new(),
            Box::new(html::Minifier {
                keep_special_comments: true,
                ..Default::default()
            }),
            None,
        ),
        "html/TestHTMLKeepWhitespace" => minifier_run(
            M::new(),
            Box::new(html::Minifier {
                keep_whitespace: true,
                ..Default::default()
            }),
            None,
        ),
        "html/TestHTMLKeepQuotes" => minifier_run(
            M::new(),
            Box::new(html::Minifier {
                keep_quotes: true,
                ..Default::default()
            }),
            None,
        ),
        "html/TestHTMLURL" => {
            let mut m = special_config("t-html");
            Box::new(move |_, url, input| {
                m.url = url;
                let mut out = Vec::new();
                let mut r = tdewolff_parse::buffer::Reader::new(cp(input));
                let err = html::minify(&m, &mut out, &mut r, None).err();
                (out, err)
            })
        }
        "html/TestHTMLGoTemplates" => minifier_run(
            special_config("t-css"),
            Box::new(html::Minifier {
                template_delims: ["{{".into(), "}}".into()],
                ..Default::default()
            }),
            None,
        ),
        "html/TestHTMLPHPTemplates" => minifier_run(
            special_config("t-css"),
            Box::new(html::Minifier {
                template_delims: ["<?".into(), "?>".into()],
                ..Default::default()
            }),
            None,
        ),
        "css/TestCSS" => minifier_run(M::new(), Box::new(css::Minifier::default()), None),
        "css/TestCSSInline" => minifier_run(M::new(), Box::new(css::Minifier::default()), inline()),
        "css/TestCSSKeepCSS2" => minifier_run(
            M::new(),
            Box::new(css::Minifier {
                keep_css2: true,
                ..Default::default()
            }),
            inline(),
        ),
        "json/TestJSON" => minifier_run(M::new(), Box::new(json::Minifier::default()), None),
        "json/TestJSON_IgnoreNumbers" => minifier_run(
            M::new(),
            Box::new(json::Minifier {
                keep_numbers: true,
                ..Default::default()
            }),
            None,
        ),
        "svg/TestSVG" => minifier_run(M::new(), Box::new(svg::Minifier::default()), None),
        "svg/TestSVGStyle" => minifier_run(
            special_config("t-css"),
            Box::new(svg::Minifier::default()),
            None,
        ),
        "svg/TestSVGPrecision" => minifier_run(
            M::new(),
            Box::new(svg::Minifier {
                precision: 1,
                ..Default::default()
            }),
            None,
        ),
        "svg/TestSVGInline" => minifier_run(
            M::new(),
            Box::new(svg::Minifier {
                inline: true,
                ..Default::default()
            }),
            None,
        ),
        "svg/TestPathData" | "svg/TestPathDataTruncated" => {
            let prec = if name == "svg/TestPathData" { 0 } else { 3 };
            let mut p = svg::PathData::new(&svg::Minifier {
                precision: prec,
                ..Default::default()
            });
            Box::new(move |_, _, input| (p.shorten_path_data(cp(input)).to_vec(), None))
        }
        "xml/TestXML" => minifier_run(M::new(), Box::new(xml::Minifier::default()), None),
        "xml/TestXMLKeepWhitespace" => minifier_run(
            M::new(),
            Box::new(xml::Minifier {
                keep_whitespace: true,
            }),
            None,
        ),
        "minify/TestMediatype" => Box::new(|_, _, input| (mediatype(cp(input)).to_vec(), None)),
        "minify/TestDataURI" => {
            let m = special_config("t-datauri");
            Box::new(move |_, _, input| (data_uri(&m, cp(input)).to_vec(), None))
        }
        "minify/TestDecimal" => Box::new(|_, _, input| (decimal(cp(input), -1).to_vec(), None)),
        "minify/TestDecimalTruncate" => {
            Box::new(|a, _, input| (decimal(cp(input), a).to_vec(), None))
        }
        "minify/TestNumber" => Box::new(|_, _, input| (number(cp(input), -1).to_vec(), None)),
        "minify/TestNumberTruncate" => {
            Box::new(|a, _, input| (number(cp(input), a).to_vec(), None))
        }
        _ => panic!("unknown upstream test {}", name),
    }
}

#[test]
fn upstream_tables() {
    let recs = records("upstream");
    let mut mm = Mismatches::new("upstream");
    let mut cur_name = String::new();
    let mut run: Option<Run> = None;
    let (mut rows, mut expected_ok, mut js_rows) = (0, 0, 0);
    for r in recs.iter().filter(|r| r[0] == "up") {
        let name = &r[1];
        if *name != cur_name {
            cur_name = name.clone();
            run = Some(upstream_run(name));
        }
        let arg: i64 = if r[3] == "-" {
            0
        } else {
            r[3].parse().unwrap()
        };
        let url = if r[4] == "-" {
            None
        } else {
            Some(Url {
                scheme: String::from_utf8(unhex(&r[4])).unwrap(),
            })
        };
        let input = unhex(&r[5]);
        let expected = unhex(&r[6]);
        let go_out = unhex(&r[7]);
        let (out, err) = (run.as_mut().unwrap())(arg, url, &input);
        rows += 1;
        if go_out == expected && r[8] == "-" {
            expected_ok += 1;
        } else {
            js_rows += 1;
        }
        mm.check(out == go_out && err_str(&err) == r[8], || {
            format!(
                "{} #{}: in={}\n  go={} err={}\n  rs={} err={}",
                name,
                r[2],
                lossy(&input),
                lossy(&go_out),
                r[8],
                lossy(&out),
                err_str(&err)
            )
        });
    }
    eprintln!(
        "upstream rows: {} (equal to upstream expectation: {}, differing because they need the js minifier: {})",
        rows, expected_ok, js_rows
    );
    mm.finish();
}

// ---------------------------------------------------------------------
// minify_test.go (hand-ported; AddCmd/Reader/Writer/ResponseWriter/
// Middleware are not ported)

fn dummy_err() -> GoError {
    GoError::Other(b"dummy error".to_vec())
}

fn test_m() -> M {
    let mut m = M::new();
    m.add_func("dummy/copy", |_m, w, r, _p| {
        let (b, _) = tdewolff_parse::read_all(r);
        let _ = w.write_go(&b);
        Ok(())
    });
    m.add_func("dummy/nil", |_m, _w, _r, _p| Ok(()));
    m.add_func("dummy/err", |_m, _w, _r, _p| Err(dummy_err()));
    m.add_func("dummy/charset", |_m, w, _r, p| {
        let v = p
            .and_then(|p| p.get(b"charset".as_slice()))
            .cloned()
            .unwrap_or_default();
        let _ = w.write(&v);
        Ok(())
    });
    m.add_func("dummy/params", |m, w, r, p| {
        let get = |k: &[u8]| p.and_then(|p| p.get(k)).cloned().unwrap_or_default();
        let mut mt = get(b"type");
        mt.push(b'/');
        mt.extend_from_slice(&get(b"sub"));
        m.minify(mt, w, r)
    });
    m.add_func("type/sub", |_m, w, _r, _p| {
        let _ = w.write(b"type/sub");
        Ok(())
    });
    m.add_func_regexp(Regexp::must_compile("^type/.+$"), |_m, w, _r, _p| {
        let _ = w.write(b"type/*");
        Ok(())
    });
    m.add_func_regexp(Regexp::must_compile("^.+/.+$"), |_m, w, _r, _p| {
        let _ = w.write(b"*/*");
        Ok(())
    });
    m
}

struct NilReader;

impl GoReader for NilReader {
    fn read(&mut self, _p: &mut [u8]) -> (usize, Option<GoError>) {
        (0, Some(GoError::Eof))
    }
}

#[test]
fn test_minify() {
    let m = test_m();
    let mut w = Vec::new();
    assert!(is_err_not_exist(
        &m.minify("?", &mut w, &mut NilReader).unwrap_err()
    ));
    assert!(m.minify("dummy/nil", &mut w, &mut NilReader).is_ok());
    assert_eq!(
        m.minify("dummy/err", &mut w, &mut NilReader).unwrap_err(),
        dummy_err()
    );

    let b = GoBytes::from_slice(b"test");
    let (out, err) = m.bytes("dummy/nil", b.clone());
    assert!(err.is_none());
    assert_eq!(out.to_vec(), b"", "dummy/nil returns empty byte slice");
    let (out, err) = m.bytes("?", b.clone());
    assert!(is_err_not_exist(&err.unwrap()), "minifier doesn't exist");
    assert_eq!(
        out.to_vec(),
        b"test",
        "return input when minifier doesn't exist"
    );

    let (out, err) = m.string("dummy/nil", b"test");
    assert!(err.is_none());
    assert_eq!(out, b"");
    let (out, err) = m.string("?", b"test");
    assert!(is_err_not_exist(&err.unwrap()));
    assert_eq!(out, b"test");
    assert_eq!(
        err_not_exist().error_bytes(),
        b"minifier does not exist for mimetype"
    );
}

struct DummyMinifier;

impl Minifier for DummyMinifier {
    fn minify(
        &self,
        _m: &M,
        _w: &mut dyn Writer,
        _r: &mut dyn GoReader,
        _p: Option<&Params>,
    ) -> Result<(), GoError> {
        Err(dummy_err())
    }
}

#[test]
fn test_add() {
    let mut m = M::new();
    let mut w = Vec::new();
    m.add("dummy/err", Arc::new(DummyMinifier));
    assert_eq!(
        m.minify("dummy/err", &mut w, &mut NilReader),
        Err(dummy_err())
    );

    m.add_regexp(Regexp::must_compile("err1$"), Arc::new(DummyMinifier));
    assert_eq!(
        m.minify("dummy/err1", &mut w, &mut NilReader),
        Err(dummy_err())
    );

    m.add_func("dummy/err", |_m, _w, _r, _p| Err(dummy_err()));
    assert_eq!(
        m.minify("dummy/err", &mut w, &mut NilReader),
        Err(dummy_err())
    );

    m.add_func_regexp(Regexp::must_compile("err2$"), |_m, _w, _r, _p| {
        Err(dummy_err())
    });
    assert_eq!(
        m.minify("dummy/err2", &mut w, &mut NilReader),
        Err(dummy_err())
    );
}

#[test]
fn test_match() {
    let m = test_m();
    let (pattern, params, _) = m.match_("dummy/copy; a=b");
    assert_eq!(pattern, b"dummy/copy");
    assert_eq!(params.unwrap().get(b"a".as_slice()).unwrap(), b"b");

    let (pattern, _, _) = m.match_("type/foobar");
    assert_eq!(pattern, b"^type/.+$");

    let (_, _, minifier) = m.match_("dummy/");
    assert!(minifier.is_none());
}

#[test]
fn test_wildcard() {
    let m = test_m();
    for (mimetype, expected) in [
        ("type/sub", "type/sub"),
        ("type/*", "type/*"),
        ("*/*", "*/*"),
        ("type/sub2", "type/*"),
        ("type2/sub", "*/*"),
        ("dummy/charset;charset=UTF-8", "UTF-8"),
        ("dummy/charset; charset = UTF-8 ", "UTF-8"),
        ("dummy/params;type=type;sub=two2", "type/*"),
    ] {
        let mut w = Vec::new();
        let mut r = tdewolff_parse::buffer::Reader::new(GoBytes::from_slice(b""));
        m.minify(mimetype, &mut w, &mut r).unwrap();
        assert_eq!(String::from_utf8(w).unwrap(), expected, "{}", mimetype);
    }
}

// ---------------------------------------------------------------------
// Reader / writer / nested-minifier error tests of html, css, svg, xml,
// json (hand-ported)

/// github.com/tdewolff/test.ErrorWriter: fails after n writes.
struct ErrorWriter(usize);

impl Writer for ErrorWriter {
    fn write(&mut self, b: &[u8]) -> Result<usize, GoError> {
        if self.0 == 0 {
            return Err(err_plain());
        }
        self.0 -= 1;
        Ok(b.len())
    }
}

/// github.com/tdewolff/test.ErrorReader(0): fails at the first read.
struct ErrorReader;

impl GoReader for ErrorReader {
    fn read(&mut self, p: &mut [u8]) -> (usize, Option<GoError>) {
        if p.is_empty() {
            return (0, None);
        }
        (0, Some(err_plain()))
    }
}

#[test]
fn test_reader_errors() {
    let m = M::new();
    let minifiers: Vec<Box<dyn Minifier>> = vec![
        Box::new(html::Minifier::default()),
        Box::new(css::Minifier::default()),
        Box::new(svg::Minifier::default()),
        Box::new(xml::Minifier::default()),
        Box::new(json::Minifier::default()),
    ];
    for mf in minifiers {
        let mut w = Vec::new();
        let err = mf.minify(&m, &mut w, &mut ErrorReader, None);
        assert_eq!(err, Err(err_plain()), "return error at first read");
    }
}

#[test]
fn test_writer_errors() {
    let html_tests: &[(&str, &[usize])] = &[
        ("<!doctype>", &[0]),
        ("text", &[0]),
        ("<foo attr=val>", &[0, 1, 2, 3, 4, 5]),
        ("</foo>", &[0]),
        ("<style>x</style>", &[2]),
        ("<textarea>x</textarea>", &[2]),
        ("<code>x</code>", &[2]),
        ("<pre>x</pre>", &[2]),
        ("<svg>x</svg>", &[0]),
        ("<math>x</math>", &[0]),
        ("<!--[if IE 6]> text <![endif]-->", &[0, 1, 2]),
        ("<![if IE 6]> text <![endif]>", &[0]),
    ];
    let mut m = M::new();
    m.add(
        "text/html",
        Arc::new(html::Minifier {
            keep_special_comments: true,
            ..Default::default()
        }),
    );
    for (input, ns) in html_tests {
        for &n in *ns {
            let mut r = tdewolff_parse::buffer::Reader::new(cp(input.as_bytes()));
            let err = m.minify("text/html", &mut ErrorWriter(n), &mut r);
            assert_eq!(err, Err(err_plain()), "{} {}", input, n);
        }
    }

    let css_tests: &[(&str, &[usize])] = &[
        ("@import 'file'", &[0, 2]),
        ("@media all{}", &[0, 2, 3, 4]),
        (
            "a[id^=\"L\"]{margin:2in!important;color:red}",
            &[0, 4, 6, 7, 8, 9, 10, 11],
        ),
        ("a{color:rgb(255,0,0)}", &[4]),
        ("a{color:rgb(255,255,255)}", &[4]),
        ("a{color:hsl(0,100%,50%)}", &[4]),
        ("a{color:hsl(360,100%,100%)}", &[4]),
        ("a{color:f(arg)}", &[4]),
        ("<!--", &[0]),
        ("/*!comment*/", &[0, 1, 2]),
        ("a{--var:val}", &[2, 3, 4]),
        ("a{*color:0}", &[2, 3]),
        ("a{color:0;baddecl 5}", &[5]),
        ("a[id=\"x\" i],b{color:0}", &[5, 8]),
        ("a{color:()!important}", &[4, 6]),
        ("a{margin:5 4}", &[5]),
        ("a{margin=5}", &[2, 3]),
        ("a;", &[0]),
        ("a{000}", &[2]),
    ];
    for (input, ns) in css_tests {
        for &n in *ns {
            let mut r = tdewolff_parse::buffer::Reader::new(cp(input.as_bytes()));
            let err = css::minify(&M::new(), &mut ErrorWriter(n), &mut r, None);
            assert_eq!(err, Err(err_plain()), "{} {}", input, n);
        }
    }

    let svg_tests: &[(&str, &[usize])] = &[
        (
            "<!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\" \"foo.dtd\" [ <!ENTITY x \"bar\"> ]>",
            &[0],
        ),
        ("abc", &[0]),
        ("<style>abc</style>", &[2]),
        ("<![CDATA[ <<<< ]]>", &[0]),
        ("<![CDATA[ <<<<< ]]>", &[0]),
        ("<path d=\"x\"/>", &[0, 1, 2, 3, 4, 5]),
        ("<path></path>", &[1]),
        ("<svg>x</svg>", &[1, 3]),
        ("<svg>x</svg >", &[3]),
    ];
    for (input, ns) in svg_tests {
        for &n in *ns {
            let mut r = tdewolff_parse::buffer::Reader::new(cp(input.as_bytes()));
            let err = svg::minify(&M::new(), &mut ErrorWriter(n), &mut r, None);
            assert_eq!(err, Err(err_plain()), "{} {}", input, n);
        }
    }

    let xml_tests: &[(&str, &[usize])] = &[
        ("<!DOCTYPE foo>", &[0]),
        ("<?xml?>", &[0, 1]),
        ("<a x=y z=\"val\">", &[0, 1, 2, 3, 4, 8, 9]),
        ("<foo/>", &[1]),
        ("</foo>", &[0]),
        ("<foo></foo>", &[1]),
        ("<![CDATA[data<<<<<]]>", &[0]),
        ("text", &[0]),
    ];
    for (input, ns) in xml_tests {
        for &n in *ns {
            let mut r = tdewolff_parse::buffer::Reader::new(cp(input.as_bytes()));
            let err = xml::minify(&M::new(), &mut ErrorWriter(n), &mut r, None);
            assert_eq!(err, Err(err_plain()), "{} {}", input, n);
        }
    }

    //01    234  56  78
    let json_tests: &[(&str, &[usize])] = &[("{\"key\":[100,200]}", &[0, 1, 2, 3, 4, 5, 7, 8])];
    for (input, ns) in json_tests {
        for &n in *ns {
            let mut r = tdewolff_parse::buffer::Reader::new(cp(input.as_bytes()));
            let err = json::minify(&M::new(), &mut ErrorWriter(n), &mut r, None);
            assert_eq!(err, Err(err_plain()), "{} {}", input, n);
        }
    }
}

#[test]
fn test_html_minify_errors() {
    let m = special_config("errplain");
    for input in [
        "<style>abc</style>",
        "<p style=\"abc\"/>",
        "<p onclick=\"abc\"/>",
        "<svg></svg>",
        "<math></math>",
    ] {
        let mut w = Vec::new();
        let mut r = tdewolff_parse::buffer::Reader::new(cp(input.as_bytes()));
        let err = html::minify(&m, &mut w, &mut r, None);
        assert_eq!(err, Err(err_plain()), "{}", input);
    }
}

#[test]
fn test_svg_minify_errors() {
    let m = special_config("svgerr");
    for input in [
        "<style>abc</style>",
        "<style><![CDATA[abc]]></style>",
        "<path style=\"abc\"/>",
    ] {
        let mut w = Vec::new();
        let mut r = tdewolff_parse::buffer::Reader::new(cp(input.as_bytes()));
        let err = svg::minify(&m, &mut w, &mut r, None);
        assert_eq!(err, Err(err_plain()), "{}", input);
    }
}

#[test]
fn test_special_tag_closing() {
    let mut m = M::new();
    m.add("text/html", Arc::new(html::Minifier::default()));
    m.add_func("text/css", copy_func);
    let input = b"<style></script></style>";
    let mut w = Vec::new();
    let mut r = tdewolff_parse::buffer::Reader::new(cp(input));
    html::minify(&m, &mut w, &mut r, None).unwrap();
    assert_eq!(w, input);
}

#[test]
fn test_css_token_string() {
    use tdewolff_parse::css::{FunctionToken, IdentToken};
    let t = css::Token::new(
        IdentToken,
        GoBytes::from_slice(b"data"),
        css::Hash(0),
        css::Hash(0),
    );
    assert_eq!(t.string(), b"Ident(data)");
    let mut f = css::Token::new(
        FunctionToken,
        GoBytes::from_slice(b"func("),
        css::Hash(0),
        css::Hash(0),
    );
    f.args.push(t);
    assert_eq!(f.string(), b"func(Ident(data))");
}

#[test]
fn test_config_names_exist() {
    for name in common::configs::CONFIG_NAMES {
        let _ = config(name);
    }
}

// ---------------------------------------------------------------------
// buffer_test.go (hand-ported)

#[test]
fn test_html_buffer() {
    //    0 12  3           45   6   7   8             9   0
    let s = b"<p><a href=\"//url\">text</a>text<!--comment--></p>";
    let r = tdewolff_parse::Input::new_string(s);
    let mut z = html::TokenBuffer::new(r.clone(), tdewolff_parse::html::Lexer::new(r));

    let tok = z.shift().clone();
    assert!(tok.hash == html::P, "first token is <p>");
    assert_eq!(
        z.pos_len(),
        (0, 0),
        "shift first token and restore position/length"
    );

    assert!(z.peek(2).hash == html::Href, "third token is href");
    assert_eq!(z.pos_len(), (0, 3), "two tokens after peeking");

    assert!(z.peek(8).hash == html::P, "ninth token is <p>");
    assert_eq!(z.pos_len(), (0, 9), "nine tokens after peeking");

    assert!(
        z.peek(9).token_type == tdewolff_parse::html::ErrorToken,
        "tenth token is an error"
    );
    assert_eq!(
        z.peek_index(9),
        z.peek_index(10),
        "tenth and eleventh tokens are EOF"
    );
    assert_eq!(z.pos_len().1, 10, "ten tokens after peeking");

    let _ = z.shift();
    let tok = z.shift().clone();
    assert!(tok.hash == html::A, "third token is <a>");
    assert_eq!(z.pos_len().0, 2, "don't change position after peeking");
}

#[test]
fn test_svg_buffer() {
    //    0   12     3            4 5   6   7 8   9    01
    let s = b"<svg><path d=\"M0 0L1 1z\"/>text<tag/>text</svg>";
    let r = tdewolff_parse::Input::new_string(s);
    let mut z = svg::TokenBuffer::new(r.clone(), tdewolff_parse::xml::Lexer::new(r));

    let tok = z.shift().clone();
    assert!(tok.hash == svg::Svg, "first token is <svg>");
    assert_eq!(z.pos_len(), (0, 0));

    assert!(z.peek(2).hash == svg::D, "third token is d");
    assert_eq!(z.pos_len(), (0, 3));

    assert!(z.peek(8).hash == svg::Svg, "ninth token is <svg>");
    assert_eq!(z.pos_len(), (0, 9));

    assert!(z.peek(9).token_type == tdewolff_parse::xml::ErrorToken);
    assert_eq!(z.peek_index(9), z.peek_index(10));
    assert_eq!(z.pos_len().1, 10);

    let _ = z.shift();
    let tok = z.shift().clone();
    assert!(tok.hash == svg::Path, "third token is <path>");
    assert_eq!(z.pos_len().0, 2);
}

#[test]
fn test_svg_attributes() {
    let r = tdewolff_parse::Input::new_string(
        b"<rect x=\"0\" y=\"1\" width=\"2\" height=\"3\" rx=\"4\" ry=\"5\"/>",
    );
    let l = tdewolff_parse::xml::Lexer::new(r.clone());
    let mut tb = svg::TokenBuffer::new(r, l);
    tb.shift();
    for _ in 0..2 {
        // run twice to ensure similar results
        let attrs = tb.attributes(&[svg::X, svg::Y, svg::Width, svg::Height, svg::Rx, svg::Ry]);
        for i in 0..6 {
            let idx = attrs[i].expect("attr must not be nil");
            let val = tb.token_mut(idx).attr_val.to_vec();
            assert_eq!(
                val,
                i.to_string().into_bytes(),
                "attr data is bad at position {}",
                i
            );
        }
    }
}

#[test]
fn test_xml_buffer() {
    //    0 12  3           45   6   7   8             9   0
    let s = b"<p><a href=\"//url\">text</a>text<!--comment--></p>";
    let mut z = xml::TokenBuffer::new(tdewolff_parse::xml::Lexer::new(
        tdewolff_parse::Input::new_string(s),
    ));

    let tok = z.shift().clone();
    assert_eq!(tok.text.to_vec(), b"p", "first token is <p>");
    assert_eq!(z.pos_len(), (0, 0));

    assert_eq!(z.peek(2).text.to_vec(), b"href", "third token is href");
    assert_eq!(z.pos_len(), (0, 3));

    assert_eq!(z.peek(8).text.to_vec(), b"p", "ninth token is <p>");
    assert_eq!(z.pos_len(), (0, 9));

    assert!(z.peek(9).token_type == tdewolff_parse::xml::ErrorToken);
    assert_eq!(z.peek_index(9), z.peek_index(10));
    assert_eq!(z.pos_len().1, 10);

    let _ = z.shift();
    let tok = z.shift().clone();
    assert_eq!(tok.text.to_vec(), b"a", "third token is <a>");
    assert_eq!(z.pos_len().0, 2);
}

#[test]
fn test_js_passthrough_without_minifier() {
    // With no JS minifier registered, inline scripts and on* handlers are
    // written as-is (ErrNotExist); with one registered its output is used.
    let input = b"<script> x = 1; </script><a onclick=\" javascript: go() \">x</a>";
    let m = config("seeksnack");
    let (out, err, _) = run_m(&m, b"text/html", input);
    assert!(err.is_none());
    assert_eq!(
        String::from_utf8(out).unwrap(),
        "<script> x = 1; </script><a onclick=\" go()\">x</a>"
    );
    let m = special_config("dummyjs");
    let (out, err, _) = run_m(&m, b"text/html", input);
    assert!(err.is_none());
    assert_eq!(
        String::from_utf8(out).unwrap(),
        "<script>x = 1;</script><a onclick=go()>x</a>"
    );
}

#[test]
fn test_m_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<M>();
    assert_send_sync::<html::Minifier>();
    assert_send_sync::<css::Minifier>();
    assert_send_sync::<svg::Minifier>();
    assert_send_sync::<xml::Minifier>();
    assert_send_sync::<json::Minifier>();
}
