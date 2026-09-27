// Go: github.com/yuin/goldmark@v1.7.12/extension/table.go

use std::sync::{Arc, LazyLock};

use super::ast::{self as east, Alignment};
use crate::ast::{self as gast, Ast, AttrValue, NodeId, WalkStatus};
use crate::parser::{self, AstTransformer, Context, ContextKey, OptionValue, ParagraphTransformer};
use crate::renderer::html::{self, HtmlRendererOption};
use crate::renderer::{self, NodeRenderer, NodeRendererFuncRegisterer, RendererOption};
use crate::text::{Reader, Segment, new_segment};
use crate::util::{self, BufWriter, BytesFilter, CopyOnWriteBuffer};
use crate::{Extender, Markdown};

static ESCAPED_PIPE_CELL_LIST_KEY: LazyLock<ContextKey> = LazyLock::new(parser::new_context_key);

/// Go: `escapedPipeCell` (the list holds pointers, so later `Pos`/
/// `Transformed` updates are visible through it; the port keeps the entries
/// in the context's list and updates them by index).
#[derive(Debug, Clone)]
struct EscapedPipeCell {
    cell: NodeId,
    pos: Vec<i64>,
    transformed: bool,
}

/// TableCellAlignMethod indicates how are table cells aligned in HTML format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableCellAlignMethod {
    /// TableCellAlignDefault renders alignments by default method.
    /// With XHTML, alignments are rendered as an align attribute.
    /// With HTML5, alignments are rendered as a style attribute.
    Default = 0,
    /// TableCellAlignAttribute renders alignments as an align attribute.
    Attribute = 1,
    /// TableCellAlignStyle renders alignments as a style attribute.
    Style = 2,
    /// TableCellAlignNone does not care about alignments.
    /// If you using classes or other styles, you can add these attributes
    /// in an ASTTransformer.
    None = 3,
}

/// TableConfig struct holds options for the extension.
#[derive(Clone)]
pub struct TableConfig {
    pub config: html::Config,
    /// TableCellAlignMethod indicates how are table celss aligned.
    pub table_cell_align_method: TableCellAlignMethod,
}

// Go: extension/table.go:NewTableConfig
/// NewTableConfig returns a new Config with defaults.
pub fn new_table_config() -> TableConfig {
    TableConfig {
        config: html::new_config(),
        table_cell_align_method: TableCellAlignMethod::Default,
    }
}

const OPT_TABLE_CELL_ALIGN_METHOD: &str = "TableTableCellAlignMethod";

impl TableConfig {
    // Go: extension/table.go:TableConfig.SetOption
    /// SetOption implements renderer.SetOptioner.
    pub fn set_option(&mut self, name: &str, value: &OptionValue) {
        match name {
            OPT_TABLE_CELL_ALIGN_METHOD => {
                self.table_cell_align_method = *value
                    .downcast_ref::<TableCellAlignMethod>()
                    .expect("interface conversion: not extension.TableCellAlignMethod")
            }
            _ => self.config.set_option(name, value),
        }
    }
}

/// TableOption interface is a functional option interface for the extension.
#[derive(Clone)]
pub enum TableOption {
    /// Go: WithTableHTMLOptions.
    HtmlOptions(Vec<Arc<dyn HtmlRendererOption>>),
    /// Go: WithTableCellAlignMethod.
    CellAlignMethod(TableCellAlignMethod),
}

impl TableOption {
    /// SetTableOption sets given option to the extension.
    pub fn set_table_option(&self, c: &mut TableConfig) {
        match self {
            TableOption::HtmlOptions(v) => {
                for o in v {
                    o.set_html_option(&mut c.config);
                }
            }
            TableOption::CellAlignMethod(a) => c.table_cell_align_method = *a,
        }
    }
}

