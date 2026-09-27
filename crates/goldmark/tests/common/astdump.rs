//! The AST dump of tools/go-oracle/goldmark/astdump.go: every node's kind,
//! type, child count, lines, attributes (with typed values), node-specific
//! fields and (optionally) the deprecated `Text` value, one line per node in
//! pre-order. It pins the API surface Hugo's nh-markup reads (segments,
//! attributes, heading/list/link fields), which the HTML does not show.

use goldmark::ast::{Ast, AttrValue, Attribute, NodeId, NodeType, NodeValue};
use goldmark::extension::ast as east;
use goldmark::text::Segment;

fn q(out: &mut Vec<u8>, b: &[u8]) {
    out.push(b'"');
    for &c in b {
        if (0x20..0x7f).contains(&c) && c != b'"' && c != b'\\' {
            out.push(c);
        } else {
            out.extend_from_slice(format!("\\x{c:02x}").as_bytes());
        }
    }
    out.push(b'"');
}

fn seg(out: &mut Vec<u8>, s: &Segment) {
    out.extend_from_slice(format!("{}:{}:{}", s.start, s.stop, s.padding).as_bytes());
    if s.force_newline {
        out.extend_from_slice(b":n");
    }
}

fn b(v: bool) -> &'static str {
    if v { "1" } else { "0" }
}

