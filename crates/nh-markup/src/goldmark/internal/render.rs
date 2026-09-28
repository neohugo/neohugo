//! Port of `markup/goldmark/internal/render/context.go`.
//!
//! Owner: Wave B task T06 (markup).

//! Go `markup/goldmark/internal/render/context.go`: the render buffer with a position stack used
//! to capture hook `.Text` (push position on enter, pop the rendered slice on exit), per-kind
//! ordinals and value stacks, and `TextPlain` (with its first-child-only quirk).
//!
//! Go passes the `*render.Context` to goldmark as its `util.BufWriter` and Hugo's renderers
//! type-assert it back. Here the Hugo renderers receive the [`Context`] directly (see
//! `goldmark::convert`, which dispatches the node renderers itself), and goldmark's own
//! renderers write into the context's [`BufWriter`].

use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use go_unicode::utf8;
use go_value::Value;
use goldmark::ast::{Ast, NodeId, NodeType, NodeValue};
use nh_common::text::Position;

use crate::converter::converter::{DocumentContext, RenderContext};
use crate::converter::hooks::{GetRendererFunc, Renderer, RendererType};

/// Go: `render.BufWriter` (a `*bytes.Buffer` with `Available`/`Buffered`/`Flush`).
#[derive(Default)]
pub struct BufWriter {
    pub buf: Vec<u8>,
}

impl goldmark::util::BufWriter for BufWriter {
    fn write(&mut self, p: &[u8]) {
        self.buf.extend_from_slice(p);
    }
    // Go: markup/goldmark/internal/render/context.go:Available
    fn available(&self) -> i64 {
        i64::MAX
    }
    // Go: markup/goldmark/internal/render/context.go:Buffered
    fn buffered(&self) -> i64 {
        self.buf.len() as i64
    }
    // Go: markup/goldmark/internal/render/context.go:Flush
    fn flush(&mut self) -> Result<(), goldmark::Error> {
        Ok(())
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Go: `render.RenderContextDataHolder` (the `ContextData` of a [`Context`]).
pub struct RenderContextDataHolder<'a> {
    pub rctx: &'a RenderContext<'a>,
    pub dctx: &'a DocumentContext,
}

impl<'a> RenderContextDataHolder<'a> {
    // Go: markup/goldmark/internal/render/context.go:RenderContext
    pub fn render_context(&self) -> &'a RenderContext<'a> {
        self.rctx
    }

    // Go: markup/goldmark/internal/render/context.go:DocumentContext
    pub fn document_context(&self) -> &'a DocumentContext {
        self.dctx
    }
}

/// Go: `render.Context`.
pub struct Context<'a> {
    pub w: BufWriter,
    pub data: RenderContextDataHolder<'a>,
    pub(crate) positions: Vec<usize>,
    pub(crate) pids: Vec<u64>,
    pub(crate) ordinals: HashMap<goldmark::ast::NodeKind, i64>,
    pub(crate) values: HashMap<goldmark::ast::NodeKind, Vec<Box<dyn Any + Send>>>,
}

impl<'a> Context<'a> {
    pub fn new(data: RenderContextDataHolder<'a>) -> Self {
        Context {
            w: BufWriter::default(),
            data,
            positions: Vec::new(),
            pids: Vec::new(),
            ordinals: HashMap::new(),
            values: HashMap::new(),
        }
    }

    /// Go: `ctx.Len()` of the embedded buffer.
    pub fn len(&self) -> usize {
        self.w.buf.len()
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.w.buf.is_empty()
    }

    /// Go: `ctx.RenderContext()`.
    pub fn render_context(&self) -> &'a RenderContext<'a> {
        self.data.render_context()
    }

