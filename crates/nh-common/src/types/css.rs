//! Port of `common/types/css/csstypes.go`.
//!
//! Owner: Wave B task T01 (common-values).

use go_value::{GoString, Kind, Object, Value};

/// Go: `css.QuotedString` — a string that needs to be quoted in CSS (`css.Quoted`, Sass vars;
/// unused by seeksnack). A named string type: its `underlying` value is the string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuotedString(pub GoString);

/// Go: `css.UnquotedString` — a string that does not need to be quoted in CSS.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnquotedString(pub GoString);

crate::go_methods!(QuotedString {});
crate::go_methods!(UnquotedString {});

impl Object for QuotedString {
    crate::object_basics!("css.QuotedString");
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn underlying(&self) -> Option<Value> {
        Some(Value::String(self.0.clone()))
    }
}

impl Object for UnquotedString {
    crate::object_basics!("css.UnquotedString");
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn underlying(&self) -> Option<Value> {
        Some(Value::String(self.0.clone()))
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/types/css/csstypes.go (20 lines; 0/0 funcs executed)
//   types: QuotedString, UnquotedString
// ---------------------------------------------------------------------------
