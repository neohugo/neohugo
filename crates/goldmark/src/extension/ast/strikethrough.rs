// Go: github.com/yuin/goldmark@v1.7.12/extension/ast/strikethrough.go

use std::sync::LazyLock;

use super::custom_node;
use crate::ast::{Ast, NodeId, NodeKind, NodeType, new_node_kind};

/// A Strikethrough struct represents a strikethrough of GFM text.
#[derive(Debug, Clone, Default)]
pub struct Strikethrough;
custom_node!(Strikethrough);

/// KindStrikethrough is a NodeKind of the Strikethrough node.
pub static KIND_STRIKETHROUGH: LazyLock<NodeKind> =
    LazyLock::new(|| new_node_kind("Strikethrough"));

// Go: extension/ast/strikethrough.go:NewStrikethrough
/// NewStrikethrough returns a new Strikethrough node.
pub fn new_strikethrough(ast: &mut Ast) -> NodeId {
    ast.new_custom_node(
        *KIND_STRIKETHROUGH,
        NodeType::Inline,
        Box::new(Strikethrough),
    )
}
