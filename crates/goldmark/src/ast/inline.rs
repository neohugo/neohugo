// Go: github.com/yuin/goldmark@v1.7.12/ast/inline.go

use super::{Ast, NodeId, NodeValue};
use crate::text::{Segment, Segments, new_segments};
use crate::util;

pub(crate) const TEXT_SOFT_LINE_BREAK: u8 = 1 << 0;
pub(crate) const TEXT_HARD_LINE_BREAK: u8 = 1 << 1;
pub(crate) const TEXT_RAW: u8 = 1 << 2;
pub(crate) const TEXT_CODE: u8 = 1 << 3;

// Go: ast/inline.go:textFlagsString
pub(crate) fn text_flags_string(flags: u8) -> String {
    let mut buf: Vec<&str> = Vec::new();
    if flags & TEXT_SOFT_LINE_BREAK != 0 {
        buf.push("SoftLineBreak");
    }
    if flags & TEXT_HARD_LINE_BREAK != 0 {
        buf.push("HardLineBreak");
    }
    if flags & TEXT_RAW != 0 {
        buf.push("Raw");
    }
    if flags & TEXT_CODE != 0 {
        buf.push("Code");
    }
    buf.join(", ")
}

/// A Text struct represents a textual content of the Markdown text.
#[derive(Debug, Clone, Copy, Default)]
pub struct Text {
    /// Segment is a position in a source text.
    pub segment: Segment,
    pub(crate) flags: u8,
}

impl Text {
    /// A Text value with the given segment and no flags (Go:
    /// `ast.NewTextSegment(v)` without allocating a node; for
    /// `ast.NewAutoLink(typ, value)`).
    pub fn new(segment: Segment) -> Text {
        Text { segment, flags: 0 }
    }

    // Go: ast/inline.go:Text.SoftLineBreak
    /// SoftLineBreak returns true if this node ends with a new line,
    /// otherwise false.
    pub fn soft_line_break(&self) -> bool {
        self.flags & TEXT_SOFT_LINE_BREAK != 0
    }

    // Go: ast/inline.go:Text.SetSoftLineBreak
    /// SetSoftLineBreak sets whether this node ends with a new line.
    pub fn set_soft_line_break(&mut self, v: bool) {
        if v {
            self.flags |= TEXT_SOFT_LINE_BREAK;
        } else {
            self.flags &= !TEXT_SOFT_LINE_BREAK;
        }
    }

    // Go: ast/inline.go:Text.IsRaw
    /// IsRaw returns true if this text should be rendered without unescaping
    /// back slash escapes and resolving references.
    pub fn is_raw(&self) -> bool {
        self.flags & TEXT_RAW != 0
    }

    // Go: ast/inline.go:Text.SetRaw
    /// SetRaw sets whether this text should be rendered as raw contents.
    pub fn set_raw(&mut self, v: bool) {
        if v {
            self.flags |= TEXT_RAW;
        } else {
            self.flags &= !TEXT_RAW;
        }
    }

    // Go: ast/inline.go:Text.HardLineBreak
    /// HardLineBreak returns true if this node ends with a hard line break.
    /// See https://spec.commonmark.org/0.30/#hard-line-breaks for details.
    pub fn hard_line_break(&self) -> bool {
        self.flags & TEXT_HARD_LINE_BREAK != 0
    }

    // Go: ast/inline.go:Text.SetHardLineBreak
    /// SetHardLineBreak sets whether this node ends with a hard line break.
    pub fn set_hard_line_break(&mut self, v: bool) {
        if v {
            self.flags |= TEXT_HARD_LINE_BREAK;
        } else {
            self.flags &= !TEXT_HARD_LINE_BREAK;
        }
    }

    // Go: ast/inline.go:Text.Value
    /// Value returns a value of this node.
    /// SoftLineBreaks are not included in the returned value.
    pub fn value<'a>(&self, source: &'a [u8]) -> std::borrow::Cow<'a, [u8]> {
        self.segment.value(source)
    }
}

/// A String struct is a textual content that has a concrete value.
#[derive(Debug, Clone, Default)]
pub struct StringNode {
    pub value: Vec<u8>,
    pub(crate) flags: u8,
}

