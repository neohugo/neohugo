//! Go: github.com/yuin/goldmark@v1.7.12/ast — AST nodes that represent
//! markdown elements.
//!
//! Go's `ast.Node` interface values are heap objects linked by pointers. The
//! port stores every node in an arena ([`Ast`]) and refers to nodes by
//! [`NodeId`]; the parent / first-child / last-child / sibling links, the
//! child count (including Go's bookkeeping quirks) and the attributes live in
//! the arena slot, and the node-specific fields in [`NodeValue`]. Ids are
//! never reused, so id equality is Go pointer identity.

use std::any::Any;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, LazyLock, RwLock};

use crate::text::Segments;

mod block;
mod inline;

pub use block::*;
pub use inline::*;

/// A NodeType indicates what type a node belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeType {
    /// TypeBlock indicates that a node is kind of block nodes.
    Block = 1,
    /// TypeInline indicates that a node is kind of inline nodes.
    Inline = 2,
    /// TypeDocument indicates that a node is kind of document nodes.
    Document = 3,
}

/// NodeKind indicates more specific type than NodeType.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct NodeKind(pub u32);

static KIND_NAMES: LazyLock<RwLock<Vec<String>>> = LazyLock::new(|| {
    RwLock::new(
        [
            "",
            // ast/block.go
            "Document",
            "TextBlock",
            "Paragraph",
            "Heading",
            "ThematicBreak",
            "CodeBlock",
            "FencedCodeBlock",
            "Blockquote",
            "List",
            "ListItem",
            "HTMLBlock",
            // ast/inline.go
            "Text",
            "String",
            "CodeSpan",
            "Emphasis",
            "Link",
            "Image",
            "AutoLink",
            "RawHTML",
            // parser/delimiter.go, parser/link.go
            "Delimiter",
            "LinkLabelState",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect(),
    )
});

impl NodeKind {
    // Go: ast/ast.go:NodeKind.String
    /// The name the kind was registered with.
    pub fn name(&self) -> String {
        KIND_NAMES.read().unwrap()[self.0 as usize].clone()
    }
}

impl fmt::Display for NodeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name())
    }
}

// Go: ast/ast.go:NewNodeKind
/// NewNodeKind returns a new Kind value.
///
/// Extensions allocate their kinds once, e.g. in a `LazyLock`.
pub fn new_node_kind(name: &str) -> NodeKind {
    let mut names = KIND_NAMES.write().unwrap();
    names.push(name.to_string());
    NodeKind((names.len() - 1) as u32)
}

/// KindDocument is a NodeKind of the Document node.
pub const KIND_DOCUMENT: NodeKind = NodeKind(1);
/// KindTextBlock is a NodeKind of the TextBlock node.
pub const KIND_TEXT_BLOCK: NodeKind = NodeKind(2);
/// KindParagraph is a NodeKind of the Paragraph node.
pub const KIND_PARAGRAPH: NodeKind = NodeKind(3);
/// KindHeading is a NodeKind of the Heading node.
pub const KIND_HEADING: NodeKind = NodeKind(4);
/// KindThematicBreak is a NodeKind of the ThematicBreak node.
pub const KIND_THEMATIC_BREAK: NodeKind = NodeKind(5);
/// KindCodeBlock is a NodeKind of the CodeBlock node.
pub const KIND_CODE_BLOCK: NodeKind = NodeKind(6);
/// KindFencedCodeBlock is a NodeKind of the FencedCodeBlock node.
pub const KIND_FENCED_CODE_BLOCK: NodeKind = NodeKind(7);
/// KindBlockquote is a NodeKind of the Blockquote node.
pub const KIND_BLOCKQUOTE: NodeKind = NodeKind(8);
/// KindList is a NodeKind of the List node.
pub const KIND_LIST: NodeKind = NodeKind(9);
/// KindListItem is a NodeKind of the ListItem node.
pub const KIND_LIST_ITEM: NodeKind = NodeKind(10);
/// KindHTMLBlock is a NodeKind of the HTMLBlock node.
pub const KIND_HTML_BLOCK: NodeKind = NodeKind(11);
/// KindText is a NodeKind of the Text node.
pub const KIND_TEXT: NodeKind = NodeKind(12);
/// KindString is a NodeKind of the String node.
pub const KIND_STRING: NodeKind = NodeKind(13);
/// KindCodeSpan is a NodeKind of the CodeSpan node.
pub const KIND_CODE_SPAN: NodeKind = NodeKind(14);
/// KindEmphasis is a NodeKind of the Emphasis node.
pub const KIND_EMPHASIS: NodeKind = NodeKind(15);
/// KindLink is a NodeKind of the Link node.
pub const KIND_LINK: NodeKind = NodeKind(16);
/// KindImage is a NodeKind of the Image node.
pub const KIND_IMAGE: NodeKind = NodeKind(17);
/// KindAutoLink is a NodeKind of the AutoLink node.
pub const KIND_AUTO_LINK: NodeKind = NodeKind(18);
/// KindRawHTML is a NodeKind of the RawHTML node.
pub const KIND_RAW_HTML: NodeKind = NodeKind(19);
/// parser.kindDelimiter.
pub const KIND_DELIMITER: NodeKind = NodeKind(20);
/// parser.kindLinkLabelState.
pub const KIND_LINK_LABEL_STATE: NodeKind = NodeKind(21);

