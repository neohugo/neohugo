//! Shared support for the nh-markup oracle tests: fixture reading, the markup configs, the
//! Rust twins of the Go oracle's hook renderers (tools/go-oracle/nh-markup/mdoracle) and the
//! canonical dump format.

#![allow(dead_code)]

use std::io::Read;
use std::sync::{Arc, Mutex};

use go_value::{GoString, HostCtx, Kind, Map, Object, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_common::text::Position;
use nh_markup::converter::converter::{
    Converter, DocumentContext, Provider, RenderContext, ResultRender,
};
use nh_markup::converter::hooks::{
    BlockquoteRenderer, CodeBlockRenderer, GetRendererFunc, HeadingRenderer, LinkRenderer,
    Renderer, RendererType, TableRenderer,
};
use nh_markup::goldmark::convert::GoldmarkProvider;
use nh_markup::markup_config::{self, Config as MarkupConfig};
use serde_json::Value as J;

// ---------------------------------------------------------------------------
// Fixtures

/// Reads a gzip-compressed JSON fixture below `tests/fixtures`.
pub fn read_fixture(rel: &str) -> J {
    let path = format!("{}/tests/fixtures/{rel}", env!("CARGO_MANIFEST_DIR"));
    let f = std::fs::File::open(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let mut s = String::new();
    flate2::read::GzDecoder::new(f)
        .read_to_string(&mut s)
        .expect("gunzip fixture");
    serde_json::from_str(&s).expect("fixture JSON")
}

/// A byte string of the fixtures (`mdoracle.B`): a JSON string, or `{"b64": ...}`.
pub fn bytes(v: &J) -> Vec<u8> {
    match v {
        J::Null => Vec::new(),
        J::String(s) => s.as_bytes().to_vec(),
        J::Object(o) => base64_decode(o["b64"].as_str().expect("b64")),
        other => panic!("not a byte string: {other}"),
    }
}

fn base64_decode(s: &str) -> Vec<u8> {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let mut acc: u32 = 0;
    let mut bits = 0;
    for c in s.bytes() {
        if c == b'=' {
            break;
        }
        let v = T.iter().position(|&x| x == c).expect("base64 char") as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    out
}

pub fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

// ---------------------------------------------------------------------------
// Configs and converters

/// Decodes the markup config of a TOML site config (Go: `markup_config.Decode`).
pub fn decode_markup(toml: &str) -> Result<MarkupConfig> {
    let prov = nh_config::config_loader::from_toml_config_string(toml);
    markup_config::decode(&prov)
}

/// The goldmark provider for a TOML site config.
pub fn provider(toml: &str) -> (Arc<dyn Provider>, Arc<MarkupConfig>) {
    let m = Arc::new(decode_markup(toml).expect("decode markup config"));
    let p = GoldmarkProvider::new_from_config(m.clone(), false, None).expect("provider");
    (p, m)
}

pub fn converter(p: &Arc<dyn Provider>, dctx: DocumentContext) -> Arc<dyn Converter> {
    p.new_converter(dctx).expect("converter")
}

pub fn document_context(name: &str) -> DocumentContext {
    DocumentContext {
        document: Value::Invalid,
        document_lookup: None,
        document_id: name.to_string(),
        document_name: String::new(),
        filename: name.to_string(),
    }
}

/// The outcome of one conversion, as the oracle records it.
pub enum Outcome {
    Html(ResultRender),
    Err(String),
    Panic(String),
}

/// Converts, catching panics (Go recovers them in the oracle).
pub fn convert(
    conv: &dyn Converter,
    src: &[u8],
    render_toc: bool,
    get: GetRendererFunc,
) -> Outcome {
    // The Go panics the fixtures record are expected: keep their messages off stderr (only
    // while converting; test failures still print).
    thread_local! {
        static IN_CONVERT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }
    static QUIET: std::sync::Once = std::sync::Once::new();
    QUIET.call_once(|| {
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if !IN_CONVERT.with(|c| c.get()) {
                prev(info)
            }
        }));
    });
    IN_CONVERT.with(|c| c.set(true));
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let host = ();
        let rctx = RenderContext {
            ctx: &host,
            src,
            render_toc,
            get_renderer: Some(get),
        };
        conv.convert(&rctx)
    }));
    IN_CONVERT.with(|c| c.set(false));
    match res {
        Ok(Ok(r)) => Outcome::Html(r),
        Ok(Err(e)) => Outcome::Err(e.to_string()),
        Err(p) => Outcome::Panic(
            p.downcast_ref::<String>()
                .cloned()
                .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default(),
        ),
    }
}