    /// Go: `ctx.DocumentContext()`.
    pub fn document_context(&self) -> &'a DocumentContext {
        self.data.document_context()
    }

    /// Go: `ctx.RenderContext().GetRenderer(t, id)` (a nil `GetRenderer` func panics in Go).
    pub fn get_renderer(&self, t: RendererType, id: &Value) -> Option<Renderer> {
        let f: &GetRendererFunc = self
            .render_context()
            .get_renderer
            .as_ref()
            .expect("invalid memory address or nil pointer dereference: nil GetRenderer");
        f(t, id)
    }

    // Go: markup/goldmark/internal/render/context.go:GetAndIncrementOrdinal
    pub fn get_and_increment_ordinal(&mut self, kind: goldmark::ast::NodeKind) -> i64 {
        let e = self.ordinals.entry(kind).or_insert(0);
        let v = *e;
        *e += 1;
        v
    }

    // Go: markup/goldmark/internal/render/context.go:PushPos
    pub fn push_pos(&mut self, n: usize) {
        self.positions.push(n);
    }

    // Go: markup/goldmark/internal/render/context.go:PopPos
    pub fn pop_pos(&mut self) -> usize {
        // Go indexes `positions[len-1]` and panics on an empty stack.
        self.positions.pop().expect("index out of range [-1]")
    }

    /// Go: `PopRenderedString` — `buf[pos:]` as string, then `Truncate(pos)`.
    // Go: markup/goldmark/internal/render/context.go:PopRenderedString
    pub fn pop_rendered_string(&mut self) -> Vec<u8> {
        let pos = self.pop_pos();
        let s = self.w.buf[pos..].to_vec();
        self.w.buf.truncate(pos);
        s
    }

    /// PushPid pushes a new page ID to the stack.
    // Go: markup/goldmark/internal/render/context.go:PushPid
    pub fn push_pid(&mut self, pid: u64) {
        self.pids.push(pid);
    }

    /// PeekPid returns the current page ID without removing it from the stack.
    // Go: markup/goldmark/internal/render/context.go:PeekPid
    pub fn peek_pid(&self) -> u64 {
        self.pids.last().copied().unwrap_or(0)
    }

    /// PopPid pops the last page ID from the stack.
    // Go: markup/goldmark/internal/render/context.go:PopPid
    pub fn pop_pid(&mut self) -> u64 {
        self.pids.pop().unwrap_or(0)
    }

    // Go: markup/goldmark/internal/render/context.go:PushValue
    pub fn push_value(&mut self, k: goldmark::ast::NodeKind, v: Box<dyn Any + Send>) {
        self.values.entry(k).or_default().push(v);
    }

    // Go: markup/goldmark/internal/render/context.go:PopValue
    pub fn pop_value(&mut self, k: goldmark::ast::NodeKind) -> Option<Box<dyn Any + Send>> {
        self.values.get_mut(&k).and_then(|v| v.pop())
    }

    /// Go returns the stored pointer; the port returns the stored value mutably.
    // Go: markup/goldmark/internal/render/context.go:PeekValue
    pub fn peek_value(&mut self, k: goldmark::ast::NodeKind) -> Option<&mut Box<dyn Any + Send>> {
        self.values.get_mut(&k).and_then(|v| v.last_mut())
    }
}

/// GetPageAndPageInner returns the current page and the inner page for the given context.
// Go: markup/goldmark/internal/render/context.go:GetPageAndPageInner
pub fn get_page_and_page_inner(rctx: &Context<'_>) -> (Value, Value) {
    let p = rctx.document_context().document.clone();
    let pid = rctx.peek_pid();
    if pid > 0
        && let Some(lookup) = &rctx.document_context().document_lookup
    {
        let v = lookup(pid);
        // Go: `if v := lookup(pid); v != nil`.
        if !v.is_invalid() {
            return (p, v);
        }
    }
    (p.clone(), p)
}

/// A position resolver (Go `hooks.ElementPositionResolver`, which hugolib's hook renderers
/// implement): gets the context's `PositionerSourceTarget` bytes.
pub type PositionResolver = Arc<dyn Fn(&[u8]) -> Option<Position> + Send + Sync>;

/// Go: `render.hookBase` (the `hooks.BaseContext` of table, blockquote, code block and
/// passthrough contexts).
pub struct HookBase {
    pub page: Value,
    pub page_inner: Value,
    pub ordinal: i64,
    /// Go `getSourceSample()` (computed eagerly; it is a slice of the source).
    pub source_sample: Vec<u8>,
    filename: String,
    resolver: Option<PositionResolver>,
    pos: OnceLock<Position>,
}

impl HookBase {
    // Go: markup/goldmark/internal/render/context.go:(*hookBase).Page
    pub fn page(&self) -> Value {
        self.page.clone()
    }

    // Go: markup/goldmark/internal/render/context.go:(*hookBase).PageInner
    pub fn page_inner(&self) -> Value {
        self.page_inner.clone()
    }

    // Go: markup/goldmark/internal/render/context.go:(*hookBase).Ordinal
    pub fn ordinal(&self) -> i64 {
        self.ordinal
    }