/// An attribute value (Go `interface{}`) as produced by goldmark's attribute
/// parser or set by extensions.
#[derive(Clone)]
pub enum AttrValue {
    /// Go `[]byte` (the attribute parser's strings and names).
    Bytes(Vec<u8>),
    /// Go `string` (may hold arbitrary bytes).
    String(Vec<u8>),
    /// Go `float64` (attribute numbers).
    Float(f64),
    /// Go `bool` (`true` / `false` literals).
    Bool(bool),
    /// Go `nil` (the `null` literal).
    Nil,
    /// Go `[]interface{}` (attribute arrays).
    Array(Vec<AttrValue>),
    /// Go `parser.Attributes` (nested `{...}`).
    Attributes(Vec<Attribute>),
    /// Any other Go value an extension stores.
    Other(Arc<dyn Any + Send + Sync>),
}

impl fmt::Debug for AttrValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AttrValue::Bytes(b) => write!(f, "Bytes({:?})", String::from_utf8_lossy(b)),
            AttrValue::String(b) => write!(f, "String({:?})", String::from_utf8_lossy(b)),
            AttrValue::Float(v) => write!(f, "Float({v})"),
            AttrValue::Bool(v) => write!(f, "Bool({v})"),
            AttrValue::Nil => write!(f, "Nil"),
            AttrValue::Array(v) => f.debug_tuple("Array").field(v).finish(),
            AttrValue::Attributes(v) => f.debug_tuple("Attributes").field(v).finish(),
            AttrValue::Other(_) => write!(f, "Other(..)"),
        }
    }
}

impl AttrValue {
    /// The bytes of a `[]byte` value.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            AttrValue::Bytes(b) => Some(b),
            _ => None,
        }
    }
}

/// An Attribute is an attribute of the Node.
#[derive(Clone, Debug)]
pub struct Attribute {
    pub name: Vec<u8>,
    pub value: AttrValue,
}

/// A node identifier in an [`Ast`] arena (Go: a `Node` pointer).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub u32);

/// Node-specific behaviour of an extension node (Go: the methods a custom
/// node type implements on top of `ast.BaseBlock` / `ast.BaseInline`).
pub trait CustomNode: Any + Send + Sync + fmt::Debug {
    /// IsRaw returns true if contents should be rendered as 'raw' contents.
    fn is_raw(&self) -> bool {
        false
    }
    /// Text (deprecated in Go): `None` uses the `BaseNode.Text` default
    /// (children's text).
    fn text(&self, _ast: &Ast, _n: NodeId, _source: &[u8]) -> Option<Vec<u8>> {
        None
    }
    /// SoftLineBreak, for nodes that implement it (used by `BaseNode.Text`).
    fn soft_line_break(&self) -> Option<bool> {
        None
    }
    /// The key/value pairs `Dump` prints.
    fn dump_fields(&self, _source: &[u8]) -> Vec<(String, String)> {
        Vec::new()
    }
    /// Downcasting support.
    fn as_any(&self) -> &dyn Any;
    /// Downcasting support.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// The node-specific fields of every goldmark node type.
#[derive(Debug)]
pub enum NodeValue {
    Document(Document),
    TextBlock,
    Paragraph,
    Heading(Heading),
    ThematicBreak,
    CodeBlock,
    FencedCodeBlock(FencedCodeBlock),
    Blockquote,
    List(List),
    ListItem(ListItem),
    HTMLBlock(HTMLBlock),
    Text(Text),
    String(StringNode),
    CodeSpan,
    Emphasis(Emphasis),
    Link(Link),
    Image(Link),
    AutoLink(AutoLink),
    RawHTML(RawHTML),
    /// parser.Delimiter
    Delimiter(crate::parser::Delimiter),
    /// parser.linkLabelState
    LinkLabelState(crate::parser::LinkLabelState),
    /// A node type defined outside goldmark's core.
    Custom(Box<dyn CustomNode>),
}

impl NodeValue {
    fn builtin_kind_type(&self) -> (NodeKind, NodeType) {
        match self {
            NodeValue::Document(_) => (KIND_DOCUMENT, NodeType::Document),
            NodeValue::TextBlock => (KIND_TEXT_BLOCK, NodeType::Block),
            NodeValue::Paragraph => (KIND_PARAGRAPH, NodeType::Block),
            NodeValue::Heading(_) => (KIND_HEADING, NodeType::Block),
            NodeValue::ThematicBreak => (KIND_THEMATIC_BREAK, NodeType::Block),
            NodeValue::CodeBlock => (KIND_CODE_BLOCK, NodeType::Block),
            NodeValue::FencedCodeBlock(_) => (KIND_FENCED_CODE_BLOCK, NodeType::Block),
            NodeValue::Blockquote => (KIND_BLOCKQUOTE, NodeType::Block),
            NodeValue::List(_) => (KIND_LIST, NodeType::Block),
            NodeValue::ListItem(_) => (KIND_LIST_ITEM, NodeType::Block),
            NodeValue::HTMLBlock(_) => (KIND_HTML_BLOCK, NodeType::Block),
            NodeValue::Text(_) => (KIND_TEXT, NodeType::Inline),
            NodeValue::String(_) => (KIND_STRING, NodeType::Inline),
            NodeValue::CodeSpan => (KIND_CODE_SPAN, NodeType::Inline),
            NodeValue::Emphasis(_) => (KIND_EMPHASIS, NodeType::Inline),
            NodeValue::Link(_) => (KIND_LINK, NodeType::Inline),
            NodeValue::Image(_) => (KIND_IMAGE, NodeType::Inline),
            NodeValue::AutoLink(_) => (KIND_AUTO_LINK, NodeType::Inline),
            NodeValue::RawHTML(_) => (KIND_RAW_HTML, NodeType::Inline),
            NodeValue::Delimiter(_) => (KIND_DELIMITER, NodeType::Inline),
            NodeValue::LinkLabelState(_) => (KIND_LINK_LABEL_STATE, NodeType::Inline),
            NodeValue::Custom(_) => panic!("custom nodes need an explicit kind"),
        }
    }
}

/// An arena slot: Go's `BaseNode` / `BaseBlock` fields plus the node value.
#[derive(Debug)]
pub struct NodeData {
    kind: NodeKind,
    typ: NodeType,
    first_child: Option<NodeId>,
    last_child: Option<NodeId>,
    parent: Option<NodeId>,
    next: Option<NodeId>,
    prev: Option<NodeId>,
    child_count: i64,
    attributes: Option<Vec<Attribute>>,
    lines: Segments,
    blank_previous_lines: bool,
    /// The node-specific fields.
    pub value: NodeValue,
}

/// The node arena. A parsed document is an `Ast` plus its root [`NodeId`].
#[derive(Debug, Default)]
pub struct Ast {
    nodes: Vec<NodeData>,
}

/// Go: `WalkStatus` represents a current status of the Walk function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkStatus {
    /// WalkStop indicates no more walking needed.
    Stop = 1,
    /// WalkSkipChildren indicates that Walk wont walk on children of current
    /// node.
    SkipChildren = 2,
    /// WalkContinue indicates that Walk can continue to walk.
    Continue = 3,
}