/// Runs `f` on a thread with a large stack (Hugo renders on goroutines).
pub fn big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(f)
        .expect("spawn")
        .join()
        .expect("join")
}

// ---------------------------------------------------------------------------
// The canonical dump format (mdoracle.Field / DumpVal).

pub fn field(b: &mut Vec<u8>, name: &str, v: &[u8]) {
    b.extend_from_slice(format!("{name}:{}:", v.len()).as_bytes());
    b.extend_from_slice(v);
    b.push(b'\n');
}

fn field_b(b: &mut Vec<u8>, name: &[u8], v: &[u8]) {
    b.extend_from_slice(name);
    b.extend_from_slice(format!(":{}:", v.len()).as_bytes());
    b.extend_from_slice(v);
    b.push(b'\n');
}

pub fn dump_val(v: &Value) -> Vec<u8> {
    match v {
        Value::Invalid => b"nil".to_vec(),
        Value::String(s) => [b"s:".as_slice(), s.as_bytes()].concat(),
        Value::Bool(b) => format!("b:{b}").into_bytes(),
        Value::Int(i, _) => format!("i:{i}").into_bytes(),
        Value::Float(f, _) => {
            let mut o = b"f:".to_vec();
            o.extend_from_slice(go_strconv::format_float(*f, b'g', -1, 64).as_bytes());
            o
        }
        Value::TypedNil(t) if &**t == "[][2]int" => b"r:nil".to_vec(),
        Value::List(l) if l.ty.go_name() == "[][2]int" => {
            let mut o = b"r:".to_vec();
            for r in &l.items {
                let r = r.as_list().expect("[2]int");
                let a = match (&r.items[0], &r.items[1]) {
                    (Value::Int(a, _), Value::Int(b, _)) => format!("[{a},{b}]"),
                    _ => panic!("bad range"),
                };
                o.extend_from_slice(a.as_bytes());
            }
            o
        }
        Value::Map(m) => dump_map(m),
        Value::Object(o) if o.type_name() == "hstring.HTML" => {
            let s = o.go_string().expect("HTML string");
            [b"h:".as_slice(), s.as_bytes()].concat()
        }
        other => panic!("dump_val: {}", other.go_type_name()),
    }
}

fn dump_map(m: &Map) -> Vec<u8> {
    let mut b = b"m{".to_vec();
    for (k, v) in &m.entries {
        field_b(&mut b, k.as_bytes(), &dump_val(v));
    }
    b.push(b'}');
    b
}

/// Calls a zero-argument method of a context object.
pub fn call(v: &Value, name: &str) -> Value {
    let o = v.as_object().expect("object");
    assert!(o.has_method(name), "{} has no method {name}", o.type_name());
    let host = ();
    o.call_method(&host, name, &[])
        .expect("method")
        .unwrap_or_else(|e| panic!("{name}: {}", e.message()))
}

fn s(v: &Value) -> Vec<u8> {
    match v {
        Value::String(s) => s.as_bytes().to_vec(),
        Value::Object(o) => o.go_string().expect("string-like").as_bytes().to_vec(),
        other => panic!("not a string: {}", other.go_type_name()),
    }
}

fn int(v: &Value) -> i64 {
    match v {
        Value::Int(i, _) => *i,
        other => panic!("not an int: {}", other.go_type_name()),
    }
}

fn html_escape(s: &[u8]) -> Vec<u8> {
    let mut o = Vec::new();
    for &c in s {
        match c {
            b'&' => o.extend_from_slice(b"&amp;"),
            b'"' => o.extend_from_slice(b"&quot;"),
            b'<' => o.extend_from_slice(b"&lt;"),
            b'>' => o.extend_from_slice(b"&gt;"),
            c => o.push(c),
        }
    }
    o
}

// ---------------------------------------------------------------------------
// Renderers

/// Table rows as (alignment, text) cells.
type Rows = Vec<Vec<(Vec<u8>, Vec<u8>)>>;