impl RendererOption for TableOption {
    fn set_config(self: Box<Self>, c: &mut renderer::Config) {
        match *self {
            TableOption::HtmlOptions(v) => {
                for o in v {
                    o.set_renderer_config(c);
                }
            }
            TableOption::CellAlignMethod(a) => {
                c.options
                    .insert(OPT_TABLE_CELL_ALIGN_METHOD.to_string(), Arc::new(a));
            }
        }
    }
}

// Go: extension/table.go:WithTableHTMLOptions
/// WithTableHTMLOptions is functional option that wraps goldmark HTMLRenderer options.
pub fn with_table_html_options(opts: Vec<Arc<dyn HtmlRendererOption>>) -> TableOption {
    TableOption::HtmlOptions(opts)
}

// Go: extension/table.go:WithTableCellAlignMethod
/// WithTableCellAlignMethod is a functional option that indicates how are table cells aligned in HTML format.
pub fn with_table_cell_align_method(a: TableCellAlignMethod) -> TableOption {
    TableOption::CellAlignMethod(a)
}

// Go: extension/table.go:isTableDelim
fn is_table_delim(bs: &[u8]) -> bool {
    let (w, _) = util::indent_width(bs, 0);
    if w > 3 {
        return false;
    }
    for &b in bs {
        if !(util::is_space(b) || b == b'-' || b == b'|' || b == b':') {
            return false;
        }
    }
    true
}

/// Go RE2 `\s` (Perl class): `[\t\n\f\r ]`.
fn is_re_space(c: u8) -> bool {
    matches!(c, b'\t' | b'\n' | b'\x0c' | b'\r' | b' ')
}

/// Matches `^\s*<colon?>\-+<colon?>\s*$` (Go's `$` without `(?m)` is the end
/// of the text). The patterns have no ambiguity: after the leading spaces
/// the rest must be exactly the optional colon, a dash run, the optional
/// colon and trailing spaces.
fn match_delim(col: &[u8], left: bool, right: bool) -> bool {
    let mut i = 0;
    while i < col.len() && is_re_space(col[i]) {
        i += 1;
    }
    if left {
        if i >= col.len() || col[i] != b':' {
            return false;
        }
        i += 1;
    }
    let dash_start = i;
    while i < col.len() && col[i] == b'-' {
        i += 1;
    }
    if i == dash_start {
        return false;
    }
    if right {
        if i >= col.len() || col[i] != b':' {
            return false;
        }
        i += 1;
    }
    while i < col.len() && is_re_space(col[i]) {
        i += 1;
    }
    i == col.len()
}

// Go: extension/table.go:tableDelimLeft (`^\s*\:\-+\s*$`)
pub(crate) fn table_delim_left(col: &[u8]) -> bool {
    match_delim(col, true, false)
}

// Go: extension/table.go:tableDelimRight (`^\s*\-+\:\s*$`)
pub(crate) fn table_delim_right(col: &[u8]) -> bool {
    match_delim(col, false, true)
}

// Go: extension/table.go:tableDelimCenter (`^\s*\:\-+\:\s*$`)
pub(crate) fn table_delim_center(col: &[u8]) -> bool {
    match_delim(col, true, true)
}

// Go: extension/table.go:tableDelimNone (`^\s*\-+\s*$`)
pub(crate) fn table_delim_none(col: &[u8]) -> bool {
    match_delim(col, false, false)
}

struct TableParagraphTransformer;

// Go: extension/table.go:NewTableParagraphTransformer
/// NewTableParagraphTransformer returns  a new ParagraphTransformer
/// that can transform paragraphs into tables.
pub fn new_table_paragraph_transformer() -> Box<dyn ParagraphTransformer> {
    Box::new(TableParagraphTransformer)
}

