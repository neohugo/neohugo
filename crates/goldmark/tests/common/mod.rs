//! Shared test helpers: the GMF fixture reader (format written by
//! tools/go-oracle/goldmark/render.go) and the named goldmark configurations
//! the oracle renders with.
#![allow(dead_code)]

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use std::sync::Arc;

use goldmark::ast::{Ast, AttrValue, NodeId};
use goldmark::extension;
use goldmark::parser::{self, AstTransformer};
use goldmark::renderer::html;

pub struct Record {
    pub name: String,
    pub fields: HashMap<String, Vec<u8>>,
    pub keys: Vec<String>,
}

impl Record {
    pub fn get(&self, k: &str) -> &[u8] {
        self.fields
            .get(k)
            .unwrap_or_else(|| panic!("{}: no field {k}", self.name))
    }

    pub fn str(&self, k: &str) -> String {
        String::from_utf8(self.get(k).to_vec()).unwrap()
    }
}

pub fn read_file(path: &Path) -> Vec<u8> {
    let raw = std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    if path.extension().is_some_and(|e| e == "gz") {
        let mut d = flate2::read::GzDecoder::new(&raw[..]);
        let mut out = Vec::new();
        d.read_to_end(&mut out).unwrap();
        return out;
    }
    raw
}

pub fn parse_gmf(data: &[u8]) -> Vec<Record> {
    let mut recs: Vec<Record> = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let nl = i + data[i..].iter().position(|&c| c == b'\n').unwrap();
        let line = std::str::from_utf8(&data[i..nl]).unwrap();
        i = nl + 1;
        if let Some(name) = line.strip_prefix("=== ") {
            recs.push(Record {
                name: name.to_string(),
                fields: HashMap::new(),
                keys: Vec::new(),
            });
            continue;
        }
        let sp = line.rfind(' ').unwrap();
        let key = line[..sp].to_string();
        let n: usize = line[sp + 1..].parse().unwrap();
        let v = data[i..i + n].to_vec();
        assert_eq!(data[i + n], b'\n');
        i += n + 1;
        let rec = recs.last_mut().unwrap();
        rec.keys.push(key.clone());
        rec.fields.insert(key, v);
    }
    recs
}

pub fn fixture(name: &str) -> Vec<Record> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    parse_gmf(&read_file(&path))
}

/// The goldmark instances of tools/go-oracle/goldmark/render.go:newMarkdown.
pub fn new_markdown(name: &str) -> goldmark::Markdown {
    use goldmark::{new, with_parser_options, with_renderer_options};
    match name {
        "default" => new(vec![]),
        "xu" => new(vec![with_renderer_options(vec![
            Box::new(html::with_xhtml()),
            Box::new(html::with_unsafe()),
        ])]),
        "attr" => new(vec![with_parser_options(vec![
            parser::with_attribute(),
            Box::new(parser::with_auto_heading_id()),
        ])]),
        "unsafe" => new(vec![with_renderer_options(vec![Box::new(
            html::with_unsafe(),
        )])]),
        "hard" => new(vec![with_renderer_options(vec![
            Box::new(html::with_hard_wraps()),
            Box::new(html::with_xhtml()),
        ])]),
        "all" => new(vec![
            with_parser_options(vec![
                parser::with_attribute(),
                Box::new(parser::with_auto_heading_id()),
            ]),
            with_renderer_options(vec![
                Box::new(html::with_unsafe()),
                Box::new(html::with_xhtml()),
                Box::new(html::with_hard_wraps()),
            ]),
        ]),
        "escspace" => new(vec![
            with_parser_options(vec![parser::with_escaped_space()]),
            with_renderer_options(vec![
                Box::new(html::with_writer(html::new_writer(vec![
                    html::with_escaped_space(),
                ]))),
                Box::new(html::with_unsafe()),
            ]),
        ]),
        "ea-simple" => new(vec![with_renderer_options(vec![Box::new(
            html::with_east_asian_line_breaks(html::EastAsianLineBreaks::Simple),
        )])]),
        "ea-css3" => new(vec![with_renderer_options(vec![Box::new(
            html::with_east_asian_line_breaks(html::EastAsianLineBreaks::Css3Draft),
        )])]),
        _ => new_ext_markdown(name),
    }
}

