//! Port of `golang.org/x/net@v0.41.0/html/parse.go` (`Parse`: the HTML5 tree construction
//! algorithm with every insertion mode, foreign content and Go's documented divergences).
//!
//! Insertion modes are an enum ([`Im`]) instead of Go func values. Where Go panics (a bad parser
//! state, an index out of range, a nil dereference) the port records the Go panic text, stops
//! and returns it as an error (`Err(text)`); accessors used at such sites return a harmless
//! stand-in once the error is recorded.

use super::atom::{self as a, Atom};
use super::doctype::{WHITESPACE, parse_doctype, trim_left};
use super::foreign::*;
use super::node::{Document, Node, NodeId, NodeType, SCOPE_MARKER};
use super::token::{Token, TokenType, Tokenizer};

/// Go: `insertionMode` func values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Im {
    Initial,
    BeforeHtml,
    BeforeHead,
    InHead,
    InHeadNoscript,
    AfterHead,
    InBody,
    Text,
    InTable,
    InCaption,
    InColumnGroup,
    InTableBody,
    InRow,
    InCell,
    InSelect,
    InSelectInTable,
    InTemplate,
    AfterBody,
    InFrameset,
    AfterFrameset,
    AfterAfterBody,
    AfterAfterFrameset,
    IgnoreTheRemainingTokens,
    /// A nil insertionMode (Go: `p.im = p.templateStack.top()` of an empty stack); calling it
    /// panics.
    Nil,
}

/// Go: `scope`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scope {
    Default,
    ListItem,
    Button,
    Table,
    TableRow,
    TableBody,
    Select,
}

/// Go's runtime error texts for the panic sites.
const NIL_DEREF: &str = "runtime error: invalid memory address or nil pointer dereference";

fn index_oob(i: isize, len: usize) -> String {
    format!("runtime error: index out of range [{i}] with length {len}")
}

/// Go: `parser`.
pub(crate) struct Parser {
    /// tokenizer provides the tokens for the parser.
    tokenizer: Tokenizer,
    /// tok is the most recently read token.
    tok: Token,
    /// Self-closing tags like <hr/> are treated as start tags, except that hasSelfClosingToken
    /// is set while they are being processed.
    has_self_closing_token: bool,
    /// The node arena.
    pub(crate) d: Document,
    /// doc is the document root element.
    pub(crate) doc: NodeId,
    /// The stack of open elements (section 12.2.4.2) and active formatting elements (section
    /// 12.2.4.3).
    oe: Vec<NodeId>,
    afe: Vec<NodeId>,
    /// Element pointers (section 12.2.4.4).
    head: Option<NodeId>,
    form: Option<NodeId>,
    /// Other parsing state flags (section 12.2.4.5).
    scripting: bool,
    frameset_ok: bool,
    /// The stack of template insertion modes
    template_stack: Vec<Im>,
    /// im is the current insertion mode.
    im: Im,
    /// originalIM is the insertion mode to go back to after completing a text or inTableText
    /// insertion mode.
    original_im: Option<Im>,
    /// fosterParenting is whether new elements should be inserted according to the foster
    /// parenting rules (section 12.2.6.1).
    foster_parenting: bool,
    /// quirks is whether the parser is operating in "quirks mode."
    quirks: bool,
    /// fragment is whether the parser is parsing an HTML fragment.
    fragment: bool,
    /// context is the context element when parsing an HTML fragment (section 12.4).
    context: Option<NodeId>,
    /// The first Go panic (the port stops there).
    err: Option<String>,
}

impl Parser {
    // ------------------------------------------------------------------
    // Error and stack helpers.

    fn fail(&mut self, msg: impl Into<String>) {
        if self.err.is_none() {
            self.err = Some(msg.into());
        }
    }

    fn failed(&self) -> bool {
        self.err.is_some()
    }

    /// `n.Type`, etc. of a node.
    fn n(&self, id: NodeId) -> &Node {
        &self.d.nodes[id]
    }

    fn n_mut(&mut self, id: NodeId) -> &mut Node {
        &mut self.d.nodes[id]
    }

    /// Go: `p.oe[i]` (a failed index access records Go's panic and yields the document).
    fn oe_at(&mut self, i: isize) -> NodeId {
        if i < 0 || i as usize >= self.oe.len() {
            let len = self.oe.len();
            self.fail(index_oob(i, len));
            return self.doc;
        }
        self.oe[i as usize]
    }

    /// Go: `nodeStack.pop()` (panics on an empty stack).
    fn oe_pop(&mut self) -> NodeId {
        match self.oe.pop() {
            Some(n) => n,
            None => {
                self.fail(index_oob(-1, 0));
                self.doc
            }
        }
    }

    /// Go: `p.oe.top()` dereferenced (nil when empty).
    fn oe_top_deref(&mut self) -> NodeId {
        match self.oe.last() {
            Some(&n) => n,
            None => {
                self.fail(NIL_DEREF);
                self.doc
            }
        }
    }

    /// Go: `nodeStack.index(n)`.
    fn stack_index(s: &[NodeId], n: NodeId) -> isize {
        for i in (0..s.len()).rev() {
            if s[i] == n {
                return i as isize;
            }
        }
        -1
    }

    /// Go: `nodeStack.contains(a)`.
    fn oe_contains(&self, at: Atom) -> bool {
        self.oe
            .iter()
            .any(|&n| self.n(n).data_atom == at && self.n(n).namespace.is_empty())
    }

    /// Go: `nodeStack.insert(i, n)`.
    fn stack_insert(s: &mut Vec<NodeId>, i: isize, n: NodeId) -> Result<(), String> {
        if i < 0 || i as usize > s.len() {
            return Err(format!(
                "runtime error: slice bounds out of range [{}:{}]",
                i + 1,
                s.len() + 1
            ));
        }
        s.insert(i as usize, n);
        Ok(())
    }

    /// Go: `nodeStack.remove(n)`.
    fn stack_remove(s: &mut Vec<NodeId>, n: NodeId) {
        let i = Self::stack_index(s, n);
        if i == -1 {
            return;
        }
        s.remove(i as usize);
    }

    fn append_child(&mut self, parent: NodeId, c: NodeId) {
        if let Err(e) = self.d.append_child(parent, c) {
            self.fail(e);
        }
    }

    fn remove_child(&mut self, parent: NodeId, c: NodeId) {
        if let Err(e) = self.d.remove_child(parent, c) {
            self.fail(e);
        }
    }

    // ------------------------------------------------------------------

    // Go: html/parse.go:top
    fn top(&self) -> NodeId {
        match self.oe.last() {
            Some(&n) => n,
            None => self.doc,
        }
    }

    /// popUntil pops the stack of open elements at the highest element whose tag is in
    /// matchTags, provided there is no higher element in the scope's stop tags. It returns
    /// whether or not there was such an element.
    // Go: html/parse.go:popUntil
    fn pop_until(&mut self, s: Scope, match_tags: &[Atom]) -> bool {
        let i = self.index_of_element_in_scope(s, match_tags);
        if i != -1 {
            self.oe.truncate(i as usize);
            return true;
        }
        false
    }

    /// indexOfElementInScope returns the index in p.oe of the highest element whose tag is in
    /// matchTags that is in scope. If no matching element is in scope, it returns -1.
    // Go: html/parse.go:indexOfElementInScope
    fn index_of_element_in_scope(&self, s: Scope, match_tags: &[Atom]) -> isize {
        for i in (0..self.oe.len()).rev() {
            let n = self.n(self.oe[i]);
            let tag_atom = n.data_atom;
            if n.namespace.is_empty() {
                if match_tags.contains(&tag_atom) {
                    return i as isize;
                }
                match s {
                    Scope::Default => {}
                    Scope::ListItem => {
                        if tag_atom == a::OL || tag_atom == a::UL {
                            return -1;
                        }
                    }
                    Scope::Button => {
                        if tag_atom == a::BUTTON {
                            return -1;
                        }
                    }
                    Scope::Table => {
                        if tag_atom == a::HTML || tag_atom == a::TABLE || tag_atom == a::TEMPLATE {
                            return -1;
                        }
                    }
                    Scope::Select => {
                        if tag_atom != a::OPTGROUP && tag_atom != a::OPTION {
                            return -1;
                        }
                    }
                    // Go panics "unreachable"; no caller passes these scopes.
                    Scope::TableRow | Scope::TableBody => return -1,
                }
            }
            if matches!(s, Scope::Default | Scope::ListItem | Scope::Button) {
                let stop: &[Atom] = match n.namespace.as_slice() {
                    b"" => &[
                        a::APPLET,
                        a::CAPTION,
                        a::HTML,
                        a::TABLE,
                        a::TD,
                        a::TH,
                        a::MARQUEE,
                        a::OBJECT,
                        a::TEMPLATE,
                    ],
                    b"math" => &[a::ANNOTATION_XML, a::MI, a::MN, a::MO, a::MS, a::MTEXT],
                    b"svg" => &[a::DESC, a::FOREIGN_OBJECT, a::TITLE],
                    _ => &[],
                };
                if stop.contains(&tag_atom) {
                    return -1;
                }
            }
        }
        -1
    }

    /// elementInScope is like popUntil, except that it doesn't modify the stack of open
    /// elements.
    // Go: html/parse.go:elementInScope
    fn element_in_scope(&self, s: Scope, match_tags: &[Atom]) -> bool {
        self.index_of_element_in_scope(s, match_tags) != -1
    }

    /// clearStackToContext pops elements off the stack of open elements until a scope-defined
    /// element is found.
    // Go: html/parse.go:clearStackToContext
    fn clear_stack_to_context(&mut self, s: Scope) {
        for i in (0..self.oe.len()).rev() {
            let tag_atom = self.n(self.oe[i]).data_atom;
            let stop = match s {
                Scope::Table => {
                    tag_atom == a::HTML || tag_atom == a::TABLE || tag_atom == a::TEMPLATE
                }
                Scope::TableRow => {
                    tag_atom == a::HTML || tag_atom == a::TR || tag_atom == a::TEMPLATE
                }
                Scope::TableBody => {
                    tag_atom == a::HTML
                        || tag_atom == a::TBODY
                        || tag_atom == a::TFOOT
                        || tag_atom == a::THEAD
                        || tag_atom == a::TEMPLATE
                }
                _ => {
                    self.fail("unreachable");
                    return;
                }
            };
            if stop {
                self.oe.truncate(i + 1);
                return;
            }
        }
    }

    /// parseGenericRawTextElement implements the generic raw text element parsing algorithm
    /// defined in 12.2.6.2.
    // Go: html/parse.go:parseGenericRawTextElement
    fn parse_generic_raw_text_element(&mut self) {
        self.add_element();
        self.original_im = Some(self.im);
        self.im = Im::Text;
    }

