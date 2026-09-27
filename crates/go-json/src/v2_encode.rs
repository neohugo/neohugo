//! Port of `encoding/json/v2_encode.go` (go1.27.1): `Marshal` and
//! `MarshalIndent` (implemented with encoding/json/v2 and v1 options).

use go_value::Value;

use crate::arshal;
use crate::error::Error;
use crate::jsonflags;
use crate::jsonopts::{Opt, bool_opt, default_options_v1};
use crate::v2_indent::append_indent;
use crate::v2_inject::transform_marshal_error;

// Go: v2_encode.go:Marshal
/// Marshal returns the JSON encoding of v, with `<`, `>` and `&` escaped
/// in strings (Go's default). See the crate documentation for how each
/// [`Value`] is encoded.
pub fn marshal(v: &Value) -> Result<Vec<u8>, Error> {
    // Go: jsonv2.Marshal(v, DefaultOptionsV1())
    arshal::marshal(v, &[Opt::Struct(Box::new(default_options_v1()))])
        .map_err(transform_marshal_error)
}

/// Like [`marshal`], with HTML escaping chosen by the caller: the bytes an
/// `Encoder` with `SetEscapeHTML(escape_html)` writes, without its trailing
/// newline. (Convenience; Go has no such function.)
pub fn marshal_with(v: &Value, escape_html: bool) -> Result<Vec<u8>, Error> {
    arshal::marshal(
        v,
        &[
            Opt::Struct(Box::new(default_options_v1())),
            bool_opt(jsonflags::ESCAPE_FOR_HTML, escape_html),
        ],
    )
    .map_err(transform_marshal_error)
}

// Go: v2_encode.go:MarshalIndent
/// MarshalIndent is like [`marshal`] but applies [`crate::indent`] to format the output.
/// Each JSON element in the output will begin on a new line beginning with prefix
/// followed by one or more copies of indent according to the indentation nesting.
pub fn marshal_indent(
    v: &Value,
    prefix: impl AsRef<[u8]>,
    indent: impl AsRef<[u8]>,
) -> Result<Vec<u8>, Error> {
    let b = marshal(v)?;
    let mut out = Vec::new();
    append_indent(&mut out, &b, prefix.as_ref(), indent.as_ref())?;
    Ok(out)
}
