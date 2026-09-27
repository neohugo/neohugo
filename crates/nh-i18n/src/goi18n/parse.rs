//! Module `goi18n::parse`.
//!
//! PORT i18n/parse.go
//!
//! Owner: Wave B task T17 (i18n).


use nh_common::Result;

use super::message::Message;

/// Go: `i18n.ParseMessageFileBytes` internals — a map is a message iff it has a reserved key with a
/// string value; otherwise a namespace (`parent.child` ids); a bare string is `{other: value}`.
// Go: go-i18n i18n/parse.go:parseMessageFileBytes
pub fn parse_messages(v: &go_value::Value) -> Result<Vec<Message>> {
    todo!()
}
