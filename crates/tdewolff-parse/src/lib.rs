//! Byte-exact Rust port of `github.com/tdewolff/parse/v2@v2.8.1` (all
//! packages except `js`, which lives in the `tdewolff-parse-js` crate).
//!
//! Go `[]byte` values are modelled by [`GoBytes`] (shared backing array with
//! Go slice/append/aliasing semantics) and `*parse.Input` by [`Input`] (a
//! shared handle). See `PORTING.md` for the module map and deviations.

// Faithful-port allowances: Go names (constants like `html::Svg`,
// `css::Font_Face`), Go-style loops and long boolean chains.
#![allow(non_upper_case_globals)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::nonminimal_bool)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::len_zero)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::enum_variant_names)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::comparison_chain)]
#![allow(clippy::needless_return)]
#![allow(clippy::precedence)]
#![allow(clippy::should_implement_trait)] // Go's Lexer.Next is not an Iterator
#![allow(clippy::if_same_then_else)]

pub mod base64;
pub mod buffer;
pub mod common;
pub mod css;
pub mod error;
pub mod gobytes;
pub mod gomath;
pub mod html;
pub mod input;
pub mod json;
pub mod position;
pub mod strconv;
pub mod utf8;
pub mod util;
pub mod xml;

pub use common::{
    DATA_URI_ENCODING_TABLE, EntityMap, NilMap, Params, RevEntityMap, URL_ENCODING_TABLE,
    append_escape, data_uri, decode_url, dimension, encode_url, mediatype, number, quote_entity,
    replace_entities, replace_multiple_whitespace, replace_multiple_whitespace_and_entities,
};
pub use error::{Error, GoError, is_eof, new_error, new_error_lexer, new_error_lexer_err};
pub use gobytes::{ByteView, GoBytes, SubView};
pub use input::{GoReader, Input, IoReader, read_all};
pub use position::{position, position_input};
pub use util::{
    Indenter, copy, equal_fold, is_all_whitespace, is_newline, is_whitespace, printable, to_lower,
    to_lower_slice, trim_whitespace, trim_whitespace_bounds,
};
