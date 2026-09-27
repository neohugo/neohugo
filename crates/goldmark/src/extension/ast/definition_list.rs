// Go: github.com/yuin/goldmark@v1.7.12/extension/ast/definition_list.go

use std::sync::LazyLock;

use super::custom_node;
use crate::ast::{Ast, NodeId, NodeKind, NodeType, new_node_kind};

/// A DefinitionList struct represents a definition list of Markdown
/// (PHPMarkdownExtra) text.
#[derive(Debug, Clone, Default)]
pub struct DefinitionList {
    pub offset: i64,
    /// Go: `TemporaryParagraph *gast.Paragraph` (`None` is nil).
    pub temporary_paragraph: Option<NodeId>,
}
custom_node!(DefinitionList);

/// KindDefinitionList is a NodeKind of the DefinitionList node.
pub static KIND_DEFINITION_LIST: LazyLock<NodeKind> =
    LazyLock::new(|| new_node_kind("DefinitionList"));

// Go: extension/ast/definition_list.go:NewDefinitionList
/// NewDefinitionList returns a new DefinitionList node.
pub fn new_definition_list(ast: &mut Ast, offset: i64, para: Option<NodeId>) -> NodeId {
    ast.new_custom_node(
        *KIND_DEFINITION_LIST,
        NodeType::Block,
        Box::new(DefinitionList {
            offset,
            temporary_paragraph: para,
        }),
    )
}

/// A DefinitionTerm struct represents a definition list term of Markdown
/// (PHPMarkdownExtra) text.
#[derive(Debug, Clone, Default)]
pub struct DefinitionTerm;
custom_node!(DefinitionTerm);

/// KindDefinitionTerm is a NodeKind of the DefinitionTerm node.
pub static KIND_DEFINITION_TERM: LazyLock<NodeKind> =
    LazyLock::new(|| new_node_kind("DefinitionTerm"));

// Go: extension/ast/definition_list.go:NewDefinitionTerm
/// NewDefinitionTerm returns a new DefinitionTerm node.
pub fn new_definition_term(ast: &mut Ast) -> NodeId {
    ast.new_custom_node(
        *KIND_DEFINITION_TERM,
        NodeType::Block,
        Box::new(DefinitionTerm),
    )
}

/// A DefinitionDescription struct represents a definition list description of Markdown
/// (PHPMarkdownExtra) text.
#[derive(Debug, Clone, Default)]
pub struct DefinitionDescription {
    pub is_tight: bool,
}
custom_node!(DefinitionDescription);

/// KindDefinitionDescription is a NodeKind of the DefinitionDescription node.
pub static KIND_DEFINITION_DESCRIPTION: LazyLock<NodeKind> =
    LazyLock::new(|| new_node_kind("DefinitionDescription"));

// Go: extension/ast/definition_list.go:NewDefinitionDescription
/// NewDefinitionDescription returns a new DefinitionDescription node.
pub fn new_definition_description(ast: &mut Ast) -> NodeId {
    ast.new_custom_node(
        *KIND_DEFINITION_DESCRIPTION,
        NodeType::Block,
        Box::new(DefinitionDescription::default()),
    )
}