impl Ast {
    /// A new, empty arena.
    pub fn new() -> Ast {
        Ast { nodes: Vec::new() }
    }

    /// Number of nodes ever allocated.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// True if no node was allocated.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Allocates a detached node of a built-in type.
    pub fn new_node(&mut self, value: NodeValue) -> NodeId {
        let (kind, typ) = value.builtin_kind_type();
        self.alloc(kind, typ, value)
    }

    /// Allocates a detached extension node of the given kind and type.
    pub fn new_custom_node(
        &mut self,
        kind: NodeKind,
        typ: NodeType,
        value: Box<dyn CustomNode>,
    ) -> NodeId {
        self.alloc(kind, typ, NodeValue::Custom(value))
    }

    fn alloc(&mut self, kind: NodeKind, typ: NodeType, value: NodeValue) -> NodeId {
        let id = NodeId(self.nodes.len() as u32);
        self.nodes.push(NodeData {
            kind,
            typ,
            first_child: None,
            last_child: None,
            parent: None,
            next: None,
            prev: None,
            child_count: 0,
            attributes: None,
            lines: Segments::default(),
            blank_previous_lines: false,
            value,
        });
        id
    }

    fn d(&self, n: NodeId) -> &NodeData {
        &self.nodes[n.0 as usize]
    }

    fn dm(&mut self, n: NodeId) -> &mut NodeData {
        &mut self.nodes[n.0 as usize]
    }

    /// The node's value.
    pub fn value(&self, n: NodeId) -> &NodeValue {
        &self.d(n).value
    }

    /// The node's value, mutably.
    pub fn value_mut(&mut self, n: NodeId) -> &mut NodeValue {
        &mut self.dm(n).value
    }