impl ParagraphTransformer for TableParagraphTransformer {
    // Go: extension/table.go:tableParagraphTransformer.Transform
    fn transform<'a>(
        &self,
        ast: &mut Ast,
        node: NodeId,
        reader: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) {
        // `lines` is a pointer to the node's Segments in Go: its length is
        // re-read on every iteration.
        if ast.lines(node).len() < 2 {
            return;
        }
        let mut i: i64 = 1;
        while i < ast.lines(node).len() {
            let alignments = self.parse_delimiter(ast.lines(node).at(i), reader);
            let Some(alignments) = alignments else {
                i += 1;
                continue;
            };
            let seg = ast.lines(node).at(i - 1);
            let header = self.parse_row(ast, seg, &alignments, true, reader, pc);
            let Some(header) = header else {
                return;
            };
            if alignments.len() as i64 != ast.child_count(header) {
                return;
            }
            let table = east::new_table(ast);
            ast.custom_mut::<east::Table>(table).unwrap().alignments = alignments.clone();
            let th = east::new_table_header(ast, header);
            ast.append_child(table, th);
            let mut j = i + 1;
            while j < ast.lines(node).len() {
                let seg = ast.lines(node).at(j);
                let row = self
                    .parse_row(ast, seg, &alignments, false, reader, pc)
                    .expect("parseRow returns a row for body lines");
                ast.append_child(table, row);
                j += 1;
            }
            ast.lines_mut(node).set_sliced(0, i - 1);
            let parent = ast.parent(node).expect("paragraph has a parent");
            ast.insert_after(parent, node, table);
            if ast.lines(node).len() == 0 {
                ast.remove_child(parent, node);
            } else {
                let mut last = ast.lines(node).at(i - 2);
                last.stop -= 1; // trim last newline(\n)
                ast.lines_mut(node).set(i - 2, last);
            }
            i += 1;
        }
    }
}

impl TableParagraphTransformer {
    // Go: extension/table.go:tableParagraphTransformer.parseRow
    /// Returns `None` only where Go returns nil (never: Go returns a row).
    fn parse_row<'a>(
        &self,
        ast: &mut Ast,
        segment: Segment,
        alignments: &[Alignment],
        is_header: bool,
        reader: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) -> Option<NodeId> {
        let source = reader.source();
        let segment = segment.trim_left_space(source);
        let segment = segment.trim_right_space(source);
        let line = segment.value(source);
        let mut pos: usize = 0;
        let mut limit = line.len();
        let row = east::new_table_row(ast, alignments.to_vec());
        if !line.is_empty() && line[pos] == b'|' {
            pos += 1;
        }
        if !line.is_empty() && line[limit - 1] == b'|' {
            limit -= 1;
        }
        let mut i = 0usize;
        while pos < limit {
            let mut alignment = Alignment::None;
            if i >= alignments.len() {
                if !is_header {
                    return Some(row);
                }
            } else {
                alignment = alignments[i];
            }

            // index of this cell's entry in the context's escaped list
            let mut escaped_cell: Option<usize> = None;
            let node = east::new_table_cell(ast);
            ast.custom_mut::<east::TableCell>(node).unwrap().alignment = alignment;
            let mut has_backtick = false;
            let mut closure = pos;
            while closure < limit {
                if line[closure] == b'`' {
                    has_backtick = true;
                }
                if line[closure] == b'|' {
                    if closure == 0 || line[closure - 1] != b'\\' {
                        break;
                    } else if has_backtick {
                        if escaped_cell.is_none() {
                            let list = pc
                                .compute_if_absent(*ESCAPED_PIPE_CELL_LIST_KEY, || {
                                    Box::new(Vec::<EscapedPipeCell>::new())
                                })
                                .downcast_mut::<Vec<EscapedPipeCell>>()
                                .expect("escaped pipe cell list");
                            list.push(EscapedPipeCell {
                                cell: node,
                                pos: Vec::new(),
                                transformed: false,
                            });
                            escaped_cell = Some(list.len() - 1);
                        }
                        let list = pc
                            .get_as_mut::<Vec<EscapedPipeCell>>(*ESCAPED_PIPE_CELL_LIST_KEY)
                            .expect("escaped pipe cell list");
                        list[escaped_cell.unwrap()]
                            .pos
                            .push(segment.start + closure as i64 - 1);
                    }
                }
                closure += 1;
            }
            let seg = new_segment(segment.start + pos as i64, segment.start + closure as i64);
            let seg = seg.trim_left_space(source);
            let seg = seg.trim_right_space(source);
            ast.lines_mut(node).append(seg);
            ast.append_child(row, node);
            pos = closure + 1;
            i += 1;
        }
        while i < alignments.len() {
            let c = east::new_table_cell(ast);
            ast.append_child(row, c);
            i += 1;
        }
        Some(row)
    }

    // Go: extension/table.go:tableParagraphTransformer.parseDelimiter
    fn parse_delimiter<'a>(
        &self,
        segment: Segment,
        reader: &mut dyn Reader<'a>,
    ) -> Option<Vec<Alignment>> {
        let line = segment.value(reader.source());
        if !is_table_delim(&line) {
            return None;
        }
        let mut cols: Vec<&[u8]> = line.split(|&c| c == b'|').collect();
        if util::is_blank(cols[0]) {
            cols.remove(0);
        }
        if !cols.is_empty() && util::is_blank(cols[cols.len() - 1]) {
            cols.pop();
        }

        // Go: `var alignments []ast.Alignment` stays nil when there is no
        // column, and a nil slice means "not a delimiter row" to Transform.
        let mut alignments: Option<Vec<Alignment>> = None;
        for col in cols {
            let a = if table_delim_left(col) {
                Alignment::Left
            } else if table_delim_right(col) {
                Alignment::Right
            } else if table_delim_center(col) {
                Alignment::Center
            } else if table_delim_none(col) {
                Alignment::None
            } else {
                return None;
            };
            alignments.get_or_insert_with(Vec::new).push(a);
        }
        alignments
    }
}