fn rows(v: &Value) -> Option<Rows> {
    match v {
        Value::TypedNil(_) => None,
        Value::List(l) => Some(
            l.items
                .iter()
                .map(|row| {
                    row.as_list()
                        .expect("row")
                        .items
                        .iter()
                        .map(|c| {
                            let o = c.as_object().expect("cell");
                            assert_eq!(o.type_name(), "hooks.TableCell");
                            (
                                s(&o.field("Alignment").expect("Alignment")),
                                s(&o.field("Text").expect("Text")),
                            )
                        })
                        .collect()
                })
                .collect(),
        ),
        other => panic!("rows: {}", other.go_type_name()),
    }
}

fn write_rows(b: &mut Vec<u8>, rows: Option<Rows>, tag: &str) {
    for row in rows.unwrap_or_default() {
        b.extend_from_slice(b"\n      <tr>");
        for (align, text) in row {
            b.extend_from_slice(format!("\n          <{tag}").as_bytes());
            if !align.is_empty() {
                b.extend_from_slice(b" style=\"text-align: ");
                b.extend_from_slice(&align);
                b.push(b'"');
            }
            b.push(b'>');
            b.extend_from_slice(&text);
            b.extend_from_slice(format!("</{tag}>").as_bytes());
        }
        b.extend_from_slice(b"\n      </tr>");
    }
}

/// `mdoracle.RenderTableReplica`.
pub fn render_table_replica(w: &mut Vec<u8>, ctx: &Value) {
    let mut b = b"<table".to_vec();
    if let Value::Map(attrs) = call(ctx, "Attributes") {
        for (k, v) in &attrs.entries {
            b.push(b' ');
            b.extend_from_slice(k.as_bytes());
            b.extend_from_slice(b"=\"");
            b.extend_from_slice(&html_escape(&dump_val(v)));
            b.push(b'"');
        }
    }
    b.extend_from_slice(b">\n  <thead>");
    write_rows(&mut b, rows(&call(ctx, "THead")), "th");
    b.extend_from_slice(b"\n  </thead>\n  <tbody>");
    write_rows(&mut b, rows(&call(ctx, "TBody")), "td");
    b.extend_from_slice(b"\n  </tbody>\n</table>\n");
    w.extend_from_slice(&b);
}

/// `mdoracle.TableReplica`.
pub struct TableReplica;

impl TableRenderer for TableReplica {
    fn render_table(&self, _cctx: HostCtx<'_>, w: &mut Vec<u8>, ctx: &Value) -> Result<()> {
        render_table_replica(w, ctx);
        Ok(())
    }
}

/// `mdoracle.CodeReplica`.
pub struct CodeReplica;

impl CodeBlockRenderer for CodeReplica {
    fn render_codeblock(&self, _cctx: HostCtx<'_>, w: &mut Vec<u8>, ctx: &Value) -> Result<()> {
        w.extend_from_slice(b"<pre data-lang=\"");
        w.extend_from_slice(&html_escape(&s(&call(ctx, "Type"))));
        w.extend_from_slice(
            format!("\" data-ordinal=\"{}\">", int(&call(ctx, "Ordinal"))).as_bytes(),
        );
        w.extend_from_slice(&html_escape(&s(&call(ctx, "Inner"))));
        w.extend_from_slice(b"</pre>\n");
        Ok(())
    }
}

/// `mdoracle.ReplicaRenderers`.
pub fn replica_renderers() -> GetRendererFunc {
    Arc::new(|t, _id| match t {
        RendererType::Table => Some(Renderer::Table(Arc::new(TableReplica))),
        RendererType::CodeBlock => Some(Renderer::CodeBlock(Arc::new(CodeReplica))),
        _ => None,
    })
}

/// `mdoracle.Recorder`.
#[derive(Default)]
pub struct Recorder {
    pub records: Mutex<Vec<Vec<u8>>>,
    seq: Mutex<i64>,
}