    /// The extension payload of a custom node, downcast to `T`.
    pub fn custom<T: 'static>(&self, n: NodeId) -> Option<&T> {
        match &self.d(n).value {
            NodeValue::Custom(c) => c.as_any().downcast_ref::<T>(),
            _ => None,
        }
    }

    /// The extension payload of a custom node, downcast to `T`, mutably.
    pub fn custom_mut<T: 'static>(&mut self, n: NodeId) -> Option<&mut T> {
        match &mut self.dm(n).value {
            NodeValue::Custom(c) => c.as_any_mut().downcast_mut::<T>(),
            _ => None,
        }
    }

    // Go: Node.Type
    /// Type returns a type of this node.
    pub fn typ(&self, n: NodeId) -> NodeType {
        self.d(n).typ
    }

    // Go: Node.Kind
    /// Kind returns a kind of this node.
    pub fn kind(&self, n: NodeId) -> NodeKind {
        self.d(n).kind
    }

    // Go: ast/ast.go:BaseNode.NextSibling
    /// NextSibling returns a next sibling node of this node.
    pub fn next_sibling(&self, n: NodeId) -> Option<NodeId> {
        self.d(n).next
    }

    // Go: ast/ast.go:BaseNode.PreviousSibling
    /// PreviousSibling returns a previous sibling node of this node.
    pub fn previous_sibling(&self, n: NodeId) -> Option<NodeId> {
        self.d(n).prev
    }

    // Go: ast/ast.go:BaseNode.Parent
    /// Parent returns a parent node of this node.
    pub fn parent(&self, n: NodeId) -> Option<NodeId> {
        self.d(n).parent
    }

    // Go: ast/ast.go:BaseNode.SetParent
    /// SetParent sets a parent node to this node.
    pub fn set_parent(&mut self, n: NodeId, v: Option<NodeId>) {
        self.dm(n).parent = v;
    }

    // Go: ast/ast.go:BaseNode.SetPreviousSibling
    /// SetPreviousSibling sets a previous sibling node to this node.
    pub fn set_previous_sibling(&mut self, n: NodeId, v: Option<NodeId>) {
        self.dm(n).prev = v;
    }

    // Go: ast/ast.go:BaseNode.SetNextSibling
    /// SetNextSibling sets a next sibling node to this node.
    pub fn set_next_sibling(&mut self, n: NodeId, v: Option<NodeId>) {
        self.dm(n).next = v;
    }

    // Go: ast/ast.go:BaseNode.HasChildren
    /// HasChildren returns true if this node has any children, otherwise false.
    pub fn has_children(&self, n: NodeId) -> bool {
        self.d(n).first_child.is_some()
    }

    // Go: ast/ast.go:BaseNode.ChildCount
    /// ChildCount returns a total number of children (Go's counter, which
    /// `InsertBefore` over-counts in some cases).
    pub fn child_count(&self, n: NodeId) -> i64 {
        self.d(n).child_count
    }

    // Go: ast/ast.go:BaseNode.FirstChild
    /// FirstChild returns a first child of this node.
    pub fn first_child(&self, n: NodeId) -> Option<NodeId> {
        self.d(n).first_child
    }

    // Go: ast/ast.go:BaseNode.LastChild
    /// LastChild returns a last child of this node.
    pub fn last_child(&self, n: NodeId) -> Option<NodeId> {
        self.d(n).last_child
    }

    /// The children of `n` (a snapshot).
    pub fn children(&self, n: NodeId) -> Vec<NodeId> {
        let mut ret = Vec::new();
        let mut c = self.first_child(n);
        while let Some(id) = c {
            ret.push(id);
            c = self.next_sibling(id);
        }
        ret
    }

    // Go: ast/ast.go:ensureIsolated
    fn ensure_isolated(&mut self, v: NodeId) {
        if let Some(p) = self.parent(v) {
            self.remove_child(p, v);
        }
    }

    // Go: ast/ast.go:BaseNode.RemoveChild
    /// RemoveChild removes a node child from this node.
    /// If a node child is not children of this node, RemoveChild nothing to do.
    pub fn remove_child(&mut self, this: NodeId, v: NodeId) {
        if self.parent(v) != Some(this) {
            return;
        }
        self.dm(this).child_count -= 1;
        let prev = self.previous_sibling(v);
        let next = self.next_sibling(v);
        if let Some(prev) = prev {
            self.set_next_sibling(prev, next);
        } else {
            self.dm(this).first_child = next;
        }
        if let Some(next) = next {
            self.set_previous_sibling(next, prev);
        } else {
            self.dm(this).last_child = prev;
        }
        self.set_parent(v, None);
        self.set_previous_sibling(v, None);
        self.set_next_sibling(v, None);
    }

    // Go: ast/ast.go:BaseNode.RemoveChildren
    /// RemoveChildren removes all children from this node.
    pub fn remove_children(&mut self, this: NodeId) {
        let mut c = self.first_child(this);
        while let Some(cid) = c {
            self.set_parent(cid, None);
            self.set_previous_sibling(cid, None);
            let next = self.next_sibling(cid);
            self.set_next_sibling(cid, None);
            c = next;
        }
        let d = self.dm(this);
        d.first_child = None;
        d.last_child = None;
        d.child_count = 0;
    }

    // Go: ast/ast.go:BaseNode.SortChildren
    /// SortChildren sorts childrens by comparator.
    pub fn sort_children(
        &mut self,
        this: NodeId,
        mut comparator: impl FnMut(&Ast, NodeId, NodeId) -> i64,
    ) {
        let mut sorted: Option<NodeId> = None;
        let mut current = self.first_child(this);
        while let Some(cur) = current {
            let next = self.next_sibling(cur);
            if sorted.is_none() || comparator(self, sorted.unwrap(), cur) >= 0 {
                self.set_next_sibling(cur, sorted);
                if let Some(s) = sorted {
                    self.set_previous_sibling(s, Some(cur));
                }
                sorted = Some(cur);
                self.set_previous_sibling(cur, None);
            } else {
                let mut c = sorted.unwrap();
                while let Some(cn) = self.next_sibling(c) {
                    if comparator(self, cn, cur) < 0 {
                        c = cn;
                    } else {
                        break;
                    }
                }
                let cnext = self.next_sibling(c);
                self.set_next_sibling(cur, cnext);
                self.set_previous_sibling(cur, Some(c));
                if let Some(cn) = cnext {
                    self.set_previous_sibling(cn, Some(cur));
                }
                self.set_next_sibling(c, Some(cur));
            }
            current = next;
        }
        self.dm(this).first_child = sorted;
        let mut c = self.first_child(this);
        while let Some(cid) = c {
            self.dm(this).last_child = Some(cid);
            c = self.next_sibling(cid);
        }
    }

    // Go: ast/ast.go:BaseNode.AppendChild
    /// AppendChild append a node child to the tail of the children.
    pub fn append_child(&mut self, this: NodeId, v: NodeId) {
        self.ensure_isolated(v);
        if self.first_child(this).is_none() {
            self.dm(this).first_child = Some(v);
            self.set_next_sibling(v, None);
            self.set_previous_sibling(v, None);
        } else {
            let last = self.last_child(this).expect("last child");
            self.set_next_sibling(last, Some(v));
            self.set_previous_sibling(v, Some(last));
        }
        self.set_parent(v, Some(this));
        let d = self.dm(this);
        d.last_child = Some(v);
        d.child_count += 1;
    }

    // Go: ast/ast.go:BaseNode.ReplaceChild
    /// ReplaceChild replace a node v1 with a node insertee.
    /// If v1 is not children of this node, ReplaceChild append a insetee to the
    /// tail of the children.
    pub fn replace_child(&mut self, this: NodeId, v1: NodeId, insertee: NodeId) {
        self.insert_before(this, Some(v1), insertee);
        self.remove_child(this, v1);
    }

    // Go: ast/ast.go:BaseNode.InsertAfter
    /// InsertAfterinserts a node insertee after a node v1.
    pub fn insert_after(&mut self, this: NodeId, v1: NodeId, insertee: NodeId) {
        let next = self.next_sibling(v1);
        self.insert_before(this, next, insertee);
    }

    // Go: ast/ast.go:BaseNode.InsertBefore
    /// InsertBefore inserts a node insertee before a node v1.
    /// If v1 is not children of this node, InsertBefore append a insetee to the
    /// tail of the children.
    ///
    /// Like Go, the child count is incremented before appending when `v1` is
    /// nil (so it is counted twice) and also when `v1` is not a child.
    pub fn insert_before(&mut self, this: NodeId, v1: Option<NodeId>, insertee: NodeId) {
        self.dm(this).child_count += 1;
        let Some(v1) = v1 else {
            self.append_child(this, insertee);
            return;
        };
        self.ensure_isolated(insertee);
        if self.parent(v1) == Some(this) {
            let c = v1;
            let prev = self.previous_sibling(c);
            if let Some(prev) = prev {
                self.set_next_sibling(prev, Some(insertee));
                self.set_previous_sibling(insertee, Some(prev));
            } else {
                self.dm(this).first_child = Some(insertee);
                self.set_previous_sibling(insertee, None);
            }
            self.set_next_sibling(insertee, Some(c));
            self.set_previous_sibling(c, Some(insertee));
            self.set_parent(insertee, Some(this));
        }
    }

    // Go: ast/ast.go:BaseNode.OwnerDocument (and Document.OwnerDocument)
    /// OwnerDocument returns this node's owner document.
    /// If this node is not a child of the Document node, OwnerDocument
    /// returns nil.
    pub fn owner_document(&self, n: NodeId) -> Option<NodeId> {
        if self.kind(n) == KIND_DOCUMENT {
            return Some(n);
        }
        let mut d = self.parent(n)?;
        loop {
            match self.parent(d) {
                None => {
                    if matches!(self.value(d), NodeValue::Document(_)) {
                        return Some(d);
                    }
                    break;
                }
                Some(p) => d = p,
            }
        }
        None
    }

    // Go: Node.Text (all implementations)
    /// Text returns text values of this node.
    ///
    /// Deprecated in Go: Use other properties of the node to get the text value.
    pub fn text(&self, n: NodeId, source: &[u8]) -> Vec<u8> {
        match self.value(n) {
            NodeValue::TextBlock
            | NodeValue::Paragraph
            | NodeValue::CodeBlock
            | NodeValue::FencedCodeBlock(_) => self.lines(n).value(source),
            NodeValue::HTMLBlock(h) => {
                let mut ret = self.lines(n).value(source);
                if h.has_closure() {
                    ret.extend_from_slice(&h.closure_line.value(source));
                }
                ret
            }
            NodeValue::Text(t) => t.segment.value(source).into_owned(),
            NodeValue::String(s) => s.value.clone(),
            NodeValue::AutoLink(a) => self.auto_link_label(a, source),
            NodeValue::RawHTML(r) => r.segments.value(source),
            NodeValue::Delimiter(d) => d.segment.value(source).into_owned(),
            NodeValue::LinkLabelState(s) => s.segment.value(source).into_owned(),
            NodeValue::Custom(c) => match c.text(self, n, source) {
                Some(t) => t,
                None => self.base_text(n, source),
            },
            _ => self.base_text(n, source),
        }
    }

    // Go: ast/ast.go:BaseNode.Text
    fn base_text(&self, n: NodeId, source: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();
        let mut c = self.first_child(n);
        while let Some(cid) = c {
            buf.extend_from_slice(&self.text(cid, source));
            let slb = match self.value(cid) {
                NodeValue::Text(t) => t.soft_line_break(),
                NodeValue::Custom(c) => c.soft_line_break().unwrap_or(false),
                _ => false,
            };
            if slb {
                buf.push(b'\n');
            }
            c = self.next_sibling(cid);
        }
        buf
    }

    // Go: BaseBlock.HasBlankPreviousLines / BaseInline panics
    /// HasBlankPreviousLines returns true if the row before this node is blank,
    /// otherwise false.
    /// This method is valid only for block nodes.
    pub fn has_blank_previous_lines(&self, n: NodeId) -> bool {
        if self.typ(n) == NodeType::Inline {
            panic!("can not call with inline nodes.");
        }
        self.d(n).blank_previous_lines
    }

    // Go: BaseBlock.SetBlankPreviousLines
    /// SetBlankPreviousLines sets whether the row before this node is blank.
    /// This method is valid only for block nodes.
    pub fn set_blank_previous_lines(&mut self, n: NodeId, v: bool) {
        if self.typ(n) == NodeType::Inline {
            panic!("can not call with inline nodes.");
        }
        self.dm(n).blank_previous_lines = v;
    }

    // Go: BaseBlock.Lines
    /// Lines returns text segments that hold positions in a source.
    /// This method is valid only for block nodes.
    pub fn lines(&self, n: NodeId) -> &Segments {
        if self.typ(n) == NodeType::Inline {
            panic!("can not call with inline nodes.");
        }
        &self.d(n).lines
    }

    // Go: BaseBlock.Lines (the returned pointer used for mutation)
    /// Lines, mutably.
    pub fn lines_mut(&mut self, n: NodeId) -> &mut Segments {
        if self.typ(n) == NodeType::Inline {
            panic!("can not call with inline nodes.");
        }
        &mut self.dm(n).lines
    }

    // Go: BaseBlock.SetLines
    /// SetLines sets text segments that hold positions in a source.
    /// This method is valid only for block nodes.
    pub fn set_lines(&mut self, n: NodeId, v: &Segments) {
        if self.typ(n) == NodeType::Inline {
            panic!("can not call with inline nodes.");
        }
        self.dm(n).lines = v.clone();
    }

    // Go: Node.IsRaw (all implementations)
    /// IsRaw returns true if contents should be rendered as 'raw' contents.
    pub fn is_raw(&self, n: NodeId) -> bool {
        match self.value(n) {
            NodeValue::CodeBlock | NodeValue::FencedCodeBlock(_) | NodeValue::HTMLBlock(_) => true,
            NodeValue::Text(t) => t.is_raw(),
            NodeValue::String(s) => s.is_raw(),
            NodeValue::Custom(c) => c.is_raw(),
            _ => false,
        }
    }

    // Go: ast/ast.go:BaseNode.SetAttribute
    /// SetAttribute sets the given value to the attributes.
    pub fn set_attribute(&mut self, n: NodeId, name: &[u8], value: AttrValue) {
        let d = self.dm(n);
        match &mut d.attributes {
            None => {
                let mut v = Vec::with_capacity(10);
                v.push(Attribute {
                    name: name.to_vec(),
                    value,
                });
                d.attributes = Some(v);
            }
            Some(attrs) => {
                for a in attrs.iter_mut() {
                    if a.name == name {
                        a.name = name.to_vec();
                        a.value = value;
                        return;
                    }
                }
                attrs.push(Attribute {
                    name: name.to_vec(),
                    value,
                });
            }
        }
    }

    // Go: ast/ast.go:BaseNode.SetAttributeString
    /// SetAttributeString sets the given value to the attributes.
    pub fn set_attribute_string(&mut self, n: NodeId, name: &str, value: AttrValue) {
        self.set_attribute(n, name.as_bytes(), value);
    }

    // Go: ast/ast.go:BaseNode.Attribute
    /// Attribute returns a (attribute value, true) if an attribute
    /// associated with the given name is found, otherwise
    /// (nil, false)
    pub fn attribute(&self, n: NodeId, name: &[u8]) -> Option<&AttrValue> {
        let attrs = self.d(n).attributes.as_ref()?;
        attrs.iter().find(|a| a.name == name).map(|a| &a.value)
    }

    // Go: ast/ast.go:BaseNode.AttributeString
    /// AttributeString returns a (attribute value, true) if an attribute
    /// associated with the given name is found, otherwise
    /// (nil, false)
    pub fn attribute_string(&self, n: NodeId, s: &str) -> Option<&AttrValue> {
        self.attribute(n, s.as_bytes())
    }

    // Go: ast/ast.go:BaseNode.Attributes
    /// Attributes returns a list of attributes.
    /// This may be a nil if there are no attributes.
    pub fn attributes(&self, n: NodeId) -> Option<&[Attribute]> {
        self.d(n).attributes.as_deref()
    }

    /// Attributes, mutably (Go code mutates the returned slice in place).
    pub fn attributes_mut(&mut self, n: NodeId) -> Option<&mut Vec<Attribute>> {
        self.dm(n).attributes.as_mut()
    }

    // Go: ast/ast.go:BaseNode.RemoveAttributes
    /// RemoveAttributes removes all attributes from this node.
    pub fn remove_attributes(&mut self, n: NodeId) {
        self.dm(n).attributes = None;
    }

    // Go: Node.Dump / ast/ast.go:DumpHelper
    /// Dump dumps an AST tree structure (Go prints to stdout; this returns
    /// the text). Map entries are printed in sorted key order.
    pub fn dump(&self, n: NodeId, source: &[u8], level: usize) -> String {
        let mut out = String::new();
        self.dump_to(&mut out, n, source, level);
        out
    }

    fn dump_to(&self, out: &mut String, n: NodeId, source: &[u8], level: usize) {
        let indent = "    ".repeat(level);
        let lossy = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
        let flags = |f: u8| {
            let fs = text_flags_string(f);
            if fs.is_empty() { fs } else { format!("({fs})") }
        };
        let mut kv: BTreeMap<String, String> = BTreeMap::new();
        match self.value(n) {
            NodeValue::Text(t) => {
                out.push_str(&format!(
                    "{indent}Text{}: \"{}\"\n",
                    flags(t.flags),
                    lossy(&t.segment.value(source)).trim_end_matches('\n')
                ));
                return;
            }
            NodeValue::String(s) => {
                out.push_str(&format!(
                    "{indent}String{}: \"{}\"\n",
                    flags(s.flags),
                    lossy(&s.value).trim_end_matches('\n')
                ));
                return;
            }
            NodeValue::Delimiter(d) => {
                out.push_str(&format!(
                    "{indent}Delimiter: \"{}\"\n",
                    lossy(&d.segment.value(source))
                ));
                return;
            }
            NodeValue::LinkLabelState(s) => {
                out.push_str(&format!(
                    "{indent}linkLabelState: \"{}\"\n",
                    lossy(&s.segment.value(source))
                ));
                return;
            }
            NodeValue::Heading(h) => {
                kv.insert("Level".into(), h.level.to_string());
            }
            NodeValue::FencedCodeBlock(f) => {
                if let Some(info) = f.info {
                    kv.insert(
                        "Info".into(),
                        format!("\"{}\"", lossy(&self.text(info, source))),
                    );
                }
            }
            NodeValue::List(l) => {
                kv.insert("Ordered".into(), l.is_ordered().to_string());
                kv.insert("Marker".into(), (l.marker as char).to_string());
                kv.insert("Tight".into(), l.is_tight.to_string());
                if l.is_ordered() {
                    kv.insert("Start".into(), l.start.to_string());
                }
            }
            NodeValue::ListItem(li) => {
                kv.insert("Offset".into(), li.offset.to_string());
            }
            NodeValue::Emphasis(e) => {
                kv.insert("Level".into(), e.level.to_string());
            }
            NodeValue::Link(l) | NodeValue::Image(l) => {
                kv.insert("Destination".into(), lossy(&l.destination));
                kv.insert("Title".into(), lossy(l.title.as_deref().unwrap_or(b"")));
            }
            NodeValue::AutoLink(a) => {
                kv.insert("Value".into(), lossy(&self.auto_link_label(a, source)));
            }
            NodeValue::RawHTML(r) => {
                kv.insert("RawText".into(), lossy(&r.segments.value(source)));
            }
            NodeValue::HTMLBlock(h) => {
                out.push_str(&format!("{indent}HTMLBlock {{\n"));
                let indent2 = "    ".repeat(level + 1);
                out.push_str(&format!("{indent2}RawText: \""));
                for i in 0..self.lines(n).len() {
                    let s = self.lines(n).at(i);
                    out.push_str(&lossy(&source[s.start as usize..s.stop as usize]));
                }
                out.push_str("\"\n");
                for c in self.children(n) {
                    self.dump_to(out, c, source, level + 1);
                }
                if h.has_closure() {
                    out.push_str(&format!(
                        "{indent2}Closure: \"{}\"\n",
                        lossy(&h.closure_line.value(source))
                    ));
                }
                out.push_str(&format!(
                    "{indent2}HasBlankPreviousLines: {}\n",
                    self.has_blank_previous_lines(n)
                ));
                out.push_str(&format!("{indent}}}\n"));
                return;
            }
            NodeValue::Custom(c) => {
                for (k, v) in c.dump_fields(source) {
                    kv.insert(k, v);
                }
            }
            _ => {}
        }
        // Go: ast/ast.go:DumpHelper
        out.push_str(&format!("{indent}{} {{\n", self.kind(n)));
        let indent2 = "    ".repeat(level + 1);
        if self.typ(n) == NodeType::Block {
            out.push_str(&format!("{indent2}RawText: \""));
            for i in 0..self.lines(n).len() {
                let line = self.lines(n).at(i);
                out.push_str(&lossy(&line.value(source)));
            }
            out.push_str("\"\n");
            out.push_str(&format!(
                "{indent2}HasBlankPreviousLines: {}\n",
                self.has_blank_previous_lines(n)
            ));
        }
        for (k, v) in kv {
            out.push_str(&format!("{indent2}{k}: {v}\n"));
        }
        for c in self.children(n) {
            self.dump_to(out, c, source, level + 1);
        }
        out.push_str(&format!("{indent}}}\n"));
    }
}

