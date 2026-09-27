//! The named minifier configurations of tools/go-oracle/tdewolff-minify
//! (minifiers.go `configs`), mirrored exactly.
#![allow(dead_code)]

use std::sync::Arc;

use tdewolff_minify::{
    GoError, GoReader, M, Minifier, Params, Regexp, Url, Writer, css, html, json, svg, xml,
};

/// Regexps registered by neohugo's minifiers.New (minifiers/minifiers.go).
pub const JS_PATTERN: &str = "^(application|text)/(x-)?(java|ecma)script$";
pub const JSON_PATTERN: &str = r"^(application|text)/(x-|(ld|manifest)\+)?json$";

/// neohugo's minifiers.New with the seeksnack [minify] config, without any
/// JS minifier.
pub fn seeksnack_m() -> M {
    let mut m = M::new();
    m.add(
        "text/css",
        Arc::new(css::Minifier {
            precision: 0,
            keep_css2: true,
            ..Default::default()
        }),
    );
    let j: Arc<dyn Minifier> = Arc::new(json::Minifier::default());
    m.add("application/json", j.clone());
    m.add_regexp(Regexp::must_compile(JSON_PATTERN), j);
    m.add(
        "image/svg+xml",
        Arc::new(svg::Minifier {
            keep_comments: false,
            precision: 0,
            ..Default::default()
        }),
    );
    let x: Arc<dyn Minifier> = Arc::new(xml::Minifier {
        keep_whitespace: false,
    });
    m.add("application/rss+xml", x.clone());
    m.add("application/xml", x);
    m.add(
        "text/html",
        Arc::new(html::Minifier {
            keep_document_tags: true,
            keep_special_comments: true,
            keep_end_tags: true,
            keep_default_attr_vals: true,
            keep_whitespace: false,
            ..Default::default()
        }),
    );
    m
}

fn all_m(
    h: html::Minifier,
    c: css::Minifier,
    j: json::Minifier,
    s: svg::Minifier,
    x: xml::Minifier,
) -> M {
    let mut m = M::new();
    m.add("text/html", Arc::new(h));
    m.add("text/css", Arc::new(c));
    let j: Arc<dyn Minifier> = Arc::new(j);
    m.add("application/json", j.clone());
    m.add_regexp(Regexp::must_compile(JSON_PATTERN), j);
    m.add("image/svg+xml", Arc::new(s));
    let x: Arc<dyn Minifier> = Arc::new(x);
    m.add("application/xml", x.clone());
    m.add("application/rss+xml", x.clone());
    m.add_regexp(Regexp::must_compile("[/+]xml$"), x);
    m
}

fn d() -> (css::Minifier, json::Minifier, svg::Minifier, xml::Minifier) {
    Default::default()
}

fn delims(t: [&str; 2]) -> [String; 2] {
    [t[0].to_string(), t[1].to_string()]
}

/// The configuration named `name` (panics on unknown names).
pub fn config(name: &str) -> M {
    let (c, j, s, x) = d();
    match name {
        "seeksnack" => seeksnack_m(),
        "default" => all_m(html::Minifier::default(), c, j, s, x),
        "html-keepall" => all_m(
            html::Minifier {
                keep_comments: true,
                keep_default_attr_vals: true,
                keep_document_tags: true,
                keep_end_tags: true,
                keep_quotes: true,
                keep_whitespace: true,
                ..Default::default()
            },
            c,
            j,
            s,
            x,
        ),
        "html-endtags" => all_m(
            html::Minifier {
                keep_end_tags: true,
                ..Default::default()
            },
            c,
            j,
            s,
            x,
        ),
        "html-special" => all_m(
            html::Minifier {
                keep_special_comments: true,
                ..Default::default()
            },
            c,
            j,
            s,
            x,
        ),
        "html-ws" => all_m(
            html::Minifier {
                keep_whitespace: true,
                ..Default::default()
            },
            c,
            j,
            s,
            x,
        ),
        "html-quotes" => all_m(
            html::Minifier {
                keep_quotes: true,
                ..Default::default()
            },
            c,
            j,
            s,
            x,
        ),
        "html-gotmpl" => all_m(
            html::Minifier {
                template_delims: delims(html::GoTemplateDelims),
                ..Default::default()
            },
            c,
            j,
            s,
            x,
        ),
        "html-php" => all_m(
            html::Minifier {
                template_delims: delims(html::PHPTemplateDelims),
                ..Default::default()
            },
            c,
            j,
            s,
            x,
        ),
        "url-http" => {
            let mut m = all_m(html::Minifier::default(), c, j, s, x);
            m.url = Some(Url {
                scheme: "http".to_string(),
            });
            m
        }
        "url-https" => {
            let mut m = all_m(html::Minifier::default(), c, j, s, x);
            m.url = Some(Url {
                scheme: "https".to_string(),
            });
            m
        }
        "css2-prec3" => all_m(
            html::Minifier::default(),
            css::Minifier {
                keep_css2: true,
                precision: 3,
                ..Default::default()
            },
            json::Minifier {
                precision: 3,
                ..Default::default()
            },
            svg::Minifier {
                precision: 3,
                ..Default::default()
            },
            xml::Minifier {
                keep_whitespace: true,
            },
        ),
        "prec1-inline" => all_m(
            html::Minifier::default(),
            css::Minifier {
                precision: 1,
                inline: true,
                ..Default::default()
            },
            json::Minifier {
                keep_numbers: true,
                ..Default::default()
            },
            svg::Minifier {
                precision: 1,
                inline: true,
                keep_comments: true,
            },
            xml::Minifier::default(),
        ),
        _ => special_config(name),
    }
}