    /// Resolved on first use (Go: `sync.Once`).
    // Go: markup/goldmark/internal/render/context.go:(*hookBase).Position
    pub fn position(&self) -> Position {
        self.pos
            .get_or_init(|| {
                if let Some(r) = &self.resolver
                    && let Some(p) = r(&self.source_sample)
                {
                    return p;
                }
                Position {
                    filename: self.filename.clone(),
                    line_number: 1,
                    column_number: 1,
                    offset: 0,
                }
            })
            .clone()
    }

    /// For internal use.
    // Go: markup/goldmark/internal/render/context.go:(*hookBase).PositionerSourceTarget
    pub fn positioner_source_target(&self) -> &[u8] {
        &self.source_sample
    }
}

/// NewBaseContext creates a new BaseContext. `get_source_sample` is Go's optional
/// `getSourceSample` (`None` = `extractSourceSample(n, src)`); `resolver` is the renderer's
/// `ElementPositionResolver`, if it is one.
// Go: markup/goldmark/internal/render/context.go:NewBaseContext
pub fn new_base_context(
    rctx: &Context<'_>,
    resolver: Option<PositionResolver>,
    ast: &Ast,
    n: NodeId,
    src: &[u8],
    get_source_sample: Option<Vec<u8>>,
    ordinal: i64,
) -> HookBase {
    let source_sample = match get_source_sample {
        Some(s) => s,
        None => extract_source_sample(ast, n, src),
    };
    let (page, page_inner) = get_page_and_page_inner(rctx);
    HookBase {
        page,
        page_inner,
        ordinal,
        source_sample,
        filename: rctx.document_context().filename.clone(),
        resolver,
        pos: OnceLock::new(),
    }
}

/// extractSourceSample returns a sample of the source for the given node.
// Go: markup/goldmark/internal/render/context.go:extractSourceSample
fn extract_source_sample(ast: &Ast, n: NodeId, src: &[u8]) -> Vec<u8> {
    if ast.typ(n) == NodeType::Inline {
        // Go returns the segment of a `*passthrough.PassthroughInline` (passthrough is not
        // supported, so no such node exists) and nil for every other inline node.
        return Vec::new();
    }

    let get_start_stop = |n: Option<NodeId>| -> (i64, i64) {
        let Some(n) = n else {
            return (0, 0);
        };
        let (mut start, mut stop) = (0, 0);
        let lines = ast.lines(n);
        let mut i = 0;
        while i < lines.len() && i < 2 {
            let line = lines.at(i);
            if i == 0 {
                start = line.start;
            }
            stop = line.stop;
            i += 1;
        }
        (start, stop)
    };

    let (mut start, mut stop) = get_start_stop(Some(n));
    if stop == 0 {
        // Try first child.
        (start, stop) = get_start_stop(ast.first_child(n));
    }

    if stop > 0 {
        return src[start as usize..stop as usize].to_vec();
    }
    Vec::new()
}

/// Go: `render.TextPlain(n, source)` — a plain text representation of the given node,
/// resolving leftover named HTML entities. Follows only the FIRST child of non-text inline
/// nodes.
// Go: markup/goldmark/internal/render/context.go:TextPlain
pub fn text_plain(ast: &Ast, n: NodeId, source: &[u8]) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut c = ast.first_child(n);
    while let Some(cc) = c {
        text_plain_to(ast, Some(cc), source, &mut buf);
        c = ast.next_sibling(cc);
    }
    goldmark::util::resolve_entity_names(&buf).into_owned()
}

/// Go recurses into `c.FirstChild()` for other node types; this is a tail call, so the port
/// loops (deep inline nesting cannot overflow the stack).
// Go: markup/goldmark/internal/render/context.go:textPlainTo
fn text_plain_to(ast: &Ast, mut c: Option<NodeId>, source: &[u8], buf: &mut Vec<u8>) {
    while let Some(n) = c {
        match ast.value(n) {
            NodeValue::RawHTML(r) => {
                let s = strip_html(&r.segments.value(source));
                buf.extend_from_slice(go_unicode::strings::trim_space(&s));
                return;
            }
            NodeValue::String(s) => {
                buf.extend_from_slice(&s.value);
                return;
            }
            NodeValue::Text(t) => {
                buf.extend_from_slice(&t.segment.value(source));
                if t.hard_line_break() || t.soft_line_break() {
                    buf.push(b'\n');
                }
                return;
            }
            // `*east.Emoji` (goldmark-emoji) is not supported: the emoji extension is an
            // explicit error in `newMarkdown`, so no such node exists.
            _ => c = ast.first_child(n),
        }
    }
}

const HUGO_NEW_LINE_PLACEHOLDER: &[u8] = b"___hugonl_";