impl std::ops::Index<NodeId> for Ast {
    type Output = NodeData;
    fn index(&self, n: NodeId) -> &NodeData {
        self.d(n)
    }
}

/// A walker error (Go `error`).
pub type WalkError = crate::Error;

// Go: ast/ast.go:Walk
/// Walk walks a AST tree by the depth first search algorithm.
///
/// The walker may mutate the tree; as in Go, a node's first child is read
/// after the entering callback and each next sibling after the child's walk.
/// (Go recurses on a growable goroutine stack; this walks with an explicit
/// stack so deeply nested documents cannot overflow a thread stack.)
pub fn walk(
    ast: &mut Ast,
    n: NodeId,
    walker: &mut dyn FnMut(&mut Ast, NodeId, bool) -> Result<WalkStatus, WalkError>,
) -> Result<(), WalkError> {
    walk_generic(ast, n, &mut |ast: &mut Ast| ast, walker)
}

/// [`walk`] over an immutable tree (used by the renderer).
pub fn walk_ref(
    ast: &Ast,
    n: NodeId,
    walker: &mut dyn FnMut(&Ast, NodeId, bool) -> Result<WalkStatus, WalkError>,
) -> Result<(), WalkError> {
    let mut a = ast;
    walk_generic(
        &mut a,
        n,
        &mut |a: &mut &Ast| *a,
        &mut |a: &mut &Ast, n, entering| walker(a, n, entering),
    )
}