impl StringNode {
    // Go: ast/inline.go:String.IsRaw
    /// IsRaw returns true if this text should be rendered without unescaping
    /// back slash escapes and resolving references.
    pub fn is_raw(&self) -> bool {
        self.flags & TEXT_RAW != 0
    }

    // Go: ast/inline.go:String.SetRaw
    /// SetRaw sets whether this text should be rendered as raw contents.
    pub fn set_raw(&mut self, v: bool) {
        if v {
            self.flags |= TEXT_RAW;
        } else {
            self.flags &= !TEXT_RAW;
        }
    }

    // Go: ast/inline.go:String.IsCode
    /// IsCode returns true if this text should be rendered without any
    /// modifications.
    pub fn is_code(&self) -> bool {
        self.flags & TEXT_CODE != 0
    }

    // Go: ast/inline.go:String.SetCode
    /// SetCode sets whether this text should be rendered without any modifications.
    pub fn set_code(&mut self, v: bool) {
        if v {
            self.flags |= TEXT_CODE;
        } else {
            self.flags &= !TEXT_CODE;
        }
    }
}

/// An Emphasis struct represents an emphasis of Markdown text.
#[derive(Debug, Clone, Copy)]
pub struct Emphasis {
    /// Level is a level of the emphasis.
    pub level: i64,
}

/// The fields of Link and Image (Go: `baseLink`).
#[derive(Debug, Clone, Default)]
pub struct Link {
    /// Destination is a destination(URL) of this link.
    pub destination: Vec<u8>,
    /// Title is a title of this link (`None` is Go's nil title).
    pub title: Option<Vec<u8>>,
}

/// AutoLinkType defines kind of auto links.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoLinkType {
    /// AutoLinkEmail indicates that an autolink is an email address.
    Email = 1,
    /// AutoLinkURL indicates that an autolink is a generic URL.
    Url = 2,
}

/// An AutoLink struct represents an autolink of the Markdown text.
#[derive(Debug, Clone)]
pub struct AutoLink {
    /// Type is a type of this autolink.
    pub auto_link_type: AutoLinkType,
    /// Protocol specified a protocol of the link.
    pub protocol: Option<Vec<u8>>,
    /// The value (Go: `value *Text`, a Text node not in the tree).
    pub value: Text,
}

/// A RawHTML struct represents an inline raw HTML of the Markdown text.
#[derive(Debug, Clone, Default)]
pub struct RawHTML {
    pub segments: Segments,
}

impl Ast {
    // Go: ast/inline.go:NewText
    /// NewText returns a new Text node.
    pub fn new_text(&mut self) -> NodeId {
        self.new_node(NodeValue::Text(Text::default()))
    }

    // Go: ast/inline.go:NewTextSegment
    /// NewTextSegment returns a new Text node with the given source position.
    pub fn new_text_segment(&mut self, v: Segment) -> NodeId {
        self.new_node(NodeValue::Text(Text {
            segment: v,
            flags: 0,
        }))
    }

    // Go: ast/inline.go:NewRawTextSegment
    /// NewRawTextSegment returns a new Text node with the given source position.
    /// The new node should be rendered as raw contents.
    pub fn new_raw_text_segment(&mut self, v: Segment) -> NodeId {
        let mut t = Text {
            segment: v,
            flags: 0,
        };
        t.set_raw(true);
        self.new_node(NodeValue::Text(t))
    }

    // Go: ast/inline.go:NewString
    /// NewString returns a new String node.
    pub fn new_string(&mut self, v: Vec<u8>) -> NodeId {
        self.new_node(NodeValue::String(StringNode { value: v, flags: 0 }))
    }

    // Go: ast/inline.go:NewCodeSpan
    /// NewCodeSpan returns a new CodeSpan node.
    pub fn new_code_span(&mut self) -> NodeId {
        self.new_node(NodeValue::CodeSpan)
    }

    // Go: ast/inline.go:NewEmphasis
    /// NewEmphasis returns a new Emphasis node with the given level.
    pub fn new_emphasis(&mut self, level: i64) -> NodeId {
        self.new_node(NodeValue::Emphasis(Emphasis { level }))
    }