/// neohugo's typographer substitutions (convert.go:toTypographicPunctuationMap
/// with the default config).
fn hugo_typographer() -> Box<dyn goldmark::Extender> {
    use extension::TypographicPunctuation::*;
    extension::new_typographer(vec![extension::with_typographic_substitutions(&[
        (LeftSingleQuote, "&lsquo;"),
        (RightSingleQuote, "&rsquo;"),
        (LeftDoubleQuote, "&ldquo;"),
        (RightDoubleQuote, "&rdquo;"),
        (EnDash, "&ndash;"),
        (EmDash, "&mdash;"),
        (Ellipsis, "&hellip;"),
        (LeftAngleQuote, "&laquo;"),
        (RightAngleQuote, "&raquo;"),
        (Apostrophe, "&rsquo;"),
    ])])
}

/// The goldmark extensions neohugo installs for seeksnack (same order).
pub fn hugo_extensions() -> Vec<Box<dyn goldmark::Extender>> {
    vec![
        extension::table(),
        extension::strikethrough(),
        extension::linkify(),
        extension::task_list(),
        hugo_typographer(),
        extension::definition_list(),
        extension::footnote(),
    ]
}

/// extension/table_test.go's tableStyleTransformer (tolerant of documents
/// that do not start with a table, like the oracle's copy).
struct TableStyleTransformer;

impl AstTransformer for TableStyleTransformer {
    fn transform<'a>(
        &self,
        ast: &mut Ast,
        node: NodeId,
        _reader: &mut dyn goldmark::text::Reader<'a>,
        _pc: &mut parser::Context,
    ) {
        let Some(fc) = ast.first_child(node) else {
            return;
        };
        if ast.kind(fc) != *extension::ast::KIND_TABLE {
            return;
        }
        let Some(cell) = ast.first_child(fc).and_then(|h| ast.first_child(h)) else {
            return;
        };
        ast.set_attribute_string(cell, "style", AttrValue::Bytes(b"font-size:1em".to_vec()));
    }
}

/// extension/footnote_test.go's footnoteID transformer.
struct FootnoteId;

impl AstTransformer for FootnoteId {
    fn transform<'a>(
        &self,
        ast: &mut Ast,
        node: NodeId,
        _reader: &mut dyn goldmark::text::Reader<'a>,
        _pc: &mut parser::Context,
    ) {
        ast.document_mut(node).unwrap().meta().insert(
            "footnote-prefix".to_string(),
            Arc::new("article12-".to_string()),
        );
    }
}

fn is_re_space(c: u8) -> bool {
    matches!(c, b'\t' | b'\n' | b'\x0c' | b'\r' | b' ')
}

/// Go `regexp.MustCompile(`\w+://[^\s]+`).FindSubmatchIndex` (leftmost-first).
fn scheme_url_regexp(b: &[u8]) -> Option<(usize, usize)> {
    let is_word = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    for p in 0..b.len() {
        if !is_word(b[p]) {
            continue;
        }
        let mut q = p;
        while q < b.len() && is_word(b[q]) {
            q += 1;
        }
        if b[q..].starts_with(b"://") && q + 3 < b.len() && !is_re_space(b[q + 3]) {
            let mut e = q + 3;
            while e < b.len() && !is_re_space(b[e]) {
                e += 1;
            }
            return Some((p, e));
        }
    }
    None
}

/// Go `regexp.MustCompile(<literal>).FindSubmatchIndex`.
fn literal_regexp(lit: &'static [u8]) -> Arc<dyn extension::LinkifyRegexp> {
    Arc::new(move |b: &[u8]| {
        let i = b.windows(lit.len()).position(|w| w == lit)?;
        Some((i, i + lit.len()))
    })
}