struct TableASTTransformer;

// Go: extension/table.go:NewTableASTTransformer
/// NewTableASTTransformer returns a parser.ASTTransformer for tables.
pub fn new_table_ast_transformer() -> Box<dyn AstTransformer> {
    Box::new(TableASTTransformer)
}

impl AstTransformer for TableASTTransformer {
    // Go: extension/table.go:tableASTTransformer.Transform
    fn transform<'a>(
        &self,
        ast: &mut Ast,
        _node: NodeId,
        _reader: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) {
        let Some(lst) = pc.get_as::<Vec<EscapedPipeCell>>(*ESCAPED_PIPE_CELL_LIST_KEY) else {
            return;
        };
        let mut lst = lst.clone();
        pc.set(*ESCAPED_PIPE_CELL_LIST_KEY, None);
        for vi in 0..lst.len() {
            if lst[vi].transformed {
                continue;
            }
            let cell = lst[vi].cell;
            let _ = gast::walk(
                ast,
                cell,
                &mut |ast: &mut Ast, n: NodeId, entering: bool| {
                    if !entering || ast.kind(n) != gast::KIND_CODE_SPAN {
                        return Ok(WalkStatus::Continue);
                    }

                    let mut c = ast.first_child(n);
                    while let Some(cid) = c {
                        let next = ast.next_sibling(cid);
                        if ast.kind(cid) != gast::KIND_TEXT {
                            c = next;
                            continue;
                        }
                        let parent = ast.parent(cid).expect("text has a parent");
                        // Go: ts := &c.(*gast.Text).Segment — the segment of the
                        // original node c, which is never modified below.
                        let ts = ast.text_node(cid).unwrap().segment;
                        let mut n = cid;
                        for v in lst.iter_mut() {
                            for &pos in &v.pos {
                                if ts.start <= pos && pos < ts.stop {
                                    let segment = ast
                                        .text_node(n)
                                        .expect("interface conversion: not *ast.Text")
                                        .segment;
                                    let n1 = ast.new_raw_text_segment(segment.with_stop(pos));
                                    let n2 = ast.new_raw_text_segment(segment.with_start(pos + 1));
                                    ast.insert_after(parent, n, n1);
                                    ast.insert_after(parent, n1, n2);
                                    ast.remove_child(parent, n);
                                    n = n2;
                                    v.transformed = true;
                                }
                            }
                        }
                        c = next;
                    }
                    Ok(WalkStatus::Continue)
                },
            );
        }
    }
}

