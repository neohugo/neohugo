//! Go: github.com/tdewolff/parse/v2/html — an HTML5 lexer.

pub mod hash;
pub mod lex;
pub mod util;

pub use hash::*;
pub use lex::*;
pub use util::escape_attr_val;