/// Go `io.Copy(w, r)` as a minifier (upstream tests' dummy css/js).
pub fn copy_func(
    _m: &M,
    w: &mut dyn Writer,
    r: &mut dyn GoReader,
    _p: Option<&Params>,
) -> Result<(), GoError> {
    let (data, err) = tdewolff_parse::read_all(r);
    w.write_go(&data)?;
    match err {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// github.com/tdewolff/test.ErrPlain
pub fn err_plain() -> GoError {
    GoError::Other(b"error".to_vec())
}

fn err_plain_func(
    _m: &M,
    _w: &mut dyn Writer,
    _r: &mut dyn GoReader,
    _p: Option<&Params>,
) -> Result<(), GoError> {
    Err(err_plain())
}

/// Stand-in JS minifier: writes the input with ASCII whitespace trimmed.
fn trim_copy_func(
    _m: &M,
    w: &mut dyn Writer,
    r: &mut dyn GoReader,
    _p: Option<&Params>,
) -> Result<(), GoError> {
    let (data, err) = tdewolff_parse::read_all(r);
    if let Some(e) = err {
        return Err(e);
    }
    w.write_go(&tdewolff_parse::trim_whitespace(&data))?;
    Ok(())
}

/// Returns a `*parse.Error` positioned in the middle of the input.
fn err_js_func(
    _m: &M,
    _w: &mut dyn Writer,
    r: &mut dyn GoReader,
    _p: Option<&Params>,
) -> Result<(), GoError> {
    let (data, err) = tdewolff_parse::read_all(r);
    if let Some(e) = err {
        return Err(e);
    }
    let n = data.len();
    let mut rd = tdewolff_parse::buffer::Reader::new(data);
    Err(GoError::Parse(Box::new(tdewolff_parse::new_error(
        Some(&mut rd),
        (n / 2) as isize,
        b"dummy js error".to_vec(),
    ))))
}

fn html_default() -> Arc<dyn Minifier> {
    Arc::new(html::Minifier::default())
}

/// The special configurations of the oracle (special.go `specialConfigs`).
pub fn special_config(name: &str) -> M {
    let mut m = M::new();
    match name {
        "t-empty" => {}
        "t-copycssjs" => {
            m.add("text/html", html_default());
            m.add_func("text/css", copy_func);
            m.add_func("application/javascript", copy_func);
        }
        "t-html" => {
            m.add("text/html", html_default());
        }
        "t-htmlcsssvg" => {
            m.add("text/html", html_default());
            m.add("text/css", Arc::new(css::Minifier::default()));
            m.add("image/svg+xml", Arc::new(svg::Minifier::default()));
        }
        "t-css" => {
            m.add("text/css", Arc::new(css::Minifier::default()));
        }
        "t-datauri" => {
            m.add_func("text/x", copy_func);
        }
        "errplain" => {
            m.add(
                "text/html",
                Arc::new(html::Minifier {
                    keep_special_comments: true,
                    ..Default::default()
                }),
            );
            m.add_func("text/css", err_plain_func);
            m.add_func("application/javascript", err_plain_func);
            m.add_func("image/svg+xml", err_plain_func);
            m.add_func("application/mathml+xml", err_plain_func);
        }
        "svgerr" => {
            m.add("image/svg+xml", Arc::new(svg::Minifier::default()));
            m.add_func("text/css", err_plain_func);
        }
        "dummyjs" => {
            m = seeksnack_m();
            m.add_func_regexp(Regexp::must_compile(JS_PATTERN), trim_copy_func);
        }
        "errjs" => {
            m = seeksnack_m();
            m.add_func_regexp(Regexp::must_compile(JS_PATTERN), err_js_func);
        }
        "xml-json-rx" => {
            m.add("text/html", html_default());
            m.add("text/css", Arc::new(css::Minifier::default()));
            m.add("image/svg+xml", Arc::new(svg::Minifier::default()));
            m.add_regexp(
                Regexp::must_compile("[/+]json$"),
                Arc::new(json::Minifier::default()),
            );
            m.add_regexp(
                Regexp::must_compile("[/+]xml$"),
                Arc::new(xml::Minifier::default()),
            );
        }
        _ => panic!("unknown config {}", name),
    }
    m
}

/// All configuration names, in oracle order.
pub const CONFIG_NAMES: &[&str] = &[
    "seeksnack",
    "default",
    "html-keepall",
    "html-endtags",
    "html-special",
    "html-ws",
    "html-quotes",
    "html-gotmpl",
    "html-php",
    "url-http",
    "url-https",
    "css2-prec3",
    "prec1-inline",
];
