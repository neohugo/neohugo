// Go: github.com/yuin/goldmark@v1.7.12/extension/ast/footnote.go

use std::sync::LazyLock;

use super::custom_node;
use crate::ast::{Ast, NodeId, NodeKind, NodeType, new_node_kind};

/// A FootnoteLink struct represents a link to a footnote of Markdown
/// (PHP Markdown Extra) text.
#[derive(Debug, Clone, Default)]
pub struct FootnoteLink {
    pub index: i64,
    pub ref_count: i64,
    pub ref_index: i64,
}
custom_node!(FootnoteLink, |n| vec![
    ("Index".to_string(), n.index.to_string()),
    ("RefCount".to_string(), n.ref_count.to_string()),
    ("RefIndex".to_string(), n.ref_index.to_string()),
]);

/// KindFootnoteLink is a NodeKind of the FootnoteLink node.
pub static KIND_FOOTNOTE_LINK: LazyLock<NodeKind> = LazyLock::new(|| new_node_kind("FootnoteLink"));

// Go: extension/ast/footnote.go:NewFootnoteLink
/// NewFootnoteLink returns a new FootnoteLink node.
pub fn new_footnote_link(ast: &mut Ast, index: i64) -> NodeId {
    ast.new_custom_node(
        *KIND_FOOTNOTE_LINK,
        NodeType::Inline,
        Box::new(FootnoteLink {
            index,
            ref_count: 0,
            ref_index: 0,
        }),
    )
}

/// A FootnoteBacklink struct represents a link to a footnote of Markdown
/// (PHP Markdown Extra) text.
#[derive(Debug, Clone, Default)]
pub struct FootnoteBacklink {
    pub index: i64,
    pub ref_count: i64,
    pub ref_index: i64,
}
custom_node!(FootnoteBacklink, |n| vec![
    ("Index".to_string(), n.index.to_string()),
    ("RefCount".to_string(), n.ref_count.to_string()),
    ("RefIndex".to_string(), n.ref_index.to_string()),
]);

/// KindFootnoteBacklink is a NodeKind of the FootnoteBacklink node.
pub static KIND_FOOTNOTE_BACKLINK: LazyLock<NodeKind> =
    LazyLock::new(|| new_node_kind("FootnoteBacklink"));

// Go: extension/ast/footnote.go:NewFootnoteBacklink
/// NewFootnoteBacklink returns a new FootnoteBacklink node.
pub fn new_footnote_backlink(ast: &mut Ast, index: i64) -> NodeId {
    ast.new_custom_node(
        *KIND_FOOTNOTE_BACKLINK,
        NodeType::Inline,
        Box::new(FootnoteBacklink {
            index,
            ref_count: 0,
            ref_index: 0,
        }),
    )
}

/// A Footnote struct represents a footnote of Markdown
/// (PHP Markdown Extra) text.
#[derive(Debug, Clone, Default)]
pub struct Footnote {
    pub ref_: Vec<u8>,
    pub index: i64,
}
custom_node!(Footnote, |n| vec![
    ("Index".to_string(), n.index.to_string()),
    (
        "Ref".to_string(),
        String::from_utf8_lossy(&n.ref_).into_owned()
    ),
]);

/// KindFootnote is a NodeKind of the Footnote node.
pub static KIND_FOOTNOTE: LazyLock<NodeKind> = LazyLock::new(|| new_node_kind("Footnote"));

// Go: extension/ast/footnote.go:NewFootnote
/// NewFootnote returns a new Footnote node.
pub fn new_footnote(ast: &mut Ast, ref_: Vec<u8>) -> NodeId {
    ast.new_custom_node(
        *KIND_FOOTNOTE,
        NodeType::Block,
        Box::new(Footnote { ref_, index: -1 }),
    )
}

/// A FootnoteList struct represents footnotes of Markdown
/// (PHP Markdown Extra) text.
#[derive(Debug, Clone, Default)]
pub struct FootnoteList {
    pub count: i64,
}
custom_node!(FootnoteList, |n| vec![(
    "Count".to_string(),
    n.count.to_string()
)]);

/// KindFootnoteList is a NodeKind of the FootnoteList node.
pub static KIND_FOOTNOTE_LIST: LazyLock<NodeKind> = LazyLock::new(|| new_node_kind("FootnoteList"));

// Go: extension/ast/footnote.go:NewFootnoteList
/// NewFootnoteList returns a new FootnoteList node.
pub fn new_footnote_list(ast: &mut Ast) -> NodeId {
    ast.new_custom_node(
        *KIND_FOOTNOTE_LIST,
        NodeType::Block,
        Box::new(FootnoteList { count: 0 }),
    )
}