/// TableHTMLRenderer is a renderer.NodeRenderer implementation that
/// renders Table nodes.
pub struct TableHTMLRenderer {
    pub table_config: TableConfig,
}

// Go: extension/table.go:NewTableHTMLRenderer
/// NewTableHTMLRenderer returns a new TableHTMLRenderer.
pub fn new_table_html_renderer(opts: &[TableOption]) -> Box<dyn NodeRenderer> {
    let mut r = TableHTMLRenderer {
        table_config: new_table_config(),
    };
    for opt in opts {
        opt.set_table_option(&mut r.table_config);
    }
    Box::new(r)
}

impl NodeRenderer for TableHTMLRenderer {
    // Go: extension/table.go:TableHTMLRenderer.RegisterFuncs
    fn register_funcs(self: Arc<Self>, reg: &mut dyn NodeRendererFuncRegisterer) {
        reg.register(
            *east::KIND_TABLE,
            html::bind(&self, TableHTMLRenderer::render_table),
        );
        reg.register(
            *east::KIND_TABLE_HEADER,
            html::bind(&self, TableHTMLRenderer::render_table_header),
        );
        reg.register(
            *east::KIND_TABLE_ROW,
            html::bind(&self, TableHTMLRenderer::render_table_row),
        );
        reg.register(
            *east::KIND_TABLE_CELL,
            html::bind(&self, TableHTMLRenderer::render_table_cell),
        );
    }

    // Go: TableConfig.SetOption (promoted)
    fn set_option(&mut self, name: &str, value: &OptionValue) {
        self.table_config.set_option(name, value);
    }
}

/// TableAttributeFilter defines attribute names which table elements can have.
///
/// - align: Deprecated
/// - bgcolor: Deprecated
/// - border: Deprecated
/// - cellpadding: Deprecated
/// - cellspacing: Deprecated
/// - frame: Deprecated
/// - rules: Deprecated
/// - summary: Deprecated
/// - width: Deprecated.
pub static TABLE_ATTRIBUTE_FILTER: LazyLock<BytesFilter> = LazyLock::new(|| {
    html::GLOBAL_ATTRIBUTE_FILTER
        .extend_string("align,bgcolor,border,cellpadding,cellspacing,frame,rules,summary,width")
});

/// TableHeaderAttributeFilter defines attribute names which <thead> elements can have.
pub static TABLE_HEADER_ATTRIBUTE_FILTER: LazyLock<BytesFilter> = LazyLock::new(|| {
    html::GLOBAL_ATTRIBUTE_FILTER.extend_string("align,bgcolor,char,charoff,valign")
});

/// TableRowAttributeFilter defines attribute names which <tr> elements can have.
pub static TABLE_ROW_ATTRIBUTE_FILTER: LazyLock<BytesFilter> = LazyLock::new(|| {
    html::GLOBAL_ATTRIBUTE_FILTER.extend_string("align,bgcolor,char,charoff,valign")
});

/// TableThCellAttributeFilter defines attribute names which table <th> cells can have.
pub static TABLE_TH_CELL_ATTRIBUTE_FILTER: LazyLock<BytesFilter> = LazyLock::new(|| {
    html::GLOBAL_ATTRIBUTE_FILTER.extend_string(
        "abbr,align,axis,bgcolor,char,charoff,colspan,headers,height,rowspan,scope,valign,width",
    )
});

