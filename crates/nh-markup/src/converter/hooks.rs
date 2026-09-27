//! Port of `markup/converter/hooks/hooks.go`.
//!
//! Owner: Wave B task T06 (markup).


//! Go `markup/converter/hooks`: render-hook contexts and renderer interfaces. The contexts are
//! created by the goldmark glue (nh-markup) and handed to the renderer as template *data* values
//! (they implement `go_value::Object`: `.Destination`, `.Text`, `.PageInner` ...). The renderers
//! are implemented by nh-hugolib (`hookRendererTemplate`, executing `_markup/render-*.html`).

use std::any::Any;
use std::borrow::Cow;
use std::sync::Arc;

use go_value::{GoString, HostCtx, Map, Object, Value};
use nh_common::types::hstring::Html;
use nh_common::Result;

/// Go: `hooks.RendererType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RendererType {
    Link,
    Image,
    Heading,
    CodeBlock,
    Passthrough,
    Blockquote,
    Table,
}

/// A render hook (Go returns `any` from `GetRendererFunc`; callers type-assert).
#[derive(Clone)]
pub enum Renderer {
    Link(Arc<dyn LinkRenderer>),
    Heading(Arc<dyn HeadingRenderer>),
    CodeBlock(Arc<dyn CodeBlockRenderer>),
    Blockquote(Arc<dyn BlockquoteRenderer>),
    Table(Arc<dyn TableRenderer>),
    Passthrough(Arc<dyn PassthroughRenderer>),
}

/// Go: `hooks.GetRendererFunc func(t RendererType, id any) any`. `None` = no hook -> default
/// rendering (Go returns nil). Note Hugo *always* has a table hook (embedded render-table).
pub type GetRendererFunc = Arc<dyn Fn(RendererType, &Value) -> Option<Renderer> + Send + Sync>;

/// Go: `hooks.LinkRenderer` (also used for images).
pub trait LinkRenderer: Send + Sync {
    /// `ctx` is a [`LinkContext`] / [`ImageLinkContext`] object value.
    fn render_link(&self, cctx: HostCtx<'_>, w: &mut Vec<u8>, ctx: &Value) -> Result<()>;
}

/// Go: `hooks.HeadingRenderer`.
pub trait HeadingRenderer: Send + Sync {
    fn render_heading(&self, cctx: HostCtx<'_>, w: &mut Vec<u8>, ctx: &Value) -> Result<()>;
}

/// Go: `hooks.CodeBlockRenderer`.
pub trait CodeBlockRenderer: Send + Sync {
    fn render_codeblock(&self, cctx: HostCtx<'_>, w: &mut Vec<u8>, ctx: &Value) -> Result<()>;
    /// Go: `IsDefaultCodeBlockRendererProvider`.
    fn is_default_code_block_renderer(&self) -> bool {
        false
    }
}

/// Go: `hooks.BlockquoteRenderer`.
pub trait BlockquoteRenderer: Send + Sync {
    fn render_blockquote(&self, cctx: HostCtx<'_>, w: &mut Vec<u8>, ctx: &Value) -> Result<()>;
}

/// Go: `hooks.TableRenderer`.
pub trait TableRenderer: Send + Sync {
    fn render_table(&self, cctx: HostCtx<'_>, w: &mut Vec<u8>, ctx: &Value) -> Result<()>;
}

/// Go: `hooks.PassthroughRenderer`.
pub trait PassthroughRenderer: Send + Sync {
    fn render_passthrough(&self, cctx: HostCtx<'_>, w: &mut Vec<u8>, ctx: &Value) -> Result<()>;
}

/// Go: `hooks.TableCell`.
#[derive(Clone, Debug, Default)]
pub struct TableCell {
    pub text: Html,
    /// "left", "right", "center" or "".
    pub alignment: String,
}

/// Go: `hooks.TableRow` (`[]TableCell`, a named slice type `hooks.TableRow`).
pub type TableRow = Vec<TableCell>;

/// Go: `hooks.Table`.
#[derive(Clone, Debug, Default)]
pub struct Table {
    pub thead: Vec<TableRow>,
    pub tbody: Vec<TableRow>,
}

/// `hooks.TableCell` as a template value (fields `Text`, `Alignment`).
impl Object for TableCell {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("hooks.TableCell")
    }
    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Struct
    }
    fn has_method(&self, _name: &str) -> bool {
        false
    }
    fn call_method(&self, _ctx: HostCtx<'_>, _name: &str, _args: &[Value]) -> Option<go_value::Result<Value>> {
        None
    }
    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "Text" => Some(self.text.value()),
            "Alignment" => Some(Value::string(self.alignment.as_str())),
            _ => None,
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// `[]hooks.TableRow` / `hooks.TableRow` as template values.
pub fn table_rows_to_value(rows: &[TableRow]) -> Value {
    todo!("List Named(\"[]hooks.TableRow\") of List Named(\"hooks.TableRow\") of TableCell objects")
}

/// Go: `hooks.AttributesProvider` (`Attributes() map[string]any`), sorted-key map.
pub fn attributes_value(attrs: &Map) -> Value {
    Value::Map(Arc::new(attrs.clone()))
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/converter/hooks/hooks.go (235 lines; 0/0 funcs executed)
//   types: AttributesProvider, LinkContext, ImageLinkContext, CodeblockContext, TableContext, BaseContext,
//          BlockquoteContext, PositionerSourceTargetProvider, PassthroughContext, AttributesOptionsSliceProvider,
//          LinkRenderer, CodeBlockRenderer, BlockquoteRenderer, TableRenderer, PassthroughRenderer,
//          IsDefaultCodeBlockRendererProvider, HeadingContext, PageProvider, HeadingRenderer, ElementPositionResolver,
//          RendererType, GetRendererFunc, TableCell, TableRow, Table
// ---------------------------------------------------------------------------