/// The exported method set of a hook context object (Go's `reflect` method names, sorted).
fn methods_of(ctx: &Value) -> Vec<&'static str> {
    use nh_markup::goldmark::{blockquotes, codeblocks, render_hooks, tables};
    let o = ctx.as_object().expect("object");
    let a = o.as_any();
    let mut m: Vec<&'static str> = if a.is::<render_hooks::LinkContext>() {
        render_hooks::LinkContext::GO_METHODS.to_vec()
    } else if a.is::<render_hooks::ImageLinkContext>() {
        render_hooks::ImageLinkContext::GO_METHODS.to_vec()
    } else if a.is::<render_hooks::HeadingContext>() {
        render_hooks::HeadingContext::GO_METHODS.to_vec()
    } else if a.is::<blockquotes::BlockquoteContext>() {
        blockquotes::BlockquoteContext::GO_METHODS.to_vec()
    } else if a.is::<tables::TableContext>() {
        tables::TableContext::GO_METHODS.to_vec()
    } else if a.is::<codeblocks::CodeBlockContext>() {
        codeblocks::CodeBlockContext::GO_METHODS.to_vec()
    } else {
        panic!("unknown context {}", o.type_name())
    };
    for name in &m {
        assert!(o.has_method(name), "{} lacks {name}", o.type_name());
    }
    m.sort();
    m
}

impl Recorder {
    fn next(&self, kind: &str, ctx: &Value) -> (i64, Vec<u8>) {
        let mut seq = self.seq.lock().unwrap();
        *seq += 1;
        let mut b = Vec::new();
        field(&mut b, "kind", kind.as_bytes());
        field(&mut b, "seq", seq.to_string().as_bytes());
        field(&mut b, "type", object_type(ctx).as_bytes());
        field(&mut b, "methods", methods_of(ctx).join(",").as_bytes());
        (*seq, b)
    }

    fn done(&self, b: Vec<u8>) {
        self.records.lock().unwrap().push(b);
    }
}

fn dump_page(b: &mut Vec<u8>, ctx: &Value) {
    field(b, "Page", &dump_val(&call(ctx, "Page")));
    field(b, "PageInner", &dump_val(&call(ctx, "PageInner")));
}

fn dump_slice(v: &Value) -> Vec<u8> {
    let mut sb = Vec::new();
    if let Value::List(l) = v {
        for a in &l.items {
            let o = a.as_object().expect("attribute");
            assert_eq!(o.type_name(), "attributes.Attribute");
            field_b(
                &mut sb,
                &s(&o.field("Name").expect("Name")),
                &dump_val(&o.field("Value").expect("Value")),
            );
        }
    }
    sb
}

fn dump_attrs(b: &mut Vec<u8>, ctx: &Value) {
    field(b, "Attributes", &dump_val(&call(ctx, "Attributes")));
    field(b, "Options", &dump_val(&call(ctx, "Options")));
    field(
        b,
        "AttributesSlice",
        &dump_slice(&call(ctx, "AttributesSlice")),
    );
    field(b, "OptionsSlice", &dump_slice(&call(ctx, "OptionsSlice")));
}

fn dump_position(b: &mut Vec<u8>, ctx: &Value) {
    let p = call(ctx, "Position");
    let o = p.as_object().expect("position");
    assert_eq!(o.type_name(), "text.Position");
    let mut v = s(&o.field("Filename").unwrap());
    v.extend_from_slice(
        format!(
            "|{}|{}|{}",
            int(&o.field("LineNumber").unwrap()),
            int(&o.field("ColumnNumber").unwrap()),
            int(&o.field("Offset").unwrap())
        )
        .as_bytes(),
    );
    field(b, "Position", &v);
}

/// `mdoracle.resolver`.
fn resolve(sample: &[u8]) -> Option<Position> {
    Some(Position {
        filename: format!(
            "sample={}",
            String::from_utf8(sample.to_vec()).expect("UTF-8 sample")
        ),
        line_number: sample.len() as i64,
        column_number: 7,
        offset: -1,
    })
}

pub struct LinkRec(pub Arc<Recorder>);

