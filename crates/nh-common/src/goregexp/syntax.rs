//! Port of Go's `regexp/syntax` (go1.27.1): the RE2 parser, simplifier and compiler.
//!
//! Unicode classes (`\pL`, `\p{Thai}`, `(?i)` folding) use `go-unicode`'s tables, which are
//! generated from go1.27.1 (Unicode 17.0.0).

pub mod compile;
pub mod parse;
mod perl_groups;
pub mod prog;
pub mod regexp;
pub mod simplify;

pub use compile::compile;
pub use parse::{Error, ErrorCode, parse};
pub use prog::{
    EMPTY_BEGIN_LINE, EMPTY_BEGIN_TEXT, EMPTY_END_LINE, EMPTY_END_TEXT, EMPTY_NO_WORD_BOUNDARY,
    EMPTY_WORD_BOUNDARY, EmptyOp, Inst, InstOp, Prog, empty_op_context, is_word_char,
};
pub use regexp::{
    CLASS_NL, DOT_NL, FOLD_CASE, Flags, LITERAL, MATCH_NL, NON_GREEDY, ONE_LINE, Op, PERL, PERL_X,
    POSIX, Regexp, SIMPLE, UNICODE_GROUPS, WAS_DOLLAR,
};
