//! Byte-exact Rust port of `github.com/tdewolff/parse/v2@v2.8.1/js`: the
//! ECMAScript lexer ([`Lexer`]), the parser ([`parse`]) with automatic
//! semicolon insertion, regular-expression detection, arrow-function
//! speculation and scope analysis, and the AST ([`Ast`], an arena of
//! [`Node`]s addressed by [`NodeId`], with [`Scope`]s addressed by
//! [`ScopeId`]).
//!
//! See `PORTING.md` for the module map, the arena model and deviations.

// Faithful-port allowances: Go names for constants (`ErrorToken`,
// `OpAssign`, `NoDecl`, ...), Go-style control flow (else after return,
// explicit `return` in match arms, long boolean chains mirroring Go's
// precedence), and a parser with many parameters/locals.
#![allow(non_upper_case_globals)]
#![allow(clippy::needless_return)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::nonminimal_bool)]
#![allow(clippy::precedence)]
#![allow(clippy::len_zero)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::should_implement_trait)] // Go's Lexer.Next is not an Iterator
#![allow(clippy::new_without_default)]
#![allow(clippy::comparison_chain)]
#![allow(clippy::if_same_then_else)]

pub mod ast;
mod ast_js;
mod ast_string;
pub mod dump;
pub mod lex;
pub mod parse;
pub mod table;
pub mod tokentype;
pub mod util;

pub use ast::*;
pub use ast_js::{ERR_INVALID_JSON, JsWriter};
pub use lex::{Lexer, is_identifier_continue, is_identifier_end, is_identifier_start};
pub use parse::{NESTED_EXPR_LIMIT, NESTED_STMT_LIMIT, Options, parse};
pub use table::*;
pub use tokentype::*;
pub use util::{as_decimal_literal, as_identifier_name};