impl LinkRenderer for LinkRec {
    fn render_link(&self, _cctx: HostCtx<'_>, w: &mut Vec<u8>, ctx: &Value) -> Result<()> {
        let o = ctx.as_object().expect("object");
        let is_image = o.type_name() == "goldmark.imageLinkContext";
        if !is_image {
            assert_eq!(o.type_name(), "goldmark.linkContext");
        }
        let (seq, mut b) = self.0.next(if is_image { "image" } else { "link" }, ctx);
        let dest = s(&call(ctx, "Destination"));
        let text = s(&call(ctx, "Text"));
        field(&mut b, "Destination", &dest);
        field(&mut b, "Title", &s(&call(ctx, "Title")));
        field(&mut b, "Text", &text);
        field(&mut b, "PlainText", &s(&call(ctx, "PlainText")));
        dump_page(&mut b, ctx);
        dump_attrs(&mut b, ctx);
        if is_image {
            field(&mut b, "IsBlock", &dump_val(&call(ctx, "IsBlock")));
            field(&mut b, "Ordinal", &dump_val(&call(ctx, "Ordinal")));
        }
        self.0.done(b);
        if dest == b"hook-error" {
            return Err(Error::new("link hook failed"));
        }
        if is_image {
            w.extend_from_slice(format!("[I{seq}]").as_bytes());
        } else {
            w.extend_from_slice(format!("[L{seq}|").as_bytes());
            w.extend_from_slice(&text);
            w.push(b']');
        }
        Ok(())
    }
}

pub struct HeadingRec(pub Arc<Recorder>);

impl HeadingRenderer for HeadingRec {
    fn render_heading(&self, _cctx: HostCtx<'_>, w: &mut Vec<u8>, ctx: &Value) -> Result<()> {
        assert_eq!(
            ctx.as_object().unwrap().type_name(),
            "goldmark.headingContext"
        );
        let (_, mut b) = self.0.next("heading", ctx);
        let level = int(&call(ctx, "Level"));
        let anchor = s(&call(ctx, "Anchor"));
        let text = s(&call(ctx, "Text"));
        field(&mut b, "Level", &dump_val(&call(ctx, "Level")));
        field(&mut b, "Anchor", &anchor);
        field(&mut b, "Text", &text);
        field(&mut b, "PlainText", &s(&call(ctx, "PlainText")));
        dump_page(&mut b, ctx);
        dump_attrs(&mut b, ctx);
        self.0.done(b);
        w.extend_from_slice(format!("<h{level} id=\"").as_bytes());
        w.extend_from_slice(&anchor);
        w.extend_from_slice(b"\">");
        w.extend_from_slice(&text);
        w.extend_from_slice(format!("</h{level}>\n").as_bytes());
        Ok(())
    }
}

pub struct BlockquoteRec(pub Arc<Recorder>);

impl BlockquoteRenderer for BlockquoteRec {
    fn render_blockquote(&self, _cctx: HostCtx<'_>, w: &mut Vec<u8>, ctx: &Value) -> Result<()> {
        assert_eq!(
            ctx.as_object().unwrap().type_name(),
            "*blockquotes.blockquoteContext"
        );
        let (seq, mut b) = self.0.next("blockquote", ctx);
        let text = s(&call(ctx, "Text"));
        field(&mut b, "Type", &s(&call(ctx, "Type")));
        field(&mut b, "AlertType", &s(&call(ctx, "AlertType")));
        field(&mut b, "AlertTitle", &s(&call(ctx, "AlertTitle")));
        field(&mut b, "AlertSign", &s(&call(ctx, "AlertSign")));
        field(&mut b, "Text", &text);
        field(&mut b, "Ordinal", &dump_val(&call(ctx, "Ordinal")));
        dump_page(&mut b, ctx);
        dump_position(&mut b, ctx);
        dump_attrs(&mut b, ctx);
        self.0.done(b);
        if text.windows(5).any(|x| x == b"BQERR") {
            return Err(Error::new("blockquote hook failed"));
        }
        w.extend_from_slice(format!("<bq{seq}>").as_bytes());
        w.extend_from_slice(&text);
        w.extend_from_slice(b"</bq>\n");
        Ok(())
    }

    fn resolve_position(&self, sample: &[u8]) -> Option<Position> {
        resolve(sample)
    }
}

pub struct TableRec(pub Arc<Recorder>);

impl TableRenderer for TableRec {
    fn render_table(&self, _cctx: HostCtx<'_>, w: &mut Vec<u8>, ctx: &Value) -> Result<()> {
        assert_eq!(ctx.as_object().unwrap().type_name(), "*tables.tableContext");
        let (_, mut b) = self.0.next("table", ctx);
        field(&mut b, "Ordinal", &dump_val(&call(ctx, "Ordinal")));
        dump_page(&mut b, ctx);
        dump_position(&mut b, ctx);
        dump_attrs(&mut b, ctx);
        for name in ["THead", "TBody"] {
            let mut rb = Vec::new();
            match rows(&call(ctx, name)) {
                None => rb.extend_from_slice(b"nil"),
                Some(rows) => {
                    for row in rows {
                        rb.push(b'[');
                        for (align, text) in row {
                            field_b(&mut rb, &align, &text);
                        }
                        rb.push(b']');
                    }
                }
            }
            field(&mut b, name, &rb);
        }
        self.0.done(b);
        render_table_replica(w, ctx);
        Ok(())
    }