    /// generateImpliedEndTags pops nodes off the stack of open elements as long as the top node
    /// has a tag name of dd, dt, li, optgroup, option, p, rb, rp, rt or rtc. If exceptions are
    /// specified, nodes with that name will not be popped off.
    // Go: html/parse.go:generateImpliedEndTags
    fn generate_implied_end_tags(&mut self, exceptions: &[&[u8]]) {
        let mut i = self.oe.len() as isize - 1;
        'outer: while i >= 0 {
            let n = self.n(self.oe[i as usize]);
            if n.typ != NodeType::Element {
                break;
            }
            match n.data_atom {
                a::DD
                | a::DT
                | a::LI
                | a::OPTGROUP
                | a::OPTION
                | a::P
                | a::RB
                | a::RP
                | a::RT
                | a::RTC => {
                    for except in exceptions {
                        if n.data == *except {
                            break 'outer;
                        }
                    }
                    i -= 1;
                    continue;
                }
                _ => {}
            }
            break;
        }
        self.oe.truncate((i + 1) as usize);
    }

    /// addChild adds a child node n to the top element, and pushes n onto the stack of open
    /// elements if it is an element node.
    // Go: html/parse.go:addChild
    fn add_child(&mut self, n: NodeId) {
        if self.should_foster_parent() {
            self.foster_parent(n);
        } else {
            let t = self.top();
            self.append_child(t, n);
        }

        if self.n(n).typ == NodeType::Element {
            self.oe.push(n);
        }
    }

    /// shouldFosterParent returns whether the next node to be added should be foster parented.
    // Go: html/parse.go:shouldFosterParent
    fn should_foster_parent(&self) -> bool {
        if self.foster_parenting {
            return matches!(
                self.n(self.top()).data_atom,
                a::TABLE | a::TBODY | a::TFOOT | a::THEAD | a::TR
            );
        }
        false
    }

    /// fosterParent adds a child node according to the foster parenting rules. Section
    /// 12.2.6.1, "foster parenting".
    // Go: html/parse.go:fosterParent
    fn foster_parent(&mut self, n: NodeId) {
        let mut table: Option<NodeId> = None;
        let mut template: Option<NodeId> = None;
        let mut i = self.oe.len() as isize - 1;
        while i >= 0 {
            if self.n(self.oe[i as usize]).data_atom == a::TABLE {
                table = Some(self.oe[i as usize]);
                break;
            }
            i -= 1;
        }

        let mut j = self.oe.len() as isize - 1;
        while j >= 0 {
            if self.n(self.oe[j as usize]).data_atom == a::TEMPLATE {
                template = Some(self.oe[j as usize]);
                break;
            }
            j -= 1;
        }

        if let Some(t) = template
            && (table.is_none() || j > i)
        {
            self.append_child(t, n);
            return;
        }

        let mut parent = match table {
            // The foster parent is the html element.
            None => Some(self.oe_at(0)),
            Some(t) => self.n(t).parent,
        };
        if parent.is_none() {
            parent = Some(self.oe_at(i - 1));
        }
        let parent = parent.expect("set");
        if self.failed() {
            return;
        }

        let prev = match table {
            Some(t) => self.n(t).prev_sibling,
            None => self.n(parent).last_child,
        };
        if let Some(pv) = prev
            && self.n(pv).typ == NodeType::Text
            && self.n(n).typ == NodeType::Text
        {
            let data = self.n(n).data.clone();
            self.n_mut(pv).data.extend_from_slice(&data);
            return;
        }

        if let Err(e) = self.d.insert_before(parent, n, table) {
            self.fail(e);
        }
    }

    /// addText adds text to the preceding node if it is a text node, or else it calls addChild
    /// with a new text node.
    // Go: html/parse.go:addText
    fn add_text(&mut self, text: &[u8]) {
        if text.is_empty() {
            return;
        }

        if self.should_foster_parent() {
            let n = self
                .d
                .add(Node::new(NodeType::Text, 0, text.to_vec(), Vec::new()));
            self.foster_parent(n);
            return;
        }

        let t = self.top();
        if let Some(n) = self.n(t).last_child
            && self.n(n).typ == NodeType::Text
        {
            self.n_mut(n).data.extend_from_slice(text);
            return;
        }
        let n = self
            .d
            .add(Node::new(NodeType::Text, 0, text.to_vec(), Vec::new()));
        self.add_child(n);
    }

    /// addElement adds a child element based on the current token.
    // Go: html/parse.go:addElement
    fn add_element(&mut self) {
        let n = self.d.add(Node::new(
            NodeType::Element,
            self.tok.data_atom,
            self.tok.data.clone(),
            self.tok.attr.clone(),
        ));
        self.add_child(n);
    }

    /// Adds a comment node with the current token's data to the top element.
    fn add_comment_child(&mut self) {
        let n = self.d.add(Node::new(
            NodeType::Comment,
            0,
            self.tok.data.clone(),
            Vec::new(),
        ));
        self.add_child(n);
    }

    /// Appends a comment node with the current token's data to `parent`.
    fn append_comment(&mut self, parent: NodeId) {
        let n = self.d.add(Node::new(
            NodeType::Comment,
            0,
            self.tok.data.clone(),
            Vec::new(),
        ));
        self.append_child(parent, n);
    }

    /// Section 12.2.4.3.
    // Go: html/parse.go:addFormattingElement
    fn add_formatting_element(&mut self) {
        let tag_atom = self.tok.data_atom;
        let attr = self.tok.attr.clone();
        self.add_element();

        // Implement the Noah's Ark clause, but with three per family instead of two.
        let mut identical_elements = 0;
        let mut i = self.afe.len() as isize - 1;
        'find_identical_elements: while i >= 0 {
            let nid = self.afe[i as usize];
            i -= 1;
            let n = self.n(nid);
            if n.typ == NodeType::ScopeMarker {
                break;
            }
            if n.typ != NodeType::Element {
                continue;
            }
            if !n.namespace.is_empty() {
                continue;
            }
            if n.data_atom != tag_atom {
                continue;
            }
            if n.attr.len() != attr.len() {
                continue;
            }
            'compare_attributes: for t0 in &n.attr {
                for t1 in &attr {
                    if t0.key == t1.key && t0.namespace == t1.namespace && t0.val == t1.val {
                        // Found a match for this attribute, continue with the next attribute.
                        continue 'compare_attributes;
                    }
                }
                // If we get here, there is no attribute that matches a. Therefore the element
                // is not identical to the new one.
                continue 'find_identical_elements;
            }

            identical_elements += 1;
            if identical_elements >= 3 {
                Self::stack_remove(&mut self.afe, nid);
            }
        }

        let t = self.top();
        self.afe.push(t);
    }

    /// Section 12.2.4.3.
    // Go: html/parse.go:clearActiveFormattingElements
    fn clear_active_formatting_elements(&mut self) {
        loop {
            let Some(n) = self.afe.pop() else {
                self.fail(index_oob(-1, 0));
                return;
            };
            if self.afe.is_empty() || self.n(n).typ == NodeType::ScopeMarker {
                return;
            }
        }
    }

    /// Section 12.2.4.3.
    // Go: html/parse.go:reconstructActiveFormattingElements
    fn reconstruct_active_formatting_elements(&mut self) {
        let Some(&top) = self.afe.last() else {
            return;
        };
        let mut n = top;
        if self.n(n).typ == NodeType::ScopeMarker || Self::stack_index(&self.oe, n) != -1 {
            return;
        }
        let mut i = self.afe.len() as isize - 1;
        while self.n(n).typ != NodeType::ScopeMarker && Self::stack_index(&self.oe, n) == -1 {
            if i == 0 {
                i = -1;
                break;
            }
            i -= 1;
            n = self.afe[i as usize];
        }
        loop {
            i += 1;
            let clone = self.d.clone_node(self.afe[i as usize]);
            self.add_child(clone);
            self.afe[i as usize] = clone;
            if i as usize == self.afe.len() - 1 {
                break;
            }
        }
    }

    /// Section 12.2.5.
    // Go: html/parse.go:acknowledgeSelfClosingTag
    fn acknowledge_self_closing_tag(&mut self) {
        self.has_self_closing_token = false;
    }

    /// setOriginalIM sets the insertion mode to return to after completing a text or
    /// inTableText insertion mode.
    // Go: html/parse.go:setOriginalIM
    fn set_original_im(&mut self) {
        if self.original_im.is_some() {
            self.fail("html: bad parser state: originalIM was set twice");
            return;
        }
        self.original_im = Some(self.im);
    }

    /// Section 12.2.4.1, "reset the insertion mode".
    // Go: html/parse.go:resetInsertionMode
    fn reset_insertion_mode(&mut self) {
        let mut i = self.oe.len() as isize - 1;
        while i >= 0 {
            let mut n = self.oe[i as usize];
            let last = i == 0;
            if last && let Some(c) = self.context {
                n = c;
            }

            match self.n(n).data_atom {
                a::SELECT => {
                    if !last {
                        let first = self.oe[0];
                        let mut ancestor = n;
                        while ancestor != first {
                            let idx = Self::stack_index(&self.oe, ancestor) - 1;
                            ancestor = self.oe_at(idx);
                            if self.failed() {
                                return;
                            }
                            match self.n(ancestor).data_atom {
                                a::TEMPLATE => {
                                    self.im = Im::InSelect;
                                    return;
                                }
                                a::TABLE => {
                                    self.im = Im::InSelectInTable;
                                    return;
                                }
                                _ => {}
                            }
                        }
                    }
                    self.im = Im::InSelect;
                }
                // TODO: remove this divergence from the HTML5 spec.
                a::TD | a::TH => self.im = Im::InCell,
                a::TR => self.im = Im::InRow,
                a::TBODY | a::THEAD | a::TFOOT => self.im = Im::InTableBody,
                a::CAPTION => self.im = Im::InCaption,
                a::COLGROUP => self.im = Im::InColumnGroup,
                a::TABLE => self.im = Im::InTable,
                a::TEMPLATE => {
                    // TODO: remove this divergence from the HTML5 spec.
                    if !self.n(n).namespace.is_empty() {
                        i -= 1;
                        continue;
                    }
                    self.im = self.template_stack.last().copied().unwrap_or(Im::Nil);
                }
                // TODO: remove this divergence from the HTML5 spec.
                a::HEAD => self.im = Im::InHead,
                a::BODY => self.im = Im::InBody,
                a::FRAMESET => self.im = Im::InFrameset,
                a::HTML => {
                    if self.head.is_none() {
                        self.im = Im::BeforeHead;
                    } else {
                        self.im = Im::AfterHead;
                    }
                }
                _ => {
                    if last {
                        self.im = Im::InBody;
                        return;
                    }
                    i -= 1;
                    continue;
                }
            }
            return;
        }
    }

    /// Runs the insertion mode `im` (Go: `p.im(p)` / `inBodyIM(p)` / ...).
    fn call(&mut self, im: Im) -> bool {
        match im {
            Im::Initial => initial_im(self),
            Im::BeforeHtml => before_html_im(self),
            Im::BeforeHead => before_head_im(self),
            Im::InHead => in_head_im(self),
            Im::InHeadNoscript => in_head_noscript_im(self),
            Im::AfterHead => after_head_im(self),
            Im::InBody => in_body_im(self),
            Im::Text => text_im(self),
            Im::InTable => in_table_im(self),
            Im::InCaption => in_caption_im(self),
            Im::InColumnGroup => in_column_group_im(self),
            Im::InTableBody => in_table_body_im(self),
            Im::InRow => in_row_im(self),
            Im::InCell => in_cell_im(self),
            Im::InSelect => in_select_im(self),
            Im::InSelectInTable => in_select_in_table_im(self),
            Im::InTemplate => in_template_im(self),
            Im::AfterBody => after_body_im(self),
            Im::InFrameset => in_frameset_im(self),
            Im::AfterFrameset => after_frameset_im(self),
            Im::AfterAfterBody => after_after_body_im(self),
            Im::AfterAfterFrameset => after_after_frameset_im(self),
            Im::IgnoreTheRemainingTokens => true,
            Im::Nil => {
                self.fail(NIL_DEREF);
                true
            }
        }
    }

    /// Go: `p.parseImpliedToken(t, dataAtom, data)` — parses a token as though it had appeared
    /// in the parser's input.
    // Go: html/parse.go:parseImpliedToken
    fn parse_implied_token(&mut self, t: TokenType, data_atom: Atom) {
        let real_token = std::mem::replace(
            &mut self.tok,
            Token {
                typ: t,
                data_atom,
                data: a::string(data_atom).to_vec(),
                attr: Vec::new(),
            },
        );
        let self_closing = self.has_self_closing_token;
        self.has_self_closing_token = false;
        self.parse_current_token();
        self.tok = real_token;
        self.has_self_closing_token = self_closing;
    }

    /// parseCurrentToken runs the current token through the parsing routines until it is
    /// consumed.
    // Go: html/parse.go:parseCurrentToken
    fn parse_current_token(&mut self) {
        if self.tok.typ == TokenType::SelfClosingTag {
            self.has_self_closing_token = true;
            self.tok.typ = TokenType::StartTag;
        }

        let mut consumed = false;
        while !consumed {
            if self.failed() {
                return;
            }
            if self.in_foreign_content() {
                consumed = parse_foreign_content(self);
            } else {
                consumed = self.call(self.im);
            }
        }

        if self.has_self_closing_token {
            // This is a parse error, but ignore it.
            self.has_self_closing_token = false;
        }
    }

    // Go: html/parse.go:adjustedCurrentNode
    fn adjusted_current_node(&self) -> Option<NodeId> {
        if self.oe.len() == 1
            && self.fragment
            && let Some(c) = self.context
        {
            return Some(c);
        }
        self.oe.last().copied()
    }

    /// Section 12.2.6.
    // Go: html/parse.go:inForeignContent
    fn in_foreign_content(&self) -> bool {
        if self.oe.is_empty() {
            return false;
        }
        let n = self.adjusted_current_node().expect("non-empty");
        if self.n(n).namespace.is_empty() {
            return false;
        }
        if math_ml_text_integration_point(&self.d, n) {
            if self.tok.typ == TokenType::StartTag
                && self.tok.data_atom != a::MGLYPH
                && self.tok.data_atom != a::MALIGNMARK
            {
                return false;
            }
            if self.tok.typ == TokenType::Text {
                return false;
            }
        }
        if self.n(n).namespace == b"math"
            && self.n(n).data_atom == a::ANNOTATION_XML
            && self.tok.typ == TokenType::StartTag
            && self.tok.data_atom == a::SVG
        {
            return false;
        }
        if html_integration_point(&self.d, n)
            && (self.tok.typ == TokenType::StartTag || self.tok.typ == TokenType::Text)
        {
            return false;
        }
        if self.tok.typ == TokenType::Error {
            return false;
        }
        true
    }

    // Go: html/parse.go:parse
    fn parse(&mut self) -> Result<(), String> {
        // Iterate until EOF. Any other error will cause an early return.
        let mut eof = false;
        while !eof {
            // CDATA sections are allowed only in foreign content.
            let allow = match self.oe.last() {
                Some(&n) => !self.n(n).namespace.is_empty(),
                None => false,
            };
            self.tokenizer.allow_cdata(allow);
            // Read and parse the next token.
            self.tokenizer.next();
            self.tok = self.tokenizer.token();
            if self.tok.typ == TokenType::Error {
                eof = self.tokenizer.err_is_eof();
            }
            self.parse_current_token();
            if let Some(e) = self.err.take() {
                return Err(e);
            }
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // The adoption agency and "any other end tag".

    // Go: html/parse.go:inBodyEndTagFormatting
    fn in_body_end_tag_formatting(&mut self, tag_atom: Atom, tag_name: &[u8]) {
        // This is the "adoption agency" algorithm, described at
        // https://html.spec.whatwg.org/multipage/syntax.html#adoptionAgency

        // Steps 1-2
        let current = self.oe_top_deref();
        if self.failed() {
            return;
        }
        if self.n(current).data == tag_name && Self::stack_index(&self.afe, current) == -1 {
            self.oe_pop();
            return;
        }

        // Steps 3-5. The outer loop.
        for _ in 0..8 {
            // Step 6. Find the formatting element.
            let mut formatting_element: Option<NodeId> = None;
            for j in (0..self.afe.len()).rev() {
                if self.n(self.afe[j]).typ == NodeType::ScopeMarker {
                    break;
                }
                if self.n(self.afe[j]).data_atom == tag_atom {
                    formatting_element = Some(self.afe[j]);
                    break;
                }
            }
            let Some(formatting_element) = formatting_element else {
                self.in_body_end_tag_other(tag_atom, tag_name);
                return;
            };

            // Step 7. Ignore the tag if formatting element is not in the stack of open
            // elements.
            let fe_index = Self::stack_index(&self.oe, formatting_element);
            if fe_index == -1 {
                Self::stack_remove(&mut self.afe, formatting_element);
                return;
            }
            // Step 8. Ignore the tag if formatting element is not in the scope.
            if !self.element_in_scope(Scope::Default, &[tag_atom]) {
                // Ignore the tag.
                return;
            }

            // Step 9. This step is omitted because it's just a parse error but no need to
            // return.

            // Steps 10-11. Find the furthest block.
            let mut furthest_block: Option<NodeId> = None;
            for &e in &self.oe[fe_index as usize..] {
                if is_special_element(&self.d, e) {
                    furthest_block = Some(e);
                    break;
                }
            }
            let Some(furthest_block) = furthest_block else {
                let mut e = self.oe_pop();
                while e != formatting_element {
                    if self.failed() {
                        return;
                    }
                    e = self.oe_pop();
                }
                Self::stack_remove(&mut self.afe, e);
                return;
            };

            // Steps 12-13. Find the common ancestor and bookmark node.
            let common_ancestor = self.oe_at(fe_index - 1);
            if self.failed() {
                return;
            }
            let mut bookmark = Self::stack_index(&self.afe, formatting_element);

            // Step 14. The inner loop. Find the lastNode to reparent.
            let mut last_node = furthest_block;
            let mut node;
            let mut x = Self::stack_index(&self.oe, furthest_block);
            // Step 14.1.
            let mut j = 0;
            loop {
                // Step 14.2.
                j += 1;
                // Step. 14.3.
                x -= 1;
                node = self.oe_at(x);
                if self.failed() {
                    return;
                }
                // Step 14.4. Go to the next step if node is formatting element.
                if node == formatting_element {
                    break;
                }
                // Step 14.5. Remove node from the list of active formatting elements if inner
                // loop counter is greater than three and node is in the list of active
                // formatting elements.
                let ni = Self::stack_index(&self.afe, node);
                if j > 3 && ni > -1 {
                    Self::stack_remove(&mut self.afe, node);
                    // If any element of the list of active formatting elements is removed, we
                    // need to take care whether bookmark should be decremented or not.
                    if ni <= bookmark {
                        bookmark -= 1;
                    }
                    continue;
                }
                // Step 14.6. Continue the next inner loop if node is not in the list of active
                // formatting elements.
                if Self::stack_index(&self.afe, node) == -1 {
                    Self::stack_remove(&mut self.oe, node);
                    continue;
                }
                // Step 14.7.
                let clone = self.d.clone_node(node);
                let ai = Self::stack_index(&self.afe, node) as usize;
                self.afe[ai] = clone;
                let oi = Self::stack_index(&self.oe, node);
                if oi < 0 {
                    self.fail(index_oob(oi, self.oe.len()));
                    return;
                }
                self.oe[oi as usize] = clone;
                node = clone;
                // Step 14.8.
                if last_node == furthest_block {
                    bookmark = Self::stack_index(&self.afe, node) + 1;
                }
                // Step 14.9.
                if let Some(p) = self.n(last_node).parent {
                    self.remove_child(p, last_node);
                }
                self.append_child(node, last_node);
                // Step 14.10.
                last_node = node;
            }

            // Step 15. Reparent lastNode to the common ancestor, or for misnested table nodes,
            // to the foster parent.
            if let Some(p) = self.n(last_node).parent {
                self.remove_child(p, last_node);
            }
            match self.n(common_ancestor).data_atom {
                a::TABLE | a::TBODY | a::TFOOT | a::THEAD | a::TR => {
                    self.foster_parent(last_node);
                }
                _ => {
                    self.append_child(common_ancestor, last_node);
                }
            }

            // Steps 16-18. Reparent nodes from the furthest block's children to a clone of the
            // formatting element.
            let clone = self.d.clone_node(formatting_element);
            if let Err(e) = self.d.reparent_children(clone, furthest_block) {
                self.fail(e);
            }
            self.append_child(furthest_block, clone);

            // Step 19. Fix up the list of active formatting elements.
            let old_loc = Self::stack_index(&self.afe, formatting_element);
            if old_loc != -1 && old_loc < bookmark {
                // Move the bookmark with the rest of the list.
                bookmark -= 1;
            }
            Self::stack_remove(&mut self.afe, formatting_element);
            if let Err(e) = Self::stack_insert(&mut self.afe, bookmark, clone) {
                self.fail(e);
                return;
            }

            // Step 20. Fix up the stack of open elements.
            Self::stack_remove(&mut self.oe, formatting_element);
            let fi = Self::stack_index(&self.oe, furthest_block) + 1;
            if let Err(e) = Self::stack_insert(&mut self.oe, fi, clone) {
                self.fail(e);
                return;
            }
            if self.failed() {
                return;
            }
        }
    }

    /// inBodyEndTagOther performs the "any other end tag" algorithm for inBodyIM.
    // Go: html/parse.go:inBodyEndTagOther
    fn in_body_end_tag_other(&mut self, tag_atom: Atom, tag_name: &[u8]) {
        for i in (0..self.oe.len()).rev() {
            let n = self.n(self.oe[i]);
            // The if condition here is equivalent to (p.oe[i].Data == tagName).
            if n.data_atom == tag_atom && (tag_atom != 0 || n.data == tag_name) {
                self.oe.truncate(i);
                break;
            }
            if is_special_element(&self.d, self.oe[i]) {
                break;
            }
        }
    }
}

/// `strings.Replace(s, "\x00", "", -1)`.
fn remove_nul(s: &[u8]) -> Vec<u8> {
    s.iter().copied().filter(|&c| c != 0).collect()
}

/// `strings.Map` keeping only ' ', '\t', '\n', '\f', '\r' (runes; the others are dropped).
fn keep_whitespace(s: &[u8]) -> Vec<u8> {
    // Every rune but the five ASCII whitespace ones is dropped; invalid bytes decode to
    // U+FFFD and are dropped too, so a byte filter gives the same bytes.
    s.iter()
        .copied()
        .filter(|&c| matches!(c, b' ' | b'\t' | b'\n' | b'\x0c' | b'\r'))
        .collect()
}

/// Section 12.2.6.4.1.
// Go: html/parse.go:initialIM
fn initial_im(p: &mut Parser) -> bool {
    match p.tok.typ {
        TokenType::Text => {
            p.tok.data = trim_left(&p.tok.data, WHITESPACE).to_vec();
            if p.tok.data.is_empty() {
                // It was all whitespace, so ignore it.
                return true;
            }
        }
        TokenType::Comment => {
            let doc = p.doc;
            p.append_comment(doc);
            return true;
        }
        TokenType::Doctype => {
            let (n, quirks) = parse_doctype(&p.tok.data);
            let n = p.d.add(n);
            let doc = p.doc;
            p.append_child(doc, n);
            p.quirks = quirks;
            p.im = Im::BeforeHtml;
            return true;
        }
        _ => {}
    }
    p.quirks = true;
    p.im = Im::BeforeHtml;
    false
}

/// Section 12.2.6.4.2.
// Go: html/parse.go:beforeHTMLIM
fn before_html_im(p: &mut Parser) -> bool {
    match p.tok.typ {
        TokenType::Doctype => {
            // Ignore the token.
            return true;
        }
        TokenType::Text => {
            p.tok.data = trim_left(&p.tok.data, WHITESPACE).to_vec();
            if p.tok.data.is_empty() {
                // It was all whitespace, so ignore it.
                return true;
            }
        }
        TokenType::StartTag => {
            if p.tok.data_atom == a::HTML {
                p.add_element();
                p.im = Im::BeforeHead;
                return true;
            }
        }
        TokenType::EndTag => match p.tok.data_atom {
            a::HEAD | a::BODY | a::HTML | a::BR => {
                p.parse_implied_token(TokenType::StartTag, a::HTML);
                return false;
            }
            _ => {
                // Ignore the token.
                return true;
            }
        },
        TokenType::Comment => {
            let doc = p.doc;
            p.append_comment(doc);
            return true;
        }
        _ => {}
    }
    p.parse_implied_token(TokenType::StartTag, a::HTML);
    false
}

/// Section 12.2.6.4.3.
// Go: html/parse.go:beforeHeadIM
fn before_head_im(p: &mut Parser) -> bool {
    match p.tok.typ {
        TokenType::Text => {
            p.tok.data = trim_left(&p.tok.data, WHITESPACE).to_vec();
            if p.tok.data.is_empty() {
                // It was all whitespace, so ignore it.
                return true;
            }
        }
        TokenType::StartTag => match p.tok.data_atom {
            a::HEAD => {
                p.add_element();
                p.head = Some(p.top());
                p.im = Im::InHead;
                return true;
            }
            a::HTML => return in_body_im(p),
            _ => {}
        },
        TokenType::EndTag => match p.tok.data_atom {
            a::HEAD | a::BODY | a::HTML | a::BR => {
                p.parse_implied_token(TokenType::StartTag, a::HEAD);
                return false;
            }
            _ => {
                // Ignore the token.
                return true;
            }
        },
        TokenType::Comment => {
            p.add_comment_child();
            return true;
        }
        TokenType::Doctype => {
            // Ignore the token.
            return true;
        }
        _ => {}
    }

    p.parse_implied_token(TokenType::StartTag, a::HEAD);
    false
}

/// Section 12.2.6.4.4.
// Go: html/parse.go:inHeadIM
fn in_head_im(p: &mut Parser) -> bool {
    match p.tok.typ {
        TokenType::Text => {
            let s = trim_left(&p.tok.data, WHITESPACE).to_vec();
            if s.len() < p.tok.data.len() {
                // Add the initial whitespace to the current node.
                let ws = p.tok.data[..p.tok.data.len() - s.len()].to_vec();
                p.add_text(&ws);
                if s.is_empty() {
                    return true;
                }
                p.tok.data = s;
            }
        }
        TokenType::StartTag => match p.tok.data_atom {
            a::HTML => return in_body_im(p),
            a::BASE | a::BASEFONT | a::BGSOUND | a::LINK | a::META => {
                p.add_element();
                p.oe_pop();
                p.acknowledge_self_closing_tag();
                return true;
            }
            a::NOSCRIPT => {
                if p.scripting {
                    p.parse_generic_raw_text_element();
                    return true;
                }
                p.add_element();
                p.im = Im::InHeadNoscript;
                // Don't let the tokenizer go into raw text mode when scripting is disabled.
                p.tokenizer.next_is_not_raw_text();
                return true;
            }
            a::SCRIPT | a::TITLE => {
                p.add_element();
                p.set_original_im();
                p.im = Im::Text;
                return true;
            }
            a::NOFRAMES | a::STYLE => {
                p.parse_generic_raw_text_element();
                return true;
            }
            a::HEAD => {
                // Ignore the token.
                return true;
            }
            a::TEMPLATE => {
                // TODO: remove this divergence from the HTML5 spec.
                //
                // As a workaround, if we are mixing foreign content and templates, just ignore
                // the rest of the HTML.
                for &e in &p.oe {
                    if !p.n(e).namespace.is_empty() {
                        p.im = Im::IgnoreTheRemainingTokens;
                        return true;
                    }
                }

                p.add_element();
                p.afe.push(SCOPE_MARKER);
                p.frameset_ok = false;
                p.im = Im::InTemplate;
                p.template_stack.push(Im::InTemplate);
                return true;
            }
            _ => {}
        },
        TokenType::EndTag => match p.tok.data_atom {
            a::HEAD => {
                p.oe_pop();
                p.im = Im::AfterHead;
                return true;
            }
            a::BODY | a::HTML | a::BR => {
                p.parse_implied_token(TokenType::EndTag, a::HEAD);
                return false;
            }
            a::TEMPLATE => {
                if !p.oe_contains(a::TEMPLATE) {
                    return true;
                }
                // TODO: remove this further divergence from the HTML5 spec.
                p.generate_implied_end_tags(&[]);
                for i in (0..p.oe.len()).rev() {
                    let n = p.n(p.oe[i]);
                    if n.namespace.is_empty() && n.data_atom == a::TEMPLATE {
                        p.oe.truncate(i);
                        break;
                    }
                }
                p.clear_active_formatting_elements();
                p.template_stack.pop();
                p.reset_insertion_mode();
                return true;
            }
            _ => {
                // Ignore the token.
                return true;
            }
        },
        TokenType::Comment => {
            p.add_comment_child();
            return true;
        }
        TokenType::Doctype => {
            // Ignore the token.
            return true;
        }
        _ => {}
    }

    p.parse_implied_token(TokenType::EndTag, a::HEAD);
    false
}

/// Section 12.2.6.4.5.
// Go: html/parse.go:inHeadNoscriptIM
fn in_head_noscript_im(p: &mut Parser) -> bool {
    match p.tok.typ {
        TokenType::Doctype => {
            // Ignore the token.
            return true;
        }
        TokenType::StartTag => match p.tok.data_atom {
            a::HTML => return in_body_im(p),
            a::BASEFONT | a::BGSOUND | a::LINK | a::META | a::NOFRAMES | a::STYLE => {
                return in_head_im(p);
            }
            a::HEAD => {
                // Ignore the token.
                return true;
            }
            a::NOSCRIPT => {
                // Don't let the tokenizer go into raw text mode even when a <noscript> tag is
                // in "in head noscript" insertion mode.
                p.tokenizer.next_is_not_raw_text();
                // Ignore the token.
                return true;
            }
            _ => {}
        },
        TokenType::EndTag => match p.tok.data_atom {
            a::NOSCRIPT | a::BR => {}
            _ => {
                // Ignore the token.
                return true;
            }
        },
        TokenType::Text => {
            let s = trim_left(&p.tok.data, WHITESPACE);
            if s.is_empty() {
                // It was all whitespace.
                return in_head_im(p);
            }
        }
        TokenType::Comment => return in_head_im(p),
        _ => {}
    }
    p.oe_pop();
    if p.n(p.top()).data_atom != a::HEAD {
        p.fail("html: the new current node will be a head element.");
        return true;
    }
    p.im = Im::InHead;
    if p.tok.data_atom == a::NOSCRIPT {
        return true;
    }
    false
}

/// Section 12.2.6.4.6.
// Go: html/parse.go:afterHeadIM
fn after_head_im(p: &mut Parser) -> bool {
    match p.tok.typ {
        TokenType::Text => {
            let s = trim_left(&p.tok.data, WHITESPACE).to_vec();
            if s.len() < p.tok.data.len() {
                // Add the initial whitespace to the current node.
                let ws = p.tok.data[..p.tok.data.len() - s.len()].to_vec();
                p.add_text(&ws);
                if s.is_empty() {
                    return true;
                }
                p.tok.data = s;
            }
        }
        TokenType::StartTag => match p.tok.data_atom {
            a::HTML => return in_body_im(p),
            a::BODY => {
                p.add_element();
                p.frameset_ok = false;
                p.im = Im::InBody;
                return true;
            }
            a::FRAMESET => {
                p.add_element();
                p.im = Im::InFrameset;
                return true;
            }
            a::BASE
            | a::BASEFONT
            | a::BGSOUND
            | a::LINK
            | a::META
            | a::NOFRAMES
            | a::SCRIPT
            | a::STYLE
            | a::TEMPLATE
            | a::TITLE => {
                let Some(head) = p.head else {
                    // Go appends a nil *Node and dereferences it.
                    p.fail(NIL_DEREF);
                    return true;
                };
                p.oe.push(head);
                let r = in_head_im(p);
                // Go: `defer p.oe.remove(p.head)`.
                if let Some(head) = p.head {
                    Parser::stack_remove(&mut p.oe, head);
                }
                return r;
            }
            a::HEAD => {
                // Ignore the token.
                return true;
            }
            _ => {}
        },
        TokenType::EndTag => match p.tok.data_atom {
            a::BODY | a::HTML | a::BR => {
                // Drop down to creating an implied <body> tag.
            }
            a::TEMPLATE => return in_head_im(p),
            _ => {
                // Ignore the token.
                return true;
            }
        },
        TokenType::Comment => {
            p.add_comment_child();
            return true;
        }
        TokenType::Doctype => {
            // Ignore the token.
            return true;
        }
        _ => {}
    }

    p.parse_implied_token(TokenType::StartTag, a::BODY);
    p.frameset_ok = true;
    if p.tok.typ == TokenType::Error {
        // Stop parsing.
        return true;
    }
    false
}

/// copyAttributes copies attributes of src not found on dst to dst.
// Go: html/parse.go:copyAttributes
fn copy_attributes(p: &mut Parser, dst: NodeId) {
    if p.tok.attr.is_empty() {
        return;
    }
    let mut keys: Vec<Vec<u8>> = p.n(dst).attr.iter().map(|t| t.key.clone()).collect();
    for t in p.tok.attr.clone() {
        if !keys.contains(&t.key) {
            keys.push(t.key.clone());
            p.n_mut(dst).attr.push(t);
        }
    }
}

/// Section 12.2.6.4.7.
// Go: html/parse.go:inBodyIM
fn in_body_im(p: &mut Parser) -> bool {
    match p.tok.typ {
        TokenType::Text => {
            let mut d: &[u8] = &p.tok.data.clone();
            let n = p.oe_top_deref();
            if p.failed() {
                return true;
            }
            if matches!(p.n(n).data_atom, a::PRE | a::LISTING) && p.n(n).first_child.is_none() {
                // Ignore a newline at the start of a <pre> block.
                if !d.is_empty() && d[0] == b'\r' {
                    d = &d[1..];
                }
                if !d.is_empty() && d[0] == b'\n' {
                    d = &d[1..];
                }
            }
            let d = remove_nul(d);
            if d.is_empty() {
                return true;
            }
            p.reconstruct_active_formatting_elements();
            p.add_text(&d);
            if p.frameset_ok && !trim_left(&d, WHITESPACE).is_empty() {
                // There were non-whitespace characters inserted.
                p.frameset_ok = false;
            }
        }
        TokenType::StartTag => match p.tok.data_atom {
            a::HTML => {
                if p.oe_contains(a::TEMPLATE) {
                    return true;
                }
                let first = p.oe_at(0);
                if p.failed() {
                    return true;
                }
                copy_attributes(p, first);
            }
            a::BASE
            | a::BASEFONT
            | a::BGSOUND
            | a::LINK
            | a::META
            | a::NOFRAMES
            | a::SCRIPT
            | a::STYLE
            | a::TEMPLATE
            | a::TITLE => return in_head_im(p),
            a::BODY => {
                if p.oe_contains(a::TEMPLATE) {
                    return true;
                }
                if p.oe.len() >= 2 {
                    let body = p.oe[1];
                    if p.n(body).typ == NodeType::Element && p.n(body).data_atom == a::BODY {
                        p.frameset_ok = false;
                        copy_attributes(p, body);
                    }
                }
            }
            a::FRAMESET => {
                if !p.frameset_ok || p.oe.len() < 2 || p.n(p.oe[1]).data_atom != a::BODY {
                    // Ignore the token.
                    return true;
                }
                let body = p.oe[1];
                if let Some(parent) = p.n(body).parent {
                    p.remove_child(parent, body);
                }
                p.oe.truncate(1);
                p.add_element();
                p.im = Im::InFrameset;
                return true;
            }
            a::ADDRESS
            | a::ARTICLE
            | a::ASIDE
            | a::BLOCKQUOTE
            | a::CENTER
            | a::DETAILS
            | a::DIALOG
            | a::DIR
            | a::DIV
            | a::DL
            | a::FIELDSET
            | a::FIGCAPTION
            | a::FIGURE
            | a::FOOTER
            | a::HEADER
            | a::HGROUP
            | a::MAIN
            | a::MENU
            | a::NAV
            | a::OL
            | a::P
            | a::SEARCH
            | a::SECTION
            | a::SUMMARY
            | a::UL => {
                p.pop_until(Scope::Button, &[a::P]);
                p.add_element();
            }
            a::H1 | a::H2 | a::H3 | a::H4 | a::H5 | a::H6 => {
                p.pop_until(Scope::Button, &[a::P]);
                let n = p.top();
                if matches!(
                    p.n(n).data_atom,
                    a::H1 | a::H2 | a::H3 | a::H4 | a::H5 | a::H6
                ) {
                    p.oe_pop();
                }
                p.add_element();
            }
            a::PRE | a::LISTING => {
                p.pop_until(Scope::Button, &[a::P]);
                p.add_element();
                // The newline, if any, will be dealt with by the TextToken case.
                p.frameset_ok = false;
            }
            a::FORM => {
                if p.form.is_some() && !p.oe_contains(a::TEMPLATE) {
                    // Ignore the token
                    return true;
                }
                p.pop_until(Scope::Button, &[a::P]);
                p.add_element();
                if !p.oe_contains(a::TEMPLATE) {
                    p.form = Some(p.top());
                }
            }
            a::LI => {
                p.frameset_ok = false;
                for i in (0..p.oe.len()).rev() {
                    let node = p.oe[i];
                    match p.n(node).data_atom {
                        a::LI => {
                            p.oe.truncate(i);
                        }
                        a::ADDRESS | a::DIV | a::P => continue,
                        _ => {
                            if !is_special_element(&p.d, node) {
                                continue;
                            }
                        }
                    }
                    break;
                }
                p.pop_until(Scope::Button, &[a::P]);
                p.add_element();
            }
            a::DD | a::DT => {
                p.frameset_ok = false;
                for i in (0..p.oe.len()).rev() {
                    let node = p.oe[i];
                    match p.n(node).data_atom {
                        a::DD | a::DT => {
                            p.oe.truncate(i);
                        }
                        a::ADDRESS | a::DIV | a::P => continue,
                        _ => {
                            if !is_special_element(&p.d, node) {
                                continue;
                            }
                        }
                    }
                    break;
                }
                p.pop_until(Scope::Button, &[a::P]);
                p.add_element();
            }
            a::PLAINTEXT => {
                p.pop_until(Scope::Button, &[a::P]);
                p.add_element();
            }
            a::BUTTON => {
                p.pop_until(Scope::Default, &[a::BUTTON]);
                p.reconstruct_active_formatting_elements();
                p.add_element();
                p.frameset_ok = false;
            }
            a::A => {
                let mut i = p.afe.len() as isize - 1;
                while i >= 0 && p.n(p.afe[i as usize]).typ != NodeType::ScopeMarker {
                    let n = p.afe[i as usize];
                    if p.n(n).typ == NodeType::Element && p.n(n).data_atom == a::A {
                        p.in_body_end_tag_formatting(a::A, b"a");
                        Parser::stack_remove(&mut p.oe, n);
                        Parser::stack_remove(&mut p.afe, n);
                        break;
                    }
                    i -= 1;
                }
                if p.failed() {
                    return true;
                }
                p.reconstruct_active_formatting_elements();
                p.add_formatting_element();
            }
            a::B
            | a::BIG
            | a::CODE
            | a::EM
            | a::FONT
            | a::I
            | a::S
            | a::SMALL
            | a::STRIKE
            | a::STRONG
            | a::TT
            | a::U => {
                p.reconstruct_active_formatting_elements();
                p.add_formatting_element();
            }
            a::NOBR => {
                p.reconstruct_active_formatting_elements();
                if p.element_in_scope(Scope::Default, &[a::NOBR]) {
                    p.in_body_end_tag_formatting(a::NOBR, b"nobr");
                    p.reconstruct_active_formatting_elements();
                }
                p.add_formatting_element();
            }
            a::APPLET | a::MARQUEE | a::OBJECT => {
                p.reconstruct_active_formatting_elements();
                p.add_element();
                p.afe.push(SCOPE_MARKER);
                p.frameset_ok = false;
            }
            a::TABLE => {
                if !p.quirks {
                    p.pop_until(Scope::Button, &[a::P]);
                }
                p.add_element();
                p.frameset_ok = false;
                p.im = Im::InTable;
                return true;
            }
            a::AREA | a::BR | a::EMBED | a::IMG | a::INPUT | a::KEYGEN | a::WBR => {
                p.reconstruct_active_formatting_elements();
                p.add_element();
                p.oe_pop();
                p.acknowledge_self_closing_tag();
                if p.tok.data_atom == a::INPUT {
                    for t in &p.tok.attr {
                        if t.key == b"type" && go_unicode::strings::equal_fold(&t.val, b"hidden") {
                            // Skip setting framesetOK = false
                            return true;
                        }
                    }
                }
                p.frameset_ok = false;
            }
            a::PARAM | a::SOURCE | a::TRACK => {
                p.add_element();
                p.oe_pop();
                p.acknowledge_self_closing_tag();
            }
            a::HR => {
                p.pop_until(Scope::Button, &[a::P]);
                p.add_element();
                p.oe_pop();
                p.acknowledge_self_closing_tag();
                p.frameset_ok = false;
            }
            a::IMAGE => {
                p.tok.data_atom = a::IMG;
                p.tok.data = a::string(a::IMG).to_vec();
                return false;
            }
            a::TEXTAREA => {
                p.add_element();
                p.set_original_im();
                p.frameset_ok = false;
                p.im = Im::Text;
            }
            a::XMP => {
                p.pop_until(Scope::Button, &[a::P]);
                p.reconstruct_active_formatting_elements();
                p.frameset_ok = false;
                p.parse_generic_raw_text_element();
            }
            a::IFRAME => {
                p.frameset_ok = false;
                p.parse_generic_raw_text_element();
            }
            a::NOEMBED => {
                p.parse_generic_raw_text_element();
            }
            a::NOSCRIPT => {
                if p.scripting {
                    p.parse_generic_raw_text_element();
                    return true;
                }
                p.reconstruct_active_formatting_elements();
                p.add_element();
                // Don't let the tokenizer go into raw text mode when scripting is disabled.
                p.tokenizer.next_is_not_raw_text();
            }
            a::SELECT => {
                p.reconstruct_active_formatting_elements();
                p.add_element();
                p.frameset_ok = false;
                p.im = Im::InSelect;
                return true;
            }
            a::OPTGROUP | a::OPTION => {
                if p.n(p.top()).data_atom == a::OPTION {
                    p.oe_pop();
                }
                p.reconstruct_active_formatting_elements();
                p.add_element();
            }
            a::RB | a::RTC => {
                if p.element_in_scope(Scope::Default, &[a::RUBY]) {
                    p.generate_implied_end_tags(&[]);
                }
                p.add_element();
            }
            a::RP | a::RT => {
                if p.element_in_scope(Scope::Default, &[a::RUBY]) {
                    p.generate_implied_end_tags(&[b"rtc"]);
                }
                p.add_element();
            }
            a::MATH | a::SVG => {
                p.reconstruct_active_formatting_elements();
                if p.tok.data_atom == a::MATH {
                    adjust_attribute_names(&mut p.tok.attr, math_ml_attribute_adjustments);
                } else {
                    adjust_attribute_names(&mut p.tok.attr, svg_attribute_adjustments);
                }
                adjust_foreign_attributes(&mut p.tok.attr);
                p.add_element();
                let t = p.top();
                let ns = p.tok.data.clone();
                p.n_mut(t).namespace = ns;
                if p.has_self_closing_token {
                    p.oe_pop();
                    p.acknowledge_self_closing_tag();
                }
                return true;
            }
            a::CAPTION
            | a::COL
            | a::COLGROUP
            | a::FRAME
            | a::HEAD
            | a::TBODY
            | a::TD
            | a::TFOOT
            | a::TH
            | a::THEAD
            | a::TR => {
                // Ignore the token.
            }
            _ => {
                p.reconstruct_active_formatting_elements();
                p.add_element();
            }
        },
        TokenType::EndTag => match p.tok.data_atom {
            a::BODY => {
                if p.element_in_scope(Scope::Default, &[a::BODY]) {
                    p.im = Im::AfterBody;
                }
            }
            a::HTML => {
                if p.element_in_scope(Scope::Default, &[a::BODY]) {
                    p.parse_implied_token(TokenType::EndTag, a::BODY);
                    return false;
                }
                return true;
            }
            a::ADDRESS
            | a::ARTICLE
            | a::ASIDE
            | a::BLOCKQUOTE
            | a::BUTTON
            | a::CENTER
            | a::DETAILS
            | a::DIALOG
            | a::DIR
            | a::DIV
            | a::DL
            | a::FIELDSET
            | a::FIGCAPTION
            | a::FIGURE
            | a::FOOTER
            | a::HEADER
            | a::HGROUP
            | a::LISTING
            | a::MAIN
            | a::MENU
            | a::NAV
            | a::OL
            | a::PRE
            | a::SEARCH
            | a::SECTION
            | a::SUMMARY
            | a::UL => {
                let t = p.tok.data_atom;
                p.pop_until(Scope::Default, &[t]);
            }
            a::FORM => {
                if p.oe_contains(a::TEMPLATE) {
                    let i = p.index_of_element_in_scope(Scope::Default, &[a::FORM]);
                    if i == -1 {
                        // Ignore the token.
                        return true;
                    }
                    p.generate_implied_end_tags(&[]);
                    let n = p.oe_at(i);
                    if p.failed() {
                        return true;
                    }
                    if p.n(n).data_atom != a::FORM {
                        // Ignore the token.
                        return true;
                    }
                    p.pop_until(Scope::Default, &[a::FORM]);
                } else {
                    let node = p.form;
                    p.form = None;
                    let i = p.index_of_element_in_scope(Scope::Default, &[a::FORM]);
                    let Some(node) = node else {
                        // Ignore the token.
                        return true;
                    };
                    if i == -1 || p.oe[i as usize] != node {
                        // Ignore the token.
                        return true;
                    }
                    p.generate_implied_end_tags(&[]);
                    Parser::stack_remove(&mut p.oe, node);
                }
            }
            a::P => {
                if !p.element_in_scope(Scope::Button, &[a::P]) {
                    p.parse_implied_token(TokenType::StartTag, a::P);
                }
                p.pop_until(Scope::Button, &[a::P]);
            }
            a::LI => {
                p.pop_until(Scope::ListItem, &[a::LI]);
            }
            a::DD | a::DT => {
                let t = p.tok.data_atom;
                p.pop_until(Scope::Default, &[t]);
            }
            a::H1 | a::H2 | a::H3 | a::H4 | a::H5 | a::H6 => {
                p.pop_until(Scope::Default, &[a::H1, a::H2, a::H3, a::H4, a::H5, a::H6]);
            }
            a::A
            | a::B
            | a::BIG
            | a::CODE
            | a::EM
            | a::FONT
            | a::I
            | a::NOBR
            | a::S
            | a::SMALL
            | a::STRIKE
            | a::STRONG
            | a::TT
            | a::U => {
                let (t, d) = (p.tok.data_atom, p.tok.data.clone());
                p.in_body_end_tag_formatting(t, &d);
            }
            a::APPLET | a::MARQUEE | a::OBJECT => {
                let t = p.tok.data_atom;
                if p.pop_until(Scope::Default, &[t]) {
                    p.clear_active_formatting_elements();
                }
            }
            a::BR => {
                p.tok.typ = TokenType::StartTag;
                return false;
            }
            a::TEMPLATE => return in_head_im(p),
            _ => {
                let (t, d) = (p.tok.data_atom, p.tok.data.clone());
                p.in_body_end_tag_other(t, &d);
            }
        },
        TokenType::Comment => {
            p.add_comment_child();
        }
        TokenType::Error => {
            // TODO: remove this divergence from the HTML5 spec.
            if !p.template_stack.is_empty() {
                p.im = Im::InTemplate;
                return false;
            }
            for &e in &p.oe {
                match p.n(e).data_atom {
                    a::DD
                    | a::DT
                    | a::LI
                    | a::OPTGROUP
                    | a::OPTION
                    | a::P
                    | a::RB
                    | a::RP
                    | a::RT
                    | a::RTC
                    | a::TBODY
                    | a::TD
                    | a::TFOOT
                    | a::TH
                    | a::THEAD
                    | a::TR
                    | a::BODY
                    | a::HTML => {}
                    _ => return true,
                }
            }
        }
        TokenType::Doctype | TokenType::SelfClosingTag => {}
    }

    true
}

/// Section 12.2.6.4.8.
// Go: html/parse.go:textIM
fn text_im(p: &mut Parser) -> bool {
    match p.tok.typ {
        TokenType::Error => {
            p.oe_pop();
        }
        TokenType::Text => {
            let data = p.tok.data.clone();
            let mut d: &[u8] = &data;
            let n = p.oe_top_deref();
            if p.failed() {
                return true;
            }
            if p.n(n).data_atom == a::TEXTAREA && p.n(n).first_child.is_none() {
                // Ignore a newline at the start of a <textarea> block.
                if !d.is_empty() && d[0] == b'\r' {
                    d = &d[1..];
                }
                if !d.is_empty() && d[0] == b'\n' {
                    d = &d[1..];
                }
            }
            if d.is_empty() {
                return true;
            }
            p.add_text(d);
            return true;
        }
        TokenType::EndTag => {
            p.oe_pop();
        }
        _ => {}
    }
    match p.original_im.take() {
        Some(im) => p.im = im,
        None => p.im = Im::Nil,
    }
    p.tok.typ == TokenType::EndTag
}

/// Section 12.2.6.4.9.
// Go: html/parse.go:inTableIM
fn in_table_im(p: &mut Parser) -> bool {
    match p.tok.typ {
        TokenType::Text => {
            p.tok.data = remove_nul(&p.tok.data);
            let top = p.oe_top_deref();
            if p.failed() {
                return true;
            }
            if matches!(
                p.n(top).data_atom,
                a::TABLE | a::TBODY | a::TFOOT | a::THEAD | a::TR
            ) {
                let d = &p.tok.data;
                let trimmed = trim_left(d, WHITESPACE);
                if trimmed.iter().all(|c| WHITESPACE.contains(c)) {
                    let d = d.clone();
                    p.add_text(&d);
                    return true;
                }
            }
        }
        TokenType::StartTag => match p.tok.data_atom {
            a::CAPTION => {
                p.clear_stack_to_context(Scope::Table);
                p.afe.push(SCOPE_MARKER);
                p.add_element();
                p.im = Im::InCaption;
                return true;
            }
            a::COLGROUP => {
                p.clear_stack_to_context(Scope::Table);
                p.add_element();
                p.im = Im::InColumnGroup;
                return true;
            }
            a::COL => {
                p.parse_implied_token(TokenType::StartTag, a::COLGROUP);
                return false;
            }
            a::TBODY | a::TFOOT | a::THEAD => {
                p.clear_stack_to_context(Scope::Table);
                p.add_element();
                p.im = Im::InTableBody;
                return true;
            }
            a::TD | a::TH | a::TR => {
                p.parse_implied_token(TokenType::StartTag, a::TBODY);
                return false;
            }
            a::TABLE => {
                if p.pop_until(Scope::Table, &[a::TABLE]) {
                    p.reset_insertion_mode();
                    return false;
                }
                // Ignore the token.
                return true;
            }
            a::STYLE | a::SCRIPT | a::TEMPLATE => return in_head_im(p),
            a::INPUT => {
                let hidden = p.tok.attr.iter().any(|t| {
                    t.key == b"type" && go_unicode::strings::equal_fold(&t.val, b"hidden")
                });
                if hidden {
                    p.add_element();
                    p.oe_pop();
                    return true;
                }
                // Otherwise drop down to the default action.
            }
            a::FORM => {
                if p.oe_contains(a::TEMPLATE) || p.form.is_some() {
                    // Ignore the token.
                    return true;
                }
                p.add_element();
                p.form = Some(p.oe_pop());
            }
            a::SELECT => {
                p.reconstruct_active_formatting_elements();
                if matches!(
                    p.n(p.top()).data_atom,
                    a::TABLE | a::TBODY | a::TFOOT | a::THEAD | a::TR
                ) {
                    p.foster_parenting = true;
                }
                p.add_element();
                p.foster_parenting = false;
                p.frameset_ok = false;
                p.im = Im::InSelectInTable;
                return true;
            }
            _ => {}
        },
        TokenType::EndTag => match p.tok.data_atom {
            a::TABLE => {
                if p.pop_until(Scope::Table, &[a::TABLE]) {
                    p.reset_insertion_mode();
                    return true;
                }
                // Ignore the token.
                return true;
            }
            a::BODY
            | a::CAPTION
            | a::COL
            | a::COLGROUP
            | a::HTML
            | a::TBODY
            | a::TD
            | a::TFOOT
            | a::TH
            | a::THEAD
            | a::TR => {
                // Ignore the token.
                return true;
            }
            a::TEMPLATE => return in_head_im(p),
            _ => {}
        },
        TokenType::Comment => {
            p.add_comment_child();
            return true;
        }
        TokenType::Doctype => {
            // Ignore the token.
            return true;
        }
        TokenType::Error => return in_body_im(p),
        TokenType::SelfClosingTag => {}
    }

    p.foster_parenting = true;
    let r = in_body_im(p);
    // Go: `defer func() { p.fosterParenting = false }()`.
    p.foster_parenting = false;
    r
}

/// Section 12.2.6.4.11.
// Go: html/parse.go:inCaptionIM
fn in_caption_im(p: &mut Parser) -> bool {
    match p.tok.typ {
        TokenType::StartTag => match p.tok.data_atom {
            a::CAPTION | a::COL | a::COLGROUP | a::TBODY | a::TD | a::TFOOT | a::THEAD | a::TR => {
                if !p.pop_until(Scope::Table, &[a::CAPTION]) {
                    // Ignore the token.
                    return true;
                }
                p.clear_active_formatting_elements();
                p.im = Im::InTable;
                return false;
            }
            a::SELECT => {
                p.reconstruct_active_formatting_elements();
                p.add_element();
                p.frameset_ok = false;
                p.im = Im::InSelectInTable;
                return true;
            }
            _ => {}
        },
        TokenType::EndTag => match p.tok.data_atom {
            a::CAPTION => {
                if p.pop_until(Scope::Table, &[a::CAPTION]) {
                    p.clear_active_formatting_elements();
                    p.im = Im::InTable;
                }
                return true;
            }
            a::TABLE => {
                if !p.pop_until(Scope::Table, &[a::CAPTION]) {
                    // Ignore the token.
                    return true;
                }
                p.clear_active_formatting_elements();
                p.im = Im::InTable;
                return false;
            }
            a::BODY
            | a::COL
            | a::COLGROUP
            | a::HTML
            | a::TBODY
            | a::TD
            | a::TFOOT
            | a::TH
            | a::THEAD
            | a::TR => {
                // Ignore the token.
                return true;
            }
            _ => {}
        },
        _ => {}
    }
    in_body_im(p)
}

/// Section 12.2.6.4.12.
// Go: html/parse.go:inColumnGroupIM
fn in_column_group_im(p: &mut Parser) -> bool {
    match p.tok.typ {
        TokenType::Text => {
            let s = trim_left(&p.tok.data, WHITESPACE).to_vec();
            if s.len() < p.tok.data.len() {
                // Add the initial whitespace to the current node.
                let ws = p.tok.data[..p.tok.data.len() - s.len()].to_vec();
                p.add_text(&ws);
                if s.is_empty() {
                    return true;
                }
                p.tok.data = s;
            }
        }
        TokenType::Comment => {
            p.add_comment_child();
            return true;
        }
        TokenType::Doctype => {
            // Ignore the token.
            return true;
        }
        TokenType::StartTag => match p.tok.data_atom {
            a::HTML => return in_body_im(p),
            a::COL => {
                p.add_element();
                p.oe_pop();
                p.acknowledge_self_closing_tag();
                return true;
            }
            a::TEMPLATE => return in_head_im(p),
            _ => {}
        },
        TokenType::EndTag => match p.tok.data_atom {
            a::COLGROUP => {
                let top = p.oe_top_deref();
                if p.failed() {
                    return true;
                }
                if p.n(top).data_atom == a::COLGROUP {
                    p.oe_pop();
                    p.im = Im::InTable;
                }
                return true;
            }
            a::COL => {
                // Ignore the token.
                return true;
            }
            a::TEMPLATE => return in_head_im(p),
            _ => {}
        },
        TokenType::Error => return in_body_im(p),
        TokenType::SelfClosingTag => {}
    }
    let top = p.oe_top_deref();
    if p.failed() {
        return true;
    }
    if p.n(top).data_atom != a::COLGROUP {
        return true;
    }
    p.oe_pop();
    p.im = Im::InTable;
    false
}

/// Section 12.2.6.4.13.
// Go: html/parse.go:inTableBodyIM
fn in_table_body_im(p: &mut Parser) -> bool {
    match p.tok.typ {
        TokenType::StartTag => match p.tok.data_atom {
            a::TR => {
                p.clear_stack_to_context(Scope::TableBody);
                p.add_element();
                p.im = Im::InRow;
                return true;
            }
            a::TD | a::TH => {
                p.parse_implied_token(TokenType::StartTag, a::TR);
                return false;
            }
            a::CAPTION | a::COL | a::COLGROUP | a::TBODY | a::TFOOT | a::THEAD => {
                if p.pop_until(Scope::Table, &[a::TBODY, a::THEAD, a::TFOOT]) {
                    p.im = Im::InTable;
                    return false;
                }
                // Ignore the token.
                return true;
            }
            _ => {}
        },
        TokenType::EndTag => match p.tok.data_atom {
            a::TBODY | a::TFOOT | a::THEAD => {
                let t = p.tok.data_atom;
                if p.element_in_scope(Scope::Table, &[t]) {
                    p.clear_stack_to_context(Scope::TableBody);
                    p.oe_pop();
                    p.im = Im::InTable;
                }
                return true;
            }
            a::TABLE => {
                if p.pop_until(Scope::Table, &[a::TBODY, a::THEAD, a::TFOOT]) {
                    p.im = Im::InTable;
                    return false;
                }
                // Ignore the token.
                return true;
            }
            a::BODY | a::CAPTION | a::COL | a::COLGROUP | a::HTML | a::TD | a::TH | a::TR => {
                // Ignore the token.
                return true;
            }
            _ => {}
        },
        TokenType::Comment => {
            p.add_comment_child();
            return true;
        }
        _ => {}
    }

    in_table_im(p)
}

/// Section 12.2.6.4.14.
// Go: html/parse.go:inRowIM
fn in_row_im(p: &mut Parser) -> bool {
    match p.tok.typ {
        TokenType::StartTag => match p.tok.data_atom {
            a::TD | a::TH => {
                p.clear_stack_to_context(Scope::TableRow);
                p.add_element();
                p.afe.push(SCOPE_MARKER);
                p.im = Im::InCell;
                return true;
            }
            a::CAPTION | a::COL | a::COLGROUP | a::TBODY | a::TFOOT | a::THEAD | a::TR => {
                if p.pop_until(Scope::Table, &[a::TR]) {
                    p.im = Im::InTableBody;
                    return false;
                }
                // Ignore the token.
                return true;
            }
            _ => {}
        },
        TokenType::EndTag => match p.tok.data_atom {
            a::TR => {
                if p.pop_until(Scope::Table, &[a::TR]) {
                    p.im = Im::InTableBody;
                    return true;
                }
                // Ignore the token.
                return true;
            }
            a::TABLE => {
                if p.pop_until(Scope::Table, &[a::TR]) {
                    p.im = Im::InTableBody;
                    return false;
                }
                // Ignore the token.
                return true;
            }
            a::TBODY | a::TFOOT | a::THEAD => {
                let t = p.tok.data_atom;
                if p.element_in_scope(Scope::Table, &[t]) {
                    p.parse_implied_token(TokenType::EndTag, a::TR);
                    return false;
                }
                // Ignore the token.
                return true;
            }
            a::BODY | a::CAPTION | a::COL | a::COLGROUP | a::HTML | a::TD | a::TH => {
                // Ignore the token.
                return true;
            }
            _ => {}
        },
        _ => {}
    }

    in_table_im(p)
}

/// Section 12.2.6.4.15.
// Go: html/parse.go:inCellIM
fn in_cell_im(p: &mut Parser) -> bool {
    match p.tok.typ {
        TokenType::StartTag => match p.tok.data_atom {
            a::CAPTION
            | a::COL
            | a::COLGROUP
            | a::TBODY
            | a::TD
            | a::TFOOT
            | a::TH
            | a::THEAD
            | a::TR => {
                if p.pop_until(Scope::Table, &[a::TD, a::TH]) {
                    // Close the cell and reprocess.
                    p.clear_active_formatting_elements();
                    p.im = Im::InRow;
                    return false;
                }
                // Ignore the token.
                return true;
            }
            a::SELECT => {
                p.reconstruct_active_formatting_elements();
                p.add_element();
                p.frameset_ok = false;
                p.im = Im::InSelectInTable;
                return true;
            }
            _ => {}
        },
        TokenType::EndTag => match p.tok.data_atom {
            a::TD | a::TH => {
                let t = p.tok.data_atom;
                if !p.pop_until(Scope::Table, &[t]) {
                    // Ignore the token.
                    return true;
                }
                p.clear_active_formatting_elements();
                p.im = Im::InRow;
                return true;
            }
            a::BODY | a::CAPTION | a::COL | a::COLGROUP | a::HTML => {
                // Ignore the token.
                return true;
            }
            a::TABLE | a::TBODY | a::TFOOT | a::THEAD | a::TR => {
                let t = p.tok.data_atom;
                if !p.element_in_scope(Scope::Table, &[t]) {
                    // Ignore the token.
                    return true;
                }
                // Close the cell and reprocess.
                if p.pop_until(Scope::Table, &[a::TD, a::TH]) {
                    p.clear_active_formatting_elements();
                }
                p.im = Im::InRow;
                return false;
            }
            _ => {}
        },
        _ => {}
    }
    in_body_im(p)
}

/// Section 12.2.6.4.16.
// Go: html/parse.go:inSelectIM
fn in_select_im(p: &mut Parser) -> bool {
    match p.tok.typ {
        TokenType::Text => {
            let d = remove_nul(&p.tok.data);
            p.add_text(&d);
        }
        TokenType::StartTag => match p.tok.data_atom {
            a::HTML => return in_body_im(p),
            a::OPTION => {
                if p.n(p.top()).data_atom == a::OPTION {
                    p.oe_pop();
                }
                p.add_element();
            }
            a::OPTGROUP => {
                if p.n(p.top()).data_atom == a::OPTION {
                    p.oe_pop();
                }
                if p.n(p.top()).data_atom == a::OPTGROUP {
                    p.oe_pop();
                }
                p.add_element();
            }
            a::SELECT => {
                if !p.pop_until(Scope::Select, &[a::SELECT]) {
                    // Ignore the token.
                    return true;
                }
                p.reset_insertion_mode();
            }
            a::INPUT | a::KEYGEN | a::TEXTAREA => {
                if p.element_in_scope(Scope::Select, &[a::SELECT]) {
                    p.parse_implied_token(TokenType::EndTag, a::SELECT);
                    return false;
                }
                // In order to properly ignore <textarea>, we need to change the tokenizer mode.
                p.tokenizer.next_is_not_raw_text();
                // Ignore the token.
                return true;
            }
            a::SCRIPT | a::TEMPLATE => return in_head_im(p),
            a::IFRAME
            | a::NOEMBED
            | a::NOFRAMES
            | a::NOSCRIPT
            | a::PLAINTEXT
            | a::STYLE
            | a::TITLE
            | a::XMP => {
                // Don't let the tokenizer go into raw text mode when there are raw tags to be
                // ignored. These tags should be ignored from the tokenizer properly.
                p.tokenizer.next_is_not_raw_text();
                // Ignore the token.
                return true;
            }
            _ => {}
        },
        TokenType::EndTag => match p.tok.data_atom {
            a::OPTION => {
                if p.n(p.top()).data_atom == a::OPTION {
                    p.oe_pop();
                }
            }
            a::OPTGROUP => {
                let mut i = p.oe.len() as isize - 1;
                let n = p.oe_at(i);
                if p.failed() {
                    return true;
                }
                if p.n(n).data_atom == a::OPTION {
                    i -= 1;
                }
                let n = p.oe_at(i);
                if p.failed() {
                    return true;
                }
                if p.n(n).data_atom == a::OPTGROUP {
                    p.oe.truncate(i as usize);
                }
            }
            a::SELECT => {
                if !p.pop_until(Scope::Select, &[a::SELECT]) {
                    // Ignore the token.
                    return true;
                }
                p.reset_insertion_mode();
            }
            a::TEMPLATE => return in_head_im(p),
            _ => {}
        },
        TokenType::Comment => {
            p.add_comment_child();
        }
        TokenType::Doctype => {
            // Ignore the token.
            return true;
        }
        TokenType::Error => return in_body_im(p),
        TokenType::SelfClosingTag => {}
    }

    true
}

/// Section 12.2.6.4.17.
// Go: html/parse.go:inSelectInTableIM
fn in_select_in_table_im(p: &mut Parser) -> bool {
    if matches!(p.tok.typ, TokenType::StartTag | TokenType::EndTag)
        && matches!(
            p.tok.data_atom,
            a::CAPTION | a::TABLE | a::TBODY | a::TFOOT | a::THEAD | a::TR | a::TD | a::TH
        )
    {
        let t = p.tok.data_atom;
        if p.tok.typ == TokenType::EndTag && !p.element_in_scope(Scope::Table, &[t]) {
            // Ignore the token.
            return true;
        }
        // This is like p.popUntil(selectScope, a.Select), but it also matches <math select>,
        // not just <select>. Matching the MathML tag is arguably incorrect (conceptually), but
        // it mimics what Chromium does.
        for i in (0..p.oe.len()).rev() {
            if p.n(p.oe[i]).data_atom == a::SELECT {
                p.oe.truncate(i);
                break;
            }
        }
        p.reset_insertion_mode();
        return false;
    }
    in_select_im(p)
}

/// Section 12.2.6.4.18.
// Go: html/parse.go:inTemplateIM
fn in_template_im(p: &mut Parser) -> bool {
    match p.tok.typ {
        TokenType::Text | TokenType::Comment | TokenType::Doctype => in_body_im(p),
        TokenType::StartTag => {
            let next = match p.tok.data_atom {
                a::BASE
                | a::BASEFONT
                | a::BGSOUND
                | a::LINK
                | a::META
                | a::NOFRAMES
                | a::SCRIPT
                | a::STYLE
                | a::TEMPLATE
                | a::TITLE => return in_head_im(p),
                a::CAPTION | a::COLGROUP | a::TBODY | a::TFOOT | a::THEAD => Im::InTable,
                a::COL => Im::InColumnGroup,
                a::TR => Im::InTableBody,
                a::TD | a::TH => Im::InRow,
                _ => Im::InBody,
            };
            if p.template_stack.pop().is_none() {
                p.fail(index_oob(-1, 0));
                return true;
            }
            p.template_stack.push(next);
            p.im = next;
            false
        }
        TokenType::EndTag => match p.tok.data_atom {
            a::TEMPLATE => in_head_im(p),
            _ => {
                // Ignore the token.
                true
            }
        },
        TokenType::Error => {
            if !p.oe_contains(a::TEMPLATE) {
                // Ignore the token.
                return true;
            }
            // TODO: remove this divergence from the HTML5 spec.
            p.generate_implied_end_tags(&[]);
            for i in (0..p.oe.len()).rev() {
                let n = p.n(p.oe[i]);
                if n.namespace.is_empty() && n.data_atom == a::TEMPLATE {
                    p.oe.truncate(i);
                    break;
                }
            }
            p.clear_active_formatting_elements();
            if p.template_stack.pop().is_none() {
                p.fail(index_oob(-1, 0));
                return true;
            }
            p.reset_insertion_mode();
            false
        }
        TokenType::SelfClosingTag => false,
    }
}

/// Section 12.2.6.4.19.
// Go: html/parse.go:afterBodyIM
fn after_body_im(p: &mut Parser) -> bool {
    match p.tok.typ {
        TokenType::Error => {
            // Stop parsing.
            return true;
        }
        TokenType::Text => {
            let s = trim_left(&p.tok.data, WHITESPACE);
            if s.is_empty() {
                // It was all whitespace.
                return in_body_im(p);
            }
        }
        TokenType::StartTag => {
            if p.tok.data_atom == a::HTML {
                return in_body_im(p);
            }
        }
        TokenType::EndTag => {
            if p.tok.data_atom == a::HTML {
                if !p.fragment {
                    p.im = Im::AfterAfterBody;
                }
                return true;
            }
        }
        TokenType::Comment => {
            // The comment is attached to the <html> element.
            if p.oe.is_empty() || p.n(p.oe[0]).data_atom != a::HTML {
                p.fail(
                    "html: bad parser state: <html> element not found, in the after-body insertion mode",
                );
                return true;
            }
            let html = p.oe[0];
            p.append_comment(html);
            return true;
        }
        _ => {}
    }
    p.im = Im::InBody;
    false
}

/// Section 12.2.6.4.20.
// Go: html/parse.go:inFramesetIM
fn in_frameset_im(p: &mut Parser) -> bool {
    match p.tok.typ {
        TokenType::Comment => {
            p.add_comment_child();
        }
        TokenType::Text => {
            // Ignore all text but whitespace.
            let s = keep_whitespace(&p.tok.data);
            if !s.is_empty() {
                p.add_text(&s);
            }
        }
        TokenType::StartTag => match p.tok.data_atom {
            a::HTML => return in_body_im(p),
            a::FRAMESET => {
                p.add_element();
            }
            a::FRAME => {
                p.add_element();
                p.oe_pop();
                p.acknowledge_self_closing_tag();
            }
            a::NOFRAMES => return in_head_im(p),
            _ => {}
        },
        TokenType::EndTag => {
            if p.tok.data_atom == a::FRAMESET {
                let top = p.oe_top_deref();
                if p.failed() {
                    return true;
                }
                if p.n(top).data_atom != a::HTML {
                    p.oe_pop();
                    let top = p.oe_top_deref();
                    if p.failed() {
                        return true;
                    }
                    if p.n(top).data_atom != a::FRAMESET {
                        p.im = Im::AfterFrameset;
                        return true;
                    }
                }
            }
        }
        _ => {
            // Ignore the token.
        }
    }
    true
}

/// Section 12.2.6.4.21.
// Go: html/parse.go:afterFramesetIM
fn after_frameset_im(p: &mut Parser) -> bool {
    match p.tok.typ {
        TokenType::Comment => {
            p.add_comment_child();
        }
        TokenType::Text => {
            // Ignore all text but whitespace.
            let s = keep_whitespace(&p.tok.data);
            if !s.is_empty() {
                p.add_text(&s);
            }
        }
        TokenType::StartTag => match p.tok.data_atom {
            a::HTML => return in_body_im(p),
            a::NOFRAMES => return in_head_im(p),
            _ => {}
        },
        TokenType::EndTag => {
            if p.tok.data_atom == a::HTML {
                p.im = Im::AfterAfterFrameset;
                return true;
            }
        }
        _ => {
            // Ignore the token.
        }
    }
    true
}

/// Section 12.2.6.4.22.
// Go: html/parse.go:afterAfterBodyIM
fn after_after_body_im(p: &mut Parser) -> bool {
    match p.tok.typ {
        TokenType::Error => {
            // Stop parsing.
            return true;
        }
        TokenType::Text => {
            let s = trim_left(&p.tok.data, WHITESPACE);
            if s.is_empty() {
                // It was all whitespace.
                return in_body_im(p);
            }
        }
        TokenType::StartTag => {
            if p.tok.data_atom == a::HTML {
                return in_body_im(p);
            }
        }
        TokenType::Comment => {
            let doc = p.doc;
            p.append_comment(doc);
            return true;
        }
        TokenType::Doctype => return in_body_im(p),
        _ => {}
    }
    p.im = Im::InBody;
    false
}

/// Section 12.2.6.4.23.
// Go: html/parse.go:afterAfterFramesetIM
fn after_after_frameset_im(p: &mut Parser) -> bool {
    match p.tok.typ {
        TokenType::Comment => {
            let doc = p.doc;
            p.append_comment(doc);
        }
        TokenType::Text => {
            // Ignore all text but whitespace.
            let s = keep_whitespace(&p.tok.data);
            if !s.is_empty() {
                p.tok.data = s;
                return in_body_im(p);
            }
        }
        TokenType::StartTag => match p.tok.data_atom {
            a::HTML => return in_body_im(p),
            a::NOFRAMES => return in_head_im(p),
            _ => {}
        },
        TokenType::Doctype => return in_body_im(p),
        _ => {
            // Ignore the token.
        }
    }
    true
}

/// Go: `whitespaceOrNUL`.
const WHITESPACE_OR_NUL: &[u8] = b" \t\r\n\x0c\x00";

/// Section 12.2.6.5
// Go: html/parse.go:parseForeignContent
fn parse_foreign_content(p: &mut Parser) -> bool {
    match p.tok.typ {
        TokenType::Text => {
            if p.frameset_ok {
                p.frameset_ok = trim_left(&p.tok.data, WHITESPACE_OR_NUL).is_empty();
            }
            p.tok.data =
                go_unicode::bytes::replace_all(&p.tok.data, b"\x00", "\u{fffd}".as_bytes());
            let d = p.tok.data.clone();
            p.add_text(&d);
        }
        TokenType::Comment => {
            p.add_comment_child();
        }
        TokenType::StartTag => {
            if !p.fragment {
                let mut b = breakout(&p.tok.data);
                if p.tok.data_atom == a::FONT {
                    for attr in &p.tok.attr {
                        if matches!(attr.key.as_slice(), b"color" | b"face" | b"size") {
                            b = true;
                            break;
                        }
                    }
                }
                if b {
                    for i in (0..p.oe.len()).rev() {
                        let n = p.oe[i];
                        if p.n(n).namespace.is_empty()
                            || html_integration_point(&p.d, n)
                            || math_ml_text_integration_point(&p.d, n)
                        {
                            p.oe.truncate(i + 1);
                            break;
                        }
                    }
                    return false;
                }
            }
            let Some(current) = p.adjusted_current_node() else {
                p.fail(NIL_DEREF);
                return true;
            };
            match p.n(current).namespace.as_slice() {
                b"math" => adjust_attribute_names(&mut p.tok.attr, math_ml_attribute_adjustments),
                b"svg" => {
                    // Adjust SVG tag names. The tokenizer lower-cases tag names, but SVG wants
                    // e.g. "foreignObject" with a capital second "O".
                    if let Some(x) = svg_tag_name_adjustments(&p.tok.data) {
                        p.tok.data_atom = a::lookup(x);
                        p.tok.data = x.to_vec();
                    }
                    adjust_attribute_names(&mut p.tok.attr, svg_attribute_adjustments);
                }
                _ => {
                    p.fail("html: bad parser state: unexpected namespace");
                    return true;
                }
            }
            adjust_foreign_attributes(&mut p.tok.attr);
            let namespace = p.n(current).namespace.clone();
            p.add_element();
            let t = p.top();
            let ns_empty = namespace.is_empty();
            p.n_mut(t).namespace = namespace;
            if !ns_empty {
                // Don't let the tokenizer go into raw text mode in foreign content (e.g. in an
                // SVG <title> tag).
                p.tokenizer.next_is_not_raw_text();
            }
            if p.has_self_closing_token {
                p.oe_pop();
                p.acknowledge_self_closing_tag();
            }
        }
        TokenType::EndTag => {
            for i in (0..p.oe.len()).rev() {
                if p.n(p.oe[i]).namespace.is_empty() {
                    return p.call(p.im);
                }
                if go_unicode::strings::equal_fold(&p.n(p.oe[i]).data, &p.tok.data) {
                    p.oe.truncate(i);
                    break;
                }
            }
            return true;
        }
        _ => {
            // Ignore the token.
        }
    }
    true
}

/// Go: `html.Parse(r)` — returns the parse tree for the HTML from the given input, or the text
/// of the Go panic the input triggers.
// Go: html/parse.go:ParseWithOptions
pub fn parse(input: &[u8]) -> Result<(Document, NodeId), String> {
    let mut d = Document::new();
    let doc = d.add(Node::new(NodeType::Document, 0, Vec::new(), Vec::new()));
    let mut p = Parser {
        tokenizer: Tokenizer::new(input),
        tok: Token::default(),
        has_self_closing_token: false,
        d,
        doc,
        oe: Vec::new(),
        afe: Vec::new(),
        head: None,
        form: None,
        scripting: true,
        frameset_ok: true,
        template_stack: Vec::new(),
        im: Im::Initial,
        original_im: None,
        foster_parenting: false,
        quirks: false,
        fragment: false,
        context: None,
        err: None,
    };
    p.parse()?;
    Ok((p.d, p.doc))
}
