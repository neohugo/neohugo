//! Module `goi18n::template`.
//!
//! PORT internal/template.go (text/template, no funcs, lazy parse)
//!
//! Owner: Wave B task T17 (i18n).


use go_value::Value;
use nh_common::Result;

/// Go: go-i18n `internal/template.go` — Go text/template with no funcs, parsed lazily; fast path
/// returns the source when it has no left delimiter. Printing follows text/template (`<no value>`
/// for nil, Go `%v` floats).
#[derive(Clone, Debug)]
pub struct Template {
    pub src: String,
    pub left_delim: String,
    pub right_delim: String,
}

impl Template {
    pub fn execute(&self, data: &Value) -> Result<String> {
        todo!("gotemplate text engine (Wave A)")
    }
}
