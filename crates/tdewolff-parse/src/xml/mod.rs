//! Go: github.com/tdewolff/parse/v2/xml — an XML1.0 lexer.

pub mod lex;
pub mod util;

pub use lex::*;
pub use util::{escape_attr_val, escape_cdata_val};
