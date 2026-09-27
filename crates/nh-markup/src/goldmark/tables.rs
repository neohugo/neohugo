//! Port of `markup/goldmark/tables/tables.go`.
//!
//! Owner: Wave B task T06 (markup).


//! Go `markup/goldmark/tables`: tables are ALWAYS rendered through the table hook (embedded
//! `_markup/render-table.html`, one copy per output format).

use std::sync::Arc;

use go_value::{Object, Value};

use crate::converter::hooks::TableRow;
use crate::internal::attributes::AttributesHolder;

/// Go: `tables.tableContext` (template type `*tables.tableContext`): `.Page`, `.PageInner`,
/// `.Ordinal`, `.Position`, `.Attributes`, `.THead`, `.TBody`.
#[derive(Clone)]
pub struct TableContext {
    pub page: Value,
    pub page_inner: Value,
    pub ordinal: i64,
    pub attributes: Arc<AttributesHolder>,
    pub thead: Vec<TableRow>,
    pub tbody: Vec<TableRow>,
}

nh_common::go_methods!(TableContext {
    "Page" => |c, _x, _a| Ok(c.page.clone()),
    "PageInner" => |c, _x, _a| Ok(c.page_inner.clone()),
    "Ordinal" => |c, _x, _a| Ok(Value::int(c.ordinal)),
    "Attributes" => |c, _x, _a| Ok(Value::map(c.attributes.attributes())),
    "THead" => |c, _x, _a| Ok(crate::converter::hooks::table_rows_to_value(&c.thead)),
    "TBody" => |c, _x, _a| Ok(crate::converter::hooks::table_rows_to_value(&c.tbody)),
});

impl Object for TableContext {
    nh_common::object_basics!("*tables.tableContext");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/goldmark/tables/tables.go (175 lines; 10/10 funcs executed)
//   types: (group), tableContext
// EX L34-36: New() goldmark.Extender
// EX L38-42: (e *ext) Extend(m goldmark.Markdown)
// EX L44-47: newHTMLRenderer() renderer.NodeRenderer
// EX L49-54: (r *htmlRenderer) RegisterFuncs(reg renderer.NodeRendererFuncRegisterer)
// EX L56-98: (r *htmlRenderer) renderTable(w util.BufWriter, source []byte, n ast.Node, entering bool) (ast.WalkStatus, error)
// EX L100-106: (r *htmlRenderer) peekTable(ctx *render.Context) *hooks.Table
// EX L108-144: (r *htmlRenderer) renderCell(w util.BufWriter, source []byte, node ast.Node, entering bool) (ast.WalkStatus, error)
// EX L146-159: (r *htmlRenderer) renderHeaderOrRow(w util.BufWriter, source []byte, n ast.Node, entering bool) (ast.WalkStatus, error)
// EX L169-171: (c *tableContext) THead() []hooks.TableRow
// EX L173-175: (c *tableContext) TBody() []hooks.TableRow
// ---------------------------------------------------------------------------
