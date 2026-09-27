// Go: github.com/yuin/goldmark@v1.7.12/ast/block.go

use std::any::Any;
use std::collections::BTreeMap;
use std::sync::Arc;

use super::{Ast, NodeId, NodeValue};
use crate::text::{Segment, new_segment};

/// A Document struct is a root node of Markdown text.
#[derive(Debug, Default)]
pub struct Document {
    meta: Option<BTreeMap<String, Arc<dyn Any + Send + Sync>>>,
}

impl Document {
    // Go: ast/block.go:Document.Meta
    /// Meta returns metadata of this document.
    pub fn meta(&mut self) -> &mut BTreeMap<String, Arc<dyn Any + Send + Sync>> {
        self.meta.get_or_insert_with(BTreeMap::new)
    }

    /// Meta without creating it (read access for renderers, which only
    /// hold a shared AST; `None` is Go's nil map).
    pub fn get_meta(&self) -> Option<&BTreeMap<String, Arc<dyn Any + Send + Sync>>> {
        self.meta.as_ref()
    }

    // Go: ast/block.go:Document.SetMeta
    /// SetMeta sets given metadata to this document.
    pub fn set_meta(&mut self, meta: BTreeMap<String, Arc<dyn Any + Send + Sync>>) {
        let m = self.meta.get_or_insert_with(BTreeMap::new);
        for (k, v) in meta {
            m.insert(k, v);
        }
    }

    // Go: ast/block.go:Document.AddMeta
    /// AddMeta adds given metadata to this document.
    pub fn add_meta(&mut self, key: &str, value: Arc<dyn Any + Send + Sync>) {
        self.meta
            .get_or_insert_with(BTreeMap::new)
            .insert(key.to_string(), value);
    }
}

/// A Heading struct represents headings like SetextHeading and ATXHeading.
#[derive(Debug, Clone, Copy)]
pub struct Heading {
    /// Level returns a level of this heading.
    /// This value is between 1 and 6.
    pub level: i64,
}

/// A FencedCodeBlock struct represents a fenced code block of Markdown text.
#[derive(Debug, Clone, Copy)]
pub struct FencedCodeBlock {
    /// Info returns a info text of this fenced code block (a detached Text node).
    pub info: Option<NodeId>,
}

/// A List struct represents a list of Markdown text.
#[derive(Debug, Clone, Copy)]
pub struct List {
    /// Marker is a marker character like '-', '+', ')' and '.'.
    pub marker: u8,
    /// IsTight is a true if this list is a 'tight' list.
    /// See https://spec.commonmark.org/0.30/#loose for details.
    pub is_tight: bool,
    /// Start is an initial number of this ordered list.
    /// If this list is not an ordered list, Start is 0.
    pub start: i64,
}

impl List {
    // Go: ast/block.go:List.IsOrdered
    /// IsOrdered returns true if this list is an ordered list, otherwise false.
    pub fn is_ordered(&self) -> bool {
        self.marker == b'.' || self.marker == b')'
    }

    // Go: ast/block.go:List.CanContinue
    /// CanContinue returns true if this list can continue with
    /// the given mark and a list type, otherwise false.
    pub fn can_continue(&self, marker: u8, is_ordered: bool) -> bool {
        marker == self.marker && is_ordered == self.is_ordered()
    }
}

/// A ListItem struct represents a list item of Markdown text.
#[derive(Debug, Clone, Copy)]
pub struct ListItem {
    /// Offset is an offset position of this item.
    pub offset: i64,
}

/// HTMLBlockType represents kinds of an html blocks.
/// See https://spec.commonmark.org/0.30/#html-blocks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HTMLBlockType {
    Type1 = 1,
    Type2 = 2,
    Type3 = 3,
    Type4 = 4,
    Type5 = 5,
    Type6 = 6,
    Type7 = 7,
}

/// An HTMLBlock struct represents an html block of Markdown text.
#[derive(Debug, Clone, Copy)]
pub struct HTMLBlock {
    /// Type is a type of this html block.
    pub html_block_type: HTMLBlockType,
    /// ClosureLine is a line that closes this html block.
    pub closure_line: Segment,
}

impl HTMLBlock {
    // Go: ast/block.go:HTMLBlock.HasClosure
    /// HasClosure returns true if this html block has a closure line,
    /// otherwise false.
    pub fn has_closure(&self) -> bool {
        self.closure_line.start >= 0
    }
}

impl Ast {
    // Go: ast/block.go:NewDocument
    /// NewDocument returns a new Document node.
    pub fn new_document(&mut self) -> NodeId {
        self.new_node(NodeValue::Document(Document::default()))
    }

    // Go: ast/block.go:NewTextBlock
    /// NewTextBlock returns a new TextBlock node.
    pub fn new_text_block(&mut self) -> NodeId {
        self.new_node(NodeValue::TextBlock)
    }

    // Go: ast/block.go:NewParagraph
    /// NewParagraph returns a new Paragraph node.
    pub fn new_paragraph(&mut self) -> NodeId {
        self.new_node(NodeValue::Paragraph)
    }

    // Go: ast/block.go:IsParagraph
    /// IsParagraph returns true if the given node implements the Paragraph interface,
    /// otherwise false.
    pub fn is_paragraph(&self, node: Option<NodeId>) -> bool {
        matches!(node.map(|n| self.value(n)), Some(NodeValue::Paragraph))
    }

