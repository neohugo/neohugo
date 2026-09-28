//! Port of `markup/goldmark/tables/tables.go`.
//!
//! Owner: Wave B task T06 (markup).

//! Go `markup/goldmark/tables`: tables are ALWAYS rendered through the table hook (embedded
//! `_markup/render-table.html`, one copy per output format).

use std::sync::Arc;

use go_value::{GoString, Value};
use goldmark::ast::{Ast, NodeId, WalkStatus};
use goldmark::extension::ast::{
    self as east, Alignment, KIND_TABLE, KIND_TABLE_CELL, KIND_TABLE_HEADER, KIND_TABLE_ROW,
};
use goldmark::parser::OptionValue;
use nh_common::types::hstring::Html;

use super::convert::{HugoNodeRenderer, HugoRegisterer, RenderFunc, file_error_from_pos};
use super::internal::render::{self, Context as RenderCtx, HookBase};
use crate::converter::hooks::{Renderer, RendererType, Table, TableCell, TableRow};
use crate::internal::attributes::{self, AttributesHolder};

/// Go: `tables.New()` — the node renderer (priority 100) is registered by the converter.
pub(crate) fn new_html_renderer() -> HtmlRenderer {
    HtmlRenderer
}

/// Go: `tables.htmlRenderer`.
pub(crate) struct HtmlRenderer;

impl HugoNodeRenderer for HtmlRenderer {
    // Go: markup/goldmark/tables/tables.go:RegisterFuncs
    fn register_funcs(self: Arc<Self>, reg: &mut HugoRegisterer) {
        let r = self.clone();
        reg.register(
            *KIND_TABLE,
            RenderFunc::hugo(move |w, s, a, n, e| r.render_table(w, s, a, n, e)),
        );
        let r = self.clone();
        reg.register(
            *KIND_TABLE_HEADER,
            RenderFunc::hugo(move |w, s, a, n, e| r.render_header_or_row(w, s, a, n, e)),
        );
        let r = self.clone();
        reg.register(
            *KIND_TABLE_ROW,
            RenderFunc::hugo(move |w, s, a, n, e| r.render_header_or_row(w, s, a, n, e)),
        );
        let r = self;
        reg.register(
            *KIND_TABLE_CELL,
            RenderFunc::hugo(move |w, s, a, n, e| r.render_cell(w, s, a, n, e)),
        );
    }

    fn set_option(&mut self, _name: &str, _value: &OptionValue) {}
}

type R = Result<WalkStatus, goldmark::Error>;

impl HtmlRenderer {
    // Go: markup/goldmark/tables/tables.go:renderTable
    fn render_table(
        &self,
        ctx: &mut RenderCtx<'_>,
        source: &[u8],
        a: &Ast,
        n: NodeId,
        entering: bool,
    ) -> R {
        if entering {
            // This will be modified below.
            ctx.push_value(*KIND_TABLE, Box::new(Table::default()));
            return Ok(WalkStatus::Continue);
        }

        let v = ctx.pop_value(*KIND_TABLE).expect("table not found");
        let table = *v.downcast::<Table>().expect("table value");

        let renderer = ctx
            .get_renderer(RendererType::Table, &Value::Invalid)
            .expect("table hook renderer not found");

        let ordinal = ctx.get_and_increment_ordinal(*KIND_TABLE);

        let cr = match renderer {
            Renderer::Table(t) => t,
            _ => panic!(
                "interface conversion: renderer is not hooks.TableRenderer: missing method RenderTable"
            ),
        };
        let resolver = {
            let cr = cr.clone();
            Some(Arc::new(move |sample: &[u8]| cr.resolve_position(sample))
                as render::PositionResolver)
        };

        let tctx = TableContext {
            base: Arc::new(render::new_base_context(
                ctx, resolver, a, n, source, None, ordinal,
            )),
            attributes: Arc::new(attributes::new(
                a.attributes(n).unwrap_or(&[]),
                attributes::AttributesOwnerType::General,
            )),
            thead: table.thead,
            tbody: table.tbody,
        };
        let base = tctx.base.clone();

        let cctx = ctx.render_context().ctx;
        if let Err(err) = cr.render_table(cctx, &mut ctx.w.buf, &Value::object(tctx)) {
            return Err(file_error_from_pos(err, base.position()));
        }

        Ok(WalkStatus::Continue)
    }

    // Go: markup/goldmark/tables/tables.go:peekTable
    fn peek_table<'c>(&self, ctx: &'c mut RenderCtx<'_>) -> &'c mut Table {
        ctx.peek_value(*KIND_TABLE)
            .expect("table not found")
            .downcast_mut::<Table>()
            .expect("table value")
    }

