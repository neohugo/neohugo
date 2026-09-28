//! Port of `golang.org/x/net@v0.41.0/html/node.go`.
//!
//! Go's `*Node` pointers become indices into the [`Document`] arena ([`NodeId`]). Node identity
//! (Go pointer equality) is index equality. The single Go `scopeMarker` node is arena slot
//! [`SCOPE_MARKER`].

use super::atom::Atom;
use super::token::Attribute;

/// Go: `html.NodeType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeType {
    Error,
    Text,
    Document,
    Element,
    Comment,
    Doctype,
    /// RawNode nodes are not returned by the parser, but can be part of the Node tree passed to
    /// func Render to insert raw HTML (without escaping).
    Raw,
    /// scopeMarker nodes are pushed onto the list of active formatting elements.
    ScopeMarker,
}

/// An index into [`Document::nodes`] (Go: `*Node`).
pub type NodeId = usize;

/// The arena slot of Go's `scopeMarker`.
pub const SCOPE_MARKER: NodeId = 0;

/// Go: `html.Node`.
#[derive(Clone, Debug)]
pub struct Node {
    pub parent: Option<NodeId>,
    pub first_child: Option<NodeId>,
    pub last_child: Option<NodeId>,
    pub prev_sibling: Option<NodeId>,
    pub next_sibling: Option<NodeId>,

    pub typ: NodeType,
    pub data_atom: Atom,
    pub data: Vec<u8>,
    pub namespace: Vec<u8>,
    pub attr: Vec<Attribute>,
}

impl Node {
    /// A detached node.
    pub fn new(typ: NodeType, data_atom: Atom, data: Vec<u8>, attr: Vec<Attribute>) -> Node {
        Node {
            parent: None,
            first_child: None,
            last_child: None,
            prev_sibling: None,
            next_sibling: None,
            typ,
            data_atom,
            data,
            namespace: Vec::new(),
            attr,
        }
    }
}

/// The node arena of one parse.
#[derive(Clone, Debug)]
pub struct Document {
    pub nodes: Vec<Node>,
}

impl Default for Document {
    fn default() -> Self {
        Document::new()
    }
}

impl Document {
    /// A new arena holding the scope marker.
    pub fn new() -> Document {
        Document {
            nodes: vec![Node::new(NodeType::ScopeMarker, 0, Vec::new(), Vec::new())],
        }
    }

    /// Adds a detached node.
    pub fn add(&mut self, n: Node) -> NodeId {
        self.nodes.push(n);
        self.nodes.len() - 1
    }

    /// The children of n, in order.
    pub fn children(&self, n: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut c = self.nodes[n].first_child;
        while let Some(id) = c {
            out.push(id);
            c = self.nodes[id].next_sibling;
        }
        out
    }

    /// Go: `(n *Node) InsertBefore(newChild, oldChild)` — inserts newChild as a child of n,
    /// immediately before oldChild in the sequence of n's children (at the end when oldChild
    /// is None). Errors (Go panics) if newChild already has a parent or siblings.
    // Go: html/node.go:InsertBefore
    pub fn insert_before(
        &mut self,
        n: NodeId,
        new_child: NodeId,
        old_child: Option<NodeId>,
    ) -> Result<(), String> {
        let nc = &self.nodes[new_child];
        if nc.parent.is_some() || nc.prev_sibling.is_some() || nc.next_sibling.is_some() {
            return Err("html: InsertBefore called for an attached child Node".into());
        }
        let (prev, next) = match old_child {
            Some(old) => (self.nodes[old].prev_sibling, Some(old)),
            None => (self.nodes[n].last_child, None),
        };
        match prev {
            Some(p) => self.nodes[p].next_sibling = Some(new_child),
            None => self.nodes[n].first_child = Some(new_child),
        }
        match next {
            Some(x) => self.nodes[x].prev_sibling = Some(new_child),
            None => self.nodes[n].last_child = Some(new_child),
        }
        let nc = &mut self.nodes[new_child];
        nc.parent = Some(n);
        nc.prev_sibling = prev;
        nc.next_sibling = next;
        Ok(())
    }

    /// Go: `(n *Node) AppendChild(c)` — adds a node c as a child of n. Errors (Go panics) if c
    /// already has a parent or siblings.
    // Go: html/node.go:AppendChild
    pub fn append_child(&mut self, n: NodeId, c: NodeId) -> Result<(), String> {
        let cn = &self.nodes[c];
        if cn.parent.is_some() || cn.prev_sibling.is_some() || cn.next_sibling.is_some() {
            return Err("html: AppendChild called for an attached child Node".into());
        }
        let last = self.nodes[n].last_child;
        match last {
            Some(l) => self.nodes[l].next_sibling = Some(c),
            None => self.nodes[n].first_child = Some(c),
        }
        self.nodes[n].last_child = Some(c);
        let cn = &mut self.nodes[c];
        cn.parent = Some(n);
        cn.prev_sibling = last;
        Ok(())
    }

    /// Go: `(n *Node) RemoveChild(c)` — removes a node c that is a child of n. Errors (Go
    /// panics) if c is not a child of n.
    // Go: html/node.go:RemoveChild
    pub fn remove_child(&mut self, n: NodeId, c: NodeId) -> Result<(), String> {
        if self.nodes[c].parent != Some(n) {
            return Err("html: RemoveChild called for a non-child Node".into());
        }
        let (prev, next) = (self.nodes[c].prev_sibling, self.nodes[c].next_sibling);
        if self.nodes[n].first_child == Some(c) {
            self.nodes[n].first_child = next;
        }
        if let Some(x) = next {
            self.nodes[x].prev_sibling = prev;
        }
        if self.nodes[n].last_child == Some(c) {
            self.nodes[n].last_child = prev;
        }
        if let Some(p) = prev {
            self.nodes[p].next_sibling = next;
        }
        let cn = &mut self.nodes[c];
        cn.parent = None;
        cn.prev_sibling = None;
        cn.next_sibling = None;
        Ok(())
    }

    /// Go: `reparentChildren(dst, src)` — reparents all of src's child nodes to dst.
    // Go: html/node.go:reparentChildren
    pub fn reparent_children(&mut self, dst: NodeId, src: NodeId) -> Result<(), String> {
        while let Some(child) = self.nodes[src].first_child {
            self.remove_child(src, child)?;
            self.append_child(dst, child)?;
        }
        Ok(())
    }

    /// Go: `(n *Node) clone()` — a new node with the same type, data and attributes (no
    /// namespace, no links).
    // Go: html/node.go:clone
    pub fn clone_node(&mut self, n: NodeId) -> NodeId {
        let src = &self.nodes[n];
        let m = Node::new(src.typ, src.data_atom, src.data.clone(), src.attr.clone());
        self.add(m)
    }
}