/// TableTdCellAttributeFilter defines attribute names which table <td> cells can have.
pub static TABLE_TD_CELL_ATTRIBUTE_FILTER: LazyLock<BytesFilter> = LazyLock::new(|| {
    html::GLOBAL_ATTRIBUTE_FILTER.extend_string(
        "abbr,align,axis,bgcolor,char,charoff,colspan,headers,height,rowspan,scope,valign,width",
    )
});

type R = Result<WalkStatus, crate::Error>;

impl TableHTMLRenderer {
    // Go: extension/table.go:TableHTMLRenderer.renderTable
    fn render_table(
        &self,
        w: &mut dyn BufWriter,
        _source: &[u8],
        ast: &Ast,
        n: NodeId,
        entering: bool,
    ) -> R {
        if entering {
            w.write_string("<table");
            if ast.attributes(n).is_some() {
                html::render_attributes(w, ast, n, Some(&TABLE_ATTRIBUTE_FILTER));
            }
            w.write_string(">\n");
        } else {
            w.write_string("</table>\n");
        }
        Ok(WalkStatus::Continue)
    }

    // Go: extension/table.go:TableHTMLRenderer.renderTableHeader
    fn render_table_header(
        &self,
        w: &mut dyn BufWriter,
        _source: &[u8],
        ast: &Ast,
        n: NodeId,
        entering: bool,
    ) -> R {
        if entering {
            w.write_string("<thead");
            if ast.attributes(n).is_some() {
                html::render_attributes(w, ast, n, Some(&TABLE_HEADER_ATTRIBUTE_FILTER));
            }
            w.write_string(">\n");
            w.write_string("<tr>\n"); // Header <tr> has no separate handle
        } else {
            w.write_string("</tr>\n");
            w.write_string("</thead>\n");
            if ast.next_sibling(n).is_some() {
                w.write_string("<tbody>\n");
            }
        }
        Ok(WalkStatus::Continue)
    }

    // Go: extension/table.go:TableHTMLRenderer.renderTableRow
    fn render_table_row(
        &self,
        w: &mut dyn BufWriter,
        _source: &[u8],
        ast: &Ast,
        n: NodeId,
        entering: bool,
    ) -> R {
        if entering {
            w.write_string("<tr");
            if ast.attributes(n).is_some() {
                html::render_attributes(w, ast, n, Some(&TABLE_ROW_ATTRIBUTE_FILTER));
            }
            w.write_string(">\n");
        } else {
            w.write_string("</tr>\n");
            let parent = ast.parent(n).expect("row has a parent");
            if ast.last_child(parent) == Some(n) {
                w.write_string("</tbody>\n");
            }
        }
        Ok(WalkStatus::Continue)
    }