    // Go: markup/goldmark/tables/tables.go:renderCell
    fn render_cell(
        &self,
        ctx: &mut RenderCtx<'_>,
        _source: &[u8],
        a: &Ast,
        node: NodeId,
        entering: bool,
    ) -> R {
        if entering {
            // Store the current pos so we can capture the rendered text.
            ctx.push_pos(ctx.len());
            return Ok(WalkStatus::Continue);
        }

        let n = east::table_cell(a, node).expect("not a TableCell");

        let text = ctx.pop_rendered_string();

        let alignment = match n.alignment {
            Alignment::Left => "left",
            Alignment::Right => "right",
            Alignment::Center => "center",
            _ => "",
        };

        let cell = TableCell {
            text: Html(GoString::new(text)),
            alignment: alignment.to_string(),
        };

        let parent_is_header = a
            .parent(node)
            .is_some_and(|p| a.kind(p) == *KIND_TABLE_HEADER);
        let table = self.peek_table(ctx);
        // Go indexes `THead[len-1]` and panics on an empty slice.
        if parent_is_header {
            table
                .thead
                .last_mut()
                .expect("index out of range [-1]")
                .push(cell);
        } else {
            table
                .tbody
                .last_mut()
                .expect("index out of range [-1]")
                .push(cell);
        }

        Ok(WalkStatus::Continue)
    }

    // Go: markup/goldmark/tables/tables.go:renderHeaderOrRow
    fn render_header_or_row(
        &self,
        ctx: &mut RenderCtx<'_>,
        _source: &[u8],
        a: &Ast,
        n: NodeId,
        entering: bool,
    ) -> R {
        let table = self.peek_table(ctx);
        if entering {
            if a.kind(n) == *KIND_TABLE_HEADER {
                table.thead.push(TableRow::new());
            } else {
                table.tbody.push(TableRow::new());
            }
            return Ok(WalkStatus::Continue);
        }

        Ok(WalkStatus::Continue)
    }
}

/// Go: `tables.tableContext` (template type `*tables.tableContext`): the `hooks.BaseContext`
/// (`.Page`, `.PageInner`, `.Ordinal`, `.Position`), the `*AttributesHolder` methods, `.THead`,
/// `.TBody`.
#[derive(Clone)]
pub struct TableContext {
    pub base: Arc<HookBase>,
    pub attributes: Arc<AttributesHolder>,
    pub thead: Vec<TableRow>,
    pub tbody: Vec<TableRow>,
}

fn no_args(args: &[Value], name: &str) -> go_value::Result<()> {
    nh_common::object::args::exactly(args, 0, name)
}

nh_common::go_methods!(TableContext {
    "Page" => |c, _x, a| { no_args(a, "Page")?; Ok(c.base.page()) },
    "PageInner" => |c, _x, a| { no_args(a, "PageInner")?; Ok(c.base.page_inner()) },
    "Ordinal" => |c, _x, a| { no_args(a, "Ordinal")?; Ok(Value::int(c.base.ordinal())) },
    "Position" => |c, _x, a| { no_args(a, "Position")?; Ok(crate::converter::hooks::position_value(c.base.position())) },
    "Attributes" => |c, _x, a| { no_args(a, "Attributes")?; Ok(Value::map(c.attributes.attributes())) },
    "Options" => |c, _x, a| { no_args(a, "Options")?; Ok(Value::map(c.attributes.options())) },
    "AttributesSlice" => |c, _x, a| { no_args(a, "AttributesSlice")?; Ok(attributes::attribute_slice_value(c.attributes.attributes_slice())) },
    "OptionsSlice" => |c, _x, a| { no_args(a, "OptionsSlice")?; Ok(attributes::attribute_slice_value(c.attributes.options_slice())) },
    "THead" => |c, _x, a| { no_args(a, "THead")?; Ok(crate::converter::hooks::table_rows_to_value(&c.thead)) },
    "TBody" => |c, _x, a| { no_args(a, "TBody")?; Ok(crate::converter::hooks::table_rows_to_value(&c.tbody)) },
});

impl go_value::Object for TableContext {
    nh_common::object_basics!("*tables.tableContext");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/goldmark/tables/tables.go (175 lines; 10/10 funcs executed)
//   types: (group), tableContext
// OK L34-36: New() goldmark.Extender (the renderer is registered by the converter)
// OK L38-42: (e *ext) Extend(m goldmark.Markdown)
// OK L44-47: newHTMLRenderer() renderer.NodeRenderer
// OK L49-54: (r *htmlRenderer) RegisterFuncs(reg renderer.NodeRendererFuncRegisterer)
// OK L56-98: (r *htmlRenderer) renderTable(w util.BufWriter, source []byte, n ast.Node, entering bool) (ast.WalkStatus, error)
// OK L100-106: (r *htmlRenderer) peekTable(ctx *render.Context) *hooks.Table
// OK L108-144: (r *htmlRenderer) renderCell(w util.BufWriter, source []byte, node ast.Node, entering bool) (ast.WalkStatus, error)
// OK L146-159: (r *htmlRenderer) renderHeaderOrRow(w util.BufWriter, source []byte, n ast.Node, entering bool) (ast.WalkStatus, error)
// OK L169-171: (c *tableContext) THead() []hooks.TableRow
// OK L173-175: (c *tableContext) TBody() []hooks.TableRow
// ---------------------------------------------------------------------------
