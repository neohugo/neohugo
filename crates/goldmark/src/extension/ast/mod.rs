//! Go: github.com/yuin/goldmark@v1.7.12/extension/ast — AST nodes that
//! represent extension's elements.
//!
//! Every Go node type is a payload struct implementing
//! [`crate::ast::CustomNode`]; the Go `NewX` constructors allocate the node
//! in an [`Ast`] arena and return its id. Read a payload back with
//! `ast.custom::<Table>(n)` (or the typed helpers below).

use crate::ast::{Ast, NodeId};

mod definition_list;
mod footnote;
mod strikethrough;
mod table;
mod tasklist;

pub use definition_list::*;
pub use footnote::*;
pub use strikethrough::*;
pub use table::*;
pub use tasklist::*;

/// Implements [`crate::ast::CustomNode`] with only the downcasting hooks
/// (and optional `Dump` fields).
macro_rules! custom_node {
    ($t:ty) => {
        impl $crate::ast::CustomNode for $t {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }
        }
    };
    ($t:ty, |$s:ident| $fields:expr) => {
        impl $crate::ast::CustomNode for $t {
            fn dump_fields(&self, _source: &[u8]) -> Vec<(String, String)> {
                let $s = self;
                $fields
            }
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }
        }
    };
}
pub(crate) use custom_node;

/// The payload of `n` downcast to `T`, panicking like a failed Go type
/// assertion (`node.(*ast.T)`) when `n` is not a `T`.
pub(crate) fn must<'a, T: 'static>(ast: &'a Ast, n: NodeId, what: &str) -> &'a T {
    ast.custom::<T>(n)
        .unwrap_or_else(|| panic!("interface conversion: ast.Node is not *ast.{what}"))
}

/// Mutable [`must`].
pub(crate) fn must_mut<'a, T: 'static>(ast: &'a mut Ast, n: NodeId, what: &str) -> &'a mut T {
    ast.custom_mut::<T>(n)
        .unwrap_or_else(|| panic!("interface conversion: ast.Node is not *ast.{what}"))
}