/// The extension configurations of tools/go-oracle/goldmark/ext.go:newExtMarkdown.
pub fn new_ext_markdown(name: &str) -> goldmark::Markdown {
    use goldmark::{new, with_extensions, with_parser_options, with_renderer_options};
    let unsafe_ = || with_renderer_options(vec![Box::new(html::with_unsafe())]);
    let xu = || {
        with_renderer_options(vec![
            Box::new(html::with_xhtml()),
            Box::new(html::with_unsafe()),
        ])
    };
    match name {
        "hugo" => new(vec![
            with_extensions(hugo_extensions()),
            with_parser_options(vec![parser::with_attribute()]),
            unsafe_(),
        ]),
        "hugo-autoid" => new(vec![
            with_extensions(hugo_extensions()),
            with_parser_options(vec![
                parser::with_attribute(),
                Box::new(parser::with_auto_heading_id()),
            ]),
            unsafe_(),
        ]),
        "hugo-xhtml" => new(vec![
            with_extensions(hugo_extensions()),
            with_parser_options(vec![parser::with_attribute()]),
            with_renderer_options(vec![
                Box::new(html::with_unsafe()),
                Box::new(html::with_xhtml()),
                Box::new(html::with_hard_wraps()),
            ]),
        ]),
        "x-table" => new(vec![
            with_renderer_options(vec![
                Box::new(html::with_unsafe()),
                Box::new(html::with_xhtml()),
            ]),
            with_extensions(vec![extension::table()]),
        ]),
        "x-strike" => new(vec![
            unsafe_(),
            with_extensions(vec![extension::strikethrough()]),
        ]),
        "x-linkify" => new(vec![unsafe_(), with_extensions(vec![extension::linkify()])]),
        "x-tasklist" => new(vec![
            unsafe_(),
            with_extensions(vec![extension::task_list()]),
        ]),
        "x-deflist" => new(vec![
            unsafe_(),
            with_extensions(vec![extension::definition_list()]),
        ]),
        "x-footnote" => new(vec![
            unsafe_(),
            with_extensions(vec![extension::footnote()]),
        ]),
        "x-typo" => new(vec![
            unsafe_(),
            with_extensions(vec![extension::typographer()]),
        ]),
        "x-gfm" => new(vec![unsafe_(), with_extensions(vec![extension::gfm()])]),
        "x-cjk" => new(vec![xu(), with_extensions(vec![extension::cjk()])]),
        "x-cjk-esc" => new(vec![
            xu(),
            with_extensions(vec![extension::new_cjk(vec![
                extension::with_escaped_space(),
            ])]),
        ]),
        "x-cjk-linkify" => new(vec![
            xu(),
            with_extensions(vec![
                extension::new_cjk(vec![extension::with_escaped_space()]),
                extension::linkify(),
            ]),
        ]),
        "x-cjk-css3" => new(vec![
            xu(),
            with_extensions(vec![extension::new_cjk(vec![
                extension::with_east_asian_line_breaks(&[
                    extension::EastAsianLineBreaks::Css3Draft,
                ]),
            ])]),
        ]),
        "x-all" => new(vec![
            with_extensions(vec![
                extension::gfm(),
                extension::definition_list(),
                extension::footnote(),
                extension::typographer(),
                extension::cjk(),
            ]),
            with_parser_options(vec![
                parser::with_attribute(),
                Box::new(parser::with_auto_heading_id()),
            ]),
            with_renderer_options(vec![
                Box::new(html::with_unsafe()),
                Box::new(html::with_xhtml()),
            ]),
        ]),
        "x-table-attr" => new(vec![
            unsafe_(),
            with_extensions(vec![extension::new_table(vec![
                extension::with_table_cell_align_method(extension::TableCellAlignMethod::Attribute),
            ])]),
        ]),
        "x-table-style" => new(vec![
            xu(),
            with_extensions(vec![extension::new_table(vec![
                extension::with_table_cell_align_method(extension::TableCellAlignMethod::Style),
            ])]),
        ]),
        "x-table-none" => new(vec![
            xu(),
            with_extensions(vec![extension::new_table(vec![
                extension::with_table_cell_align_method(extension::TableCellAlignMethod::None),
            ])]),
        ]),
        "x-table-styletr" => new(vec![
            with_parser_options(vec![parser::with_ast_transformers(vec![
                goldmark::util::prioritized(
                    Box::new(TableStyleTransformer) as Box<dyn AstTransformer>,
                    0,
                ),
            ])]),
            unsafe_(),
            with_extensions(vec![extension::new_table(vec![
                extension::with_table_cell_align_method(extension::TableCellAlignMethod::Style),
            ])]),
        ]),
        "x-footnote-opts" => new(vec![
            unsafe_(),
            with_extensions(vec![extension::new_footnote(vec![
                extension::with_footnote_id_prefix("article12-"),
                extension::with_footnote_link_class("link-class"),
                extension::with_footnote_backlink_class("backlink-class"),
                extension::with_footnote_link_title("link-title-%%-^^"),
                extension::with_footnote_backlink_title("backlink-title"),
                extension::with_footnote_backlink_html("^"),
                extension::with_footnote_html_options(vec![Arc::new(html::with_xhtml())]),
            ])]),
        ]),
        "x-footnote-fn" => new(vec![
            with_parser_options(vec![parser::with_ast_transformers(vec![
                goldmark::util::prioritized(Box::new(FootnoteId) as Box<dyn AstTransformer>, 100),
            ])]),
            unsafe_(),
            with_extensions(vec![extension::new_footnote(vec![
                extension::with_footnote_id_prefix_function(Arc::new(|ast: &Ast, n: NodeId| {
                    let doc = ast.owner_document(n)?;
                    let v = ast.document(doc)?.get_meta()?.get("footnote-prefix")?;
                    Some(v.downcast_ref::<String>().unwrap().as_bytes().to_vec())
                })),
                extension::with_footnote_link_class(b"link-class"),
                extension::with_footnote_backlink_class(b"backlink-class"),
                extension::with_footnote_link_title(b"link-title-%%-^^"),
                extension::with_footnote_backlink_title(b"backlink-title"),
                extension::with_footnote_backlink_html(b"^"),
            ])]),
        ]),
        "x-linkify-proto" => new(vec![
            xu(),
            with_extensions(vec![extension::new_linkify(vec![
                extension::with_linkify_allowed_protocols(&["ssh:"]),
                extension::with_linkify_url_regexp(Arc::new(scheme_url_regexp)),
            ])]),
        ]),
        "x-linkify-www" => new(vec![
            xu(),
            with_extensions(vec![extension::new_linkify(vec![
                extension::with_linkify_www_regexp(literal_regexp(b"www.example.com")),
            ])]),
        ]),
        "x-linkify-email" => new(vec![
            xu(),
            with_extensions(vec![extension::new_linkify(vec![
                extension::with_linkify_email_regexp(literal_regexp(b"user@example.com")),
            ])]),
        ]),
        "x-typo-custom" => {
            use extension::TypographicPunctuation::*;
            new(vec![
                unsafe_(),
                with_extensions(vec![extension::new_typographer(vec![
                    extension::with_typographic_substitutions(&[
                        (LeftSingleQuote, ""),
                        (EnDash, "&#8211;"),
                        (Ellipsis, ""),
                        (Apostrophe, "'"),
                        (LeftAngleQuote, "<<"),
                    ]),
                ])]),
            ])
        }
        _ => panic!("unknown config {name}"),
    }
}