    // Go: ast/block.go:NewHeading
    /// NewHeading returns a new Heading node.
    pub fn new_heading(&mut self, level: i64) -> NodeId {
        self.new_node(NodeValue::Heading(Heading { level }))
    }

    // Go: ast/block.go:NewThematicBreak
    /// NewThematicBreak returns a new ThematicBreak node.
    pub fn new_thematic_break(&mut self) -> NodeId {
        self.new_node(NodeValue::ThematicBreak)
    }

    // Go: ast/block.go:NewCodeBlock
    /// NewCodeBlock returns a new CodeBlock node.
    pub fn new_code_block(&mut self) -> NodeId {
        self.new_node(NodeValue::CodeBlock)
    }

    // Go: ast/block.go:NewFencedCodeBlock
    /// NewFencedCodeBlock return a new FencedCodeBlock node.
    pub fn new_fenced_code_block(&mut self, info: Option<NodeId>) -> NodeId {
        self.new_node(NodeValue::FencedCodeBlock(FencedCodeBlock { info }))
    }

    // Go: ast/block.go:FencedCodeBlock.Language
    /// Language returns an language in an info string.
    /// Language returns nil if this node does not have an info string.
    /// (Go caches the value in the node; it is recomputed here.)
    pub fn fenced_code_block_language(&self, n: NodeId, source: &[u8]) -> Option<Vec<u8>> {
        let NodeValue::FencedCodeBlock(f) = self.value(n) else {
            return None;
        };
        let info_id = f.info?;
        let NodeValue::Text(t) = self.value(info_id) else {
            return None;
        };
        let info = t.segment.value(source);
        let mut i = 0;
        while i < info.len() {
            if info[i] == b' ' {
                break;
            }
            i += 1;
        }
        Some(info[..i].to_vec())
    }

    // Go: ast/block.go:NewBlockquote
    /// NewBlockquote returns a new Blockquote node.
    pub fn new_blockquote(&mut self) -> NodeId {
        self.new_node(NodeValue::Blockquote)
    }

    // Go: ast/block.go:NewList
    /// NewList returns a new List node.
    pub fn new_list(&mut self, marker: u8) -> NodeId {
        self.new_node(NodeValue::List(List {
            marker,
            is_tight: true,
            start: 0,
        }))
    }

    // Go: ast/block.go:NewListItem
    /// NewListItem returns a new ListItem node.
    pub fn new_list_item(&mut self, offset: i64) -> NodeId {
        self.new_node(NodeValue::ListItem(ListItem { offset }))
    }

    // Go: ast/block.go:NewHTMLBlock
    /// NewHTMLBlock returns a new HTMLBlock node.
    pub fn new_html_block(&mut self, typ: HTMLBlockType) -> NodeId {
        self.new_node(NodeValue::HTMLBlock(HTMLBlock {
            html_block_type: typ,
            closure_line: new_segment(-1, -1),
        }))
    }

    /// The Heading fields of `n`, if it is a heading.
    pub fn heading(&self, n: NodeId) -> Option<&Heading> {
        match self.value(n) {
            NodeValue::Heading(h) => Some(h),
            _ => None,
        }
    }

    /// The List fields of `n`, if it is a list.
    pub fn list(&self, n: NodeId) -> Option<&List> {
        match self.value(n) {
            NodeValue::List(l) => Some(l),
            _ => None,
        }
    }

    /// The List fields of `n`, mutably.
    pub fn list_mut(&mut self, n: NodeId) -> Option<&mut List> {
        match self.value_mut(n) {
            NodeValue::List(l) => Some(l),
            _ => None,
        }
    }

    /// The ListItem fields of `n`, if it is a list item.
    pub fn list_item(&self, n: NodeId) -> Option<&ListItem> {
        match self.value(n) {
            NodeValue::ListItem(l) => Some(l),
            _ => None,
        }
    }

    /// The HTMLBlock fields of `n`, if it is an HTML block.
    pub fn html_block(&self, n: NodeId) -> Option<&HTMLBlock> {
        match self.value(n) {
            NodeValue::HTMLBlock(h) => Some(h),
            _ => None,
        }
    }

    /// The HTMLBlock fields of `n`, mutably.
    pub fn html_block_mut(&mut self, n: NodeId) -> Option<&mut HTMLBlock> {
        match self.value_mut(n) {
            NodeValue::HTMLBlock(h) => Some(h),
            _ => None,
        }
    }

    /// The FencedCodeBlock fields of `n`, if it is a fenced code block.
    pub fn fenced_code_block(&self, n: NodeId) -> Option<&FencedCodeBlock> {
        match self.value(n) {
            NodeValue::FencedCodeBlock(f) => Some(f),
            _ => None,
        }
    }

    /// The Document fields of `n`, if it is a Document.
    pub fn document(&self, n: NodeId) -> Option<&Document> {
        match self.value(n) {
            NodeValue::Document(d) => Some(d),
            _ => None,
        }
    }

    /// The Document fields of `n`, mutably.
    pub fn document_mut(&mut self, n: NodeId) -> Option<&mut Document> {
        match self.value_mut(n) {
            NodeValue::Document(d) => Some(d),
            _ => None,
        }
    }
}