/// A piece of an attribute value dump (explicit stack: values can be nested
/// as deep as the input).
enum Piece<'a> {
    Value(&'a AttrValue),
    List(&'a [Attribute]),
    Name(&'a [u8]),
    Lit(&'static [u8]),
}

fn attr_pieces(out: &mut Vec<u8>, first: Piece<'_>) {
    let mut stack = vec![first];
    while let Some(p) = stack.pop() {
        match p {
            Piece::Lit(s) => out.extend_from_slice(s),
            Piece::Name(n) => {
                q(out, n);
                out.push(b'=');
            }
            Piece::List(a) => {
                out.push(b'{');
                stack.push(Piece::Lit(b"}"));
                for (i, x) in a.iter().enumerate().rev() {
                    stack.push(Piece::Value(&x.value));
                    stack.push(Piece::Name(&x.name));
                    if i > 0 {
                        stack.push(Piece::Lit(b";"));
                    }
                }
            }
            Piece::Value(v) => match v {
                AttrValue::Bytes(x) => {
                    out.push(b'b');
                    q(out, x);
                }
                AttrValue::String(x) => {
                    out.push(b's');
                    q(out, x);
                }
                AttrValue::Float(f) => {
                    out.extend_from_slice(format!("f{:016x}", f.to_bits()).as_bytes())
                }
                AttrValue::Bool(x) => out.extend_from_slice(if *x { b"true" } else { b"false" }),
                AttrValue::Nil => out.extend_from_slice(b"nil"),
                AttrValue::Array(a) => {
                    out.push(b'[');
                    stack.push(Piece::Lit(b"]"));
                    for (i, x) in a.iter().enumerate().rev() {
                        stack.push(Piece::Value(x));
                        if i > 0 {
                            stack.push(Piece::Lit(b","));
                        }
                    }
                }
                AttrValue::Attributes(a) => stack.push(Piece::List(a)),
                AttrValue::Other(_) => out.push(b'?'),
            },
        }
    }
}

fn attr_list(out: &mut Vec<u8>, a: &[Attribute]) {
    attr_pieces(out, Piece::List(a));
}

/// tools/go-oracle/goldmark/redteam.go:attrVecRecord: `parser.ParseAttributes`
/// on a text reader and on a block reader over the input's lines; the
/// result and the reader position afterwards.
pub fn attr_vec(input: &[u8], block: bool) -> Vec<u8> {
    use goldmark::text::{self, Reader};
    let r = std::panic::catch_unwind(|| {
        let mut segs = text::new_segments();
        let mut off = 0;
        while off < input.len() {
            let end = match input[off..].iter().position(|&c| c == b'\n') {
                Some(i) => off + i + 1,
                None => input.len(),
            };
            segs.append(text::new_segment(off as i64, end as i64));
            off = end;
        }
        let mut tr = text::new_reader(input);
        let mut br = text::new_block_reader(input, Some(&segs));
        let rd: &mut dyn Reader = if block { &mut br } else { &mut tr };
        let mut out = Vec::new();
        match goldmark::parser::parse_attributes(rd) {
            Some(attrs) => attr_list(&mut out, &attrs),
            None => out.extend_from_slice(b"false true"),
        }
        let (line, pos) = rd.position();
        out.extend_from_slice(format!(" @{line}:").as_bytes());
        seg(&mut out, &pos);
        out
    });
    r.unwrap_or_else(|_| b"PANIC".to_vec())
}

fn aligns(a: &[east::Alignment]) -> String {
    a.iter().map(|x| x.as_str()).collect::<Vec<_>>().join(",")
}

/// tools/go-oracle/goldmark/astdump.go:tocText: a heading's inline subtrees
/// rendered one by one, as Hugo's TOC transformer does (the renderer API on
/// nodes that are not the document).
fn toc_text(md: Option<&goldmark::Markdown>, ast: &Ast, h: NodeId, src: &[u8]) -> Vec<u8> {
    use goldmark::ast::{
        KIND_AUTO_LINK, KIND_CODE_SPAN, KIND_EMPHASIS, KIND_IMAGE, KIND_LINK, KIND_RAW_HTML,
        KIND_STRING, KIND_TEXT, WalkStatus,
    };
    let Some(md) = md else {
        return Vec::new();
    };
    let mut buf: Vec<u8> = Vec::new();
    let _ = goldmark::ast::walk_ref(ast, h, &mut |ast, n, entering| {
        if !entering || n == h {
            return Ok(WalkStatus::Continue);
        }
        let k = ast.kind(n);
        let container = [KIND_CODE_SPAN, KIND_LINK, KIND_IMAGE, KIND_EMPHASIS].contains(&k)
            || k == *east::KIND_STRIKETHROUGH;
        let leaf = [KIND_AUTO_LINK, KIND_RAW_HTML, KIND_TEXT, KIND_STRING].contains(&k);
        if (container || leaf) && md.renderer().render(&mut buf, src, ast, n).is_err() {
            buf.extend_from_slice(b"ERROR");
        }
        Ok(if container {
            WalkStatus::SkipChildren
        } else {
            WalkStatus::Continue
        })
    });
    buf
}

#[allow(clippy::too_many_arguments)]
fn node(
    out: &mut Vec<u8>,
    md: Option<&goldmark::Markdown>,
    ast: &Ast,
    n: NodeId,
    src: &[u8],
    depth: usize,
    with_text: bool,
) {
    out.extend(std::iter::repeat_n(b' ', depth));
    let typ = ast.typ(n);
    out.extend_from_slice(
        format!(
            "{} t={} cc={}",
            ast.kind(n),
            match typ {
                NodeType::Block => 1,
                NodeType::Inline => 2,
                NodeType::Document => 3,
            },
            ast.child_count(n)
        )
        .as_bytes(),
    );
    if typ != NodeType::Inline {
        out.extend_from_slice(b" lines=");
        let lines = ast.lines(n);
        for i in 0..lines.len() {
            if i > 0 {
                out.push(b',');
            }
            seg(out, &lines.at(i));
        }
        out.extend_from_slice(format!(" blank={}", b(ast.has_blank_previous_lines(n))).as_bytes());
    }
    out.extend_from_slice(format!(" raw={}", b(ast.is_raw(n))).as_bytes());
    if let Some(a) = ast.attributes(n) {
        out.extend_from_slice(b" attrs=");
        attr_list(out, a);
    }
    match ast.value(n) {
        NodeValue::Heading(h) => {
            out.extend_from_slice(format!(" lvl={} toc=", h.level).as_bytes());
            let t = toc_text(md, ast, n, src);
            q(out, &t);
        }
        NodeValue::FencedCodeBlock(f) => {
            out.extend_from_slice(b" info=");
            match f.info {
                Some(i) => seg(out, &ast.text_node(i).unwrap().segment),
                None => out.push(b'-'),
            }
            out.extend_from_slice(b" lang=");
            match ast.fenced_code_block_language(n, src) {
                Some(l) => q(out, &l),
                None => out.push(b'-'),
            }
        }
        NodeValue::HTMLBlock(h) => {
            out.extend_from_slice(format!(" ht={} cl=", h.html_block_type as i64).as_bytes());
            seg(out, &h.closure_line);
        }
        NodeValue::List(l) => out.extend_from_slice(
            format!(" m={} tight={} start={}", l.marker, b(l.is_tight), l.start).as_bytes(),
        ),
        NodeValue::ListItem(li) => out.extend_from_slice(format!(" off={}", li.offset).as_bytes()),
        NodeValue::Text(t) => {
            out.extend_from_slice(b" seg=");
            seg(out, &t.segment);
            out.extend_from_slice(
                format!(
                    " soft={} hard={}",
                    b(t.soft_line_break()),
                    b(t.hard_line_break())
                )
                .as_bytes(),
            );
        }
        NodeValue::String(s) => {
            out.extend_from_slice(b" v=");
            q(out, &s.value);
            out.extend_from_slice(format!(" code={}", b(s.is_code())).as_bytes());
        }
        NodeValue::Emphasis(e) => out.extend_from_slice(format!(" lvl={}", e.level).as_bytes()),
        NodeValue::Link(l) | NodeValue::Image(l) => {
            out.extend_from_slice(b" dest=");
            q(out, &l.destination);
            out.extend_from_slice(b" title=");
            q(out, l.title.as_deref().unwrap_or(b""));
        }
        NodeValue::AutoLink(a) => {
            out.extend_from_slice(format!(" alt={} proto=", a.auto_link_type as i64).as_bytes());
            q(out, a.protocol.as_deref().unwrap_or(b""));
            out.extend_from_slice(b" url=");
            q(out, &ast.auto_link_url(a, src));
            out.extend_from_slice(b" label=");
            q(out, &ast.auto_link_label(a, src));
        }
        NodeValue::RawHTML(r) => {
            out.extend_from_slice(b" segs=");
            for i in 0..r.segments.len() {
                if i > 0 {
                    out.push(b',');
                }
                seg(out, &r.segments.at(i));
            }
        }
        NodeValue::Custom(_) => {
            if let Some(t) = ast.custom::<east::Table>(n) {
                out.extend_from_slice(format!(" al={}", aligns(&t.alignments)).as_bytes());
            } else if let Some(t) = ast.custom::<east::TableHeader>(n) {
                out.extend_from_slice(format!(" al={}", aligns(&t.alignments)).as_bytes());
            } else if let Some(t) = ast.custom::<east::TableRow>(n) {
                out.extend_from_slice(format!(" al={}", aligns(&t.alignments)).as_bytes());
            } else if let Some(t) = ast.custom::<east::TableCell>(n) {
                out.extend_from_slice(format!(" al={}", t.alignment.as_str()).as_bytes());
            } else if let Some(t) = ast.custom::<east::TaskCheckBox>(n) {
                out.extend_from_slice(format!(" checked={}", b(t.is_checked)).as_bytes());
            } else if let Some(t) = ast.custom::<east::DefinitionList>(n) {
                out.extend_from_slice(
                    format!(
                        " off={} tmp={}",
                        t.offset,
                        b(t.temporary_paragraph.is_some())
                    )
                    .as_bytes(),
                );
            } else if let Some(t) = ast.custom::<east::DefinitionDescription>(n) {
                out.extend_from_slice(format!(" tight={}", b(t.is_tight)).as_bytes());
            } else if let Some(t) = ast.custom::<east::Footnote>(n) {
                out.extend_from_slice(b" ref=");
                q(out, &t.ref_);
                out.extend_from_slice(format!(" idx={}", t.index).as_bytes());
            } else if let Some(t) = ast.custom::<east::FootnoteLink>(n) {
                out.extend_from_slice(
                    format!(" idx={} rc={} ri={}", t.index, t.ref_count, t.ref_index).as_bytes(),
                );
            } else if let Some(t) = ast.custom::<east::FootnoteBacklink>(n) {
                out.extend_from_slice(
                    format!(" idx={} rc={} ri={}", t.index, t.ref_count, t.ref_index).as_bytes(),
                );
            } else if let Some(t) = ast.custom::<east::FootnoteList>(n) {
                out.extend_from_slice(format!(" count={}", t.count).as_bytes());
            }
        }
        _ => {}
    }
    if with_text {
        out.extend_from_slice(b" text=");
        q(out, &ast.text(n, src));
    }
    out.push(b'\n');
}

/// Dumps the tree below `root` (pre-order, explicit stack); `md` renders the
/// headings' TOC text.
pub fn dump(
    md: Option<&goldmark::Markdown>,
    ast: &Ast,
    root: NodeId,
    src: &[u8],
    with_text: bool,
) -> Vec<u8> {
    let mut out = Vec::new();
    let mut stack = vec![(root, 0usize)];
    while let Some((n, depth)) = stack.pop() {
        node(&mut out, md, ast, n, src, depth, with_text);
        let children = ast.children(n);
        for c in children.into_iter().rev() {
            stack.push((c, depth + 1));
        }
    }
    out
}

/// Go: astDump's size limit for the `text=` field (Text is quadratic in the
/// document depth).
pub const TEXT_LIMIT: usize = 4096;

/// Parses `md` with the named config (Go: `m.Parser().Parse(text.NewReader(md))`)
/// and dumps the AST; a panic becomes "PANIC".
pub fn dump_cfg(mds: &mut super::Markdowns, cfg: &str, md: &[u8]) -> Vec<u8> {
    let m = mds.get(cfg);
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut pc = goldmark::parser::new_context(vec![]);
        let doc = m.parse(md, &mut pc);
        dump(Some(m), &doc.ast, doc.root, md, md.len() <= TEXT_LIMIT)
    }));
    r.unwrap_or_else(|_| b"PANIC".to_vec())
}

/// tools/go-oracle/goldmark/redteam.go (redteamTexts): `Node.Text` of the
/// document and of the first inline node in document order, joined by NUL.
pub fn texts_cfg(mds: &mut super::Markdowns, cfg: &str, md: &[u8]) -> Vec<u8> {
    let m = mds.get(cfg);
    let mut pc = goldmark::parser::new_context(vec![]);
    let doc = m.parse(md, &mut pc);
    let mut out = doc.ast.text(doc.root, md);
    out.push(0);
    let mut n = Some(doc.root);
    while let Some(x) = n {
        if doc.ast.typ(x) == NodeType::Inline {
            out.extend_from_slice(&doc.ast.text(x, md));
            break;
        }
        n = doc.ast.first_child(x);
    }
    out
}
