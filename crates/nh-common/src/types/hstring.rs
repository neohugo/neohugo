//! Port of `common/types/hstring/stringtypes.go`.
//!
//! Owner: Wave B task T01 (common-values).

use std::any::Any;
use std::borrow::Cow;

use go_value::{GoString, HostCtx, Kind, Object, SafeKind, Value};

/// Go: `hstring.HTML` — a named string type returned by render-hook `.Text`/`.PlainText`.
/// Printed via `String()`; html/template escapers see it as `template.HTML` through
/// `PrintableValue()` (Hugo's `indirect` override in htmltemplate/hugo_template.go).
///
/// As a named basic type it reports its underlying `string` through
/// [`Object::underlying`] (fmt verbs, `cast`, truthiness, hashing see a string). It does not
/// implement `types.Zeroer`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Html(pub GoString);

impl Html {
    /// The template value of this `hstring.HTML`.
    pub fn value(&self) -> Value {
        Value::object(self.clone())
    }

    // Go: common/types/hstring/stringtypes.go:String
    pub fn string(&self) -> GoString {
        self.0.clone()
    }

    // Go: common/types/hstring/stringtypes.go:PrintableValue
    /// PrintableValue returns the value as `template.HTML`.
    pub fn printable_value(&self) -> Value {
        Value::Safe(SafeKind::Html, self.0.clone())
    }
}

impl Object for Html {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("hstring.HTML")
    }
    fn kind(&self) -> Kind {
        // A named string type is a value, not a pointer: `Kind::Struct` is the value-model
        // stand-in; `underlying` gives the real (string) kind.
        Kind::Struct
    }
    fn has_method(&self, name: &str) -> bool {
        matches!(name, "String" | "PrintableValue")
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        name: &str,
        args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        match name {
            "String" => Some(
                crate::object::args::exactly(args, 0, name).map(|_| Value::String(self.string())),
            ),
            "PrintableValue" => {
                Some(crate::object::args::exactly(args, 0, name).map(|_| self.printable_value()))
            }
            _ => None,
        }
    }
    fn go_string(&self) -> Option<GoString> {
        Some(self.0.clone())
    }
    fn printable_value(&self) -> Option<Value> {
        Some(Html::printable_value(self))
    }
    fn underlying(&self) -> Option<Value> {
        Some(Value::String(self.0.clone()))
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
// OK L30-32: (s HTML) String() string
// OK L34-36: (s HTML) PrintableValue() any
// ---------------------------------------------------------------------------
