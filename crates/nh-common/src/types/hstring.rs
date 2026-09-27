//! Port of `common/types/hstring/stringtypes.go`.
//!
//! Owner: Wave B task T01 (common-values).


use std::any::Any;
use std::borrow::Cow;

use go_value::{GoString, HostCtx, Object, SafeKind, Value};

/// Go: `hstring.HTML` — a named string type returned by render-hook `.Text`/`.PlainText`.
/// Printed via `String()`; html/template escapers see it as `template.HTML` through
/// `PrintableValue()` (Hugo's `indirect` override in htmltemplate/hugo_template.go).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Html(pub GoString);

impl Html {
    pub fn value(&self) -> Value {
        Value::object(self.clone())
    }
}

impl Object for Html {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("hstring.HTML")
    }
    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Struct
    }
    fn has_method(&self, name: &str) -> bool {
        matches!(name, "String" | "PrintableValue")
    }
    fn call_method(&self, _ctx: HostCtx<'_>, name: &str, _args: &[Value]) -> Option<go_value::Result<Value>> {
        match name {
            "String" => Some(Ok(Value::String(self.0.clone()))),
            "PrintableValue" => Some(Ok(Value::Safe(SafeKind::Html, self.0.clone()))),
            _ => None,
        }
    }
    fn is_zero(&self) -> Option<bool> {
        // A string kind: truthiness is len > 0 (not a Zeroer).
        Some(self.0.is_empty())
    }
    fn go_string(&self) -> Option<GoString> {
        Some(self.0.clone())
    }
    fn printable_value(&self) -> Option<Value> {
        Some(Value::Safe(SafeKind::Html, self.0.clone()))
    }
    fn marshal_json(&self) -> Option<go_value::Result<Vec<u8>>> {
        None
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/types/hstring/stringtypes.go (36 lines; 2/2 funcs executed)
//   types: HTML
// EX L30-32: (s HTML) String() string
// EX L34-36: (s HTML) PrintableValue() any
// ---------------------------------------------------------------------------
