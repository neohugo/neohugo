//! Port of `markup/internal/attributes/attributes.go`.
//!
//! Owner: Wave B task T06 (markup).


use std::sync::Arc;

use go_value::{Map, Value};

/// Go: `attributes.AttributesOwnerType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttributesOwnerType {
    General,
    CodeBlock,
}

/// Go: `attributes.Attribute`.
#[derive(Clone, Debug)]
pub struct Attribute {
    pub name: String,
    pub value: Value,
}

/// Go: `attributes.AttributesHolder` — node attributes as a sorted `map[string]any` plus options.
#[derive(Clone, Debug, Default)]
pub struct AttributesHolder {
    pub attributes: Vec<Attribute>,
    pub options: Vec<Attribute>,
}

impl AttributesHolder {
    /// Go: `Attributes() map[string]any`.
    pub fn attributes(&self) -> Map {
        todo!()
    }
}

/// Go: `attributes.Empty`.
pub fn empty() -> Arc<AttributesHolder> {
    Arc::new(AttributesHolder::default())
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/internal/attributes/attributes.go (225 lines; 3/9 funcs executed)
//   types: AttributesOwnerType, Attribute, AttributesHolder, Attributes
// EX L43-47: init()
// EX L57-117: New(astAttributes []ast.Attribute, ownerType AttributesOwnerType) *AttributesHolder
//    L124-126: (a Attribute) ValueString() string
// EX L147-155: (a *AttributesHolder) Attributes() map[string]any
//    L157-165: (a *AttributesHolder) Options() map[string]any
//    L167-169: (a *AttributesHolder) AttributesSlice() []Attribute
//    L171-173: (a *AttributesHolder) OptionsSlice() []Attribute
//    L178-200: RenderASTAttributes(w hugio.FlexiWriter, attributes ...ast.Attribute)
//    L205-225: RenderAttributes(w hugio.FlexiWriter, skipClass bool, attributes ...Attribute)
// ---------------------------------------------------------------------------