    // Go: extension/table.go:TableHTMLRenderer.renderTableCell
    //
    // Go stores the computed style with n.SetAttributeString("style", ...)
    // before rendering the attributes. The renderer here has a shared AST,
    // so the same attribute list is computed locally (same position, same
    // value); the node is left unchanged (see PORTING.md).
    fn render_table_cell(
        &self,
        w: &mut dyn BufWriter,
        _source: &[u8],
        ast: &Ast,
        node: NodeId,
        entering: bool,
    ) -> R {
        let n = east::must::<east::TableCell>(ast, node, "TableCell");
        let mut tag = "td";
        let parent = ast.parent(node).expect("cell has a parent");
        if ast.kind(parent) == *east::KIND_TABLE_HEADER {
            tag = "th";
        }
        if entering {
            w.write_string("<");
            w.write_string(tag);
            let mut attrs: Option<Vec<gast::Attribute>> = ast.attributes(node).map(|a| a.to_vec());
            if n.alignment != Alignment::None {
                let mut amethod = self.table_config.table_cell_align_method;
                if amethod == TableCellAlignMethod::Default {
                    if self.table_config.config.xhtml {
                        amethod = TableCellAlignMethod::Attribute;
                    } else {
                        amethod = TableCellAlignMethod::Style;
                    }
                }
                match amethod {
                    TableCellAlignMethod::Attribute => {
                        // Skip align render if overridden
                        if ast.attribute_string(node, "align").is_none() {
                            w.write_string(" align=\"");
                            w.write_string(n.alignment.as_str());
                            w.write_string("\"");
                        }
                    }
                    TableCellAlignMethod::Style => {
                        let v = ast.attribute_string(node, "style");
                        let mut cob = match v {
                            Some(v) => {
                                let b = match v {
                                    AttrValue::Bytes(b) => b.as_slice(),
                                    _ => panic!(
                                        "interface conversion: interface {{}} is not []uint8"
                                    ),
                                };
                                let mut cob = CopyOnWriteBuffer::new(b);
                                cob.append_byte(b';');
                                cob
                            }
                            None => CopyOnWriteBuffer::new(&[]),
                        };
                        let style = format!("text-align:{}", n.alignment.as_str());
                        cob.append(style.as_bytes());
                        set_attribute(&mut attrs, b"style", AttrValue::Bytes(cob.bytes().to_vec()));
                    }
                    _ => {}
                }
            }
            if let Some(attrs) = &attrs {
                if tag == "td" {
                    html::render_attribute_list(w, attrs, Some(&TABLE_TD_CELL_ATTRIBUTE_FILTER)); // <td>
                } else {
                    html::render_attribute_list(w, attrs, Some(&TABLE_TH_CELL_ATTRIBUTE_FILTER)); // <th>
                }
            }
            w.write_byte(b'>');
        } else {
            w.write_string("</");
            w.write_string(tag);
            w.write_string(">\n");
        }
        Ok(WalkStatus::Continue)
    }
}

// Go: ast/ast.go:BaseNode.SetAttribute on a local copy of the list.
fn set_attribute(attrs: &mut Option<Vec<gast::Attribute>>, name: &[u8], value: AttrValue) {
    match attrs {
        None => {
            *attrs = Some(vec![gast::Attribute {
                name: name.to_vec(),
                value,
            }]);
        }
        Some(list) => {
            for a in list.iter_mut() {
                if a.name == name {
                    a.value = value;
                    return;
                }
            }
            list.push(gast::Attribute {
                name: name.to_vec(),
                value,
            });
        }
    }
}

/// Go: `type table struct{ options []TableOption }`.
pub struct TableExt {
    options: Vec<TableOption>,
}

// Go: extension/table.go:Table
/// Table is an extension that allow you to use GFM tables .
pub fn table() -> Box<dyn Extender> {
    Box::new(TableExt {
        options: Vec::new(),
    })
}

// Go: extension/table.go:NewTable
/// NewTable returns a new extension with given options.
pub fn new_table(opts: Vec<TableOption>) -> Box<dyn Extender> {
    Box::new(TableExt { options: opts })
}

impl Extender for TableExt {
    // Go: extension/table.go:table.Extend
    fn extend(&self, m: &mut Markdown) {
        m.parser().add_options(vec![
            parser::with_paragraph_transformers(vec![util::prioritized(
                new_table_paragraph_transformer(),
                200,
            )]),
            parser::with_ast_transformers(vec![util::prioritized(new_table_ast_transformer(), 0)]),
        ]);
        m.renderer()
            .add_options(vec![renderer::with_node_renderers(vec![
                util::prioritized(new_table_html_renderer(&self.options), 500),
            ])]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delim_regexps() {
        assert!(table_delim_left(b" :--- "));
        assert!(table_delim_left(b":-\n"));
        assert!(!table_delim_left(b":-:"));
        assert!(table_delim_right(b"-:"));
        assert!(table_delim_center(b"\t:-:\r\n"));
        assert!(table_delim_none(b"---"));
        assert!(!table_delim_none(b""));
        assert!(!table_delim_none(b"- -"));
        assert!(!table_delim_none(b" "));
    }
}