/// A walk frame: the node and the child currently being walked (`None`
/// before the first child is read).
struct WalkFrame {
    node: NodeId,
    child: Option<NodeId>,
}

// Go: ast/ast.go:walkHelper, as an explicit-stack loop with the same call
// sequence and the same reads of FirstChild/NextSibling.
fn walk_generic<T, A: AstView + ?Sized>(
    t: &mut T,
    n: NodeId,
    view: &mut dyn FnMut(&mut T) -> &A,
    walker: &mut dyn FnMut(&mut T, NodeId, bool) -> Result<WalkStatus, WalkError>,
) -> Result<(), WalkError> {
    let mut stack: Vec<WalkFrame> = Vec::new();
    // entering n
    let status = walker(t, n, true)?;
    if status == WalkStatus::Stop {
        return Ok(());
    }
    if status == WalkStatus::SkipChildren {
        walker(t, n, false)?;
        return Ok(());
    }
    stack.push(WalkFrame {
        node: n,
        child: None,
    });
    while let Some(top) = stack.last_mut() {
        // the next child to walk: first child, or the sibling of the one
        // just finished
        let next = match top.child {
            None => view(t).first_child_of(top.node),
            Some(c) => view(t).next_sibling_of(c),
        };
        match next {
            Some(c) => {
                top.child = Some(c);
                let status = walker(t, c, true)?;
                if status == WalkStatus::Stop {
                    return Ok(());
                }
                if status == WalkStatus::SkipChildren {
                    if walker(t, c, false)? == WalkStatus::Stop {
                        return Ok(());
                    }
                } else {
                    stack.push(WalkFrame {
                        node: c,
                        child: None,
                    });
                }
            }
            None => {
                let node = top.node;
                stack.pop();
                if walker(t, node, false)? == WalkStatus::Stop {
                    return Ok(());
                }
            }
        }
    }
    Ok(())
}

/// Read access used by [`walk_generic`].
trait AstView {
    fn first_child_of(&self, n: NodeId) -> Option<NodeId>;
    fn next_sibling_of(&self, n: NodeId) -> Option<NodeId>;
}

impl AstView for Ast {
    fn first_child_of(&self, n: NodeId) -> Option<NodeId> {
        self.first_child(n)
    }
    fn next_sibling_of(&self, n: NodeId) -> Option<NodeId> {
        self.next_sibling(n)
    }
}
