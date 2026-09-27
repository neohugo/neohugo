//! Go: parse/v2/js/util.go

use tdewolff_parse::ByteView;

use crate::ast::{Ast, Node, NodeId};
use crate::lex::{IDENTIFIER_START_TABLE, IDENTIFIER_TABLE};

// Go: util.go:isLHSExpr
/// A nil expression is an LHS expression (as in Go, where a nil interface
/// matches none of the listed types).
pub(crate) fn is_lhs_expr(ast: &Ast, i: NodeId) -> bool {
    !matches!(
        ast.node(i),
        Node::CommaExpr(_)
            | Node::CondExpr(_)
            | Node::YieldExpr(_)
            | Node::ArrowFunc(_)
            | Node::BinaryExpr(_)
            | Node::UnaryExpr(_)
    )
}

// Go: util.go:AsIdentifierName
/// Returns true if a valid identifier name is given.
pub fn as_identifier_name<B: ByteView + ?Sized>(b: &B) -> bool {
    if b.len() == 0 || !IDENTIFIER_START_TABLE[b.at(0) as usize] {
        return false;
    }

    let mut i = 1;
    while i < b.len() {
        if IDENTIFIER_TABLE[b.at(i) as usize] {
            i += 1;
        } else {
            return false;
        }
    }
    true
}

// Go: util.go:AsDecimalLiteral
/// Returns true if a valid decimal literal is given.
pub fn as_decimal_literal<B: ByteView + ?Sized>(b: &B) -> bool {
    if b.len() == 0 || (b.at(0) < b'0' || b'9' < b.at(0)) && (b.at(0) != b'.' || b.len() == 1) {
        return false;
    } else if b.at(0) == b'0' {
        return b.len() == 1;
    }
    let mut i = 1;
    while i < b.len() && b'0' <= b.at(i) && b.at(i) <= b'9' {
        i += 1;
    }
    if i < b.len() && b.at(i) == b'.' && b.at(0) != b'.' {
        i += 1;
        while i < b.len() && b'0' <= b.at(i) && b.at(i) <= b'9' {
            i += 1;
        }
    }
    i == b.len()
}