/// Go: `tpl.StripHTML(s)` (tpl/template.go), which `TextPlain` calls for raw HTML: tags are
/// stripped with html/template's `stripTags` state machine, `</p>`/`<br>`/`<br />` become
/// newlines and runs of `unicode.IsSpace` runes collapse to their first rune.
///
/// nh-tpl owns the Go function (T13); it is ported here privately because nh-tpl's version is
/// not available yet.
// Go: tpl/template.go:StripHTML
pub(crate) fn strip_html(s: &[u8]) -> Vec<u8> {
    // Shortcut strings with no tags in them
    if !s.iter().any(|&c| c == b'<' || c == b'>') {
        return s.to_vec();
    }

    // strings.NewReplacer("\n", " ", "</p>", placeholder, "<br>", placeholder,
    // "<br />", placeholder).Replace(s)
    let mut pre = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        let rest = &s[i..];
        if rest[0] == b'\n' {
            pre.push(b' ');
            i += 1;
        } else if rest.starts_with(b"</p>") || rest.starts_with(b"<br>") {
            pre.extend_from_slice(HUGO_NEW_LINE_PLACEHOLDER);
            i += 4;
        } else if rest.starts_with(b"<br />") {
            pre.extend_from_slice(HUGO_NEW_LINE_PLACEHOLDER);
            i += 6;
        } else {
            pre.push(rest[0]);
            i += 1;
        }
    }
    let pre_replaced = pre != s;

    let mut s = gotemplate::html::strip_tags(&pre);

    if pre_replaced {
        s = replace_all(&s, HUGO_NEW_LINE_PLACEHOLDER, b"\n");
    }

    let mut was_space = false;
    let mut b = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        let (r, size) = utf8::decode_rune(&s[i..]);
        let is_space = go_unicode::is_space(r);
        if !is_space || !was_space {
            // Go `WriteRune`: an invalid byte (decoded as U+FFFD) is written as U+FFFD.
            utf8::append_rune(&mut b, r);
        }
        was_space = is_space;
        i += size;
    }

    if !b.is_empty() {
        s = b;
    }

    s
}

fn replace_all(s: &[u8], old: &[u8], new: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        if s[i..].starts_with(old) {
            out.extend_from_slice(new);
            i += old.len();
        } else {
            out.push(s[i]);
            i += 1;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/goldmark/internal/render/context.go (327 lines; 15/25 funcs executed)
//   types: BufWriter, Context, ContextData, RenderContextDataHolder, hookBase
// OK L41-43: (b *BufWriter) Available() int
// OK L45-47: (b *BufWriter) Buffered() int
// OK L49-51: (b *BufWriter) Flush() error
// OK L62-69: (ctx *Context) GetAndIncrementOrdinal(kind ast.NodeKind) int
// OK L71-73: (ctx *Context) PushPos(n int)
// OK L75-80: (ctx *Context) PopPos() int
// OK L82-87: (ctx *Context) PopRenderedString() string
// OK L90-92: (ctx *Context) PushPid(pid uint64)
// OK L95-100: (ctx *Context) PeekPid() uint64
// OK L103-111: (ctx *Context) PopPid() uint64
// OK L113-118: (ctx *Context) PushValue(k ast.NodeKind, v any)
// OK L120-132: (ctx *Context) PopValue(k ast.NodeKind) any
// OK L134-143: (ctx *Context) PeekValue(k ast.NodeKind) any
// OK L155-157: (ctx *RenderContextDataHolder) RenderContext() converter.RenderContext
// OK L159-161: (ctx *RenderContextDataHolder) DocumentContext() converter.DocumentContext
// OK L166-206: extractSourceSample(n ast.Node, src []byte) []byte
// OK L209-220: GetPageAndPageInner(rctx *Context) (any, any)
// OK L223-251: NewBaseContext(rctx *Context, renderer any, n ast.Node, src []byte, getSourceSample func() []byte, ordinal int) hooks.BaseContext
// OK L268-270: (c *hookBase) Page() any
// OK L272-274: (c *hookBase) PageInner() any
// OK L276-278: (c *hookBase) Ordinal() int
// OK L280-285: (c *hookBase) Position() htext.Position
// OK L288-290: (c *hookBase) PositionerSourceTarget() []byte
// OK L296-304: TextPlain(n ast.Node, source []byte) string
// OK L306-327: textPlainTo(c ast.Node, source []byte, buf *bytes.Buffer)
// ---------------------------------------------------------------------------