pub struct Markdowns(HashMap<String, goldmark::Markdown>);

impl Markdowns {
    pub fn new() -> Self {
        Markdowns(HashMap::new())
    }

    /// Renders like the oracle's convert: a panic becomes "PANIC: ...".
    pub fn convert(&mut self, cfg: &str, md: &[u8]) -> Vec<u8> {
        let m = self
            .0
            .entry(cfg.to_string())
            .or_insert_with(|| new_markdown(cfg));
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut out: Vec<u8> = Vec::new();
            match m.convert(md, &mut out) {
                Ok(()) => out,
                Err(e) => format!("ERROR: {e}").into_bytes(),
            }
        }));
        match r {
            Ok(v) => v,
            Err(_) => b"PANIC".to_vec(),
        }
    }
}

/// Compares Rust output with the Go output of every record; returns the
/// number of records checked and panics listing the first failures.
pub fn check_records(recs: &[Record]) -> usize {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let mut mds = Markdowns::new();
    let mut failures = Vec::new();
    for r in recs {
        let cfg = r.str("cfg");
        let got = mds.convert(&cfg, r.get("md"));
        let want = r.get("html");
        let ok = if want.starts_with(b"PANIC: ") {
            got == b"PANIC"
        } else {
            got == want
        };
        if !ok {
            failures.push(format!(
                "--- {}\nmd:   {:?}\nwant: {:?}\ngot:  {:?}",
                r.name,
                String::from_utf8_lossy(r.get("md")),
                String::from_utf8_lossy(want),
                String::from_utf8_lossy(&got)
            ));
        }
    }
    std::panic::set_hook(prev);
    if !failures.is_empty() {
        let n = failures.len();
        failures.truncate(15);
        panic!(
            "{n}/{} records differ:\n{}",
            recs.len(),
            failures.join("\n")
        );
    }
    recs.len()
}