    fn resolve_position(&self, sample: &[u8]) -> Option<Position> {
        resolve(sample)
    }
}

pub struct CodeRec(pub Arc<Recorder>, pub bool);

impl CodeBlockRenderer for CodeRec {
    fn render_codeblock(&self, _cctx: HostCtx<'_>, w: &mut Vec<u8>, ctx: &Value) -> Result<()> {
        assert_eq!(
            ctx.as_object().unwrap().type_name(),
            "*codeblocks.codeBlockContext"
        );
        let (seq, mut b) = self.0.next("codeblock", ctx);
        let typ = s(&call(ctx, "Type"));
        let inner = s(&call(ctx, "Inner"));
        field(&mut b, "Type", &typ);
        field(&mut b, "Inner", &inner);
        field(&mut b, "Ordinal", &dump_val(&call(ctx, "Ordinal")));
        dump_page(&mut b, ctx);
        dump_position(&mut b, ctx);
        dump_attrs(&mut b, ctx);
        self.0.done(b);
        if typ == b"errlang" {
            return Err(Error::new("code block hook failed"));
        }
        w.extend_from_slice(format!("<pre{seq} lang=\"").as_bytes());
        w.extend_from_slice(&typ);
        w.extend_from_slice(b"\">");
        w.extend_from_slice(&inner);
        w.extend_from_slice(b"</pre>\n");
        Ok(())
    }

    fn is_default_code_block_renderer(&self) -> bool {
        self.1
    }

    fn resolve_position(&self, sample: &[u8]) -> Option<Position> {
        resolve(sample)
    }
}

/// `mdoracle.RecordingRenderers`.
pub fn recording_renderers(r: Arc<Recorder>) -> GetRendererFunc {
    Arc::new(move |t, id| match t {
        RendererType::Link | RendererType::Image => {
            Some(Renderer::Link(Arc::new(LinkRec(r.clone()))))
        }
        RendererType::Heading => Some(Renderer::Heading(Arc::new(HeadingRec(r.clone())))),
        RendererType::Blockquote => Some(Renderer::Blockquote(Arc::new(BlockquoteRec(r.clone())))),
        RendererType::Table => Some(Renderer::Table(Arc::new(TableRec(r.clone())))),
        RendererType::CodeBlock => {
            let lang = s(id);
            if lang == b"nohook" {
                return None;
            }
            Some(Renderer::CodeBlock(Arc::new(CodeRec(
                r.clone(),
                lang.starts_with(b"d"),
            ))))
        }
        _ => None,
    })
}

// ---------------------------------------------------------------------------
// Values for the page objects of the hooks oracle.

pub fn page_value(s: &str) -> Value {
    Value::String(GoString::new(s.as_bytes().to_vec()))
}

/// Kept for symmetry with the Go objects (unused helpers silence).
pub fn kind_of(v: &Value) -> Option<Kind> {
    v.as_object().map(|o| o.kind())
}

pub fn object_type(v: &Value) -> String {
    v.as_object()
        .map(|o| o.type_name().into_owned())
        .unwrap_or_default()
}

pub fn go_type(o: &dyn Object) -> String {
    o.type_name().into_owned()
}

