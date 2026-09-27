//! Port of Go's `encoding/json` v1 API as shipped in go1.27.1 over
//! [`go_value::Value`].
//!
//! go1.27.1 builds with `GOEXPERIMENT=jsonv2` on by default, so the v1 API
//! (`json.Marshal`, `json.NewEncoder`, `json.Unmarshal`, ...) is the thin
//! layer of `encoding/json/v2_*.go` over `encoding/json/v2` (arshaling) and
//! `encoding/json/jsontext` (syntax), with the v1 compatibility options.
//! This crate ports that stack; the classic `encode.go`/`decode.go`/
//! `scanner.go` implementation is not what the golden toolchain runs.
//!
//! Encoding ([`marshal`], [`marshal_indent`], [`Encoder`]):
//!
//! - `Invalid` (nil interface) and `TypedNil` (nil pointer/map/slice) → `null`
//! - `Bool`, `Int`, `Uint` → `true`/`false`, decimal
//! - `Float` → `'f'` format with shortest digits unless `|f| < 1e-6` or
//!   `|f| >= 1e21` (float32 values compare as float32), then `'e'` with
//!   `e-07` → `e-7`; NaN/±Inf are an [`Error::UnsupportedValue`]
//! - `String`/`Safe` → JSON string: `\"`, `\\`, `\b \f \n \r \t`, other
//!   control bytes as lower-case `\u00xx`, U+2028/U+2029 escaped, `<`, `>`,
//!   `&` escaped when HTML escaping is on (the default), invalid UTF-8
//!   replaced by a literal U+FFFD
//! - `Time` → RFC 3339 with nanoseconds (errors for years outside
//!   [0,9999] and zone offsets of 24h or more)
//! - `List` → array; a `[]uint8` list → base64 string
//! - `Map` → object with keys in byte order
//! - `Object` → `marshal_json` (validated and reformatted, HTML-escaped
//!   like Go), else `marshal_text` (as a string), else by kind:
//!   `map_keys`/`map_get`, `list`, or `struct_fields`; [`JsonStruct`]
//!   describes a Go struct with `json` tags; [`Number`] and [`RawMessage`]
//!   are Go's `json.Number` and `json.RawMessage`
//!
//! Decoding ([`unmarshal`], [`unmarshal_map`], [`Decoder`]) produces what
//! Go stores in an `interface{}`: `map[string]interface {}` (`Map` of
//! `MapType::StringAny`, last duplicate name wins), `[]interface {}`,
//! float64 (or [`Number`] with `UseNumber`), string, bool and nil.
//!
//! [`valid`], [`compact`], [`indent`] and [`html_escape`] are the
//! `v2_scanner.go`/`v2_indent.go` functions. Errors carry Go's exact
//! messages and offsets.

// Lints that fight a faithful line-by-line port.
#![allow(clippy::needless_range_loop)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::precedence)]
#![allow(clippy::nonminimal_bool)]
#![allow(clippy::comparison_chain)]
#![allow(clippy::type_complexity)]
#![allow(clippy::if_same_then_else)]
#![allow(clippy::needless_late_init)]
#![allow(clippy::let_and_return)]

mod arshal;
mod encode_struct;
mod error;
mod goerr;
mod jsonflags;
mod jsonopts;
mod jsontext;
mod jsonwire;
mod stack;
mod v2_decode;
mod v2_encode;
mod v2_indent;
mod v2_inject;
mod v2_scanner;
mod v2_stream;

pub use arshal::methods::{Number, RawMessage};
pub use encode_struct::{JsonField, JsonStruct};
pub use error::{Error, MarshalerError, SyntaxError, UnmarshalTypeError};
pub use v2_decode::{unmarshal, unmarshal_map, unmarshal_map_partial, unmarshal_partial};
pub use v2_encode::{marshal, marshal_indent, marshal_with};
pub use v2_indent::{compact, html_escape, indent};
pub use v2_scanner::valid;
pub use v2_stream::{Decoder, Encoder, Token};