    // Go: ast/inline.go:NewLink
    /// NewLink returns a new Link node.
    pub fn new_link(&mut self) -> NodeId {
        self.new_node(NodeValue::Link(Link::default()))
    }

    // Go: ast/inline.go:NewImage
    /// NewImage returns a new Image node (moving the link's children).
    pub fn new_image(&mut self, link: NodeId) -> NodeId {
        let l = self.link(link).expect("NewImage of a non-link").clone();
        let c = self.new_node(NodeValue::Image(Link {
            destination: l.destination,
            title: l.title,
        }));
        let mut n = self.first_child(link);
        while let Some(nid) = n {
            let next = self.next_sibling(nid);
            self.remove_child(link, nid);
            self.append_child(c, nid);
            n = next;
        }
        c
    }

    // Go: ast/inline.go:NewAutoLink
    /// NewAutoLink returns a new AutoLink node.
    pub fn new_auto_link(&mut self, typ: AutoLinkType, value: Text) -> NodeId {
        self.new_node(NodeValue::AutoLink(AutoLink {
            auto_link_type: typ,
            protocol: None,
            value,
        }))
    }

    // Go: ast/inline.go:AutoLink.URL
    /// URL returns an url of this node.
    pub fn auto_link_url(&self, n: &AutoLink, source: &[u8]) -> Vec<u8> {
        if let Some(protocol) = &n.protocol {
            let s = n.value.segment;
            let mut ret = Vec::with_capacity((protocol.len() as i64 + s.len() + 3) as usize);
            ret.extend_from_slice(protocol);
            ret.extend_from_slice(b"://");
            ret.extend_from_slice(&n.value.value(source));
            return ret;
        }
        n.value.value(source).into_owned()
    }

    // Go: ast/inline.go:AutoLink.Label
    /// Label returns a label of this node.
    pub fn auto_link_label(&self, n: &AutoLink, source: &[u8]) -> Vec<u8> {
        n.value.value(source).into_owned()
    }

    // Go: ast/inline.go:NewRawHTML
    /// NewRawHTML returns a new RawHTML node.
    pub fn new_raw_html(&mut self) -> NodeId {
        self.new_node(NodeValue::RawHTML(RawHTML {
            segments: new_segments(),
        }))
    }

    // Go: ast/inline.go:CodeSpan.IsBlank
    /// IsBlank returns true if this node consists of spaces, otherwise false.
    pub fn code_span_is_blank(&self, n: NodeId, source: &[u8]) -> bool {
        let mut c = self.first_child(n);
        while let Some(cid) = c {
            let text = self
                .text_node(cid)
                .expect("CodeSpan child is not a Text")
                .segment;
            if !util::is_blank(&text.value(source)) {
                return false;
            }
            c = self.next_sibling(cid);
        }
        true
    }

    // Go: ast/inline.go:Text.Merge
    /// Merge merges a Node n into this node.
    /// Merge returns true if the given node has been merged, otherwise false.
    pub fn text_merge(&mut self, n: NodeId, node: NodeId, source: &[u8]) -> bool {
        let Some(t) = self.text_node(node).copied() else {
            return false;
        };
        let nt = *self.text_node(n).expect("Merge on a non-Text");
        if nt.segment.stop != t.segment.start
            || t.segment.padding != 0
            || source[(nt.segment.stop - 1) as usize] == b'\n'
            || t.is_raw() != nt.is_raw()
        {
            return false;
        }
        let nt = self.text_node_mut(n).unwrap();
        nt.segment.stop = t.segment.stop;
        nt.set_soft_line_break(t.soft_line_break());
        nt.set_hard_line_break(t.hard_line_break());
        true
    }

    /// The Text fields of `n`, if it is a Text node.
    pub fn text_node(&self, n: NodeId) -> Option<&Text> {
        match self.value(n) {
            NodeValue::Text(t) => Some(t),
            _ => None,
        }
    }

    /// The Text fields of `n`, mutably.
    pub fn text_node_mut(&mut self, n: NodeId) -> Option<&mut Text> {
        match self.value_mut(n) {
            NodeValue::Text(t) => Some(t),
            _ => None,
        }
    }