/// `mdoracle.DumpMarkupConfig`.
pub fn dump_markup_config(m: &nh_markup::markup_config::Config) -> String {
    let mut b = String::new();
    let mut w = |k: &str, v: String| {
        b.push_str("Markup.");
        b.push_str(k);
        b.push('=');
        b.push_str(&v);
        b.push('\n');
    };
    w("DefaultMarkdownHandler", m.default_markdown_handler.clone());
    let h = &m.highlight;
    w("Highlight.Style", h.style.clone());
    w("Highlight.CodeFences", h.code_fences.to_string());
    w("Highlight.WrapperClass", h.wrapper_class.clone());
    w("Highlight.NoClasses", h.no_classes.to_string());
    w("Highlight.LineNos", h.line_nos.to_string());
    w(
        "Highlight.LineNumbersInTable",
        h.line_numbers_in_table.to_string(),
    );
    w("Highlight.AnchorLineNos", h.anchor_line_nos.to_string());
    w("Highlight.LineAnchors", h.line_anchors.clone());
    w("Highlight.LineNoStart", h.line_no_start.to_string());
    w("Highlight.Hl_Lines", h.hl_lines.clone());
    w("Highlight.Hl_inline", h.hl_inline.to_string());
    w(
        "Highlight.HL_lines_parsed",
        format!("len {}", h.hl_lines_parsed.len()),
    );
    for (i, r) in h.hl_lines_parsed.iter().enumerate() {
        w(&format!("Highlight.HL_lines_parsed[{i}]"), "len 2".into());
        w(
            &format!("Highlight.HL_lines_parsed[{i}][0]"),
            r[0].to_string(),
        );
        w(
            &format!("Highlight.HL_lines_parsed[{i}][1]"),
            r[1].to_string(),
        );
    }
    w("Highlight.TabWidth", h.tab_width.to_string());
    w("Highlight.GuessSyntax", h.guess_syntax.to_string());
    let t = &m.table_of_contents;
    w("TableOfContents.StartLevel", t.start_level.to_string());
    w("TableOfContents.EndLevel", t.end_level.to_string());
    w("TableOfContents.Ordered", t.ordered.to_string());
    let g = &m.goldmark;
    w(
        "Goldmark.Renderer.HardWraps",
        g.renderer.hard_wraps.to_string(),
    );
    w("Goldmark.Renderer.XHTML", g.renderer.xhtml.to_string());
    w("Goldmark.Renderer.Unsafe", g.renderer.unsafe_.to_string());
    let p = &g.parser;
    w(
        "Goldmark.Parser.AutoHeadingID",
        p.auto_heading_id.to_string(),
    );
    w(
        "Goldmark.Parser.AutoDefinitionTermID",
        p.auto_definition_term_id.to_string(),
    );
    w("Goldmark.Parser.AutoIDType", p.auto_id_type.clone());
    w(
        "Goldmark.Parser.Attribute.Title",
        p.attribute.title.to_string(),
    );
    w(
        "Goldmark.Parser.Attribute.Block",
        p.attribute.block.to_string(),
    );
    w(
        "Goldmark.Parser.WrapStandAloneImageWithinParagraph",
        p.wrap_stand_alone_image_within_paragraph.to_string(),
    );
    w(
        "Goldmark.Parser.AutoHeadingIDType",
        p.auto_heading_id_type.clone(),
    );
    let e = &g.extensions;
    let ty = &e.typographer;
    w(
        "Goldmark.Extensions.Typographer.Disable",
        ty.disable.to_string(),
    );
    w(
        "Goldmark.Extensions.Typographer.LeftSingleQuote",
        ty.left_single_quote.clone(),
    );
    w(
        "Goldmark.Extensions.Typographer.RightSingleQuote",
        ty.right_single_quote.clone(),
    );
    w(
        "Goldmark.Extensions.Typographer.LeftDoubleQuote",
        ty.left_double_quote.clone(),
    );
    w(
        "Goldmark.Extensions.Typographer.RightDoubleQuote",
        ty.right_double_quote.clone(),
    );
    w("Goldmark.Extensions.Typographer.EnDash", ty.en_dash.clone());
    w("Goldmark.Extensions.Typographer.EmDash", ty.em_dash.clone());
    w(
        "Goldmark.Extensions.Typographer.Ellipsis",
        ty.ellipsis.clone(),
    );
    w(
        "Goldmark.Extensions.Typographer.LeftAngleQuote",
        ty.left_angle_quote.clone(),
    );
    w(
        "Goldmark.Extensions.Typographer.RightAngleQuote",
        ty.right_angle_quote.clone(),
    );
    w(
        "Goldmark.Extensions.Typographer.Apostrophe",
        ty.apostrophe.clone(),
    );
    w("Goldmark.Extensions.Footnote", e.footnote.to_string());
    w(
        "Goldmark.Extensions.DefinitionList",
        e.definition_list.to_string(),
    );
    w(
        "Goldmark.Extensions.Extras.Delete.Enable",
        e.extras.delete.enable.to_string(),
    );
    w(
        "Goldmark.Extensions.Extras.Insert.Enable",
        e.extras.insert.enable.to_string(),
    );
    w(
        "Goldmark.Extensions.Extras.Mark.Enable",
        e.extras.mark.enable.to_string(),
    );
    w(
        "Goldmark.Extensions.Extras.Subscript.Enable",
        e.extras.subscript.enable.to_string(),
    );
    w(
        "Goldmark.Extensions.Extras.Superscript.Enable",
        e.extras.superscript.enable.to_string(),
    );
    w(
        "Goldmark.Extensions.Passthrough.Enable",
        e.passthrough.enable.to_string(),
    );
    for (name, d) in [
        ("Inline", &e.passthrough.delimiters.inline),
        ("Block", &e.passthrough.delimiters.block),
    ] {
        w(
            &format!("Goldmark.Extensions.Passthrough.Delimiters.{name}"),
            format!("len {}", d.len()),
        );
        for (i, x) in d.iter().enumerate() {
            w(
                &format!("Goldmark.Extensions.Passthrough.Delimiters.{name}[{i}]"),
                format!("len {}", x.len()),
            );
            for (j, y) in x.iter().enumerate() {
                w(
                    &format!("Goldmark.Extensions.Passthrough.Delimiters.{name}[{i}][{j}]"),
                    y.clone(),
                );
            }
        }
    }
    w("Goldmark.Extensions.Table", e.table.to_string());
    w(
        "Goldmark.Extensions.Strikethrough",
        e.strikethrough.to_string(),
    );
    w("Goldmark.Extensions.Linkify", e.linkify.to_string());
    w(
        "Goldmark.Extensions.LinkifyProtocol",
        e.linkify_protocol.clone(),
    );
    w("Goldmark.Extensions.TaskList", e.task_list.to_string());
    w("Goldmark.Extensions.CJK.Enable", e.cjk.enable.to_string());
    w(
        "Goldmark.Extensions.CJK.EastAsianLineBreaks",
        e.cjk.east_asian_line_breaks.to_string(),
    );
    w(
        "Goldmark.Extensions.CJK.EastAsianLineBreaksStyle",
        e.cjk.east_asian_line_breaks_style.clone(),
    );
    w(
        "Goldmark.Extensions.CJK.EscapedSpace",
        e.cjk.escaped_space.to_string(),
    );
    w(
        "Goldmark.DuplicateResourceFiles",
        g.duplicate_resource_files.to_string(),
    );
    for (name, h) in [
        ("Image", &g.render_hooks.image),
        ("Link", &g.render_hooks.link),
    ] {
        match &h.enable_default {
            None => w(
                &format!("Goldmark.RenderHooks.{name}.EnableDefault"),
                "nil".into(),
            ),
            Some(b) => w(
                &format!("Goldmark.RenderHooks.{name}.EnableDefault*"),
                b.to_string(),
            ),
        }
        w(
            &format!("Goldmark.RenderHooks.{name}.UseEmbedded"),
            h.use_embedded.clone(),
        );
    }
    let a = &m.asciidoc_ext;
    w("AsciidocExt.Backend", a.backend.clone());
    w(
        "AsciidocExt.Extensions",
        format!("len {}", a.extensions.len()),
    );
    for (i, x) in a.extensions.iter().enumerate() {
        w(&format!("AsciidocExt.Extensions[{i}]"), x.clone());
    }
    w(
        "AsciidocExt.Attributes",
        format!("len {}", a.attributes.len()),
    );
    for (k, v) in &a.attributes {
        w(&format!("AsciidocExt.Attributes[{k}]"), v.clone());
    }
    w(
        "AsciidocExt.NoHeaderOrFooter",
        a.no_header_or_footer.to_string(),
    );
    w("AsciidocExt.SafeMode", a.safe_mode.clone());
    w("AsciidocExt.SectionNumbers", a.section_numbers.to_string());
    w("AsciidocExt.Verbose", a.verbose.to_string());
    w("AsciidocExt.Trace", a.trace.to_string());
    w("AsciidocExt.FailureLevel", a.failure_level.clone());
    w(
        "AsciidocExt.WorkingFolderCurrent",
        a.working_folder_current.to_string(),
    );
    w("AsciidocExt.PreserveTOC", a.preserve_toc.to_string());
    b
}