    /// The String fields of `n`, if it is a String node.
    pub fn string_node(&self, n: NodeId) -> Option<&StringNode> {
        match self.value(n) {
            NodeValue::String(t) => Some(t),
            _ => None,
        }
    }

    /// The String fields of `n`, mutably.
    pub fn string_node_mut(&mut self, n: NodeId) -> Option<&mut StringNode> {
        match self.value_mut(n) {
            NodeValue::String(t) => Some(t),
            _ => None,
        }
    }

    /// The Link fields of `n`, if it is a Link.
    pub fn link(&self, n: NodeId) -> Option<&Link> {
        match self.value(n) {
            NodeValue::Link(l) => Some(l),
            _ => None,
        }
    }

    /// The Link fields of `n`, mutably.
    pub fn link_mut(&mut self, n: NodeId) -> Option<&mut Link> {
        match self.value_mut(n) {
            NodeValue::Link(l) => Some(l),
            _ => None,
        }
    }

    /// The Image fields of `n`, if it is an Image.
    pub fn image(&self, n: NodeId) -> Option<&Link> {
        match self.value(n) {
            NodeValue::Image(l) => Some(l),
            _ => None,
        }
    }

    /// The Emphasis fields of `n`, if it is an Emphasis.
    pub fn emphasis(&self, n: NodeId) -> Option<&Emphasis> {
        match self.value(n) {
            NodeValue::Emphasis(e) => Some(e),
            _ => None,
        }
    }

    /// The AutoLink fields of `n`, if it is an AutoLink.
    pub fn auto_link(&self, n: NodeId) -> Option<&AutoLink> {
        match self.value(n) {
            NodeValue::AutoLink(a) => Some(a),
            _ => None,
        }
    }

    /// The AutoLink fields of `n`, mutably.
    pub fn auto_link_mut(&mut self, n: NodeId) -> Option<&mut AutoLink> {
        match self.value_mut(n) {
            NodeValue::AutoLink(a) => Some(a),
            _ => None,
        }
    }

    /// The RawHTML fields of `n`, if it is a RawHTML node.
    pub fn raw_html(&self, n: NodeId) -> Option<&RawHTML> {
        match self.value(n) {
            NodeValue::RawHTML(r) => Some(r),
            _ => None,
        }
    }

    /// The RawHTML fields of `n`, mutably.
    pub fn raw_html_mut(&mut self, n: NodeId) -> Option<&mut RawHTML> {
        match self.value_mut(n) {
            NodeValue::RawHTML(r) => Some(r),
            _ => None,
        }
    }
}

// Go: ast/inline.go:MergeOrAppendTextSegment
/// MergeOrAppendTextSegment merges a given s into the last child of the parent if
/// it can be merged, otherwise creates a new Text node and appends it to after current
/// last child.
pub fn merge_or_append_text_segment(ast: &mut Ast, parent: NodeId, s: Segment) {
    let last = ast.last_child(parent);
    if let Some(last) = last
        && let Some(t) = ast.text_node_mut(last)
        && t.segment.stop == s.start
        && !t.soft_line_break()
    {
        t.segment = t.segment.with_stop(s.stop);
        return;
    }
    let t = ast.new_text_segment(s);
    ast.append_child(parent, t);
}

// Go: ast/inline.go:MergeOrReplaceTextSegment
/// MergeOrReplaceTextSegment merges a given s into a previous sibling of the node n
/// if a previous sibling of the node n is *Text, otherwise replaces Node n with s.
pub fn merge_or_replace_text_segment(ast: &mut Ast, parent: NodeId, n: NodeId, s: Segment) {
    let prev = ast.previous_sibling(n);
    if let Some(prev) = prev
        && let Some(t) = ast.text_node_mut(prev)
        && t.segment.stop == s.start
        && !t.soft_line_break()
    {
        t.segment = t.segment.with_stop(s.stop);
        ast.remove_child(parent, n);
        return;
    }
    let t = ast.new_text_segment(s);
    ast.replace_child(parent, n, t);
}
